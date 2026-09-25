//! Mesh Phase 2C1 relay: reach a trusted peer over a WAN rendezvous + encrypted
//! relay when a direct connection is impossible.
//!
//! # The boundary
//!
//! A relay is **connectivity infrastructure, not a peer**. It never holds a
//! [`ModuleId`](apeireth_core::kernel::ModuleId), a
//! [`CapabilityId`](apeireth_core::kernel::CapabilityId), a `ModuleRegistry`, a
//! `CapabilityRegistry`, or any `KnownPeer` trust. Nodes A and B trust *each
//! other*; neither trusts the relay. The relay answers "how to reach", never
//! "who to trust" — that stays with the local `TrustStore`.
//!
//! # Rendezvous vs Relay
//!
//! Two strictly separate concepts that a single server process may host:
//!
//! - **Rendezvous** — "where is Node B currently reachable?" A signed, expiring
//!   [`PresenceRecord`](apeireth_mesh_protocol::rendezvous::PresenceRecord)
//!   that proves key possession and is *not* a trust grant.
//! - **Relay** — "forward opaque bytes between A and B." The relay only ever
//!   sees end-to-end ciphertext ([`e2e::SecureChannel`]).
//!
//! # Correctness path, not optimization
//!
//! Relay is the correctness path for WAN Mesh. Hole punching / NAT traversal
//! (STUN, TURN, ICE, UPnP — deliberately absent here) may one day become an
//! *optimization* on top, but a working Mesh must never depend on traversal
//! succeeding. Direct-then-relay is the connection policy (see
//! [`crate::connector::PeerConnector`]); this module provides the relay half.

pub mod connection;
pub mod e2e;
pub mod presence;
pub mod transport;

pub use connection::RelayMeshConnection;
pub use e2e::{handshake, HandshakeRole, OpaqueTunnel, SecureChannel, E2E_HANDSHAKE_DOMAIN};
pub use presence::{now_ms, signed_presence};
pub use transport::{
    RelayCircuit, RelayEndpoint, RelayError, RelayMeshListener, RelayMeshTransport,
};
