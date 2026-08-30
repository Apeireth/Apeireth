//! FoldMarker: placeholder format for unfolded content.
//!
//! Placeholders embed only the *byte length* of the payload so the folded
//! string stays small. The original bytes live in [`FoldMarker::payload`] and
//! are restored by [`crate::context_fold::unfold`].

use serde::{Deserialize, Serialize};

/// Kind of collapse represented by a marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerKind {
    /// Full content replaced.
    Full,
    /// Head-tail collapse (middle is the payload).
    HeadTail,
    /// Semantic fold placeholder (low-relevance segment; payload = original).
    Semantic,
}

impl MarkerKind {
    /// Format template used by diagnostics (`{}` is not interpolated here).
    pub fn placeholder_format(&self) -> &'static str {
        match self {
            MarkerKind::Full => "<<FOLDED:{}>>",
            MarkerKind::HeadTail => "<<HEADTAIL:{}>>",
            MarkerKind::Semantic => "<<SEMANTIC:{}>>",
        }
    }
}

/// A lossless fold placeholder: kind + original payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldMarker {
    /// Collapse kind.
    pub kind: MarkerKind,
    /// Original bytes restored on unfold.
    pub payload: String,
}

impl FoldMarker {
    /// Construct a marker from any string-like payload.
    pub fn new(kind: MarkerKind, payload: impl Into<String>) -> Self {
        Self {
            kind,
            payload: payload.into(),
        }
    }

    /// Format this marker as a placeholder string suitable for embedding in
    /// folded content. Uses UTF-8 **byte** length so the placeholder stays small.
    pub fn format_placeholder(&self) -> String {
        let len = self.payload.len();
        match self.kind {
            MarkerKind::Full => format!("<<FOLDED:{len} bytes>>"),
            MarkerKind::HeadTail => format!("<<HEADTAIL:{len} bytes>>"),
            MarkerKind::Semantic => format!("<<SEMANTIC:{len} bytes>>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_kind_format() {
        assert_eq!(MarkerKind::Full.placeholder_format(), "<<FOLDED:{}>>");
        assert_eq!(MarkerKind::HeadTail.placeholder_format(), "<<HEADTAIL:{}>>");
        assert_eq!(MarkerKind::Semantic.placeholder_format(), "<<SEMANTIC:{}>>");
    }

    #[test]
    fn full_marker_placeholder() {
        let m = FoldMarker::new(MarkerKind::Full, "secret content");
        let p = m.format_placeholder();
        assert!(p.contains("FOLDED"));
        assert!(p.contains("14"));
    }

    #[test]
    fn head_tail_marker_placeholder() {
        let m = FoldMarker::new(MarkerKind::HeadTail, "middle");
        let p = m.format_placeholder();
        assert!(p.contains("HEADTAIL"));
    }

    #[test]
    fn marker_new_takes_into_string() {
        let m = FoldMarker::new(MarkerKind::Full, String::from("test"));
        assert_eq!(m.payload, "test");
    }

    #[test]
    fn marker_serde_roundtrip() {
        let m = FoldMarker::new(MarkerKind::Semantic, "段");
        let s = serde_json::to_string(&m).unwrap();
        let back: FoldMarker = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }
}
