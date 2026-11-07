//! 护栏**收容执行面**实证：同会话内**不能重复越权**。
//!
//! # 这个文件回答什么问题
//!
//! 审计（W-33，`docs/status/guard-wiring-audit-2026-10-27.md` §4-P0-1）实测：
//! `EnforcementDirective::from_decision` 早就算出了「撤销会话授权」的义务，
//! 但**护栏 crate 外零消费者** ⇒ ⇒ 全仓唯一 `PermissionPolicy::revoke` 是
//! **面板上的人工撤销** ⇒ ⇒ **同会话内可重复越权**
//! （一次高危越权被拒后，会话授权仍在 ⇒ 换个动作再来一次照样能过）。
//!
//! ⇒ 本文件**不**只测「义务被算出来」，而是**真的跑一遍**：
//!
//! 1. 只读意图下的越权写动作 ⇒ 生产同形的守门链**拒绝**；
//! 2. 护栏**发布**收容义务 ⇒ 执行方**消费**它 ⇒ **该会话的授权被撤销**；
//! 3. ⇒ **同会话再来一次** ⇒ **仍被拒**，且这次的理由是「**授权已撤销**」
//!    （不是「意图只读」）—— 这一条才是「重复越权被堵住」的**机器证明**；
//! 4. ⇒ **别的会话不受影响**（撤销**不跨会话**）。
//!
//! # 口径（四级）
//!
//! * **IMPLEMENTED**：收容指令 / 撤销状态 / 执行闸（`apeireth_guard::containment`）；
//! * **PRODUCTION WIRED**：CLI 装配根把它装进治理链入口（静态锚点见文末两条测试）；
//! * **DEFAULT ENABLED**：生产路径**默认装**（无 env 开关）；
//! * **HARDWARE VALIDATED**：**不适用**（纯软件判定链，无硬件面）。
//!
//! # 诚实边界
//!
//! * 本文件测的是**护栏 crate 内**的真实执行闸 + 生产同形的装配；
//!   CLI 装配根那侧用 `include_str!` 源码锚点核对（静态证据，与运行时证据**分开**标注）；
//! * 本文件**不**宣称「完整生产运行时端到端跑过一次越权拦截」—— 那要起真实
//!   provider 与真实工具，超出护栏 crate 的依赖面。

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, ChainGovernanceHook, Decision, GovernanceChain, GovernanceHook, GovernancePipeline,
    GovernanceRequest, GovernanceVerdict, TurnSecurityContext,
};
use apeireth_guard::{
    containment::{
        apply, ContainmentConsumer, ContainmentDirective, ContainmentGate, GrantRevocations,
    },
    BehaviorChainGuardHook, EnforcementDirective, IntentInput, IntentInterpreter,
    RuleIntentInterpreter,
};

// ── 生产装配面的源码锚点（静态证据） ────────────────────────────────────────
//
// 与 `wiring_evidence.rs` 同一手法：`include_str!` 是编译期读文件，不引入依赖。
const CLI_ROOT: &str = include_str!("../../../adapters/cli/src/lib.rs");

/// 测试内**执行方**（生产同形：CLI 装配根的 `SessionGrantContainmentConsumer`）。
///
/// # 为什么测试里也有一份
///
/// 护栏 crate **不能**依赖 CLI（分层：护栏不该知道上层怎么执行）⇒ 测试无法直接
/// 引用 CLI 的执行方 ⇒ 这里按**同一形状**实现一份（同一套护栏 crate 的
/// `ContainmentDirective::from_enforcement` + `apply`）⇒ **口径同源**，
/// 且 CLI 那侧由文末的**源码锚点**钉住。
struct TestContainmentConsumer {
    revocations: Arc<Mutex<GrantRevocations>>,
    consumed: Mutex<Vec<ConsumedObligation>>,
}

/// 一次被消费的收容义务（可断言「消费了几条、撤销没撤销」）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConsumedObligation {
    session_id: String,
    revoke_session_grant: bool,
}

impl TestContainmentConsumer {
    fn new(revocations: Arc<Mutex<GrantRevocations>>) -> Self {
        Self {
            revocations,
            consumed: Mutex::new(Vec::new()),
        }
    }

    fn consumed(&self) -> Vec<ConsumedObligation> {
        self.consumed.lock().expect("consumed lock").clone()
    }
}

impl ContainmentConsumer for TestContainmentConsumer {
    fn consume(&self, obligation: &EnforcementDirective, session_id: &str) {
        let directive = ContainmentDirective::from_enforcement(obligation, session_id);
        let mut revocations = self.revocations.lock().expect("revocations lock");
        apply(&mut revocations, &directive);
        self.consumed
            .lock()
            .expect("consumed lock")
            .push(ConsumedObligation {
                session_id: session_id.to_string(),
                revoke_session_grant: directive.revoke_session_grant.is_some(),
            });
    }

    fn is_revoked(&self, session_id: &str) -> bool {
        self.revocations
            .lock()
            .expect("revocations lock")
            .is_revoked(session_id)
    }
}

/// 生产同形的治理栈（护栏 + **收容执行闸**）。
///
/// # 形状照抄生产
///
/// 生产（CLI 装配根）：`runtime → governance_chain → 收容执行闸 → 会话权限预设
/// → 治理 pipeline → … → 护栏（末位）`。
/// 这里照抄同一形状 ⇒ 测出来的口径与生产**同源**。
struct ProductionStack {
    /// 护栏 hook 本身（要读事件 / 状态）。
    hook: Arc<BehaviorChainGuardHook>,
    /// 收容执行闸（护栏 crate 内的真实实现）。
    gate: Arc<ContainmentGate>,
    /// 生产同形的链入口（runtime 实际拿到的 `Arc<dyn GovernanceHook>`）。
    chain: Arc<ChainGovernanceHook>,
    /// 执行方（撤销落点）。
    consumer: Arc<TestContainmentConsumer>,
    /// **会话授权撤销状态**（执行方持有的权威状态）。
    revocations: Arc<Mutex<GrantRevocations>>,
}

impl ProductionStack {
    fn new() -> Self {
        let hook = Arc::new(BehaviorChainGuardHook::new());
        let pipeline = Arc::new(GovernancePipeline::new().with(hook.clone()));
        let revocations = Arc::new(Mutex::new(GrantRevocations::default()));
        let consumer = Arc::new(TestContainmentConsumer::new(Arc::clone(&revocations)));
        let gate = Arc::new(ContainmentGate::new(
            pipeline.clone(),
            Arc::clone(&hook),
            consumer.clone(),
        ));
        let chain = Arc::new(GovernanceChain::new(gate.clone()));
        Self {
            hook,
            gate,
            chain: Arc::new(ChainGovernanceHook::new(chain)),
            consumer,
            revocations,
        }
    }

    fn is_revoked(&self, session: &SessionId) -> bool {
        self.revocations
            .lock()
            .expect("revocations lock")
            .is_revoked(&session.to_string())
    }
}

/// 按**网关生产路径**的同形造可信意图（`gateway/src/canonical_entry.rs`）。
///
/// 用户的原话只要求「检查 / 解释」⇒ 规则意图解释器判成**只读意图**
/// （`mutation_policy = Deny`）。这就是「越权」的**基线**：用户没授权任何写面。
fn read_only_turn_context(session: &SessionId) -> TurnSecurityContext {
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: session.to_string(),
        trace_id: String::new(),
        user_request: "检查 src/main.rs 的逻辑并解释一下".to_string(),
        created_at_ms: 0,
    });
    TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent)
}

/// 按**runtime 派发点**的同形造请求（`runtime/src/canonical/execute.rs`）。
fn dispatch_request<'a>(
    capability: &'a CapabilityId,
    arguments: &'a serde_json::Value,
    context: &'a TurnSecurityContext,
    session: SessionId,
    trace: TraceId,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        session,
        trace,
        1,
    )
    .with_action_id("call-1")
    .with_optional_security_context(Some(context))
}

/// 写面参数：一个**真实**的补丁（不是空参数）。
fn write_patch_arguments() -> serde_json::Value {
    serde_json::json!({
        "patch": "*** Begin Patch\n*** Update File: src/main.rs\n@@\n-old\n+NEW\n*** End Patch"
    })
}

/// 写面能力 id（受控文件写入面，在护栏的能力安全注册表里有显式描述符）。
fn write_capability() -> CapabilityId {
    CapabilityId::new("tool.apply_patch").expect("capability id")
}

/// 越权写动作在**只读意图**下的判定（生产同形的链入口）。
async fn escalate(
    stack: &ProductionStack,
    session: SessionId,
    trace: TraceId,
) -> GovernanceVerdict {
    let context = read_only_turn_context(&session);
    let capability = write_capability();
    let arguments = write_patch_arguments();
    let request = dispatch_request(&capability, &arguments, &context, session, trace);
    stack.chain.evaluate_verbose(&request).await
}

/// **实证（义务已发布）**：高危拒绝 ⇒ 护栏**发布**「撤销会话授权」的收容义务。
///
/// 这一条只证明「护栏**发出**了指令」；**真正**的结论在下面两条
/// （执行方消费了 ⇒ 同会话再来一次仍被拒）。
#[tokio::test]
async fn the_high_risk_denial_publishes_a_revoke_the_session_grant_obligation() {
    let stack = ProductionStack::new();
    let session = SessionId::new();

    let verdict = escalate(&stack, session, TraceId::new()).await;
    assert!(
        matches!(verdict.decision, Decision::Deny { .. }),
        "只读意图下的写动作必须被拒，得到的是 {:?}",
        verdict.decision
    );

    let events = stack.hook.recent_events(Some(10));
    assert_eq!(events.len(), 1, "一次评估应留下一条判定事件");
    let obligation = events[0]
        .containment
        .as_ref()
        .expect("高危拒绝必须发布收容义务");
    assert!(
        obligation.revoke_session_grant,
        "只读意图下的越权写动作必须带「撤销会话授权」义务（否则收容执行面无从撤销）"
    );
    assert!(
        obligation.mark_trace_high_risk,
        "高风险 trace 标记义务应同时发布"
    );
    assert!(
        obligation.compensation_hint.is_some(),
        "补偿线索义务应同时发布（由别的执行面处理，不静默丢）"
    );
}

/// **实证（执行面消费了）**：护栏发布的义务 ⇒ 执行方消费 ⇒ **该会话授权被撤销**。
#[tokio::test]
async fn the_executor_revokes_the_session_grant_after_the_high_risk_denial() {
    let stack = ProductionStack::new();
    let session = SessionId::new();

    escalate(&stack, session, TraceId::new()).await;

    assert!(
        stack.is_revoked(&session),
        "高危拒绝后该会话的授权必须被撤销（此前：拒绝了但授权还在 ⇒ 可重复越权）"
    );
    let consumed = stack.consumer.consumed();
    assert_eq!(consumed.len(), 1, "执行方应消费到一条收容义务");
    assert_eq!(consumed[0].session_id, session.to_string());
    assert!(
        consumed[0].revoke_session_grant,
        "消费到的义务必须真的指向「撤销会话授权」"
    );
    assert_eq!(
        stack.gate.consumed_obligations(),
        1,
        "执行闸应记录已消费的义务数（可观测）"
    );
}

/// **核心实证**：同会话**再来一次** ⇒ **仍被拒**，且理由是「**授权已撤销**」。
///
/// # 这才是「重复越权被堵住」的机器证明
///
/// 只测「义务被发出」不算数 —— 必须证明**再来一次仍然过不去**。
#[tokio::test]
async fn the_same_session_cannot_repeat_the_privilege_escalation_after_the_revoke() {
    let stack = ProductionStack::new();
    let session = SessionId::new();

    // ① 第一次：只读意图下的越权写动作 ⇒ 拒绝。
    let first = escalate(&stack, session, TraceId::new()).await;
    assert!(matches!(first.decision, Decision::Deny { .. }));
    let first_reason = first.decision.reason().unwrap_or_default().to_string();
    assert!(
        first_reason.contains("alignment") || first_reason.contains("intent"),
        "第一次拒绝的理由应指向意图对齐（证明拦它的是护栏的对齐层），得到的是：{first_reason}"
    );

    // ② 同会话**再来一次**（新的回合 = 新的 trace）⇒ 仍被拒。
    let second = escalate(&stack, session, TraceId::new()).await;
    assert!(
        matches!(second.decision, Decision::Deny { .. }),
        "同会话再来一次必须仍被拒，得到的是 {:?}",
        second.decision
    );

    // ③ 这次的理由是「**授权已撤销**」，**不是**「意图只读」。
    let second_reason = second.decision.reason().unwrap_or_default().to_string();
    assert!(
        second_reason.contains("session grant revoked"),
        "第二次拒绝的理由必须是「授权已撤销」，得到的是：{second_reason}"
    );
    assert!(
        !second_reason.contains("alignment"),
        "第二次拒绝不应再以「意图对齐」为理由（那说明收容义务没被执行），得到的是：{second_reason}"
    );

    // 机器证明的**原样账**（跑一遍就能看到「第一次为什么拒 / 第二次为什么拒」）：
    println!(
        "[containment-evidence] session={session}\n\
         [containment-evidence]   第 1 次拒绝理由: {first_reason}\n\
         [containment-evidence]   撤销后授权状态: revoked={}\n\
         [containment-evidence]   第 2 次拒绝理由: {second_reason}",
        stack.is_revoked(&session)
    );
}

/// **实证（撤销不跨会话）**：别的会话**不受影响**。
#[tokio::test]
async fn the_revocation_does_not_reach_the_other_sessions() {
    let stack = ProductionStack::new();
    let session_a = SessionId::new();
    let session_b = SessionId::new();

    // A 越权 ⇒ 拒绝 ⇒ A 被撤销。
    escalate(&stack, session_a, TraceId::new()).await;
    assert!(
        stack.is_revoked(&session_a),
        "A 触发了高危拒绝 ⇒ A 的授权被撤销"
    );

    // B 尚未触碰 ⇒ **不受影响**。
    assert!(
        !stack.is_revoked(&session_b),
        "撤销不得跨会话（B 不该因为 A 的越权而失去授权）"
    );

    // B 的动作仍按「意图只读」被拒（理由**不是**「授权已撤销」）。
    let verdict_b = escalate(&stack, session_b, TraceId::new()).await;
    assert!(matches!(verdict_b.decision, Decision::Deny { .. }));
    let reason_b = verdict_b.decision.reason().unwrap_or_default().to_string();
    assert!(
        !reason_b.contains("session grant revoked"),
        "B 的拒绝理由不得是「授权已撤销」（撤销不跨会话），得到的是：{reason_b}"
    );
    assert!(
        reason_b.contains("alignment") || reason_b.contains("intent"),
        "B 的拒绝理由应仍是意图对齐，得到的是：{reason_b}"
    );
}

/// **实证（撤销是显式的，没有自动恢复）**：撤销**不随回合/时间失效**。
///
/// # 为什么这条重要
///
/// 若撤销会在下一次评估时「自动恢复」，那么「同会话再来一次仍被拒」只在**紧邻**
/// 的那一次成立 ⇒ 换个回合照样能越权 ⇒ 洞没堵住。
#[tokio::test]
async fn a_revoked_session_is_not_restored_automatically() {
    let stack = ProductionStack::new();
    let session = SessionId::new();

    escalate(&stack, session, TraceId::new()).await;
    assert!(stack.is_revoked(&session));

    // 换回合、换 trace ⇒ 仍被拒（撤销只认会话，不认回合）。
    let later = escalate(&stack, session, TraceId::new()).await;
    assert!(matches!(later.decision, Decision::Deny { .. }));
    let reason = later.decision.reason().unwrap_or_default().to_string();
    assert!(
        reason.contains("session grant revoked"),
        "撤销不得自动恢复，得到的是：{reason}"
    );
}

/// **实证（不再回头问护栏）**：已撤销的会话 ⇒ 执行闸**直接拒**，不再进护栏判定。
///
/// # 为什么这条重要
///
/// 「撤销后仍去问护栏」⇒ 护栏若因状态变化给出放行 ⇒ 撤销形同虚设 ⇒
/// fail-closed 要求**先看撤销状态**。
#[tokio::test]
async fn a_revoked_session_is_refused_before_the_guard_is_consulted_again() {
    let stack = ProductionStack::new();
    let session = SessionId::new();

    escalate(&stack, session, TraceId::new()).await;
    let events_before = stack.hook.recent_events(Some(50)).len();

    escalate(&stack, session, TraceId::new()).await;

    let events_after = stack.hook.recent_events(Some(50)).len();
    assert_eq!(
        events_after, events_before,
        "已撤销的会话不应再进护栏判定（执行闸先拒 ⇒ fail-closed）"
    );
}

/// **接线实证（静态）**：CLI 生产装配根**真的**把收容执行闸装进治理链入口。
///
/// 运行时证据只能证明「收容闸会撤权」；接线证据才证明「生产的治理链里**有**它」。
/// 两者缺一不可。静态锚点取真实生产源码 ⇒ 源码一改，这条断言就红 ⇒ 不会静默腐烂。
#[test]
fn the_cli_composition_root_mounts_the_containment_executor_on_the_governance_entry() {
    assert!(
        CLI_ROOT.contains("ContainmentGate::new("),
        "CLI 生产装配根必须构造收容执行闸（否则义务算了没人执行）"
    );
    assert!(
        CLI_ROOT.contains("governance_chain::production(containment)"),
        "收容执行闸必须装在治理链入口（runtime 真正问的那一个 hook）"
    );
    assert!(
        CLI_ROOT.contains("impl ContainmentConsumer for SessionGrantContainmentConsumer"),
        "CLI 侧必须有执行方（护栏 crate 外消费收容义务）"
    );
}

/// **接线实证（静态）**：执行面在**护栏 crate 外**消费护栏算出的收容义务。
///
/// 台账条目 `enforcement.directive_consumer` 的锚点就是 `EnforcementDirective`
/// ⇒ 这一条把「护栏 crate 外有真消费者」钉在源码上（`scripts/check_guard_wiring.py`
/// 会用同一锚点机械核对 ⇒ 台账与代码对不上就红）。
#[test]
fn the_execution_surface_outside_the_guard_crate_consumes_the_enforcement_directive() {
    assert!(
        CLI_ROOT.contains("fn consume(&self, obligation: &EnforcementDirective, session_id: &str)"),
        "CLI 侧执行方必须按护栏算出的义务口径消费（不得自己另算阈值）"
    );
    assert!(
        CLI_ROOT.contains("ContainmentDirective::from_enforcement(obligation, session_id)"),
        "执行方必须经护栏 crate 的口径桥把义务变成可执行指令"
    );
}

/// **接线实证（静态）**：本批**没有**动 pipeline 的装配顺序（审计 P0-2 的事）。
///
/// 护栏仍是 pipeline 的**末位**闸；收容执行闸装在**治理链入口外侧**，
/// 不进 pipeline ⇒ 上游闸拒绝掉的动作仍然不进护栏行为链（那条洞本批不碰）。
#[test]
fn the_pipeline_assembly_order_is_untouched_by_this_batch() {
    assert!(
        CLI_ROOT.contains(".with(guard_hook.clone())"),
        "护栏 hook 仍应是治理 pipeline 的末位闸（P0-2 本批不碰）"
    );
}
