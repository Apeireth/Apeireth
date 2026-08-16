//! `apeireth-context-fold` - R144 context folding.
//!
//! Per `reports/vcp-plugin-gap-analysis-2026-08-12.md` §9.5 / §9.6 Decision:
//! borrows VCP `ContextFoldingV2` 激活占位符 design but Rust-native re-impl.
//!
//! 3 modules:
//! 1. **fold** — FoldStrategy (Truncate / HeadTail / Summary / MarkerReplace) + fold/unfold
//! 2. **marker** — FoldMarker (可展开 placeholder format)
//! 3. **accumulator** — 跨 session token 累计 (近似 tiktoken 字符 / 4)
//!
//! 记忆域深化 §5.1 / backlog N11 增强 (2 modules):
//! 4. **semantic** — 语义折叠 (注入段按相关度评分, 低相关段折叠为摘要占位;
//!    VCP ContextFoldingV2 精神; 嵌入可 mock + 确定性内置评分器)
//! 5. **fold_block** — FoldBlock 分级显隐 (`[===vcp_fold:阈值===]` 行标记,
//!    相似度≥阈值才展开, 未展开留"还收纳了 N 组"提示; VCP foldProtocol 精神)
//!
//! **Honest** (per O-5 不假装):
//! - Token counting uses chars/4 approximation (no tiktoken dep)
//! - Summary strategy requires user-supplied callback (no internal LLM)
//! - Marker replace preserves original bytes (lossless unfold)
//! - Semantic fold summary defaults to char truncation (summarizer injectable, no internal LLM)

#![warn(missing_docs)]

pub mod fold;
// R177: organ invariants (5 tests + 2 Kani)
pub mod accumulator;
pub mod fold_block;
pub mod marker;
mod organ_kani_proofs;
pub mod semantic;

pub use accumulator::{AccumulatorSnapshot, TokenAccumulator};
pub use fold::{fold, unfold, FoldResult, FoldStrategy};
pub use fold_block::{
    has_fold_markers, parse_fold_blocks, render_fold_blocks, FoldBlock, FoldBlockRender,
};
pub use marker::{FoldMarker, MarkerKind};
pub use semantic::{
    cosine, fold_segments, unfold_semantic, BigramOverlapScorer, Embedder, EmbeddingScorer,
    FoldedSegment, RelevanceScorer, SemanticFoldOptions, SemanticFoldOutcome,
};

/// R144 deliverables (per v2 plan §9.5):
/// - 3 modules (fold / marker / accumulator)
pub const R144_DELIVERABLES: usize = 3;
