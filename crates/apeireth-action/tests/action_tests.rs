//! 集成测试: apeireth-action 端到端路径 (执行 → 表达 → 沉默 → 回滚).
//!
//! 涵盖:
//! - execute_plan 路径 (含 12 键拒绝 + 成功应用)
//! - rollback_tx 路径 (含 NotFound 情况)
//! - express 多通道路径 (Text / Structured / MultiModal)
//! - silence 完整决策矩阵 (NotSilent / Deliberate / NoConsent / EthicalDoubt)
//! - 端到端: plan → execute → rollback → tx_log 回零

use apeireth_action::{
    ActionAtom, ActionEngine, ActionExecution, ActionExpression, ActionIntent, ActionPlan,
    ActionSilence, ExecutionResult, ExpressionChannel, RollbackResult, SilenceReason,
};
use apeireth_core::ActionTarget;

fn engine() -> ActionEngine {
    ActionEngine::new()
}

fn safe_target() -> ActionTarget {
    ActionTarget::NormalAction("noop".to_string())
}

#[test]
fn integration_execute_plan_full_lifecycle() {
    let eng = engine();
    let plan = ActionPlan::new(
        safe_target(),
        vec!["s1".to_string(), "s2".to_string()],
        "lifecycle",
    );

    let result = eng.execute_plan(&plan);
    assert!(result.is_applied(), "first execute must apply");
    assert_eq!(eng.tx_count(), 1);

    let tx = result.tx_id().expect("applied returns tx_id");
    let rollback = eng.rollback_tx(tx);
    assert!(rollback.is_rolled_back(), "rollback must succeed");
    assert_eq!(eng.tx_count(), 0, "tx_log must be empty after rollback");
}

#[test]
fn integration_rollback_unknown_tx_returns_not_found() {
    let eng = engine();
    let fake_tx = apeireth_action::new_tx_id();
    let res = eng.rollback_tx(fake_tx);
    assert!(matches!(res, RollbackResult::NotFound(_)));
}

#[test]
fn integration_execute_plan_rejects_modify_l0_ha() {
    let eng = engine();
    let plan = ActionPlan::new(
        ActionTarget::ModifyL0HA,
        vec!["forbidden".to_string()],
        "l0_attempt",
    );
    // 即使 steps 非空, 12 键 hardcode 也应拒绝
    let result = eng.execute_plan(&plan);
    assert!(result.is_failed(), "ModifyL0HA must be hardcode-blocked");
    assert_eq!(eng.tx_count(), 0, "rejected plans must NOT pollute tx_log");
}

#[test]
fn integration_express_multi_channel_routes_correctly() {
    let eng = engine();
    let intent = ActionIntent::new(safe_target()).with_body_hint("hello world");

    let text = eng.express(&intent, ExpressionChannel::Text);
    assert_eq!(text.channel, ExpressionChannel::Text);
    assert_eq!(text.text_payload(), "hello world");

    let structured = eng.express(&intent, ExpressionChannel::Structured);
    assert_eq!(structured.channel, ExpressionChannel::Structured);
    let json = structured.to_json().expect("json");
    assert!(json.contains("\"intent_id\""));
    assert!(json.contains("\"action\""));

    let multi = eng.express(&intent, ExpressionChannel::MultiModal);
    assert_eq!(multi.channel, ExpressionChannel::MultiModal);
    assert_eq!(multi.text_payload(), "hello world");
}

#[test]
fn integration_silence_decision_matrix() {
    let eng = engine();

    // NormalAction (无 SILENT 前缀) → NotSilent
    let normal = ActionIntent::new(safe_target());
    assert!(!eng.should_silence(&normal));
    assert_eq!(eng.reason_for_silence(&normal), SilenceReason::NotSilent);

    // NormalAction with "SILENT:" 前缀 → Deliberate
    let deliberate = ActionIntent::new(safe_target()).with_body_hint("SILENT: hold this");
    assert!(eng.should_silence(&deliberate));
    assert_eq!(
        eng.reason_for_silence(&deliberate),
        SilenceReason::Deliberate
    );

    // PretendPerfect → NoConsent
    let pretend = ActionIntent::new(ActionTarget::PretendPerfect);
    assert!(eng.should_silence(&pretend));
    assert_eq!(eng.reason_for_silence(&pretend), SilenceReason::NoConsent);

    // ModifyL0HA → EthicalDoubt
    let l0 = ActionIntent::new(ActionTarget::ModifyL0HA);
    assert!(eng.should_silence(&l0));
    assert_eq!(eng.reason_for_silence(&l0), SilenceReason::EthicalDoubt);

    // ReorganizeOnion → EthicalDoubt
    let reorganize = ActionIntent::new(ActionTarget::ReorganizeOnion);
    assert_eq!(
        eng.reason_for_silence(&reorganize),
        SilenceReason::EthicalDoubt
    );
}

#[test]
fn integration_dispatch_atom_wraps_to_plan() {
    let eng = engine();
    let atom = ActionAtom::new(safe_target(), "single_payload");
    let result = eng.dispatch_atom(atom);
    assert!(matches!(result, ExecutionResult::Applied(_)));
    assert_eq!(eng.tx_count(), 1);
}

#[test]
fn integration_priority_ordering_in_silence_reasons() {
    // SilenceReason::priority 应该把 EthicalDoubt 排最前 (高优先级 = 紧急)
    let reasons = [
        SilenceReason::NotSilent,
        SilenceReason::Deliberate,
        SilenceReason::NoNeed,
        SilenceReason::OutOfScope,
        SilenceReason::NoConsent,
        SilenceReason::EthicalDoubt,
    ];
    let max = reasons.iter().max_by_key(|r| r.priority()).copied();
    assert_eq!(max, Some(SilenceReason::EthicalDoubt));
    // 直接验证 priority 数值
    assert_eq!(SilenceReason::EthicalDoubt.priority(), 5);
    assert_eq!(SilenceReason::NotSilent.priority(), 0);
}
