//! 检索维度自进化接线层 (RetrievalTuningWire) —— V4-c 的落地接线。
//!
//! **职责** (引擎本身保持纯确定性、无 IO):
//! - 持有 [`RetrievalTuningEngine`], 把**注入后命中回执** (检索命中注入后、
//!   后续检索轮的选中回执) 解析成使用证据 ([`TuningEvidence`]) 喂给引擎;
//!   本代码库无回合引用标记, 回执是「后续使用」的唯一真值面 (不编造信号);
//! - 引擎每产生一条 [`RetrievalTuningRecord`] → **追加写 `retrieval_tuning.jsonl`**
//!   (数据目录, 落位与 `tuning-log.jsonl` 同目录; 每行一个逐记录信封,
//!   版本戳随记录走) —— 这是「自进化账」(append-only) 的载体;
//! - 把引擎的生效值装进进程内 override, 检索排序使用点实时吃到调权
//!   (调权真生效, 不装学习)。
//!
//! **开关**: `APEIRETH_ENABLE_RETRIEVAL_TUNING=1` 才构造
//! ([`RetrievalTuningWire::from_env`]); 默认关 fail-closed。关 = 无回执、
//! 无证据、无调权、无账。
//!
//! **会话维度** (多会话串扰修复, 2026-11-04): 调权**学习**按**会话隔离** ⇒ ⇒
//! 每个会话一套「引擎 + 待决注入集」⇒ ⇒ A 会话调权学到的权重**不会**进 B
//! 会话的引擎 ⇒ ⇒ B 的检索不会吃到 A 学出来的权重。
//! ⇒ ⇒ 桶键口径 = 生效值落地的口径 (同一套键, 见
//! [`apeireth_memory::retrieval_tuning::effective_bucket_key`]) ⇒ ⇒ 不新造
//! 第二套分桶语义。
//!
//! **0 装诚实**:
//! - 持久化失败 fail-open (调整仍生效) 但计入 [`RetrievalTuningWire::log_failures`],
//!   不静默吞掉"账丢了";
//! - 无回执面 (召回路径未提供选中回执) = [`SignalSurface::Absent`] —— **显式
//!   不进化**, 不是"暂时没证据";
//! - 空召回轮不是机会轮 (检索什么都没命中 = 对已注入条目无证据), 证据保持
//!   待决, 绝不拿"没有回执"当负信号。
//!
//! **失败 → 学习桥** (2026-11-06 接线):
//! - **正证据侧**: 注入回执 → 证据 → 权重 (本文件 [`RetrievalTuningWire::record_injection_round`]);
//! - **负证据侧**: 失败 → 归因 (那一轮的注入回执) → 负证据 → 权重**退**
//!   ([`RetrievalTuningWire::record_failure_with_injection_attribution`] +
//!   [`RetrievalTuningWire::settle_failures`]);
//! - ⚠️ **没有归因的失败 = 零证据** (不许瞎归因: 归因缺失就**学不到**, 但**不污染**);
//! - ⚠️ **结算按会话隔离** (失败结算用 `observe_failures_since_in_session`,
//!   不用不带会话的旧入口) ⇒ ⇒ A 会话的失败**不会**去调 B 会话的权重。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use apeireth_core::stored_doc::{self, DocCompat};
use apeireth_memory::quality_drift::QualityDriftMonitor;
use apeireth_memory::reflexion::{
    FailureAttribution, FailureKind, FailureRecord, ReflexionError, ReflexionStore,
};
use apeireth_memory::reflexion_tuning::{observe_failures_since_in_session, ReflexionTuningCursor};
use apeireth_memory::retrieval_tuning::{
    RetrievalTuningEngine, RetrievalTuningRecord, SelectedAttribution, TuningEvidence,
};

/// 自进化账文件名 (数据目录, 与 `tuning-log.jsonl` 同目录落位)。
pub const RETRIEVAL_TUNING_LOG_FILE: &str = "retrieval_tuning.jsonl";

/// 自进化账逐记录信封身份 (与读取端的版本契约一致)。
pub const RETRIEVAL_TUNING_DOC_NAME: &str = "retrieval-tuning-record";
/// 自进化账记录格式版本。
pub const RETRIEVAL_TUNING_DOC_VERSION: u32 = 1;

/// 自进化账的逐记录版本契约 (信封)。
pub fn retrieval_tuning_doc_compat() -> DocCompat {
    DocCompat::exact(RETRIEVAL_TUNING_DOC_NAME, RETRIEVAL_TUNING_DOC_VERSION)
}

/// 检索调权开关 env (默认关, 仅 `1` 开)。
pub const RETRIEVAL_TUNING_ENABLE_ENV: &str = "APEIRETH_ENABLE_RETRIEVAL_TUNING";

/// 开关解析 (纯函数): 仅 `1` 开, 其余一律关 (默认关 fail-closed)。
pub fn retrieval_tuning_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 证据信号面状态 (0 装诚实)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSurface {
    /// 注入回执面在场: 已观察到有选中回执的检索轮。
    Present {
        /// 有选中回执的检索轮数。
        selective_rounds: u64,
    },
    /// 无注入回执面 (召回路径未提供选中回执): **显式不进化**。
    Absent,
}

impl SignalSurface {
    /// 如实说明 (无面 = 显式不进化, 不编造"被引用/被追问"信号)。
    pub fn note(self) -> &'static str {
        match self {
            Self::Present { .. } => "注入后命中回执面在场 (证据随回执裁决)",
            Self::Absent => "无注入回执面: 显式不进化 (不编造使用信号)",
        }
    }
}

/// 一个待决检索轮 (上个有选中回执的注入集, 等下一个机会轮裁决)。
#[derive(Debug, Clone)]
struct PendingRound {
    receipts: Vec<SelectedAttribution>,
}

/// **一个会话的调权学习状态** (按会话隔离 ⇒ 互不串扰)。
#[derive(Debug)]
struct SessionTuningState {
    /// 该会话的调权引擎 (层权重 + 条目显著性 + 证据账 + 漂移监控)。
    engine: RetrievalTuningEngine,
    /// 该会话的待决注入集 (上个有选中回执的注入集; **不跨会话** ⇒ ⇒
    /// A 会话的注入集不会被 B 会话的回执裁决)。
    pending: Option<PendingRound>,
    /// ⭐ **那一轮的注入回执** (每轮都更新; 空轮 = 空)。
    ///
    /// ⚠️ **为什么不能直接拿 `pending` 当失败归因**: `pending` 是
    /// 「**待结算**的注入集」, **空轮不更新它** ⇒ ⇒ 空轮之后它仍是
    /// **上一轮**的回执 ⇒ ⇒ 拿它当失败归因 = **拿过期回执瞎归因**。
    /// ⇒ ⇒ 这里另记一份「那一轮」的回执 (每轮都写, 空轮写空)。
    last_round_receipts: Vec<SelectedAttribution>,
    /// ⭐ 失败结算游标 (会话内增量 ⇒ **幂等**: 同一批失败只结算一次)。
    failure_cursor: ReflexionTuningCursor,
    /// ⭐ 本会话**拥有**的失败 seq (**老数据兜底**: 接线层记账)。
    ///
    /// ⚠️ **为什么还需要它** (2026-11-06 根治后仍保留):
    /// 新失败**真的落了** `session_key` (登记侧走
    /// `record_failure_with_attribution_and_session`) ⇒ ⇒ **重启后仍能结算**;
    /// 但**老数据** (会话键字段引入前写的失败) 落盘里**没有**会话键
    /// ⇒ ⇒ 归谁所有只能由**接线层记账**, 结算时通过会话作用域视图补上标签。
    owned_failures: BTreeSet<usize>,
    /// 最近更新序号 (诊断面: 谁的待决集是最近的)。
    updated: u64,
}

impl SessionTuningState {
    fn new() -> Self {
        Self {
            engine: new_tuning_engine(),
            pending: None,
            last_round_receipts: Vec::new(),
            failure_cursor: ReflexionTuningCursor::default(),
            owned_failures: BTreeSet::new(),
            updated: 0,
        }
    }
}

/// 构造**会话级**调权引擎 (基线默认权重 + 检索质量漂移监控)。
///
/// ⚠️ 参数口径: EMA α=0.05 (基线稳、迟钝) · |z|≥2.0 算偏离 · **连续 3 批**
/// 才告警 ⇒ ⇒ 单批波动**不告警** (防假告警)。
fn new_tuning_engine() -> RetrievalTuningEngine {
    // 记忆线 (2026-11-03): 给自进化引擎挂上**检索质量漂移监控**
    // ⇒ ⇒ 每批证据的命中率进监控器 ⇒ ⇒ 连续多批偏离才告警
    // ⇒ ⇒ **这就是 `QualityDriftMonitor` 的生产消费点** (此前全仓零消费者)。
    RetrievalTuningEngine::new().with_quality_drift(QualityDriftMonitor::new(
        &[apeireth_memory::retrieval_tuning::RETRIEVAL_HIT_RATE_METRIC],
        0.05,
        2.0,
        3,
    ))
}

/// ⭐ **进程级「当前调权接线层」弱引用槽**（失败面找学习面的唯一入口）。
///
/// # 为什么需要它（诚实说明）
/// 失败闭环模块（`cognitive.reflexion`）与检索调权接线层
/// （[`RetrievalTuningWire`]）是**两个独立模块**，生产装配层
/// （`canonical/production.rs`）只把接线层注入**记忆召回模块**
/// ⇒ ⇒ 失败闭环模块**拿不到**同一个接线层 ⇒ ⇒ 失败面与学习面**接不上**
/// （= 「失败 → 学习桥」断在最后一米）。
/// ⇒ ⇒ 这里给一个**进程级弱引用槽**：接线层构造时**自报家门**，
/// 失败面按需取用 ⇒ ⇒ 两侧**共用同一份**按会话分桶的引擎与注入回执
/// （不新造第二份状态 ⇒ 不串扰）。
///
/// ⚠️ **弱引用**：接线层被丢弃 ⇒ 槽自动失效 ⇒ 失败面退回「无归因」
/// （= 不瞎归因, 不污染）。
static ACTIVE_WIRE: OnceLock<Mutex<Option<Weak<RetrievalTuningWire>>>> = OnceLock::new();

/// 检索维度自进化接线层: 回执 → 证据 → 引擎调权 → 账落盘 + 生效值落地。
///
/// ⚠️ **会话维度** (多会话串扰修复, 2026-11-04): 调权**学习**按**会话隔离**
/// ⇒ ⇒ 每个会话一套「引擎 + 待决注入集」⇒ ⇒ A 会话调权学到的权重**不会**
/// 进 B 会话的引擎 ⇒ ⇒ B 的检索不会吃到 A 学出来的权重。
#[derive(Debug)]
pub struct RetrievalTuningWire {
    /// **按会话分桶**的调权学习状态 (引擎 + 待决注入集)。
    ///
    /// ⚠️ **桶键口径 = 生效值落地的口径** (同一套键 ⇒ 不新造第二套分桶语义):
    /// [`apeireth_memory::retrieval_tuning::effective_bucket_key`] ⇒ ⇒ 读侧
    /// (`effective_ranking_or_for_session`) 与写侧**同一个桶**。
    sessions: Mutex<BTreeMap<String, SessionTuningState>>,
    /// ⭐ **会话 id → 桶键** 映射 (记忆召回模块登记; 失败面按会话 id 找桶)。
    ///
    /// ⚠️ **为什么需要它**: 桶键**不一定**等于会话 id
    /// （画像分区路径: 桶键 = 分区键, 会话 id = 会话）⇒ ⇒ 两侧各算各的
    /// ⇒ ⇒ 键对不上 ⇒ ⇒ 失败面查不到「那一轮的注入回执」⇒ ⇒ **学不到**。
    /// ⇒ ⇒ 让**记忆召回模块**（唯一知道桶键口径的那一侧）登记映射
    /// ⇒ ⇒ 失败面**按会话 id 问桶** ⇒ ⇒ 桶键口径**只有一处**。
    session_buckets: Mutex<BTreeMap<String, String>>,
    /// 进程级「最近一次调权真值」 (诊断面; **学习**本身按会话隔离)。
    last_move: Mutex<Option<RetrievalTuningRecord>>,
    /// 进程级「最近一批漂移告警明细」 (诊断面)。
    last_alarms: Mutex<Vec<apeireth_memory::DriftAlarm>>,
    /// 自进化账的**进程级**序号 (账是进程级单文件 ⇒ 序号进程级单调)。
    ledger_seq: AtomicU64,
    /// 待决注入集的「最近更新」序号 (诊断面: 谁的待决集是最近的)。
    update_seq: AtomicU64,
    /// 自进化账 (`retrieval_tuning.jsonl`, append-only)。
    log_path: PathBuf,
    /// 账追加失败计数 (0 装诚实: 持久化失败不静默)。
    log_failures: AtomicU64,
    /// 检索轮计数 (全部轮)。
    rounds: AtomicU64,
    /// 机会轮计数 (有选中回执的检索轮; > 0 = 信号面在场)。
    selective_rounds: AtomicU64,
    /// **检索质量漂移告警累计数**（生产消费点：`QualityDriftMonitor` 在生产里
    /// 监视「注入的记忆下一轮是否再次命中」的命中率 ⇒ ⇒ 命中率持续下滑 ⇒ ⇒ 计数涨）。
    ///
    /// ⚠️ **私有** ⇒ 出口走 [`Self::quality_drift_alarms`]（那个方法已被生产消费）。
    quality_alarms: AtomicU64,
}

impl RetrievalTuningWire {
    /// 构造接线层 (引擎以默认评分权重为基线; 生效值在首次调权时落地)。
    ///
    /// ⚠️ **不发布**进程级句柄 (本构造器返回的不是 `Arc`) ⇒ ⇒ 要让失败面
    /// 找到它, 用 [`Self::from_env`] (生产路径) 或 [`Self::publish_as_active`]。
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            sessions: Mutex::new(BTreeMap::new()),
            session_buckets: Mutex::new(BTreeMap::new()),
            last_move: Mutex::new(None),
            last_alarms: Mutex::new(Vec::new()),
            ledger_seq: AtomicU64::new(0),
            update_seq: AtomicU64::new(0),
            log_path,
            log_failures: AtomicU64::new(0),
            rounds: AtomicU64::new(0),
            selective_rounds: AtomicU64::new(0),
            quality_alarms: AtomicU64::new(0),
        }
    }

    /// 从 env 构造: `APEIRETH_ENABLE_RETRIEVAL_TUNING=1` 才启用 (默认关 → None,
    /// fail-closed)。
    ///
    /// ⚠️ 启用时**自报家门**（发布为进程级当前接线层）⇒ ⇒ 失败闭环模块
    /// 能找到**同一个**接线层 ⇒ ⇒ 「失败 → 学习桥」在生产**接得上**。
    pub fn from_env(log_path: PathBuf) -> Option<Arc<Self>> {
        let raw = std::env::var(RETRIEVAL_TUNING_ENABLE_ENV).ok();
        if !retrieval_tuning_enabled_from_env_value(raw.as_deref()) {
            return None;
        }
        let wire = Arc::new(Self::new(log_path));
        Self::publish_as_active(&wire);
        Some(wire)
    }

    /// ⭐ **发布**本接线层为**进程级当前接线层**（弱引用 ⇒ 不 prolong 生命周期）。
    pub fn publish_as_active(wire: &Arc<Self>) {
        let slot = ACTIVE_WIRE.get_or_init(|| Mutex::new(None));
        let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Some(Arc::downgrade(wire));
    }

    /// ⭐ **进程级当前接线层**（没有 ⇒ `None` ⇒ 失败面退回「无归因」）。
    pub fn active() -> Option<Arc<Self>> {
        let slot = ACTIVE_WIRE.get_or_init(|| Mutex::new(None));
        let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.as_ref().and_then(Weak::upgrade)
    }

    /// 自进化账路径 (自述记忆节与调参面板按同一落位读取)。
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// 注入回执 (机会轮): 一轮召回选中注入的 `(id, 显著维度)` 集合。
    ///
    /// 调用即裁决上个机会轮的注入集: 再次被命中 = 正信号, 机会在场而未再
    /// 命中 = 负信号 (注入后命中回执的最简可用口径); 本集转为待决。
    ///
    /// ⚠️ **会话维度** (多会话串扰修复, 2026-11-04): 学习状态按**会话分桶**
    /// ⇒ ⇒ ① **引擎**取当前会话的桶 (不共用一个进程级引擎 ⇒ ⇒ A 学出来的
    /// 权重不会进 B 的引擎); ② **待决注入集**也取当前会话的桶 (A 的注入集
    /// 不会被 B 的回执裁决); ③ 生效值装进**同一个键**的生效值桶
    /// (显式键 ⇒ 不依赖作用域还在不在)。
    pub fn record_injection_round(&self, receipts: &[SelectedAttribution], at_epoch_ms: i64) {
        if receipts.is_empty() {
            self.record_empty_round(at_epoch_ms);
            return;
        }
        self.rounds.fetch_add(1, Ordering::Relaxed);
        self.selective_rounds.fetch_add(1, Ordering::Relaxed);
        // ⚠️ **会话键口径 = 生效值落地的口径** (同一套键 ⇒ 不新造第二套语义):
        // 有会话作用域 ⇒ 作用域的键; 没有 ⇒ 进程级默认桶键。
        let session_key = apeireth_memory::retrieval_tuning::effective_bucket_key();
        let updated = self.update_seq.fetch_add(1, Ordering::Relaxed);

        let (records, effective, batch_alarms) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let state = sessions
                .entry(session_key.clone())
                .or_insert_with(SessionTuningState::new);
            state.updated = updated;
            // ⭐ **那一轮的注入回执** (每轮都更新): 失败归因用它
            // ⇒ ⇒ 「这一轮注入了什么」有据可查, 不用过期回执瞎归因。
            state.last_round_receipts = receipts.to_vec();
            let previous = state.pending.replace(PendingRound {
                receipts: receipts.to_vec(),
            });
            let Some(previous) = previous else {
                // 首轮 (本会话还没有待决集) ⇒ 无证据可裁决 ⇒ 不调权。
                return;
            };
            let evidence = resolve_evidence(&previous.receipts, receipts, at_epoch_ms);
            let records = state.engine.observe_batch(&evidence);
            // 记忆线（2026-11-03）：取本批的漂移告警（有 ⇒ 计数涨 ⇒ 面板/运维可读）。
            let batch_alarms = state
                .engine
                .quality_drift()
                .map(|monitor| monitor.alarms().to_vec())
                .unwrap_or_default();
            let effective = state.engine.effective();
            (records, effective, batch_alarms)
        };
        if !records.is_empty() {
            // 装进**当前会话的桶** (显式键 ⇒ 与读侧同一个桶 ⇒ 调权真生效)。
            apeireth_memory::retrieval_tuning::install_effective_for_session(
                &session_key,
                effective,
            );
        }
        if !batch_alarms.is_empty() {
            self.quality_alarms
                .fetch_add(batch_alarms.len() as u64, Ordering::Relaxed);
        }
        {
            let mut last_alarms = self
                .last_alarms
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *last_alarms = batch_alarms;
        }
        if let Some(last) = records.last() {
            let mut last_move = self
                .last_move
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *last_move = Some(last.clone());
        }
        for record in &records {
            self.persist(record);
        }
    }

    /// 空召回轮 (无选中回执): 不是机会轮 —— 对已注入条目**无证据**, 待决证据
    /// 保持待决 (绝不拿"没有回执"当负信号)。
    ///
    /// ⚠️ **会话键口径**: 本方法**不裁决证据**, 但**要**把「**这一轮什么都
    /// 没注入**」记进**当前会话桶** (会话作用域内调用 ⇒ 键 = 作用域的键)
    /// ⇒ ⇒ 失败归因不会拿到**上一轮**的回执 (不瞎归因)。
    pub fn record_empty_round(&self, _at_epoch_ms: i64) {
        self.rounds.fetch_add(1, Ordering::Relaxed);
        let session_key = apeireth_memory::retrieval_tuning::effective_bucket_key();
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = sessions
            .entry(session_key)
            .or_insert_with(SessionTuningState::new);
        // ⚠️ **只清「那一轮回执」**, **不动 `pending`**: 待决证据的裁决语义不变。
        state.last_round_receipts.clear();
    }

    /// 证据信号面现况 (无回执面 = 显式不进化)。
    pub fn signal_surface(&self) -> SignalSurface {
        let selective_rounds = self.selective_rounds.load(Ordering::Relaxed);
        if selective_rounds > 0 {
            SignalSurface::Present { selective_rounds }
        } else {
            SignalSurface::Absent
        }
    }

    /// 进化次数 (真实调权移动数)。
    ///
    /// ⚠️ **进程级口径** (诊断面): 学习按会话隔离, 但**账**是进程级单文件
    /// ⇒ ⇒ 这里给**各会话移动数之和** (= 账面行数)。
    pub fn evolution_count(&self) -> u64 {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .map(|state| state.engine.evolution_count())
            .sum()
    }

    /// 最近一次调权真值 (何时/因何证据/何维度/前后权重)。
    ///
    /// ⚠️ **进程级口径** (诊断面): 跨会话的「最近一次」。
    pub fn last_move(&self) -> Option<RetrievalTuningRecord> {
        self.last_move
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 当前生效层权重 (调权后真值)。
    ///
    /// ⚠️ **会话维度**: 取**当前会话桶**里那个引擎的权重 (与生效值落地
    /// 同一个键 ⇒ 口径一致); 当前会话还没调过 ⇒ 回落基线默认值。
    pub fn effective_ranking(&self) -> apeireth_memory::MemoryRankingConfig {
        let session_key = apeireth_memory::retrieval_tuning::effective_bucket_key();
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&session_key)
            .map(|state| state.engine.ranking())
            .unwrap_or_default()
    }

    /// 账追加失败计数 (0 装诚实: 持久化失败不静默)。
    pub fn log_failures(&self) -> u64 {
        self.log_failures.load(Ordering::Relaxed)
    }

    /// **检索质量漂移告警累计数**（记忆线：生产里 `QualityDriftMonitor` 的出口）。
    ///
    /// ⇒ ⇒ **> 0** ⇒ 「注入的记忆下一轮不再命中」的命中率**连续多批偏离基线**
    /// ⇒ ⇒ 检索质量在漂移 ⇒ ⇒ 值得查（嵌入源？排序权重？索引？）。
    ///
    /// ⚠️ **两个返回值的口径不同**（别混）：
    /// - **计数 = 累计**（历史上**一共**报过多少次告警）⇒ 不回退；
    /// - **明细 = 当前批次**（`QualityDriftMonitor::alarms()` 只保留最后一次 `record` 的告警）
    ///   ⇒ ⇒ 偏离结束后**明细会变空**，但**计数仍在**。
    ///
    /// ⚠️ **为什么不是 `quality_alarm_count`**：那个名字曾作为**独立公开方法**存在
    /// ⇒ ⇒ 但**零消费者** ⇒ ⇒ 孤儿 API 棘轮会把它记成「新增孤儿」
    /// ⇒ ⇒ 改成**与告警明细一起返回**（一个出口 ⇒ 有消费者 ⇒ 不孤儿）。
    pub fn quality_drift_alarms(&self) -> (u64, Vec<apeireth_memory::DriftAlarm>) {
        let count = self.quality_alarms.load(Ordering::Relaxed);
        let alarms = self
            .last_alarms
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        (count, alarms)
    }

    /// 待决注入集 (诊断/测试用)。
    ///
    /// ⚠️ **会话维度**: 学习按会话隔离 ⇒ ⇒ 待决集也按会话分桶 ⇒ 这里给
    /// **当前会话桶**的待决集; 当前会话没有待决集时 ⇒ 给**最近更新**的那一个
    /// (跨会话诊断视图; 学习裁决本身永远只看本会话的桶)。
    ///
    /// ⚠️ **不要拿它当失败归因**: 它是「**待结算**的注入集」, **空轮不更新**
    /// ⇒ ⇒ 可能过期一轮 ⇒ ⇒ 归因请用 [`Self::record_failure_with_injection_attribution`]
    /// (它取的是「**那一轮**的注入回执」)。
    pub fn pending_receipts(&self) -> Vec<SelectedAttribution> {
        let session_key = apeireth_memory::retrieval_tuning::effective_bucket_key();
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(state) = sessions.get(&session_key) {
            if let Some(pending) = state.pending.as_ref() {
                return pending.receipts.clone();
            }
        }
        sessions
            .values()
            .max_by_key(|state| state.updated)
            .and_then(|state| state.pending.as_ref())
            .map(|pending| pending.receipts.clone())
            .unwrap_or_default()
    }

    /// ⭐ **登记**「会话 id ⇒ 桶键」映射 (记忆召回模块每轮登记一次)。
    ///
    /// ⚠️ **谁登记**: **记忆召回模块**（唯一知道桶键口径的那一侧:
    /// 桶键 = 分区键或会话 id, 由召回路径决定）⇒ ⇒ 失败面**按会话 id 问桶**
    /// ⇒ ⇒ 桶键口径**只有一处**（不各算各的 ⇒ 不对不上 ⇒ 学得到）。
    pub fn register_session_bucket(&self, session_id: &str, bucket_key: &str) {
        if session_id.trim().is_empty() || bucket_key.trim().is_empty() {
            return;
        }
        let mut buckets = self
            .session_buckets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        buckets.insert(session_id.to_string(), bucket_key.to_string());
    }

    /// ⭐ **按会话 id 问桶键** (失败面用; 没登记过 ⇒ `None` ⇒ 调用方自决)。
    pub fn bucket_key_for_session(&self, session_id: &str) -> Option<String> {
        let buckets = self
            .session_buckets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        buckets.get(session_id).cloned()
    }

    /// ⭐ **失败登记（带归因）**：失败 ⇒ 落进**真 store** + 归因 = **那一轮的注入回执**。
    ///
    /// # 为什么归因取「那一轮的注入回执」
    /// 失败意味着「**当时被判定为相关的那些记忆，没帮上忙**」
    /// ⇒ ⇒ 归因必须是**这一轮真的注入了的记忆**，而不是任何一轮的记忆。
    /// 接线层按会话记着「**那一轮**注入了什么」
    /// ([`SessionTuningState::last_round_receipts`]) ⇒ ⇒ 这里取它。
    ///
    /// # 口径
    /// * **归因 = 回执**（[`SelectedAttribution`] ⇒ [`FailureAttribution`]）
    ///   ⇒ ⇒ 转换**复用记忆 crate 已有的 `From` 实现**（不自造重复映射）;
    /// * ⚠️ **没有回执 ⇒ 零归因** ⇒ ⇒ 这条失败**学不到**（**不许瞎归因**）;
    /// * 归因**真的落进真 store**（生产 store 覆写了
    ///   `record_failure_with_attribution` ⇒ 归因**落盘**，不是只改返回值）;
    /// * ⭐ **会话键也真的落进真 store**（2026-11-06 根治）：
    ///   走 `record_failure_with_attribution_and_session` ⇒ ⇒ **会话键落盘**
    ///   ⇒ ⇒ **重启后**这条失败**仍能**被本会话结算（不再只靠进程内记账）;
    /// * 本会话**拥有**这条失败（记账 ⇒ 结算时按会话隔离）⇒ ⇒ **老数据兜底**。
    pub fn record_failure_with_injection_attribution(
        &self,
        store: &dyn ReflexionStore,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        at_epoch_ms: i64,
        session_key: &str,
    ) -> Result<FailureRecord, ReflexionError> {
        // ⭐ 归因 = **那一轮**的注入回执（严格本会话; 空轮 = 空 ⇒ 零归因）。
        let attributions: Vec<FailureAttribution> = self
            .last_round_receipts_for_session(session_key)
            .into_iter()
            // ⭐ **复用已有转换**（`reflexion.rs` 的 `From<SelectedAttribution>`）。
            .map(FailureAttribution::from)
            .collect();
        // ⭐ **会话键口径 = 传进来的这个桶键**（不另算一套）⇒ ⇒ 与
        // [`Self::settle_failures`] 用的是**同一个键** ⇒ ⇒ 结算匹配得上。
        let record = store.record_failure_with_attribution_and_session(
            kind,
            task_type,
            summary,
            at_epoch_ms,
            attributions,
            Some(session_key),
        )?;
        // ⚠️ **进程内记账保留**（兜底**老数据**: 老记录没有会话键 ⇒ ⇒ 靠它补标签）。
        self.claim_failure(session_key, record.seq);
        Ok(record)
    }

    /// ⭐ **失败结算**：本会话累计的失败 ⇒ **负证据** ⇒ 权重**退**。
    ///
    /// # 为什么用 `observe_failures_since_in_session`
    /// 生产调权引擎**按会话分桶** ⇒ ⇒ 不带会话的旧入口会让
    /// **A 会话的失败去调 B 会话的权重**（跨会话串扰）
    /// ⇒ ⇒ 这里显式指定会话（[`observe_failures_since_in_session`]）。
    ///
    /// # 会话隔离怎么落地（**真落盘 + 进程内兜底**，2026-11-06 根治）
    /// * **新数据**：登记侧走 `record_failure_with_attribution_and_session`
    ///   ⇒ ⇒ **会话键真的落进真 store** ⇒ ⇒ 结算时**直接按落盘的会话键**过滤
    ///   ⇒ ⇒ **重启后仍能结算**（不再只靠进程内状态）;
    /// * **老数据**（会话键字段引入前写的）：落盘里**没有**会话键
    ///   ⇒ ⇒ 靠**会话作用域视图**（[`SessionScopedReflexionView`]）+ **进程内记账**
    ///   给**本会话拥有**的失败补上会话标签 ⇒ ⇒ **老数据仍能结算**（兜底）。
    ///
    /// # 口径
    /// * **幂等**：游标只增 ⇒ ⇒ 同一批失败**只结算一次**（不重复喂 ⇒ 不抖）;
    /// * **批量**：本会话累计的失败**一批**喂进去 ⇒ ⇒ `observe_batch`
    ///   每个目标**只动一步**（逐条喂会一步一挪 ⇒ 快速撞走廊地板）;
    /// * **落账 + 生效**：调权记录进自进化账，生效值装进**本会话**的桶
    ///   ⇒ ⇒ 下一轮检索吃到**退过的**权重。
    pub fn settle_failures(
        &self,
        store: &dyn ReflexionStore,
        session_key: &str,
    ) -> Result<usize, ReflexionError> {
        let (count, records, effective, alarms) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let state = sessions
                .entry(session_key.to_string())
                .or_insert_with(SessionTuningState::new);
            let before = state.engine.evolution_count() as usize;
            let view = SessionScopedReflexionView {
                inner: store,
                session_key,
                owned: &state.owned_failures,
            };
            // ⚠️⭐ **必须用 `_in_session` 版**（会话隔离 = 安全边界）。
            let count = observe_failures_since_in_session(
                &view,
                &mut state.engine,
                &mut state.failure_cursor,
                Some(session_key),
            )?;
            // 游标已越过 ⇒ 已结算的失败不必再记账（记账集合只留未结算的）。
            state
                .owned_failures
                .retain(|seq| *seq > state.failure_cursor.last_seq);
            let records: Vec<RetrievalTuningRecord> = state.engine.history()[before..].to_vec();
            let effective = state.engine.effective();
            let alarms = state
                .engine
                .quality_drift()
                .map(|monitor| monitor.alarms().to_vec())
                .unwrap_or_default();
            (count, records, effective, alarms)
        };
        if !records.is_empty() {
            // 装进**本会话**的桶（与读侧同一个键 ⇒ 调权真生效）。
            apeireth_memory::retrieval_tuning::install_effective_for_session(
                session_key,
                effective,
            );
        }
        if !alarms.is_empty() {
            self.quality_alarms
                .fetch_add(alarms.len() as u64, Ordering::Relaxed);
        }
        {
            let mut last_alarms = self
                .last_alarms
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *last_alarms = alarms;
        }
        if let Some(last) = records.last() {
            let mut last_move = self
                .last_move
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *last_move = Some(last.clone());
        }
        for record in &records {
            self.persist(record);
        }
        Ok(count)
    }

    /// **那一轮**的注入回执 (严格本会话; 本会话没有 ⇒ **空**)。
    ///
    /// ⚠️ **不跨会话回落**（与诊断面 [`Self::pending_receipts`] 不同）
    /// ⇒ ⇒ 归因绝不串到别的会话的记忆上。
    fn last_round_receipts_for_session(&self, session_key: &str) -> Vec<SelectedAttribution> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sessions
            .get(session_key)
            .map(|state| state.last_round_receipts.clone())
            .unwrap_or_default()
    }

    /// 记账: 这条失败**归本会话**所有（真 store 不记会话标签 ⇒ 接线层记）。
    fn claim_failure(&self, session_key: &str, seq: usize) {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = sessions
            .entry(session_key.to_string())
            .or_insert_with(SessionTuningState::new);
        state.owned_failures.insert(seq);
    }

    /// 账追加 (JSONL 追加, 每行一个逐记录信封; append-only)。
    ///
    /// ⚠️ **序号取进程级**: 学习按会话隔离 ⇒ ⇒ 各会话引擎的 `seq` 是**会话内**
    /// 序号; 但**账是进程级单文件** ⇒ ⇒ 落账时按追加顺序重编进程级序号
    /// ⇒ ⇒ 账面 `seq` 仍单调递增 (跨会话不重复)。
    fn persist(&self, record: &RetrievalTuningRecord) {
        let mut record = record.clone();
        record.seq = self.ledger_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let doc = stored_doc::StoredDoc::new(
            RETRIEVAL_TUNING_DOC_NAME,
            RETRIEVAL_TUNING_DOC_VERSION,
            vec![RETRIEVAL_TUNING_DOC_VERSION],
            record,
        );
        let Ok(line) = serde_json::to_string(&doc) else {
            self.log_failures.fetch_add(1, Ordering::Relaxed);
            return;
        };
        use std::io::Write;
        let appended = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .and_then(|mut file| writeln!(file, "{line}"));
        if appended.is_err() {
            self.log_failures.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// ⭐ **会话作用域的 store 视图**：真 store 的**读穿包装**（不新建 store, 不造数据）。
///
/// # 为什么需要它（**兜底**，不是唯一手段）
/// **老数据**（会话键字段引入前写的失败）落盘里**没有** `session_key`
/// ⇒ ⇒ 会话隔离的判据（`observe_failures_since_in_session`）会把它们**跳过**
/// ⇒ ⇒ 这里在**读**的时候给**本会话拥有**的失败补上会话标签
/// ⇒ ⇒ **老数据仍能结算**（进程内记账兜底）。
///
/// ⚠️ **新数据走真落盘**（2026-11-06 根治）：登记侧写 `session_key`
/// ⇒ ⇒ 视图**原样读回**（真值为准）⇒ ⇒ **重启后仍能结算**。
///
/// # 口径
/// * **读穿**：所有数据来自**真 store**（生产同款文件版），**不造数据**;
/// * **只补标签**：别的会话的失败**原样**（`session_key` 仍是 `None`）
///   ⇒ ⇒ 会话模式下被跳过 ⇒ ⇒ **不串扰**;
/// * **写操作照旧透传**给真 store（视图本身不缓存任何状态）。
struct SessionScopedReflexionView<'a> {
    /// 真 store（生产同款）。
    inner: &'a dyn ReflexionStore,
    /// 本视图的会话键。
    session_key: &'a str,
    /// 本会话**拥有**的失败 seq（接线层记账）。
    owned: &'a BTreeSet<usize>,
}

impl ReflexionStore for SessionScopedReflexionView<'_> {
    fn record_failure(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
    ) -> Result<FailureRecord, ReflexionError> {
        self.inner
            .record_failure(kind, task_type, summary, timestamp_epoch_ms)
    }

    fn record_failure_with_attribution(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
        attributions: Vec<FailureAttribution>,
    ) -> Result<FailureRecord, ReflexionError> {
        self.inner.record_failure_with_attribution(
            kind,
            task_type,
            summary,
            timestamp_epoch_ms,
            attributions,
        )
    }

    /// ⚠️ **写操作照旧透传**给真 store（视图**不缓存**任何状态 ⇒ 不造数据）。
    fn record_failure_with_attribution_and_session(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
        attributions: Vec<FailureAttribution>,
        session_key: Option<&str>,
    ) -> Result<FailureRecord, ReflexionError> {
        self.inner.record_failure_with_attribution_and_session(
            kind,
            task_type,
            summary,
            timestamp_epoch_ms,
            attributions,
            session_key,
        )
    }

    fn process_unreflected(
        &self,
        critic: &dyn apeireth_memory::reflexion::Critic,
        timestamp_epoch_ms: i64,
    ) -> Result<usize, ReflexionError> {
        self.inner.process_unreflected(critic, timestamp_epoch_ms)
    }

    /// ⭐ **读穿 + 补会话标签**（本会话拥有的失败 ⇒ 标上会话键）。
    ///
    /// ⚠️ **只补「没有会话键」的**（2026-11-06 根治）：
    /// * **新数据**：真 store **已经落了** `session_key` ⇒ ⇒ **原样读回**（真值为准）;
    /// * **老数据**：本字段引入前写的失败**没有**会话键 ⇒ ⇒ 靠**进程内记账**补
    ///   ⇒ ⇒ **老数据仍能结算**（兜底，不是重复）。
    fn list_failures(&self) -> Result<Vec<FailureRecord>, ReflexionError> {
        let mut records = self.inner.list_failures()?;
        for record in &mut records {
            if record.session_key.is_none() && self.owned.contains(&record.seq) {
                record.session_key = Some(self.session_key.to_string());
            }
        }
        Ok(records)
    }

    fn list_reflections(
        &self,
    ) -> Result<Vec<apeireth_memory::reflexion::ReflectionText>, ReflexionError> {
        self.inner.list_reflections()
    }

    fn retry_injection(
        &self,
        task_type: &str,
        budget_chars: usize,
    ) -> Result<Option<String>, ReflexionError> {
        self.inner.retry_injection(task_type, budget_chars)
    }
}

/// 证据裁决 (注入后命中回执): 上个机会轮的注入条目, 在本机会轮再命中 = 正
/// 信号, 未再命中 = 负信号 (机会在场)。
fn resolve_evidence(
    previous: &[SelectedAttribution],
    current: &[SelectedAttribution],
    at_epoch_ms: i64,
) -> Vec<TuningEvidence> {
    previous
        .iter()
        .map(|receipt| {
            if current.iter().any(|hit| hit.id == receipt.id) {
                TuningEvidence::positive(&receipt.id, receipt.dominant_dimension, at_epoch_ms)
            } else {
                TuningEvidence::negative(&receipt.id, receipt.dominant_dimension, at_epoch_ms)
            }
        })
        .collect()
}

/// 自进化账读回结果 (自述记忆节真值源: 进化次数 / 最近调权真值)。
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalTuningLedgerSummary {
    /// 账面记录 (文件序, 新 → 旧即追加序)。
    pub records: Vec<RetrievalTuningRecord>,
    /// 被跳过的坏行数 (坏行跳过并计数, 不静默)。
    pub skipped: usize,
}

impl RetrievalTuningLedgerSummary {
    /// 进化次数 (账面真实移动行数)。
    pub fn evolution_count(&self) -> u64 {
        self.records.len() as u64
    }

    /// 最近一次调权真值。
    pub fn last_move(&self) -> Option<&RetrievalTuningRecord> {
        self.records.last()
    }
}

/// 逐行读回自进化账 (append-only 账面真值; 缺档 = 空账如实, 非错误)。
pub fn read_retrieval_tuning_log(path: &Path) -> Result<RetrievalTuningLedgerSummary, String> {
    let scan = stored_doc::scan_records_compat::<RetrievalTuningRecord>(
        path,
        &retrieval_tuning_doc_compat(),
    )
    .map_err(|error| format!("retrieval tuning log read failed: {error}"))?;
    Ok(RetrievalTuningLedgerSummary {
        records: scan
            .records
            .into_iter()
            .map(|record| record.into_body())
            .collect(),
        skipped: scan.skipped,
    })
}

/// 自述记忆节真值映射 (纯函数): 自进化账摘要 → 记忆节「进化次数 / 最近调权」。
///
/// 进化次数 = 账面真实移动行数 (空账 = 0 如实); 最近调权 `None` = 尚未调权
/// (显式 null, 不编移动)。
pub fn memory_ledger_tuning_fields(
    summary: &RetrievalTuningLedgerSummary,
) -> (
    Option<u64>,
    Option<apeireth_tools_canonical::RetrievalTuningMove>,
) {
    let last = summary
        .last_move()
        .map(|record| apeireth_tools_canonical::RetrievalTuningMove {
            target: record.target.clone(),
            signal: record.signal.clone(),
            previous: record.previous,
            next: record.next,
            at_epoch_ms: record.at_epoch_ms,
            evidence: record.evidence.clone(),
        });
    (Some(summary.evolution_count()), last)
}

/// 自进化账落位: 与 `tuning-log.jsonl` 同目录 (session db 同目录口径)。
pub fn retrieval_tuning_log_path() -> PathBuf {
    retrieval_tuning_log_path_from_session_db(
        std::env::var("APEIRETH_SESSION_DB")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    )
}

/// [`retrieval_tuning_log_path`] 的纯函数内核 (可测, 不读 env)。
pub fn retrieval_tuning_log_path_from_session_db(session_db: Option<String>) -> PathBuf {
    let session_db = session_db.unwrap_or_else(|| ".apeireth/sessions.sqlite3".to_string());
    let session_path = Path::new(&session_db);
    let dir = session_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    dir.join(RETRIEVAL_TUNING_LOG_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::retrieval_tuning::RetrievalDimension;

    /// 生效值落地是进程级全局槽 + env 读取是进程级状态: 触碰两者的用例串行。
    static WIRE_TEST_GUARD: Mutex<()> = Mutex::new(());

    fn lock_wire() -> std::sync::MutexGuard<'static, ()> {
        WIRE_TEST_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn temp_log(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-retrieval-tuning-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(RETRIEVAL_TUNING_LOG_FILE)
    }

    fn receipt(id: &str, dimension: Option<RetrievalDimension>) -> SelectedAttribution {
        SelectedAttribution::new(id, dimension)
    }

    /// **记忆线**：漂移监控的告警出口真的有值（且**没漂移时是 0**）。
    ///
    /// ⇒ ⇒ 钉住两件事：
    /// ① **没漂移 ⇒ 告警数 0**（不误报）；
    /// ② **命中率持续下滑 ⇒ 告警数涨**（真能报出来，不是摆设）。
    #[test]
    fn quality_drift_alarms_report_zero_until_the_hit_rate_keeps_dropping() {
        let _guard = lock_wire();
        let wire = RetrievalTuningWire::new(temp_log("quality-drift"));
        assert_eq!(wire.quality_drift_alarms().0, 0, "刚起来不该有告警");

        // ⚠️ **漂移 = 命中率从高跌到低**（不是「一直低」）
        // ⇒ ⇒ 「一直低」是**稳定差** ⇒ EMA 基线也跟着低 ⇒ z≈0 ⇒ **不告警**（这是对的）。
        // ⇒ ⇒ 先几轮**全命中**（命中率 1.0，基线抬到 1.0），
        // ⇒ ⇒ 再几轮**全未命中**（命中率 0.0 ⇒ 相对基线偏离巨大 ⇒ ⇒ 连续 3 批 ⇒ 告警）。
        //
        // ⚠️ **每轮注入 4 条** ⇒ ⇒ 每批 4 条证据 ⇒ ⇒ 满足「样本至少 3 条」的下限。
        let ids: Vec<String> = (0..4).map(|index| format!("m{index}")).collect();
        // 前 5 轮：注入的 4 条下一轮**全部再命中** ⇒ ⇒ 命中率 1.0
        for round in 0..5 {
            let receipts: Vec<SelectedAttribution> =
                ids.iter().map(|id| receipt(id, None)).collect();
            wire.record_injection_round(&receipts, 1_000 + i64::from(round) * 1_000);
            wire.record_injection_round(&receipts, 1_500 + i64::from(round) * 1_000);
        }
        // 后 6 轮：注入的 4 条下一轮**一条都不再命中** ⇒ ⇒ 命中率 0.0 ⇒ ⇒ 持续偏离
        //
        // ⚠️ **下一轮必须「非空」** ⇒ ⇒ 空轮走 `record_empty_round`
        // ⇒ ⇒ **不是机会轮 ⇒ 不裁决证据**（绝不拿「没有回执」当负信号）
        // ⇒ ⇒ 所以要注入**另一批**条目 ⇒ ⇒ 上一批「机会在场而未再命中」⇒ ⇒ 负证据成立。
        let other: Vec<String> = (0..4).map(|index| format!("other{index}")).collect();
        for round in 0..6 {
            let receipts: Vec<SelectedAttribution> =
                ids.iter().map(|id| receipt(id, None)).collect();
            let next: Vec<SelectedAttribution> = other.iter().map(|id| receipt(id, None)).collect();
            wire.record_injection_round(&receipts, 100_000 + i64::from(round) * 1_000);
            wire.record_injection_round(&next, 100_500 + i64::from(round) * 1_000);
        }
        let (count, _alarms) = wire.quality_drift_alarms();
        assert!(
            count > 0,
            "命中率从 1.0 跌到 0.0 ⇒ 连续 3 批偏离 ⇒ 该报出漂移告警（实测 {count} 条）"
        );
    }

    #[test]
    fn the_switch_is_fail_closed_except_the_exact_opt_in() {
        assert!(!retrieval_tuning_enabled_from_env_value(None), "默认关");
        for raw in ["0", "true", "yes", "on", "", " 1x"] {
            assert!(
                !retrieval_tuning_enabled_from_env_value(Some(raw)),
                "{raw:?} 不许开 (fail-closed)"
            );
        }
        assert!(retrieval_tuning_enabled_from_env_value(Some("1")));
        assert!(retrieval_tuning_enabled_from_env_value(Some(" 1 ")));

        let _guard = lock_wire();
        let log = temp_log("switch");
        std::env::set_var(RETRIEVAL_TUNING_ENABLE_ENV, "0");
        assert!(RetrievalTuningWire::from_env(log.clone()).is_none());
        std::env::set_var(RETRIEVAL_TUNING_ENABLE_ENV, "1");
        assert!(RetrievalTuningWire::from_env(log.clone()).is_some());
        std::env::remove_var(RETRIEVAL_TUNING_ENABLE_ENV);
        assert!(
            RetrievalTuningWire::from_env(log.clone()).is_none(),
            "未设 = 关"
        );
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 正负信号判定 (有信号面): 下个机会轮再命中 = 正, 机会在场未再命中 = 负。
    #[test]
    fn receipts_resolve_into_positive_and_negative_evidence() {
        let _guard = lock_wire();
        let log = temp_log("resolve");
        let wire = RetrievalTuningWire::new(log.clone());

        wire.record_injection_round(
            &[
                receipt("mem-a", Some(RetrievalDimension::Semantic)),
                receipt("mem-b", Some(RetrievalDimension::Lexical)),
            ],
            1_000,
        );
        assert_eq!(wire.evolution_count(), 0, "首轮回执待决, 无证据不调权");
        assert_eq!(
            wire.signal_surface(),
            SignalSurface::Present {
                selective_rounds: 1
            }
        );

        wire.record_injection_round(
            &[
                receipt("mem-a", Some(RetrievalDimension::Semantic)),
                receipt("mem-c", None),
            ],
            2_000,
        );
        // mem-a 再命中 → 正 (条目 + semantic); mem-b 未再命中 → 负 (条目 + lexical)。
        assert_eq!(wire.evolution_count(), 4, "{:?}", wire.last_move());
        assert_eq!(wire.last_move().unwrap().at_epoch_ms, 2_000);
        assert!((wire.effective_ranking().semantic_weight - 0.26).abs() < 1e-9);
        assert!((wire.effective_ranking().lexical_weight - 0.19).abs() < 1e-9);
        let pending = wire.pending_receipts();
        assert_eq!(
            pending.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["mem-a", "mem-c"],
            "本机会轮注入集转待决"
        );
        assert_eq!(wire.log_failures(), 0);
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// **两个会话的调权学习互不影响** (多会话串扰回归)。
    ///
    /// 旧实现: 接线层持有**单个进程级引擎** ⇒ ⇒ A 会话调权学到的权重会被
    /// B 会话的引擎**继承** ⇒ ⇒ B 的检索吃到 A 学出来的权重 (串扰); 待决
    /// 注入集也是进程级单槽 ⇒ ⇒ A 的注入集会被 B 的回执裁决 (证据跨会话)。
    /// ⇒ ⇒ 现在: **引擎 + 待决注入集**都按**会话分桶** (与生效值落地同一套键)。
    #[test]
    fn two_sessions_do_not_share_their_tuning_learning() {
        let _guard = lock_wire();
        apeireth_memory::retrieval_tuning::clear_effective();
        let log = temp_log("session-isolation");
        let wire = RetrievalTuningWire::new(log.clone());

        // A 会话: 注入 mem-a (semantic) ⇒ 下一轮**不再命中** ⇒ 负证据 ⇒ 层权重下调一步。
        {
            let _scope = apeireth_memory::retrieval_tuning::enter_session_scope("session-a");
            wire.record_injection_round(
                &[receipt("mem-a", Some(RetrievalDimension::Semantic))],
                1_000,
            );
            wire.record_injection_round(&[receipt("mem-other", None)], 2_000);
        }
        // B 会话: 注入 mem-b (semantic) ⇒ 下一轮**再命中** ⇒ 正证据 ⇒ 层权重上调一步。
        {
            let _scope = apeireth_memory::retrieval_tuning::enter_session_scope("session-b");
            wire.record_injection_round(
                &[receipt("mem-b", Some(RetrievalDimension::Semantic))],
                1_000,
            );
            wire.record_injection_round(
                &[receipt("mem-b", Some(RetrievalDimension::Semantic))],
                2_000,
            );
        }

        let fallback = apeireth_memory::MemoryRankingConfig::default();
        let ranking_a = apeireth_memory::retrieval_tuning::effective_ranking_or_for_session(
            "session-a",
            fallback,
        );
        let ranking_b = apeireth_memory::retrieval_tuning::effective_ranking_or_for_session(
            "session-b",
            fallback,
        );
        let baseline = fallback.semantic_weight;
        let step = apeireth_memory::retrieval_tuning::DIMENSION_STEP;
        assert!(
            (ranking_a.semantic_weight - (baseline - step)).abs() < 1e-9,
            "A: 负证据 ⇒ 层权重下调一步: {:?}",
            ranking_a.semantic_weight
        );
        assert!(
            (ranking_b.semantic_weight - (baseline + step)).abs() < 1e-9,
            "B: 正证据 ⇒ 层权重上调一步 (不许继承 A 的下调): {:?}",
            ranking_b.semantic_weight
        );
        assert_ne!(
            ranking_a, ranking_b,
            "两个会话的检索权重必须互不影响 (不许串扰)"
        );

        // 条目显著性同样按会话隔离。
        let entry_step = apeireth_memory::retrieval_tuning::ENTRY_STEP;
        assert!(
            (apeireth_memory::retrieval_tuning::entry_salience_for_session("session-a", "mem-a")
                + entry_step)
                .abs()
                < 1e-12,
            "A: mem-a 负证据 ⇒ 显著性下调一步"
        );
        assert!(
            (apeireth_memory::retrieval_tuning::entry_salience_for_session("session-b", "mem-b")
                - entry_step)
                .abs()
                < 1e-12,
            "B: mem-b 正证据 ⇒ 显著性上调一步"
        );
        assert_eq!(
            apeireth_memory::retrieval_tuning::entry_salience_for_session("session-b", "mem-a"),
            0.0,
            "B 不得吃到 A 的条目显著性"
        );

        // 账是**进程级单文件** ⇒ 两会话的记录都在账里; 序号进程级单调 (跨会话不重复)。
        assert_eq!(wire.evolution_count(), 4, "两会话各 2 条移动 (条目 + 维度)");
        let summary = read_retrieval_tuning_log(&log).expect("读回");
        assert_eq!(summary.evolution_count(), 4);
        assert_eq!(summary.skipped, 0);
        for (index, record) in summary.records.iter().enumerate() {
            assert_eq!(
                record.seq,
                index as u64 + 1,
                "seq 进程级单调 (跨会话不重复)"
            );
        }
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 无信号面 = 显式不进化: 不调权、不落账、状态照实说。
    #[test]
    fn without_a_signal_surface_the_wire_reports_absence_and_never_evolves() {
        let _guard = lock_wire();
        let log = temp_log("no-surface");
        let wire = RetrievalTuningWire::new(log.clone());

        for at in [1_000, 2_000, 3_000] {
            wire.record_empty_round(at);
        }
        assert_eq!(wire.signal_surface(), SignalSurface::Absent);
        assert_eq!(wire.evolution_count(), 0);
        assert!(wire.last_move().is_none());
        assert!(!log.exists(), "无证据 = 无账 (不造账)");
        assert!(SignalSurface::Absent.note().contains("显式不进化"));
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 空召回轮不是机会轮: 待决证据保持待决, 直到下个有选中回执的轮。
    #[test]
    fn empty_rounds_leave_evidence_pending_until_the_next_selective_round() {
        let _guard = lock_wire();
        let log = temp_log("pending");
        let wire = RetrievalTuningWire::new(log.clone());

        wire.record_injection_round(&[receipt("mem-d", Some(RetrievalDimension::Graph))], 1_000);
        wire.record_empty_round(2_000);
        wire.record_empty_round(3_000);
        assert_eq!(wire.evolution_count(), 0, "空轮无证据, 不裁决");

        wire.record_injection_round(&[receipt("mem-e", None)], 4_000);
        assert_eq!(wire.evolution_count(), 2, "机会轮到来才裁决 (mem-d 负)");
        assert!(wire.effective_ranking().graph_weight < 0.05);
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 自进化账 append-only: 只追加不改写, 前行字节不变, seq 单调前后值接龙。
    #[test]
    fn the_tuning_ledger_is_append_only_and_traceable() {
        let _guard = lock_wire();
        let log = temp_log("ledger");
        let wire = RetrievalTuningWire::new(log.clone());

        // 机会轮 2 裁决轮 1 (mem-f 负 → 2 行), 机会轮 3 裁决轮 2 (mem-g 负 → 2 行)。
        wire.record_injection_round(
            &[receipt("mem-f", Some(RetrievalDimension::Recency))],
            1_000,
        );
        wire.record_injection_round(
            &[receipt("mem-g", Some(RetrievalDimension::Recency))],
            2_000,
        );
        let after_two = std::fs::read_to_string(&log).expect("账已落");
        let lines_after_two: Vec<&str> = after_two.lines().collect();
        assert_eq!(lines_after_two.len(), 2, "{after_two}");

        wire.record_injection_round(
            &[receipt("mem-h", Some(RetrievalDimension::Recency))],
            3_000,
        );
        let after_three = std::fs::read_to_string(&log).expect("账已落");
        let lines_after_three: Vec<&str> = after_three.lines().collect();
        assert_eq!(lines_after_three.len(), 4);
        assert_eq!(
            &lines_after_three[..2],
            &lines_after_two[..],
            "append-only: 前行字节不改写"
        );

        let summary = read_retrieval_tuning_log(&log).expect("读回");
        assert_eq!(summary.evolution_count(), 4);
        assert_eq!(summary.skipped, 0);
        for (index, record) in summary.records.iter().enumerate() {
            assert_eq!(record.seq, index as u64 + 1, "seq 单调递增");
            assert!(
                (record.next - record.previous).abs() <= 0.01 + 1e-12,
                "步长有界"
            );
            if record.target.starts_with("entry:") {
                assert!(
                    record.next.abs() <= 0.05 + 1e-12,
                    "条目显著性走廊不越界: {}",
                    record.target
                );
            } else {
                assert!(
                    (record.next - 0.10).abs() <= 0.05 + 1e-12,
                    "维度走廊 (基线 ±0.05) 不越界: {}",
                    record.target
                );
            }
        }
        let recency: Vec<_> = summary
            .records
            .iter()
            .filter(|record| record.target == "dimension:recency")
            .collect();
        for pair in recency.windows(2) {
            assert!(
                (pair[0].next - pair[1].previous).abs() < 1e-12,
                "前后值接龙可回溯"
            );
        }
        assert!(summary
            .last_move()
            .unwrap()
            .evidence
            .contains("注入后命中回执"));
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 账读回: 缺档 = 空账如实 (非错误); 坏行跳过并计数。
    #[test]
    fn ledger_reads_back_empty_when_absent_and_counts_bad_lines() {
        let _guard = lock_wire();
        let log = temp_log("scan");
        let summary = read_retrieval_tuning_log(&log).expect("缺档 = 空账");
        assert_eq!(summary.evolution_count(), 0);
        assert!(summary.last_move().is_none());

        std::fs::write(&log, "{ not json\n").unwrap();
        let summary = read_retrieval_tuning_log(&log).expect("坏行不致命");
        assert_eq!(summary.evolution_count(), 0);
        assert_eq!(summary.skipped, 1, "坏行跳过并计数 (不静默)");
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    /// 自述记忆节真值映射: 进化次数 / 最近调权取自账面真值 (与账同源)。
    #[test]
    fn memory_ledger_tuning_fields_map_the_ledger_truth() {
        let _guard = lock_wire();
        let log = temp_log("mapping");
        let wire = RetrievalTuningWire::new(log.clone());
        // 空账: 进化 0 + 最近调权显式 None (不编移动)。
        let summary = read_retrieval_tuning_log(&log).expect("读回");
        let (count, last) = memory_ledger_tuning_fields(&summary);
        assert_eq!(count, Some(0));
        assert!(last.is_none());

        wire.record_injection_round(
            &[receipt("mem-x", Some(RetrievalDimension::Novelty))],
            1_000,
        );
        wire.record_injection_round(&[receipt("mem-y", None)], 2_000);
        let summary = read_retrieval_tuning_log(&log).expect("读回");
        let (count, last) = memory_ledger_tuning_fields(&summary);
        assert_eq!(
            count,
            Some(2),
            "mem-x 注入后未再命中 → 负, 条目 + 维度各一行"
        );
        let last = last.expect("最近调权真值");
        // 同批记录按目标 id 序落账, 最后一行 = 条目显著性那行。
        assert_eq!(last.target, "entry:mem-x");
        assert_eq!(last.signal, "negative");
        assert_eq!(last.at_epoch_ms, 2_000);
        assert!(last.evidence.contains("注入后命中回执"));
        assert_eq!(
            wire.evolution_count(),
            count.unwrap(),
            "自述计数与接线层同账"
        );
        let _ = std::fs::remove_dir_all(log.parent().unwrap());
    }

    #[test]
    fn the_ledger_path_sits_beside_the_session_db() {
        #[cfg(windows)]
        assert_eq!(
            retrieval_tuning_log_path_from_session_db(Some(r"C:\data\sessions.sqlite3".into())),
            PathBuf::from(r"C:\data").join(RETRIEVAL_TUNING_LOG_FILE)
        );
        assert_eq!(
            retrieval_tuning_log_path_from_session_db(None),
            PathBuf::from(".apeireth").join(RETRIEVAL_TUNING_LOG_FILE)
        );
    }
}
