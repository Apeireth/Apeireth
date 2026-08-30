//! Semantic fold: keep high-relevance injection segments, collapse the rest.
//!
//! Complements budget truncation ([`crate::context_fold::fold`]): semantic fold
//! decides *who stays*, budget truncation decides *how much*. The rendered
//! product can be passed straight into `fold()` for a hard byte cap.
//!
//! Honest defaults:
//! - Scoring goes through [`RelevanceScorer`]. [`Embedder`] is injectable;
//!   tests use a mock. Built-in [`BigramOverlapScorer`] is deterministic and
//!   dependency-free.
//! - Summary has no internal LLM: default is a char truncation; callers may
//!   inject a summarizer callback.
//! - Placeholders are lossless: marker payload stores the original segment.

use super::marker::{FoldMarker, MarkerKind};

/// Relevance scorer: returns `[0.0, 1.0]`, `1.0` = fully relevant to the query.
pub trait RelevanceScorer {
    /// Score one injection segment against the query.
    fn score(&self, query: &str, segment: &str) -> f32;
}

/// Embedding source (mockable, no vector-store dependency).
pub trait Embedder {
    /// Encode text as a vector.
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// Cosine similarity, clamped to `[0.0, 1.0]`. Dimension mismatch / zero
/// vectors return `0.0`.
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

/// Embedding-backed scorer. The embedder is injected and may be mocked.
pub struct EmbeddingScorer<E: Embedder> {
    /// Injected embedding source.
    pub embedder: E,
}

impl<E: Embedder> EmbeddingScorer<E> {
    /// Construct a scorer around `embedder`.
    pub fn new(embedder: E) -> Self {
        Self { embedder }
    }
}

impl<E: Embedder> RelevanceScorer for EmbeddingScorer<E> {
    fn score(&self, query: &str, segment: &str) -> f32 {
        cosine(&self.embedder.embed(query), &self.embedder.embed(segment))
    }
}

/// Deterministic built-in scorer: character-bigram overlap
/// (`|A∩B| / sqrt(|A|·|B|)`). Texts shorter than 2 non-whitespace chars
/// (no bigrams) score `0.0`.
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
        let mut a_sorted = a;
        a_sorted.sort_unstable();
        a_sorted.dedup();
        let mut b_sorted = b;
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

/// Semantic-fold parameters.
#[derive(Debug, Clone)]
pub struct SemanticFoldOptions {
    /// Relevance threshold: `score < threshold` is folded; `score >= threshold`
    /// is kept.
    pub threshold: f32,
    /// Character cap for the default truncation summarizer.
    pub summary_chars: usize,
}

impl Default for SemanticFoldOptions {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            summary_chars: 80,
        }
    }
}

/// Record of one collapsed segment.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldedSegment {
    /// Index in the input segment list.
    pub index: usize,
    /// Relevance score that triggered the fold.
    pub score: f32,
    /// Summary (default truncation, or injected summarizer output).
    pub summary: String,
    /// Placeholder marker (payload = original segment).
    pub marker: FoldMarker,
    /// Full placeholder line rendered into the product (unique per index).
    pub placeholder_line: String,
}

/// Product of a semantic fold.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticFoldOutcome {
    /// Rendered product (blank-line separated).
    pub rendered: String,
    /// Number of kept (unfolded) segments.
    pub kept: usize,
    /// Collapsed-segment records (original text for lossless unfold).
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

/// Fold an injection-segment list: low-relevance segments become summary
/// placeholders; the rest stay verbatim.
///
/// - Empty / all-whitespace segments are dropped (they do not consume budget).
/// - `summarizer = None` → first `summary_chars` characters.
/// - Non-finite threshold is treated as `0.0` (fail-open: keep all).
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
            parts.push(folded.last().expect("just pushed").placeholder_line.clone());
        }
    }
    SemanticFoldOutcome {
        rendered: parts.join("\n\n"),
        kept,
        folded,
    }
}

/// Lossless unfold: replace each placeholder line with the original segment.
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

    /// Deterministic mock embedder: "天气"→[1,0], "代码"→[0,1], else mixed.
    struct MockEmbedder;
    impl Embedder for MockEmbedder {
        fn embed(&self, text: &str) -> Vec<f32> {
            if text.contains("天气") {
                vec![1.0, 0.0]
            } else if text.contains("代码") {
                vec![0.0, 1.0]
            } else {
                vec![0.1, 0.1]
            }
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
        assert_eq!(cosine(&[1.0], &[1.0, 0.0]), 0.0);
        assert_eq!(cosine(&[0.0], &[0.0]), 0.0);
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
        assert_eq!(a, s.score("天气预报", "天气预报"));
        assert_eq!(s.score("", "天气预报"), 0.0);
        assert_eq!(s.score("天", "天气预报"), 0.0);
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
        let s = EmbeddingScorer::new(MockEmbedder);
        let segs = ["北京天气晴", "一段代码实现"];
        let out = fold_segments(&segs, "今天天气如何", &s, &opts(), None);
        let budgeted = crate::context_fold::fold(
            &out.rendered,
            crate::context_fold::FoldStrategy::Truncate,
            10,
        )
        .unwrap();
        assert!(budgeted.folded.len() <= 10);
        let ht = crate::context_fold::fold(
            &out.rendered,
            crate::context_fold::FoldStrategy::HeadTail,
            12,
        )
        .unwrap();
        assert!(!ht.folded.is_empty());
    }

    #[test]
    fn utf8_summary_truncation_safe() {
        let long = "天气预报今天多云转晴明天有雨";
        let t = truncate_chars(long, 6);
        assert_eq!(t.chars().count(), 7);
        assert!(t.ends_with('…'));
        assert_eq!(truncate_chars("短", 6), "短");
        assert_eq!(truncate_chars("任意", 0), "");
    }
}
