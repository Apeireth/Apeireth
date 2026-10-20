//! 生物机制族 W3.5 · 生命周期装配面接线测试（SG-18，`APEIRETH_ENABLE_BIO_LIFECYCLE`）。
//!
//! 覆盖装配级契约（不是内核单测的重复）：
//! ① env 开关只认 `"1"`，其余 fail-closed；关 = 不构造、一次时钟都不读；
//! ② 开关关（defense-in-depth）= 所有钩子 `continue_()`、零 overlay、账本不动；
//! ③ 开关开 = 纯观测模块，所有钩子仍 `continue_()`（不改变回合可观察行为）；
//! ④ 真实信号链：`AfterToolResult` 工具结果 → 账本 → `AfterTurn` 迁移；
//! ⑤ 工具名解析（结果名优先、回退调用名、未注册工具首次出现即入账）；
//! ⑥ 失败证据只在超过试用失败上限后退役（内核语义），留墓碑；
//! ⑦ 闲置退役走注入时钟 + 冷静期（不读系统时钟）；
//! ⑧ 已退役资产被真实使用不静默复活（拒绝计数 + 留墓碑）；
//! ⑨ 容量淘汰计数非静默；⑩ 观测确定性与 self_status 探测口只读；
//! ⑪ `BioMechanismKind::Lifecycle` 注册后自述格 = `wired` 真值。
//!
//! **0 装诚实**：`production.rs` 未注册本模块（Lead 总装）；无 live E2E；
//! 技能/记忆/插件三类资产无真实信号源，本测试也不编造。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_orchestration::lifecycle::{
    LifecyclePolicy, LifecycleStage, LifecycleTransition, RetireReason,
    DEFAULT_RETIRE_AFTER_UNUSED_MS,
};
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedResponse, ToolCall, ToolResult};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_lifecycle::{
    bio_lifecycle_enabled_from_env_value, bio_lifecycle_probe, bio_lifecycle_wiring_from_env_value,
    BioLifecycleModule, BioLifecycleState, LifecycleClock, BIO_LIFECYCLE_ENABLE_ENV,
    BIO_LIFECYCLE_MODULE_ID,
};
use apeireth_runtime_assembly::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleDirective, ModuleError,
    ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
    ModuleOutcome,
};
use apeireth_runtime_assembly::canonical::self_status_source::BioMechanismRegistry;
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_tools_canonical::self_status::{BioMechanismKind, BIO_STAGE_WIRED};
use async_trait::async_trait;

/// 所有测试钩子可枚举的点（含两个真实数据钩子）。
const ALL_HOOKS: [HookPoint; 8] = [
    HookPoint::TurnStart,
    HookPoint::BeforeModelCall,
    HookPoint::AfterModelResponse,
    HookPoint::BeforeToolCall,
    HookPoint::AfterToolResult,
    HookPoint::BeforeFinalCommit,
    HookPoint::AfterTurn,
    HookPoint::OnError,
];

const NOW_MS: u64 = 1_760_000_000_000;
const DAY_MS: u64 = 24 * 60 * 60 * 1_000;

/// 可步进 + 计数读数的注入时钟（测试永不读系统时钟）。
struct TestClock {
    now: Arc<AtomicU64>,
    reads: Arc<AtomicU64>,
}

impl TestClock {
    fn new(at_ms: u64) -> Self {
        Self {
            now: Arc::new(AtomicU64::new(at_ms)),
            reads: Arc::new(AtomicU64::new(0)),
        }
    }

    fn clock(&self) -> LifecycleClock {
        let now = Arc::clone(&self.now);
        let reads = Arc::clone(&self.reads);
        Arc::new(move || {
            reads.fetch_add(1, Ordering::SeqCst);
            now.load(Ordering::SeqCst)
        })
    }

    fn set(&self, at_ms: u64) {
        self.now.store(at_ms, Ordering::SeqCst);
    }

    fn reads(&self) -> u64 {
        self.reads.load(Ordering::SeqCst)
    }
}

struct NoopInvoker;

#[async_trait]
impl ModuleInvoker for NoopInvoker {
    async fn invoke(
        &self,
        _request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        Ok(ModuleInvocationResponse {
            response: NormalizedResponse::text("assistant", "test-model", ""),
            served_by: CapabilityId::new("provider.test").expect("capability id"),
        })
    }
}

struct NoSubLoop;

#[async_trait]
impl SubLoopSpawner for NoSubLoop {
    async fn spawn(&self, _spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        Err(SubLoopError::NoModel)
    }
}

static NO_SUB_LOOP: NoSubLoop = NoSubLoop;

fn call(name: &str) -> ToolCall {
    ToolCall {
        id: format!("call_{name}"),
        name: name.to_string(),
        arguments: serde_json::json!({}),
    }
}

fn ok_result(name: &str) -> ToolResult {
    ToolResult::ok(format!("call_{name}"), serde_json::json!({"ok": true})).with_name(name)
}

fn error_result(name: &str) -> ToolResult {
    ToolResult::permanent_error(format!("call_{name}"), "boom").with_name(name)
}

async fn run_hook(
    module: &dyn BehaviorModule,
    hook: HookPoint,
    tool_call: Option<&ToolCall>,
    tool_result: Option<&ToolResult>,
) -> Result<ModuleOutcome, ModuleError> {
    let session = SessionId::new();
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let messages = [NormalizedMessage::user("CANARY-USER-TEXT-bio-lifecycle")];
    let ctx = ModuleContext {
        session_id: &session,
        model: "test-model",
        messages: &messages,
        candidate: None,
        tool_call,
        tool_result,
        invocation: &invocation,
        module_id: module.manifest().id.as_str(),
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module.on_hook(hook, &ctx).await
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

// ============================================
// ① env 开关：只认 "1"，默认关，关 = 不构造、不读时钟
// ============================================

#[test]
fn env_gate_is_fail_closed_and_lazy() {
    assert_eq!(BIO_LIFECYCLE_ENABLE_ENV, "APEIRETH_ENABLE_BIO_LIFECYCLE");
    assert!(bio_lifecycle_enabled_from_env_value(Some("1")));
    assert!(bio_lifecycle_enabled_from_env_value(Some(" 1 ")));

    let clock = TestClock::new(NOW_MS);
    for raw in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("TRUE"),
        Some("yes"),
        Some("on"),
        Some("2"),
        Some("01"),
        Some("1.0"),
    ] {
        assert!(
            !bio_lifecycle_enabled_from_env_value(raw),
            "非 \"1\" 一律关: {raw:?}"
        );
        assert!(
            bio_lifecycle_wiring_from_env_value(raw, clock.clock(), &names(&["filesystem"]))
                .is_none(),
            "关 = 不构造: {raw:?}"
        );
    }
    assert_eq!(clock.reads(), 0, "关 = 一次时钟都不读 (懒加载)");

    let wiring = bio_lifecycle_wiring_from_env_value(
        Some("1"),
        clock.clock(),
        &names(&["filesystem", "  ", "filesystem"]),
    )
    .expect("开关开");
    assert_eq!(wiring.module().manifest().id, BIO_LIFECYCLE_MODULE_ID);
    assert_eq!(
        wiring.state().observation().assets_total,
        1,
        "空白名跳过、重复名只注册一次"
    );
    assert_eq!(clock.reads(), 1, "开 = 播种恰好读一次注入时钟");
}

// ============================================
// ② 开关关（defense-in-depth）：所有钩子零变化
// ============================================

#[tokio::test]
async fn a_disabled_module_is_zero_change_on_every_hook() {
    let clock = TestClock::new(NOW_MS);
    let state = Arc::new(BioLifecycleState::new(clock.clock()));
    state.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    let before = state.observation();
    let module = BioLifecycleModule::new(false, Arc::clone(&state));

    let call = call("filesystem");
    let result = ok_result("filesystem");
    for hook in ALL_HOOKS {
        let outcome = run_hook(&module, hook, Some(&call), Some(&result))
            .await
            .expect("hook");
        assert_eq!(
            outcome,
            ModuleOutcome::continue_(),
            "开关关 = 既有行为逐字节一致 ({hook:?})"
        );
    }
    assert_eq!(clock.reads(), 0, "关 = 一个钩子都不读时钟");
    assert_eq!(state.observation(), before, "关 = 账本一个字节都不动");
}

// ============================================
// ③ 开关开 = 纯观测：钩子恒 continue_，零 overlay / 零指令
// ============================================

#[tokio::test]
async fn an_enabled_module_never_changes_turn_output() {
    let clock = TestClock::new(NOW_MS);
    let wiring =
        bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&["filesystem"]))
            .expect("开关开");
    let module = wiring.module();
    let call = call("filesystem");
    let result = ok_result("filesystem");

    for hook in ALL_HOOKS {
        let outcome = run_hook(module.as_ref(), hook, Some(&call), Some(&result))
            .await
            .expect("hook");
        assert_eq!(outcome.directive, ModuleDirective::Continue);
        assert!(
            outcome.prompt_overlays.is_empty(),
            "纯观测模块零 overlay ({hook:?})"
        );
        assert_eq!(outcome, ModuleOutcome::continue_());
    }
}

// ============================================
// ④ 真实信号链：AfterToolResult → 账本 → AfterTurn 迁移
// ============================================

#[tokio::test]
async fn a_real_tool_result_reaches_the_ledger_and_promotes_to_mature() {
    let clock = TestClock::new(NOW_MS);
    let wiring =
        bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&["filesystem"]))
            .expect("开关开");
    let module = wiring.module();
    let state = wiring.state();
    let call = call("filesystem");
    let result = ok_result("filesystem");

    let seeded = state.observation();
    assert_eq!(seeded.stage_counts.get("newborn"), Some(&1));
    assert_eq!(
        state.ledger().get("filesystem").expect("seeded").use_count,
        0,
        "播种 = 注册事实, 不是使用"
    );

    for _ in 0..3 {
        run_hook(
            module.as_ref(),
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&result),
        )
        .await
        .expect("tool result");
        run_hook(module.as_ref(), HookPoint::AfterTurn, None, None)
            .await
            .expect("turn");
    }

    let asset = state.ledger().get("filesystem").expect("asset").clone();
    assert_eq!(asset.stage, LifecycleStage::Mature);
    assert_eq!(asset.use_count, 3);
    let observation = state.observation();
    assert_eq!(observation.stage_counts.get("mature"), Some(&1));
    assert!(observation.transitions_total >= 2, "新生→试用→成熟");
    assert!(observation.last_action.is_some());
}

// ============================================
// ⑤ 工具名解析：结果名优先，回退调用名；未注册工具首现入账
// ============================================

#[tokio::test]
async fn tool_result_name_wins_over_call_name_and_unknown_tools_register() {
    let clock = TestClock::new(NOW_MS);
    let wiring =
        bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&[])).expect("开关开");
    let module = wiring.module();
    let state = wiring.state();

    // 结果自带名字 = 实际派发的工具名（优先于调用侧名字）。
    let declared = call("declared_name");
    let mut actual = ok_result("actual_name");
    actual.name = Some("actual_name".to_string());
    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        Some(&declared),
        Some(&actual),
    )
    .await
    .expect("hook");
    assert!(state.ledger().get("actual_name").is_some());
    assert!(state.ledger().get("declared_name").is_none());

    // 结果无名字 → 回退调用侧名字；未注册工具首次出现即入账。
    let fallback = call("dynamic_tool");
    let anonymous = ToolResult::ok("call_dynamic_tool", serde_json::json!({"ok": true}));
    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        Some(&fallback),
        Some(&anonymous),
    )
    .await
    .expect("hook");
    let ledger = state.ledger();
    let asset = ledger.get("dynamic_tool").expect("dynamic tool");
    assert_eq!(asset.stage, LifecycleStage::Newborn);
    assert_eq!(asset.use_count, 1);

    // 结果名空白 + 调用名空白 → 无可归属资产，不编名字。
    let blank_call = call("   ");
    let blank_result = ok_result("   ");
    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        Some(&blank_call),
        Some(&blank_result),
    )
    .await
    .expect("hook");
    assert_eq!(state.observation().assets_total, 2);
}

// ============================================
// ⑥ 失败证据：超过试用失败上限才退役（留墓碑）
// ============================================

#[tokio::test]
async fn failed_tool_results_retire_only_after_the_trial_failure_limit() {
    let clock = TestClock::new(NOW_MS);
    let wiring = bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&["flaky"]))
        .expect("开关开");
    let module = wiring.module();
    let state = wiring.state();
    let call = call("flaky");
    let result = error_result("flaky");

    for _ in 0..LifecyclePolicy::default().max_trial_failures {
        run_hook(
            module.as_ref(),
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&result),
        )
        .await
        .expect("failure");
        run_hook(module.as_ref(), HookPoint::AfterTurn, None, None)
            .await
            .expect("turn");
    }
    let asset = state.ledger().get("flaky").expect("asset").clone();
    assert_eq!(
        asset.stage,
        LifecycleStage::Newborn,
        "达到上限但未超过, 不退役"
    );
    assert_eq!(asset.trial_failures, 3);
    assert!(state.observation().last_transitions.is_empty());

    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        Some(&call),
        Some(&result),
    )
    .await
    .expect("fourth failure");
    run_hook(module.as_ref(), HookPoint::AfterTurn, None, None)
        .await
        .expect("turn");

    let asset = state.ledger().get("flaky").expect("asset").clone();
    assert_eq!(asset.stage, LifecycleStage::Retired);
    let tombstone = asset.tombstone.expect("退役必须留墓碑");
    assert_eq!(tombstone.reason, RetireReason::Failed);
    assert!(tombstone.recoverable);
    assert_eq!(state.observation().retired_total, 1);
    assert!(matches!(
        state.observation().last_transitions.last(),
        Some(LifecycleTransition::Retired {
            reason: RetireReason::Failed,
            ..
        })
    ));
}

// ============================================
// ⑦ 闲置退役：注入时钟 + 冷静期（90 天 + 7 天）
// ============================================

#[test]
fn idle_retirement_uses_the_injected_clock_and_grace_window() {
    let clock = TestClock::new(NOW_MS);
    let wiring = bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&["idle"]))
        .expect("开关开");
    let state = wiring.state();
    let threshold = DEFAULT_RETIRE_AFTER_UNUSED_MS;
    let grace = LifecyclePolicy::default().grace_period_ms;

    let retiring = state.evaluate_at(NOW_MS + threshold);
    assert_eq!(retiring.len(), 1);
    assert!(matches!(
        retiring[0],
        LifecycleTransition::EnteredRetiring { .. }
    ));
    assert_eq!(
        state.ledger().get("idle").expect("asset").stage,
        LifecycleStage::Retiring
    );

    assert!(
        state.evaluate_at(NOW_MS + threshold + grace - 1).is_empty(),
        "冷静期未满不得落墓碑"
    );
    let retired = state.evaluate_at(NOW_MS + threshold + grace);
    assert!(matches!(
        retired.as_slice(),
        [LifecycleTransition::Retired {
            reason: RetireReason::Unused,
            ..
        }]
    ));
}

// ============================================
// ⑧ 已退役资产被真实使用：不静默复活
// ============================================

#[tokio::test]
async fn retired_assets_are_not_silently_revived_by_real_use() {
    let clock = TestClock::new(NOW_MS);
    let wiring = bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&["flaky"]))
        .expect("开关开");
    let module = wiring.module();
    let state = wiring.state();
    let call = call("flaky");
    let failure = error_result("flaky");
    for _ in 0..=LifecyclePolicy::default().max_trial_failures {
        run_hook(
            module.as_ref(),
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&failure),
        )
        .await
        .expect("failure");
        run_hook(module.as_ref(), HookPoint::AfterTurn, None, None)
            .await
            .expect("turn");
    }
    assert_eq!(
        state.ledger().get("flaky").expect("asset").stage,
        LifecycleStage::Retired
    );

    let success = ok_result("flaky");
    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        Some(&call),
        Some(&success),
    )
    .await
    .expect("hook");

    let asset = state.ledger().get("flaky").expect("asset").clone();
    assert_eq!(asset.stage, LifecycleStage::Retired, "墓碑不被静默抹掉");
    assert_eq!(asset.use_count, 0, "拒绝的使用不计数");
    let observation = state.observation();
    assert_eq!(observation.rejected_after_retire_total, 1);
    assert!(
        observation
            .last_action
            .as_deref()
            .is_some_and(|action| action.contains("ignored (retired)")),
        "拒绝必须留痕: {:?}",
        observation.last_action
    );
}

// ============================================
// ⑨ 容量淘汰：计数非静默、规则确定
// ============================================

#[test]
fn capacity_eviction_is_counted_and_deterministic() {
    let clock = TestClock::new(NOW_MS);
    let state = Arc::new(BioLifecycleState::with_policy(
        LifecyclePolicy::default(),
        2,
        clock.clock(),
    ));
    let seeded = state.seed_tool_names_at(&names(&["tool.c", "tool.a", "tool.b"]), NOW_MS);
    assert_eq!(seeded, 3);
    let observation = state.observation();
    assert_eq!(observation.assets_total, 2, "容量含墓碑");
    assert_eq!(observation.evicted_total, 1, "淘汰必须计数, 不静默丢");
    let ledger = state.ledger();
    assert!(
        ledger.get("tool.a").is_none(),
        "同层级按 asset_id 字典序淘汰"
    );
    assert!(ledger.get("tool.b").is_some());
    assert!(ledger.get("tool.c").is_some());
}

// ============================================
// ⑩ 观测确定性 + self_status 探测口只读
// ============================================

#[tokio::test]
async fn observation_is_deterministic_and_the_probe_is_read_only() {
    async fn replay(clock: &TestClock, order: &[&str]) -> String {
        let wiring = bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(order))
            .expect("开关开");
        let module = wiring.module();
        let state = wiring.state();
        let call = call("filesystem");
        let result = ok_result("filesystem");
        run_hook(
            module.as_ref(),
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&result),
        )
        .await
        .expect("use");
        run_hook(module.as_ref(), HookPoint::AfterTurn, None, None)
            .await
            .expect("turn");
        serde_json::to_string(&state.observation()).expect("observation json")
    }

    let clock_a = TestClock::new(NOW_MS);
    let clock_b = TestClock::new(NOW_MS);
    let forward = replay(&clock_a, &["filesystem", "search"]).await;
    let backward = replay(&clock_b, &["search", "filesystem"]).await;
    // 播种顺序不同 → `last_action` 可能不同; 账本/迁移/计数必须同.
    let forward_json: serde_json::Value = serde_json::from_str(&forward).expect("json");
    let backward_json: serde_json::Value = serde_json::from_str(&backward).expect("json");
    for key in [
        "assets_total",
        "active_total",
        "retired_total",
        "stage_counts",
        "transitions_total",
        "last_transitions",
    ] {
        assert_eq!(forward_json[key], backward_json[key], "{key}");
    }

    let wiring =
        bio_lifecycle_wiring_from_env_value(Some("1"), clock_a.clock(), &names(&["filesystem"]))
            .expect("开关开");
    let state = wiring.state();
    let probe = bio_lifecycle_probe(Arc::clone(&state));
    let before = state.observation();
    let reading = probe.snapshot(42).expect("probe reads");
    assert_eq!(
        state.observation(),
        before,
        "探测纯读: 不 evaluate、不改账本"
    );
    assert!(reading.wired);
    assert_eq!(
        reading.threshold,
        Some(DEFAULT_RETIRE_AFTER_UNUSED_MS as f64)
    );
    let value = reading.value.expect("value");
    assert_eq!(value["assets_total"], 1);
    assert_eq!(value["observed_at_ms"], 42);
    assert_eq!(value["stage_counts"]["newborn"], 1);
}

// ============================================
// ⑪ self_status 聚合：注册探测口后该格 = wired 真值
// ============================================

#[test]
fn the_self_status_registry_renders_the_lifecycle_row_as_wired() {
    let clock = TestClock::new(NOW_MS);
    let wiring =
        bio_lifecycle_wiring_from_env_value(Some("1"), clock.clock(), &names(&["filesystem"]))
            .expect("开关开");
    let state = wiring.state();
    let mut registry = BioMechanismRegistry::new(true);
    registry.register_bio_probe(
        BioMechanismKind::Lifecycle,
        bio_lifecycle_probe(Arc::clone(state)),
    );

    let status = registry.snapshot();
    let row = status
        .reading(BioMechanismKind::Lifecycle)
        .expect("lifecycle row");
    assert_eq!(row.stage, BIO_STAGE_WIRED);
    assert!(row.wired, "注册探测口 = 接线事实");
    assert!(row.value.is_some(), "读数来自账本真值");
    assert!(row.reason.is_none());
}
