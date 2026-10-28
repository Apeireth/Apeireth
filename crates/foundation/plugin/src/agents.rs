//! Owner-bound instance ports. Providers, loops and storage remain above this layer.
use crate::task_scope::ModuleTaskScope;
use apeireth_core::kernel::{
    AgentId, CapabilityId, ModuleInstanceId, NamespaceId, PluginId, ProfileId, RequestId, SessionId,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AgentOwner {
    Host(String),
    Module {
        definition: PluginId,
        instance: ModuleInstanceId,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPrincipal {
    pub profile: ProfileId,
    pub namespace: NamespaceId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBudget {
    pub max_requests: u64,
    pub max_tokens: u64,
    pub max_tool_calls: u64,
    pub max_cost_micros: Option<u64>,
    pub turn_timeout_ms: u64,
}
impl Default for AgentBudget {
    fn default() -> Self {
        Self {
            max_requests: 8,
            max_tokens: 8192,
            max_tool_calls: 32,
            max_cost_micros: None,
            turn_timeout_ms: 120_000,
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: Option<String>,
    pub provider: Option<CapabilityId>,
    /// Empty means no tools; neither registration nor parenthood grants access.
    pub tools: Vec<CapabilityId>,
    pub principal: Option<AgentPrincipal>,
    pub parent: Option<AgentId>,
    pub budget: AgentBudget,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Ready,
    Running,
    Paused,
    WaitingApproval,
    Stopping,
    Stopped,
    Failed,
    Destroyed,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub requests: u64,
    pub tool_calls: u64,
    pub latency_millis: u64,
    pub estimated_cost_micros: u64,
    pub unpriced_requests: u64,
    pub unreported_usage_requests: u64,
    pub unknown_cache_requests: u64,
    pub errors: u64,
    pub provider_errors: u64,
    pub cancellations: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: AgentId,
    pub session: SessionId,
    pub owner: AgentOwner,
    pub principal: Option<AgentPrincipal>,
    pub status: AgentStatus,
    pub usage: AgentUsage,
    pub last_error: Option<String>,
}
#[derive(Debug, Clone)]
pub struct AgentReply {
    pub text: String,
    pub request: RequestId,
    pub usage: AgentUsage,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentErrorCode {
    NotReady,
    OwnerStopped,
    LimitExceeded,
    ScopeDenied,
    NamespaceDenied,
    BudgetExceeded,
    Cancelled,
    Paused,
    Provider,
    PendingApproval,
    InvalidConfig,
    NotFound,
}
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{code:?}: {message}")]
pub struct AgentError {
    pub code: AgentErrorCode,
    pub message: String,
}
impl AgentError {
    pub fn new(code: AgentErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
pub type AgentResult<T> = Result<T, AgentError>;

#[async_trait]
pub trait AgentInstance: Send + Sync {
    fn id(&self) -> AgentId;
    fn snapshot(&self) -> AgentSnapshot;
    async fn run(&self, message: String) -> AgentResult<AgentReply>;
    fn pause(&self) -> AgentResult<()>;
    fn resume(&self) -> AgentResult<()>;
    async fn stop(&self) -> AgentResult<()>;
    async fn destroy(&self) -> AgentResult<()>;
}
#[async_trait]
pub trait ModuleAgents: Send + Sync {
    /// Wait from an owned background task, never synchronously inside initialize/start.
    async fn wait_ready(&self) -> AgentResult<()> {
        Ok(())
    }
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<Arc<dyn AgentInstance>>;
}
/// Supplied only by a host. A scoped PluginContext does not expose this factory.
pub trait ModuleAgentFactory: Send + Sync {
    fn bind(
        &self,
        definition: PluginId,
        instance: ModuleInstanceId,
        scope: ModuleTaskScope,
    ) -> Arc<dyn ModuleAgents>;
}

impl AgentUsage {
    pub fn accumulate(&mut self, other: &Self) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.cached_tokens = self.cached_tokens.saturating_add(other.cached_tokens);
        self.requests = self.requests.saturating_add(other.requests);
        self.tool_calls = self.tool_calls.saturating_add(other.tool_calls);
        self.latency_millis = self.latency_millis.saturating_add(other.latency_millis);
        self.estimated_cost_micros = self
            .estimated_cost_micros
            .saturating_add(other.estimated_cost_micros);
        self.unpriced_requests = self
            .unpriced_requests
            .saturating_add(other.unpriced_requests);
        self.unreported_usage_requests = self
            .unreported_usage_requests
            .saturating_add(other.unreported_usage_requests);
        self.unknown_cache_requests = self
            .unknown_cache_requests
            .saturating_add(other.unknown_cache_requests);
        self.errors = self.errors.saturating_add(other.errors);
        self.provider_errors = self.provider_errors.saturating_add(other.provider_errors);
        self.cancellations = self.cancellations.saturating_add(other.cancellations);
    }
}

impl AgentUsage {
    pub fn complete_cost_estimate(&self) -> Option<u64> {
        (self.unpriced_requests == 0 && self.unreported_usage_requests == 0)
            .then_some(self.estimated_cost_micros)
    }
    pub fn complete_token_count(&self) -> Option<u64> {
        (self.unreported_usage_requests == 0)
            .then_some(self.input_tokens.saturating_add(self.output_tokens))
    }
    pub fn complete_cache_count(&self) -> Option<u64> {
        (self.unknown_cache_requests == 0).then_some(self.cached_tokens)
    }
}
