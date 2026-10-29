//! 问责闭环第 ③ 步 · 自动回填的**端到端**接线测试。
//!
//! 覆盖链路：**已声明影响 → 显式登记归属 → 工具调用失败 → 账目变 `failed`**。
//!
//! 三条红线在端到端层再钉一次：
//! - 工具**成功** ⇒ 后果**不变**（绝不自动填 `confirmed`）；
//! - **没有登记** ⇒ 不动（不编造归属）；
//! - **重复失败** ⇒ 不产生重复记录。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{system_clock, CapabilityId, SessionId};
use apeireth_protocol::canonical::{NormalizedResponse, ToolOutcome, ToolResult};
use apeireth_runtime_assembly::accountability::{
    AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome,
};
use apeireth_runtime_assembly::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleDirective,
    ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
};
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_runtime_assembly::consequence::ConsequenceTracker;
use apeireth_runtime_assembly::consequence_backfill::{
    ConsequenceBackfillModule, CONSEQUENCE_BACKFILL_MODULE_ID, CONSEQUENCE_LEDGER_FILE,
};
use apeireth_runtime_assembly::egress_gate::EGRESS_GATE_ENV;
use apeireth_runtime_assembly::{
    JudgeConfig, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use async_trait::async_trait;

/// 空副作用的模块调用器（钩子上下文必填；本测试不调它）。
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

/// 空子循环（钩子上下文必填；本测试不调它）。
struct NoSubLoop;

#[async_trait]
impl SubLoopSpawner for NoSubLoop {
    async fn spawn(&self, _spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        Err(SubLoopError::NoModel)
    }
}

static NO_SUB_LOOP: NoSubLoop = NoSubLoop;

/// env 串行锁（本文件内改 env 的测试互斥）。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "apeireth-backfill-e2e-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).expect("mkdir");
    root
}

/// 落一条已声明影响（行动 id = `act-1`）。
fn declare_impact(root: &PathBuf) {
    let ledger = AccountabilityLedger::new(Some(root.join("impact-ledger.jsonl")));
    assert!(
        ledger.append(&ImpactStatement {
            action_id: "act-1".into(),
            at_ms: 1_700_000_000_000,
            boundary: Boundary::UserToOrg,
            facts_used: Vec::new(),
            inferences: Vec::new(),
            grants: Vec::new(),
            affected: Affected::NamedParty,
            reversible: false,
            destination: "smtp:hr@example.com".into(),
            outcome: Outcome::Unknown,
        }),
        "影响声明应落盘"
    );
}

fn failed(call_id: &str) -> ToolResult {
    ToolResult {
        tool_call_id: call_id.to_string(),
        name: Some("shell.exec".into()),
        outcome: ToolOutcome::Error {
            message: "boom".into(),
            retryable: false,
        },
    }
}

fn succeeded(call_id: &str) -> ToolResult {
    ToolResult::ok(call_id, serde_json::json!({"sent": true}))
}

fn tracker(root: &PathBuf) -> ConsequenceTracker {
    ConsequenceTracker::with_ledger(root.join(CONSEQUENCE_LEDGER_FILE))
}

/// 端到端：工具失败 ⇒ 对应影响的 outcome 从 `Unknown` 变 `failed`。
#[test]
fn tool_failure_flips_the_declared_impact_to_failed() {
    let root = temp_root("failure");
    declare_impact(&root);
    let module = ConsequenceBackfillModule::from_data_root(&root);
    // 显式登记归属（调用方知道两个 id；系统不推断）。
    assert!(module.backfiller().bind("act-1", "call_1").is_bound());

    // 工具失败 ⇒ 自动回填。
    assert_eq!(
        module.observe_tool_result(&failed("call_1")).as_deref(),
        Some("act-1")
    );
    assert_eq!(
        tracker(&root).current_outcome("act-1"),
        Outcome::Failed,
        "工具失败 ⇒ 对应影响的账面变 failed"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 端到端：工具成功 ⇒ **不变**（仍 `Unknown`，不自动填 confirmed）。
#[test]
fn tool_success_leaves_the_impact_unknown() {
    let root = temp_root("success");
    declare_impact(&root);
    let module = ConsequenceBackfillModule::from_data_root(&root);
    module.backfiller().bind("act-1", "call_1");

    assert_eq!(module.observe_tool_result(&succeeded("call_1")), None);
    let tracker = tracker(&root);
    assert_eq!(tracker.current_outcome("act-1"), Outcome::Unknown);
    assert_eq!(tracker.counts().confirmed, 0, "绝不自动填 confirmed");
    assert_eq!(tracker.counts().failed, 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// 端到端：没有登记 ⇒ **不动**（不编造归属）。
#[test]
fn unmapped_failure_changes_nothing() {
    let root = temp_root("unmapped");
    declare_impact(&root);
    let module = ConsequenceBackfillModule::from_data_root(&root);

    assert_eq!(module.observe_tool_result(&failed("call_x")), None);
    let tracker = tracker(&root);
    assert_eq!(tracker.current_outcome("act-1"), Outcome::Unknown);
    assert_eq!(tracker.counts().tracked_actions, 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// 端到端：重复失败 ⇒ 幂等（一条记录、计数不膨胀）。
#[test]
fn repeated_failure_is_idempotent_end_to_end() {
    let root = temp_root("idempotent");
    declare_impact(&root);
    let module = ConsequenceBackfillModule::from_data_root(&root);
    module.backfiller().bind("act-1", "call_1");

    assert!(module.observe_tool_result(&failed("call_1")).is_some());
    assert!(module.observe_tool_result(&failed("call_1")).is_some());
    let tracker = tracker(&root);
    assert_eq!(tracker.current_outcome("act-1"), Outcome::Failed);
    assert_eq!(tracker.events_for("act-1").len(), 1, "不得重复记录");
    assert_eq!(tracker.counts().failed, 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// 全开关关的配置（默认开族一律显式关；与 startup_instrumentation 同口径）。
fn all_switches_off() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        judge: JudgeConfig {
            enabled: false,
            ..JudgeConfig::default()
        },
        filesystem: false,
        search: false,
        repo: false,
        repo_map: false,
        self_status: false,
        ..ProductionModulesConfig::default()
    }
}

/// 生产接线：问责面开关**开** ⇒ 自动回填面进模块表；**关** ⇒ 零注册。
///
/// 这一条钉住「默认关 = 不注册」的纪律（问责闭环第 ③ 步与问责面同一开关）。
#[test]
fn production_registers_the_backfill_face_only_with_the_accountability_switch() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let clock = system_clock();
    let module_id = "accountability.backfill".to_string();

    // ① 关：零注册（未启用 = 不注册）。
    std::env::remove_var(EGRESS_GATE_ENV);
    let off = ProductionModules::build(
        all_switches_off(),
        ProductionBackends::default(),
        clock.clone(),
    )
    .expect("装配应成功");
    assert!(
        !off.ids().contains(&module_id),
        "问责面关闭 ⇒ 自动回填面不注册: {:?}",
        off.ids()
    );

    // ② 开：进模块表（首位）。
    std::env::set_var(EGRESS_GATE_ENV, "1");
    let on = ProductionModules::build(all_switches_off(), ProductionBackends::default(), clock)
        .expect("装配应成功");
    assert_eq!(
        on.ids().first().map(String::as_str),
        Some("accountability.backfill"),
        "问责面开启 ⇒ 自动回填面在模块表首位: {:?}",
        on.ids()
    );
    std::env::remove_var(EGRESS_GATE_ENV);
}

/// **生产触发路径**：`AfterToolResult` 钩子 ⇒ 自动回填（装配根注册的就是这个面）。
#[tokio::test]
async fn after_tool_result_hook_backfills_the_failure() {
    let root = temp_root("hook");
    declare_impact(&root);
    let module = ConsequenceBackfillModule::from_data_root(&root);
    module.backfiller().bind("act-1", "call_1");

    let session = SessionId::new();
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let result = failed("call_1");
    let ctx = ModuleContext {
        session_id: &session,
        model: "test-model",
        messages: &[],
        candidate: None,
        tool_call: None,
        tool_result: Some(&result),
        invocation: &invocation,
        module_id: CONSEQUENCE_BACKFILL_MODULE_ID,
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    let outcome = module
        .on_hook(HookPoint::AfterToolResult, &ctx)
        .await
        .expect("观测面不得让回合失败");
    assert!(
        matches!(outcome.directive, ModuleDirective::Continue),
        "观测面不改回合走向"
    );
    assert_eq!(
        tracker(&root).current_outcome("act-1"),
        Outcome::Failed,
        "钩子路径同样回填 failed"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **其它钩子不看**：不是工具结果 ⇒ 什么都不做（不编造信号）。
#[tokio::test]
async fn other_hooks_never_backfill() {
    let root = temp_root("hook-other");
    declare_impact(&root);
    let module = ConsequenceBackfillModule::from_data_root(&root);
    module.backfiller().bind("act-1", "call_1");

    let session = SessionId::new();
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let result = failed("call_1");
    let ctx = ModuleContext {
        session_id: &session,
        model: "test-model",
        messages: &[],
        candidate: None,
        tool_call: None,
        tool_result: Some(&result),
        invocation: &invocation,
        module_id: CONSEQUENCE_BACKFILL_MODULE_ID,
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module
        .on_hook(HookPoint::BeforeToolCall, &ctx)
        .await
        .expect("观测面不得让回合失败");
    assert_eq!(
        tracker(&root).current_outcome("act-1"),
        Outcome::Unknown,
        "只有 AfterToolResult 才回填"
    );
    let _ = std::fs::remove_dir_all(&root);
}
