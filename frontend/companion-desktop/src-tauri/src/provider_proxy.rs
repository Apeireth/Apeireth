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
    relay(url, method, headers, body)
        .await
        .map_err(|message| sanitize_error(&message, &secrets))
}

async fn relay(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<ProviderResponse, String> {
    let target = validate_url(url)?;
    let verb = validate_method(method)?;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| format!("provider_request: HTTP client build failed: {error}"))?;

    let mut header_map = reqwest::header::HeaderMap::new();
    for (name, value) in headers {
        // Header errors are reported without echoing the offending values.
        let parsed_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| "provider_request: invalid header name".to_string())?;
        let parsed_value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|_| "provider_request: invalid header value".to_string())?;
        header_map.append(parsed_name, parsed_value);
    }

    let mut request = client.request(verb, target).headers(header_map);
    if let Some(body) = body {
        request = request.body(body.to_string());
    }

    let mut response = request
        .send()
        .await
        .map_err(|error| format!("provider_request: request failed: {}", error.without_url()))?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    // Refuse oversized payloads up front when the length is declared; the
    // chunked case is caught by BoundedBody below.
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

    Ok(ProviderResponse {
        status,
        body_text: String::from_utf8_lossy(&body_acc.into_bytes()).into_owned(),
        content_type,
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
}
