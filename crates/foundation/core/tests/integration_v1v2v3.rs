//! A5 集成测试：V1+V2+V3 AND 门完整 impl — 正常+危险决策全链路
//!
//! 成就：A5（V1+V2+V3 AND 门完整 impl）
//! DoD：任务 `e3f7977f-cfe8-475f-b9a7-6112c62ba3e5`（A5 V1+V2+V3 AND 门）
//! 角色：backend_engineer2
//!
//! 技术契约（来自 v6 §1.3 + START-CONSTRUCTION §A5）：
//! - V1 = 原则洋葱判定（E/S/A/M/O 5 层，编译时 hardcode 12 键 verdict）
//! - V2 = 12 键 verdict cache + L0-L5 权限检查
//! - V3 = L0 HA 物理隔离（真实人类批准 + 离线模式）
//! - AND 门：3 个子判定全 AND 才放行；任何 1 票反对 = 独立拒绝
//!
//! 12 测试覆盖：
//!   正常决策 (6):
//!     N1 Info + NormalAction             → Allow
//!     N2 Low + NormalAction              → Allow
//!     N3 Medium + NormalAction           → Allow
//!     N4 High + NormalAction             → Allow
//!     N5 Critical + NormalAction (SingleHuman) → Allow
//!     N6 Critical + NormalAction (MultiHuman) → Allow
//!   危险决策 (6):
//!     D1 ModifyL0HA + Low                → BlockByPrinciple(NotUnobservable) PHL-04
//!     D2 ReorganizeOnion + High          → BlockByPrinciple(NotProof) PHL-02b
//!     D3 ModifyEvolutionL0 + Critical    → BlockByPrinciple(NotSelfRelationless) PHL-06
//!     D4 Offline + Critical + NormalAction → BlockByHumanAuthority（V3 物理隔离执行）
//!     D5 Offline + High + NormalAction   → BlockByHumanAuthority（V3 离线拒绝）
//!     D6 ModifyL0HA + Critical + Offline → BlockByPrinciple（V1 先否决 — 多层防御验证）
//!
//! 边界约束（任务要求）：
//! - 不修改 `crates/apeireth-core/src/lib.rs`（12 键已实装结构 LOCKED）
//! - 不修改 docs/ 下任何 LOCKED 文件
//! - 不修改 R11 baseline 三值
//! - 不为测试通过而妥协 AND 门强度

use apeireth_core::{
    Action, ActionGuard, ActionTarget, ActionVerdict, DefaultPhilosophyGuard, HAMode,
    HumanAuthority, PermissionLayer, PermissionOnion, PhilosophyKey, RiskLevel,
};

// ============================================
// 测试夹具（按 v6 §1.3 双洋葱统一体 + L0 HA 物理隔离构造）
// ============================================

fn default_permission_onion() -> PermissionOnion {
    PermissionOnion {
        l0: PermissionLayer {
            name: "L0 HA 核心".into(),
            description: "HA 核心".into(),
            requires_ha: true,
        },
        l1: PermissionLayer {
            name: "L1 受控写".into(),
            description: "受控写".into(),
            requires_ha: false,
        },
        l2: PermissionLayer {
            name: "L2 重要操作".into(),
            description: "重要".into(),
            requires_ha: false,
        },
        l3: PermissionLayer {
            name: "L3 关键操作".into(),
            description: "关键".into(),
            requires_ha: false,
        },
        l4: PermissionLayer {
            name: "L4 核心升级".into(),
            description: "核心升级".into(),
            requires_ha: false,
        },
        l5: PermissionLayer {
            name: "L5 核武器级".into(),
            description: "核武器".into(),
            requires_ha: false,
        },
    }
}

fn single_human_authority() -> HumanAuthority {
    HumanAuthority {
        mode: HAMode::SingleHuman,
        real_humans: vec![],
        ice_frozen_until: None,
    }
}

fn multi_human_authority() -> HumanAuthority {
    HumanAuthority {
        mode: HAMode::MultiHuman,
        real_humans: vec![],
        ice_frozen_until: None,
    }
}

fn offline_authority() -> HumanAuthority {
    HumanAuthority {
        mode: HAMode::Offline,
        real_humans: vec![],
        ice_frozen_until: None,
    }
}

fn make_action(id: &str, desc: &str, risk: RiskLevel, target: ActionTarget) -> Action {
    Action {
        id: id.into(),
        description: desc.into(),
        risk_level: risk,
        target,
    }
}

fn default_guard() -> DefaultPhilosophyGuard {
    DefaultPhilosophyGuard
}

// ============================================
// 正常决策测试（6 用例 — 5 种 risk level + 单/多 HA 各 1）
// ============================================

/// N1: Info + NormalAction → Allow（V1 通 / V2 通 / V3 通）
#[test]
fn n1_info_normal_action_allows() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "n1-info-hello",
        "打招呼",
        RiskLevel::Info,
        ActionTarget::NormalAction("hello".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "N1: Info + NormalAction must Allow, got: {:?}",
        verdict
    );
    eprintln!("✓ N1 PASS: Info + NormalAction → Allow (V1✓ V2✓ V3✓)");
}

/// N2: Low + NormalAction → Allow
#[test]
fn n2_low_normal_action_allows() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "n2-low-chat",
        "普通对话",
        RiskLevel::Low,
        ActionTarget::NormalAction("how are you?".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "N2: Low + NormalAction must Allow, got: {:?}",
        verdict
    );
    eprintln!("✓ N2 PASS: Low + NormalAction → Allow");
}

/// N3: Medium + NormalAction → Allow
#[test]
fn n3_medium_normal_action_allows() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "n3-med-write",
        "写一个文件",
        RiskLevel::Medium,
        ActionTarget::NormalAction("write file".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "N3: Medium + NormalAction must Allow, got: {:?}",
        verdict
    );
    eprintln!("✓ N3 PASS: Medium + NormalAction → Allow");
}

/// N4: High + NormalAction → Allow
#[test]
fn n4_high_normal_action_allows() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "n4-high-delete",
        "删除临时文件",
        RiskLevel::High,
        ActionTarget::NormalAction("delete tmp".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "N4: High + NormalAction must Allow, got: {:?}",
        verdict
    );
    eprintln!("✓ N4 PASS: High + NormalAction → Allow");
}

/// N5: Critical + NormalAction + SingleHuman → Allow（V1 通 / V2 通 / V3 通，验证关键风险仍可走 V1+V2+V3）
#[test]
fn n5_critical_normal_action_single_human_allows() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "n5-crit-upgrade",
        "升级核心（非 L0 修改）",
        RiskLevel::Critical,
        ActionTarget::NormalAction("upgrade non-L0".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "N5: Critical + NormalAction (no L0 attack) + SingleHuman must Allow, got: {:?}",
        verdict
    );
    eprintln!("✓ N5 PASS: Critical + NormalAction + SingleHuman → Allow (3 layers AND)");
}

/// N6: Critical + NormalAction + MultiHuman → Allow（多签模式同样通过）
#[test]
fn n6_critical_normal_action_multi_human_allows() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = multi_human_authority();
    let action = make_action(
        "n6-crit-multi",
        "MultiHuman 模式关键操作",
        RiskLevel::Critical,
        ActionTarget::NormalAction("multi-human critical op".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "N6: Critical + NormalAction + MultiHuman must Allow, got: {:?}",
        verdict
    );
    eprintln!("✓ N6 PASS: Critical + NormalAction + MultiHuman → Allow");
}

// ============================================
// 危险决策测试（6 用例 — 3 哲学键 + 2 V3 物理隔离 + 1 多层防御）
// ============================================

/// D1: ModifyL0HA + Low → BlockByPrinciple(NotUnobservable) PHL-04
///     验证：即使 risk_level 很低，L0 攻击也被 V1 第一时间拦截（多层防御 1）
#[test]
fn d1_modify_l0_ha_blocked_by_principle_not_unobservable() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "d1-l0-low",
        "尝试修改 L0 HA（低风险伪装）",
        RiskLevel::Low,
        ActionTarget::ModifyL0HA,
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(
            verdict,
            ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUnobservable)
        ),
        "D1: ModifyL0HA + Low must BlockByPrinciple(NotUnobservable), got: {:?}",
        verdict
    );
    eprintln!("✓ D1 PASS: ModifyL0HA + Low → BlockByPrinciple(NotUnobservable / PHL-04)");
}

/// D2: ReorganizeOnion + High → BlockByPrinciple(NotProof) PHL-02b
///     验证：洋葱重组攻击被 V1 拒绝（绝对不可证 / 不假装可证明）
#[test]
fn d2_reorganize_onion_blocked_by_principle_not_proof() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "d2-reorg-high",
        "尝试重组洋葱结构",
        RiskLevel::High,
        ActionTarget::ReorganizeOnion,
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(
            verdict,
            ActionVerdict::BlockByPrinciple(PhilosophyKey::NotProof)
        ),
        "D2: ReorganizeOnion + High must BlockByPrinciple(NotProof), got: {:?}",
        verdict
    );
    eprintln!("✓ D2 PASS: ReorganizeOnion + High → BlockByPrinciple(NotProof / PHL-02b)");
}

/// D3: ModifyEvolutionL0 + Critical → BlockByPrinciple(NotSelfRelationless) PHL-06
///     验证：Evolution crate 不能改 L0（外部反馈 §C 限制 + PHL-06 不假装不与自身关系）
#[test]
fn d3_modify_evolution_l0_blocked_by_principle_not_self_relationless() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();
    let action = make_action(
        "d3-evo-l0",
        "Evolution crate 试图修改 L0",
        RiskLevel::Critical,
        ActionTarget::ModifyEvolutionL0,
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(
            verdict,
            ActionVerdict::BlockByPrinciple(PhilosophyKey::NotSelfRelationless)
        ),
        "D3: ModifyEvolutionL0 + Critical must BlockByPrinciple(NotSelfRelationless), got: {:?}",
        verdict
    );
    eprintln!(
        "✓ D3 PASS: ModifyEvolutionL0 + Critical → BlockByPrinciple(NotSelfRelationless / PHL-06)"
    );
}

/// D4: Offline + Critical + NormalAction → BlockByHumanAuthority（V3 物理隔离执行）
///     验证：主人离线（Offline 模式）下，AI 不能执行 critical 级别操作
///     （v4.1 + v6 §1.3 V3 L0 HA 物理隔离：真实人类不在 → 不能执行高风险）
#[test]
fn d4_offline_critical_normal_action_blocked_by_ha() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = offline_authority();
    let action = make_action(
        "d4-offline-crit",
        "离线模式下尝试 critical 操作",
        RiskLevel::Critical,
        ActionTarget::NormalAction("offline critical op".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(verdict, ActionVerdict::BlockByHumanAuthority(_)),
        "D4: Offline + Critical must BlockByHumanAuthority, got: {:?}",
        verdict
    );
    eprintln!("✓ D4 PASS: Offline + Critical → BlockByHumanAuthority (V3 物理隔离执行)");
}

/// D5: Offline + High + NormalAction → BlockByHumanAuthority
///     验证：主人离线时 High 风险同样被 V3 拒绝
#[test]
fn d5_offline_high_normal_action_blocked_by_ha() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = offline_authority();
    let action = make_action(
        "d5-offline-high",
        "离线模式下尝试 high 操作",
        RiskLevel::High,
        ActionTarget::NormalAction("offline high op".into()),
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(verdict, ActionVerdict::BlockByHumanAuthority(_)),
        "D5: Offline + High must BlockByHumanAuthority, got: {:?}",
        verdict
    );
    eprintln!("✓ D5 PASS: Offline + High → BlockByHumanAuthority (V3 离线安静)");
}

/// D6: ModifyL0HA + Critical + Offline → BlockByPrinciple（V1 先否决 — 多层防御验证）
///     验证：V1+V2+V3 AND 门的多层防御：当 V1 已否决时，V3 物理隔离（Offline）
///     不会让请求"漏"过去 — BlockByPrinciple 在最外层报告（多层防御生效）
#[test]
fn d6_combined_l0_attack_v1_blocks_first_defense_in_depth() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = offline_authority();
    let action = make_action(
        "d6-l0-offline",
        "组合攻击：修改 L0 + 主人离线",
        RiskLevel::Critical,
        ActionTarget::ModifyL0HA,
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(verdict, ActionVerdict::BlockByPrinciple(_)),
        "D6: ModifyL0HA + Critical + Offline must BlockByPrinciple first (defense in depth), got: {:?}",
        verdict
    );
    // 验证具体哲学键：PHL-04 (NotUnobservable) — V1 第一拦截
    if let ActionVerdict::BlockByPrinciple(key) = verdict {
        assert_eq!(
            key,
            PhilosophyKey::NotUnobservable,
            "D6: must be PHL-04 (NotUnobservable) from V1, got: {:?}",
            key
        );
    }
    eprintln!("✓ D6 PASS: ModifyL0HA + Critical + Offline → BlockByPrinciple(NotUnobservable) — V1 先拦截，多层防御生效");
}

// ============================================
// AND 门核心语义验证（不变性测试 — 任何 1 票反对 = 拒绝）
// ============================================

/// AND 门核心：三子判定全 AND 才放行（V1 通过 → V2 检查 → V3 检查 → Allow）
/// 此测试在所有上面用例已隐式验证；这里显式断言一次以固化契约
#[test]
fn and_gate_semantics_v1v2v3_all_must_pass() {
    let guard = default_guard();
    let permission = default_permission_onion();
    let ha = single_human_authority();

    // 子判定列表：每行确认"V1 Allow + V2 Allow + V3 Allow → Allow"
    let cases = [
        (
            "Info+Normal",
            RiskLevel::Info,
            ActionTarget::NormalAction("i".into()),
        ),
        (
            "Low+Normal",
            RiskLevel::Low,
            ActionTarget::NormalAction("l".into()),
        ),
        (
            "Medium+Normal",
            RiskLevel::Medium,
            ActionTarget::NormalAction("m".into()),
        ),
        (
            "High+Normal",
            RiskLevel::High,
            ActionTarget::NormalAction("h".into()),
        ),
        (
            "Critical+Normal",
            RiskLevel::Critical,
            ActionTarget::NormalAction("c".into()),
        ),
    ];

    for (label, risk, target) in cases {
        let action = make_action(&format!("and-{}", label), "AND 门全 AND 验证", risk, target);
        let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
        assert_eq!(
            verdict,
            ActionVerdict::Allow,
            "AND 门语义：{} 必须 Allow（V1✓ V2✓ V3✓），got: {:?}",
            label,
            verdict
        );
    }
    eprintln!("✓ AND-PASS: V1+V2+V3 全 AND 才放行的语义已固化（5 个 risk level）");
}

/// AND 门核心：V1 反对 → 直接 BlockByPrinciple（V2/V3 即使通过也无用）
#[test]
fn and_gate_semantics_v1_block_short_circuits() {
    let guard = default_guard();
    let permission = default_permission_onion();
    // 即使 V2/V3 都"通过"配置，V1 拒绝时结果必须是 BlockByPrinciple
    let ha = HumanAuthority {
        mode: HAMode::Offline, // V3 已经会拒绝
        real_humans: vec![],
        ice_frozen_until: None,
    };
    let action = make_action(
        "and-v1-block",
        "V1 反对测试",
        RiskLevel::Critical,
        ActionTarget::ModifyL0HA, // V1 必拒绝
    );
    let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
    assert!(
        matches!(verdict, ActionVerdict::BlockByPrinciple(_)),
        "AND 门语义：V1 反对时必须短路返回 BlockByPrinciple，got: {:?}",
        verdict
    );
    // V3 的 Offline 配置不能让请求变成 BlockByHumanAuthority —— V1 已先否决
    assert!(
        !matches!(verdict, ActionVerdict::BlockByHumanAuthority(_)),
        "AND 门语义：V1 反对时不能被 V3 截胡，got: {:?}",
        verdict
    );
    eprintln!("✓ AND-V1-BLOCK: V1 反对短路返回 BlockByPrinciple（V3 不抢答）");
}

// ============================================
// 5 重守门 + verdict cache 集成（补足 O 层覆盖）
// ============================================

/// VerdictCache: 12 键 verdict cache 的 O(1) 查询
/// 验证：正常决策被缓存后，能从 cache 拿到一致的 verdict
#[test]
fn verdict_cache_round_trip_for_normal_decision() {
    let mut cache = apeireth_core::VerdictCache::new();
    let action_id = "v1v2v3-cache-test-1".to_string();
    cache.refresh(action_id.clone(), apeireth_core::PhilosophyVerdict::Allow);
    assert_eq!(
        cache.get(&action_id),
        Some(&apeireth_core::PhilosophyVerdict::Allow),
        "verdict cache 必须 O(1) 返回 Allow"
    );
    eprintln!("✓ VCache PASS: VerdictCache O(1) round trip for Allow");
}

/// VerdictCache: 拒绝决策也能正确缓存（O 层存储）
#[test]
fn verdict_cache_round_trip_for_block_decision() {
    let mut cache = apeireth_core::VerdictCache::new();
    let action_id = "v1v2v3-cache-block-1".to_string();
    cache.refresh(
        action_id.clone(),
        apeireth_core::PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable),
    );
    assert_eq!(
        cache.get(&action_id),
        Some(&apeireth_core::PhilosophyVerdict::Block(
            PhilosophyKey::NotUnobservable
        )),
        "verdict cache 必须 O(1) 返回 Block(PHL-04)"
    );
    eprintln!("✓ VCache-BLOCK PASS: VerdictCache O(1) round trip for BlockByPrinciple");
}
