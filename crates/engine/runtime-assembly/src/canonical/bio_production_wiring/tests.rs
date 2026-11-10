//! `bio_production_wiring` 的单元测试（从模块文件拆出）。
//!
//! 为什么拆：`bio_production_wiring.rs` 在 `scripts/arch-baseline.txt` 里登记了
//! 尺寸上限（只减不增）⇒ 把 ~500 行测试留在模块文件里会挡住**行为面接线**
//! （SG-20 修复计划进提示词）这种真实功能增长 ⇒ 做**真实拆分**而不是删注释装样。
//! 测试仍能触达父模块私有项（`use super::*` ⇒ 同 crate 同模块树）。
//!
//! 运行：`cargo test -p apeireth-runtime-assembly --lib -- bio_production_wiring`

use super::*;
use std::sync::atomic::AtomicUsize;

use apeireth_guard::immune_memory::{ImmuneMemory, IncidentKind, SeverityTier};
use apeireth_orchestration::nociception::{FailureClass, Habituation, Recommendation};
use apeireth_orchestration::quorum::QuorumDecision;
use apeireth_orchestration::task_queue::{TaskQueueConfig, TaskStore};
use apeireth_organ::nociception::{
    BioPainObservation, BioPainSnapshot, BioRecoveryPhaseCounts, BioRecoverySnapshot,
};
use apeireth_tools_canonical::self_status::{
    render_snapshot, BioMechanismsStatus, BIO_STAGE_WIRED,
};
use apeireth_tools_canonical::SelfStatusSource;

use crate::canonical::self_status_source::BioMechanismRegistry;

use apeireth_core::kernel::CapabilityId;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;

struct StubTool {
    id: CapabilityId,
    name: &'static str,
}

#[async_trait]
impl ToolCapability for StubTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(self.name)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::json!({}))
    }
}

fn stub_tool(id: &'static str, name: &'static str) -> Arc<dyn ToolCapability> {
    Arc::new(StubTool {
        id: CapabilityId::new(id).expect("static capability id is valid"),
        name,
    })
}

/// 与生产同款的最小自述来源（空名册，无其它探测口）。
fn source() -> ProductionSelfStatusSource {
    ProductionSelfStatusSource::new(Vec::new(), false, None)
}

fn lifecycle_state() -> Arc<BioLifecycleState> {
    let clock: LifecycleClock = Arc::new(|| 1_000_u64);
    let wiring = bio_lifecycle_wiring_from_enabled_with_store_dir(
        true,
        clock,
        &["filesystem".to_string()],
        None,
    )
    .expect("lifecycle wiring");
    Arc::clone(wiring.state())
}

fn armed_slots() -> BioProbeSlots {
    let slots = BioProbeSlots::new();
    slots.arm(true);
    slots
}

fn tone_state() -> Arc<ToneState> {
    let clock: crate::canonical::care_heartbeat::bio_tone::ToneClock = Arc::new(|| 1_000_u64);
    Arc::new(ToneState::new(clock))
}

fn ritual_store() -> Arc<RitualStateStore> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let seq = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "apeireth-bio-ritual-probe-{}-{seq}.json",
        std::process::id()
    ));
    Arc::new(RitualStateStore::at(path))
}
fn social_repair_source() -> Arc<SocialRepairObservationSource> {
    Arc::new(SocialRepairObservationSource::new(
        BioAssemblySwitches::from_flags(true, false),
    ))
}

fn homeostasis_source() -> Arc<HomeostasisObservationSource> {
    Arc::new(HomeostasisObservationSource::new(
        BioAssemblySwitches::from_flags(true, true),
        Some(24_000),
    ))
}

fn bio_status(source: &ProductionSelfStatusSource) -> BioMechanismsStatus {
    source.snapshot().bio_mechanisms.expect("bio face mounted")
}

/// 一份有真实记录的共享免疫账（同一签名观察 2 次 = 适应记忆）。
fn immune_memory() -> SharedImmuneMemory {
    let memory: SharedImmuneMemory =
        Arc::new(parking_lot::Mutex::new(ImmuneMemory::with_capacity(8)));
    {
        let mut guard = memory.lock();
        guard.observe(
            IncidentKind::ToolPoison,
            "tool descriptor",
            SeverityTier::Tier1Note,
            false,
            1_000,
        );
        guard.observe(
            IncidentKind::ToolPoison,
            "tool descriptor",
            SeverityTier::Tier1Note,
            false,
            2_000,
        );
    }
    memory
}

fn pain_snapshot() -> BioPainSnapshot {
    BioPainSnapshot {
        enabled: true,
        assessed: 3,
        repaired: 1,
        observations: vec![BioPainObservation {
            capability: "tool.web_fetch".to_string(),
            cause_key: "transient:0123456789abcdef".to_string(),
            class: FailureClass::Transient,
            at_ms: 700,
            habituation: Habituation::Sensitized,
            chronic: true,
            recommended: Recommendation::Diagnose,
            rationale: "chronic same cause".to_string(),
        }],
        groups: Vec::new(),
        recovery: BioRecoverySnapshot {
            active_cases: 3,
            closed_cases: 2,
            rejected_transitions: 1,
            phases: BioRecoveryPhaseCounts {
                hemostasis: 1,
                inflammation: 1,
                proliferation: 1,
                remodeling: 0,
                resolved: 0,
            },
            cases: Vec::new(),
        },
    }
}

fn pain_reader(snapshot: BioPainSnapshot) -> PainSnapshotReader {
    Arc::new(move || Ok(snapshot.clone()))
}

fn task_queue() -> Arc<TaskQueue> {
    let store: Arc<dyn TaskStore> =
        Arc::new(crate::SqliteTaskStore::in_memory().expect("in-memory task store"));
    Arc::new(TaskQueue::open(store, TaskQueueConfig::default()).expect("queue opens"))
}

fn play_state() -> Arc<BioPlayState> {
    Arc::new(BioPlayState::new(
        std::env::temp_dir(),
        apeireth_core::kernel::system_clock(),
    ))
}

/// ⑪ 开关关时安装点不武装槽：所有 fill 都是 no-op，槽保持全空。
#[test]
fn install_point_disarms_the_slots_when_the_switch_is_off() {
    let slots = BioProbeSlots::new();
    let queue = task_queue();
    let play = play_state();
    let _source = install_bio_self_status_with(source(), Some(&queue), Some(&play), &slots, false);
    assert!(!slots.is_armed(), "开关关 = 槽不武装");
    assert!(slots.is_empty(), "开关关 = 槽全空");
    assert!(!slots.fill_immune_memory(Some(immune_memory())));
    assert!(!slots.fill_care_status(Arc::new(CareHeartbeatStatus::default)));
    assert!(!slots.fill_quorum_trace(Arc::new(|| None)));
    assert!(!slots.fill_pain_reader(pain_reader(pain_snapshot())));
    assert!(slots.is_empty(), "未武装 = fill 一律 no-op");
}

/// ⑫ 默认关：开关关时自述帧与「只挂观测面、不注册任何 probe」逐字节一致，
/// 且迟绑定读取口一次都不被调用（槽没被 fill，registry 也没启用）。
#[test]
fn bio_switch_off_keeps_the_frame_identical_and_never_reads() {
    let reads = Arc::new(AtomicUsize::new(0));
    let counting = {
        let reads = Arc::clone(&reads);
        Arc::new(move || {
            reads.fetch_add(1, Ordering::SeqCst);
            CareHeartbeatStatus::default()
        })
    };
    let slots = BioProbeSlots::new();
    assert!(!slots.fill_care_status(counting), "未武装 = 不收句柄");

    let baseline = source().with_bio_self_status(false).snapshot();
    let installed = install_bio_self_status_with(source(), None, None, &slots, false);
    assert_eq!(
        render_snapshot(&baseline),
        render_snapshot(&installed.snapshot()),
        "默认关 = 自述帧逐字节不变"
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0, "关时零读取");
    let status = bio_status(&installed);
    assert!(!status.enabled);
    assert!(status.mechanisms.is_none(), "关时整节 null, 一格也不采");
}

/// ① 免疫记忆槽：安装后迟绑定填入 guard 同一份共享账 → 该格真值。
#[test]
fn late_bound_immune_memory_slot_feeds_the_probe_after_install() {
    let slots = armed_slots();
    let source = install_bio_self_status_with(source(), None, None, &slots, true);
    assert!(slots.fill_immune_memory(Some(immune_memory())));
    let status = bio_status(&source);
    let row = status
        .reading(BioMechanismKind::ImmuneMemory)
        .expect("immune row");
    assert!(row.wired);
    assert_eq!(row.stage, BIO_STAGE_WIRED);
    let value = row.value.as_ref().expect("immune value");
    assert_eq!(value["records"], 1);
    assert_eq!(value["open"], 1);
    assert_eq!(value["known_patterns"], 1, "同签名观察 >= 2 = 适应记忆");
    assert_eq!(value["kinds"]["tool_poison"], 1);
    assert!(row.reason.is_none());
}

/// ② 关怀心跳槽：迟绑定 care 生产者同一状态闭包 → 该格真值。
#[test]
fn late_bound_care_status_slot_feeds_the_probe_after_install() {
    let slots = armed_slots();
    let source = install_bio_self_status_with(source(), None, None, &slots, true);
    assert!(slots.fill_care_status(Arc::new(|| CareHeartbeatStatus {
        next_poll_at_ms: Some(9_000),
        last_hit_at_ms: Some(8_000),
        polls: 7,
        hits: 2,
    })));
    let status = bio_status(&source);
    let row = status
        .reading(BioMechanismKind::Attachment)
        .expect("attachment row");
    assert!(row.wired);
    let value = row.value.as_ref().expect("care value");
    assert_eq!(value["polls"], 7);
    assert_eq!(value["hits"], 2);
    assert_eq!(value["next_poll_at_ms"], 9_000);
    assert_eq!(value["last_hit_at_ms"], 8_000);
}

/// ③ 审议门槽：迟绑定 organ 模块同一门账读取口 → 该格真值（不含理由原文）。
#[test]
fn late_bound_quorum_trace_slot_feeds_the_probe_after_install() {
    let slots = armed_slots();
    let source = install_bio_self_status_with(source(), None, None, &slots, true);
    assert!(slots.fill_quorum_trace(Arc::new(|| {
        Some(QuorumTrace {
            at_ms: 4_200,
            convened: false,
            escalation: None,
            denial: Some(QuorumDecision::Hold {
                score: 0.25,
                need: 0.75,
            }),
        })
    })));
    let status = bio_status(&source);
    let row = status
        .reading(BioMechanismKind::Quorum)
        .expect("quorum row");
    assert!(row.wired);
    let value = row.value.as_ref().expect("quorum value");
    assert_eq!(value["traces"], 1);
    assert_eq!(value["denial"], "hold");
    assert_eq!(value["denial_score"], 0.25);
    assert_eq!(value["denial_need"], 0.75);
}

/// ④ 痛觉/再生槽：同一 runtime 读取口同时喂两格（痛觉格 + 再生四期格）。
#[test]
fn late_bound_pain_reader_feeds_both_pain_and_recovery_cells() {
    let slots = armed_slots();
    let source = install_bio_self_status_with(source(), None, None, &slots, true);
    assert!(slots.fill_pain_reader(pain_reader(pain_snapshot())));
    let status = bio_status(&source);

    let pain = status
        .reading(BioMechanismKind::Nociception)
        .expect("pain row");
    assert!(pain.wired);
    let value = pain.value.as_ref().expect("pain value");
    assert_eq!(value["enabled"], true);
    assert_eq!(value["assessed"], 3);
    assert_eq!(value["repaired"], 1);
    assert_eq!(value["observations"], 1);
    assert_eq!(value["chronic_observations"], 1);

    let recovery = status
        .reading(BioMechanismKind::Recovery)
        .expect("recovery row");
    assert!(recovery.wired);
    let value = recovery.value.as_ref().expect("recovery value");
    assert_eq!(value["active_cases"], 3);
    assert_eq!(value["closed_cases"], 2);
    assert_eq!(value["hemostasis"], 1);
    assert_eq!(value["inflammation"], 1);
    assert_eq!(value["proliferation"], 1);
}

/// ⑤ 空槽：7 条迟绑定 probe 注册即接线（`wired:true`），但值显式 `null`
/// + 真因「未接线」，整帧仍成功（不撒谎成 0，也不整帧失败）。
#[test]
fn empty_slots_render_a_null_cell_with_the_not_wired_reason() {
    let slots = armed_slots();
    let source = install_bio_self_status_with(source(), None, None, &slots, true);
    let status = bio_status(&source);
    assert!(status.enabled);
    for kind in [
        BioMechanismKind::ImmuneMemory,
        BioMechanismKind::Attachment,
        BioMechanismKind::Quorum,
        BioMechanismKind::Nociception,
        BioMechanismKind::Recovery,
        BioMechanismKind::EcosystemHealth,
        BioMechanismKind::Lifecycle,
        BioMechanismKind::Neuromodulation,
        BioMechanismKind::Circadian,
        BioMechanismKind::Ritual,
        BioMechanismKind::Homeostasis,
        BioMechanismKind::SocialRepair,
    ] {
        let row = status.reading(kind).expect("late-bound row");
        assert!(row.wired, "注册 = 接线事实成立: {kind:?}");
        assert_eq!(row.stage, BIO_STAGE_WIRED, "{kind:?}");
        assert!(row.value.is_none(), "空槽绝不编值: {kind:?}");
        assert!(
            row.reason.as_deref().unwrap_or("").contains("未接线"),
            "{kind:?} 必须带真因: {:?}",
            row.reason
        );
    }
    let rows = status.mechanisms.as_ref().expect("14 格在场");
    assert_eq!(rows.len(), 14);
    let unwired = rows.values().filter(|row| !row.wired).count();
    assert_eq!(unwired, 2, "未注册的 2 格如实未接线 (14 - 12 注册)");
}

/// ⑥ 全填 + 任务队列 + 练习账 + 工具生态观测 + 生命周期账 + 调质/昼夜/仪式/上下文装配/社会修复观测：
/// wired 计数 = 14/14，且 14 格值全为真值（不是 null）。
#[test]
fn filled_slots_raise_the_wired_count_to_fourteen() {
    let slots = armed_slots();
    let queue = task_queue();
    let play = play_state();
    let source = install_bio_self_status_with(source(), Some(&queue), Some(&play), &slots, true);
    assert!(slots.fill_immune_memory(Some(immune_memory())));
    assert!(slots.fill_care_status(Arc::new(CareHeartbeatStatus::default)));
    assert!(slots.fill_quorum_trace(Arc::new(|| None)));
    assert!(slots.fill_pain_reader(pain_reader(pain_snapshot())));
    assert!(slots.fill_ecosystem_profiles(&[stub_tool("tool.filesystem", "filesystem")]));
    assert!(slots.fill_lifecycle_state(Some(&lifecycle_state())));
    assert!(slots.fill_tone_state(Some(&tone_state())));
    assert!(slots.fill_circadian_config(CircadianProbeConfig::new(0, true)));
    assert!(slots.fill_ritual_store(Some(&ritual_store())));
    assert!(slots.fill_homeostasis(Some(&homeostasis_source())));
    assert!(slots.fill_social_repair(Some(&social_repair_source())));
    let status = bio_status(&source);
    let rows = status.mechanisms.as_ref().expect("14 格在场");
    let wired: Vec<&BioMechanismKind> = BioMechanismKind::ALL
        .iter()
        .filter(|kind| rows.get(kind).is_some_and(|row| row.wired))
        .collect();
    assert_eq!(
            wired.len(),
            14,
            "2 (capacity/play) + 12 迟绑定 (含工具生态/生命周期/调质/昼夜/仪式/上下文装配/社会修复) = 14/14"
        );
    for kind in [
        BioMechanismKind::Capacity,
        BioMechanismKind::Play,
        BioMechanismKind::ImmuneMemory,
        BioMechanismKind::Attachment,
        BioMechanismKind::Quorum,
        BioMechanismKind::Nociception,
        BioMechanismKind::Recovery,
        BioMechanismKind::EcosystemHealth,
        BioMechanismKind::Lifecycle,
        BioMechanismKind::Neuromodulation,
        BioMechanismKind::Circadian,
        BioMechanismKind::Ritual,
        BioMechanismKind::Homeostasis,
        BioMechanismKind::SocialRepair,
    ] {
        let row = rows.get(&kind).expect("wired row");
        assert!(row.wired, "{kind:?}");
        assert!(row.value.is_some(), "已填句柄 = 真值: {kind:?}");
    }
}

/// ⑦ 武装前 fill 是 no-op；武装后同一句柄才真正落槽（迟绑定顺序契约）。
#[test]
fn fills_are_noops_until_the_install_point_arms_the_slots() {
    let slots = BioProbeSlots::new();
    assert!(!slots.is_armed());
    assert!(!slots.fill_pain_reader(pain_reader(pain_snapshot())));
    assert!(!slots.fill_quorum_trace(Arc::new(|| None)));
    assert!(slots.is_empty(), "安装点之前槽恒空");
    slots.arm(true);
    assert!(slots.fill_pain_reader(pain_reader(pain_snapshot())));
    assert!(slots.fill_quorum_trace(Arc::new(|| None)));
    assert!(!slots.is_empty());
}

/// ⑧ 槽是 `Arc` 共享的：安装点拿的是 clone，组合根用另一句柄 fill 后
/// probe 仍读到同一状态（迟绑定 = 同一组槽，不是复制）。
#[test]
fn slot_clones_share_the_same_handles_with_the_registered_probes() {
    let slots = armed_slots();
    let captured = slots.clone();
    let source = install_bio_self_status_with(source(), None, None, &captured, true);
    assert!(
        slots.fill_immune_memory(Some(immune_memory())),
        "用原句柄填"
    );
    let status = bio_status(&source);
    let row = status
        .reading(BioMechanismKind::ImmuneMemory)
        .expect("immune row");
    assert_eq!(
        row.value.as_ref().expect("value")["records"],
        1,
        "clone 与原句柄共享同一槽"
    );
}

/// ⑨ 已填状态序列化稳定：同输入同输出（时钟注入固定值，零系统时钟）。
#[test]
fn filled_bio_mechanisms_status_serializes_stably() {
    let slots = armed_slots();
    assert!(slots.fill_immune_memory(Some(immune_memory())));
    assert!(slots.fill_care_status(Arc::new(|| CareHeartbeatStatus {
        next_poll_at_ms: None,
        last_hit_at_ms: Some(5),
        polls: 1,
        hits: 1,
    })));
    assert!(slots.fill_pain_reader(pain_reader(pain_snapshot())));
    let mut registry = BioMechanismRegistry::new(true).with_now_ms(Arc::new(|| 42));
    registry.register_bio_probe(BioMechanismKind::ImmuneMemory, slots.immune_probe());
    registry.register_bio_probe(BioMechanismKind::Attachment, slots.care_probe());
    registry.register_bio_probe(BioMechanismKind::Nociception, slots.pain_probe());
    registry.register_bio_probe(BioMechanismKind::Recovery, slots.recovery_probe());
    let first = serde_json::to_value(registry.snapshot()).expect("serialize");
    let second = serde_json::to_value(registry.snapshot()).expect("serialize");
    assert_eq!(first, second, "同输入同输出");
    assert_eq!(
        first["mechanisms"]["immune_memory"]["value"]["observed_at_ms"],
        42
    );
}

/// ⑩ 槽空/槽填切换是同一格的状态变化，不新增机制格（14 格闭合词表）。
#[test]
fn late_binding_never_adds_or_removes_mechanism_rows() {
    let slots = armed_slots();
    let source = install_bio_self_status_with(source(), None, None, &slots, true);
    let before = bio_status(&source);
    assert_eq!(before.mechanisms.as_ref().expect("rows").len(), 14);
    assert!(slots.fill_pain_reader(pain_reader(pain_snapshot())));
    let after = bio_status(&source);
    let rows = after.mechanisms.as_ref().expect("rows");
    assert_eq!(rows.len(), 14, "填入句柄不新增机制格");
    assert_eq!(
        rows[&BioMechanismKind::Nociception].wired,
        true,
        "同一格从 null 转真值"
    );
}

// ── SG-20 社会修复 → 提示词修复指令（行为面真接线，2026-11-08）──────────
//
// 事件侧 = 总线订阅者（`SocialRepairObservationSource` 写共享槽）
// 回合侧 = `ObservedBioAssemblyModule`（每回合读槽 → 既有 `with_repair_plan` 渲染）
// 两侧走**同一个 Arc**；开关关 = 不构造订阅者、不注册模块、不读槽。

use crate::canonical::bio_social_repair::repair_plan_slot;
use crate::canonical::cognitive::BIO_REPAIR_GUIDANCE_ENABLE_ENV;
use crate::canonical::module::{
    InvocationContext, ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse,
    ModuleInvoker, PromptOverlay,
};
use crate::canonical::nervous_bus::{ApprovalSocialOutcome, NervousBus, NervousSignal};
use crate::canonical::subloop::{SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec};
use apeireth_core::kernel::SessionId;
use apeireth_organ::social_repair::{plan_repair, UncertaintySignal};
use apeireth_protocol::canonical::{ContentPart, NormalizedResponse};

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

/// 一次 TurnStart 钩子调用（与生产同形：模块拿到的就是这几样上下文）。
async fn run_turn(module: &Arc<dyn Module>) -> ModuleOutcome {
    let session = SessionId::new();
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let ctx = ModuleContext {
        session_id: &session,
        model: "test-model",
        messages: &[],
        candidate: None,
        tool_call: None,
        tool_result: None,
        invocation: &invocation,
        module_id: "bio_assembly",
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module
        .on_hook(HookPoint::TurnStart, &ctx)
        .await
        .expect("hook must not fail")
}

fn overlay_texts(outcome: &ModuleOutcome) -> Vec<String> {
    outcome
        .prompt_overlays
        .iter()
        .map(|overlay: &PromptOverlay| ContentPart::join_text(&overlay.message().content))
        .collect()
}

fn switchboard_with_repair_guidance() -> NervousSwitchboard {
    NervousSwitchboard::from_lookup(|name| {
        (name == BIO_REPAIR_GUIDANCE_ENABLE_ENV).then(|| "1".to_owned())
    })
    .expect("switchboard parses")
}

fn rejection_signal() -> NervousSignal {
    NervousSignal::approval_social(ApprovalSocialOutcome::Rejected, "cli.approval")
}

/// ⭐ 行为面真接线：一次**真实总线**上的人工拒绝 ⇒ 下一回合提示词里出现修复
/// 指令块；拒绝之前同模块零 overlay（零变化），且同一计划在后续回合仍有效。
#[tokio::test]
async fn bus_rejection_becomes_a_repair_directive_block_in_the_next_turn() {
    let mut modules: Vec<Arc<dyn Module>> = Vec::new();
    let clock = apeireth_core::kernel::system_clock();
    let switchboard = switchboard_with_repair_guidance();
    let handles = install_bio_modules(&mut modules, &clock, &[], None, None, &switchboard);
    let source = handles
        .social_repair_source
        .clone()
        .expect("修复指引开 ⇒ 观测源已构造");
    let bus = NervousBus::new();
    assert_eq!(source.subscribe(&bus), 1, "订阅者已挂到总线");
    assert_eq!(modules.len(), 1, "开关开 = 装配面模块已注册");
    let module = Arc::clone(&modules[0]);

    let before = run_turn(&module).await;
    assert!(
        overlay_texts(&before).is_empty(),
        "还没被拒 ⇒ 零 overlay（接线前后同一行为）"
    );

    assert_eq!(bus.publish(&rejection_signal()), 1, "拒绝信号投递给订阅者");
    assert_eq!(source.rejections_seen(), 1);

    let after = run_turn(&module).await;
    let texts = overlay_texts(&after);
    assert_eq!(texts.len(), 1, "拒绝之后：一回合一块修复指令");
    assert!(
        texts[0].starts_with("[交互修复指令]"),
        "渲染段首必须是修复指令块: {}",
        texts[0]
    );
    assert!(texts[0].ends_with("[/交互修复指令]"));
    assert!(
        texts[0].contains("acknowledge_uncertainty"),
        "计划动作真的进了提示词（不是空块）: {}",
        texts[0]
    );

    let again = run_turn(&module).await;
    assert_eq!(
        overlay_texts(&again).len(),
        1,
        "同一计划在下一回合仍有效（读后不清空）"
    );
}

/// ⭐ 关档零变化且**零读取**：开关关 ⇒ 不构造观测源、不注册模块；即使槽被人为
/// 填了计划，装饰器也不读（不产生修复指令块）。
#[tokio::test]
async fn repair_guidance_off_never_reads_the_plan_slot() {
    let mut modules: Vec<Arc<dyn Module>> = Vec::new();
    let clock = apeireth_core::kernel::system_clock();
    let switchboard = NervousSwitchboard::default();
    let handles = install_bio_modules(&mut modules, &clock, &[], None, None, &switchboard);
    assert!(handles.social_repair_source.is_none(), "关档 = 零订阅源");
    assert!(modules.is_empty(), "全关 = 不注册任何模块");

    let slot = repair_plan_slot();
    let plan = plan_repair(
        &[UncertaintySignal::UserCorrection {
            detail: "test".to_owned(),
            weight: 1.0,
        }],
        None,
    );
    *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(plan);

    let switches = BioAssemblySwitches::disabled();
    let bare: Arc<dyn Module> = Arc::new(ObservedBioAssemblyModule::new(
        Arc::new(BioAssemblyModule::new(switches)),
        Arc::new(HomeostasisObservationSource::new(switches, None)),
        Arc::clone(&slot),
        None,
    ));
    let outcome = run_turn(&bare).await;
    assert!(
        overlay_texts(&outcome).is_empty(),
        "关档 ⇒ 槽里有计划也不读（默认关零变化）"
    );
}
