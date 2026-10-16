//! Provider HTTP relay for the desktop webview (zero-egress hardening).
//!
//! The webview must not reach provider APIs directly: the window CSP pins
//! `connect-src` to loopback, and every provider call is relayed through the
//! `provider_request` command so the request leaves from the Rust side.
//!
//! Security boundaries enforced here:
//! - only http/https URLs are forwarded (any other scheme is rejected)
//! - the whole request (send + body read) is bounded by [`REQUEST_TIMEOUT`]
//! - the relayed response body is capped at [`MAX_RESPONSE_BYTES`]
//! - credential header values never appear in logs or error strings
//!   ([`sanitize_error`], layered on the [`DesktopLogger`] redaction passes)

use crate::logging::DesktopLogger;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

/// Cap on a relayed response body (5 MiB). Model lists and provider error
/// payloads are far smaller; anything larger is refused instead of buffered.
pub const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

/// Whole-request timeout for a relayed call (30 s).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Marker substituted for credential material in logs and error strings.
pub const REDACTED: &str = "[REDACTED]";

/// Header names whose values are credential material. Values of these headers
/// are scrubbed from every message this module emits.
const SENSITIVE_HEADERS: [&str; 6] = [
    "authorization",
    "proxy-authorization",
    "x-api-key",
    "api-key",
    "x-auth-token",
    "cookie",
];

/// Response relayed back to the webview, mirroring the IPC return contract
/// `{ status, body_text, content_type }`.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderResponse {
    pub status: u16,
    pub body_text: String,
    pub content_type: Option<String>,
}

/// Validate the relay target and return it parsed.
///
/// Only `http`/`https` URLs with a host are forwarded; every other scheme
/// (`file:`, `ftp:`, `javascript:`, `data:`, ...) is refused.
pub fn validate_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed =
        reqwest::Url::parse(url.trim()).map_err(|_| "provider_request: invalid URL".to_string())?;
    match (parsed.scheme(), parsed.host_str()) {
        ("http" | "https", Some(_)) => Ok(parsed),
        ("http" | "https", None) => Err("provider_request: URL has no host".to_string()),
        _ => Err(format!(
            "provider_request: URL scheme '{}' is not allowed (http/https only)",
            parsed.scheme()
        )),
    }
}

/// Validate the relay method against a small allowlist.
///
/// The relay carries provider API calls (JSON request/response), not
/// arbitrary verbs, so tunnel-style methods (CONNECT) and diagnostics
/// (TRACE) are refused alongside anything malformed.
pub fn validate_method(method: &str) -> Result<reqwest::Method, String> {
    let upper = method.trim().to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD"
    ) {
        reqwest::Method::from_bytes(upper.as_bytes())
            .map_err(|_| "provider_request: invalid method".to_string())
    } else {
        Err(format!(
            "provider_request: method '{}' is not allowed",
            upper
        ))
    }
}

fn is_sensitive_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_HEADERS.iter().any(|entry| *entry == lower)
}

/// Values of credential-bearing headers, collected so they can be scrubbed
/// from any message this module produces.
pub fn collect_secret_values(headers: &HashMap<String, String>) -> Vec<String> {
    headers
        .iter()
        .filter(|(name, value)| is_sensitive_header(name) && !value.is_empty())
        .map(|(_, value)| value.clone())
        .collect()
}

/// Scrub credential material from text destined for a log line or an error
/// message: exact header values first, then the shared log redaction passes
/// (named assignments, `Bearer <token>`, bare `sk-…` key prefixes).
pub fn sanitize_error(message: &str, secrets: &[String]) -> String {
    let mut out = message.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret.as_str(), REDACTED);
        }
    }
    DesktopLogger::redact_secrets(&out)
}

/// Accumulates relayed response bytes while enforcing a hard size cap.
#[derive(Debug)]
pub struct BoundedBody {
    buf: Vec<u8>,
    limit: usize,
}

impl BoundedBody {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            buf: Vec::new(),
            limit,
        }
    }

    /// Append a chunk, or fail once the cap would be exceeded.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), String> {
        if self.buf.len().saturating_add(chunk.len()) > self.limit {
            return Err(format!(
                "provider_request: response body exceeds the {} byte cap",
                self.limit
            ));
        }
        self.buf.extend_from_slice(chunk);
        Ok(())
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

/// Machine-readable reason carried by the synthetic "not configured" frame.
///
/// Same vocabulary the gateway capability face already uses for a supported
/// but unconfigured capability (`reason: provider_not_configured`), so callers
/// see one convention across faces.
pub const PROVIDER_NOT_CONFIGURED: &str = "provider_not_configured";

/// Status of the synthetic "not configured" frame.
///
/// Deliberately not `401`: this frame is minted locally and must never be
/// confusable with a provider's own authentication verdict. (The first-boot
/// logs this replaces showed live `status=401` rows produced by reaching the
/// real endpoint with an empty key.)
pub const PROVIDER_NOT_CONFIGURED_STATUS: u16 = 503;

/// Whether the relay target is the loopback face (the local gateway).
///
/// Local calls legitimately carry no provider credential and must relay
/// unchanged; the not-configured gate applies to outbound provider calls only.
fn is_loopback_target(target: &reqwest::Url) -> bool {
    let Some(host) = target.host_str() else {
        // No host at all: `validate_url` refuses such targets before this runs.
        return true;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// Whether the request carries any non-empty credential header.
fn has_credential(headers: &HashMap<String, String>) -> bool {
    headers
        .iter()
        .any(|(name, value)| is_sensitive_header(name) && !value.trim().is_empty())
}

/// The synthetic "not configured" frame: an explicit local answer for a probe
/// or model-list call made without any provider credential.
///
/// Shape follows the gateway error-frame convention
/// (`{"error": {code, message, solution}}`) plus the explicit `reason` callers
/// key their "not configured" UX off.
pub fn provider_not_configured_frame(host: &str) -> ProviderResponse {
    let body = serde_json::json!({
        "error": {
            "code": PROVIDER_NOT_CONFIGURED,
            "reason": PROVIDER_NOT_CONFIGURED,
            "message": "provider key not configured; outbound call suppressed",
            "solution": "在设置中填写提供商密钥后重试",
        },
        "host": host,
    });
    ProviderResponse {
        status: PROVIDER_NOT_CONFIGURED_STATUS,
        body_text: body.to_string(),
        content_type: Some("application/json".to_string()),
    }
}

/// Whether `response` is the synthetic frame minted by
/// [`provider_not_configured_frame`] (drives the relay's log face).
pub fn is_provider_not_configured_frame(response: &ProviderResponse) -> bool {
    response.status == PROVIDER_NOT_CONFIGURED_STATUS
        && response.body_text.contains(PROVIDER_NOT_CONFIGURED)
}

/// No-credential gate: with no provider key in the request, an outbound call
/// would only ever be answered with an authentication error by the far side.
/// Such calls are answered locally with [`provider_not_configured_frame`] and
/// never leave the process. Loopback targets and any request carrying a
/// non-empty credential header pass through untouched.
fn unconfigured_suppression(
    target: &reqwest::Url,
    headers: &HashMap<String, String>,
) -> Option<ProviderResponse> {
    if is_loopback_target(target) || has_credential(headers) {
        return None;
    }
    Some(provider_not_configured_frame(
        target.host_str().unwrap_or("-"),
    ))
}

/// One validated relay request handed to the outbound transport.
///
/// No `Debug` on purpose: its headers carry credential values, and this module
/// never lets those reach a log line or error string.
#[derive(Clone)]
pub struct RelayRequest {
    /// Validated relay target.
    pub url: reqwest::Url,
    /// Validated relay verb.
    pub method: reqwest::Method,
    /// Caller-supplied headers, forwarded as given.
    pub headers: HashMap<String, String>,
    /// Optional request body.
    pub body: Option<String>,
}

/// The transport's answer for one relayed call, before IPC framing.
#[derive(Debug, Clone)]
pub struct RelayResponse {
    /// HTTP status as returned by the target.
    pub status: u16,
    /// Response body text.
    pub body_text: String,
    /// Response content type, when declared.
    pub content_type: Option<String>,
}

/// The relay's outbound HTTP face.
///
/// Split out so tests can drive the relay against a fake transport and assert
/// exactly which requests leave the process: the not-configured gate must
/// produce **zero** transport calls.
pub trait RelayTransport: Send + Sync {
    /// Perform one relayed HTTP call.
    fn send(
        &self,
        request: RelayRequest,
    ) -> impl std::future::Future<Output = Result<RelayResponse, String>> + Send;
}

/// Production transport: one bounded [`reqwest`] call (timeout + body cap).
pub struct ReqwestTransport;

impl RelayTransport for ReqwestTransport {
    // 保持 RPITIT 形态（不用 async fn 简写）：显式 `+ Send` 把返回 future 钉成
    // Send，转发面因此可被 tauri 命令的 Send future 消费（async fn 写法会丢该约束）。
    #[allow(clippy::manual_async_fn)]
    fn send(
        &self,
        request: RelayRequest,
    ) -> impl std::future::Future<Output = Result<RelayResponse, String>> + Send {
        async move {
            let client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .map_err(|error| format!("provider_request: HTTP client build failed: {error}"))?;

            let mut header_map = reqwest::header::HeaderMap::new();
            for (name, value) in &request.headers {
                // Header errors are reported without echoing the offending values.
                let parsed_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| "provider_request: invalid header name".to_string())?;
                let parsed_value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|_| "provider_request: invalid header value".to_string())?;
                header_map.append(parsed_name, parsed_value);
            }

            let mut outbound = client
                .request(request.method, request.url)
                .headers(header_map);
            if let Some(body) = request.body {
                outbound = outbound.body(body);
            }

            let mut response = outbound.send().await.map_err(|error| {
                format!("provider_request: request failed: {}", error.without_url())
            })?;

            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.to_string());

            // Refuse oversized payloads up front when the length is declared;
            // the chunked case is caught by BoundedBody below.
            if let Some(length) = response.content_length() {
                if length > MAX_RESPONSE_BYTES as u64 {
                    return Err(format!(
                        "provider_request: response body exceeds the {MAX_RESPONSE_BYTES} byte cap"
                    ));
                }
            }

            let mut body_acc = BoundedBody::with_limit(MAX_RESPONSE_BYTES);
            while let Some(chunk) = response.chunk().await.map_err(|error| {
                format!(
                    "provider_request: response read failed: {}",
                    error.without_url()
                )
            })? {
                body_acc.push(&chunk)?;
            }

            Ok(RelayResponse {
                status,
                body_text: String::from_utf8_lossy(&body_acc.into_bytes()).into_owned(),
                content_type,
            })
        }
    }
}

/// Relay one provider HTTP call and return the response as text.
///
/// Every error string leaves through [`sanitize_error`], so credential values
/// cannot surface to the webview or the desktop log. The request URL is never
/// echoed in errors (`without_url`), keeping URL-carried credentials out too.
pub async fn send_provider_request(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<ProviderResponse, String> {
    let secrets = collect_secret_values(headers);
    relay_via(&ReqwestTransport, url, method, headers, body)
        .await
        .map_err(|message| sanitize_error(&message, &secrets))
}

/// Relay one call over `transport`, applying validation and the
/// not-configured gate first: a credential-less outbound call is answered with
/// the synthetic frame and never reaches the transport.
async fn relay_via<T: RelayTransport>(
    transport: &T,
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<ProviderResponse, String> {
    let target = validate_url(url)?;
    let verb = validate_method(method)?;
    if let Some(frame) = unconfigured_suppression(&target, headers) {
        return Ok(frame);
    }
    let response = transport
        .send(RelayRequest {
            url: target,
            method: verb,
            headers: headers.clone(),
            body: body.map(str::to_string),
        })
        .await?;
    Ok(ProviderResponse {
        status: response.status,
        body_text: response.body_text,
        content_type: response.content_type,
    })
}

/// Relay a provider HTTP request on behalf of the webview.
///
/// Arguments mirror the frontend contract: `{ url, method, headers, body }`;
/// the return value is `{ status, body_text, content_type }`. The log line
/// carries a fixed-field summary only — no header values, no body, no URL.
#[tauri::command]
pub async fn provider_request(
    logger: tauri::State<'_, std::sync::Arc<DesktopLogger>>,
    url: String,
    method: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
) -> Result<ProviderResponse, String> {
    let headers = headers.unwrap_or_default();
    let outcome = send_provider_request(&url, &method, &headers, body.as_deref()).await;

    let verb = method.trim().to_ascii_uppercase();
    let host = reqwest::Url::parse(url.trim())
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_else(|| "-".to_string());
    match &outcome {
        Ok(response) if is_provider_not_configured_frame(response) => logger.log_desktop(
            crate::logging::LogLevel::Info,
            &format!(
                "provider_request skipped method={verb} host={host} reason={PROVIDER_NOT_CONFIGURED}"
            ),
        ),
        Ok(response) => logger.log_desktop(
            crate::logging::LogLevel::Info,
            &format!(
                "provider_request ok method={verb} host={host} status={} bytes={}",
                response.status,
                response.body_text.len()
            ),
        ),
        Err(message) => logger.log_desktop(
            crate::logging::LogLevel::Warn,
            &format!("provider_request failed method={verb} host={host} error={message}"),
        ),
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_url_schemes() {
        for url in [
            "file:///C:/Windows/System32/config/SAM",
            "ftp://example.com/models",
            "javascript:alert(1)",
            "data:text/html;base64,PGh0bWw+",
            "ws://127.0.0.1:8080/",
            "mailto:someone@example.com",
            "",
            "not a url",
        ] {
            assert!(validate_url(url).is_err(), "{url} must be rejected");
        }
    }

    #[test]
    fn accepts_http_and_https_urls() {
        for url in [
            "https://api.deepseek.com/v1/models",
            "http://127.0.0.1:8080/health",
        ] {
            assert!(validate_url(url).is_ok(), "{url} must be relayable");
        }
    }

    #[test]
    fn rejects_unsupported_methods() {
        for method in ["CONNECT", "TRACE", "TRACK", "", "GET / HTTP/1.1"] {
            assert!(
                validate_method(method).is_err(),
                "{method:?} must be rejected"
            );
        }
        assert!(
            validate_method("post").is_ok(),
            "verbs are case-insensitive"
        );
    }

    #[test]
    fn enforces_response_body_cap() {
        let mut body = BoundedBody::with_limit(8);
        body.push(b"1234").expect("under cap");
        body.push(b"5678").expect("exactly at cap");
        assert_eq!(body.into_bytes(), b"12345678");

        let mut body = BoundedBody::with_limit(8);
        body.push(b"12345678").expect("exactly at cap");
        assert!(
            body.push(b"x").is_err(),
            "one byte over the cap must be refused"
        );
    }

    #[test]
    fn response_cap_is_five_mebibytes() {
        assert_eq!(MAX_RESPONSE_BYTES, 5 * 1024 * 1024);
    }

    #[test]
    fn collects_only_credential_header_values() {
        let headers = HashMap::from([
            (
                "Authorization".to_string(),
                "Bearer sk-secret-1".to_string(),
            ),
            ("x-api-key".to_string(), "mm-secret-2".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]);
        let secrets = collect_secret_values(&headers);
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains(&"Bearer sk-secret-1".to_string()));
        assert!(secrets.contains(&"mm-secret-2".to_string()));
    }

    #[test]
    fn scrubs_credential_values_from_messages() {
        let secrets = vec!["Bearer sk-secret-1".to_string(), "mm-secret-2".to_string()];
        let message = "provider_request: request failed for Bearer sk-secret-1 with mm-secret-2";
        let sanitized = sanitize_error(message, &secrets);
        assert!(!sanitized.contains("sk-secret-1"), "leaked: {sanitized}");
        assert!(!sanitized.contains("mm-secret-2"), "leaked: {sanitized}");
    }

    #[test]
    fn scrubs_pattern_matched_credentials_without_header_context() {
        let sanitized = sanitize_error("upstream: Authorization: Bearer sk-abcd1234", &[]);
        assert!(!sanitized.contains("sk-abcd1234"), "leaked: {sanitized}");
    }

    #[test]
    fn keeps_non_secret_diagnostics_intact() {
        let message = "provider_request: request failed: connection refused";
        assert_eq!(sanitize_error(message, &[]), message);
    }

    // -----------------------------------------------------------------------
    // E-06 未配置即外呼收口: 假传输面记录每一次出网请求, 用零调用断言钉住
    // "无凭据不外呼"; 有凭据时请求/响应原样过境 (行为零变化)。
    // -----------------------------------------------------------------------

    /// Test transport that records every request it is asked to send and
    /// answers with one fixed canned response.
    struct FakeTransport {
        calls: std::sync::Mutex<Vec<RelayRequest>>,
    }

    impl FakeTransport {
        fn new() -> Self {
            Self {
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn call_count(&self) -> usize {
            self.calls
                .lock()
                .map(|calls| calls.len())
                .unwrap_or_else(|poisoned| poisoned.into_inner().len())
        }

        fn last_call(&self) -> RelayRequest {
            self.calls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .last()
                .expect("at least one transport call")
                .clone()
        }
    }

    impl RelayTransport for FakeTransport {
        // 与生产实现同形：RPITIT + 显式 `+ Send`（见 `ReqwestTransport::send`）。
        #[allow(clippy::manual_async_fn)]
        fn send(
            &self,
            request: RelayRequest,
        ) -> impl std::future::Future<Output = Result<RelayResponse, String>> + Send {
            async move {
                self.calls
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(request);
                Ok(RelayResponse {
                    status: 200,
                    body_text: r#"{"object":"list","data":[]}"#.to_string(),
                    content_type: Some("application/json".to_string()),
                })
            }
        }
    }

    #[tokio::test]
    async fn unconfigured_remote_probe_is_answered_without_egress() {
        let transport = FakeTransport::new();
        let response = relay_via(
            &transport,
            "https://api.example.com/v1/models",
            "GET",
            &HashMap::new(),
            None,
        )
        .await
        .expect("the gate answers locally");
        assert_eq!(
            transport.call_count(),
            0,
            "a credential-less outbound call must never leave the process"
        );
        assert!(
            is_provider_not_configured_frame(&response),
            "expected the not-configured frame, got: {:?}",
            response
        );
        assert!(
            response
                .body_text
                .contains("\"reason\":\"provider_not_configured\""),
            "the frame must carry an explicit reason: {}",
            response.body_text
        );
    }

    #[tokio::test]
    async fn empty_credential_headers_count_as_unconfigured() {
        let transport = FakeTransport::new();
        let headers = HashMap::from([
            ("Authorization".to_string(), "  ".to_string()),
            ("x-api-key".to_string(), String::new()),
        ]);
        let response = relay_via(
            &transport,
            "https://api.example.com/v1/models",
            "GET",
            &headers,
            None,
        )
        .await
        .expect("the gate answers locally");
        assert_eq!(
            transport.call_count(),
            0,
            "an empty key must not produce an outbound call"
        );
        assert!(is_provider_not_configured_frame(&response));
    }

    #[tokio::test]
    async fn a_configured_call_relays_unchanged() {
        let transport = FakeTransport::new();
        let headers =
            HashMap::from([("Authorization".to_string(), "Bearer sk-live-1".to_string())]);
        let response = relay_via(
            &transport,
            "https://api.example.com/v1/models",
            "GET",
            &headers,
            Some(r#"{"probe":true}"#),
        )
        .await
        .expect("configured calls relay");
        assert_eq!(
            transport.call_count(),
            1,
            "a configured call reaches the transport exactly once"
        );
        let sent = transport.last_call();
        assert_eq!(sent.url.as_str(), "https://api.example.com/v1/models");
        assert_eq!(sent.method, reqwest::Method::GET);
        assert_eq!(
            sent.headers.get("Authorization").map(String::as_str),
            Some("Bearer sk-live-1"),
            "credentials pass through untouched"
        );
        assert_eq!(sent.body.as_deref(), Some(r#"{"probe":true}"#));
        assert_eq!(
            response.status, 200,
            "the transport response is relayed as-is"
        );
        assert_eq!(response.body_text, r#"{"object":"list","data":[]}"#);
        assert!(!is_provider_not_configured_frame(&response));
    }

    #[tokio::test]
    async fn loopback_targets_relay_without_credentials() {
        for url in [
            "http://127.0.0.1:8080/v1/models",
            "http://[::1]:8080/v1/models",
            "http://localhost:8080/health",
        ] {
            let transport = FakeTransport::new();
            let response = relay_via(&transport, url, "GET", &HashMap::new(), None)
                .await
                .expect("local face relays");
            assert_eq!(
                transport.call_count(),
                1,
                "{url} is the loopback face and must relay without a credential"
            );
            assert_eq!(response.status, 200);
        }
    }

    #[test]
    fn not_configured_frame_is_explicit_and_is_not_an_auth_verdict() {
        let frame = provider_not_configured_frame("api.example.com");
        assert_eq!(frame.status, PROVIDER_NOT_CONFIGURED_STATUS);
        assert_ne!(
            frame.status, 401,
            "the local frame must never pose as a provider auth verdict"
        );
        assert_eq!(frame.content_type.as_deref(), Some("application/json"));
        let body: serde_json::Value = serde_json::from_str(&frame.body_text).expect("JSON frame");
        assert_eq!(body["error"]["reason"], PROVIDER_NOT_CONFIGURED);
        assert_eq!(body["error"]["code"], PROVIDER_NOT_CONFIGURED);
        assert!(body["error"]["solution"].is_string());
        assert_eq!(body["host"], "api.example.com");
        assert!(is_provider_not_configured_frame(&frame));
    }
}
