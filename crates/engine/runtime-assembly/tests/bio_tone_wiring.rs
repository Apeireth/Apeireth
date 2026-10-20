//! 生物机制族 W3.3 · 调质 → 语气装配面接线测试 (`APEIRETH_ENABLE_BIO_TONE`, 默认关)。
//!
//! 覆盖装配级契约 (不是 W1/W3.1 内核单测的重复):
//! 1. env 开关解析: 仅 `"1"` 开, 其余 fail-closed; 关 = 不构造 (懒加载);
//! 2. 默认关: 所有钩子 `ModuleOutcome::continue_()`, 零 overlay (即使状态已非基线);
//! 3. 基线逐字回退: `tone_hint_with_params(bond, baseline) == tone_hint(bond)`,
//!    且开关开但基线档 = 零 overlay (不编造调质语气);
//! 4. 显式反馈事件 → 状态变化 → 一个尾部独立 system 块 (固定标签 + 方向片段);
//! 5. overlay 只在 `TurnStart` 出现, 恰好一块, 不改写历史 / 不含用户原文;
//! 6. 九类反馈档到内核事件的一一映射与文档化方向;
//! 7. 关系快照显式注入 + 共享 `Arc<Mutex<NeuromodulatorState>>` 可观测。
//!
//! **0 装诚实**: 真实反馈事件源 (工具结果 / 用户纠错检测 / 社交反馈识别) 未接 —— 本文件
//! 只用显式 `apply_feedback` 注入; 关系快照缺省 `default()`; `production.rs` 未注册
//! (注册由 Lead 后续做); 无 live E2E。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_organ::neuromodulation::{BehaviorParams, NeuromodulatorKind};
use apeireth_organ::tone::{tone_hint, tone_hint_with_params, BondCharacterSnapshot, WarmthLevel};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedMessage, NormalizedResponse,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_tone::{
    bio_tone_enabled_from_env_value, bio_tone_module_from_env_value,
    bio_tone_wiring_from_env_value, BioToneModule, BioToneTurn, ToneClock, ToneFeedbackKind,
    ToneState, BIO_TONE_ENABLE_ENV, BIO_TONE_MODULE_ID, TONE_CLAUSE_LABEL,
};
use apeireth_runtime_assembly::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleDirective, ModuleError,
    ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
    ModuleOutcome,
};
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use async_trait::async_trait;

/// 冻结注入时刻 (测试永不读系统时钟)。
const NOW_MS: u64 = 1_760_000_000_000;
/// 用户原文探针: overlay 里出现即视为泄漏。
const CANARY: &str = "CANARY-USER-TEXT-bio-tone";

fn frozen_clock() -> ToneClock {
    Arc::new(|| NOW_MS)
}

fn state() -> Arc<ToneState> {
    Arc::new(ToneState::new(frozen_clock()))
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
    module: &dyn BehaviorModule,
    hook: HookPoint,
    messages: &[NormalizedMessage],
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
        tool_result: None,
        invocation: &invocation,
        module_id: module.manifest().id.as_str(),
        error: None,
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

fn turn_text(turn: &BioToneTurn) -> String {
    let overlay = turn.overlay.as_ref().expect("非基线档必须有 overlay");
    ContentPart::join_text(&overlay.message().content)
}

// ============================================
// 1. env 开关: 只认 "1", 默认关, 关 = 不构造
// ============================================

#[test]
fn tone_env_switch_is_fail_closed_and_lazy() {
    assert_eq!(BIO_TONE_ENABLE_ENV, "APEIRETH_ENABLE_BIO_TONE");
    assert!(bio_tone_enabled_from_env_value(Some("1")));
    assert!(bio_tone_enabled_from_env_value(Some(" 1 ")));

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
            !bio_tone_enabled_from_env_value(raw),
            "非 \"1\" 一律关: {raw:?}"
        );
        assert!(
            bio_tone_wiring_from_env_value(raw, frozen_clock()).is_none(),
            "开关关 = 不构造 (懒加载): {raw:?}"
        );
    }

    let wiring = bio_tone_wiring_from_env_value(Some("1"), frozen_clock()).expect("开关开");
    assert_eq!(wiring.module().manifest().id, BIO_TONE_MODULE_ID);
    assert!(
        wiring.state().is_baseline(),
        "新装配 = 基线调质 (没有事件就没有调制)"
    );
    assert_eq!(
        wiring.state().apply_feedback(ToneFeedbackKind::Quiet).at_ms,
        NOW_MS
    );
}

// ============================================
// 2. 默认关 zero-change: 所有钩子零 overlay (即使状态已非基线)
// ============================================

#[tokio::test]
async fn the_disabled_module_is_zero_change_on_every_hook() {
    let state = state();
    for _ in 0..2 {
        state.apply_feedback(ToneFeedbackKind::Threat);
    }
    let module = BioToneModule::new(false, Arc::clone(&state));
    assert!(!module.assemble().baseline, "状态确实已非基线");

    let messages = [NormalizedMessage::user(CANARY)];
    for hook in [
        HookPoint::TurnStart,
        HookPoint::BeforeModelCall,
        HookPoint::AfterModelResponse,
        HookPoint::AfterTurn,
        HookPoint::OnError,
    ] {
        let outcome = run_hook(&module, hook, &messages).await.expect("hook");
        assert_eq!(
            outcome,
            ModuleOutcome::continue_(),
            "开关关 = 既有行为逐字节一致 ({hook:?})"
        );
    }
}

// ============================================
// 3. 基线逐字回退 + 开关开但基线档 = 零 overlay
// ============================================

#[tokio::test]
async fn baseline_tone_falls_back_verbatim_and_emits_zero_overlay() {
    let baseline = BehaviorParams::baseline();
    let snapshots = [
        BondCharacterSnapshot::default(),
        BondCharacterSnapshot {
            trust: 0.5,
            ..BondCharacterSnapshot::default()
        },
        BondCharacterSnapshot {
            trust: 0.8,
            resonance: 0.7,
            ..BondCharacterSnapshot::default()
        },
        BondCharacterSnapshot {
            resonance: 0.5,
            ..BondCharacterSnapshot::default()
        },
    ];
    for snapshot in snapshots {
        assert_eq!(
            tone_hint_with_params(&snapshot, &baseline),
            tone_hint(&snapshot),
            "基线参数必须逐字回退既有 tone_hint"
        );
    }

    let state = state();
    assert_eq!(
        state.current_tone_hint(),
        tone_hint(&state.bond_snapshot()),
        "装配面基线档逐字回退"
    );
    let module = BioToneModule::new(true, Arc::clone(&state));
    let turn = module.assemble();
    assert!(turn.baseline);
    assert!(
        turn.overlay.is_none(),
        "开关开但基线档 = 零 overlay (没有反馈事件就不编调质语气)"
    );

    let outcome = run_hook(&module, HookPoint::TurnStart, &[])
        .await
        .expect("hook");
    assert_eq!(outcome, ModuleOutcome::continue_());
}

// ============================================
// 4. 事件注入 → 状态变化 → 一个尾部独立块 (固定标签 + 方向)
// ============================================

#[tokio::test]
async fn feedback_injection_surfaces_one_tail_overlay_with_direction() {
    let state = state();
    let module = BioToneModule::new(true, Arc::clone(&state));
    assert!(module.assemble().overlay.is_none(), "事件前: 零 overlay");

    let receipts: Vec<_> = (0..3)
        .map(|_| state.apply_feedback(ToneFeedbackKind::SocialPositive))
        .collect();
    assert!(receipts.iter().all(|receipt| receipt.changed()));
    assert_eq!(receipts[0].event_code, "social_positive");
    assert_eq!(
        state.tone_style().warmth,
        WarmthLevel::Affectionate,
        "社交正反馈累计 → 温度档上升"
    );

    let turn = module.assemble();
    assert!(!turn.baseline);
    let text = turn_text(&turn);
    assert!(
        text.starts_with(tone_hint(&state.bond_snapshot())),
        "保留既有基线前缀 (缓存前缀不破): {text}"
    );
    assert!(text.contains(TONE_CLAUSE_LABEL), "固定标签: {text}");
    assert!(text.contains("更温暖"), "方向片段: {text}");

    let outcome = run_hook(
        &module,
        HookPoint::TurnStart,
        &[NormalizedMessage::user(CANARY)],
    )
    .await
    .expect("hook");
    assert_eq!(outcome.directive, ModuleDirective::Continue);
    assert_eq!(overlay_text(&outcome), text, "钩子输出 = 纯装配结果");
}

// ============================================
// 5. overlay 只在 TurnStart; 独立 system 块; 不改写历史 / 不含用户原文
// ============================================

#[tokio::test]
async fn the_overlay_is_one_standalone_system_block_only_at_turn_start() {
    let state = state();
    for _ in 0..2 {
        state.apply_feedback(ToneFeedbackKind::Threat);
    }
    let module = BioToneModule::new(true, Arc::clone(&state));
    let messages = [
        NormalizedMessage::system("既有 system 前缀"),
        NormalizedMessage::user(CANARY),
    ];

    let start = run_hook(&module, HookPoint::TurnStart, &messages)
        .await
        .expect("hook");
    assert_eq!(start.prompt_overlays.len(), 1, "恰好一个尾部独立块");
    assert_eq!(start.directive, ModuleDirective::Continue);
    assert_eq!(
        start.prompt_overlays[0].message().role,
        MessageRole::System,
        "overlay 是 system 块"
    );
    assert!(
        start.prompt_overlays[0].cross_source_label().is_none(),
        "本面文本不含跨源材料, 不套 untrusted envelope"
    );
    let text = overlay_text(&start);
    assert!(text.contains("更谨慎"), "威胁 → 谨慎档: {text}");
    assert!(!text.contains(CANARY), "overlay 不得含用户原文: {text}");

    for hook in [
        HookPoint::BeforeModelCall,
        HookPoint::AfterModelResponse,
        HookPoint::BeforeToolCall,
        HookPoint::AfterToolResult,
        HookPoint::BeforeFinalCommit,
        HookPoint::AfterTurn,
        HookPoint::OnError,
    ] {
        let outcome = run_hook(&module, hook, &messages).await.expect("hook");
        assert!(
            outcome.prompt_overlays.is_empty(),
            "非 TurnStart 钩子零输出 ({hook:?})"
        );
    }
}

// ============================================
// 6. 九类反馈档 → 内核事件映射与文档化方向
// ============================================

#[test]
fn feedback_kinds_map_one_to_one_and_move_the_documented_axes() {
    let cases = [
        (
            ToneFeedbackKind::RewardPrediction {
                expected: 0.0,
                actual: 1.0,
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
    for (feedback, expected_code) in cases {
        assert_eq!(feedback.code(), expected_code);
        assert_eq!(feedback.to_event().code(), expected_code);
        let state = state();
        let receipt = state.apply_feedback(feedback);
        assert_eq!(receipt.event_code, expected_code);
        assert!(receipt.changed(), "{expected_code} 应改变六维状态");
        assert_eq!(receipt.at_ms, NOW_MS, "时刻来自注入时钟");
    }

    let baseline = state().vector();
    let corrected = state();
    corrected.apply_feedback(ToneFeedbackKind::UserCorrection);
    assert!(
        corrected.vector().acetylcholine > baseline.acetylcholine,
        "用户纠错抬乙酰胆碱"
    );

    let threatened = state();
    threatened.apply_feedback(ToneFeedbackKind::Threat);
    assert!(
        threatened.vector().cortisol > baseline.cortisol
            && threatened.vector().noradrenaline > baseline.noradrenaline,
        "威胁抬皮质醇/去甲肾上腺素"
    );

    let rewarded = state();
    rewarded.apply_feedback(ToneFeedbackKind::RewardPrediction {
        expected: 0.0,
        actual: 1.0,
    });
    assert!(
        rewarded.vector().dopamine > baseline.dopamine,
        "正 RPE 抬多巴胺"
    );

    let quiet = state();
    quiet.apply_feedback(ToneFeedbackKind::Quiet);
    assert!(
        quiet.vector().noradrenaline < baseline.noradrenaline,
        "安静期降唤醒"
    );
}

// ============================================
// 7. 关系快照显式注入 + 共享状态可观测
// ============================================

#[tokio::test]
async fn the_host_can_inject_the_bond_snapshot_and_share_the_arc_state() {
    let state = state();
    state.set_bond_snapshot(BondCharacterSnapshot {
        trust: 0.8,
        resonance: 0.7,
        ..BondCharacterSnapshot::default()
    });
    for _ in 0..3 {
        state.apply_feedback(ToneFeedbackKind::SocialPositive);
    }

    // 宿主自带状态 + env 开关 helper: 开关关 = None, 开 = 同一状态上的模块。
    assert!(bio_tone_module_from_env_value(None, Arc::clone(&state)).is_none());
    let module =
        bio_tone_module_from_env_value(Some("1"), Arc::clone(&state)).expect("开关开 = 构造");
    assert_eq!(module.manifest().id, BIO_TONE_MODULE_ID);

    let outcome = run_hook(module.as_ref(), HookPoint::TurnStart, &[])
        .await
        .expect("hook");
    let text = overlay_text(&outcome);
    assert!(
        text.starts_with("轻松亲切"),
        "基底句来自宿主注入的关系快照: {text}"
    );
    assert!(text.contains("更温暖"), "{text}");

    // 共享 Arc<Mutex<NeuromodulatorState>> 是唯一状态真值 (先各取快照, 避免同锁重入)。
    let shared = state.neuromodulator_state();
    let shared_oxytocin = {
        let guard = shared.lock().expect("lock");
        guard.vector().get(NeuromodulatorKind::Oxytocin)
    };
    let local_oxytocin = state.vector().oxytocin;
    assert_eq!(shared_oxytocin, local_oxytocin, "模块与宿主读同一本状态");

    // 关系快照不改变调质状态, 只改变语气基底句。
    assert!(state.vector().oxytocin > 0.5);
}

// ============================================
// 8. 注入时钟是唯一时间源 (装配面不读系统时钟)
// ============================================

#[test]
fn the_clock_is_injected_and_read_once_per_feedback() {
    let reads = Arc::new(AtomicU64::new(0));
    let observed = Arc::clone(&reads);
    let clock: ToneClock = Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
        NOW_MS
    });
    let state = ToneState::new(clock);
    assert_eq!(reads.load(Ordering::SeqCst), 0, "构造不读时钟");
    let receipt = state.apply_feedback(ToneFeedbackKind::Novelty);
    assert_eq!(receipt.at_ms, NOW_MS);
    assert_eq!(reads.load(Ordering::SeqCst), 1);
}
