//! G-P1 指标数据源: 回合收束指标的**进程内滚动窗口**(会话 + 全局两层)。
//!
//! 这是遥测面板 (`GET /v1/panel/metrics`) 的唯一数据面: 回合延迟分布、token
//! 流向、收束分类计数从这里读。设计口径:
//!
//! * **一收束一拍**: 每个回合收束点记一拍 [`TurnMetrics`],按成功/失败/待批
//!   三分类分别计数。待批拍之后的审批续跑会再收束,再记一拍 (与
//!   `RuntimeEvent` 的收束事件同口径 —— 同一回合可以有多个收束拍)。
//! * **usage 计量同源**: token 列取自回合收束处的 usage 计量,与聊天头
//!   turn-telemetry 同一数据源,不另立第二套计量。
//! * **内存环形, 不落盘**: 留存策略就是"进程内滚动" —— 每层一个环形缓冲,
//!   窗口 = 最近 N 拍 (N 配置默认 [`DEFAULT_METRICS_WINDOW_TURNS`]),进程退出
//!   即清空。会话环有驻留上限 ([`MAX_TRACKED_SESSIONS`]),观测面不会自己
//!   变成负担。
//! * **未知不冒充**: 计量源没有的列 (计价; provider 未报缓存列时的命中) 记
//!   `None`,聚合与端点都如实报"未知"(`null` / 遥测条同款 "—"),绝不用 0 冒充。
//!   缓存命中列自 G-P1 缺口①接入: provider 归一化 usage 报缓存列时这里取真值
//!   ([`TurnMetrics::succeeded`] 的取数口), 不报则照旧 "—"。
//!
//! 分位口径: 最近秩 (nearest-rank) —— 排序后取 `ceil(p × n)` 位 (1-based)。

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use apeireth_core::kernel::SessionId;

use super::approval::PendingApprovalView;
use super::error::RuntimeResult;
use super::execute::{ApprovalResolution, TurnOutcome, TurnResponse};

/// 滚动窗口默认大小: 每层保留最近 50 个收束拍 (N 的配置默认值)。
pub const DEFAULT_METRICS_WINDOW_TURNS: usize = 50;

/// 窗口配置的收口上界。观测面是旁路状态,不允许一个配置值把它变成内存
/// 负担 (与 `MAX_SESSION_LOCKS` 同一防呆思路)。
pub const MAX_METRICS_WINDOW_TURNS: usize = 4096;

/// 同时驻留的会话环上限: 超限时驱逐最久未记录的会话环。全局环 + 会话环的
/// 内存上界因此是 `窗口 × (1 + MAX_TRACKED_SESSIONS)` 拍。
pub const MAX_TRACKED_SESSIONS: usize = 256;

/// 一拍的收束分类 (成功/失败/待批分别计数)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcomeKind {
    /// 回合收束于最终助手回复。
    Succeeded,
    /// 回合收束于失败 (治理否决完成、轮上限、provider 耗尽等)。
    Failed,
    /// 回合收束于待批挂起。
    PendingApproval,
}

/// 一个回合收束拍的指标记录 (字段表见 `docs/04-internal/backend-enhancement/
/// G-observability.md`)。
///
/// `input_tokens` / `output_tokens` / `tool_calls` / `rounds` 用 `Option`:
/// 失败/待批收束处这些计量可能不在场,`None` = 未知,聚合只统计有值的拍,
/// 不把未知记成 0。
#[derive(Debug, Clone, PartialEq)]
pub struct TurnMetrics {
    /// 收束分类。
    pub outcome: TurnOutcomeKind,
    /// 收束段执行耗时 (毫秒)。不含会话排队与人工审批等待。
    pub latency_ms: u64,
    /// 输入 token (usage 计量, turn-telemetry 同源)。`None` = 收束处无计量。
    pub input_tokens: Option<u64>,
    /// 输出 token (同上口径)。
    pub output_tokens: Option<u64>,
    /// 提示缓存命中输入 token。`None` = usage 计量无缓存列 (未知, 非 0)。
    pub cache_hit_tokens: Option<u64>,
    /// 缓存命中率 (命中数/输入数, 0.0–1.0)。`None` = 命中未知或输入为 0。
    pub cache_hit_rate: Option<f64>,
    /// 回合内工具派发数。`None` = 收束处无派发轨迹 (失败/待批收束)。
    pub tool_calls: Option<u32>,
    /// 回合消耗的逻辑轮数。`None` = 收束处轮数不可知。
    pub rounds: Option<u32>,
    /// 回合成本 (微单位, 可选列)。`None` = 未计价 —— 计价换算点唯一归
    /// 花费线的回合计量 (`super::quota`),指标面不做第二次换算。
    pub cost_micros: Option<u64>,
}

/// 毫秒口径的收束段耗时 (饱和收敛, 不回绕)。
fn latency_ms(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// 工具派发数的 u32 收敛 (M11 同口径: 溢出封顶, 不回绕)。
fn dispatch_count(response: &TurnResponse) -> u32 {
    u32::try_from(response.trace.capability_dispatches()).unwrap_or(u32::MAX)
}

impl TurnMetrics {
    /// 成功收束拍: usage 计量在场 (turn-telemetry 同源),轮数与派发数取自
    /// 回合记录。
    pub fn succeeded(elapsed: Duration, response: &TurnResponse) -> Self {
        let input_tokens = u64::from(response.usage.prompt_tokens);
        // CACHE 格取数口 (G-P1 缺口①接入, 只改这一处): 命中数取自 usage 计量
        // 的缓存列 (`NormalizedUsage::prompt_cache_hit_tokens`); provider 未报
        // 缓存列 = None ("—" = 未知, 不冒充 0)。命中率 = 命中数/输入数
        // (G-P1 冻结口径; 命中未知或输入为 0 → None)。
        let cache_hit_tokens = response.usage.prompt_cache_hit_tokens.map(u64::from);
        let cache_hit_rate = match (cache_hit_tokens, input_tokens) {
            (Some(hit), input) if input > 0 => Some(hit as f64 / input as f64),
            _ => None,
        };
        Self {
            outcome: TurnOutcomeKind::Succeeded,
            latency_ms: latency_ms(elapsed),
            input_tokens: Some(input_tokens),
            output_tokens: Some(u64::from(response.usage.completion_tokens)),
            cache_hit_tokens,
            cache_hit_rate,
            tool_calls: Some(dispatch_count(response)),
            rounds: Some(response.rounds),
            // 可选列: 计价换算唯一归花费线, 本面不二次换算 (—)。
            cost_micros: None,
        }
    }

    /// 待批收束拍: 轮数已知,usage/派发计量不在收束处 (未知, 非 0)。
    pub fn pending(elapsed: Duration, view: &PendingApprovalView) -> Self {
        Self {
            outcome: TurnOutcomeKind::PendingApproval,
            latency_ms: latency_ms(elapsed),
            input_tokens: None,
            output_tokens: None,
            cache_hit_tokens: None,
            cache_hit_rate: None,
            tool_calls: None,
            rounds: Some(view.rounds_used),
            cost_micros: None,
        }
    }

    /// 失败收束拍: 除延迟与分类外收束处无在场计量。
    pub fn failed(elapsed: Duration) -> Self {
        Self {
            outcome: TurnOutcomeKind::Failed,
            latency_ms: latency_ms(elapsed),
            input_tokens: None,
            output_tokens: None,
            cache_hit_tokens: None,
            cache_hit_rate: None,
            tool_calls: None,
            rounds: None,
            cost_micros: None,
        }
    }
}

/// 一组数值的窗口摘要: 均值 / P50 / P95 / 计数 (口径: 最近秩)。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DistributionSummary {
    /// 参与统计的样本数 (无值拍不进计数)。
    pub count: u64,
    /// 算术均值 (count=0 时为 0;端点在空态改报 null)。
    pub mean: f64,
    /// 最近秩 P50。
    pub p50: u64,
    /// 最近秩 P95。
    pub p95: u64,
    /// 窗口内累计和。
    pub sum: u64,
}

impl DistributionSummary {
    /// 摘要一列有值样本 (未排序输入, 内部排序, 不改调用方数据)。
    pub fn of(values: &[u64]) -> Self {
        let count = values.len() as u64;
        if count == 0 {
            return Self::default();
        }
        let sum = values
            .iter()
            .fold(0u64, |acc, value| acc.saturating_add(*value));
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        Self {
            count,
            mean: sum as f64 / count as f64,
            p50: nearest_rank(&sorted, 0.50),
            p95: nearest_rank(&sorted, 0.95),
            sum,
        }
    }
}

/// 最近秩分位: 排序升序取 `ceil(p × n)` 位 (1-based);空集回 0。
fn nearest_rank(sorted: &[u64], percentile: f64) -> u64 {
    let Some(&first) = sorted.first() else {
        return 0;
    };
    let n = sorted.len() as f64;
    let rank = (percentile * n).ceil().clamp(1.0, n) as usize;
    sorted.get(rank - 1).copied().unwrap_or(first)
}

/// 一层 (会话或全局) 的滚动窗口聚合。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowAggregate {
    /// 窗口内收束拍计数 (= 成功 + 失败 + 待批)。
    pub turns: u64,
    /// 成功收束计数。
    pub succeeded: u64,
    /// 失败收束计数。
    pub failed: u64,
    /// 待批收束计数。
    pub pending_approval: u64,
    /// 回合延迟分布 (毫秒;全部收束拍都有延迟值)。
    pub latency_ms: DistributionSummary,
    /// 输入 token 分布 (仅收束处有 usage 计量的拍)。
    pub input_tokens: DistributionSummary,
    /// 输出 token 分布 (口径同上)。
    pub output_tokens: DistributionSummary,
    /// 轮数分布 (仅轮数已知的拍)。
    pub rounds: DistributionSummary,
    /// 工具派发数分布 (仅派发数已知的拍)。
    pub tool_calls: DistributionSummary,
    /// 缓存命中累计。`None` = 窗口内无任何缓存计量 (未知, "—")。
    pub cache_hit_tokens: Option<u64>,
    /// 缓存命中率 = Σ命中 / Σ输入 (命中计量拍)。`None` = 命中未知或输入为 0。
    pub cache_hit_rate: Option<f64>,
    /// 成本累计 (微单位)。`None` = 窗口内无计价拍。
    pub cost_micros: Option<u64>,
}

impl WindowAggregate {
    /// 聚合一环收束拍。
    fn of(samples: &VecDeque<TurnMetrics>) -> Self {
        let mut aggregate = Self {
            turns: samples.len() as u64,
            ..Self::default()
        };
        let mut latency = Vec::with_capacity(samples.len());
        let mut input = Vec::new();
        let mut output = Vec::new();
        let mut rounds = Vec::new();
        let mut tool_calls = Vec::new();
        let mut cache_hit_tokens = 0u64;
        let mut cache_hit_input = 0u64;
        let mut cache_known = false;
        let mut cost_micros = 0u64;
        let mut cost_known = false;
        for sample in samples {
            match sample.outcome {
                TurnOutcomeKind::Succeeded => aggregate.succeeded += 1,
                TurnOutcomeKind::Failed => aggregate.failed += 1,
                TurnOutcomeKind::PendingApproval => aggregate.pending_approval += 1,
            }
            latency.push(sample.latency_ms);
            if let Some(value) = sample.input_tokens {
                input.push(value);
            }
            if let Some(value) = sample.output_tokens {
                output.push(value);
            }
            if let Some(value) = sample.rounds {
                rounds.push(u64::from(value));
            }
            if let Some(value) = sample.tool_calls {
                tool_calls.push(u64::from(value));
            }
            // 命中率口径: 命中数与输入数同拍在场才进分子分母 (同源计量)。
            if let (Some(hit), Some(prompt)) = (sample.cache_hit_tokens, sample.input_tokens) {
                cache_known = true;
                cache_hit_tokens = cache_hit_tokens.saturating_add(hit);
                cache_hit_input = cache_hit_input.saturating_add(prompt);
            }
            if let Some(cost) = sample.cost_micros {
                cost_known = true;
                cost_micros = cost_micros.saturating_add(cost);
            }
        }
        aggregate.latency_ms = DistributionSummary::of(&latency);
        aggregate.input_tokens = DistributionSummary::of(&input);
        aggregate.output_tokens = DistributionSummary::of(&output);
        aggregate.rounds = DistributionSummary::of(&rounds);
        aggregate.tool_calls = DistributionSummary::of(&tool_calls);
        aggregate.cache_hit_tokens = cache_known.then_some(cache_hit_tokens);
        aggregate.cache_hit_rate = (cache_known && cache_hit_input > 0)
            .then(|| cache_hit_tokens as f64 / cache_hit_input as f64);
        aggregate.cost_micros = cost_known.then_some(cost_micros);
        aggregate
    }
}

/// 一个会话层的聚合与其归属。
#[derive(Debug, Clone, PartialEq)]
pub struct SessionAggregate {
    /// 会话 id。
    pub session: SessionId,
    /// 该会话的窗口聚合。
    pub aggregate: WindowAggregate,
}

/// 端点一次读走的原子快照 (单锁读取, 两层读数不撕裂)。
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
    /// 生效窗口大小 (拍/层)。
    pub window_turns: usize,
    /// 全局层聚合。
    pub global: WindowAggregate,
    /// 会话层聚合 (最近记录序, 仅含窗口内有拍的会话)。
    pub sessions: Vec<SessionAggregate>,
}

/// 一个会话环: 收束拍环形缓冲 + 最近记录序 (留存驱逐用)。
struct SessionRing {
    last_seq: u64,
    samples: VecDeque<TurnMetrics>,
}

/// 注册表内部状态 (单锁保护;读侧一次加锁取整张快照)。
#[derive(Default)]
struct MetricsState {
    seq: u64,
    global: VecDeque<TurnMetrics>,
    sessions: BTreeMap<SessionId, SessionRing>,
}

/// poison 容错锁 (L 组): 指标面是观测状态,持锁线程 panic 不应级联成之后
/// 所有读数的 panic (与 runtime 同口径)。
fn lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 会话 + 全局两层的进程内滚动窗口指标注册表。
///
/// 记入即入环,读侧聚合只看窗口内的拍。线程安全: 记入与快照读可以并发,
/// 快照读是单锁原子读 (两层读数一致)。
pub struct MetricsRegistry {
    window: usize,
    state: Mutex<MetricsState>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_METRICS_WINDOW_TURNS)
    }
}

impl std::fmt::Debug for MetricsRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsRegistry")
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

impl MetricsRegistry {
    /// 一个窗口为 `window_turns` 拍/层的注册表 (收口到 1..=
    /// [`MAX_METRICS_WINDOW_TURNS`])。
    pub fn new(window_turns: usize) -> Self {
        Self {
            window: window_turns.clamp(1, MAX_METRICS_WINDOW_TURNS),
            state: Mutex::new(MetricsState::default()),
        }
    }

    /// 生效窗口大小 (拍/层)。
    pub fn window_turns(&self) -> usize {
        self.window
    }

    /// 记入一个收束拍 (会话层 + 全局层各入一环)。
    pub fn record(&self, session: SessionId, metrics: TurnMetrics) {
        let mut state = lock_or_recover(&self.state);
        state.seq += 1;
        let seq = state.seq;
        push_bounded(&mut state.global, metrics.clone(), self.window);
        let ring = state
            .sessions
            .entry(session)
            .or_insert_with(|| SessionRing {
                last_seq: seq,
                samples: VecDeque::new(),
            });
        ring.last_seq = seq;
        push_bounded(&mut ring.samples, metrics, self.window);
        if state.sessions.len() > MAX_TRACKED_SESSIONS {
            let evictable = state
                .sessions
                .iter()
                .min_by_key(|(_, ring)| ring.last_seq)
                .map(|(id, _)| *id);
            if let Some(evictable) = evictable {
                state.sessions.remove(&evictable);
            }
        }
    }

    /// 回合收束记一拍: 成功/待批自 [`TurnOutcome`] 分类。
    pub fn record_turn(&self, session: SessionId, elapsed: Duration, outcome: &TurnOutcome) {
        let metrics = match outcome {
            TurnOutcome::Completed(response) => TurnMetrics::succeeded(elapsed, response),
            TurnOutcome::PendingApproval(view) => TurnMetrics::pending(elapsed, view),
        };
        self.record(session, metrics);
    }

    /// 失败收束记一拍。
    pub fn record_failure(&self, session: SessionId, elapsed: Duration) {
        self.record(session, TurnMetrics::failed(elapsed));
    }

    /// 回合收束结果记一拍: `Ok` 按 [`Self::record_turn`] 分类,`Err` 记失败拍。
    pub fn record_outcome(
        &self,
        session: SessionId,
        elapsed: Duration,
        result: &RuntimeResult<TurnOutcome>,
    ) {
        match result {
            Ok(outcome) => self.record_turn(session, elapsed, outcome),
            Err(_) => self.record_failure(session, elapsed),
        }
    }

    /// 审批续跑收束记一拍: `Resumed` 按 [`Self::record_turn`] 分类,`Err` 记
    /// 失败拍;非收束的审批生命周期结果 (未找到/过期/已决等) 不记拍 —— 与
    /// 收束事件的口径一致。
    pub fn record_resolution(
        &self,
        session: SessionId,
        elapsed: Duration,
        result: &RuntimeResult<ApprovalResolution>,
    ) {
        match result {
            Ok(ApprovalResolution::Resumed(outcome)) => self.record_turn(session, elapsed, outcome),
            Err(_) => self.record_failure(session, elapsed),
            Ok(_) => {}
        }
    }

    /// 会话层窗口聚合;会话无拍时 `None` (端点报显式空)。
    pub fn session_aggregate(&self, session: SessionId) -> Option<WindowAggregate> {
        let state = lock_or_recover(&self.state);
        state
            .sessions
            .get(&session)
            .map(|ring| WindowAggregate::of(&ring.samples))
    }

    /// 全局层窗口聚合 (空窗口为显式空聚合)。
    pub fn global_aggregate(&self) -> WindowAggregate {
        let state = lock_or_recover(&self.state);
        WindowAggregate::of(&state.global)
    }

    /// 两层一起读走的原子快照 (并发读安全: 聚合不撕裂)。
    pub fn snapshot(&self) -> MetricsSnapshot {
        let state = lock_or_recover(&self.state);
        MetricsSnapshot {
            window_turns: self.window,
            global: WindowAggregate::of(&state.global),
            sessions: state
                .sessions
                .iter()
                .map(|(session, ring)| SessionAggregate {
                    session: *session,
                    aggregate: WindowAggregate::of(&ring.samples),
                })
                .collect(),
        }
    }
}

/// 环形入拍: 超窗丢最旧。
fn push_bounded(ring: &mut VecDeque<TurnMetrics>, metrics: TurnMetrics, window: usize) {
    if ring.len() >= window {
        ring.pop_front();
    }
    ring.push_back(metrics);
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    use apeireth_core::kernel::{ApprovalId, CapabilityId, RequestId, Timestamp, TraceId};
    use apeireth_protocol::canonical::{NormalizedUsage, ToolCall};

    use super::super::error::RuntimeError;
    use super::super::trace::{ExecutionTrace, TraceEvent};
    use super::*;

    fn session() -> SessionId {
        SessionId::new()
    }

    fn usage_response(prompt: u32, completion: u32, rounds: u32) -> TurnResponse {
        let session = session();
        let request = RequestId::new();
        TurnResponse {
            session,
            request,
            text: "ok".into(),
            served_by: CapabilityId::new("provider.fake").unwrap(),
            usage: NormalizedUsage::new(prompt, completion),
            rounds,
            trace: ExecutionTrace::new(TraceId::new(), session, request),
        }
    }

    fn pending_view(rounds_used: u32) -> PendingApprovalView {
        let at = Timestamp::from_epoch_millis(1).unwrap();
        PendingApprovalView {
            approval_id: ApprovalId::new(),
            session_id: session(),
            request_id: RequestId::new(),
            trace_id: TraceId::new(),
            round: rounds_used,
            capability_id: CapabilityId::new("tool.gated").unwrap(),
            tool_name: "gated".into(),
            tool_call: ToolCall {
                id: "call_1".into(),
                name: "gated".into(),
                arguments: serde_json::json!({}),
            },
            effective_invocation: None,
            governance_hook: "policy".into(),
            governance_reason: "needs a human".into(),
            command_text: "gated".into(),
            arguments_summary: "{}".into(),
            authorization_families: Vec::new(),
            action_binding_digest: String::new(),
            created_at: at,
            expires_at: at,
            operation_fingerprint: "fp".into(),
            generated_text: String::new(),
            rounds_used,
        }
    }

    fn beat(outcome: TurnOutcomeKind, latency_ms: u64, input_tokens: Option<u64>) -> TurnMetrics {
        TurnMetrics {
            outcome,
            latency_ms,
            input_tokens,
            output_tokens: input_tokens.map(|value| value / 2),
            cache_hit_tokens: None,
            cache_hit_rate: None,
            tool_calls: Some(1),
            rounds: Some(1),
            cost_micros: None,
        }
    }

    /// 窗口聚合: 均值/P50/P95 按最近秩口径钉死 (样本 1..=10)。
    #[test]
    fn window_percentiles_and_mean_follow_nearest_rank() {
        let mut ring = VecDeque::new();
        for latency in 1..=10u64 {
            ring.push_back(beat(TurnOutcomeKind::Succeeded, latency, Some(100)));
        }
        let aggregate = WindowAggregate::of(&ring);
        assert_eq!(aggregate.latency_ms.count, 10);
        assert_eq!(aggregate.latency_ms.sum, 55);
        assert!((aggregate.latency_ms.mean - 5.5).abs() < 1e-9);
        assert_eq!(aggregate.latency_ms.p50, 5, "ceil(0.5×10)=5th");
        assert_eq!(aggregate.latency_ms.p95, 10, "ceil(0.95×10)=10th");
        // 只有有值拍进 token 列: 10 拍 × 100 输入。
        assert_eq!(aggregate.input_tokens.sum, 1000);
        assert_eq!(aggregate.input_tokens.count, 10);
    }

    /// 奇数窗口与小样本分位: 4 样本 p50=第2位, p95=第4位。
    #[test]
    fn percentiles_on_small_windows_take_the_ceiling_rank() {
        let summary = DistributionSummary::of(&[40, 10, 30, 20]);
        assert_eq!(summary.count, 4);
        assert_eq!(summary.sum, 100);
        assert!((summary.mean - 25.0).abs() < 1e-9);
        assert_eq!(summary.p50, 20);
        assert_eq!(summary.p95, 40);
    }

    /// 成功/失败/待批分计: 三类收束各记一拍, 计数互不串。
    #[test]
    fn success_failure_and_pending_beats_count_separately() {
        let registry = MetricsRegistry::default();
        let session = session();
        registry.record_outcome(
            session,
            Duration::from_millis(10),
            &Ok(TurnOutcome::Completed(usage_response(10, 5, 1))),
        );
        registry.record_outcome(
            session,
            Duration::from_millis(20),
            &Err(RuntimeError::misconfigured("denied")),
        );
        registry.record_outcome(
            session,
            Duration::from_millis(30),
            &Ok(TurnOutcome::PendingApproval(pending_view(2))),
        );
        let aggregate = registry.session_aggregate(session).expect("one session");
        assert_eq!(aggregate.turns, 3);
        assert_eq!(aggregate.succeeded, 1);
        assert_eq!(aggregate.failed, 1);
        assert_eq!(aggregate.pending_approval, 1);
        assert_eq!(aggregate.input_tokens.count, 1, "只有成功拍带 usage 计量");
        assert_eq!(aggregate.rounds.sum, 3, "成功拍 1 轮 + 待批拍 2 轮");
    }

    /// 审批生命周期的非收束结果 (未找到/过期) 不记拍, 与收束事件口径一致。
    #[test]
    fn non_closure_approval_results_record_no_beat() {
        let registry = MetricsRegistry::default();
        let session = session();
        registry.record_resolution(
            session,
            Duration::from_millis(5),
            &Ok(ApprovalResolution::NotFound),
        );
        registry.record_resolution(
            session,
            Duration::from_millis(5),
            &Ok(ApprovalResolution::Expired),
        );
        assert_eq!(registry.global_aggregate().turns, 0);
    }

    /// 会话层与全局层独立聚合: 两会话互不污染, 全局是两层之和。
    #[test]
    fn session_and_global_layers_aggregate_separately() {
        let registry = MetricsRegistry::default();
        let (a, b) = (session(), session());
        registry.record(a, beat(TurnOutcomeKind::Succeeded, 10, Some(100)));
        registry.record(a, beat(TurnOutcomeKind::Succeeded, 30, Some(200)));
        registry.record(b, beat(TurnOutcomeKind::Failed, 50, None));

        let first = registry.session_aggregate(a).expect("session a");
        assert_eq!(first.turns, 2);
        assert_eq!(first.latency_ms.sum, 40);
        assert_eq!(first.input_tokens.sum, 300);

        let second = registry.session_aggregate(b).expect("session b");
        assert_eq!(second.turns, 1);
        assert_eq!(second.failed, 1);
        assert_eq!(second.input_tokens.count, 0, "失败拍无 usage 计量不进计数");

        let global = registry.global_aggregate();
        assert_eq!(global.turns, 3);
        assert_eq!(global.succeeded, 2);
        assert_eq!(global.failed, 1);
        assert_eq!(registry.snapshot().sessions.len(), 2);
    }

    /// 窗口 = 最近 N 拍 (N 可配置, 默认 50): 超窗丢最旧。
    #[test]
    fn window_keeps_the_last_n_turns_and_is_configurable() {
        assert_eq!(MetricsRegistry::default().window_turns(), 50);
        assert_eq!(MetricsRegistry::new(0).window_turns(), 1, "0 收口到 1");
        let registry = MetricsRegistry::new(2);
        let session = session();
        for latency in 1..=3u64 {
            registry.record(session, beat(TurnOutcomeKind::Succeeded, latency, Some(1)));
        }
        let aggregate = registry.session_aggregate(session).expect("session");
        assert_eq!(aggregate.turns, 2);
        assert_eq!(aggregate.latency_ms.sum, 5, "只留最近两拍 (2+3)");
        assert_eq!(registry.global_aggregate().turns, 2);
    }

    /// 空窗口 = 显式空聚合 (计数 0, 分布计数 0), 不是缺字段。
    #[test]
    fn empty_window_is_an_explicit_empty_aggregate() {
        let registry = MetricsRegistry::default();
        let aggregate = registry.global_aggregate();
        assert_eq!(aggregate.turns, 0);
        assert_eq!(aggregate.succeeded, 0);
        assert_eq!(aggregate.latency_ms.count, 0);
        assert_eq!(
            registry.session_aggregate(session()),
            None,
            "无拍会话 = 显式空"
        );
        assert!(registry.snapshot().sessions.is_empty());
    }

    /// 未知列不冒充 0: 无缓存/计价计量的拍, 聚合如实报 None。
    #[test]
    fn unknown_cache_and_cost_columns_stay_unknown_not_zero() {
        let registry = MetricsRegistry::default();
        let session = session();
        registry.record(session, beat(TurnOutcomeKind::Succeeded, 10, Some(100)));
        let aggregate = registry.session_aggregate(session).expect("session");
        assert_eq!(aggregate.cache_hit_tokens, None);
        assert_eq!(aggregate.cache_hit_rate, None);
        assert_eq!(aggregate.cost_micros, None);
    }

    /// 命中率口径: Σ命中/Σ输入, 仅命中计量在场的拍进分子分母。
    #[test]
    fn cache_hit_rate_is_the_sum_of_hits_over_sum_of_input() {
        let mut ring = VecDeque::new();
        for (input, hit) in [(100u64, 25u64), (100, 75), (50, 0)] {
            ring.push_back(TurnMetrics {
                cache_hit_tokens: Some(hit),
                input_tokens: Some(input),
                ..beat(TurnOutcomeKind::Succeeded, 1, Some(input))
            });
        }
        let aggregate = WindowAggregate::of(&ring);
        assert_eq!(aggregate.cache_hit_tokens, Some(100));
        assert!(
            (aggregate.cache_hit_rate.expect("known") - (100.0 / 250.0)).abs() < 1e-9,
            "命中率 = Σ命中/Σ输入"
        );
    }

    /// 并发读安全: 多线程记入 + 多线程快照读, 结果不丢不撕 (每张快照内部
    /// 恒等式成立)。
    #[test]
    fn concurrent_recording_and_snapshots_stay_atomic() {
        const WRITERS: usize = 8;
        const PER_WRITER: usize = 64;
        let registry = Arc::new(MetricsRegistry::new(WRITERS * PER_WRITER));
        let observed = AtomicUsize::new(0);
        thread::scope(|scope| {
            for _ in 0..WRITERS {
                let registry = Arc::clone(&registry);
                scope.spawn(move || {
                    for round in 0..PER_WRITER {
                        let outcome = match round % 3 {
                            0 => TurnOutcomeKind::Succeeded,
                            1 => TurnOutcomeKind::Failed,
                            _ => TurnOutcomeKind::PendingApproval,
                        };
                        registry.record(session(), beat(outcome, round as u64, Some(1)));
                    }
                });
            }
            for _ in 0..4 {
                let registry = Arc::clone(&registry);
                let observed = &observed;
                scope.spawn(move || {
                    for _ in 0..200 {
                        let snapshot = registry.snapshot();
                        let global = &snapshot.global;
                        assert_eq!(
                            global.turns,
                            global.succeeded + global.failed + global.pending_approval,
                            "快照必须是原子读: 分类计数之和 = 总拍数"
                        );
                        observed.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        assert!(observed.load(Ordering::Relaxed) >= 800, "读者确实跑过");
        let global = registry.global_aggregate();
        assert_eq!(global.turns, (WRITERS * PER_WRITER) as u64, "一拍不丢");
        let expected_succeeded = (0..PER_WRITER).filter(|round| round % 3 == 0).count();
        assert_eq!(global.succeeded, (WRITERS * expected_succeeded) as u64);
    }

    /// 留存策略: 会话环有驻留上限, 超限驱逐最久未记录的会话 (观测面不成负担)。
    #[test]
    fn session_rings_evict_the_least_recently_recorded_at_the_cap() {
        let registry = MetricsRegistry::new(MAX_TRACKED_SESSIONS + 1);
        let first = session();
        registry.record(first, beat(TurnOutcomeKind::Succeeded, 1, Some(1)));
        for _ in 0..MAX_TRACKED_SESSIONS {
            registry.record(session(), beat(TurnOutcomeKind::Succeeded, 1, Some(1)));
        }
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.sessions.len(), MAX_TRACKED_SESSIONS, "驻留上限");
        assert!(
            snapshot.sessions.iter().all(|entry| entry.session != first),
            "最久未记录的会话环被驱逐"
        );
        assert_eq!(snapshot.global.turns, (MAX_TRACKED_SESSIONS + 1) as u64);
    }

    /// 成功拍的 usage 计量同源: 输入/输出/轮数/派发数取自回合记录。
    #[test]
    fn succeeded_beat_takes_token_and_round_columns_from_the_turn_record() {
        let mut response = usage_response(10, 5, 3);
        response.trace.record(
            Timestamp::from_epoch_millis(1).unwrap(),
            TraceEvent::CapabilityDispatched {
                capability: CapabilityId::new("tool.calculator").unwrap(),
                tool_call_id: "call_1".into(),
                round: 1,
            },
        );
        let metrics = TurnMetrics::succeeded(Duration::from_millis(7), &response);
        assert_eq!(metrics.outcome, TurnOutcomeKind::Succeeded);
        assert_eq!(metrics.latency_ms, 7);
        assert_eq!(metrics.input_tokens, Some(10));
        assert_eq!(metrics.output_tokens, Some(5));
        assert_eq!(metrics.rounds, Some(3));
        assert_eq!(metrics.tool_calls, Some(1));
        assert_eq!(metrics.cache_hit_tokens, None);
        assert_eq!(metrics.cost_micros, None);
    }

    /// CACHE 格取数口点亮 (G-P1 缺口①接入): 成功拍的缓存列取自 usage 计量的
    /// 缓存列**真值**, 命中率 = 命中/输入; 未报缓存列照旧 "—" (未知不当 0)。
    #[test]
    fn succeeded_beat_lights_the_cache_columns_from_the_usage_cache_column() {
        let mut response = usage_response(100, 5, 1);
        response.usage.prompt_cache_hit_tokens = Some(25);
        let metrics = TurnMetrics::succeeded(Duration::from_millis(1), &response);
        assert_eq!(metrics.cache_hit_tokens, Some(25));
        assert!(
            (metrics.cache_hit_rate.expect("known") - 0.25).abs() < 1e-9,
            "命中率 = 命中/输入"
        );

        // 未报缓存列 = 未知 (—), 不冒充 0。
        let unknown = usage_response(100, 5, 1);
        let metrics = TurnMetrics::succeeded(Duration::from_millis(1), &unknown);
        assert_eq!(metrics.cache_hit_tokens, None);
        assert_eq!(metrics.cache_hit_rate, None);

        // 命中已知但输入为 0: 命中率未知 (除零不编造)。
        let mut zero_input = usage_response(0, 5, 1);
        zero_input.usage.prompt_cache_hit_tokens = Some(10);
        let metrics = TurnMetrics::succeeded(Duration::from_millis(1), &zero_input);
        assert_eq!(metrics.cache_hit_tokens, Some(10));
        assert_eq!(metrics.cache_hit_rate, None);
    }

    /// CACHE 格端到端: usage 缓存列 → 成功拍 → 窗口聚合 (Σ命中/Σ输入 真值;
    /// 无缓存计量的拍不进分子分母, 同源计量口径)。
    #[test]
    fn cache_columns_flow_from_the_usage_cache_column_into_the_window_aggregate() {
        let registry = MetricsRegistry::default();
        let session = session();
        for (prompt, hit) in [(100u32, 25u32), (100, 75)] {
            let mut response = usage_response(prompt, 5, 1);
            response.usage.prompt_cache_hit_tokens = Some(hit);
            registry.record(
                session,
                TurnMetrics::succeeded(Duration::from_millis(1), &response),
            );
        }
        // 一拍未报缓存列: 不进分子分母。
        registry.record(
            session,
            TurnMetrics::succeeded(Duration::from_millis(1), &usage_response(50, 5, 1)),
        );
        let aggregate = registry.session_aggregate(session).expect("session");
        assert_eq!(aggregate.cache_hit_tokens, Some(100));
        assert!(
            (aggregate.cache_hit_rate.expect("known") - 0.5).abs() < 1e-9,
            "命中率 = Σ命中/Σ输入 = 100/200"
        );
    }
}
