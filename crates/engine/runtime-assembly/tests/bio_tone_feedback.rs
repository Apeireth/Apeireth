//! 生物机制族 W3.4 · 调质 → 语气「真实反馈事件源」装配级测试
//! (`APEIRETH_ENABLE_BIO_TONE`, 默认关)。
//!
//! 覆盖本批补的半程 (不是 W1/W3.1 内核单测、也不是既有 `bio_tone_wiring.rs`
//! 静态接线的重复):
//! 1. 开关关: 装配零构造零注册, 无状态句柄/无 attach 回调 (逐字节零变化);
//! 2. 开关开: 状态句柄与注册模块共享同一 `Arc<ToneState>`; 无反馈 = 基线零 overlay;
//! 3. 真实工具失败事件 (`AfterToolResult` + `ToolResult::Error`) → 档位移动 →
//!    `TurnStart` 出现一个尾部独立 system 块 (端到端模块级);
//! 4. 真实工具成功事件 → 同向可观测移动 + 非基线档 overlay;
//! 5. 九类反馈档 → 内核事件一一映射 (文档口径不因本批接线改变);
//! 6. 同事件重复投递 = 重复脉冲 (非幂等), 到 clamp 界后有界;
//! 7. `attach_tone_feedback_sink` 注入点: 开关关不回调, 开关开把同一状态交给
//!    既有反馈点, 事件源旁路注入后注册模块的 TurnStart 立刻可见。
//!
//! **0 装诚实**: 真实源 = 运行时既有钩子上的**结局事件** (工具结算 Ok/Error、
//! 运行时错误), 不读用户文本、不做情感推断; 用户显式好评/差评采集入口仍未接
//! (四处接口无生产调用方), 七档只有显式注入口; 无 live E2E; 开关关 = 缺省。

use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{CapabilityId, Clock, SessionId, Timestamp};
use apeireth_organ::tone::CautionLevel;
use apeireth_protocol::canonical::{
    ContentPart, NormalizedMessage, NormalizedResponse, ToolResult,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_tone::{
    ToneFeedbackKind, ToneState, BIO_TONE_ENABLE_ENV, BIO_TONE_MODULE_ID, TONE_CLAUSE_LABEL,
};
use apeireth_runtime_assembly::canonical::module::{
    AgentModule, HookPoint, InvocationContext, ModuleContext, ModuleDirective, ModuleError,
    ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
    ModuleOutcome,
};
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, JudgeConfig, ModuleBuildProbe, ProductionBackends,
    ProductionModules, ProductionModulesConfig,
};
use async_trait::async_trait;

/// 冻结时钟时刻 (测试永不读系统时钟; 事件全部同一逻辑时刻 = 零衰减)。
const NOW_MS: i64 = 1_760_000_000_000;
/// 用户原文探针: overlay 里出现即视为泄漏。
const CANARY: &str = "CANARY-USER-TEXT-bio-tone-feedback";

/// 进程 env 只在装配期读: 全文件共用一把锁, 只在**同步装配段**持有, 不跨 await。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// env 开关守卫: Drop 时清掉, panic 也不污染后续测试。
struct EnvSwitch;

impl EnvSwitch {
    fn set(value: &str) -> Self {
        std::env::set_var(BIO_TONE_ENABLE_ENV, value);
        Self
    }

    fn unset() -> Self {
        std::env::remove_var(BIO_TONE_ENABLE_ENV);
        Self
    }
}

impl Drop for EnvSwitch {
    fn drop(&mut self) {
        std::env::remove_var(BIO_TONE_ENABLE_ENV);
    }
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(NOW_MS)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

/// 全默认开族显式关: 空后端也能装配 (只留本批被测的 env 门)。
fn base_config() -> ProductionModulesConfig {
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
        self_status: false,
        ..ProductionModulesConfig::default()
    }
}

fn build() -> (ProductionModules, Arc<CountingBuildProbe>) {
    let probe = CountingBuildProbe::new();
    let concrete: Arc<dyn ModuleBuildProbe> = Arc::clone(&probe) as Arc<dyn ModuleBuildProbe>;
    let instrument = BuildInstrumentation::default().with_probe(concrete);
    let modules = ProductionModules::build_instrumented(
        base_config(),
        ProductionBackends::default(),
        clock(),
        &instrument,
    )
    .expect("装配必须成功");
    (modules, probe)
}

/// 开/关两种装配都在锁内完成 (env 只在 `build` 内读; 之后状态自持有, 与 env 无关)。
fn build_with_env(value: Option<&str>) -> (ProductionModules, Arc<CountingBuildProbe>) {
    let _lock = env_lock();
    let _switch = match value {
        Some(value) => EnvSwitch::set(value),
        None => EnvSwitch::unset(),
    };
    build()
}

fn tone_module(modules: &ProductionModules) -> Arc<dyn AgentModule> {
    modules
        .modules()
        .iter()
        .find(|module| module.manifest().id == BIO_TONE_MODULE_ID)
        .cloned()
        .unwrap_or_else(|| panic!("开关开必须注册 {BIO_TONE_MODULE_ID} 模块"))
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

async fn run_hook(
    module: &dyn AgentModule,
    hook: HookPoint,
    messages: &[NormalizedMessage],
    tool_result: Option<&ToolResult>,
    error: Option<&str>,
) -> Result<ModuleOutcome, ModuleError> {
    let session = SessionId::new();
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let ctx = ModuleContext {
        session_id: &session,
        model: "test-model",
        messages,
        candidate: None,
        tool_call: None,
        tool_result,
        invocation: &invocation,
        module_id: module.manifest().id.as_str(),
        error,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module.on_hook(hook, &ctx).await
}

fn overlay_text(outcome: &ModuleOutcome) -> String {
    assert_eq!(outcome.prompt_overlays.len(), 1, "恰好一个尾部独立块");
    ContentPart::join_text(&outcome.prompt_overlays[0].message().content)
}

// ============================================
// 1. 开关关 = 零构造 / 零注册 / 零状态 / 零 attach
// ============================================

#[test]
fn the_switch_off_assembly_has_no_state_no_module_and_no_attach() {
    let (modules, probe) = build_with_env(None);

    assert!(
        modules.bio_tone_state().is_none(),
        "开关关 = 无调质状态句柄 (不构造)"
    );
    assert!(
        !modules.ids().iter().any(|id| id == BIO_TONE_MODULE_ID),
        "开关关 = 不注册 tone 模块: {:?}",
        modules.ids()
    );
    assert_eq!(probe.total(), 0, "未启用族零构造 (懒加载语义)");

    let mut called = false;
    assert!(
        !modules.attach_tone_feedback_sink(|_| called = true),
        "开关关 = attach 返回 false"
    );
    assert!(!called, "开关关 = 注入回调不执行 (零订阅)");
}

// ============================================
// 2. 开关开 = 状态句柄 + 注册模块 + 无反馈零 overlay
// ============================================

#[tokio::test]
async fn the_switch_on_assembly_exposes_one_state_and_stays_silent_without_feedback() {
    let (modules, _probe) = build_with_env(Some("1"));
    let state = modules.bio_tone_state().expect("开关开 = 状态句柄");
    assert!(state.is_baseline(), "新装配 = 基线调质 (无事件无调制)");

    let registered: Vec<_> = modules
        .ids()
        .into_iter()
        .filter(|id| id == BIO_TONE_MODULE_ID)
        .collect();
    assert_eq!(registered.len(), 1, "tone 模块恰好注册一次");

    // attach 注入点交出的就是 getter 的同一 Arc (共享状态, 不复制)。
    let mut attached = None;
    assert!(modules.attach_tone_feedback_sink(|sink| attached = Some(sink)));
    let attached = attached.expect("开关开 = 回调收到状态");
    assert!(
        Arc::ptr_eq(&state, &attached),
        "getter 与注入点必须共享同一 Arc<ToneState>"
    );

    // 无反馈事件 = 基线零 overlay (开关开也不编调质语气)。
    let module = tone_module(&modules);
    let outcome = run_hook(module.as_ref(), HookPoint::TurnStart, &[], None, None)
        .await
        .expect("hook");
    assert_eq!(
        outcome,
        ModuleOutcome::continue_(),
        "无反馈 = 零 overlay / 零指令"
    );
    assert!(state.is_baseline());
}

// ============================================
// 3. 真实失败事件 → 档位移动 → TurnStart overlay (端到端模块级)
// ============================================

#[tokio::test]
async fn real_tool_failures_move_the_state_and_surface_a_cautious_overlay() {
    let (modules, _probe) = build_with_env(Some("1"));
    let module = tone_module(&modules);
    let state: Arc<ToneState> = modules.bio_tone_state().expect("开关开");

    let failed = ToolResult::permanent_error("call-1", "permission denied");
    for _ in 0..3 {
        let outcome = run_hook(
            module.as_ref(),
            HookPoint::AfterToolResult,
            &[NormalizedMessage::user(CANARY)],
            Some(&failed),
            None,
        )
        .await
        .expect("tool result hook");
        assert_eq!(
            outcome,
            ModuleOutcome::continue_(),
            "事件钩子只推进状态, 不产 overlay、不改指令"
        );
    }

    assert!(!state.is_baseline(), "真实失败事件必须把档位推离基线");
    assert_eq!(
        state.tone_style().caution,
        CautionLevel::Cautious,
        "失败累计 → 谨慎档 (确定性映射)"
    );

    let outcome = run_hook(
        module.as_ref(),
        HookPoint::TurnStart,
        &[NormalizedMessage::user(CANARY)],
        None,
        None,
    )
    .await
    .expect("turn start hook");
    assert_eq!(outcome.directive, ModuleDirective::Continue);
    let text = overlay_text(&outcome);
    assert!(text.contains(TONE_CLAUSE_LABEL), "固定标签: {text}");
    assert!(text.contains("更谨慎"), "失败方向片段: {text}");
    assert!(!text.contains(CANARY), "overlay 不得含用户原文: {text}");
}

// ============================================
// 4. 真实成功事件 → 同向可观测移动 + 非基线 overlay
// ============================================

#[tokio::test]
async fn real_tool_successes_move_the_state_and_surface_an_overlay() {
    let (modules, _probe) = build_with_env(Some("1"));
    let module = tone_module(&modules);
    let state: Arc<ToneState> = modules.bio_tone_state().expect("开关开");
    let before = state.vector();

    let ok = ToolResult::ok("call-ok", serde_json::json!({ "status": "done" }));
    for _ in 0..4 {
        run_hook(
            module.as_ref(),
            HookPoint::AfterToolResult,
            &[],
            Some(&ok),
            None,
        )
        .await
        .expect("tool result hook");
    }

    assert!(
        state.vector().dopamine > before.dopamine,
        "工具成功 (TaskSuccess) 抬多巴胺: {} -> {}",
        before.dopamine,
        state.vector().dopamine
    );
    assert!(!state.is_baseline(), "成功事件累计也必须可观测地移动档位");

    let outcome = run_hook(module.as_ref(), HookPoint::TurnStart, &[], None, None)
        .await
        .expect("turn start hook");
    let text = overlay_text(&outcome);
    assert!(text.contains(TONE_CLAUSE_LABEL), "固定标签: {text}");
}

// ============================================
// 5. 九类反馈档 → 内核事件一一映射 (文档口径不变)
// ============================================

#[test]
fn nine_feedback_kinds_map_one_to_one_to_their_documented_events() {
    let cases = [
        (
            ToneFeedbackKind::RewardPrediction {
                expected: 0.2,
                actual: 0.8,
            },
            "reward_prediction_error",
        ),
        (ToneFeedbackKind::TaskSuccess, "task_success"),
        (ToneFeedbackKind::TaskFailure, "task_failure"),
        (ToneFeedbackKind::SocialPositive, "social_positive"),
        (ToneFeedbackKind::SocialNegative, "social_negative"),
        (ToneFeedbackKind::Threat, "threat"),
        (ToneFeedbackKind::Novelty, "novelty"),
        (ToneFeedbackKind::Quiet, "quiet"),
        (ToneFeedbackKind::UserCorrection, "user_correction"),
    ];
    assert_eq!(cases.len(), 9, "九类全覆盖");
    for (feedback, code) in cases {
        assert_eq!(feedback.code(), code);
        assert_eq!(feedback.to_event().code(), code);
    }

    // 本批新接的两档在状态上真的落向文档方向。
    let state = ToneState::new(Arc::new(|| NOW_MS as u64));
    let baseline = state.vector();
    assert!(state
        .apply_feedback(ToneFeedbackKind::TaskSuccess)
        .changed());
    assert!(state.vector().dopamine > baseline.dopamine);

    let failed = ToneState::new(Arc::new(|| NOW_MS as u64));
    let before = failed.vector();
    assert!(failed
        .apply_feedback(ToneFeedbackKind::TaskFailure)
        .changed());
    assert!(failed.vector().cortisol > before.cortisol);
    assert!(failed.vector().serotonin < before.serotonin);
}

// ============================================
// 6. 同事件重复投递 = 重复脉冲 (非幂等), clamp 后有界
// ============================================

#[tokio::test]
async fn repeated_identical_events_are_not_deduplicated_under_the_module_surface() {
    let (modules, _probe) = build_with_env(Some("1"));
    let module = tone_module(&modules);
    let state: Arc<ToneState> = modules.bio_tone_state().expect("开关开");
    let failed = ToolResult::retryable_error("call-retry", "upstream timeout");

    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        &[],
        Some(&failed),
        None,
    )
    .await
    .expect("第一次投递");
    let after_first = state.vector();
    run_hook(
        module.as_ref(),
        HookPoint::AfterToolResult,
        &[],
        Some(&failed),
        None,
    )
    .await
    .expect("第二次投递");
    let after_second = state.vector();

    assert_ne!(
        after_first, after_second,
        "同一事件重复投递 = 重复脉冲 (非幂等, 不去重)"
    );
    assert!(
        after_second.cortisol >= after_first.cortisol,
        "同向事件必须同向累计"
    );

    // 有界: 推到 clamp 界后同向事件不再改变状态 (内核 changed 口径)。
    let mut bounded = false;
    for _ in 0..256 {
        if !state
            .apply_feedback(ToneFeedbackKind::TaskFailure)
            .changed()
        {
            bounded = true;
            break;
        }
    }
    assert!(bounded, "clamp 界必须存在 (有界, 不无限漂移)");
}

// ============================================
// 7. attach 注入点: 事件源旁路注入后注册模块立即可见
// ============================================

#[tokio::test]
async fn the_attach_injection_point_feeds_the_same_state_the_module_reads() {
    let (modules, _probe) = build_with_env(Some("1"));
    let module = tone_module(&modules);

    let mut attached = None;
    assert!(modules.attach_tone_feedback_sink(|sink| attached = Some(sink)));
    let sink = attached.expect("开关开 = 注入点交状态");
    for _ in 0..3 {
        sink.apply_feedback(ToneFeedbackKind::TaskFailure);
    }

    let outcome = run_hook(module.as_ref(), HookPoint::TurnStart, &[], None, None)
        .await
        .expect("turn start hook");
    let text = overlay_text(&outcome);
    assert!(
        text.contains(TONE_CLAUSE_LABEL) && text.contains("更谨慎"),
        "注入点与模块读同一状态 (旁路注入立即可见): {text}"
    );
}

// ============================================
// 8. 事件载荷缺失 / 非事件钩子不编馈 (模块面)
// ============================================

#[tokio::test]
async fn hooks_without_a_real_payload_never_move_the_state() {
    let (modules, _probe) = build_with_env(Some("1"));
    let module = tone_module(&modules);
    let state: Arc<ToneState> = modules.bio_tone_state().expect("开关开");
    let before = state.vector();

    for hook in [
        HookPoint::BeforeModelCall,
        HookPoint::AfterModelResponse,
        HookPoint::BeforeToolCall,
        HookPoint::BeforeFinalCommit,
        HookPoint::AfterTurn,
        HookPoint::AfterToolResult,
        HookPoint::OnError,
    ] {
        let outcome = run_hook(module.as_ref(), hook, &[], None, None)
            .await
            .expect("hook");
        assert_eq!(
            outcome,
            ModuleOutcome::continue_(),
            "{hook:?} 无可观测载荷 = 零输出"
        );
    }
    assert_eq!(state.vector(), before, "缺载荷 = 无事件 = 状态不动");
    assert!(state.is_baseline());
}
