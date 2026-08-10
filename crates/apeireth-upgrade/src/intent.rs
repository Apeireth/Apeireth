//! UpgradeIntent — 升级意图结构 + 状态机 (OTA 阶段 1/7).
//!
//! 升级 Intent 是整个 OTA 管线的"前置契约": 在下载/切换/监控之前, 升级目标、来源、
//! 范围、回滚策略必须先冻结为可审计的结构, 后续 6 阶段(Intent → CouncilReview →
//! MultiSig → Download → Switchover → Monitor → Done/Rollback) 都基于该 intent.
//!
//! Intent 状态机 (5 状态, 与 OtaStage 区分 — Intent 阶段本身的子状态):
//! - Drafting     — 起草中, 内容可变
//! - Submitted    — 已提交智囊团审议, 内容冻结
//! - Approved     — 智囊团通过, 等待多签
//! - Rejected     — 智囊团拒绝 (终态)
//! - Withdrawn    — 主动撤回 (终态)
//!
//! 合法转换:
//! Drafting -> Submitted
//! Submitted -> Approved | Rejected | Withdrawn
//! Approved -> Withdrawn (升级方可撤回, 但多签已开始时不可撤回, 由调用方控制)
//!
//! **禁止**: 不修改 apeireth-core 任何已实装类型签名.

use chrono::Utc;
use uuid::Uuid;

use crate::manifest::UpgradeKind;

/// 升级意图状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentStatus {
    /// 起草中.
    Drafting,
    /// 已提交审议.
    Submitted,
    /// 已通过.
    Approved,
    /// 已拒绝 (终态).
    Rejected,
    /// 已撤回 (终态).
    Withdrawn,
}

impl IntentStatus {
    /// 是否终态 (Rejected / Withdrawn).
    pub fn is_terminal(self) -> bool {
        matches!(self, IntentStatus::Rejected | IntentStatus::Withdrawn)
    }
}

/// 升级范围 (Scope) — 描述升级影响的子模块.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeScope {
    /// 受影响模块路径列表 (如 `["apeireth-memory", "apeireth-cognition"]`).
    pub modules: Vec<String>,
    /// 是否修改 E 层.
    pub touches_e_layer: bool,
    /// 是否需要数据库迁移.
    pub requires_db_migration: bool,
    /// 是否需要数据回填 (backfill).
    pub requires_backfill: bool,
}

impl UpgradeScope {
    /// 构造新范围.
    pub fn new(modules: Vec<String>) -> Self {
        Self {
            modules,
            touches_e_layer: false,
            requires_db_migration: false,
            requires_backfill: false,
        }
    }

    /// 标记涉及 E 层.
    pub fn with_e_layer(mut self) -> Self {
        self.touches_e_layer = true;
        self
    }

    /// 标记需要 DB migration.
    pub fn with_db_migration(mut self) -> Self {
        self.requires_db_migration = true;
        self
    }

    /// 标记需要 backfill.
    pub fn with_backfill(mut self) -> Self {
        self.requires_backfill = true;
        self
    }

    /// 范围是否"高危" (E 层 / DB schema 变更 / backfill).
    pub fn is_high_risk(&self) -> bool {
        self.touches_e_layer || self.requires_db_migration || self.requires_backfill
    }
}

/// 升级意图 — 完整审计契约.
#[derive(Debug, Clone)]
pub struct UpgradeIntent {
    /// Intent ID.
    pub id: Uuid,
    /// 关联 manifest ID (逻辑外键, 不强制 FK).
    pub manifest_id: Uuid,
    /// 目标版本.
    pub target_version: String,
    /// 当前版本 (rollback 基线).
    pub current_version: String,
    /// 升级种类.
    pub kind: UpgradeKind,
    /// 升级范围.
    pub scope: UpgradeScope,
    /// 升级理由.
    pub rationale: String,
    /// 申请人 (carrier id / agent id).
    pub requester: String,
    /// 创建时间戳.
    pub created_at: i64,
    /// 提交时间戳 (Submitted 时填).
    pub submitted_at: Option<i64>,
    /// 当前状态.
    pub status: IntentStatus,
}

impl UpgradeIntent {
    /// 构造新意图 (默认 Drafting).
    pub fn new(
        manifest_id: Uuid,
        target_version: impl Into<String>,
        current_version: impl Into<String>,
        kind: UpgradeKind,
        requester: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            manifest_id,
            target_version: target_version.into(),
            current_version: current_version.into(),
            kind,
            scope: UpgradeScope::new(Vec::new()),
            rationale: rationale.into(),
            requester: requester.into(),
            created_at: Utc::now().timestamp(),
            submitted_at: None,
            status: IntentStatus::Drafting,
        }
    }

    /// 设置 scope.
    pub fn with_scope(mut self, scope: UpgradeScope) -> Self {
        self.scope = scope;
        self
    }

    /// 是否高危 (kind=ELayerMutation 或 scope.is_high_risk).
    pub fn is_high_risk(&self) -> bool {
        matches!(self.kind, UpgradeKind::ELayerMutation) || self.scope.is_high_risk()
    }
}

/// Intent 状态机 — 显式合法转换.
pub struct IntentStateMachine {
    intent: UpgradeIntent,
}

impl IntentStateMachine {
    /// 包装现有 intent.
    pub fn wrap(intent: UpgradeIntent) -> Self {
        Self { intent }
    }

    /// 取当前意图引用.
    pub fn intent(&self) -> &UpgradeIntent {
        &self.intent
    }

    /// 取当前状态.
    pub fn status(&self) -> IntentStatus {
        self.intent.status
    }

    /// 提交审议: Drafting -> Submitted.
    pub fn submit(&mut self) -> Result<(), IntentTransitionError> {
        if self.intent.status != IntentStatus::Drafting {
            return Err(IntentTransitionError::Illegal {
                from: self.intent.status,
                to: IntentStatus::Submitted,
            });
        }
        if self.intent.target_version.is_empty() {
            return Err(IntentTransitionError::Invalid(
                "empty target_version".into(),
            ));
        }
        if self.intent.rationale.is_empty() {
            return Err(IntentTransitionError::Invalid("empty rationale".into()));
        }
        self.intent.status = IntentStatus::Submitted;
        self.intent.submitted_at = Some(Utc::now().timestamp());
        Ok(())
    }

    /// 智囊团通过: Submitted -> Approved.
    pub fn approve(&mut self) -> Result<(), IntentTransitionError> {
        self.transition(IntentStatus::Approved)
    }

    /// 智囊团拒绝: Submitted -> Rejected.
    pub fn reject(&mut self) -> Result<(), IntentTransitionError> {
        self.transition(IntentStatus::Rejected)
    }

    /// 主动撤回: Drafting/Submitted/Approved -> Withdrawn.
    pub fn withdraw(&mut self) -> Result<(), IntentTransitionError> {
        self.transition(IntentStatus::Withdrawn)
    }

    fn transition(&mut self, to: IntentStatus) -> Result<(), IntentTransitionError> {
        let from = self.intent.status;
        let legal = matches!(
            (from, to),
            (IntentStatus::Drafting, IntentStatus::Submitted)
                | (IntentStatus::Submitted, IntentStatus::Approved)
                | (IntentStatus::Submitted, IntentStatus::Rejected)
                | (IntentStatus::Submitted, IntentStatus::Withdrawn)
                | (IntentStatus::Drafting, IntentStatus::Withdrawn)
                | (IntentStatus::Approved, IntentStatus::Withdrawn)
        );
        if !legal {
            return Err(IntentTransitionError::Illegal { from, to });
        }
        self.intent.status = to;
        Ok(())
    }
}

/// Intent 状态机错误.
#[derive(Debug, thiserror::Error)]
pub enum IntentTransitionError {
    /// 非法转换.
    #[error("illegal intent transition: {from:?} -> {to:?}")]
    Illegal {
        /// 当前状态.
        from: IntentStatus,
        /// 目标状态.
        to: IntentStatus,
    },
    /// 参数非法.
    #[error("invalid intent: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_intent() -> UpgradeIntent {
        UpgradeIntent::new(
            Uuid::new_v4(),
            "v1.1.0",
            "v1.0.0",
            UpgradeKind::Patch,
            "carrier-a",
            "fix memory leak",
        )
    }

    #[test]
    fn intent_initial_status_is_drafting() {
        let i = sample_intent();
        assert_eq!(i.status, IntentStatus::Drafting);
        assert!(!i.status.is_terminal());
    }

    #[test]
    fn intent_submit_requires_non_empty_fields() {
        let mut sm = IntentStateMachine::wrap(sample_intent());
        assert!(sm.submit().is_ok());

        let mut bad = IntentStateMachine::wrap(UpgradeIntent::new(
            Uuid::new_v4(),
            "",
            "v1.0.0",
            UpgradeKind::Patch,
            "x",
            "y",
        ));
        assert!(bad.submit().is_err());
    }

    #[test]
    fn intent_full_happy_path() {
        let mut sm = IntentStateMachine::wrap(sample_intent());
        assert_eq!(sm.status(), IntentStatus::Drafting);
        sm.submit().unwrap();
        assert_eq!(sm.status(), IntentStatus::Submitted);
        sm.approve().unwrap();
        assert_eq!(sm.status(), IntentStatus::Approved);
        sm.withdraw().unwrap();
        assert_eq!(sm.status(), IntentStatus::Withdrawn);
        assert!(sm.status().is_terminal());
    }

    #[test]
    fn intent_reject_from_submitted() {
        let mut sm = IntentStateMachine::wrap(sample_intent());
        sm.submit().unwrap();
        sm.reject().unwrap();
        assert_eq!(sm.status(), IntentStatus::Rejected);
        assert!(sm.status().is_terminal());
    }

    #[test]
    fn intent_illegal_double_submit() {
        let mut sm = IntentStateMachine::wrap(sample_intent());
        sm.submit().unwrap();
        assert!(sm.submit().is_err());
    }

    #[test]
    fn intent_illegal_approve_from_drafting() {
        let mut sm = IntentStateMachine::wrap(sample_intent());
        assert!(sm.approve().is_err());
    }

    #[test]
    fn intent_illegal_revoke_terminal() {
        let mut sm = IntentStateMachine::wrap(sample_intent());
        sm.submit().unwrap();
        sm.reject().unwrap();
        // 已 Rejected, 不能再次 approve / withdraw
        assert!(sm.approve().is_err());
        assert!(sm.withdraw().is_err());
    }

    #[test]
    fn intent_scope_high_risk_flags() {
        let s = UpgradeScope::new(vec!["apeireth-core".into()])
            .with_e_layer()
            .with_db_migration();
        assert!(s.is_high_risk());
        assert!(s.touches_e_layer);
        assert!(s.requires_db_migration);
        assert!(!s.requires_backfill);

        let safe = UpgradeScope::new(vec!["apeireth-bench".into()]);
        assert!(!safe.is_high_risk());
    }

    #[test]
    fn intent_is_high_risk_for_e_layer_kind() {
        let i = UpgradeIntent::new(
            Uuid::new_v4(),
            "v2.0.0",
            "v1.0.0",
            UpgradeKind::ELayerMutation,
            "carrier",
            "modify E layer",
        );
        assert!(i.is_high_risk());

        let patch = UpgradeIntent::new(
            Uuid::new_v4(),
            "v1.0.1",
            "v1.0.0",
            UpgradeKind::Patch,
            "carrier",
            "patch",
        );
        assert!(!patch.is_high_risk());
    }
}
