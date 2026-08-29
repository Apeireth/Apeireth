//! P-arch (2026-08-27): A1 council + A2 team-lead + scene-d 例 3 Orchestrator.
//!
//! The Council service provides the typed advisor boundary and a bounded,
//! deterministic evaluation path. Provider calls are injected by the runtime
//! through a small adapter; this crate never owns a provider client.
//!
//! 借鉴 v1 `apeireth-team-lead` (14 调度工具) + `apeireth-council` (7 advisor
//! + 按住机制) + `apeireth-orchestrator` (plan/impl/review). **v2 形态**:
//! - `Advisor` trait (per-domain LLM 评审; 7 advisor 是 7 个具体 struct)
//! - `Council::decide(proposal) -> CouncilVerdict` (按住 + 多意见加权)
//! - `Orchestrator::dispatch(spec) -> SubagentOutcome` (调 subagent)
//! - `SubagentSpec` (plan/impl/review 等 role + 隔离 LLM 实例 + JSON protocol)
//!
//! **SelfAssessmentCache** (scene-d 例 2 触发) 移到 `apeireth-plugin::self_assessment` 模块
//! (canonical 单 source of truth, per 子代理审查 1.2, 2026-08-27). 本 crate 0 重复定义.
//!
//! **Runtime boundary**:
//! - `Council::decide` remains the compatibility path for local advisors.
//! - `Council::decide_with_invoker` runs bounded advisor side-calls with
//!   per-advisor and overall timeouts; the runtime supplies the invoker.
//! - `Orchestrator` trait 0 装: 真正调 subagent 需 runtime 介入 (subagent
//!   是 LLM factory 独立实例, 不在本 crate 范围)
//! - 全部 `pub use` 都是 re-export; 不引入新 LLM dep
//!
//! **架构原则**:
//! - trait 在 foundation (跟 plugin / governance / credentials 同级)
//! - impl 留 v2.0.0-rc (runtime 调 LLM, 不在本 crate)
//! - 与 v1 的 14 调度工具 API 1:1 对应, 但 trait 化让 impl 可换 (CLI / TUI / HTTP 都走同 trait)
//! - 多 instance 隔离 (per scene-d §3): subagent 必须是 LLM factory 独立实例, 禁共享 prompt
//!
//! 0 触碰 LOCKED: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline
//!   0 改; 新增 0 触碰现有任何 crate.

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::SessionId;
use async_trait::async_trait;
use futures::future::join_all;
use serde::{Deserialize, Serialize};

pub mod context_rot;
pub mod continuation;
pub mod council;
pub mod llm;

pub use context_rot::{
    apply_ops, compact_then_budget, extractive_summary, query_tokens, repetition_factor,
    rot_breakdown, rot_score, BudgetedBlock, CompactionOp, Compactor, DeterministicCompactor,
    RotBreakdown, RotConfig, Segment,
};
pub use continuation::{
    ContinuationSnapshot, ContinuationStore, EditAction, FileContinuationStore,
    InMemoryContinuationStore, PendingToolCall, SegmentEditor,
};

// ============================================
// Council (A1)
// ============================================

/// 7 个 advisor 领域 (per v1 apeireth-council + 设计意图)
/// 与 13 键原则洋葱的 S (价值层) 对应.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AdvisorKind {
    /// 安全性 (PII / 注入 / 凭据泄漏 / 自我禁用)
    Safety,
    /// 性能 (延迟 / 成本 / token budget)
    Performance,
    /// 哲学对齐 (13 键原则 / 三洋葱)
    Philosophy,
    /// 历史 (类似决策的历史结果)
    History,
    /// 策略 (主人长期目标)
    Strategy,
    /// 伦理 (主人显式表达的伦理边界)
    Ethics,
    /// 法律 (合规 / 区域)
    Legal,
}

/// Typed decision returned by one advisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisorDecision {
    /// The candidate is acceptable.
    Allow,
    /// The candidate should be regenerated.
    Retry,
    /// The candidate must not be accepted.
    Stop,
    /// The advisor cannot make a decision.
    Abstain,
}

/// Typed, bounded result returned by one advisor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisorVerdict {
    /// Normalized score in the inclusive range 0..=1.
    pub score: f64,
    /// Typed advisor decision.
    pub verdict: AdvisorDecision,
    /// Short actionable critique.
    pub critique: String,
    /// Optional advisor confidence in the inclusive range 0..=1.
    pub confidence: Option<f64>,
}

impl AdvisorVerdict {
    /// Construct a valid typed verdict.
    pub fn new(
        score: f64,
        verdict: AdvisorDecision,
        critique: impl Into<String>,
        confidence: Option<f64>,
    ) -> Result<Self, String> {
        let verdict = Self {
            score,
            verdict,
            critique: critique.into(),
            confidence,
        };
        verdict.validate()?;
        Ok(verdict)
    }

    /// Validate bounded advisor output before aggregation.
    pub fn validate(&self) -> Result<(), String> {
        if !self.score.is_finite() || !(0.0..=1.0).contains(&self.score) {
            return Err("advisor score must be finite and between 0 and 1".into());
        }
        if self
            .confidence
            .is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
        {
            return Err("advisor confidence must be finite and between 0 and 1".into());
        }
        if self.critique.chars().count() > 2_000 {
            return Err("advisor critique exceeds 2000 characters".into());
        }
        Ok(())
    }
}

/// Advisor trait: 每个领域 1 个 LLM 调用
#[async_trait::async_trait]
pub trait Advisor: Send + Sync {
    /// 稳定名
    fn name(&self) -> &'static str;
    /// 类型
    fn kind(&self) -> AdvisorKind;
    /// Local/compatibility evaluation path. Runtime-backed advisors use the
    /// injected [`CouncilInvoker`] path instead.
    async fn evaluate(&self, proposal: &Proposal) -> AdvisorVerdict;
}

/// 提案: 多领域评审对象 (per v1: 行动 / 提案 / 演化)
#[derive(Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// 提案 id
    pub id: String,
    /// 提交者 (subagent 或 runtime)
    pub proposer: String,
    /// 提案内容 (JSON 序列化, 0 装 不约束 schema)
    pub payload: serde_json::Value,
    /// 提交时间
    pub submitted_at: i64,
    /// 关联 session
    pub session_id: SessionId,
}

/// Council 多领域评审 (按住机制 + 加权投票)
pub struct Council {
    advisors: Vec<Arc<dyn Advisor>>,
    config: CouncilConfig,
}

/// Legacy Council verdict retained for compatibility with local advisors.
#[derive(Debug, Clone, PartialEq)]
pub enum CouncilVerdict {
    /// 通过 (多数 Allow, 无强反对)
    Approved,
    /// 拒绝 (含按住: 30% Advisor 反对 / 一致反对)
    Vetoed { by: AdvisorKind, reason: String },
    /// 需人工批准 (v2 治理的 Deny vs RequireApproval 区分, per ROADMAP P0)
    /// The runtime-backed path exposes the typed `CouncilDecision` instead.
    DeferToHuman { reason: String },
}

/// Typed aggregate decision emitted by the bounded Council path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilDecision {
    /// Keep the candidate and continue the canonical loop.
    Continue,
    /// Ask the canonical loop for another candidate.
    Retry,
    /// Reject the current candidate.
    Stop,
    /// The Council could not reach a safe decision.
    DeferToHuman,
}

/// One ordered advisor evaluation in a Council result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisorEvaluation {
    /// Stable advisor name.
    pub advisor: String,
    /// Advisor domain.
    pub kind: AdvisorKind,
    /// Typed advisor output.
    pub verdict: AdvisorVerdict,
}

/// A bounded advisor failure. The reason is returned to the caller but is not
/// emitted to low-cardinality telemetry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisorFailure {
    /// Stable advisor name.
    pub advisor: String,
    /// Advisor domain.
    pub kind: AdvisorKind,
    /// Stable failure category.
    pub category: String,
    /// Legible failure detail.
    pub reason: String,
}

/// Deterministic, typed Council aggregate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouncilResult {
    /// Mean score across valid, non-abstaining advisor verdicts.
    pub aggregate_score: f64,
    /// Aggregate control decision.
    pub decision: CouncilDecision,
    /// Advisors that returned a non-abstaining valid result.
    pub supporting_advisors: Vec<String>,
    /// Results in advisor registration order, never task completion order.
    pub evaluations: Vec<AdvisorEvaluation>,
    /// Per-advisor failures in advisor registration order.
    pub failures: Vec<AdvisorFailure>,
    /// Number of invocations attempted by the bounded batch.
    pub side_call_count: usize,
    /// Whether the overall Council deadline elapsed.
    pub timed_out: bool,
}

impl CouncilResult {
    /// Short feedback suitable for the canonical retry directive.
    pub fn retry_feedback(&self) -> String {
        let mut feedback = self
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.verdict.verdict == AdvisorDecision::Retry)
            .map(|evaluation| evaluation.verdict.critique.as_str())
            .filter(|critique| !critique.trim().is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        if feedback.is_empty() {
            feedback = "Council majority requested another candidate".into();
        }
        feedback.chars().take(2_000).collect()
    }
}

/// Failure returned by an injected Council side-call adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CouncilCallError {
    /// Provider/runtime side-call failed.
    Provider(String),
    /// The advisor response was not valid typed JSON.
    Malformed(String),
}

impl std::fmt::Display for CouncilCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(reason) => write!(f, "provider: {reason}"),
            Self::Malformed(reason) => write!(f, "malformed: {reason}"),
        }
    }
}

impl std::error::Error for CouncilCallError {}

/// Runtime adapter for Council advisor side-calls.
#[async_trait]
pub trait CouncilInvoker: Send + Sync {
    /// Invoke one advisor through the canonical runtime-owned side-call path.
    async fn invoke(
        &self,
        advisor: Arc<dyn Advisor>,
        proposal: &Proposal,
    ) -> Result<AdvisorVerdict, CouncilCallError>;
}

/// Bounded Council execution settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CouncilConfig {
    /// Maximum number of registered advisors evaluated for one proposal.
    pub max_advisors: usize,
    /// Deadline for one advisor side-call.
    pub per_advisor_timeout: Duration,
    /// Deadline for the whole Council batch.
    pub overall_timeout: Duration,
}

impl Default for CouncilConfig {
    fn default() -> Self {
        Self {
            max_advisors: 7,
            per_advisor_timeout: Duration::from_secs(10),
            overall_timeout: Duration::from_secs(60),
        }
    }
}

impl CouncilConfig {
    fn normalized(self) -> Self {
        Self {
            max_advisors: self.max_advisors.max(1),
            per_advisor_timeout: self.per_advisor_timeout.max(Duration::from_millis(1)),
            overall_timeout: self.overall_timeout.max(Duration::from_millis(1)),
        }
    }
}

impl Council {
    /// Seven deterministic advisors for compatibility and no-provider tests.
    pub fn default_allow() -> Self {
        Self {
            advisors: vec![
                Arc::new(NoopAdvisor::new(AdvisorKind::Safety)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Performance)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Philosophy)),
                Arc::new(NoopAdvisor::new(AdvisorKind::History)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Strategy)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Ethics)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Legal)),
            ],
            config: CouncilConfig::default(),
        }
    }

    /// Seven named LLM advisor slots. The runtime supplies the actual
    /// provider side-call through [`CouncilInvoker`].
    pub fn default_llm() -> Self {
        Self {
            advisors: vec![
                Arc::new(SlotAdvisor::new(AdvisorKind::Safety)),
                Arc::new(SlotAdvisor::new(AdvisorKind::Performance)),
                Arc::new(SlotAdvisor::new(AdvisorKind::Philosophy)),
                Arc::new(SlotAdvisor::new(AdvisorKind::History)),
                Arc::new(SlotAdvisor::new(AdvisorKind::Strategy)),
                Arc::new(SlotAdvisor::new(AdvisorKind::Ethics)),
                Arc::new(SlotAdvisor::new(AdvisorKind::Legal)),
            ],
            config: CouncilConfig::default(),
        }
    }

    /// Construct the seven real LLM advisor slots for the compatibility API.
    /// Each advisor gets its own provider instance when evaluated.
    pub fn with_factory(factory: Arc<dyn llm::LlmFactory>, model: impl Into<String>) -> Self {
        let model = model.into();
        Self::new(council::default_seven_advisors(factory, &model))
    }

    /// 自定义 advisors
    pub fn new(advisors: Vec<Arc<dyn Advisor>>) -> Self {
        Self {
            advisors,
            config: CouncilConfig::default(),
        }
    }

    /// Override the bounded Council settings without introducing a separate
    /// runtime or configuration framework.
    #[must_use]
    pub fn with_config(mut self, config: CouncilConfig) -> Self {
        self.config = config.normalized();
        self
    }

    /// Borrow the registered advisors for inspection and integration tests.
    pub fn advisors(&self) -> &[Arc<dyn Advisor>] {
        &self.advisors
    }

    /// Compatibility path for callers that own local `Advisor` implementations.
    /// It evaluates the bounded advisor batch in parallel and maps the typed
    /// results to the historical `CouncilVerdict`; runtime-backed side-calls
    /// should use [`Self::decide_with_invoker`] when failure categories and
    /// ordered evidence are needed.
    pub async fn decide(&self, proposal: &Proposal) -> CouncilVerdict {
        let per_advisor_timeout = self.config.per_advisor_timeout;
        let batch = join_all(self.advisors.iter().take(self.config.max_advisors).map(
            |advisor| async move {
                let result =
                    tokio::time::timeout(per_advisor_timeout, advisor.evaluate(proposal)).await;
                (advisor, result)
            },
        ));
        let Ok(completed) = tokio::time::timeout(self.config.overall_timeout, batch).await else {
            return CouncilVerdict::DeferToHuman {
                reason: format!(
                    "overall Council timeout elapsed after {} seconds",
                    self.config.overall_timeout.as_secs()
                ),
            };
        };

        let mut evaluations = Vec::with_capacity(completed.len());
        for (advisor, result) in completed {
            let Ok(verdict) = result else {
                return CouncilVerdict::DeferToHuman {
                    reason: format!("advisor {} timeout elapsed", advisor.name()),
                };
            };
            if let Err(reason) = verdict.validate() {
                return CouncilVerdict::DeferToHuman {
                    reason: format!(
                        "advisor {} returned malformed verdict: {reason}",
                        advisor.name()
                    ),
                };
            }
            evaluations.push(AdvisorEvaluation {
                advisor: advisor.name().into(),
                kind: advisor.kind(),
                verdict,
            });
        }
        Self::legacy_verdict(&evaluations)
    }

    /// Run advisor side-calls through an injected runtime adapter.
    ///
    /// Futures are bounded by `max_advisors`, each call has its own deadline,
    /// and the aggregate has a hard overall deadline. Results are sorted back
    /// into registration order before aggregation, so completion order cannot
    /// change the decision.
    pub async fn decide_with_invoker(
        &self,
        proposal: &Proposal,
        invoker: &dyn CouncilInvoker,
    ) -> CouncilResult {
        let selected = self
            .advisors
            .iter()
            .take(self.config.max_advisors)
            .enumerate()
            .map(|(index, advisor)| (index, Arc::clone(advisor)))
            .collect::<Vec<_>>();
        let attempted = Arc::new(AtomicUsize::new(0));
        let per_advisor_timeout = self.config.per_advisor_timeout;
        let attempted_for_batch = Arc::clone(&attempted);
        let batch = async move {
            join_all(selected.into_iter().map(|(index, advisor)| {
                let attempted = Arc::clone(&attempted_for_batch);
                async move {
                    attempted.fetch_add(1, Ordering::Relaxed);
                    let result = tokio::time::timeout(
                        per_advisor_timeout,
                        invoker.invoke(Arc::clone(&advisor), proposal),
                    )
                    .await;
                    (index, advisor, result)
                }
            }))
            .await
        };

        let batch_result = tokio::time::timeout(self.config.overall_timeout, batch).await;
        let attempted_side_calls = attempted.load(Ordering::Relaxed);
        let Ok(mut completed) = batch_result else {
            return CouncilResult {
                aggregate_score: 0.0,
                decision: CouncilDecision::DeferToHuman,
                supporting_advisors: Vec::new(),
                evaluations: Vec::new(),
                failures: vec![AdvisorFailure {
                    advisor: "council".into(),
                    kind: AdvisorKind::Safety,
                    category: "overall_timeout".into(),
                    reason: "overall Council timeout elapsed".into(),
                }],
                side_call_count: attempted_side_calls,
                timed_out: true,
            };
        };

        completed.sort_by_key(|(index, _, _)| *index);
        let mut evaluations = Vec::new();
        let mut failures = Vec::new();
        for (_, advisor, result) in completed {
            match result {
                Ok(Ok(verdict)) => match verdict.validate() {
                    Ok(()) => evaluations.push(AdvisorEvaluation {
                        advisor: advisor.name().into(),
                        kind: advisor.kind(),
                        verdict,
                    }),
                    Err(reason) => failures.push(AdvisorFailure {
                        advisor: advisor.name().into(),
                        kind: advisor.kind(),
                        category: "malformed".into(),
                        reason,
                    }),
                },
                Ok(Err(error)) => failures.push(AdvisorFailure {
                    advisor: advisor.name().into(),
                    kind: advisor.kind(),
                    category: match error {
                        CouncilCallError::Provider(_) => "provider_failure",
                        CouncilCallError::Malformed(_) => "malformed",
                    }
                    .into(),
                    reason: error.to_string(),
                }),
                Err(_) => failures.push(AdvisorFailure {
                    advisor: advisor.name().into(),
                    kind: advisor.kind(),
                    category: "advisor_timeout".into(),
                    reason: "advisor timeout elapsed".into(),
                }),
            }
        }

        Self::aggregate(evaluations, failures, attempted_side_calls, false)
    }

    fn aggregate(
        evaluations: Vec<AdvisorEvaluation>,
        failures: Vec<AdvisorFailure>,
        side_call_count: usize,
        timed_out: bool,
    ) -> CouncilResult {
        let decisive = evaluations
            .iter()
            .filter(|evaluation| evaluation.verdict.verdict != AdvisorDecision::Abstain)
            .collect::<Vec<_>>();
        let aggregate_score = if decisive.is_empty() {
            0.0
        } else {
            decisive
                .iter()
                .map(|evaluation| evaluation.verdict.score)
                .sum::<f64>()
                / decisive.len() as f64
        };
        let stop = decisive
            .iter()
            .find(|evaluation| evaluation.verdict.verdict == AdvisorDecision::Stop);
        let retries = decisive
            .iter()
            .filter(|evaluation| evaluation.verdict.verdict == AdvisorDecision::Retry)
            .count();
        let allows = decisive
            .iter()
            .filter(|evaluation| evaluation.verdict.verdict == AdvisorDecision::Allow)
            .count();
        let decision = if stop.is_some() {
            CouncilDecision::Stop
        } else if retries > allows {
            CouncilDecision::Retry
        } else if decisive.is_empty() && !failures.is_empty() {
            CouncilDecision::DeferToHuman
        } else {
            CouncilDecision::Continue
        };
        let supporting_advisors = decisive
            .iter()
            .map(|evaluation| evaluation.advisor.clone())
            .collect();
        CouncilResult {
            aggregate_score,
            decision,
            supporting_advisors,
            evaluations,
            failures,
            side_call_count,
            timed_out,
        }
    }

    fn legacy_verdict(evaluations: &[AdvisorEvaluation]) -> CouncilVerdict {
        if let Some(veto) = evaluations.iter().find(|evaluation| {
            matches!(
                evaluation.verdict.verdict,
                AdvisorDecision::Stop | AdvisorDecision::Retry
            )
        }) {
            return CouncilVerdict::Vetoed {
                by: veto.kind,
                reason: veto.verdict.critique.clone(),
            };
        }
        CouncilVerdict::Approved
    }
}

/// Noop advisor (v2.0 alpha 0 装)
struct NoopAdvisor {
    kind: AdvisorKind,
}

impl NoopAdvisor {
    fn new(kind: AdvisorKind) -> Self {
        Self { kind }
    }
}

#[async_trait::async_trait]
impl Advisor for NoopAdvisor {
    fn name(&self) -> &'static str {
        match self.kind {
            AdvisorKind::Safety => "noop_safety",
            AdvisorKind::Performance => "noop_performance",
            AdvisorKind::Philosophy => "noop_philosophy",
            AdvisorKind::History => "noop_history",
            AdvisorKind::Strategy => "noop_strategy",
            AdvisorKind::Ethics => "noop_ethics",
            AdvisorKind::Legal => "noop_legal",
        }
    }

    fn kind(&self) -> AdvisorKind {
        self.kind
    }

    async fn evaluate(&self, _proposal: &Proposal) -> AdvisorVerdict {
        AdvisorVerdict::new(1.0, AdvisorDecision::Allow, "noop allow", Some(1.0))
            .expect("static noop verdict is valid")
    }
}

/// A named LLM advisor slot. It is intentionally provider-free; the runtime
/// adapter turns the slot into one isolated ModuleInvoker request.
struct SlotAdvisor {
    kind: AdvisorKind,
}

impl SlotAdvisor {
    fn new(kind: AdvisorKind) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Advisor for SlotAdvisor {
    fn name(&self) -> &'static str {
        match self.kind {
            AdvisorKind::Safety => "llm_safety",
            AdvisorKind::Performance => "llm_performance",
            AdvisorKind::Philosophy => "llm_philosophy",
            AdvisorKind::History => "llm_history",
            AdvisorKind::Strategy => "llm_strategy",
            AdvisorKind::Ethics => "llm_ethics",
            AdvisorKind::Legal => "llm_legal",
        }
    }

    fn kind(&self) -> AdvisorKind {
        self.kind
    }

    async fn evaluate(&self, _proposal: &Proposal) -> AdvisorVerdict {
        AdvisorVerdict::new(
            0.0,
            AdvisorDecision::Abstain,
            "runtime invoker required",
            None,
        )
        .expect("static slot verdict is valid")
    }
}

// ============================================
// Orchestrator (A2 + scene-d 例 3)
// ============================================

/// Subagent 角色 (v1 三件套 + 扩展)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubagentRole {
    /// 规划 (把长程任务拆 plan)
    Planner,
    /// 实施 (按 plan 写代码)
    Implementer,
    /// 评审 (互审核心: 独立 LLM 评审其他 agent 输出)
    Reviewer,
    /// 测试 (Plan/Impl 后跑测试)
    Tester,
    /// 文档
    Documenter,
}

/// Subagent 规格 (per v1 team-lead 14 调度工具, trait 化)
#[derive(Clone, Serialize, Deserialize)]
pub struct SubagentSpec {
    /// 唯一 id
    pub id: String,
    /// 角色
    pub role: SubagentRole,
    /// 任务标题
    pub title: String,
    /// 任务详细 (JSON 序列化)
    pub payload: serde_json::Value,
    /// 用哪个 model (per scene-d §5 决策 1: 同 provider 不同 model 隔离)
    /// None = provider 默认 model
    pub model: Option<String>,
    /// 隔离的 system prompt
    pub system_prompt: Option<String>,
    /// 最多允许的 governance 决策: RequireApproval 自动 deny
    pub require_human_approval: bool,
}

/// Subagent 输出结果
#[derive(Clone, Serialize, Deserialize)]
pub struct SubagentOutcome {
    /// 对应 spec.id
    pub spec_id: String,
    /// 角色
    pub role: SubagentRole,
    /// 输出 (JSON 序列化, 角色-specific)
    pub output: serde_json::Value,
    /// 是否成功
    pub success: bool,
    /// 错误信息 (success=false 时)
    pub error: Option<String>,
    /// 完成时间
    pub completed_at: i64,
}

/// 互审结果 (scene-d 例 3)
#[derive(Clone, Serialize, Deserialize)]
pub struct ReviewVerdict {
    /// 评审的 subagent output id
    pub reviewed_spec_id: String,
    /// 通过 / 需返工
    pub passed: bool,
    /// 评分 (0.0-1.0)
    pub score: f64,
    /// 阻塞性问题 (passed=false 时必填)
    pub blocking_issues: Vec<String>,
    /// 可选建议 (passed=true 也可填)
    pub suggestions: Vec<String>,
    /// 评审者 subagent id
    pub reviewer_id: String,
}

/// Orchestrator trait: 调度 subagent (per v1 team-lead 14 工具 trait 化)
#[async_trait::async_trait]
pub trait Orchestrator: Send + Sync {
    /// 启动 orchestrator (装配期)
    async fn start(&mut self) -> Result<(), OrchestratorError>;
    /// 停止
    async fn stop(&mut self) -> Result<(), OrchestratorError>;
    /// 分发 subagent (async 调 LLM factory 独立实例, scene-d §3 多 instance 隔离)
    async fn dispatch(&self, spec: SubagentSpec) -> Result<SubagentOutcome, OrchestratorError>;
    /// 长程任务: plan + impl + review 串联
    async fn orchestrate(
        &self,
        title: String,
        payload: serde_json::Value,
    ) -> Result<Vec<SubagentOutcome>, OrchestratorError> {
        // 默认实现: planner -> implementer -> reviewer 三步
        // clone payload 防 partial move (serde_json::Value not Copy)
        let plan = self
            .dispatch(SubagentSpec {
                id: format!("{title}-plan"),
                role: SubagentRole::Planner,
                title: format!("{title} (plan)"),
                payload: payload.clone(),
                model: None,
                system_prompt: None,
                require_human_approval: true, // 重大任务先给主人看 plan
            })
            .await?;
        let plan_output = plan.output.clone();
        let impl_outcome = self
            .dispatch(SubagentSpec {
                id: format!("{title}-impl"),
                role: SubagentRole::Implementer,
                title: format!("{title} (impl)"),
                payload: plan_output,
                model: None,
                system_prompt: None,
                require_human_approval: false,
            })
            .await?;
        let impl_output = impl_outcome.output.clone();
        let review = self
            .dispatch(SubagentSpec {
                id: format!("{title}-review"),
                role: SubagentRole::Reviewer,
                title: format!("{title} (review)"),
                payload: impl_output,
                model: None, // 关键: reviewer 必须用 **不同 model** (scene-d §5 决策 1)
                system_prompt: None,
                require_human_approval: false,
            })
            .await?;
        Ok(vec![plan, impl_outcome, review])
    }
}

/// Orchestrator 错误
#[derive(Debug)]
pub enum OrchestratorError {
    /// 0 装: subagent 启动失败
    SubagentFailed(String),
    /// 0 装: 主人审批拒绝
    HumanDenied(String),
    /// governance 拒绝 (runtime 拒绝给 subagent 该 capability)
    GovernanceDenied(String),
    /// IO 错误
    Io(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubagentFailed(msg) => write!(f, "subagent failed: {msg}"),
            Self::HumanDenied(msg) => write!(f, "human denied: {msg}"),
            Self::GovernanceDenied(msg) => write!(f, "governance denied: {msg}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

// Send / Sync 由编译器自动派生: 字段全 String, 满足 Send + Sync 自动推导条件.
// crate 内 #![forbid(unsafe_code)] 不允许 unsafe impl, 但自动派生无需 unsafe impl.
// RC-5 runtime 集成时跨任务传输安全.

// SelfAssessmentCache (scene-d 例 2) **0 在此 crate** (per 子代理审查 1.2):
// 之前这里有 `SelfAssessment` / `Deviation` / `SelfAssessmentCache` 同名不同定义
// (plugin::SelfAssessment 字段多, 含 id + reviewer_id, 比本处的字段全).
// **单一 source of truth**: plugin::self_assessment::SelfAssessment 是 canonical
// 类型, plugin::self_assessment::SelfAssessmentStore 是 trait.
// Orchestration 改为注入 `Arc<dyn SelfAssessmentStore>`, 0 重复定义.

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    fn sample_proposal() -> Proposal {
        Proposal {
            id: "p-1".into(),
            proposer: "runtime".into(),
            payload: serde_json::json!({"action": "deploy"}),
            submitted_at: 1_700_000_000,
            session_id: SessionId::new(),
        }
    }

    #[tokio::test]
    async fn council_default_approves_everything() {
        let c = Council::default_allow();
        let v = c.decide(&sample_proposal()).await;
        assert_eq!(v, CouncilVerdict::Approved);
    }

    #[tokio::test]
    async fn council_vetoes_on_any_deny() {
        struct DenyAll;
        #[async_trait::async_trait]
        impl Advisor for DenyAll {
            fn name(&self) -> &'static str {
                "deny_all"
            }
            fn kind(&self) -> AdvisorKind {
                AdvisorKind::Safety
            }
            async fn evaluate(&self, _: &Proposal) -> AdvisorVerdict {
                AdvisorVerdict::new(0.1, AdvisorDecision::Stop, "test veto", Some(1.0)).unwrap()
            }
        }
        let c = Council::new(vec![
            Arc::new(DenyAll),
            Arc::new(NoopAdvisor::new(AdvisorKind::Safety)),
        ]);
        match c.decide(&sample_proposal()).await {
            CouncilVerdict::Vetoed { by, reason } => {
                assert_eq!(by, AdvisorKind::Safety);
                assert!(reason.contains("test veto"));
            }
            _ => panic!("expected Vetoed"),
        }
    }

    // 注: SelfAssessment / SelfAssessmentCache 测试在 `apeireth-plugin` crate 的
    // `self_assessment` 模块 (canonical 单 source of truth), 不在本 crate 重复.

    #[test]
    fn subagent_spec_roundtrip_json() {
        let spec = SubagentSpec {
            id: "spec-1".into(),
            role: SubagentRole::Planner,
            title: "design".into(),
            payload: serde_json::json!({"x": 1}),
            model: Some("anthropic/claude-3-5".into()),
            system_prompt: Some("you are planner".into()),
            require_human_approval: true,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let back: SubagentSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, spec.id);
        assert_eq!(back.role, spec.role);
        assert_eq!(back.require_human_approval, true);
    }

    #[test]
    fn review_verdict_roundtrip_json() {
        let v = ReviewVerdict {
            reviewed_spec_id: "impl-1".into(),
            passed: false,
            score: 0.4,
            blocking_issues: vec!["missing test".into()],
            suggestions: vec!["add unit test".into()],
            reviewer_id: "reviewer-A".into(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: ReviewVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(back.passed, false);
        assert_eq!(back.score, 0.4);
        assert_eq!(back.blocking_issues, vec!["missing test".to_string()]);
    }

    struct ScriptedAdvisor {
        name: &'static str,
        kind: AdvisorKind,
    }

    #[async_trait]
    impl Advisor for ScriptedAdvisor {
        fn name(&self) -> &'static str {
            self.name
        }

        fn kind(&self) -> AdvisorKind {
            self.kind
        }

        async fn evaluate(&self, _proposal: &Proposal) -> AdvisorVerdict {
            AdvisorVerdict::new(1.0, AdvisorDecision::Allow, "compat", Some(1.0)).unwrap()
        }
    }

    struct ScriptedInvoker {
        decisions:
            std::collections::BTreeMap<AdvisorKind, Result<AdvisorVerdict, CouncilCallError>>,
        delays: std::collections::BTreeMap<AdvisorKind, Duration>,
    }

    #[async_trait]
    impl CouncilInvoker for ScriptedInvoker {
        async fn invoke(
            &self,
            advisor: Arc<dyn Advisor>,
            _proposal: &Proposal,
        ) -> Result<AdvisorVerdict, CouncilCallError> {
            if let Some(delay) = self.delays.get(&advisor.kind()) {
                tokio::time::sleep(*delay).await;
            }
            self.decisions
                .get(&advisor.kind())
                .cloned()
                .unwrap_or_else(|| Err(CouncilCallError::Provider("missing script".into())))
        }
    }

    fn council_with_kinds(kinds: &[AdvisorKind]) -> Council {
        Council::new(
            kinds
                .iter()
                .enumerate()
                .map(|(index, kind)| {
                    Arc::new(ScriptedAdvisor {
                        name: Box::leak(format!("advisor-{index}").into_boxed_str()),
                        kind: *kind,
                    }) as Arc<dyn Advisor>
                })
                .collect(),
        )
    }

    fn verdict(decision: AdvisorDecision, score: f64) -> AdvisorVerdict {
        AdvisorVerdict::new(score, decision, "script", Some(0.9)).unwrap()
    }

    #[tokio::test]
    async fn bounded_council_aggregates_in_registration_order() {
        let council = council_with_kinds(&[
            AdvisorKind::Safety,
            AdvisorKind::Performance,
            AdvisorKind::Philosophy,
        ])
        .with_config(CouncilConfig {
            max_advisors: 3,
            per_advisor_timeout: Duration::from_millis(100),
            overall_timeout: Duration::from_secs(1),
        });
        let invoker = ScriptedInvoker {
            decisions: [
                (
                    AdvisorKind::Safety,
                    Ok(verdict(AdvisorDecision::Allow, 0.9)),
                ),
                (
                    AdvisorKind::Performance,
                    Ok(verdict(AdvisorDecision::Allow, 0.8)),
                ),
                (
                    AdvisorKind::Philosophy,
                    Ok(verdict(AdvisorDecision::Allow, 0.7)),
                ),
            ]
            .into_iter()
            .collect(),
            delays: [
                (AdvisorKind::Safety, Duration::from_millis(20)),
                (AdvisorKind::Performance, Duration::from_millis(1)),
                (AdvisorKind::Philosophy, Duration::from_millis(10)),
            ]
            .into_iter()
            .collect(),
        };
        let result = council
            .decide_with_invoker(&sample_proposal(), &invoker)
            .await;
        assert_eq!(result.decision, CouncilDecision::Continue);
        assert_eq!(result.side_call_count, 3);
        assert_eq!(
            result
                .evaluations
                .iter()
                .map(|evaluation| evaluation.kind)
                .collect::<Vec<_>>(),
            vec![
                AdvisorKind::Safety,
                AdvisorKind::Performance,
                AdvisorKind::Philosophy
            ]
        );
        assert!((result.aggregate_score - 0.8).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn council_decision_semantics_cover_retry_stop_and_failures() {
        let proposal = sample_proposal();
        let retry_council = council_with_kinds(&[
            AdvisorKind::Safety,
            AdvisorKind::Performance,
            AdvisorKind::Philosophy,
        ]);
        let retry_invoker = ScriptedInvoker {
            decisions: [
                (
                    AdvisorKind::Safety,
                    Ok(verdict(AdvisorDecision::Retry, 0.3)),
                ),
                (
                    AdvisorKind::Performance,
                    Ok(verdict(AdvisorDecision::Retry, 0.4)),
                ),
                (
                    AdvisorKind::Philosophy,
                    Ok(verdict(AdvisorDecision::Allow, 0.8)),
                ),
            ]
            .into_iter()
            .collect(),
            delays: Default::default(),
        };
        let result = retry_council
            .decide_with_invoker(&proposal, &retry_invoker)
            .await;
        assert_eq!(result.decision, CouncilDecision::Retry);

        let stop_council = council_with_kinds(&[AdvisorKind::Safety, AdvisorKind::Performance]);
        let stop_invoker = ScriptedInvoker {
            decisions: [
                (
                    AdvisorKind::Safety,
                    Ok(verdict(AdvisorDecision::Allow, 1.0)),
                ),
                (
                    AdvisorKind::Performance,
                    Ok(verdict(AdvisorDecision::Stop, 0.0)),
                ),
            ]
            .into_iter()
            .collect(),
            delays: Default::default(),
        };
        let result = stop_council
            .decide_with_invoker(&proposal, &stop_invoker)
            .await;
        assert_eq!(result.decision, CouncilDecision::Stop);

        let failure_council = council_with_kinds(&[AdvisorKind::Safety]);
        let failure_invoker = ScriptedInvoker {
            decisions: [(
                AdvisorKind::Safety,
                Err(CouncilCallError::Malformed("bad json".into())),
            )]
            .into_iter()
            .collect(),
            delays: Default::default(),
        };
        let result = failure_council
            .decide_with_invoker(&proposal, &failure_invoker)
            .await;
        assert_eq!(result.decision, CouncilDecision::DeferToHuman);
        assert_eq!(result.failures[0].category, "malformed");
    }

    #[tokio::test]
    async fn council_per_advisor_timeout_is_fail_open_to_defer_when_all_timeout() {
        let council = council_with_kinds(&[AdvisorKind::Safety]).with_config(CouncilConfig {
            max_advisors: 1,
            per_advisor_timeout: Duration::from_millis(5),
            overall_timeout: Duration::from_secs(1),
        });
        let invoker = ScriptedInvoker {
            decisions: [(
                AdvisorKind::Safety,
                Ok(verdict(AdvisorDecision::Allow, 1.0)),
            )]
            .into_iter()
            .collect(),
            delays: [(AdvisorKind::Safety, Duration::from_millis(50))]
                .into_iter()
                .collect(),
        };
        let result = council
            .decide_with_invoker(&sample_proposal(), &invoker)
            .await;
        assert_eq!(result.decision, CouncilDecision::DeferToHuman);
        assert_eq!(result.failures[0].category, "advisor_timeout");
    }
}
