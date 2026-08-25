//! v1 era apeireth-state transcription (3 named files + 1 supporting error.rs).
//!
//! Source: crates/_archived/v1.0-legacy/apeireth-state/src
//! Files transcribed verbatim:
//!   - shared_state.rs    (SharedState trait + SharedStateMode enum + read/write guards)
//!   - organ.rs           (9-organ enum compile-time hardcode)
//!   - mode_once_lock.rs  (OnceLockState process-global lazy init)
//!   - error.rs           (SUPPORTING — v1 StateError 5-variant enum, not in 14-file count
//!                          but required by shared_state.rs / mode_once_lock.rs)
//!
//! Note: v1 imports `crate::error::*` / `crate::organ::*` / `crate::shared_state::*`
//! were rewritten to `super::*` for this nested layout (preserves semantics).

pub mod error;
pub mod mode_once_lock;
pub mod organ;
pub mod shared_state;
