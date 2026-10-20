//! W3.2 装配面集成测试: 修复指令 + 上下文策略 (两个开关默认关).
//!
//! 覆盖: 开关解析 fail-closed / 默认关零变化 (纯装配面 + 钩子逐字段相等) /
//! 注入 RepairPlan → system 独立块含渲染段且动作 ≤ 2 / 空 plan 不变 /
//! ContextPressure 高 → 收敛策略 / 无预算数据 → 默认策略 /
//! 策略对非核心块真折叠 / 用户原文与信号原文不进装配文本.
//!
//! 装配面能力级别: IMPLEMENTED + 默认关 (不是 DEFAULT ENABLED、不是
//! PRODUCTION WIRED); 本文件只证明装配级行为, 不证明 production.rs 已注册模块.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_organ::homeostasis::{RegulationAction, Severity};
use apeireth_organ::prompt_assembly::{repair_guidance, REPAIR_DIRECTIVE_PLACEHOLDER};
use apeireth_organ::social_repair::{
    plan_repair, JointAttention, JointAttentionTarget, RepairMove, RepairPlan, UncertaintySignal,
    MAX_REPAIR_MOVES,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedMessage, NormalizedResponse,
};
use apeireth_runtime_assembly::canonical::cognitive::{
    assemble_bio_turn, bio_switch_enabled_from_env_value, BioAssemblyModule, BioAssemblySwitches,
    BioContextBlock, BioTurnInput, BIO_ASSEMBLY_MODULE_ID, BIO_CONTEXT_POLICY_ENABLE_ENV,
    BIO_REPAIR_GUIDANCE_ENABLE_ENV,
};
use apeireth_runtime_assembly::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleInvocationError,
    ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker, ModuleOutcome,
};
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use async_trait::async_trait;

/// 用户原文探针: 装配文本里出现即视为泄漏.
const CANARY: &str = "CANARY-USER-TEXT-7f3a";

fn user_correction(detail: &str) -> UncertaintySignal {
    UncertaintySignal::UserCorrection {
        detail: detail.into(),
        weight: 0.9,
    }
}

fn contradiction(detail: &str) -> UncertaintySignal {
    UncertaintySignal::Contradiction {
        detail: detail.into(),
        weight: 0.5,
    }
}

fn attention_mismatch() -> JointAttention {
    JointAttention {
        target: JointAttentionTarget::Task {
            label: "对齐任务".into(),
        },
        agree: false,
    }
}

/// 只开修复指令开关.
fn repair_only() -> BioAssemblySwitches {
    BioAssemblySwitches::disabled().with_repair_guidance(true)
}

/// 只开上下文策略开关.
fn policy_only() -> BioAssemblySwitches {
    BioAssemblySwitches::disabled().with_context_policy(true)
}

fn empty_plan() -> RepairPlan {
    RepairPlan {
        moves: Vec::new(),
        ask_user: false,
        rationale: "无可执行动作".into(),
        max_moves: MAX_REPAIR_MOVES,
    }
}

fn overlay_text(outcome: &ModuleOutcome, index: usize) -> String {
    ContentPart::join_text(&outcome.prompt_overlays[index].message().content)
}

fn text_len(overlay: &apeireth_runtime_assembly::canonical::module::PromptOverlay) -> usize {
    ContentPart::join_text(&overlay.message().content)
        .chars()
        .count()
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
    module: &BioAssemblyModule,
    hook: HookPoint,
    messages: &[NormalizedMessage],
) -> ModuleOutcome {
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
        module_id: BIO_ASSEMBLY_MODULE_ID,
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module
        .on_hook(hook, &ctx)
        .await
        .expect("hook must not fail")
}

/// 1) 开关值解析: 只有显式 `1` 开, 其余一律关 (fail-closed).
#[test]
fn switch_value_parsing_is_fail_closed() {
    assert!(bio_switch_enabled_from_env_value(Some("1")));
    assert!(bio_switch_enabled_from_env_value(Some(" 1 ")));
    for raw in ["", "0", "true", "yes", "on", "2", "1x", "TRUE", " 0 ", "01"] {
        assert!(
            !bio_switch_enabled_from_env_value(Some(raw)),
            "{raw:?} 必须关 (fail-closed)"
        );
    }
    assert!(!bio_switch_enabled_from_env_value(None));

    let off = BioAssemblySwitches::from_values(None, Some("0"));
    assert!(off.is_disabled());
    let repair = BioAssemblySwitches::from_values(Some("1"), None);
    assert_eq!(
        (repair.repair_guidance(), repair.context_policy()),
        (true, false)
    );
    let policy = BioAssemblySwitches::from_values(Some("no"), Some("1"));
    assert_eq!(
        (policy.repair_guidance(), policy.context_policy()),
        (false, true)
    );
    assert!(!policy.is_disabled());
}

/// 2) env 开关: 未设 = 关; 非 `1` 不开; `1` 才逐开关打开.
#[test]
fn env_switches_default_off_and_only_explicit_one_enables() {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::env::remove_var(BIO_REPAIR_GUIDANCE_ENABLE_ENV);
    std::env::remove_var(BIO_CONTEXT_POLICY_ENABLE_ENV);
    assert!(
        BioAssemblySwitches::from_env().is_disabled(),
        "未设 = 两个开关都关"
    );
    std::env::set_var(BIO_REPAIR_GUIDANCE_ENABLE_ENV, "true");
    assert!(
        BioAssemblySwitches::from_env().is_disabled(),
        "非 1 (true) 不开"
    );
    std::env::set_var(BIO_REPAIR_GUIDANCE_ENABLE_ENV, "1");
    let repair = BioAssemblySwitches::from_env();
    assert!(repair.repair_guidance() && !repair.context_policy());
    std::env::set_var(BIO_CONTEXT_POLICY_ENABLE_ENV, "1");
    let both = BioAssemblySwitches::from_env();
    assert!(both.repair_guidance() && both.context_policy() && !both.is_disabled());
    std::env::remove_var(BIO_REPAIR_GUIDANCE_ENABLE_ENV);
    std::env::remove_var(BIO_CONTEXT_POLICY_ENABLE_ENV);
    assert!(BioAssemblySwitches::from_env().is_disabled());
}

/// 3) 两开关关: 即使输入全给 (plan + 块 + 高压力预算) 也零输出、默认策略.
#[test]
fn switches_off_emit_nothing_even_with_full_input() {
    let plan = plan_repair(&[user_correction("c")], Some(&attention_mismatch()));
    let input = BioTurnInput::new()
        .with_repair_plan(plan)
        .with_context_block(BioContextBlock::new("块正文"))
        .with_context_usage(900, 1_000);
    let assembly = assemble_bio_turn(BioAssemblySwitches::disabled(), &input);
    assert!(assembly.overlays.is_empty(), "默认关 = 零 overlay");
    assert!(assembly.policy.is_default(), "默认关 = 默认策略");
    assert!(assembly.regulation.is_none(), "默认关 = 不构造体征");
    assert!(!assembly.repair_registered);
    assert_eq!(assembly.repair_moves, 0);
    assert!(assembly.effective_budget_chars.is_none());
    assert!(assembly.undefined_vars.is_empty());
}

/// 4) 模块钩子默认关: 每个钩子都逐字段等于 `ModuleOutcome::continue_()`.
#[tokio::test]
async fn module_hook_with_switches_off_equals_continue_on_every_hook() {
    let plan = plan_repair(&[user_correction("c")], Some(&attention_mismatch()));
    let module = BioAssemblyModule::new(BioAssemblySwitches::disabled())
        .with_repair_plan(Some(plan))
        .with_context_blocks(vec![BioContextBlock::new("x".repeat(500))])
        .with_context_budget(16);
    assert!(module.switches().is_disabled());
    assert_eq!(module.manifest().id, BIO_ASSEMBLY_MODULE_ID);

    let messages = vec![NormalizedMessage::user(CANARY)];
    for hook in [
        HookPoint::TurnStart,
        HookPoint::BeforeModelCall,
        HookPoint::AfterTurn,
    ] {
        let outcome = run_hook(&module, hook, &messages).await;
        assert_eq!(
            outcome,
            ModuleOutcome::continue_(),
            "{hook:?} 在默认关下必须逐字段等于 continue_ (无 overlay / 无 directive)"
        );
    }
}

/// 5) 注入 RepairPlan: system 独立块 = 渲染段逐字节一致, 占位符已展开, 动作 ≤ 2.
#[tokio::test]
async fn injected_repair_plan_renders_system_block_with_at_most_two_moves() {
    let signals = [user_correction("用户纠正"), contradiction("前后矛盾")];
    let plan = plan_repair(&signals, Some(&attention_mismatch()));
    assert_eq!(plan.moves.len(), MAX_REPAIR_MOVES, "计划动作数 ≤ 2");
    let expected = repair_guidance(&plan)
        .expect("计划可渲染")
        .directive()
        .to_string();

    let module = BioAssemblyModule::new(repair_only()).with_repair_plan(Some(plan.clone()));
    let outcome = run_hook(&module, HookPoint::TurnStart, &[]).await;
    assert_eq!(outcome.prompt_overlays.len(), 1, "修复段是独立块");
    assert_eq!(
        outcome.prompt_overlays[0].message().role,
        MessageRole::System
    );
    let text = overlay_text(&outcome, 0);
    assert_eq!(text, expected, "装配文本 = 渲染段逐字节一致");
    assert!(
        !text.contains(REPAIR_DIRECTIVE_PLACEHOLDER),
        "占位符必须已展开"
    );
    assert!(text.starts_with("[交互修复指令]"));
    assert!(text.ends_with("[/交互修复指令]"));

    let assembly = module.assemble(None);
    assert!(assembly.repair_registered);
    assert_eq!(assembly.repair_moves, plan.moves.len());
    assert!(assembly.repair_moves <= MAX_REPAIR_MOVES);
    assert!(assembly.undefined_vars.is_empty(), "无残留占位符");
}

/// 6) 空 plan / 只有问句动作但 `ask_user == false`: 不注册 source, 输出不变.
#[test]
fn empty_or_question_only_plan_registers_no_source() {
    let assembly = assemble_bio_turn(
        repair_only(),
        &BioTurnInput::new().with_repair_plan(empty_plan()),
    );
    assert!(assembly.overlays.is_empty(), "空 plan = 不变");
    assert!(!assembly.repair_registered);
    assert_eq!(assembly.repair_moves, 0);
    assert!(assembly.policy.is_default());

    let question_only = RepairPlan {
        moves: vec![RepairMove::AskClarifying],
        ask_user: false,
        rationale: "只有问句动作".into(),
        max_moves: MAX_REPAIR_MOVES,
    };
    let assembly = assemble_bio_turn(
        repair_only(),
        &BioTurnInput::new().with_repair_plan(question_only),
    );
    assert!(assembly.overlays.is_empty(), "问句无许可 = 无可渲染动作");
    assert!(!assembly.repair_registered);
    assert!(assembly.undefined_vars.is_empty());
}

/// 7) ContextPressure 高 (0.85) → 收敛策略; 压力正常 → 默认策略 (空计划).
#[test]
fn high_context_pressure_selects_convergence_policy() {
    let high = assemble_bio_turn(
        policy_only(),
        &BioTurnInput::new().with_context_usage(850, 1_000),
    );
    assert!(
        high.policy.consolidate_before_overflow,
        "High 档 → 提前整编"
    );
    assert!(high.policy.reduce_recall_depth, "High 档 → 降召回深度");
    assert_eq!(
        high.effective_budget_chars,
        Some(1_000 / 2 * 3 / 4),
        "收敛预算 = 预算 /2 ×3/4"
    );
    let regulation = high.regulation.expect("有预算数据 = 有体征");
    assert!(regulation.severity >= Severity::High);
    assert!(regulation
        .actions()
        .contains(&RegulationAction::ConsolidateBeforeOverflow));

    let normal = assemble_bio_turn(
        policy_only(),
        &BioTurnInput::new().with_context_usage(100, 1_000),
    );
    assert!(normal.policy.is_default(), "压力正常 → 默认策略 (零变化)");
    assert!(normal.regulation.expect("有预算数据 = 有体征").is_noop());
    assert_eq!(normal.effective_budget_chars, Some(1_000));
}

/// 8) 策略对非核心块真折叠: critical 压力下 3×100 字块 → 2×50 字 (只留 2 层 + 折叠).
#[test]
fn critical_pressure_folds_injected_non_core_blocks() {
    let mut input = BioTurnInput::new();
    for ch in ['a', 'b', 'c'] {
        input = input.with_context_block(BioContextBlock::new(ch.to_string().repeat(100)));
    }

    // 参照组: 同一批块 + 策略开关开但压力正常 → 默认策略, 三块原样.
    let calm = assemble_bio_turn(policy_only(), &input.clone().with_context_usage(100, 300));
    assert!(calm.policy.is_default());
    assert_eq!(calm.overlays.len(), 3);
    assert!(calm.overlays.iter().all(|overlay| text_len(overlay) == 100));

    let input = input.with_context_usage(280, 300);
    let on = assemble_bio_turn(policy_only(), &input);
    assert!(on.policy.consolidate_before_overflow);
    assert!(on.policy.reduce_recall_depth);
    assert_eq!(on.effective_budget_chars, Some(300 / 2 * 3 / 4));
    assert_eq!(on.overlays.len(), 2, "降召回深度: 只留 2 个非核心层");
    assert!(
        on.overlays.iter().all(|overlay| text_len(overlay) == 50),
        "提前整编: 非核心块折叠到一半"
    );
}

/// 9) 策略开关开但无预算数据: 默认策略 + 不限额 (不发明预算, 行为不变).
#[test]
fn policy_on_without_budget_data_keeps_default_policy() {
    let input = BioTurnInput::new().with_context_block(BioContextBlock::new("块正文"));
    let assembly = assemble_bio_turn(policy_only(), &input);
    assert!(assembly.policy.is_default());
    assert!(assembly.regulation.is_none());
    assert!(assembly.effective_budget_chars.is_none());
    assert_eq!(assembly.overlays.len(), 1, "无预算 = 不裁剪");

    // 只有已用量、没有预算: 同样不构造体征 (不猜占用率).
    let used_only = BioTurnInput {
        context_used_chars: Some(999),
        ..BioTurnInput::new()
    };
    let assembly = assemble_bio_turn(policy_only(), &used_only);
    assert!(assembly.policy.is_default());
    assert!(assembly.regulation.is_none());
    assert!(assembly.effective_budget_chars.is_none());
}

/// 10) 开关开但没有任何显式注入: 钩子仍逐字段等于 `continue_()` (零变化).
#[tokio::test]
async fn switches_on_without_injected_data_is_still_continue() {
    let module = BioAssemblyModule::new(
        BioAssemblySwitches::disabled()
            .with_repair_guidance(true)
            .with_context_policy(true),
    );
    assert!(!module.switches().is_disabled());
    let messages = vec![NormalizedMessage::user(CANARY)];
    let outcome = run_hook(&module, HookPoint::TurnStart, &messages).await;
    assert_eq!(
        outcome,
        ModuleOutcome::continue_(),
        "开关开但无 plan / 无预算 = 不注册、不改策略、不发 overlay"
    );
}

/// 11) 用户原文 / 信号 detail / 计划 rationale 都不进装配文本.
#[tokio::test]
async fn assembled_text_never_copies_user_or_signal_text() {
    let signals = [
        user_correction(&format!("{CANARY}-DETAIL")),
        contradiction(&format!("{CANARY}-DETAIL-2")),
    ];
    let plan = plan_repair(&signals, Some(&attention_mismatch()));
    assert!(!plan.rationale.contains(CANARY), "计划 rationale 不含原文");
    let module = BioAssemblyModule::new(repair_only().with_context_policy(true))
        .with_repair_plan(Some(plan))
        .with_context_budget(64);
    let messages = vec![NormalizedMessage::user(format!(
        "{CANARY} 这是一整句用户原话, 绝不该进装配段{}",
        "x".repeat(80)
    ))];
    let outcome = run_hook(&module, HookPoint::TurnStart, &messages).await;
    assert_eq!(outcome.prompt_overlays.len(), 1, "只有修复段");
    let text = overlay_text(&outcome, 0);
    assert!(!text.contains(CANARY), "用户原文进装配段: {text}");
    assert!(!text.contains("DETAIL"), "信号 detail 进装配段: {text}");

    // 同一回合: 压力确实由「已有用量」推出来 (只取长度, 不取文本).
    let pressure = module.assemble(Some(160));
    assert!(pressure.policy.consolidate_before_overflow, "压力高 → 收敛");
    assert!(pressure.policy.escalate_to_human, "critical → 升级人工标记");
}
