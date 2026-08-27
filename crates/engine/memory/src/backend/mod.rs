//! P-arch (2026-08-27): MemoryBackend trait 0 装接口 (O-6 重构批次 Refactor-1).
//!
//! **O-6 重构**: trait 抽象层搬到 `apeireth-plugin` (foundation), impl 留在本 crate (engine).
//! 单向依赖: memory → plugin. `MemoryResult` 仍在本 crate, 因为它是 domain-specific
//! 错误类型 (含 Io/AppendOnly/Identity 等 memory 专属); 0 触碰 24 个子模块 public API.
//!
//! 四个具体实现:
//! - [`SqliteBackend`] — v2.0.0-rc.1 纯 SQL 重写, 走 `SqliteConnectionPool` (RC-1 真实现,
//!   0 委托给 `SqliteMemoryStore`)
//! - [`FileBackend`] — JSON Lines 明文 append-only (alpha 0 装, opt-in 明文 fallback)
//! - [`EncryptedFileBackend`] — v2.0.0-rc.1 RC-10 AES-256-GCM 加密 File backend (opt-in 加密)
//! - [`InMemoryBackend`] — `HashMap`，仅测试用
//!
//! **不重写 SQL**: trait **不**委托给现有 `SqliteMemoryStore`; SqliteBackend 自起 3 个真
//! SQL INSERT/SELECT. (历史: 0 装占位时委托, RC-1 重写为直接 SQL.)
//!
//! **0 触碰承诺**（per `v2-unabsorbed-features.md` §5 P1）:
//! - 现有 `SqliteMemoryStore` 不改
//! - 现有 `EpisodeStore` / `NoteStore` / `HistoryStream` trait 不改
//! - 现有 24 个子模块的 public API 不改
//! - FileBackend 明文 0 改 (RC-10 EncryptedFileBackend 是新增 opt-in, 0 替换)

// Trait 抽象层在 plugin (P-arch 2026-08-27 O-6 重构); 这里是 re-export 保持 v1 兼容路径
pub use apeireth_plugin::memory_backend::{BackendKind, MemoryBackend};

// 4 backend impl (P-arch 2026-08-27 + RC-1 纯 SQL 重写 + RC-10 AES-GCM 加密)
// `pub mod` 声明让测试模块 + 外部 import 都能看到 4 个具体 backend
pub mod sqlite;
pub mod file;
pub mod file_encrypted;
pub mod in_memory;

use apeireth_core::kernel::memory::Episode;

use crate::append_only::HistoryEntry;
use crate::MemoryResult;
use crate::StreamKind;
