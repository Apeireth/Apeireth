//! The execution-side port: how a mesh request reaches the canonical runtime.
//!
//! # One dispatch world
//!
//! There is no mesh-private dispatch path. A remote invocation ends in the same
//! place a local one does — the origin node's `PluginManager`, behind the same
//! canonical [`GovernanceHook`] — and the only new machinery is *context*:
//! where the call came from ([`ActionSource`]), when it must finish, and
//! whether it has been cancelled.
//!
//! [`MeshCapabilityExecutor`] is that port. [`ManagerExecutor`] is its canonical
//! implementation over a [`ModuleManagerView`] (the runtime itself, or any
//! holder of a manager plus governance). Keeping the port trait-based is what
//! lets `apeireth-mesh` sit beside the runtime without depending on it, and
//! what keeps `PluginManager` ignorant of sockets.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock, PluginId, SessionId, TraceId};
use apeireth_governance::{Action, ActionSource, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::{PluginError, PluginManager};
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use async_trait::async_trait;

use crate::error::MeshExecError;
use crate::route::CapabilityCatalogEntry;

/// Cooperative cancellation for one capability invocation.
///
/// Shared between the calling side, the wire (`Cancel` messages), and the
/// executing capability. Cancellation is best-effort by contract: a capability
/// that never checks the flag runs to completion, exactly as a local cancelled
/// task can.
#[derive(Debug, Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    /// A fresh, uncancelled flag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Everything an executing capability needs to know about one invocation.
///
/// Trace and session cross the network unchanged, so a remote execution is
/// visible inside the same causal span as the local work that caused it —
/// mesh RPC must never become a telemetry black hole.
#[derive(Debug, Clone)]
pub struct MeshInvokeContext {
    /// The calling side's session.
    pub session: SessionId,
    /// The causal span, propagated from the caller without re-minting.
    pub trace: TraceId,
    /// The calling side's agent-loop round, for round-bounding policies.
    pub round: u32,
    /// Where the call originates: local work, or a named remote node.
    pub source: ActionSource,
    /// Caller deadline in epoch milliseconds, when one was set.
    pub deadline_epoch_ms: Option<u64>,
    /// Cooperative cancellation handle for this invocation.
    pub cancelled: CancelFlag,
}

impl MeshInvokeContext {
    /// A local invocation context with no deadline.
    pub fn local(session: SessionId, trace: TraceId, round: u32) -> Self {
        Self {
            session,
            trace,
            round,
            source: ActionSource::Local,
            deadline_epoch_ms: None,
            cancelled: CancelFlag::new(),
        }
    }

    /// Set the caller's deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline_epoch_ms: Option<u64>) -> Self {
        self.deadline_epoch_ms = deadline_epoch_ms;
        self
    }

    /// Set where the call originates.
    #[must_use]
    pub fn with_source(mut self, source: ActionSource) -> Self {
        self.source = source;
        self
    }
}

/// The canonical execution port behind every mesh invocation.
///
/// Implemented over `PluginManager` + canonical governance; a test double may
/// implement it too, but production dispatch must always land here rather than
/// in a mesh-private registry.
#[async_trait]
pub trait MeshCapabilityExecutor: Send + Sync {
    /// This runtime's canonical capability view (from the manager's derived
    /// views — never from a module's private bag).
    fn catalog(&self) -> Vec<CapabilityCatalogEntry>;

    /// Judge and execute one tool call against this runtime's modules.
    async fn execute_tool(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        call: &ToolCall,
    ) -> Result<ToolResult, MeshExecError>;

    /// Request-side authorization for an outbound call, through the same
    /// canonical governance. The mesh owns no policy engine of its own.
    async fn authorize(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        arguments: &serde_json::Value,
    ) -> Result<(), MeshExecError>;
}

/// Read access to the canonical runtime pieces mesh dispatch needs.
///
/// The port that lets `apeireth-mesh` execute against a real `PluginManager`
/// without depending on the runtime crate: the runtime (or any holder of a
/// manager plus governance) implements this one trait and [`ManagerExecutor`]
/// does the rest. Access to the manager is scoped through a closure so a
/// holder is free to guard it with a lock.
pub trait ModuleManagerView: Send + Sync {
    /// Run `f` against the one canonical module/capability world.
    fn with_manager<R>(&self, f: impl FnOnce(&PluginManager) -> R) -> R;

    /// The one canonical decision point.
    fn governance(&self) -> &Arc<dyn GovernanceHook>;

    /// The time source for deadline checks.
    fn clock(&self) -> &Arc<dyn Clock>;
}

/// The canonical [`MeshCapabilityExecutor`] over a [`ModuleManagerView`].
///
/// This is what makes "remote execution is an extension of the Plugin/Capability
/// system" true rather than aspirational: dispatch resolves through the
/// canonical capability index to a real `ToolCapability` owned by a real
/// module, under real governance.
pub struct ManagerExecutor<V> {
    view: Arc<V>,
}

impl<V: ModuleManagerView> ManagerExecutor<V> {
    /// An executor over the given runtime view.
    pub fn new(view: Arc<V>) -> Self {
        Self { view }
    }

    /// The underlying view, for callers that need the runtime surface.
    pub fn view(&self) -> &Arc<V> {
        &self.view
    }

    fn clock_now_ms(&self) -> i64 {
        apeireth_core::kernel::Timestamp::from_clock(self.view.clock().as_ref()).epoch_millis()
    }

    fn check_preconditions(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
    ) -> Result<(), MeshExecError> {
        if ctx.cancelled.is_cancelled() {
            return Err(MeshExecError::Cancelled {
                capability: capability.clone(),
            });
        }
        if let Some(deadline) = ctx.deadline_epoch_ms {
            if self.clock_now_ms().max(0) as u64 > deadline {
                return Err(MeshExecError::DeadlineExceeded {
                    capability: capability.clone(),
                });
            }
        }
        Ok(())
    }

    async fn authorize_capability(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        arguments: &serde_json::Value,
    ) -> Result<(), MeshExecError> {
        self.check_preconditions(ctx, capability)?;
        let action = Action::CapabilityDispatch {
            capability,
            arguments,
        };
        let verdict = self
            .view
            .governance()
            .evaluate_verbose(
                &GovernanceRequest::new(action, ctx.session, ctx.trace, ctx.round)
                    .with_source(ctx.source.clone()),
            )
            .await;
        match verdict.decision {
            Decision::Allow => Ok(()),
            Decision::Deny { reason } => Err(MeshExecError::Denied {
                capability: capability.clone(),
                reason,
            }),
            // A remote (or otherwise unattended) call must never mint a hidden
            // human approval: fail closed and say so.
            Decision::RequireApproval { reason } => Err(MeshExecError::ApprovalRequired {
                capability: capability.clone(),
                reason,
            }),
        }
    }
}

#[async_trait]
impl<V: ModuleManagerView + 'static> MeshCapabilityExecutor for ManagerExecutor<V> {
    fn catalog(&self) -> Vec<CapabilityCatalogEntry> {
        // The canonical derived view only: one entry per declared capability,
        // owned by a real PluginId, with availability from the owner's current
        // dispatchability. No mesh-private tool bag — the PluginManager's
        // derived index is the single source of who owns what locally.
        self.view.with_manager(|manager| {
            manager
                .records()
                .into_iter()
                .filter_map(|record| {
                    let id = record.descriptor.id.clone();
                    let kind = apeireth_mesh_protocol::MeshCapabilityKind::from_id(&id)
                        .unwrap_or(apeireth_mesh_protocol::MeshCapabilityKind::Extension);
                    // The model-facing declaration is projected from a real
                    // tool object — schema is implementation metadata and is
                    // never guessed. A tool whose owner cannot dispatch right
                    // now still projects the schema its owner exports, as
                    // unavailable; a tool-kind capability with no object left to
                    // speak for it cannot form a well-formed advert entry and is
                    // left out entirely.
                    let declaration = manager.plugins().get(record.owner).ok().and_then(|plugin| {
                        plugin
                            .tools()
                            .into_iter()
                            .find(|tool| tool.id() == &id)
                            .map(|tool| tool.declaration())
                    });
                    if declaration.is_none()
                        && kind == apeireth_mesh_protocol::MeshCapabilityKind::Tool
                    {
                        return None;
                    }
                    let available = manager.tool(&id).is_ok() || manager.provider(&id).is_ok();
                    let description = declaration
                        .as_ref()
                        .and_then(|declaration| declaration.description.clone())
                        .unwrap_or_else(|| record.descriptor.description.clone());
                    Some(CapabilityCatalogEntry {
                        id,
                        kind,
                        owner: record.owner.clone(),
                        description,
                        declaration,
                        available,
                    })
                })
                .collect()
        })
    }

    async fn execute_tool(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        call: &ToolCall,
    ) -> Result<ToolResult, MeshExecError> {
        // Canonical governance first, with full source context — a refused
        // invocation must never reach the capability implementation.
        self.authorize_capability(ctx, capability, &call.arguments)
            .await?;

        let tool = self
            .view
            .with_manager(|manager| manager.tool(capability))
            .map_err(|error| match error {
                PluginError::NotActive {
                    capability,
                    plugin,
                    state,
                } => MeshExecError::NotAvailable {
                    capability,
                    reason: format!("owning plugin {plugin} is {state}"),
                },
                PluginError::UnknownCapability(capability) => {
                    MeshExecError::UnknownCapability { capability }
                }
                other => MeshExecError::Internal {
                    capability: capability.clone(),
                    message: other.to_string(),
                },
            })?;

        if ctx.cancelled.is_cancelled() {
            return Err(MeshExecError::Cancelled {
                capability: capability.clone(),
            });
        }
        Ok(tool.invoke(call).await)
    }

    async fn authorize(
        &self,
        ctx: &MeshInvokeContext,
        capability: &CapabilityId,
        arguments: &serde_json::Value,
    ) -> Result<(), MeshExecError> {
        self.authorize_capability(ctx, capability, arguments).await
    }
}

/// Build an advert-ready routing snapshot from an executor's canonical view.
pub fn catalog_to_modules(
    catalog: Vec<CapabilityCatalogEntry>,
) -> Vec<apeireth_mesh_protocol::ModuleAdvert> {
    use std::collections::BTreeMap;

    use apeireth_mesh_protocol::{CapabilityAdvertEntry, ModuleAdvert};

    let mut by_module: BTreeMap<PluginId, ModuleAdvert> = BTreeMap::new();
    for entry in catalog {
        let advert = by_module
            .entry(entry.owner.clone())
            .or_insert_with(|| ModuleAdvert {
                module: entry.owner.clone(),
                capabilities: Vec::new(),
            });
        advert.capabilities.push(CapabilityAdvertEntry {
            id: entry.id,
            kind: entry.kind,
            description: entry.description,
            available: entry.available,
            declaration: entry.declaration,
        });
    }
    by_module.into_values().collect()
}
