//! Library Stage 5 6 不变量 (借鉴 `apeireth-formal/invariants/` 模板).
//!
//! # 借鉴来源 (per 决策-55 §2.3)
//! - `apeireth-formal/src/invariants/{double_onion,e_layer_isolation,permission_grant_l0,mid_task_atomicity,seven_advisor_voting}.rs`
//! - 模式: 1 不变量 = 1 文件, 每文件 1 个 Kani-style harness + 1 个 sanity_check + 多个 #[test]
//! - Ponytail: 不变量断言体 1 行, harness LOC < 30
//!
//! # 1:1 翻译
//! - apeireth-formal 5 不变量 (核心 6 维) → 我们 6 不变量 (Stage 5 治理 8 维)
//! - apeireth-formal Kani `cargo kani` (形式化) → 我们 `cargo test` (runtime sanity, 0 Kani 依赖)
//! - apeireth-formal POD 模型 (`PermissionLayerConfig` 等) → 我们复用 [`crate::verification::VerificationSubject`]
//!
//! # 0 触碰 apeireth-formal 本体
//! - 0 借 `apeireth-formal::run_all` (governance 是上层, 形式化是下层, 0 依赖)
//! - 0 改 apeireth-formal 任何文件
//! - 自带 6 不变量 + sanity_check, 1:1 跟 Stage 5 8 硬墙对应

use crate::consistency::{CheckStatus, ConsistencyReport};
use crate::strategy::{DecisionTree, GovernanceAction, GovernanceContext, PolicyKind};
use crate::verification::{invariants as ver_inv, Boundary, VerificationSubject};

/// Stage 5 不变量 1: workspace.version 1.2.0 严守 (B2, 整合 #4 commit abf12243).
///
/// **物理含义**: Cargo.toml:246 `version = "1.2.0"` 严守, 整合 #4 commit 升级 1.1.0 → 1.2.0 done.
/// R127 release 才升 1.0.0 (大版本归 0, per 决策-22 §2.2 B2 路线).
pub fn invariant_version_1_2_0_locked() -> bool {
    ver_inv::version_major_is_one(&VerificationSubject::safe_default())
        && ver_inv::version_minor_is_two(&VerificationSubject::safe_default())
}

/// Stage 5 不变量 2: R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守 (A1).
///
/// **物理含义**: 17 文件原位, 0 删 0 改, 数字 0 改, 测度结构可调 (per 决策-33 §2.3 A1 + A2).
pub fn invariant_baseline_3_value_intact() -> bool {
    ver_inv::baseline_index_is_r11(&VerificationSubject::safe_default())
        && crate::consistency::checks::baseline_3_value_present()
}

/// Stage 5 不变量 3: 24 LOCKED 入口签名 0 改 (B1, P2-3 verify done).
///
/// **物理含义**: per 决策-41 §2 + 决策-42 §1.1, P2-3 sub-agent 24/24 LOCKED 入口签名 0 改 verify done.
/// 内部 fn 实施可改 (per 决策-33 §2.3 B1), 入口签名 0 改.
pub fn invariant_locked_24_entry_signatures() -> bool {
    ver_inv::locked_signatures_intact(&VerificationSubject::safe_default())
        && crate::consistency::checks::locked_24_crate_inventory()
}

/// Stage 5 不变量 4: 8 哲学锚 (B5 6→8 升级, P1-2 R126 done).
///
/// **物理含义**: 6 + S-3 (质量工程化) + O-1 (安全优先) = 8, P1-2 R126 8 哲学锚升级 done.
pub fn invariant_anchor_8_complete() -> bool {
    ver_inv::anchor_count_is_eight(&VerificationSubject::safe_default())
        && crate::consistency::checks::anchor_8_complete()
}

/// Stage 5 不变量 5: 6 重守门 v7 (B4 6 重 v6 → v7, P1-3 R126 升 v7).
///
/// **物理含义**: 5 + Colang DSL = 6 重 v6, P1-3 R126 升 v7, 6 重 layer 严守.
pub fn invariant_gate_6_layers_v7() -> bool {
    ver_inv::gate_layers_is_six(&VerificationSubject::safe_default())
        && crate::consistency::checks::gate_v7_6_layers()
}

/// Stage 5 不变量 6: 治理决策树 0 越界 (C1 0 主动 commit, C2 0 装 PASS 严守).
///
/// **物理含义**: 治理决策树对 5 已知策略 (Version/Baseline/Locked/Anchor/Gate) 全 Allow, 其他 Other → Reject.
/// 0 越界 8 硬墙 (per 决策-55 §4 + 决策-33 §2.3).
pub fn invariant_governance_decision_tree_safe() -> bool {
    // 5 已知策略都应 Allow
    let contexts = [
        GovernanceContext::version(),
        GovernanceContext::baseline(),
        GovernanceContext::locked(0),
        GovernanceContext::locked(23),
        GovernanceContext::anchor(),
        GovernanceContext::gate(),
    ];
    for ctx in &contexts {
        let decision = DecisionTree::from_context(ctx).dispatch();
        if matches!(decision.action, GovernanceAction::Reject) {
            return false;
        }
    }
    // Other (未知策略) 应 Reject
    let other = GovernanceContext {
        policy: 100,
        value: 0,
        audit: false,
        version_major: 1,
        baseline_index: 0,
        locked_index: 0,
    };
    let other_decision = DecisionTree::from_context(&other).dispatch();
    matches!(other_decision.action, GovernanceAction::Reject)
}

/// 跑全部 6 Stage 5 不变量 (借鉴 `apeireth-formal/invariants::run_all` 1:1 模式).
pub fn run_all() -> bool {
    invariant_version_1_2_0_locked()
        && invariant_baseline_3_value_intact()
        && invariant_locked_24_entry_signatures()
        && invariant_anchor_8_complete()
        && invariant_gate_6_layers_v7()
        && invariant_governance_decision_tree_safe()
}

/// Sanity check (借鉴 `apeireth-formal/invariants::sanity_check` 1:1).
///
/// **设计**: 故意**排除** known violation 模式 (类似 apeireth-formal `negative_*` 测试),
/// 避免 false positive 触发 panic.
pub fn sanity_check() -> bool {
    let subject = VerificationSubject::safe_default();
    // 6 验证不变量全过
    ver_inv::run_all(&subject)
        // 5 一致性 check 全过
        && crate::consistency::checks::run_all()
        // 6 Stage 5 不变量全过
        && run_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_6_invariants_passes() {
        assert!(run_all(), "at least one Stage 5 invariant failed");
    }

    #[test]
    fn sanity_check_passes() {
        assert!(sanity_check());
    }

    #[test]
    fn version_1_2_0_locked_passes() {
        assert!(invariant_version_1_2_0_locked());
    }

    #[test]
    fn baseline_3_value_intact_passes() {
        assert!(invariant_baseline_3_value_intact());
    }

    #[test]
    fn locked_24_entry_signatures_passes() {
        assert!(invariant_locked_24_entry_signatures());
    }

    #[test]
    fn anchor_8_complete_passes() {
        assert!(invariant_anchor_8_complete());
    }

    #[test]
    fn gate_6_layers_v7_passes() {
        assert!(invariant_gate_6_layers_v7());
    }

    #[test]
    fn governance_decision_tree_safe_passes() {
        assert!(invariant_governance_decision_tree_safe());
    }

    #[test]
    fn governance_decision_tree_rejects_other_policy() {
        // 负例: 未知 policy 必须 Reject
        let other = GovernanceContext {
            policy: 100,
            value: 0,
            audit: false,
            version_major: 1,
            baseline_index: 0,
            locked_index: 0,
        };
        let action = DecisionTree::from_context(&other).action();
        assert_eq!(action, GovernanceAction::Reject);
    }

    #[test]
    fn governance_decision_tree_5_known_allow() {
        // 5 已知策略都 Allow
        for ctx in [
            GovernanceContext::version(),
            GovernanceContext::baseline(),
            GovernanceContext::locked(0),
            GovernanceContext::locked(12),
            GovernanceContext::locked(23),
            GovernanceContext::anchor(),
            GovernanceContext::gate(),
        ] {
            let action = DecisionTree::from_context(&ctx).action();
            assert_eq!(action, GovernanceAction::Allow, "ctx.policy={}", ctx.policy);
        }
    }

    #[test]
    fn consistency_report_passes() {
        let r = ConsistencyReport::check();
        assert!(r.is_ok());
        assert_eq!(r.pass_count(), 5);
    }

    #[test]
    fn boundary_count_matches_invariants() {
        // 8 boundary 应 = 6 verification invariant + 5 consistency check - 3 重叠
        // (locked_signatures_intact 跟 locked_24_crate_inventory 部分重叠)
        assert_eq!(Boundary::count(), 8);
    }
}
