//! FoldBlock graded reveal (foldProtocol hierarchical design, Rust-native).
//!
//! Documents are split on line markers `[===vcp_fold:threshold===]`
//! (optional `[===vcp_fold:threshold::desc:description===]`). Render expands a
//! block only when `similarity >= threshold`; hidden blocks collapse to a
//! "还收纳了 N 组" hint.
//!
//! Honest scope:
//! - A marker whose threshold fails to parse is treated as ordinary content.
//! - Empty documents yield an empty block list (callers decide the fallback).
//! - Non-finite similarity is treated as `0.0`.
//! - No regex dependency (line-level hand parse).

use serde::{Deserialize, Serialize};

/// `[===vcp_fold:threshold===]` line-marker prefix (compared after trim).
pub const FOLD_MARKER_PREFIX: &str = "[===";
/// Line-marker suffix.
pub const FOLD_MARKER_SUFFIX: &str = "===]";
/// Protocol field prefix inside the marker.
pub const FOLD_FIELD: &str = "vcp_fold:";
/// Description-field separator.
pub const FOLD_DESC_SEP: &str = "::desc:";

/// One fold block (same-level content sliced by a line marker).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldBlock {
    /// Expand when `similarity >= threshold` (equality included).
    pub threshold: f32,
    /// Optional block description (hints / debug).
    pub description: String,
    /// Block body (trimmed).
    pub content: String,
}

fn parse_marker_line(line: &str) -> Option<(f32, String)> {
    let t = line.trim();
    let rest = t
        .strip_prefix(FOLD_MARKER_PREFIX)?
        .strip_suffix(FOLD_MARKER_SUFFIX)?
        .trim();
    let body = rest.strip_prefix(FOLD_FIELD)?;
    let (th_part, desc) = match body.find(FOLD_DESC_SEP) {
        Some(i) => (
            &body[..i],
            body[i + FOLD_DESC_SEP.len()..].trim().to_string(),
        ),
        None => (body, String::new()),
    };
    let threshold: f32 = th_part.trim().parse().ok()?;
    Some((threshold, desc))
}

/// Whether the document contains any fold line markers.
pub fn has_fold_markers(content: &str) -> bool {
    content.lines().any(|l| parse_marker_line(l).is_some())
}

/// Split a document into [`FoldBlock`]s on line markers.
///
/// - Content before the first marker becomes a `threshold = 0.0` preamble
///   block (always-expand band).
/// - Empty document / no content → empty list.
pub fn parse_fold_blocks(content: &str) -> Vec<FoldBlock> {
    let mut blocks: Vec<FoldBlock> = Vec::new();
    let mut threshold = 0.0f32;
    let mut description = String::new();
    let mut buf: Vec<&str> = Vec::new();
    let mut opened = false;

    for line in content.lines() {
        if let Some((th, desc)) = parse_marker_line(line) {
            if opened || !buf.is_empty() {
                let c = buf.join("\n").trim().to_string();
                blocks.push(FoldBlock {
                    threshold,
                    description,
                    content: c,
                });
            }
            threshold = th;
            description = desc;
            buf.clear();
            opened = true;
        } else {
            buf.push(line);
        }
    }
    if opened || !buf.is_empty() {
        let c = buf.join("\n").trim().to_string();
        blocks.push(FoldBlock {
            threshold,
            description,
            content: c,
        });
    }
    blocks
}

/// Graded-reveal render result.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldBlockRender {
    /// Rendered product: expanded bodies (blank-line separated) + stash hint.
    pub rendered: String,
    /// Number of expanded blocks.
    pub expanded: usize,
    /// Number of hidden (stashed) blocks.
    pub hidden: usize,
    /// Stash hint line (empty when nothing is hidden).
    pub stash_hint: String,
}

/// Graded reveal: expand when `block.threshold <= similarity`.
///
/// Equality expands. Non-finite similarity is treated as `0.0`.
pub fn render_fold_blocks(blocks: &[FoldBlock], similarity: f32) -> FoldBlockRender {
    let sim = if similarity.is_finite() {
        similarity
    } else {
        0.0
    };
    let expanded_blocks: Vec<&FoldBlock> = blocks.iter().filter(|b| b.threshold <= sim).collect();
    let hidden = blocks.len() - expanded_blocks.len();
    let stash_hint = if hidden > 0 {
        format!("[已折叠] 还收纳了 {hidden} 组内容 (相似度未达阈值)")
    } else {
        String::new()
    };
    let mut rendered = expanded_blocks
        .iter()
        .map(|b| b.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if !stash_hint.is_empty() {
        if !rendered.is_empty() {
            rendered.push_str("\n\n");
        }
        rendered.push_str(&stash_hint);
    }
    FoldBlockRender {
        rendered,
        expanded: expanded_blocks.len(),
        hidden,
        stash_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "[===vcp_fold:0.0===]\n基础信息 A\n[===vcp_fold: 0.35 ::desc: 中级===]\n进阶内容 B\n[===vcp_fold:0.7===]\n深度内容 C";

    #[test]
    fn parse_three_blocks_with_desc() {
        let blocks = parse_fold_blocks(DOC);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].threshold, 0.0);
        assert_eq!(blocks[0].content, "基础信息 A");
        assert_eq!(blocks[1].threshold, 0.35);
        assert_eq!(blocks[1].description, "中级");
        assert_eq!(blocks[2].threshold, 0.7);
        assert!(blocks[2].description.is_empty());
    }

    #[test]
    fn preamble_before_first_marker_is_zero_block() {
        let blocks = parse_fold_blocks("前言内容\n[===vcp_fold:0.5===]\n正文");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].threshold, 0.0);
        assert_eq!(blocks[0].content, "前言内容");
        assert_eq!(blocks[1].content, "正文");
    }

    #[test]
    fn no_markers_single_block() {
        let blocks = parse_fold_blocks("纯文本文档");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].threshold, 0.0);
        assert_eq!(blocks[0].content, "纯文本文档");
        assert!(!has_fold_markers("纯文本文档"));
        assert!(has_fold_markers(DOC));
    }

    #[test]
    fn invalid_threshold_line_is_content() {
        let blocks = parse_fold_blocks("[===vcp_fold:abc===]\n内容");
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].content.contains("[===vcp_fold:abc===]"));
    }

    #[test]
    fn empty_content_yields_empty_blocks() {
        assert!(parse_fold_blocks("").is_empty());
        assert!(!has_fold_markers(""));
    }

    #[test]
    fn render_expands_by_threshold() {
        let blocks = parse_fold_blocks(DOC);
        let r = render_fold_blocks(&blocks, 0.5);
        assert_eq!(r.expanded, 2);
        assert_eq!(r.hidden, 1);
        assert!(r.rendered.contains("基础信息 A"));
        assert!(r.rendered.contains("进阶内容 B"));
        assert!(!r.rendered.contains("深度内容 C"));
        assert!(r.rendered.contains("还收纳了 1 组"));
    }

    #[test]
    fn threshold_boundary_equal_expands() {
        let blocks = parse_fold_blocks(DOC);
        let r = render_fold_blocks(&blocks, 0.7);
        assert_eq!(r.expanded, 3);
        assert_eq!(r.hidden, 0);
        assert!(r.stash_hint.is_empty());
        assert!(r.rendered.contains("深度内容 C"));
    }

    #[test]
    fn low_similarity_hides_all_but_zero() {
        let blocks = parse_fold_blocks(DOC);
        let r = render_fold_blocks(&blocks, 0.0);
        assert_eq!(r.expanded, 1);
        assert_eq!(r.hidden, 2);
        assert!(r.rendered.contains("还收纳了 2 组"));
    }

    #[test]
    fn empty_blocks_render_empty() {
        let r = render_fold_blocks(&[], 0.9);
        assert_eq!(r.rendered, "");
        assert_eq!(r.expanded, 0);
        assert_eq!(r.hidden, 0);
        assert!(r.stash_hint.is_empty());
    }

    #[test]
    fn non_finite_similarity_treated_as_zero() {
        let blocks = parse_fold_blocks(DOC);
        let r = render_fold_blocks(&blocks, f32::NAN);
        assert_eq!(r.expanded, 1);
        assert_eq!(r.hidden, 2);
    }

    #[test]
    fn fold_block_serde_roundtrip() {
        let b = FoldBlock {
            threshold: 0.5,
            description: "d".into(),
            content: "c".into(),
        };
        let s = serde_json::to_string(&b).unwrap();
        let b2: FoldBlock = serde_json::from_str(&s).unwrap();
        assert_eq!(b, b2);
    }
}
