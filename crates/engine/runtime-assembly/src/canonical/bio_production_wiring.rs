//! 生物机制族生产装配扩展点（W3.2 / W3.3 / W3.5，SG-18）。
//!
//! `production.rs` 只保留调用与字段；本文件承载：
//! - [`install_bio_modules`]：W3.2 `BioAssemblyModule` + W3.3 `bio_tone` +
//!   W3.5 `bio_lifecycle` 的注册点（env 默认关，关 = 不构造不注册）；
//! - [`install_bio_play`]：W3.3 游戏练习场四工具注册点（env 默认关）；
//! - [`install_bio_self_status`]：W3.2 自述 `bio_mechanisms` 子对象挂面；
//! - [`ProductionModules`] 的生物句柄 getter（字段定义仍在 `production.rs`）。
//!
//! 本模块由 `production.rs` 以 `#[path = "bio_production_wiring.rs"]` 声明为
//! 子模块（`canonical.rs` 本批禁改；子模块天然可见装配根私有字段）。
//!
//! # 0 装诚实
//! - 生命周期模块**不新增 `ModuleFamily` 变体**：env 开也只注册模块，不上
//!   构造探针统计（构造点不上报，避免为单一模块膨胀族枚举）；
//! - 状态仅进程内（无持久化）；`bio_lifecycle_probe` 未注册进 self_status
//!   （Lead 后续）；无 live E2E。

#![forbid(unsafe_code)]

use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_orchestration::task_queue::TaskQueue;
use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::self_status::BioMechanismKind;

use crate::canonical::capability::CapabilityProvider;
use crate::canonical::care_heartbeat::bio_lifecycle::{
    bio_lifecycle_enabled_from_env_value, bio_lifecycle_wiring_from_env_value, BioLifecycleState,
    LifecycleClock, BIO_LIFECYCLE_ENABLE_ENV,
};
use crate::canonical::care_heartbeat::bio_tone::{bio_tone_wiring_from_env, ToneState};
use crate::canonical::cognitive::{BioAssemblyModule, BioAssemblySwitches};
use crate::canonical::module::Module;
use crate::canonical::play_tools::BioPlayState;
use crate::canonical::self_status_source::{
    capacity_bio_probe, play_probe, ProductionSelfStatusSource,
};

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
) -> BioWiringHandles {
    // W3.2: 修复指令段 + 策略化上下文块的 TurnStart overlay 装配点。
    // 两个开关默认关（`APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE` /
    // `APEIRETH_ENABLE_BIO_CONTEXT_POLICY`，只认 "1"）——**全关 = 不注册**
    // （懒加载语义：未启用 = 不构造 = 不注册，探针零上报）；开时也只装配
    // 装配根**显式注入**的 `RepairPlan` / 上下文块 / 预算。
    let bio_switches = BioAssemblySwitches::from_env();
    if !bio_switches.is_disabled() {
        modules.push(Arc::new(BioAssemblyModule::new(bio_switches)));
    }

    // W3.3: 调质→tone 的 TurnStart 尾部 overlay 装配点。`APEIRETH_ENABLE_BIO_TONE`
    // 只认 "1" —— **全关 = 不构造不注册**。开时构造共享 `ToneState` + 模块，
    // 并把状态句柄交给装配根（`bio_tone_state` / `attach_tone_feedback_sink`）。
    let mut tone_state = None;
    if let Some(wiring) = bio_tone_wiring_from_env({
        let clock = Arc::clone(clock);
        Arc::new(move || clock.now().timestamp_millis() as u64)
    }) {
        tone_state = Some(Arc::clone(wiring.state()));
        modules.push(wiring.module() as Arc<dyn Module>);
    }

    // W3.5 (SG-18): 资产生命周期观测模块。`APEIRETH_ENABLE_BIO_LIFECYCLE` 只认
    // "1" —— 关 = 不构造状态、不构造模块、一次时钟都不读。开时以装配根的真实
    // 工具声明名播种 `Newborn` 资产，状态句柄交给装配根供自述 probe 迟绑定。
    let mut lifecycle_state = None;
    let lifecycle_raw = std::env::var(BIO_LIFECYCLE_ENABLE_ENV).ok();
    if bio_lifecycle_enabled_from_env_value(lifecycle_raw.as_deref()) {
        let lifecycle_clock: LifecycleClock = {
            let clock = Arc::clone(clock);
            Arc::new(move || clock.now().timestamp_millis() as u64)
        };
        let registered_tool_names: Vec<String> = capabilities
            .iter()
            .map(|tool| tool.declaration().name)
            .collect();
        if let Some(wiring) = bio_lifecycle_wiring_from_env_value(
            lifecycle_raw.as_deref(),
            lifecycle_clock,
            &registered_tool_names,
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
) -> Option<Arc<BioPlayState>> {
    let play =
        crate::canonical::play_tools::bio_play_wiring_from_env(workspace_root, Arc::clone(clock))?;
    let state = Arc::clone(play.state());
    for module in play.into_modules() {
        instrument.constructed(ModuleFamily::BioPlay);
        capabilities.extend(module.capabilities());
    }
    Some(state)
}

/// W3.2 自述 `capabilities.bio_mechanisms` 挂面。
///
/// 挂面 = 子对象恒在场（开关关时 `mechanisms: null` + 原因「未启用」，
/// 零 probe 调用；开关 `APEIRETH_ENABLE_BIO_SELF_STATUS` 只认 "1"）。
/// 承载力探测口读**同一本**任务队列（与 `tool.task_board` 同队同账）；
/// 练习场账本（W3.5 接入）与四工具面同一 `Arc`；其余 12 条一律
/// `wired:false` + 「未接线」，不编数值。
pub(super) fn install_bio_self_status(
    source: ProductionSelfStatusSource,
    task_board: Option<&Arc<TaskQueue>>,
    bio_play_state: Option<&Arc<BioPlayState>>,
) -> ProductionSelfStatusSource {
    let mut source = source.with_bio_self_status_from_env();
    if let Some(queue) = task_board {
        source.register_bio_probe(
            BioMechanismKind::Capacity,
            capacity_bio_probe(Arc::clone(queue), None),
        );
    }
    if let Some(play) = bio_play_state {
        source.register_bio_probe(BioMechanismKind::Play, play_probe(Arc::clone(play)));
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
}
