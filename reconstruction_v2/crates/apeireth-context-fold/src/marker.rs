//! Fold marker — placeholder format for unfold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Summary,
    Truncate,
    Semantic,
}

#[derive(Debug, Clone)]
pub struct FoldMarker {
    pub kind: MarkerKind,
    pub threshold: f64,
    pub content: String,
}

impl FoldMarker {
    pub fn new(kind: MarkerKind, threshold: f64, content: impl Into<String>) -> Self {
        Self { kind, threshold, content: content.into() }
    }

    pub fn render(&self) -> String {
        format!("[===vcp_fold:{:?}={:.2}===]", self.kind, self.threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_render() {
        let m = FoldMarker::new(MarkerKind::Summary, 0.75, "hi");
        let s = m.render();
        assert!(s.contains("vcp_fold"));
        assert!(s.contains("0.75"));
    }

    #[test]
    fn marker_variants() {
        let _ = MarkerKind::Summary;
        let _ = MarkerKind::Truncate;
        let _ = MarkerKind::Semantic;
    }
}
