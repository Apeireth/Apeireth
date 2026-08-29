//! Bounded Module-owned SubLoops.
//!
//! # Invariants
//!
//! 1. A SubLoop executes on a private, ephemeral transcript. It never mutates
//!    the parent user-facing session.
//! 2. A SubLoop has strictly bounded rounds, bounded timeouts, and shared
//!    budget limits.
//! 3. A SubLoop has an explicit capability allowlist. Only tools matching the
//!    allowlist are available during its execution.
//! 4. A SubLoop never emits events directly to the user frontend; it returns a
//!    structured [`SubLoopResult`] to its owning module.
//! 5. A SubLoop is never a second user-facing loop.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{
    NormalizedFinishReason, NormalizedMessage, NormalizedRequest, NormalizedUsage, ToolCall,
    ToolResult,
};
use async_trait::async_trait;
use thiserror::Error;

use super::module::{
    authorize_isolated_completion, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleTurnState, DEFAULT_MAX_INVOCATION_DEPTH,
};
use super::provider::ProviderRouter;

/// Specification defining the bounded constraints and goals of a SubLoop.
#[derive(Debug, Clone)]
pub struct SubLoopSpec {
    /// Bounded round limit.
    pub max_rounds: u32,
    /// Explicit list of capability IDs allowed to be executed within this SubLoop.
    pub allowed_capabilities: Vec<CapabilityId>,
    /// Optional execution timeout across the whole SubLoop.
    pub timeout: Option<Duration>,
    /// Initial messages for the ephemeral transcript.
    pub messages: Vec<NormalizedMessage>,
    /// System prompt instructions for the SubLoop.
    pub system_prompt: Option<String>,
    /// Target model override. If None, inherits the parent turn's model.
    pub model: Option<String>,
}

impl SubLoopSpec {
    /// Create a single-round, zero-tool isolated prompt spec.
    pub fn single_turn(prompt: impl Into<String>) -> Self {
        Self {
            max_rounds: 1,
            allowed_capabilities: Vec::new(),
            timeout: None,
            messages: vec![NormalizedMessage::user(prompt.into())],
            system_prompt: None,
            model: None,
        }
    }

    /// Add a system prompt.
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Set round limit.
    pub fn with_max_rounds(mut self, max_rounds: u32) -> Self {
        self.max_rounds = max_rounds;
        self
    }

    /// Set timeout duration.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Allow a capability ID.
    pub fn with_allowed_capability(mut self, capability: CapabilityId) -> Self {
        self.allowed_capabilities.push(capability);
        self
    }

    /// Set model override.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Structured outcome returned upon SubLoop completion.
#[derive(Debug, Clone)]
pub struct SubLoopResult {
    /// Final text content produced by the SubLoop.
    pub text: String,
    /// Number of rounds executed.
    pub rounds: u32,
    /// Aggregate token usage.
    pub usage: NormalizedUsage,
    /// History of tool invocations performed within the ephemeral session.
    pub tool_results: Vec<ToolResult>,
}

/// Failures that can occur during bounded SubLoop execution.
#[derive(Debug, Error)]
pub enum SubLoopError {
    /// Turn budget for side invocations exceeded.
    #[error("subloop budget exceeded: limit is {limit}")]
    BudgetExceeded { limit: usize },
    /// Nesting recursion limit exceeded.
    #[error("subloop recursion limit {depth} exceeds maximum {maximum}")]
    RecursionLimit { depth: u8, maximum: u8 },
    /// No model available.
    #[error("subloop execution has no model")]
    NoModel,
    /// Specified timeout expired.
    #[error("subloop execution timed out")]
    Timeout,
    /// Round limit reached without convergence.
    #[error("subloop reached round limit ({rounds}) without final response")]
    RoundLimitReached { rounds: u32 },
    /// Requested tool capability was denied by the SubLoop allowlist.
    #[error("subloop tool {name:?} is not permitted by capability allowlist")]
    CapabilityDenied { name: String },
    /// Provider failure.
    #[error("subloop provider failed: {reason}")]
    Provider { reason: String },
    /// Canonical completion governance refused the SubLoop provider call.
    #[error("subloop completion refused by governance: {reason}")]
    Denied { reason: String },
    /// SubLoops cannot mint a hidden human approval for completions.
    #[error("subloop completion requires approval which is not permitted: {reason}")]
    ApprovalRequired { reason: String },
}

impl From<ModuleInvocationError> for SubLoopError {
    fn from(err: ModuleInvocationError) -> Self {
        match err {
            ModuleInvocationError::BudgetExceeded { limit } => Self::BudgetExceeded { limit },
            ModuleInvocationError::RecursionLimit { depth, maximum } => {
                Self::RecursionLimit { depth, maximum }
            }
            ModuleInvocationError::NoModel => Self::NoModel,
            ModuleInvocationError::Provider { reason } => Self::Provider { reason },
            ModuleInvocationError::Denied { reason } => Self::Denied { reason },
            ModuleInvocationError::ApprovalRequired { reason } => Self::ApprovalRequired { reason },
        }
    }
}

/// Trait implemented by runtime hosts to spawn bounded SubLoops.
#[async_trait]
pub trait SubLoopSpawner: Send + Sync {
    /// Spawn and execute a bounded SubLoop with a private transcript.
    async fn spawn(&self, spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError>;
}

/// Runtime-owned spawner that executes bounded SubLoops against router and tools.
///
/// An owned turn-scoped handle: it carries one invocation's session, trace,
/// model and budget context, clones share that invocation's `ModuleTurnState`,
/// and it is not task-local or global ambient authority — it exists only where
/// the runtime hands it out for the current invocation. It must not be
/// retained as persistent module state across turns.
#[derive(Clone)]
pub struct RuntimeSubLoopSpawner {
    router: Arc<ProviderRouter>,
    tools: Vec<Arc<dyn ToolCapability>>,
    governance: Arc<dyn GovernanceHook>,
    session_id: SessionId,
    trace_id: TraceId,
    current_model: Arc<str>,
    module_id: Arc<str>,
    state: Arc<ModuleTurnState>,
    depth: u8,
}

impl RuntimeSubLoopSpawner {
    /// Create a new SubLoop spawner scoped to a module hook.
    pub(crate) fn new(
        router: Arc<ProviderRouter>,
        tools: Vec<Arc<dyn ToolCapability>>,
        governance: Arc<dyn GovernanceHook>,
        session_id: SessionId,
        trace_id: TraceId,
        current_model: impl Into<Arc<str>>,
        module_id: impl Into<Arc<str>>,
        state: Arc<ModuleTurnState>,
        parent_depth: u8,
    ) -> Self {
        Self {
            router,
            tools,
            governance,
            session_id,
            trace_id,
            current_model: current_model.into(),
            module_id: module_id.into(),
            state,
            depth: parent_depth.saturating_add(1),
        }
    }

    async fn spawn_bounded(&self, spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        if self.depth > DEFAULT_MAX_INVOCATION_DEPTH {
            return Err(SubLoopError::RecursionLimit {
                depth: self.depth,
                maximum: DEFAULT_MAX_INVOCATION_DEPTH,
            });
        }
        let model = spec.model.unwrap_or_else(|| self.current_model.to_string());
        if model.is_empty() {
            return Err(SubLoopError::NoModel);
        }
        if spec.max_rounds == 0 {
            return Err(SubLoopError::RoundLimitReached { rounds: 0 });
        }

        // Filter available tools by the spec's capability allowlist for model-facing declarations
        let allowed_tools: Vec<Arc<dyn ToolCapability>> = self
            .tools
            .iter()
            .filter(|t| spec.allowed_capabilities.iter().any(|c| c == t.id()))
            .cloned()
            .collect();

        let tool_declarations: Vec<_> = allowed_tools.iter().map(|t| t.declaration()).collect();

        // Build ephemeral transcript
        let mut transcript: Vec<NormalizedMessage> = Vec::new();
        if let Some(system) = spec.system_prompt {
            transcript.push(NormalizedMessage::system(system));
        }
        transcript.extend(spec.messages);

        let mut round = 0;
        let mut total_usage = NormalizedUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        let mut collected_tool_results = Vec::new();

        while round < spec.max_rounds {
            round += 1;
            self.state.reserve().map_err(SubLoopError::from)?;

            let mut req = NormalizedRequest::new(model.clone(), transcript.clone());
            if !tool_declarations.is_empty() {
                req.tools = tool_declarations.clone();
            }

            authorize_isolated_completion(
                self.governance.as_ref(),
                self.session_id,
                self.trace_id,
                &model,
                transcript.len(),
                round,
            )
            .await?;

            let routed =
                self.router
                    .complete(&req)
                    .await
                    .map_err(|error| SubLoopError::Provider {
                        reason: format!("module {} subloop: {error}", self.module_id),
                    })?;

            let response = routed.response;
            total_usage.prompt_tokens += response.usage.prompt_tokens;
            total_usage.completion_tokens += response.usage.completion_tokens;
            total_usage.total_tokens += response.usage.total_tokens;

            // If the model called tools
            if response.finish_reason == Some(NormalizedFinishReason::ToolCalls)
                && !response.tool_calls.is_empty()
            {
                transcript.push(NormalizedMessage::assistant_with_tool_calls(
                    &response.content,
                    response.tool_calls.clone(),
                ));

                for call in &response.tool_calls {
                    let result = self
                        .dispatch_subloop_tool(call, &spec.allowed_capabilities, round)
                        .await;
                    collected_tool_results.push(result.clone());
                    transcript.push(result.into_message());
                }
            } else {
                // Model completed with text
                return Ok(SubLoopResult {
                    text: response.content,
                    rounds: round,
                    usage: total_usage,
                    tool_results: collected_tool_results,
                });
            }
        }

        Err(SubLoopError::RoundLimitReached {
            rounds: spec.max_rounds,
        })
    }

    async fn dispatch_subloop_tool(
        &self,
        call: &ToolCall,
        allowlist: &[CapabilityId],
        round: u32,
    ) -> ToolResult {
        // 1. Look up tool among all active runtime tools
        let Some(tool) = self
            .tools
            .iter()
            .find(|t| t.declaration().name == call.name)
        else {
            return ToolResult::permanent_error(
                &call.id,
                format!("no tool named {:?} is available", call.name),
            )
            .with_name(&call.name);
        };

        let capability = tool.id();

        // 2. Check SubLoop allowlist (allowlist is an ADDITIONAL constraint)
        if !allowlist.iter().any(|c| c == capability) {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "tool {:?} ({capability}) is not in the subloop capability allowlist",
                    call.name
                ),
            )
            .with_name(&call.name);
        }

        // 3. Canonical Governance evaluation (must never be bypassed!)
        let action = Action::CapabilityDispatch {
            capability,
            arguments: &call.arguments,
        };
        let verdict = self
            .governance
            .evaluate_verbose(&GovernanceRequest::new(
                action,
                self.session_id,
                self.trace_id,
                round,
            ))
            .await;

        match verdict.decision {
            Decision::Allow => {
                // Succeeded all checks: invoke capability
                tool.invoke(call).await
            }
            Decision::Deny { reason } => {
                // Denied by governance: do NOT invoke tool
                ToolResult::permanent_error(&call.id, format!("refused by governance: {reason}"))
                    .with_name(&call.name)
            }
            Decision::RequireApproval { reason } => {
                // SubLoops cannot perform interactive user approvals; fail cleanly without invoking
                ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "subloop capability requires interactive approval which is not permitted in subloops: {reason}"
                    ),
                )
                .with_name(&call.name)
            }
        }
    }
}

#[async_trait]
impl SubLoopSpawner for RuntimeSubLoopSpawner {
    async fn spawn(&self, spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        let timeout_opt = spec.timeout;
        let fut = self.spawn_bounded(spec);
        if let Some(dur) = timeout_opt {
            match tokio::time::timeout(dur, fut).await {
                Ok(res) => res,
                Err(_) => Err(SubLoopError::Timeout),
            }
        } else {
            fut.await
        }
    }
}

#[async_trait]
impl ModuleInvoker for RuntimeSubLoopSpawner {
    async fn invoke(
        &self,
        request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        if self.depth > DEFAULT_MAX_INVOCATION_DEPTH {
            return Err(ModuleInvocationError::RecursionLimit {
                depth: self.depth,
                maximum: DEFAULT_MAX_INVOCATION_DEPTH,
            });
        }
        let model = request
            .model
            .unwrap_or_else(|| self.current_model.to_string());
        if model.is_empty() {
            return Err(ModuleInvocationError::NoModel);
        }
        self.state.reserve()?;

        let mut messages = Vec::with_capacity(2);
        if let Some(system) = request.system {
            messages.push(NormalizedMessage::system(system));
        }
        messages.push(NormalizedMessage::user(request.input));

        authorize_isolated_completion(
            self.governance.as_ref(),
            self.session_id,
            self.trace_id,
            &model,
            messages.len(),
            1,
        )
        .await?;

        let routed = self
            .router
            .complete(&NormalizedRequest::new(model, messages))
            .await
            .map_err(|error| ModuleInvocationError::Provider {
                reason: format!("module {}: {error}", self.module_id),
            })?;

        Ok(ModuleInvocationResponse {
            response: routed.response,
            served_by: routed.served_by,
        })
    }
}
