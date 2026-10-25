//! 生物机制族 W3.5 · 资产生命周期装配面（SG-18，默认关）。
//!
//! # 机制与信号链（只观测记账，0 动作）
//!
//! 内核 [`apeireth_orchestration::lifecycle`]（W2 已入库，本批**零改动**）：
//! `Newborn → Trial → Mature → Retiring → Retired` 状态机 + 固定容量 +
//! 墓碑可恢复。本模块只做四件事：
//!
//! ```text
//! 工具声明清单 (装配根注入)                      ── seed_tool_names ──┐
//! nervous.bridge → NervousBus (R1b-2b 总线订阅)  ── observe_tool_outcome ─┤
//!                                                                        ├─ LifecycleLedger (内存, 注入时钟)
//! HookPoint::AfterTurn (真实回合收口, 仍走钩子)  ── evaluate ────────────┘
//!                                                                        └─ BioLifecycleObservation / BioMechanismProbe
//! ```
//!
//! - **信号源 1（真实，R1b-2b 起走总线订阅）**：桥把 `HookPoint::AfterToolResult`
//!   翻成 `NervousSignal::ToolOutcome`（工具名 = 结果名优先、回退调用名），
//!   本模块的 [`BioLifecycleBusSubscriber`] 订阅后写入账本；模块 `on_hook`
//!   不再扫描该钩子（同一事件只推进一次，无双写）。
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
//! W3.6 起账本可选绑定文件账（见下节「可选持久化」），不改变以上信号口径与
//! 「零动作」语义：账本仍是内存真值，文件只是重启恢复用的快照。
//!
//! # 可选持久化（W3.6 收尾，默认关）
//!
//! 账本默认**纯内存**：不传目录 = 与 W3.5 逐字节一致（零文件、零读写）。
//! 显式传入 care 目录时，[`BioLifecycleState`] 绑定 [`LifecycleStateStore`]：
//!
//! - 启动装载 `<care_dir>/lifecycle_state.json`（StoredDoc 信封
//!   [`LIFECYCLE_STATE_DOC_NAME`] v1）；缺失 = 合法空账；坏档 = 空账 + 诊断 +
//!   **本进程拒写**（原档逐字节不动）；
//! - 写入时机：每回合收口 [`BioLifecycleState::evaluate`] 后，**且自上次成功
//!   落盘以来账本确有变化**时原子写一次（确定性：无 debounce、不读系统时钟）；
//!   显式 [`BioLifecycleState::persist`] 可提前刷盘。播种是每个启动从真实声明
//!   清单重建的注册事实，不单独落盘（随下一次回合收口一起写）；
//! - 持久化子集：全部 [`LifecycleRecord`]（含墓碑）+ 策略 + 容量 + 三个累计
//!   计数；`last_transitions` / `last_action` 是瞬时面，**不落盘**；
//! - 单写者假设：同进程一把锁串行读写；跨进程文件锁未做（见 store 模块注）。
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
//! - 装配根已由 Lead 总装注册（`installation via
//!   bio_production_wiring::install_bio_modules`，见 `production.rs`）；
//!   [`bio_lifecycle_wiring_from_enabled_with_store_dir`] 接收装配根已解析的
//!   布尔开关（R1b-1b），本模块不自行读 env；
//! - `canonical.rs` 的模块声明由 Lead 独占：模块暂挂在
//!   [`super::care_heartbeat`] 下编译，对外路径
//!   `canonical::care_heartbeat::bio_lifecycle`；Lead 注册时可迁到
//!   `canonical.rs`（见 `care_heartbeat.rs` 顶部同款迁移说明）；
//! - **self_status 已注册（R1b-2b）**：装配根经迟绑定槽注册
//!   `BioMechanismKind::Lifecycle` probe（[`bio_lifecycle_probe`]，值 = 账本只读快照）；
//!   开关开时候迟绑定槽被填，关时该格 `null` + 未接线真因（不编读数）；
//! - **持久化默认关**：文件账已实现并测试，但生产装配根尚未传入 care 目录
//!   （`bio_production_wiring.rs` 本批禁改，接线由 Lead 做一行）；不传目录 =
//!   纯内存 = 今天的行为，一次文件 I/O 都没有。**不是 DEFAULT ENABLED**；
//! - **无 telemetry 落盘 / 无 live E2E**：重启恢复由装配级测试用临时目录覆盖，
//!   未在真实宿主进程重启复测；`last_transitions` 只保留最近一次非空
//!   `evaluate` 批次（瞬时面，不落盘）；
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
//!   补墓碑后交还（本模块只计数，不归档——归档面留给上层）；
//! - 文件账（绑定时）：缺失 = 合法空态；坏档拒开且本进程拒写（原档不覆盖）；
//!   写只在 `evaluate` 后、且账本自上次成功落盘以来确有变化时发生一次，
//!   同输入序列 → 同字节；
//! - 默认关 / 无目录：零文件、零读写、零额外时钟读取，观测面与纯内存逐字节一致。

#![forbid(unsafe_code)]

// W3.6 收尾：文件账存储。本批禁改 `canonical.rs`，经 `#[path]` 挂在本模块下编译，
// 对外路径 `canonical::care_heartbeat::bio_lifecycle::lifecycle_state_store`；
// Lead 迁移本模块到 `canonical.rs` 时该子模块随父模块一起走，无需另加声明。
#[path = "lifecycle_state_store.rs"]
pub mod lifecycle_state_store;

pub use self::lifecycle_state_store::{
    lifecycle_state_doc_compat, LifecycleCounters, LifecycleStateBody, LifecycleStateStore,
    LifecycleStateStoreError, LIFECYCLE_STATE_DOC_NAME, LIFECYCLE_STATE_DOC_VERSION,
    LIFECYCLE_STATE_FILE_NAME,
};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_orchestration::lifecycle::{
    AssetKind, LifecycleError, LifecycleLedger, LifecyclePolicy, LifecycleRecord,
    LifecycleTransition, DEFAULT_CAPACITY,
};
use apeireth_tools_canonical::self_status::{BioMechanismProbe, BioMechanismReading};
use serde::{Deserialize, Serialize};

use crate::canonical::module::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use crate::canonical::nervous_bus::{NervousBus, NervousSignal, NervousSignalKind};

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
    /// **连续**工具失败数（D-3 关怀势能场 distress 信号源）。
    pub consecutive_tool_failures: u64,
    /// 最近一次工具成功时刻（epoch 毫秒）；`None` = 尚无成功。
    pub last_tool_success_ms: Option<u64>,
}

/// 生命周期装配状态（装配面唯一持有者）：内存账本 + 注入时钟 + 只读计数器。
///
/// 线程安全：`Mutex<LifecycleLedger>` + 原子计数器；钩子串行执行，锁竞争可忽略。
/// **不读系统时钟、不触碰任何外部资产**；文件 I/O 仅在显式绑定
/// [`LifecycleStateStore`] 后发生（默认不绑定 = 纯内存）。
pub struct BioLifecycleState {
    ledger: Mutex<LifecycleLedger>,
    clock: LifecycleClock,
    last_transitions: Mutex<Vec<LifecycleTransition>>,
    transitions_total: AtomicU64,
    evicted_total: AtomicU64,
    rejected_after_retire_total: AtomicU64,
    last_action: Mutex<Option<String>>,
    /// 可选文件账；`None` = 纯内存（默认，零文件 I/O）。
    store: Option<LifecycleStateStore>,
    /// 自上次成功落盘以来账本是否确有变化（决定 `evaluate` 后是否写一次）。
    dirty: AtomicBool,
    /// 持久化诊断（启动装载失败真因 / 落盘失败计数与最近真因）。
    persistence: Mutex<PersistenceDiagnostics>,
    /// **连续**工具失败数（成功即清零）。关怀势能场 distress 信号源（D-3）。
    ///
    /// 由 [`observe_tool_outcome_at`](Self::observe_tool_outcome_at) 在真实
    /// `ToolOutcome` 总线回调里维护 —— 不是估算、不是采样。
    consecutive_tool_failures: AtomicU64,
    /// 最近一次**工具成功**时刻（epoch 毫秒）；`0` = 尚无成功。
    /// 关怀势能场 fatigue / silence 信号源（D-3）。
    last_tool_success_ms: AtomicU64,
}

/// 持久化诊断（只增不减的失败计数 + 最近真因；启动装载失败 = 本进程拒写）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PersistenceDiagnostics {
    /// 启动装载失败真因；`Some` = 坏档，保持空账且**拒绝后续写入**。
    load_error: Option<String>,
    /// 落盘失败次数（进程内累计；成功不清零历史）。
    save_failures: u64,
    /// 最近一次落盘失败真因。
    last_save_error: Option<String>,
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
            store: None,
            dirty: AtomicBool::new(false),
            persistence: Mutex::new(PersistenceDiagnostics::default()),
            consecutive_tool_failures: AtomicU64::new(0),
            last_tool_success_ms: AtomicU64::new(0),
        }
    }

    /// 绑定文件账（**构造期链式调用**，启动即装载，不 panic）。
    ///
    /// - 文件缺失 = 合法空账（不创建文件）；
    /// - 装载成功 = 账本 / 策略 / 容量 / 累计计数原样恢复，瞬时观测面从空开始；
    /// - 坏档 = 空账 + [`Self::persistence_load_diagnostic`] 真因，且本进程
    ///   **拒绝写入**（不覆盖原档）。
    ///
    /// 运行期不换账：绑定后再调用本方法会用磁盘内容覆盖内存账，不在本 API
    /// 契约内（只应在 `new` / `with_policy` 之后立刻调用一次）。
    #[must_use]
    pub fn with_store(mut self, store: LifecycleStateStore) -> Self {
        self.store = Some(store);
        self.dirty.store(false, Ordering::SeqCst);
        *self.lock_persistence() = PersistenceDiagnostics::default();
        self.restore_from_store();
        self
    }

    /// [`Self::with_store`] 的 care 目录便捷形态：
    /// 绑定 `<care_dir>/lifecycle_state.json`（目录不存在时缺省为空账，
    /// 首次落盘原子写会按需创建父目录）。
    #[must_use]
    pub fn with_store_from_dir(mut self, care_dir: impl Into<PathBuf>) -> Self {
        self.with_store(LifecycleStateStore::under_care_dir(care_dir))
    }

    /// 是否绑定了文件账（默认 `false` = 纯内存，零文件 I/O）。
    #[must_use]
    pub fn persistence_enabled(&self) -> bool {
        self.store.is_some()
    }

    /// 绑定的 `lifecycle_state.json` 路径（未绑定 = `None`）。
    #[must_use]
    pub fn persistence_path(&self) -> Option<&Path> {
        self.store.as_ref().map(LifecycleStateStore::path)
    }

    /// 启动装载诊断：`None` = 文件缺失或装载成功；`Some` = 坏档真因
    /// （此时账本为空且本进程拒写）。
    #[must_use]
    pub fn persistence_load_diagnostic(&self) -> Option<String> {
        self.lock_persistence().load_error.clone()
    }

    /// 落盘失败次数（进程内累计；无文件账 = 恒 0）。
    #[must_use]
    pub fn persistence_save_failures(&self) -> u64 {
        self.lock_persistence().save_failures
    }

    /// 最近一次落盘失败真因（成功后保留历史，便于排障）。
    #[must_use]
    pub fn persistence_last_save_error(&self) -> Option<String> {
        self.lock_persistence().last_save_error.clone()
    }

    /// 显式刷盘（优雅停机 / 测试）：
    ///
    /// - `Ok(false)` = 未绑定文件账（纯内存，零 I/O，账本不动）；
    /// - `Ok(true)` = 已原子写一次；
    /// - `Err` = 坏档拒写 / 写失败（真因同时计入诊断，账本仍是内存真值）。
    pub fn persist(&self) -> Result<bool, String> {
        if self.store.is_none() {
            return Ok(false);
        }
        match self.persist_ledger() {
            Ok(()) => Ok(true),
            Err(error) => {
                let message = error.to_string();
                self.note_save_failure(&message);
                Err(message)
            }
        }
    }

    /// **连续**工具失败数（成功即清零；真实 `ToolOutcome` 回调维护）。
    ///
    /// 关怀势能场 distress 信号源（D-3）：0 = 无失败；数值越大越挫败。
    #[must_use]
    pub fn consecutive_tool_failures(&self) -> u64 {
        self.consecutive_tool_failures.load(Ordering::SeqCst)
    }

    /// 最近一次工具**成功**时刻（epoch 毫秒）；`None` = 尚无成功。
    ///
    /// 关怀势能场 fatigue / silence 信号源（D-3）。
    #[must_use]
    pub fn last_tool_success_ms(&self) -> Option<u64> {
        match self.last_tool_success_ms.load(Ordering::SeqCst) {
            0 => None,
            at => Some(at),
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
            self.dirty.store(true, Ordering::SeqCst);
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
                self.dirty.store(true, Ordering::SeqCst);
                // D-3 信号源: 连续失败计数 + 最近成功时刻（成功即清零连续失败）。
                if ok {
                    self.consecutive_tool_failures.store(0, Ordering::SeqCst);
                    self.last_tool_success_ms.store(at_ms, Ordering::SeqCst);
                } else {
                    self.consecutive_tool_failures
                        .fetch_add(1, Ordering::SeqCst);
                }
                let label = if ok { "ok" } else { "error" };
                self.set_last_action(format!("tool_result {name} {label}"));
                Ok(())
            }
            Err(LifecycleError::AlreadyRetired(_)) => {
                self.dirty.store(true, Ordering::SeqCst);
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
    ///
    /// 绑定文件账时，本方法是**唯一自动落盘点**：评估完成后若账本自上次成功
    /// 落盘以来确有变化（工具结果 / 淘汰 / 迁移 / 计数），原子写一次；无变化
    /// 或未绑定 = 不写。落盘失败不影响返回的迁移，也不打断回合（真因进诊断）。
    pub fn evaluate_at(&self, now_ms: u64) -> Vec<LifecycleTransition> {
        let transitions = self.lock_ledger().evaluate(now_ms);
        if !transitions.is_empty() {
            self.dirty.store(true, Ordering::SeqCst);
            self.transitions_total
                .fetch_add(transitions.len() as u64, Ordering::SeqCst);
            if let Some(last) = transitions.last() {
                self.set_last_action(describe_transition(last));
            }
            *self.lock_last_transitions() = transitions.clone();
        }
        self.persist_after_evaluate();
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
            consecutive_tool_failures: self.consecutive_tool_failures(),
            last_tool_success_ms: self.last_tool_success_ms(),
        }
    }

    fn note_eviction(&self, evicted: &LifecycleRecord) {
        self.dirty.store(true, Ordering::SeqCst);
        self.evicted_total.fetch_add(1, Ordering::SeqCst);
        self.set_last_action(format!(
            "evicted {} ({})",
            evicted.asset_id,
            evicted.stage.as_str()
        ));
    }

    /// 启动装载（仅构造期调用）：成功 = 账本 / 计数替换；失败 = 空账 + 诊断。
    fn restore_from_store(&self) {
        let Some(store) = &self.store else {
            return;
        };
        match store.load() {
            Ok(None) => {}
            Ok(Some(body)) => match body.ledger() {
                Ok(ledger) => {
                    *self.lock_ledger() = ledger;
                    self.transitions_total
                        .store(body.counters.transitions_total, Ordering::SeqCst);
                    self.evicted_total
                        .store(body.counters.evicted_total, Ordering::SeqCst);
                    self.rejected_after_retire_total
                        .store(body.counters.rejected_after_retire_total, Ordering::SeqCst);
                }
                Err(error) => self.note_load_error(error.to_string()),
            },
            Err(error) => self.note_load_error(error.to_string()),
        }
    }

    fn note_load_error(&self, error: String) {
        self.lock_persistence().load_error = Some(bounded(error));
    }

    /// `evaluate` 后的自动落盘：未绑定 / 无变化 = 不写；失败只记诊断。
    fn persist_after_evaluate(&self) {
        if self.store.is_none() || !self.dirty.load(Ordering::SeqCst) {
            return;
        }
        if let Err(error) = self.persist_ledger() {
            self.note_save_failure(&error.to_string());
        }
    }

    /// 一次完整的「整本账快照 → 原子写」。
    ///
    /// 启动装载失败（坏档）时**拒绝写入**：不覆盖原档，返回真因。
    /// dirty 在快照前清除、失败时恢复：快照期间发生的并发变更会把 dirty 重新
    /// 置真，宁可多写一次也不丢一次更新。
    fn persist_ledger(&self) -> Result<(), LifecycleStateStoreError> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        if let Some(reason) = self.lock_persistence().load_error.clone() {
            return Err(LifecycleStateStoreError::Refused(format!(
                "store load failed at startup; refusing to overwrite: {reason}"
            )));
        }
        self.dirty.store(false, Ordering::SeqCst);
        let body = {
            let ledger = self.lock_ledger();
            LifecycleStateBody::from_ledger(
                &ledger,
                LifecycleCounters {
                    transitions_total: self.transitions_total.load(Ordering::SeqCst),
                    evicted_total: self.evicted_total.load(Ordering::SeqCst),
                    rejected_after_retire_total: self
                        .rejected_after_retire_total
                        .load(Ordering::SeqCst),
                },
            )
        };
        store.save(&body).inspect_err(|_| {
            self.dirty.store(true, Ordering::SeqCst);
        })
    }

    fn note_save_failure(&self, error: &str) {
        let mut persistence = self.lock_persistence();
        persistence.save_failures = persistence.save_failures.saturating_add(1);
        persistence.last_save_error = Some(bounded(error.to_string()));
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

    fn lock_persistence(&self) -> MutexGuard<'_, PersistenceDiagnostics> {
        self.persistence
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

/// 资产生命周期观测模块（`AfterTurn` 真实钩子 + R1b-2b 总线订阅 `ToolOutcome`）。
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
impl BehaviorModule for BioLifecycleModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if !self.enabled {
            return Ok(ModuleOutcome::continue_());
        }
        // R1b-2b: 工具结果面已收敛到总线订阅（同一事件只推进一次）；
        // 本钩子只保留回合收口评估，`AfterToolResult` = 零动作。
        if hook == HookPoint::AfterTurn {
            self.state.evaluate();
        }
        Ok(ModuleOutcome::continue_())
    }
}

/// R1b-2b · 生命周期总线订阅者：把桥发布的 [`NervousSignal::ToolOutcome`] 写进账本。
///
/// 与模块共享同一 [`BioLifecycleState`]（同一 `Arc`、同一本账）；工具名解析已在
/// 桥内完成（结果名优先、回退调用名），本订阅者不再接触钩子上下文。
/// `ErrorObserved` 不是工具使用事实（不归属资产），明确忽略。
#[derive(Clone)]
pub struct BioLifecycleBusSubscriber {
    state: Arc<BioLifecycleState>,
}

impl BioLifecycleBusSubscriber {
    /// 绑定共享账本。
    #[must_use]
    pub fn new(state: Arc<BioLifecycleState>) -> Self {
        Self { state }
    }

    /// 共享账本（测试 / 观测同一真值）。
    #[must_use]
    pub fn state(&self) -> &Arc<BioLifecycleState> {
        &self.state
    }

    /// 处理一条信号；返回是否真的记了一次工具使用/失败。
    #[must_use]
    pub fn handle(&self, signal: &NervousSignal) -> bool {
        let NervousSignal::ToolOutcome { ok, tool, .. } = signal else {
            return false;
        };
        let Some(name) = tool.as_deref().filter(|name| !name.trim().is_empty()) else {
            return false;
        };
        self.state.observe_tool_outcome(name, *ok).is_ok()
    }

    /// 订阅总线上的 `ToolOutcome`；返回订阅句柄序号（关 = 本方法不被调用）。
    pub fn subscribe(&self, bus: &NervousBus) -> usize {
        let subscriber = self.clone();
        bus.subscribe(NervousSignalKind::ToolOutcome, move |signal| {
            subscriber.handle(signal);
        })
    }
}

/// 装配结果：共享生命周期状态 + 一个模块（`module()` 交给装配根注册）。
pub struct BioLifecycleWiring {
    state: Arc<BioLifecycleState>,
    module: Arc<dyn BehaviorModule>,
}

impl BioLifecycleWiring {
    /// 共享生命周期状态（观测口 / 后续显式治理动作同一真值）。
    #[must_use]
    pub fn state(&self) -> &Arc<BioLifecycleState> {
        &self.state
    }

    /// 待注册模块（`RuntimeBuilder::with_module` / `ProductionModules` 同口）。
    #[must_use]
    pub fn module(&self) -> Arc<dyn BehaviorModule> {
        Arc::clone(&self.module)
    }

    /// R1b-2b 总线订阅者（与模块共享同一账本；开关开才构造）。
    #[must_use]
    pub fn subscriber(&self) -> BioLifecycleBusSubscriber {
        BioLifecycleBusSubscriber::new(Arc::clone(&self.state))
    }

    /// 启动装载诊断（纯内存 / 装载成功 = `None`；坏档 = 真因）。
    #[must_use]
    pub fn persistence_load_diagnostic(&self) -> Option<String> {
        self.state.persistence_load_diagnostic()
    }
}

/// 开关值 → 模块（开关关 = `None`；供宿主自带 [`BioLifecycleState`] 时使用）。
#[must_use]
pub fn bio_lifecycle_module_from_env_value(
    raw: Option<&str>,
    state: Arc<BioLifecycleState>,
) -> Option<Arc<dyn BehaviorModule>> {
    bio_lifecycle_enabled_from_env_value(raw)
        .then(|| Arc::new(BioLifecycleModule::new(true, state)) as Arc<dyn BehaviorModule>)
}

/// 开关值 → (共享状态 + 模块 + 可选文件账目录)；开关关 = `None`
/// （懒加载：不构造状态、不读时钟、不碰目录）。
///
/// `registered_tool_names` 是装配根持有的**真实工具声明名**（例如
/// `ProductionModules::capabilities().iter().map(|tool| tool.declaration().name)`）：
/// 它们以 `Newborn` 入账，之后按真实派发结果记账。传空切片 = 只跟踪运行期
/// 首次出现的工具（动态发现场景），不编造种子。
///
/// `store_dir = None` = 纯内存（与 W3.5 逐字节一致，零文件 I/O）；
/// `Some(care_dir)` = 启动装载 `<care_dir>/lifecycle_state.json`，之后每回合
/// 收口 `evaluate` 后按需原子写（见 [`BioLifecycleState::evaluate_at`]）。
/// 坏档不 panic：状态保持空账、真因经 [`BioLifecycleWiring::persistence_load_diagnostic`]
/// 返回，且本进程拒写（不覆盖原档）。
#[must_use]
pub fn bio_lifecycle_wiring_from_env_value_with_store_dir(
    raw: Option<&str>,
    clock: LifecycleClock,
    registered_tool_names: &[String],
    store_dir: Option<&Path>,
) -> Option<BioLifecycleWiring> {
    if !bio_lifecycle_enabled_from_env_value(raw) {
        return None;
    }
    let state = match store_dir {
        Some(dir) => BioLifecycleState::new(clock).with_store_from_dir(dir),
        None => BioLifecycleState::new(clock),
    };
    state.seed_tool_names(registered_tool_names);
    let state = Arc::new(state);
    let module: Arc<dyn BehaviorModule> =
        Arc::new(BioLifecycleModule::new(true, Arc::clone(&state)));
    Some(BioLifecycleWiring { state, module })
}

/// 兼容入口：不绑定文件账（纯内存，行为与 W3.5 逐字节一致）。
///
/// 等价于 [`bio_lifecycle_wiring_from_env_value_with_store_dir`] 传
/// `store_dir = None`；生产接线请用带目录的新入口（`bio_production_wiring.rs`
/// 本批禁改，接线由 Lead 做）。
#[must_use]
pub fn bio_lifecycle_wiring_from_env_value(
    raw: Option<&str>,
    clock: LifecycleClock,
    registered_tool_names: &[String],
) -> Option<BioLifecycleWiring> {
    bio_lifecycle_wiring_from_env_value_with_store_dir(raw, clock, registered_tool_names, None)
}

/// 从装配根已解析的布尔开关装配（fail-closed 口径同
/// [`bio_lifecycle_wiring_from_env_value`]；R1b-1b：env 读取收敛到
/// `NervousSwitchboard`，本 helper 不读 env）。
///
/// `store_dir = None` = 纯内存；`Some(care_dir)` = 绑定文件账。开关关时目录
/// 一次都不碰（返回 `None`）。
#[must_use]
pub fn bio_lifecycle_wiring_from_enabled_with_store_dir(
    enabled: bool,
    clock: LifecycleClock,
    registered_tool_names: &[String],
    store_dir: Option<&Path>,
) -> Option<BioLifecycleWiring> {
    bio_lifecycle_wiring_from_env_value_with_store_dir(
        enabled.then_some("1"),
        clock,
        registered_tool_names,
        store_dir,
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

    #[test]
    fn bus_subscriber_writes_shared_ledger_and_ignores_non_tool_signals() {
        let state = state();
        state.seed_tool_names_at(&["filesystem".to_string()], NOW_MS);
        let subscriber = BioLifecycleBusSubscriber::new(Arc::clone(&state));
        let bus = NervousBus::new();
        assert_eq!(subscriber.subscribe(&bus), 1);

        // 成功/失败都是真实使用事实；ErrorObserved 不归属工具, 不碰账本。
        assert_eq!(
            bus.publish(&NervousSignal::tool_outcome(
                true,
                Some("filesystem"),
                false
            )),
            1
        );
        assert_eq!(
            state.ledger().get("filesystem").expect("asset").use_count,
            1
        );
        let before = state.observation();
        assert!(!subscriber.handle(&NervousSignal::error_observed("boom")));
        assert!(!subscriber.handle(&NervousSignal::tool_outcome(true, None, false)));
        assert!(!subscriber.handle(&NervousSignal::tool_outcome(true, Some("   "), false)));
        assert_eq!(state.observation(), before, "无名信号不改账本");
    }

    #[test]
    fn wiring_exposes_a_bus_subscriber_that_shares_the_module_ledger() {
        let wiring = bio_lifecycle_wiring_from_enabled_with_store_dir(
            true,
            frozen_clock(),
            &["filesystem".to_string()],
            None,
        )
        .expect("开关开");
        let subscriber = wiring.subscriber();
        let bus = NervousBus::new();
        assert_eq!(subscriber.subscribe(&bus), 1);
        assert_eq!(
            bus.publish(&NervousSignal::tool_outcome(
                true,
                Some("filesystem"),
                false
            )),
            1
        );
        assert_eq!(
            wiring
                .state()
                .ledger()
                .get("filesystem")
                .expect("asset")
                .use_count,
            1,
            "订阅者与模块共享同一账本"
        );
    }
}
