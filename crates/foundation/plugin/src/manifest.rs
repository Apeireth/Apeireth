//! What a module declares about itself before it runs.
//!
//! The manifest is deliberately inert data. The manager reads it to detect
//! duplicate ids, duplicate capabilities, missing dependencies, and dependency
//! cycles *before* calling anyone's `initialize`. A system that discovers a
//! conflict halfway through boot has already run half its modules' start-up code
//! and has no clean way back.
//!
//! `origin` records **how the module arrived** (compiled in, installed as a
//! package, loaded from a development build) and nothing else. It never
//! participates in dispatch, governance, or ordering: a builtin module and an
//! installed one are the same runtime object with a different provenance stamp.
//! A future `location` (local vs remote node) is a separate axis and is
//! deliberately not modelled here.

use apeireth_core::kernel::{CapabilityId, Metadata, ModuleId};
use serde::{Deserialize, Serialize};

use crate::capability::{CapabilityDescriptor, CapabilityKind};
use crate::error::{PluginError, PluginResult};
use crate::module::{CapabilityOwnership, HookPoint};

/// A module's self-declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// Stable identity, e.g. `builtin.calculator`.
    pub id: ModuleId,
    /// Module version. Free-form; not interpreted by the manager.
    pub version: String,
    /// Short display name, e.g. `Organ cognition`.
    #[serde(default)]
    pub name: String,
    /// What this module is for, in one line.
    pub description: String,
    /// How this module arrived. Provenance only; never runtime semantics.
    #[serde(default)]
    pub origin: ModuleOrigin,
    /// Capabilities this module provides.
    pub capabilities: Vec<CapabilityDescriptor>,
    /// How this module's capability set relates to its manifest.
    ///
    /// The single declaration of static vs dynamic capability ownership:
    /// the manager reads it from here and nowhere else, so there is no second
    /// place that could disagree. Defaults to [`CapabilityOwnership::Static`].
    #[serde(default)]
    pub capability_ownership: CapabilityOwnership,
    /// Modules that must be active before this one starts.
    ///
    /// Used only for ordering. A module does not gain access to its
    /// dependencies' internals by naming them; it reaches them, like everyone
    /// else, through the capability registry.
    pub dependencies: Vec<ModuleId>,
    /// Lifecycle points this module observes.
    ///
    /// Declaration of intent and the authoritative dispatch list: the runtime
    /// calls a module's hooks only for the points declared here. A module whose
    /// `on_hook` handles a point it never declared is never called for it.
    #[serde(default)]
    pub hooks: Vec<HookPoint>,
    /// Additional annotations.
    pub metadata: Metadata,
    /// Alternative lookup keys for this module id.
    ///
    /// Recovered from legacy agent alias maps. These are **not** a second
    /// identity: [`ModuleId`] remains unique, and [`crate::ModuleRegistry`]
    /// still keys on it. Aliases exist so a caller can resolve `@coder` to
    /// `builtin.coder` through [`crate::alias::AliasIndex`] without inventing
    /// a second registry. Empty by default; omitted JSON deserializes as empty.
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// How a module arrived.
///
/// Kept beside the manifest because provenance travels with the declaration.
/// See [`crate::module::ModuleOrigin`] for the semantics.
pub use crate::module::ModuleOrigin;

/// Compatibility alias for [`ModuleManifest`].
pub use self::ModuleManifest as PluginManifest;

impl ModuleManifest {
    /// Begin a manifest.
    pub fn new(id: ModuleId, version: impl Into<String>, description: impl Into<String>) -> Self {
        let description = description.into();
        Self {
            id,
            version: version.into(),
            // Without an explicit display name the description is the most
            // honest one available; `with_name` overrides it.
            name: description.clone(),
            description,
            origin: ModuleOrigin::System,
            capabilities: Vec::new(),
            capability_ownership: CapabilityOwnership::Static,
            dependencies: Vec::new(),
            hooks: Vec::new(),
            metadata: Metadata::new(),
            aliases: Vec::new(),
        }
    }

    /// Begin a manifest for a statically-compiled module.
    ///
    /// The id is a literal in the calling crate, so it is validated here and a
    /// malformed literal panics with the offending string rather than failing
    /// silently at first dispatch. Nothing dynamic may use this constructor.
    pub fn system(id: &str, version: impl Into<String>, description: impl Into<String>) -> Self {
        let id = ModuleId::new(id)
            .unwrap_or_else(|error| panic!("static module id {id:?} is invalid: {error}"));
        Self::new(id, version, description)
    }

    /// Declare a capability.
    ///
    /// Rejects a second declaration of the same id by the same module; that is
    /// always a mistake, and catching it here keeps the registry's duplicate
    /// check about genuine cross-module conflicts.
    pub fn declare(mut self, capability: CapabilityDescriptor) -> PluginResult<Self> {
        if self.capabilities.iter().any(|c| c.id == capability.id) {
            return Err(PluginError::DuplicateCapability {
                capability: capability.id,
                incumbent: self.id.clone(),
                challenger: self.id,
            });
        }
        self.capabilities.push(capability);
        Ok(self)
    }

    /// Declare a capability from its parts.
    pub fn declare_capability(
        self,
        id: CapabilityId,
        kind: CapabilityKind,
        description: impl Into<String>,
    ) -> PluginResult<Self> {
        let descriptor = CapabilityDescriptor::new(id, kind, description)?;
        self.declare(descriptor)
    }

    /// Declare that `dependency` must be active first. Repeats are ignored.
    #[must_use]
    pub fn depends_on(mut self, dependency: ModuleId) -> Self {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
        self
    }

    /// Declare a lifecycle point this module observes.
    #[must_use]
    pub fn observes(mut self, hook: HookPoint) -> Self {
        if !self.hooks.contains(&hook) {
            self.hooks.push(hook);
        }
        self
    }

    /// Declare several observed lifecycle points.
    #[must_use]
    pub fn observes_all<I>(mut self, hooks: I) -> Self
    where
        I: IntoIterator<Item = HookPoint>,
    {
        for hook in hooks {
            self = self.observes(hook);
        }
        self
    }

    /// Set the provenance stamp.
    #[must_use]
    pub fn with_origin(mut self, origin: ModuleOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Declare how this module's capability set relates to its manifest.
    ///
    /// The one place static vs dynamic capability ownership is declared. Only
    /// a module with genuine dynamic ownership (an MCP server discovering tools
    /// at runtime) should use [`CapabilityOwnership::Dynamic`].
    #[must_use]
    pub fn with_capability_ownership(mut self, capability_ownership: CapabilityOwnership) -> Self {
        self.capability_ownership = capability_ownership;
        self
    }

    /// Set the short display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Builder-style annotation.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Declare an alternative lookup key. Empty strings and duplicates (including
    /// the module id itself) are ignored.
    #[must_use]
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        let alias = alias.into();
        if alias.is_empty() || alias == self.id.as_str() || self.aliases.iter().any(|a| a == &alias)
        {
            return self;
        }
        self.aliases.push(alias);
        self
    }

    /// Declare several alternative lookup keys.
    #[must_use]
    pub fn with_aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for alias in aliases {
            self = self.with_alias(alias);
        }
        self
    }

    /// Module id plus declared aliases, id first, duplicates collapsed.
    pub fn lookup_keys(&self) -> Vec<&str> {
        let mut out = vec![self.id.as_str()];
        for alias in &self.aliases {
            if !out.contains(&alias.as_str()) {
                out.push(alias.as_str());
            }
        }
        out
    }

    /// Whether `key` is this module's id or one of its aliases.
    pub fn matches_lookup(&self, key: &str) -> bool {
        self.id.as_str() == key || self.aliases.iter().any(|a| a == key)
    }

    /// The declaration for `id`, if this module provides it.
    pub fn capability(&self, id: &CapabilityId) -> Option<&CapabilityDescriptor> {
        self.capabilities.iter().find(|c| &c.id == id)
    }

    /// Declared capabilities of a given kind.
    pub fn capabilities_of_kind(
        &self,
        kind: CapabilityKind,
    ) -> impl Iterator<Item = &CapabilityDescriptor> {
        self.capabilities.iter().filter(move |c| c.kind == kind)
    }

    /// Whether this module declares that it observes `hook`.
    pub fn observes_hook(&self, hook: HookPoint) -> bool {
        self.hooks.contains(&hook)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ModuleManifest {
        ModuleManifest::system("builtin.calculator", "1.0.0", "Arithmetic evaluation")
            .declare_capability(
                CapabilityId::new("tool.calculator").unwrap(),
                CapabilityKind::Tool,
                "Evaluate an arithmetic expression",
            )
            .unwrap()
    }

    #[test]
    fn a_manifest_indexes_its_own_declarations() {
        let m = manifest();
        let id = CapabilityId::new("tool.calculator").unwrap();
        assert!(m.capability(&id).is_some());
        assert!(m
            .capability(&CapabilityId::new("tool.absent").unwrap())
            .is_none());
        assert_eq!(m.capabilities_of_kind(CapabilityKind::Tool).count(), 1);
        assert_eq!(m.capabilities_of_kind(CapabilityKind::Provider).count(), 0);
    }

    #[test]
    fn declaring_the_same_capability_twice_is_rejected() {
        let err = manifest()
            .declare_capability(
                CapabilityId::new("tool.calculator").unwrap(),
                CapabilityKind::Tool,
                "again",
            )
            .unwrap_err();
        assert!(matches!(err, PluginError::DuplicateCapability { .. }));
    }

    #[test]
    fn a_mislabelled_capability_is_rejected_at_declaration() {
        let err = ModuleManifest::system("p", "1", "d")
            .declare_capability(
                CapabilityId::new("tool.x").unwrap(),
                CapabilityKind::Provider,
                "mislabelled",
            )
            .unwrap_err();
        assert!(matches!(err, PluginError::KindMismatch { .. }));
    }

    #[test]
    fn repeated_dependencies_collapse() {
        let dep = ModuleId::new("builtin.other").unwrap();
        let m = manifest().depends_on(dep.clone()).depends_on(dep.clone());
        assert_eq!(m.dependencies, vec![dep]);
    }

    #[test]
    fn round_trips_through_json() {
        let m = manifest()
            .depends_on(ModuleId::new("builtin.other").unwrap())
            .with_metadata("author", "apeireth")
            .with_alias("@calc")
            .observes(HookPoint::TurnStart)
            .with_origin(ModuleOrigin::Installed);
        let back: ModuleManifest =
            serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn aliases_dedup_and_lookup_keys_include_id() {
        let m = manifest()
            .with_alias("@calc")
            .with_alias("@calc")
            .with_alias("builtin.calculator")
            .with_alias("");
        assert_eq!(m.aliases, vec!["@calc".to_string()]);
        assert_eq!(m.lookup_keys(), ["builtin.calculator", "@calc"]);
        assert!(m.matches_lookup("builtin.calculator"));
        assert!(m.matches_lookup("@calc"));
        assert!(!m.matches_lookup("@other"));
    }

    #[test]
    fn json_without_the_newer_fields_deserializes_with_defaults() {
        let json = r#"{
            "id": "builtin.calculator",
            "version": "1.0.0",
            "description": "Arithmetic evaluation",
            "capabilities": [],
            "dependencies": [],
            "metadata": {}
        }"#;
        let m: ModuleManifest = serde_json::from_str(json).unwrap();
        assert!(m.aliases.is_empty());
        assert!(m.hooks.is_empty());
        assert_eq!(m.origin, ModuleOrigin::System);
        assert_eq!(m.lookup_keys(), ["builtin.calculator"]);
    }

    #[test]
    fn a_static_manifest_defaults_to_system_origin_and_a_display_name() {
        let m = ModuleManifest::system("cognitive.organs", "1.0.0", "Organ cognition");
        assert_eq!(m.origin, ModuleOrigin::System);
        assert_eq!(m.name, "Organ cognition");
        assert!(m.observes_hook(HookPoint::AfterTurn) == false);
        assert!(m
            .observes(HookPoint::AfterTurn)
            .observes_hook(HookPoint::AfterTurn));
    }

    #[test]
    #[should_panic(expected = "static module id")]
    fn a_malformed_static_id_panics_with_the_offending_string() {
        let _ = ModuleManifest::system("Builtin.Bad Id", "1.0.0", "never registered");
    }
}
