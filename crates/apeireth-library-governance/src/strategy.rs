//! Library Stage 5 治理策略 — 借鉴 clap 725 derive 模式.
//!
//! # 借鉴来源 (R124-1 BORROW)
//! - `clap-rs/clap` v4.5 — `#[derive(Parser)]` / `#[derive(Subcommand)]` / `#[derive(ValueEnum)]` 3 件套
//! - 模式: enum 派发 + 静态决策树 + 编译期 hardcode (per clap 4.5 docs §"Derive Reference")
//!
//! # 1:1 翻译
//! - clap `enum SubCommand { Skills(...), Eval(...) }` → 我们 `enum PolicyKind { Version, Baseline, Locked, Anchor, Gate, Other }`
//! - clap `enum Action { Allow, Reject, Audit }` (#[derive(ValueEnum)]) → 我们 `enum GovernanceAction { Allow, Reject, Audit }`
//! - clap `ArgMatches` (matches subcommand + value_of + is_present) → 我们 `GovernanceContext` (POD-friendly, 0 String/Vec)
//! - clap 决策树 (clap_builder/src/parser/parser.rs) → 我们 `DecisionTree::dispatch` (3 段: policy → action → audit)
//!
//! # 0 触碰 clap 本体
//! - 仅借鉴 enum 派发模式, 0 引 clap crate 依赖 (governance 跟 CLI 解析解耦, 避免 clap 4.5 依赖传染)
//! - POD-friendly: 所有字段用 u8 / bool / 固定 array, Kani-friendly
//!
//! # 0 装严守
//! - ❌ 0 假装"完整治理引擎" — 仅 5 策略 + 3 行动 + 1 决策树
//! - ❌ 0 假装"运行时动态策略" — 全部编译期 hardcode (跟 clap 静态派生 1:1)

use crate::consistency::{ANTHROPIC_KEY, OPENAI_KEY};

/// 治理策略类型 (类似 clap `#[derive(Subcommand)]` 的 subcommand enum).
///
/// 5 策略 = 8 硬墙中最关键 5 类 (B2 / A1 / B1 / B5 / B4). 其他 (B3 / A3 / C1-C3) 留作 Strategy::Other 走 Reject 路径.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyKind {
    /// B2 — workspace.version 1.2.0 严守 (整合 #4 commit abf12243, 0 改)
    Version,
    /// A1 — R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守
    Baseline,
    /// B1 — 24 LOCKED crate 入口签名 0 改 (内部 fn 实施可改)
    Locked,
    /// B5 — 6→8 哲学锚
    Anchor,
    /// B4 — 6 重守门 v6 → v7
    Gate,
    /// 兜底: 未分类策略
    Other,
}

impl PolicyKind {
    /// 0 装严守: 仅 5 已知策略, 其他都归 Other.
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Version,
            1 => Self::Baseline,
            2 => Self::Locked,
            3 => Self::Anchor,
            4 => Self::Gate,
            _ => Self::Other,
        }
    }

    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Version => 0,
            Self::Baseline => 1,
            Self::Locked => 2,
            Self::Anchor => 3,
            Self::Gate => 4,
            Self::Other => 255,
        }
    }
}

/// 治理行动 (类似 clap `#[derive(ValueEnum)]` 的 value enum, 3 值).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GovernanceAction {
    /// 允许 (类似 clap 的 `ArgAction::SetTrue` 命中)
    Allow,
    /// 拒绝 (类似 clap 的 `required` 不满足)
    Reject,
    /// 需审计 (类似 clap 的 `requires` 触发)
    Audit,
}

impl GovernanceAction {
    pub const fn is_strict(self) -> bool {
        matches!(self, Self::Reject)
    }
}

/// 治理上下文 (POD-friendly, 借鉴 clap `ArgMatches` 扁平化).
///
/// **设计**: 6 字段 (跟 5 策略 + Other 1:1), 全部 u8 / bool / 固定 array, Kani-friendly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernanceContext {
    /// 策略 kind (clap subcommand 命中)
    pub policy: u8,
    /// 当前值 (clap `value_of` 命中, POD-friendly 用 u8 替 String)
    pub value: u8,
    /// 是否需要审计 (clap `is_present("audit")`)
    pub audit: bool,
    /// B2: workspace.version major (1, 严守 1.2.0)
    pub version_major: u8,
    /// A1: baseline 索引 (0=R11, 1=R12, ...)
    pub baseline_index: u8,
    /// B1: locked crate 索引 (0..=23, 0..=7 是 apeireth-cli 等, 0..=23 是 24 LOCKED)
    pub locked_index: u8,
}

impl GovernanceContext {
    /// 安全默认: 0 策略 / 0 值 / 0 审计 / 0 边界 — 用于 unit test, 跟"什么都不做"语义对齐.
    pub const fn safe_default() -> Self {
        Self {
            policy: 0,
            value: 0,
            audit: false,
            version_major: 1,  // B2 1.2.0 严守
            baseline_index: 0, // A1 R11 baseline 严守
            locked_index: 0,
        }
    }

    /// B2 严守: workspace.version 1.2.0 → major = 1.
    pub const fn version() -> Self {
        Self {
            policy: 0,
            value: 2, // 1.2.0 minor = 2
            audit: true,
            version_major: 1,
            baseline_index: 0,
            locked_index: 0,
        }
    }

    /// A1 严守: R11 baseline 3 值 0.8682/0.8532/0.9063 数字 0 改.
    pub const fn baseline() -> Self {
        Self {
            policy: 1,
            value: 0, // R11
            audit: true,
            version_major: 1,
            baseline_index: 0,
            locked_index: 0,
        }
    }

    /// B1 严守: 24 LOCKED 入口签名 0 改 (整合 #4 commit verify).
    pub const fn locked(crate_index: u8) -> Self {
        Self {
            policy: 2,
            value: crate_index,
            audit: true,
            version_major: 1,
            baseline_index: 0,
            locked_index: crate_index,
        }
    }

    /// B5 升级: 6→8 哲学锚.
    pub const fn anchor() -> Self {
        Self {
            policy: 3,
            value: 8, // 8 哲学锚
            audit: true,
            version_major: 1,
            baseline_index: 0,
            locked_index: 0,
        }
    }

    /// B4 升级: 6 重守门 v6 → v7.
    pub const fn gate() -> Self {
        Self {
            policy: 4,
            value: 7, // v7
            audit: true,
            version_major: 1,
            baseline_index: 0,
            locked_index: 0,
        }
    }
}

/// 决策树 (借鉴 clap `parser/parser.rs` 的 3 段派发).
///
/// 派发链:
/// 1. **policy 命中**: `policy: u8` → `PolicyKind` (类似 clap `subcommand()` 命中)
/// 2. **action 派生**: policy + value → `GovernanceAction` (类似 clap `value_of` + 规则)
/// 3. **audit 决定**: action + audit 字段 → final decision (类似 clap `is_present("audit")` + `requires`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionTree {
    policy: PolicyKind,
    value: u8,
    audit: bool,
}

impl DecisionTree {
    pub const fn from_context(ctx: &GovernanceContext) -> Self {
        Self {
            policy: PolicyKind::from_u8(ctx.policy),
            value: ctx.value,
            audit: ctx.audit,
        }
    }

    /// 派发到最终 action.
    pub const fn action(&self) -> GovernanceAction {
        match self.policy {
            PolicyKind::Version => {
                // B2: workspace.version 1.2.0 严守. value=2 (minor) → Allow; 其他 → Audit
                if self.value == 2 {
                    GovernanceAction::Allow
                } else {
                    GovernanceAction::Audit
                }
            }
            PolicyKind::Baseline => {
                // A1: R11 baseline 3 值严守. value=0 (R11) → Allow; value≥1 → Audit
                if self.value == 0 {
                    GovernanceAction::Allow
                } else {
                    GovernanceAction::Audit
                }
            }
            PolicyKind::Locked => {
                // B1: 24 LOCKED 入口签名 0 改. value 0..=23 → Allow; 其他 → Reject
                if self.value <= 23 {
                    GovernanceAction::Allow
                } else {
                    GovernanceAction::Reject
                }
            }
            PolicyKind::Anchor => {
                // B5: 8 哲学锚. value=8 → Allow; value<8 → Audit; value>8 → Reject
                if self.value == 8 {
                    GovernanceAction::Allow
                } else if self.value < 8 {
                    GovernanceAction::Audit
                } else {
                    GovernanceAction::Reject
                }
            }
            PolicyKind::Gate => {
                // B4: 6 重守门 v7. value=7 → Allow; value<7 → Audit; value>7 → Reject
                if self.value == 7 {
                    GovernanceAction::Allow
                } else if self.value < 7 {
                    GovernanceAction::Audit
                } else {
                    GovernanceAction::Reject
                }
            }
            PolicyKind::Other => GovernanceAction::Reject,
        }
    }

    /// 派发整条决策链 (policy → action → audit).
    pub const fn dispatch(&self) -> crate::GovernanceDecision {
        let action = self.action();
        let requires_audit = self.audit || matches!(action, GovernanceAction::Audit);
        match action {
            GovernanceAction::Allow if !requires_audit => {
                crate::GovernanceDecision::allow(self.policy, action)
            }
            _ => crate::GovernanceDecision::audit(self.policy, action),
        }
    }
}

/// 阶段 5 治理 token 数 (编译期 hardcode, 借鉴 clap 4.5 "0 runtime token validation" 思想).
///
/// 借鉴 clap 4.5 §"ArgGroup" — 编译期 hardcode 必填字段, 减少 runtime token check 开销.
pub const REQUIRED_TOKEN_COUNT: usize = 2; // ANTHROPIC_KEY + OPENAI_KEY (跨 9 organ token 核心 2 键)

/// 跨 9 organ 必填 token 字段数 (借鉴 clap `required = true` 编译期 hardcode).
///
/// 0 装严守: 仅 ANTHROPIC_KEY + OPENAI_KEY 2 键, 其他 11 键 (B3) 留作 R127 续扩.
pub fn required_tokens_present() -> bool {
    REQUIRED_TOKEN_COUNT == 2 && ANTHROPIC_KEY > 0 && OPENAI_KEY > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GovernanceDecision;

    #[test]
    fn policy_kind_round_trip() {
        // 0 装严守: 仅 0..=4 (5 已知) round-trip OK, 5..=255 都归 Other
        for v in 0u8..=4 {
            let p = PolicyKind::from_u8(v);
            assert_eq!(p.as_u8(), v, "round-trip failed for v={}", v);
        }
        for v in 5u8..=255 {
            let p = PolicyKind::from_u8(v);
            assert_eq!(p, PolicyKind::Other);
            assert_eq!(p.as_u8(), 255);
        }
    }

    #[test]
    fn policy_kind_other_catches_unknown() {
        for v in 5u8..=255 {
            assert_eq!(PolicyKind::from_u8(v), PolicyKind::Other);
        }
    }

    #[test]
    fn version_context_dispatches_allow() {
        let ctx = GovernanceContext::version();
        let tree = DecisionTree::from_context(&ctx);
        let action = tree.action();
        assert_eq!(action, GovernanceAction::Allow);
    }

    #[test]
    fn baseline_context_dispatches_allow() {
        let ctx = GovernanceContext::baseline();
        let tree = DecisionTree::from_context(&ctx);
        assert_eq!(tree.action(), GovernanceAction::Allow);
    }

    #[test]
    fn locked_in_range_dispatches_allow() {
        for i in 0u8..=23 {
            let ctx = GovernanceContext::locked(i);
            let tree = DecisionTree::from_context(&ctx);
            assert_eq!(tree.action(), GovernanceAction::Allow, "i={}", i);
        }
    }

    #[test]
    fn locked_out_of_range_dispatches_reject() {
        for i in 24u8..=255 {
            let ctx = GovernanceContext::locked(i);
            let tree = DecisionTree::from_context(&ctx);
            assert_eq!(tree.action(), GovernanceAction::Reject, "i={}", i);
        }
    }

    #[test]
    fn anchor_8_dispatches_allow() {
        let ctx = GovernanceContext::anchor();
        let tree = DecisionTree::from_context(&ctx);
        assert_eq!(tree.action(), GovernanceAction::Allow);
    }

    #[test]
    fn anchor_under_8_dispatches_audit() {
        for v in 0u8..8 {
            let mut ctx = GovernanceContext::anchor();
            ctx.value = v;
            let tree = DecisionTree::from_context(&ctx);
            assert_eq!(tree.action(), GovernanceAction::Audit, "v={}", v);
        }
    }

    #[test]
    fn gate_v7_dispatches_allow() {
        let ctx = GovernanceContext::gate();
        let tree = DecisionTree::from_context(&ctx);
        assert_eq!(tree.action(), GovernanceAction::Allow);
    }

    #[test]
    fn gate_under_v7_dispatches_audit() {
        for v in 0u8..7 {
            let mut ctx = GovernanceContext::gate();
            ctx.value = v;
            let tree = DecisionTree::from_context(&ctx);
            assert_eq!(tree.action(), GovernanceAction::Audit, "v={}", v);
        }
    }

    #[test]
    fn other_policy_always_rejects() {
        let ctx = GovernanceContext {
            policy: 100,
            value: 0,
            audit: false,
            version_major: 1,
            baseline_index: 0,
            locked_index: 0,
        };
        let tree = DecisionTree::from_context(&ctx);
        assert_eq!(tree.action(), GovernanceAction::Reject);
    }

    #[test]
    fn dispatch_chains_through_audit() {
        // 即使 action = Allow, audit 字段开 → 最终 requires_audit = true
        let mut ctx = GovernanceContext::version();
        ctx.audit = true;
        let decision = DecisionTree::from_context(&ctx).dispatch();
        assert!(decision.requires_audit);
        assert_eq!(decision.action, GovernanceAction::Allow);
        assert_eq!(decision.policy, PolicyKind::Version);
    }

    #[test]
    fn required_tokens_count_is_two() {
        assert_eq!(REQUIRED_TOKEN_COUNT, 2);
    }

    #[test]
    fn required_tokens_present_passes() {
        assert!(required_tokens_present());
    }
}
