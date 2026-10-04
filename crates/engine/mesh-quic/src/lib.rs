//! Apeireth Mesh Phase 2A: authenticated QUIC transport.
//!
//! # What this crate is
//!
//! The "how bytes move" half of the mesh — and only that. Ownership,
//! authorization, routing, and the agent turn all live above the
//! [`MeshConnection`] boundary: a QUIC connection owns no module, no
//! capability implementation, and no authority of any kind.
//!
//! # Trust, stated once
//!
//! The TLS **certificate key is the mesh identity key** (see [`tls`]): a
//! completed handshake against the pinned verifier proves "I control the
//! private key pinned for this NodeId", mutually, replay-immune. There is
//! no TOFU — pins come from explicit known-peer configuration and every
//! unpinned, mismatched, or misattributed identity fails closed. A socket
//! address is not an identity.
//!
//! # Bounded by construction
//!
//! Length-prefixed frames with a checked maximum (allocation happens only
//! after the length check), bounded inbound and outbound queues with real
//! backpressure, and connection/handshake deadlines. All network input is
//! untrusted and no network-facing path panics on it.

#![deny(unsafe_code)]

pub mod config;
pub mod connection;
pub mod endpoint;
pub mod framing;
pub mod tls;
pub mod transport;

pub use config::QuicTransportConfig;
pub use connection::QuicMeshConnection;
pub use endpoint::{
    ObserveError, PreparedMeshEndpoint, QuicPeerEndpoint, DEFAULT_OBSERVE_TIMEOUT_MS,
};
pub use transport::{testing, QuicMeshListener, QuicMeshTransport};
