//! Single-draft document with bounded undo/redo (M6). No I/O.

const MAX_UNDO: usize = 20;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraftDocument {
    text: String,
    undo: Vec<String>,
    redo: Vec<String>,
}

impl DraftDocument {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Replace full text if changed; pushes previous onto undo stack.
    pub fn apply(&mut self, new_text: impl Into<String>) {
        let new_text = new_text.into();
        if new_text == self.text {
            return;
        }
        self.undo.push(std::mem::replace(&mut self.text, new_text));
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Insert `chunk` at `start..end` (UTF-8 byte indices must be char boundaries).
    pub fn replace_range(&mut self, start: usize, end: usize, chunk: &str) -> Result<(), String> {
        if start > end || end > self.text.len() {
            return Err("draft_range_invalid".into());
        }
        if !self.text.is_char_boundary(start) || !self.text.is_char_boundary(end) {
            return Err("draft_range_invalid".into());
        }
        let mut next = String::with_capacity(self.text.len() + chunk.len());
        next.push_str(&self.text[..start]);
        next.push_str(chunk);
        next.push_str(&self.text[end..]);
        self.apply(next);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.text, prev));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.text, next));
        true
    }

    pub fn clear(&mut self) {
        if self.text.is_empty() && self.undo.is_empty() && self.redo.is_empty() {
            return;
        }
        self.apply(String::new());
        // clear also wipes history after empty apply — keep one undo to empty? Spec: clear with confirm.
        // After clear, allow undo back once via apply; then drop stacks for a hard clear:
        self.undo.clear();
        self.redo.clear();
        self.text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_and_undo_redo() {
        let mut d = DraftDocument::new();
        d.apply("a");
        d.apply("ab");
        d.apply("abc");
        assert_eq!(d.text(), "abc");
        assert!(d.undo());
        assert_eq!(d.text(), "ab");
        assert!(d.redo());
        assert_eq!(d.text(), "abc");
    }

    #[test]
    fn undo_stack_caps_at_20() {
        let mut d = DraftDocument::new();
        for i in 0..25 {
            d.apply(format!("v{i}"));
        }
        assert_eq!(d.text(), "v24");
        let mut steps = 0;
        while d.undo() {
            steps += 1;
        }
        assert_eq!(steps, 20);
        assert_eq!(d.text(), "v4");
    }

    #[test]
    fn replace_range_inserts() {
        let mut d = DraftDocument::from_text("你好世界");
        // 你好|世界 — insert in middle
        let mid = "你好".len();
        d.replace_range(mid, mid, "，").unwrap();
        assert_eq!(d.text(), "你好，世界");
    }

    #[test]
    fn clear_empties() {
        let mut d = DraftDocument::from_text("x");
        d.apply("xy");
        d.clear();
        assert_eq!(d.text(), "");
        assert!(!d.can_undo());
    }
}
