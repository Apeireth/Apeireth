//! The five-stage tool execution pipeline.
//!
//! Policy and execution used to be entangled: whatever a call site happened to
//! check, in whatever order it happened to check it, decided what ran and what
//! the model was told. This pipeline makes the chain explicit. One call walks
//! five stages, always in this order:
//!
//! 1. [`ExecutionStage::PreExecute`] — the pre-execute waterfall: policy hooks
//!    answer `allow` / `deny` / `cancel` / `ask`, first non-allow wins
//!    (see [`pre`]);
//! 2. [`ExecutionStage::Guard`] — the monotonic guard set: guards may only
//!    refuse, never allow, and a refusal cannot be flipped back
//!    (see [`apeireth_governance::tool_guard`]);
//! 3. [`ExecutionStage::Around`] — timeout (through the deadline timer
//!    library) and retry wrap the execution (see [`around`]);
//! 4. [`ExecutionStage::PostExecute`] — the correction channel: `accept` /
//!    `replace` / `block`, with superseded results archived (see [`post`]);
//! 5. [`ExecutionStage::Normalize`] — the output contract: the result is
//!    frozen into [`ToolOutcome { ok, output_text, meta }`] and checked
//!    against the declared schema (see [`contract`]).
//!
//! The stage order is a property of the pipeline, not of its callers: the
//! record built along the way lists the stages actually visited, in order.
//!
//! # Default behaviour is unchanged behaviour
//!
//! A pipeline with no hooks, no guards, no deadline, no retry, and no schema
//! runs the tool once and emits its result untouched — byte-for-byte what the
//! call produced before this chain existed. Every stage is opt-in.

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

// The guard interface consulted by stage 2 is owned by governance; it is
// re-exported here so the whole five-stage vocabulary is reachable from one
// place without a second definition.
pub use apeireth_governance::{GuardRefusal, MonotonicGuards, ToolGuard, ToolGuardRequest};

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
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

    /// Position in the canonical order, zero first.
    pub const fn order(self) -> usize {
        match self {
            Self::PreExecute => 0,
            Self::Guard => 1,
            Self::Around => 2,
            Self::PostExecute => 3,
            Self::Normalize => 4,
        }
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

/// The record of one pipeline run: stages visited, in order, and every result
/// a stage superseded (kept, never dropped).
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
/// Refusals, cancellations, human-approval requirements, guard refusals,
/// timeouts, exhausted retries, correction blocks, and contract violations all
/// land here — there is no half state and no unnamed failure. Every variant
/// carries a stable [`PipelineFailure::code`] and closes into a model-facing
/// result via [`PipelineFailure::emit`].
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
    /// The around stage failed in the timeout family; `code` is one of the
    /// deadline library's stable timeout codes.
    Timeout {
        /// Stable timeout-family code.
        code: &'static str,
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
            Self::Timeout { code } => code,
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
            Self::Timeout { code } => format!("timeout failure ({code})"),
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
    pub fn emit(&self, call_id: &str, name: Option<&str>) -> ToolResult {
        let message = format!("{}: {}", self.code(), self.message());
        let result = match self {
            Self::Timeout { .. } | Self::RetriesExhausted { .. } => {
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
/// Built per capability or shared across them; every stage is opt-in, and the
/// default pipeline runs tools exactly as they ran before.
#[derive(Default)]
pub struct ToolExecutionPipeline {
    pre: pre::PreExecuteWaterfall,
    guards: MonotonicGuards,
    around: around::AroundPolicy,
    post: post::PostExecuteWaterfall,
    schema: Option<contract::OutputSchema>,
}

impl ToolExecutionPipeline {
    /// A pipeline with no hooks, no guards, no deadline, no retry, and no
    /// schema: execution behaviour is unchanged.
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

    /// Install a whole monotonic guard set (appended to any guards already
    /// listening, keeping their order).
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

    /// Run one call through the five stages, executing a previously frozen
    /// invocation instead of the original call arguments.
    pub async fn run_frozen(
        &self,
        tool: &dyn ToolCapability,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
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
            match around::run_around(tool, call, frozen, &self.around).await {
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
/// Identity, declarations, and frozen invocations are forwarded unchanged;
/// only execution goes through the pipeline. With a default pipeline the
/// emitted result is exactly the wrapped capability's own.
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

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        self.inner.freeze_invocation(call)
    }
}
