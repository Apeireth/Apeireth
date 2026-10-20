//! 生物机制族 W3.5 · 资产生命周期装配面（SG-18，默认关）。
//!
//! # 机制与信号链（只观测记账，0 动作）
//!
//! 内核 [`apeireth_orchestration::lifecycle`]（W2 已入库，本批**零改动**）：
//! `Newborn → Trial → Mature → Retiring → Retired` 状态机 + 固定容量 +
//! 墓碑可恢复。本模块只做四件事：
//!
//! ```text
//! 工具声明清单 (装配根注入)                 ── seed_tool_names ──┐
//! HookPoint::AfterToolResult (真实派发结果) ── observe_tool_outcome ─┤
//!                                                                  ├─ LifecycleLedger (内存, 注入时钟)
//! HookPoint::AfterTurn (真实回合收口)      ── evaluate ────────────┘
//!                                                                  └─ BioLifecycleObservation / BioMechanismProbe
//! ```
//!
//! - **信号源 1（真实）**：`HookPoint::AfterToolResult` 的 `tool_result.is_ok()`
//!   与工具名（优先 `ToolResult::name`，回退 `ToolCall::name`）。每次真实派发
//!   记一次成功使用或失败尝试。
//! - **信号源 2（真实）**：装配根调用 [`ProductionModules::capabilities`]
//!   得到的工具声明清单，经 [`bio_lifecycle_wiring_from_env_value`] 注入为
//!   `Newborn` 资产（**注册事实**，不是编造使用）。所以长期未被触达的工具
//!   会按内核策略进入 `Retiring` / `Retired(Unused)`。
//! - **信号源 3（真实）**：`HookPoint::AfterTurn` 收口时按注入时钟跑一次
//!   `evaluate`，迁移只在真实回合边界发生（不在钩子里读系统时钟本身）。
//!
//! 没有真实信号的三类资产（`Skill` / `MemoryTrack` / `Plugin`）**不注册**：
//! 运行时模块 ABI 只暴露工具派发结果，不暴露技能调用、记忆轨道触达、插件
//! 装载/卸载事件；凭格式编造使用次数 = 伪接线。内核仍支持这三类，留给有
//! 真实事件源的接线批。
//!
//! # 开关（默认关，fail-closed）
//!
//! 装配门 = env [`BIO_LIFECYCLE_ENABLE_ENV`]（`APEIRETH_ENABLE_BIO_LIFECYCLE`，
//! **只认 `"1"`**；未设置 / 空 / `0` / `true` / `on` / 其它一律关）。
//! 关 = 不构造状态、不构造模块、**一次时钟都不读**（懒加载）。
//! [`BioLifecycleModule::new`] 的 `enabled = false` 是第二道门（defense-in-depth）：
//! 钩子直接 `ModuleOutcome::continue_()`，不读时钟、不碰账本。
//!
//! # 零变化语义（三层）
//!
//! 1. **开关关**：不注册任何模块/钩子，装配行为与未接线逐字节一致；
//! 2. **开关开**：本模块是**纯观测模块** —— 所有钩子恒回
//!    `ModuleOutcome::continue_()`，零 prompt overlay、零 `Retry`/`Stop` 指令，
//!    不改写会话、不短路工具派发、不改变回合结果；唯一副作用是进程内内存账；
//! 3. **不删除、不降权、不注销**：内核只给判定，本装配面连判定都不执行
//!    （不使用 `retire`/`revive` 做动作，不触碰工具注册表）；墓碑只落在账本里。
//!
//! # 0 装诚实（能力级别：IMPLEMENTED + 接线入口已备；**非** PRODUCTION WIRED）
//!
//! - `production.rs` 本批禁改（另一施工线独占）：**装配根尚未注册本模块**，
//!   [`bio_lifecycle_wiring_from_env`] 是留给 Lead 总装的入口；
//! - `canonical.rs` 本批禁改（Lead 独占）：模块暂挂在
//!   [`super::care_heartbeat`] 下编译，对外路径
//!   `canonical::care_heartbeat::bio_lifecycle`；Lead 注册时可迁到
//!   `canonical.rs`（见 `care_heartbeat.rs` 顶部同款迁移说明）；
//! - **self_status 未注册**：本模块提供 [`bio_lifecycle_probe`]
//!   （`BioMechanismProbe`，值 = 账本只读快照），但注册一行
//!   `register_bio_probe(BioMechanismKind::Lifecycle, ...)` 由 Lead 在装配根做；
//! - **无持久化 / 无遥测落盘 / 无 live E2E**：重启丢账本；`last_transitions`
//!   只保留最近一次非空 `evaluate` 批次；
//! - **不宣称清理**：题面「生命周期→工具/技能/记忆清理」中的「清理」本批不做，
//!   内核文档也明确「只给判定，不做动作」；本装配面保持同一纪律。
//!
//! # 不变量
//!
//! - 账本迭代/迁移顺序恒为 `asset_id` 字典序（内核 `BTreeMap`），同输入同输出；
//! - 所有时间由 [`LifecycleClock`] 注入（生产 = 系统时钟，测试 = 冻结/步进时钟），
//!   本模块自身不读系统时钟；
//! - `Retired` 资产被再次使用时**不自动复活**（不静默抹掉墓碑）：只累加
//!   `rejected_after_retire_total` 并更新 `last_action`，复活仍走内核显式
//!   `revive`（本装配面不提供动作入口）；
//! - 容量淘汰不是静默丢记录：`evicted_total` 累加，被淘汰记录由内核按规则
//!   补墓碑后交还（本模块只计数，不归档——归档面留给上层）。

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_orchestration::lifecycle::{
    AssetKind, LifecycleError, LifecycleLedger, LifecyclePolicy, LifecycleRecord,
    LifecycleTransition, DEFAULT_CAPACITY,
};
use apeireth_tools_canonical::self_status::{BioMechanismProbe, BioMechanismReading};
use serde::{Deserialize, Serialize};

use crate::canonical::module::{
    AgentModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};

/// 装配开闸 env 名（仅显式 `"1"` 开，其余一律关 fail-closed）。
pub const BIO_LIFECYCLE_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_LIFECYCLE";

/// 模块稳定标识（自述 / 注册去重 / 测试断言共用）。
pub const BIO_LIFECYCLE_MODULE_ID: &str = "bio_lifecycle";

/// `last_action` 摘要的字符上限（观测面有界，避免工具名/原因无界膨胀）。
const LAST_ACTION_MAX_CHARS: usize = 120;

/// 生命周期时钟：返回 epoch 毫秒（`u64`）。时间显式注入，账本不读系统时钟。
pub type LifecycleClock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// 开关解析（纯函数）：仅显式 `"1"` 开；未设置 / 空 / `0` / `true` / `on` /
/// 任意其它值一律关（默认关 fail-closed）。`trim` 后比较，因此 `" 1 "` 算开。
#[must_use]
pub fn bio_lifecycle_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 一次账本观测（只读快照，自述面 / 诊断 / 测试真值）。
///
/// 字段全部来自内核账本现读值，不做二次解释；`stage_counts` 的键是内核
/// [`LifecycleStage::as_str`] 的稳定短名（只含出现的阶段）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BioLifecycleObservation {
    /// 账本容量上限（含墓碑）。
    pub capacity: usize,
    /// 记录条数（含墓碑）。
    pub assets_total: usize,
    /// 未退役记录数（`Retiring` 仍算活着）。
    pub active_total: usize,
    /// 已退役墓碑数。
    pub retired_total: usize,
    /// 按稳定阶段短名统计（只含当前出现的阶段；顺序由 `BTreeMap` 固定）。
    pub stage_counts: BTreeMap<String, u64>,
    /// 累计迁移数（`evaluate` 返回值之和，单调不减）。
    pub transitions_total: u64,
    /// 累计容量淘汰数（内核交还的记录，不静默丢）。
    pub evicted_total: u64,
    /// 累计「已退役资产再次被真实使用」次数（拒绝静默复活）。
    pub rejected_after_retire_total: u64,
    /// 最近一次**非空** `evaluate` 的迁移批次（按内核确定性顺序）。
    pub last_transitions: Vec<LifecycleTransition>,
    /// 最近一次观测/迁移的有界摘要；尚无动作 = `None`。
    pub last_action: Option<String>,
    /// 当前生效策略（阈值真值，供自述阈值字段使用）。
    pub policy: LifecyclePolicy,
}

/// 生命周期装配状态（装配面唯一持有者）：内存账本 + 注入时钟 + 只读计数器。
///
/// 线程安全：`Mutex<LifecycleLedger>` + 原子计数器；钩子串行执行，锁竞争可忽略。
/// **不读系统时钟、不落盘、不触碰任何外部资产**。
pub struct BioLifecycleState {
    ledger: Mutex<LifecycleLedger>,
    clock: LifecycleClock,
    last_transitions: Mutex<Vec<LifecycleTransition>>,
    transitions_total: AtomicU64,
    evicted_total: AtomicU64,
    rejected_after_retire_total: AtomicU64,
    last_action: Mutex<Option<String>>,
}

impl BioLifecycleState {
    /// 以文档化默认策略 + 默认容量（[`DEFAULT_CAPACITY`]）构造；时钟注入。
    #[must_use]
    pub fn new(clock: LifecycleClock) -> Self {
        Self::with_policy(LifecyclePolicy::default(), DEFAULT_CAPACITY, clock)
    }

    /// 以显式策略 + 容量构造（测试 / 上层调参用；容量由内核钳到至少 1）。
    #[must_use]
    pub fn with_policy(policy: LifecyclePolicy, capacity: usize, clock: LifecycleClock) -> Self {
        Self {
            ledger: Mutex::new(LifecycleLedger::new(policy, capacity)),
            clock,
            last_transitions: Mutex::new(Vec::new()),
            transitions_total: AtomicU64::new(0),
            evicted_total: AtomicU64::new(0),
            rejected_after_retire_total: AtomicU64::new(0),
            last_action: Mutex::new(None),
        }
    }

    /// 注入时钟现值（毫秒）。模块的每个观测钩子恰好读一次。
    #[must_use]
    pub fn now_ms(&self) -> u64 {
        (self.clock)()
    }

    /// 账本只读快照（克隆；宿主 / 探测口可读全量记录与墓碑）。
    #[must_use]
    pub fn ledger(&self) -> LifecycleLedger {
        self.lock_ledger().clone()
    }

    /// 把装配根给出的工具声明名注册为 `Newborn` 资产（时刻取注入时钟）。
    ///
    /// 返回新增条数；重复名（含已退役墓碑）按「历史不被静默覆盖」跳过，
    /// 空白名跳过。注册即事实：它们尚未被使用，`use_count = 0`。
    pub fn seed_tool_names(&self, tool_names: &[String]) -> usize {
        let now_ms = self.now_ms();
        self.seed_tool_names_at(tool_names, now_ms)
    }

    /// [`seed_tool_names`](Self::seed_tool_names) 的显式时刻形态（测试 / 回放）。
    pub fn seed_tool_names_at(&self, tool_names: &[String], now_ms: u64) -> usize {
        let mut ledger = self.lock_ledger();
        let mut added = 0usize;
        for tool_name in tool_names {
            let name = tool_name.trim();
            if name.is_empty() {
                continue;
            }
            match ledger.register(name, AssetKind::Tool, now_ms) {
                Ok(registration) => {
                    added = added.saturating_add(1);
                    if let Some(evicted) = registration.evicted {
                        self.note_eviction(&evicted);
                    }
                }
                // 重复声明：生态以名字为主键，首次出现为准（不覆盖墓碑）。
                Err(LifecycleError::AlreadyExists(_)) => {}
                // 空白名已在上面过滤；其它错误在 `register` 语义下不可达。
                Err(_) => {}
            }
        }
        if added > 0 {
            self.set_last_action(format!("seeded {added} tools"));
        }
        added
    }

    /// 记录一次真实工具派发结果（时刻取注入时钟）。
    pub fn observe_tool_outcome(&self, tool_name: &str, ok: bool) -> Result<(), String> {
        let now_ms = self.now_ms();
        self.observe_tool_outcome_at(tool_name, ok, now_ms)
    }

    /// [`observe_tool_outcome`](Self::observe_tool_outcome) 的显式时刻形态。
    ///
    /// - 未注册工具（动态发现 / 装配后新增）：先补注册为 `Newborn` 再记账；
    /// - 成功 → `record_use`（`Retiring` 被救回 `Trial` 的内核语义不变）；
    /// - 失败 → `record_failure`（只记试用证据，不直接改阶段；退役由
    ///   `evaluate` 统一做）；
    /// - 已退役资产再次被使用 → **不自动复活**，累加拒绝计数并返回 `Ok(())`
    ///   （观测模块不得因账本状态打断真实回合）。
    pub fn observe_tool_outcome_at(
        &self,
        tool_name: &str,
        ok: bool,
        at_ms: u64,
    ) -> Result<(), String> {
        let name = tool_name.trim();
        if name.is_empty() {
            return Err("tool name must not be blank".to_string());
        }
        let mut ledger = self.lock_ledger();
        if ledger.get(name).is_none() {
            let registration = ledger
                .register(name, AssetKind::Tool, at_ms)
                .map_err(|error| error.to_string())?;
            if let Some(evicted) = registration.evicted {
                self.note_eviction(&evicted);
            }
        }
        let recorded = if ok {
            ledger.record_use(name, at_ms)
        } else {
            ledger.record_failure(name, at_ms)
        };
        match recorded {
            Ok(()) => {
                let label = if ok { "ok" } else { "error" };
                self.set_last_action(format!("tool_result {name} {label}"));
                Ok(())
            }
            Err(LifecycleError::AlreadyRetired(_)) => {
                self.rejected_after_retire_total
                    .fetch_add(1, Ordering::SeqCst);
                self.set_last_action(format!("tool_result {name} ignored (retired)"));
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// 按注入时钟跑一次 `evaluate`，返回并留存迁移。
    pub fn evaluate(&self) -> Vec<LifecycleTransition> {
        let now_ms = self.now_ms();
        self.evaluate_at(now_ms)
    }

    /// [`evaluate`](Self::evaluate) 的显式时刻形态（测试 / 回放）。
    pub fn evaluate_at(&self, now_ms: u64) -> Vec<LifecycleTransition> {
        let transitions = self.lock_ledger().evaluate(now_ms);
        if transitions.is_empty() {
            return transitions;
        }
        self.transitions_total
            .fetch_add(transitions.len() as u64, Ordering::SeqCst);
        if let Some(last) = transitions.last() {
            self.set_last_action(describe_transition(last));
        }
        *self.lock_last_transitions() = transitions.clone();
        transitions
    }

    /// 只读观测快照（确定性：阶段计数与最近迁移批次均有固定顺序）。
    #[must_use]
    pub fn observation(&self) -> BioLifecycleObservation {
        let ledger = self.lock_ledger();
        let mut stage_counts: BTreeMap<String, u64> = BTreeMap::new();
        for record in ledger.active() {
            *stage_counts
                .entry(record.stage.as_str().to_string())
                .or_insert(0) += 1;
        }
        for record in ledger.retired() {
            *stage_counts
                .entry(record.stage.as_str().to_string())
                .or_insert(0) += 1;
        }
        BioLifecycleObservation {
            capacity: ledger.capacity(),
            assets_total: ledger.len(),
            active_total: ledger.active().len(),
            retired_total: ledger.retired().len(),
            stage_counts,
            transitions_total: self.transitions_total.load(Ordering::SeqCst),
            evicted_total: self.evicted_total.load(Ordering::SeqCst),
            rejected_after_retire_total: self.rejected_after_retire_total.load(Ordering::SeqCst),
            last_transitions: self.lock_last_transitions().clone(),
            last_action: self.lock_last_action().clone(),
            policy: ledger.policy(),
        }
    }

    fn note_eviction(&self, evicted: &LifecycleRecord) {
        self.evicted_total.fetch_add(1, Ordering::SeqCst);
        self.set_last_action(format!(
            "evicted {} ({})",
            evicted.asset_id,
            evicted.stage.as_str()
        ));
    }

    fn set_last_action(&self, action: String) {
        *self.lock_last_action() = Some(bounded(action));
    }

    fn lock_ledger(&self) -> MutexGuard<'_, LifecycleLedger> {
        self.ledger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_last_transitions(&self) -> MutexGuard<'_, Vec<LifecycleTransition>> {
        self.last_transitions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_last_action(&self) -> MutexGuard<'_, Option<String>> {
        self.last_action
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl std::fmt::Debug for BioLifecycleState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BioLifecycleState")
            .field("observation", &self.observation())
            .finish_non_exhaustive()
    }
}

/// 迁移的有界人读摘要（观测面 `last_action` 用，不含用户原文）。
fn describe_transition(transition: &LifecycleTransition) -> String {
    match transition {
        LifecycleTransition::Promoted {
            asset_id, from, to, ..
        } => format!("promoted {} {} -> {}", asset_id, from.as_str(), to.as_str()),
        LifecycleTransition::EnteredRetiring { asset_id, .. } => {
            format!("entered_retiring {asset_id}")
        }
        LifecycleTransition::Retired {
            asset_id, reason, ..
        } => {
            format!("retired {} ({})", asset_id, reason.as_str())
        }
        LifecycleTransition::Revived { asset_id, .. } => format!("revived {asset_id}"),
    }
}

/// 字符级截断（不按字节切，避免多字节断言面出现半个字符）。
fn bounded(text: String) -> String {
    if text.chars().count() <= LAST_ACTION_MAX_CHARS {
        return text;
    }
    let mut truncated: String = text.chars().take(LAST_ACTION_MAX_CHARS).collect();
    truncated.push('…');
    truncated
}

/// 资产生命周期观测模块（`AfterToolResult` + `AfterTurn` 两个真实钩子）。
///
/// - 关 → `ModuleOutcome::continue_()`，不读时钟、不碰账本；
/// - 开 → 记录真实工具结果、回合收口评估一次，**恒回 `continue_()`**：
///   零 overlay、零指令，不改变回合任何可观察行为。
pub struct BioLifecycleModule {
    manifest: ModuleManifest,
    enabled: bool,
    state: Arc<BioLifecycleState>,
}

impl BioLifecycleModule {
    /// 按显式开关 + 共享生命周期状态构造。
    #[must_use]
    pub fn new(enabled: bool, state: Arc<BioLifecycleState>) -> Self {
        Self {
            manifest: ModuleManifest::new(BIO_LIFECYCLE_MODULE_ID, "Bio Asset Lifecycle"),
            enabled,
            state,
        }
    }

    /// 开关现况。
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 共享生命周期状态（观测 / 手动注入同一真值）。
    #[must_use]
    pub fn state(&self) -> &Arc<BioLifecycleState> {
        &self.state
    }

    /// 把一次工具结果映射进账本；返回是否真的记录了一次结果。
    ///
    /// 工具名优先取 `ToolResult::name`，缺省回退 `ToolCall::name`；两者都无
    /// （或为空白）= 本钩子没有可归属资产，跳过（不编名字）。
    #[must_use]
    pub fn record_tool_result(
        &self,
        tool_call: Option<&apeireth_protocol::canonical::ToolCall>,
        tool_result: Option<&apeireth_protocol::canonical::ToolResult>,
    ) -> bool {
        let Some(result) = tool_result else {
            return false;
        };
        let name = result
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                tool_call
                    .map(|call| call.name.as_str())
                    .filter(|name| !name.trim().is_empty())
            });
        let Some(name) = name else {
            return false;
        };
        self.state
            .observe_tool_outcome(name, result.is_ok())
            .is_ok()
    }
}

#[async_trait::async_trait]
impl AgentModule for BioLifecycleModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if !self.enabled {
            return Ok(ModuleOutcome::continue_());
        }
        match hook {
            HookPoint::AfterToolResult => {
                self.record_tool_result(ctx.tool_call, ctx.tool_result);
            }
            HookPoint::AfterTurn => {
                self.state.evaluate();
            }
            _ => {}
        }
        Ok(ModuleOutcome::continue_())
    }
}

/// 装配结果：共享生命周期状态 + 一个模块（`module()` 交给装配根注册）。
pub struct BioLifecycleWiring {
    state: Arc<BioLifecycleState>,
    module: Arc<dyn AgentModule>,
}

impl BioLifecycleWiring {
    /// 共享生命周期状态（观测口 / 后续显式治理动作同一真值）。
    #[must_use]
    pub fn state(&self) -> &Arc<BioLifecycleState> {
        &self.state
    }

    /// 待注册模块（`RuntimeBuilder::with_module` / `ProductionModules` 同口）。
    #[must_use]
    pub fn module(&self) -> Arc<dyn AgentModule> {
        Arc::clone(&self.module)
    }
}

/// 开关值 → 模块（开关关 = `None`；供宿主自带 [`BioLifecycleState`] 时使用）。
#[must_use]
pub fn bio_lifecycle_module_from_env_value(
    raw: Option<&str>,
    state: Arc<BioLifecycleState>,
) -> Option<Arc<dyn AgentModule>> {
    bio_lifecycle_enabled_from_env_value(raw)
        .then(|| Arc::new(BioLifecycleModule::new(true, state)) as Arc<dyn AgentModule>)
}

/// 开关值 → (共享状态 + 模块)；开关关 = `None`（懒加载，一次时钟都不读）。
///
/// `registered_tool_names` 是装配根持有的**真实工具声明名**（例如
/// `ProductionModules::capabilities().iter().map(|tool| tool.declaration().name)`）：
/// 它们以 `Newborn` 入账，之后按真实派发结果记账。传空切片 = 只跟踪运行期
/// 首次出现的工具（动态发现场景），不编造种子。
#[must_use]
pub fn bio_lifecycle_wiring_from_env_value(
    raw: Option<&str>,
    clock: LifecycleClock,
    registered_tool_names: &[String],
) -> Option<BioLifecycleWiring> {
    if !bio_lifecycle_enabled_from_env_value(raw) {
        return None;
    }
    let state = Arc::new(BioLifecycleState::new(clock));
    state.seed_tool_names(registered_tool_names);
    let module: Arc<dyn AgentModule> = Arc::new(BioLifecycleModule::new(true, Arc::clone(&state)));
    Some(BioLifecycleWiring { state, module })
}

/// 从 env 现值装配（同 [`bio_lifecycle_wiring_from_env_value`] 的 fail-closed 口径）。
#[must_use]
pub fn bio_lifecycle_wiring_from_env(
    clock: LifecycleClock,
    registered_tool_names: &[String],
) -> Option<BioLifecycleWiring> {
    bio_lifecycle_wiring_from_env_value(
        std::env::var(BIO_LIFECYCLE_ENABLE_ENV).ok().as_deref(),
        clock,
        registered_tool_names,
    )
}

/// self_status 观测口：`capabilities.bio_mechanisms.lifecycle` 的只读探测口。
///
/// 由装配根注册（`BioMechanismRegistry::register_bio_probe(BioMechanismKind::Lifecycle,
/// bio_lifecycle_probe(state))`）；**本批未注册**（`production.rs` 禁改）。
/// 探测是纯读：不 `evaluate`、不改账本、不读系统时钟（时刻由注册表传入）。
#[must_use]
pub fn bio_lifecycle_probe(state: Arc<BioLifecycleState>) -> Arc<dyn BioMechanismProbe> {
    Arc::new(BioLifecycleProbe { state })
}

/// 探测口本体（只读现读账本）。
struct BioLifecycleProbe {
    state: Arc<BioLifecycleState>,
}

impl BioMechanismProbe for BioLifecycleProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let observation = self.state.observation();
        let mut value = serde_json::to_value(&observation)
            .map_err(|error| format!("lifecycle observation serialize failed: {error}"))?;
        if let Some(object) = value.as_object_mut() {
            object.insert("observed_at_ms".to_string(), serde_json::json!(now_ms));
        }
        // 阈值 = 闲置退役线（毫秒），与内核策略同源；展示口径为 JSON number。
        let threshold = observation.policy.retire_after_unused_ms as f64;
        Ok(BioMechanismReading::wired(
            Some(value),
            Some(threshold),
            observation.last_action,
        ))
    }
}

#[cfg(test)]
mod tests {
    use apeireth_orchestration::lifecycle::LifecycleStage;

    use super::*;

    const NOW_MS: u64 = 1_760_000_000_000;

    fn frozen_clock() -> LifecycleClock {
        Arc::new(|| NOW_MS)
    }

    fn state() -> Arc<BioLifecycleState> {
        Arc::new(BioLifecycleState::new(frozen_clock()))
    }

    #[test]
    fn env_switch_only_accepts_explicit_one_and_is_lazy() {
        assert_eq!(BIO_LIFECYCLE_ENABLE_ENV, "APEIRETH_ENABLE_BIO_LIFECYCLE");
        assert!(bio_lifecycle_enabled_from_env_value(Some("1")));
        assert!(bio_lifecycle_enabled_from_env_value(Some(" 1 ")));
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
                "非 \"1\" 一律关 (fail-closed): {raw:?}"
            );
            assert!(
                bio_lifecycle_wiring_from_env_value(raw, frozen_clock(), &[]).is_none(),
                "关 = 不构造: {raw:?}"
            );
        }
        let wiring =
            bio_lifecycle_wiring_from_env_value(Some("1"), frozen_clock(), &[]).expect("开");
        assert_eq!(wiring.module().manifest().id, BIO_LIFECYCLE_MODULE_ID);
        assert_eq!(wiring.state().observation().assets_total, 0);
    }

    #[test]
    fn seeded_assets_are_newborn_and_promote_on_use_evaluate() {
        let state = state();
        let seeds = ["filesystem".to_string(), "search".to_string()];
        assert_eq!(state.seed_tool_names(&seeds), 2);
        let observation = state.observation();
        assert_eq!(observation.assets_total, 2);
        assert_eq!(observation.stage_counts.get("newborn"), Some(&2));
        assert_eq!(observation.stage_counts.get("trial"), None);

        state
            .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
            .expect("record use");
        assert_eq!(
            state.ledger().get("filesystem").expect("asset").stage,
            LifecycleStage::Newborn,
            "使用本身只记账, 晋升由 evaluate 决定"
        );
        let transitions = state.evaluate_at(NOW_MS + 2);
        assert_eq!(transitions.len(), 1);
        assert_eq!(state.observation().stage_counts.get("trial"), Some(&1));
        assert_eq!(state.observation().transitions_total, 1);
    }

    #[test]
    fn observation_is_deterministic_under_registration_order() {
        let forward = state();
        forward.seed_tool_names_at(&["tool.b".to_string(), "tool.a".to_string()], NOW_MS);
        let backward = state();
        backward.seed_tool_names_at(&["tool.a".to_string(), "tool.b".to_string()], NOW_MS);
        for state in [&forward, &backward] {
            state
                .observe_tool_outcome_at("tool.a", true, NOW_MS + 1)
                .expect("use");
            state
                .observe_tool_outcome_at("tool.b", false, NOW_MS + 1)
                .expect("failure");
            state.evaluate_at(NOW_MS + 2);
        }
        // last_action 含首次注册批次时可能不同; 迁移/计数/阶段必须一致。
        let left = forward.observation();
        let right = backward.observation();
        assert_eq!(left.stage_counts, right.stage_counts);
        assert_eq!(left.last_transitions, right.last_transitions);
        assert_eq!(left.assets_total, right.assets_total);
    }

    #[test]
    fn the_probe_is_read_only_and_self_status_ready() {
        let state = state();
        state.seed_tool_names_at(&["tool.a".to_string()], NOW_MS);
        state
            .observe_tool_outcome_at("tool.a", true, NOW_MS + 1)
            .expect("use");
        state.evaluate_at(NOW_MS + 2);
        let probe = bio_lifecycle_probe(Arc::clone(&state));
        let before = state.observation();
        let reading = probe.snapshot(123).expect("probe reads");
        let after = state.observation();
        assert_eq!(before, after, "探测必须是纯读: 不 evaluate、不改账本");
        assert!(reading.wired);
        assert_eq!(reading.threshold, Some(90.0 * 24.0 * 60.0 * 60.0 * 1000.0));
        assert!(reading.last_action.is_some());
        let value = reading.value.expect("value");
        assert_eq!(value["assets_total"], 1);
        assert_eq!(value["observed_at_ms"], 123);
        assert_eq!(value["stage_counts"]["trial"], 1);
    }
}
