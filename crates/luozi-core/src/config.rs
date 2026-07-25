use serde::{Deserialize, Serialize};

/// Current on-disk / IPC config schema. Bump only with a migration plan.
pub const DEFAULT_SCHEMA_VERSION: u32 = 1;

/// Application configuration for M1 skeleton.
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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: DEFAULT_SCHEMA_VERSION,
            continue_speaking_shortcut: "Control+Alt+Space".into(),
            voice_edit_shortcut: "Control+Alt+M".into(),
            hold_to_talk: true,
            shortcuts_provisional: true,
            language: "auto".into(),
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
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let cfg = AppConfig {
            schema_version: 2,
            ..AppConfig::default()
        };
        assert!(cfg.validate().is_err());
    }
}
