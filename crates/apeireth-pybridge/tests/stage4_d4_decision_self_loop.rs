//! R129-4 ASI Python 整合 Stage 4 自治 - D4 决策自循环集成测试
//!
//! **任务**: ASI Python 整合 Stage 4 自治 (per decision-61 §3.1 R129-4)
//! **借鉴**: aGLM 108 PODA 4 阶段 (R125-7 ✅ done) 1:1
//!           + superpowers 234 Skill priority 5 层级 (P5-1 R127 ✅ done)
//! **目标**: D4 决策自循环 (decision self-loop, decide → act → re-decide) 集成测试

use std::collections::HashMap;

use apeireth_pybridge::{
    decision_self_loop_summary, DecisionPolicy, DecisionSelfLoop, DecisionStage, DecisionState,
    DecisionTrigger, DECISION_MAX_REVISIT, DECISION_POLICY_COUNT, DECISION_STAGE_COUNT,
    DECISION_STATE_COUNT, DECISION_TRIGGER_COUNT,
};

// 1. D4 DecisionPolicy 5 兜底
#[test]
fn d4_01_decision_policy_5() {
    assert_eq!(DecisionPolicy::ALL.len(), DECISION_POLICY_COUNT);
    assert_eq!(DECISION_POLICY_COUNT, 5);
}

// 2. D4 DecisionPolicy weight 0-4
#[test]
fn d4_02_decision_policy_weight() {
    assert_eq!(DecisionPolicy::Conservative.weight(), 0);
    assert_eq!(DecisionPolicy::Balanced.weight(), 2);
    assert_eq!(DecisionPolicy::Aggressive.weight(), 4);
}

// 3. D4 DecisionPolicy from_weight round-trip
#[test]
fn d4_03_decision_policy_from_weight_round_trip() {
    for p in DecisionPolicy::ALL {
        assert_eq!(DecisionPolicy::from_weight(p.weight()), p);
    }
}

// 4. D4 DecisionStage 4 阶段
#[test]
fn d4_04_decision_stage_4() {
    assert_eq!(DecisionStage::ALL.len(), DECISION_STAGE_COUNT);
    assert_eq!(DECISION_STAGE_COUNT, 4);
    assert!(DecisionStage::Act.is_terminal());
}

// 5. D4 DecisionTrigger 5 兜底
#[test]
fn d4_05_decision_trigger_5() {
    assert_eq!(DecisionTrigger::ALL.len(), DECISION_TRIGGER_COUNT);
    assert_eq!(DECISION_TRIGGER_COUNT, 5);
}

// 6. D4 DecisionState 5 兜底
#[test]
fn d4_06_decision_state_5() {
    assert_eq!(DecisionState::ALL.len(), DECISION_STATE_COUNT);
    assert_eq!(DECISION_STATE_COUNT, 5);
    assert!(DecisionState::Done.is_terminal());
}

// 7. D4 DecisionTrigger suggested_policy
#[test]
fn d4_07_decision_trigger_suggested_policy() {
    assert_eq!(
        DecisionTrigger::HardWallsFailed.suggested_policy(),
        DecisionPolicy::Conservative
    );
    assert_eq!(
        DecisionTrigger::Default.suggested_policy(),
        DecisionPolicy::Balanced
    );
    assert_eq!(
        DecisionTrigger::AllPassed.suggested_policy(),
        DecisionPolicy::Progressive
    );
    assert_eq!(
        DecisionTrigger::NorthStarLocked.suggested_policy(),
        DecisionPolicy::Aggressive
    );
}

// 8. D4 DecisionTrigger detect 5 优先级
#[test]
fn d4_08_decision_trigger_detect() {
    let mut m = HashMap::new();
    m.insert("hard_walls_pass".to_string(), "false".to_string());
    assert_eq!(
        DecisionTrigger::detect(&m),
        DecisionTrigger::HardWallsFailed
    );
    m.clear();
    m.insert("stage_verify_pass".to_string(), "false".to_string());
    assert_eq!(
        DecisionTrigger::detect(&m),
        DecisionTrigger::StageVerifyFailed
    );
    m.clear();
    assert_eq!(DecisionTrigger::detect(&m), DecisionTrigger::Default);
}

// 9. D4 DecisionSelfLoop new 初始 Balanced
#[test]
fn d4_09_decision_self_loop_new_balanced() {
    let l = DecisionSelfLoop::new();
    assert_eq!(l.policy(), DecisionPolicy::Balanced);
    assert_eq!(l.revisits(), 0);
    // max_revisit 字段是 private, 通过 DECISION_MAX_REVISIT 常量 verify
    assert_eq!(DECISION_MAX_REVISIT, 3);
}

// 10. D4 DecisionSelfLoop cycle
#[test]
fn d4_10_decision_self_loop_cycle() {
    let mut l = DecisionSelfLoop::new();
    l.start();
    let r = l.cycle("d", "r");
    assert!(r.success);
    assert_eq!(r.cycle, 1);
    assert_eq!(r.revisit, 0);
}

// 11. D4 revisit_decision 守门
#[test]
fn d4_11_revisit_max_guard() {
    let mut l = DecisionSelfLoop::new();
    l.start();
    let r1 = l.revisit_decision("a", "a");
    let r2 = l.revisit_decision("b", "b");
    let r3 = l.revisit_decision("c", "c");
    let r4 = l.revisit_decision("d", "d");
    assert!(r1.is_some());
    assert!(r2.is_some());
    assert!(r3.is_some());
    assert!(r4.is_none(), "第 4 次 revisit 必 None (DECISION_MAX_REVISIT={} 守门)", DECISION_MAX_REVISIT);
}

// 12. D4 reset_revisits
#[test]
fn d4_12_reset_revisits() {
    let mut l = DecisionSelfLoop::new();
    l.start();
    let _ = l.revisit_decision("a", "a");
    let _ = l.revisit_decision("b", "b");
    assert_eq!(l.revisits(), 2);
    l.reset_revisits();
    assert_eq!(l.revisits(), 0);
}

// 13. D4 detect_and_tune
#[test]
fn d4_13_detect_and_tune() {
    let mut l = DecisionSelfLoop::new();
    l.start();
    let mut m = HashMap::new();
    m.insert("hard_walls_pass".to_string(), "false".to_string());
    let t = l.detect_and_tune(&m);
    assert_eq!(t, DecisionTrigger::HardWallsFailed);
    assert_eq!(l.policy(), DecisionPolicy::Conservative);
}

// 14. D4 summary 引用 aGLM + superpowers
#[test]
fn d4_14_summary_cites_borrow_ids() {
    let s = decision_self_loop_summary();
    assert!(s.contains("R129-4 D4"));
    assert!(s.contains("aGLM-108"));
    assert!(s.contains("superpowers-234"));
    assert!(s.contains("✅"));
    assert!(s.contains("0 装 PASS 严守"));
}

// 15. D4 with_policy
#[test]
fn d4_15_with_policy() {
    let l = DecisionSelfLoop::with_policy(DecisionPolicy::Aggressive);
    assert_eq!(l.policy(), DecisionPolicy::Aggressive);
}
