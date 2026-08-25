#![allow(unexpected_cfgs)]

//! v1 era apeireth-motivation transcription (4 files).
//!
//! Source: crates/_archived/v1.0-legacy/apeireth-motivation/src
//! Files transcribed verbatim:
//!   - bridge_kani_proofs.rs
//!   - consciousness_bridge.rs
//!   - life_force_bridge.rs
//!   - organ_kani_proofs.rs (kept verbatim)

// v1 lib.rs (transcribed + reconstructed to wire the modules below)
#[cfg(feature = "v1-motivation")]
pub mod lib;
#[cfg(feature = "v1-motivation")]
pub mod bridge_kani_proofs;
#[cfg(feature = "v1-motivation")]
pub mod consciousness_bridge;
#[cfg(feature = "v1-motivation")]
pub mod life_force_bridge;
#[cfg(feature = "v1-motivation")]
mod organ_kani_proofs;
