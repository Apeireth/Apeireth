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
//! `SystemPrompt` + `ToolCalls` + `Streaming` are advertised: the wire is the
//! OpenAI Chat Completions protocol, which streams over SSE with
//! `stream:true` (incremental deltas through `complete_streaming`). Images are
//! still rejected, so `Vision` is not claimed.

use std::collections::BTreeMap;
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
/// Default model list when none is configured.
pub const DEFAULT_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "gpt-4",
    "gpt-3.5-turbo",
    "o1",
    "o1-mini",
    "o3-mini",
    "deepseek-chat",
    "deepseek-reasoner",
    "qwen-turbo",
    "qwen-plus",
    "qwen-max",
];
/// Default per-request timeout.
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;

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
    ///
    /// Reasoning models (deepseek-v4-flash, live-verified 2026-09-08/28) burn
    /// their whole output budget on `reasoning_content` and can finish with an
    /// EMPTY `content` when `max_tokens` is left to the vendor default (W1
    /// organ hit this: 500 always truncated, 2048 still 1/3 empty; 4096 works).
    /// The canonical chain never sets max_tokens, so default it here to give
    /// reasoning headroom instead of shipping empty responses to the UI.
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
        let mut body = openai_chat::build_request_body(request, &wire_model, self.id.as_str())?;
        if body.get("max_tokens").is_none() {
            body["max_tokens"] = serde_json::json!(4096);
        }
        Ok(body)
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

    /// Stream the completion over the vendor's SSE wire, forwarding content
    /// deltas to `on_delta` as they arrive, and reassembling the accumulated
    /// stream into the canonical full-response shape (same adapter as the
    /// non-streaming path, so tool calls / finish reason / usage normalize
    /// identically).
    async fn complete_streaming(
        &self,
        request: &NormalizedRequest,
        on_delta: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<apeireth_protocol::canonical::NormalizedResponse, ProviderError> {
        let key = self.resolve_key()?;
        let mut body = self.adapt_request(request)?;
        body["stream"] = serde_json::Value::Bool(true);
        body["stream_options"] = serde_json::json!({"include_usage": true});
        let url = openai_chat::join_endpoint(&self.base_url, "chat/completions");

        let send_result = self
            .http
            .post(&url)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .bearer_auth(key.expose())
            .json(&body)
            .send()
            .await;

        let mut response = match send_result {
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

        let mut buffer: Vec<u8> = Vec::new();
        let mut full_text = String::new();
        let mut tool_ids: BTreeMap<usize, String> = BTreeMap::new();
        let mut tool_names: BTreeMap<usize, String> = BTreeMap::new();
        let mut tool_args: BTreeMap<usize, String> = BTreeMap::new();
        let mut finish_reason: Option<String> = None;
        let mut usage: Option<serde_json::Value> = None;

        loop {
            let chunk = response
                .chunk()
                .await
                .map_err(|e| ProviderError::BadResponse {
                    provider: self.id.to_string(),
                    detail: format!("stream read: {e}"),
                })?;
            let Some(chunk) = chunk else {
                break;
            };
            buffer.extend_from_slice(&chunk);
            while let Some(end) = find_sse_frame_end(&buffer) {
                let frame: Vec<u8> = buffer.drain(..end).collect();
                for raw_line in String::from_utf8_lossy(&frame).split('\n') {
                    let line = raw_line.trim_end_matches('\r').trim();
                    let Some(payload) = line.strip_prefix("data:") else {
                        continue;
                    };
                    let payload = payload.trim();
                    if payload.is_empty() || payload == "[DONE]" {
                        continue;
                    }
                    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
                        continue;
                    };
                    if let Some(usage_value) = value.get("usage").filter(|u| !u.is_null()) {
                        usage = Some(usage_value.clone());
                    }
                    let Some(choices) = value.get("choices").and_then(|c| c.as_array()) else {
                        continue;
                    };
                    for choice in choices {
                        let Some(delta) = choice.get("delta") else {
                            continue;
                        };
                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                            if !content.is_empty() {
                                on_delta(content.to_string());
                                full_text.push_str(content);
                            }
                        }
                        if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array())
                        {
                            for call in tool_calls {
                                let index = call.get("index").and_then(|i| i.as_u64()).unwrap_or(0)
                                    as usize;
                                if let Some(id) = call.get("id").and_then(|i| i.as_str()) {
                                    tool_ids.insert(index, id.to_string());
                                }
                                let function = call.get("function");
                                if let Some(name) = function
                                    .and_then(|f| f.get("name"))
                                    .and_then(|n| n.as_str())
                                {
                                    tool_names.insert(index, name.to_string());
                                }
                                if let Some(args) = function
                                    .and_then(|f| f.get("arguments"))
                                    .and_then(|a| a.as_str())
                                {
                                    tool_args.entry(index).or_default().push_str(args);
                                }
                            }
                        }
                        if let Some(reason) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                            if !reason.is_empty() {
                                finish_reason = Some(reason.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Reassemble the accumulated stream into the canonical full-response
        // shape and run it through the same adapter as the non-streaming path.
        let mut message = serde_json::json!({ "role": "assistant", "content": full_text });
        if !tool_ids.is_empty() {
            let calls: Vec<serde_json::Value> = tool_ids
                .keys()
                .map(|index| {
                    serde_json::json!({
                        "id": tool_ids.get(index).cloned().unwrap_or_default(),
                        "type": "function",
                        "function": {
                            "name": tool_names.get(index).cloned().unwrap_or_default(),
                            "arguments": tool_args.get(index).cloned().unwrap_or_default(),
                        },
                    })
                })
                .collect();
            message["tool_calls"] = serde_json::Value::Array(calls);
        }
        let mut synthetic = serde_json::json!({
            "choices": [{
                "message": message,
                "finish_reason": finish_reason.unwrap_or_else(|| "stop".to_string()),
            }],
        });
        if let Some(usage) = usage {
            synthetic["usage"] = usage;
        }
        self.adapt_response(synthetic, &request.model)
    }
}

/// Position just past the next blank-line SSE frame separator.
fn find_sse_frame_end(buffer: &[u8]) -> Option<usize> {
    let lf = buffer.windows(2).position(|w| w == b"\n\n").map(|i| i + 2);
    let crlf = buffer
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4);
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
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
/// canonical id. `SystemPrompt` + `ToolCalls` are advertised — the
/// implementation transports tool declarations / calls / results (2026-09-08)
/// The wire is OpenAI Chat Completions with `stream:false` by default and
/// `stream:true` (incremental SSE) through [`ProviderCapability::complete_streaming`].
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
            [
                ModelFeature::SystemPrompt,
                ModelFeature::ToolCalls,
                ModelFeature::Streaming,
            ],
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

    /// One-shot mock vendor speaking the OpenAI SSE stream protocol.
    async fn sse_mock_server(frames: &'static [&'static str]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let request_text = String::from_utf8_lossy(&buf);
            assert!(
                request_text.contains("\"stream\":true"),
                "streaming call must request stream:true: {request_text}"
            );
            let head =
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
            let mut out = head.to_string();
            for frame in frames {
                out.push_str(frame);
                out.push_str("\n\n");
            }
            let _ = socket.write_all(out.as_bytes()).await;
            let _ = socket.flush().await;
            // Give the client time to read before the close.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });
        format!("http://{addr}")
    }

    /// The streaming path forwards incremental content deltas (not one final
    /// blob) and reassembles the same canonical response the non-streaming
    /// path would produce.
    #[tokio::test]
    async fn complete_streaming_forwards_incremental_deltas() {
        let base_url = sse_mock_server(&[
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2,\"total_tokens\":7}}",
            "data: [DONE]",
        ])
        .await;
        let resolver = Arc::new(Mutex::new(Some(Arc::new(
            apeireth_plugin::StaticCredentials::new().with(OPENAI_COMPATIBLE_API_KEY, "sk-mock"),
        ) as Arc<dyn CredentialResolver>)));
        let cap = OpenAiCompatibleProviderCapability::new(
            base_url,
            vec!["gpt-4o-mini".into()],
            http(),
            DEFAULT_TIMEOUT_MS,
            resolver,
        )
        .expect("capability builds");

        let deltas = Arc::new(Mutex::new(Vec::<String>::new()));
        let deltas_sink = Arc::clone(&deltas);
        let response = cap
            .complete_streaming(
                &request(),
                Arc::new(move |delta: String| {
                    deltas_sink.lock().unwrap().push(delta);
                }),
            )
            .await
            .expect("streaming completes");

        let deltas = deltas.lock().unwrap().clone();
        assert_eq!(deltas, vec!["你".to_string(), "好".to_string()]);
        assert_eq!(response.content, "你好");
        assert_eq!(
            response.finish_reason,
            Some(NormalizedFinishReason::Stop),
            "finish_reason from the final stream chunk"
        );
        assert_eq!(
            response.usage,
            NormalizedUsage {
                prompt_tokens: 5,
                completion_tokens: 2,
                total_tokens: 7,
            }
        );
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
        // Truthful features: SystemPrompt + ToolCalls + Streaming
        // (2026-09-08 tool transport; 2026-09-10 incremental SSE).
        assert!(cap.models()[0].supports(ModelFeature::SystemPrompt));
        assert!(cap.models()[0].supports(ModelFeature::ToolCalls));
        assert!(cap.models()[0].supports(ModelFeature::Streaming));
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
    fn adapt_request_transports_tools_and_rejects_images() {
        let cap = capability(empty_resolver_slot());
        // 工具声明进入原生 function 形状 (2026-09-08 tool transport).
        let mut req = request();
        req.tools
            .push(apeireth_protocol::canonical::NormalizedTool::new("t"));
        let body = cap.adapt_request(&req).expect("tools are transported");
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
        assert_eq!(body["tools"][0]["function"]["name"], "t");
        // 图像仍拒绝 (未声明 Vision).
        let mut img = request();
        img.messages.push(NormalizedMessage {
            role: apeireth_protocol::canonical::MessageRole::User,
            content: vec![apeireth_protocol::canonical::ContentPart::ImageUrl {
                url: "https://example.invalid/i.png".into(),
                detail: None,
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        });
        let err = cap.adapt_request(&img).unwrap_err();
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
