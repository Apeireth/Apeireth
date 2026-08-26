//! Canonical controlled Fetch capability.
//!
//! `tool.fetch` is a deliberately narrow GET-only HTTP(S) text fetch. It is a
//! [`ToolCapability`], not a browser, an API client, or a file downloader.
//!
//! # Ownership
//!
//! - Fetch owns the model schema, content-type gating, UTF-8 text rendering,
//!   and the tool result shape.
//! - [`crate::egress`] owns destination validation, DNS resolution, DNS
//!   pinning, HTTP execution, redirect revalidation, TLS, proxy policy, and
//!   the transport body limit.
//! - Governance owns the permission decision.
//! - Runtime owns orchestration and dispatch.
//!
//! Fetch does not use the shell, does not call `reqwest` directly, does not
//! resolve DNS for policy decisions, and does not duplicate IP classification.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::egress::{ControlledEgress, EgressAllowList, EgressError, EgressPolicy};

/// Maximum accepted URL length in bytes.
pub const FETCH_URL_MAX_BYTES: usize = 8 * 1024;

const FETCH_FROZEN_VERSION: u32 = 1;
const FETCH_METHOD: &str = "GET";

/// Trusted configuration for `tool.fetch`.
///
/// The controlled egress transport owns timeout, response size, redirect, and
/// policy enforcement. Fetch adds only the safe, fixed request headers it
/// sends. Fetch is disabled by default; passing a [`FetchConfig`] to
/// [`crate::BuiltinToolsOptions`] explicitly enables it.
#[derive(Debug, Clone)]
pub struct FetchConfig {
    egress: Arc<ControlledEgress>,
    user_agent: Option<String>,
}

impl FetchConfig {
    /// Build a fetch config from an already configured controlled transport.
    pub fn new(egress: Arc<ControlledEgress>) -> Self {
        Self {
            egress,
            user_agent: None,
        }
    }

    /// Public-internet-only fetch. Local, private, link-local, and other
    /// non-public destinations are denied by the transport.
    pub fn public_internet_only() -> Self {
        Self::new(Arc::new(ControlledEgress::new(
            EgressPolicy::PublicInternetOnly,
        )))
    }

    /// Deny-all fetch. Every request will fail closed at the transport.
    pub fn deny_all() -> Self {
        Self::new(Arc::new(ControlledEgress::new(EgressPolicy::DenyAll)))
    }

    /// Fetch restricted to an exact allow-list. Useful for tests and trusted
    /// private-network configurations.
    pub fn explicit_allow_list(list: EgressAllowList) -> Self {
        Self::new(Arc::new(ControlledEgress::new(
            EgressPolicy::ExplicitAllowList(list),
        )))
    }

    /// Fetch with no destination restrictions. Explicit opt-out reserved for
    /// trusted configurations. Never the default, never model-selectable.
    pub fn unrestricted() -> Self {
        Self::new(Arc::new(ControlledEgress::new(EgressPolicy::Unrestricted)))
    }

    /// Set the fixed factual User-Agent header. The model cannot override it.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn egress(&self) -> &Arc<ControlledEgress> {
        &self.egress
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
}

#[derive(Debug, Deserialize)]
struct FetchParams {
    url: String,
}

/// The exact, versioned execution inputs frozen at approval time.
///
/// This is Fetch's own payload schema. Runtime treats it as opaque
/// `serde_json::Value`; Fetch owns deserialization and must execute these
/// fields — and only these fields — when resuming an approved operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FetchFrozenInvocation {
    version: u32,
    method: String,
    url: String,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_redirects: usize,
    egress_policy: EgressPolicy,
    user_agent: Option<String>,
}

/// The M3A controlled Fetch tool.
pub struct FetchTool {
    id: CapabilityId,
    config: FetchConfig,
}

impl FetchTool {
    pub fn new(config: FetchConfig) -> Self {
        Self {
            id: CapabilityId::new("tool.fetch").unwrap(),
            config,
        }
    }

    pub fn config(&self) -> &FetchConfig {
        &self.config
    }

    fn declaration_parameters() -> ToolParameters {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": FETCH_URL_MAX_BYTES,
                    "description": "HTTP(S) URL to fetch with GET. URL userinfo, unsupported schemes, and non-http(s) schemes are rejected."
                }
            },
            "required": ["url"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(
            parameters
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter(),
        );
        params
    }

    fn parse_params(&self, call: &ToolCall) -> Result<FetchParams, ToolResult> {
        let object = call.arguments.as_object().ok_or_else(|| {
            ToolResult::permanent_error(&call.id, "fetch arguments must be a JSON object")
                .with_name("fetch")
        })?;

        for key in object.keys() {
            if key != "url" {
                return Err(ToolResult::permanent_error(
                    &call.id,
                    format!("unknown fetch parameter {key:?}; only \"url\" is supported"),
                )
                .with_name("fetch"));
            }
        }

        let params: FetchParams = serde_json::from_value(call.arguments.clone()).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("invalid fetch parameters: {e}"))
                .with_name("fetch")
        })?;

        Ok(params)
    }

    fn normalize_url(&self, call: &ToolCall, raw: &str) -> Result<Url, ToolResult> {
        if raw.is_empty() {
            return Err(
                ToolResult::permanent_error(&call.id, "fetch url must not be empty")
                    .with_name("fetch"),
            );
        }
        if raw.len() > FETCH_URL_MAX_BYTES {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "fetch url is {} bytes; the maximum is {FETCH_URL_MAX_BYTES} bytes",
                    raw.len()
                ),
            )
            .with_name("fetch"));
        }

        let mut url = Url::parse(raw).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("invalid fetch url: {e}"))
                .with_name("fetch")
        })?;

        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported fetch url scheme {:?}; only http and https are supported",
                    url.scheme()
                ),
            )
            .with_name("fetch"));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ToolResult::permanent_error(
                &call.id,
                "fetch url userinfo is not supported and must not be sent",
            )
            .with_name("fetch"));
        }
        if url.host_str().is_none() {
            return Err(
                ToolResult::permanent_error(&call.id, "fetch url must include a host")
                    .with_name("fetch"),
            );
        }

        // Fragments are never sent over HTTP. Strip before freezing/execution
        // so the effective operation is the actual HTTP request.
        url.set_fragment(None);
        Ok(url)
    }

    fn build_frozen(&self, call: &ToolCall) -> Result<FetchFrozenInvocation, ToolResult> {
        let params = self.parse_params(call)?;
        let url = self.normalize_url(call, &params.url)?;

        let egress = self.config.egress();
        let timeout_ms = egress.timeout().as_millis() as u64;
        let max_response_bytes = egress.max_response_bytes();
        let max_redirects = egress.max_redirects();
        let egress_policy = egress.policy().clone();

        Ok(FetchFrozenInvocation {
            version: FETCH_FROZEN_VERSION,
            method: FETCH_METHOD.to_string(),
            url: url.to_string(),
            timeout_ms,
            max_response_bytes,
            max_redirects,
            egress_policy,
            user_agent: self.config.user_agent().map(ToOwned::to_owned),
        })
    }

    fn display_invocation(frozen: &FetchFrozenInvocation) -> serde_json::Value {
        serde_json::json!({
            "version": frozen.version,
            "method": frozen.method,
            "url": frozen.url,
            "egress_policy": format!("{:?}", frozen.egress_policy),
            "timeout_ms": frozen.timeout_ms,
            "max_response_bytes": frozen.max_response_bytes,
            "max_redirects": frozen.max_redirects,
        })
    }

    async fn execute_frozen(&self, call: &ToolCall, frozen: &FetchFrozenInvocation) -> ToolResult {
        if frozen.version != FETCH_FROZEN_VERSION {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen fetch invocation version {} (expected {})",
                    frozen.version, FETCH_FROZEN_VERSION
                ),
            )
            .with_name("fetch");
        }
        if frozen.method != FETCH_METHOD {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen fetch method {:?}; only GET is supported",
                    frozen.method
                ),
            )
            .with_name("fetch");
        }

        // Reconstruct the transport from the frozen declarative policy and
        // bounds. DNS is intentionally not frozen: the actual connection is
        // still resolved and revalidated at execution time by the controlled
        // transport.
        let egress = ControlledEgress::new(frozen.egress_policy.clone())
            .with_timeout(Duration::from_millis(frozen.timeout_ms))
            .with_max_response_bytes(frozen.max_response_bytes)
            .with_max_redirects(frozen.max_redirects);

        self.execute_with_egress(call, &egress, frozen.user_agent.as_deref(), &frozen.url)
            .await
    }

    async fn execute_with_egress(
        &self,
        call: &ToolCall,
        egress: &ControlledEgress,
        user_agent: Option<&str>,
        url: &str,
    ) -> ToolResult {
        let mut headers: Vec<(&str, &str)> = Vec::with_capacity(2);
        headers.push(("accept", "*/*"));
        if let Some(user_agent) = user_agent {
            headers.push(("user-agent", user_agent));
        }

        let response = match egress.get_with_headers(url, &headers).await {
            Ok(response) => response,
            Err(error) => return map_egress_error(&call.id, error),
        };

        let content_type = response.content_type.clone();
        let body = response.body;

        // Determine whether the response is a textual resource and decode it.
        let (decoded, effective_content_type) =
            match decode_text_body(&call.id, &body, content_type.as_deref()) {
                Ok(decoded) => decoded,
                Err(result) => return result,
            };

        let final_url = sanitize_final_url(&response.final_url);
        let value = serde_json::json!({
            "url": final_url,
            "status": response.status,
            "content_type": effective_content_type,
            "content_length": response.content_length,
            "body": decoded,
            "bytes": body.len(),
            "redirects": response.redirects,
        });

        ToolResult::ok(&call.id, value).with_name("fetch")
    }
}

fn map_egress_error(call_id: &str, error: EgressError) -> ToolResult {
    let retryable = matches!(
        &error,
        EgressError::Timeout
            | EgressError::ConnectionFailed(_)
            | EgressError::ResolutionFailed { .. }
    );
    if retryable {
        ToolResult::retryable_error(call_id, format!("fetch failed: {error}")).with_name("fetch")
    } else {
        ToolResult::permanent_error(call_id, format!("fetch failed: {error}")).with_name("fetch")
    }
}

fn sanitize_final_url(raw: &str) -> String {
    match Url::parse(raw) {
        Ok(mut url) => {
            url.set_fragment(None);
            url.to_string()
        }
        Err(_) => raw.to_string(),
    }
}

fn media_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn charset_from_content_type(content_type: &str) -> Option<String> {
    content_type.split(';').skip(1).find_map(|part| {
        let part = part.trim();
        let lower = part.to_ascii_lowercase();
        let prefix = "charset=";
        if lower.starts_with(prefix) {
            let value = part[prefix.len()..].trim().trim_matches('"');
            Some(value.to_ascii_lowercase())
        } else {
            None
        }
    })
}

fn is_textual_media_type(content_type: &str) -> bool {
    let essence = media_type_essence(content_type);
    essence.starts_with("text/")
        || essence == "application/json"
        || essence == "application/xml"
        || essence == "application/xhtml+xml"
        || essence == "application/javascript"
        || essence.ends_with("+json")
        || essence.ends_with("+xml")
}

fn decode_text_body(
    call_id: &str,
    body: &[u8],
    content_type: Option<&str>,
) -> Result<(String, Option<String>), ToolResult> {
    if let Some(content_type) = content_type {
        if !is_textual_media_type(content_type) {
            return Err(ToolResult::permanent_error(
                call_id,
                format!(
                    "unsupported media type {:?}; fetch v1 returns only bounded textual resources",
                    media_type_essence(content_type)
                ),
            )
            .with_name("fetch"));
        }

        if let Some(charset) = charset_from_content_type(content_type) {
            if charset != "utf-8"
                && charset != "utf8"
                && charset != "ascii"
                && charset != "us-ascii"
            {
                return Err(ToolResult::permanent_error(
                    call_id,
                    format!("unsupported charset={charset}; fetch v1 supports UTF-8 and ASCII"),
                )
                .with_name("fetch"));
            }
        }

        match String::from_utf8(body.to_vec()) {
            Ok(text) => Ok((text, Some(content_type.to_string()))),
            Err(_) => Err(ToolResult::permanent_error(
                call_id,
                "response body is not valid UTF-8".to_string(),
            )
            .with_name("fetch")),
        }
    } else if body.is_empty() {
        // A 204 or otherwise empty body with no Content-Type is an empty text
        // result, not an error.
        Ok((String::new(), None))
    } else if body.contains(&0u8) {
        Err(ToolResult::permanent_error(
            call_id,
            "response body has no Content-Type and contains NUL bytes; refusing to treat it as text"
                .to_string(),
        )
        .with_name("fetch"))
    } else {
        match String::from_utf8(body.to_vec()) {
            Ok(text) => Ok((text, None)),
            Err(_) => Err(ToolResult::permanent_error(
                call_id,
                "response body has no Content-Type and is not valid UTF-8; refusing to treat it as text"
                    .to_string(),
            )
            .with_name("fetch")),
        }
    }
}

#[async_trait]
impl ToolCapability for FetchTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("fetch")
            .with_description(
                "Fetches one bounded HTTP(S) text resource using Apeireth's \
                 policy-enforced controlled egress transport. GET only; no cookies, \
                 credentials, proxy, or binary downloads.",
            )
            .with_parameters(Self::declaration_parameters())
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call)?;
        let payload = serde_json::to_value(&frozen).map_err(|e| {
            ToolResult::permanent_error(
                &call.id,
                format!("failed to serialize frozen fetch invocation: {e}"),
            )
            .with_name("fetch")
        })?;
        let display = Self::display_invocation(&frozen);
        Ok(Some(FrozenInvocation::new(payload, display)))
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let Some(frozen) = frozen else {
            return self.invoke(call).await;
        };

        let fetch_frozen: FetchFrozenInvocation =
            match serde_json::from_value(frozen.payload.clone()) {
                Ok(fetch_frozen) => fetch_frozen,
                Err(e) => {
                    return ToolResult::permanent_error(
                        &call.id,
                        format!("frozen fetch invocation is invalid: {e}"),
                    )
                    .with_name("fetch")
                }
            };

        self.execute_frozen(call, &fetch_frozen).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let frozen = match self.build_frozen(call) {
            Ok(frozen) => frozen,
            Err(result) => return result,
        };
        self.execute_frozen(call, &frozen).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool() -> FetchTool {
        FetchTool::new(FetchConfig::deny_all())
    }

    #[test]
    fn declaration_is_honest_and_not_browser_named() {
        let declaration = tool().declaration();
        assert_eq!(declaration.name, "fetch");
        let description = declaration.description.unwrap_or_default();
        assert!(!description.contains("browser"), "{description}");
        assert!(!description.contains("safe internet"), "{description}");
        assert!(description.contains("controlled egress"), "{description}");
    }

    #[test]
    fn schema_allows_only_url() {
        let params = FetchTool::declaration_parameters();
        let object = params.get("additionalProperties").and_then(|v| v.as_bool());
        assert_eq!(object, Some(false));
        let required = params.get("required").and_then(|v| v.as_array());
        assert!(required.is_some_and(|r| r.len() == 1 && r[0] == "url"));
    }

    #[test]
    fn empty_url_is_rejected() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": "" }),
        };
        let result = tool().invoke_sync(&call);
        assert!(!result.is_ok());
        assert!(
            result.render().contains("must not be empty"),
            "{}",
            result.render()
        );
    }

    #[test]
    fn unsupported_scheme_is_rejected() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": "file:///etc/passwd" }),
        };
        let result = tool().invoke_sync(&call);
        assert!(!result.is_ok());
        assert!(result.render().contains("scheme"), "{}", result.render());
    }

    #[test]
    fn userinfo_is_rejected_without_leaking_password() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": "https://user:secret@example.com/" }),
        };
        let result = tool().invoke_sync(&call);
        assert!(!result.is_ok());
        let text = result.render();
        assert!(!text.contains("secret"), "{text}");
        assert!(text.contains("userinfo"), "{text}");
    }

    #[test]
    fn oversized_url_is_rejected() {
        let long_url = format!("https://example.com/{}", "a".repeat(FETCH_URL_MAX_BYTES));
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": long_url }),
        };
        let result = tool().invoke_sync(&call);
        assert!(!result.is_ok());
        assert!(result.render().contains("maximum"), "{}", result.render());
    }

    #[test]
    fn unknown_parameter_is_rejected() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": "https://example.com/", "method": "POST" }),
        };
        let result = tool().invoke_sync(&call);
        assert!(!result.is_ok());
        assert!(
            result.render().contains("unknown fetch parameter"),
            "{}",
            result.render()
        );
    }

    #[test]
    fn freeze_invocation_rejects_invalid_url() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": "not a url" }),
        };
        let frozen = tool().freeze_invocation(&call);
        assert!(frozen.is_err());
    }

    #[test]
    fn frozen_display_contains_policy_and_bounds_but_no_fragment() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "fetch".into(),
            arguments: json!({ "url": "https://example.com/?q=hello#frag" }),
        };
        let frozen = tool().freeze_invocation(&call).unwrap().unwrap();
        let display = serde_json::to_string(&frozen.display).unwrap();
        assert!(display.contains("url"));
        assert!(display.contains("egress_policy"));
        assert!(display.contains("timeout_ms"));
        assert!(display.contains("max_response_bytes"));
        assert!(display.contains("https://example.com/?q=hello"));
        assert!(!display.contains("#frag"));
    }

    // Helper to call async invoke from sync tests.
    impl FetchTool {
        fn invoke_sync(&self, call: &ToolCall) -> ToolResult {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(self.invoke(call))
        }
    }
}
