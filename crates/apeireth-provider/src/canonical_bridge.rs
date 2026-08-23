//! Compatibility boundary for production providers during canonical cutover.
//!
//! This module deliberately contains no routing, retry, session, governance, or
//! tool-loop logic. It translates the legacy [`apeireth_llm_iface::LlmProvider`]
//! contract into one canonical [`apeireth_plugin::ProviderCapability`] owned by
//! one canonical plugin. The adapter can be deleted when the wrapped provider
//! implements the canonical capability contract itself.

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId};
use apeireth_llm_iface::{ChatMessage, LlmError, LlmProvider, LlmRequest};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginError, PluginManifest, PluginResult,
    ProviderCapability, ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use async_trait::async_trait;

/// A legacy provider exposed through the canonical provider capability trait.
pub struct LegacyLlmCapability {
    id: CapabilityId,
    models: Vec<ModelDescriptor>,
    provider: Arc<dyn LlmProvider>,
}

impl LegacyLlmCapability {
    fn new(
        id: CapabilityId,
        model_ids: Vec<String>,
        provider: Arc<dyn LlmProvider>,
    ) -> PluginResult<Self> {
        if model_ids.is_empty() {
            return Err(PluginError::InvalidArguments {
                capability: id,
                reason: "a compatibility provider must declare at least one model".into(),
            });
        }

        let mut models = Vec::with_capacity(model_ids.len());
        for model in model_ids {
            if models
                .iter()
                .any(|known: &ModelDescriptor| known.id.as_str() == model)
            {
                continue;
            }
            models.push(ModelDescriptor::new(ModelId::new(model)?, id.clone()));
        }

        Ok(Self {
            id,
            models,
            provider,
        })
    }

    fn adapt_request(&self, request: &NormalizedRequest) -> Result<LlmRequest, ProviderError> {
        if !request.tools.is_empty() {
            return Err(ProviderError::BadResponse {
                provider: self.id.to_string(),
                detail:
                    "legacy compatibility provider cannot transport canonical tool declarations"
                        .into(),
            });
        }

        let mut messages = Vec::with_capacity(request.messages.len());
        for message in &request.messages {
            if !message.tool_calls.is_empty() || message.role == MessageRole::Tool {
                return Err(ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail:
                        "legacy compatibility provider cannot transport canonical tool calls/results"
                            .into(),
                });
            }

            if message
                .content
                .iter()
                .any(|part| matches!(part, ContentPart::ImageUrl { .. }))
            {
                return Err(ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail: "legacy compatibility provider only supports text content".into(),
                });
            }

            let content = ContentPart::join_text(&message.content);
            messages.push(match message.role {
                MessageRole::System => ChatMessage::system(content),
                MessageRole::User => ChatMessage::user(content),
                MessageRole::Assistant => ChatMessage::assistant(content),
                MessageRole::Tool => unreachable!("tool messages were rejected above"),
            });
        }

        let mut legacy = LlmRequest::new(request.model.clone(), messages);
        if let Some(temperature) = request.temperature {
            legacy.temperature = temperature.clamp(0.0, 2.0);
        }
        if let Some(max_tokens) = request.max_tokens {
            legacy.max_tokens = max_tokens.min(32_768);
        }
        legacy.stop = request.stop.clone();
        Ok(legacy)
    }

    fn adapt_error(&self, error: LlmError) -> ProviderError {
        let provider = self.id.to_string();
        match error {
            LlmError::RateLimited { retry_after_ms, .. } => ProviderError::RateLimited {
                provider,
                retry_after_ms,
            },
            LlmError::Timeout { timeout_ms, .. } => ProviderError::Timeout {
                provider,
                timeout_ms,
            },
            LlmError::Network { detail, .. } => ProviderError::Network { provider, detail },
            LlmError::AuthFailed(detail) => ProviderError::AuthFailed { provider, detail },
            LlmError::BadResponse { detail, .. } => ProviderError::BadResponse { provider, detail },
            other => ProviderError::BadResponse {
                provider,
                detail: other.to_string(),
            },
        }
    }
}

#[async_trait]
impl ProviderCapability for LegacyLlmCapability {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        self.models.clone()
    }

    fn supports_model(&self, model: &str) -> bool {
        self.provider.supports_model(model)
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let legacy_request = self.adapt_request(request)?;
        let response = self
            .provider
            .complete(legacy_request)
            .await
            .map_err(|error| self.adapt_error(error))?;

        Ok(NormalizedResponse {
            id: format!("compat-{}", self.id),
            model: response.model,
            content: response.content,
            finish_reason: Some(NormalizedFinishReason::from_openai(&response.finish_reason)),
            usage: NormalizedUsage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

/// One canonical plugin owning one compatibility provider capability.
pub struct CompatibilityProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<LegacyLlmCapability>,
}

impl CompatibilityProviderPlugin {
    /// Wrap a legacy provider without moving routing or execution semantics out
    /// of the canonical runtime.
    pub fn new(
        plugin_id: PluginId,
        capability_id: CapabilityId,
        models: Vec<String>,
        provider: Arc<dyn LlmProvider>,
    ) -> PluginResult<Self> {
        let capability = Arc::new(LegacyLlmCapability::new(
            capability_id.clone(),
            models,
            provider,
        )?);
        let manifest = PluginManifest::new(
            plugin_id,
            env!("CARGO_PKG_VERSION"),
            "Temporary legacy LLM compatibility provider",
        )
        .declare_capability(
            capability_id,
            CapabilityKind::Provider,
            "Canonical compatibility provider",
        )?;

        Ok(Self {
            manifest,
            provider: capability,
        })
    }
}

#[async_trait]
impl Plugin for CompatibilityProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}
