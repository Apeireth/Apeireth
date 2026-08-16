//! FoldBlock 分级显隐 (backlog N11, VCP `foldProtocol` 精神 Rust 原生移植).
//!
//! 同文档分级显隐: 文档用行标记 `[===vcp_fold:阈值===]`
//! (可选 `[===vcp_fold:阈值::desc:区块描述===]`) 切成多个块,
//! 渲染时按当前上下文相似度分级展开——**相似度 ≥ 阈值才展开**,
//! 未展开的块整体隐藏, 只留「还收纳了 N 组」提示。
//!
//! 参考 (只读): `research/source/vcptoolbox/modules/foldProtocol.js`。
//!
//! **0 装 PASS (诚实标注)**:
//! - 阈值解析失败 (如 `[===vcp_fold:abc===]`) 的行按普通内容处理
//!   (VCP 用正则 `[0-9.]+` 先行过滤, 行为等价: 不匹配即内容);
//! - 空文档 → 空块列表 (VCP 会塞兜底文案; Rust 侧由调用方决定兜底, 不假装);
//! - 相似度非有限值 (NaN/inf) → 按 0.0 处理;
//! - 无 regex 依赖 (行级手工解析, 语义与 VCP FOLD_REGEX 对齐)。

use serde::{Deserialize, Serialize};

/// `[===vcp_fold:阈值===]` 行标记前缀 (trim 后比较)。
pub const FOLD_MARKER_PREFIX: &str = "[===";
/// 行标记后缀。
pub const FOLD_MARKER_SUFFIX: &str = "===]";
/// 块协议字段前缀。
pub const FOLD_FIELD: &str = "vcp_fold:";
/// 描述字段分隔。
pub const FOLD_DESC_SEP: &str = "::desc:";

/// 单个折叠块 (行标记切分出的同级内容)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldBlock {
    /// 展开阈值: 相似度 ≥ threshold 才展开 (边界含等号)。
    pub threshold: f32,
    /// 区块描述 (可选, 供提示/调试)。
    pub description: String,
    /// 块正文 (已 trim)。
    pub content: String,
}

/// 解析单行标记 → (阈值, 描述); 非标记行返回 None。
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

/// 文档是否含折叠行标记。
pub fn has_fold_markers(content: &str) -> bool {
    content.lines().any(|l| parse_marker_line(l).is_some())
}

/// 把文档按行标记切成 FoldBlock 列表。
///
/// - 首个标记之前的内容归入 threshold=0.0 的前置块 (永远展开档);
/// - 空文档/无内容 → 空列表 (调用方自行兜底)。
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

/// 分级显隐渲染结果。
#[derive(Debug, Clone, PartialEq)]
pub struct FoldBlockRender {
    /// 渲染产物: 展开块正文 (空行分隔) + 收纳提示行。
    pub rendered: String,
    /// 展开块数。
    pub expanded: usize,
    /// 被收纳 (隐藏) 块数。
    pub hidden: usize,
    /// 收纳提示行 (无隐藏时为空串)。
    pub stash_hint: String,
}

/// 按相似度分级显隐渲染: `block.threshold <= similarity` 展开, 其余收纳。
///
/// 边界语义: 相似度恰好等于阈值 → 展开 (≥ 含等号)。
/// 非有限相似度 (NaN/inf) 按 0.0 处理。
pub fn render_fold_blocks(blocks: &[FoldBlock], similarity: f32) -> FoldBlockRender {
    let sim = if similarity.is_finite() {
        similarity
    } else {
        0.0
    };
    let expanded_blocks: Vec<&FoldBlock> = blocks.iter().filter(|b| b.threshold <= sim).collect();
    let hidden = blocks.len() - expanded_blocks.len();
    let stash_hint = if hidden > 0 {
        format!("[已折叠] 还收纳了 {} 组内容 (相似度未达阈值)", hidden)
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
        // 0 装: 阈值解析失败的行不视为标记
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
        // 相似度 == 阈值 → 展开 (≥ 含等号)
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
        assert_eq!(r.expanded, 1); // threshold=0.0 块恒展开
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
