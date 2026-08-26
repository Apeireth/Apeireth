//! What a capability *is*.
//!
//! A capability is a named, typed thing the runtime can use. Its identity is a
//! [`CapabilityId`], its category is a [`CapabilityKind`], and the two must agree:
//! `tool.shell` is a [`CapabilityKind::Tool`] and cannot be registered as
//! anything else.
//!
//! Requiring both a stable id and a typed kind is the point. An id alone gives
//! stringly-typed dispatch; a kind alone gives no stable name to configure or log.
//! Together they let the runtime ask "give me the tool named `tool.shell`" and be
//! told no if what is registered under that name is not a tool.

use apeireth_core::kernel::{CapabilityId, Metadata};
use serde::{Deserialize, Serialize};

use crate::error::{PluginError, PluginResult};

/// The category of a capability.
///
/// Each kind owns a reserved id prefix, which is what makes a capability id
/// self-describing: reading `provider.anthropic` in a log tells you it is a
/// provider without consulting a registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// Something a model can call, e.g. `tool.shell`, `tool.filesystem`.
    Tool,
    /// Something that can serve a completion, e.g. `provider.anthropic`.
    Provider,
    /// Durable recall, e.g. `memory.sqlite`.
    Memory,
    /// A channel to an external process or peer, e.g. `transport.mcp`.
    Transport,
    /// Something that watches the runtime, e.g. `observer.tracing`.
    Observer,
    /// Something that decides when work runs, e.g. `scheduler.cron`.
    Scheduler,
    /// A capability that does not fit the categories above.
    ///
    /// A deliberate escape hatch, not a dumping ground: reaching for this
    /// repeatedly means a kind is missing from the list.
    Extension,
}

impl CapabilityKind {
    /// The reserved id prefix for this kind.
    pub const fn id_prefix(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Provider => "provider",
            Self::Memory => "memory",
            Self::Transport => "transport",
            Self::Observer => "observer",
            Self::Scheduler => "scheduler",
            Self::Extension => "extension",
        }
    }

    /// Every kind, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Tool,
        Self::Provider,
        Self::Memory,
        Self::Transport,
        Self::Observer,
        Self::Scheduler,
        Self::Extension,
    ];

    /// The kind whose prefix matches this id, if any.
    pub fn from_id(id: &CapabilityId) -> Option<Self> {
        let prefix = id.kind_segment();
        Self::ALL.into_iter().find(|k| k.id_prefix() == prefix)
    }
}

impl std::fmt::Display for CapabilityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id_prefix())
    }
}

/// A capability a plugin declares it provides.
///
/// This is the *declaration*, not the implementation. Declarations are what a
/// manifest carries and what the registry indexes; the implementation is reached
/// through the owning plugin once it is active.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    /// Stable identity, e.g. `tool.calculator`.
    pub id: CapabilityId,
    /// Category. Must agree with `id`'s prefix.
    pub kind: CapabilityKind,
    /// What this capability does, in one line.
    pub description: String,
    /// Additional annotations.
    pub metadata: Metadata,
}

impl CapabilityDescriptor {
    /// Declare a capability, checking that the id's prefix matches the kind.
    pub fn new(
        id: CapabilityId,
        kind: CapabilityKind,
        description: impl Into<String>,
    ) -> PluginResult<Self> {
        let actual = id.kind_segment();
        if actual != kind.id_prefix() {
            return Err(PluginError::KindMismatch {
                actual: actual.to_string(),
                expected: kind.id_prefix(),
                capability: id,
            });
        }
        Ok(Self {
            id,
            kind,
            description: description.into(),
            metadata: Metadata::new(),
        })
    }

    /// Builder-style annotation.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_distinct_prefix() {
        let mut prefixes: Vec<&str> = CapabilityKind::ALL.iter().map(|k| k.id_prefix()).collect();
        let count = prefixes.len();
        prefixes.sort_unstable();
        prefixes.dedup();
        assert_eq!(prefixes.len(), count, "prefixes must be unique");
    }

    #[test]
    fn a_kind_is_recoverable_from_a_conventional_id() {
        for kind in CapabilityKind::ALL {
            let id = CapabilityId::new(format!("{}.example", kind.id_prefix())).unwrap();
            assert_eq!(CapabilityKind::from_id(&id), Some(kind));
        }
        assert_eq!(
            CapabilityKind::from_id(&CapabilityId::new("nonsense.example").unwrap()),
            None
        );
    }

    #[test]
    fn declaring_a_capability_under_the_wrong_kind_is_rejected() {
        let err = CapabilityDescriptor::new(
            CapabilityId::new("provider.anthropic").unwrap(),
            CapabilityKind::Tool,
            "mislabelled",
        )
        .unwrap_err();

        match err {
            PluginError::KindMismatch {
                actual, expected, ..
            } => {
                assert_eq!(actual, "provider");
                assert_eq!(expected, "tool");
            }
            other => panic!("expected KindMismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_matching_declaration_is_accepted() {
        let d = CapabilityDescriptor::new(
            CapabilityId::new("tool.calculator").unwrap(),
            CapabilityKind::Tool,
            "Evaluate arithmetic",
        )
        .unwrap()
        .with_metadata("risk", "low");

        assert_eq!(d.kind, CapabilityKind::Tool);
        assert_eq!(d.metadata.get("risk"), Some("low"));
    }

    #[test]
    fn round_trips_through_json() {
        let d = CapabilityDescriptor::new(
            CapabilityId::new("transport.mcp").unwrap(),
            CapabilityKind::Transport,
            "MCP stdio transport",
        )
        .unwrap();
        let back: CapabilityDescriptor =
            serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(d, back);
    }
}
