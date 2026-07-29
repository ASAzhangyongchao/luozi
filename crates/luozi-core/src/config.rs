use serde::{Deserialize, Serialize};

use crate::providers::AsrProtocol;

/// Current on-disk / IPC config schema. Bump only with a migration plan.
pub const DEFAULT_SCHEMA_VERSION: u32 = 1;

pub const GROQ_PROVIDER_ID: &str = "groq";
pub const GROQ_DEFAULT_BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const GROQ_DEFAULT_MODEL: &str = "whisper-large-v3-turbo";

/// Text AI is a separate capability from ASR (spec §8.6); consent/key must not be shared silently.
pub const GROQ_TEXT_AI_PROVIDER_ID: &str = "textai.groq";
pub const GROQ_TEXT_AI_DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

/// Where to send audio for ASR (spec §8.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AsrMode {
    LocalOnly,
    CloudOnly,
    #[default]
    Auto,
}

impl AsrMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnly => "localOnly",
            Self::CloudOnly => "cloudOnly",
            Self::Auto => "auto",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::LocalOnly => "仅本地",
            Self::CloudOnly => "仅云端",
            Self::Auto => "自动（本地优先）",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Auto => Self::LocalOnly,
            Self::LocalOnly => Self::CloudOnly,
            Self::CloudOnly => Self::Auto,
        }
    }
}

/// Cloud ASR settings without secrets (Key lives in OS credential store).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAsrConfig {
    pub provider_id: String,
    pub base_url: String,
    pub model: String,
    /// Keychain account id, e.g. `asr.groq`.
    pub credential_ref: String,
    /// Vendor wire protocol (defaults to OpenAI transcriptions for old configs).
    #[serde(default)]
    pub protocol: AsrProtocol,
}

impl Default for CloudAsrConfig {
    fn default() -> Self {
        Self {
            provider_id: GROQ_PROVIDER_ID.into(),
            base_url: GROQ_DEFAULT_BASE_URL.into(),
            model: GROQ_DEFAULT_MODEL.into(),
            credential_ref: "asr.groq".into(),
            protocol: AsrProtocol::OpenaiTranscriptions,
        }
    }
}

impl CloudAsrConfig {
    pub fn host(&self) -> Option<String> {
        url_host(&self.base_url)
    }
}

/// Text AI (chat) settings without secrets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAiConfig {
    pub provider_id: String,
    pub base_url: String,
    pub model: String,
    /// Keychain account id, e.g. `textai.groq`.
    pub credential_ref: String,
}

impl Default for TextAiConfig {
    fn default() -> Self {
        Self {
            provider_id: GROQ_TEXT_AI_PROVIDER_ID.into(),
            base_url: GROQ_DEFAULT_BASE_URL.into(),
            model: GROQ_TEXT_AI_DEFAULT_MODEL.into(),
            credential_ref: "textai.groq".into(),
        }
    }
}

impl TextAiConfig {
    pub fn host(&self) -> Option<String> {
        url_host(&self.base_url)
    }
}

/// Extract host from an http(s) URL without pulling in a URL crate.
pub fn url_host(base_url: &str) -> Option<String> {
    let rest = base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))?;
    let host = rest
        .split('/')
        .next()?
        .split('@')
        .next_back()?
        .split(':')
        .next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// Application configuration for M1+ skeleton.
///
/// Shortcut fields are **provisional** until dual-platform M0 Go. Do not treat
/// them as a shipped product promise in user-facing copy without that gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub schema_version: u32,
    /// Provisional macOS continue-speaking binding from M0 Spike.
    pub continue_speaking_shortcut: String,
    /// Provisional macOS voice-edit binding from M0 Spike.
    pub voice_edit_shortcut: String,
    /// Design default: hold-to-talk.
    pub hold_to_talk: bool,
    /// When true, UI/README must say shortcuts are provisional.
    pub shortcuts_provisional: bool,
    /// Recognition language preference; `"auto"` selects automatically.
    pub language: String,
    /// Local / cloud / auto routing (M5).
    #[serde(default)]
    pub asr_mode: AsrMode,
    /// Active cloud preset (no API key).
    #[serde(default)]
    pub cloud_asr: CloudAsrConfig,
    /// Text AI chat preset (no API key). Separate consent from ASR.
    #[serde(default)]
    pub text_ai: TextAiConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: DEFAULT_SCHEMA_VERSION,
            continue_speaking_shortcut: "Control+Alt+Space".into(),
            voice_edit_shortcut: "Control+Alt+Shift+Space".into(),
            hold_to_talk: true,
            shortcuts_provisional: true,
            language: "auto".into(),
            asr_mode: AsrMode::Auto,
            cloud_asr: CloudAsrConfig::default(),
            text_ai: TextAiConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != DEFAULT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema_version {}; expected {DEFAULT_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.continue_speaking_shortcut.trim().is_empty() {
            return Err("continue_speaking_shortcut must not be empty".into());
        }
        if self.voice_edit_shortcut.trim().is_empty() {
            return Err("voice_edit_shortcut must not be empty".into());
        }
        if self.language.trim().is_empty() {
            return Err("language must not be empty".into());
        }
        if self.cloud_asr.provider_id.trim().is_empty() {
            return Err("cloud_asr.provider_id must not be empty".into());
        }
        if self.cloud_asr.base_url.trim().is_empty() {
            return Err("cloud_asr.base_url must not be empty".into());
        }
        if self.cloud_asr.credential_ref.trim().is_empty() {
            return Err("cloud_asr.credential_ref must not be empty".into());
        }
        if self.text_ai.provider_id.trim().is_empty() {
            return Err("text_ai.provider_id must not be empty".into());
        }
        if self.text_ai.base_url.trim().is_empty() {
            return Err("text_ai.base_url must not be empty".into());
        }
        if self.text_ai.credential_ref.trim().is_empty() {
            return Err("text_ai.credential_ref must not be empty".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_schema_v1_and_provisional() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.schema_version, 1);
        assert!(cfg.shortcuts_provisional);
        assert!(cfg.hold_to_talk);
        assert_eq!(cfg.language, "auto");
        assert_eq!(cfg.asr_mode, AsrMode::Auto);
        assert_eq!(cfg.cloud_asr.provider_id, GROQ_PROVIDER_ID);
        cfg.validate().expect("default config valid");
    }

    #[test]
    fn json_roundtrip_stable() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
        assert!(json.contains("\"schemaVersion\": 1"));
        assert!(json.contains("\"shortcutsProvisional\": true"));
        assert!(json.contains("\"asrMode\""));
    }

    #[test]
    fn old_json_without_m5_fields_deserializes() {
        let json = r#"{
          "schemaVersion": 1,
          "continueSpeakingShortcut": "Control+Alt+Space",
          "voiceEditShortcut": "Control+Alt+M",
          "holdToTalk": true,
          "shortcutsProvisional": true,
          "language": "auto"
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.asr_mode, AsrMode::Auto);
        assert_eq!(cfg.cloud_asr.model, GROQ_DEFAULT_MODEL);
        assert_eq!(cfg.text_ai.model, GROQ_TEXT_AI_DEFAULT_MODEL);
        assert_eq!(cfg.voice_edit_shortcut, "Control+Alt+M");
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let cfg = AppConfig {
            schema_version: 2,
            ..AppConfig::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn url_host_parses_groq() {
        assert_eq!(
            url_host(GROQ_DEFAULT_BASE_URL).as_deref(),
            Some("api.groq.com")
        );
    }

    #[test]
    fn asr_mode_cycles() {
        assert_eq!(AsrMode::Auto.cycle(), AsrMode::LocalOnly);
        assert_eq!(AsrMode::LocalOnly.cycle(), AsrMode::CloudOnly);
        assert_eq!(AsrMode::CloudOnly.cycle(), AsrMode::Auto);
    }
}
