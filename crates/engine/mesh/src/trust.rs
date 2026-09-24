//! Explicit known-peer trust for Phase 0.
//!
//! Three rules, held apart on purpose:
//!
//! - *Discovered ≠ Trusted*: nothing is discovered in Phase 0 at all — a peer
//!   exists here only because an operator listed it — and even a future
//!   discovery mechanism may only ever report what it found. Discovery never
//!   grants anything.
//! - *Trusted ≠ AuthorizedForEverything*: a trusted peer may connect and may
//!   *request* capability execution. Every request is still judged by the
//!   canonical [`apeireth_governance::GovernanceHook`] on the executing node.
//! - Unknown or untrusted peers fail closed at the handshake, before any
//!   capability metadata or invocation traffic can flow.

use apeireth_core::kernel::NodeId;

use crate::error::MeshError;
use crate::identity::KeyFingerprint;

/// How far a known peer is trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PeerTrust {
    /// May connect and request capability execution, subject to governance.
    Trusted,
    /// Known but not trusted: refused at the handshake.
    #[default]
    Untrusted,
}

/// One operator-configured peer: identity, endpoint, trust, and — for real
/// transports — the pinned identity key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPeer {
    /// The peer's canonical node identity.
    pub node: NodeId,
    /// Where to reach it. Transport-specific; never an identity.
    pub endpoint: String,
    /// How far it is trusted.
    pub trust: PeerTrust,
    /// Pinned public-key fingerprint for this NodeId.
    ///
    /// The no-TOFU rule lives here: an unpinned entry is only usable on
    /// transports with no cryptographic identity claim (the in-memory test
    /// fabric). Real transports **require** a pin — unknown key, mismatched
    /// key, and right-key-wrong-node all fail closed. `None` never means
    /// "accept whatever arrives"; it means "this entry cannot authenticate".
    pub key_fingerprint: Option<KeyFingerprint>,
}

/// The set of peers this runtime admits.
///
/// Empty by default, which denies everyone: a runtime that has not been told
/// who its peers are must not talk to anybody.
#[derive(Debug, Clone, Default)]
pub struct TrustPolicy {
    peers: Vec<KnownPeer>,
}

impl TrustPolicy {
    /// A policy that admits nobody.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace a trusted peer with a pinned identity key. The form real
    /// transports require.
    #[must_use]
    pub fn with_trusted_pinned(
        mut self,
        node: NodeId,
        endpoint: impl Into<String>,
        key_fingerprint: KeyFingerprint,
    ) -> Self {
        self.upsert(KnownPeer {
            node,
            endpoint: endpoint.into(),
            trust: PeerTrust::Trusted,
            key_fingerprint: Some(key_fingerprint),
        });
        self
    }

    /// Add or replace a trusted peer without a key pin (legacy in-memory use).
    #[must_use]
    pub fn with_trusted(mut self, node: NodeId, endpoint: impl Into<String>) -> Self {
        self.upsert(KnownPeer {
            node,
            endpoint: endpoint.into(),
            trust: PeerTrust::Trusted,
            key_fingerprint: None,
        });
        self
    }

    /// Add or replace an explicitly untrusted (known-but-refused) peer.
    #[must_use]
    pub fn with_untrusted(mut self, node: NodeId, endpoint: impl Into<String>) -> Self {
        self.upsert(KnownPeer {
            node,
            endpoint: endpoint.into(),
            trust: PeerTrust::Untrusted,
            key_fingerprint: None,
        });
        self
    }

    /// The pinned fingerprint for `node`, when one is configured.
    pub fn pinned_fingerprint(&self, node: &NodeId) -> Option<&KeyFingerprint> {
        self.known_peer(node)?.key_fingerprint.as_ref()
    }

    /// Add or replace a peer entry.
    pub fn upsert(&mut self, peer: KnownPeer) {
        self.peers.retain(|existing| existing.node != peer.node);
        self.peers.push(peer);
    }

    /// The entry for `node`, if this runtime knows it.
    pub fn known_peer(&self, node: &NodeId) -> Option<&KnownPeer> {
        self.peers.iter().find(|peer| &peer.node == node)
    }

    /// Every configured peer, in configuration order.
    pub fn peers(&self) -> &[KnownPeer] {
        &self.peers
    }

    /// Whether `node` may use the mesh at all.
    pub fn is_trusted(&self, node: &NodeId) -> bool {
        self.known_peer(node)
            .is_some_and(|p| p.trust == PeerTrust::Trusted)
    }

    /// Fail closed unless `node` is known and trusted.
    pub fn authorize(&self, node: &NodeId) -> Result<(), MeshError> {
        match self.known_peer(node) {
            None => Err(MeshError::Untrusted {
                node: node.to_string(),
                reason: "not in known peers".to_string(),
            }),
            Some(peer) if peer.trust != PeerTrust::Trusted => Err(MeshError::Untrusted {
                node: node.to_string(),
                reason: "explicitly untrusted".to_string(),
            }),
            Some(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> NodeId {
        NodeId::new(id).unwrap()
    }

    #[test]
    fn an_empty_policy_denies_everyone() {
        let policy = TrustPolicy::new();
        assert!(policy.authorize(&node("node-b")).is_err());
    }

    #[test]
    fn trust_is_explicit_and_can_be_withdrawn_without_forgetting_the_peer() {
        let policy = TrustPolicy::new()
            .with_trusted(node("node-b"), "memory://node-b")
            .with_untrusted(node("node-c"), "memory://node-c");
        assert!(policy.authorize(&node("node-b")).is_ok());
        let err = policy.authorize(&node("node-c")).unwrap_err();
        assert!(matches!(err, MeshError::Untrusted { .. }));
        // The peer stays *known*; being untrusted is not being invisible.
        assert_eq!(policy.peers().len(), 2);
        assert!(policy.known_peer(&node("node-c")).is_some());
    }
}
