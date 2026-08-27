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
//! This crate replaces that with two registries and a rule: [`PluginRegistry`]
//! owns plugins, [`CapabilityRegistry`] owns the id-to-owner index, and nothing
//! else stores a second copy of either fact. The index is an index, not a copy —
//! a capability's declaration continues to live only in its owner's manifest.
//! Typed views over the registries are encouraged, and [`PluginManager`] provides
//! them; second sources of truth are not.
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
//!    PluginRegistry                CapabilityRegistry (id -> owner)
//!         \                              /
//!          `------  PluginManager  ------'
//!                  lifecycle + typed views
//!                          |
//!            ToolCapability / ProviderCapability
//! ```

#![deny(unsafe_code)]

pub mod capability;
pub mod credentials;
pub mod error;
pub mod experience;
pub mod manager;
pub mod manifest;
pub mod memory_backend;
pub mod perception;
pub mod plugin;
pub mod provider;
pub mod registry;
pub mod tool;

pub use capability::{CapabilityDescriptor, CapabilityKind};
pub use credentials::{CredentialResolver, NoCredentials, Secret, StaticCredentials};
pub use error::{PluginError, PluginResult};
pub use manager::PluginManager;
pub use manifest::PluginManifest;
// O-6 锚兑现 #12: 统一 capability trait 错误通道 `CapabilityResult<T>` 在 crate root
// 可用, 避免每个 use 点写 `crate::memory_backend::CapabilityResult`.
pub use memory_backend::CapabilityResult;
pub use plugin::{Plugin, PluginContext};
pub use provider::{ProviderCapability, ProviderError};
pub use registry::{CapabilityRecord, CapabilityRegistry, PluginRegistry};
pub use tool::{FrozenInvocation, ToolCapability};
