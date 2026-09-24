//! Bounded module-owned SubLoops: the spec, the outcome, and the host port.
//!
//! The spec/result/error types and the [`SubLoopSpawner`] port live in this
//! foundation crate next to the rest of the module ABI, because a module needs
//! them to ask for a side loop and a runtime needs to provide one — and the
//! direction of that dependency is module -> port, host -> implementation.
//! The runtime owns the only implementation
//! (`apeireth_runtime::canonical::subloop::RuntimeSubLoopSpawner`).
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

use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedUsage, ToolResult};
use async_trait::async_trait;
use thiserror::Error;

use crate::module::ModuleInvocationError;

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
    #[must_use]
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

/// Port implemented by runtime hosts to spawn bounded SubLoops.
///
/// A module receives one through [`crate::module::ModuleContext::subloop`] for
/// the duration of a hook. It is turn-scoped: the handle must not be retained
/// as persistent module state.
#[async_trait]
pub trait SubLoopSpawner: Send + Sync {
    /// Spawn and execute a bounded SubLoop with a private transcript.
    async fn spawn(&self, spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_turn_spec_is_bounded_and_tool_free() {
        let spec = SubLoopSpec::single_turn("summarize this")
            .with_model("mock-model")
            .with_max_rounds(1);
        assert_eq!(spec.max_rounds, 1);
        assert!(spec.allowed_capabilities.is_empty());
        assert_eq!(spec.messages.len(), 1);
        assert_eq!(spec.model.as_deref(), Some("mock-model"));
    }

    #[test]
    fn an_allowlisted_capability_is_named_by_id() {
        let id = CapabilityId::new("tool.repo").unwrap();
        let spec = SubLoopSpec::single_turn("inspect").with_allowed_capability(id.clone());
        assert_eq!(spec.allowed_capabilities, vec![id]);
    }
}
