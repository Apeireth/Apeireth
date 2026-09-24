//! Runtime ↔ mesh wiring: the port that lets a [`MeshNode`] execute against
//! this runtime's canonical `ModuleManager` and governance.
//!
//! This is deliberately the entire integration surface. The mesh crate owns
//! every semantic (handshake, trust, routing, projection, cancellation); the
//! runtime owns every fact (modules, capabilities, lifecycle, policy). The two
//! meet only through [`ModuleManagerView`], so neither can grow a second view
//! of the other's world — a remote invocation lands in the same
//! governance-then-`ModuleManager` path a local one takes, and the network
//! still only changes *where execution happens*.

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock};
use apeireth_governance::GovernanceHook;
use apeireth_plugin::ModuleManager;

pub use apeireth_mesh::{
    CallHandle, CallTarget, CapabilityCatalogEntry, CapabilityRoute, InvokeMeta, ManagerExecutor,
    MeshCapabilityExecutor, MeshError, MeshExecError, MeshInvokeContext, MeshNode, MeshNodeConfig,
    ModuleLocation, ModuleManagerView, RemoteCapability, RouteSelection, RouteTable,
};

use super::runtime::Runtime;

impl ModuleManagerView for Runtime {
    fn with_manager<R>(&self, f: impl FnOnce(&ModuleManager) -> R) -> R {
        f(Runtime::module_manager(self))
    }

    fn governance(&self) -> &Arc<dyn GovernanceHook> {
        Runtime::governance(self)
    }

    fn clock(&self) -> &Arc<dyn Clock> {
        Runtime::clock(self)
    }
}

/// Assemble the canonical mesh executor over an assembled runtime.
///
/// `Arc<Runtime>` is the handle the gateway already passes around, so a host
/// that owns a runtime can expose it to a mesh node with one call:
///
/// ```text
/// let executor = runtime_mesh_executor(Arc::clone(&runtime));
/// let node = MeshNode::new(config, Arc::new(executor), runtime.clock().clone());
/// ```
pub fn runtime_mesh_executor(runtime: Arc<Runtime>) -> ManagerExecutor<Runtime> {
    ManagerExecutor::new(runtime)
}

/// Capability routes known to a mesh node for one capability, re-exported for
/// hosts that introspect routing without importing the mesh crate directly.
pub fn mesh_routes_for(node: &MeshNode, capability: &CapabilityId) -> Vec<CapabilityRoute> {
    node.routes_for(capability)
}
