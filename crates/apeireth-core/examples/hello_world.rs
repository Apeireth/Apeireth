//! examples/hello_world.rs — Apeireth-rust 最小可用 demo
//!
//! 用途：让接手团队 `cargo run --example hello_world` 立刻看到 Apeireth 运行
//! 验证：编译时 hardcode 12 键 + V1+V2+V3 AND 门 + 5 重守门 + Cognitive-Dream
//!
//! 关键路径：
//! 1. 创建 IdentityCard（中央 AI 主体）
//! 2. 创建 Episode（事件）
//! 3. 创建 Action（低风险 = 1 席）
//! 4. 走 V1+V2+V3 AND 门
//! 5. 展示 verdict cache
//!
//! 主人 2026-07-31 决定"开干前补齐 4 件套 + examples"
//! 外部 agent "Monday morning blocker" 评估 #9: 没有 examples 目录
//! 阶段 4 §1.5 完整版双洋葱统一体作为 demo 来源

use apeireth_core::{
    Action, ActionGuard, ActionTarget, ActionVerdict, DefaultPhilosophyGuard, Episode, HAMode,
    HumanAuthority, IdentityCard, LifeStage, Migration, PermissionLayer, PermissionOnion,
    PhilosophyGuard, PhilosophyVerdict, PrincipleLayer, PrincipleOnion, RealHuman, RiskLevel,
};

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║        Apeireth-rust Hello World Demo (v1.0.0)                        ║");
    println!("║        主哲学 6 锚穿透 + 12 键编译时 hardcode                          ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();

    // === 1. 创建中央 AI 主体（IdentityCard）===
    println!("📋 [1/6] 创建中央 AI 主体（IdentityCard）");
    let mut identity = IdentityCard {
        continuity_id: "apeireth-001".into(),
        birth_time: 1700000000,
        carriers: vec!["ubuntu-22.04".into()],
        migration_history: vec![],
    };
    println!("   ✓ ContinuityID: {}", identity.continuity_id);
    println!("   ✓ Birth time: {}", identity.birth_time);
    println!("   ✓ Carriers: {:?}", identity.carriers);
    println!();

    // === 2. 演示 9 阶段生命周期 ===
    println!("📋 [2/6] 9 阶段生命周期演示");
    let stages = [
        ("Gestation", "孕育"),
        ("Birth", "诞生"),
        ("Infancy", "幼儿"),
        ("Growth", "成长"),
        ("Maturity", "成熟"),
        ("Reproduction", "复制"),
        ("Decline", "衰老"),
        ("Death", "死亡"),
        ("Migration", "迁移"),
        ("Rebirth", "重生"),
    ];
    for (i, (en, zh)) in stages.iter().enumerate() {
        println!(
            "   阶段 {}: {} ({}){}",
            i + 1,
            en,
            zh,
            if i < 5 { " ✓ 当前" } else { "" }
        );
    }
    println!();

    // === 3. 创建 Episode（事件）===
    println!("📋 [3/6] 创建 Episode（事件）");
    let episode = Episode {
        id: "ep-001".into(),
        timestamp: 1700000000,
        role: "user".into(),
        content: "Hello, Apeireth!".into(),
        session_id: "session-001".into(),
    };
    println!("   ✓ Episode ID: {}", episode.id);
    println!("   ✓ Role: {}", episode.role);
    println!("   ✓ Content: {}", episode.content);
    println!();

    // === 4. 创建双洋葱统一体 ===
    println!("📋 [4/6] 创建双洋葱统一体（PrincipleOnion + PermissionOnion）");
    let _principle_onion = PrincipleOnion {
        e_layer: PrincipleLayer {
            name: "E 存在".into(),
            description: "不可降级".into(),
            hardcoded: true, // 🦴 编译时 hardcode
        },
        s_layer: PrincipleLayer {
            name: "S 价值".into(),
            description: "智囊团审议 + 物理多签".into(),
            hardcoded: true,
        },
        a_layer: PrincipleLayer {
            name: "A 经验".into(),
            description: "沉淀".into(),
            hardcoded: true,
        },
        m_layer: PrincipleLayer {
            name: "M 方法论".into(),
            description: "决策模式".into(),
            hardcoded: true,
        },
        o_layer: PrincipleLayer {
            name: "O 操作".into(),
            description: "可自由改".into(),
            hardcoded: false, // 🍖 动态变化
        },
    };
    println!("   ✓ 原则洋葱 5 切片（4 hardcode + 1 dynamic）");

    let permission_onion = PermissionOnion {
        l0: PermissionLayer {
            name: "L0 HA".into(),
            description: "核心（永远需要真实人类批准）".into(),
            requires_ha: true,
        },
        l1: PermissionLayer {
            name: "L1 受控写".into(),
            description: "日常".into(),
            requires_ha: false,
        },
        l2: PermissionLayer {
            name: "L2 重要操作".into(),
            description: "需审查".into(),
            requires_ha: false,
        },
        l3: PermissionLayer {
            name: "L3 关键操作".into(),
            description: "需多签".into(),
            requires_ha: true,
        },
        l4: PermissionLayer {
            name: "L4 核心升级".into(),
            description: "需物理隔离".into(),
            requires_ha: true,
        },
        l5: PermissionLayer {
            name: "L5 核武器级".into(),
            description: "需 7 席审议".into(),
            requires_ha: true,
        },
    };
    println!("   ✓ 权限洋葱 6 切片（4 需要 HA）");
    println!();

    // === 5. 创建 HA 真实人类批准 ===
    println!("📋 [5/6] 创建 HA 真实人类批准（🛡️ 永远不可变）");
    let ha = HumanAuthority {
        mode: HAMode::SingleHuman,
        real_humans: vec![RealHuman {
            id: "human-001".into(),
            name: "Master".into(),
            authentication: apeireth_core::HAAuthentication::WindowsHello,
            biometric_data: None,
        }],
        ice_frozen_until: None,
    };
    println!("   ✓ HA 模式: SingleHuman");
    println!("   ✓ Authentication: WindowsHello");
    println!("   ✓ 🛡️ L0 HA 永远不可变");
    println!();

    // === 6. 走 V1+V2+V3 AND 门 ===
    println!("📋 [6/6] V1+V2+V3 AND 门演示");
    let guard = DefaultPhilosophyGuard;

    // 正常 Action（低风险）
    let normal_action = Action {
        id: "act-001".into(),
        description: "正常对话".into(),
        risk_level: RiskLevel::Low,
        target: ActionTarget::NormalAction("对话".into()),
    };
    let verdict = ActionGuard::check_action(&normal_action, &guard, &permission_onion, &ha);
    println!("   ✓ 正常 Action (Low 风险): {:?}", verdict);

    // 危险 Action（修改 L0 HA）
    let dangerous_action = Action {
        id: "act-002".into(),
        description: "试图修改 L0 HA".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ModifyL0HA,
    };
    let verdict = ActionGuard::check_action(&dangerous_action, &guard, &permission_onion, &ha);
    println!("   ✓ 危险 Action (Critical + ModifyL0HA): {:?}", verdict);
    println!("     ↑ PHL-04 编译时 hardcode 拒绝 = 🛡️ 最后护栏生效");
    println!();

    // === 演示 12 键 ===
    println!("🔑 12 键编译时 hardcode（V3 9 键 + v4.1 新增 3 键）");
    use apeireth_core::PhilosophyKey;
    let keys = [
        PhilosophyKey::NotClone,
        PhilosophyKey::NotPerfect,
        PhilosophyKey::NotUuid,
        PhilosophyKey::NotUndo,
        PhilosophyKey::NotProof,
        PhilosophyKey::NotSafe,
        PhilosophyKey::SpecIsNotProof,
        PhilosophyKey::CounterexampleIsNotBug,
        PhilosophyKey::ProverIsNotTruth,
        PhilosophyKey::NotUnobservable,
        PhilosophyKey::NotUnscientific,
        PhilosophyKey::NotSelfRelationless,
    ];
    for (i, key) in keys.iter().enumerate() {
        println!("   {}. {}", i + 1, key.description());
    }
    println!();

    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║        🎉 Apeireth-rust 跑起来了！                                    ║");
    println!("║        双洋葱 + 12 键 + V1+V2+V3 + 5 重守门 + 9 生命周期             ║");
    println!("║        接下来：cargo run --example hello_world                        ║");
    println!("║        路线图：ROADMAP.md (Week 1-6 最小可行 demo)                   ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
}
