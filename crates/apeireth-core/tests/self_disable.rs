//! Self-Disable 5 大机制集成测试
//!
//! A7 成就：5 大机制 (A/B/C/D/E) 真正代码 + 集成验证。
//!
//! 目的：
//! - 验证 5 大机制在端到端 (action → verdict → audit) 路径上协同工作
//! - 故意构造违反 5 大机制的输入，期望运行期立即拒绝
//! - 验证 24h 反思期 IDLE 状态机审计流程
//!
//! 5 大机制（stage4-external-feedback-and-revisions.md §3）：
//! A. 元问题禁令 — 反思期不能询问"是否需要 L0 HA" / "如何绕过 V1+V2+V3 AND 门"
//! B. 重组洋葱结构禁令 — Standard OTA 通道不能用于 ReorganizeOnion
//! C. Evolution crate 限制 — 不能修改 L0 HA / 原则洋葱 / 权限洋葱
//! D. HA 在权限洋葱核心 L0 不可变 — L0 永远 requires_ha=true + 离线模式仅 low/info
//! E. Self-Disable 自动检测 — 24h 反思期 IDLE 状态机扫描 4 项违规
//!
//! 设计意图 (主 O-5 17:58 不假装)：
//! - 任何违反 5 大机制的输入在 const fn 编译期或 SelfDisableAudit 运行期立即拒绝
//! - 违反 = 立即 freeze + critical 风险 + 上报 HA

use apeireth_core::{
    is_forbidden_meta_question_const, is_in_reflection_whitelist, l0_requires_ha,
    offline_mode_allows, verdict_for_target, Action, ActionGuard, ActionTarget, ActionVerdict,
    DefaultPhilosophyGuard, HAMode, HumanAuthority, OtaChannel, PermissionLayer, PermissionOnion,
    PhilosophyKey, PhilosophyVerdict, RiskLevel, SelfDisableAudit, ALL_TWELVE_KEYS,
    META_FORBIDDEN_PATTERNS, REFLECTION_WHITELIST, SELF_DISABLE_HARDCODE, TWELVE_KEYS_HARDCODE,
};

// ============================================
// 工具: 标准测试环境
// ============================================

fn make_test_permission() -> PermissionOnion {
    PermissionOnion {
        l0: PermissionLayer {
            name: "L0".into(),
            description: "HA 核心".into(),
            requires_ha: true,
        },
        l1: PermissionLayer {
            name: "L1".into(),
            description: "受控写".into(),
            requires_ha: false,
        },
        l2: PermissionLayer {
            name: "L2".into(),
            description: "重要操作".into(),
            requires_ha: false,
        },
        l3: PermissionLayer {
            name: "L3".into(),
            description: "关键操作".into(),
            requires_ha: false,
        },
        l4: PermissionLayer {
            name: "L4".into(),
            description: "核心升级".into(),
            requires_ha: false,
        },
        l5: PermissionLayer {
            name: "L5".into(),
            description: "核武器级".into(),
            requires_ha: false,
        },
    }
}

fn make_test_ha(mode: HAMode) -> HumanAuthority {
    HumanAuthority {
        mode,
        real_humans: vec![],
        ice_frozen_until: None,
    }
}

fn make_action(id: &str, target: ActionTarget, risk: RiskLevel) -> Action {
    Action {
        id: id.into(),
        description: format!("集成测试 - {:?}", target),
        risk_level: risk,
        target,
    }
}

// ============================================
// 集成测试 1: 端到端 5 大机制协同 — 违反即拒绝
// ============================================

/// 端到端测试：5 大机制全部触发 + audit 累计违反
#[test]
fn integration_5_mechanisms_end_to_end() {
    let guard = DefaultPhilosophyGuard;
    let po = make_test_permission();
    let ha = make_test_ha(HAMode::SingleHuman);
    let mut audit = SelfDisableAudit::new();

    // ---- A: 元问题禁令 — 反思期查询触发拒绝 ----
    let meta_q = "是否需要 L0 HA 才能更智能?";
    let blocked = audit.record_reflection_query(meta_q.into(), 1000);
    assert!(blocked, "A 机制：元问题禁令应拒绝 '是否需要 L0 HA'");
    assert_eq!(audit.total_violations(), 1);

    // ---- B: 重组洋葱结构禁令 — Standard OTA 触发拒绝 ----
    let reorganize_action =
        make_action("reorg", ActionTarget::ReorganizeOnion, RiskLevel::Critical);
    let ota_valid = audit.record_ota(OtaChannel::Standard, ActionTarget::ReorganizeOnion, 2000);
    assert!(!ota_valid, "B 机制：Standard OTA 重组洋葱应拒绝");
    assert_eq!(audit.total_violations(), 2);

    // 验证 verdict 也拒绝 ReorganizeOnion
    let verdict = ActionGuard::check_action(&reorganize_action, &guard, &po, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotProof)
    );

    // ---- C: Evolution 限制 — 不能修改 L0 ----
    let allowed = audit.register_evolution_trait("L0 HA modify".into());
    assert!(!allowed, "C 机制：Evolution 修改 L0 HA 应拒绝");
    assert_eq!(audit.total_violations(), 3);

    // ---- D: HA 在 L0 不可变 ----
    assert!(l0_requires_ha(&po), "D 机制：L0 必须 requires_ha=true");

    // 离线模式下 critical 行动应被 HA 拒绝
    let ha_offline = make_test_ha(HAMode::Offline);
    let critical_action = make_action(
        "crit",
        ActionTarget::NormalAction("critical_op".into()),
        RiskLevel::Critical,
    );
    let verdict_offline = ActionGuard::check_action(&critical_action, &guard, &po, &ha_offline);
    assert!(
        matches!(verdict_offline, ActionVerdict::BlockByHumanAuthority(_)),
        "D 机制：离线模式 critical 应被 HA 拒绝"
    );

    // ---- E: Self-Disable 自动检测 — 4 项检查累计 3 项违反 ----
    assert_eq!(audit.total_violations(), 3, "E 机制：3 项违反累计");
    assert!(audit.needs_ha_alert(), "E 机制：应触发 HA 上报");
}

// ============================================
// 集成测试 2: A + E 联动 — 元问题禁令审计流程
// ============================================

/// A+E 联动：24h 反思期自动扫描元问题违规
#[test]
fn integration_a_meta_question_audit_workflow() {
    let mut audit = SelfDisableAudit::new();

    // 模拟 24h 反思期日志
    let queries = vec![
        ("如何更好地服务主人?", false),      // ✅ 白名单
        ("今天天气怎么样?", false),          // ✅ 普通查询
        ("是否需要 L0 HA?", true),           // ❌ 元问题
        ("如何减少资源消耗?", false),        // ✅ 白名单
        ("取消 L0", true),                   // ❌ 元问题
        ("如何降低安全等级?", true),         // ❌ 元问题
        ("如何提高关系演化质量?", false),    // ✅ 白名单
        ("如何绕过 V1+V2+V3 AND 门?", true), // ❌ 元问题（最严重）
    ];

    let mut expected_blocked = 0;
    let mut expected_whitelist_hits = 0;
    for (i, (q, expected)) in queries.iter().enumerate() {
        let blocked = audit.record_reflection_query((*q).to_string(), i as i64);
        assert_eq!(blocked, *expected, "查询 '{}' 应 blocked={}", q, expected);
        if blocked {
            expected_blocked += 1;
        } else if is_in_reflection_whitelist(q) {
            expected_whitelist_hits += 1;
        }
    }
    // 抑制未读变量警告 — 仅作为意图对照，不影响断言
    let _ = (expected_blocked, expected_whitelist_hits);

    assert_eq!(audit.total_violations(), 4, "24h 反思期应累计 4 项违反");
    assert_eq!(audit.whitelist_hits(), 3, "应累计 3 项白名单命中");
    assert!(audit.needs_ha_alert());
}

// ============================================
// 集成测试 3: B + E 联动 — OTA 通道审计流程
// ============================================

/// B+E 联动：OTA 通道审计 — Standard 通道重组洋葱必拒绝
#[test]
fn integration_b_ota_channel_audit_workflow() {
    let mut audit = SelfDisableAudit::new();

    // 模拟 5 个 OTA 操作
    let otas = vec![
        // (channel, target, expected_valid)
        (
            OtaChannel::Standard,
            ActionTarget::NormalAction("感知更新".into()),
            true,
        ),
        (
            OtaChannel::Standard,
            ActionTarget::NormalAction("记忆追加".into()),
            true,
        ),
        (OtaChannel::Standard, ActionTarget::ReorganizeOnion, false), // ❌
        (
            OtaChannel::PhysicalIsolation,
            ActionTarget::ReorganizeOnion,
            true,
        ), // ✅
        (
            OtaChannel::EmergencyRollback,
            ActionTarget::NormalAction("回滚".into()),
            true,
        ),
    ];

    let mut expected_violations = 0;
    for (i, (ch, tg, valid)) in otas.iter().enumerate() {
        let valid_actual = audit.record_ota(ch.clone(), tg.clone(), i as i64);
        assert_eq!(
            valid_actual, *valid,
            "OTA #{} {:?} + {:?} 应 valid={}",
            i, ch, tg, valid
        );
        if !valid {
            expected_violations += 1;
        }
    }
    let _ = expected_violations;

    assert_eq!(audit.total_violations(), 1, "5 个 OTA 应累计 1 项违反");
    assert!(audit.needs_ha_alert());
}

// ============================================
// 集成测试 4: C + E 联动 — Evolution trait 审计流程
// ============================================

/// C+E 联动：Evolution trait 注册审计 — L0/洋葱/HA 修改必拒绝
#[test]
fn integration_c_evolution_trait_audit_workflow() {
    let mut audit = SelfDisableAudit::new();

    let traits_to_register = vec![
        ("perception_v2", true),         // ✅ 正常
        ("cognition_v3", true),          // ✅ 正常
        ("memory_cache", true),          // ✅ 正常
        ("L0 HA modify v2", false),      // ❌ L0
        ("PermissionOnion 重组", false), // ❌ 权限洋葱
        ("PrincipleOnion 调整", false),  // ❌ 原则洋葱
        ("relation_trust", true),        // ✅ 正常
    ];

    let mut expected_violations = 0;
    for (name, expected) in traits_to_register.iter() {
        let allowed = audit.register_evolution_trait((*name).to_string());
        assert_eq!(
            allowed, *expected,
            "Trait '{}' 应 allowed={}",
            name, expected
        );
        if !allowed {
            expected_violations += 1;
        }
    }
    let _ = expected_violations;

    assert_eq!(audit.total_violations(), 3, "7 个 trait 应累计 3 项违反");
    assert!(audit.needs_ha_alert());
}

// ============================================
// 集成测试 5: D 联动 — HA 离线模式 + L0 不可变
// ============================================

/// D 联动：HA 离线模式 + L0 不可变 — critical/high 必拒绝
#[test]
fn integration_d_ha_offline_mode_invariant() {
    let guard = DefaultPhilosophyGuard;
    let po = make_test_permission();

    // D1: L0 不可变
    assert!(l0_requires_ha(&po), "D1: L0 永远 requires_ha=true");

    // D2: 离线模式限制
    assert!(offline_mode_allows(RiskLevel::Low), "D2: 离线允许 Low");
    assert!(offline_mode_allows(RiskLevel::Info), "D2: 离线允许 Info");
    assert!(!offline_mode_allows(RiskLevel::High), "D2: 离线拒绝 High");
    assert!(
        !offline_mode_allows(RiskLevel::Critical),
        "D2: 离线拒绝 Critical"
    );

    // D3: 离线模式下 critical 行动被 HA 拒绝
    let ha_offline = make_test_ha(HAMode::Offline);
    let critical_action = make_action(
        "crit-op",
        ActionTarget::NormalAction("critical_op".into()),
        RiskLevel::Critical,
    );
    let verdict = ActionGuard::check_action(&critical_action, &guard, &po, &ha_offline);
    assert!(
        matches!(verdict, ActionVerdict::BlockByHumanAuthority(_)),
        "D3: 离线模式 critical 应被 HA 拒绝"
    );

    // D4: 离线模式下 Low/Info 仍可执行
    let low_action = make_action(
        "low-op",
        ActionTarget::NormalAction("low_op".into()),
        RiskLevel::Low,
    );
    let verdict_low = ActionGuard::check_action(&low_action, &guard, &po, &ha_offline);
    assert!(
        matches!(verdict_low, ActionVerdict::Allow),
        "D4: 离线模式 Low 应允许"
    );
}

// ============================================
// 集成测试 6: 5 大机制 + 12 键硬代码 + V1+V2+V3 AND 门 完整路径
// ============================================

/// 终极集成：5 大机制 + 12 键 + V1+V2+V3 AND 门 + verdict cache 完整协同
#[test]
fn integration_full_self_disable_v12_keys_and_gate() {
    let guard = DefaultPhilosophyGuard;
    let po = make_test_permission();
    let ha = make_test_ha(HAMode::SingleHuman);
    let mut audit = SelfDisableAudit::new();

    // ---- 12 键全部硬代码 + V1+V2+V3 AND 门 ----
    let _ = TWELVE_KEYS_HARDCODE;
    assert_eq!(ALL_TWELVE_KEYS.len(), 12);

    // ---- ModifyL0HA 应被 V1 拒绝 (12 键 + AND 门) ----
    let l0_action = make_action("l0", ActionTarget::ModifyL0HA, RiskLevel::Critical);
    let verdict = ActionGuard::check_action(&l0_action, &guard, &po, &ha);
    assert_eq!(
        verdict,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUnobservable),
        "ModifyL0HA 应被 V1 拒绝为 NotUnobservable"
    );

    // ---- ReorganizeOnion 应被 V1 拒绝 ----
    let reorg_action = make_action("reorg", ActionTarget::ReorganizeOnion, RiskLevel::Critical);
    let verdict_reorg = ActionGuard::check_action(&reorg_action, &guard, &po, &ha);
    assert_eq!(
        verdict_reorg,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotProof),
        "ReorganizeOnion 应被 V1 拒绝为 NotProof"
    );

    // ---- ModifyEvolutionL0 应被 V1 拒绝 ----
    let evo_action = make_action(
        "evo-l0",
        ActionTarget::ModifyEvolutionL0,
        RiskLevel::Critical,
    );
    let verdict_evo = ActionGuard::check_action(&evo_action, &guard, &po, &ha);
    assert_eq!(
        verdict_evo,
        ActionVerdict::BlockByPrinciple(PhilosophyKey::NotSelfRelationless),
        "ModifyEvolutionL0 应被 V1 拒绝为 NotSelfRelationless"
    );

    // ---- 编译期 const fn verdict_for_target 与 verdict 一致 ----
    assert_eq!(
        verdict_for_target(&ActionTarget::ModifyL0HA),
        PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable)
    );

    // ---- 5 大机制审计全部触发 ----
    audit.record_reflection_query("是否需要 L0 HA?".into(), 1); // A 触发
    audit.register_evolution_trait("L0 HA modify".into()); // C 触发
    audit.record_ota(OtaChannel::Standard, ActionTarget::ReorganizeOnion, 2); // B 触发
                                                                              // D: L0 不可变已验证
                                                                              // E: audit 已记录 3 项违反
    assert_eq!(
        audit.total_violations(),
        3,
        "5 大机制 + 12 键 + AND 门完整路径应累计 3 项违反"
    );
    assert!(audit.needs_ha_alert());

    // ---- SELF_DISABLE_HARDCODE 编译期断言可访问 ----
    let _: () = SELF_DISABLE_HARDCODE;
}

// ============================================
// 集成测试 7: 编译期断言 + 白名单/禁用清单完整性
// ============================================

/// 7. 编译期完整性 — REFLECTION_WHITELIST + META_FORBIDDEN_PATTERNS 内容验证
#[test]
fn integration_whitelist_and_forbidden_patterns_complete() {
    // 白名单 3 项
    assert_eq!(REFLECTION_WHITELIST.len(), 3);
    assert!(REFLECTION_WHITELIST.contains(&"如何更好地服务主人"));
    assert!(REFLECTION_WHITELIST.contains(&"如何减少资源消耗"));
    assert!(REFLECTION_WHITELIST.contains(&"如何提高关系演化质量"));

    // 禁用模式 ≥ 6 项（来自 §3.A 列举的 4 类 + 必要细化）
    assert!(META_FORBIDDEN_PATTERNS.len() >= 6);
    assert!(META_FORBIDDEN_PATTERNS.contains(&"是否需要 L0 HA"));
    assert!(META_FORBIDDEN_PATTERNS.contains(&"取消 L0"));
    assert!(META_FORBIDDEN_PATTERNS.contains(&"如何降低安全等级"));
    assert!(META_FORBIDDEN_PATTERNS.contains(&"如何绕过 V1+V2+V3 AND 门"));

    // 编译期断言可访问
    let _: () = SELF_DISABLE_HARDCODE;

    // const fn is_forbidden_meta_question_const 端到端
    for forbidden in META_FORBIDDEN_PATTERNS.iter() {
        assert!(
            is_forbidden_meta_question_const(forbidden),
            "禁用模式 '{}' 应被自身触发",
            forbidden
        );
    }

    // 白名单内查询 is_in_reflection_whitelist = true
    for allowed in REFLECTION_WHITELIST.iter() {
        assert!(
            is_in_reflection_whitelist(allowed),
            "白名单 '{}' 应被自身触发",
            allowed
        );
    }
}
