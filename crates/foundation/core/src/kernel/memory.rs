//! `apeireth-core::memory` — 主路径核心类型 (R11 Episode/Note/Session/IdentityCard)
//!
//! 拆自 `lib.rs` line 22-91 (R131 架构债清理).
//! 0 触碰公开签名 — `use apeireth_core::Episode` 等不破坏 (lib.rs `pub use memory::*`).
//!
//! 包含:
//! - Episode: 一次对话/事件 (append-only)
//! - Note: 从 Episode 提炼的知识
//! - Session: 一次完整对话周期
//! - IdentityCard: 主体连续性 ID (跨载体唯一)
//! - Migration: 跨载体迁移事件

use serde::{Deserialize, Serialize};

/// Episode: 一次对话/事件 (append-only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// 唯一 episode ID
    pub id: String,
    /// 事件时间戳 (epoch seconds)
    pub timestamp: i64,
    /// 角色 ("user" / "assistant" / "system")
    pub role: String,
    /// 对话内容
    pub content: String,
    /// 所属 session ID
    pub session_id: String,
}

/// Note: 从 Episode 提炼的知识 (可更新/合并/遗忘)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// 唯一 note ID
    pub id: String,
    /// 提炼时间戳
    pub timestamp: i64,
    /// 知识内容
    pub content: String,
    /// 来源 episode IDs
    pub source_episode_ids: Vec<String>,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 标签 (用于检索)
    pub tags: Vec<String>,
}

/// Session: 一次完整对话周期
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 唯一 session ID
    pub id: String,
    /// 启动时间戳
    pub started_at: i64,
    /// 最后活跃时间戳
    pub last_active_at: i64,
}

/// IdentityCard: 主体连续性 ID (跨载体唯一)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCard {
    /// 跨载体唯一 ID (DID + 单调版本号 + 物理多签)
    pub continuity_id: String,
    /// 诞生时间戳
    pub birth_time: i64,
    /// 当前所在载体列表 (跨载体)
    pub carriers: Vec<String>,
    /// 跨载体迁移历史
    pub migration_history: Vec<Migration>,
}

/// 跨载体迁移事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// 源载体 ID
    pub from_carrier: String,
    /// 目标载体 ID
    pub to_carrier: String,
    /// 迁移时间戳
    pub timestamp: i64,
}

/// HistoryEntry: 6 历史流条目 (per stream_kind::StreamKind canonical)
///
/// **位置** (O-6 锚 #18 兑现, 2026-08-27): canonical 在 `apeireth_core::kernel::memory::HistoryEntry`,
/// memory crate 通过 `pub use` re-export 保持 v1 compat. plugin::MemoryBackend trait method
/// `append_stream` / `list_stream` 用 typed struct 替代 `serde_json::Value` 占位.
///
/// **LOCKED**: 字段不变 (D2 §5.3 #2 强制: subject_id + subject_rev 必填),
/// 序列化兼容 (serde derive 不变, JSON 跨进程 0 异常).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    /// 条目 id
    pub id: String,
    /// 主体 ID (D2 §5.3 #2 必填)
    pub subject_id: String,
    /// 主体版本号 (D2 §5.3 #2 必填)
    pub subject_rev: i64,
    /// 可选 session 关联
    pub session_id: Option<String>,
    /// 创建时间 (unix seconds)
    pub created_at: i64,
    /// 自由结构化 payload (JSON 序列化)
    pub payload: serde_json::Value,
    /// 来源 (`ai_generated` / `human_overridden` / `council_synthesized`)
    pub source: String,
    /// 标签
    pub tags: Vec<String>,
    /// 软删除标记; `None` = 未删除
    pub tombstoned_at: Option<i64>,
}
