//! The anthropic provider as a first-class canonical capability.
//!
//! This is the Phase 2 protocol-diversity proof: a provider whose wire shape
//! (Anthropic Messages API) differs deliberately from the OpenAI Chat
//! Completions shape Phase 1 migrated, yet which reaches the runtime through
//! the **same** `ProviderCapability` / `ProviderRouter` / `CredentialResolver`
//! plumbing. The runtime names no vendor and knows no Anthropic protocol
//! detail; every difference is contained in this module.
//!
//! # What differs from the OpenAI-compatible provider, and where it lives
//!
//! | Difference | Contained in |
//! | --- | --- |
//! | `x-api-key` + `anthropic-version` headers (not Bearer) | `complete` |
//! | `POST {base_url}/v1/messages` (not `/chat/completions`) | `complete` |
//! | system prompt → top-level `system` field (not a messages entry) | `adapt_request` |
//! | `max_tokens` required | `adapt_request` |
//! | response `content[].type=="text"` → text | `adapt_response` |
//! | `stop_reason` → `NormalizedFinishReason::from_anthropic` | `adapt_response` |
//! | `usage.input_tokens`/`output_tokens` → `NormalizedUsage` | `adapt_response` |
//!
//! Ported faithfully from the repository's existing
//! `apeireth_api::llm::providers::anthropic_compat::AnthropicCompatibleProvider`
//! (an `LlmProvider`), but **not** wrapped around it: translation, the HTTP
//! client, and credential resolution are owned here against the canonical
//! contract. The legacy provider's internal retry loop is dropped — the
//! canonical router owns cross-provider fallback, so this `complete` makes
//! exactly one HTTP attempt (§34).
//!
//! # Eager validation and the resolver slot
//!
//! Same shape as the minimax provider: `PluginManager::register` validates
//! `providers()` against the manifest before `initialize` runs, so the
//! capability is constructed at registration with a shared [`ResolverSlot`]
//! that starts empty; [`AnthropicProviderPlugin::initialize`] fills it, and
//! `complete` reads it per turn. The slot holds a resolver handle, never a
//! secret.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, PluginId};
use apeireth_plugin::{
    CapabilityKind, CredentialResolver, Plugin, PluginContext, PluginError, PluginManifest,
    PluginResult, ProviderCapability, ProviderError, Secret,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use async_trait::async_trait;

use crate::credentials::ANTHROPIC_API_KEY;
use crate::provider_model::{find_model, ProviderModel};

/// Stable capability identity for the anthropic provider.
const CAPABILITY_ID: &str = "provider.anthropic";
/// Stable plugin identity owning the anthropic capability.
const PLUGIN_ID: &str = "builtin.anthropic";
/// Default Anthropic-protocol endpoint. A vendor endpoint constant is
/// acceptable configuration; a machine-specific path is not (§19). Follows the
/// repository's existing default (minimaxi's Anthropic-compatible gateway).
pub const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com/anthropic";
/// Default model list when none is configured (repository convention).
pub const DEFAULT_MODELS: &[&str] = &[
    "MiniMax-M3",
    "claude-3-7-sonnet-20250219",
    "claude-3-7-sonnet",
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet",
    "claude-3-5-haiku-20241022",
    "claude-3-5-haiku",
    "claude-3-opus-20240229",
    "claude-3-opus",
    "claude-sonnet-4-5",
    "claude-sonnet-5",
];
/// Anthropic API version header value (repository convention).
pub const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Default per-request timeout.
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// `max_tokens` is required by the Anthropic Messages API. The canonical
/// request makes it optional, so a default is applied when absent — matching
/// the legacy `LlmRequest` default the existing provider received.
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// A handle to the credential resolver, shared between a plugin and its
/// capability across the registration→initialize timing gap. Holds a resolver
/// handle only; no secret is ever stored here.
type ResolverSlot = Arc<Mutex<Option<Arc<dyn CredentialResolver>>>>;

/// The anthropic provider as a canonical [`ProviderCapability`].
///
/// Owns its vendor transport (a `reqwest::Client` and the Anthropic Messages
/// API translation) and resolves its API key through a [`CredentialResolver`]
/// on every call.
pub struct AnthropicProviderCapability {
    id: CapabilityId,
    models: Vec<ProviderModel>,
    base_url: String,
    http: reqwest::Client,
    timeout_ms: u64,
    credential_key: String,
    resolver: ResolverSlot,
}

impl AnthropicProviderCapability {
    /// Build a capability against an explicit `base_url`, model list, and
    /// injected HTTP client (tests point this at a mock server).
    pub fn new(
        base_url: impl Into<String>,
        models: Vec<String>,
        http: reqwest::Client,
        timeout_ms: u64,
        resolver: ResolverSlot,
    ) -> PluginResult<Self> {
        let id = CapabilityId::new(CAPABILITY_ID)?;
        let models = build_models(&id, models)?;
        Ok(Self {
            id,
            models,
            base_url: base_url.into(),
            http,
            timeout_ms,
            credential_key: ANTHROPIC_API_KEY.to_string(),
            resolver,
        })
    }

    /// Resolve the API key for this turn, or fail permanently. A missing key
    /// is permanent (§40): falling back would mask a misconfiguration.
    fn resolve_key(&self) -> Result<Secret, ProviderError> {
        let resolver = {
            let guard = self.resolver.lock().expect("resolver slot lock poisoned");
            guard.clone().ok_or_else(|| ProviderError::AuthFailed {
                provider: self.id.to_string(),
                detail: format!(
                    "no credential resolver attached; cannot resolve {}",
                    self.credential_key
                ),
            })?
        };
        resolver
            .resolve(&self.credential_key)
            .ok_or_else(|| ProviderError::AuthFailed {
                provider: self.id.to_string(),
                detail: format!("missing API key for {}", self.credential_key),
            })
    }

    /// Translate a canonical request into an Anthropic Messages API body.
    ///
    /// System messages are extracted into the top-level `system` field
    /// (concatenated with `\n` when multiple) — the Anthropic protocol does not
    /// carry system as a messages entry. `max_tokens` is required by the API
    /// and defaulted when the canonical request omits it. Tools, tool calls,
    /// tool results, and image content are not transported by this provider and
    /// surface as a permanent `BadResponse` rather than being silently dropped
    /// (§10/§28/§29).
    fn adapt_request(
        &self,
        request: &NormalizedRequest,
    ) -> Result<serde_json::Value, ProviderError> {
        if !request.tools.is_empty() {
            return Err(ProviderError::BadResponse {
                provider: self.id.to_string(),
                detail: "anthropic canonical provider does not transport tool declarations".into(),
            });
        }

        // Map the canonical requested model to the vendor wire name.
        let wire_model = find_model(&self.models, &request.model)
            .map(|m| m.wire_name().to_string())
            .ok_or_else(|| ProviderError::BadResponse {
                provider: self.id.to_string(),
                detail: format!("model {} is not served by {}", request.model, self.id),
            })?;

        let mut system_parts: Vec<String> = Vec::new();
        let mut messages = Vec::with_capacity(request.messages.len());
        for message in &request.messages {
            if !message.tool_calls.is_empty()
                || message.role == apeireth_protocol::canonical::MessageRole::Tool
            {
                return Err(ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail: "anthropic canonical provider does not transport tool calls/results"
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
                    detail: "anthropic canonical provider only supports text content".into(),
                });
            }

            let text = ContentPart::join_text(&message.content);
            match message.role {
                apeireth_protocol::canonical::MessageRole::System => {
                    if !text.is_empty() {
                        system_parts.push(text);
                    }
                }
                apeireth_protocol::canonical::MessageRole::User => {
                    messages.push(serde_json::json!({ "role": "user", "content": text }));
                }
                apeireth_protocol::canonical::MessageRole::Assistant => {
                    messages.push(serde_json::json!({ "role": "assistant", "content": text }));
                }
                apeireth_protocol::canonical::MessageRole::Tool => {
                    unreachable!("tool messages rejected above")
                }
            }
        }

        if messages.is_empty() {
            // The Anthropic Messages API requires at least one user/assistant
            // message; a system-only request is invalid (repository behavior).
            return Err(ProviderError::BadResponse {
                provider: self.id.to_string(),
                detail: "anthropic messages api requires at least one user/assistant message"
                    .into(),
            });
        }

        let max_tokens = request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS).min(32_768);
        let mut body = serde_json::json!({
            "model": wire_model,
            "max_tokens": max_tokens,
            "messages": messages,
        });
        if !system_parts.is_empty() {
            body["system"] = serde_json::json!(system_parts.join("\n"));
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = serde_json::json!(temperature.clamp(0.0, 2.0));
        }
        if !request.stop.is_empty() {
            body["stop_sequences"] = serde_json::json!(request.stop);
        }
        Ok(body)
    }

    /// Classify a vendor HTTP outcome into a canonical [`ProviderError`].
    fn classify_status(&self, status: reqwest::StatusCode, body_text: String) -> ProviderError {
        let provider = self.id.to_string();
        match status.as_u16() {
            401 | 403 => ProviderError::AuthFailed {
                provider,
                detail: format!("vendor returned {status}: {body_text}"),
            },
            429 => ProviderError::RateLimited {
                provider,
                retry_after_ms: 1_000,
            },
            408 | 504 => ProviderError::Timeout {
                provider,
                timeout_ms: self.timeout_ms,
            },
            _ if status.is_server_error() => ProviderError::Refused {
                provider,
                detail: format!("vendor returned {status}: {body_text}"),
            },
            _ => ProviderError::BadResponse {
                provider,
                detail: format!("vendor returned {status}: {body_text}"),
            },
        }
    }

    /// Parse an Anthropic Messages API response into a canonical response.
    ///
    /// Extracts the first `content` block whose `type == "text"` (the
    /// repository's existing behavior), maps `stop_reason` through
    /// [`NormalizedFinishReason::from_anthropic`], and reads
    /// `usage.input_tokens`/`output_tokens`. No vendor field leaks upward.
    fn adapt_response(
        &self,
        body: serde_json::Value,
        request_model: &str,
    ) -> Result<NormalizedResponse, ProviderError> {
        let provider = self.id.to_string();

        let content = body
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or_else(|| ProviderError::BadResponse {
                provider: provider.clone(),
                detail: "anthropic response has no content array".into(),
            })?;
        let text = content
            .iter()
            .find(|block| {
                block
                    .get("type")
                    .and_then(|t| t.as_str())
                    .is_some_and(|t| t == "text")
            })
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let stop_reason = body
            .get("stop_reason")
            .and_then(|s| s.as_str())
            .unwrap_or("end_turn");
        let finish_reason = NormalizedFinishReason::from_anthropic(stop_reason);

        let usage = body
            .get("usage")
            .map(|u| NormalizedUsage {
                prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                    as u32,
                total_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32
                    + u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(request_model)
            .to_string();

        Ok(NormalizedResponse {
            id: body
                .get("id")
                .and_then(|i| i.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| format!("anthropic-{}", self.id)),
            model,
            content: text,
            finish_reason: Some(finish_reason),
            usage,
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

#[async_trait]
impl ProviderCapability for AnthropicProviderCapability {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        self.models.iter().map(|m| m.descriptor.clone()).collect()
    }

    /// Match by canonical id or vendor spelling, via [`ProviderModel::matches`].
    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m.matches(model))
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        // One HTTP attempt. The router, not this provider, owns fallback.
        let key = self.resolve_key()?;
        let body = self.adapt_request(request)?;
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        // Anthropic auth: x-api-key + anthropic-version. Not Bearer.
        let send_result = self
            .http
            .post(&url)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .header("x-api-key", key.expose())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await;

        let response = match send_result {
            Ok(resp) => resp,
            Err(err) if err.is_timeout() => {
                return Err(ProviderError::Timeout {
                    provider: self.id.to_string(),
                    timeout_ms: self.timeout_ms,
                });
            }
            Err(err) => {
                return Err(ProviderError::Network {
                    provider: self.id.to_string(),
                    detail: err.to_string(),
                });
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(self.classify_status(status, body_text));
        }

        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail: format!("response json parse: {e}"),
                })?;

        self.adapt_response(body, &request.model)
    }
}

impl std::fmt::Debug for AnthropicProviderCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicProviderCapability")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("models", &self.models.len())
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}

/// One canonical plugin owning the anthropic provider capability.
///
/// Constructible with config alone so the capability it returns from
/// `providers()` exists at registration time and passes eager manifest
/// validation. The credential resolver is captured later in `initialize`.
pub struct AnthropicProviderPlugin {
    manifest: PluginManifest,
    capability: Arc<AnthropicProviderCapability>,
    resolver: ResolverSlot,
}

impl AnthropicProviderPlugin {
    /// Build the plugin with an explicit base URL, model list, and HTTP client.
    pub fn new(
        base_url: impl Into<String>,
        models: Vec<String>,
        http: reqwest::Client,
        timeout_ms: u64,
    ) -> PluginResult<Self> {
        let resolver: ResolverSlot = Arc::new(Mutex::new(None));
        let capability = Arc::new(AnthropicProviderCapability::new(
            base_url,
            models,
            http,
            timeout_ms,
            Arc::clone(&resolver),
        )?);
        let manifest = PluginManifest::new(
            PluginId::new(PLUGIN_ID)?,
            env!("CARGO_PKG_VERSION"),
            "Anthropic (Messages API) provider, canonical capability",
        )
        .declare_capability(
            CapabilityId::new(CAPABILITY_ID)?,
            CapabilityKind::Provider,
            "Anthropic Messages API completions",
        )?;
        Ok(Self {
            manifest,
            capability,
            resolver,
        })
    }

    /// Build the plugin from environment configuration, with safe vendor
    /// defaults for non-secret fields. Follows the repository's existing
    /// convention: `APEIRETH_ANTHROPIC_URL`, `APEIRETH_ANTHROPIC_MODELS`. The
    /// API key is **not** read here — it is resolved per-turn through the
    /// resolver from `APEIRETH_ANTHROPIC_KEY`.
    pub fn from_env() -> PluginResult<Self> {
        let base_url = std::env::var("APEIRETH_ANTHROPIC_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let models = std::env::var("APEIRETH_ANTHROPIC_MODELS")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                s.split(',')
                    .map(|m| m.trim().to_string())
                    .filter(|m| !m.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| DEFAULT_MODELS.iter().map(|s| s.to_string()).collect());
        let http = reqwest::Client::builder().build().map_err(|e| {
            PluginError::Core(apeireth_core::kernel::CoreError::precondition(format!(
                "reqwest client build failed: {e}"
            )))
        })?;
        Ok(Self::new(base_url, models, http, DEFAULT_TIMEOUT_MS)?)
    }

    /// The configured canonical model ids, in declaration order.
    pub fn model_ids(&self) -> Vec<String> {
        self.capability
            .models
            .iter()
            .map(|m| m.canonical_id().as_str().to_string())
            .collect()
    }

    /// The configured base URL (non-secret configuration).
    pub fn base_url(&self) -> &str {
        &self.capability.base_url
    }

    /// Attach a credential resolver without booting a full runtime (tests).
    #[doc(hidden)]
    pub fn attach_resolver_for_test(&self, resolver: Arc<dyn CredentialResolver>) {
        let mut slot = self.resolver.lock().expect("resolver slot lock poisoned");
        *slot = Some(resolver);
    }

    /// The canonical capability this plugin owns, for direct testing.
    #[doc(hidden)]
    pub fn provider_for_test(&self) -> Arc<AnthropicProviderCapability> {
        Arc::clone(&self.capability)
    }
}

#[async_trait]
impl Plugin for AnthropicProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        let mut slot = self.resolver.lock().expect("resolver slot lock poisoned");
        *slot = Some(Arc::clone(&ctx.credentials));
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        let mut slot = self.resolver.lock().expect("resolver slot lock poisoned");
        *slot = None;
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.capability) as Arc<dyn ProviderCapability>]
    }
}

impl std::fmt::Debug for AnthropicProviderPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicProviderPlugin")
            .field("manifest", &self.manifest.id)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

/// Build the provider model list from configured ids, de-duplicating by
/// canonical id. Features advertised are only those the implementation
/// supports: system messages (SystemPrompt). This provider sends no `stream`
/// and rejects tools/images/tool-results, so it does not advertise Streaming,
/// ToolCalls, or Vision (§9/§30).
fn build_models(id: &CapabilityId, model_ids: Vec<String>) -> PluginResult<Vec<ProviderModel>> {
    if model_ids.is_empty() {
        return Err(PluginError::InvalidArguments {
            capability: id.clone(),
            reason: "anthropic provider must declare at least one model".into(),
        });
    }
    let mut models = Vec::with_capacity(model_ids.len());
    for model in model_ids {
        let canonical = model.to_ascii_lowercase();
        if models
            .iter()
            .any(|known: &ProviderModel| known.canonical_id().as_str() == canonical)
        {
            continue;
        }
        models.push(ProviderModel::from_configured(
            model,
            id,
            [ModelFeature::SystemPrompt],
        )?);
    }
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::{MessageRole, NormalizedMessage};

    fn empty_resolver_slot() -> ResolverSlot {
        Arc::new(Mutex::new(None))
    }

    fn http() -> reqwest::Client {
        reqwest::Client::builder().build().expect("client builds")
    }

    fn capability(resolver: ResolverSlot) -> AnthropicProviderCapability {
        AnthropicProviderCapability::new(
            "https://api.minimaxi.com/anthropic",
            vec!["MiniMax-M3".into()],
            http(),
            DEFAULT_TIMEOUT_MS,
            resolver,
        )
        .expect("capability builds")
    }

    fn request() -> NormalizedRequest {
        NormalizedRequest::new(
            "MiniMax-M3",
            vec![
                NormalizedMessage::system("be brief"),
                NormalizedMessage::user("hi"),
            ],
        )
    }

    #[test]
    fn builds_with_stable_ids_and_truthful_features() {
        let cap = capability(empty_resolver_slot());
        assert_eq!(cap.id().as_str(), "provider.anthropic");
        assert_eq!(cap.models().len(), 1);
        assert_eq!(cap.models()[0].id.as_str(), "minimax-m3");
        assert_eq!(cap.models()[0].display_name.as_deref(), Some("MiniMax-M3"));
        assert!(cap.supports_model("MiniMax-M3"));
        assert!(cap.supports_model("minimax-m3"));
        assert!(!cap.supports_model("claude-sonnet-4-5"));
        // Truthful features: only SystemPrompt.
        assert!(cap.models()[0].supports(ModelFeature::SystemPrompt));
        assert!(!cap.models()[0].supports(ModelFeature::Streaming));
        assert!(!cap.models()[0].supports(ModelFeature::ToolCalls));
        assert!(!cap.models()[0].supports(ModelFeature::Vision));
    }

    #[test]
    fn adapt_request_extracts_system_and_keeps_user_assistant() {
        let cap = capability(empty_resolver_slot());
        let body = cap.adapt_request(&request()).expect("adapts");
        assert_eq!(body["system"], "be brief");
        assert_eq!(body["model"], "MiniMax-M3", "vendor wire name");
        assert_eq!(body["max_tokens"], 1024, "required, defaulted");
        let messages = body["messages"].as_array().expect("array");
        assert_eq!(messages.len(), 1, "only the user message remains");
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "hi");
        // No stream field (repository behavior).
        assert!(body.get("stream").is_none());
    }

    #[test]
    fn adapt_request_concatenates_multiple_system_messages() {
        let cap = capability(empty_resolver_slot());
        let req = NormalizedRequest::new(
            "MiniMax-M3",
            vec![
                NormalizedMessage::system("rule one"),
                NormalizedMessage::system("rule two"),
                NormalizedMessage::user("hi"),
            ],
        );
        let body = cap.adapt_request(&req).expect("adapts");
        assert_eq!(body["system"], "rule one\nrule two");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn adapt_request_rejects_system_only_request() {
        let cap = capability(empty_resolver_slot());
        let req = NormalizedRequest::new("MiniMax-M3", vec![NormalizedMessage::system("only")]);
        let err = cap.adapt_request(&req).unwrap_err();
        assert!(matches!(err, ProviderError::BadResponse { .. }));
    }

    #[test]
    fn adapt_request_rejects_tools_and_images() {
        let cap = capability(empty_resolver_slot());
        let mut req = request();
        req.tools
            .push(apeireth_protocol::canonical::NormalizedTool::new("t"));
        let err = cap.adapt_request(&req).unwrap_err();
        assert!(matches!(err, ProviderError::BadResponse { .. }));
    }

    #[test]
    fn adapt_request_maps_canonical_id_to_wire_name() {
        let cap = capability(empty_resolver_slot());
        let req = NormalizedRequest::new("minimax-m3", vec![NormalizedMessage::user("hi")]);
        let body = cap.adapt_request(&req).expect("adapts");
        assert_eq!(body["model"], "MiniMax-M3", "wire name, not canonical id");
    }

    #[test]
    fn adapt_response_maps_text_stop_reason_and_usage() {
        let cap = capability(empty_resolver_slot());
        let body = serde_json::json!({
            "id": "msg_x",
            "model": "MiniMax-M3",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "hello back"}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let resp = cap.adapt_response(body, "MiniMax-M3").expect("adapts");
        assert_eq!(resp.content, "hello back");
        assert_eq!(resp.id, "msg_x");
        assert_eq!(resp.model, "MiniMax-M3");
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn adapt_response_maps_each_stop_reason() {
        let cap = capability(empty_resolver_slot());
        for (wire, expected) in [
            ("end_turn", NormalizedFinishReason::Stop),
            ("max_tokens", NormalizedFinishReason::Length),
            ("stop_sequence", NormalizedFinishReason::StopSequence),
            ("tool_use", NormalizedFinishReason::ToolCalls),
            ("unknown", NormalizedFinishReason::Other),
        ] {
            let body = serde_json::json!({
                "stop_reason": wire,
                "content": [{"type": "text", "text": "x"}],
                "usage": {"input_tokens": 1, "output_tokens": 1}
            });
            let resp = cap.adapt_response(body, "m").expect("adapts");
            assert_eq!(resp.finish_reason, Some(expected), "{wire}");
        }
    }

    #[test]
    fn adapt_response_takes_first_text_block_and_skips_others() {
        let cap = capability(empty_resolver_slot());
        let body = serde_json::json!({
            "content": [
                {"type": "tool_use", "id": "t1", "name": "n", "input": {}},
                {"type": "text", "text": "the answer"}
            ],
            "usage": {"input_tokens": 0, "output_tokens": 0}
        });
        let resp = cap.adapt_response(body, "m").expect("adapts");
        assert_eq!(resp.content, "the answer");
    }

    #[test]
    fn missing_resolver_fails_permanently_without_network() {
        let cap = capability(empty_resolver_slot());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(cap.complete(&request()))
            .expect_err("missing resolver must fail");
        assert!(matches!(err, ProviderError::AuthFailed { .. }));
        assert!(!err.is_retryable());
    }

    #[test]
    fn classify_status_maps_each_category() {
        let cap = capability(empty_resolver_slot());
        let auth = cap.classify_status(reqwest::StatusCode::UNAUTHORIZED, "bad key".into());
        assert!(matches!(auth, ProviderError::AuthFailed { .. }) && !auth.is_retryable());

        let rate = cap.classify_status(reqwest::StatusCode::TOO_MANY_REQUESTS, "".into());
        assert!(matches!(rate, ProviderError::RateLimited { .. }) && rate.is_retryable());

        let timeout = cap.classify_status(reqwest::StatusCode::GATEWAY_TIMEOUT, "".into());
        assert!(matches!(timeout, ProviderError::Timeout { .. }) && timeout.is_retryable());

        let server = cap.classify_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "boom".into());
        assert!(matches!(server, ProviderError::Refused { .. }) && !server.is_retryable());

        let bad = cap.classify_status(reqwest::StatusCode::BAD_REQUEST, "nope".into());
        assert!(matches!(bad, ProviderError::BadResponse { .. }) && !bad.is_retryable());
    }

    #[test]
    fn debug_does_not_leak_secrets() {
        let cap = capability(empty_resolver_slot());
        let printed = format!("{cap:?}");
        assert!(printed.contains("provider.anthropic"));
        assert!(!printed.contains("sk-"));
    }
}
