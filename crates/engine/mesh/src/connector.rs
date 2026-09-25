//! Connection strategy: how a node chooses *how* to reach a peer.
//!
//! Runtime semantics never change based on reachability. A peer is a peer
//! whether it is one LAN hop away or reached through a WAN relay. This module
//! supplies only the **transport** decision, above which
//! [`crate::node::MeshNode`] runs the exact same handshake, trust-pin check,
//! adverts, and invocations.
//!
//! # Policy (Phase 2C1)
//!
//! Deliberately simple and deterministic — no opaque scoring:
//!
//! ```text
//! try each strategy in order → first success wins
//! ```
//!
//! The default WAN order is **Direct, then Relay**: a node first attempts the
//! known direct endpoint (fast, no server dependency), and only falls back to
//! the rendezvous/relay path when direct dialing fails. A LAN/direct connection
//! therefore never gains an unnecessary server dependency.
//!
//! This is the seam where hole punching / NAT traversal may later be inserted
//! as an *optimization* between direct and relay, without touching any caller:
//! a traversal strategy is just another [`MeshTransport`] in the list.

use std::sync::Arc;

use async_trait::async_trait;

use crate::transport::{MeshConnection, MeshTransport, MeshTransportError};

/// One named reachability strategy in a [`PeerConnector`] chain.
pub struct Strategy {
    /// Human-readable label for diagnostics (`"direct"`, `"relay"`, …).
    pub name: &'static str,
    /// The transport that dials it.
    pub transport: Arc<dyn MeshTransport>,
}

/// A composite [`MeshTransport`] that tries an ordered list of strategies.
///
/// Holds no mesh semantics — it only decides which link to hand back. The
/// returned [`MeshConnection`] is whatever the winning strategy produced; the
/// mesh cannot (and must not) tell the difference.
pub struct PeerConnector {
    strategies: Vec<Strategy>,
}

impl PeerConnector {
    /// A connector over the given ordered strategies (first = preferred).
    pub fn new(strategies: Vec<Strategy>) -> Self {
        Self { strategies }
    }

    /// The WAN policy: try the direct transport first, fall back to relay.
    ///
    /// `direct` is expected to be the direct QUIC transport; `relay` the relay
    /// transport. Either may be absent (`None`) to build a relay-only or
    /// direct-only connector (a relay-only connector is how a WAN test forces
    /// the fallback path).
    pub fn direct_then_relay(
        direct: Option<Arc<dyn MeshTransport>>,
        relay: Option<Arc<dyn MeshTransport>>,
    ) -> Self {
        let mut strategies = Vec::new();
        if let Some(direct) = direct {
            strategies.push(Strategy {
                name: "direct",
                transport: direct,
            });
        }
        if let Some(relay) = relay {
            strategies.push(Strategy {
                name: "relay",
                transport: relay,
            });
        }
        Self { strategies }
    }
}

#[async_trait]
impl MeshTransport for PeerConnector {
    async fn dial(&self, endpoint: &str) -> Result<Arc<dyn MeshConnection>, MeshTransportError> {
        let mut last: Option<MeshTransportError> = None;
        for strategy in &self.strategies {
            match strategy.transport.dial(endpoint).await {
                Ok(link) => return Ok(link),
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| {
            MeshTransportError::Io("no connection strategy available".to_string())
        }))
    }
}
