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
}
