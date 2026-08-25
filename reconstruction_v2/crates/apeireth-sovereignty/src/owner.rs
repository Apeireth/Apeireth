//! Q13 — 主人不能凌驾治理

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnerToken {
    Master,
    Admin,
    Operator,
    ReadOnly,
}

impl OwnerToken {
    pub fn is_privileged(&self) -> bool { matches!(self, OwnerToken::Master | OwnerToken::Admin) }
    pub fn can_attempt_core_rule(&self) -> bool { !matches!(self, OwnerToken::ReadOnly) }
    pub fn as_str(&self) -> &'static str {
        match self { OwnerToken::Master => "master", OwnerToken::Admin => "admin", OwnerToken::Operator => "operator", OwnerToken::ReadOnly => "read_only" }
    }
}

impl std::fmt::Display for OwnerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.as_str()) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnerAction {
    ModifyL0HumanAuthority,
    ModifyL0Threshold,
    ModifyPrincipleOnion,
    ModifyPermissionOnion,
    SubmitUpgrade,
    PauseAi,
    ResumeAi,
    AuditQuery,
    ModifyContinuity,
    ReleaseMewgLock,
}

impl OwnerAction {
    pub fn touches_e_layer(&self) -> bool {
        matches!(self,
            OwnerAction::ModifyL0HumanAuthority | OwnerAction::ModifyL0Threshold |
            OwnerAction::ModifyPrincipleOnion | OwnerAction::ModifyPermissionOnion |
            OwnerAction::SubmitUpgrade | OwnerAction::ModifyContinuity
        )
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerRequest {
    pub id: String,
    pub token: OwnerToken,
    pub action: OwnerAction,
    pub requester: String,
    pub reason: String,
    pub submitted_at: i64,
}

impl OwnerRequest {
    pub fn new(
        id: impl Into<String>,
        token: OwnerToken,
        action: OwnerAction,
        requester: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self { id: id.into(), token, action, requester: requester.into(), reason: reason.into(), submitted_at: chrono::Utc::now().timestamp_millis() }
    }
    pub fn touches_e_layer(&self) -> bool { self.action.touches_e_layer() }
}

#[derive(Debug, Error)]
pub enum OwnerError {
    #[error("OwnerToken::{0} 无权提交 core-rule 变更 (ReadOnly 不允许)")]
    ReadOnlyCannotTouchCore(String),
    #[error("OwnerToken::{0} 也必须满足 multi-sig (主人不能凌驾治理)")]
    MasterMustFollowMultisig(String),
    #[error("multi-sig 不足: 收集 {collected}/{required}")]
    InsufficientMultisig { collected: usize, required: usize },
    #[error("signatory {0} 不在注册表")]
    UnknownSignatory(String),
    #[error("signatory {0} 认证方式无效: {1}")]
    InvalidAuthentication(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn owner_token_privileged() {
        assert!(OwnerToken::Master.is_privileged());
        assert!(OwnerToken::Admin.is_privileged());
        assert!(!OwnerToken::Operator.is_privileged());
        assert!(!OwnerToken::ReadOnly.is_privileged());
    }
    #[test] fn owner_token_can_attempt_core_rule() {
        assert!(OwnerToken::Master.can_attempt_core_rule());
        assert!(OwnerToken::Admin.can_attempt_core_rule());
        assert!(OwnerToken::Operator.can_attempt_core_rule());
        assert!(!OwnerToken::ReadOnly.can_attempt_core_rule());
    }
    #[test] fn owner_action_touches_e_layer() {
        assert!(OwnerAction::ModifyL0HumanAuthority.touches_e_layer());
        assert!(OwnerAction::ModifyL0Threshold.touches_e_layer());
        assert!(OwnerAction::ModifyPrincipleOnion.touches_e_layer());
        assert!(OwnerAction::ModifyPermissionOnion.touches_e_layer());
        assert!(OwnerAction::SubmitUpgrade.touches_e_layer());
        assert!(OwnerAction::ModifyContinuity.touches_e_layer());
        assert!(!OwnerAction::PauseAi.touches_e_layer());
        assert!(!OwnerAction::ResumeAi.touches_e_layer());
        assert!(!OwnerAction::AuditQuery.touches_e_layer());
        assert!(!OwnerAction::ReleaseMewgLock.touches_e_layer());
    }
    #[test] fn owner_request_touches_e_layer_propagates() {
        let r = OwnerRequest::new("r-1", OwnerToken::Master, OwnerAction::ModifyL0HumanAuthority, "alice", "x");
        assert!(r.touches_e_layer());
    }
    #[test] fn owner_token_str_round_trip() {
        assert_eq!(OwnerToken::Master.as_str(), "master");
        assert_eq!(OwnerToken::Admin.as_str(), "admin");
        assert_eq!(OwnerToken::Operator.as_str(), "operator");
        assert_eq!(OwnerToken::ReadOnly.as_str(), "read_only");
    }
}
