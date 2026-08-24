//! Provider-local model identity: canonical id ↔ vendor wire name.
//!
//! A canonical [`ModelId`] is stable, normalized, and lowercase (the core id
//! grammar requires it). A vendor's wire model name is whatever the vendor's
//! API expects in the request body — often mixed-case (`MiniMax-M3`,
//! `claude-sonnet-4-5`). These are two different things and must not be
//! conflated:
//!
//! ```text
//!   canonical model id (routing, supports_model, logs)
//!        ↓  provider-local mapping (this module)
//!   vendor wire model (HTTP request body)
//! ```
//!
//! [`ProviderModel`] pairs a [`ModelDescriptor`] (the canonical identity a
//! runtime routes on) with the exact `wire_name` the provider sends to the
//! vendor. This keeps the mapping **inside the provider** — the runtime, router,
//! gateway, and CLI never learn a wire name, and [`ModelDescriptor::display_name`]
//! stays purely presentational.
//!
//! This is deliberately a small struct, not a model subsystem: two providers
//! each hold a `Vec<ProviderModel>`. See ARCHITECTURE.md §5-7.

use apeireth_core::kernel::{CapabilityId, ModelId};
use apeireth_protocol::canonical::ModelDescriptor;

/// A model a provider can serve, with its canonical identity and vendor wire
/// name kept explicitly separate.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderModel {
    /// The canonical descriptor the runtime routes on.
    pub descriptor: ModelDescriptor,
    /// The exact model string sent in the vendor HTTP request body.
    pub wire_name: String,
}

impl ProviderModel {
    /// Build a model from a configured id and its features.
    ///
    /// `configured_id` is lower-cased into the canonical [`ModelId`]; the
    /// original spelling becomes `wire_name` (and the descriptor's
    /// `display_name` when it differs from the canonical id). A provider whose
    /// canonical and wire spellings already match passes the same string for
    /// both.
    pub fn from_configured(
        configured_id: String,
        provider: &CapabilityId,
        features: impl IntoIterator<Item = apeireth_protocol::canonical::ModelFeature>,
    ) -> Result<Self, apeireth_plugin::PluginError> {
        // The canonical id is lower-cased and any character the core id grammar
        // forbids (only `a-z 0-9 . - _` are allowed) is folded to `-`. So a
        // vendor model like `Qwen/Qwen3-32B` becomes the canonical id
        // `qwen-qwen3-32b` while its wire name stays `Qwen/Qwen3-32B`. This keeps
        // routing identity stable and lowercase without losing the wire spelling.
        let canonical: String = configured_id
            .to_ascii_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let mut descriptor =
            ModelDescriptor::new(ModelId::new(canonical.clone())?, provider.clone());
        for feature in features {
            descriptor = descriptor.with_feature(feature);
        }
        // display_name is presentational; carry the configured spelling when
        // it differs from the canonical id in any way — case or characters
        // folded to `-`. When the configured spelling already equals the
        // canonical id (lowercase, no folded characters), display_name stays
        // None so there is no redundant alias.
        if configured_id != canonical {
            descriptor.display_name = Some(configured_id.clone());
        }
        Ok(Self {
            descriptor,
            wire_name: configured_id,
        })
    }

    /// The canonical id (routing identity).
    pub fn canonical_id(&self) -> &ModelId {
        &self.descriptor.id
    }

    /// The vendor wire name (HTTP body identity).
    pub fn wire_name(&self) -> &str {
        &self.wire_name
    }
}

/// A small lookup over a provider's models.
///
/// Resolves a request's model string — which may arrive as either the canonical
/// id (`minimax-m3`) or the vendor spelling (`MiniMax-M3`) — to the
/// [`ProviderModel`] that serves it, so the provider can map back to the wire
/// name. Returns `None` when no registered model matches; the provider then
/// surfaces that as a routing/unsupported error rather than guessing.
impl ProviderModel {
    /// Whether `requested` names this model, by canonical id or vendor spelling.
    pub fn matches(&self, requested: &str) -> bool {
        self.descriptor.id.as_str() == requested.to_ascii_lowercase()
            || self.wire_name == requested
            || self.descriptor.display_name.as_deref() == Some(requested)
    }
}

/// Find the model serving `requested` in a provider's list, by canonical id or
/// vendor spelling.
pub fn find_model<'a>(models: &'a [ProviderModel], requested: &str) -> Option<&'a ProviderModel> {
    models.iter().find(|m| m.matches(requested))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ModelFeature;

    fn provider() -> CapabilityId {
        CapabilityId::new("provider.minimax").unwrap()
    }

    #[test]
    fn separates_canonical_id_from_vendor_wire_name() {
        let m = ProviderModel::from_configured("MiniMax-M3".into(), &provider(), []).unwrap();
        assert_eq!(m.canonical_id().as_str(), "minimax-m3");
        assert_eq!(m.wire_name(), "MiniMax-M3");
        assert_eq!(m.descriptor.display_name.as_deref(), Some("MiniMax-M3"));
    }

    #[test]
    fn canonical_and_wire_match_when_already_lowercase() {
        let m =
            ProviderModel::from_configured("claude-sonnet-4-5".into(), &provider(), []).unwrap();
        assert_eq!(m.canonical_id().as_str(), "claude-sonnet-4-5");
        assert_eq!(m.wire_name(), "claude-sonnet-4-5");
        assert!(m.descriptor.display_name.is_none());
    }

    #[test]
    fn matches_either_canonical_or_vendor_spelling() {
        let m = ProviderModel::from_configured(
            "MiniMax-M3".into(),
            &provider(),
            [ModelFeature::SystemPrompt],
        )
        .unwrap();
        assert!(m.matches("minimax-m3"));
        assert!(m.matches("MiniMax-M3"));
        assert!(!m.matches("gpt-4o"));
    }

    #[test]
    fn find_model_resolves_a_request_to_the_wire_name() {
        let models = vec![
            ProviderModel::from_configured("MiniMax-M3".into(), &provider(), []).unwrap(),
            ProviderModel::from_configured("MiniMax-M3-thinking".into(), &provider(), []).unwrap(),
        ];
        // A canonical-id request maps back to the vendor wire name.
        let resolved = find_model(&models, "minimax-m3").unwrap();
        assert_eq!(resolved.wire_name(), "MiniMax-M3");
        // A vendor-spelling request resolves to the same model.
        assert_eq!(
            find_model(&models, "MiniMax-M3-thinking")
                .unwrap()
                .wire_name(),
            "MiniMax-M3-thinking"
        );
        // An unknown model resolves to nothing.
        assert!(find_model(&models, "gpt-4o").is_none());
    }

    #[test]
    fn features_are_carried_onto_the_descriptor() {
        let m = ProviderModel::from_configured(
            "MiniMax-M3".into(),
            &provider(),
            [ModelFeature::SystemPrompt],
        )
        .unwrap();
        assert!(m.descriptor.supports(ModelFeature::SystemPrompt));
        assert!(!m.descriptor.supports(ModelFeature::Streaming));
    }
}
