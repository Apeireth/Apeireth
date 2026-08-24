//! The minimax provider as a first-class canonical capability.
//!
//! This is the first production provider to implement
//! [`ProviderCapability`](apeireth_plugin::ProviderCapability) directly, rather
//! than reaching the runtime through the temporary `LegacyLlmCapability` bridge.
//! The runtime therefore names no vendor here: it sees only `provider.minimax`
//! in its router, and this module is the one place that knows minimaxi speaks
//! the OpenAI Chat Completions protocol.
//!
//! # What moved where
//!
//! Ported from the legacy `apeireth_api::llm::providers::apeireth_api::ApeirethApiProvider`
//! (an `LlmProvider`), but not wrapped around it:
//!
//! - **Request/response translation** is owned here, against the canonical
//!   [`NormalizedRequest`]/[`NormalizedResponse`] contract. Vendor JSON never
//!   leaves this module.
//! - **The HTTP client** is owned here (`reqwest::Client`, injectable). The
//!   runtime and gateway hold no vendor HTTP.
//! - **Credentials** arrive through [`CredentialResolver`], never as a stored
//!   `String`. The provider asks for `provider.minimax.api_key` at call time.
//! - **Retry is gone.** The legacy provider retried internally; the canonical
//!   router owns cross-provider fallback, so this `complete` makes exactly one
//!   HTTP attempt and classifies the outcome. One retry owner per layer.
//!
//! # Eager validation and the resolver slot
//!
//! `PluginManager::register` calls `providers()` and validates it against the
//! manifest *before* `initialize` runs — so the capability must exist at
//! registration time. The credential resolver, by contrast, only reaches the
//! plugin through `PluginContext` during `initialize`. The bridge between the
//! two timings is a shared [`ResolverSlot`]: the plugin and the capability each
//! hold an `Arc` to it, the plugin fills it in `initialize`, and `complete`
//! reads it on every turn. The slot holds a resolver *handle*, never a secret.
//!
//! [`NormalizedRequest`]: apeireth_protocol::canonical::NormalizedRequest
//! [`NormalizedResponse`]: apeireth_protocol::canonical::NormalizedResponse
//! [`CredentialResolver`]: apeireth_plugin::CredentialResolver

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, ModelId, PluginId};
use apeireth_plugin::{
    CapabilityKind, CredentialResolver, Plugin, PluginContext, PluginError, PluginManifest,
    PluginResult, ProviderCapability, ProviderError, Secret,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, ModelFeature, NormalizedFinishReason,
    NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use async_trait::async_trait;

use crate::credentials::MINIMAX_API_KEY;

/// Stable capability identity for the minimax provider.
const CAPABILITY_ID: &str = "provider.minimax";
/// Stable plugin identity owning the minimax capability.
const PLUGIN_ID: &str = "builtin.minimax";
/// Default minimaxi OpenAI-protocol endpoint. A vendor endpoint constant is
/// acceptable configuration; a machine-specific path is not (§19).
pub const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com/v1";
/// Default model list when none is configured.
pub const DEFAULT_MODELS: &[&str] = &["MiniMax-M3", "MiniMax-M3-thinking"];
/// Default per-request timeout.
const DEFAULT_TIMEOUT_MS: u64 = 60_000;

/// A handle to the credential resolver, shared between a plugin and its
/// capability across the registration→initialize timing gap.
///
/// Starts empty (`None`) so the capability can be constructed — and pass eager
/// manifest validation — before `Plugin::initialize` has the resolver to give.
/// Holds a resolver *handle* only; no secret is ever stored here.
type ResolverSlot = Arc<Mutex<Option<Arc<dyn CredentialResolver>>>>;

/// The minimax provider as a canonical [`ProviderCapability`].
///
/// Owns its vendor transport (a `reqwest::Client` and the OpenAI Chat
/// Completions translation) and resolves its API key through a
/// [`CredentialResolver`] on every call. Construct it with
/// [`MinimaxProviderCapability::new`] or the [`MinimaxProviderPlugin`] builder.
pub struct MinimaxProviderCapability {
    id: CapabilityId,
    models: Vec<ModelDescriptor>,
    base_url: String,
    http: reqwest::Client,
    timeout_ms: u64,
    credential_key: String,
    resolver: ResolverSlot,
}

impl MinimaxProviderCapability {
    /// Build a capability against an explicit `base_url`, model list, and
    /// injected HTTP client (tests point this at a mock server).
    ///
    /// `resolver` is the shared slot the owning plugin fills in `initialize`.
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
            credential_key: MINIMAX_API_KEY.to_string(),
            resolver,
        })
    }

    /// Resolve the API key for this turn, or fail permanently.
    ///
    /// A missing key is permanent, not transient: falling back to another
    /// provider would mask a misconfiguration (§40). Surfacing it here as
    /// [`ProviderError::AuthFailed`] stops the router from cascading.
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

    /// Translate a canonical request into the OpenAI Chat Completions JSON body.
    ///
    /// Text content of each message is joined; image/tool parts are not
    /// transported by this provider (the legacy bridge rejected them too — this
    /// is a faithful port, not a regression). Tools and tool results are
    /// surfaced as a permanent `BadResponse` so a caller can see the provider
    /// cannot transport them, rather than silently dropping them.
    fn adapt_request(
        &self,
        request: &NormalizedRequest,
    ) -> Result<serde_json::Value, ProviderError> {
        if !request.tools.is_empty() {
            return Err(ProviderError::BadResponse {
                provider: self.id.to_string(),
                detail: "minimax canonical provider does not transport tool declarations".into(),
            });
        }

        let mut messages = Vec::with_capacity(request.messages.len());
        for message in &request.messages {
            if !message.tool_calls.is_empty() || message.role == MessageRole::Tool {
                return Err(ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail: "minimax canonical provider does not transport tool calls/results"
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
                    detail: "minimax canonical provider only supports text content".into(),
                });
            }

            let role = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => unreachable!("tool messages rejected above"),
            };
            messages.push(serde_json::json!({
                "role": role,
                "content": ContentPart::join_text(&message.content),
            }));
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });
        if let Some(temperature) = request.temperature {
            body["temperature"] = serde_json::json!(temperature.clamp(0.0, 2.0));
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens.min(32_768));
        }
        if !request.stop.is_empty() {
            body["stop"] = serde_json::json!(request.stop);
        }
        Ok(body)
    }

    /// Classify a vendor HTTP outcome into a canonical [`ProviderError`].
    ///
    /// Mirrors the classification the legacy `ApeirethApiProvider` already
    /// performed, ported to the canonical error enum. Transport failures are
    /// transient; auth and policy failures are permanent.
    fn classify_status(&self, status: reqwest::StatusCode, body_text: String) -> ProviderError {
        let provider = self.id.to_string();
        match status.as_u16() {
            401 | 403 => ProviderError::AuthFailed {
                provider,
                detail: format!("vendor returned {status}: {body_text}"),
            },
            429 => {
                let retry_after_ms = parse_retry_after_ms(&body_text).unwrap_or(1_000);
                ProviderError::RateLimited {
                    provider,
                    retry_after_ms,
                }
            }
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

    /// Parse an OpenAI Chat Completions response into a canonical response.
    fn adapt_response(
        &self,
        body: serde_json::Value,
        request_model: &str,
    ) -> Result<NormalizedResponse, ProviderError> {
        let provider = self.id.to_string();

        let choices = body
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| ProviderError::BadResponse {
                provider: provider.clone(),
                detail: "response has no choices array".into(),
            })?;
        let choice = choices.first().ok_or_else(|| ProviderError::BadResponse {
            provider: provider.clone(),
            detail: "response choices array is empty".into(),
        })?;

        let content = choice
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|f| f.as_str())
            .unwrap_or("stop");

        let usage = body
            .get("usage")
            .map(|u| NormalizedUsage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
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
                .unwrap_or_else(|| format!("minimax-{}", self.id)),
            model,
            content,
            finish_reason: Some(NormalizedFinishReason::from_openai(finish_reason)),
            usage,
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

#[async_trait]
impl ProviderCapability for MinimaxProviderCapability {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        self.models.clone()
    }

    /// Case-insensitive match against the canonical (lowercase) model ids.
    ///
    /// The router calls this with the request's model string, which carries the
    /// vendor's spelling (`MiniMax-M3`); the canonical id is `minimax-m3`.
    /// Matching case-insensitively lets a request for either spelling route
    /// here, while the vendor spelling is preserved on the wire by
    /// [`MinimaxProviderCapability::adapt_request`].
    fn supports_model(&self, model: &str) -> bool {
        let needle = model.to_ascii_lowercase();
        self.models
            .iter()
            .any(|m| m.id.as_str() == needle)
            // Also accept the vendor display spelling, in case a model was
            // configured with mixed case and matched against its display_name.
            || self
                .models
                .iter()
                .any(|m| m.display_name.as_deref() == Some(model))
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        // One HTTP attempt. The router, not this provider, owns fallback.
        let key = self.resolve_key()?;
        let body = self.adapt_request(request)?;
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let send_result = self
            .http
            .post(&url)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .bearer_auth(key.expose())
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
            // Avoid surfacing the Authorization header (it was never in the
            // response body); the body_text is the vendor's own error payload.
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

impl std::fmt::Debug for MinimaxProviderCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinimaxProviderCapability")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("models", &self.models.len())
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}

/// One canonical plugin owning the minimax provider capability.
///
/// Constructible with config alone — so the capability it returns from
/// `providers()` exists at registration time and passes eager manifest
/// validation. The credential resolver is captured later in `initialize` and
/// shared into the capability through the [`ResolverSlot`].
pub struct MinimaxProviderPlugin {
    manifest: PluginManifest,
    capability: Arc<MinimaxProviderCapability>,
    resolver: ResolverSlot,
}

impl MinimaxProviderPlugin {
    /// Build the plugin with an explicit base URL, model list, and HTTP client.
    ///
    /// The resolver slot is created here and shared between plugin and
    /// capability; it starts empty and is filled in `initialize`.
    pub fn new(
        base_url: impl Into<String>,
        models: Vec<String>,
        http: reqwest::Client,
        timeout_ms: u64,
    ) -> PluginResult<Self> {
        let resolver: ResolverSlot = Arc::new(Mutex::new(None));
        let capability = Arc::new(MinimaxProviderCapability::new(
            base_url,
            models,
            http,
            timeout_ms,
            Arc::clone(&resolver),
        )?);
        let manifest = PluginManifest::new(
            PluginId::new(PLUGIN_ID)?,
            env!("CARGO_PKG_VERSION"),
            "Minimax (minimaxi) provider, canonical capability",
        )
        .declare_capability(
            CapabilityId::new(CAPABILITY_ID)?,
            CapabilityKind::Provider,
            "Minimax OpenAI-compatible completions",
        )?;
        Ok(Self {
            manifest,
            capability,
            resolver,
        })
    }

    /// Build the plugin from environment configuration, with safe vendor
    /// defaults for non-secret fields.
    ///
    /// Precedence (§42): `APEIRETH_API_URL` > [`DEFAULT_BASE_URL`];
    /// `APEIRETH_API_MODELS` > [`DEFAULT_MODELS`]. The API key is **not** read
    /// here — it is resolved per-turn through the resolver.
    pub fn from_env() -> PluginResult<Self> {
        let base_url = std::env::var("APEIRETH_API_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let models = std::env::var("APEIRETH_API_MODELS")
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

    /// The configured model ids, in declaration order.
    pub fn model_ids(&self) -> Vec<String> {
        self.capability
            .models
            .iter()
            .map(|m| m.id.as_str().to_string())
            .collect()
    }

    /// The configured base URL (non-secret configuration).
    pub fn base_url(&self) -> &str {
        &self.capability.base_url
    }
}

#[async_trait]
impl Plugin for MinimaxProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        // The resolver arrives here, after registration. Fill the shared slot
        // so the capability can resolve credentials on its next turn.
        let mut slot = self.resolver.lock().expect("resolver slot lock poisoned");
        *slot = Some(Arc::clone(&ctx.credentials));
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        // Drop the resolver handle on shutdown; no resources to release beyond it.
        let mut slot = self.resolver.lock().expect("resolver slot lock poisoned");
        *slot = None;
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.capability) as Arc<dyn ProviderCapability>]
    }
}

impl std::fmt::Debug for MinimaxProviderPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinimaxProviderPlugin")
            .field("manifest", &self.manifest.id)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

/// Build the model descriptor list from configured ids, de-duplicating.
///
/// Canonical [`ModelId`]s are lowercase by contract (the core id grammar
/// requires it), but minimaxi's wire model names are mixed-case (`MiniMax-M3`).
/// Each configured id is therefore lower-cased into a stable canonical id, and
/// the original vendor spelling is kept as the descriptor's `display_name`. The
/// provider matches models case-insensitively ([`MinimaxProviderCapability::supports_model`])
/// and sends the request's model string to the vendor verbatim, so the wire
/// name is preserved.
fn build_models(id: &CapabilityId, model_ids: Vec<String>) -> PluginResult<Vec<ModelDescriptor>> {
    if model_ids.is_empty() {
        return Err(PluginError::InvalidArguments {
            capability: id.clone(),
            reason: "minimax provider must declare at least one model".into(),
        });
    }
    let mut models = Vec::with_capacity(model_ids.len());
    for model in model_ids {
        let canonical = model.to_ascii_lowercase();
        if models
            .iter()
            .any(|known: &ModelDescriptor| known.id.as_str() == canonical)
        {
            continue;
        }
        let descriptor = ModelDescriptor::new(ModelId::new(canonical.clone())?, id.clone())
            .with_feature(ModelFeature::SystemPrompt)
            .with_feature(ModelFeature::Streaming)
            .with_feature(ModelFeature::ToolCalls);
        // Preserve the vendor spelling for display; it is metadata, not identity.
        let descriptor = if model != canonical {
            ModelDescriptor {
                display_name: Some(model),
                ..descriptor
            }
        } else {
            descriptor
        };
        models.push(descriptor);
    }
    Ok(models)
}

/// Parse the vendor's `Retry-After` hint into milliseconds.
///
/// `retry-after` arrives as a header in real responses; for unit-testability of
/// the classification (which receives the body text) this helper also accepts a
/// plain seconds integer. Returns `None` when no hint is present.
fn parse_retry_after_ms(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    trimmed.parse::<u64>().ok().map(|secs| secs * 1_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_resolver_slot() -> ResolverSlot {
        Arc::new(Mutex::new(None))
    }

    fn http() -> reqwest::Client {
        reqwest::Client::builder().build().expect("client builds")
    }

    fn capability(resolver: ResolverSlot) -> MinimaxProviderCapability {
        MinimaxProviderCapability::new(
            "https://api.minimaxi.com/v1",
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
                apeireth_protocol::canonical::NormalizedMessage::system("be brief"),
                apeireth_protocol::canonical::NormalizedMessage::user("hi"),
            ],
        )
    }

    #[test]
    fn builds_with_stable_ids_and_one_model() {
        let cap = capability(empty_resolver_slot());
        assert_eq!(cap.id().as_str(), "provider.minimax");
        assert_eq!(cap.models().len(), 1);
        // Canonical id is lower-cased; the vendor spelling is display_name.
        assert_eq!(cap.models()[0].id.as_str(), "minimax-m3");
        assert_eq!(cap.models()[0].display_name.as_deref(), Some("MiniMax-M3"));
        // Matching is case-insensitive so either spelling routes here.
        assert!(cap.supports_model("MiniMax-M3"));
        assert!(cap.supports_model("minimax-m3"));
        assert!(!cap.supports_model("gpt-4o"));
    }

    #[test]
    fn empty_model_list_is_rejected() {
        let err = MinimaxProviderCapability::new(
            "https://api.minimaxi.com/v1",
            Vec::new(),
            http(),
            DEFAULT_TIMEOUT_MS,
            empty_resolver_slot(),
        )
        .unwrap_err();
        assert!(matches!(err, PluginError::InvalidArguments { .. }), "{err}");
    }

    #[test]
    fn missing_resolver_fails_permanently_without_network() {
        // No resolver attached (slot is None): must fail before any HTTP.
        let cap = capability(empty_resolver_slot());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(cap.complete(&request()))
            .expect_err("missing resolver must fail");
        assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
        assert!(!err.is_retryable(), "missing key is permanent");
    }

    #[test]
    fn missing_credential_with_resolver_fails_permanently() {
        // Resolver attached but resolves nothing (NoCredentials): same outcome.
        let slot: ResolverSlot =
            Arc::new(Mutex::new(Some(Arc::new(apeireth_plugin::NoCredentials))));
        let cap = capability(slot);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(cap.complete(&request()))
            .expect_err("missing key must fail");
        assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
    }

    #[test]
    fn adapt_request_translates_roles_and_content() {
        let cap = capability(empty_resolver_slot());
        let body = cap.adapt_request(&request()).expect("adapts");
        assert_eq!(body["model"], "MiniMax-M3");
        assert_eq!(body["stream"], false);
        let messages = body["messages"].as_array().expect("array");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "be brief");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hi");
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
    fn adapt_response_maps_content_usage_and_finish_reason() {
        let cap = capability(empty_resolver_slot());
        let body = serde_json::json!({
            "id": "chatcmpl-x",
            "model": "MiniMax-M3",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "hello back"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        });
        let resp = cap.adapt_response(body, "MiniMax-M3").expect("adapts");
        assert_eq!(resp.content, "hello back");
        assert_eq!(resp.id, "chatcmpl-x");
        assert_eq!(resp.model, "MiniMax-M3");
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn adapt_response_omits_usage_when_absent_without_fabricating() {
        let cap = capability(empty_resolver_slot());
        let body = serde_json::json!({
            "choices": [{"message": {"content": "ok"}, "finish_reason": "length"}]
        });
        let resp = cap.adapt_response(body, "MiniMax-M3").expect("adapts");
        assert_eq!(resp.usage, NormalizedUsage::default());
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Length));
    }

    #[test]
    fn classify_status_maps_each_category() {
        let cap = capability(empty_resolver_slot());
        let auth = cap.classify_status(reqwest::StatusCode::UNAUTHORIZED, "bad key".into());
        assert!(matches!(auth, ProviderError::AuthFailed { .. }) && !auth.is_retryable());

        let rate = cap.classify_status(reqwest::StatusCode::TOO_MANY_REQUESTS, "2".into());
        assert!(matches!(rate, ProviderError::RateLimited { .. }) && rate.is_retryable());

        let timeout = cap.classify_status(reqwest::StatusCode::GATEWAY_TIMEOUT, "".into());
        assert!(matches!(timeout, ProviderError::Timeout { .. }) && timeout.is_retryable());

        let server = cap.classify_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "boom".into());
        assert!(matches!(server, ProviderError::Refused { .. }) && !server.is_retryable());

        let bad = cap.classify_status(reqwest::StatusCode::BAD_REQUEST, "nope".into());
        assert!(matches!(bad, ProviderError::BadResponse { .. }) && !bad.is_retryable());
    }

    #[test]
    fn parse_retry_after_accepts_seconds() {
        assert_eq!(parse_retry_after_ms("2"), Some(2_000));
        assert_eq!(parse_retry_after_ms("not-a-number"), None);
    }

    #[test]
    fn debug_does_not_leak_secrets() {
        // The capability holds no secret, but assert Debug stays structural.
        let cap = capability(empty_resolver_slot());
        let printed = format!("{cap:?}");
        assert!(printed.contains("provider.minimax"));
        assert!(!printed.contains("sk-"));
    }
}
