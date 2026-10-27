//! 护栏接线**实证**：越权动作在生产治理栈上**到底被拦不拦**。
//!
//! # 这个文件回答什么问题
//!
//! `crates/engine/guard` 是「越权命中恒 0」这条安全红线的执行者。但
//! 「护栏写了这么多」**不等于**「护栏真在拦」—— 接线与否此前**无人验证过**。
//!
//! ⇒ 本文件**不读代码下结论**，而是**真的跑一遍生产形态的治理栈**：
//!
//! 1. 按生产装配的**同形**（`GovernanceChain` → `GovernancePipeline` → 护栏 hook）
//!    构造治理入口；
//! 2. 按生产路径的**同形**（网关 `RuleIntentInterpreter` 产意图 → runtime
//!    `GovernanceRequest` 带 `TurnSecurityContext`）造一个「用户只让检查代码、
//!    agent 却要改文件」的**越权动作**；
//! 3. 断言：**被拒**，且**写面没被碰到**（没落库 / 没改数据）。
//!
//! # 为什么能在这里跑
//!
//! 护栏 crate 的依赖里就有 `apeireth-governance`（治理栈本体）与 `apeireth-core`
//! ⇒ **不需要**新增任何依赖，也**不需要**改动任何 `lib.rs`。
//! 护栏 crate **之外**的装配面（CLI 装配根、网关入口、runtime 派发闸）用
//! `include_str!` 做**源码锚点**核对 —— 这是静态证据，与上面的运行时证据**分开**标注，
//! **不混为一谈**。
//!
//! # 诚实边界
//!
//! * 本文件**不**宣称「整个生产运行时端到端跑过一次越权拦截」——
//!   完整端到端要起真实 provider / 真实工具，超出本 crate 的依赖面；
//! * 本文件**能**宣称的是：**护栏 hook 在生产同形的治理栈上，对越权动作给出
//!   `Deny`，且 runtime 的派发闸（源码锚点）对 `Deny` 是「拒绝即帧、不派发」**。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, ChainGovernanceHook, Decision, GovernanceChain, GovernanceHook, GovernancePipeline,
    GovernanceRequest, GovernanceVerdict, TurnSecurityContext,
};
use apeireth_guard::{
    BehaviorChainGuardHook, GuardDecisionLog, IntentInput, IntentInterpreter, RuleIntentInterpreter,
};

// ── 生产装配面的源码锚点（静态证据） ────────────────────────────────────────
//
// 为什么用 `include_str!` 而不是新增依赖：`apeireth-cli` 依赖护栏 crate，
// 护栏 crate 再依赖 `apeireth-cli` 会构成**依赖环**（且会动 `Cargo.lock`，
// 撞上 `--locked` 门禁）。`include_str!` 是**编译期**读文件，不引入任何依赖。
const CLI_ROOT: &str = include_str!("../../../adapters/cli/src/lib.rs");
const GATEWAY_ENTRY: &str = include_str!("../../../adapters/gateway/src/canonical_entry.rs");
const RUNTIME_DISPATCH: &str = include_str!("../../../engine/runtime/src/canonical/execute.rs");

/// 生产形态的治理栈（护栏部分）。
///
/// # 为什么照抄这条链的形状
///
/// 生产上护栏**不是独立闸**：它挂在治理 pipeline 末尾，再被 `GovernanceChain`
/// 包一层（`cli::governance_chain::production()`）。若这里只 `new` 一个裸 hook，
/// 测出来的口径与生产**不同源** ⇒ 证据不作数。
struct ProductionStack {
    /// 护栏 hook 本身（要读事件 / 状态）。
    hook: Arc<BehaviorChainGuardHook>,
    /// 生产同形的 pipeline（护栏是**最后一道**闸）。
    pipeline: Arc<GovernancePipeline>,
    /// 生产同形的链入口（runtime 实际拿到的 `Arc<dyn GovernanceHook>`）。
    chain: Arc<ChainGovernanceHook>,
}

impl ProductionStack {
    fn new() -> Self {
        let hook = Arc::new(BehaviorChainGuardHook::new());
        let pipeline = Arc::new(GovernancePipeline::new().with(hook.clone()));
        let chain = Arc::new(GovernanceChain::new(pipeline.clone()));
        Self {
            hook,
            pipeline,
            chain: Arc::new(ChainGovernanceHook::new(chain)),
        }
    }
}

/// 按**网关生产路径**的同形造可信意图（`gateway/src/canonical_entry.rs`）。
///
/// 用户的原话只要求「检查 / 解释」⇒ 规则意图解释器判成**只读意图**
/// （`mutation_policy = Deny`）。这就是「越权」的**基线**：
/// 用户没授权任何写面。
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
///
/// 字段与顺序照抄 runtime：`Action::CapabilityDispatch` + `with_action_id` +
/// `with_optional_security_context`。
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

/// 复刻 runtime 的**派发闸**：允许 ⇒ 真写；拒绝 ⇒ 只落错误结果，不写。
///
/// # 为什么要有这个探针
///
/// 「被拒」只是一条 verdict；「**没改数据**」才是安全结论。
/// runtime 的派发闸（`execute.rs` 的 `Decision::Deny` 分支）是**拒绝即帧、
/// 不派发**——本探针照抄那条语义，用一个计数器把「有没有真的写」变成可断言的事实。
#[derive(Default)]
struct WriteProbe {
    writes: usize,
    last_error: Option<String>,
}

impl WriteProbe {
    async fn dispatch(&mut self, hook: &dyn GovernanceHook, request: &GovernanceRequest<'_>) {
        let verdict = hook.evaluate_verbose(request).await;
        match verdict.decision {
            Decision::Allow => self.writes += 1,
            Decision::Deny { reason } => {
                self.last_error = Some(format!("refused by governance: {reason}"));
            }
            Decision::RequireApproval { reason } => {
                self.last_error = Some(format!("pending approval: {reason}"));
            }
        }
    }
}

/// **核心实证**：只读意图下的写动作 ⇒ 生产治理栈**拒绝**，且**写面没被碰到**。
///
/// # 断言链
///
/// 1. 链入口（runtime 真正拿到的那个 hook）给出 `Deny`；
/// 2. pipeline 层给出 `Deny` 且**指名护栏**（`hook == "behavior_chain_guard"`）
///    ⇒ 拒绝**确实来自护栏**，不是被前面的授权闸拦掉的；
/// 3. 写探针计数为 0 ⇒ **没改数据**。
#[tokio::test]
async fn a_write_the_turn_intent_never_asked_for_is_denied_and_never_written() {
    let stack = ProductionStack::new();
    let session = SessionId::new();
    let trace = TraceId::new();
    let context = read_only_turn_context(&session);
    let capability = write_capability();
    let arguments = write_patch_arguments();
    let request = dispatch_request(&capability, &arguments, &context, session, trace);

    // ① 链入口（生产同形）必须拒绝。
    let chain_verdict: GovernanceVerdict = stack.chain.evaluate_verbose(&request).await;
    assert!(
        matches!(chain_verdict.decision, Decision::Deny { .. }),
        "只读意图下的写动作必须被拒，但链入口给出的是 {:?}",
        chain_verdict.decision
    );
    assert_eq!(
        chain_verdict.hook, "governance_chain",
        "生产同形的链入口就是 governance_chain（ADR-0010 §9）"
    );

    // ② pipeline 层：拒绝**指名护栏**（证明是护栏拦的，不是更前面的授权闸）。
    let pipeline_verdict = stack.pipeline.evaluate_verbose(&request).await;
    assert!(
        matches!(pipeline_verdict.decision, Decision::Deny { .. }),
        "pipeline 层也必须拒绝，得到的是 {:?}",
        pipeline_verdict.decision
    );
    assert_eq!(
        pipeline_verdict.hook, "behavior_chain_guard",
        "拒绝必须由护栏给出（否则说明护栏没接在这条路径上）"
    );
    let reason = pipeline_verdict
        .decision
        .reason()
        .unwrap_or_default()
        .to_string();
    assert!(
        reason.contains("alignment") || reason.contains("intent"),
        "拒绝理由应指向意图对齐，得到的是：{reason}"
    );

    // ②b 事件流里要能看到**为什么**拒：只读意图下的写动作 ⇒ 未请求的写。
    // （本次共评估两次：链入口一次、pipeline 一次 ⇒ 事件流两条，都是拒绝。）
    let events = stack.hook.recent_events(Some(10));
    assert_eq!(events.len(), 2, "两次评估应留下两条判定事件");
    assert!(
        events.iter().all(|event| event.decision == "deny"),
        "两条判定都应是拒绝，得到的是 {:?}",
        events
    );
    assert!(
        events.iter().any(|event| event
            .reasons
            .iter()
            .any(|reason| reason == "read_only_task_requested_mutation")),
        "拒绝理由应含「只读意图下的写动作」，得到的是 {:?}",
        events
    );

    // ③ 没改数据：写探针计数恒 0。
    let mut probe = WriteProbe::default();
    probe.dispatch(stack.chain.as_ref(), &request).await;
    assert_eq!(probe.writes, 0, "被拒的动作**不得**触达写面");
    assert!(
        probe
            .last_error
            .as_deref()
            .is_some_and(|message| message.starts_with("refused by governance")),
        "runtime 派发闸对 Deny 的语义是「拒绝即帧」，得到的是 {:?}",
        probe.last_error
    );
}

/// **实证（默认关能力不背锅）**：护栏的越权拦截**不依赖**任何默认关闭的开关。
///
/// # 为什么这条重要
///
/// 护栏有三块能力是**默认关**的：数据集记录器、ML 分类器、免疫记忆。
/// 若越权拦截要靠它们才生效 ⇒ 「默认部署下的越权命中恒 0」就不成立。
/// ⇒ 这里用**默认构造**（什么都不挂）再跑一遍：仍然拒。
#[tokio::test]
async fn the_deny_path_needs_no_opt_in_feature() {
    let hook = BehaviorChainGuardHook::new();
    let status = hook.status();
    assert!(
        !status.dataset_recording_enabled,
        "数据集记录器默认关（这是既有口径，本测试只作基线记录）"
    );
    assert!(
        !status.ml_classifier_available,
        "ML 分类器默认不可用（默认走确定性快/链守卫）"
    );

    let session = SessionId::new();
    let context = read_only_turn_context(&session);
    let capability = write_capability();
    let arguments = write_patch_arguments();
    let request = dispatch_request(&capability, &arguments, &context, session, TraceId::new());

    let verdict = hook.evaluate_verbose(&request).await;
    assert!(
        matches!(verdict.decision, Decision::Deny { .. }),
        "默认构造（无模型 / 无数据集 / 无免疫记忆）也必须拒，得到的是 {:?}",
        verdict.decision
    );
    assert_eq!(verdict.hook, "behavior_chain_guard");
}

/// **实证（审计面）**：拒绝会落进判定账 ⇒ 事后可追责。
///
/// 与生产同形：CLI 装配根用 `guard_decision_wiring::attach_decision_log` 挂同一个
/// `GuardDecisionLog`（append-only JSONL）。
#[tokio::test]
async fn a_denial_is_persisted_to_the_decision_ledger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("guard-decisions.jsonl");
    let log = GuardDecisionLog::open(&path).expect("open decision log");
    let hook = BehaviorChainGuardHook::new().with_decision_log(Arc::new(log));

    let session = SessionId::new();
    let context = read_only_turn_context(&session);
    let capability = write_capability();
    let arguments = write_patch_arguments();
    let request = dispatch_request(&capability, &arguments, &context, session, TraceId::new());

    let verdict = hook.evaluate_verbose(&request).await;
    assert!(matches!(verdict.decision, Decision::Deny { .. }));

    let log = GuardDecisionLog::open(&path).expect("reopen decision log");
    let events = log.read_recent(10).expect("read recent");
    assert_eq!(events.len(), 1, "账里应有一条判定");
    assert_eq!(events[0].decision, "deny", "落盘的是拒绝");
    assert_eq!(events[0].capability_id, "tool.apply_patch");
}

/// **缺口实证**：拒绝会**算出**收容义务（撤销会话授权 / 标记高风险 trace /
/// 补偿线索），但这些义务**只被带进事件流**，护栏 crate 之外**零消费者**
/// ⇒ 「算出来没人执行」。
///
/// # 这条为什么写进测试
///
/// 缺口清单里最靠前的一条就是它。写在这里 ⇒ 它**不是**「忘了写文档」，
/// 而是**可复核的现状**：义务确实被算出来（断言成立），也确实没有执行面
/// （由 `scripts/check_guard_wiring.py` 的台账条目 `enforcement.directive_consumer`
/// 机械核对）。
#[tokio::test]
async fn the_deny_produces_a_containment_directive_that_only_rides_the_event_stream() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let context = read_only_turn_context(&session);
    let capability = write_capability();
    let arguments = write_patch_arguments();
    let request = dispatch_request(&capability, &arguments, &context, session, TraceId::new());

    let verdict = hook.evaluate_verbose(&request).await;
    assert!(matches!(verdict.decision, Decision::Deny { .. }));

    let events = hook.recent_events(Some(10));
    assert_eq!(events.len(), 1, "应有且只有一条判定事件");
    let containment = events[0]
        .containment
        .as_ref()
        .expect("拒绝应带收容义务（当前实现如此）");
    assert!(
        containment.mark_trace_high_risk,
        "高风险 trace 标记义务应被算出"
    );
    assert!(
        containment.compensation_hint.is_some(),
        "补偿线索义务应被算出"
    );
    // 义务**只**出现在事件流里：护栏没有「执行撤销会话授权」的口子。
    // （执行面在护栏 crate 之外 ⇒ 台账条目 `enforcement.directive_consumer` = NOT WIRED。）
}

/// **接线实证（静态）**：生产装配根**真的**把护栏 hook 挂进了治理 pipeline。
///
/// # 为什么静态证据也要写进测试
///
/// 运行时证据只能证明「护栏会拒」；接线证据才证明「生产的治理栈里**有**护栏」。
/// 两者缺一不可。静态锚点用 `include_str!` 取真实生产源码 ⇒ 源码一改，
/// 这条断言就红 ⇒ 不会静默腐烂。
#[test]
fn the_production_composition_root_registers_the_guard_hook_in_the_governance_pipeline() {
    // CLI 装配根：护栏 hook 是 pipeline 的**最后一道**闸。
    assert!(
        CLI_ROOT.contains(".with(guard_hook.clone())"),
        "CLI 生产装配根必须把护栏 hook 挂进治理 pipeline"
    );
    assert!(
        CLI_ROOT.contains("BehaviorChainGuardHook::new()"),
        "CLI 生产装配根必须真的构造护栏 hook"
    );
    // 生产治理栈 = 治理 pipeline 被 `GovernanceChain` 包一层（ADR-0010 §9）。
    assert!(
        CLI_ROOT.contains("governance_chain::production("),
        "生产治理入口必须走 governance_chain::production（链单入口）"
    );
    // 治理 pipeline 交给 runtime 的那个口。
    assert!(
        CLI_ROOT.contains("builder.with_governance("),
        "生产装配根必须把治理栈交给 runtime（否则 runtime 默认 fail-closed 拒一切）"
    );
}

/// **接线实证（静态）**：网关生产入口**真的**按护栏口径造可信意图。
///
/// 没有意图 ⇒ 护栏的对齐层降级 ⇒ 越权判定会弱化（`intent_missing_assessment`）。
/// ⇒ 这一条是「越权能被识别」的**前置**接线。
#[test]
fn the_gateway_production_entry_builds_the_trusted_intent_the_guard_aligns_against() {
    assert!(
        GATEWAY_ENTRY.contains("RuleIntentInterpreter.interpret("),
        "网关生产入口必须用护栏的意图解释器造可信信封"
    );
    assert!(
        GATEWAY_ENTRY.contains("with_intent(intent)"),
        "可信意图必须绑进 TurnSecurityContext 送进 runtime"
    );
}

/// **接线实证（静态）**：runtime 派发闸对 `Deny` 是「拒绝即帧、不派发」。
///
/// # 为什么这条不能省
///
/// 护栏拒了**不够**——必须确认 runtime **不执行**这个动作。
/// 这一条把 runtime 的源码钉在测试里：runtime 改了派发语义 ⇒ 这条测试红。
#[test]
fn the_runtime_dispatch_gate_refuses_on_deny_without_dispatching() {
    assert!(
        RUNTIME_DISPATCH.contains("Decision::Deny { reason } => {"),
        "runtime 派发闸必须显式处理 Deny 分支"
    );
    assert!(
        RUNTIME_DISPATCH.contains("refused by governance:"),
        "runtime 派发闸对 Deny 的语义是「拒绝即帧」"
    );
    assert!(
        RUNTIME_DISPATCH.contains("ToolResult::permanent_error("),
        "被拒动作必须变成永久错误结果，而不是被派发出去"
    );
}
