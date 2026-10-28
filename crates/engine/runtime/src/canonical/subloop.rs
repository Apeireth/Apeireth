//! Bounded Module-owned SubLoops.
//!
//! # Invariants
//!
//! 1. A SubLoop executes on a private instance transcript. It never mutates
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

use apeireth_core::kernel::{AgentId, CapabilityId, RequestId, SessionId, TraceId};
use apeireth_governance::GovernanceHook;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{
    NormalizedMessage, NormalizedRequest, NormalizedUsage, ToolResult,
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
    /// Initial messages for the private instance transcript.
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
    /// History of tool invocations performed within the private session.
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

#[derive(Clone)]
pub(crate) struct SubLoopExecution {
    pub request: RequestId,
    pub agent: AgentId,
    pub runtime: Arc<super::Runtime>,
    pub security: Option<apeireth_governance::TurnSecurityContext>,
}

/// Runtime-owned compatibility spawner over the existing instance execution port.
///
/// An owned turn-scoped handle: it carries one invocation's session, trace,
/// model and budget context, clones share that invocation's `ModuleTurnState`,
/// and it is not task-local or global ambient authority — it exists only where
/// the runtime hands it out for the current invocation. It must not be
/// retained as persistent module state across turns.
#[derive(Clone)]
pub struct RuntimeSubLoopSpawner {
    execution: SubLoopExecution,
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
        execution: SubLoopExecution,
    ) -> Self {
        Self {
            execution,
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
        let started = std::time::Instant::now();
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

        let tools = self
            .tools
            .iter()
            .filter(|tool| spec.allowed_capabilities.contains(tool.id()))
            .map(|tool| tool.id().clone())
            .collect();
        let child = self
            .execution
            .runtime
            .spawn_side_agent(
                &self.module_id,
                self.session_id,
                apeireth_plugin::agents::AgentConfig {
                    tools,
                    model: Some(model.clone()),
                    ..Default::default()
                },
                self.execution.security.clone(),
            )
            .await
            .map_err(|e| self.task_error(e, spec.max_rounds))?;
        let result = child
            .agent
            .execute(apeireth_plugin::agents::AgentTask {
                messages: spec.messages,
                system: spec.system_prompt,
                model: Some(model),
                max_rounds: spec.max_rounds,
                timeout: spec
                    .timeout
                    .map(|limit| limit.saturating_sub(started.elapsed())),
                mode: apeireth_plugin::agents::AgentExecutionMode::Isolated,
                request_gate: Some(self.state.clone()),
                origin: Some(apeireth_plugin::agents::AgentTaskOrigin {
                    agent: self.execution.agent,
                    session: self.session_id,
                    request: self.execution.request,
                    trace: self.trace_id,
                }),
            })
            .await;
        // Cancellation ends admission first; completion is reported only after
        // the existing instance stop boundary has drained native tool leases.
        child
            .agent
            .stop()
            .await
            .map_err(|e| self.task_error(e, spec.max_rounds))?;
        let reply = result.map_err(|e| self.task_error(e, spec.max_rounds))?;
        Ok(SubLoopResult {
            text: reply.response.content,
            rounds: reply.rounds,
            usage: reply.usage,
            tool_results: reply.tool_results,
        })
    }

    fn task_error(&self, error: apeireth_plugin::agents::AgentError, rounds: u32) -> SubLoopError {
        use apeireth_plugin::agents::AgentErrorCode;
        match error.code {
            AgentErrorCode::BudgetExceeded if self.state.used() >= self.state.limit() => {
                SubLoopError::BudgetExceeded {
                    limit: self.state.limit(),
                }
            }
            AgentErrorCode::RoundLimitExceeded => SubLoopError::RoundLimitReached { rounds },
            AgentErrorCode::Denied => SubLoopError::Denied {
                reason: error.message,
            },
            AgentErrorCode::ApprovalRequired | AgentErrorCode::PendingApproval => {
                SubLoopError::ApprovalRequired {
                    reason: error.message,
                }
            }
            AgentErrorCode::Timeout => SubLoopError::Timeout,
            _ => SubLoopError::Provider {
                reason: error.to_string(),
            },
        }
    }
}

#[async_trait]
impl SubLoopSpawner for RuntimeSubLoopSpawner {
    async fn spawn(&self, spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        self.spawn_bounded(spec).await
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
            .complete_for_session(
                self.session_id,
                self.execution.request,
                &NormalizedRequest::new(model, messages),
                None,
                None,
            )
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
