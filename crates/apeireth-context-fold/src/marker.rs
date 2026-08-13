//! FoldMarker: placeholder format for unfolded content.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// Full content replaced
    Full,
    /// Head-tail collapse (middle is the payload)
    HeadTail,
}

impl MarkerKind {
    pub fn placeholder_format(&self) -> &'static str {
        match self {
            MarkerKind::Full => "<<FOLDED:{}>>",
            MarkerKind::HeadTail => "<<HEADTAIL:{}>>",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FoldMarker {
    pub kind: MarkerKind,
    pub payload: String,
}

impl FoldMarker {
    pub fn new(kind: MarkerKind, payload: impl Into<String>) -> Self {
        Self { kind, payload: payload.into() }
    }
    /// Format this marker as a placeholder string (suitable for embedding in
    /// folded content).
    pub fn format_placeholder(&self) -> String {
        // Use byte length to keep placeholder small
        let len = self.payload.len();
        match self.kind {
            MarkerKind::Full => format!("<<FOLDED:{} bytes>>", len),
            MarkerKind::HeadTail => format!("<<HEADTAIL:{} bytes>>", len),
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
    }

    #[test]
    fn full_marker_placeholder() {
        let m = FoldMarker::new(MarkerKind::Full, "secret content");
        let p = m.format_placeholder();
        assert!(p.contains("FOLDED"));
        assert!(p.contains("14")); // length of "secret content"
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
}