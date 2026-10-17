//! V5-c 守望巡检漏斗 (安全守望 · 暴露面体检的巡检化): 筛查→精查→复核→报告。
//!
//! V5-b 暴露面体检回答「暴露了什么」, 本层回答「巡检一遍要花多少、可信度多高、
//! 比上次好了还是坏了」: 把体检六项速扫装进**四段漏斗**, 配三件硬机制 ——
//! 成本闸 (每段耗时/候选上限, 超限截断如实)、断点续查 (巡检状态落库, 中途死
//! 重启续跑)、任务化 (巡检 = 持久任务, 队列同账, 失败留账只能人判)。
//!
//! # 漏斗管线图
//!
//! ```text
//!  ┌────────────────── 守望巡检漏斗 (筛查 → 精查 → 复核 → 报告) ──────────────────┐
//!  │                                                                              │
//!  │ ①筛查段 (快而宽)     ②精查段 (慢而准)     ③复核段 (降误报)    ④报告段        │
//!  │ 六项速扫 ──候选──►  高风险逐项深查  ──►   二次测量一致才报 ──► 漏斗报告        │
//!  │ (V5-b 现有面)        · 明文点→字段级      瞬时态→剔除(降噪)    · 各段计数/耗时 │
//!  │ run_exposure_audit     取证(脱敏后呈现)   风险漂移→剔除         · 降噪率        │
//!  │ → 风险候选            · 备份排除→逐件核对                      · 增量 diff      │
//!  │                      · 凭据面→路径元数据                        (新增/消失/未变)│
//!  │                                                                              │
//!  │ 成本闸: 每段 max_duration_ms / max_candidates —— 超限截断如实 (truncated +    │
//!  │         held_back 留账, 不静默丢件; 截断的余量在续跑会话里继续)              │
//!  │ 断点续查: 巡检状态落库 (每件一落) —— 中途猝死重启续跑, 已做不重做            │
//!  │ 任务接缝: 巡检 = 持久任务 (V6-a 队列语义同款, 对队列只读消费) —— 失败留账     │
//!  │           只能人判, 失败绝不自动重试                                         │
//!  └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # 硬语义 (与 V5-b 体检同族)
//!
//! - **被测面只读零写**: 漏斗不写被测数据面任何字节; 落库的是巡检自己的工作区
//!   ([`checkpoint::PatrolStateStore`]), 与被测面分开;
//! - **零明文留痕**: 精查取证脱敏后呈现 ([`evidence::DeepEvidence`]), 凭据面件
//!   内容零读取, 报告/落库档里没有任何秘密值;
//! - **二次测量一致才报**: 复核段剔除瞬时态与风险漂移 (降噪可度量), 未复核的
//!   候选如实「留存未复核」不作结论;
//! - **诚实口径**: 成本闸超限即截断并如实记账; 未完成的巡检不与上次比 (diff
//!   只在完成巡检产出); 落库档损坏拒绝即帧, 不静默重建。

pub mod checkpoint;
pub mod diff;
pub mod evidence;
pub mod report;
pub mod task_seam;

use std::sync::Arc;

use apeireth_orchestration::task_queue::TaskCaller;

use crate::exposure_audit::funnel::checkpoint::PATROL_ASSIGNEE;
use crate::exposure_audit::funnel::evidence::{deep_check, Candidate};
use crate::exposure_audit::report::RiskLevel;
use crate::exposure_audit::{run_exposure_audit_at, ExposureAuditConfig};

pub use checkpoint::{
    DeepProgress, JsonPatrolStore, MemoryPatrolStore, PatrolCheckpoint, PatrolStateStore,
    RecheckProgress, ScreenProgress, CHECKPOINT_SCHEMA, CHECKPOINT_SCHEMA_VERSION,
    PATROL_STATE_FILE,
};
pub use diff::DiffSummary;
pub use evidence::{DeepEvidence, DeepEvidenceKind};
pub use report::{
    ConfirmedCandidate, FunnelReport, NoiseStats, StageStats, FUNNEL_REPORT_SCHEMA,
    FUNNEL_REPORT_SCHEMA_VERSION, STAGE_ORDER,
};
pub use task_seam::{PatrolTaskAccount, TaskFailureRecord};

/// 巡检时钟口 (epoch 毫秒; 测试注入可控时钟, 生产用系统时钟)。
pub type ClockFn = Arc<dyn Fn() -> i64 + Send + Sync>;

/// 一段的成本闸 (最大耗时 + 最大候选数; 超限截断如实)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageBudget {
    /// 本段最大处理件数 (本会话口径)。
    pub max_candidates: usize,
    /// 本段最大耗时 (毫秒, 本会话口径)。
    pub max_duration_ms: u64,
}

impl Default for StageBudget {
    fn default() -> Self {
        Self {
            max_candidates: 512,
            max_duration_ms: 60_000,
        }
    }
}

/// 漏斗成本闸 (筛查/精查/复核各一把; 报告段是纯组装段, 无逐项闸 ——
/// 零假旋钮: 不设用不上的闸, 耗时仍如实记账)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunnelBudget {
    /// 筛查段闸 (快而宽: 上限从宽)。
    pub screen: StageBudget,
    /// 精查段闸 (慢而准: 上限从严, 大头成本在这)。
    pub deep: StageBudget,
    /// 复核段闸 (二次测量)。
    pub recheck: StageBudget,
}

impl Default for FunnelBudget {
    fn default() -> Self {
        Self {
            screen: StageBudget {
                max_candidates: 1_024,
                max_duration_ms: 30_000,
            },
            deep: StageBudget {
                max_candidates: 64,
                max_duration_ms: 120_000,
            },
            recheck: StageBudget {
                max_candidates: 1_024,
                max_duration_ms: 60_000,
            },
        }
    }
}

impl FunnelBudget {
    /// JSON 回显 (报告里的口径可复核)。
    pub fn to_json_value(&self) -> serde_json::Value {
        fn stage_json(stage: StageBudget) -> serde_json::Value {
            serde_json::json!({
                "max_candidates": stage.max_candidates,
                "max_duration_ms": stage.max_duration_ms,
            })
        }
        serde_json::json!({
            "screen": stage_json(self.screen),
            "deep": stage_json(self.deep),
            "recheck": stage_json(self.recheck),
        })
    }
}

/// 漏斗运行配置 (体检配置 + 成本闸 + 精查线 + 时钟口)。
#[derive(Clone)]
pub struct FunnelConfig {
    /// 体检配置 (被测面 + 探测口; 漏斗只读消费)。
    pub audit: ExposureAuditConfig,
    /// 成本闸。
    pub budget: FunnelBudget,
    /// 精查线: 风险级 ≥ 此线的候选进精查段 (默认中风险 —— 明文点/备份排除
    /// 命中都在精查口径内; 收紧到 High 只深查高风险面)。
    pub deep_risk_floor: RiskLevel,
    /// 时钟口。
    pub clock: ClockFn,
}

impl std::fmt::Debug for FunnelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunnelConfig")
            .field("audit", &self.audit)
            .field("budget", &self.budget)
            .field("deep_risk_floor", &self.deep_risk_floor)
            .field("clock", &"configured")
            .finish()
    }
}

impl FunnelConfig {
    /// 缺省配置 (系统时钟 + 默认成本闸 + 精查线中风险)。
    pub fn new(audit: ExposureAuditConfig) -> Self {
        Self {
            audit,
            budget: FunnelBudget::default(),
            deep_risk_floor: RiskLevel::Medium,
            clock: Arc::new(crate::exposure_audit::now_millis),
        }
    }

    /// 指定成本闸。
    pub fn with_budget(mut self, budget: FunnelBudget) -> Self {
        self.budget = budget;
        self
    }

    /// 指定精查线。
    pub fn with_deep_risk_floor(mut self, floor: RiskLevel) -> Self {
        self.deep_risk_floor = floor;
        self
    }

    /// 指定时钟口 (确定性测试)。
    pub fn with_clock(mut self, clock: ClockFn) -> Self {
        self.clock = clock;
        self
    }
}

/// 漏斗域失败 (错误即帧: 全部带真因, 不静默降级)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatrolError {
    /// 落库读写失败。
    Store {
        /// 失败的操作。
        operation: &'static str,
        /// 人读原因。
        reason: String,
    },
    /// 落库档损坏/不合法 (拒绝即帧, 不静默重建)。
    Checkpoint {
        /// 人读原因。
        reason: String,
    },
    /// 任务账转移被拒 (队列同款错误形状)。
    Task(apeireth_orchestration::task_queue::TaskQueueError),
}

impl std::fmt::Display for PatrolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store { operation, reason } => {
                write!(f, "patrol checkpoint {operation}: {reason}")
            }
            Self::Checkpoint { reason } => write!(f, "patrol checkpoint refused: {reason}"),
            Self::Task(error) => write!(f, "patrol task account refused: {error}"),
        }
    }
}

impl std::error::Error for PatrolError {}

impl From<apeireth_orchestration::task_queue::TaskQueueError> for PatrolError {
    fn from(error: apeireth_orchestration::task_queue::TaskQueueError) -> Self {
        Self::Task(error)
    }
}

/// 跑一次巡检 (漏斗四段; 未完成的巡检自动断点续查: 读档续跑, 已做不重做)。
///
/// 产出 [`FunnelReport`]: 四段账 + 降噪账 + 增量 diff (完成巡检才有) + 脱敏
/// 取证。每处理一件立即落库 —— 中途猝死, 重启调本函数即续跑。
pub fn run_patrol(
    config: &FunnelConfig,
    store: &dyn PatrolStateStore,
) -> Result<FunnelReport, PatrolError> {
    let now = (config.clock)();
    let (mut checkpoint, resumed) = match store.load()? {
        Some(existing) if !existing.completed => {
            let mut checkpoint = existing;
            // 崩溃恢复 (与队列 open 同款): 中途死的执行段回 Pending, 等续跑。
            checkpoint.task.recover_on_resume();
            (checkpoint, true)
        }
        Some(existing) => (
            PatrolCheckpoint::fresh(
                patrol_id_word(now),
                existing.baseline_patrol_id,
                existing.baseline_keys,
            ),
            false,
        ),
        None => (
            PatrolCheckpoint::fresh(patrol_id_word(now), None, Vec::new()),
            false,
        ),
    };

    // 巡检 = 任务: 执行段开始 (自报必须是执行者本人, 队列同款)。
    let runner = TaskCaller::agent(PATROL_ASSIGNEE);
    checkpoint.task.begin(&runner)?;

    // ── 段一: 筛查 (快而宽) ──────────────────────────────────────────────
    let screen_start = (config.clock)();
    let admitted_before = checkpoint.screen.admitted;
    if !checkpoint.screen.is_done() {
        // 六项速扫 = V5-b 现有面, 只读消费; 发现 → 风险候选 (整份落库, 只扫一次)。
        if !checkpoint.screen.scanned {
            let scan = run_exposure_audit_at(&config.audit, screen_start);
            checkpoint.screen.total_findings =
                scan.items.iter().map(|item| item.finding_count()).sum();
            checkpoint.screen.candidates = Candidate::from_report(&scan);
            checkpoint.screen.scanned = true;
        }
        // 成本闸: 逐件准入 (超限截断; 余量留账, 续跑会话继续准入)。
        while checkpoint.screen.admitted < checkpoint.screen.candidates.len() {
            if gate_hit(
                checkpoint.screen.admitted - admitted_before,
                screen_start,
                config,
                config.budget.screen,
            ) {
                break;
            }
            checkpoint.screen.admitted += 1;
            store.save(&checkpoint)?;
        }
        store.save(&checkpoint)?;
    }
    let screen_stats = StageStats {
        stage: STAGE_ORDER[0].to_string(),
        candidates_in: checkpoint.screen.total_findings,
        processed: checkpoint.screen.admitted - admitted_before,
        held_back: checkpoint
            .screen
            .candidates
            .len()
            .saturating_sub(checkpoint.screen.admitted),
        truncated: !checkpoint.screen.is_done(),
        duration_ms: elapsed_ms(screen_start, (config.clock)()),
    };

    // ── 段二: 精查 (慢而准) ──────────────────────────────────────────────
    let deep_start = (config.clock)();
    let bundle = super::resolve_bundle(&config.audit);
    let targets: Vec<Candidate> = checkpoint
        .screen
        .admitted_candidates()
        .iter()
        .filter(|candidate| candidate.risk >= config.deep_risk_floor)
        .cloned()
        .collect();
    let deep_done_before = checkpoint.deep.done_keys.len();
    for candidate in &targets {
        // 断点续查: 已精查的不重做。
        if checkpoint.deep.done_keys.contains(&candidate.key) {
            continue;
        }
        // 成本闸 (本会话口径: 只数这次处理了几件)。
        if gate_hit(
            checkpoint.deep.done_keys.len() - deep_done_before,
            deep_start,
            config,
            config.budget.deep,
        ) {
            break;
        }
        let evidence = deep_check(&config.audit, bundle.as_deref(), candidate);
        checkpoint.deep.evidence.push(evidence);
        checkpoint.deep.done_keys.push(candidate.key.clone());
        // 每件一落库: 中途猝死, 重启只做余下。
        store.save(&checkpoint)?;
    }
    store.save(&checkpoint)?;
    let deep_done_now = checkpoint.deep.done_keys.len() >= targets.len();
    let deep_stats = StageStats {
        stage: STAGE_ORDER[1].to_string(),
        candidates_in: targets.len(),
        processed: checkpoint.deep.done_keys.len() - deep_done_before,
        held_back: targets
            .len()
            .saturating_sub(checkpoint.deep.done_keys.len()),
        truncated: !deep_done_now,
        duration_ms: elapsed_ms(deep_start, (config.clock)()),
    };

    // ── 段三: 复核 (降误报: 二次测量一致才报) ────────────────────────────
    let recheck_start = (config.clock)();
    if !checkpoint.recheck.started {
        // 二次测量 (同一把尺子再量一次; 瞬时态在这里现形)。只采一次,
        // 跨续跑不换尺子。
        let second_scan = run_exposure_audit_at(&config.audit, recheck_start);
        checkpoint.recheck.second = Candidate::from_report(&second_scan);
        checkpoint.recheck.started = true;
        store.save(&checkpoint)?;
    }
    let recheck_done_before = checkpoint.recheck.done_keys.len();
    let admitted = checkpoint.screen.admitted_candidates().to_vec();
    for candidate in &admitted {
        if checkpoint.recheck.done_keys.contains(&candidate.key) {
            continue;
        }
        if gate_hit(
            checkpoint.recheck.done_keys.len() - recheck_done_before,
            recheck_start,
            config,
            config.budget.recheck,
        ) {
            break;
        }
        let second = checkpoint
            .recheck
            .second
            .iter()
            .find(|probe| probe.key == candidate.key);
        match second {
            Some(again) if again.risk == candidate.risk => {
                checkpoint.recheck.confirmed.push(candidate.key.clone());
            }
            Some(_) => checkpoint
                .recheck
                .dropped_risk_shift
                .push(candidate.key.clone()),
            None => checkpoint
                .recheck
                .dropped_transient
                .push(candidate.key.clone()),
        }
        checkpoint.recheck.done_keys.push(candidate.key.clone());
        store.save(&checkpoint)?;
    }
    store.save(&checkpoint)?;
    let recheck_done_now = checkpoint.recheck.done_keys.len() >= admitted.len();
    let held_unverified = admitted
        .len()
        .saturating_sub(checkpoint.recheck.done_keys.len());
    let recheck_stats = StageStats {
        stage: STAGE_ORDER[2].to_string(),
        candidates_in: admitted.len(),
        processed: checkpoint.recheck.done_keys.len() - recheck_done_before,
        held_back: held_unverified,
        truncated: !recheck_done_now,
        duration_ms: elapsed_ms(recheck_start, (config.clock)()),
    };

    // ── 段四: 报告 (漏斗报告 + 增量视角) ─────────────────────────────────
    let report_start = (config.clock)();
    let confirmed: Vec<ConfirmedCandidate> = checkpoint
        .recheck
        .confirmed
        .iter()
        .filter_map(|key| {
            let candidate = checkpoint
                .screen
                .candidates
                .iter()
                .find(|candidate| candidate.key == *key)?;
            let evidence = checkpoint
                .deep
                .evidence
                .iter()
                .find(|evidence| evidence.key == *key)
                .cloned();
            Some(ConfirmedCandidate {
                candidate: candidate.clone(),
                deep_checked: evidence.is_some(),
                evidence,
            })
        })
        .collect();

    let complete_now = checkpoint.screen.is_done() && deep_done_now && recheck_done_now;
    let diff = if complete_now {
        let current_keys: Vec<String> = checkpoint.recheck.confirmed.clone();
        let summary = DiffSummary::between(
            checkpoint.baseline_patrol_id.clone(),
            &checkpoint.baseline_keys,
            &current_keys,
        );
        // 完成即换基线: 下次巡检与本次比 (增量视角逐次前移)。
        checkpoint.completed = true;
        checkpoint.baseline_patrol_id = Some(checkpoint.patrol_id.clone());
        checkpoint.baseline_keys = current_keys;
        checkpoint.task.complete(&runner)?;
        store.save(&checkpoint)?;
        Some(summary)
    } else {
        // 未完成不与上次比 (如实不产出 diff); 状态已落库, 重启续跑。
        checkpoint
            .task
            .note_error("会话结束于成本闸截断, 巡检未完成 (断点续查中)");
        store.save(&checkpoint)?;
        None
    };

    let noise = NoiseStats::from_counts(
        checkpoint.screen.admitted,
        checkpoint.recheck.confirmed.len(),
        checkpoint.recheck.dropped_transient.len(),
        checkpoint.recheck.dropped_risk_shift.len(),
        held_unverified,
    );
    let report_stats = StageStats {
        stage: STAGE_ORDER[3].to_string(),
        candidates_in: confirmed.len(),
        processed: confirmed.len(),
        held_back: 0,
        truncated: false,
        duration_ms: elapsed_ms(report_start, (config.clock)()),
    };

    Ok(FunnelReport {
        patrol_id: checkpoint.patrol_id.clone(),
        resumed,
        attempts: checkpoint.task.attempts,
        completed: complete_now,
        generated_at_ms: (config.clock)(),
        task: checkpoint.task.clone(),
        stages: vec![screen_stats, deep_stats, recheck_stats, report_stats],
        noise,
        confirmed,
        diff,
        budget: config.budget,
    })
}

/// 巡检 id 词面 (时刻派号; 幂等键同源)。
fn patrol_id_word(now_ms: i64) -> String {
    format!("patrol-{now_ms}")
}

/// 耗时 (毫秒; 回拨钳 0, 不编时间)。
fn elapsed_ms(start_ms: i64, end_ms: i64) -> u64 {
    u64::try_from(end_ms.saturating_sub(start_ms)).unwrap_or(0)
}

/// 成本闸判定 (本会话口径): 已处理件数 ≥ max_candidates, 或 已耗时 ≥
/// max_duration_ms, 即不再处理下一件 (超限截断如实)。
fn gate_hit(
    processed_this_session: usize,
    stage_start_ms: i64,
    config: &FunnelConfig,
    budget: StageBudget,
) -> bool {
    processed_this_session >= budget.max_candidates
        || elapsed_ms(stage_start_ms, (config.clock)()) >= budget.max_duration_ms
}
