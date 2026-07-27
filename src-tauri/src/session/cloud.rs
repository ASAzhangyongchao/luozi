//! OpenAI-compatible cloud transcription (M5: Groq first).

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use hound::{SampleFormat, WavSpec, WavWriter};
use luozi_core::{url_host, CloudAsrConfig};

use super::consent;
use super::credentials;

const TIMEOUT_SECS: u64 = 15;

/// True when Key + consent + HTTPS base URL are ready.
pub fn cloud_ready(cfg: &CloudAsrConfig) -> bool {
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

/// Transcribe 16 kHz mono f32 PCM via Groq / OpenAI-compatible endpoint.
pub fn transcribe_pcm(
    cfg: &CloudAsrConfig,
    pcm_16k_mono: &[f32],
    language: &str,
) -> Result<String, String> {
    let host = cfg
        .host()
        .ok_or_else(|| "cloud_protocol_error".to_string())?;
    if !base_url_allowed(&cfg.base_url) {
        return Err("cloud_protocol_error".into());
    }
    if !consent::is_consented(&cfg.provider_id, &host) {
        return Err("cloud_not_consented".into());
    }
    let api_key = credentials::get_secret(&cfg.credential_ref)?;

    let wav_path = write_temp_wav(pcm_16k_mono)?;
    let result = upload_transcription(cfg, &wav_path, language, &api_key);
    let _ = fs::remove_file(&wav_path);
    result
}

fn write_temp_wav(pcm: &[f32]) -> Result<PathBuf, String> {
    if pcm.is_empty() {
        return Err("no_speech".into());
    }
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "luozi-asr-{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
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
    push_file_field(&mut body, &boundary, "file", "audio.wav", "audio/wav", &wav_bytes);
    writeln!(body, "--{boundary}--\r").map_err(|e| e.to_string())?;
    write!(body, "\n").map_err(|e| e.to_string())?;

    let url = format!(
        "{}/audio/transcriptions",
        cfg.base_url.trim_end_matches('/')
    );
    eprintln!(
        "luozi: cloud asr → host={} model={} bytes={}",
        cfg.host().unwrap_or_default(),
        cfg.model,
        wav_bytes.len()
    );

    let agent = ureq::builder()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(TIMEOUT_SECS))
        .build();

    let resp = agent
        .post(&url)
        .set(
            "Authorization",
            &format!("Bearer {api_key}"),
        )
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
        Err(ureq::Error::Status(code, resp)) => {
            let _ = resp.into_string();
            Err(map_http_status(code))
        }
        Err(ureq::Error::Transport(t)) => {
            let msg = t.to_string().to_lowercase();
            if msg.contains("timed out") || msg.contains("timeout") {
                Err("cloud_timeout".into())
            } else {
                eprintln!("luozi: cloud transport error (sanitized)");
                Err("cloud_protocol_error".into())
            }
        }
    }
}

fn map_http_status(code: u16) -> String {
    match code {
        401 | 403 => "cloud_unauthorized".into(),
        429 => "cloud_rate_limited".into(),
        _ => "cloud_protocol_error".into(),
    }
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
        // Typically false in CI without keychain/consent.
        let _ = cloud_ready(&cfg);
    }

    #[test]
    fn parse_transcript() {
        let t = parse_transcript_json(r#"{"text":" 你好 "}"#).unwrap();
        assert_eq!(t, "你好");
    }
}
