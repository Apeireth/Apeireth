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
