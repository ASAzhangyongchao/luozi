//! Draft persistence + last transcript TTL (M6).

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use luozi_core::DraftDocument;
use serde::{Deserialize, Serialize};

use super::asr::APP_SUPPORT_DIR_NAME;

const LAST_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftStateDto {
    pub text: String,
    pub can_undo: bool,
    pub can_redo: bool,
    pub has_last_transcript: bool,
    pub save_status: String,
}

struct LastTranscript {
    text: String,
    at: Instant,
}

pub struct DraftStore {
    doc: Mutex<DraftDocument>,
    last: Mutex<Option<LastTranscript>>,
    dirty: Mutex<bool>,
}

impl Default for DraftStore {
    fn default() -> Self {
        let text = load_from_disk().unwrap_or_default();
        Self {
            doc: Mutex::new(DraftDocument::from_text(text)),
            last: Mutex::new(None),
            dirty: Mutex::new(false),
        }
    }
}

fn draft_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support")
            .join(APP_SUPPORT_DIR_NAME)
            .join("draft.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_SUPPORT_DIR_NAME)
            .join("draft.json")
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiskDraft {
    text: String,
}

fn load_from_disk() -> Option<String> {
    let bytes = fs::read(draft_path()).ok()?;
    let disk: DiskDraft = serde_json::from_slice(&bytes).ok()?;
    Some(disk.text)
}

fn atomic_write(text: &str) -> Result<(), String> {
    let path = draft_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("draft_dir_failed: {e}"))?;
    }
    let tmp = path.with_extension("json.partial");
    let payload = DiskDraft {
        text: text.to_string(),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| format!("draft_write_failed: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("draft_rename_failed: {e}"))?;
    Ok(())
}

impl DraftStore {
    pub fn snapshot(&self) -> DraftStateDto {
        let doc = self.doc.lock().ok();
        let has_last = self
            .last
            .lock()
            .ok()
            .map(|g| match g.as_ref() {
                Some(l) if l.at.elapsed() < LAST_TTL => true,
                _ => false,
            })
            .unwrap_or(false);
        match doc {
            Some(d) => DraftStateDto {
                text: d.text().to_string(),
                can_undo: d.can_undo(),
                can_redo: d.can_redo(),
                has_last_transcript: has_last,
                save_status: "ok".into(),
            },
            None => DraftStateDto {
                text: String::new(),
                can_undo: false,
                can_redo: false,
                has_last_transcript: has_last,
                save_status: "lock_failed".into(),
            },
        }
    }

    pub fn save_text(&self, text: String) -> Result<DraftStateDto, String> {
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            doc.apply(text);
        }
        self.flush()?;
        Ok(self.snapshot())
    }

    pub fn flush(&self) -> Result<(), String> {
        let text = self
            .doc
            .lock()
            .map_err(|_| "draft_lock_failed")?
            .text()
            .to_string();
        atomic_write(&text)?;
        if let Ok(mut d) = self.dirty.lock() {
            *d = false;
        }
        Ok(())
    }

    pub fn undo(&self) -> Result<DraftStateDto, String> {
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            if !doc.undo() {
                return Ok(self.snapshot());
            }
        }
        self.flush()?;
        Ok(self.snapshot())
    }

    pub fn redo(&self) -> Result<DraftStateDto, String> {
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            if !doc.redo() {
                return Ok(self.snapshot());
            }
        }
        self.flush()?;
        Ok(self.snapshot())
    }

    pub fn clear(&self) -> Result<DraftStateDto, String> {
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            doc.clear();
        }
        self.flush()?;
        Ok(self.snapshot())
    }

    pub fn remember_transcript(&self, text: &str) {
        let t = text.trim();
        if t.is_empty() {
            return;
        }
        if let Ok(mut g) = self.last.lock() {
            *g = Some(LastTranscript {
                text: t.to_string(),
                at: Instant::now(),
            });
        }
    }

    pub fn take_last_transcript(&self) -> Option<String> {
        let mut g = self.last.lock().ok()?;
        match g.take() {
            Some(l) if l.at.elapsed() < LAST_TTL => Some(l.text),
            _ => None,
        }
    }

    pub fn load_last_into_draft(&self) -> Result<DraftStateDto, String> {
        let Some(last) = self.take_last_transcript() else {
            return Err("last_transcript_missing".into());
        };
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            if doc.text().trim().is_empty() {
                doc.apply(last);
            } else {
                let mut next = doc.text().to_string();
                if !next.ends_with('\n') {
                    next.push('\n');
                }
                next.push_str(&last);
                doc.apply(next);
            }
        }
        self.flush()?;
        Ok(self.snapshot())
    }

    /// Insert transcript into draft at end (selection handled by frontend when open).
    pub fn append_transcript(&self, chunk: &str) -> Result<(), String> {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            return Ok(());
        }
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            let mut next = doc.text().to_string();
            if !next.is_empty() && !next.ends_with('\n') && !next.ends_with(' ') {
                // Prefer paragraph break for consecutive dictation into draft.
                next.push('\n');
            }
            next.push_str(chunk);
            doc.apply(next);
        }
        self.remember_transcript(chunk);
        self.flush()?;
        Ok(())
    }

    pub fn insert_at(&self, start: usize, end: usize, chunk: &str) -> Result<DraftStateDto, String> {
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            doc.replace_range(start, end, chunk)?;
        }
        self.remember_transcript(chunk);
        self.flush()?;
        Ok(self.snapshot())
    }
}
