//! Q13 — 主人不能凌驾治理 (OwnerToken + 强制 multi-sig)
//!
//! **硬约束** (P25 Q13 LOCKED):
//! 1. Master 也必须走 multi-sig — 不能凌驾治理
//! 2. 任何 token 改 core-rule (E 层) 都必须触发 MEWG + 反思期
//! 3. SovereigntyHook 不允许加 bypass 路径给 Master
//! 4. ReadOnly token 提交 core-rule change → 立即拒绝
//! 5. Operator token 提交 core-rule change → 走 5 重治理 (MEWG + 反思期)
//! 6. Admin token 提交 core-rule change → 走 5 重治理 (MEWG + 反思期)
//! 7. Master token 提交 core-rule change → 走 5 重治理 (MEWG + 反思期) — 与上述一致
//!
//! **架构**:
//! ```text
//!   OwnerToken (Master/Admin/Operator/ReadOnly)
//!      ↓
//!   OwnerDecision { token, action, touches_e_layer, ... }
//!      ↓
//!   MultiSigPolicy::process_owner_request  ← token 校验 + 强制 multi-sig
//!      ↓
//!   Governance::process_owner_decision     ← 5 重治理 (无 bypass)
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 主人令牌 — 4 级权限 (Q13 LOCKED)
///
/// **关键**: Master 仍是 token — **不能凌驾治理**。 Master token 也必须经过
/// `MultiSigPolicy::process_owner_request` 验证 + `Governance.process_owner_decision`
/// 5 重治理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnerToken {
    /// 最高权限 — 主人。 但仍必须走 multi-sig + 5 重治理。
    Master,
    /// 高级权限 — 副主 / 联合主。
    Admin,
    /// 操作权限 — 日常运维。
    Operator,
    /// 只读权限 — 审计 / 观察。
    ReadOnly,
}

impl OwnerToken {
    /// 是否为最高权限 (Master / Admin) — 仅用于审计
    pub fn is_privileged(&self) -> bool {
        matches!(self, OwnerToken::Master | OwnerToken::Admin)
    }

    /// 是否能提交 core-rule (E 层) 变更
    ///
    /// **Q13 硬约束**: Master / Admin / Operator 都可以尝试, ReadOnly 不行。
    /// 但任何尝试都必须走 5 重治理 (MultiSigPolicy + Governance.process)。
    pub fn can_attempt_core_rule(&self) -> bool {
        !matches!(self, OwnerToken::ReadOnly)
    }

    /// 序列化字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            OwnerToken::Master => "master",
            OwnerToken::Admin => "admin",
            OwnerToken::Operator => "operator",
            OwnerToken::ReadOnly => "read_only",
        }
    }
}

impl std::fmt::Display for OwnerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 主人动作类型 — 用于 owner request 中的 action 字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnerAction {
    /// 修改 L0 HA (human authority)
    ModifyL0HumanAuthority,
    /// 修改 L0 HA 阈值 (MultiSigPolicy.required)
    ModifyL0Threshold,
    /// 修改原则洋葱 (Principle Onion)
    ModifyPrincipleOnion,
    /// 修改权限洋葱 (Permission Onion)
    ModifyPermissionOnion,
    /// 提交升级 (Patch / Minor / Major / Emergency)
    SubmitUpgrade,
    /// 暂停 AI (self-pause)
    PauseAi,
    /// 恢复 AI (resume)
    ResumeAi,
    /// 审计查询 (read-only)
    AuditQuery,
    /// 修改人类连续性 (subject continuity)
    ModifyContinuity,
    /// 释放 MEWG 锁定 (release MEWG lock) — 仅 Master
    ReleaseMewgLock,
}

impl OwnerAction {
    /// 是否触及 core-rule (E 层)
    ///
    /// **Q13 硬约束**: 任何 `touches_e_layer=true` 的请求必须走 5 重治理
    /// (MultiSigPolicy + Governance.process_owner_decision)。
    pub fn touches_e_layer(&self) -> bool {
        matches!(
            self,
            OwnerAction::ModifyL0HumanAuthority
                | OwnerAction::ModifyL0Threshold
                | OwnerAction::ModifyPrincipleOnion
                | OwnerAction::ModifyPermissionOnion
                | OwnerAction::SubmitUpgrade
                | OwnerAction::ModifyContinuity
        )
    }

    /// 序列化
    pub fn as_str(&self) -> &'static str {
        match self {
            OwnerAction::ModifyL0HumanAuthority => "modify_l0_human_authority",
            OwnerAction::ModifyL0Threshold => "modify_l0_threshold",
            OwnerAction::ModifyPrincipleOnion => "modify_principle_onion",
            OwnerAction::ModifyPermissionOnion => "modify_permission_onion",
            OwnerAction::SubmitUpgrade => "submit_upgrade",
            OwnerAction::PauseAi => "pause_ai",
            OwnerAction::ResumeAi => "resume_ai",
            OwnerAction::AuditQuery => "audit_query",
            OwnerAction::ModifyContinuity => "modify_continuity",
            OwnerAction::ReleaseMewgLock => "release_mewg_lock",
        }
    }
}

/// 主人请求 — OwnerToken + Action + Reason
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerRequest {
    /// 唯一 ID
    pub id: String,
    /// 主人令牌
    pub token: OwnerToken,
    /// 动作类型
    pub action: OwnerAction,
    /// 发起人 ID (人类 ID / AI ID)
    pub requester: String,
    /// 请求理由
    pub reason: String,
    /// 提交时间 (epoch ms)
    pub submitted_at: i64,
}

impl OwnerRequest {
    /// 构造主人请求
    pub fn new(
        id: impl Into<String>,
        token: OwnerToken,
        action: OwnerAction,
        requester: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            token,
            action,
            requester: requester.into(),
            reason: reason.into(),
            submitted_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 是否触及 E 层
    pub fn touches_e_layer(&self) -> bool {
        self.action.touches_e_layer()
    }
}

/// 主人请求校验错误
#[derive(Debug, Error)]
pub enum OwnerError {
    /// ReadOnly token 提交 core-rule 变更 — 立即拒绝
    #[error("OwnerToken::{0} 无权提交 core-rule 变更 (ReadOnly 不允许)")]
    ReadOnlyCannotTouchCore(String),
    /// Master 也不能凌驾 — multi-sig 不足
    #[error("OwnerToken::{0} 也必须满足 multi-sig (主人不能凌驾治理)")]
    MasterMustFollowMultisig(String),
    /// Multi-signature 不足 (适用于所有 token, 包括 Master)
    #[error("multi-sig 不足: 收集 {collected}/{required}")]
    InsufficientMultisig {
        /// 已收集签名数
        collected: usize,
        /// 所需签名数
        required: usize,
    },
    /// Signatory 不在 MultiSigPolicy 注册表中
    #[error("signatory {0} 不在注册表")]
    UnknownSignatory(String),
    /// 认证方式无效 (e.g. Online-only signatory 但 offline)
    #[error("signatory {0} 认证方式无效: {1}")]
    InvalidAuthentication(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_token_master_is_privileged() {
        assert!(OwnerToken::Master.is_privileged());
        assert!(OwnerToken::Admin.is_privileged());
        assert!(!OwnerToken::Operator.is_privileged());
        assert!(!OwnerToken::ReadOnly.is_privileged());
    }

    #[test]
    fn owner_token_can_attempt_core_rule() {
        assert!(OwnerToken::Master.can_attempt_core_rule());
        assert!(OwnerToken::Admin.can_attempt_core_rule());
        assert!(OwnerToken::Operator.can_attempt_core_rule());
        assert!(!OwnerToken::ReadOnly.can_attempt_core_rule());
    }

    #[test]
    fn owner_action_touches_e_layer_classification() {
        // 触及 E 层
        assert!(OwnerAction::ModifyL0HumanAuthority.touches_e_layer());
        assert!(OwnerAction::ModifyL0Threshold.touches_e_layer());
        assert!(OwnerAction::ModifyPrincipleOnion.touches_e_layer());
        assert!(OwnerAction::ModifyPermissionOnion.touches_e_layer());
        assert!(OwnerAction::SubmitUpgrade.touches_e_layer());
        assert!(OwnerAction::ModifyContinuity.touches_e_layer());
        // 不触及 E 层
        assert!(!OwnerAction::PauseAi.touches_e_layer());
        assert!(!OwnerAction::ResumeAi.touches_e_layer());
        assert!(!OwnerAction::AuditQuery.touches_e_layer());
        assert!(!OwnerAction::ReleaseMewgLock.touches_e_layer());
    }

    #[test]
    fn owner_request_touches_e_layer_propagates() {
        let req = OwnerRequest::new(
            "r-1",
            OwnerToken::Master,
            OwnerAction::ModifyL0HumanAuthority,
            "alice",
            "critical fix",
        );
        assert!(req.touches_e_layer());
    }

    #[test]
    fn owner_token_str_round_trip() {
        assert_eq!(OwnerToken::Master.as_str(), "master");
        assert_eq!(OwnerToken::Admin.as_str(), "admin");
        assert_eq!(OwnerToken::Operator.as_str(), "operator");
        assert_eq!(OwnerToken::ReadOnly.as_str(), "read_only");
    }
}
