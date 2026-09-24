//! Apeireth Mesh Phase 0: connect runtimes, don't build a second one.
//!
//! # The one idea
//!
//! **Remote execution is an extension of the Module/Capability system.** A
//! module still owns its implementation at its origin node; a capability still
//! describes what can be done; the mesh only changes *where execution happens*.
//! `tool.mesh.echo` means the same thing whether it resolves to
//! [`route::CapabilityRoute::local`] or to a remote node — and `origin`
//! (system / installed / development / MCP) stays a different axis from
//! `location` ([`ModuleLocation::Local`] / [`ModuleLocation::Remote`]).
//!
//! # What Phase 0 contains
//!
//! A complete semantic slice over an in-memory transport:
//!
//! - stable [`NodeId`] identity, separate from display names;
//! - persistent peer connections with a fail-closed handshake
//!   (exact protocol-version match, no silent downgrade; identity binding);
//! - explicit known-peer trust ([`trust::TrustPolicy`]): discovered ≠ trusted ≠
//!   authorized-for-everything, and every execution is still judged by the
//!   canonical governance hook on the executing node;
//! - read-only capability projections from peer adverts — metadata only, never
//!   a second implementation ownership, never able to shadow a local
//!   capability ([`route::RouteTable`] is strictly namespaced by route scope);
//! - generic capability invocation ending in the origin node's
//!   `ModuleManager`, under governance, with trace propagated,
//!   best-effort cancellation, deadlines, and heartbeats.
//!
//! # What Phase 0 deliberately does not contain
//!
//! NAT traversal, relays, STUN/TURN, DHT, gossip discovery, distributed memory,
//! session handoff, global scheduling, agent migration, QUIC, or a PKI. The
//! transport boundary ([`transport::MeshTransport`]) is where a real TCP or
//! QUIC link plugs in later without touching any of the semantics above.

#![deny(unsafe_code)]

pub mod error;
pub mod executor;
pub mod node;
pub mod peer;
pub mod route;
pub mod transport;
pub mod trust;

pub use error::{MeshError, MeshExecError};
pub use executor::{
    catalog_to_modules, CancelFlag, ManagerExecutor, MeshCapabilityExecutor, MeshInvokeContext,
    ModuleManagerView,
};
pub use node::{
    CallHandle, CallTarget, InvokeMeta, MeshNode, MeshNodeConfig, DEFAULT_HEARTBEAT_TIMEOUT_MS,
};
pub use peer::{Peer, PeerState};
pub use route::{
    CapabilityCatalogEntry, CapabilityRoute, RemoteCapability, RouteSelection, RouteTable,
};
pub use transport::{
    linked_pair, InMemoryLink, InMemoryListener, InMemoryMeshNetwork, InMemoryMeshTransport,
    MeshConnection, MeshTransport, MeshTransportError,
};
pub use trust::{KnownPeer, PeerTrust, TrustPolicy};

pub use apeireth_mesh_protocol::NodeId;
pub use apeireth_mesh_protocol::{
    Cancel, CapabilityAdvert, CapabilityCall, CapabilityError, CapabilityOutcome, CapabilityResult,
    Heartbeat, Hello, HelloAck, InvokeCapability, LocatedModuleId, MeshCapabilityKind,
    MeshEnvelope, MeshErrorCode, MeshPayload, MeshProtocolError, ModuleAdvert, ModuleLocation,
    NodeDescriptor, MESH_PROTOCOL_VERSION,
};
