//! Local dictation history (Typeless-inspired, Mac App Support JSON).

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::asr::APP_SUPPORT_DIR_NAME;

const MAX_ENTRIES: usize = 50;
/// Cap each stored transcript so history.json stays small on disk.
const MAX_CHARS: usize = 2_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItemDto {
    pub id: String,
    pub text: String,
    pub created_at: i64,
    pub preview: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiskHistory {
    schema_version: u32,
    items: Vec<HistoryItemDto>,
}

pub struct HistoryStore {
    items: Mutex<Vec<HistoryItemDto>>,
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self {
            items: Mutex::new(load_from_disk().unwrap_or_default()),
        }
    }
}

fn history_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support")
            .join(APP_SUPPORT_DIR_NAME)
            .join("history.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_SUPPORT_DIR_NAME)
            .join("history.json")
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn now_secs() -> i64 {
    (now_millis() / 1000) as i64
}

fn truncate_chars(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push('…');
    out
}

fn preview_of(text: &str) -> String {
    truncate_chars(text.trim(), 120)
}

fn load_from_disk() -> Option<Vec<HistoryItemDto>> {
    let bytes = fs::read(history_path()).ok()?;
    let disk: DiskHistory = serde_json::from_slice(&bytes).ok()?;
    Some(disk.items)
}

fn atomic_write(items: &[HistoryItemDto]) -> Result<(), String> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("history_dir_failed: {e}"))?;
    }
    let tmp = path.with_extension("json.partial");
    let payload = DiskHistory {
        schema_version: 1,
        items: items.to_vec(),
    };
    // Compact JSON — smaller on disk than pretty print.
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| format!("history_write_failed: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("history_rename_failed: {e}"))?;
    Ok(())
}

impl HistoryStore {
    pub fn list(&self) -> Vec<HistoryItemDto> {
        self.items
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn push(&self, text: &str) {
        let t = text.trim();
        if t.is_empty() {
            return;
        }
        let body = truncate_chars(t, MAX_CHARS);
        let item = HistoryItemDto {
            id: format!("{}", now_millis()),
            text: body.clone(),
            created_at: now_secs(),
            preview: preview_of(&body),
        };
        let Ok(mut g) = self.items.lock() else {
            return;
        };
        g.insert(0, item);
        g.truncate(MAX_ENTRIES);
        let _ = atomic_write(&g);
    }

    pub fn get(&self, id: &str) -> Option<HistoryItemDto> {
        self.items.lock().ok()?.iter().find(|i| i.id == id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_text() {
        assert_eq!(truncate_chars("你好", 10), "你好");
    }

    #[test]
    fn truncate_limits_long_text() {
        let s: String = "测".repeat(50);
        let out = truncate_chars(&s, 10);
        assert_eq!(out.chars().count(), 11); // 10 + ellipsis
        assert!(out.ends_with('…'));
    }
}
