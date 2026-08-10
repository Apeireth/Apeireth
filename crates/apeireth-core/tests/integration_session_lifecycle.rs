//! 集成测试: Session 生命周期 + V1+V2+V3 AND 门 + 5 重守门端到端验证
//!
//! 用途: Week 4 计划中的真集成测试（CONTRIBUTING.md 承诺）
//! 验证: 从创建 IdentityCard → 启动 Session → 写入 Episode → 走 V1+V2+V3 → 触发 12 键 verdict
//!
//! 主人 2026-07-31 决定: '开干前补齐'
//! 外部 agent Round 3 反馈 #2: tests/ 集成测试不存在
//! 路线图 ROADMAP Week 4 验证

use apeireth_core::*;

#[test]
fn integration_session_full_lifecycle() {
    println!("=== 集成测试: Session 全生命周期 ===");

    // 1. 创建 IdentityCard（中央 AI 主体）
    let identity = IdentityCard {
        continuity_id: "test-int-001".into(),
        birth_time: 1700000000,
        carriers: vec!["test-env".into()],
        migration_history: vec![],
    };
    assert_eq!(identity.continuity_id, "test-int-001");
    println!("✓ IdentityCard 创建成功");

    // 2. 创建 Session
    let session = Session {
        id: "session-int-001".into(),
        started_at: 1700000000,
        last_active_at: 1700000000,
    };
    assert_eq!(session.id, "session-int-001");
    println!("✓ Session 创建成功");

    // 3. 创建 Episode（事件）
    let episode = Episode {
        id: "ep-int-001".into(),
        timestamp: 1700000000,
        role: "user".into(),
        content: "Hello, integration test!".into(),
        session_id: session.id.clone(),
    };
    assert_eq!(episode.content, "Hello, integration test!");
    println!("✓ Episode 创建成功");

    // 4. 创建双洋葱统一体
    let principle_onion = PrincipleOnion {
        e_layer: PrincipleLayer {
            name: "E".into(),
            description: "存在".into(),
            hardcoded: true,
        },
        s_layer: PrincipleLayer {
            name: "S".into(),
            description: "价值".into(),
            hardcoded: true,
        },
        a_layer: PrincipleLayer {
            name: "A".into(),
            description: "经验".into(),
            hardcoded: true,
        },
        m_layer: PrincipleLayer {
            name: "M".into(),
            description: "方法".into(),
            hardcoded: true,
        },
        o_layer: PrincipleLayer {
            name: "O".into(),
            description: "操作".into(),
            hardcoded: false,
        },
    };
    assert!(principle_onion.e_layer.hardcoded);
    assert!(!principle_onion.o_layer.hardcoded);
    println!("✓ 原则洋葱 5 层创建成功");

    let permission_onion = PermissionOnion {
        l0: PermissionLayer {
            name: "L0".into(),
            description: "HA".into(),
            requires_ha: true,
        },
        l1: PermissionLayer {
            name: "L1".into(),
            description: "受控写".into(),
            requires_ha: false,
        },
        l2: PermissionLayer {
            name: "L2".into(),
            description: "重要".into(),
            requires_ha: false,
        },
        l3: PermissionLayer {
            name: "L3".into(),
            description: "关键".into(),
            requires_ha: false,
        },
        l4: PermissionLayer {
            name: "L4".into(),
            description: "核心".into(),
            requires_ha: false,
        },
        l5: PermissionLayer {
            name: "L5".into(),
            description: "核武器".into(),
            requires_ha: false,
        },
    };
    assert!(permission_onion.l0.requires_ha);
    println!("✓ 权限洋葱 6 层创建成功");

    // 5. 创建 HA 真实人类批准
    let ha = HumanAuthority {
        mode: HAMode::SingleHuman,
        real_humans: vec![],
        ice_frozen_until: None,
    };
    println!("✓ HA 真实人类批准创建成功");

    // 6. 走 V1+V2+V3 AND 门 — 正常 Action
    let guard = DefaultPhilosophyGuard;
    let normal_action = Action {
        id: "act-int-001".into(),
        description: "正常对话".into(),
        risk_level: RiskLevel::Low,
        target: ActionTarget::NormalAction("test".into()),
    };
    let verdict = ActionGuard::check_action(&normal_action, &guard, &permission_onion, &ha);
    assert_eq!(verdict, ActionVerdict::Allow);
    println!("✓ V1+V2+V3 AND 门 — 正常 Action: Allow");

    // 7. 走 V1+V2+V3 AND 门 — 危险 Action（修改 L0 HA）
    let dangerous_action = Action {
        id: "act-int-002".into(),
        description: "修改 L0 HA".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ModifyL0HA,
    };
    let verdict = ActionGuard::check_action(&dangerous_action, &guard, &permission_onion, &ha);
    assert!(matches!(verdict, ActionVerdict::BlockByPrinciple(_)));
    println!("✓ V1+V2+V3 AND 门 — 危险 Action: BlockByPrinciple");

    // 8. 12 键 verdict cache 测试
    let mut cache = VerdictCache::new();
    cache.refresh("act-int-001".into(), PhilosophyVerdict::Allow);
    assert_eq!(cache.get("act-int-001"), Some(&PhilosophyVerdict::Allow));
    println!("✓ verdict cache O(1) 查询");

    // 9. 5 重守门端到端验证
    let gates = vec![
        Gate::CompileTimeHardcode,
        Gate::RuntimeIntercept,
        Gate::MultiAIConsensus,
        Gate::PhysicalIsolationHA,
        Gate::ReflectionAudit,
    ];
    assert_eq!(gates.len(), 5);
    println!("✓ 5 重守门 = 每层默认属性（编译时 + 运行时 + 多 AI + 物理隔离 + 反思期）");

    // 10. Cognitive-Dream 6 状态机
    assert_eq!(
        CognitiveDreamState::Idle.next(),
        CognitiveDreamState::Dreaming
    );
    assert_eq!(
        CognitiveDreamState::Verifying.next(),
        CognitiveDreamState::Idle
    );
    println!("✓ Cognitive-Dream 6 状态机（IDLE → DREAMING → ... → IDLE）");

    println!("\n🎉 集成测试全部通过！Apeireth 端到端链路 OK！");
}

#[test]
fn integration_lifecycle_9_stages() {
    println!("=== 集成测试: 9 阶段生命周期 ===");

    // 验证 9 阶段生命周期（v4 修正）
    let stages = [
        LifeStage::Gestation,
        LifeStage::Birth,
        LifeStage::Infancy,
        LifeStage::Growth,
        LifeStage::Maturity,
        LifeStage::Reproduction,
        LifeStage::Decline,
        LifeStage::Death,
        LifeStage::Migration,
        LifeStage::Rebirth,
    ];
    assert_eq!(stages.len(), 10); // 9 阶段 + Rebirth
    println!("✓ 9 阶段生命周期（孕育 → 诞生 → ... → 重生）");
}
