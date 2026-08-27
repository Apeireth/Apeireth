//! 记忆后端抽象 trait (P-arch, 2026-08-27).
//!
//! 三个具体实现:
//! - [`SqliteBackend`] — 包装现有 `SqliteMemoryStore`（v1 compat facade，委托）
//! - [`FileBackend`] — JSON Lines 明文 append-only（keyring 加密后续）
//! - [`InMemoryBackend`] — `HashMap`，仅测试用
//!
//! **不重写 SQL**：trait 委托给现有 `SqliteMemoryStore` 的成熟实现；
//! 只是给"加新后端"一个清晰的 trait 边界。
//!
//! **0 触碰承诺**（per `v2-unabsorbed-features.md` §5 P1）:
//! - 现有 `SqliteMemoryStore` 不改
//! - 现有 `EpisodeStore` / `NoteStore` / `HistoryStream` trait 不改
//! - 现有 24 个子模块的 public API 不改
//! - 0 装 PASS：File backend v2.0 是明文 JSON，encryption 留 v2.1

use apeireth_core::Episode;

use crate::append_only::HistoryEntry;
use crate::MemoryResult;
use crate::StreamKind;

/// 后端类型标识。
///
/// 0 假装 PASS：所有 impl 都真实实现，**不**"假装有加密"等尚未落地能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// SQLite（WAL + 现有 migrations）—— 默认后端，向后兼容
    Sqlite,
    /// JSON Lines append-only 文件（明文，encryption 待 v2.1）
    File,
    /// 进程内 HashMap（仅测试，进程重启数据丢失）
    InMemory,
}

impl BackendKind {
    /// 稳定标签字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::File => "file",
            Self::InMemory => "in_memory",
        }
    }
}

/// 记忆后端抽象。
///
/// **核心契约**：
/// - **append-only 写入**：所有 put_* / append_* 操作不可变
///   （违反 append-only 语义是后端实现的 bug，不是 trait 的允许行为）
/// - **同步 API**：当前 v2 runtime 在 spawn_blocking 里调 memory，sync trait 足够
///   （async trait 留给未来 P-arch 评估）
/// - **Send + Sync**：后端实例可跨线程共享
/// - **错误统一**：所有错误走 [`MemoryError`](crate::MemoryError)
pub trait MemoryBackend: Send + Sync {
    /// 后端稳定标签 (e.g. `"sqlite"`, `"file"`, `"in_memory"`)
    fn name(&self) -> &'static str;

    /// 后端类型枚举
    fn kind(&self) -> BackendKind;

    /// 健康检查（启动时调，确认后端可用）
    fn ping(&self) -> MemoryResult<()> {
        Ok(())
    }

    // ===== Episode 操作 =====

    /// 写入一条 Episode (append-only)。
    fn put_episode(&self, ep: &Episode) -> MemoryResult<()>;

    /// 按 id 读取一条 Episode。
    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>>;

    /// 检索某 session 的最近 N 条 Episode（按时间升序，末尾 N 条）。
    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>>;

    // ===== 6 历史流操作 =====

    /// 追加一条历史流条目 (append-only)。
    fn append_stream(&self, kind: StreamKind, entry: HistoryEntry) -> MemoryResult<()>;

    /// 列出某 session 的某流最近 N 条（按时间升序，末尾 N 条，未 tombstone 的）。
    fn list_stream(
        &self,
        kind: StreamKind,
        session_id: &str,
        n: usize,
    ) -> MemoryResult<Vec<HistoryEntry>>;
}

// ===== 三个实现 =====

mod file;
mod in_memory;
mod sqlite;

pub use file::FileBackend;
pub use in_memory::InMemoryBackend;
pub use sqlite::SqliteBackend;
