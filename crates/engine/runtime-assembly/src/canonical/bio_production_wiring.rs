//! 生物机制族生产装配扩展点（W3.2 / W3.3 / W3.5 / W3.6，SG-18）。
//!
//! `production.rs` 只保留调用与字段；本文件承载：
//! - [`install_bio_modules`]：W3.2 `BioAssemblyModule` + W3.3 `bio_tone` +
//!   W3.5 `bio_lifecycle` 的注册点（env 默认关，关 = 不构造不注册）；
//! - [`install_bio_play`]：W3.3 游戏练习场四工具注册点（env 默认关）；
//! - [`install_bio_self_status`]：W3.2 自述 `bio_mechanisms` 子对象挂面；
//! - [`BioProbeSlots`]（W3.6 / R1b-2b / Wave 4 观测补格）：自述 probe 的
//!   **共享迟绑定槽** —— probe 先注册、句柄后填（免疫记忆 / 关怀心跳 / 审议门账 /
//!   痛觉再生读取口 / 生命周期账 / 调质状态 / 昼夜节律配置 / 仪式账），默认关时
//!   槽不武装、`fill_*` 全 no-op、零读取；
//! - [`ProductionModules`] 的生物句柄 getter（字段定义仍在 `production.rs`）。
//!
//! 本模块由 `production.rs` 以 `#[path = "bio_production_wiring.rs"]` 声明为
//! 子模块（`canonical.rs` 本批禁改；子模块天然可见装配根私有字段）。
//!
//! # 0 装诚实
//! - 生命周期模块**不新增 `ModuleFamily` 变体**：env 开也只注册模块，不上
//!   构造探针统计（构造点不上报，避免为单一模块膨胀族枚举）；
//! - 状态仅进程内（无持久化）；`bio_lifecycle_probe` 未注册进 self_status
//!   （Lead 后续）；无 live E2E；
//! - 迟绑定槽只存**只读句柄**：槽空 = 该格 `null` + 真因「未接线」，不编值；
//!   槽填 = 与生产同源句柄（guard 同一份免疫账 / care 生产者同一状态闭包 /
//!   organ 模块同一门账 / runtime 只读快照）。**不新增开关、不做 I/O 写**。

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::Clock;
use apeireth_guard::hook::SharedImmuneMemory;
use apeireth_orchestration::task_queue::TaskQueue;
use apeireth_organ::social_repair::RepairPlan;
use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::self_status::{
    BioMechanismKind, BioMechanismProbe, BioMechanismReading,
};

use crate::canonical::bio_extra_probes::{
    circadian_probe, homeostasis_bio_probe, neuromodulation_probe, ritual_state_probe,
    CircadianProbeConfig, HomeostasisObservationSource,
};
use crate::canonical::bio_social_repair::{
    repair_plan_slot, social_repair_bio_probe, RepairPlanSlot, SocialRepairObservationSource,
};
use crate::canonical::capability::CapabilityProvider;
use crate::canonical::care_heartbeat::bio_lifecycle::{
    bio_lifecycle_probe, bio_lifecycle_wiring_from_enabled_with_store_dir, BioLifecycleState,
    LifecycleClock,
};
use crate::canonical::care_heartbeat::bio_tone::{
    bio_tone_wiring_from_enabled, BioToneBusSubscriber, ToneState,
};
use crate::canonical::care_heartbeat::CareHeartbeatStatus;
use crate::canonical::care_reminder::RitualStateStore;
use crate::canonical::cognitive::{
    assemble_bio_turn, BioAssemblyModule, BioAssemblySwitches, BioTurnInput,
};
use crate::canonical::module::{
    BehaviorModule, HookPoint, Module, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use crate::canonical::nervous_switchboard::{BioSwitch, NervousSwitchboard};
use crate::canonical::orchestrator::QuorumTrace;
use crate::canonical::organ_module::OrganModule;
use crate::canonical::play_tools::BioPlayState;
use crate::canonical::self_status_source::{
    capacity_bio_probe, care_heartbeat_probe, immune_quorum_probe, pain_probe, play_probe,
    quorum_trace_probe, recovery_probe, PainSnapshotReader, ProductionSelfStatusSource,
};
use crate::canonical::tool_ecosystem_probe::{tool_ecosystem_bio_probe, ToolEcosystemSlot};

use super::{BuildInstrumentation, ModuleFamily, ProductionModules};

/// 生物装配句柄：开关关时对应字段为 `None`（未构造）。
pub(super) struct BioWiringHandles {
    /// W3.3 调质状态（与注册模块共享同一 `Arc`）。
    pub(super) tone_state: Option<Arc<ToneState>>,
    /// R1b-2a 调质总线订阅者（开关开才构造；装配根拿到总线后 `subscribe`）。
    pub(super) tone_subscriber: Option<BioToneBusSubscriber>,
    /// W3.5 生命周期状态（与注册模块共享同一 `Arc`）。
    pub(super) lifecycle_state: Option<Arc<BioLifecycleState>>,
    /// Wave 4 SG-14 上下文装配观测源（与装配面模块同一回合记录；开关关 = None）。
    pub(super) homeostasis_source: Option<Arc<HomeostasisObservationSource>>,
    /// Wave 4 SG-20 社会修复观测源（订阅人工拒绝信号；修复指引族开关关 = None）。
    pub(super) social_repair_source: Option<Arc<SocialRepairObservationSource>>,
}

/// 在装配根的同一位置注册 W3.2 / W3.3 / W3.5 模块。
///
/// 注册顺序保持 W3.2 → W3.3 不变，W3.5 紧随其后；三者开关默认关，
/// 关 = 不构造、不注册、不读时钟。`capabilities` 是装配根此刻已收集的
/// **真实工具能力面**，其声明名作为生命周期账本的 `Newborn` 种子。
pub(super) fn install_bio_modules(
    modules: &mut Vec<Arc<dyn Module>>,
    clock: &Arc<dyn Clock>,
    capabilities: &[Arc<dyn ToolCapability>],
    lifecycle_store_dir: Option<&std::path::Path>,
    context_budget_chars: Option<usize>,
    switchboard: &NervousSwitchboard,
) -> BioWiringHandles {
    // W3.2: 修复指令段 + 策略化上下文块的 TurnStart overlay 装配点。
    // 两个开关值由装配根从 `NervousSwitchboard` 一次性解析后传入（R1b-1b：
    // 本函数不再读 env）——**全关 = 不注册**（懒加载语义：未启用 = 不构造 =
    // 不注册，探针零上报）；开时也只装配装配根**显式注入**的 `RepairPlan` /
    // 上下文块 / 预算。
    let bio_switches = BioAssemblySwitches::from_flags(
        switchboard.switch_enabled(BioSwitch::RepairGuidance),
        switchboard.switch_enabled(BioSwitch::ContextPolicy),
    );
    let mut homeostasis_source = None;
    // ⭐ SG-20 行为面共享槽（2026-11-08）：事件侧（社会修复订阅者）写、回合侧
    // （装配面模块装饰器）读 —— 两侧同一个 `Arc`。`BioAssemblyModule` 是装配期
    // 构造一次的长寿模块，而 `RepairPlan` 是回合间事件的产物 ⇒ 必须经槽传递，
    // 否则计划永远停在装配快照（恒 `None`）。开关关 = 槽建了也没人写没人读。
    let sg20_plan_slot = repair_plan_slot();
    if !bio_switches.is_disabled() {
        // Wave 4 (SG-14): 组装器同一预算注入装配面模块 —— 上下文策略开关开时该预算
        // 参与本回合「压力 → 调节 → 策略」链路（开关关 = 不注册 = 零变化）。
        let mut module = BioAssemblyModule::new(bio_switches);
        let context_budget = context_budget_chars.filter(|budget| *budget > 0);
        if let Some(budget) = context_budget {
            module = module.with_context_budget(budget);
        }
        let homeostasis = Arc::new(HomeostasisObservationSource::new(
            bio_switches,
            context_budget_chars,
        ));
        modules.push(Arc::new(ObservedBioAssemblyModule::new(
            Arc::new(module),
            Arc::clone(&homeostasis),
            Arc::clone(&sg20_plan_slot),
            context_budget,
        )));
        homeostasis_source = Some(homeostasis);
    }
    // Wave 4 (SG-20): 社会修复观测源 —— 事件源 = 人工拒绝审批（ApprovalSocial=Rejected）。
    // 只在修复指引族开关开时构造；总线订阅由装配根在总线就绪后注册（关 = None = 零订阅）。
    // ⭐ 与上面同一个槽 ⇒ 拒绝事件写进去的正是装配面下一回合要读的计划。
    let social_repair_source = bio_switches.repair_guidance().then(|| {
        Arc::new(SocialRepairObservationSource::with_slot(
            bio_switches,
            Arc::clone(&sg20_plan_slot),
        ))
    });

    // W3.3: 调质→tone 的 TurnStart 尾部 overlay 装配点。开关值来自
    // `NervousSwitchboard`（R1b-1b）—— **关 = 不构造不注册**。开时构造共享
    // `ToneState` + 模块，并把状态句柄交给装配根（`bio_tone_state` /
    // `attach_tone_feedback_sink`）。
    //
    // R1b-2a: 事件源收敛到总线订阅——句柄额外携带总线订阅者，装配根拿到总线后
    // 注册（关 = `None` = 零订阅者）。模块 `on_hook` 不再扫描 AfterToolResult /
    // OnError，同一事件只经桥 → 总线 → 订阅者一次。
    let mut tone_state = None;
    let mut tone_subscriber = None;
    if let Some(wiring) =
        bio_tone_wiring_from_enabled(switchboard.switch_enabled(BioSwitch::Tone), {
            let clock = Arc::clone(clock);
            Arc::new(move || clock.now().timestamp_millis() as u64)
        })
    {
        tone_state = Some(Arc::clone(wiring.state()));
        tone_subscriber = Some(wiring.subscriber());
        modules.push(wiring.module() as Arc<dyn Module>);
    }

    // W3.5 (SG-18): 资产生命周期观测模块。开关值来自 `NervousSwitchboard`
    // （R1b-1b）—— 关 = 不构造状态、不构造模块、一次时钟都不读。开时以装配根
    // 的真实工具声明名播种 `Newborn` 资产，状态句柄交给装配根供自述 probe 迟绑定。
    let mut lifecycle_state = None;
    if switchboard.switch_enabled(BioSwitch::Lifecycle) {
        let lifecycle_clock: LifecycleClock = {
            let clock = Arc::clone(clock);
            Arc::new(move || clock.now().timestamp_millis() as u64)
        };
        let registered_tool_names: Vec<String> = capabilities
            .iter()
            .map(|tool| tool.declaration().name)
            .collect();
        if let Some(wiring) = bio_lifecycle_wiring_from_enabled_with_store_dir(
            true,
            lifecycle_clock,
            &registered_tool_names,
            lifecycle_store_dir,
        ) {
            lifecycle_state = Some(Arc::clone(wiring.state()));
            modules.push(wiring.module() as Arc<dyn Module>);
        }
    }

    BioWiringHandles {
        tone_state,
        tone_subscriber,
        lifecycle_state,
        homeostasis_source,
        social_repair_source,
    }
}

/// Wave 4 SG-14: 装配面模块的**观测装饰器** —— 行为仍由 [`BioAssemblyModule`]
/// 产出（同一 `assemble`、同一开关/预算），本包装只把每回合装配结果记进
/// [`HomeostasisObservationSource`]，供自述 homeostasis 格读数。
///
/// 为什么用装饰器而不是改 `cognitive.rs`：`cognitive.rs` 已有架构线 size 例外
/// （EXC-0002，owner=architecture-line）；生物线的观测职责留在生物装配扩展点，
/// 不占他线的例外额度，也不改行为模块契约。
struct ObservedBioAssemblyModule {
    inner: Arc<BioAssemblyModule>,
    source: Arc<HomeostasisObservationSource>,
    /// SG-20 行为面：社会修复订阅者写的计划槽（读侧就在本装饰器）。
    plan_slot: RepairPlanSlot,
    /// 装配根注入的上下文预算（与 `inner` 同值；有计划时按同一输入重放装配）。
    context_budget_chars: Option<usize>,
}

impl ObservedBioAssemblyModule {
    fn new(
        inner: Arc<BioAssemblyModule>,
        source: Arc<HomeostasisObservationSource>,
        plan_slot: RepairPlanSlot,
        context_budget_chars: Option<usize>,
    ) -> Self {
        Self {
            inner,
            source,
            plan_slot,
            context_budget_chars,
        }
    }

    /// 本回合待生效的修复计划（**开关关 ⇒ 连锁都不拿**）。
    ///
    /// 开关判断放在这里（而不是只靠 `assemble_bio_turn` 内部的那道）是为了让
    /// 「默认关 = 零读取」可被测试断言：关档下槽即使被人为填了也不会被读。
    fn latest_repair_plan(&self) -> Option<RepairPlan> {
        if !self.inner.switches().repair_guidance() {
            return None;
        }
        self.plan_slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// 可见 transcript 字符数（只取长度；文本本身不进装配输入，与内核同口径）。
fn visible_transcript_chars(messages: &[apeireth_protocol::canonical::NormalizedMessage]) -> usize {
    messages
        .iter()
        .map(|message| {
            apeireth_protocol::canonical::ContentPart::join_text(&message.content)
                .chars()
                .count()
        })
        .sum()
}

#[async_trait::async_trait]
impl BehaviorModule for ObservedBioAssemblyModule {
    fn manifest(&self) -> &ModuleManifest {
        self.inner.manifest()
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart || self.inner.switches().is_disabled() {
            return Ok(ModuleOutcome::continue_());
        }
        let used_chars = self
            .inner
            .switches()
            .context_policy()
            .then(|| visible_transcript_chars(ctx.messages));
        // ⭐ SG-20 行为面（2026-11-08）：每回合从共享槽读「上一次人工拒绝算出的修复
        // 计划」。无计划（从未被拒 / 开关关）= 走原路径（逐字段零变化）；有计划 =
        // 用同一组输入重放装配，**唯一差别**是 `repair_plan` ⇒ 多出一块独立 system
        // 修复指令段（`[交互修复指令] … [/交互修复指令]`）。
        //
        // ⚠️ **不变量**：本装饰器在装配期只给 `inner` 注入过「上下文预算」一种输入
        // （`install_bio_modules` 未注入 `context_blocks` / 未注入计划）⇒ 重放时
        // `context_blocks = Vec::new()` 与 `inner` 的现状等价。若将来有装配根在
        // 构造期注入上下文块，必须改为「从 inner 取块」（或在 `BioAssemblyModule`
        // 上开一个 `assemble_with_repair_plan` 端口），否则计划存在时那批块会被漏掉。
        let assembly = match self.latest_repair_plan() {
            Some(plan) => assemble_bio_turn(
                self.inner.switches(),
                &BioTurnInput {
                    repair_plan: Some(plan),
                    context_blocks: Vec::new(),
                    context_budget_chars: self.context_budget_chars,
                    context_used_chars: used_chars,
                },
            ),
            None => self.inner.assemble(used_chars),
        };
        self.source.record(used_chars, assembly.clone());
        let mut outcome = ModuleOutcome::continue_();
        for overlay in assembly.overlays {
            outcome = outcome.with_prompt_overlay(overlay);
        }
        Ok(outcome)
    }
}
/// 生物机制族 W3.3 · 游戏练习场四工具注册点（`APEIRETH_ENABLE_BIO_PLAY` 默认关）。
///
/// 注册即闸：env 只认 "1" **且** 工作区根已注入（缺根 = 无法证明
/// `sandbox_dir` 包含性 = 不注册）；全关 = 不构造 = 零注册。四工具模块共享
/// 同一本内存练习账本，产物只走 `PlayOutcome.artifacts` 元数据；审批钩子仍由
/// 适配器接线（本装配根不注册）。
///
/// 返回共享练习账本句柄（关 = `None`），供自述 `bio_mechanisms.play` probe 只读接入。
pub(super) fn install_bio_play(
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    clock: &Arc<dyn Clock>,
    workspace_root: Option<&std::path::Path>,
    instrument: &BuildInstrumentation,
    enabled: bool,
) -> Option<Arc<BioPlayState>> {
    let play = crate::canonical::play_tools::bio_play_wiring_from_enabled(
        enabled,
        workspace_root,
        Arc::clone(clock),
    )?;
    let state = Arc::clone(play.state());
    for module in play.into_modules() {
        instrument.constructed(ModuleFamily::BioPlay);
        capabilities.extend(module.capabilities());
    }
    Some(state)
}

/// 生物机制族 W3.7 · 仪式完成面两工具注册点（复用族总闸
/// `APEIRETH_ENABLE_BIO_CARE_CANDIDATES`，默认关）。
///
/// 注册即闸：env 只认 "1" **且** 关怀数据目录已注入（缺目录 = 无处落盘 =
/// 不注册）；全关 = 不构造 = 零注册、不创建 `ritual_state.json`。两工具共享
/// 同一本 `<care_dir>/ritual_state.json` 存储（与 care 心跳同一目录）。
///
/// **0 装诚实**：不新增 `ModuleFamily` 构造探针（与 `install_bio_modules` 的
/// 生命周期模块同口径：不为单一模块膨胀族枚举）。
///
/// 返回两工具共享的仪式账句柄（供组装根填 `BioProbeSlots` 的 SG-22 观测槽；
/// 开关关 / 缺目录 = `None`，槽保持空 = 该格如实「未接线」）。
pub(super) fn install_bio_ritual_tools(
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    clock: &Arc<dyn Clock>,
    care_dir: Option<&std::path::Path>,
    enabled: bool,
) -> Option<Arc<RitualStateStore>> {
    let Some(wiring) = crate::canonical::ritual_tools::bio_ritual_wiring_from_enabled(
        enabled,
        care_dir,
        Arc::clone(clock),
    ) else {
        return None;
    };
    let store = Arc::new(wiring.state().store().clone());
    for module in wiring.into_modules() {
        capabilities.extend(module.capabilities());
    }
    Some(store)
}

/// 迟绑定槽未填时 probe 回的真因（单格 `null` + 该原因，整帧仍成功）。
const BIO_PROBE_SLOT_EMPTY_REASON: &str = "未接线: 迟绑定句柄未填 (装配根尚未交付该句柄, 不编值)";

/// 关怀心跳态读取口（迟绑定）：与自述关怀节心跳两列同一真值源。
pub type CareStatusReader = Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync>;

/// 审议门账读取口（迟绑定）：返回 organ 模块最近一次 gated 门账。
pub type QuorumTraceReader = Arc<dyn Fn() -> Option<QuorumTrace> + Send + Sync>;

/// 生物机制族自述 probe 的**共享迟绑定槽**（W3.6）。
///
/// 装配根在 `ProductionModules::build` 里创建本槽并交给
/// [`install_bio_self_status`]；probe 闭包只捕获**槽的 `Arc`**，句柄在装配之后
/// 由组合根填入（免疫记忆经 `attach_immune_memory` / care 生产者 / runtime 的
/// 痛觉只读读取口 / organ 模块的门账读取口）。槽内全部是只读句柄，不新造账本。
///
/// 契约（0 装诚实）：
/// - **默认关**：自述开关关时 [`install_bio_self_status`] 不注册任何迟绑定
///   probe，且不武装本槽 —— 后续 `fill_*` 一律 no-op，槽保持全空（`is_empty`），
///   一次读取都不发生，自述帧逐字节不变；
/// - **开关开**：10 条 probe 先注册（接线事实 `wired:true`），句柄未到 = 该格
///   `value:null` + 真因「未接线」，整帧仍成功；句柄填入后同格转真值；
/// - 槽只存**只读共享句柄**（同一份账 / 同一状态闭包），不复制状态、不叠加第二
///   层缓冲；重复 fill 覆盖为最后一个句柄（与 `attach` 一次调用语义一致）。
#[derive(Clone, Default)]
pub struct BioProbeSlots {
    /// 自述开关生效时由安装点武装；未武装 = 所有 `fill_*` no-op（默认关）。
    armed: Arc<AtomicBool>,
    /// SG-15 免疫记忆：guard hook 正在使用的同一份共享账。
    immune: Arc<Mutex<Option<SharedImmuneMemory>>>,
    /// SG-19 依恋/重聚：常驻关怀心跳生产者同一状态闭包。
    care_status: Arc<Mutex<Option<CareStatusReader>>>,
    /// SG-24 群体感应审议门：organ 模块最近一次 gated 门账读取口。
    quorum_trace: Arc<Mutex<Option<QuorumTraceReader>>>,
    /// SG-16/17 痛觉 + 再生：runtime 只读快照读取口（两格共用同一读取口）。
    pain_reader: Arc<Mutex<Option<PainSnapshotReader>>>,
    /// SG-25 工具生态健康：装配末端填入的静态工具剖面快照（只读）。
    ecosystem: Arc<ToolEcosystemSlot>,
    /// R1b-2b SG-18 生命周期：注册模块同一本账的只读读取口。
    lifecycle: Arc<Mutex<Option<Arc<BioLifecycleState>>>>,
    /// Wave 4 SG-13 调质：`BioToneModule` / 总线订阅者同一份状态。
    tone: Arc<Mutex<Option<Arc<ToneState>>>>,
    /// Wave 4 SG-23 昼夜节律：开关板时区偏移 + 门开关（纯函数快照）。
    circadian: Arc<Mutex<Option<CircadianProbeConfig>>>,
    /// Wave 4 SG-22 仪式：`tool.ritual_done/skip` 写入的同一本仪式账。
    ritual: Arc<Mutex<Option<Arc<RitualStateStore>>>>,
    /// Wave 4 SG-14 上下文装配：装配面模块同一回合记录（只读观测源）。
    homeostasis: Arc<Mutex<Option<Arc<HomeostasisObservationSource>>>>,
    /// Wave 4 SG-20 社会修复：人工拒绝信号订阅源的只读观测源。
    social_repair: Arc<Mutex<Option<Arc<SocialRepairObservationSource>>>>,
}

/// 槽锁读取（持锁线程 panic 后槽内容未损坏，按 L 组口径恢复而非级联 panic）。
fn lock_slot<T>(slot: &Mutex<Option<T>>) -> std::sync::MutexGuard<'_, Option<T>> {
    slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl BioProbeSlots {
    /// 空槽（未武装；由装配根在自述安装点按开关显式武装）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否已武装（自述开关开 = 安装点注册了 10 条迟绑定 probe）。
    #[must_use]
    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }

    /// 九个槽是否全空（默认关证据：关时恒空，`fill_*` 全 no-op）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        lock_slot(&self.immune).is_none()
            && lock_slot(&self.care_status).is_none()
            && lock_slot(&self.quorum_trace).is_none()
            && lock_slot(&self.pain_reader).is_none()
            && self.ecosystem.is_empty()
            && lock_slot(&self.lifecycle).is_none()
            && lock_slot(&self.tone).is_none()
            && lock_slot(&self.circadian).is_none()
            && lock_slot(&self.ritual).is_none()
            && lock_slot(&self.homeostasis).is_none()
            && lock_slot(&self.social_repair).is_none()
    }

    /// 安装点武装/解除（只由 [`install_bio_self_status`] 调；非公开）。
    pub(super) fn arm(&self, armed: bool) {
        self.armed.store(armed, Ordering::Release);
    }

    /// 填免疫记忆共享句柄（guard hook 同一份）；返回是否真的填了。
    ///
    /// 未武装 / 句柄为 `None` = no-op（默认关零变化；hook 未挂免疫记忆 = 不接）。
    pub fn fill_immune_memory(&self, memory: Option<SharedImmuneMemory>) -> bool {
        match (self.is_armed(), memory) {
            (true, Some(memory)) => {
                *lock_slot(&self.immune) = Some(memory);
                true
            }
            _ => false,
        }
    }

    /// 填关怀心跳态读取口（`CareHeartbeatSource::status_probe` 同一闭包）。
    pub fn fill_care_status(&self, reader: CareStatusReader) -> bool {
        if !self.is_armed() {
            return false;
        }
        *lock_slot(&self.care_status) = Some(reader);
        true
    }

    /// 填审议门账读取口（organ 模块 `last_quorum_trace` 同一能力面）。
    pub fn fill_quorum_trace(&self, reader: QuorumTraceReader) -> bool {
        if !self.is_armed() {
            return false;
        }
        *lock_slot(&self.quorum_trace) = Some(reader);
        true
    }

    /// 填 runtime 痛觉/再生只读快照读取口（两格共用同一读取口）。
    pub fn fill_pain_reader(&self, reader: PainSnapshotReader) -> bool {
        if !self.is_armed() {
            return false;
        }
        *lock_slot(&self.pain_reader) = Some(reader);
        true
    }

    /// 填生命周期账读取口（与 `bio_lifecycle` 注册模块同一 `Arc`）。
    ///
    /// 未武装 / 无状态 = no-op（开关关 = 槽恒空，零读取）；返回是否真的填了。
    pub fn fill_lifecycle_state(&self, state: Option<&Arc<BioLifecycleState>>) -> bool {
        match (self.is_armed(), state) {
            (true, Some(state)) => {
                *lock_slot(&self.lifecycle) = Some(Arc::clone(state));
                true
            }
            _ => false,
        }
    }

    /// 填调质状态句柄（与 `BioToneModule` / 总线订阅者同一 `Arc`）。
    ///
    /// 未武装 / 无状态 = no-op（开关关 = 槽恒空，零读取）；返回是否真的填了。
    pub fn fill_tone_state(&self, state: Option<&Arc<ToneState>>) -> bool {
        match (self.is_armed(), state) {
            (true, Some(state)) => {
                *lock_slot(&self.tone) = Some(Arc::clone(state));
                true
            }
            _ => false,
        }
    }

    /// 填昼夜节律观测配置（开关板时区偏移 + 门开关；纯数据，无句柄）。
    pub fn fill_circadian_config(&self, config: CircadianProbeConfig) -> bool {
        if !self.is_armed() {
            return false;
        }
        *lock_slot(&self.circadian) = Some(config);
        true
    }

    /// 填仪式账句柄（与 `tool.ritual_done/skip` 同一本 `<care_dir>/ritual_state.json`）。
    ///
    /// 未武装 / 无账本（关怀数据目录未配 = 工具面本就不注册）= no-op，如实不接。
    pub fn fill_ritual_store(&self, store: Option<&Arc<RitualStateStore>>) -> bool {
        match (self.is_armed(), store) {
            (true, Some(store)) => {
                *lock_slot(&self.ritual) = Some(Arc::clone(store));
                true
            }
            _ => false,
        }
    }

    /// 填上下文装配观测源（与 `ObservedBioAssemblyModule` 同一回合记录）。
    ///
    /// 未武装 / 无模块（两个装配开关全关 = 模块本就不注册）= no-op，如实不接。
    pub fn fill_homeostasis(&self, source: Option<&Arc<HomeostasisObservationSource>>) -> bool {
        match (self.is_armed(), source) {
            (true, Some(source)) => {
                *lock_slot(&self.homeostasis) = Some(Arc::clone(source));
                true
            }
            _ => false,
        }
    }

    /// 填社会修复观测源（订阅人工拒绝信号；与自述 probe 同一 `Arc`）。
    ///
    /// 未武装 / 无源（修复指引族开关关 = 源本就不构造）= no-op，如实不接。
    pub fn fill_social_repair(&self, source: Option<&Arc<SocialRepairObservationSource>>) -> bool {
        match (self.is_armed(), source) {
            (true, Some(source)) => {
                *lock_slot(&self.social_repair) = Some(Arc::clone(source));
                true
            }
            _ => false,
        }
    }

    /// Wave 4 SG-14 homeostasis probe（迟绑定：槽空 = 未接线真因）。
    fn homeostasis_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.homeostasis),
            build: homeostasis_bio_probe,
        })
    }
    /// Wave 4 SG-20 社会修复 probe（迟绑定：槽空 = 未接线真因）。
    fn social_repair_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.social_repair),
            build: social_repair_bio_probe,
        })
    }

    /// 从 organ 模块填审议门账读取口（模块未装配 = no-op，如实不接）。
    pub(super) fn fill_from_organ_module(&self, module: Option<&Arc<OrganModule>>) -> bool {
        let Some(module) = module.cloned() else {
            return false;
        };
        self.fill_quorum_trace(Arc::new(move || module.last_quorum_trace()))
    }

    /// 填 SG-25 工具生态静态剖面（装配末端能力表装配完毕后调用一次）。
    ///
    /// 未武装（自述开关关）= no-op，槽保持空，零读取、零投影；返回是否真的填了。
    pub(super) fn fill_ecosystem_profiles(&self, capabilities: &[Arc<dyn ToolCapability>]) -> bool {
        if !self.is_armed() {
            return false;
        }
        self.ecosystem.fill(capabilities)
    }

    /// SG-25 工具生态解剖面槽（装配安装点绑定 probe 用；槽空 = 未接线真因）。
    fn ecosystem_slot(&self) -> Arc<ToolEcosystemSlot> {
        Arc::clone(&self.ecosystem)
    }

    /// SG-15 免疫记忆 probe（槽空 = `Err(未接线)` → 单格 null + 真因）。
    fn immune_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.immune),
            build: immune_quorum_probe,
        })
    }

    /// SG-19 依恋/重聚 probe（同一迟绑定语义）。
    fn care_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.care_status),
            build: care_heartbeat_probe,
        })
    }

    /// SG-24 审议门 probe（同一迟绑定语义）。
    fn quorum_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.quorum_trace),
            build: quorum_trace_probe,
        })
    }

    /// SG-16 痛觉 probe（与再生共用同一 runtime 读取口）。
    fn pain_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.pain_reader),
            build: pain_probe,
        })
    }

    /// SG-17 再生四期 probe（与痛觉共用同一 runtime 读取口）。
    fn recovery_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.pain_reader),
            build: recovery_probe,
        })
    }

    /// R1b-2b SG-18 生命周期 probe（迟绑定：槽空 = 未接线真因）。
    fn lifecycle_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.lifecycle),
            build: bio_lifecycle_probe,
        })
    }

    /// Wave 4 SG-13 调质 probe（槽空 = 未接线真因）。
    fn tone_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.tone),
            build: neuromodulation_probe,
        })
    }

    /// Wave 4 SG-23 昼夜节律 probe（纯函数快照，槽空 = 未接线真因）。
    fn circadian_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.circadian),
            build: circadian_probe,
        })
    }

    /// Wave 4 SG-22 仪式 probe（槽空 = 未接线真因）。
    fn ritual_probe(&self) -> Arc<dyn BioMechanismProbe> {
        Arc::new(LateBoundSlotProbe {
            slot: Arc::clone(&self.ritual),
            build: ritual_state_probe,
        })
    }
}

/// 槽绑定 probe：每次采集现读槽（短持锁、克隆句柄后立即放锁）。
///
/// 槽空 = `Err(未接线…)`（注册表折成单格 `null` + 真因，不整帧失败）；
/// 槽填 = 转调既有 probe 实现（同一采集逻辑，不复制一份口径）。
struct LateBoundSlotProbe<T> {
    slot: Arc<Mutex<Option<T>>>,
    build: fn(T) -> Arc<dyn BioMechanismProbe>,
}

impl<T: Clone + Send + 'static> BioMechanismProbe for LateBoundSlotProbe<T> {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        // 语句结束即放锁：调用内层 probe 时不持槽锁（免疫 probe 自己取账本锁）。
        let handle = lock_slot(&self.slot).clone();
        match handle {
            Some(handle) => (self.build)(handle).snapshot(now_ms),
            None => Err(BIO_PROBE_SLOT_EMPTY_REASON.to_string()),
        }
    }
}

/// W3.2 自述 `capabilities.bio_mechanisms` 挂面（W3.6 增迟绑定槽）。
///
/// 挂面 = 子对象恒在场（开关关时 `mechanisms: null` + 原因「未启用」，
/// 零 probe 调用；开关 `APEIRETH_ENABLE_BIO_SELF_STATUS` 只认 "1"）。
/// 承载力探测口读**同一本**任务队列（与 `tool.task_board` 同队同账）；
/// 练习场账本（W3.5 接入）与四工具面同一 `Arc`；W3.6/W3.7/R1b-2b/Wave-4 的 10 条
/// 迟绑定 probe（免疫记忆 / 关怀心跳 / 审议门 / 痛觉 / 再生 / 工具生态 / 生命周期 /
/// 调质 / 昼夜节律 / 仪式）只在开关开时注册 —— 注册即
/// 接线事实，句柄未到 = 单格 `null` + 「未接线」真因；开关关 = 一条都不注册、
/// 槽不武装、`fill_*` 全 no-op。其余格一律 `wired:false` + 「未接线」，不编数值。
pub(super) fn install_bio_self_status(
    source: ProductionSelfStatusSource,
    task_board: Option<&Arc<TaskQueue>>,
    bio_play_state: Option<&Arc<BioPlayState>>,
    slots: &BioProbeSlots,
    enabled: bool,
) -> ProductionSelfStatusSource {
    install_bio_self_status_with(source, task_board, bio_play_state, slots, enabled)
}

/// [`install_bio_self_status`] 的显式开关形态（纯函数式，测试不碰进程 env）。
fn install_bio_self_status_with(
    source: ProductionSelfStatusSource,
    task_board: Option<&Arc<TaskQueue>>,
    bio_play_state: Option<&Arc<BioPlayState>>,
    slots: &BioProbeSlots,
    enabled: bool,
) -> ProductionSelfStatusSource {
    let mut source = source.with_bio_self_status(enabled);
    // 槽仅在开关生效时武装：关 = 后续 fill_* 全 no-op、槽恒空、零读取。
    slots.arm(enabled);
    if let Some(queue) = task_board {
        source.register_bio_probe(
            BioMechanismKind::Capacity,
            capacity_bio_probe(Arc::clone(queue), None),
        );
    }
    if let Some(play) = bio_play_state {
        source.register_bio_probe(BioMechanismKind::Play, play_probe(Arc::clone(play)));
    }
    if enabled {
        source.register_bio_probe(BioMechanismKind::ImmuneMemory, slots.immune_probe());
        source.register_bio_probe(BioMechanismKind::Attachment, slots.care_probe());
        source.register_bio_probe(BioMechanismKind::Quorum, slots.quorum_probe());
        source.register_bio_probe(BioMechanismKind::Nociception, slots.pain_probe());
        source.register_bio_probe(BioMechanismKind::Recovery, slots.recovery_probe());
        source.register_bio_probe(
            BioMechanismKind::EcosystemHealth,
            tool_ecosystem_bio_probe(slots.ecosystem_slot()),
        );
        source.register_bio_probe(BioMechanismKind::Lifecycle, slots.lifecycle_probe());
        source.register_bio_probe(BioMechanismKind::Neuromodulation, slots.tone_probe());
        source.register_bio_probe(BioMechanismKind::Circadian, slots.circadian_probe());
        source.register_bio_probe(BioMechanismKind::Ritual, slots.ritual_probe());
        source.register_bio_probe(BioMechanismKind::Homeostasis, slots.homeostasis_probe());
        source.register_bio_probe(BioMechanismKind::SocialRepair, slots.social_repair_probe());
    }
    source
}

impl ProductionModules {
    /// 生物机制族 W3.3: 共享调质状态句柄 (开关关 = `None`; 与注册模块同一 `Arc`).
    ///
    /// 只读句柄: 状态推进仍只走 [`ToneState::apply_feedback`] (事件源经
    /// [`Self::attach_tone_feedback_sink`] 取得), 本口不改变模块装配与行为.
    #[must_use]
    pub fn bio_tone_state(&self) -> Option<Arc<ToneState>> {
        self.bio_tone_state.as_ref().map(Arc::clone)
    }

    /// 真实反馈事件源注入点 (attach): 把共享调质状态交给既有反馈点.
    ///
    /// **默认关 = `false` 且回调不执行** (零订阅、零状态、零行为变化). 开关开时
    /// 回调收到与注册模块**同一** `Arc<ToneState>`, 事件源在自己的既有反馈发生点
    /// 旁路调用 [`ToneState::apply_feedback`] —— 只交句柄, 不新建总线、不复制状态、
    /// 不叠加第二层缓冲; 重复 attach 不改变事件口径 (一次调用 = 一次脉冲).
    pub fn attach_tone_feedback_sink(&self, register: impl FnOnce(Arc<ToneState>)) -> bool {
        match &self.bio_tone_state {
            Some(state) => {
                register(Arc::clone(state));
                true
            }
            None => false,
        }
    }

    /// 生物机制族 W3.5 (SG-18): 共享生命周期状态句柄 (开关关 = `None`).
    ///
    /// 与注册模块同一 `Arc`; 只读观测入口, 不触发 `evaluate`、不改账本。
    /// self_status 的 `bio_lifecycle_probe` 由 Lead 后续经本口迟绑定注册.
    #[must_use]
    pub fn bio_lifecycle_state(&self) -> Option<Arc<BioLifecycleState>> {
        self.bio_lifecycle_state.as_ref().map(Arc::clone)
    }

    /// 生物机制族 W3.6: 自述 probe 的迟绑定共享槽句柄 (与装配根同一组槽).
    ///
    /// 组合根在 `build` 之后用它填 care / 痛觉再生 / 审议门句柄 (免疫记忆经
    /// [`Self::attach_immune_memory`] 同时填)。自述开关关时槽未武装, 所有
    /// `fill_*` 一律 no-op —— 默认关零变化, 回滚 = 不调用 fill.
    #[must_use]
    pub fn bio_probe_slots(&self) -> BioProbeSlots {
        self.bio_probe_slots.clone()
    }
}

#[cfg(test)]
#[path = "bio_production_wiring/tests.rs"]
mod tests;
