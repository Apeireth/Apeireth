//! What a plugin declares about itself before it runs.
//!
//! The manifest is deliberately inert data. The manager reads it to detect
//! duplicate ids, duplicate capabilities, missing dependencies, and dependency
//! cycles *before* calling anyone's `initialize`. A system that discovers a
//! conflict halfway through boot has already run half its plugins' start-up code
//! and has no clean way back.

use apeireth_core::kernel::{CapabilityId, Metadata, PluginId};
use serde::{Deserialize, Serialize};

use crate::capability::{CapabilityDescriptor, CapabilityKind};
use crate::error::{PluginError, PluginResult};

/// A plugin's self-declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Stable identity, e.g. `builtin.calculator`.
    pub id: PluginId,
    /// Plugin version. Free-form; not interpreted by the manager.
    pub version: String,
    /// What this plugin is for, in one line.
    pub description: String,
    /// Capabilities this plugin provides.
    pub capabilities: Vec<CapabilityDescriptor>,
    /// Plugins that must be active before this one starts.
    ///
    /// Used only for ordering. A plugin does not gain access to its
    /// dependencies' internals by naming them; it reaches them, like everyone
    /// else, through the capability registry.
    pub dependencies: Vec<PluginId>,
    /// Additional annotations.
    pub metadata: Metadata,
    /// Alternative lookup keys for this plugin id.
    ///
    /// Recovered from legacy agent alias maps. These are **not** a second
    /// identity: [`PluginId`] remains unique, and [`crate::PluginRegistry`]
    /// still keys on it. Aliases exist so a caller can resolve `@coder` to
    /// `builtin.coder` through [`crate::alias::AliasIndex`] without inventing
    /// a second registry. Empty by default; omitted JSON deserializes as empty.
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl PluginManifest {
    /// Begin a manifest.
    pub fn new(id: PluginId, version: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            version: version.into(),
            description: description.into(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            metadata: Metadata::new(),
            aliases: Vec::new(),
        }
    }

    /// Declare a capability.
    ///
    /// Rejects a second declaration of the same id by the same plugin; that is
    /// always a mistake, and catching it here keeps the registry's duplicate
    /// check about genuine cross-plugin conflicts.
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
    pub fn depends_on(mut self, dependency: PluginId) -> Self {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
        self
    }

    /// Builder-style annotation.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Declare an alternative lookup key. Empty strings and duplicates (including
    /// the plugin id itself) are ignored.
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

    /// Plugin id plus declared aliases, id first, duplicates collapsed.
    pub fn lookup_keys(&self) -> Vec<&str> {
        let mut out = vec![self.id.as_str()];
        for alias in &self.aliases {
            if !out.contains(&alias.as_str()) {
                out.push(alias.as_str());
            }
        }
        out
    }

    /// Whether `key` is this plugin's id or one of its aliases.
    pub fn matches_lookup(&self, key: &str) -> bool {
        self.id.as_str() == key || self.aliases.iter().any(|a| a == key)
    }

    /// The declaration for `id`, if this plugin provides it.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest::new(
            PluginId::new("builtin.calculator").unwrap(),
            "1.0.0",
            "Arithmetic evaluation",
        )
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
        let err = PluginManifest::new(PluginId::new("p").unwrap(), "1", "d")
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
        let dep = PluginId::new("builtin.other").unwrap();
        let m = manifest().depends_on(dep.clone()).depends_on(dep.clone());
        assert_eq!(m.dependencies, vec![dep]);
    }

    #[test]
    fn round_trips_through_json() {
        let m = manifest()
            .depends_on(PluginId::new("builtin.other").unwrap())
            .with_metadata("author", "apeireth")
            .with_alias("@calc");
        let back: PluginManifest =
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
    fn json_without_aliases_deserializes_empty() {
        let json = r#"{
            "id": "builtin.calculator",
            "version": "1.0.0",
            "description": "Arithmetic evaluation",
            "capabilities": [],
            "dependencies": [],
            "metadata": {}
        }"#;
        let m: PluginManifest = serde_json::from_str(json).unwrap();
        assert!(m.aliases.is_empty());
        assert_eq!(m.lookup_keys(), ["builtin.calculator"]);
    }
}
