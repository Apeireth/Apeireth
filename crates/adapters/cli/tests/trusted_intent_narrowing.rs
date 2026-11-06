//! C1 接线验收: `constrain_to_trusted`（**模型提案 → 用户可信信封收窄**）在生产
//! 治理链里真的起作用。
//!
//! # 行为差异（接线前不可用 / 接线后可用）
//!
//! - **接线前**（只有内层闸：授权 / 审批档 / 护栏）：护栏的对齐层只对**少数高危类**
//!   升级，**良性但未被请求的操作类**（最典型：只读意图下的 `NetworkRead`）落在
//!   0.5 分档 ⇒ **不升级** ⇒ 模型自己去联网抓 ⇒ **放行**。
//! - **接线后**（链的 Gate 段装了收窄闸）：模型提案被收窄到用户可信信封 ⇒ 提案
//!   被削空 ⇒ 超出可信意图 ⇒ **停人工审批**。
//!
//! # 反向护栏（不许无谓收紧）
//!
//! - 用户明说「改代码」时模型改代码 ⇒ 收窄后仍有「改」这一族 ⇒ **不新增审批**。
//! - 内层闸已拒 / 已要审批 ⇒ 收窄闸**原样返回**（只收紧、不放宽）。
//! - 运行时内部能力（`tool.behavior.*`）⇒ 豁免（不是模型提案）。

use std::sync::Arc;

use apeireth_cli::governance_chain::{adapt_gate_with, ProposalNarrowingGate};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, InMemoryAuditSink, TurnSecurityContext,
};
use apeireth_guard::{IntentInput, IntentInterpreter, RuleIntentInterpreter};
use serde_json::Value;

/// 恒放行的内层闸（把「收窄闸自己带来的差异」与内层判定分开）。
struct AllowAll;

#[async_trait::async_trait]
impl GovernanceHook for AllowAll {
    fn name(&self) -> &str {
        "allow_all"
    }

    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::Allow
    }
}

/// 恒拒绝的内层闸（验证收窄闸**不放宽**既有判定）。
struct DenyAll;

#[async_trait::async_trait]
impl GovernanceHook for DenyAll {
    fn name(&self) -> &str {
        "deny_all"
    }

    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::deny("内层闸拒绝")
    }
}

/// 用户请求 → 可信意图信封（生产同一条解释路径：`RuleIntentInterpreter`）。
fn trusted_intent(user_request: &str) -> TurnSecurityContext {
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: "session-under-test".to_string(),
        trace_id: "trace-under-test".to_string(),
        user_request: user_request.to_string(),
        created_at_ms: 0,
    });
    TurnSecurityContext::new(intent.intent_id.clone(), "trace-under-test").with_intent(intent)
}

/// 一次能力派发的判定。
async fn decide(
    hook: &Arc<dyn GovernanceHook>,
    capability: &CapabilityId,
    arguments: &Value,
    context: &TurnSecurityContext,
) -> Decision {
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
    .with_security_context(context);
    hook.evaluate(&request).await
}

/// **接线前不可用 / 接线后可用**: 只读意图 + 模型联网 ⇒ 接线前放行，接线后停人工审批。
#[tokio::test]
async fn a_model_proposal_beyond_the_trusted_intent_stops_for_human_approval() {
    let inner: Arc<dyn GovernanceHook> = Arc::new(AllowAll);
    let context = trusted_intent("看一下这个文件");
    let capability = CapabilityId::new("tool.fetch").expect("valid capability id");
    let arguments = Value::Null;

    // 接线前: 只有内层闸 ⇒ 放行（护栏对齐层对良性未请求类不升级）。
    let before = decide(&inner, &capability, &arguments, &context).await;
    assert_eq!(
        before,
        Decision::Allow,
        "接线前（无收窄闸）这一提案是放行的 —— 这就是接线前的行为"
    );

    // 接线后: 收窄闸 ⇒ 提案被收窄到可信信封 ⇒ 削空 ⇒ 停人工审批。
    let gated: Arc<dyn GovernanceHook> = Arc::new(ProposalNarrowingGate::new(Arc::clone(&inner)));
    let after = decide(&gated, &capability, &arguments, &context).await;
    assert!(
        matches!(after, Decision::RequireApproval { .. }),
        "接线后必须停人工审批（不许默认放行）: {after:?}"
    );
}

/// **生产装配真的带上了收窄闸**（`adapt_gate_with` = 生产链入口的装配面）。
#[tokio::test]
async fn the_production_chain_entry_carries_the_narrowing_gate() {
    let inner: Arc<dyn GovernanceHook> = Arc::new(AllowAll);
    let chain = adapt_gate_with(inner, Arc::new(InMemoryAuditSink::new()));
    let context = trusted_intent("看一下这个文件");
    let capability = CapabilityId::new("tool.fetch").expect("valid capability id");
    let arguments = Value::Null;

    let decision = decide(&chain, &capability, &arguments, &context).await;
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "生产链入口必须带收窄闸: {decision:?}"
    );
}

/// **不无谓收紧**: 用户明说「改代码」时模型改代码 ⇒ 不新增审批。
#[tokio::test]
async fn a_proposal_inside_the_trusted_intent_is_not_escalated() {
    let inner: Arc<dyn GovernanceHook> = Arc::new(AllowAll);
    let gated: Arc<dyn GovernanceHook> = Arc::new(ProposalNarrowingGate::new(Arc::clone(&inner)));
    let context = trusted_intent("修改一下这个文件");
    let capability = CapabilityId::new("tool.apply_patch").expect("valid capability id");
    let arguments = Value::Null;

    let decision = decide(&gated, &capability, &arguments, &context).await;
    assert_eq!(
        decision,
        Decision::Allow,
        "用户明说要改 ⇒ 提案在可信意图内 ⇒ 收窄闸不得无谓收紧"
    );
}

/// **只收紧、不放宽**: 内层闸已拒 ⇒ 收窄闸原样返回拒绝。
#[tokio::test]
async fn the_narrowing_gate_never_relaxes_an_existing_refusal() {
    let inner: Arc<dyn GovernanceHook> = Arc::new(DenyAll);
    let gated: Arc<dyn GovernanceHook> = Arc::new(ProposalNarrowingGate::new(Arc::clone(&inner)));
    let context = trusted_intent("看一下这个文件");
    let capability = CapabilityId::new("tool.fetch").expect("valid capability id");
    let arguments = Value::Null;

    let decision = decide(&gated, &capability, &arguments, &context).await;
    assert!(
        matches!(decision, Decision::Deny { .. }),
        "内层已拒 ⇒ 收窄闸不得放宽: {decision:?}"
    );
}

/// **运行时内部能力豁免**: `tool.behavior.*` 不是模型提案 ⇒ 不按意图对齐收窄。
#[tokio::test]
async fn runtime_internal_capabilities_are_exempt_from_narrowing() {
    let inner: Arc<dyn GovernanceHook> = Arc::new(AllowAll);
    let gated: Arc<dyn GovernanceHook> = Arc::new(ProposalNarrowingGate::new(Arc::clone(&inner)));
    let context = trusted_intent("看一下这个文件");
    let capability =
        CapabilityId::new("tool.behavior.cognitive.memory_writeback").expect("valid capability id");
    let arguments = Value::Null;

    let decision = decide(&gated, &capability, &arguments, &context).await;
    assert_eq!(
        decision,
        Decision::Allow,
        "运行时自己的认知调用不是模型提案 ⇒ 不得被意图收窄拦下"
    );
}

/// **没有可信意图 ⇒ 不收窄**（意图缺失那一档由护栏的 fail-closed 分级负责）。
#[tokio::test]
async fn without_a_trusted_intent_there_is_nothing_to_narrow() {
    let inner: Arc<dyn GovernanceHook> = Arc::new(AllowAll);
    let gated: Arc<dyn GovernanceHook> = Arc::new(ProposalNarrowingGate::new(Arc::clone(&inner)));
    let capability = CapabilityId::new("tool.fetch").expect("valid capability id");
    let arguments = Value::Null;
    let context = TurnSecurityContext::new("intent:trace", "trace");

    let decision = decide(&gated, &capability, &arguments, &context).await;
    assert_eq!(
        decision,
        Decision::Allow,
        "没有可信意图就没有可比的天花板 ⇒ 收窄闸不动手"
    );
}
