//! Compatibility spellings for the module ABI.
//!
//! The runtime extension entity is a **module**; these aliases exist so that
//! embedders written against the old "plugin" vocabulary keep compiling while
//! they migrate. Every alias points at the same type, in the same registry, on
//! the same lifecycle path — none of them is a second runtime primitive.
//!
//! This module is scheduled for deletion once the callers are gone; new code
//! should name [`crate::Module`], [`crate::ModuleManifest`], and
//! [`crate::ModuleManager`] directly.

pub use crate::manager::ModuleManager as PluginManager;
pub use crate::manifest::ModuleManifest as PluginManifest;
pub use crate::module::{Module as Plugin, ModuleExports, ModuleInitContext as PluginContext};
pub use crate::registry::ModuleRegistry as PluginRegistry;
