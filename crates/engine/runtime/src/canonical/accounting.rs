//! Request facts and budgets at actual provider/tool boundaries, never a second loop.
use super::quota::{CostRateMicros, CostRateTable};
use super::{RuntimeError, RuntimeResult};
use apeireth_core::kernel::{AgentId, CapabilityId, RequestId, SessionId, TaskId};
use apeireth_plugin::agents::{AgentBudget, AgentConfig, AgentOwner, AgentUsage};
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_protocol::canonical::NormalizedUsage;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestUsageFact {
    pub id: TaskId,
    pub request: RequestId,
    pub agent: AgentId,
    pub owner: AgentOwner,
    pub provider: CapabilityId,
    pub reported: Option<NormalizedUsage>,
    pub estimated_cost_micros: Option<u64>,
    pub latency_millis: u64,
    pub failed: bool,
    pub cancelled: bool,
}
#[derive(Debug, Clone)]
pub struct SharedUsageBudget {
    pub max_requests: u64,
    pub max_tokens: u64,
    pub max_tool_calls: u64,
    pub max_cost_micros: Option<u64>,
}
impl From<&AgentBudget> for SharedUsageBudget {
    fn from(b: &AgentBudget) -> Self {
        Self {
            max_requests: b.max_requests,
            max_tokens: b.max_tokens,
            max_tool_calls: b.max_tool_calls,
            max_cost_micros: b.max_cost_micros,
        }
    }
}
struct Account {
    agent: AgentId,
    owner: AgentOwner,
    config: Option<AgentConfig>,
    instance_budget: bool,
    scope: Option<ModuleTaskScope>,
    usage: AgentUsage,
    facts: VecDeque<RequestUsageFact>,
}
impl Account {
    fn legacy(session: SessionId) -> Self {
        Self {
            agent: AgentId::from_uuid(session.as_uuid()),
            owner: AgentOwner::Host("legacy".into()),
            config: None,
            instance_budget: false,
            scope: None,
            usage: AgentUsage::default(),
            facts: VecDeque::new(),
        }
    }
}
struct ProviderSlot {
    permits: Arc<tokio::sync::Semaphore>,
    backoff: Mutex<(u32, Option<Instant>, bool)>,
}
#[derive(Default)]
pub struct UsageAccounting {
    accounts: Mutex<BTreeMap<SessionId, Account>>,
    rates: RwLock<Option<Arc<dyn CostRateTable>>>,
    owner_limits: Mutex<BTreeMap<AgentOwner, SharedUsageBudget>>,
    runtime_limit: RwLock<Option<SharedUsageBudget>>,
    limits: RwLock<BTreeMap<CapabilityId, Arc<ProviderSlot>>>,
}
fn denied(reason: impl Into<String>) -> RuntimeError {
    RuntimeError::Denied {
        hook: "instance_budget".into(),
        reason: reason.into(),
    }
}
impl UsageAccounting {
    pub fn set_runtime_budget(&self, budget: Option<SharedUsageBudget>) {
        *self
            .runtime_limit
            .write()
            .unwrap_or_else(|e| e.into_inner()) = budget;
    }
    pub fn bind_owner_budget(&self, owner: AgentOwner, budget: SharedUsageBudget) {
        self.owner_limits
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(owner)
            .or_insert(budget);
    }
    fn shared_usage(
        accounts: &BTreeMap<SessionId, Account>,
        owner: Option<&AgentOwner>,
    ) -> AgentUsage {
        let mut usage = AgentUsage::default();
        for a in accounts
            .values()
            .filter(|a| owner.is_none_or(|o| &a.owner == o))
        {
            usage.accumulate(&a.usage);
        }
        usage
    }
    fn shared_checks(
        &self,
        accounts: &BTreeMap<SessionId, Account>,
        owner: &AgentOwner,
        model: &str,
        rate: Option<CostRateMicros>,
    ) -> RuntimeResult<()> {
        if let Some(budget) = self
            .owner_limits
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(owner)
        {
            Self::check_usage(
                budget,
                &Self::shared_usage(accounts, Some(owner)),
                model,
                rate,
            )?;
        }
        if let Some(budget) = self
            .runtime_limit
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            Self::check_usage(budget, &Self::shared_usage(accounts, None), model, rate)?;
        }
        Ok(())
    }
    pub fn set_rates(&self, rates: Option<Arc<dyn CostRateTable>>) {
        *self.rates.write().unwrap_or_else(|e| e.into_inner()) = rates;
    }
    pub fn set_provider_concurrency(
        &self,
        provider: CapabilityId,
        limit: usize,
    ) -> RuntimeResult<()> {
        if limit == 0 || limit > tokio::sync::Semaphore::MAX_PERMITS {
            return Err(denied("invalid provider concurrency"));
        }
        let mut limits = self.limits.write().unwrap_or_else(|e| e.into_inner());
        if limits.contains_key(&provider) {
            return Err(denied(
                "provider limit already bound; rebuild host to replace it",
            ));
        }
        limits.insert(
            provider,
            Arc::new(ProviderSlot {
                permits: Arc::new(tokio::sync::Semaphore::new(limit)),
                backoff: Mutex::new((0, None, false)),
            }),
        );
        Ok(())
    }
    pub async fn permit(
        &self,
        provider: &CapabilityId,
    ) -> RuntimeResult<Option<tokio::sync::OwnedSemaphorePermit>> {
        let permit = self
            .limits
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(provider)
            .cloned();
        let Some(slot) = permit else {
            return Ok(None);
        };
        let permit = slot
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| denied("provider stopping"))?;
        loop {
            let (_, next, blocked) = *slot.backoff.lock().unwrap_or_else(|e| e.into_inner());
            if blocked {
                return Err(denied(
                    "provider retry deadline is outside supported bounds",
                ));
            }
            let Some(next) = next.filter(|next| *next > Instant::now()) else {
                break;
            };
            tokio::time::sleep_until(tokio::time::Instant::from_std(next)).await;
        }
        Ok(Some(permit))
    }
    pub(crate) fn cooling(&self, provider: &CapabilityId) -> bool {
        self.limits
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(provider)
            .is_some_and(|slot| {
                let (_, next, blocked) = *slot.backoff.lock().unwrap_or_else(|e| e.into_inner());
                blocked || next.is_some_and(|time| time > Instant::now())
            })
    }
    pub(crate) fn can_probe(&self, provider: &CapabilityId) -> bool {
        self.limits
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(provider)
            .is_some_and(|slot| {
                let (failures, next, blocked) =
                    *slot.backoff.lock().unwrap_or_else(|e| e.into_inner());
                failures > 0 && !blocked && next.is_some_and(|time| time <= Instant::now())
            })
    }
    pub(crate) fn provider_outcome(
        &self,
        provider: &CapabilityId,
        error: Option<&apeireth_plugin::ProviderError>,
    ) {
        let limits = self.limits.read().unwrap_or_else(|e| e.into_inner());
        let Some(slot) = limits.get(provider) else {
            return;
        };
        let mut state = slot.backoff.lock().unwrap_or_else(|e| e.into_inner());
        match error {
            None => *state = (0, None, false),
            Some(error) if error.is_retryable() => {
                state.0 = state.0.saturating_add(1);
                let base = 100_u64
                    .saturating_mul(1_u64 << state.0.saturating_sub(1).min(6))
                    .min(5_000);
                let wait = match error {
                    apeireth_plugin::ProviderError::RateLimited { retry_after_ms, .. } => {
                        base.max(*retry_after_ms)
                    }
                    _ => base,
                };
                state.1 = Instant::now().checked_add(std::time::Duration::from_millis(wait));
                state.2 = state.1.is_none();
            }
            Some(_) => {}
        }
    }
    pub fn register(
        &self,
        session: SessionId,
        agent: AgentId,
        owner: AgentOwner,
        config: AgentConfig,
        scope: ModuleTaskScope,
    ) -> RuntimeResult<()> {
        let mut accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        if accounts.contains_key(&session) {
            return Err(denied("session already has accounting ownership"));
        }
        accounts.insert(
            session,
            Account {
                agent,
                owner,
                config: Some(config),
                instance_budget: true,
                scope: Some(scope),
                usage: AgentUsage::default(),
                facts: VecDeque::new(),
            },
        );
        Ok(())
    }
    /// Trusted Host selection: keep the existing session quota as the spend
    /// policy. Identity, cancellation and unknown usage facts stay managed.
    pub(crate) fn use_session_quotas(&self, session: SessionId) {
        self.accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&session)
            .expect("registered main")
            .instance_budget = false;
    }
    pub(crate) fn attach_main_session(
        &self,
        session: SessionId,
        main: SessionId,
    ) -> RuntimeResult<()> {
        let mut accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        if accounts.contains_key(&session) {
            return Err(denied("session already has accounting ownership"));
        }
        let source = accounts.get(&main).expect("registered main");
        let account = Account {
            agent: source.agent,
            owner: source.owner.clone(),
            config: source.config.clone(),
            instance_budget: source.instance_budget,
            scope: source.scope.clone(),
            usage: AgentUsage::default(),
            facts: VecDeque::new(),
        };
        accounts.insert(session, account);
        Ok(())
    }
    pub(crate) fn agent_usage(&self, agent: AgentId) -> AgentUsage {
        let accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        let mut usage = AgentUsage::default();
        for account in accounts.values().filter(|account| account.agent == agent) {
            usage.accumulate(&account.usage);
        }
        usage
    }
    pub(crate) fn agent_requests(&self, agent: AgentId) -> Vec<RequestUsageFact> {
        self.accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|account| account.agent == agent)
            .flat_map(|account| account.facts.iter().cloned())
            .collect()
    }
    pub fn requested_provider(&self, session: SessionId) -> Option<CapabilityId> {
        self.accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .and_then(|a| a.config.as_ref())
            .and_then(|c| c.provider.clone())
    }
    fn check(account: &Account, model: &str, rate: Option<CostRateMicros>) -> RuntimeResult<()> {
        if account.scope.as_ref().is_some_and(|scope| !scope.is_open()) {
            return Err(denied("agent or owner is stopped"));
        }
        let Some(config) = account.config.as_ref().filter(|_| account.instance_budget) else {
            return Ok(());
        };
        Self::check_usage(
            &SharedUsageBudget::from(&config.budget),
            &account.usage,
            model,
            rate,
        )
    }
    fn check_usage(
        b: &SharedUsageBudget,
        u: &AgentUsage,
        model: &str,
        rate: Option<CostRateMicros>,
    ) -> RuntimeResult<()> {
        if u.requests >= b.max_requests
            || u.input_tokens.saturating_add(u.output_tokens) >= b.max_tokens
        {
            return Err(denied("agent request/token budget exhausted"));
        }
        if u.unreported_usage_requests > u.provider_errors {
            return Err(denied(
                "reported usage is unavailable; further budgeted requests are refused",
            ));
        }
        if let Some(max) = b.max_cost_micros {
            if rate.is_none() || u.unpriced_requests > 0 {
                return Err(denied(format!("no usable cost estimate for {model}")));
            }
            if u.estimated_cost_micros >= max {
                return Err(denied("estimated cost budget exhausted"));
            }
        }
        Ok(())
    }
    pub fn begin(
        self: &Arc<Self>,
        session: SessionId,
        request: RequestId,
        provider: CapabilityId,
        model: &str,
    ) -> RuntimeResult<RequestMeter> {
        self.begin_gated(session, request, provider, model, None)
    }
    pub(crate) fn begin_gated(
        self: &Arc<Self>,
        session: SessionId,
        request: RequestId,
        provider: CapabilityId,
        model: &str,
        gate: Option<&dyn apeireth_plugin::agents::AgentRequestGate>,
    ) -> RuntimeResult<RequestMeter> {
        let rate = self
            .rates
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|r| r.rate_for(model));
        let mut accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        let owner = accounts
            .entry(session)
            .or_insert_with(|| Account::legacy(session))
            .owner
            .clone();
        self.shared_checks(&accounts, &owner, model, rate)?;
        let account = accounts.get_mut(&session).expect("account just inserted");
        Self::check(account, model, rate)?;
        if let Some(gate) = gate {
            gate.reserve()?;
        }
        account.usage.requests = account.usage.requests.saturating_add(1);
        Ok(RequestMeter {
            ledger: self.clone(),
            session,
            request,
            agent: account.agent,
            owner: account.owner.clone(),
            provider,
            rate,
            started: Instant::now(),
            done: false,
        })
    }
    pub fn output_budget(&self, session: SessionId) -> Option<u32> {
        let accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        let account = accounts.get(&session)?;
        let mut remaining = account.config.as_ref().map(|c| {
            c.budget.max_tokens.saturating_sub(
                account
                    .usage
                    .input_tokens
                    .saturating_add(account.usage.output_tokens),
            )
        });
        let mut clamp = |budget: &SharedUsageBudget, usage: AgentUsage| {
            let value = budget
                .max_tokens
                .saturating_sub(usage.input_tokens.saturating_add(usage.output_tokens));
            remaining = Some(remaining.unwrap_or(value).min(value));
        };
        if let Some(budget) = self
            .owner_limits
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&account.owner)
        {
            clamp(budget, Self::shared_usage(&accounts, Some(&account.owner)));
        }
        if let Some(budget) = self
            .runtime_limit
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            clamp(budget, Self::shared_usage(&accounts, None));
        }
        remaining.map(|tokens| tokens.min(u64::from(u32::MAX)) as u32)
    }
    pub(crate) fn owner(&self, session: SessionId) -> Option<AgentOwner> {
        self.accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .map(|account| account.owner.clone())
    }
    pub fn tool_call(&self, session: SessionId) -> RuntimeResult<()> {
        let mut accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        let owner = accounts
            .entry(session)
            .or_insert_with(|| Account::legacy(session))
            .owner
            .clone();
        if self
            .owner_limits
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&owner)
            .is_some_and(|b| {
                Self::shared_usage(&accounts, Some(&owner)).tool_calls >= b.max_tool_calls
            })
            || self
                .runtime_limit
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .is_some_and(|b| Self::shared_usage(&accounts, None).tool_calls >= b.max_tool_calls)
        {
            return Err(denied("shared tool budget exhausted"));
        }
        let a = accounts.get_mut(&session).expect("account just inserted");
        if a.scope.as_ref().is_some_and(|s| !s.is_open()) {
            return Err(denied("agent stopped"));
        }
        if a.config
            .as_ref()
            .filter(|_| a.instance_budget)
            .is_some_and(|c| a.usage.tool_calls >= c.budget.max_tool_calls)
        {
            return Err(denied("agent tool budget exhausted"));
        }
        a.usage.tool_calls = a.usage.tool_calls.saturating_add(1);
        Ok(())
    }
    pub fn tool_meter(self: &Arc<Self>, session: SessionId) -> ToolMeter {
        ToolMeter {
            ledger: self.clone(),
            session,
            settled: false,
            failed: false,
        }
    }
    fn tool_outcome(&self, session: SessionId, failed: bool, cancelled: bool) {
        let mut accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(a) = accounts.get_mut(&session) {
            a.usage.errors = a.usage.errors.saturating_add(u64::from(failed));
            a.usage.cancellations = a.usage.cancellations.saturating_add(u64::from(cancelled));
        }
    }
    pub fn snapshot(&self, session: SessionId) -> AgentUsage {
        self.accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .map(|a| a.usage.clone())
            .unwrap_or_default()
    }
    pub fn recent_requests(&self, session: SessionId) -> Vec<RequestUsageFact> {
        self.accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .map(|a| a.facts.iter().cloned().collect())
            .unwrap_or_default()
    }
    pub fn aggregate(&self, owner: Option<&AgentOwner>) -> AgentUsage {
        let accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        let mut result = AgentUsage::default();
        for account in accounts
            .values()
            .filter(|a| owner.is_none_or(|o| o == &a.owner))
        {
            result.accumulate(&account.usage);
        }
        result
    }
}
pub struct RequestMeter {
    ledger: Arc<UsageAccounting>,
    session: SessionId,
    request: RequestId,
    agent: AgentId,
    owner: AgentOwner,
    provider: CapabilityId,
    rate: Option<CostRateMicros>,
    started: Instant,
    done: bool,
}
impl RequestMeter {
    pub fn finish(&mut self, usage: Option<&NormalizedUsage>, failed: bool) {
        self.record(usage, failed, false);
    }
    fn record(&mut self, usage: Option<&NormalizedUsage>, failed: bool, cancelled: bool) {
        if self.done {
            return;
        }
        self.done = true;
        // The old normalized ABI cannot distinguish missing usage from an all-zero
        // report without a cache column. Preserve that uncertainty explicitly.
        let reported = usage
            .filter(|u| {
                u.total_tokens > 0
                    || u.prompt_tokens > 0
                    || u.completion_tokens > 0
                    || u.prompt_cache_hit_tokens.is_some()
            })
            .cloned();
        let cost = reported.as_ref().and_then(|u| {
            self.rate
                .map(|r| r.cost_micros(u64::from(u.prompt_tokens), u64::from(u.completion_tokens)))
        });
        let fact = RequestUsageFact {
            id: TaskId::new(),
            request: self.request,
            agent: self.agent,
            owner: self.owner.clone(),
            provider: self.provider.clone(),
            reported,
            estimated_cost_micros: cost,
            latency_millis: u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            failed,
            cancelled,
        };
        let mut accounts = self
            .ledger
            .accounts
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let a = accounts
            .get_mut(&self.session)
            .expect("meter retains account");
        let u = &mut a.usage;
        if let Some(report) = &fact.reported {
            u.input_tokens = u
                .input_tokens
                .saturating_add(u64::from(report.prompt_tokens));
            u.output_tokens = u
                .output_tokens
                .saturating_add(u64::from(report.completion_tokens));
            u.cached_tokens = u
                .cached_tokens
                .saturating_add(u64::from(report.prompt_cache_hit_tokens.unwrap_or(0)));
        } else {
            u.unreported_usage_requests = u.unreported_usage_requests.saturating_add(1);
        }
        if fact
            .reported
            .as_ref()
            .is_none_or(|r| r.prompt_cache_hit_tokens.is_none())
        {
            u.unknown_cache_requests = u.unknown_cache_requests.saturating_add(1);
        }
        match cost {
            Some(c) => u.estimated_cost_micros = u.estimated_cost_micros.saturating_add(c),
            None => u.unpriced_requests = u.unpriced_requests.saturating_add(1),
        }
        u.errors = u.errors.saturating_add(u64::from(failed));
        u.provider_errors = u.provider_errors.saturating_add(u64::from(failed));
        u.cancellations = u.cancellations.saturating_add(u64::from(cancelled));
        u.latency_millis = u.latency_millis.saturating_add(fact.latency_millis);
        if a.facts.len() == 64 {
            a.facts.pop_front();
        }
        a.facts.push_back(fact);
    }
}
impl Drop for RequestMeter {
    fn drop(&mut self) {
        if !self.done {
            self.record(None, false, true);
        }
    }
}

/// Counts resource executions, not permission refusals or duplicate events.
pub struct ToolMeter {
    ledger: Arc<UsageAccounting>,
    session: SessionId,
    settled: bool,
    failed: bool,
}
impl ToolMeter {
    pub fn settle(&mut self, failed: bool) {
        if failed && !self.failed {
            self.ledger.tool_outcome(self.session, true, false);
            self.failed = true;
        }
        self.settled = true;
    }
}
impl Drop for ToolMeter {
    fn drop(&mut self) {
        if !self.settled {
            self.ledger.tool_outcome(self.session, false, true);
        }
    }
}
