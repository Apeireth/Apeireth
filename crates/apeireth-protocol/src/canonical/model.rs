//! What a model is and what it can do.
//!
//! [`ModelDescriptor`] lets the runtime decide *whether a model can serve a
//! request* without asking a vendor-specific question. "Does this model support
//! tools?" must be answerable from data, not from a `match` on a provider name
//! buried in the router — that `match` is how provider knowledge leaks upward
//! into orchestration.

use apeireth_core::kernel::{CapabilityId, Metadata, ModelId};
use serde::{Deserialize, Serialize};

/// A capability a model may or may not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelFeature {
    /// Accepts tool definitions and can emit tool calls.
    ToolCalls,
    /// Can stream its response incrementally.
    Streaming,
    /// Accepts image content parts.
    Vision,
    /// Honours a dedicated system instruction.
    SystemPrompt,
    /// Can be constrained to emit schema-valid JSON.
    StructuredOutput,
}

/// A model offered by a provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    /// Stable model identifier, e.g. `claude-opus-5`.
    pub id: ModelId,
    /// The provider capability that serves this model, e.g. `provider.anthropic`.
    pub provider: CapabilityId,
    /// Human-facing name, when it differs usefully from the id.
    pub display_name: Option<String>,
    /// Total context window in tokens, when known.
    pub context_window: Option<u32>,
    /// Maximum tokens the model will generate in one response, when known.
    pub max_output_tokens: Option<u32>,
    /// Features this model supports. Absence means "not supported", never
    /// "unknown"; a descriptor that cannot state a feature should omit the model.
    pub features: Vec<ModelFeature>,
    /// Provider-specific annotations.
    pub metadata: Metadata,
}

impl ModelDescriptor {
    /// A descriptor with no optional fields and no features.
    pub fn new(id: ModelId, provider: CapabilityId) -> Self {
        Self {
            id,
            provider,
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            features: Vec::new(),
            metadata: Metadata::new(),
        }
    }

    /// Builder-style feature declaration. Repeats are ignored.
    #[must_use]
    pub fn with_feature(mut self, feature: ModelFeature) -> Self {
        if !self.features.contains(&feature) {
            self.features.push(feature);
        }
        self
    }

    /// Builder-style context window.
    #[must_use]
    pub const fn with_context_window(mut self, tokens: u32) -> Self {
        self.context_window = Some(tokens);
        self
    }

    /// Builder-style output cap.
    #[must_use]
    pub const fn with_max_output_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }

    /// Whether this model supports `feature`.
    pub fn supports(&self, feature: ModelFeature) -> bool {
        self.features.contains(&feature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> ModelDescriptor {
        ModelDescriptor::new(
            ModelId::new("fake-model-1").unwrap(),
            CapabilityId::new("provider.fake").unwrap(),
        )
        .with_feature(ModelFeature::ToolCalls)
        .with_feature(ModelFeature::SystemPrompt)
        .with_context_window(200_000)
    }

    #[test]
    fn tool_support_is_answerable_from_data_alone() {
        let d = descriptor();
        assert!(d.supports(ModelFeature::ToolCalls));
        assert!(!d.supports(ModelFeature::Vision));
        assert_eq!(d.provider.kind_segment(), "provider");
    }

    #[test]
    fn repeated_feature_declarations_are_idempotent() {
        let d = descriptor().with_feature(ModelFeature::ToolCalls);
        assert_eq!(
            d.features
                .iter()
                .filter(|f| **f == ModelFeature::ToolCalls)
                .count(),
            1
        );
    }

    #[test]
    fn round_trips_through_json() {
        let d = descriptor();
        let back: ModelDescriptor =
            serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn a_malformed_provider_id_is_rejected_at_parse_time() {
        let json = r#"{
            "id": "fake-model-1",
            "provider": "Provider.Fake",
            "display_name": null,
            "context_window": null,
            "max_output_tokens": null,
            "features": [],
            "metadata": {}
        }"#;
        assert!(serde_json::from_str::<ModelDescriptor>(json).is_err());
    }
}
