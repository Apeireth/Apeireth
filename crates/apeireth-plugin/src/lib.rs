//! Canonical plugin and capability model.
//!
//! # The one idea
//!
//! **A plugin is a capability provider.** Not a tool. One plugin may provide any
//! number of capabilities of any kinds, and every capability in the system —
//! tools, providers, memory backends, transports — is reached the same way,
//! through one registry.
//!
//! # Why this crate exists
//!
//! The workspace it converges had three separate `PluginRegistry` types, three
//! separate `Capability` structs, and two separate `ToolRegistry` types. None was
//! wrong on its own; the defect was that no single one could answer "what can
//! this runtime do". Each new integration added a registry, and each registry
//! became another place that had to be consulted, kept in sync, and eventually
//! contradicted the others.
//!
//! This crate replaces that with one declaration model — a plugin declares
//! capabilities, each with a stable id and a typed kind — over which the
//! canonical registries are built.
//!
//! # Layering
//!
//! Depends on `apeireth-core` and `apeireth-protocol`. Must not depend on
//! `apeireth-runtime`: capabilities are things the runtime composes, so a
//! capability that knew about the runtime would invert the whole arrangement.
//!
//! # Scope
//!
//! Static, in-process plugins. Dynamic library loading, WASM, hot reload, remote
//! plugins, and a marketplace are out of scope. See [`plugin::Plugin`].
//!
//! # Shape
//!
//! ```text
//!   PluginManifest  declares  ->  CapabilityDescriptor (id + kind)
//!         |                              |
//!    Plugin (lifecycle)          ToolCapability / ProviderCapability
//! ```

#![deny(unsafe_code)]

pub mod capability;
pub mod credentials;
pub mod error;
pub mod manifest;
pub mod plugin;
pub mod provider;
pub mod tool;

pub use capability::{CapabilityDescriptor, CapabilityKind};
pub use credentials::{CredentialResolver, NoCredentials, Secret, StaticCredentials};
pub use error::{PluginError, PluginResult};
pub use manifest::PluginManifest;
pub use plugin::{Plugin, PluginContext};
pub use provider::{ProviderCapability, ProviderError};
pub use tool::ToolCapability;
