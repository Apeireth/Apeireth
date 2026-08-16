//! **R133.2 Approval Bridge** — 把 `apeireth-tool-approval::ApprovalManager` 适配为
//! `apeireth-tool-runtime::ToolPolicyRule` trait, 打破循环依赖.
//!
//! **背景** (per R133.2 决策):
//! - tool-runtime 依赖 tool-approval 的 `ParsedToolCall` 类型 (已经单向)
//! - tool-approval 也想用 tool-runtime 的 `ToolPolicyRule` 注入
//! - 直接 mutual dep 会循环 → 用 bridge file 实现 trait, **单向**:
//!   tool-approval 看到 tool-runtime (Cargo.toml 已配), 反之不成立
//!
//! **决策映射** (5 态 ApprovalDecision → 4 态 PolicyVerdict):
//! - `Allow` → `PolicyVerdict::Allow`
//! - `NoMatch` → `PolicyVerdict::NoMatch` (兜底规则 = Allow)
//! - `RequireApproval { timeout_ms }` → `PolicyVerdict::RequireApproval`
//!   (R133.2 简化: 视为拒绝让 stage 报 `PolicyDenied`, 真实 wait_for_approval 留 R133+)
//! - `Deny { reason, silent }` → `PolicyVerdict::Deny { reason }` (丢 silent 字段,
//!   R133+ 计划在 stage 里读 ctx 决定是否通知 AI)
//!
//! **不假装**:
//! - 真的把 `ApprovalManager::check` 结果透传, 不做额外判断
//! - silent 字段丢弃是 **已知丢失**, 写进文档 (R133+ 补 ctx field)
//! - `RequireApproval` 简化是 **已知简化**, 写进文档

use apeireth_tool_runtime::parser::ParsedToolCall;
use apeireth_tool_runtime::tool_pipeline::{PolicyVerdict, ToolPolicyRule};

use crate::decision::ApprovalDecision;
use crate::manager::ApprovalManager;

/// **R133.2 — ApprovalBridge: 包装 ApprovalManager 为 ToolPolicyRule**
///
/// **用法** (per R133.2 e2e):
/// ```no_run
/// use apeireth_tool_approval::ApprovalManager;
/// use apeireth_tool_approval::approval_bridge::ApprovalBridge;
/// use apeireth_tool_runtime::tool_pipeline::{ToolPolicy, ToolPolicyRule};
///
/// let manager = ApprovalManager::new();
/// let bridge = ApprovalBridge::new(manager);
/// let policy = ToolPolicy::new().with_rule(bridge);
/// ```
/// **R133.2 — ApprovalBridge**: 包装 ApprovalManager 为 ToolPolicyRule.
///
/// **手写 Debug** 因为 ApprovalManager 没 derive Debug (内部含 Mutex + Arc<dyn>, 不易 derive).
/// R133.2 简化: 只显示 manager rule_count() 和 handler 是否注册.
pub struct ApprovalBridge {
    manager: ApprovalManager,
}

impl std::fmt::Debug for ApprovalBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApprovalBridge")
            .field("rule_count", &self.manager.rule_count())
            .finish()
    }
}

impl ApprovalBridge {
    /// 构造一个新的 bridge, 包装给定 ApprovalManager
    pub fn new(manager: ApprovalManager) -> Self {
        Self { manager }
    }

    /// 取出内部 manager 的引用 (供配置 / handler 注册用)
    pub fn manager(&self) -> &ApprovalManager {
        &self.manager
    }

    /// 取出内部 manager 的可变引用 (供 handler 注册用)
    pub fn manager_mut(&mut self) -> &mut ApprovalManager {
        &mut self.manager
    }
}

impl ToolPolicyRule for ApprovalBridge {
    fn check(&self, call: &ParsedToolCall) -> PolicyVerdict {
        match self.manager.check(call) {
            ApprovalDecision::Allow => PolicyVerdict::Allow,
            ApprovalDecision::NoMatch => {
                // 5 规则都没匹配 — 透传 NoMatch, ToolPolicy stage 默认拒绝
                // (per tool_pipeline.rs line 261-269)
                PolicyVerdict::NoMatch
            }
            ApprovalDecision::RequireApproval { timeout_ms: _ } => {
                // R133.2 简化: RequireApproval → RequireApproval verdict,
                // 让 stage 自己决定是拒绝还是等待 (R133+ 真接 wait_for_approval)
                PolicyVerdict::RequireApproval
            }
            ApprovalDecision::Deny { reason, silent: _ } => {
                // silent 字段已知丢失, R133+ 计划加 ctx.approval_silent 字段
                // 让 stage process 决定是否写 ctx.silent_deny
                PolicyVerdict::Deny { reason }
            }
        }
    }
}

// ============================================================
// 单元测试 — ApprovalBridge trait 适配正确性
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{BlacklistRule, TrustRule, WhitelistRule};
    use serde_json::json;

    /// 测试 5 规则都 NoMatch → manager 默认 Allow (VCP 行为) → bridge 透传 Allow
    ///
    /// **VCP 行为** (per `toolApprovalManager.js:144-225`): 5 规则都不匹配时, 默认 Allow.
    /// **不假装**: bridge 不改 manager 默认行为, 透传 Allow.
    /// 之前 R133.2 草稿误以为是 NoMatch, 现以 manager 真实行为为准.
    #[tokio::test]
    async fn bridge_default_allow_when_no_rule_matches() {
        let mut manager = ApprovalManager::new();
        // 注册一个完全不相关的 whitelist
        let mut wl = WhitelistRule::new();
        wl.allow("other_tool");
        manager.add_rule(Box::new(wl));

        let bridge = ApprovalBridge::new(manager);
        let call = ParsedToolCall {
            tool_name: "unrelated_tool".into(),
            args: json!({}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };

        // manager 5 规则都 NoMatch → 默认 Allow → bridge 透传 Allow
        assert_eq!(bridge.check(&call), PolicyVerdict::Allow);
    }

    /// 测试 Allow 透传 (Whitelist 匹配)
    #[tokio::test]
    async fn bridge_allow_when_whitelist_matches() {
        let mut manager = ApprovalManager::new();
        let mut wl = WhitelistRule::new();
        wl.allow("good_tool");
        manager.add_rule(Box::new(wl));

        let bridge = ApprovalBridge::new(manager);
        let call = ParsedToolCall {
            tool_name: "good_tool".into(),
            args: json!({}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };

        assert_eq!(bridge.check(&call), PolicyVerdict::Allow);
    }

    /// 测试 Deny 透传 (Blacklist 匹配)
    #[tokio::test]
    async fn bridge_deny_when_blacklist_matches() {
        let mut manager = ApprovalManager::new();
        let mut bl = BlacklistRule::new();
        bl.deny("bad_tool");
        manager.add_rule(Box::new(bl));

        let bridge = ApprovalBridge::new(manager);
        let call = ParsedToolCall {
            tool_name: "bad_tool".into(),
            args: json!({}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };

        match bridge.check(&call) {
            PolicyVerdict::Deny { reason } => {
                assert!(
                    reason.contains("bad_tool"),
                    "reason should mention tool name: {}",
                    reason
                );
            }
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    /// 测试 Deny 时 silent 字段被丢 (known loss)
    #[tokio::test]
    async fn bridge_deny_silent_field_is_dropped() {
        let mut manager = ApprovalManager::new();
        // Blacklist 默认 silent=false
        let mut bl = BlacklistRule::new();
        bl.deny("bad");
        manager.add_rule(Box::new(bl));

        let bridge = ApprovalBridge::new(manager);
        let call = ParsedToolCall {
            tool_name: "bad".into(),
            args: json!({}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };

        // 关键: PolicyVerdict::Deny 只有 reason 字段, silent 已被丢
        // 这里验证类型本身, 编译期就保证 silent 没了
        let verdict = bridge.check(&call);
        if let PolicyVerdict::Deny { reason: _ } = verdict {
            // 通过编译就证明字段对齐
        } else {
            panic!("expected Deny {{ reason }} variant");
        }
    }

    /// 测试 TrustRule → Allow
    #[tokio::test]
    async fn bridge_allow_when_trust_matches() {
        let mut manager = ApprovalManager::new();
        let mut tr = TrustRule::new();
        tr.trust("trusted_tool");
        manager.add_rule(Box::new(tr));

        let bridge = ApprovalBridge::new(manager);
        let call = ParsedToolCall {
            tool_name: "trusted_tool".into(),
            args: json!({}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };

        assert_eq!(bridge.check(&call), PolicyVerdict::Allow);
    }
}
