//! 3 生命周期 (persistent / ephemeral / dynamic)
//!
//! **设计**:
//! - `Persistent`: 永久在场 (e.g. safety / philosophy / ethics / legal — 7 强制)
//! - `Ephemeral`: 短期在场 (按 query 召唤, query 结束自动注销)
//! - `Dynamic`: 按需召集 (异步生成 + 缓存复用)

use crate::advisor::AdvisorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Advisor 生命周期模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorLifecycle {
    /// 永久在场 — 7 强制 advisor 的默认模式
    Persistent,
    /// 短期 — query 结束自动注销
    Ephemeral,
    /// 按需 — 异步生成 + 缓存复用
    Dynamic,
}

impl fmt::Display for AdvisorLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Persistent => "persistent",
            Self::Ephemeral => "ephemeral",
            Self::Dynamic => "dynamic",
        };
        f.write_str(name)
    }
}

/// 生命周期管理 — 跟踪 ephemeral / dynamic 成员的创建与销毁。
#[derive(Debug, Default)]
pub struct LifecycleManager {
    /// ephemeral 成员: session_id → advisor_id
    ephemeral: HashMap<String, Vec<AdvisorId>>,
    /// dynamic 缓存: advisor_id → 命中次数
    dynamic_cache: HashMap<AdvisorId, u64>,
    /// 统计
    spawned_ephemeral: u64,
    spawned_dynamic: u64,
    destroyed_ephemeral: u64,
}

impl LifecycleManager {
    /// 创建新管理器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册 ephemeral advisor (session-bound)。
    pub fn register_ephemeral(&mut self, session_id: impl Into<String>, advisor_id: AdvisorId) {
        self.ephemeral
            .entry(session_id.into())
            .or_default()
            .push(advisor_id);
        self.spawned_ephemeral += 1;
    }

    /// 查询结束 → 销毁 session 的所有 ephemeral advisor。
    pub fn end_session(&mut self, session_id: &str) -> usize {
        let removed = self
            .ephemeral
            .remove(session_id)
            .map(|v| v.len())
            .unwrap_or(0);
        self.destroyed_ephemeral += removed as u64;
        removed
    }

    /// 缓存查询 (dynamic advisor)。
    pub fn cache_hit(&mut self, advisor_id: &AdvisorId) -> bool {
        let entry = self.dynamic_cache.entry(advisor_id.clone()).or_insert(0);
        *entry += 1;
        true
    }

    /// 当前活跃 ephemeral 成员数。
    pub fn active_ephemeral(&self) -> usize {
        self.ephemeral.values().map(|v| v.len()).sum()
    }

    /// 缓存大小 (dynamic advisor)。
    pub fn dynamic_cache_size(&self) -> usize {
        self.dynamic_cache.len()
    }
}

/// 生命周期统计 (synthesis 报告用)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleStats {
    /// persistent 成员数 (7 强制固定为 7)
    pub persistent: usize,
    /// ephemeral 累计创建
    pub ephemeral_spawned: u64,
    /// ephemeral 累计销毁
    pub ephemeral_destroyed: u64,
    /// dynamic 缓存命中数
    pub dynamic_cache_hits: u64,
}

impl LifecycleStats {
    /// 从 manager 提取统计。
    pub fn from_manager(manager: &LifecycleManager, persistent_count: usize) -> Self {
        Self {
            persistent: persistent_count,
            ephemeral_spawned: manager.spawned_ephemeral,
            ephemeral_destroyed: manager.destroyed_ephemeral,
            dynamic_cache_hits: manager.dynamic_cache_cache_total(),
        }
    }
}

impl LifecycleManager {
    /// dynamic 缓存总命中 (供 [`LifecycleStats`] 使用)。
    pub fn dynamic_cache_cache_total(&self) -> u64 {
        self.dynamic_cache.values().sum()
    }
}
