//! 生物机制族生产装配扩展点（W3.2 / W3.3 / W3.5 / W3.6，SG-18）。
//!
//! `production.rs` 只保留调用与字段；本文件承载：
//! - [`install_bio_modules`]：W3.2 `BioAssemblyModule` + W3.3 `bio_tone` +
//!   W3.5 `bio_lifecycle` 的注册点（env 默认关，关 = 不构造不注册）；
//! - [`install_bio_play`]：W3.3 游戏练习场四工具注册点（env 默认关）；
//! - [`install_bio_self_status`]：W3.2 自述 `bio_mechanisms` 子对象挂面；
//! - [`BioProbeSlots`]（W3.6）：自述 probe 的**共享迟绑定槽** —— probe 先注册、
//!   句柄后填（免疫记忆 / 关怀心跳 / 审议门账 / 痛觉再生读取口），默认关时
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
use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::self_status::{
    BioMechanismKind, BioMechanismProbe, BioMechanismReading,
};

use crate::canonical::capability::CapabilityProvider;
use crate::canonical::care_heartbeat::bio_lifecycle::{
    bio_lifecycle_wiring_from_enabled_with_store_dir, BioLifecycleState, LifecycleClock,
};
use crate::canonical::care_heartbeat::bio_tone::{bio_tone_wiring_from_enabled, ToneState};
use crate::canonical::care_heartbeat::CareHeartbeatStatus;
use crate::canonical::cognitive::{BioAssemblyModule, BioAssemblySwitches};
use crate::canonical::module::Module;
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
    /// W3.5 生命周期状态（与注册模块共享同一 `Arc`）。
    pub(super) lifecycle_state: Option<Arc<BioLifecycleState>>,
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
    if !bio_switches.is_disabled() {
        modules.push(Arc::new(BioAssemblyModule::new(bio_switches)));
    }

    // W3.3: 调质→tone 的 TurnStart 尾部 overlay 装配点。开关值来自
    // `NervousSwitchboard`（R1b-1b）—— **关 = 不构造不注册**。开时构造共享
    // `ToneState` + 模块，并把状态句柄交给装配根（`bio_tone_state` /
    // `attach_tone_feedback_sink`）。
    let mut tone_state = None;
    if let Some(wiring) =
        bio_tone_wiring_from_enabled(switchboard.switch_enabled(BioSwitch::Tone), {
            let clock = Arc::clone(clock);
            Arc::new(move || clock.now().timestamp_millis() as u64)
        })
    {
        tone_state = Some(Arc::clone(wiring.state()));
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
        lifecycle_state,
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
pub(super) fn install_bio_ritual_tools(
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    clock: &Arc<dyn Clock>,
    care_dir: Option<&std::path::Path>,
    enabled: bool,
) {
    let Some(wiring) = crate::canonical::ritual_tools::bio_ritual_wiring_from_enabled(
        enabled,
        care_dir,
        Arc::clone(clock),
    ) else {
        return;
    };
    for module in wiring.into_modules() {
        capabilities.extend(module.capabilities());
    }
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
/// - **开关开**：6 条 probe 先注册（接线事实 `wired:true`），句柄未到 = 该格
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

    /// 是否已武装（自述开关开 = 安装点注册了 6 条迟绑定 probe）。
    #[must_use]
    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }

    /// 五个槽是否全空（默认关证据：关时恒空，`fill_*` 全 no-op）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        lock_slot(&self.immune).is_none()
            && lock_slot(&self.care_status).is_none()
            && lock_slot(&self.quorum_trace).is_none()
            && lock_slot(&self.pain_reader).is_none()
            && self.ecosystem.is_empty()
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
/// 练习场账本（W3.5 接入）与四工具面同一 `Arc`；W3.6/W3.7 的 6 条迟绑定 probe
/// （免疫记忆 / 关怀心跳 / 审议门 / 痛觉 / 再生）只在开关开时注册 —— 注册即
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
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    use apeireth_guard::immune_memory::{ImmuneMemory, IncidentKind, SeverityTier};
    use apeireth_orchestration::nociception::{FailureClass, Habituation, Recommendation};
    use apeireth_orchestration::quorum::QuorumDecision;
    use apeireth_orchestration::task_queue::{TaskQueueConfig, TaskStore};
    use apeireth_runtime::canonical::{
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

    fn armed_slots() -> BioProbeSlots {
        let slots = BioProbeSlots::new();
        slots.arm(true);
        slots
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
        let _source =
            install_bio_self_status_with(source(), Some(&queue), Some(&play), &slots, false);
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

    /// ⑤ 空槽：6 条迟绑定 probe 注册即接线（`wired:true`），但值显式 `null`
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
        assert_eq!(unwired, 8, "未注册的 8 格如实未接线 (14 - 6 注册)");
    }

    /// ⑥ 全填 + 任务队列 + 练习账 + 工具生态观测：wired 计数 = 8/14，
    /// 且 8 格值全为真值（不是 null）。
    #[test]
    fn filled_slots_raise_the_wired_count_to_eight() {
        let slots = armed_slots();
        let queue = task_queue();
        let play = play_state();
        let source =
            install_bio_self_status_with(source(), Some(&queue), Some(&play), &slots, true);
        assert!(slots.fill_immune_memory(Some(immune_memory())));
        assert!(slots.fill_care_status(Arc::new(CareHeartbeatStatus::default)));
        assert!(slots.fill_quorum_trace(Arc::new(|| None)));
        assert!(slots.fill_pain_reader(pain_reader(pain_snapshot())));
        assert!(slots.fill_ecosystem_profiles(&[stub_tool("tool.filesystem", "filesystem")]));
        let status = bio_status(&source);
        let rows = status.mechanisms.as_ref().expect("14 格在场");
        let wired: Vec<&BioMechanismKind> = BioMechanismKind::ALL
            .iter()
            .filter(|kind| rows.get(kind).is_some_and(|row| row.wired))
            .collect();
        assert_eq!(
            wired.len(),
            8,
            "2 (capacity/play) + 6 迟绑定 (含工具生态) = 8/14"
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
}
