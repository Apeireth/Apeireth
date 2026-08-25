//! Cross-platform machine identity providers.
//!
//! **Status**: ⚠️ stub — v2 port placeholder.
//!
//! The original v1 `apeireth-host::machine_id` module contained the real
//! cross-platform machine-id detection logic. During the v1 → v2 reconstruction
//! that logic was deliberately stubbed out so that downstream crates which only
//! depend on the keyring surface continue to compile.
//!
//! All names exported by `lib.rs` are present below as inert stubs.

#![allow(missing_docs)]

use std::fmt;

/// Machine identity error stub.
#[derive(Debug, Clone)]
pub enum MachineIdError {
    /// The OS-specific detector was not implemented in the v2 stub yet.
    UnsupportedPlatform(String),
    /// I/O failure (unused in stub but kept for API parity).
    Io(String),
}

impl fmt::Display for MachineIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MachineIdError::UnsupportedPlatform(p) => {
                write!(f, "machine_id: platform not supported (v2 stub): {p}")
            }
            MachineIdError::Io(e) => write!(f, "machine_id: io error (v2 stub): {e}"),
        }
    }
}

impl std::error::Error for MachineIdError {}

/// Standard-machine-id result alias.
pub type MachineIdResultStd = Result<String, MachineIdError>;

/// Generic machine-id result alias (kept for API parity).
pub type MachineIdResult<T> = Result<T, MachineIdError>;

/// Exported machine-id snapshot (stub).
#[derive(Debug, Clone)]
pub struct MachineIdExport {
    /// Stable opaque identifier string (stub value).
    pub id: String,
    /// Platform name (e.g. "windows", "linux").
    pub platform: String,
}

impl MachineIdExport {
    /// Stub constructor.
    #[must_use]
    pub fn new(id: impl Into<String>, platform: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            platform: platform.into(),
        }
    }
}

/// Machine identity value type alias (stub).
#[derive(Debug, Clone)]
pub struct MachineId(pub String);

impl MachineId {
    /// Stub constructor.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// Stub accessor.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Detect the current platform (stub — returns "unknown").
pub fn detect() -> MachineIdResultStd {
    Ok("unknown".to_string())
}

/// Derive an opaque id (stub — returns "v2-stub-machine-id").
#[must_use]
pub fn derive_id() -> String {
    "v2-stub-machine-id".to_string()
}

/// Get the local machine-id (stub).
pub fn get_machine_id() -> MachineIdResultStd {
    Ok("v2-stub-machine-id".to_string())
}

/// Hash a machine-id into a stable hex string (stub).
#[must_use]
pub fn hash_machine_id(input: &str) -> String {
    // Trivial deterministic stub: hex length 16.
    let mut out = String::with_capacity(input.len().min(64));
    for b in input.bytes().take(32) {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Legacy placeholder retained for callers referencing the empty v1 module.
pub fn placeholder() {}
