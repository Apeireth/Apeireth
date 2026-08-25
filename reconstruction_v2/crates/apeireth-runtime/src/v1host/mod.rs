//! v1 era apeireth-host transcription (4 named files).
//!
//! Source: crates/_archived/v1.0-legacy/apeireth-host/src
//! Files transcribed verbatim:
//!   - lib.rs             (RECONSTRUCTED — v1 lib also had machine_id + organ_kani_proofs
//!                          which are out-of-scope; this v2 lib wires only the 3 modules
//!                          listed in the transcription task)
//!   - atomic_write.rs    (write tmp → rename → cleanup, JsonSupport 1:1)
//!   - three_way.rs       (3-way conflict detection before destructive ops)
//!   - keyring.rs         (OS keyring + AES-256-GCM encrypted fallback)

// v1 lib.rs (transcribed + reconstructed to wire only the 3 in-scope modules)
pub mod lib;
// v1 in-scope modules
pub mod atomic_write;
pub mod keyring;
pub mod three_way;

pub use three_way::{
    detect, detect_with_force, ConflictDiff, DetectOutcome, FileEntry, FileScope, FileSnapshot,
    ThreeWayComparable, ThreeWayConflict, ThreeWayError,
};
