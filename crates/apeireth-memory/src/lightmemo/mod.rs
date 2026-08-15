//! `apeireth-memory-lightmemo` - R142 LightMemo memory system.
//!
//! Rust port of AgentMemory v2.1.0 (4-layer closed-loop memory):
//! - L1: file persistence (rusqlite + blob storage)
//! - L2: vector store (in-memory cosine similarity)
//! - L3: tag store (inverted index)
//! - L4: LCM compressor (chunking + summarization hooks)
//!
//! Plus subsystems:
//! - MemoryManager (cross-layer CRUD)
//! - DecayEngine (Ebbinghaus-style forgetting curve)
//! - DreamSubsystem (offline consolidation)
//! - MultiPipeSearch (BM25 + vector + tag fusion)
//!
//! Per `reports/vcp-plugin-gap-analysis-2026-08-12.md` §9.4 Decision 5,
//! borrows AgentMemory v2.1.0 design (own prior open-source work, MIT licensed).
//! Rust-native re-implementation; not FFI Python.
//!
//! **Honest scope** (per O-5 不假装):
//! - 4 layers are real (sqlite + cosine + inverted index)
//! - LCM compressor is hook-only (calls user-supplied callback; no internal LLM)
//! - Decay engine is real (configurable half-life)
//! - Dream subsystem is hook-only (user provides consolidation callback)
//! - Multi-pipe search fuses 3 sources (BM25-lite + cosine + tag)
//! - 28-module port is 真实覆盖 ~12 核心 modules + stubs for余下 (per v2 plan §9.5)

#![warn(missing_docs)]

pub mod l1_file;
pub mod l2_vector;
pub mod l3_tag;
pub mod l4_lcm;
pub mod progression; // R179 P1-11: 4-layer progressive
pub mod manager;
pub mod decay;
pub mod dream;
pub mod search;
pub mod pipe;
pub mod sleep_cycle;
pub mod librarian;
pub mod adapter;
pub mod mcp;
pub mod compat;
pub mod enhanced;

pub use l1_file::{L1FileStore, FileEntry as L1Entry};
pub use l2_vector::{L2VectorStore, VectorEntry};
pub use l3_tag::TagIndex;
pub use l4_lcm::{L4LcmCompressor, LcmChunk, LcmCallback};
// R179 P1-11: 4 层渐进
pub use progression::{Layer, LayerProgression};
pub use manager::{MemoryManager, MemoryItem, MemoryError};
pub use decay::{DecayEngine, DecayConfig};
pub use dream::{DreamSubsystem, DreamCallback};
pub use search::{SearchPipeline, SearchHit, SearchMode};
pub use pipe::{SearchPipe, FusionStrategy};
pub use sleep_cycle::{SleepCycle, SleepConfig};
pub use librarian::{Librarian, Category};
pub use adapter::{AdapterRegistry, MemoryAdapter, MemorySource, SourceKind, ConversationAdapter, FileAdapter};
pub use mcp::{LightMemoMcp, LightMemoTool};
pub use compat::{LightMemoCommand, LightMemoCompatRouter, LIGHTMEMO_COMMAND_COUNT};
pub use enhanced::EnhancedLightMemo;

/// R142 deliverables (per v2 plan §9.5):
/// - 11 modules (L1/L2/L3/L4 + manager + decay + dream + search/pipe + mcp + compat + enhanced)
/// - 4-layer closed-loop architecture
/// - Multi-pipe search (BM25 + vector + tag fusion)
pub const R142_DELIVERABLES: usize = 12;

/// Module count per v2 plan §9.5 (target: 28 modules for full port).
/// Honest current: 15 modules after R143 expansion (R142 was 12).
pub const R143_MODULE_COUNT: usize = 15;