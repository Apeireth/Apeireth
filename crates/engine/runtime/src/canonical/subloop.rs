//! Bounded module-owned SubLoops: the runtime-side implementation.
//!
//! The spec, the result, the failure type, and the [`SubLoopSpawner`] port are
//! declared in `apeireth-plugin` next to the rest of the module ABI, because a
//! module needs them to ask for a side loop while the runtime provides it. This
//! module owns the one implementation,
//! [`RuntimeSubLoopSpawner`], plus the
//! [`ModuleInvoker`](crate::canonical::ModuleInvoker) implementation that lets a
//! spawner also serve a single isolated completion.
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
//!    structured result to its owning module.
//! 5. A SubLoop is never a second user-facing loop.
//!
//! The tool set a SubLoop may draw from is whatever the canonical module manager
//! reports as dispatchable — there is no second tool world to consult.

use std::sync::Arc;

pub use apeireth_plugin::subloop::{SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{
    NormalizedFinishReason, NormalizedMessage, NormalizedRequest, NormalizedUsage, ToolCall,
    ToolResult,
};
use async_trait::async_trait;

use super::module::{
    authorize_isolated_completion, ModuleInvocationError, ModuleInvocationRequest,
    ModuleInvocationResponse, ModuleInvoker, ModuleTurnState, DEFAULT_MAX_INVOCATION_DEPTH,
};
use super::provider::ProviderRouter;

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
            // 与 M11 同类: usage 累加不得回绕 (debug panic / release 静默截断
            // 都会污染记账)。
            total_usage.prompt_tokens = total_usage
                .prompt_tokens
                .saturating_add(response.usage.prompt_tokens);
            total_usage.completion_tokens = total_usage
                .completion_tokens
                .saturating_add(response.usage.completion_tokens);
            total_usage.total_tokens = total_usage
                .total_tokens
                .saturating_add(response.usage.total_tokens);

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
        // 1. Look up tool among all active runtime tools.
        // M20: 按 name fail-closed, 与 capability.rs `find_by_name` 同语义 ——
        // 多个同名工具时不猜第一个 (旧实现 `find(...)` 命中即返回), 直接判
        // 歧义拒绝, 避免"注册了谁就调度谁"的隐性选择。
        let mut name_matches = self
            .tools
            .iter()
            .filter(|t| t.declaration().name == call.name);
        let Some(tool) = name_matches.next() else {
            return ToolResult::permanent_error(
                &call.id,
                format!("no tool named {:?} is available", call.name),
            )
            .with_name(&call.name);
        };
        if name_matches.next().is_some() {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "ambiguous tool name {:?}: more than one registered tool claims this model-facing name; refusing to dispatch (fail-closed)",
                    call.name
                ),
            )
            .with_name(&call.name);
        }

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
        // M20: 绑定真实 action_id (provider tool-call id) —— 依赖 action_id
        // 的审批去重/审计关联此前在 subloop 上失效 (治理 hook 只能自行派生
        // 一个与 tool_call_id 不同的标识, 数据集标签因此错配)。
        let action = Action::CapabilityDispatch {
            capability,
            arguments: &call.arguments,
        };
        let verdict = self
            .governance
            .evaluate_verbose(
                &GovernanceRequest::new(action, self.session_id, self.trace_id, round)
                    .with_action_id(&call.id),
            )
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
