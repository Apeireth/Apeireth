//! The generic OpenAI-compatible provider as a first-class canonical capability.
//!
//! This is the third ordinary HTTP provider to converge on the canonical
//! `ProviderCapability` architecture. Unlike `provider.minimax` (a vendor) and
//! `provider.anthropic` (a different protocol), this provider is **generic**:
//! it speaks the OpenAI Chat Completions wire protocol against any configured
//! endpoint (OpenAI, a proxy, a self-hosted gateway, Ollama, Together, vLLM).
//!
//! # Identity is a protocol family, not a vendor (§8/§9)
//!
//! The stable capability id is `provider.openai-compatible` — a protocol family,
//! not `provider.openai` (which would misleadingly imply vendor == OpenAI).
//! `base_url`, model list, and credential key are configuration; the protocol
//! family and the vendor are three different concepts and are not collapsed into
//! one string.
//!
//! # Protocol reuse, not duplication (§13/§14)
//!
//! The OpenAI Chat Completions request/response/status logic is shared with
//! minimax via the [`crate::openai_chat`] helper. This provider owns only its
//! identity, plugin, credential key, model mapping, and configuration (§16).
//!
//! # Faithful to the legacy implementation
//!
//! Ported from `apeireth_api::llm::providers::openai_compat::OpenAiCompatibleProvider`
//! (an `LlmProvider`), but not wrapped around it. Bearer auth; `POST
//! {base_url}/chat/completions`; `choices[0].message.content`; `usage`;
//! `finish_reason` → [`NormalizedFinishReason::from_openai`]. The legacy
//! provider's non-streaming `complete` is already a single attempt (no internal
//! retry loop), which matches the canonical retry-ownership rule (§35).
//!
//! # Feature truthfulness (§6/§25)
//!
//! Only `SystemPrompt` is advertised. The implementation sends `stream:false`
//! and rejects tools/images/tool-results, so it does not claim `Streaming`,
//! `ToolCalls`, or `Vision`.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, PluginId};
use apeireth_plugin::{
    CapabilityKind, CredentialResolver, Plugin, PluginContext, PluginError, PluginManifest,
    PluginResult, ProviderCapability, ProviderError, Secret,
};
use apeireth_protocol::canonical::{ModelDescriptor, ModelFeature, NormalizedRequest};
use async_trait::async_trait;

use crate::credentials::OPENAI_COMPATIBLE_API_KEY;
use crate::openai_chat;
use crate::provider_model::{find_model, ProviderModel};

/// Stable capability identity for the generic OpenAI-compatible provider. A
/// protocol family, not a vendor (§8/§9).
const CAPABILITY_ID: &str = "provider.openai-compatible";
/// Stable plugin identity owning the capability.
const PLUGIN_ID: &str = "builtin.openai-compatible";
/// Default endpoint. A vendor-safe default (OpenAI's public API) is acceptable
/// configuration (§19); the provider is generic and `base_url` is overridable.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
/// Default per-request timeout.
const DEFAULT_TIMEOUT_MS: u64 = 60_000;

/// A handle to the credential resolver, shared between a plugin and its
/// capability across the registration→initialize timing gap. Holds a resolver
/// handle only; no secret is ever stored here.
type ResolverSlot = Arc<Mutex<Option<Arc<dyn CredentialResolver>>>>;

/// The generic OpenAI-compatible provider as a canonical [`ProviderCapability`].
///
/// Owns its vendor transport (a `reqwest::Client` and the OpenAI Chat
/// Completions translation via the shared [`openai_chat`] helper) and resolves
/// its API key through a [`CredentialResolver`] on every call.
pub struct OpenAiCompatibleProviderCapability {
    id: CapabilityId,
    models: Vec<ProviderModel>,
    base_url: String,
    http: reqwest::Client,
    timeout_ms: u64,
    credential_key: String,
    resolver: ResolverSlot,
}

impl OpenAiCompatibleProviderCapability {
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
            credential_key: OPENAI_COMPATIBLE_API_KEY.to_string(),
            resolver,
        })
    }

    /// Resolve the API key for this turn, or fail permanently. A missing key
    /// is permanent (§40): the legacy config made `api_key_env` mandatory, so a
    /// missing key is a misconfiguration, not an anonymous request (§19/§20).
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

    /// Translate a canonical request into the OpenAI Chat Completions body.
    /// Delegates to the shared [`openai_chat`] helper; this provider supplies
    /// the vendor-resolved wire model name and its own id for attribution.
    fn adapt_request(
        &self,
        request: &NormalizedRequest,
    ) -> Result<serde_json::Value, ProviderError> {
        let wire_model = find_model(&self.models, &request.model)
            .map(|m| m.wire_name().to_string())
            .ok_or_else(|| ProviderError::BadResponse {
                provider: self.id.to_string(),
                detail: format!("model {} is not served by {}", request.model, self.id),
            })?;
        openai_chat::build_request_body(request, &wire_model, self.id.as_str())
    }

    /// Classify a vendor HTTP outcome. Delegates to the shared [`openai_chat`]
    /// status classifier.
    fn classify_status(&self, status: reqwest::StatusCode, body_text: String) -> ProviderError {
        openai_chat::classify_status(status, body_text, self.id.as_str(), self.timeout_ms)
    }

    /// Parse an OpenAI Chat Completions response. Delegates to the shared
    /// [`openai_chat`] response parser.
    fn adapt_response(
        &self,
        body: serde_json::Value,
        request_model: &str,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, ProviderError> {
        openai_chat::parse_response(body, request_model, self.id.as_str())
    }
}

#[async_trait]
impl ProviderCapability for OpenAiCompatibleProviderCapability {
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
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, ProviderError> {
        // One HTTP attempt. The router, not this provider, owns fallback.
        let key = self.resolve_key()?;
        let body = self.adapt_request(request)?;
        let url = openai_chat::join_endpoint(&self.base_url, "chat/completions");

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

impl std::fmt::Debug for OpenAiCompatibleProviderCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatibleProviderCapability")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("models", &self.models.len())
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}

/// One canonical plugin owning the generic OpenAI-compatible capability.
///
/// Constructible with config alone so the capability it returns from
/// `providers()` exists at registration time and passes eager manifest
/// validation. The credential resolver is captured later in `initialize`.
pub struct OpenAiCompatibleProviderPlugin {
    manifest: PluginManifest,
    capability: Arc<OpenAiCompatibleProviderCapability>,
    resolver: ResolverSlot,
}

impl OpenAiCompatibleProviderPlugin {
    /// Build the plugin with an explicit base URL, model list, and HTTP client.
    pub fn new(
        base_url: impl Into<String>,
        models: Vec<String>,
        http: reqwest::Client,
        timeout_ms: u64,
    ) -> PluginResult<Self> {
        let resolver: ResolverSlot = Arc::new(Mutex::new(None));
        let capability = Arc::new(OpenAiCompatibleProviderCapability::new(
            base_url,
            models,
            http,
            timeout_ms,
            Arc::clone(&resolver),
        )?);
        let manifest = PluginManifest::new(
            PluginId::new(PLUGIN_ID)?,
            env!("CARGO_PKG_VERSION"),
            "Generic OpenAI-compatible provider, canonical capability",
        )
        .declare_capability(
            CapabilityId::new(CAPABILITY_ID)?,
            CapabilityKind::Provider,
            "OpenAI Chat Completions (generic compatible endpoint)",
        )?;
        Ok(Self {
            manifest,
            capability,
            resolver,
        })
    }

    /// Build the plugin from environment configuration, with a safe vendor
    /// default for the non-secret base URL. `APEIRETH_OPENAI_URL` /
    /// `APEIRETH_OPENAI_MODELS` follow the repository's `APEIRETH_<VENDOR>_*`
    /// naming pattern. The API key is **not** read here — it is resolved
    /// per-turn through the resolver from `OPENAI_API_KEY`.
    pub fn from_env() -> PluginResult<Self> {
        let base_url = std::env::var("APEIRETH_OPENAI_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let models = std::env::var("APEIRETH_OPENAI_MODELS")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                s.split(',')
                    .map(|m| m.trim().to_string())
                    .filter(|m| !m.is_empty())
                    .collect::<Vec<_>>()
            })
            // Generic provider: no hardcoded model default. The caller must
            // configure models; an empty list is rejected at build_models.
            .unwrap_or_default();
        let http = reqwest::Client::builder().build().map_err(|e| {
            PluginError::Core(apeireth_core::kernel::CoreError::precondition(format!(
                "reqwest client build failed: {e}"
            )))
        })?;
        Self::new(base_url, models, http, DEFAULT_TIMEOUT_MS)
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
    pub fn provider_for_test(&self) -> Arc<OpenAiCompatibleProviderCapability> {
        Arc::clone(&self.capability)
    }
}

#[async_trait]
impl Plugin for OpenAiCompatibleProviderPlugin {
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

impl std::fmt::Debug for OpenAiCompatibleProviderPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatibleProviderPlugin")
            .field("manifest", &self.manifest.id)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

/// Build the provider model list from configured ids, de-duplicating by
/// canonical id. Only `SystemPrompt` is advertised — the implementation sends
/// `stream:false` and rejects tools/images/tool-results (§6/§25).
fn build_models(id: &CapabilityId, model_ids: Vec<String>) -> PluginResult<Vec<ProviderModel>> {
    if model_ids.is_empty() {
        return Err(PluginError::InvalidArguments {
            capability: id.clone(),
            reason: "openai-compatible provider must declare at least one model".into(),
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
    use apeireth_protocol::canonical::{
        NormalizedFinishReason, NormalizedMessage, NormalizedUsage,
    };

    fn empty_resolver_slot() -> ResolverSlot {
        Arc::new(Mutex::new(None))
    }

    fn http() -> reqwest::Client {
        reqwest::Client::builder().build().expect("client builds")
    }

    fn capability(resolver: ResolverSlot) -> OpenAiCompatibleProviderCapability {
        OpenAiCompatibleProviderCapability::new(
            "https://api.openai.com/v1",
            vec!["gpt-4o-mini".into()],
            http(),
            DEFAULT_TIMEOUT_MS,
            resolver,
        )
        .expect("capability builds")
    }

    fn request() -> NormalizedRequest {
        NormalizedRequest::new(
            "gpt-4o-mini",
            vec![
                NormalizedMessage::system("be brief"),
                NormalizedMessage::user("hi"),
            ],
        )
    }

    #[test]
    fn builds_with_stable_protocol_family_id_and_truthful_features() {
        let cap = capability(empty_resolver_slot());
        // Identity is a protocol family, not a vendor.
        assert_eq!(cap.id().as_str(), "provider.openai-compatible");
        assert_eq!(cap.models().len(), 1);
        assert_eq!(cap.models()[0].id.as_str(), "gpt-4o-mini");
        assert!(cap.supports_model("gpt-4o-mini"));
        assert!(cap.supports_model("GPT-4O-Mini"), "case-insensitive");
        assert!(!cap.supports_model("minimax-m3"), "distinct from minimax");
        // Truthful features: only SystemPrompt.
        assert!(cap.models()[0].supports(ModelFeature::SystemPrompt));
        assert!(!cap.models()[0].supports(ModelFeature::Streaming));
        assert!(!cap.models()[0].supports(ModelFeature::ToolCalls));
        assert!(!cap.models()[0].supports(ModelFeature::Vision));
    }

    #[test]
    fn empty_model_list_is_rejected() {
        let err = OpenAiCompatibleProviderCapability::new(
            "https://api.openai.com/v1",
            Vec::new(),
            http(),
            DEFAULT_TIMEOUT_MS,
            empty_resolver_slot(),
        )
        .unwrap_err();
        assert!(matches!(err, PluginError::InvalidArguments { .. }), "{err}");
    }

    #[test]
    fn adapt_request_maps_canonical_id_to_wire_name() {
        let cap = capability(empty_resolver_slot());
        let req = NormalizedRequest::new("gpt-4o-mini", vec![NormalizedMessage::user("hi")]);
        let body = cap.adapt_request(&req).expect("adapts");
        assert_eq!(body["model"], "gpt-4o-mini");
        assert_eq!(body["stream"], false);
        // A canonical-id request (already lowercase) maps to the same wire name.
    }

    #[test]
    fn adapt_request_preserves_mixed_case_wire_name() {
        let cap = OpenAiCompatibleProviderCapability::new(
            "https://api.openai.com/v1",
            vec!["Qwen/Qwen3-32B".into()],
            http(),
            DEFAULT_TIMEOUT_MS,
            empty_resolver_slot(),
        )
        .expect("capability builds");
        // Canonical id is lower-cased and the forbidden `/` is folded to `-`;
        // the wire name preserves the vendor spelling verbatim.
        assert_eq!(cap.models()[0].id.as_str(), "qwen-qwen3-32b");
        assert_eq!(
            cap.models()[0].display_name.as_deref(),
            Some("Qwen/Qwen3-32B")
        );
        let req = NormalizedRequest::new("qwen-qwen3-32b", vec![NormalizedMessage::user("hi")]);
        let body = cap.adapt_request(&req).expect("adapts");
        assert_eq!(
            body["model"], "Qwen/Qwen3-32B",
            "wire name, not canonical id"
        );
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
            "model": "gpt-4o-mini",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "hello back"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        });
        let resp = cap.adapt_response(body, "gpt-4o-mini").expect("adapts");
        assert_eq!(resp.content, "hello back");
        assert_eq!(resp.id, "chatcmpl-x");
        assert_eq!(resp.model, "gpt-4o-mini");
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn adapt_response_omits_usage_when_absent() {
        let cap = capability(empty_resolver_slot());
        let body = serde_json::json!({
            "choices": [{"message": {"content": "ok"}, "finish_reason": "length"}]
        });
        let resp = cap.adapt_response(body, "m").expect("adapts");
        assert_eq!(resp.usage, NormalizedUsage::default());
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Length));
        assert_eq!(
            resp.id,
            format!("openai-{}", CAPABILITY_ID),
            "synthetic id falls back to provider-tagged"
        );
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
        let auth = cap.classify_status(reqwest::StatusCode::UNAUTHORIZED, "bad".into());
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
    fn debug_does_not_leak_secrets() {
        let cap = capability(empty_resolver_slot());
        let printed = format!("{cap:?}");
        assert!(printed.contains("provider.openai-compatible"));
        assert!(!printed.contains("sk-"));
    }
}
