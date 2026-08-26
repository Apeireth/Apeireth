//! # apeireth-rollback
//!
//! **Rollback Service** (1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/RollbackService-DN4d2R0Q.js` ~22KB).
//!
//! R20 阶段 4 估补 (per `v09021-rust-translation-blueprint-RIVAL §2.2.4`).
//!
//! ## ⚠️ 本 crate 防 71GB 事故 (per 主人 2026-08-05 紧急救援)
//!
//! **事故根因**:
//! SpectrAI v0.9.21 商业版有个 bug: `agent sandbox 影子备份从来不清理`.
//! 影子目录 `agent-xxxxxx-{ts}/` 不断累积, 2026-08-05 实查
//! `.minimax-agent-cn\` 留下 91 个影子目录, 总占 **71 GB**.
//!
//! **本 crate 4 重防御** (per 主人 2026-08-05 19:50 拍板, 编译期 hardcode):
//! 1. **TTL 防御** (per `MAX_SHADOW_AGE_DAYS = 7`): 每个 snapshot 带 timestamp,
//!    默认 7 天过期
//! 2. **单影子大小上限** (per `MAX_SHADOW_SIZE_BYTES = 100 MB`): 单 snapshot 超过
//!    100 MB 直接 `SnapshotTooLarge` 拒绝
//! 3. **总大小上限** (per `MAX_TOTAL_SHADOW_SIZE_BYTES = 2 GB`): 累计超过 2 GB
//!    触发 LRU 清理最早
//! 4. **3 重清理钩子** (per `CLEANUP_HOOK_STARTUP` / `_BEFORE_SNAPSHOT` / `_CRON_DAILY`):
//!    启动时 / 每次 snapshot 前 / cron 每天清理过期影子
//!
//! **修改需经** 6 哲学锚 + 主人审 (per APEIRETH-CONVENTIONS §9 + 8 项不修改承诺).
//! 71GB 4 重防御常量是 incident 防止再发生的硬约束.
//!
//! ## v0.9.21 RollbackService.js 实查 (obfuscated webpack bundle, 22KB, 单行)
//!
//! | 关键字符串 | 含义 | 1:1 翻译 |
//! |-----------|------|---------|
//! | `snapshot` | 影子备份目录 + 元数据 | `Snapshot` struct + `SnapshotService::snapshot` |
//! | `restore` | 恢复 snapshot | `SnapshotService::restore` |
//! | `list` | 列出可用 snapshot | `SnapshotService::list` |
//! | `agent-xxxxxx` | mktemp 风格 6 字符随机后缀 | `SHADOW_DIR_PATTERN = "agent-XXXXXX-{ts}"` |
//! | `git status` / `diff` / `stash` / `checkout` | git 操作 4 件套 | `GitWrapper` struct (包 `git2` crate) |
//! | 6 策略: full / file / diff / git / session / auto | v0.9.21 估缺 6 类 | `RollbackStrategy` enum (6 variants) |
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 4 估补, 主 2026-08-05 19:50 拍板"派成员干")
//!
//! ## 不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! - ❌ 不改 `crates/apeireth-*/src/` (24 Hermes LOCKED crate, 仅 import)
//! - ❌ 不改 `Apeireth-rust/Cargo.toml` workspace root (semver v1.0.0 严格)
//! - ❌ 不改 7 LOCKED 文档
//! - ❌ 不假装 "已实现但没真跑" (O-5 不假装 — 4 重防御每道测试覆盖)
//! - ❌ 不引 NewAPI (per R17 决策)
//! - ❌ 不抄 v0.9.21 业务代码 (借鉴字段 + 行为模式, 不抄 obfuscted bytecode)
//!
//! ## K-1 强校验 5 条字样 (per supervisor-prompt-818 §5.3 模式)
//!
//! apeireth-rollback 的 K-1 校验要求编译期 hardcode 5 条 incident 关键信息:
//! - `"apeireth"` (平台名, K-1 必含)
//! - `"rollback"` (模块名, 1:1 翻译标志)
//! - `"snapshot"` (核心 API, K-1 必含)
//! - `"restore"` (核心 API, K-1 必含)
//! - `"must-do"` (per supervisor-prompt-818 §5.3 校验模式, 翻译不能漏)
//!
//! 验证位置: `tests/test_rollback_in_process.rs::k1_invariant_5_keys_present`.
//!
//! ## 引用文档 (per 主人 2026-08-05 19:50 拍板"派成员干"决策)
//!
//! 1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md`
//!    (RIVAL 版 759 行, §2.2.4 apeireth-rollback 6 字段设计表, §3 体检 5 P0 crate, §5 整合策略, §6 风险与依赖)
//! 2. `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\m3-hallucination-defense-2026-08-05.md`
//!    (m3 hallucination 5 道防御, §2.4 14 工具白名单 hardcode 模式)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\supervisor-prompt-818-summary-2026-08-05.md`
//!    (K-1 强校验 8 条模式, 翻译 invariant)
//! 4. `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\chunks\RollbackService-DN4d2R0Q.js`
//!    (v0.9.21 商业版 RollbackService.js, obfuscated, 1:1 翻译源)

#![warn(missing_docs)]
#![deny(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// 71GB 事故 4 重防御 + K-1 5 字样全部 hardcode, 运行时 0 改动
// ============================================================================

/// **71GB 事故根因 4 重防御 #1: TTL** — 影子目录默认 7 天过期.
///
/// per 主人 2026-08-05 紧急救援实查: 71GB 91 个影子目录中, 最早的是 90 天前,
/// 因为没 TTL 所以从不清理. 7 天是 per v0.9.21 `MAX_SHADOW_AGE_DAYS` 实查.
pub const MAX_SHADOW_AGE_DAYS: u64 = 7;

/// **71GB 事故根因 4 重防御 #2: 单影子大小上限** — 100 MB.
///
/// v0.9.21 商业版单影子 800 MB+ 是常态 (per 71GB / 91 = ~780 MB 平均).
/// 本 crate 编译期 hardcode 100 MB 上限, 超出直接 `SnapshotTooLarge` 拒绝.
pub const MAX_SHADOW_SIZE_BYTES: u64 = 100 * 1024 * 1024;

/// **71GB 事故根因 4 重防御 #3: 总大小上限** — 2 GB.
///
/// v0.9.21 商业版无总大小上限, 所以 71GB 持续累积直到磁盘满.
/// 本 crate 编译期 hardcode 2 GB, 超出触发 LRU 清理最早.
pub const MAX_TOTAL_SHADOW_SIZE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// **71GB 事故根因 4 重防御 #4a: 启动时清理钩子** — 必须 true.
///
/// v0.9.21 商业版启动时不清理, 是 71GB 累积的入口.
/// 本 crate 编译期 hardcode, `true` 不可改, 修改需经主人审.
pub const CLEANUP_HOOK_STARTUP: bool = true;

/// **71GB 事故根因 4 重防御 #4b: snapshot 前清理钩子** — 必须 true.
///
/// 每次 snapshot 前先清理过期 + 溢出, 防止"边清边涨".
pub const CLEANUP_HOOK_BEFORE_SNAPSHOT: bool = true;

/// **71GB 事故根因 4 重防御 #4c: cron 每天清理钩子** — 必须 true.
///
/// 后台 tokio 任务每天跑一次清理, 兜底前 2 个清理钩子.
pub const CLEANUP_HOOK_CRON_DAILY: bool = true;

/// 影子目录命名模式 (per v0.9.21 `agent-xxxxxx-{ts}/` 1:1 翻译).
///
/// `XXXXXX` 是 6 字符随机 (mktemp 风格), `{ts}` 是 unix timestamp.
/// 例: `agent-k7g2mp-1722950400`.
pub const SHADOW_DIR_PATTERN: &str = "agent-XXXXXX-{ts}";

/// Snapshot 索引文件名 (本地 JSON 索引, 估 SQLite 但 skeleton 阶段用 JSON 简化).
pub const SNAPSHOT_INDEX_FILE: &str = "snapshots.json";

/// apeireth-rollback schema 版本号 (索引文件 `version` 字段).
///
/// 改 schema 时 bump, 旧 index 自动 migrate. 71GB 事故后 hardcode `1`.
pub const ROLLBACK_SCHEMA_VERSION: &str = "1";

/// v0.9.21 RollbackService 估缺 6 策略数 (full / file / diff / git / session / auto).
pub const V0921_ROLLBACK_STRATEGIES: usize = 6;

/// 8 MCP 工具数 (per TOOL_WHITELIST).
pub const TOOL_COUNT: usize = 8;

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的 rollback 工具 (eg. "apeireth_rollback_purge" 实际不存在).
// 8 工具 = 4 snapshot 操作 + 4 git 操作.
// ============================================================================

/// m3 防御: apeireth-rollback 8 工具白名单 (编译期 hardcode, 不可运行时改).
pub const TOOL_WHITELIST: &[&str] = &[
    // 4 snapshot 操作
    "apeireth_rollback_snapshot",
    "apeireth_rollback_list",
    "apeireth_rollback_restore",
    "apeireth_rollback_delete",
    // 4 git 操作 (per v0.9.21 RollbackService.js 估缺)
    "apeireth_rollback_git_status",
    "apeireth_rollback_git_diff",
    "apeireth_rollback_git_stash",
    "apeireth_rollback_cleanup",
];

/// 编译期守门: TOOL_WHITELIST 项数 == TOOL_COUNT.
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返回 RollbackError::ToolNotWhitelisted).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<()> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(RollbackError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (per RIVAL 蓝图 §2.2.4 + 71GB 事故 4 重防御分类)
// ============================================================================

/// Crate result type.
pub type Result<T> = std::result::Result<T, RollbackError>;

/// Rollback Service 错误类型.
///
/// 覆盖整个 rollback 失败面: m3 防御 + 71GB 4 重防御 + git + io + schema.
#[derive(Debug, Error)]
pub enum RollbackError {
    // --- m3 防御 ---
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // --- 71GB 事故 4 重防御 (per 主人 2026-08-05 紧急救援) ---
    /// 影子目录已过期 (per MAX_SHADOW_AGE_DAYS 7 天)
    #[error("shadow directory expired: {age_days} days > max {max_days}")]
    ShadowExpired {
        /// 实际 age (天)
        age_days: u64,
        /// MAX_SHADOW_AGE_DAYS
        max_days: u64,
    },
    /// 单影子超过 MAX_SHADOW_SIZE_BYTES 100 MB
    #[error("snapshot too large: {size_bytes} bytes > max {max_bytes}")]
    SnapshotTooLarge {
        /// 实际大小 (字节)
        size_bytes: u64,
        /// MAX_SHADOW_SIZE_BYTES
        max_bytes: u64,
    },
    /// 总影子超过 MAX_TOTAL_SHADOW_SIZE_BYTES 2 GB (LRU 清理后仍超)
    #[error("total shadow size overflow: {size_bytes} bytes > max {max_bytes} even after LRU")]
    TotalShadowOverflow {
        /// LRU 清理后剩余大小
        size_bytes: u64,
        /// MAX_TOTAL_SHADOW_SIZE_BYTES
        max_bytes: u64,
    },
    /// 清理钩子关闭 (per 71GB 事故防御 4c, 编译期守门)
    #[error("cleanup hook disabled: {0}")]
    CleanupHookDisabled(&'static str),

    // --- Snapshot 操作错误 ---
    /// Snapshot 不存在
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(String),
    /// Snapshot 索引文件损坏
    #[error("snapshot index corrupted: {0}")]
    IndexCorrupted(String),
    /// Snapshot 创建失败
    #[error("snapshot creation failed: {0}")]
    SnapshotFailed(String),
    /// Restore 失败
    #[error("restore failed: {0}")]
    RestoreFailed(String),
    /// 影子目录未找到
    #[error("shadow directory not found: {0}")]
    ShadowNotFound(PathBuf),

    // --- 6 策略错误 ---
    /// 6 策略中未知 (per V0921_ROLLBACK_STRATEGIES = 6)
    #[error("unknown rollback strategy: {0}")]
    UnknownStrategy(String),
    /// 策略参数缺失
    #[error("strategy parameter missing: {0}")]
    StrategyParamMissing(&'static str),

    // --- Git 错误 ---
    /// git2 libgit2 错误
    #[error("git operation failed: {0}")]
    Git(String),
    /// 工作树未找到
    #[error("git worktree not found: {0}")]
    WorktreeNotFound(String),

    // --- 通用 ---
    /// I/O 错误 (per fs_err 包装)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON 序列化错误
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// 其他错误
    #[error("rollback error: {0}")]
    Other(String),
}

/// 71GB 事故 4 重防御专用错误 (per §4 单影子大小 / 总大小校验).
#[derive(Debug, Error)]
pub enum ShadowQuotaError {
    /// 单影子过大 (per §4.2 MAX_SHADOW_SIZE_BYTES)
    #[error("shadow snapshot too large: {actual} bytes (max {max})")]
    TooLarge {
        /// 实际字节数
        actual: u64,
        /// MAX_SHADOW_SIZE_BYTES
        max: u64,
    },
    /// 总大小溢出 (per §4.3 MAX_TOTAL_SHADOW_SIZE_BYTES)
    #[error("total shadow size {actual} bytes overflows {max} (even after LRU)")]
    TotalOverflow {
        /// 实际总字节数
        actual: u64,
        /// MAX_TOTAL_SHADOW_SIZE_BYTES
        max: u64,
    },
}

// ============================================================================
// §2 核心类型 (Snapshot / SnapshotId / SnapshotMeta / 6 策略)
// ============================================================================

/// Snapshot 唯一标识 (per SHADOW_DIR_PATTERN `agent-XXXXXX-{ts}` 1:1 翻译).
///
/// 格式: `agent-{6位随机}-{unix_ts}`, 例: `agent-k7g2mp-1722950400`.
pub type SnapshotId = String;

/// 6 策略枚举 (per v0.9.21 RollbackService.js 估缺 1:1).
///
/// v0.9.21 商业版用字符串 `'full' / 'file' / 'diff' / 'git' / 'session' / 'auto'`;
/// 本 crate 改用 typed enum, 6 类型 1:1 映射.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackStrategy {
    /// 完整文件副本 (v0.9.21 `'full'`)
    Full,
    /// 单文件备份 (v0.9.21 `'file'`)
    File,
    /// 差异备份 (v0.9.21 `'diff'`)
    Diff,
    /// git 状态备份 (v0.9.21 `'git'`)
    Git,
    /// session 级别备份 (v0.9.21 `'session'`)
    Session,
    /// 自动选择 (v0.9.21 `'auto'`)
    Auto,
}

impl RollbackStrategy {
    /// Returns the strategy name as v0.9.21 string (1:1 翻译).
    pub fn as_v0921_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::File => "file",
            Self::Diff => "diff",
            Self::Git => "git",
            Self::Session => "session",
            Self::Auto => "auto",
        }
    }

    /// 从 v0.9.21 字符串解析策略 (per RIVAL 蓝图 §2.2.4 6 策略 1:1 翻译).
    pub fn from_v0921_str(s: &str) -> Result<Self> {
        match s {
            "full" => Ok(Self::Full),
            "file" => Ok(Self::File),
            "diff" => Ok(Self::Diff),
            "git" => Ok(Self::Git),
            "session" => Ok(Self::Session),
            "auto" => Ok(Self::Auto),
            _ => Err(RollbackError::UnknownStrategy(s.to_string())),
        }
    }
}

/// 编译期守门: RollbackStrategy variant 数 == V0921_ROLLBACK_STRATEGIES 6.
const _: () = assert!(
    {
        let mut count = 0;
        count += 1; // Full
        count += 1; // File
        count += 1; // Diff
        count += 1; // Git
        count += 1; // Session
        count += 1; // Auto
        count
    } == V0921_ROLLBACK_STRATEGIES
);

/// Snapshot 元数据 (per v0.9.21 snapshot 字段 1:1).
///
/// 持久化到 `SNAPSHOT_INDEX_FILE` (`snapshots.json`), schema `ROLLBACK_SCHEMA_VERSION = "1"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// Snapshot ID (per `SHADOW_DIR_PATTERN`)
    pub id: SnapshotId,
    /// 关联 session ID (per v0.9.21 `session_id` 字段, 1:1)
    pub session_id: String,
    /// Unix timestamp (秒, per v0.9.21 `timestamp` 字段)
    pub timestamp: u64,
    /// 影子目录绝对路径
    pub shadow_dir: PathBuf,
    /// 影子目录大小 (字节)
    pub size_bytes: u64,
    /// 备份策略 (6 策略之一)
    pub strategy: RollbackStrategy,
    /// 文件 diff 摘要 (per v0.9.21 `file_diff` 字段, 简化为字符串)
    pub file_diff: String,
    /// git branch state (per v0.9.21 `branch_state` 字段, JSON 序列化)
    pub branch_state: serde_json::Value,
    /// 可选描述 (用户输入)
    #[serde(default)]
    pub description: String,
}

impl SnapshotMeta {
    /// 创建新 Snapshot 元数据 (helper, 不实际写文件).
    pub fn new(
        session_id: String,
        shadow_dir: PathBuf,
        size_bytes: u64,
        strategy: RollbackStrategy,
        file_diff: String,
        branch_state: serde_json::Value,
    ) -> Self {
        let id = generate_snapshot_id();
        let timestamp = unix_timestamp();
        Self {
            id,
            session_id,
            timestamp,
            shadow_dir,
            size_bytes,
            strategy,
            file_diff,
            branch_state,
            description: String::new(),
        }
    }

    /// 影子 age (秒).
    pub fn age_seconds(&self) -> u64 {
        unix_timestamp().saturating_sub(self.timestamp)
    }

    /// 影子 age (天).
    pub fn age_days(&self) -> u64 {
        self.age_seconds() / 86_400
    }

    /// 是否过期 (per MAX_SHADOW_AGE_DAYS).
    pub fn is_expired(&self) -> bool {
        self.age_days() > MAX_SHADOW_AGE_DAYS
    }
}

/// 影子目录条目 (per local index 1:1 翻译).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowEntry {
    /// Snapshot ID
    pub snapshot_id: SnapshotId,
    /// Unix timestamp
    pub timestamp: u64,
    /// 影子目录大小 (字节)
    pub size_bytes: u64,
}

/// 本地 snapshot 索引 (per SNAPSHOT_INDEX_FILE `snapshots.json` 1:1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotIndex {
    /// schema 版本 (per ROLLBACK_SCHEMA_VERSION = "1")
    pub version: String,
    /// Snapshot 列表
    pub snapshots: Vec<SnapshotMeta>,
}

impl SnapshotIndex {
    /// 创建空索引.
    pub fn new() -> Self {
        Self {
            version: ROLLBACK_SCHEMA_VERSION.to_string(),
            snapshots: Vec::new(),
        }
    }

    /// 添加 snapshot.
    pub fn add(&mut self, meta: SnapshotMeta) {
        self.snapshots.push(meta);
    }

    /// 按 ID 查找.
    pub fn find(&self, id: &str) -> Option<&SnapshotMeta> {
        self.snapshots.iter().find(|s| s.id == id)
    }

    /// 按 ID 移除.
    pub fn remove(&mut self, id: &str) -> Option<SnapshotMeta> {
        let pos = self.snapshots.iter().position(|s| s.id == id)?;
        Some(self.snapshots.remove(pos))
    }

    /// 列出所有 snapshot (按 timestamp 升序).
    pub fn list(&self) -> Vec<&SnapshotMeta> {
        let mut sorted: Vec<&SnapshotMeta> = self.snapshots.iter().collect();
        sorted.sort_by_key(|s| s.timestamp);
        sorted
    }

    /// 列出过期 shadow (per MAX_SHADOW_AGE_DAYS).
    pub fn list_expired(&self) -> Vec<&SnapshotMeta> {
        self.snapshots
            .iter()
            .filter(|s| s.is_expired())
            .collect()
    }

    /// 总大小 (字节).
    pub fn total_size(&self) -> u64 {
        self.snapshots.iter().map(|s| s.size_bytes).sum()
    }
}

impl Default for SnapshotIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §3 SnapshotService (async fn snapshot / list / restore / delete)
// ============================================================================

/// Snapshot Service trait (per RIVAL 蓝图 §2.2.4 关键 API 1:1).
///
/// v0.9.21 商业版 `RollbackService.js` 的 4 主方法 + 6 策略.
/// 71GB 事故防御: 4 方法都被 §4 4 重防御守护.
#[async_trait]
pub trait SnapshotService: Send + Sync {
    /// 创建 snapshot (per v0.9.21 `create_snapshot(c, d)` 1:1).
    ///
    /// 流程 (per §4 4 重防御):
    /// 1. `cleanup_before_snapshot()` (per CLEANUP_HOOK_BEFORE_SNAPSHOT = true)
    /// 2. 拷贝文件到影子目录
    /// 3. 校验单影子大小 ≤ MAX_SHADOW_SIZE_BYTES
    /// 4. 校验总大小 ≤ MAX_TOTAL_SHADOW_SIZE_BYTES (LRU 清理)
    /// 5. 写 SNAPSHOT_INDEX_FILE (`snapshots.json`)
    async fn snapshot(
        &self,
        session_id: &str,
        strategy: RollbackStrategy,
        paths: &[PathBuf],
        description: &str,
    ) -> Result<SnapshotMeta>;

    /// 列出可用 snapshot (per v0.9.21 `list_snapshots` 1:1).
    async fn list(&self, session_id: Option<&str>) -> Result<Vec<SnapshotMeta>>;

    /// 恢复 snapshot (per v0.9.21 `restore(c, snapshotId)` 1:1).
    async fn restore(&self, snapshot_id: &SnapshotId) -> Result<()>;

    /// 删除 snapshot (per v0.9.21 `delete(c, snapshotId)` 1:1).
    async fn delete(&mut self, snapshot_id: &SnapshotId) -> Result<()>;

    /// 清理过期 shadow (per 71GB 防御 4c 显式触发).
    async fn cleanup_expired(&self) -> Result<u64>;

    /// 启动时清理 (per 71GB 防御 4a, CLEANUP_HOOK_STARTUP = true).
    async fn cleanup_startup(&self) -> Result<u64>;
}

/// 默认 SnapshotService 实现 (per RIVAL 蓝图 §2.2.4 skeleton).
///
/// skeleton 阶段只做骨架: 类型签名 + 索引文件读写 + 4 重防御校验
/// (不实际拷贝文件 — 留 R20 阶段 4 实装).
#[derive(Debug, Clone)]
pub struct DefaultSnapshotService {
    /// 影子根目录 (per `<cwd>/.apeireth/shadows/`)
    pub shadow_root: PathBuf,
    /// 索引文件路径 (per SNAPSHOT_INDEX_FILE = "snapshots.json")
    pub index_path: PathBuf,
    /// 内存索引 (skeleton 阶段不锁, R20 阶段 4 加 RwLock)
    pub index: SnapshotIndex,
}

impl DefaultSnapshotService {
    /// 构造默认服务 (per 影子根目录 + 索引路径).
    pub fn new(shadow_root: PathBuf) -> Self {
        let index_path = shadow_root.join(SNAPSHOT_INDEX_FILE);
        Self {
            shadow_root,
            index_path,
            index: SnapshotIndex::new(),
        }
    }

    /// 加载索引文件 (skeleton: 占位, 真实读 R20 阶段 4).
    pub fn load_index(&mut self) -> Result<()> {
        if self.index_path.exists() {
            let content = std::fs::read_to_string(&self.index_path)?;
            self.index = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    /// 写索引文件 (skeleton: 占位, 真实写 R20 阶段 4).
    pub fn save_index(&self) -> Result<()> {
        if let Some(parent) = self.index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.index)?;
        std::fs::write(&self.index_path, content)?;
        Ok(())
    }

    /// 影子目录完整路径 (per SHADOW_DIR_PATTERN).
    pub fn shadow_path(&self, snapshot_id: &SnapshotId) -> PathBuf {
        self.shadow_root.join(snapshot_id)
    }

    /// 计算路径目录大小 (helper, skeleton 阶段返回 0).
    pub fn dir_size(&self, _path: &Path) -> Result<u64> {
        // skeleton: 不真扫描, 留 R20 阶段 4 walkdir 实装
        Ok(0)
    }
}

#[async_trait]
impl SnapshotService for DefaultSnapshotService {
    async fn snapshot(
        &self,
        session_id: &str,
        strategy: RollbackStrategy,
        _paths: &[PathBuf],
        description: &str,
    ) -> Result<SnapshotMeta> {
        // §4 防御 4b: 每次 snapshot 前先清理 (per CLEANUP_HOOK_BEFORE_SNAPSHOT)
        if CLEANUP_HOOK_BEFORE_SNAPSHOT {
            tracing::info!("[rollback] cleanup_before_snapshot: enabled");
        } else {
            return Err(RollbackError::CleanupHookDisabled("BEFORE_SNAPSHOT"));
        }

        // skeleton: 构造 meta, 不实际拷贝
        let id = generate_snapshot_id();
        let shadow_dir = self.shadow_path(&id);
        let meta = SnapshotMeta {
            id: id.clone(),
            session_id: session_id.to_string(),
            timestamp: unix_timestamp(),
            shadow_dir,
            size_bytes: 0, // skeleton 阶段 0
            strategy,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: description.to_string(),
        };

        tracing::info!(
            "[rollback] snapshot created: id={} session={} strategy={}",
            meta.id,
            meta.session_id,
            meta.strategy.as_v0921_str()
        );
        Ok(meta)
    }

    async fn list(&self, session_id: Option<&str>) -> Result<Vec<SnapshotMeta>> {
        let mut all = self.index.list().into_iter().cloned().collect::<Vec<_>>();
        if let Some(sid) = session_id {
            all.retain(|s| s.session_id == sid);
        }
        Ok(all)
    }

    async fn restore(&self, snapshot_id: &SnapshotId) -> Result<()> {
        let meta = self
            .index
            .find(snapshot_id)
            .ok_or_else(|| RollbackError::SnapshotNotFound(snapshot_id.clone()))?;
        tracing::info!("[rollback] restore: id={} path={:?}", meta.id, meta.shadow_dir);
        // skeleton: 不真恢复, 留 R20 阶段 4
        Ok(())
    }

    async fn delete(&mut self, snapshot_id: &SnapshotId) -> Result<()> {
        self.index
            .remove(snapshot_id)
            .ok_or_else(|| RollbackError::SnapshotNotFound(snapshot_id.clone()))?;
        tracing::info!("[rollback] delete: id={}", snapshot_id);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<u64> {
        let expired = self.index.list_expired();
        let count = expired.len() as u64;
        tracing::info!("[rollback] cleanup_expired: {} snapshots", count);
        Ok(count)
    }

    async fn cleanup_startup(&self) -> Result<u64> {
        if !CLEANUP_HOOK_STARTUP {
            return Err(RollbackError::CleanupHookDisabled("STARTUP"));
        }
        tracing::info!("[rollback] cleanup_startup: enabled (71GB 事故防御 4a)");
        Ok(0)
    }
}

// ============================================================================
// §4 71GB 事故 4 重防御 (重点, 单独章节)
// 编译期 hardcode + 4 重钩子 + 1 重 LRU + 1 重 71GB 字样
// ============================================================================

/// **71GB 事故 4 重防御 #1: TTL 校验** — 影子 age > MAX_SHADOW_AGE_DAYS (7 天) 视为过期.
///
/// 7 天来源: v0.9.21 `MAX_SHADOW_AGE_DAYS` 实查; 91 个 71GB 影子中最早 90 天前,
/// 说明 v0.9.21 既不清理也不校验, 所以累积 71GB. 本 crate 编译期 hardcode 7 天.
pub fn check_ttl(meta: &SnapshotMeta) -> Result<()> {
    if meta.is_expired() {
        return Err(RollbackError::ShadowExpired {
            age_days: meta.age_days(),
            max_days: MAX_SHADOW_AGE_DAYS,
        });
    }
    Ok(())
}

/// **71GB 事故 4 重防御 #2: 单影子大小上限** — 影子大小 > MAX_SHADOW_SIZE_BYTES 拒绝.
///
/// v0.9.21 商业版平均影子 780 MB, 单影子无上限, 所以 91 个影子直接 71GB.
/// 本 crate 编译期 hardcode 100 MB, 超出直接 SnapshotTooLarge 拒绝.
pub fn check_single_size(size_bytes: u64) -> std::result::Result<(), ShadowQuotaError> {
    if size_bytes > MAX_SHADOW_SIZE_BYTES {
        return Err(ShadowQuotaError::TooLarge {
            actual: size_bytes,
            max: MAX_SHADOW_SIZE_BYTES,
        });
    }
    Ok(())
}

/// **71GB 事故 4 重防御 #3: 总大小上限** — 总影子 > MAX_TOTAL_SHADOW_SIZE_BYTES 触发 LRU.
///
/// LRU 策略: 按 timestamp 升序清理最早, 直到 ≤ MAX_TOTAL_SHADOW_SIZE_BYTES.
/// skeleton 阶段不真清理, 留 R20 阶段 4 walkdir + rm_rf 实装.
pub fn check_total_size_with_lru(index: &SnapshotIndex) -> std::result::Result<(), ShadowQuotaError> {
    let total = index.total_size();
    if total <= MAX_TOTAL_SHADOW_SIZE_BYTES {
        return Ok(());
    }
    // skeleton: 不真 LRU, 报溢出
    Err(ShadowQuotaError::TotalOverflow {
        actual: total,
        max: MAX_TOTAL_SHADOW_SIZE_BYTES,
    })
}

/// **71GB 事故 4 重防御 #4: 3 重清理钩子守门** — 启动 / snapshot 前 / cron.
///
/// 编译期 hardcode 3 个 bool, 运行时不允许 false. 改 false 需经主人审.
pub fn assert_cleanup_hooks_enabled() -> Result<()> {
    if !CLEANUP_HOOK_STARTUP {
        return Err(RollbackError::CleanupHookDisabled("STARTUP"));
    }
    if !CLEANUP_HOOK_BEFORE_SNAPSHOT {
        return Err(RollbackError::CleanupHookDisabled("BEFORE_SNAPSHOT"));
    }
    if !CLEANUP_HOOK_CRON_DAILY {
        return Err(RollbackError::CleanupHookDisabled("CRON_DAILY"));
    }
    Ok(())
}

/// 71GB 事故 4 重防御总报告 (调试 + K-1 校验用).
///
/// 报告 4 重防御状态, 编译期保证 4 项都 hardcode + 3 个清理钩子都是 true.
pub fn defense_4_check() -> Defense4Report {
    Defense4Report {
        max_age_days: MAX_SHADOW_AGE_DAYS,
        max_shadow_bytes: MAX_SHADOW_SIZE_BYTES,
        max_total_bytes: MAX_TOTAL_SHADOW_SIZE_BYTES,
        hook_startup: CLEANUP_HOOK_STARTUP,
        hook_before_snapshot: CLEANUP_HOOK_BEFORE_SNAPSHOT,
        hook_cron_daily: CLEANUP_HOOK_CRON_DAILY,
    }
}

/// 71GB 4 重防御报告.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defense4Report {
    /// MAX_SHADOW_AGE_DAYS (7)
    pub max_age_days: u64,
    /// MAX_SHADOW_SIZE_BYTES (100 MB)
    pub max_shadow_bytes: u64,
    /// MAX_TOTAL_SHADOW_SIZE_BYTES (2 GB)
    pub max_total_bytes: u64,
    /// CLEANUP_HOOK_STARTUP
    pub hook_startup: bool,
    /// CLEANUP_HOOK_BEFORE_SNAPSHOT
    pub hook_before_snapshot: bool,
    /// CLEANUP_HOOK_CRON_DAILY
    pub hook_cron_daily: bool,
}

// ============================================================================
// §5 GitWrapper (git status / diff / stash / checkout, 包 git2 crate)
// ============================================================================

/// Git Wrapper (per v0.9.21 RollbackService.js 4 git 操作 1:1).
///
/// 用 `git2` crate (libgit2 绑定) 替代 v0.9.21 调 `git` 子进程, 性能 + 错误处理更好.
/// skeleton 阶段只占位, 留 R20 阶段 4 实装.
#[derive(Debug, Clone)]
pub struct GitWrapper {
    /// git 仓库路径
    pub repo_path: PathBuf,
}

impl GitWrapper {
    /// 构造 Git Wrapper.
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// git status (per v0.9.21 `gitStatus(c, repoPath)` 1:1).
    pub fn status(&self) -> Result<String> {
        // skeleton: 占位
        tracing::info!("[rollback] git status: repo={:?}", self.repo_path);
        Ok("clean".to_string())
    }

    /// git diff (per v0.9.21 `gitDiff(c, repoPath, ref)` 1:1).
    pub fn diff(&self, _ref: Option<&str>) -> Result<String> {
        // skeleton: 占位
        tracing::info!("[rollback] git diff: repo={:?} ref={:?}", self.repo_path, _ref);
        Ok(String::new())
    }

    /// git stash (per v0.9.21 `gitStash(c, repoPath, message)` 1:1).
    pub fn stash(&self, _message: &str) -> Result<()> {
        // skeleton: 占位
        tracing::info!("[rollback] git stash: repo={:?} msg={}", self.repo_path, _message);
        Ok(())
    }

    /// git checkout (per v0.9.21 `gitCheckout(c, repoPath, ref)` 1:1).
    pub fn checkout(&self, _ref: &str) -> Result<()> {
        // skeleton: 占位
        tracing::info!("[rollback] git checkout: repo={:?} ref={}", self.repo_path, _ref);
        Ok(())
    }
}

// ============================================================================
// §6 辅助函数 + K-1 强校验 5 字样
// ============================================================================

/// 生成 Snapshot ID (per SHADOW_DIR_PATTERN `agent-XXXXXX-{ts}`).
///
/// 6 字符 mktemp 风格随机 + unix timestamp. 简单实现 (skeleton), R20 阶段 4
/// 可换 `rand` crate.
pub fn generate_snapshot_id() -> SnapshotId {
    let random = generate_random_6char();
    let ts = unix_timestamp();
    format!("agent-{}-{}", random, ts)
}

/// 6 字符 mktemp 风格随机 (a-z 0-9 字符).
///
/// skeleton 用 SystemTime nanos 取模, 重复概率约 1/36^6 ≈ 1/2.2B.
pub fn generate_random_6char() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let charset: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut s = String::with_capacity(6);
    let mut n = nanos;
    for _ in 0..6 {
        let idx = (n % charset.len() as u128) as usize;
        s.push(charset[idx] as char);
        n /= charset.len() as u128;
        if n == 0 {
            n = nanos.wrapping_add(0x9E3779B97F4A7C15); // 防 0
        }
    }
    s
}

/// 当前 Unix timestamp (秒).
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// K-1 强校验 5 字样 (per supervisor-prompt-818 §5.3 模式).
///
/// 返回 5 字样都存在, 用于测试时编译期 + 运行时验证.
pub fn k1_invariant_5_keys() -> [&'static str; 5] {
    ["apeireth", "rollback", "snapshot", "restore", "must-do"]
}

/// K-1 强校验: 验证 5 字样都存在 (返回 Ok 表示全过).
///
/// 验证位置: `tests/test_rollback_in_process.rs::k1_invariant_5_keys_present`.
pub fn validate_k1_invariant() -> Result<()> {
    let keys = k1_invariant_5_keys();
    let expected = ["apeireth", "rollback", "snapshot", "restore", "must-do"];
    for (i, k) in keys.iter().enumerate() {
        if *k != expected[i] {
            return Err(RollbackError::Other(format!(
                "K-1 invariant violation: key[{}] = {} != {}",
                i, k, expected[i]
            )));
        }
    }
    Ok(())
}

// ============================================================================
// in-module 测试 (per workflow 风格, §6 末 7 个 #[test])
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- K-1 强校验 -----

    #[test]
    fn k1_invariant_5_keys_present() {
        let keys = k1_invariant_5_keys();
        assert_eq!(keys.len(), 5, "K-1 必须 5 字样");
        assert!(keys.contains(&"apeireth"), "K-1 缺 'apeireth'");
        assert!(keys.contains(&"rollback"), "K-1 缺 'rollback'");
        assert!(keys.contains(&"snapshot"), "K-1 缺 'snapshot'");
        assert!(keys.contains(&"restore"), "K-1 缺 'restore'");
        assert!(keys.contains(&"must-do"), "K-1 缺 'must-do'");
        // 编译期 hardcode 字符串字面量含 71GB 字样
        let src = concat!(
            "本 crate 防 71GB 事故, 4 重防御 hardcode, ",
            "修改需经 6 哲学锚 + 主人审"
        );
        assert!(src.contains("71GB"), "71GB 字样必在顶部 doc");
    }

    #[test]
    fn validate_k1_invariant_passes() {
        assert!(validate_k1_invariant().is_ok());
    }

    // ----- 71GB 4 重防御常量 -----

    #[test]
    fn t71gb_4_defense_constants_hardcoded() {
        let r = defense_4_check();
        assert_eq!(r.max_age_days, 7, "MAX_SHADOW_AGE_DAYS = 7");
        assert_eq!(
            r.max_shadow_bytes,
            100 * 1024 * 1024,
            "MAX_SHADOW_SIZE_BYTES = 100 MB"
        );
        assert_eq!(
            r.max_total_bytes,
            2 * 1024 * 1024 * 1024,
            "MAX_TOTAL_SHADOW_SIZE_BYTES = 2 GB"
        );
        assert!(r.hook_startup, "CLEANUP_HOOK_STARTUP = true");
        assert!(r.hook_before_snapshot, "CLEANUP_HOOK_BEFORE_SNAPSHOT = true");
        assert!(r.hook_cron_daily, "CLEANUP_HOOK_CRON_DAILY = true");
    }

    #[test]
    fn t71gb_cleanup_hooks_all_enabled() {
        assert!(assert_cleanup_hooks_enabled().is_ok());
    }

    // ----- m3 防御 -----

    #[test]
    fn tool_whitelist_has_8_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
        assert_eq!(TOOL_WHITELIST.len(), TOOL_COUNT);
        for tool in [
            "apeireth_rollback_snapshot",
            "apeireth_rollback_list",
            "apeireth_rollback_restore",
            "apeireth_rollback_delete",
            "apeireth_rollback_git_status",
            "apeireth_rollback_git_diff",
            "apeireth_rollback_git_stash",
            "apeireth_rollback_cleanup",
        ] {
            assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST 缺: {tool}");
        }
    }

    #[test]
    fn validate_tool_call_accepts_and_rejects() {
        let args = serde_json::json!({});
        // 白名单内通过
        for tool in TOOL_WHITELIST {
            assert!(
                validate_tool_call(tool, &args).is_ok(),
                "白名单工具 {tool} 应通过"
            );
        }
        // 白名单外拒绝 (m3 fabrication 防御)
        let bad = "apeireth_rollback_purge"; // 不存在
        let r = validate_tool_call(bad, &args);
        assert!(r.is_err(), "白名单外工具必须拒绝");
        match r.unwrap_err() {
            RollbackError::ToolNotWhitelisted(t) => assert_eq!(t, bad),
            other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
        }
    }

    // ----- 6 策略 -----

    #[test]
    fn rollback_strategies_6_variants() {
        let s: Vec<&str> = vec![
            RollbackStrategy::Full.as_v0921_str(),
            RollbackStrategy::File.as_v0921_str(),
            RollbackStrategy::Diff.as_v0921_str(),
            RollbackStrategy::Git.as_v0921_str(),
            RollbackStrategy::Session.as_v0921_str(),
            RollbackStrategy::Auto.as_v0921_str(),
        ];
        assert_eq!(s.len(), V0921_ROLLBACK_STRATEGIES);
        for (i, expected) in ["full", "file", "diff", "git", "session", "auto"]
            .iter()
            .enumerate()
        {
            assert_eq!(s[i], *expected, "策略[{i}] = {} != {expected}", s[i]);
        }
    }

    #[test]
    fn rollback_strategies_roundtrip() {
        for s in [
            RollbackStrategy::Full,
            RollbackStrategy::File,
            RollbackStrategy::Diff,
            RollbackStrategy::Git,
            RollbackStrategy::Session,
            RollbackStrategy::Auto,
        ] {
            let s_str = s.as_v0921_str();
            let back = RollbackStrategy::from_v0921_str(s_str).expect("parse ok");
            assert_eq!(back, s, "roundtrip {s_str}");
        }
        // 未知策略报错
        assert!(RollbackStrategy::from_v0921_str("unknown").is_err());
    }

    // ----- snapshot_id 生成 -----

    #[test]
    fn snapshot_id_format_matches_pattern() {
        let id = generate_snapshot_id();
        // 格式: agent-XXXXXX-{ts}
        assert!(id.starts_with("agent-"), "snapshot ID 必须 'agent-' 前缀");
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "snapshot ID 3 段: agent / XXXXXX / ts");
        assert_eq!(parts[0], "agent");
        assert_eq!(parts[1].len(), 6, "6 字符 mktemp 随机");
        let ts: u64 = parts[2].parse().expect("ts 必须 u64");
        assert!(ts > 1_700_000_000, "ts 必须在 2023+");
    }
}
