//! OpenAI-compatible chat rewrite for draft scopes (M7). Separate from ASR.

use std::time::Duration;

use luozi_core::TextAiConfig;
use serde_json::json;

use super::cloud::base_url_allowed;
use super::consent;
use super::credentials;

const TIMEOUT_SECS: u64 = 20;

pub fn text_ai_ready(cfg: &TextAiConfig) -> bool {
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

/// Ask the model to rewrite only `scope_text` per `instruction`. Returns replacement plain text.
pub fn rewrite_scope(
    cfg: &TextAiConfig,
    instruction: &str,
    scope_text: &str,
) -> Result<String, String> {
    let host = cfg
        .host()
        .ok_or_else(|| "text_ai_protocol_error".to_string())?;
    if !base_url_allowed(&cfg.base_url) {
        return Err("text_ai_protocol_error".into());
    }
    if !consent::is_consented(&cfg.provider_id, &host) {
        return Err("text_ai_not_consented".into());
    }
    let api_key = credentials::get_secret(&cfg.credential_ref)
        .map_err(|_| "text_ai_unauthorized".to_string())?;

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let body = json!({
        "model": cfg.model,
        "temperature": 0.2,
        "messages": [
            {
                "role": "system",
                "content": "You rewrite Chinese/English draft text for a voice editor. Return ONLY the replacement for the given target fragment as plain text. Do not wrap in quotes or markdown. Keep digits, dates, acronyms, and proper names unchanged unless the user instruction explicitly asks to change them. Do not invent facts."
            },
            {
                "role": "user",
                "content": format!(
                    "修改要求：\n{instruction}\n\n—— 待修改片段（只改这一段，完整返回改后片段）——\n{scope_text}"
                )
            }
        ]
    });

    eprintln!("luozi: text_ai → host={} model={}", host, cfg.model);

    let agent = ureq::builder()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(TIMEOUT_SECS))
        .build();

    let payload = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_string(&payload);

    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.into_string().unwrap_or_default();
            if !(200..300).contains(&status) {
                eprintln!("luozi: text_ai http {status}");
                return Err(map_http_status(status));
            }
            let parsed: serde_json::Value =
                serde_json::from_str(&text).map_err(|_| "text_ai_protocol_error".to_string())?;
            let content = parsed
                .pointer("/choices/0/message/content")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "text_ai_invalid_patch".to_string())?;
            Ok(strip_wrapping_fences(content))
        }
        Err(ureq::Error::Status(code, _resp)) => {
            eprintln!("luozi: text_ai status {code}");
            Err(map_http_status(code))
        }
        Err(ureq::Error::Transport(t)) => {
            let msg = format!("{t}");
            eprintln!("luozi: text_ai transport: {msg}");
            if msg.to_lowercase().contains("timed out") || msg.to_lowercase().contains("timeout") {
                Err("text_ai_timeout".into())
            } else {
                Err("text_ai_protocol_error".into())
            }
        }
    }
}

fn map_http_status(code: u16) -> String {
    match code {
        401 | 403 => "text_ai_unauthorized".into(),
        429 => "text_ai_rate_limited".into(),
        _ => "text_ai_protocol_error".into(),
    }
}

fn strip_wrapping_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let rest = rest.strip_prefix("text").unwrap_or(rest);
        let rest = rest.trim_start_matches('\n');
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    t.to_string()
}
