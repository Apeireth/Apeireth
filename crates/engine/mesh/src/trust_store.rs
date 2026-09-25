//! Persistent known-peer trust.
//!
//! # What is stored, and what is not
//!
//! Only trust *metadata*: `NodeId`, endpoint, pinned key fingerprint, trust
//! level. No capabilities, no implementations, no secrets. Discovery may
//! update an endpoint for a peer whose fingerprint already matches — LAN DHCP
//! moves addresses — but a **different fingerprint for a known NodeId is a
//! security event** ([`TrustStoreError::KeyMismatch`]): identity is never
//! silently updated. Recovery is forget + re-pair, never "accept the new key".
//!
//! # Atomic writes
//!
//! The file is replaced via temp-write + rename, so a power cut cannot leave
//! a half-written peer list behind.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::NodeId;
use apeireth_mesh_protocol::KeyFingerprint;
use serde::{Deserialize, Serialize};

use crate::trust::{KnownPeer, PeerTrust};

/// Failure reading or writing the trust store.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TrustStoreError {
    /// The store file exists but does not parse.
    #[error("trust store is invalid: {0}")]
    Invalid(String),
    /// Filesystem failure.
    #[error("trust store i/o failed: {0}")]
    Io(String),
    /// A known NodeId presented a different key fingerprint. Never updated
    /// automatically — this is a security event requiring re-pairing.
    #[error("key mismatch for node {node}: pinned {expected}, presented {presented}")]
    KeyMismatch {
        /// The contested node identity.
        node: NodeId,
        /// The pinned fingerprint.
        expected: String,
        /// The newly presented fingerprint.
        presented: String,
    },
}

/// Result of reconciling discovery metadata against pinned trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointUpdate {
    /// The endpoint already matched; nothing changed.
    AlreadyCurrent,
    /// The endpoint moved (same NodeId, same fingerprint) and was updated.
    Updated {
        /// The previous endpoint.
        from: String,
        /// The new endpoint.
        to: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct TrustStoreFile {
    version: u32,
    peers: Vec<KnownPeer>,
}

/// The persisted set of known peers. All mutations save atomically.
pub struct TrustStore {
    path: PathBuf,
    peers: std::sync::Mutex<BTreeMap<NodeId, KnownPeer>>,
}

impl TrustStore {
    /// Load the store at `path`, or start empty when the file does not exist.
    pub fn load_or_new(path: impl AsRef<Path>) -> Result<Self, TrustStoreError> {
        let path = path.as_ref().to_path_buf();
        let peers = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let file: TrustStoreFile = serde_json::from_str(&text)
                    .map_err(|error| TrustStoreError::Invalid(error.to_string()))?;
                if file.version != 1 {
                    return Err(TrustStoreError::Invalid(format!(
                        "unsupported trust store version {}",
                        file.version
                    )));
                }
                file.peers
                    .into_iter()
                    .map(|peer| (peer.node.clone(), peer))
                    .collect()
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(TrustStoreError::Io(error.to_string())),
        };
        Ok(Self {
            path,
            peers: std::sync::Mutex::new(peers),
        })
    }

    /// Every known peer, in node order.
    pub fn known_peers(&self) -> Vec<KnownPeer> {
        self.peers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// One known peer.
    pub fn get(&self, node: &NodeId) -> Option<KnownPeer> {
        self.peers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(node)
            .cloned()
    }

    /// Persist a freshly paired peer as trusted with a pinned fingerprint.
    ///
    /// The pairing ceremony — bilateral user confirmation over an
    /// authenticated channel — is the only way a peer reaches this method.
    pub fn record_paired(
        &self,
        node: NodeId,
        endpoint: impl Into<String>,
        key_fingerprint: KeyFingerprint,
    ) -> Result<(), TrustStoreError> {
        {
            let mut peers = self.peers.lock().unwrap_or_else(|p| p.into_inner());
            peers.insert(
                node.clone(),
                KnownPeer {
                    node,
                    endpoint: endpoint.into(),
                    trust: PeerTrust::Trusted,
                    key_fingerprint: Some(key_fingerprint),
                },
            );
        }
        self.save()
    }

    /// Reconcile a discovery observation against pinned trust.
    ///
    /// Same NodeId + same fingerprint → endpoint may move (DHCP).
    /// Same NodeId + different fingerprint → [`TrustStoreError::KeyMismatch`]:
    /// never an automatic identity update. Unknown nodes are ignored here —
    /// discovery does not create trust.
    pub fn observe_discovered(
        &self,
        node: &NodeId,
        endpoint: impl Into<String>,
        presented: &KeyFingerprint,
    ) -> Result<EndpointUpdate, TrustStoreError> {
        let endpoint = endpoint.into();
        let mut peers = self.peers.lock().unwrap_or_else(|p| p.into_inner());
        let Some(peer) = peers.get_mut(node) else {
            return Ok(EndpointUpdate::AlreadyCurrent);
        };
        match &peer.key_fingerprint {
            Some(pinned) if pinned == presented => {
                if peer.endpoint == endpoint {
                    Ok(EndpointUpdate::AlreadyCurrent)
                } else {
                    let from = std::mem::replace(&mut peer.endpoint, endpoint.clone());
                    drop(peers);
                    self.save()?;
                    Ok(EndpointUpdate::Updated { from, to: endpoint })
                }
            }
            Some(pinned) => Err(TrustStoreError::KeyMismatch {
                node: node.clone(),
                expected: pinned.to_string(),
                presented: presented.to_string(),
            }),
            // An unpinned entry cannot validate identity claims; leave it be.
            None => Ok(EndpointUpdate::AlreadyCurrent),
        }
    }

    /// Forget a peer: remove its persisted trust. The local node identity is
    /// never touched — unpairing is about the *other* side.
    pub fn forget(&self, node: &NodeId) -> Result<bool, TrustStoreError> {
        let removed = {
            let mut peers = self.peers.lock().unwrap_or_else(|p| p.into_inner());
            peers.remove(node).is_some()
        };
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Atomic replace: temp file in the same directory, then rename.
    fn save(&self) -> Result<(), TrustStoreError> {
        let file = TrustStoreFile {
            version: 1,
            peers: self
                .peers
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .values()
                .cloned()
                .collect(),
        };
        let text = serde_json::to_string_pretty(&file)
            .map_err(|e| TrustStoreError::Invalid(e.to_string()))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| TrustStoreError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, &self.path).map_err(|e| TrustStoreError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("apeireth-trust-{tag}-{}.json", std::process::id()))
    }

    fn fp(seed: u8) -> KeyFingerprint {
        KeyFingerprint::of_public_key(&[seed; 32])
    }

    #[test]
    fn paired_peers_persist_and_reload() {
        let path = temp_path("roundtrip");
        let store = TrustStore::load_or_new(&path).unwrap();
        store
            .record_paired(NodeId::new("node-b").unwrap(), "127.0.0.1:7000", fp(1))
            .unwrap();
        let reloaded = TrustStore::load_or_new(&path).unwrap();
        let peer = reloaded.get(&NodeId::new("node-b").unwrap()).unwrap();
        assert_eq!(peer.endpoint, "127.0.0.1:7000");
        assert_eq!(peer.trust, PeerTrust::Trusted);
        assert_eq!(peer.key_fingerprint, Some(fp(1)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn endpoint_moves_are_allowed_but_key_changes_are_a_security_event() {
        let path = temp_path("endpoint");
        let store = TrustStore::load_or_new(&path).unwrap();
        let node = NodeId::new("node-b").unwrap();
        store
            .record_paired(node.clone(), "192.168.1.10:7000", fp(1))
            .unwrap();

        // Same key, new address: updated.
        let update = store
            .observe_discovered(&node, "192.168.1.42:7000", &fp(1))
            .unwrap();
        assert!(matches!(update, EndpointUpdate::Updated { .. }));
        assert_eq!(store.get(&node).unwrap().endpoint, "192.168.1.42:7000");

        // Same NodeId, different key: refused, nothing updated.
        let err = store
            .observe_discovered(&node, "192.168.1.99:7000", &fp(2))
            .unwrap_err();
        assert!(matches!(err, TrustStoreError::KeyMismatch { .. }));
        assert_eq!(store.get(&node).unwrap().endpoint, "192.168.1.42:7000");
        assert_eq!(store.get(&node).unwrap().key_fingerprint, Some(fp(1)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn discovery_alone_never_creates_trust() {
        let path = temp_path("notrust");
        let store = TrustStore::load_or_new(&path).unwrap();
        let update = store
            .observe_discovered(&NodeId::new("node-x").unwrap(), "10.0.0.5:1", &fp(9))
            .unwrap();
        assert_eq!(update, EndpointUpdate::AlreadyCurrent);
        assert!(store.get(&NodeId::new("node-x").unwrap()).is_none());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn forget_removes_only_the_peer() {
        let path = temp_path("forget");
        let store = TrustStore::load_or_new(&path).unwrap();
        let node = NodeId::new("node-b").unwrap();
        store.record_paired(node.clone(), "e", fp(1)).unwrap();
        assert!(store.forget(&node).unwrap());
        assert!(store.get(&node).is_none());
        // And the file reflects it.
        let reloaded = TrustStore::load_or_new(&path).unwrap();
        assert!(reloaded.get(&node).is_none());
        std::fs::remove_file(&path).ok();
    }
}
