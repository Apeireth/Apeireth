//! Common tool invocation boundary. Catalogs remain owned by the registries.
//! Governance admission stays in the canonical runtime, including frozen approval.

use apeireth_core::kernel::{AgentId, PluginId};
use apeireth_plugin::{
    ToolAuthorization, ToolCaller, ToolCancellation, ToolConcurrency, ToolExecutionContext,
    ToolExecutionPolicy, ToolPlatform,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use apeireth_core::kernel::{CapabilityId, SessionId, TaskId};
use apeireth_plugin::{task_scope::ModuleTaskScope, FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{
    NormalizedTool, ToolCall, ToolEventSourceV1, ToolEventV1, ToolResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// An additional restriction, never a permission grant. Missing scope allows the
/// legacy catalog; an empty Only set exposes no tool, including controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolScope {
    All,
    Only(BTreeSet<CapabilityId>),
}

impl ToolScope {
    pub fn allows(&self, id: &CapabilityId) -> bool {
        match self {
            Self::All => true,
            Self::Only(ids) => ids.contains(id),
        }
    }
}

/// Runtime ceilings; individual tools may enforce stricter limits.
#[derive(Debug, Clone)]
pub struct DispatchLimits {
    pub timeout: Duration,
    pub max_argument_bytes: usize,
    pub max_result_bytes: usize,
    pub hook_timeout: Duration,
    pub max_hook_calls: usize,
    pub hook_concurrency: usize,
}

impl Default for DispatchLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(600),
            max_argument_bytes: 16 * 1024 * 1024,
            max_result_bytes: 16 * 1024 * 1024,
            hook_timeout: Duration::from_secs(5),
            max_hook_calls: 16,
            hook_concurrency: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchErrorCode {
    InvalidArguments,
    ScopeDenied,
    Stopping,
    Timeout,
    Panicked,
    InvalidResult,
    ResultLimit,
    Unregistered,
    PlatformUnavailable,
    PermissionDenied,
    DuplicateCallId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DispatchFailure {
    pub code: DispatchErrorCode,
    pub message: String,
}

pub fn failure(call: &ToolCall, code: DispatchErrorCode, message: impl Into<String>) -> ToolResult {
    ToolResult::permanent_error(
        &call.id,
        serde_json::to_string(&DispatchFailure {
            code,
            message: message.into(),
        })
        .expect("serializable failure"),
    )
    .with_name(&call.name)
}

/// Execution policy and instance/session restrictions, never a tool registry.
pub struct ToolDispatcher {
    pub limits: DispatchLimits,
    scopes: RwLock<BTreeMap<SessionId, ToolScope>>,
    agents: RwLock<BTreeMap<SessionId, AgentId>>,
    concurrency: Mutex<BTreeMap<(CapabilityId, usize), Arc<tokio::sync::Semaphore>>>,
}

impl ToolDispatcher {
    pub fn new(limits: DispatchLimits) -> Self {
        Self {
            limits,
            scopes: RwLock::new(BTreeMap::new()),
            agents: RwLock::new(BTreeMap::new()),
            concurrency: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn scope(&self, session: SessionId) -> ToolScope {
        self.scopes
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .cloned()
            .unwrap_or(ToolScope::All)
    }
    pub(crate) fn scope_snapshot(&self, session: SessionId) -> Option<Vec<CapabilityId>> {
        match self.scope(session) {
            ToolScope::All => None,
            ToolScope::Only(ids) => Some(ids.into_iter().collect()),
        }
    }

    pub fn set_agent(&self, session: SessionId, agent: AgentId) {
        self.agents
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session, agent);
    }
    pub fn agent(&self, session: SessionId) -> AgentId {
        self.agents
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .copied()
            .unwrap_or_else(|| AgentId::from_uuid(session.as_uuid()))
    }
    fn permits(&self, tool: &dyn ToolCapability) -> Option<Arc<tokio::sync::Semaphore>> {
        let count = match tool.execution_policy().concurrency {
            ToolConcurrency::Parallel => return None,
            ToolConcurrency::Serial => 1,
            ToolConcurrency::Limited(count) => count,
        };
        Some(
            self.concurrency
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry((tool.id().clone(), count))
                .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(count)))
                .clone(),
        )
    }

    pub fn set_scope(&self, session: SessionId, scope: ToolScope) {
        self.scopes
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session, scope);
    }

    pub fn allows(&self, session: SessionId, id: &CapabilityId) -> bool {
        self.scopes
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session)
            .is_none_or(|scope| scope.allows(id))
    }

    pub fn validate(
        &self,
        session: Option<SessionId>,
        tool: &dyn ToolCapability,
        call: &ToolCall,
        legacy_open_properties: bool,
    ) -> Result<(), ToolResult> {
        if session.is_some_and(|session| !self.allows(session, tool.id())) {
            return Err(failure(
                call,
                DispatchErrorCode::ScopeDenied,
                "tool is outside this session's tool scope",
            ));
        }
        let policy = tool.execution_policy();
        if matches!(policy.concurrency, ToolConcurrency::Limited(count) if count == 0 || count > tokio::sync::Semaphore::MAX_PERMITS)
            || policy.timeout.is_some_and(|d| d.is_zero())
        {
            return Err(failure(
                call,
                DispatchErrorCode::InvalidArguments,
                "invalid registered execution limits",
            ));
        }
        if policy.platform == ToolPlatform::Desktop
            && !cfg!(any(windows, target_os = "linux", target_os = "macos"))
        {
            return Err(failure(
                call,
                DispatchErrorCode::PlatformUnavailable,
                "tool is unavailable on this platform",
            ));
        }
        if session.is_some_and(|session| {
            policy
                .required_capabilities
                .iter()
                .any(|id| !self.allows(session, id))
        }) {
            return Err(failure(
                call,
                DispatchErrorCode::ScopeDenied,
                "required capability is outside this session's scope",
            ));
        }
        if !call.arguments.is_object()
            || !fits_json(&call.arguments, self.limits.max_argument_bytes)
        {
            return Err(failure(
                call,
                DispatchErrorCode::InvalidArguments,
                "arguments must be an object within the input byte limit",
            ));
        }
        let declaration = tool.declaration();
        let mut parameters = declaration.parameters;
        // The legacy ABI ignored unknown root fields but bound them into frozen
        // approvals. Preserve that ABI explicitly; V1 and strict tools keep the
        // complete declared schema. Required fields and types are always checked.
        if legacy_open_properties
            && !declaration.strict
            && parameters.get("additionalProperties") == Some(&serde_json::Value::Bool(false))
        {
            parameters.insert("additionalProperties".into(), serde_json::Value::Bool(true));
        }
        let schema = serde_json::Value::Object(parameters);
        let validator = jsonschema::validator_for(&schema).map_err(|_| {
            failure(
                call,
                DispatchErrorCode::InvalidArguments,
                "tool parameter schema cannot be resolved locally",
            )
        })?;
        validator.validate(&call.arguments).map_err(|error| {
            // Do not echo argument values (which can contain secrets) in errors.
            failure(
                call,
                DispatchErrorCode::InvalidArguments,
                format!("arguments violate schema at {}", error.instance_path),
            )
        })
    }

    /// Preserve all three legacy invocation surfaces through this one adapter.
    pub fn adapt(
        self: &Arc<Self>,
        tool: Arc<dyn ToolCapability>,
        scope: Option<ModuleTaskScope>,
        legacy_open_properties: bool,
    ) -> Arc<dyn ToolCapability> {
        Arc::new(LegacyToolAdapter {
            tool,
            dispatcher: self.clone(),
            scope,
            legacy_open_properties,
            module: PluginId::new("host.legacy").expect("static module id"),
            context: None,
        })
    }
    pub(crate) fn adapt_context(
        self: &Arc<Self>,
        tool: Arc<dyn ToolCapability>,
        scope: ModuleTaskScope,
        legacy: bool,
        context: ToolExecutionContext,
    ) -> Arc<dyn ToolCapability> {
        Arc::new(LegacyToolAdapter {
            module: context.module.clone(),
            context: Some(context),
            tool,
            scope: Some(scope),
            legacy_open_properties: legacy,
            dispatcher: self.clone(),
        })
    }

    pub(crate) fn adapt_owned(
        self: &Arc<Self>,
        tool: Arc<dyn ToolCapability>,
        scope: ModuleTaskScope,
        legacy: bool,
        module: PluginId,
    ) -> Arc<dyn ToolCapability> {
        Arc::new(LegacyToolAdapter {
            module,
            context: None,
            tool,
            scope: Some(scope),
            legacy_open_properties: legacy,
            dispatcher: self.clone(),
        })
    }
}

/// Compatibility adapter: retains session and frozen payload without reinterpretation.
pub struct LegacyToolAdapter {
    tool: Arc<dyn ToolCapability>,
    dispatcher: Arc<ToolDispatcher>,
    scope: Option<ModuleTaskScope>,
    legacy_open_properties: bool,
    module: PluginId,
    context: Option<ToolExecutionContext>,
}

#[async_trait]
impl ToolCapability for LegacyToolAdapter {
    fn id(&self) -> &CapabilityId {
        self.tool.id()
    }
    fn declaration(&self) -> NormalizedTool {
        self.tool.declaration()
    }
    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        self.tool.freeze_invocation(call)
    }
    fn execution_policy(&self) -> ToolExecutionPolicy {
        let mut policy = self.tool.execution_policy();
        policy.legacy_arguments = self.legacy_open_properties;
        policy
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        let mut context = context.clone();
        context.module = self.module.clone();
        context.module_tasks = self
            .scope
            .clone()
            .unwrap_or_else(|| context.module_tasks.clone());
        self.execute_call(call, frozen, context.session, Some(context))
            .await
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invoke_in_session(call, None, None).await
    }
    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        self.invoke_in_session(call, frozen, None).await
    }
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.execute_call(call, frozen, session, self.context.clone())
            .await
    }
}

impl LegacyToolAdapter {
    async fn execute_call(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
        context: Option<ToolExecutionContext>,
    ) -> ToolResult {
        if let Err(error) = self.dispatcher.validate(
            session,
            self.tool.as_ref(),
            call,
            self.legacy_open_properties,
        ) {
            return error;
        }
        let policy = self.tool.execution_policy();
        let timeout = policy
            .timeout
            .unwrap_or(self.dispatcher.limits.timeout)
            .min(self.dispatcher.limits.timeout);
        let context = context.unwrap_or_else(|| ToolExecutionContext {
            agent: session
                .map(|session| self.dispatcher.agent(session))
                .unwrap_or_default(),
            session,
            turn: None,
            call_id: call.id.clone(),
            caller: ToolCaller::Host,
            module: self.module.clone(),
            tool_scope: session.and_then(|session| self.dispatcher.scope_snapshot(session)),
            authorization: ToolAuthorization::default(),
            cancellation: ToolCancellation::default(),
            deadline: Instant::now()
                .checked_add(timeout)
                .unwrap_or_else(Instant::now),
            trace: None,
            module_tasks: self.scope.clone().unwrap_or_default(),
            resource_lease: None,
        });
        if policy.permission == apeireth_plugin::ToolPermissionPolicy::RequireApproval
            && !context.authorization.approved
        {
            return failure(
                call,
                DispatchErrorCode::PermissionDenied,
                "registered tool requires an approved invocation",
            );
        }
        if context.cancellation.is_cancelled()
            || context.call_id != call.id
            || context.session != session
        {
            return failure(
                call,
                DispatchErrorCode::Stopping,
                "call cancelled or execution context mismatch",
            );
        }
        let timeout = timeout.min(context.deadline.saturating_duration_since(Instant::now()));
        let _cancel_on_drop = context.cancellation.cancel_on_drop();
        let permits = self.dispatcher.permits(self.tool.as_ref());
        let tool = self.tool.clone();
        let owned_call = call.clone();
        let frozen = frozen.cloned();
        let scope = self.scope.clone();
        // JoinSet aborts its owned task on drop, including caller cancellation.
        // No detached task may outlive a module's drain boundary.
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(async move {
            let mut context = context;
            let invoke = async {
                let _permit = match permits {
                    Some(permits) => Some(Arc::new(
                        permits
                            .acquire_owned()
                            .await
                            .expect("local semaphore stays open"),
                    )),
                    None => None,
                };
                context.resource_lease = _permit.as_ref().map(|permit| {
                    let lease: Arc<dyn Send + Sync> = permit.clone();
                    Arc::downgrade(&lease)
                });
                apeireth_plugin::tool_execution::scope_native_execution(
                    &context,
                    tool.invoke_with_context(&owned_call, frozen.as_ref(), &context),
                )
                .await
            };
            match scope {
                Some(scope) => scope.run(invoke).await,
                None => Ok(invoke.await),
            }
        });
        let result = match tokio::time::timeout(timeout, tasks.join_next()).await {
            Ok(Some(Ok(Ok(result)))) => result,
            Ok(Some(Ok(Err(_)))) => {
                return failure(
                    call,
                    DispatchErrorCode::Stopping,
                    "module stopping; execution may be incomplete",
                )
            }
            Ok(_) => {
                return failure(
                    call,
                    DispatchErrorCode::Panicked,
                    "tool task failed; execution may be incomplete",
                )
            }
            Err(_) => {
                return failure(
                    call,
                    DispatchErrorCode::Timeout,
                    "tool deadline exceeded; execution may be incomplete",
                )
            }
        };
        if result.tool_call_id != call.id
            || result.name.as_deref().is_some_and(|name| name != call.name)
        {
            return failure(
                call,
                DispatchErrorCode::InvalidResult,
                "tool returned an unrelated result; execution may have completed",
            );
        }
        if !fits_json(&result, self.dispatcher.limits.max_result_bytes) {
            return failure(
                call,
                DispatchErrorCode::ResultLimit,
                "tool output exceeds byte limit; execution may have completed",
            );
        }
        if let (Some(schema), apeireth_protocol::canonical::ToolOutcome::Ok { value }) =
            (&policy.output_schema, &result.outcome)
        {
            if jsonschema::validator_for(schema)
                .map_or(true, |validator| !validator.is_valid(value))
            {
                return failure(
                    call,
                    DispatchErrorCode::InvalidResult,
                    "tool output violates its registered schema; execution may have completed",
                );
            }
        }
        // The compatibility ABI returns ToolResult; correlation has already
        // crossed the same V1 fact boundary. Legacy durable logs stay stable.
        match ToolEventV1::new(
            TaskId::new(),
            ToolEventSourceV1::Model { module_id: None },
            call.clone(),
            result,
        ) {
            Ok(event) => event.result().clone(),
            Err(_) => failure(
                call,
                DispatchErrorCode::InvalidResult,
                "tool fact failed correlation validation",
            ),
        }
    }
}

/// Bounded byte counting without allocating another copy of a large result.
pub(crate) fn fits_json(value: &impl Serialize, limit: usize) -> bool {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("byte limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(limit), value).is_ok()
}

/// Reject malformed model batches before recording assistant calls or performing effects.
/// Wire ids are checked within a response; durable idempotency still uses ToolEventV1 identities.
pub(super) fn validate_call_batch(calls: &[ToolCall]) -> Result<(), DispatchFailure> {
    let mut ids = BTreeSet::new();
    for call in calls {
        if call.id.trim().is_empty() || !ids.insert(&call.id) {
            return Err(DispatchFailure {
                code: DispatchErrorCode::DuplicateCallId,
                message: "empty or duplicate tool call ID in one model response; no tools executed"
                    .into(),
            });
        }
    }
    Ok(())
}
