//! Per-call context and declarative limits. Neither grants permission by name.
use crate::task_scope::ModuleTaskScope;
use apeireth_core::kernel::{AgentId, CapabilityId, PluginId, RequestId, SessionId, TraceId};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[derive(Default)]
struct CancellationState {
    cancelled: AtomicBool,
    parents: Vec<ToolCancellation>,
}
#[derive(Clone, Default)]
pub struct ToolCancellation(Arc<CancellationState>);
impl std::fmt::Debug for ToolCancellation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCancellation")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}
impl ToolCancellation {
    pub fn joined(parents: Vec<Self>) -> Self {
        Self(Arc::new(CancellationState {
            cancelled: AtomicBool::new(false),
            parents,
        }))
    }
    pub fn same_call(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire) || self.0.parents.iter().any(Self::is_cancelled)
    }
    pub fn cancel_on_drop(&self) -> CancelOnDrop {
        CancelOnDrop(self.clone())
    }
}
pub struct CancelOnDrop(ToolCancellation);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCaller {
    Model,
    BehaviorModule(String),
    Module(PluginId),
    ModuleMessage {
        source: PluginId,
        instance: apeireth_core::kernel::ModuleInstanceId,
    },
    Host,
}
#[derive(Clone, Default)]
pub struct ToolAuthorization {
    /// Host-bound authorization generation; Dispatcher overwrites caller input.
    pub principal_epoch: Option<String>,
    pub intent_id: Option<String>,
    pub authorization_context: Option<String>,
    pub approved: bool,
    /// Immutable Host-only security data. Never decoded from model or message payloads.
    pub host_context: Option<Arc<dyn std::any::Any + Send + Sync>>,
}
impl std::fmt::Debug for ToolAuthorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolAuthorization")
            .field("approved", &self.approved)
            .field("has_host_context", &self.host_context.is_some())
            .finish_non_exhaustive()
    }
}
/// Runtime-created immutable call identity. Legacy direct-host calls may lack a turn.
#[derive(Clone)]
pub struct ToolExecutionContext {
    pub agent: AgentId,
    pub principal: Option<crate::agents::AgentPrincipal>,
    pub session: Option<SessionId>,
    pub turn: Option<RequestId>,
    pub call_id: String,
    pub caller: ToolCaller,
    pub module: PluginId,
    pub module_instance: Option<apeireth_core::kernel::ModuleInstanceId>,
    pub internal_review: Option<crate::system_one::InternalReviewReceipt>,
    pub message_support: bool,
    pub message_route: Option<crate::module_messages::ModuleMessageRoute>,
    pub delegated_tools: Option<Vec<CapabilityId>>,
    pub delegation: Option<Arc<ToolExecutionContext>>,
    /// Snapshot only; dispatcher checks current scope again before execution.
    pub tool_scope: Option<Vec<CapabilityId>>,
    pub authorization: ToolAuthorization,
    pub cancellation: ToolCancellation,
    pub deadline: Instant,
    pub trace: Option<TraceId>,
    pub module_tasks: ModuleTaskScope,
    /// Weak in the public context so retaining a context cannot pin a permit.
    pub resource_lease: Option<std::sync::Weak<dyn Send + Sync>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolConcurrency {
    #[default]
    Parallel,
    Serial,
    Limited(usize),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolPlatform {
    #[default]
    Any,
    Desktop,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolPermissionPolicy {
    #[default]
    Governed,
    RequireApproval,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolLifecycleRequirement {
    #[default]
    Running,
}

/// Enforced by the common adapter; individual tools can impose stricter limits.
#[derive(Debug, Clone, Default)]
pub struct ToolExecutionPolicy {
    pub output_schema: Option<serde_json::Value>,
    /// Explicit compatibility view; never an authorization grant.
    pub legacy_arguments: bool,
    pub lifecycle: ToolLifecycleRequirement,
    pub required_capabilities: Vec<CapabilityId>,
    pub permission: ToolPermissionPolicy,
    pub timeout: Option<Duration>,
    pub concurrency: ToolConcurrency,
    pub platform: ToolPlatform,
}

/// Compatibility bridge for native helpers. This is scoped to each future poll,
/// never a shared agent/session store, and never conveys permission.
#[derive(Clone)]
pub struct NativeExecutionContext {
    pub cancellation: ToolCancellation,
    pub deadline: Instant,
    pub tasks: ModuleTaskScope,
    resource_lease: Option<Arc<dyn Send + Sync>>,
    tool_context: Option<Arc<ToolExecutionContext>>,
}
std::thread_local! {
    static NATIVE_CONTEXT: std::cell::RefCell<Option<NativeExecutionContext>> = const { std::cell::RefCell::new(None) };
}
pub fn native_tool_context() -> Option<Arc<ToolExecutionContext>> {
    NATIVE_CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|context| context.tool_context.clone())
    })
}
pub fn native_execution_context() -> Option<NativeExecutionContext> {
    NATIVE_CONTEXT.with(|slot| slot.borrow().clone())
}
struct RestoreNativeContext(Option<NativeExecutionContext>);
impl Drop for RestoreNativeContext {
    fn drop(&mut self) {
        NATIVE_CONTEXT.with(|slot| {
            slot.replace(self.0.take());
        });
    }
}
pub fn with_native_execution<T>(
    context: Option<NativeExecutionContext>,
    work: impl FnOnce() -> T,
) -> T {
    let previous = NATIVE_CONTEXT.with(|slot| slot.replace(context));
    let _restore = RestoreNativeContext(previous);
    work()
}
pub async fn scope_native_execution<T>(
    context: &ToolExecutionContext,
    future: impl std::future::Future<Output = T>,
) -> T {
    let tool_context = context.message_support.then(|| Arc::new(context.clone()));
    futures::pin_mut!(future);
    futures::future::poll_fn(|cx| {
        with_native_execution(
            Some(NativeExecutionContext {
                tool_context: tool_context.clone(),
                cancellation: context.cancellation.clone(),
                deadline: context.deadline,
                tasks: context.module_tasks.clone(),
                resource_lease: context
                    .resource_lease
                    .as_ref()
                    .and_then(|lease| lease.upgrade()),
            }),
            || future.as_mut().poll(cx),
        )
    })
    .await
}

impl std::fmt::Debug for NativeExecutionContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeExecutionContext")
            .field("cancelled", &self.cancellation.is_cancelled())
            .field("holds_resource", &self.resource_lease.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_native_context_is_restored_even_after_panic() {
        let outer = NativeExecutionContext {
            cancellation: Default::default(),
            deadline: Instant::now(),
            tasks: ModuleTaskScope::new(),
            resource_lease: None,
            tool_context: None,
        };
        let inner = NativeExecutionContext {
            cancellation: Default::default(),
            deadline: Instant::now(),
            tasks: ModuleTaskScope::new(),
            resource_lease: None,
            tool_context: None,
        };
        inner.cancellation.cancel();
        assert!(native_execution_context().is_none());
        with_native_execution(Some(outer), || {
            assert!(!native_execution_context()
                .unwrap()
                .cancellation
                .is_cancelled());
            let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_native_execution(Some(inner), || {
                    assert!(native_execution_context()
                        .unwrap()
                        .cancellation
                        .is_cancelled());
                    panic!("fixture");
                })
            }));
            assert!(failure.is_err());
            assert!(!native_execution_context()
                .unwrap()
                .cancellation
                .is_cancelled());
        });
        assert!(native_execution_context().is_none());
    }
}

/// Module observations may be audited without adding empty provider messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleResultDelivery {
    Provider,
    AuditOnly,
}
