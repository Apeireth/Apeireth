//! `apeireth-context-fold` — v2 R144 context folding (FoldStrategy + FoldMarker + cross-session token accumulator).
//!
//! v1 API surface preserved: `FoldStrategy`, `FoldResult`, `FoldError`, `fold`, `unfold`,
//! `FoldMarker`, `MarkerKind`, `TokenAccumulator`, `AccumulatorSnapshot`,
//! `FoldBlock`, `FoldBlockRender`, `has_fold_markers`, `parse_fold_blocks`, `render_fold_blocks`,
//! `cosine`, `fold_segments`, `unfold_semantic`, `BigramOverlapScorer`, `Embedder`,
//! `EmbeddingScorer`, `FoldedSegment`, `RelevanceScorer`, `SemanticFoldOptions`,
//! `SemanticFoldOutcome`, `R144_DELIVERABLES`.

#![allow(missing_docs)]

pub mod accumulator;
pub mod fold;
pub mod fold_block;
pub mod marker;
pub mod semantic;

pub use accumulator::{AccumulatorSnapshot, TokenAccumulator};
pub use fold::{fold, unfold, FoldError, FoldResult, FoldStrategy};
pub use fold_block::{
    has_fold_markers, parse_fold_blocks, render_fold_blocks, FoldBlock, FoldBlockRender,
};
pub use marker::{FoldMarker, MarkerKind};
pub use semantic::{
    cosine, fold_segments, unfold_semantic, BigramOverlapScorer, Embedder, EmbeddingScorer,
    FoldedSegment, RelevanceScorer, SemanticFoldOptions, SemanticFoldOutcome,
};

/// R144 deliverables count.
pub const R144_DELIVERABLES: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deliverables_count() {
        assert_eq!(R144_DELIVERABLES, 3);
    }

    #[test]
    fn fold_truncate_basic() {
        let s = "a".repeat(100);
        let result = fold(&s, FoldStrategy::Truncate(50)).unwrap();
        assert!(result.folded_text.len() <= 50);
    }
}
