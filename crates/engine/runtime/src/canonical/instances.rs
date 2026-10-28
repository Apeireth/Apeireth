//! Instance ownership over the existing Runtime. Every turn uses Runtime.execute_outcome.
use super::accounting::UsageAccounting;
use super::dispatcher::{ToolDispatcher, ToolScope};
use super::{Runtime, RuntimeBuilder, RuntimeResult, TurnOutcome, TurnRequest};
use apeireth_core::kernel::{
    AgentId, CapabilityId, Lifecycle, ModuleInstanceId, PluginId, SessionId,
};
use apeireth_governance::TurnSecurityContext;
use apeireth_plugin::agents::*;
use apeireth_plugin::task_scope::ModuleTaskScope;
use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};
use std::time::Duration;
mod control;
mod profiles;

#[derive(Clone)]
pub struct ModuleAgentGrant {
    pub tools: BTreeSet<CapabilityId>,
    pub principals: Vec<AgentPrincipal>,
    pub max_instances: usize,
    pub max_depth: u8,
    pub budget: AgentBudget,
    pub security: Option<TurnSecurityContext>,
}
impl Default for ModuleAgentGrant {
    fn default() -> Self {
        Self {
            tools: BTreeSet::new(),
            principals: vec![],
            max_instances: 1,
            max_depth: 1,
            budget: AgentBudget::default(),
            security: None,
        }
    }
}
#[derive(Clone)]
pub struct AgentHostPolicy {
    pub max_instances: usize,
    pub max_records: usize,
    pub concurrent_turns: usize,
    pub provider_concurrency: usize,
    pub stop_timeout: Duration,
    pub max_budget: AgentBudget,
    pub runtime_budget: Option<super::accounting::SharedUsageBudget>,
    pub modules: BTreeMap<PluginId, ModuleAgentGrant>,
}
impl Default for AgentHostPolicy {
    fn default() -> Self {
        Self {
            max_instances: 16,
            max_records: 256,
            concurrent_turns: 4,
            provider_concurrency: 4,
            stop_timeout: Duration::from_secs(5),
            max_budget: AgentBudget {
                max_requests: 128,
                max_tokens: 131_072,
                max_tool_calls: 256,
                turn_timeout_ms: 120_000,
                max_cost_micros: None,
            },
            runtime_budget: None,
            modules: BTreeMap::new(),
        }
    }
}
pub struct AgentRuntime {
    inner: Arc<Inner>,
}
struct Inner {
    kernel: tokio::sync::RwLock<Runtime>,
    instances: Mutex<BTreeMap<AgentId, Arc<ManagedAgent>>>,
    policy: AgentHostPolicy,
    scope: ModuleTaskScope,
    turns: Arc<tokio::sync::Semaphore>,
    usage: Arc<UsageAccounting>,
    dispatcher: Arc<ToolDispatcher>,
    host_handles: std::sync::atomic::AtomicUsize,
    profile_switch: tokio::sync::Mutex<Option<Arc<dyn AgentInstance>>>,
}
struct Factory {
    binding: Arc<OnceLock<Weak<Inner>>>,
    ready: Arc<tokio::sync::Notify>,
}
struct BoundModule {
    binding: Arc<OnceLock<Weak<Inner>>>,
    ready: Arc<tokio::sync::Notify>,
    owner: AgentOwner,
    scope: ModuleTaskScope,
}
impl ModuleAgentFactory for Factory {
    fn bind(
        &self,
        definition: PluginId,
        instance: ModuleInstanceId,
        scope: ModuleTaskScope,
    ) -> Arc<dyn ModuleAgents> {
        Arc::new(BoundModule {
            binding: self.binding.clone(),
            ready: self.ready.clone(),
            owner: AgentOwner::Module {
                definition,
                instance,
            },
            scope,
        })
    }
}
#[async_trait]
impl ModuleAgents for BoundModule {
    async fn wait_ready(&self) -> AgentResult<()> {
        self.scope
            .run(async {
                loop {
                    let notified = self.ready.notified();
                    tokio::pin!(notified);
                    notified.as_mut().enable();
                    if let Some(binding) = self.binding.get() {
                        return binding
                            .upgrade()
                            .filter(|inner| inner.scope.is_open())
                            .map(|_| ())
                            .ok_or_else(|| error(AgentErrorCode::OwnerStopped, "runtime stopped"));
                    }
                    notified.await;
                }
            })
            .await
            .map_err(|_| {
                error(
                    AgentErrorCode::OwnerStopped,
                    "module stopped before runtime ready",
                )
            })?
    }

    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<Arc<dyn AgentInstance>> {
        if !self.scope.is_open() {
            return Err(error(AgentErrorCode::OwnerStopped, "module owner stopped"));
        }
        let inner = self.binding.get().and_then(Weak::upgrade).ok_or_else(|| {
            error(
                AgentErrorCode::NotReady,
                "managed host not bound; run agents after host initialization",
            )
        })?;
        let AgentOwner::Module { definition, .. } = &self.owner else {
            unreachable!()
        };
        let grant = inner
            .policy
            .modules
            .get(definition)
            .cloned()
            .ok_or_else(|| {
                error(
                    AgentErrorCode::ScopeDenied,
                    "host did not grant module agent creation",
                )
            })?;
        if config.tools.iter().any(|id| !grant.tools.contains(id)) {
            return Err(error(
                AgentErrorCode::ScopeDenied,
                "child tool scope exceeds module grant",
            ));
        }
        if config
            .principal
            .as_ref()
            .is_some_and(|p| !grant.principals.contains(p))
        {
            return Err(error(
                AgentErrorCode::NamespaceDenied,
                "child principal is outside module grant",
            ));
        }
        check_budget(&config.budget, &grant.budget)?;
        create(
            &inner,
            self.owner.clone(),
            self.scope.clone(),
            config,
            Some(grant),
        )
        .await
        .map(|a| a as Arc<dyn AgentInstance>)
    }
}
fn error(code: AgentErrorCode, message: impl Into<String>) -> AgentError {
    AgentError::new(code, message)
}
fn check_budget(request: &AgentBudget, ceiling: &AgentBudget) -> AgentResult<()> {
    if request.max_requests == 0
        || request.max_tokens == 0
        || request.max_tool_calls == 0
        || request.turn_timeout_ms == 0
        || request.max_requests > ceiling.max_requests
        || request.max_tokens > ceiling.max_tokens
        || request.max_tool_calls > ceiling.max_tool_calls
        || request.turn_timeout_ms > ceiling.turn_timeout_ms
        || ceiling
            .max_cost_micros
            .is_some_and(|max| request.max_cost_micros.is_none_or(|value| value > max))
    {
        return Err(error(
            AgentErrorCode::InvalidConfig,
            "invalid or excessive agent budget",
        ));
    }
    Ok(())
}
impl RuntimeBuilder {
    pub async fn build_agent_runtime(
        mut self,
        policy: AgentHostPolicy,
    ) -> RuntimeResult<AgentRuntime> {
        if policy.max_instances == 0
            || policy.max_records < policy.max_instances
            || policy.concurrent_turns == 0
            || policy.provider_concurrency == 0
            || policy.stop_timeout.is_zero()
        {
            return Err(super::RuntimeError::misconfigured(
                "invalid agent host policy",
            ));
        }
        let binding = Arc::new(OnceLock::new());
        let ready = Arc::new(tokio::sync::Notify::new());
        self.v1_protocol = true;
        self.agent_factory = Some(Arc::new(Factory {
            binding: binding.clone(),
            ready: ready.clone(),
        }));
        let kernel = self.build().await?;
        let usage = kernel.providers.accounting.clone();
        let dispatcher = kernel.dispatcher.clone();
        usage.set_runtime_budget(policy.runtime_budget.clone());
        for provider in kernel.providers.provider_ids() {
            usage.set_provider_concurrency(provider, policy.provider_concurrency)?;
        }
        let inner = Arc::new(Inner {
            kernel: tokio::sync::RwLock::new(kernel),
            instances: Mutex::new(BTreeMap::new()),
            turns: Arc::new(tokio::sync::Semaphore::new(policy.concurrent_turns)),
            scope: ModuleTaskScope::new(),
            usage,
            dispatcher,
            policy,
            host_handles: std::sync::atomic::AtomicUsize::new(1),
            profile_switch: tokio::sync::Mutex::new(None),
        });
        let _ = binding.set(Arc::downgrade(&inner));
        ready.notify_waiters();
        Ok(AgentRuntime { inner })
    }
}
impl AgentRuntime {
    /// Shared read access is concurrent across model requests; control writes are bounded.
    pub async fn kernel(&self) -> tokio::sync::RwLockReadGuard<'_, Runtime> {
        self.inner.kernel.read().await
    }
    pub async fn spawn_agent(
        &self,
        owner: impl Into<String>,
        config: AgentConfig,
    ) -> AgentResult<Arc<dyn AgentInstance>> {
        create(
            &self.inner,
            AgentOwner::Host(owner.into()),
            self.inner.scope.clone(),
            config,
            None,
        )
        .await
        .map(|a| a as Arc<dyn AgentInstance>)
    }
    pub fn snapshot(&self, id: AgentId) -> AgentResult<AgentSnapshot> {
        self.inner
            .instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .map(|agent| agent.snapshot())
            .ok_or_else(|| error(AgentErrorCode::NotFound, "unknown agent"))
    }
    pub fn instances(&self) -> Vec<AgentSnapshot> {
        self.inner
            .instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|agent| agent.snapshot())
            .collect()
    }
    pub fn usage(&self, owner: Option<&AgentOwner>) -> AgentUsage {
        self.inner.usage.aggregate(owner)
    }
    pub fn requests(&self, id: AgentId) -> AgentResult<Vec<super::accounting::RequestUsageFact>> {
        let session = self.snapshot(id)?.session;
        Ok(self.inner.usage.recent_requests(session))
    }
}
async fn create(
    inner: &Arc<Inner>,
    owner: AgentOwner,
    owner_scope: ModuleTaskScope,
    config: AgentConfig,
    grant: Option<ModuleAgentGrant>,
) -> AgentResult<Arc<ManagedAgent>> {
    check_budget(&config.budget, &inner.policy.max_budget)?;
    if !inner.scope.is_open() || !owner_scope.is_open() {
        return Err(error(
            AgentErrorCode::OwnerStopped,
            "owner or runtime stopped",
        ));
    }
    let kernel = inner.kernel.read().await;
    if let AgentOwner::Module {
        definition,
        instance,
    } = &owner
    {
        if kernel.plugins.instance_id(definition).ok() != Some(*instance)
            || kernel.plugins.state(definition).ok() != Some(Lifecycle::Active)
        {
            return Err(error(
                AgentErrorCode::OwnerStopped,
                "module activation is no longer running",
            ));
        }
    }
    if config
        .provider
        .as_ref()
        .is_some_and(|id| !kernel.providers.provider_ids().contains(id))
    {
        return Err(error(
            AgentErrorCode::InvalidConfig,
            "provider is not registered",
        ));
    }
    let visible = kernel.capabilities.capabilities();
    if config.tools.iter().any(|id| {
        !visible.iter().any(|tool| tool.id() == id)
            && !kernel
                .capabilities
                .controls()
                .iter()
                .any(|tool| tool.id() == id)
    }) {
        return Err(error(
            AgentErrorCode::ScopeDenied,
            "requested tool is not registered",
        ));
    }
    let mut instances = inner.instances.lock().unwrap_or_else(|e| e.into_inner());
    if instances.len() >= inner.policy.max_records
        || instances.values().filter(|a| a.scope.is_open()).count() >= inner.policy.max_instances
    {
        return Err(error(
            AgentErrorCode::LimitExceeded,
            "instance or audit record limit reached",
        ));
    }
    let depth = match config.parent {
        Some(parent) => instances
            .get(&parent)
            .ok_or_else(|| {
                error(
                    AgentErrorCode::NotFound,
                    "parent agent not managed by this host",
                )
            })?
            .depth
            .saturating_add(1),
        None => {
            if matches!(owner, AgentOwner::Host(_)) {
                0
            } else {
                1
            }
        }
    };
    if let Some(grant) = &grant {
        if depth > grant.max_depth
            || instances
                .values()
                .filter(|a| a.owner == owner && a.scope.is_open())
                .count()
                >= grant.max_instances
        {
            return Err(error(
                AgentErrorCode::LimitExceeded,
                "module child depth or instance limit reached",
            ));
        }
    }
    let id = AgentId::new();
    let session = SessionId::new();
    let scope = ModuleTaskScope::joined(vec![owner_scope, inner.scope.clone()]);
    if let Some(grant) = &grant {
        inner
            .usage
            .bind_owner_budget(owner.clone(), (&grant.budget).into());
    }
    inner
        .usage
        .register(session, id, owner.clone(), config.clone(), scope.clone())
        .map_err(|e| error(AgentErrorCode::InvalidConfig, e.to_string()))?;
    inner
        .dispatcher
        .bind_instance(session, id, config.principal.clone(), scope.clone());
    inner.dispatcher.set_scope(
        session,
        ToolScope::Only(config.tools.iter().cloned().collect()),
    );
    let agent = Arc::new(ManagedAgent {
        id,
        session,
        owner,
        config,
        depth,
        scope,
        state: Mutex::new((AgentStatus::Ready, None)),
        turn: tokio::sync::Mutex::new(()),
        inner: Arc::downgrade(inner),
        security: grant.and_then(|g| g.security),
        usage: inner.usage.clone(),
        sessions: kernel.sessions.clone(),
        queued: std::sync::atomic::AtomicUsize::new(0),
    });
    instances.insert(id, agent.clone());
    Ok(agent)
}
struct ManagedAgent {
    id: AgentId,
    session: SessionId,
    owner: AgentOwner,
    config: AgentConfig,
    depth: u8,
    scope: ModuleTaskScope,
    state: Mutex<(AgentStatus, Option<String>)>,
    turn: tokio::sync::Mutex<()>,
    inner: Weak<Inner>,
    security: Option<TurnSecurityContext>,
    usage: Arc<UsageAccounting>,
    sessions: Arc<super::session::SessionManager>,
    queued: std::sync::atomic::AtomicUsize,
}
struct Running<'a> {
    agent: &'a ManagedAgent,
    completed: bool,
}
impl Drop for Running<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.agent.scope.close();
            *self.agent.state.lock().unwrap_or_else(|e| e.into_inner()) = (
                AgentStatus::Stopping,
                Some("task cancelled; interrupted context is not reused".into()),
            );
        }
    }
}
#[async_trait]
impl AgentInstance for ManagedAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn snapshot(&self) -> AgentSnapshot {
        let (mut status, last_error) = self.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if !self.scope.is_open() && status != AgentStatus::Destroyed {
            status = if self.scope.active_tasks() == 0 {
                AgentStatus::Stopped
            } else {
                AgentStatus::Stopping
            };
        }
        AgentSnapshot {
            id: self.id,
            session: self.session,
            owner: self.owner.clone(),
            principal: self.config.principal.clone(),
            status,
            usage: self.usage.snapshot(self.session),
            last_error,
        }
    }
    async fn run(&self, message: String) -> AgentResult<AgentReply> {
        if message.len() > 128 * 1024 {
            return Err(error(
                AgentErrorCode::InvalidConfig,
                "agent task exceeds input limit",
            ));
        }
        let inner = self
            .inner
            .upgrade()
            .ok_or_else(|| error(AgentErrorCode::OwnerStopped, "runtime dropped"))?;
        struct Queued<'a>(&'a std::sync::atomic::AtomicUsize);
        impl Drop for Queued<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
            }
        }
        let queued = self
            .queued
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let _queued = Queued(&self.queued);
        if queued >= 4 {
            return Err(error(
                AgentErrorCode::LimitExceeded,
                "agent task queue limit reached",
            ));
        }
        let work = async {
            let _turn = self.turn.lock().await;
            {
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                if !matches!(state.0, AgentStatus::Ready) {
                    return Err(error(
                        AgentErrorCode::Paused,
                        "instance must be ready before accepting a turn",
                    ));
                }
                state.0 = AgentStatus::Running;
            }
            let mut running = Running {
                agent: self,
                completed: false,
            };
            let _permit = inner
                .turns
                .acquire()
                .await
                .map_err(|_| error(AgentErrorCode::OwnerStopped, "runtime stopped"))?;
            let kernel = inner.kernel.read().await;
            let mut request = TurnRequest::new(self.session, message);
            request.model = self.config.model.clone();
            request.security_context = self.security.clone();
            let result = kernel.execute_outcome(request).await;
            running.completed = true;
            match result {
                Ok(TurnOutcome::Completed(reply)) => {
                    *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::Ready, None);
                    Ok(AgentReply {
                        text: reply.text,
                        request: reply.request,
                        usage: inner.usage.snapshot(self.session),
                    })
                }
                Ok(TurnOutcome::PendingApproval(_)) => {
                    *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::WaitingApproval, None);
                    Err(error(
                        AgentErrorCode::PendingApproval,
                        "host approval required; no automatic approval granted",
                    ))
                }
                Err(e) => {
                    let code = if matches!(&e,super::RuntimeError::Denied{hook,..} if hook=="instance_budget")
                    {
                        AgentErrorCode::BudgetExceeded
                    } else {
                        AgentErrorCode::Provider
                    };
                    *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::Failed, Some(e.to_string()));
                    Err(error(code, e.to_string()))
                }
            }
        };
        match tokio::time::timeout(
            Duration::from_millis(self.config.budget.turn_timeout_ms),
            self.scope.run(work),
        )
        .await
        {
            Ok(Ok(result)) => result,
            _ => {
                self.scope.close();
                Err(error(
                    AgentErrorCode::Cancelled,
                    "agent cancelled or turn deadline exceeded",
                ))
            }
        }
    }
    fn pause(&self) -> AgentResult<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.0 != AgentStatus::Ready {
            return Err(error(
                AgentErrorCode::NotReady,
                "pause is defined at a turn boundary",
            ));
        }
        state.0 = AgentStatus::Paused;
        Ok(())
    }
    fn resume(&self) -> AgentResult<()> {
        if !self.scope.is_open() {
            return Err(error(
                AgentErrorCode::OwnerStopped,
                "stopped instances require a new activation",
            ));
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !matches!(state.0, AgentStatus::Paused | AgentStatus::Failed) {
            return Err(error(
                AgentErrorCode::NotReady,
                "instance is not paused or failed",
            ));
        }
        state.0 = AgentStatus::Ready;
        Ok(())
    }
    async fn stop(&self) -> AgentResult<()> {
        if self.state.lock().unwrap_or_else(|e| e.into_inner()).0 == AgentStatus::Destroyed {
            return Ok(());
        }
        self.scope.close();
        let timeout = self
            .inner
            .upgrade()
            .map(|i| i.policy.stop_timeout)
            .unwrap_or(Duration::from_secs(5));
        tokio::time::timeout(timeout, self.scope.close_and_wait())
            .await
            .map_err(|_| {
                error(
                    AgentErrorCode::NotReady,
                    "agent draining; resources remain owned",
                )
            })?;
        self.state.lock().unwrap_or_else(|e| e.into_inner()).0 = AgentStatus::Stopped;
        Ok(())
    }
    async fn destroy(&self) -> AgentResult<()> {
        self.stop().await?;
        self.sessions
            .delete(&self.session)
            .await
            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))?;
        self.state.lock().unwrap_or_else(|e| e.into_inner()).0 = AgentStatus::Destroyed;
        Ok(())
    }
}

impl Clone for AgentRuntime {
    fn clone(&self) -> Self {
        self.inner
            .host_handles
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl Drop for AgentRuntime {
    fn drop(&mut self) {
        if self
            .inner
            .host_handles
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel)
            == 1
        {
            self.inner.scope.close();
        }
    }
}
