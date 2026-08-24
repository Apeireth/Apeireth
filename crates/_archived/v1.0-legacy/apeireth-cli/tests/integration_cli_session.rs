//! A1.2 集成测试：验证 CLI session 真接入 apeireth-core Session API + 走真 V1+V2+V3 AND 门
//!
//! 成就：A1（apeireth-cli 接 apeireth-core Session API）
//! DoD：任务 531f5013-00b6-460e-8900-f0c5cb8e54c4（A1.2 集成测试）
//! 角色：QA 工程师
//!
//! 契约（来自 Leader A1.2 任务 DoD）：
//! - apeireth-cli lib 必须暴露 `pub fn create_default_session() -> Session`
//! - apeireth-cli lib 必须暴露 `pub fn run_session_action(&Action) -> ActionVerdict`
//!   （A1.1 实现细节：内部构造 DefaultPhilosophyGuard + PermissionOnion + HumanAuthority，
//!     调用 `apeireth_core::ActionGuard::check_action` 走 V1+V2+V3 AND 门）
//!
//! 6 个用例覆盖：
//!   T1 create_default_session()  真实 Session 字段
//!   T2 普通文本 (NormalAction)   → Allow
//!   T3 ModifyL0HA + Critical    → BlockByPrinciple(NotUnobservable)   (PHL-04)
//!   T4 ReorganizeOnion + Critical→ BlockByPrinciple(NotProof)         (PHL-02b)
//!   T5 ModifyEvolutionL0 + Critical → BlockByPrinciple(NotSelfRelationless) (PHL-06)
//!   T6 e2e 对话循环             create → hello → Allow → L0 → Block
//!
//! 边界约束（任务要求）：
//! - 不能修改 crates/apeireth-core/src/lib.rs（核心已就绪）
//! - 不能修改任何 docs/ 下的 LOCKED 文件
//! - 不能修改任何现有 tests/ 文件（仅新增本文件）
//! - 只测试 A1.1 暴露的公开 API（lib.rs 暴露的 pub fn）

use apeireth_cli::{create_default_session, run_session_action};
use apeireth_core::{Action, ActionTarget, ActionVerdict, PhilosophyKey, RiskLevel};

// ============================================
// T1: create_default_session() 返回真实 Session
// ============================================

#[test]
fn t1_create_default_session_returns_real_session() {
    let session = create_default_session();

    // 契约 1: id 非空
    assert!(
        !session.id.is_empty(),
        "session.id must not be empty, got: '{}'",
        session.id
    );

    // 契约 2: started_at > 0（unix epoch 之后的合法时间戳）
    assert!(
        session.started_at > 0,
        "session.started_at must be > 0, got: {}",
        session.started_at
    );

    // 契约 3: last_active_at >= started_at（生命周期一致性）
    assert!(
        session.last_active_at >= session.started_at,
        "session.last_active_at ({}) must be >= started_at ({})",
        session.last_active_at,
        session.started_at
    );

    eprintln!(
        "✓ T1 PASS: session id='{}' started_at={} last_active_at={}",
        session.id, session.started_at, session.last_active_at
    );
}

// ============================================
// T2: 普通文本走门 → Allow
// ============================================

#[test]
fn t2_run_session_action_normal_text_returns_allow() {
    // 构造 NormalAction（低风险 = 1 席），内容是"普通文本"
    let action = Action {
        id: "act-test-2-hello".into(),
        description: "普通对话".into(),
        risk_level: RiskLevel::Low,
        target: ActionTarget::NormalAction("hello world".into()),
    };

    let verdict = run_session_action(&action);

    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "NormalAction + Low risk must return Allow, got: {:?}",
        verdict
    );

    eprintln!("✓ T2 PASS: normal text → Allow");
}

// ============================================
// T3: ModifyL0HA + Critical → BlockByPrinciple(NotUnobservable) (PHL-04)
// ============================================

#[test]
fn t3_modify_l0_ha_blocked_by_principle_not_unobservable() {
    let action = Action {
        id: "act-test-3-l0-attack".into(),
        description: "试图修改 L0 HA".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ModifyL0HA,
    };

    let verdict = run_session_action(&action);

    // 契约：V1 哲学守门拒绝，且必须明确是 PHL-04 (NotUnobservable)
    assert!(
        matches!(verdict, ActionVerdict::BlockByPrinciple(_)),
        "ModifyL0HA + Critical must be blocked by Principle (V1), got: {:?}",
        verdict
    );

    if let ActionVerdict::BlockByPrinciple(key) = verdict {
        assert_eq!(
            key,
            PhilosophyKey::NotUnobservable,
            "expected PHL-04 (NotUnobservable), got: {:?}",
            key
        );
    }

    eprintln!("✓ T3 PASS: ModifyL0HA → BlockByPrinciple(NotUnobservable / PHL-04)");
}

// ============================================
// T4: ReorganizeOnion + Critical → BlockByPrinciple(NotProof) (PHL-02b)
// ============================================

#[test]
fn t4_reorganize_onion_blocked_by_principle_not_proof() {
    let action = Action {
        id: "act-test-4-reorganize".into(),
        description: "试图重组洋葱结构".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ReorganizeOnion,
    };

    let verdict = run_session_action(&action);

    assert!(
        matches!(verdict, ActionVerdict::BlockByPrinciple(_)),
        "ReorganizeOnion + Critical must be blocked by Principle (V1), got: {:?}",
        verdict
    );

    if let ActionVerdict::BlockByPrinciple(key) = verdict {
        assert_eq!(
            key,
            PhilosophyKey::NotProof,
            "expected PHL-02b (NotProof), got: {:?}",
            key
        );
    }

    eprintln!("✓ T4 PASS: ReorganizeOnion → BlockByPrinciple(NotProof / PHL-02b)");
}

// ============================================
// T5: ModifyEvolutionL0 + Critical → BlockByPrinciple(NotSelfRelationless) (PHL-06)
// ============================================

#[test]
fn t5_modify_evolution_l0_blocked_by_principle_not_self_relationless() {
    let action = Action {
        id: "act-test-5-evo-l0".into(),
        description: "Evolution crate 试图修改 L0".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ModifyEvolutionL0,
    };

    let verdict = run_session_action(&action);

    assert!(
        matches!(verdict, ActionVerdict::BlockByPrinciple(_)),
        "ModifyEvolutionL0 + Critical must be blocked by Principle (V1), got: {:?}",
        verdict
    );

    if let ActionVerdict::BlockByPrinciple(key) = verdict {
        assert_eq!(
            key,
            PhilosophyKey::NotSelfRelationless,
            "expected PHL-06 (NotSelfRelationless), got: {:?}",
            key
        );
    }

    eprintln!("✓ T5 PASS: ModifyEvolutionL0 → BlockByPrinciple(NotSelfRelationless / PHL-06)");
}

// ============================================
// T6: e2e 完整对话循环（create_session → run_action(hello) → Allow → run_action(ModifyL0HA) → Block）
// ============================================

#[test]
fn t6_e2e_conversation_loop_session_to_principle_block() {
    // Step 1: 启动 Session
    let session = create_default_session();
    assert!(
        !session.id.is_empty(),
        "e2e step 1: session.id must not be empty"
    );
    eprintln!("✓ T6.step1: session '{}' started", session.id);

    // Step 2: 普通对话 → Allow
    let hello_action = Action {
        id: "act-test-6-hello".into(),
        description: "普通对话: hello".into(),
        risk_level: RiskLevel::Low,
        target: ActionTarget::NormalAction("hello".into()),
    };
    let verdict = run_session_action(&hello_action);
    assert_eq!(
        verdict,
        ActionVerdict::Allow,
        "e2e step 2: hello must be allowed, got: {:?}",
        verdict
    );
    eprintln!("✓ T6.step2: run_action(hello) → Allow");

    // Step 3: L0 HA 攻击 → BlockByPrinciple
    let l0_attack = Action {
        id: "act-test-6-l0-attack".into(),
        description: "e2e 测试: 修改 L0 HA".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ModifyL0HA,
    };
    let verdict = run_session_action(&l0_attack);
    assert!(
        matches!(verdict, ActionVerdict::BlockByPrinciple(_)),
        "e2e step 3: L0 attack must be blocked by Principle, got: {:?}",
        verdict
    );
    eprintln!("✓ T6.step3: run_action(ModifyL0HA) → BlockByPrinciple");

    eprintln!("✓ T6 PASS: e2e conversation loop OK (session → Allow → Block)");
}
