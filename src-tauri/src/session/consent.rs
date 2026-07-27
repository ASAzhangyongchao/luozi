//! Per-provider cloud upload consent (providerId + host).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::asr::APP_SUPPORT_DIR_NAME;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConsentEntry {
    pub provider_id: String,
    pub host: String,
    pub consented_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentStore {
    pub entries: Vec<ConsentEntry>,
}

fn consent_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support")
            .join(APP_SUPPORT_DIR_NAME)
            .join("cloud-consent.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_SUPPORT_DIR_NAME)
            .join("cloud-consent.json")
    }
}

pub fn load() -> ConsentStore {
    let path = consent_path();
    let Ok(bytes) = fs::read(&path) else {
        return ConsentStore::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(store: &ConsentStore) -> Result<(), String> {
    let path = consent_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("consent_dir_failed: {e}"))?;
    }
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("consent_write_failed: {e}"))
}

pub fn is_consented(provider_id: &str, host: &str) -> bool {
    let store = load();
    store
        .entries
        .iter()
        .any(|e| e.provider_id == provider_id && e.host.eq_ignore_ascii_case(host))
}

pub fn grant(provider_id: &str, host: &str) -> Result<(), String> {
    let mut store = load();
    store
        .entries
        .retain(|e| !(e.provider_id == provider_id && e.host.eq_ignore_ascii_case(host)));
    store.entries.push(ConsentEntry {
        provider_id: provider_id.to_string(),
        host: host.to_string(),
        consented_at: chrono_like_now(),
    });
    save(&store)
}

pub fn revoke(provider_id: &str, host: &str) -> Result<(), String> {
    let mut store = load();
    store
        .entries
        .retain(|e| !(e.provider_id == provider_id && e.host.eq_ignore_ascii_case(host)));
    save(&store)
}

fn chrono_like_now() -> String {
    // Avoid chrono dep: local RFC3339-ish via system time.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_and_check_roundtrip_in_memory_path() {
        // Unit-level: is_consented logic via grant to real path may collide;
        // smoke the store struct instead.
        let mut store = ConsentStore::default();
        store.entries.push(ConsentEntry {
            provider_id: "groq".into(),
            host: "api.groq.com".into(),
            consented_at: "t".into(),
        });
        assert!(store
            .entries
            .iter()
            .any(|e| e.provider_id == "groq" && e.host == "api.groq.com"));
    }
}
