//! Clipboard write + limited undo snapshot (spec §5.3–5.4).

use std::time::{Duration, Instant};

use arboard::Clipboard;

/// How long tray「撤销上次落字」remains available after a clipboard delivery.
pub const UNDO_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub struct ClipboardUndo {
    pub previous: Option<String>,
    pub written: String,
    pub at: Instant,
}

#[derive(Default)]
pub struct ClipboardGate {
    undo: Option<ClipboardUndo>,
}

impl ClipboardGate {
    pub fn write_with_undo(&mut self, text: &str) -> Result<(), String> {
        let mut clip = Clipboard::new().map_err(|e| format!("clipboard_unavailable: {e}"))?;
        let previous = clip.get_text().ok();
        clip.set_text(text.to_string())
            .map_err(|e| format!("clipboard_write_failed: {e}"))?;
        self.undo = Some(ClipboardUndo {
            previous,
            written: text.to_string(),
            at: Instant::now(),
        });
        Ok(())
    }

    pub fn undo_last(&mut self) -> Result<String, String> {
        let Some(snap) = self.undo.take() else {
            return Err("undo_unavailable".into());
        };
        if snap.at.elapsed() > UNDO_TTL {
            return Err("undo_expired".into());
        }

        let mut clip = Clipboard::new().map_err(|e| format!("clipboard_unavailable: {e}"))?;
        let current = clip.get_text().unwrap_or_default();
        if current != snap.written {
            // User already replaced clipboard; do not clobber.
            return Err("undo_stale_clipboard".into());
        }

        match snap.previous {
            Some(prev) => {
                clip.set_text(prev.clone())
                    .map_err(|e| format!("clipboard_write_failed: {e}"))?;
                Ok(prev)
            }
            None => {
                clip.clear()
                    .map_err(|e| format!("clipboard_clear_failed: {e}"))?;
                Ok(String::new())
            }
        }
    }

    pub fn clear_expired(&mut self) {
        if let Some(snap) = &self.undo {
            if snap.at.elapsed() > UNDO_TTL {
                self.undo = None;
            }
        }
    }

    pub fn can_undo(&mut self) -> bool {
        self.clear_expired();
        self.undo.is_some()
    }
}
