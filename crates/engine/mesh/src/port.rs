//! The remote capability port: the entire surface a canonical runtime needs to
//! route tool calls across a mesh — and deliberately nothing else.
//!
//! Implementations sit on peers, handshakes, connections and transports;
//! callers see none of those types. Every descriptor is metadata — the
//! implementation stays with the remote node's module. A runtime that only
//! knows this port cannot tell a remote tool from a local one at the
//! declaration level, which is exactly the point: the model does not know
//! where a tool runs.

use apeireth_core::kernel::{CapabilityId, ModuleId, NodeId, SessionId, TraceId};
use apeireth_protocol::canonical::{
    NormalizedRequest, NormalizedResponse, NormalizedTool, ToolCall, ToolResult,
};
use async_trait::async_trait;

/// One remote tool as a peer projects it: identity and declaration, never an
/// implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteToolDescriptor {
    /// The node whose module owns the implementation.
    pub node: NodeId,
    /// The owning module id on that node.
    pub module: ModuleId,
    /// Capability identity, preserved end-to-end.
    pub capability: CapabilityId,
    /// The model-facing declaration, projected from the advert.
    pub declaration: NormalizedTool,
    /// Whether the owning module reported it dispatchable.
    pub available: bool,
}

/// Model routing metadata projected from a remote provider advert: model
/// identity, features, availability — and no provider config of any kind.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteProviderDescriptor {
    /// The node whose module owns the provider implementation.
    pub node: NodeId,
    /// The owning module id on that node.
    pub module: ModuleId,
    /// Provider capability identity, preserved end-to-end.
    pub capability: CapabilityId,
    /// The provider's model routing projection (secrets stripped upstream).
    pub models: Vec<apeireth_mesh_protocol::WireModelDescriptor>,
    /// Whether the owning module reported it dispatchable.
    pub available: bool,
}

/// Explicit route to one remote capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RemoteToolRoute {
    /// The node whose runtime will execute the call.
    pub node: NodeId,
    /// The owning module id on that node.
    pub module: ModuleId,
    /// Capability identity to execute.
    pub capability: CapabilityId,
}

/// Caller context propagated to the executing node unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInvokeContext {
    /// The causal span; never re-minted across the hop.
    pub trace: TraceId,
    /// The calling session.
    pub session: SessionId,
    /// The calling agent-loop round.
    pub round: u32,
    /// Caller deadline in epoch milliseconds, when set.
    pub deadline_epoch_ms: Option<u64>,
}

/// Why a remote invocation produced no tool result.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RemoteCapabilityError {
    /// The peer or its capability cannot serve the call right now.
    #[error("remote capability is unavailable: {0}")]
    Unavailable(String),
    /// The executing node's governance refused the invocation.
    #[error("remote capability refused by governance: {0}")]
    Denied(String),
    /// Governance wants a human; remote calls fail closed instead.
    #[error("remote capability requires approval which cannot be granted remotely: {0}")]
    ApprovalRequired(String),
    /// The invocation was cancelled before it produced a result.
    #[error("remote invocation was cancelled")]
    Cancelled,
    /// The caller's deadline elapsed first.
    #[error("remote invocation exceeded its deadline")]
    DeadlineExceeded,
    /// Anything else that kept the invocation from completing.
    #[error("remote invocation failed: {0}")]
    Failed(String),
}

/// The remote capability execution/query port.
///
/// Two methods, one rule each:
///
/// - [`remote_tools`](Self::remote_tools) returns a **fresh derived snapshot**
///   every call (no caching layer to invalidate);
/// - [`invoke_tool`](Self::invoke_tool) is the **already-authorized** remote
///   execution primitive. Callers perform their own requester-side governance
///   first; this path performs none on the calling side, so requester
///   governance happens exactly once per node per invocation.
#[async_trait]
pub trait RemoteCapabilityPort: Send + Sync {
    /// Remote tool projections currently routable (peer active, advertised
    /// available, declaration present).
    fn remote_tools(&self) -> Vec<RemoteToolDescriptor>;

    /// Remote provider projections currently routable (peer active, advertised
    /// available). Fresh derived snapshot every call — no cache.
    fn remote_providers(&self) -> Vec<RemoteProviderDescriptor>;

    /// Execute one remote tool call.
    async fn invoke_tool(
        &self,
        route: RemoteToolRoute,
        call: ToolCall,
        ctx: RemoteInvokeContext,
    ) -> Result<ToolResult, RemoteCapabilityError>;

    /// Execute one remote provider request (the already-authorized raw
    /// primitive, same rule as `invoke_tool`: callers own requester
    /// governance). The request is the canonical `NormalizedRequest` — the
    /// mesh never redefines the provider protocol.
    async fn invoke_provider(
        &self,
        route: RemoteToolRoute,
        request: NormalizedRequest,
        ctx: RemoteInvokeContext,
    ) -> Result<NormalizedResponse, RemoteCapabilityError>;
}

use crate::error::MeshError;
use crate::node::{InvokeMeta, MeshNode};
use crate::peer::PeerState;
use crate::route::RouteSelection;
use apeireth_mesh_protocol::MeshCapabilityKind;

#[async_trait]
impl RemoteCapabilityPort for MeshNode {
    fn remote_tools(&self) -> Vec<RemoteToolDescriptor> {
        // Fresh derived snapshot, availability-filtered per the routable rule:
        // peer Active, advert available, declaration present. No cache.
        self.remote_capabilities()
            .into_iter()
            .filter(|cap| {
                cap.available
                    && cap.kind == MeshCapabilityKind::Tool
                    && cap.declaration.is_some()
                    && self.peer_state(&cap.owner.node) == Some(PeerState::Active)
            })
            .filter_map(|cap| {
                Some(RemoteToolDescriptor {
                    node: cap.owner.node.clone(),
                    module: cap.owner.module.clone(),
                    capability: cap.id,
                    declaration: cap.declaration?,
                    available: cap.available,
                })
            })
            .collect()
    }

    fn remote_providers(&self) -> Vec<RemoteProviderDescriptor> {
        // Same routable rule as tools: peer Active, advertised available.
        self.remote_capabilities()
            .into_iter()
            .filter(|cap| {
                cap.available
                    && cap.kind == MeshCapabilityKind::Provider
                    && self.peer_state(&cap.owner.node) == Some(PeerState::Active)
            })
            .map(|cap| RemoteProviderDescriptor {
                node: cap.owner.node,
                module: cap.owner.module,
                capability: cap.id,
                models: cap.provider_models.unwrap_or_default(),
                available: cap.available,
            })
            .collect()
    }

    async fn invoke_tool(
        &self,
        route: RemoteToolRoute,
        call: ToolCall,
        ctx: RemoteInvokeContext,
    ) -> Result<ToolResult, RemoteCapabilityError> {
        // The already-authorized primitive: requester-side governance belongs
        // to the caller (canonical dispatch or the high-level explicit API),
        // the executing node governs once on its side. Nothing here adds a
        // second requester evaluation.
        match self
            .start_capability_call(
                &RouteSelection::Remote(route.node),
                &route.capability,
                apeireth_mesh_protocol::CapabilityCall::Tool(call),
                meta_from(ctx),
            )
            .await
            .map_err(map_mesh_error)?
            .outcome()
            .await
            .map_err(map_mesh_error)?
        {
            apeireth_mesh_protocol::CapabilityOutcome::Tool(result) => Ok(result),
            other => Err(RemoteCapabilityError::Failed(format!(
                "unexpected tool outcome shape: {other:?}"
            ))),
        }
    }

    async fn invoke_provider(
        &self,
        route: RemoteToolRoute,
        request: NormalizedRequest,
        ctx: RemoteInvokeContext,
    ) -> Result<NormalizedResponse, RemoteCapabilityError> {
        match self
            .start_capability_call(
                &RouteSelection::Remote(route.node),
                &route.capability,
                apeireth_mesh_protocol::CapabilityCall::Provider(request),
                meta_from(ctx),
            )
            .await
            .map_err(map_mesh_error)?
            .outcome()
            .await
            .map_err(map_mesh_error)?
        {
            apeireth_mesh_protocol::CapabilityOutcome::Provider(response) => Ok(response),
            other => Err(RemoteCapabilityError::Failed(format!(
                "unexpected provider outcome shape: {other:?}"
            ))),
        }
    }
}

fn meta_from(ctx: RemoteInvokeContext) -> InvokeMeta {
    InvokeMeta {
        session: ctx.session,
        trace: ctx.trace,
        round: ctx.round,
        deadline_epoch_ms: ctx.deadline_epoch_ms,
    }
}

fn map_mesh_error(error: MeshError) -> RemoteCapabilityError {
    match &error {
        MeshError::Cancelled => RemoteCapabilityError::Cancelled,
        MeshError::DeadlineExceeded => RemoteCapabilityError::DeadlineExceeded,
        MeshError::PeerUnavailable { .. }
        | MeshError::RouteUnavailable { .. }
        | MeshError::Transport(_) => RemoteCapabilityError::Unavailable(error.to_string()),
        MeshError::Remote {
            code: apeireth_mesh_protocol::MeshErrorCode::Denied,
            message,
            ..
        } => RemoteCapabilityError::Denied(message.clone()),
        MeshError::Remote {
            code: apeireth_mesh_protocol::MeshErrorCode::ApprovalRequired,
            message,
            ..
        } => RemoteCapabilityError::ApprovalRequired(message.clone()),
        other => RemoteCapabilityError::Failed(other.to_string()),
    }
}
