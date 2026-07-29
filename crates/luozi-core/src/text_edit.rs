//! Edit scope resolution, protected spans, and risk assessment (M7). No I/O.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditScopeKind {
    Selection,
    Paragraph,
    FullDocument,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditScope {
    pub start: usize,
    pub end: usize,
    pub kind: EditScopeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditRisk {
    Low,
    High { reasons: Vec<&'static str> },
}

/// Resolve the text range to rewrite from selection + instruction cues.
pub fn resolve_edit_scope(
    text: &str,
    sel_start: usize,
    sel_end: usize,
    instruction: &str,
) -> Result<EditScope, String> {
    let len = text.len();
    let (a, b) = (sel_start.min(sel_end), sel_start.max(sel_end));
    if a > len || b > len {
        return Err("edit_scope_invalid".into());
    }
    if !text.is_char_boundary(a) || !text.is_char_boundary(b) {
        return Err("edit_scope_invalid".into());
    }

    let instr = instruction.to_lowercase();
    let wants_full = instr.contains("全文")
        || instr.contains("整篇")
        || instr.contains("全部")
        || instr.contains("整个文档")
        || instr.contains("whole")
        || instr.contains("entire");

    if a != b {
        return Ok(EditScope {
            start: a,
            end: b,
            kind: EditScopeKind::Selection,
        });
    }

    if wants_full || text.trim().is_empty() {
        return Ok(EditScope {
            start: 0,
            end: len,
            kind: EditScopeKind::FullDocument,
        });
    }

    // Cursor only → current paragraph (split on blank lines).
    let (p_start, p_end) = paragraph_bounds(text, a);
    Ok(EditScope {
        start: p_start,
        end: p_end,
        kind: EditScopeKind::Paragraph,
    })
}

fn paragraph_bounds(text: &str, cursor: usize) -> (usize, usize) {
    let cursor = cursor.min(text.len());
    // Find paragraph start: after last \n\n before cursor (or start).
    let before = &text[..cursor];
    let p_start = before
        .rmatch_indices("\n\n")
        .next()
        .map(|(i, _)| i + 2)
        .unwrap_or(0);
    let after = &text[cursor..];
    let p_end = after.find("\n\n").map(|i| cursor + i).unwrap_or(text.len());
    // Trim trailing single newlines inside paragraph end for cleaner scopes.
    (p_start, p_end)
}

/// Protected spans: digit runs, simple dates, ASCII acronyms (2+ A-Z0-9).
pub fn extract_protected_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // ASCII digit run (also covers 2024-07-25 style when hyphen between digits)
        if b.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_digit()
                    || bytes[i] == b'-'
                    || bytes[i] == b'/'
                    || bytes[i] == b'.'
                    || bytes[i] == b':')
            {
                i += 1;
            }
            // trim trailing separators
            let mut end = i;
            while end > start && matches!(bytes[end - 1], b'-' | b'/' | b'.' | b':') {
                end -= 1;
            }
            if end > start {
                out.push((start, end));
            }
            continue;
        }
        // ASCII acronym / product codes: at least 2 of [A-Z0-9]
        if b.is_ascii_uppercase() || b.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_uppercase() || bytes[i].is_ascii_digit()) {
                i += 1;
            }
            if i - start >= 2 {
                out.push((start, i));
            }
            continue;
        }
        i += 1;
    }
    out
}

pub fn assess_edit_risk(
    scope: &EditScope,
    original_slice: &str,
    proposed: &str,
    instruction: &str,
) -> EditRisk {
    let mut reasons = Vec::new();
    if scope.kind == EditScopeKind::FullDocument {
        reasons.push("full_document");
    }

    let orig_chars = original_slice.chars().count().max(1);
    let prop_chars = proposed.chars().count();
    if prop_chars * 10 < orig_chars * 7 {
        // deleted more than ~30%
        reasons.push("large_deletion");
    }

    let instr = instruction.to_lowercase();
    let allows_protect_change = instr.contains("数字")
        || instr.contains("名字")
        || instr.contains("姓名")
        || instr.contains("日期")
        || instr.contains("改成")
        || instr.contains("换成")
        || instr.contains("number")
        || instr.contains("name")
        || instr.contains("date");

    if !allows_protect_change {
        for (s, e) in extract_protected_spans(original_slice) {
            let token = &original_slice[s..e];
            if !token.is_empty() && !proposed.contains(token) {
                reasons.push("protected_token_changed");
                break;
            }
        }
    }

    if reasons.is_empty() {
        EditRisk::Low
    } else {
        EditRisk::High { reasons }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wins() {
        let text = "第一段\n\n第二段内容";
        let scope = resolve_edit_scope(text, 0, "第一段".len(), "润色").unwrap();
        assert_eq!(scope.kind, EditScopeKind::Selection);
        assert_eq!(&text[scope.start..scope.end], "第一段");
    }

    #[test]
    fn paragraph_at_cursor() {
        let text = "AAA\n\nBBB行";
        let cursor = text.find('B').unwrap();
        let scope = resolve_edit_scope(text, cursor, cursor, "改简洁").unwrap();
        assert_eq!(scope.kind, EditScopeKind::Paragraph);
        assert_eq!(&text[scope.start..scope.end], "BBB行");
    }

    #[test]
    fn full_document_keyword() {
        let text = "你好世界";
        let scope = resolve_edit_scope(text, 0, 0, "把全文改正式一点").unwrap();
        assert_eq!(scope.kind, EditScopeKind::FullDocument);
        assert_eq!(scope.start, 0);
        assert_eq!(scope.end, text.len());
    }

    #[test]
    fn protected_digits_and_acronym() {
        let text = "订单 ASFF 金额 12800 元，日期 2026-07-27";
        let spans = extract_protected_spans(text);
        let tokens: Vec<&str> = spans.iter().map(|(s, e)| &text[*s..*e]).collect();
        assert!(tokens.iter().any(|t| *t == "ASFF"));
        assert!(tokens.iter().any(|t| *t == "12800"));
        assert!(tokens.iter().any(|t| *t == "2026-07-27"));
    }

    #[test]
    fn risk_high_when_number_dropped() {
        let scope = EditScope {
            start: 0,
            end: 10,
            kind: EditScopeKind::Paragraph,
        };
        let risk = assess_edit_risk(&scope, "共 12800 元", "共若干元", "润色一下");
        match risk {
            EditRisk::High { reasons } => assert!(reasons.contains(&"protected_token_changed")),
            EditRisk::Low => panic!("expected high risk"),
        }
    }

    #[test]
    fn risk_low_when_protect_kept() {
        let scope = EditScope {
            start: 0,
            end: 10,
            kind: EditScopeKind::Paragraph,
        };
        let risk = assess_edit_risk(&scope, "共 12800 元", "合计 12800 元。", "润色一下");
        assert_eq!(risk, EditRisk::Low);
    }
}
