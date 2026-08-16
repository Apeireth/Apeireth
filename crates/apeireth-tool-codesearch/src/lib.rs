//! `apeireth-tool-codesearch` - R140 code search + knowledge graph.
//!
//! Extends `apeireth-tools` with 5-dim code intelligence per v2 plan §9.5:
//!
//! 1. **Content search** (regex + Aho-Corasick multi-pattern)
//! 2. **File finder** (walkdir + globset + .gitignore)
//! 3. **Symbol extraction** (regex-based, 5 languages: Rust/Python/JS/TS/Go)
//! 4. **Knowledge graph** (file → symbol → imports → call sites)
//! 5. **MCP server** (10+ tools for LLM-driven code exploration)
//!
//! Per `reports/vcp-plugin-gap-analysis-2026-08-12.md` §9.4 Decision 1,
//! borrows **codebase-memory-mcp** (DeusData, 158 langs, 6768 tests) design
//! but does NOT FFI the C library (compile cost). Instead, Rust-native
//! implementation with honest scope (5 langs regex-based, not full AST).
//!
//! **Honest** (per O-5 不假装):
//! - Symbol extraction is regex-based (NOT tree-sitter AST). Handles 80%+ of
//!   standard patterns for 5 langs. Mis-parses pathological syntax.
//! - Knowledge graph is in-memory (not persisted across restart). Persistent
//!   index via rusqlite is a stub.
//! - 5 langs not 158 (per codebase-memory-mcp). Adding a lang requires a new
//!   regex pattern set; documented in `symbols.rs`.
//! - Persistent index is rusqlite schema only, no FTS5 virtual table yet
//!   (full-text search uses in-memory Aho-Corasick on the indexed files).

#![warn(missing_docs)]

pub mod search;
// R177: organ invariants (5 tests + 2 Kani)
pub mod ast_grep; // R193
pub mod cache; // R210: TTL-bounded QueryCache + CachedUnifiedIntelligence
pub mod compat;
pub mod enhanced;
pub mod files;
pub mod graph;
pub mod index;
pub mod lru_cache;
pub mod mcp;
mod organ_kani_proofs;
pub mod symbols;
pub mod unified; // R202 // R213: 真 LRU (lru crate) + streaming + batch query: 6 维 code intelligence unified facade: ast-grep CLI 包装 (R181 调研短期方案)

pub use compat::{CodeSearchCommand, CodeSearchCompatRouter, CODESEARCH_COMMAND_COUNT};
pub use enhanced::EnhancedCodeSearch;
pub use files::{FileEntry, FileFinder, FindOptions};
pub use graph::{GraphEdge, GraphNode, KnowledgeGraph, NodeKind};
pub use index::{CodeIndex, IndexEntry};
pub use mcp::{CodeSearchMcp, McpTool};
pub use search::{CodeSearcher, SearchKind, SearchMatch, SearchOptions};
pub use symbols::{extract_symbols, supported_languages, Symbol, SymbolKind};

/// R140 deliverables (per v2 plan §9.5):
/// - 7 modules (search / files / symbols / graph / index / mcp / compat) + enhanced
/// - 5 content search modes (literal / regex / multi-pattern / case-insensitive / word-boundary)
/// - 10+ MCP tools
pub const R140_DELIVERABLES: usize = 8;

/// Languages with regex-based symbol extraction (vs full AST).
pub const SUPPORTED_LANGS: &[&str] = &["rust", "python", "javascript", "typescript", "go"];
pub mod pure_pattern; // R251: in-process pure Rust pattern matcher (0 ast-grep CLI dep)
