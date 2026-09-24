//! Canonical module and capability model.
//!
//! # The one idea
//!
//! **A module is a capability provider, and the only runtime extension
//! entity.** Not a tool, and not a thing a "plugin" would be a second kind of.
//! One module may own any number of capabilities of any kinds, observe any
//! number of hook points, and carry a lifecycle; every capability in the system
//! — tools, providers, memory backends, transports — is reached the same way,
//! through one registry, no matter whether the module was compiled in, installed
//! as a package, or loaded from a development build.
//!
//! "Plugin" survives only as the product-layer name for a user-installed module
//! package, and as compatibility aliases in [`plugin`].
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
//! This crate replaces that with two registries and a rule: [`ModuleRegistry`]
//! owns modules, [`CapabilityRegistry`] owns the id-to-owner index, and nothing
//! else stores a second copy of either fact. The index is an index, not a copy —
//! a capability's declaration continues to live only in its owner's manifest.
//! Typed views over the registries are encouraged, and [`ModuleManager`] provides
//! them; second sources of truth are not.
//!
//! # Layering
//!
//! Depends on `apeireth-core` and `apeireth-protocol`. Must not depend on
//! `apeireth-runtime`: capabilities are things the runtime composes, so a
//! capability that knew about the runtime would invert the whole arrangement.
//! The runtime implements the two ports this crate declares — [`ModuleInvoker`]
//! and [`SubLoopSpawner`] — which is what lets modules observe a turn without
//! ever seeing a runtime type.
//!
//! # Scope
//!
//! Static, in-process modules. Dynamic library loading, WASM, hot reload, remote
//! modules, and a marketplace are out of scope. See [`Module`].
//!
//! # Shape
//!
//! ```text
//! ModuleManifest  declares  ->  CapabilityDescriptor (id + kind)
//!       |                              |
//! ModuleRegistry (canonical)     CapabilityRegistry (id -> owner index)
//!       \                              /
//!        `------  ModuleManager  ------'
//!            lifecycle + deps + hook index + typed views
//! ```

#![deny(unsafe_code)]

pub mod alias;
pub mod availability;
pub mod bounds;
pub mod capability;
pub mod credentials;
pub mod error;
pub mod experience;
pub mod llm_factory;
// O-6 归位 (2026-09-08): plugin LlmFactory → orchestration 镜像 trait 桥.
// 桥归 plugin (依赖 orchestration 且拥有 plugin 侧 trait), 单一事实源,
// CLI/gateway/测试共用 (拒绝 composition root 各写一份的复制漂移).
pub mod llm_orchestration_mirror;
pub use llm_orchestration_mirror::MirrorLlmFactory;
pub mod manager;
pub mod manifest;
pub mod mcp;
pub mod memory_backend;
pub mod module;
pub mod organ;
pub mod perception;
pub mod perception_backend;
pub mod plugin;
pub mod preference;
pub mod provider;
pub mod registry;
pub mod self_assessment;
pub mod semver;
pub mod subloop;
pub mod tool;
pub mod watch;

pub use alias::{
    aliases_by_id, index_manifests, index_pairs, unresolved_token, AliasIndex, AliasResolver,
    LruCache, DEFAULT_CACHE_SIZE, LEGACY_UNRESOLVED_PREFIX,
};
pub use availability::{Availability, AvailabilityError, AvailabilityReport};
pub use bounds::{
    audit_bounds, check_call, check_input_size, check_output_size, check_permissions,
    default_caller_permissions, is_valid_kebab, privileged_caller_permissions,
    validate_extension_name, validate_permissions, validate_plugin_manifest_text,
    validate_resource_bounds, validate_version, BoundError, ResourceBounds,
};
pub use capability::{CapabilityDescriptor, CapabilityKind};
pub use credentials::{CredentialResolver, NoCredentials, Secret, StaticCredentials};
pub use error::{ModuleResult, ModuleSystemError, PluginError, PluginResult};
pub use manager::ModuleManager;
pub use manifest::ModuleManifest;
// Root-level spellings of the historical "plugin" vocabulary, so embedders keep
// compiling unchanged. They resolve to the same types as the Module-centric
// names above; see [`plugin`] for the rationale.
pub use manifest::ModuleManifest as PluginManifest;
pub use module::{Module as Plugin, ModuleInitContext as PluginContext};
pub use registry::ModuleRegistry as PluginRegistry;
pub use semver::{
    compare as compare_semver, is_valid as is_valid_semver, parse as parse_semver, Semver,
    SemverError,
};
pub use watch::{
    discover_metadata_files, file_mtime_unix, should_invalidate, MetadataWatcher, WatchEvent,
};
// O-6 锚兑现 #12: 统一 capability trait 错误通道 `CapabilityResult<T>` 在 crate root
// 可用, 避免每个 use 点写 `crate::memory_backend::CapabilityResult`.
pub use memory_backend::CapabilityResult;
pub use module::{
    reject_tool_identity_collisions, CapabilityOwnership, HookPoint, InvocationContext,
    InvocationOrigin, Module, ModuleContext, ModuleDirective, ModuleError, ModuleExports,
    ModuleInitContext, ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse,
    ModuleInvoker, ModuleOrigin, ModuleOutcome, ModuleTurnState, PromptOverlay,
    DEFAULT_MAX_INVOCATION_DEPTH, DEFAULT_MAX_MODULE_INVOCATIONS,
};
pub use perception_backend::{
    AudioBuffer, LangHint, NoopVoiceBackend, PerceptionBackendError, ScreenshotBytes, TactileState,
    Transcription, VoiceBackend, WhisperBackend,
};
pub use provider::{ProviderCapability, ProviderError, ProviderHotConfig};
pub use registry::{CapabilityRecord, CapabilityRegistry, ModuleRegistry};
pub use subloop::{SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec};
pub use tool::{FrozenInvocation, ToolCapability};
