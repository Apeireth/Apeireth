//! 生物机制族只读探测口与装配注册表（W3.2 注册面 + W3.5 观测扩展）。
//!
//! 本文件是 [`ProductionSelfStatusSource`] 的**观测实现面**, 从
//! `self_status_source.rs` 拆出: 行为 / 公开 API / 测试语义逐字不变, 仅为把
//! 该文件行数收回 `scripts/arch-baseline.txt` 的 size 基线（2173 行）以内。
//! 公开项经 `self_status_source` 原路径重导出（`pub use`), 既有调用方与
//! 集成测试的 import 路径不变。
//!
//! 内容:
//! - [`BioMechanismRegistry`]: 只读聚合注册表（默认关 / 默认不注册 probe）;
//! - [`capacity_bio_probe`]: 承载力 SG-26（读与 `tool.task_board` 同一任务队列）;
//! - W3.5 六条真实运行时数据源 probe: 免疫记忆 SG-15 / 审议门 SG-24 /
//!   关怀心跳 SG-19 / 练习账 SG-21 / 痛觉 SG-16 / 再生 SG-17;
//! - [`ProductionSelfStatusSource`] 的 `with_*_probe` 便捷注册口。
//!
//! 0 装口径不变: probe 只负责值/阈值/最近动作; `stage` / `wired` 由注册表
//! 统一置位; 未注册 = `wired:false` + 原因「未接线」; 读失败 = 单格
//! `null` + 真因, 不整帧失败; 开关关时一个 probe 都不调用。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_guard::hook::SharedImmuneMemory;
use apeireth_guard::immune_memory::{ImmuneMemoryStore as _, IncidentStatus};
use apeireth_orchestration::quorum::QuorumDecision;
use apeireth_organ::nociception::BioPainSnapshot;
use apeireth_organ::play::PlayStatus;
use apeireth_tools_canonical::self_status::{
    bio_self_status_enabled_from_env_value, BioMechanismKind, BioMechanismProbe,
    BioMechanismReading, BioMechanismsStatus, BIO_MECHANISM_NOT_WIRED_REASON,
    BIO_SELF_STATUS_DISABLED_REASON, BIO_SELF_STATUS_ENABLE_ENV, BIO_STAGE_WIRED,
};

use crate::canonical::care_heartbeat::CareHeartbeatStatus;
use crate::canonical::orchestrator::{
    immune_record_is_escalation, OrganOrchestrator, QuorumTrace, RelationshipState,
};
use crate::canonical::play_tools::BioPlayState;

use super::ProductionSelfStatusSource;

/// 生物机制族只读聚合的装配注册表 (W3.2 装配面)。
///
/// 契约 (0 装诚实):
/// - **默认不注册任何 probe**: 开关开时 14 条机制全部 `wired: false` +
///   原因「未接线」, 不编数值;
/// - **默认关**: 开关只认 `APEIRETH_ENABLE_BIO_SELF_STATUS="1"` (fail-closed);
///   关时整节 `mechanisms: null` + 原因「未启用」, **一个 probe 都不调用**;
/// - 开关开且注册了 probe 才采值; 单格 probe 失败 = 该格 `null` + 真因,
///   不整帧失败; 采集顺序 = [`BioMechanismKind::ALL`] (确定性)。
pub struct BioMechanismRegistry {
    enabled: bool,
    now_ms: Arc<dyn Fn() -> i64 + Send + Sync>,
    probes: BTreeMap<BioMechanismKind, Arc<dyn BioMechanismProbe>>,
}

impl BioMechanismRegistry {
    /// 显式开关值 + 空注册表 (装配根注入；不读进程 env, 测试可复现)。
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            now_ms: Arc::new(crate::canonical::trait_growth::now_epoch_ms),
            probes: BTreeMap::new(),
        }
    }

    /// 从进程 env 现值构造 (`APEIRETH_ENABLE_BIO_SELF_STATUS`; 只认 `"1"`)。
    pub fn from_env() -> Self {
        Self::from_env_value(std::env::var(BIO_SELF_STATUS_ENABLE_ENV).ok().as_deref())
    }

    /// 从 env 原始值构造 (纯函数, 测试可注入, 不碰进程 env)。
    pub fn from_env_value(raw: Option<&str>) -> Self {
        Self::new(bio_self_status_enabled_from_env_value(raw))
    }

    /// 注入采集时钟 (epoch 毫秒；缺省系统时钟, 测试注入固定值保证确定性)。
    #[must_use]
    pub fn with_now_ms(mut self, now_ms: Arc<dyn Fn() -> i64 + Send + Sync>) -> Self {
        self.now_ms = now_ms;
        self
    }

    /// 开关生效值 (只认 `"1"`)。
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 注册一格机制只读探测口 (装配入口)。
    ///
    /// 注册 ≠ 启用: 开关关时不调用; 同名后注册覆盖先注册。
    pub fn register_bio_probe(
        &mut self,
        kind: BioMechanismKind,
        probe: Arc<dyn BioMechanismProbe>,
    ) {
        self.probes.insert(kind, probe);
    }

    /// 聚合一份只读快照: 关 = `mechanisms: null` + 原因; 开 = 14 格读数
    /// (未注册的格 `wired: false` + 原因「未接线」; 读失败 = null + 真因)。
    pub fn snapshot(&self) -> BioMechanismsStatus {
        if !self.enabled {
            return BioMechanismsStatus::disabled(BIO_SELF_STATUS_DISABLED_REASON);
        }
        let now_ms = (self.now_ms)();
        let mut mechanisms = BTreeMap::new();
        for kind in BioMechanismKind::ALL {
            let reading = match self.probes.get(&kind) {
                Some(probe) => match probe.snapshot(now_ms) {
                    // 注册成功 = 接线事实成立; `stage`/`wired` 由注册表统一置位,
                    // probe 只负责值/阈值/最近动作 (不冒充未接线)。
                    Ok(reading) => BioMechanismReading {
                        stage: BIO_STAGE_WIRED.to_string(),
                        wired: true,
                        ..reading
                    },
                    Err(error) => {
                        BioMechanismReading::probe_failed(format!("机制探测失败: {error}"))
                    }
                },
                None => BioMechanismReading::not_wired(BIO_MECHANISM_NOT_WIRED_REASON),
            };
            mechanisms.insert(kind, reading);
        }
        BioMechanismsStatus::enabled_with(mechanisms)
    }
}

/// 生物机制「承载力」(SG-26) 只读探测口: 读运行时**同一个**任务队列
/// ([`apeireth_orchestration::task_queue::TaskQueue`], 与 `tool.task_board`
/// 同队同账) 的分态计数真值, 以 `assigned + running` 作为已用并发承载读数。
///
/// `active_task_limit` = 装配根注入的**生效**并发上限 (`None` = 未配置,
/// 阈值显式 `null`, 绝不编阈值)。只读: 不认领/不改派/不结算任务; 读失败回
/// `Err(真因)`, 自述格显式 `null` + 原因。
pub fn capacity_bio_probe(
    queue: Arc<apeireth_orchestration::task_queue::TaskQueue>,
    active_task_limit: Option<u64>,
) -> Arc<dyn BioMechanismProbe> {
    Arc::new(CapacityTaskQueueProbe {
        queue,
        active_task_limit,
    })
}

/// 承载力探测口本体 (真实运行时任务队列读数)。
struct CapacityTaskQueueProbe {
    queue: Arc<apeireth_orchestration::task_queue::TaskQueue>,
    active_task_limit: Option<u64>,
}

impl BioMechanismProbe for CapacityTaskQueueProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let counts = self
            .queue
            .counts()
            .map_err(|error| format!("task queue read failed: {error}"))?;
        let active_tasks = counts.assigned.saturating_add(counts.running);
        // u64 → f64 是展示口径 (阈值字段是 JSON number; 不作为精确容量运算)。
        let threshold = self.active_task_limit.map(|limit| limit as f64);
        Ok(BioMechanismReading::wired(
            Some(serde_json::json!({
                "active_tasks": active_tasks,
                "pending": counts.pending,
                "assigned": counts.assigned,
                "running": counts.running,
                "observed_at_ms": now_ms,
            })),
            threshold,
            None,
        ))
    }
}

// ============================================
// W3.5 生物机制观测扩展: 装配期可取得的真实运行时数据源 probe
// ============================================
//
// 本节 probe 全部**只读**: 不改账本、不改状态机、不做 I/O 写、不返回用户原文 /
// 路径 / 记录键 / 凭据; 采集失败 = `Err(真因)`, 由注册表折成单格 `null` + 原因,
// 不整帧失败。probe 只负责值; `stage` / `wired` 由 `BioMechanismRegistry` 置位。
//
// 数据源口径 (与既有运行面同源):
// - 免疫记忆 (SG-15): guard hook 正在使用的**同一份** [`SharedImmuneMemory`];
// - 群体感应审议门 (SG-24): orchestrator 的**最近一次** gated 门账
//   ([`QuorumTrace`]); 门默认关时无门账 = 如实「尚无 trace」;
// - 关怀心跳 (SG-19 依恋/重聚): 常驻心跳生产者的**同一份**状态闭包
//   ([`CareHeartbeatStatus`], 与自述关怀节心跳两列同一真值源);
// - 游戏练习账 (SG-21): 四件套共持的**同一本** [`BioPlayState`] 内存账;
// - 痛觉 (SG-16) / 再生 (SG-17): 运行时 [`apeireth_runtime::canonical::Runtime`]
//   的只读快照, **迟绑定** (构造自述源时 runtime 句柄尚不存在), 因此走读取闭包;
//   闭包返回 `Err(真因)` = 该格 `null` + 原因 (例如 runtime 尚未挂上)。

/// 痛觉观测快照读取口 (迟绑定): 运行时构造后由装配根绑定
/// `organ::nociception::NociceptionObserver::snapshot` 的只读投影; `Err(真因)` = 该格 `null` + 原因。
pub type PainSnapshotReader = Arc<dyn Fn() -> Result<BioPainSnapshot, String> + Send + Sync>;

/// 生物机制「免疫记忆」(SG-15) 只读探测口: 读 guard hook 正在使用的**同一份**
/// [`SharedImmuneMemory`] 账本 (同一把锁, 短持锁, 不持其它锁)。
///
/// 口径: `records` = 账本记录总数; `open` / `resolved` = 未收口 / 已收口数;
/// `known_patterns` = 观察次数 `>= 2` 的适应记忆模式数 (内核
/// [`apeireth_guard::immune_memory::ImmuneMemory::known_patterns`] 同口径);
/// `escalations` = 最近一次观察处于「新签名高严重度 / 冷却后高严重度复发」的记录数
/// (与 W3.3 免疫升议注入同一判定 [`immune_record_is_escalation`]);
/// `first_seen_ms` / `last_seen_ms` = 账本首末观察时刻 (空账 = `null`)。
///
/// **脱敏**: 只回计数、机制类别的稳定名与时刻; 不回签名 / 记录键 / 原始键 /
/// 文件路径 / 凭据。**不编数值**: 账本空 = 全零计数 (实况), 不是「未接线」。
pub fn immune_quorum_probe(memory: SharedImmuneMemory) -> Arc<dyn BioMechanismProbe> {
    Arc::new(ImmuneMemoryProbe { memory })
}

/// 免疫记忆探测口本体。
struct ImmuneMemoryProbe {
    memory: SharedImmuneMemory,
}

impl BioMechanismProbe for ImmuneMemoryProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let memory = self.memory.lock();
        let records = memory.store().list();
        let mut kinds: BTreeMap<&'static str, u64> = BTreeMap::new();
        let mut open = 0u64;
        let mut resolved = 0u64;
        let mut escalations = 0u64;
        let mut first_seen_ms: Option<i64> = None;
        let mut last_seen_ms: Option<i64> = None;
        for (_, record) in &records {
            match record.status {
                IncidentStatus::Open => open = open.saturating_add(1),
                IncidentStatus::Resolved => resolved = resolved.saturating_add(1),
            }
            if immune_record_is_escalation(record) {
                escalations = escalations.saturating_add(1);
            }
            let entry = kinds.entry(record.kind.as_str()).or_insert(0);
            *entry = entry.saturating_add(1);
            first_seen_ms = Some(first_seen_ms.map_or(record.first_seen_ms, |value| {
                value.min(record.first_seen_ms)
            }));
            last_seen_ms = Some(
                last_seen_ms.map_or(record.last_seen_ms, |value| value.max(record.last_seen_ms)),
            );
        }
        let records_total = records.len();
        Ok(BioMechanismReading::wired(
            Some(serde_json::json!({
                "records": records_total,
                "open": open,
                "resolved": resolved,
                "known_patterns": memory.known_patterns(),
                "escalations": escalations,
                "kinds": kinds,
                "first_seen_ms": first_seen_ms,
                "last_seen_ms": last_seen_ms,
                "observed_at_ms": now_ms,
            })),
            None,
            None,
        ))
    }
}

/// 生物机制「群体感应审议门」(SG-24) 只读探测口: 读 orchestrator 的**最近一次**
/// gated 审议门账 ([`OrganOrchestrator::last_quorum_trace`], 同步只读, 不取门锁)。
///
/// 口径: `traces` = 门账是否存在 (`0` / `1`; 默认关 / 从未 gated = `0`);
/// `convened` = 最近一次是否召开; `escalation_score` / `reason_count` = 兑现升议
/// 时的浓度与有界理由条数; `denial` = 拒绝召开的类型 (`hold` / `cooldown`),
/// 带该分支自己的分数/差额/解禁时刻。**不回原文**: 不含 `reasons` 文本
/// (其中可能有自由文本来源), 只回计数与机器分类。
pub fn quorum_gate_probe<RS>(orchestrator: Arc<OrganOrchestrator<RS>>) -> Arc<dyn BioMechanismProbe>
where
    RS: RelationshipState + 'static,
{
    quorum_trace_probe(Arc::new(move || orchestrator.last_quorum_trace()))
}

/// 群体感应门账读取口 (迟绑定/测试注入): 返回最近一次门账。
///
/// 与 [`quorum_gate_probe`] 同一采集逻辑; orchestrator 不便以 `Arc` 捕获时,
/// 装配根可传自己的只读读取闭包 (例如由更外层句柄转调)。
pub fn quorum_trace_probe(
    reader: Arc<dyn Fn() -> Option<QuorumTrace> + Send + Sync>,
) -> Arc<dyn BioMechanismProbe> {
    Arc::new(QuorumTraceProbe { reader })
}

/// 群体感应门账探测口本体。
struct QuorumTraceProbe {
    reader: Arc<dyn Fn() -> Option<QuorumTrace> + Send + Sync>,
}

impl BioMechanismProbe for QuorumTraceProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let value = match (self.reader)() {
            None => serde_json::json!({
                "traces": 0,
                "convened": null,
                "at_ms": null,
                "escalation_score": null,
                "reason_count": 0,
                "denial": null,
                "denial_score": null,
                "denial_need": null,
                "cooldown_until_ms": null,
                "observed_at_ms": now_ms,
            }),
            Some(trace) => {
                let (denial, denial_score, denial_need, cooldown_until_ms) = match &trace.denial {
                    None => (None, None, None, None),
                    Some(QuorumDecision::Hold { score, need }) => {
                        (Some("hold"), Some(*score), Some(*need), None)
                    }
                    Some(QuorumDecision::Cooldown { until_ms, score }) => {
                        (Some("cooldown"), Some(*score), None, Some(*until_ms))
                    }
                    Some(QuorumDecision::Escalate { score, .. }) => {
                        (Some("escalate"), Some(*score), None, None)
                    }
                };
                serde_json::json!({
                    "traces": 1,
                    "convened": trace.convened,
                    "at_ms": trace.at_ms,
                    "escalation_score": trace.escalation.as_ref().map(|escalation| escalation.score),
                    "reason_count": trace.reasons().len(),
                    "denial": denial,
                    "denial_score": denial_score,
                    "denial_need": denial_need,
                    "cooldown_until_ms": cooldown_until_ms,
                    "observed_at_ms": now_ms,
                })
            }
        };
        Ok(BioMechanismReading::wired(Some(value), None, None))
    }
}

/// 生物机制「依恋 / 分离-重聚」(SG-19) 只读探测口: 读常驻关怀心跳生产者的
/// **同一份**状态闭包 ([`CareHeartbeatStatus`], 与自述关怀节心跳两列同源)。
///
/// 口径: `polls` / `hits` = 累计 poll 与心跳命中发起次数 (进程内计数, 重启清零);
/// `next_poll_at_ms` = 下次 poll 时刻 (`null` = 生产者未运行/未启动, 如实);
/// `last_hit_at_ms` = 最近一次心跳命中 (`null` = 尚无命中)。
///
/// 该读取口是**进程内快照**, 无 I/O 失败通道; 未启动 = `next_poll_at_ms: null`
/// (不是编 0, 也不是「未接线」)。
pub fn care_heartbeat_probe(
    reader: Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync>,
) -> Arc<dyn BioMechanismProbe> {
    Arc::new(CareHeartbeatProbe { reader })
}

/// 关怀心跳态探测口本体。
struct CareHeartbeatProbe {
    reader: Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync>,
}

impl BioMechanismProbe for CareHeartbeatProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let status = (self.reader)();
        Ok(BioMechanismReading::wired(
            Some(serde_json::json!({
                "polls": status.polls,
                "hits": status.hits,
                "next_poll_at_ms": status.next_poll_at_ms,
                "last_hit_at_ms": status.last_hit_at_ms,
                "observed_at_ms": now_ms,
            })),
            None,
            None,
        ))
    }
}

/// 生物机制「游戏 / 幼态延续」(SG-21) 只读探测口: 读四件套共持的**同一本**
/// [`BioPlayState`] 内存账 (与 `tool.play_*` 四工具同账)。
///
/// 口径: `sessions` = 账本 session 总数; `pending` = `planned + running`;
/// `finished` / `aborted` = 两个终态各自计数。**只回计数**: 不回 session id /
/// 主题 / 沙箱路径 / 产物条目 (账本只存内存, 重启清零, 不冒充持久)。
pub fn play_probe(state: Arc<BioPlayState>) -> Arc<dyn BioMechanismProbe> {
    Arc::new(BioPlayLedgerProbe { state })
}

/// 游戏练习账探测口本体。
struct BioPlayLedgerProbe {
    state: Arc<BioPlayState>,
}

impl BioMechanismProbe for BioPlayLedgerProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let sessions = self.state.sessions();
        let mut planned = 0u64;
        let mut running = 0u64;
        let mut finished = 0u64;
        let mut aborted = 0u64;
        for session in &sessions {
            match session.status {
                PlayStatus::Planned => planned = planned.saturating_add(1),
                PlayStatus::Running => running = running.saturating_add(1),
                PlayStatus::Finished => finished = finished.saturating_add(1),
                PlayStatus::Aborted => aborted = aborted.saturating_add(1),
            }
        }
        let sessions_total = sessions.len();
        Ok(BioMechanismReading::wired(
            Some(serde_json::json!({
                "sessions": sessions_total,
                "pending": planned.saturating_add(running),
                "planned": planned,
                "running": running,
                "finished": finished,
                "aborted": aborted,
                "observed_at_ms": now_ms,
            })),
            None,
            None,
        ))
    }
}

/// 生物机制「痛觉学习」(SG-16) 只读探测口: 读运行时痛觉账的只读快照
/// ([`BioPainSnapshot`])。
///
/// 迟绑定原因: 自述源在 runtime **之前**构造, 构造期拿不到 runtime 句柄; 因此由
/// 装配根把已装入 runtime 的 `NociceptionObserver` 快照绑进读取闭包。闭包
/// 返回 `Err(真因)` (例如 runtime 尚未挂上) = 该格 `null` + 原因, 不整帧失败。
///
/// 口径: `enabled` = 运行时痛觉开关真值 (默认关时其余计数为零, 如实);
/// `assessed` / `repaired` = 累计评估 / 生效修复次数; `pain_groups` = 账本当前
/// 分组数; `observations` / `chronic_observations` = 最近观测条数与其中慢性化数。
/// **只回计数与开关**, 不回能力名 / 原因键 / 错误原文。
pub fn pain_probe(reader: PainSnapshotReader) -> Arc<dyn BioMechanismProbe> {
    Arc::new(PainSnapshotProbe { reader })
}

/// 痛觉快照探测口本体。
struct PainSnapshotProbe {
    reader: PainSnapshotReader,
}

impl BioMechanismProbe for PainSnapshotProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let snapshot = (self.reader)()?;
        let chronic = snapshot
            .observations
            .iter()
            .filter(|observation| observation.chronic)
            .count();
        Ok(BioMechanismReading::wired(
            Some(serde_json::json!({
                "enabled": snapshot.enabled,
                "assessed": snapshot.assessed,
                "repaired": snapshot.repaired,
                "pain_groups": snapshot.groups.len(),
                "observations": snapshot.observations.len(),
                "chronic_observations": chronic,
                "observed_at_ms": now_ms,
            })),
            None,
            None,
        ))
    }
}

/// 生物机制「再生四期」(SG-17) 只读探测口: 读同一 runtime 痛觉快照里的再生案例账
/// ([`BioPainSnapshot::recovery`], 与 `NociceptionObserver::recovery_snapshot` 同源投影)。
///
/// 与 [`pain_probe`] 共用**同一读取口** (同一个 runtime 开关、同一把账本锁):
/// 装配根绑一个闭包即可同时接两格。闭包返回 `Err(真因)` = 该格 `null` + 原因。
///
/// 口径: `active_cases` / `closed_cases` / `rejected_transitions` 与
/// `hemostasis` / `inflammation` / `proliferation` / `remodeling` / `resolved`
/// 四期分布全为计数; `enabled` = 运行时开关真值。**只回计数**, 不回案例 id /
/// 能力名 / 原因键 / 证据串。
pub fn recovery_probe(reader: PainSnapshotReader) -> Arc<dyn BioMechanismProbe> {
    Arc::new(RecoverySnapshotProbe { reader })
}

/// 再生案例账探测口本体。
struct RecoverySnapshotProbe {
    reader: PainSnapshotReader,
}

impl BioMechanismProbe for RecoverySnapshotProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        let snapshot = (self.reader)()?;
        let recovery = &snapshot.recovery;
        Ok(BioMechanismReading::wired(
            Some(serde_json::json!({
                "enabled": snapshot.enabled,
                "active_cases": recovery.active_cases,
                "closed_cases": recovery.closed_cases,
                "rejected_transitions": recovery.rejected_transitions,
                "hemostasis": recovery.phases.hemostasis,
                "inflammation": recovery.phases.inflammation,
                "proliferation": recovery.phases.proliferation,
                "remodeling": recovery.phases.remodeling,
                "resolved": recovery.phases.resolved,
                "observed_at_ms": now_ms,
            })),
            None,
            None,
        ))
    }
}

impl ProductionSelfStatusSource {
    /// 便捷注册口: 免疫记忆格 (SG-15) —— 读 guard hook 同一份共享记忆账的
    /// 只读计数 (与 W3.3 免疫升议同判定; 只回计数/类别名/时刻, 不回签名/原文)。
    ///
    /// 注册口与 [`Self::register_bio_probe`] 同语义: 注册 ≠ 启用, 开关默认关。
    #[must_use]
    pub fn with_immune_probe(mut self, memory: SharedImmuneMemory) -> Self {
        self.register_bio_probe(BioMechanismKind::ImmuneMemory, immune_quorum_probe(memory));
        self
    }

    /// 便捷注册口: 群体感应审议门格 (SG-24) —— 读 orchestrator 最近一次门账的
    /// 只读投影 (同 [`quorum_gate_probe`]; 不回门账理由文本)。
    ///
    /// 注册口与 [`Self::register_bio_probe`] 同语义: 注册 ≠ 启用, 开关默认关。
    #[must_use]
    pub fn with_quorum_gate_probe<RS>(mut self, orchestrator: Arc<OrganOrchestrator<RS>>) -> Self
    where
        RS: RelationshipState + 'static,
    {
        self.register_bio_probe(BioMechanismKind::Quorum, quorum_gate_probe(orchestrator));
        self
    }

    /// 便捷注册口: 依恋/重合格 (SG-19) —— 读常驻关怀心跳态 (polls / hits /
    /// next_poll / last_hit; 与自述关怀节心跳两列同一真值源)。
    ///
    /// 注册口与 [`Self::register_bio_probe`] 同语义: 注册 ≠ 启用, 开关默认关。
    #[must_use]
    pub fn with_care_heartbeat_probe(
        mut self,
        reader: Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync>,
    ) -> Self {
        self.register_bio_probe(BioMechanismKind::Attachment, care_heartbeat_probe(reader));
        self
    }

    /// 便捷注册口: 游戏/幼态延续格 (SG-21) —— 读四件套同一本练习账的计数
    /// (只回计数, 不回 session id / 主题 / 路径)。
    ///
    /// 注册口与 [`Self::register_bio_probe`] 同语义: 注册 ≠ 启用, 开关默认关。
    #[must_use]
    pub fn with_play_probe(mut self, state: Arc<BioPlayState>) -> Self {
        self.register_bio_probe(BioMechanismKind::Play, play_probe(state));
        self
    }

    /// 便捷注册口: 痛觉格 (SG-16) —— 迟绑定的运行时痛觉快照读取口
    /// ([`pain_probe`]); `Err(真因)` = 该格 `null` + 原因。
    ///
    /// 注册口与 [`Self::register_bio_probe`] 同语义: 注册 ≠ 启用, 开关默认关。
    #[must_use]
    pub fn with_pain_probe(mut self, reader: PainSnapshotReader) -> Self {
        self.register_bio_probe(BioMechanismKind::Nociception, pain_probe(reader));
        self
    }

    /// 便捷注册口: 再生四期格 (SG-17) —— 与痛觉共用同一运行时快照读取口
    /// ([`recovery_probe`]); `Err(真因)` = 该格 `null` + 原因。
    ///
    /// 注册口与 [`Self::register_bio_probe`] 同语义: 注册 ≠ 启用, 开关默认关。
    #[must_use]
    pub fn with_recovery_probe(mut self, reader: PainSnapshotReader) -> Self {
        self.register_bio_probe(BioMechanismKind::Recovery, recovery_probe(reader));
        self
    }
}
