//! P-arch (2026-08-27): MemoryBackend trait 0 装接口 (O-6 重构批次 Refactor-1).
//!
//! **O-6 重构**: trait 抽象层搬到 `apeireth-plugin` (foundation), impl 留在本 crate (engine).
//! 单向依赖: memory → plugin. `MemoryResult` 仍在本 crate, 因为它是 domain-specific
//! 错误类型 (含 Io/AppendOnly/Identity 等 memory 专属); 0 触碰 24 个子模块 public API.
//!
//! 三个具体实现:
//! - [`SqliteBackend`] — 包装现有 `SqliteMemoryStore`（v1 compat facade，委托）
//! - [`FileBackend`] — JSON Lines 明文 append-only（keyring 加密后续）
//! - [`InMemoryBackend`] — `HashMap`，仅测试用
//!
//! **不重写 SQL**: trait 委托给现有 `SqliteMemoryStore` 的成熟实现；
//! 只是给"加新后端"一个清晰的 trait 边界。
//!
//! **0 触碰承诺**（per `v2-unabsorbed-features.md` §5 P1）:
//! - 现有 `SqliteMemoryStore` 不改
//! - 现有 `EpisodeStore` / `NoteStore` / `HistoryStream` trait 不改
//! - 现有 24 个子模块的 public API 不改
//! - 0 装 PASS：File backend v2.0 是明文 JSON，encryption 留 v2.1

// Trait 抽象层在 plugin (P-arch 2026-08-27 O-6 重构); 这里是 re-export 保持 v1 兼容路径
pub use apeireth_plugin::memory_backend::{BackendKind, MemoryBackend};

use apeireth_core::kernel::memory::Episode;

use crate::append_only::HistoryEntry;
use crate::MemoryResult;
use crate::StreamKind;
