//! Fold block — parse and render fold markers in text.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldBlock {
    pub threshold: f64,
    pub content: String,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub struct FoldBlockRender {
    pub kept: Vec<String>,
    pub collapsed_count: usize,
}

/// Detect if text has any fold markers.
pub fn has_fold_markers(text: &str) -> bool {
    text.contains("[===vcp_fold:")
}

/// Parse fold markers in text. Returns blocks with content + threshold.
pub fn parse_fold_blocks(text: &str) -> Vec<FoldBlock> {
    let mut blocks = Vec::new();
    let re = regex::Regex::new(r"\[===vcp_fold:[^=]+=([0-9.]+)===\]([^\[]*)").unwrap();
    for caps in re.captures_iter(text) {
        let threshold: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
        let content = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        blocks.push(FoldBlock { threshold, content, collapsed: false });
    }
    blocks
}

/// Render fold blocks with given similarity threshold.
pub fn render_fold_blocks(text: &str, threshold: f64) -> FoldBlockRender {
    let blocks = parse_fold_blocks(text);
    let mut kept = Vec::new();
    let mut collapsed = 0;
    for block in blocks {
        if block.threshold >= threshold {
            kept.push(block.content);
        } else {
            collapsed += 1;
        }
    }
    FoldBlockRender { kept, collapsed_count: collapsed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_markers() {
        assert!(has_fold_markers("hello [===vcp_fold:Summary=0.5===] content"));
        assert!(!has_fold_markers("plain text"));
    }

    #[test]
    fn parse_blocks() {
        let text = "[===vcp_fold:Summary=0.5===]abc[===vcp_fold:Truncate=0.8===]def";
        let blocks = parse_fold_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].threshold, 0.5);
        assert_eq!(blocks[1].threshold, 0.8);
    }

    #[test]
    fn render_blocks() {
        let text = "[===vcp_fold:S=0.3===]keep[===vcp_fold:T=0.9===]drop";
        let r = render_fold_blocks(text, 0.5);
        assert_eq!(r.kept.len(), 1);
        assert_eq!(r.collapsed_count, 1);
    }

    #[test]
    fn no_markers_passthrough() {
        assert!(!has_fold_markers("plain text"));
        let r = render_fold_blocks("plain", 0.5);
        assert_eq!(r.kept.len(), 0);
        assert_eq!(r.collapsed_count, 0);
    }
}
