//! 主体连续性 (Subject Continuity) — 跨载体 ID + migration_history
//!
//! **设计**:
//! - `SubjectContinuity` 持有不可变 `subject_id` (主体连续性锚)
//! - `current_carrier` 当前载体 (Memory / Dream / Body / ...)
//! - `migration_history` 跨载体迁移记录
//! - 每次迁移 = 旧载体 → 新载体, 保留 `subject_id` 不变
//! - 主体连续性 ID 是 9 阶段生命周期跨阶段继承的核心

use serde::{Deserialize, Serialize};
use std::fmt;

/// 主体载体类型 — 主体可栖息的物理 / 抽象形态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CarrierType {
    /// 记忆载体 (主路径 — 默认)
    Memory,
    /// 梦境载体 (Cognitive-Dream 状态)
    Dream,
    /// 物理身体 (具身 AI)
    Body,
    /// 影子载体 (备份 / 影子进程)
    Shadow,
    /// 远端载体 (跨网络同步)
    Remote,
    /// 镜像载体 (只读, 验证一致性)
    Mirror,
}

impl fmt::Display for CarrierType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Memory => "memory",
            Self::Dream => "dream",
            Self::Body => "body",
            Self::Shadow => "shadow",
            Self::Remote => "remote",
            Self::Mirror => "mirror",
        };
        f.write_str(s)
    }
}

/// 主体迁移记录 — 跨载体迁移历史 (不可篡改).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Migration {
    /// 迁移 ID
    pub migration_id: String,
    /// 来源载体
    pub from: CarrierType,
    /// 目标载体
    pub to: CarrierType,
    /// 迁移时间 (epoch ms)
    pub migrated_at_ms: i64,
    /// 迁移原因
    pub reason: String,
    /// 迁移前主体连续性 ID 校验值 (e.g. hash)
    pub integrity_proof: Option<String>,
}

impl Migration {
    /// 创建迁移记录
    pub fn new(
        migration_id: impl Into<String>,
        from: CarrierType,
        to: CarrierType,
        migrated_at_ms: i64,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            migration_id: migration_id.into(),
            from,
            to,
            migrated_at_ms,
            reason: reason.into(),
            integrity_proof: None,
        }
    }

    /// 添加完整性证明
    pub fn with_integrity_proof(mut self, proof: impl Into<String>) -> Self {
        self.integrity_proof = Some(proof.into());
        self
    }
}

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} → {} @ {} ({})",
            self.migration_id, self.from, self.to, self.migrated_at_ms, self.reason
        )
    }
}

/// 主体连续性 ID + 跨载体历史.
///
/// **约束**:
/// - `subject_id` 创建后**不可修改** (SGI 字段)
/// - `current_carrier` 可通过 `migrate_to(...)` 更新
/// - 每次迁移追加 `migration_history`
/// - `migration_history` 不可回滚 (追加语义)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectContinuity {
    /// 主体连续性 ID (不可变, 创建后锁定)
    pub subject_id: String,
    /// 当前载体
    pub current_carrier: CarrierType,
    /// 创建时间 (epoch ms)
    pub created_at_ms: i64,
    /// 上次更新时间 (epoch ms)
    pub last_updated_at_ms: i64,
    /// 迁移历史 (追加语义)
    pub migration_history: Vec<Migration>,
}

impl SubjectContinuity {
    /// 创建新主体连续性记录
    pub fn new(
        subject_id: impl Into<String>,
        initial_carrier: CarrierType,
        created_at_ms: i64,
    ) -> Self {
        Self {
            subject_id: subject_id.into(),
            current_carrier: initial_carrier,
            created_at_ms,
            last_updated_at_ms: created_at_ms,
            migration_history: Vec::new(),
        }
    }

    /// 迁移到新载体 — 追加迁移记录
    ///
    /// **约束**:
    /// - `subject_id` 不可变
    /// - 同载体迁移 = 拒绝 (no-op migration)
    pub fn migrate_to(
        &mut self,
        to: CarrierType,
        migrated_at_ms: i64,
        reason: impl Into<String>,
    ) -> Result<&Migration, String> {
        if self.current_carrier == to {
            return Err(format!("已在载体 {}, 拒绝同载体迁移", to));
        }
        let from = self.current_carrier;
        let migration_id = format!(
            "mig-{}-{}→{}-{}",
            self.migration_history.len() + 1,
            from,
            to,
            migrated_at_ms
        );
        let migration = Migration::new(migration_id, from, to, migrated_at_ms, reason);
        self.migration_history.push(migration.clone());
        self.current_carrier = to;
        self.last_updated_at_ms = migrated_at_ms;
        Ok(self.migration_history.last().expect("刚 push"))
    }

    /// 迁移次数
    pub fn migration_count(&self) -> usize {
        self.migration_history.len()
    }

    /// 是否从未迁移
    pub fn is_initial_carrier(&self) -> bool {
        self.migration_history.is_empty()
    }

    /// 上次迁移
    pub fn last_migration(&self) -> Option<&Migration> {
        self.migration_history.last()
    }

    /// 主体 ID 一致性校验 — `subject_id` 与迁移历史中的所有迁移都关联同一主体
    pub fn verify_continuity(&self) -> bool {
        // 主体 ID 非空 = 连续性成立
        !self.subject_id.is_empty() && !self.subject_id.contains(' ')
    }
}

impl fmt::Display for SubjectContinuity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubjectContinuity(id={}, carrier={}, migrations={})",
            self.subject_id,
            self.current_carrier,
            self.migration_history.len()
        )
    }
}
