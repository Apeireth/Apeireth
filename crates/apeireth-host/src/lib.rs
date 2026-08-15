//! Apeireth host infrastructure facade.
//!
//! The facade keeps security-sensitive host services in one crate while preserving
//! explicit namespaces for the two distinct responsibilities:
//! `keyring` stores secrets and `machine_id` identifies the local host.

#![warn(missing_docs)]

/// Secure OS keyring and encrypted-file fallback.
pub mod keyring;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
/// Cross-platform machine identity providers and detection.
pub mod machine_id;

pub use keyring::*;
pub use machine_id::{
    detect as detect_machine_id, derive_id, get_machine_id, hash_machine_id, MachineId,
    MachineIdError, MachineIdExport, MachineIdResult, MachineIdResultStd,
};