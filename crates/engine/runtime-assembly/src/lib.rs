//! Production composition for [`apeireth_runtime`].
//!
//! Desktop composition is the default feature. The portable host builds without
//! desktop capabilities; memory and durable profiles can be selected separately.

#![deny(unsafe_code)]

pub mod portable;
pub use portable::{AssemblyError, PortableHost};

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "desktop")]
pub use desktop::*;

#[cfg(all(feature = "memory", not(feature = "desktop")))]
#[path = "canonical/memory_module.rs"]
pub mod memory_module;
#[cfg(all(feature = "memory", feature = "desktop"))]
pub use canonical::memory_module;

#[cfg(all(feature = "profile-store", not(feature = "desktop")))]
pub mod profile_store;
