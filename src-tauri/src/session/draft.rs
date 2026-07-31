//! Draft persistence + last transcript TTL (M6) + selection / pending edit (M7).

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use luozi_core::DraftDocument;
use serde::{Deserialize, Serialize};

use super::asr::APP_SUPPORT_DIR_NAME;
use super::history::HistoryStore;

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

#[derive(Clone, Debug, Default)]
struct Selection {
    /// UTF-8 byte offsets into draft text.
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
pub struct PendingEdit {
    pub start: usize,
    pub end: usize,
    pub proposed: String,
}

struct LastTranscript {
    text: String,
    at: Instant,
}

pub struct DraftStore {
    doc: Mutex<DraftDocument>,
    last: Mutex<Option<LastTranscript>>,
    dirty: Mutex<bool>,
    selection: Mutex<Selection>,
    pending: Mutex<Option<PendingEdit>>,
    history: HistoryStore,
}

impl Default for DraftStore {
    fn default() -> Self {
        let text = load_from_disk().unwrap_or_default();
        Self {
            doc: Mutex::new(DraftDocument::from_text(text)),
            last: Mutex::new(None),
            dirty: Mutex::new(false),
            selection: Mutex::new(Selection::default()),
            pending: Mutex::new(None),
            history: HistoryStore::default(),
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
            .map(|g| matches!(g.as_ref(), Some(l) if l.at.elapsed() < LAST_TTL))
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
        let _ = self.pending.lock().map(|mut p| *p = None);
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
        self.history.push(t);
    }

    pub fn history_list(&self) -> Vec<super::history::HistoryItemDto> {
        self.history.list()
    }

    pub fn load_history_into_draft(&self, id: &str) -> Result<DraftStateDto, String> {
        let Some(item) = self.history.get(id) else {
            return Err("history_item_missing".into());
        };
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            if doc.text().trim().is_empty() {
                doc.apply(item.text);
            } else {
                let mut next = doc.text().to_string();
                if !next.ends_with('\n') {
                    next.push('\n');
                }
                next.push_str(&item.text);
                doc.apply(next);
            }
        }
        self.flush()?;
        Ok(self.snapshot())
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

    pub fn is_empty(&self) -> bool {
        self.doc
            .lock()
            .ok()
            .map(|d| d.text().trim().is_empty())
            .unwrap_or(true)
    }

    pub fn text_snapshot(&self) -> String {
        self.doc
            .lock()
            .ok()
            .map(|d| d.text().to_string())
            .unwrap_or_default()
    }

    /// UTF-8 byte offsets from the workbench textarea.
    pub fn set_selection(&self, start: usize, end: usize) {
        if let Ok(mut g) = self.selection.lock() {
            *g = Selection { start, end };
        }
    }

    pub fn selection(&self) -> (usize, usize) {
        self.selection
            .lock()
            .ok()
            .map(|g| (g.start, g.end))
            .unwrap_or((0, 0))
    }

    pub fn append_transcript(&self, chunk: &str) -> Result<(), String> {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            return Ok(());
        }
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            let mut next = doc.text().to_string();
            if !next.is_empty() && !next.ends_with('\n') && !next.ends_with(' ') {
                next.push('\n');
            }
            next.push_str(chunk);
            doc.apply(next);
        }
        self.remember_transcript(chunk);
        self.flush()?;
        Ok(())
    }

    pub fn insert_at(
        &self,
        start: usize,
        end: usize,
        chunk: &str,
    ) -> Result<DraftStateDto, String> {
        {
            let mut doc = self.doc.lock().map_err(|_| "draft_lock_failed")?;
            doc.replace_range(start, end, chunk)?;
        }
        self.flush()?;
        Ok(self.snapshot())
    }

    pub fn set_pending(&self, pending: PendingEdit) {
        if let Ok(mut g) = self.pending.lock() {
            *g = Some(pending);
        }
    }

    pub fn take_pending(&self) -> Option<PendingEdit> {
        self.pending.lock().ok().and_then(|mut g| g.take())
    }

    pub fn clear_pending(&self) {
        let _ = self.pending.lock().map(|mut g| *g = None);
    }

    pub fn apply_pending(&self) -> Result<DraftStateDto, String> {
        let Some(p) = self.take_pending() else {
            return Err("pending_edit_missing".into());
        };
        self.insert_at(p.start, p.end, &p.proposed)
    }
}
