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
//! **Honest** (per O-5 不假装):
//! - Token counting uses chars/4 approximation (no tiktoken dep)
//! - Summary strategy requires user-supplied callback (no internal LLM)
//! - Marker replace preserves original bytes (lossless unfold)

#![warn(missing_docs)]

pub mod fold;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod marker;
pub mod accumulator;

pub use fold::{FoldStrategy, FoldResult, fold, unfold};
pub use marker::{FoldMarker, MarkerKind};
pub use accumulator::{TokenAccumulator, AccumulatorSnapshot};

/// R144 deliverables (per v2 plan §9.5):
/// - 3 modules (fold / marker / accumulator)
pub const R144_DELIVERABLES: usize = 3;