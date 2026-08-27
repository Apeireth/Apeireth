//! P-arch (2026-08-27): MemoryBackend trait (O-6 重构批次 Refactor-1).
//!
//! **位置**: trait 抽象层在 `apeireth-plugin` (foundation), impl 留在 `apeireth-memory` (engine).
//! 与 `CredentialResolver` 同位: 都是 capability 抽象, plugin 管 trait 边界,
//! 业务方管 impl. 单向依赖 (memory → plugin, 不反向).
//!
//! **不重写 SQL**: impl 仍委托现有 `SqliteMemoryStore`, 0 触碰 24 个子模块的 public API.
//!
//! **0 装 PASS**: trait 是 0 装, v2.0.0-rc.1 接真 backend 时实现. 现在仅画边界.
//!
//! **架构最优依据 (O-6 锚 9)**:
//! - 总体: trait 抽象在 foundation 与 ToolCapability/ProviderCapability/CredentialResolver 三件套对齐
//! - 系统: trait 在 foundation, impl 在 engine (单向依赖, 与 plugin 体系一致)
//! - 架构: backend registry 与 plugin registry 同一抽象层, 入口语义不歧义
//!
//! **v1 compat**: `apeireth-memory::backend::MemoryBackend` 通过 re-export 仍可访问,
//! 现有 0 外部 user (15 测试全在 `apeireth-memory` 内部), 0 破坏.
//!
//! **3 阶审查** (commit message 必写明):
//! 1. 总体: 在 v2 整体语境里, 4 个 capability 抽象 (Tool/Provider/CredentialResolver/MemoryBackend) 集中 foundation, 降低 v1 era 86-crate "registry 散在多处" 的风险
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin/Provider/Tool 一致)
//! 3. 架构: 与 plugin manager 单 trait 边界, runtime 拿 `Arc<dyn MemoryBackend>` 注入, 不直接 import memory
//!
//! **关于 StreamKind / HistoryEntry**:
//! 两者当前在 `apeireth-memory::append_only` (Refactor-1 不搬运它们, 范围控制).
//! trait method `append_stream` / `list_stream` 用 `&str` (stream name) + `serde_json::Value`
//! (single entry) — 避开 plugin → memory 直接依赖.
//! rc 阶段 rc-2 任务 (Experience SQLite impl) 一起搬到 core (合并 refactor).

use apeireth_core::Episode;

/// 记忆后端错误（trait 临时错误类型；rc 阶段统一）
#[derive(Debug)]
pub struct MemoryBackendError(pub String);

impl std::fmt::Display for MemoryBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "memory backend error: {}", self.0)
    }
}

impl std::error::Error for MemoryBackendError {}

/// trait result type: `Result<T, MemoryBackendError>`, rc 阶段替换为 `MemoryResult<T>` (搬 core)
pub type MemoryBackendResult<T> = Result<T, MemoryBackendError>;

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
/// - **错误统一**：所有错误走 `MemoryBackendResult<T>` (= `Result<T, MemoryError>`)
/// - **跨模块类型**: Episode 来自 `apeireth_core` (核心域), 但 `StreamKind`/`HistoryEntry` 当前在
///   `apeireth_memory`; trait 暂不直接引用它们, 用 `&str` (stream 名字) + `serde_json::Value`
///   (单条 entry) 传, 让 impl 自己做转换. 完整类型安全等 Refactor-1-后 阶段把 StreamKind/HistoryEntry
///   升 core 时再做.
pub trait MemoryBackend: Send + Sync {
    /// 后端稳定标签 (e.g. `"sqlite"`, `"file"`, `"in_memory"`)
    fn name(&self) -> &'static str;

    /// 后端类型枚举
    fn kind(&self) -> BackendKind;

    /// 健康检查（启动时调，确认后端可用）
    fn ping(&self) -> MemoryBackendResult<()> {
        Ok(())
    }

    // ===== Episode 操作 =====

    /// 写入一条 Episode (append-only)。
    fn put_episode(&self, ep: &Episode) -> MemoryBackendResult<()>;

    /// 按 id 读取一条 Episode。
    fn get_episode(&self, id: &str) -> MemoryBackendResult<Option<Episode>>;

    /// 检索某 session 的最近 N 条 Episode（按时间升序，末尾 N 条）。
    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryBackendResult<Vec<Episode>>;

    // ===== 6 历史流操作 =====

    /// 追加一条历史流条目 (append-only)。
    ///
    /// `stream_name` 取 6 个固定值之一: `"thought"` / `"proposal"` / `"action"`
    /// / `"relation"` / `"evolution"` / `"reflection"`. 0 装 impl 内部映射到
    /// `apeireth_memory::StreamKind`; 完整的 13 键领域安全等 rc 阶段做.
    /// `entry` 是 JSON 序列化的 `HistoryEntry` (id/subject_id/subject_rev/session_id/
    /// created_at/payload/source/tags/tombstoned_at), 字段含义见 v1 era HistoryEntry.
    fn append_stream(&self, stream_name: &str, entry: serde_json::Value) -> MemoryBackendResult<()>;

    /// 列出某 session 的某流最近 N 条（按时间升序，末尾 N 条，未 tombstone 的）。
    /// 返回的 JSON 数组元素结构与 `append_stream` 的 `entry` 一致.
    fn list_stream(
        &self,
        stream_name: &str,
        session_id: &str,
        n: usize,
    ) -> MemoryBackendResult<Vec<serde_json::Value>>;
}
