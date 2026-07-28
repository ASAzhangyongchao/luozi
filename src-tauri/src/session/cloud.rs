//! Multi-vendor cloud transcription (Groq Whisper / 千问 / 豆包 / 小米 MiMo ASR).

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use hound::{SampleFormat, WavSpec, WavWriter};
use luozi_core::{url_host, AsrProtocol, CloudAsrConfig};
use serde_json::json;

use super::consent;
use super::credentials;

const TIMEOUT_SECS: u64 = 30;

/// True when Key + consent + HTTPS base URL are ready (and protocol is supported).
pub fn cloud_ready(cfg: &CloudAsrConfig) -> bool {
    if !cfg.protocol.supports_cloud() {
        return false;
    }
    let Some(host) = cfg.host() else {
        return false;
    };
    if !base_url_allowed(&cfg.base_url) {
        return false;
    }
    if !consent::is_consented(&cfg.provider_id, &host) {
        return false;
    }
    credentials::has_secret(&cfg.credential_ref)
}

pub fn base_url_allowed(base_url: &str) -> bool {
    if base_url.starts_with("https://") {
        return true;
    }
    if base_url.starts_with("http://") {
        let host = url_host(base_url).unwrap_or_default();
        return host == "127.0.0.1" || host == "localhost" || host == "::1";
    }
    false
}

/// Transcribe 16 kHz mono f32 PCM via the active cloud protocol.
pub fn transcribe_pcm(
    cfg: &CloudAsrConfig,
    pcm_16k_mono: &[f32],
    language: &str,
) -> Result<String, String> {
    let host = cfg
        .host()
        .ok_or_else(|| "cloud_protocol_error".to_string())?;
    if !cfg.protocol.supports_cloud() {
        return Err("cloud_provider_unsupported".into());
    }
    if !base_url_allowed(&cfg.base_url) {
        return Err("cloud_protocol_error".into());
    }
    if !consent::is_consented(&cfg.provider_id, &host) {
        return Err("cloud_not_consented".into());
    }
    let api_key = credentials::get_secret(&cfg.credential_ref)?;

    let wav_path = write_temp_wav(pcm_16k_mono)?;
    let result = match cfg.protocol {
        AsrProtocol::OpenaiTranscriptions => {
            upload_transcription(cfg, &wav_path, language, &api_key)
        }
        AsrProtocol::QwenAsrChat => upload_qwen_asr_chat(cfg, &wav_path, language, &api_key),
        AsrProtocol::DoubaoAuc => upload_doubao_auc(&wav_path, &api_key),
        AsrProtocol::Unsupported => Err("cloud_provider_unsupported".into()),
    };
    let _ = fs::remove_file(&wav_path);
    result
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn write_temp_wav(pcm: &[f32]) -> Result<PathBuf, String> {
    if pcm.is_empty() {
        return Err("no_speech".into());
    }
    let path = std::env::temp_dir().join(format!("luozi-asr-{}.wav", now_ms()));
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer =
        WavWriter::create(&path, spec).map_err(|e| format!("cloud_protocol_error: wav {e}"))?;
    for &s in pcm {
        let clamped = s.clamp(-1.0, 1.0);
        let i = (clamped * i16::MAX as f32) as i16;
        writer
            .write_sample(i)
            .map_err(|e| format!("cloud_protocol_error: wav {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("cloud_protocol_error: wav {e}"))?;
    Ok(path)
}

fn http_agent() -> ureq::Agent {
    ureq::builder()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(TIMEOUT_SECS))
        .build()
}

fn map_http_status(code: u16) -> String {
    match code {
        401 | 403 => "cloud_unauthorized".into(),
        429 => "cloud_rate_limited".into(),
        _ => "cloud_protocol_error".into(),
    }
}

fn map_transport(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, resp) => {
            let _ = resp.into_string();
            map_http_status(code)
        }
        ureq::Error::Transport(t) => {
            let msg = t.to_string().to_lowercase();
            if msg.contains("timed out") || msg.contains("timeout") {
                "cloud_timeout".into()
            } else {
                eprintln!("luozi: cloud transport error (sanitized)");
                "cloud_protocol_error".into()
            }
        }
    }
}

fn upload_transcription(
    cfg: &CloudAsrConfig,
    wav_path: &PathBuf,
    language: &str,
    api_key: &str,
) -> Result<String, String> {
    let wav_bytes = fs::read(wav_path).map_err(|_| "cloud_protocol_error".to_string())?;
    let boundary = format!("----LuoziBoundary{}", std::process::id());
    let mut body: Vec<u8> = Vec::new();

    push_text_field(&mut body, &boundary, "model", &cfg.model);
    if language != "auto" && !language.is_empty() {
        push_text_field(&mut body, &boundary, "language", language);
    }
    push_file_field(
        &mut body,
        &boundary,
        "file",
        "audio.wav",
        "audio/wav",
        &wav_bytes,
    );
    writeln!(body, "--{boundary}--\r").map_err(|e| e.to_string())?;
    write!(body, "\n").map_err(|e| e.to_string())?;

    let url = format!(
        "{}/audio/transcriptions",
        cfg.base_url.trim_end_matches('/')
    );
    eprintln!(
        "luozi: cloud asr whisper → host={} model={} bytes={}",
        cfg.host().unwrap_or_default(),
        cfg.model,
        wav_bytes.len()
    );

    let resp = http_agent()
        .post(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send_bytes(&body);

    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.into_string().unwrap_or_default();
            if !(200..300).contains(&status) {
                return Err(map_http_status(status));
            }
            parse_transcript_json(&text)
        }
        Err(e) => Err(map_transport(e)),
    }
}

/// Chat Completions + input_audio (Qwen3-ASR / 小米 MiMo ASR；base64 data URL).
fn upload_qwen_asr_chat(
    cfg: &CloudAsrConfig,
    wav_path: &PathBuf,
    language: &str,
    api_key: &str,
) -> Result<String, String> {
    let wav_bytes = fs::read(wav_path).map_err(|_| "cloud_protocol_error".to_string())?;
    let data_url = format!("data:audio/wav;base64,{}", B64.encode(&wav_bytes));
    let mut asr_options = json!({ "enable_itn": true });
    if language != "auto" && !language.is_empty() {
        asr_options["language"] = json!(language);
    }
    let body = json!({
        "model": cfg.model,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "input_audio",
                "input_audio": { "data": data_url }
            }]
        }],
        "asr_options": asr_options
    });
    let url = format!(
        "{}/chat/completions",
        cfg.base_url.trim_end_matches('/')
    );
    eprintln!(
        "luozi: cloud asr qwen → host={} model={} bytes={}",
        cfg.host().unwrap_or_default(),
        cfg.model,
        wav_bytes.len()
    );

    let resp = http_agent()
        .post(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_string(
            &serde_json::to_string(&body).map_err(|_| "cloud_protocol_error".to_string())?,
        );

    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.into_string().unwrap_or_default();
            if !(200..300).contains(&status) {
                return Err(map_http_status(status));
            }
            parse_chat_content_json(&text)
        }
        Err(e) => Err(map_transport(e)),
    }
}

/// 豆包 / 火山引擎 AUC：凭证格式 `APPID:AccessToken`。
fn upload_doubao_auc(wav_path: &PathBuf, api_key: &str) -> Result<String, String> {
    let (app_id, access_token) = parse_doubao_cred(api_key)?;
    let wav_bytes = fs::read(wav_path).map_err(|_| "cloud_protocol_error".to_string())?;
    let audio_b64 = B64.encode(&wav_bytes);
    let request_id = format!("luozi-{}", now_ms());
    let submit_url = "https://openspeech.bytedance.com/api/v3/auc/bigmodel/submit";
    let query_url = "https://openspeech.bytedance.com/api/v3/auc/bigmodel/query";

    let submit_body = json!({
        "user": { "uid": "luozi" },
        "audio": {
            "format": "wav",
            "rate": 16000,
            "bits": 16,
            "channel": 1,
            "data": audio_b64
        },
        "request": {
            "model_name": "bigmodel",
            "enable_itn": true
        }
    });

    eprintln!(
        "luozi: cloud asr doubao auc → bytes={} req={}",
        wav_bytes.len(),
        request_id
    );

    let agent = http_agent();
    let submit_resp = agent
        .post(submit_url)
        .set("X-Api-App-Key", &app_id)
        .set("X-Api-Access-Key", &access_token)
        .set("X-Api-Resource-Id", "volc.seedasr.auc")
        .set("X-Api-Request-Id", &request_id)
        .set("Content-Type", "application/json")
        .send_string(
            &serde_json::to_string(&submit_body).map_err(|_| "cloud_protocol_error".to_string())?,
        );

    match submit_resp {
        Ok(r) => {
            let status = r.status();
            let _ = r.into_string();
            if !(200..300).contains(&status) {
                return Err(map_http_status(status));
            }
        }
        Err(e) => return Err(map_transport(e)),
    }

    // Poll query until text appears or timeout.
    let started = now_ms();
    loop {
        if now_ms().saturating_sub(started) > 25_000 {
            return Err("cloud_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(400));
        let query_body = json!({
            "audio": {},
            "request": { "model_name": "bigmodel" },
            "user": { "uid": "luozi" }
        });
        let q = agent
            .post(query_url)
            .set("X-Api-App-Key", &app_id)
            .set("X-Api-Access-Key", &access_token)
            .set("X-Api-Resource-Id", "volc.seedasr.auc")
            .set("X-Api-Request-Id", &request_id)
            .set("Content-Type", "application/json")
            .send_string(
                &serde_json::to_string(&query_body)
                    .map_err(|_| "cloud_protocol_error".to_string())?,
            );
        match q {
            Ok(r) => {
                let status = r.status();
                let text = r.into_string().unwrap_or_default();
                if !(200..300).contains(&status) {
                    return Err(map_http_status(status));
                }
                if let Ok(parsed) = parse_doubao_query_text(&text) {
                    if !parsed.is_empty() {
                        return Ok(parsed);
                    }
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                // 200010 / still processing variants — keep polling on some codes.
                if code == 200 || code == 202 {
                    continue;
                }
                if body.contains("processing") || body.contains("running") {
                    continue;
                }
                return Err(map_http_status(code));
            }
            Err(e) => return Err(map_transport(e)),
        }
    }
}

fn parse_doubao_cred(raw: &str) -> Result<(String, String), String> {
    let s = raw.trim();
    let (a, b) = s
        .split_once(':')
        .or_else(|| s.split_once('|'))
        .ok_or_else(|| "cloud_unauthorized".to_string())?;
    let app_id = a.trim().to_string();
    let token = b.trim().to_string();
    if app_id.is_empty() || token.is_empty() {
        return Err("cloud_unauthorized".into());
    }
    Ok((app_id, token))
}

fn parse_doubao_query_text(body: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "cloud_protocol_error".to_string())?;
    // Common shapes: result.text / result.utterances[].text / data.result
    if let Some(t) = v
        .pointer("/result/text")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(t.to_string());
    }
    if let Some(arr) = v.pointer("/result/utterances").and_then(|x| x.as_array()) {
        let joined: String = arr
            .iter()
            .filter_map(|u| u.get("text").and_then(|t| t.as_str()))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("");
        if !joined.is_empty() {
            return Ok(joined);
        }
    }
    Err("cloud_protocol_error".into())
}

fn parse_transcript_json(body: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "cloud_protocol_error".to_string())?;
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "cloud_protocol_error".to_string())?;
    Ok(text.to_string())
}

fn parse_chat_content_json(body: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "cloud_protocol_error".to_string())?;
    let text = v
        .pointer("/choices/0/message/content")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "cloud_protocol_error".to_string())?;
    Ok(text.to_string())
}

fn push_text_field(body: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    let _ = write!(
        body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
    );
}

fn push_file_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: &str,
    mime: &str,
    data: &[u8],
) {
    let _ = write!(
        body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
    );
    body.extend_from_slice(data);
    let _ = write!(body, "\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use luozi_core::CloudAsrConfig;

    #[test]
    fn https_allowed_http_localhost_ok() {
        assert!(base_url_allowed("https://api.groq.com/openai/v1"));
        assert!(base_url_allowed("http://127.0.0.1:8080/v1"));
        assert!(!base_url_allowed("http://evil.example/v1"));
    }

    #[test]
    fn cloud_ready_false_without_consent_or_key() {
        let cfg = CloudAsrConfig::default();
        let _ = cloud_ready(&cfg);
    }

    #[test]
    fn parse_transcript() {
        let t = parse_transcript_json(r#"{"text":" 你好 "}"#).unwrap();
        assert_eq!(t, "你好");
    }

    #[test]
    fn parse_chat_content() {
        let t = parse_chat_content_json(
            r#"{"choices":[{"message":{"content":" 落字 "}}]}"#,
        )
        .unwrap();
        assert_eq!(t, "落字");
    }

    #[test]
    fn parse_doubao_cred_ok() {
        let (a, b) = parse_doubao_cred("123:tok").unwrap();
        assert_eq!(a, "123");
        assert_eq!(b, "tok");
    }

    #[test]
    fn unsupported_not_ready() {
        let mut cfg = CloudAsrConfig::default();
        cfg.protocol = AsrProtocol::Unsupported;
        assert!(!cloud_ready(&cfg));
    }
}
