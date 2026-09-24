//! The module ABI, and the runtime-owned invoker that implements its ports.
//!
//! The ABI itself — [`Module`], [`ModuleManifest`], [`ModuleContext`],
//! [`HookPoint`], [`ModuleOutcome`], the [`SubLoopSpawner`] port and friends —
//! lives in `apeireth-plugin` next to the capability and lifecycle model, so
//! that one module may own lifecycle, hooks, and capabilities under a single
//! trait and a single registry. This module re-exports that surface under the
//! paths the runtime has always used, and adds the two things only a runtime
//! can provide:
//!
//! - [`RuntimeModuleInvoker`], the implementation of the plugin-side
//!   [`ModuleInvoker`] port that routes an isolated module side-call through
//!   the canonical provider router under canonical completion governance;
//! - [`authorize_isolated_completion`], the shared governance gate both the
//!   invoker and the SubLoop spawner consult.
//!
//! Historical spellings ([`BehaviorModule`], [`AgentModule`],
//! [`BehaviorRegistry`]) survive as aliases of the unified types; they are
//! compatibility shims and are scheduled for removal.

pub use apeireth_plugin::{
    reject_tool_identity_collisions, HookPoint, InvocationContext, InvocationOrigin, Module,
    ModuleContext, ModuleDirective, ModuleError, ModuleExports, ModuleInitContext,
    ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
    ModuleManifest, ModuleOrigin, ModuleOutcome, ModuleTurnState, PromptOverlay, SubLoopError,
    SubLoopResult, SubLoopSpawner, SubLoopSpec, DEFAULT_MAX_INVOCATION_DEPTH,
    DEFAULT_MAX_MODULE_INVOCATIONS,
};

/// The canonical module registry, owned by the manager.
pub use apeireth_plugin::ModuleRegistry;

use std::sync::Arc;

use apeireth_core::kernel::{SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};

use super::provider::ProviderRouter;

/// Compatibility alias for the unified module trait.
///
/// A module may own hooks, capabilities, and lifecycle at once; there is no
/// longer a separate "behavior" trait to implement alongside a capability one.
pub use apeireth_plugin::Module as BehaviorModule;

/// Compatibility alias retained for existing embedders.
pub use apeireth_plugin::Module as AgentModule;

/// Compatibility alias for [`ModuleRegistry`].
pub use apeireth_plugin::ModuleRegistry as BehaviorRegistry;

/// Canonical completion governance for isolated module/SubLoop side-calls.
///
/// Side loops never mint a hidden approval: `RequireApproval` fails closed.
pub(crate) async fn authorize_isolated_completion(
    governance: &dyn GovernanceHook,
    session_id: SessionId,
    trace_id: TraceId,
    model: &str,
    message_count: usize,
    round: u32,
) -> Result<(), ModuleInvocationError> {
    let action = Action::Completion {
        model,
        message_count,
    };
    let verdict = governance
        .evaluate_verbose(&GovernanceRequest::new(action, session_id, trace_id, round))
        .await;
    match verdict.decision {
        Decision::Allow => Ok(()),
        Decision::Deny { reason } => Err(ModuleInvocationError::Denied { reason }),
        Decision::RequireApproval { reason } => {
            Err(ModuleInvocationError::ApprovalRequired { reason })
        }
    }
}

/// Runtime-owned implementation of the isolated module invoker.
///
/// An owned turn-scoped handle: it carries one invocation's session, trace,
/// model and budget context, clones share that invocation's `ModuleTurnState`,
/// and it is not task-local or global ambient authority — it exists only where
/// the runtime hands it out for the current invocation. It must not be
/// retained as persistent module state across turns.
pub struct RuntimeModuleInvoker {
    router: Arc<ProviderRouter>,
    governance: Arc<dyn GovernanceHook>,
    session_id: SessionId,
    trace_id: TraceId,
    current_model: Arc<str>,
    module_id: Arc<str>,
    state: Arc<ModuleTurnState>,
    depth: u8,
}

impl Clone for RuntimeModuleInvoker {
    fn clone(&self) -> Self {
        Self {
            router: Arc::clone(&self.router),
            governance: Arc::clone(&self.governance),
            session_id: self.session_id,
            trace_id: self.trace_id,
            current_model: Arc::clone(&self.current_model),
            module_id: Arc::clone(&self.module_id),
            state: Arc::clone(&self.state),
            depth: self.depth,
        }
    }
}

impl RuntimeModuleInvoker {
    /// Build an invoker for one invocation, one nesting level below the parent.
    pub fn new(
        router: Arc<ProviderRouter>,
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
            governance,
            session_id,
            trace_id,
            current_model: current_model.into(),
            module_id: module_id.into(),
            state,
            depth: parent_depth.saturating_add(1),
        }
    }
}

#[async_trait::async_trait]
impl ModuleInvoker for RuntimeModuleInvoker {
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
            messages.push(apeireth_protocol::canonical::NormalizedMessage::system(
                system,
            ));
        }
        messages.push(apeireth_protocol::canonical::NormalizedMessage::user(
            request.input,
        ));

        authorize_isolated_completion(
            self.governance.as_ref(),
            self.session_id,
            self.trace_id,
            &model,
            messages.len(),
            1,
        )
        .await?;

        // `complete` intentionally sends no tool declarations. It also does
        // not enter the session store or the module hook bus, so this is an
        // isolated side-call rather than a nested agent loop.
        let routed = self
            .router
            .complete(&apeireth_protocol::canonical::NormalizedRequest::new(
                model, messages,
            ))
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
