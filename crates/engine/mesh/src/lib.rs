//! Apeireth Mesh Phase 0: connect runtimes, don't build a second one.
//!
//! # The one idea
//!
//! **Remote execution is an extension of the Plugin/Capability system.** A
//! module still owns its implementation at its origin node; a capability still
//! describes what can be done; the mesh only changes *where execution happens*.
//! `tool.mesh.echo` means the same thing whether it resolves to
//! [`route::CapabilityRoute::local`] or to a remote node — and `origin`
//! (system / installed / development / MCP) stays a different axis from
//! `location` ([`PluginLocation::Local`] / [`PluginLocation::Remote`]).
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
//!   `PluginManager`, under governance, with trace propagated,
//!   best-effort cancellation, deadlines, and heartbeats.
//!
//! # What Phase 0 deliberately does not contain
//!
//! NAT traversal, relays, STUN/TURN, DHT, gossip discovery, distributed memory,
//! session handoff, global scheduling, agent migration, QUIC, or a PKI. The
//! transport boundary ([`transport::MeshTransport`]) is where a real TCP or
//! QUIC link plugs in later without touching any of the semantics above.

#![deny(unsafe_code)]

pub mod candidates;
pub mod conformance;
pub mod connector;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod identity;
mod kinds;
pub mod mdns;
pub mod node;
pub mod pairing;
pub mod peer;
pub mod port;
pub mod reconnect;
pub mod relay;
pub mod route;
pub mod transport;
pub mod trust;
pub mod trust_store;

pub use candidates::dial_plan;
pub use conformance::assert_connection_conformance;
pub use connector::{PeerConnector, Strategy};
pub use discovery::{
    DiscoveredPeer, DiscoveryBackend, DiscoveryMethod, DiscoverySink, FakeDiscoveryBackend,
    MeshDiscoveryService,
};
pub use error::{MeshError, MeshExecError};
pub use executor::{
    catalog_to_modules, CancelFlag, ManagerExecutor, MeshCapabilityExecutor, MeshInvokeContext,
    PluginManagerView,
};
pub use identity::{IdentityError, KeyFingerprint, NodeIdentity};
pub use mdns::{AdvertisedPresence, MdnsDiscovery, MESH_SERVICE_TYPE};
pub use node::{
    CallHandle, CallTarget, InvokeMeta, MeshNode, MeshNodeConfig, DEFAULT_HEARTBEAT_TIMEOUT_MS,
};
pub use pairing::{
    pair_inbound, pair_outbound, ConfirmDecision, PairedPeer, PairingError, PairingLimiter,
    PairingOptions, PairingState, PairingTranscript,
};
pub use peer::{Peer, PeerState};
pub use port::{
    RemoteCapabilityError, RemoteCapabilityPort, RemoteInvokeContext, RemoteProviderDescriptor,
    RemoteToolDescriptor, RemoteToolRoute,
};
pub use reconnect::ReconnectPolicy;
pub use route::{
    CapabilityCatalogEntry, CapabilityRoute, RemoteCapability, RouteSelection, RouteTable,
};
pub use transport::{
    linked_pair, InMemoryLink, InMemoryListener, InMemoryMeshNetwork, InMemoryMeshTransport,
    MeshConnection, MeshListener, MeshTransport, MeshTransportError,
};
pub use trust::{KnownPeer, PeerTrust, TrustPolicy};
pub use trust_store::{EndpointUpdate, TrustStore, TrustStoreError};

pub use apeireth_mesh_protocol::NodeId;
pub use apeireth_mesh_protocol::{
    decode_envelope, encode_envelope, Cancel, CapabilityAdvert, CapabilityAdvertEntry,
    CapabilityCall, CapabilityError, CapabilityOutcome, CapabilityResult, Heartbeat, Hello,
    HelloAck, InvokeCapability, LocatedPluginId, MeshCapabilityKind, MeshEnvelope, MeshErrorCode,
    MeshPayload, MeshProtocolError, NodeDescriptor, PluginAdvert, PluginLocation,
    WireModelDescriptor, MESH_PROTOCOL_VERSION,
};
