//! Fold strategy + fold/unfold operations.
//!
//! Four strategies, all UTF-8 char-boundary safe:
//! - [`FoldStrategy::Truncate`] — cut at byte `limit` walking back to a char boundary
//! - [`FoldStrategy::HeadTail`] — keep first N + last N bytes, marker in the middle
//! - [`FoldStrategy::MarkerReplace`] — replace entire content with a lossless marker
//! - [`FoldStrategy::Summary`] — honest stub: same as Truncate (no internal LLM)

use std::fmt;

use super::marker::{FoldMarker, MarkerKind};

/// How to collapse a string that exceeds `limit` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldStrategy {
    /// Simple truncation at the byte limit (char-boundary safe).
    Truncate,
    /// Keep first N + last N bytes, mark the middle as collapsed.
    HeadTail,
    /// Replace content with a marker (lossless — original stored in marker).
    MarkerReplace,
    /// Summary. Without a caller-supplied LLM this is Truncate (honest stub).
    Summary,
}

/// Result of a fold: folded text, markers for unfold, and length accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldResult {
    /// Folded (possibly placeholder-bearing) string.
    pub folded: String,
    /// Markers needed to restore the original. Empty when no collapse happened
    /// or when the strategy is lossy (Truncate / Summary).
    pub markers: Vec<FoldMarker>,
    /// Original UTF-8 byte length.
    pub original_len: usize,
    /// Folded UTF-8 byte length.
    pub folded_len: usize,
}

/// Fold failure. Currently only `limit == 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldError {
    /// `limit` must be strictly greater than zero.
    InvalidLimit,
}

impl fmt::Display for FoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => write!(f, "fold limit must be > 0"),
        }
    }
}

impl std::error::Error for FoldError {}

/// Fold `content` under `strategy` so the result is at most `limit` bytes
/// (Truncate/Summary) or the original is replaced by a placeholder
/// (MarkerReplace / HeadTail). Content already `<= limit` is a no-op.
pub fn fold(content: &str, strategy: FoldStrategy, limit: usize) -> Result<FoldResult, FoldError> {
    if limit == 0 {
        return Err(FoldError::InvalidLimit);
    }
    let original_len = content.len();
    if original_len <= limit {
        return Ok(FoldResult {
            folded: content.to_string(),
            markers: Vec::new(),
            original_len,
            folded_len: content.len(),
        });
    }
    match strategy {
        FoldStrategy::Truncate | FoldStrategy::Summary => {
            let end = find_boundary(content, limit);
            Ok(FoldResult {
                folded: content[..end].to_string(),
                markers: vec![],
                original_len,
                folded_len: end,
            })
        }
        FoldStrategy::HeadTail => {
            let half = limit / 2;
            let head_end = find_boundary(content, half);
            let tail_start = find_boundary_from_end(content, half);
            // When UTF-8 boundary walking would invert the range (pathological
            // tiny limits on multibyte text), fall back to truncate.
            if head_end >= tail_start {
                let end = find_boundary(content, limit);
                return Ok(FoldResult {
                    folded: content[..end].to_string(),
                    markers: vec![],
                    original_len,
                    folded_len: end,
                });
            }
            let marker = FoldMarker {
                kind: MarkerKind::HeadTail,
                payload: content[head_end..tail_start].to_string(),
            };
            let folded = format!(
                "{}{}{}",
                &content[..head_end],
                marker.format_placeholder(),
                &content[tail_start..]
            );
            Ok(FoldResult {
                folded_len: folded.len(),
                folded,
                markers: vec![marker],
                original_len,
            })
        }
        FoldStrategy::MarkerReplace => {
            let marker = FoldMarker {
                kind: MarkerKind::Full,
                payload: content.to_string(),
            };
            let folded = marker.format_placeholder();
            Ok(FoldResult {
                folded_len: folded.len(),
                folded,
                markers: vec![marker],
                original_len,
            })
        }
    }
}

/// Restore original content from a folded string and its markers.
///
/// Replacement is sequential `str::replace`. Callers that emit identical
/// placeholders for distinct payloads must not rely on uniqueness.
pub fn unfold(content: &str, markers: &[FoldMarker]) -> String {
    let mut out = String::from(content);
    for marker in markers {
        out = out.replace(&marker.format_placeholder(), &marker.payload);
    }
    out
}

fn find_boundary(s: &str, target: usize) -> usize {
    let mut end = target.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

fn find_boundary_from_end(s: &str, tail_len: usize) -> usize {
    let start = s.len().saturating_sub(tail_len);
    let mut end = start;
    while end < s.len() && !s.is_char_boundary(end) {
        end += 1;
    }
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
        let r = fold(
            "hello world this is a long sentence",
            FoldStrategy::HeadTail,
            10,
        )
        .unwrap();
        assert!(
            r.folded.contains("HEADTAIL"),
            "should contain HEADTAIL marker, got: {}",
            r.folded
        );
        assert!(!r.markers.is_empty());
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
        assert_eq!(
            FoldError::InvalidLimit.to_string(),
            "fold limit must be > 0"
        );
    }

    #[test]
    fn unfold_empty_markers() {
        let s = unfold("hello world", &[]);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn utf8_boundary_safe() {
        let content = "你好世界这是一个测试字符串";
        let r = fold(content, FoldStrategy::Truncate, 10).unwrap();
        assert!(content.is_char_boundary(r.folded.len()));
        assert!(!r.folded.is_empty() || content.is_empty());
    }

    #[test]
    fn fold_exact_limit_boundary_does_not_fold() {
        let s = "exact12";
        let r = fold(s, FoldStrategy::Truncate, 7).unwrap();
        assert_eq!(r.folded, s);
        assert!(r.markers.is_empty());
    }

    #[test]
    fn fold_original_len_consistent_across_strategies() {
        let s = "same content for all 4 strategies test case";
        for strategy in [
            FoldStrategy::Truncate,
            FoldStrategy::HeadTail,
            FoldStrategy::MarkerReplace,
            FoldStrategy::Summary,
        ] {
            let r = fold(s, strategy, 10).unwrap();
            assert_eq!(r.original_len, s.len(), "strategy {strategy:?}");
        }
    }

    #[test]
    fn fold_limit_zero_error_all_strategies() {
        for strategy in [
            FoldStrategy::Truncate,
            FoldStrategy::HeadTail,
            FoldStrategy::MarkerReplace,
            FoldStrategy::Summary,
        ] {
            assert!(matches!(
                fold("x", strategy, 0),
                Err(FoldError::InvalidLimit)
            ));
        }
    }

    #[test]
    fn unfold_markerreplace_round_trip() {
        let original = "this is the original content that should be preserved exactly";
        let folded = fold(original, FoldStrategy::MarkerReplace, 10).unwrap();
        let restored = unfold(&folded.folded, &folded.markers);
        assert_eq!(restored, original);
    }

    #[test]
    fn unfold_headtail_round_trip() {
        let original = "abcdefghijklmnopqrstuvwxyz0123456789";
        let folded = fold(original, FoldStrategy::HeadTail, 10).unwrap();
        let restored = unfold(&folded.folded, &folded.markers);
        assert_eq!(restored, original);
    }

    #[test]
    fn fold_headtail_keeps_first_and_last_n() {
        let s = "abcdefghijklmnopqrstuvwxyz";
        let r = fold(s, FoldStrategy::HeadTail, 10).unwrap();
        assert!(r.folded.starts_with("abcde"));
        assert!(r.folded.ends_with("vwxyz"));
        assert!(r.folded.contains("HEADTAIL"));
        assert!(r.folded.contains("16 bytes"));
        assert_eq!(r.markers.len(), 1);
        assert_eq!(r.markers[0].kind, MarkerKind::HeadTail);
        assert!(r.markers[0].payload.contains("fghijklmnopqrstu"));
    }

    #[test]
    fn fold_truncate_shortens_to_limit() {
        let r = fold("hello world this is a test", FoldStrategy::Truncate, 10).unwrap();
        assert_eq!(r.folded, "hello worl");
        assert_eq!(r.folded_len, 10);
        assert!(r.markers.is_empty());
    }

    #[test]
    fn fold_truncate_unicode_preserves_char_boundary() {
        let s = "你好世界这是测试文本";
        let limit = 9;
        let r = fold(s, FoldStrategy::Truncate, limit).unwrap();
        assert!(r.folded.len() <= limit);
        assert!(s.is_char_boundary(r.folded.len()));
        assert_eq!(r.original_len, s.len());
        assert!(r.folded.chars().count() >= 3);
    }

    #[test]
    fn integration_fold_unfold_with_unicode() {
        let original = "用户消息 user-msg 🎉 测试 unicode round-trip";
        let folded = fold(original, FoldStrategy::MarkerReplace, 20).unwrap();
        let restored = unfold(&folded.folded, &folded.markers);
        assert_eq!(restored, original);
    }

    #[test]
    fn integration_fold_then_unfold_idempotent() {
        let original = "test content for fold/unfold idempotency check, multiple times";
        for _ in 0..3 {
            let folded = fold(original, FoldStrategy::MarkerReplace, 20).unwrap();
            let restored = unfold(&folded.folded, &folded.markers);
            assert_eq!(restored, original);
        }
    }

    #[test]
    fn head_tail_tiny_multibyte_does_not_panic() {
        // limit=1 on a 3-byte char: half=0, head_end=0, tail_start = len.
        // The whole string becomes the marker payload (degenerate HeadTail,
        // still lossless). Must not panic on the range.
        let original = "你好";
        let r = fold(original, FoldStrategy::HeadTail, 1).unwrap();
        assert!(
            r.folded.contains("HEADTAIL") || r.folded.chars().count() <= original.chars().count()
        );
        if !r.markers.is_empty() {
            assert_eq!(unfold(&r.folded, &r.markers), original);
        }
    }
}
