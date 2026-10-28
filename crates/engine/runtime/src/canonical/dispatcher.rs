//! Common tool invocation boundary. Catalogs remain owned by the registries.
//! Governance admission stays in the canonical runtime, including frozen approval.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};
use std::time::Duration;

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
}

impl ToolDispatcher {
    pub fn new(limits: DispatchLimits) -> Self {
        Self {
            limits,
            scopes: RwLock::new(BTreeMap::new()),
        }
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
        })
    }
}

/// Compatibility adapter: retains session and frozen payload without reinterpretation.
pub struct LegacyToolAdapter {
    tool: Arc<dyn ToolCapability>,
    dispatcher: Arc<ToolDispatcher>,
    scope: Option<ModuleTaskScope>,
    legacy_open_properties: bool,
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
        if let Err(error) = self.dispatcher.validate(
            session,
            self.tool.as_ref(),
            call,
            self.legacy_open_properties,
        ) {
            return error;
        }
        let tool = self.tool.clone();
        let owned_call = call.clone();
        let frozen = frozen.cloned();
        let scope = self.scope.clone();
        // JoinSet aborts its owned task on drop, including caller cancellation.
        // No detached task may outlive a module's drain boundary.
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(async move {
            let invoke = tool.invoke_in_session(&owned_call, frozen.as_ref(), session);
            match scope {
                Some(scope) => scope.run(invoke).await,
                None => Ok(invoke.await),
            }
        });
        let result =
            match tokio::time::timeout(self.dispatcher.limits.timeout, tasks.join_next()).await {
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
