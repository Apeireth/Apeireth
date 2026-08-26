//! 语义折叠 (记忆域深化 §5.1, VCP `ContextFoldingV2` 精神).
//!
//! 注入段按相关度评分: **只折叠低相关段**为摘要占位, 高相关段原样保留。
//! 与预算截断 (`fold.rs`) 互补不冲突: 语义折叠决定"留谁", 预算截断决定"留多少",
//! 语义折叠产物可直接再走 `fold()` 硬预算上限。
//!
//! **0 装 PASS (诚实标注)**:
//! - 嵌入可 mock: 评分走 `RelevanceScorer` trait; `Embedder` trait 注真嵌入,
//!   `EmbeddingScorer` 余弦相似度; 测试用 `MockEmbedder` 确定性验证;
//! - 内置 `BigramOverlapScorer`: 字符二元组重叠, 0 依赖/确定性/可测;
//! - 摘要无内置 LLM: 默认截取前 `summary_chars` 字符 (用户可注入 summarizer);
//! - 占位无损: marker payload 存原段全文, `unfold_semantic` 可复原。

use crate::marker::{FoldMarker, MarkerKind};

/// 相关度评分器: 返回 [0.0, 1.0], 1.0 = 与查询完全相关。
pub trait RelevanceScorer {
    /// 对单个注入段相对查询打相关度分。
    fn score(&self, query: &str, segment: &str) -> f32;
}

/// 嵌入源 (可 mock, 0 真依赖)。真实现可接向量检索层。
pub trait Embedder {
    /// 把文本编码为向量。
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// 余弦相似度, 结果钳制到 [0.0, 1.0]。维度不匹配/零向量 → 0.0。
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    (dot / (na * nb)).clamp(0.0, 1.0)
}

/// 基于嵌入的评分器 (嵌入经 `Embedder` 注入, 可 mock)。
pub struct EmbeddingScorer<E: Embedder> {
    /// 注入的嵌入源。
    pub embedder: E,
}

impl<E: Embedder> EmbeddingScorer<E> {
    /// 用给定嵌入源构造评分器。
    pub fn new(embedder: E) -> Self {
        Self { embedder }
    }
}

impl<E: Embedder> RelevanceScorer for EmbeddingScorer<E> {
    fn score(&self, query: &str, segment: &str) -> f32 {
        cosine(&self.embedder.embed(query), &self.embedder.embed(segment))
    }
}

/// 确定性内置评分器: 字符二元组重叠 (|A∩B| / sqrt(|A|·|B|)), 0 依赖。
/// 文本长度 < 2 字符 (无二元组) → 0.0。
pub struct BigramOverlapScorer;

impl BigramOverlapScorer {
    fn bigrams(text: &str) -> Vec<(char, char)> {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        chars.windows(2).map(|w| (w[0], w[1])).collect()
    }
}

impl RelevanceScorer for BigramOverlapScorer {
    fn score(&self, query: &str, segment: &str) -> f32 {
        let a = Self::bigrams(query);
        let b = Self::bigrams(segment);
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut a_sorted = a.clone();
        a_sorted.sort_unstable();
        a_sorted.dedup();
        let mut b_sorted = b.clone();
        b_sorted.sort_unstable();
        b_sorted.dedup();
        let inter = a_sorted.iter().filter(|g| b_sorted.contains(g)).count() as f32;
        let denom = (a_sorted.len() as f32 * b_sorted.len() as f32).sqrt();
        if denom == 0.0 {
            return 0.0;
        }
        (inter / denom).clamp(0.0, 1.0)
    }
}

/// 语义折叠参数。
#[derive(Debug, Clone)]
pub struct SemanticFoldOptions {
    /// 相关度阈值: score < threshold 的段被折叠; score ≥ threshold 保留。
    pub threshold: f32,
    /// 折叠占位中摘要保留的字符数上限。
    pub summary_chars: usize,
}

/// 单个被折叠段的收纳记录。
#[derive(Debug, Clone)]
pub struct FoldedSegment {
    /// 在输入段列表中的下标。
    pub index: usize,
    /// 相关度评分。
    pub score: f32,
    /// 摘要 (默认截取, 或注入 summarizer 产物)。
    pub summary: String,
    /// 占位 marker (payload = 原段全文, 无损展开用)。
    pub marker: FoldMarker,
    /// 渲染进产物的完整占位行 (含下标, 保证唯一)。
    pub placeholder_line: String,
}

/// 语义折叠产物。
#[derive(Debug, Clone)]
pub struct SemanticFoldOutcome {
    /// 渲染产物 (段间空行分隔)。
    pub rendered: String,
    /// 保留段数。
    pub kept: usize,
    /// 折叠段收纳记录 (含原文, 供无损展开)。
    pub folded: Vec<FoldedSegment>,
}

fn truncate_chars(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// 对注入段列表做语义折叠: 低相关段折叠为摘要占位, 其余原样保留。
///
/// - 空段 (""/全空白) 直接丢弃, 不占预算;
/// - `summarizer` 为 None 时摘要 = 前 `summary_chars` 字符截取;
/// - 非有限阈值按 0.0 处理 (fail-open 全保留, 对齐 VCP 嵌入不可用降级精神)。
pub fn fold_segments<S: RelevanceScorer>(
    segments: &[&str],
    query: &str,
    scorer: &S,
    opts: &SemanticFoldOptions,
    summarizer: Option<&dyn Fn(&str) -> String>,
) -> SemanticFoldOutcome {
    let threshold = if opts.threshold.is_finite() {
        opts.threshold
    } else {
        0.0
    };
    let mut parts: Vec<String> = Vec::new();
    let mut folded: Vec<FoldedSegment> = Vec::new();
    let mut kept = 0usize;

    for (i, seg) in segments.iter().enumerate() {
        let text = seg.trim();
        if text.is_empty() {
            continue;
        }
        let score = scorer.score(query, seg).clamp(0.0, 1.0);
        if score >= threshold {
            kept += 1;
            parts.push(text.to_string());
        } else {
            let summary = match summarizer {
                Some(f) => f(text),
                None => truncate_chars(text, opts.summary_chars),
            };
            let marker = FoldMarker::new(MarkerKind::Semantic, text.to_string());
            let placeholder_line = format!(
                "[折叠#{} score={:.2}] {} {}",
                i,
                score,
                summary,
                marker.format_placeholder()
            );
            folded.push(FoldedSegment {
                index: i,
                score,
                summary,
                marker,
                placeholder_line,
            });
            parts.push(folded.last().unwrap().placeholder_line.clone());
        }
    }
    SemanticFoldOutcome {
        rendered: parts.join("\n\n"),
        kept,
        folded,
    }
}

/// 无损展开: 把占位行逐一还原为原段全文。
pub fn unfold_semantic(rendered: &str, outcome: &SemanticFoldOutcome) -> String {
    let mut out = String::from(rendered);
    for f in &outcome.folded {
        out = out.replace(&f.placeholder_line, &f.marker.payload);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性 mock 嵌入: "天气"→[1,0], "代码"→`[0,1]`, 其余混合。
    struct MockEmbedder;
    impl Embedder for MockEmbedder {
        fn embed(&self, text: &str) -> Vec<f32> {
            let mut v = vec![0.1f32, 0.1];
            if text.contains("天气") {
                v = vec![1.0, 0.0];
            } else if text.contains("代码") {
                v = vec![0.0, 1.0];
            }
            v
        }
    }

    fn opts() -> SemanticFoldOptions {
        SemanticFoldOptions {
            threshold: 0.5,
            summary_chars: 6,
        }
    }

    #[test]
    fn cosine_basic_and_edges() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert_eq!(cosine(&[], &[]), 0.0);
        assert_eq!(cosine(&[1.0], &[1.0, 0.0]), 0.0); // 维度不匹配
        assert_eq!(cosine(&[0.0], &[0.0]), 0.0); // 零向量
    }

    #[test]
    fn embedding_scorer_with_mock() {
        let s = EmbeddingScorer::new(MockEmbedder);
        assert!((s.score("今天天气如何", "北京天气晴") - 1.0).abs() < 1e-6);
        assert_eq!(s.score("今天天气如何", "一段代码"), 0.0);
    }

    #[test]
    fn bigram_scorer_deterministic_and_ordered() {
        let s = BigramOverlapScorer;
        let a = s.score("天气预报", "天气预报");
        let b = s.score("天气预报", "毫无关系xyz");
        assert!((a - 1.0).abs() < 1e-6);
        assert!(b < 0.3);
        // 确定性: 同输入多次同输出
        assert_eq!(a, s.score("天气预报", "天气预报"));
        assert_eq!(s.score("", "天气预报"), 0.0);
        assert_eq!(s.score("天", "天气预报"), 0.0); // 无二元组
    }

    #[test]
    fn folds_low_relevance_segments() {
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["北京天气晴", "一段代码实现"];
        let out = fold_segments(&segs, "今天天气如何", &s, &opts(), None);
        assert_eq!(out.kept, 1);
        assert_eq!(out.folded.len(), 1);
        assert!(out.rendered.contains("北京天气晴"));
        assert!(out.rendered.contains("[折叠#1"));
        assert!(out.rendered.contains("SEMANTIC"));
        // 摘要截取
        assert_eq!(out.folded[0].summary, "一段代码实现");
    }

    #[test]
    fn no_fold_when_all_relevant() {
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["北京天气晴", "上海天气雨"];
        let out = fold_segments(&segs, "今天天气如何", &s, &opts(), None);
        assert_eq!(out.kept, 2);
        assert!(out.folded.is_empty());
        assert_eq!(out.rendered, "北京天气晴\n\n上海天气雨");
    }

    #[test]
    fn threshold_boundary_equal_kept() {
        // score == threshold → 保留 (≥ 含等号)
        struct Half;
        impl RelevanceScorer for Half {
            fn score(&self, _q: &str, _s: &str) -> f32 {
                0.5
            }
        }
        let segs = ["段"];
        let out = fold_segments(&segs, "q", &Half, &opts(), None);
        assert_eq!(out.kept, 1);
        assert!(out.folded.is_empty());
    }

    #[test]
    fn empty_segments_dropped() {
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs: Vec<&str> = vec!["", "   ", "北京天气晴"];
        let out = fold_segments(&segs, "天气", &s, &opts(), None);
        assert_eq!(out.kept, 1);
        assert!(out.folded.is_empty());
        assert_eq!(out.rendered, "北京天气晴");
        // 全空列表
        let none: Vec<&str> = vec![];
        let out2 = fold_segments(&none, "天气", &s, &opts(), None);
        assert_eq!(out2.rendered, "");
        assert_eq!(out2.kept, 0);
    }

    #[test]
    fn unfold_restores_original_losslessly() {
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["北京天气晴", "一段代码实现"];
        let out = fold_segments(&segs, "今天天气如何", &s, &opts(), None);
        let restored = unfold_semantic(&out.rendered, &out);
        assert_eq!(restored, "北京天气晴\n\n一段代码实现");
    }

    #[test]
    fn summarizer_callback_used() {
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["一段代码实现"];
        let sum = |_: &str| String::from("AI摘要");
        let out = fold_segments(&segs, "天气", &s, &opts(), Some(&sum));
        assert!(out.rendered.contains("AI摘要"));
    }

    #[test]
    fn non_finite_threshold_fail_open_keeps_all() {
        // NaN 阈值 → 按 0.0 处理 → 全部保留 (fail-open)
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["北京天气晴"];
        let o = SemanticFoldOptions {
            threshold: f32::NAN,
            summary_chars: 4,
        };
        let out = fold_segments(&segs, "天气", &s, &o, None);
        assert_eq!(out.kept, 1);
        assert!(out.folded.is_empty());
    }

    #[test]
    fn composes_with_budget_truncation() {
        // 协作不冲突: 语义折叠产物再走 fold() 硬预算上限
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["北京天气晴", "一段代码实现"];
        let out = fold_segments(&segs, "今天天气如何", &s, &opts(), None);
        let budgeted = crate::fold(&out.rendered, crate::FoldStrategy::Truncate, 10).unwrap();
        assert!(budgeted.folded.chars().count() <= 10);
        // HeadTail 预算截断同样兼容
        let ht = crate::fold(&out.rendered, crate::FoldStrategy::HeadTail, 12).unwrap();
        assert!(!ht.folded.is_empty());
    }

    #[test]
    fn utf8_summary_truncation_safe() {
        let long = "天气预报今天多云转晴明天有雨";
        let t = truncate_chars(long, 6);
        assert_eq!(t.chars().count(), 7); // 6 字 + 省略号
        assert!(t.ends_with('…'));
        assert_eq!(truncate_chars("短", 6), "短");
        assert_eq!(truncate_chars("任意", 0), "");
    }
}
