//! The five-stage tool execution pipeline.
//!
//! Policy and execution used to be entangled: whatever a call site checked,
//! in whatever order, decided what ran and what the model was told. This
//! pipeline makes the chain explicit: one call walks five stages:
//!
//! 1. [`ExecutionStage::PreExecute`] — the pre-execute waterfall: policy hooks
//!    answer `allow` / `deny` / `cancel` / `ask`, first non-allow wins;
//! 2. [`ExecutionStage::Guard`] — the monotonic guard set: guards may only
//!    refuse, never allow, and a refusal cannot be flipped back;
//! 3. [`ExecutionStage::Around`] — timeout (deadline timer library) and retry
//!    wrap the execution (see [`around`]);
//! 4. [`ExecutionStage::PostExecute`] — the correction channel: `accept` /
//!    `replace` / `block`, superseded results archived (see [`post`]);
//! 5. [`ExecutionStage::Normalize`] — the output contract: the result is
//!    frozen into [`ToolOutcome`] and checked against the declared schema.
//!
//! The stage order is a property of the pipeline, not of its callers: the
//! record built along the way lists the stages actually visited, in order.
//!
//! # Default behaviour is unchanged behaviour
//!
//! A pipeline with no hooks, guards, deadline, retry, or schema runs the tool
//! once and emits its result untouched. Every stage is opt-in.

pub mod around;
pub mod contract;
pub mod post;
pub mod pre;

pub use around::{AroundPolicy, RetryPolicy, DEFAULT_MAX_TIMEOUT_MS};
pub use contract::{NormalizationError, OutputSchema, SchemaField, SchemaKind, ToolOutcome};
pub use post::{
    PostDecision, PostExecuteHook, PostExecuteRequest, PostExecuteWaterfall, PostVerdict,
    SupersededResult,
};
pub use pre::{
    CommandFamilyGateHook, PreDecision, PreExecuteHook, PreExecuteRequest, PreExecuteWaterfall,
    PreVerdict, RiskLevelGateHook,
};

// The guard interface consulted by stage 2 is owned by governance; re-exported
// here so the whole five-stage vocabulary is reachable from one place.
pub use apeireth_governance::{GuardRefusal, MonotonicGuards, ToolGuard, ToolGuardRequest};

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use serde::{Deserialize, Serialize};

/// The five explicit stages of one tool execution, in their fixed order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStage {
    /// Policy waterfall before anything runs.
    PreExecute,
    /// Monotonic deny-only guards.
    Guard,
    /// Timeout and retry around the execution.
    Around,
    /// Post-execute correction channel.
    PostExecute,
    /// Output normalization contract.
    Normalize,
}

impl ExecutionStage {
    /// The canonical order. Stages always run in exactly this order.
    pub const ALL: [Self; 5] = [
        Self::PreExecute,
        Self::Guard,
        Self::Around,
        Self::PostExecute,
        Self::Normalize,
    ];

    /// Stable label for records and traces.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreExecute => "pre_execute",
            Self::Guard => "guard",
            Self::Around => "around",
            Self::PostExecute => "post_execute",
            Self::Normalize => "normalize",
        }
    }

    /// Position in the canonical order, zero first: the variants are declared
    /// in that order, so the discriminant *is* the position.
    pub const fn order(self) -> usize {
        self as usize
    }
}

/// One visited stage and what happened there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageEntry {
    /// The stage visited.
    pub stage: ExecutionStage,
    /// One-line outcome of the visit.
    pub detail: String,
}

/// The record of one pipeline run: stages visited, and superseded results.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecutionRecord {
    stages: Vec<StageEntry>,
    superseded: Vec<SupersededResult>,
}

impl ExecutionRecord {
    /// An empty record.
    pub fn new() -> Self {
        Self::default()
    }

    /// The stages visited, in visit order.
    pub fn stages(&self) -> Vec<ExecutionStage> {
        self.stages.iter().map(|entry| entry.stage).collect()
    }

    /// Every stage visit with its detail, in visit order.
    pub fn entries(&self) -> &[StageEntry] {
        &self.stages
    }

    /// Every superseded result, in supersession order.
    pub fn superseded(&self) -> &[SupersededResult] {
        &self.superseded
    }

    pub(crate) fn note_stage(&mut self, stage: ExecutionStage, detail: impl Into<String>) {
        self.stages.push(StageEntry {
            stage,
            detail: detail.into(),
        });
    }

    pub(crate) fn note_superseded(&mut self, entry: SupersededResult) {
        self.superseded.push(entry);
    }
}

/// The closed failure vocabulary of the pipeline.
///
/// Refusals, cancellations, approvals, guard refusals, timeouts, exhausted
/// retries, correction blocks, and contract violations all land here. Every
/// variant carries a stable code and closes into a result via `emit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineFailure {
    /// A pre-execute hook answered `deny`.
    PreDenied {
        /// The hook that denied.
        source: String,
        /// The refusal reason as stated.
        reason: String,
    },
    /// A pre-execute hook answered `cancel`.
    PreCancelled {
        /// The hook that cancelled.
        source: String,
        /// The cancellation reason as stated.
        reason: String,
    },
    /// A pre-execute hook answered `ask`.
    NeedsApproval {
        /// The hook that asked.
        source: String,
        /// What a human is being asked to decide.
        reason: String,
    },
    /// A monotonic guard refused the call.
    GuardDenied {
        /// The refusing guard.
        guard: String,
        /// The refusal reason as stated.
        reason: String,
    },
    /// The around stage failed in the timeout family; `code` is a stable code.
    Timeout {
        /// Stable timeout-family code.
        code: &'static str,
        /// Whether the timed-out run had already **entered** the tool call.
        ///
        /// 原先没有这个判别位: 超时一律报「可重试」, 而在途调用可能已把副作用投递
        /// 出去 (git_write 的 spawn_blocking、MCP 的 tools/call) ⇒ 重试 ⇒ 副作用重复。
        /// 现在: `false` ⇒ 工具从未被进入 ⇒ 可重试; `true` ⇒ 已进入 ⇒ 永久错误。
        started: bool,
    },
    /// Retry was configured and every attempt failed.
    RetriesExhausted {
        /// Attempts made before giving up.
        attempts: u32,
        /// The last attempt's rendered failure.
        last_error: String,
    },
    /// The post-execute channel answered `block`.
    PostBlocked {
        /// The corrector that blocked.
        source: String,
        /// The blocking reason as stated.
        reason: String,
    },
    /// The output normalization contract refused the result shape.
    ContractViolation(
        /// The normalization error, kept whole.
        NormalizationError,
    ),
}

impl PipelineFailure {
    /// Stable code of the failure family.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PreDenied { .. } => "pipeline.pre_deny",
            Self::PreCancelled { .. } => "pipeline.pre_cancel",
            Self::NeedsApproval { .. } => "pipeline.pre_ask",
            Self::GuardDenied { .. } => "pipeline.guard_deny",
            Self::Timeout { code, .. } => code,
            Self::RetriesExhausted { .. } => "pipeline.retries_exhausted",
            Self::PostBlocked { .. } => "pipeline.post_block",
            Self::ContractViolation(error) => error.code(),
        }
    }

    /// Human-readable failure description.
    pub fn message(&self) -> String {
        match self {
            Self::PreDenied { source, reason } => {
                format!("pre-execute deny from {source}: {reason}")
            }
            Self::PreCancelled { source, reason } => {
                format!("pre-execute cancel from {source}: {reason}")
            }
            Self::NeedsApproval { source, reason } => {
                format!("pre-execute ask from {source}: {reason}")
            }
            Self::GuardDenied { guard, reason } => {
                format!("guard {guard} refused: {reason}")
            }
            // 超时且**未开始** ⇒ 工具从未被进入, 确实什么都没发生 ⇒ 重试安全。
            // 超时且**已开始** ⇒ 副作用可能已经落地 ⇒ 重试会重复 ⇒ 永久错误。
            Self::Timeout { started, .. } => format!("timeout failure started={started}"),
            Self::RetriesExhausted {
                attempts,
                last_error,
            } => format!("all {attempts} attempt(s) failed: {last_error}"),
            Self::PostBlocked { source, reason } => {
                format!("post-execute block from {source}: {reason}")
            }
            Self::ContractViolation(error) => format!("output normalization failed: {error}"),
        }
    }

    /// Close the failure into the model-facing result for one call.
    ///
    /// The result always carries the stable code and stays correlated to the
    /// call id; a failure is a frame, not a hole.
    ///
    /// 超时帧按「是否已开始」分流 (口径同 `canonical/execute.rs`): **未开始** ⇒
    /// 什么都没发生 ⇒ 可重试; **已开始** ⇒ 副作用可能已落地 ⇒ 永久错误。
    pub fn emit(&self, call_id: &str, name: Option<&str>) -> ToolResult {
        let message = format!("{}: {}", self.code(), self.message());
        let result = match self {
            Self::Timeout { started: false, .. } | Self::RetriesExhausted { .. } => {
                ToolResult::retryable_error(call_id, message)
            }
            _ => ToolResult::permanent_error(call_id, message),
        };
        match name {
            Some(name) => result.with_name(name),
            None => result,
        }
    }
}

/// One completed pipeline run.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedCall {
    /// The emitted, model-facing result, correlated to the call id.
    pub result: ToolResult,
    /// The frozen normalized contract view of [`ExecutedCall::result`].
    pub outcome: ToolOutcome,
    /// The closed failure when the pipeline judged the call failed.
    pub failure: Option<PipelineFailure>,
    /// Stages visited and results superseded along the way.
    pub record: ExecutionRecord,
}

impl ExecutedCall {
    /// Whether the pipeline judged this call a failure.
    pub fn is_failed(&self) -> bool {
        self.failure.is_some()
    }
}

/// The five-stage tool execution pipeline.
///
/// Built per capability or shared across them; every stage is opt-in.
#[derive(Default)]
pub struct ToolExecutionPipeline {
    pre: pre::PreExecuteWaterfall,
    guards: MonotonicGuards,
    around: around::AroundPolicy,
    post: post::PostExecuteWaterfall,
    schema: Option<contract::OutputSchema>,
}

impl ToolExecutionPipeline {
    /// A pipeline with no hooks, guards, deadline, retry, or schema: unchanged.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one pre-execute policy hook.
    #[must_use]
    pub fn with_pre_hook(mut self, hook: Arc<dyn PreExecuteHook>) -> Self {
        self.pre = self.pre.with(hook);
        self
    }

    /// Append one monotonic guard.
    #[must_use]
    pub fn with_guard(mut self, guard: Arc<dyn ToolGuard>) -> Self {
        self.guards = self.guards.with(guard);
        self
    }

    /// Install a whole monotonic guard set (appended, keeping their order).
    #[must_use]
    pub fn with_guards(mut self, guards: MonotonicGuards) -> Self {
        self.guards = self.guards.merge(guards);
        self
    }

    /// Wrap execution in the given timeout/retry policy.
    #[must_use]
    pub fn with_around(mut self, around: AroundPolicy) -> Self {
        self.around = around;
        self
    }

    /// Append one post-execute corrector.
    #[must_use]
    pub fn with_post_hook(mut self, hook: Arc<dyn PostExecuteHook>) -> Self {
        self.post = self.post.with(hook);
        self
    }

    /// Declare the output schema the contract enforces.
    #[must_use]
    pub fn with_output_schema(mut self, schema: OutputSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// The pre-execute waterfall in force.
    pub fn pre(&self) -> &pre::PreExecuteWaterfall {
        &self.pre
    }

    /// The monotonic guard set in force.
    pub fn guards(&self) -> &MonotonicGuards {
        &self.guards
    }

    /// The around policy in force.
    pub fn around(&self) -> &around::AroundPolicy {
        &self.around
    }

    /// The post-execute channel in force.
    pub fn post(&self) -> &post::PostExecuteWaterfall {
        &self.post
    }

    /// The declared output schema, when one is enforced.
    pub fn output_schema(&self) -> Option<&contract::OutputSchema> {
        self.schema.as_ref()
    }

    /// Run one call through the five stages.
    pub async fn run(&self, tool: &dyn ToolCapability, call: &ToolCall) -> ExecutedCall {
        self.run_frozen(tool, call, None).await
    }

    /// Run one call, executing a previously frozen invocation.
    pub async fn run_frozen(
        &self,
        tool: &dyn ToolCapability,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ExecutedCall {
        self.run_in_session(tool, call, frozen, None).await
    }

    /// Run one call with the turn's session in scope; host-side calls pass none.
    pub async fn run_in_session(
        &self,
        tool: &dyn ToolCapability,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ExecutedCall {
        self.run_scoped(tool, call, frozen, session, None).await
    }

    async fn run_scoped(
        &self,
        tool: &dyn ToolCapability,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
        context: Option<&apeireth_plugin::ToolExecutionContext>,
    ) -> ExecutedCall {
        let mut record = ExecutionRecord::default();

        // Stage 1: pre-execute waterfall.
        let decision = self
            .pre
            .evaluate(&pre::PreExecuteRequest::new(call, tool.id()));
        record.note_stage(ExecutionStage::PreExecute, decision.summary());
        if let Some(failure) = decision.failure() {
            return Self::finish(call, failure, record);
        }

        // Stage 2: monotonic guard set.
        let guard_request = ToolGuardRequest::new(tool.id(), &call.name, &call.arguments);
        match self.guards.deny(&guard_request) {
            Some(refusal) => {
                record.note_stage(
                    ExecutionStage::Guard,
                    format!("refused by {}", refusal.guard),
                );
                return Self::finish(
                    call,
                    PipelineFailure::GuardDenied {
                        guard: refusal.guard,
                        reason: refusal.reason,
                    },
                    record,
                );
            }
            None => record.note_stage(ExecutionStage::Guard, "no objection"),
        }

        // Stage 3: around — timeout and retry wrap the execution.
        let (mut result, attempts) =
            match around::run_around(tool, call, frozen, session, context, &self.around).await {
                Ok(executed) => executed,
                Err(failure) => {
                    record.note_stage(ExecutionStage::Around, failure.code());
                    return Self::finish(call, failure, record);
                }
            };
        record.note_stage(ExecutionStage::Around, format!("attempts={attempts}"));
        let mut failure = if !result.is_ok()
            && self.around.retry.attempts() > 1
            && attempts >= self.around.retry.attempts()
        {
            Some(PipelineFailure::RetriesExhausted {
                attempts,
                last_error: result.render(),
            })
        } else {
            None
        };

        // Stage 4: post-execute correction channel.
        let post_decision = self
            .post
            .evaluate(&post::PostExecuteRequest::new(call, &result));
        record.note_stage(ExecutionStage::PostExecute, post_decision.summary());
        match post_decision.verdict {
            post::PostVerdict::Accept => {}
            post::PostVerdict::Replace { corrected, note } => {
                record.note_superseded(SupersededResult {
                    stage: ExecutionStage::PostExecute,
                    hook: post_decision.decided_by.clone().unwrap_or_default(),
                    original: result.clone(),
                    note: note.clone(),
                });
                result = corrected;
                result.tool_call_id = call.id.clone();
                // The corrector produced the final result; whatever the
                // superseded one suffered is history, not a live failure.
                failure = None;
            }
            post::PostVerdict::Block { reason } => {
                let source = post_decision.decided_by.clone().unwrap_or_default();
                record.note_superseded(SupersededResult {
                    stage: ExecutionStage::PostExecute,
                    hook: source.clone(),
                    original: result.clone(),
                    note: reason.clone(),
                });
                let blocked = PipelineFailure::PostBlocked { source, reason };
                result = blocked.emit(&call.id, Some(call.name.as_str()));
                failure = Some(blocked);
            }
        }

        // Stage 5: output normalization contract.
        let outcome = match contract::ToolOutcome::normalize(&result, self.schema.as_ref()) {
            Ok(outcome) => {
                record.note_stage(ExecutionStage::Normalize, "contract ok");
                outcome
            }
            Err(error) => {
                record.note_superseded(SupersededResult {
                    stage: ExecutionStage::Normalize,
                    hook: "output_contract".to_string(),
                    original: result.clone(),
                    note: error.to_string(),
                });
                let violation = PipelineFailure::ContractViolation(error);
                result = violation.emit(&call.id, Some(call.name.as_str()));
                record.note_stage(ExecutionStage::Normalize, violation.code());
                failure = Some(violation);
                contract::ToolOutcome::freeze(&result)
            }
        };

        ExecutedCall {
            result,
            outcome,
            failure,
            record,
        }
    }

    /// Close a refusal into a completed run without executing anything.
    fn finish(call: &ToolCall, failure: PipelineFailure, record: ExecutionRecord) -> ExecutedCall {
        let result = failure.emit(&call.id, Some(call.name.as_str()));
        let outcome = contract::ToolOutcome::freeze(&result);
        ExecutedCall {
            result,
            outcome,
            failure: Some(failure),
            record,
        }
    }
}

/// Wraps a capability so every call walks the five explicit stages.
///
/// Identity, declarations, and frozen invocations are forwarded unchanged.
pub struct PipelinedCapability {
    inner: Arc<dyn ToolCapability>,
    pipeline: Arc<ToolExecutionPipeline>,
}

impl PipelinedCapability {
    /// Wrap `inner` in `pipeline`.
    pub fn new(inner: Arc<dyn ToolCapability>, pipeline: Arc<ToolExecutionPipeline>) -> Self {
        Self { inner, pipeline }
    }

    /// The wrapped capability.
    pub fn inner(&self) -> &Arc<dyn ToolCapability> {
        &self.inner
    }

    /// The pipeline every call walks.
    pub fn pipeline(&self) -> &Arc<ToolExecutionPipeline> {
        &self.pipeline
    }
}

#[async_trait::async_trait]
impl ToolCapability for PipelinedCapability {
    fn execution_policy(&self) -> apeireth_plugin::ToolExecutionPolicy {
        self.inner.execution_policy()
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &apeireth_plugin::ToolExecutionContext,
    ) -> ToolResult {
        self.pipeline
            .run_scoped(
                self.inner.as_ref(),
                call,
                frozen,
                context.session,
                Some(context),
            )
            .await
            .result
    }

    fn id(&self) -> &CapabilityId {
        self.inner.id()
    }

    fn declaration(&self) -> NormalizedTool {
        self.inner.declaration()
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.pipeline.run(self.inner.as_ref(), call).await.result
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        self.pipeline
            .run_frozen(self.inner.as_ref(), call, frozen)
            .await
            .result
    }

    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.pipeline
            .run_in_session(self.inner.as_ref(), call, frozen, session)
            .await
            .result
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        self.inner.freeze_invocation(call)
    }
}
