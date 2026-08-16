//! **战役 2-3 / VCP `toolApprovalManager.js:144-225` — ApprovalDecision 决策枚举**
//!
//! **字段级引用 VCP**:
//! - `getApprovalDecision` 返 `{requiresApproval: bool, notifyAiOnReject: bool, matchedRule: string}` →
//!   我们的 3 态: Allow / RequireApproval{timeout_ms} / Deny{reason, silent}
//! - VCP `SilentReject` 后缀 → `Deny { silent: true }` (拒绝时不通知 AI)
//! - VCP `matchedRule: '__APPROVE_ALL__'` → 我们的 `RequireApproval` 全局强制审批
//!
//! **3 态设计**:
//! - `Allow` — 规则放行, 直接调工具
//! - `RequireApproval { timeout_ms }` — 需主人审批, 5min 窗口 (VCP `timeoutMinutes = 5`)
//! - `Deny { reason, silent }` — 拒绝执行 (silent = 不通知 AI, VCP `::SilentReject`)
//! - `NoMatch` — 当前规则不匹配, 由下条规则判断 (5 规则按顺序)

use serde::Serialize;

/// **战役 2-3 — 审批决策 (3 态 + 1 内部态)**
///
/// **字段级引用 VCP** `toolApprovalManager.js:144-225 getApprovalDecision`:
/// - VCP 返 `{requiresApproval, notifyAiOnReject, matchedRule, matchedCommand}`
/// - 我们返 3 态 enum (更类型安全, 不需要"魔法字符串")
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ApprovalDecision {
    /// **直接通过** — 工具可立即执行
    Allow,
    /// **需要审批** (VCP `requiresApproval: true`)
    ///
    /// `timeout_ms` — 审批超时毫秒 (VCP `getTimeoutMs()` 默认 5 * 60 * 1000)
    RequireApproval {
        /// 等待主人审批的超时 (毫秒), 超时后返 false
        timeout_ms: u64,
    },
    /// **直接拒绝** (VCP `requiresApproval: true` + matchedCommand silent)
    ///
    /// `silent` — true = 拒绝时不通知 AI (VCP `::SilentReject` 后缀语义)
    Deny {
        /// 拒绝原因 (供 log + AI 反馈, silent=false 时用)
        reason: String,
        /// 静默拒绝 (VCP `::SilentReject` 后缀, true = 不通知 AI)
        silent: bool,
    },
    /// **不匹配** — 内部态, 表示当前规则不判断, 由下条规则接手
    ///
    /// 5 规则按顺序检查, 第一个非 `NoMatch` 的决策生效
    /// (Allow / RequireApproval / Deny 直接返, NoMatch 继续)
    NoMatch,
}

// ============================================================
// 结构化审批结果 (toolApprovalManager 增强 P1: 结构化拒绝 {rejected_by_user, error_type})
// ============================================================

/// **结构化拒绝错误码** (VCP `{rejected_by_user, error_type}` 吸收)
///
/// **字段级引用 VCP**: `TOOL_APPROVAL_REASON_PROTOCOL.md` 拒绝反馈字段 +
/// `toolApprovalManager.js getTimeoutMs` (超时 = 拒绝).
///
/// **用途**: 决策器/审批通道解耦 — 消费方不再解析裸错误字符串,
/// 按错误码分支 (AI 重试策略 / 前端提示 / 审计归类).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectErrorType {
    /// 主人明确点拒绝 (审批通道收到 approved=false)
    RejectedByUser,
    /// 审批窗口超时, 主人未响应 → 视为拒绝 (VCP 行为)
    ApprovalTimeout,
    /// 规则直接拒绝 (黑名单 / 频率反刷 / 审批列表), 未经主人交互
    PolicyDeny,
    /// 审批通道不可用 (未注册 handler), 视为拒绝
    ChannelUnavailable,
}

impl RejectErrorType {
    /// 错误码的稳定字符串形式 (供 log / JSON / 审计)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RejectedByUser => "rejected_by_user",
            Self::ApprovalTimeout => "approval_timeout",
            Self::PolicyDeny => "policy_deny",
            Self::ChannelUnavailable => "channel_unavailable",
        }
    }
}

/// **审批请求的匹配明细** (命令级粒度 + 静默标记的载体)
///
/// **字段级引用 VCP**: `getApprovalDecision` 返 `matchedRule / matchedCommand / notifyAiOnReject`
/// → 我们的 `matched_rule / matched_command / silent_on_reject` (typed, 不用魔法字符串).
///
/// **解耦说明**: 决策器 (规则链) 产 `ApprovalDecision` + `CheckDetail`,
/// 审批通道 (handler) 只消费决策; 两者不互相依赖.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct CheckDetail {
    /// 命中的规则名 (VCP `matchedRule`; 无命中 = None)
    pub matched_rule: Option<String>,
    /// 命中的命令级键 (VCP `matchedCommand`; 仅命令级规则命中时有值)
    pub matched_command: Option<String>,
    /// 拒绝时是否静默 (VCP `notifyAiOnReject == false`, 即 `::SilentReject` 语义)
    pub silent_on_reject: bool,
}

/// **一次被拒绝的审批的完整描述** (结构化拒绝, 取代裸错误字符串)
///
/// **字段级引用 VCP**: `{rejected_by_user, error_type}` (本结构为其超集,
/// 额外带 silent / reason / matched_rule / matched_command 供审计与反馈).
///
/// **静默语义**: `silent == true` 时, 该拒绝**不反馈给 AI** (不打扰),
/// 但照常进审计 (`ApprovalManager` audit 留痕) — VCP `::SilentReject` 语义.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rejection {
    /// 是否主人亲自拒绝 (true = 审批通道收到 approved=false; false = 超时/策略/通道缺失)
    pub rejected_by_user: bool,
    /// 结构化错误码
    pub error_type: RejectErrorType,
    /// 静默拒绝标记 (true = 不通知 AI, 仅留痕审计; VCP `::SilentReject`)
    pub silent: bool,
    /// 拒绝原因 (用户填写或规则文案; silent=true 时不得回传给 AI)
    pub reason: Option<String>,
    /// 命中的规则名 (审计用)
    pub matched_rule: Option<String>,
    /// 命中的命令级键 (审计用)
    pub matched_command: Option<String>,
}

/// **结构化审批结果** (批准 / 拒绝二态, 拒绝带完整结构)
///
/// `ApprovalManager::wait_for_approval_outcome` 的返回.
/// 旧 `wait_for_approval` (→ `Result<bool, String>`) 保持不变, 内部委托本类型.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ApprovalOutcome {
    /// 批准执行 (规则放行或主人批准)
    Approved {
        /// 命中的规则名 (无规则命中 = None)
        matched_rule: Option<String>,
        /// 命中的命令级键 (无 = None)
        matched_command: Option<String>,
    },
    /// 拒绝执行 (结构见 `Rejection`)
    Rejected(Rejection),
}

impl ApprovalOutcome {
    /// 是否批准
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }

    /// 是否拒绝
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected(_))
    }

    /// 拒绝结构引用 (批准时为 None)
    pub fn rejection(&self) -> Option<&Rejection> {
        match self {
            Self::Rejected(r) => Some(r),
            _ => None,
        }
    }

    /// 是否为静默拒绝 (拒绝且 silent=true)
    pub fn is_silent_rejection(&self) -> bool {
        matches!(self, Self::Rejected(r) if r.silent)
    }
}

impl ApprovalDecision {
    /// 是否为 `Allow`
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// 是否为 `RequireApproval`
    pub fn is_require_approval(&self) -> bool {
        matches!(self, Self::RequireApproval { .. })
    }

    /// 是否为 `Deny`
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    /// 是否为 `NoMatch` (内部态, 通常不在最终决策中出现)
    pub fn is_no_match(&self) -> bool {
        matches!(self, Self::NoMatch)
    }

    /// 终态决策 (Allow / RequireApproval / Deny, 排除 NoMatch)
    pub fn is_terminal(&self) -> bool {
        !self.is_no_match()
    }

    /// 决策的人类可读字符串 (供 log 用)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::RequireApproval { .. } => "require_approval",
            Self::Deny { silent: false, .. } => "deny",
            Self::Deny { silent: true, .. } => "deny_silent",
            Self::NoMatch => "no_match",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_allow_is_terminal() {
        let d = ApprovalDecision::Allow;
        assert!(d.is_allow());
        assert!(d.is_terminal());
        assert!(!d.is_require_approval());
        assert!(!d.is_deny());
        assert!(!d.is_no_match());
        assert_eq!(d.as_str(), "allow");
    }

    #[test]
    fn decision_require_approval_is_terminal() {
        let d = ApprovalDecision::RequireApproval {
            timeout_ms: 300_000,
        };
        assert!(d.is_require_approval());
        assert!(d.is_terminal());
        assert_eq!(d.as_str(), "require_approval");
    }

    #[test]
    fn decision_deny_variants() {
        let d_normal = ApprovalDecision::Deny {
            reason: "test".to_string(),
            silent: false,
        };
        assert!(d_normal.is_deny());
        assert!(d_normal.is_terminal());
        assert_eq!(d_normal.as_str(), "deny");

        let d_silent = ApprovalDecision::Deny {
            reason: "test".to_string(),
            silent: true,
        };
        assert!(d_silent.is_deny());
        assert!(d_silent.is_terminal());
        assert_eq!(d_silent.as_str(), "deny_silent");
    }

    #[test]
    fn decision_no_match_is_internal() {
        let d = ApprovalDecision::NoMatch;
        assert!(d.is_no_match());
        assert!(!d.is_terminal(), "NoMatch 不是终态");
    }

    #[test]
    fn decision_equality() {
        assert_eq!(ApprovalDecision::Allow, ApprovalDecision::Allow);
        assert_ne!(
            ApprovalDecision::Allow,
            ApprovalDecision::RequireApproval { timeout_ms: 1000 }
        );
        assert_eq!(
            ApprovalDecision::RequireApproval { timeout_ms: 5000 },
            ApprovalDecision::RequireApproval { timeout_ms: 5000 }
        );
    }

    #[test]
    fn decision_serialize_json() {
        // 序列化字段级: VCP getApprovalDecision 返 plain object, 我们也用 JSON-friendly
        let d = ApprovalDecision::RequireApproval {
            timeout_ms: 300_000,
        };
        let s = serde_json::to_string(&d).unwrap();
        assert!(s.contains("RequireApproval"));
        assert!(s.contains("300000"));
    }

    // ====== 结构化结果类型 (toolApprovalManager 增强 P1) ======

    #[test]
    fn reject_error_type_as_str_covers_4_codes() {
        assert_eq!(RejectErrorType::RejectedByUser.as_str(), "rejected_by_user");
        assert_eq!(
            RejectErrorType::ApprovalTimeout.as_str(),
            "approval_timeout"
        );
        assert_eq!(RejectErrorType::PolicyDeny.as_str(), "policy_deny");
        assert_eq!(
            RejectErrorType::ChannelUnavailable.as_str(),
            "channel_unavailable"
        );
    }

    #[test]
    fn check_detail_default_is_neutral() {
        let d = CheckDetail::default();
        assert_eq!(d.matched_rule, None);
        assert_eq!(d.matched_command, None);
        assert!(!d.silent_on_reject);
    }

    #[test]
    fn rejection_struct_carries_vcp_fields() {
        // VCP 结构化拒绝最小契约: rejected_by_user + error_type 必须存在
        let r = Rejection {
            rejected_by_user: true,
            error_type: RejectErrorType::RejectedByUser,
            silent: false,
            reason: Some("风险太高".to_string()),
            matched_rule: Some("approval_list".to_string()),
            matched_command: Some("rm -rf /".to_string()),
        };
        let s = serde_json::to_value(&r).unwrap();
        assert_eq!(s["rejected_by_user"], true);
        assert_eq!(s["error_type"], "rejected_by_user");
        assert_eq!(s["matched_command"], "rm -rf /");
    }

    #[test]
    fn outcome_approved_helpers() {
        let o = ApprovalOutcome::Approved {
            matched_rule: None,
            matched_command: None,
        };
        assert!(o.is_approved());
        assert!(!o.is_rejected());
        assert!(o.rejection().is_none());
        assert!(!o.is_silent_rejection());
    }

    #[test]
    fn outcome_rejected_helpers_and_silent_flag() {
        let loud = ApprovalOutcome::Rejected(Rejection {
            rejected_by_user: true,
            error_type: RejectErrorType::RejectedByUser,
            silent: false,
            reason: None,
            matched_rule: None,
            matched_command: None,
        });
        assert!(loud.is_rejected());
        assert!(loud.rejection().is_some());
        assert!(!loud.is_silent_rejection());

        let silent = ApprovalOutcome::Rejected(Rejection {
            rejected_by_user: false,
            error_type: RejectErrorType::PolicyDeny,
            silent: true,
            reason: Some("黑名单".to_string()),
            matched_rule: Some("blacklist".to_string()),
            matched_command: None,
        });
        assert!(silent.is_silent_rejection());
    }

    #[test]
    fn outcome_serializes_with_tag_and_error_code() {
        let o = ApprovalOutcome::Rejected(Rejection {
            rejected_by_user: false,
            error_type: RejectErrorType::ApprovalTimeout,
            silent: false,
            reason: None,
            matched_rule: None,
            matched_command: None,
        });
        let v = serde_json::to_value(&o).unwrap();
        assert_eq!(v["outcome"], "rejected");
        assert_eq!(v["error_type"], "approval_timeout");

        let a = ApprovalOutcome::Approved {
            matched_rule: Some("trust".to_string()),
            matched_command: None,
        };
        let va = serde_json::to_value(&a).unwrap();
        assert_eq!(va["outcome"], "approved");
        assert_eq!(va["matched_rule"], "trust");
    }
}
