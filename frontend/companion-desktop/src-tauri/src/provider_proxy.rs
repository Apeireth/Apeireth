//! Provider HTTP relay for the desktop webview (zero-egress hardening).
//!
//! The webview must not reach provider APIs directly: the window CSP pins
//! `connect-src` to loopback, and every provider call is relayed through the
//! `provider_request` command so the request leaves from the Rust side.
//!
//! Security boundaries enforced here:
//! - only http/https URLs are forwarded (any other scheme is rejected)
//! - the relay target is gated by address class (`gate_target`): the loopback
//!   face (local gateway / local model runtimes — deliberate carve-out, see
//!   `gate_target`) and public unicast only; private, link-local, shared,
//!   multicast, unspecified, documentation, benchmarking and reserved ranges
//!   are refused, for IP literals (including decimal/hex/short spellings and
//!   IPv4-mapped IPv6, which the URL parser normalizes) and for **every**
//!   address a host name resolves to
//! - DNS pinning: the connection uses exactly the addresses the gate checked
//!   (`resolve_to_addrs`), never a second resolution, which closes the
//!   first-hop DNS-rebinding window
//! - redirects are bounded by [`MAX_REDIRECT_HOPS`] and every hop re-passes the
//!   target gate at the reqwest redirect-policy layer
//! - only whitelisted request headers are forwarded (`filter_headers`):
//!   routing (`Host`), session (`Cookie`) and framing (`Content-Length`, ...)
//!   headers never leave the process
//! - the whole request (send + body read) is bounded by [`REQUEST_TIMEOUT`]
//! - the relayed response body is capped at [`MAX_RESPONSE_BYTES`]
//! - credential header values never appear in logs or error strings
//!   ([`sanitize_error`], layered on the [`DesktopLogger`] redaction passes)
//!
//! Residual risks (documented, not solved in this module):
//! - the loopback carve-out is a **class** carve-out: any local HTTP service,
//!   not only the gateway/model-runtime faces, stays reachable through the
//!   relay under IPC abuse. Narrowing it to registered local endpoints needs a
//!   cross-file endpoint-registration face (out of this file's scope).
//! - redirect hops to *other* hosts are gated per hop (scheme + address
//!   classes + resolver pre-check) but that hop connection re-resolves DNS, so
//!   a rebinding window remains on those hops; the first hop (and same-host
//!   hops) are pinned to the checked addresses.

use crate::logging::DesktopLogger;
use serde::Serialize;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
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

/// Request headers the relay forwards (lowercase names): credential faces,
/// content negotiation and content type. Everything else is dropped — routing
/// (`host`), session (`cookie`), framing (`content-length`, `transfer-encoding`,
/// `connection`, `upgrade`), forwarding (`forwarded`, `x-forwarded-*`) and
/// browser opt-ins: the far side derives routing/framing from the URL and
/// body, and the relay is not a browser.
const FORWARDED_HEADERS: [&str; 6] = [
    "authorization",
    "x-api-key",
    "api-key",
    "x-auth-token",
    "content-type",
    "accept",
];

/// Suffix rule for provider API version pins (`<api>-version` wire headers).
const VERSION_HEADER_SUFFIX: &str = "-version";

/// Maximum redirect hops the relay follows. Every hop re-passes the target
/// gate; exceeding the cap aborts the request instead of following.
pub const MAX_REDIRECT_HOPS: usize = 5;

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
/// (`file:`, `ftp:`, `javascript:`, `data:`, ...) is refused. The address-class
/// gate (`gate_target`) runs on top of this in `relay_via`; this function only
/// pins scheme and host presence.
pub fn validate_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed =
        reqwest::Url::parse(url.trim()).map_err(|_| "provider_request: invalid URL".to_string())?;
    check_scheme_and_host(&parsed).map_err(|detail| format!("provider_request: {detail}"))?;
    Ok(parsed)
}

/// Scheme/host pre-check shared by the entry gate and the redirect gate
/// (unprefixed detail; callers add their own message prefix).
fn check_scheme_and_host(target: &reqwest::Url) -> Result<(), String> {
    match (target.scheme(), target.host_str()) {
        ("http" | "https", Some(_)) => Ok(()),
        ("http" | "https", None) => Err("URL has no host".to_string()),
        _ => Err(format!(
            "URL scheme '{}' is not allowed (http/https only)",
            target.scheme()
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

/// Enforce the request-header whitelist: only [`FORWARDED_HEADERS`] (plus the
/// `*-version` suffix rule) reach the transport, values untouched. Called on
/// the caller-supplied map before anything else, so credential detection and
/// the wire see the same header set.
pub fn filter_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers
        .iter()
        .filter(|(name, _)| is_forwarded_header(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn is_forwarded_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    FORWARDED_HEADERS.contains(&lower.as_str()) || lower.ends_with(VERSION_HEADER_SUFFIX)
}

// ---------------------------------------------------------------------------
// Target address gate: which address classes the relay may talk to.
//
// Allowed: the loopback face and public unicast. Everything else is refused
// with a fixed reason label. Applied to IP literals and to every resolved
// address of a host name.
// ---------------------------------------------------------------------------

/// The target gate's verdict for one address: `Ok` for the loopback face and
/// public unicast, `Err(reason)` for every refused class.
fn gate_address(ip: std::net::IpAddr) -> Result<(), &'static str> {
    match ip {
        std::net::IpAddr::V4(v4) => gate_ipv4(v4),
        std::net::IpAddr::V6(v6) => gate_ipv6(v6),
    }
}

/// Address-class gate for one IPv4 address.
fn gate_ipv4(ip: std::net::Ipv4Addr) -> Result<(), &'static str> {
    // 127.0.0.0/8 is the deliberate loopback-face carve-out (see `gate_target`).
    if ip.is_loopback() {
        return Ok(());
    }
    let [a, b, c, _] = ip.octets();
    let denied = match (a, b, c) {
        (0, ..) => Some("unspecified"),
        (10, ..) => Some("private"),
        (100, 64..=127, _) => Some("shared"),
        (169, 254, _) => Some("link-local"),
        (172, 16..=31, _) => Some("private"),
        (192, 0, 0) => Some("reserved"),
        (192, 0, 2) => Some("documentation"),
        (192, 88, 99) => Some("reserved"),
        (192, 168, _) => Some("private"),
        (198, 18..=19, _) => Some("benchmarking"),
        (198, 51, 100) => Some("documentation"),
        (203, 0, 113) => Some("documentation"),
        (224..=239, ..) => Some("multicast"),
        (240.., ..) => Some("reserved"),
        _ => None,
    };
    denied.map_or(Ok(()), Err)
}

/// Address-class gate for one IPv6 address.
fn gate_ipv6(ip: std::net::Ipv6Addr) -> Result<(), &'static str> {
    // ::1 is the deliberate loopback-face carve-out (see `gate_target`).
    if ip.is_loopback() {
        return Ok(());
    }
    if ip.is_unspecified() {
        return Err("unspecified");
    }
    if ip.is_multicast() {
        return Err("multicast");
    }
    // IPv4-mapped (`::ffff:a.b.c.d`) and legacy IPv4-compatible (`::a.b.c.d`)
    // spellings gate as the IPv4 address they embed.
    if let Some(embedded) = embedded_ipv4(&ip) {
        return gate_ipv4(embedded);
    }
    let segments = ip.segments();
    let denied = if (segments[0] & 0xfe00) == 0xfc00 {
        Some("private") // fc00::/7 unique-local
    } else if (segments[0] & 0xffc0) == 0xfe80 {
        Some("link-local") // fe80::/10
    } else if segments[0] == 0x2001 && (segments[1] & 0xfe00) == 0 {
        Some("reserved") // 2001::/23 protocol assignments
    } else if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        Some("documentation") // 2001:db8::/32
    } else if segments[0] == 0x0100 && segments[1..4] == [0, 0, 0] {
        Some("reserved") // 100::/64 discard-only
    } else if (segments[0] & 0xe000) == 0x2000 {
        None // 2000::/3 global unicast
    } else {
        Some("non-global")
    };
    denied.map_or(Ok(()), Err)
}

/// The IPv4 address embedded in an IPv4-mapped (`::ffff:a.b.c.d`) or legacy
/// IPv4-compatible (`::a.b.c.d`) IPv6 address.
fn embedded_ipv4(ip: &std::net::Ipv6Addr) -> Option<std::net::Ipv4Addr> {
    let octets = ip.octets();
    let mapped = octets[..10] == [0u8; 10] && octets[10] == 0xff && octets[11] == 0xff;
    let compatible = octets[..12] == [0u8; 12];
    if mapped || compatible {
        Some(std::net::Ipv4Addr::new(
            octets[12], octets[13], octets[14], octets[15],
        ))
    } else {
        None
    }
}

/// Every resolved address of a host must pass `gate_address` (fail closed:
/// one refused address refuses the whole target).
fn gate_resolved_addrs(host: &str, addrs: &[std::net::SocketAddr]) -> Result<(), String> {
    for addr in addrs {
        if let Err(reason) = gate_address(addr.ip()) {
            return Err(format!(
                "target host '{host}' resolves to a refused address ({reason})"
            ));
        }
    }
    Ok(())
}

/// The host as the gate classifies it: the URL's host string with IPv6
/// brackets removed (`host_str()` keeps them — see `is_loopback_target`), so
/// bracketed literals reach the address-class gate instead of the resolver.
fn gate_host_str(target: &reqwest::Url) -> Option<String> {
    target.host_str().map(|host| {
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .to_string()
    })
}

/// Resolve (when needed) and gate the relay target; returns the addresses the
/// connection must use.
///
/// **Loopback carve-out (product design, deliberate):** loopback targets are
/// the local face — the local gateway and local model runtimes are first-class
/// relay targets (their base URLs are loopback), so 127.0.0.0/8 and ::1 pass.
/// Everything else non-public is refused. The returned addresses are the ones
/// the transport pins the connection to, so the checked addresses are the
/// connected addresses (no second resolution at connect time).
async fn gate_target(target: &reqwest::Url) -> Result<Vec<std::net::SocketAddr>, String> {
    let host =
        gate_host_str(target).ok_or_else(|| "provider_request: URL has no host".to_string())?;
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        gate_address(ip)
            .map_err(|reason| format!("provider_request: target address refused ({reason})"))?;
        return Ok(Vec::new());
    }
    let port = target.port_or_known_default().unwrap_or(0);
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| format!("provider_request: target host '{host}' did not resolve"))?
        .collect();
    if addrs.is_empty() {
        return Err(format!(
            "provider_request: target host '{host}' did not resolve"
        ));
    }
    gate_resolved_addrs(&host, &addrs).map_err(|detail| format!("provider_request: {detail}"))?;
    Ok(addrs)
}

/// Message prefix for a refused redirect hop (pin-able by tests and visible to
/// the webview as a distinct verdict).
fn redirect_refused(detail: String) -> String {
    format!("provider_request: redirect refused: {detail}")
}

/// Per-hop gate for a redirect target (synchronous face: the redirect policy
/// cannot await). Same address classes as `gate_target`, including a blocking
/// resolver pre-check for host names; see the module docs for the residual
/// (cross-host hops are checked here but connected via fresh DNS).
fn gate_redirect_target(target: &reqwest::Url) -> Result<(), String> {
    check_scheme_and_host(target).map_err(redirect_refused)?;
    let host =
        gate_host_str(target).ok_or_else(|| redirect_refused("URL has no host".to_string()))?;
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return gate_address(ip)
            .map_err(|reason| redirect_refused(format!("target address refused ({reason})")));
    }
    let port = target.port_or_known_default().unwrap_or(0);
    let addrs: Vec<std::net::SocketAddr> = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| redirect_refused(format!("target host '{host}' did not resolve")))?
        .collect();
    if addrs.is_empty() {
        return Err(redirect_refused(format!(
            "target host '{host}' did not resolve"
        )));
    }
    gate_resolved_addrs(&host, &addrs).map_err(redirect_refused)
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
/// never leave the process. "Configured" means a non-empty **forwardable**
/// credential header (the whitelist's credential faces): a credential that the
/// header filter strips does not make a call configured, because it can never
/// reach the far side. Loopback targets pass through untouched.
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
    /// Validated relay target (scheme/host + address-class gated).
    pub url: reqwest::Url,
    /// Validated relay verb.
    pub method: reqwest::Method,
    /// Whitelisted request headers to forward (see `filter_headers`);
    /// constructed by `relay_via` from the caller's map.
    pub headers: HashMap<String, String>,
    /// Optional request body.
    pub body: Option<String>,
    /// Addresses the target gate validated for this target: the transport
    /// pins the connection to them (empty for IP-literal hosts, which need no
    /// resolution).
    pub connect_addrs: Vec<std::net::SocketAddr>,
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

/// Redirect policy for the relay: bounded hops, and every hop re-passes the
/// target gate (scheme + address classes, with a blocking resolver pre-check
/// for host names). Gate verdicts travel as the error cause chain.
fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > MAX_REDIRECT_HOPS {
            return attempt.error(std::io::Error::other(redirect_refused(
                "too many redirects".to_string(),
            )));
        }
        match gate_redirect_target(attempt.url()) {
            Ok(()) => attempt.follow(),
            Err(detail) => attempt.error(std::io::Error::other(detail)),
        }
    })
}

/// Describe a transport error without echoing URLs: the top-level message
/// (`without_url`) plus its cause chain, so gate verdicts ride through (they
/// surface as causes of the redirect/request error).
fn transport_error_message(error: reqwest::Error) -> String {
    let error = error.without_url();
    let mut message = error.to_string();
    let mut cause = std::error::Error::source(&error);
    while let Some(source) = cause {
        message.push_str(": ");
        message.push_str(&source.to_string());
        cause = source.source();
    }
    message
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
            let pinned_host = request.url.host_str().map(str::to_string);
            let mut builder = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .redirect(redirect_policy());
            // DNS pinning: connect to exactly the addresses the target gate
            // checked, instead of re-resolving at connect time. (URL-embedded
            // ports always win over the pinned ports; only the IPs are pinned.)
            if !request.connect_addrs.is_empty() {
                if let Some(host) = pinned_host.as_deref() {
                    builder = builder.resolve_to_addrs(host, &request.connect_addrs);
                }
            }
            let client = builder
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
                format!(
                    "provider_request: request failed: {}",
                    transport_error_message(error)
                )
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
                    transport_error_message(error)
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

/// Relay one call over `transport`, applying validation and gates in order:
/// scheme/host → method → header whitelist → not-configured gate →
/// target address gate (DNS + classes) → transport. The whitelist runs before
/// the credential gate so "configured" is judged on the headers that can
/// actually reach the far side, and the not-configured gate runs before the
/// target gate so a credential-less outbound probe performs no network
/// activity at all — not even a DNS query.
async fn relay_via<T: RelayTransport>(
    transport: &T,
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<ProviderResponse, String> {
    let target = validate_url(url)?;
    let verb = validate_method(method)?;
    let headers = filter_headers(headers);
    if let Some(frame) = unconfigured_suppression(&target, &headers) {
        return Ok(frame);
    }
    let connect_addrs = gate_target(&target).await?;
    let response = transport
        .send(RelayRequest {
            url: target,
            method: verb,
            headers,
            body: body.map(str::to_string),
            connect_addrs,
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
        // 演进断言（B2 目标闸，理由）：relay_via 现在在出网前做地址/DNS 闸，
        // 假域名离线不可解析必然被闸下；改钉公网 IP 字面量（不出网，假传输面），
        // 被钉不变量「已配置呼叫原样过境」本身一字未改。
        let response = relay_via(
            &transport,
            "https://93.184.216.34/v1/models",
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
        assert_eq!(sent.url.as_str(), "https://93.184.216.34/v1/models");
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

    // -----------------------------------------------------------------------
    // SSRF 目标闸 (B2): 地址类闸 + 字面量变体 + 重定向闸 + 请求头白名单。
    // -----------------------------------------------------------------------

    /// One credential header, so the not-configured gate does not shadow the
    /// target gate in these assertions.
    fn configured_headers() -> HashMap<String, String> {
        HashMap::from([("Authorization".to_string(), "Bearer sk-test-1".to_string())])
    }

    /// The address the gate classifies for a URL's host (IPv6 brackets and
    /// IPv4-mapped / legacy IPv4-compatible spellings unwrapped), so variant
    /// spellings can be compared against their canonical form.
    fn host_ip(url: &str) -> std::net::IpAddr {
        let parsed = reqwest::Url::parse(url).expect("parseable test URL");
        let host = gate_host_str(&parsed).expect("host");
        let ip: std::net::IpAddr = host.parse().expect("host must be an IP literal here");
        match ip {
            std::net::IpAddr::V6(v6) => embedded_ipv4(&v6).map(std::net::IpAddr::V4).unwrap_or(ip),
            other => other,
        }
    }

    #[test]
    fn ip_literal_variants_normalize_into_their_address_class() {
        // 十进制整数 IP / 十六进制 / 短格式 / IPv4 映射 IPv6：URL 解析器已规范化，
        // 闸按规范化后的地址判类，不按字面拼写做字符串判断。
        for (variant, canonical) in [
            ("http://167772161/", "http://10.0.0.1/"),
            ("http://0xa000001/", "http://10.0.0.1/"),
            ("http://10.1/", "http://10.0.0.1/"),
            ("http://2852039166/", "http://169.254.169.254/"),
            ("http://[::ffff:10.0.0.1]/", "http://10.0.0.1/"),
            (
                "http://[::ffff:169.254.169.254]/",
                "http://169.254.169.254/",
            ),
            ("http://[::ffff:192.168.0.1]/", "http://192.168.0.1/"),
            ("http://2130706433/", "http://127.0.0.1/"),
            ("http://0x7f000001/", "http://127.0.0.1/"),
            ("http://127.1/", "http://127.0.0.1/"),
            ("http://[::ffff:127.0.0.1]/", "http://127.0.0.1/"),
        ] {
            assert_eq!(
                host_ip(variant),
                host_ip(canonical),
                "{variant} must normalize to the class of {canonical}"
            );
        }
    }

    #[tokio::test]
    async fn refuses_non_public_target_literals() {
        for url in [
            // private IPv4
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            // the same classes via decimal / hex / short spellings
            "http://167772161/",
            "http://0xa000001/",
            "http://10.1/",
            "http://2852039166/",
            // IPv4-mapped IPv6 spellings of refused IPv4 classes
            "http://[::ffff:10.0.0.1]/",
            "http://[::ffff:169.254.169.254]/",
            "http://[::ffff:192.168.0.1]/",
            "http://[::10.0.0.1]/",
            // link-local / unique-local
            "http://169.254.169.254/",
            "http://[fe80::1]/",
            "http://[fd00::1]/",
            "http://[fc00::1]/",
            // multicast / unspecified / broadcast
            "http://224.0.0.1/",
            "http://[ff02::1]/",
            "http://0.0.0.0/",
            "http://[::]/",
            "http://255.255.255.255/",
            // shared / benchmarking / documentation / protocol-assignment
            "http://100.64.0.1/",
            "http://198.18.0.1/",
            "http://192.0.2.1/",
            "http://198.51.100.1/",
            "http://203.0.113.1/",
            "http://[2001:db8::1]/",
            "http://[2001::1]/",
        ] {
            let transport = FakeTransport::new();
            let outcome = relay_via(&transport, url, "GET", &configured_headers(), None).await;
            let Err(message) = outcome else {
                panic!("{url} must be refused by the target gate");
            };
            assert!(
                message.contains("target address refused"),
                "{url} must be refused by the address-class gate: {message}"
            );
            assert_eq!(
                transport.call_count(),
                0,
                "{url} must never reach the transport"
            );
        }
    }

    #[tokio::test]
    async fn public_target_literals_relay() {
        for url in [
            "https://93.184.216.34/v1/models",
            "http://[2001:4860:4860::8888]/v1/models",
        ] {
            let transport = FakeTransport::new();
            let response = relay_via(&transport, url, "GET", &configured_headers(), None)
                .await
                .expect("public unicast targets relay");
            assert_eq!(transport.call_count(), 1, "{url} is public and must relay");
            assert_eq!(response.status, 200);
        }
    }

    #[tokio::test]
    async fn every_resolved_address_must_pass_the_gate() {
        // 混合应答：一个公网地址不能给私网地址买路（fail closed）。
        let mixed = [
            "1.2.3.4:443".parse::<std::net::SocketAddr>().expect("addr"),
            "10.0.0.1:443"
                .parse::<std::net::SocketAddr>()
                .expect("addr"),
        ];
        assert!(
            gate_resolved_addrs("mixed.test", &mixed).is_err(),
            "one refused address must refuse the whole target"
        );
        let public = ["1.2.3.4:443".parse::<std::net::SocketAddr>().expect("addr")];
        assert!(gate_resolved_addrs("public.test", &public).is_ok());
    }

    #[tokio::test]
    async fn localhost_name_is_the_local_face() {
        // DNS path pin: a name (not a literal) that resolves loopback passes
        // the gate as the local face. Offline-safe: resolved from the hosts
        // file, no network traffic.
        let target = validate_url("http://localhost:8080/v1/models").expect("valid URL");
        let addrs = gate_target(&target).await.expect("localhost resolves");
        assert!(!addrs.is_empty(), "resolution must yield addresses");
        assert!(
            addrs.iter().all(|addr| gate_address(addr.ip()).is_ok()),
            "every resolved address must be the local face"
        );
    }

    #[test]
    fn redirect_targets_re_pass_the_gate() {
        for url in [
            "http://10.0.0.1/secret",
            "http://169.254.169.254/latest/meta-data",
            "http://[::ffff:192.168.0.1]/x",
            "file:///etc/passwd",
            "http://[fd00::1]/x",
        ] {
            let target = reqwest::Url::parse(url).expect("parseable");
            assert!(
                gate_redirect_target(&target).is_err(),
                "{url} must be refused as a redirect hop"
            );
        }
        for url in [
            "http://127.0.0.1:9000/health",
            "https://93.184.216.34/v1/models",
        ] {
            let target = reqwest::Url::parse(url).expect("parseable");
            assert!(
                gate_redirect_target(&target).is_ok(),
                "{url} must pass as a redirect hop"
            );
        }
    }

    #[tokio::test]
    async fn request_headers_are_whitelisted_before_the_wire() {
        let transport = FakeTransport::new();
        let mut headers = configured_headers();
        headers.insert("x-api-key".to_string(), "sk-x-1".to_string());
        headers.insert("api-version".to_string(), "2024-01-01".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        // stripped faces: routing / session / framing / forwarding / browser
        headers.insert("Host".to_string(), "attacker.test".to_string());
        headers.insert("Cookie".to_string(), "session=1".to_string());
        headers.insert("Content-Length".to_string(), "0".to_string());
        headers.insert("Connection".to_string(), "keep-alive".to_string());
        headers.insert("X-Forwarded-For".to_string(), "1.2.3.4".to_string());
        headers.insert("user-agent".to_string(), "probe".to_string());

        relay_via(
            &transport,
            "https://93.184.216.34/v1/models",
            "GET",
            &headers,
            None,
        )
        .await
        .expect("configured call relays");
        let sent = transport.last_call();
        for kept in [
            "Authorization",
            "x-api-key",
            "api-version",
            "content-type",
            "Accept",
        ] {
            assert!(sent.headers.contains_key(kept), "{kept} must be forwarded");
        }
        for stripped in [
            "Host",
            "Cookie",
            "Content-Length",
            "Connection",
            "X-Forwarded-For",
            "user-agent",
        ] {
            assert!(
                !sent.headers.contains_key(stripped),
                "{stripped} must be stripped"
            );
        }
    }

    #[tokio::test]
    async fn stripped_credential_headers_do_not_configure_a_call() {
        // A credential that only rides on a stripped header can never reach the
        // far side, so the call stays "unconfigured" and no request leaves.
        let transport = FakeTransport::new();
        let headers = HashMap::from([("Cookie".to_string(), "session=1".to_string())]);
        let response = relay_via(
            &transport,
            "https://93.184.216.34/v1/models",
            "GET",
            &headers,
            None,
        )
        .await
        .expect("the gate answers locally");
        assert_eq!(
            transport.call_count(),
            0,
            "a stripped credential must not produce an outbound call"
        );
        assert!(is_provider_not_configured_frame(&response));
    }

    /// Local one-shot responder: answers its first connection with `response`.
    fn spawn_responder(response: String) -> (u16, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind test responder");
        let port = listener.local_addr().expect("test responder addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut buf);
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            }
        });
        (port, handle)
    }

    #[tokio::test]
    async fn redirect_to_an_internal_target_is_refused() {
        let (port, handle) = spawn_responder(
            "HTTP/1.1 302 Found\r\nLocation: http://10.0.0.1/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string(),
        );
        let outcome = send_provider_request(
            &format!("http://127.0.0.1:{port}/start"),
            "GET",
            &HashMap::new(),
            None,
        )
        .await;
        let _ = handle.join();
        let message = outcome.expect_err("the redirect hop must be refused");
        assert!(
            message.contains("redirect refused"),
            "the gate verdict must surface: {message}"
        );
    }

    #[tokio::test]
    async fn redirect_to_a_permitted_target_is_followed() {
        let (target_port, target_handle) = spawn_responder(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\nConnection: close\r\n\r\nsecond-hop-ok"
                .to_string(),
        );
        let (entry_port, entry_handle) = spawn_responder(format!(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{target_port}/models\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ));
        let response = send_provider_request(
            &format!("http://127.0.0.1:{entry_port}/start"),
            "GET",
            &HashMap::new(),
            None,
        )
        .await
        .expect("a permitted redirect hop relays");
        let _ = entry_handle.join();
        let _ = target_handle.join();
        assert_eq!(response.status, 200);
        assert_eq!(response.body_text, "second-hop-ok");
    }

    #[tokio::test]
    async fn redirect_chains_are_bounded() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loop responder");
        let port = listener.local_addr().expect("loop responder addr").port();
        // Self-redirect loop; accepts are bounded so the detached thread
        // cannot outlive the test binary by more than one stuck accept.
        std::thread::spawn(move || {
            for _ in 0..32 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut buf = [0u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut buf);
                let response = format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            }
        });
        let outcome = send_provider_request(
            &format!("http://127.0.0.1:{port}/loop"),
            "GET",
            &HashMap::new(),
            None,
        )
        .await;
        let message = outcome.expect_err("the redirect chain must be cut");
        assert!(
            message.contains("too many redirects"),
            "the hop cap must surface: {message}"
        );
    }
}
