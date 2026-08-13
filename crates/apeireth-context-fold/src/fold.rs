//! Fold strategy + fold/unfold operations.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use thiserror::Error;

use crate::marker::{FoldMarker, MarkerKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldStrategy {
    /// Simple truncation at char limit
    Truncate,
    /// Keep first N + last N chars, mark middle as collapsed
    HeadTail,
    /// Replace content with a marker (lossless — original stored in marker)
    MarkerReplace,
    /// Summary (requires user-supplied callback; here we just truncate)
    Summary,
}

#[derive(Debug, Clone)]
pub struct FoldResult {
    pub folded: String,
    pub markers: Vec<FoldMarker>,
    pub original_len: usize,
    pub folded_len: usize,
}

#[derive(Debug, Error)]
pub enum FoldError {
    #[error("fold limit must be > 0")]
    InvalidLimit,
}

pub fn fold(content: &str, strategy: FoldStrategy, limit: usize) -> Result<FoldResult, FoldError> {
    if limit == 0 { return Err(FoldError::InvalidLimit); }
    let original_len = content.len();
    if original_len <= limit {
        return Ok(FoldResult { folded: content.to_string(), markers: Vec::new(), original_len, folded_len: content.len() });
    }
    match strategy {
        FoldStrategy::Truncate => {
            let mut end = limit;
            while end > 0 && !content.is_char_boundary(end) { end -= 1; }
            Ok(FoldResult { folded: content[..end].to_string(), markers: vec![], original_len, folded_len: end })
        }
        FoldStrategy::HeadTail => {
            let half = limit / 2;
            let head_end = find_boundary(content, half);
            let tail_start = find_boundary_from_end(content, half);
            let marker = FoldMarker { kind: MarkerKind::HeadTail, payload: content[head_end..tail_start].to_string() };
            let folded = format!("{}{}{}", &content[..head_end], marker.format_placeholder(), &content[tail_start..]);
            Ok(FoldResult { folded_len: folded.len(), folded, markers: vec![marker], original_len })
        }
        FoldStrategy::MarkerReplace => {
            let marker = FoldMarker { kind: MarkerKind::Full, payload: content.to_string() };
            let folded = marker.format_placeholder();
            Ok(FoldResult { folded_len: folded.len(), folded, markers: vec![marker], original_len })
        }
        FoldStrategy::Summary => {
            // Honest stub: same as Truncate (no internal LLM)
            let mut end = limit;
            while end > 0 && !content.is_char_boundary(end) { end -= 1; }
            Ok(FoldResult { folded: content[..end].to_string(), markers: vec![], original_len, folded_len: end })
        }
    }
}

/// Unfold: restore original content from a folded result.
/// `content` is the folded string; `markers` are the original FoldMarker vecs.
pub fn unfold(content: &str, markers: &[FoldMarker]) -> String {
    let mut out = String::from(content);
    for marker in markers {
        out = out.replace(&marker.format_placeholder(), &marker.payload);
    }
    out
}

fn find_boundary(s: &str, target: usize) -> usize {
    let mut end = target.min(s.len());
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    end
}

fn find_boundary_from_end(s: &str, tail_len: usize) -> usize {
    let start = s.len().saturating_sub(tail_len);
    let mut end = start;
    while end < s.len() && !s.is_char_boundary(end) { end += 1; }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_fold_when_within_limit() {
        let r = fold("hello", FoldStrategy::Truncate, 100).unwrap();
        assert_eq!(r.folded, "hello");
        assert!(r.markers.is_empty());
    }

    #[test]
    fn truncate_strategy() {
        let r = fold("hello world", FoldStrategy::Truncate, 5).unwrap();
        assert_eq!(r.folded, "hello");
    }

    #[test]
    fn head_tail_strategy() {
        let r = fold("hello world this is a long sentence", FoldStrategy::HeadTail, 10).unwrap();
        assert!(r.folded.contains("HEADTAIL"), "should contain HEADTAIL marker, got: {}", r.folded);
        assert!(!r.markers.is_empty());
        // Unfold restores original
        let restored = unfold(&r.folded, &r.markers);
        assert_eq!(restored, "hello world this is a long sentence");
    }

    #[test]
    fn marker_replace_strategy() {
        let content = "very long content that should be replaced entirely";
        let r = fold(content, FoldStrategy::MarkerReplace, 5).unwrap();
        assert_eq!(r.markers.len(), 1);
        let restored = unfold(&r.folded, &r.markers);
        assert_eq!(restored, content);
    }

    #[test]
    fn summary_strategy_truncates() {
        let r = fold("hello world", FoldStrategy::Summary, 5).unwrap();
        assert_eq!(r.folded, "hello");
    }

    #[test]
    fn invalid_limit_errors() {
        let r = fold("hello", FoldStrategy::Truncate, 0);
        assert!(matches!(r, Err(FoldError::InvalidLimit)));
    }

    #[test]
    fn unfold_empty_markers() {
        let s = unfold("hello world", &[]);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn utf8_boundary_safe() {
        // Multibyte content
        let content = "你好世界这是一个测试字符串";
        let r = fold(content, FoldStrategy::Truncate, 10).unwrap();
        // Should not panic on boundary
        assert!(!r.folded.is_empty());
    }
}