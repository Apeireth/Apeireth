//! Memory backend capability boundary: foundation owns the trait; engine owns adapters.
//! The engine re-exports this trait for existing callers. Scope metadata remains JSON
//! so this layer does not depend on concrete memory policy or storage implementations.

use apeireth_core::kernel::{HistoryEntry, StreamKind};
use apeireth_core::Episode;

/// 统一 capability trait 错误类型 (O-6 锚兑现 #12, 2026-08-27).
///
/// 用 std `Box<dyn Error + Send + Sync>` 而非各 backend 本地 error type — 避免
/// plugin → backend crate 循环依赖. impl 端包: `Box::new(my_local_error)`.
///
/// 0 装 PASS: v2.0.0-rc 阶段迁移到 associated `type Error` trait method (impl
/// 自由选具体错误类型) + `From<impl error> for Box<dyn Error>` 自动转换.
pub type CapabilityResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

/// Whether a combined write appended evidence or accepted an idempotent replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpisodeWriteOutcome {
    Inserted,
    Existing,
}

/// 记忆后端抽象。
///
/// **核心契约**：
/// - **append-only 写入**：所有 put_* / append_* 操作不可变
/// - **同步 API**：当前 v2 runtime 在 spawn_blocking 里调 memory，sync trait 足够
/// - **Send + Sync**：后端实例可跨线程共享
/// - **错误统一**（O-6 锚 #9 #12 兑现）：所有错误走 `CapabilityResult<T>`
/// - **跨模块类型 (typed enum, O-6 锚 #2 兑现)**: Episode 来自 `apeireth_core`,
///   `StreamKind` 是 `apeireth_core::kernel::StreamKind` typed enum. HistoryEntry 字段仍走
///   `serde_json::Value` (rc 阶段评估 typed struct).
pub trait MemoryBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn kind(&self) -> BackendKind;
    fn ping(&self) -> CapabilityResult<()> {
        Ok(())
    }

    fn put_episode(&self, ep: &Episode) -> CapabilityResult<()>;

    /// Write an episode and its sidecar before publishing it to working memory.
    ///
    /// The compatibility default is sequential and may leave the episode persisted
    /// if metadata fails. It does not add metadata support to legacy backends.
    /// Transactional adapters override this method to commit both together.
    /// Legacy success reports Inserted; replay detection requires an override.
    fn put_episode_with_metadata(
        &self,
        ep: &Episode,
        metadata: serde_json::Value,
    ) -> CapabilityResult<EpisodeWriteOutcome> {
        self.put_episode(ep)?;
        self.put_episode_metadata(&ep.id, metadata)?;
        Ok(EpisodeWriteOutcome::Inserted)
    }

    fn get_episode(&self, id: &str) -> CapabilityResult<Option<Episode>>;
    fn recent_episodes(&self, session_id: &str, n: usize) -> CapabilityResult<Vec<Episode>>;

    /// Persist structured Memory Plane metadata beside an append-only episode.
    ///
    /// This optional extension keeps the foundation trait independent of the
    /// engine's concrete scope types while allowing production backends to
    /// persist scope/provenance additively. Backends without metadata support
    /// retain legacy behavior and return no metadata on reads.
    fn put_episode_metadata(
        &self,
        _episode_id: &str,
        _metadata: serde_json::Value,
    ) -> CapabilityResult<()> {
        Ok(())
    }

    /// Read structured Memory Plane metadata, if the backend supports it.
    fn get_episode_metadata(
        &self,
        _episode_id: &str,
    ) -> CapabilityResult<Option<serde_json::Value>> {
        Ok(None)
    }

    /// 追加一条历史流条目 (append-only). `kind` 是 typed enum (6 流).
    /// `entry` 是 typed `HistoryEntry` struct (O-6 锚 #18 兑现, 替代 serde_json::Value 占位).
    fn append_stream(&self, kind: StreamKind, entry: HistoryEntry) -> CapabilityResult<()>;

    /// 列出某 session 的某流最近 N 条（按时间升序，末尾 N 条，未 tombstone 的）.
    fn list_stream(
        &self,
        kind: StreamKind,
        session_id: &str,
        n: usize,
    ) -> CapabilityResult<Vec<HistoryEntry>>;
}
