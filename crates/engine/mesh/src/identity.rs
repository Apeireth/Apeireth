//! Persistent node identity: the private key that proves *this* NodeId.
//!
//! # What this is
//!
//! [`NodeId`] is a logical identity; a socket address is not one, and a
//! claimed id is not authentication. [`NodeIdentity`] owns the Ed25519
//! keypair that authenticates the claim: the same key that terminates the
//! transport's TLS channel (certificate key == mesh identity key), so
//! possession of the private key pinned for a `NodeId` is proven by the
//! handshake itself — fresh ephemeral key exchange, replay-immune.
//!
//! # Persistence
//!
//! Keys survive restarts via a small local file ([`NodeIdentity::load_or_generate`]).
//! This is a permission-restricted plain file — **not** an OS keyring and not
//! equivalent to one. On Unix the file is written `0600`; on Windows the
//! platform ACL applies (best effort, documented honestly). The private key
//! never implements `Serialize`, never appears in `Debug` output (redacted),
//! and never leaves this module except as PKCS#8 bytes for certificate
//! building.
//!
//! # What this is not
//!
//! Not a PKI, not a certificate authority, not revocation. Trust stays
//! explicit pinning in [`crate::trust::TrustPolicy`]; there is no TOFU.

use std::fmt;
use std::path::Path;

use ring::signature::KeyPair as _;
use sha2::{Digest, Sha256};

/// The pinned proof of a node's public key: hex SHA-256 of the raw 32-byte
/// Ed25519 public key. Comparable, serializable, map-friendly — and never a
/// secret.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct KeyFingerprint(String);

impl KeyFingerprint {
    /// Compute the fingerprint of a raw Ed25519 public key.
    pub fn of_public_key(public_key: &[u8]) -> Self {
        let digest = Sha256::digest(public_key);
        Self(
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )
    }

    /// The hex digest as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KeyFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Failure handling an identity or its key file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdentityError {
    /// The key file exists but does not parse as this node's identity.
    #[error("node identity file is invalid: {0}")]
    Invalid(String),
    /// The key file could not be read or written.
    #[error("node identity file i/o failed: {0}")]
    Io(String),
    /// The platform key provider refused the operation.
    #[error("node identity key operation failed: {0}")]
    Key(String),
}

/// One node's long-lived signing identity.
///
/// `Debug` is redacted: the private key never appears in logs or error
/// output, and this type deliberately implements no serde traits — the only
/// serialization is the explicit key file.
pub struct NodeIdentity {
    /// PKCS#8 v2 document of the Ed25519 private key (ring form). Held only
    /// for certificate building; never logged, never advertised.
    pkcs8: Vec<u8>,
    /// Raw 32-byte Ed25519 public key.
    public_key: [u8; 32],
    fingerprint: KeyFingerprint,
}

impl fmt::Debug for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeIdentity")
            .field("public_key", &hex(&self.public_key))
            .field("fingerprint", &self.fingerprint)
            .field("private_key", &"<redacted>")
            .finish()
    }
}

impl NodeIdentity {
    /// Mint a fresh identity. The caller persists it — a node that regenerates
    /// every boot is a node its peers cannot keep pinned.
    pub fn generate() -> Result<Self, IdentityError> {
        let rng = ring::rand::SystemRandom::new();
        let document = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|error| IdentityError::Key(error.to_string()))?;
        let pkcs8 = document.as_ref().to_vec();
        let key = ring::signature::Ed25519KeyPair::from_pkcs8(&pkcs8)
            .map_err(|error| IdentityError::Key(error.to_string()))?;
        let mut public_key = [0u8; 32];
        let raw = key.public_key().as_ref();
        if raw.len() != public_key.len() {
            return Err(IdentityError::Key(
                "unexpected Ed25519 public key length".to_string(),
            ));
        }
        public_key.copy_from_slice(raw);
        Ok(Self::from_parts(pkcs8, public_key))
    }

    /// Load an identity from `path`. Fails closed on malformed content.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, IdentityError> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|error| IdentityError::Io(error.to_string()))?;
        Self::from_file_text(&text)
    }

    /// Load from `path`, generating and persisting a fresh identity when the
    /// file does not exist yet.
    pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Self, IdentityError> {
        match Self::load(path.as_ref()) {
            Ok(identity) => Ok(identity),
            Err(IdentityError::Io(_)) => {
                let identity = Self::generate()?;
                identity.save(path.as_ref())?;
                Ok(identity)
            }
            Err(error) => Err(error),
        }
    }

    /// Persist this identity to `path`.
    ///
    /// Permission-restricted plain file (`0600` on Unix). Not an OS keyring.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), IdentityError> {
        std::fs::write(path.as_ref(), self.to_file_text())
            .map_err(|error| IdentityError::Io(error.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path.as_ref(), std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// Raw Ed25519 public key bytes.
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// The fingerprint peers pin.
    pub fn fingerprint(&self) -> &KeyFingerprint {
        &self.fingerprint
    }

    /// PKCS#8 bytes for building the TLS certificate that terminates this
    /// identity's channel.
    ///
    /// Private material — never log, advertise, or transmit this. It exists
    /// so the transport layer can bind the channel key to the identity key.
    pub fn pkcs8_der(&self) -> &[u8] {
        &self.pkcs8
    }

    fn from_parts(pkcs8: Vec<u8>, public_key: [u8; 32]) -> Self {
        let fingerprint = KeyFingerprint::of_public_key(&public_key);
        Self {
            pkcs8,
            public_key,
            fingerprint,
        }
    }

    fn to_file_text(&self) -> String {
        // Explicit, versioned key file. The only place private bytes serialize.
        format!(
            "apeireth-node-identity-v1\npublic={}\nprivate={}\n",
            hex(&self.public_key),
            hex(&self.pkcs8)
        )
    }

    fn from_file_text(text: &str) -> Result<Self, IdentityError> {
        let invalid = |why: &str| IdentityError::Invalid(why.to_string());
        let mut lines = text.lines();
        if lines.next() != Some("apeireth-node-identity-v1") {
            return Err(invalid("unknown identity file format"));
        }
        let read_field = |line: Option<&str>, name: &str| -> Result<Vec<u8>, IdentityError> {
            let line = line.ok_or_else(|| invalid(&format!("missing {name} field")))?;
            let value = line
                .strip_prefix(&format!("{name}="))
                .ok_or_else(|| invalid(&format!("malformed {name} field")))?;
            unhex(value).ok_or_else(|| invalid(&format!("{name} is not hex")))
        };
        let public_bytes = read_field(lines.next(), "public")?;
        let private_bytes = read_field(lines.next(), "private")?;
        if public_bytes.len() != 32 {
            return Err(invalid("public key must be 32 bytes"));
        }
        let mut public_key = [0u8; 32];
        public_key.copy_from_slice(&public_bytes);
        // Round-trip through the key provider so a corrupted private half is
        // rejected here rather than at first use.
        ring::signature::Ed25519KeyPair::from_pkcs8(&private_bytes)
            .map_err(|error| invalid(&format!("private key rejected: {error}")))?;
        Ok(Self::from_parts(private_bytes, public_key))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_identity_round_trips_through_the_key_file() {
        let identity = NodeIdentity::generate().unwrap();
        let text = identity.to_file_text();
        let back = NodeIdentity::from_file_text(&text).unwrap();
        assert_eq!(back.public_key(), identity.public_key());
        assert_eq!(back.fingerprint(), identity.fingerprint());
    }

    #[test]
    fn the_private_key_never_appears_in_debug_output() {
        let identity = NodeIdentity::generate().unwrap();
        let dumped = format!("{identity:?}");
        assert!(dumped.contains("<redacted>"), "{dumped}");
        assert!(
            !dumped.contains(&hex(&identity.pkcs8)),
            "Debug must not leak private key bytes"
        );
    }

    #[test]
    fn a_corrupted_private_key_is_rejected_at_load_time() {
        let identity = NodeIdentity::generate().unwrap();
        let mut text = identity.to_file_text();
        text = text.replace(&hex(&identity.pkcs8), "00");
        assert!(NodeIdentity::from_file_text(&text).is_err());
    }

    #[test]
    fn fingerprints_differ_for_different_keys() {
        let a = NodeIdentity::generate().unwrap();
        let b = NodeIdentity::generate().unwrap();
        assert_ne!(a.fingerprint(), b.fingerprint());
        assert_eq!(a.fingerprint().as_str().len(), 64, "sha-256 hex");
    }

    #[test]
    fn load_or_generate_persists_and_reuses() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-identity-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("node.key");
        let first = NodeIdentity::load_or_generate(&path).unwrap();
        let second = NodeIdentity::load_or_generate(&path).unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        std::fs::remove_file(&path).ok();
    }
}
