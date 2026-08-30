//! `apeireth-orchestration::context_fold` — lossless context folding primitives.
//!
//! Recovered from donor `apeireth-context-fold` (VCP `ContextFoldingV2` / `foldProtocol`
//! spirit, Rust-native). This is a **library** of compression heuristics. It is
//! complementary to [`crate::context_rot`] (rot_score + Retain/Remove/Replace) and
//! to tools spill (overflow-to-disk). Folding decides *how to collapse a string*;
//! rot decides *which segments to keep*; spill decides *where oversized tool
//! output lives*.
//!
//! Modules:
//! - [`fold`] — [`FoldStrategy`] Truncate / HeadTail / MarkerReplace / Summary
//! - [`marker`] — placeholder format (`<<FOLDED:N bytes>>` etc.)
//! - [`semantic`] — relevance-preserving collapse (BigramOverlap / embedder hook)
//! - [`fold_block`] — `[===vcp_fold:threshold===]` graded reveal
//! - [`accumulator`] — honest chars/4 cross-session token tally
//!
//! **Honest scope (DEFAULT OFF, not production-wired):**
//! - Token counting is `chars/4` (no tiktoken).
//! - Summary strategy is truncate unless a caller supplies a summarizer.
//! - Marker replace / semantic fold store original bytes in marker payload
//!   (lossless unfold). This crate never owns a session, loop, or LLM client.
//!
//! Coordinator may later wire these into context assembly. Do not treat
//! `pub use` as production enablement.

pub mod accumulator;
pub mod fold;
pub mod fold_block;
pub mod marker;
pub mod semantic;

pub use accumulator::{approx_tokens, AccumulatorSnapshot, TokenAccumulator};
pub use fold::{fold, unfold, FoldError, FoldResult, FoldStrategy};
pub use fold_block::{
    has_fold_markers, parse_fold_blocks, render_fold_blocks, FoldBlock, FoldBlockRender,
    FOLD_DESC_SEP, FOLD_FIELD, FOLD_MARKER_PREFIX, FOLD_MARKER_SUFFIX,
};
pub use marker::{FoldMarker, MarkerKind};
pub use semantic::{
    cosine, fold_segments, unfold_semantic, BigramOverlapScorer, Embedder, EmbeddingScorer,
    FoldedSegment, RelevanceScorer, SemanticFoldOptions, SemanticFoldOutcome,
};
