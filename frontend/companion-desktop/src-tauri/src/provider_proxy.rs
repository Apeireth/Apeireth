//! Provider HTTP relay for the desktop webview (zero-egress hardening).
//!
//! The webview must not reach provider APIs directly: the window CSP pins
//! `connect-src` to loopback, and every provider call is relayed through the
//! `provider_request` command so the request leaves from the Rust side.
//!
//! Security boundaries enforced here (each one fails closed):
//! - only http/https URLs are forwarded (any other scheme is rejected)
//! - target approval (`RelayPolicy`): a relay target must be an **approved
//!   endpoint** — either a registered local endpoint
//!   (`LocalEndpointRegistry`: the default gateway base, the first-run
//!   wizard's local-runtime base, and the live gateway endpoint) or a
//!   configured remote server (`ProviderAllowlist`: the provider base URLs the
//!   desktop pushes to the sidecar **plus** the user-managed allowed-server
//!   bases, `AllowedServersStore`). Unregistered loopback targets are refused;
//!   with no configured server, remote targets are refused outright.
//! - address-class gate (`gate_address`): every address the relay connects to
//!   must be the loopback face or public unicast — private, link-local, shared,
//!   multicast, unspecified, documentation, benchmarking and reserved ranges
//!   are refused, for IP literals (including decimal/hex/short spellings and
//!   IPv4-mapped IPv6, which the URL parser normalizes) and for **every**
//!   address a host name resolves to. A non-local-face name that resolves into
//!   the loopback class is refused too: loopback exposure is registry-only.
//! - DNS pinning (`GatedResolver`): each connection attempt resolves its host
//!   exactly once, gates every resolved address, and connects to exactly those
//!   addresses — never a second resolution. The resolver is client-wide, so
//!   this holds for the first hop and for **every redirect hop** alike.
//! - redirects are bounded by [`MAX_REDIRECT_HOPS`] and every hop re-passes
//!   scheme + address-class + target-approval checks at the reqwest
//!   redirect-policy layer
//! - only whitelisted request headers are forwarded (`filter_headers`):
//!   routing (`Host`), session (`Cookie`) and framing (`Content-Length`, ...)
//!   headers never leave the process
//! - the whole request (send + body read) is bounded by [`REQUEST_TIMEOUT`]
//! - the relayed response body is capped at [`MAX_RESPONSE_BYTES`]
//! - credential header values never appear in logs or error strings
//!   ([`sanitize_error`], layered on the [`DesktopLogger`] redaction passes)
//!
//! Residual risks (documented, not solved in this module):
//! - approval is endpoint-scoped, not user-intent-scoped: anything reachable
//!   at an approved (host, port) is relayable, and the Settings "test" button
//!   for a *not yet saved* remote provider host is refused until the config is
//!   saved (the allowlist is built from the applied provider config; the user
//!   allowed-server face is the explicit way to add more).
//! - the local registry carries two built-in faces (the default gateway base
//!   and the wizard's local-runtime base) plus the live gateway endpoint and
//!   configured local provider bases; another local service squatting one of
//!   those ports is indistinguishable from the registered face at the relay
//!   level.

use crate::logging::DesktopLogger;
use apeireth_core::stored_doc::DocCompat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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
/// (`file:`, `ftp:`, `javascript:`, `data:`, ...) is refused. The URL-level
/// target gate (`gate_target_url`) runs on top of this in `relay_via`; this
/// function only pins scheme and host presence.
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
    // 127.0.0.0/8 is the loopback *class*: necessary but not sufficient —
    // target approval (`RelayPolicy`) is what actually opens a local endpoint.
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
    // ::1 is the loopback *class*: necessary but not sufficient — target
    // approval (`RelayPolicy`) is what actually opens a local endpoint.
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
/// one refused address refuses the whole target). Additional registration
/// rule: a non-loopback-face name must not resolve **into** the loopback class
/// — loopback exposure goes through [`LocalEndpointRegistry`] only, so a name
/// that points at the local face is refused unless the name itself is the
/// local face ("localhost" / loopback literal).
fn gate_resolved_addrs(host: &str, addrs: &[std::net::SocketAddr]) -> Result<(), String> {
    for addr in addrs {
        if let Err(reason) = gate_address(addr.ip()) {
            return Err(format!(
                "target host '{host}' resolves to a refused address ({reason})"
            ));
        }
        if addr.ip().is_loopback() && !is_loopback_face_host(host) {
            return Err(format!(
                "target host '{host}' resolves to the local face without a registered local endpoint"
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

// ---------------------------------------------------------------------------
// Target approval: which endpoints the relay may talk to at all.
//
// Two explicit faces feed one approval set, matched on (host, port):
// - `LocalEndpointRegistry` — the product's local endpoints (default gateway
//   base, first-run wizard's local-runtime base, live gateway endpoint).
// - `ProviderAllowlist` — provider base URLs from the applied provider config.
// Nothing else is approved; with an empty allowlist, remote targets are
// refused outright (fail closed).
// ---------------------------------------------------------------------------

/// One approved endpoint: host (lowercase, IPv6 brackets removed) + port.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ApprovedEndpoint {
    host: String,
    port: u16,
}

/// Whether `host` names the loopback face ("localhost" or a loopback IP).
fn is_loopback_face_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// Endpoint match: exact (host, port), with loopback aliases accepted on the
/// same port — a registered `localhost:8080` face also serves `127.0.0.1:8080`
/// and `[::1]:8080`. Non-loopback entries match on exact host.
fn endpoint_matches(entry: &ApprovedEndpoint, host: &str, port: u16) -> bool {
    if entry.port != port {
        return false;
    }
    entry.host == host || (is_loopback_face_host(&entry.host) && is_loopback_face_host(host))
}

/// Whether any approved endpoint covers the (host, port) target.
fn approved_endpoints_contain(entries: &[ApprovedEndpoint], host: &str, port: u16) -> bool {
    entries
        .iter()
        .any(|entry| endpoint_matches(entry, host, port))
}

/// (host, port) of a configured base: normalized host + effective port.
///
/// Accepts a full `http(s)://host[:port][/…]` base URL (the configured provider
/// faces) and the bare `host:port` form (the user allowlist face). Approval is
/// (host, port) scoped; the path and the scheme beyond http/https play no part.
fn approved_endpoint_from_base_url(base_url: &str) -> Option<ApprovedEndpoint> {
    let trimmed = base_url.trim();
    if let Ok(parsed) = reqwest::Url::parse(trimmed) {
        if matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some() {
            let host = gate_host_str(&parsed)?;
            let port = parsed.port_or_known_default()?;
            return Some(ApprovedEndpoint {
                host: host.to_ascii_lowercase(),
                port,
            });
        }
    }
    // Bare `host:port` form: no scheme and no path — host + port only. (Any
    // scheme-bearing URL that is not http/https was already excluded above.)
    if trimmed.contains("://") || trimmed.contains('/') {
        return None;
    }
    let (host, port) = trimmed.rsplit_once(':')?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return None;
    }
    let port = port.trim().parse::<u16>().ok()?;
    Some(ApprovedEndpoint {
        host: host.to_ascii_lowercase(),
        port,
    })
}

/// Default gateway base (the frontend's initial gateway URL): the gateway face
/// before the supervisor reports a live endpoint, and the legacy fixed-port
/// face the frontend heuristics recognize.
pub const DEFAULT_GATEWAY_BASE: &str = "http://127.0.0.1:8080";

/// First-run wizard's local-runtime base (the local provider preset in the
/// wizard and in Settings).
pub const WIZARD_LOCAL_RUNTIME_BASE: &str = "http://localhost:11434/v1";

/// Explicit registration face for the relay's local (loopback) endpoints.
///
/// The relay's product-local targets are exactly: the default gateway base,
/// the wizard's local-runtime base, and the live gateway endpoint the
/// supervisor reports. Any other loopback target is refused — this registry is
/// what replaced the old "any loopback target passes" carve-out.
#[derive(Debug, Clone, Default)]
pub struct LocalEndpointRegistry {
    entries: Vec<ApprovedEndpoint>,
}

impl LocalEndpointRegistry {
    /// The built-in local faces: default gateway base + wizard local-runtime
    /// base.
    pub fn builtin() -> Self {
        let mut registry = Self::default();
        registry.register_base_url(DEFAULT_GATEWAY_BASE);
        registry.register_base_url(WIZARD_LOCAL_RUNTIME_BASE);
        registry
    }

    /// Register the local endpoint behind a base URL (host + effective port).
    /// Unparseable URLs and non-loopback bases are ignored here: remote
    /// approval belongs to [`ProviderAllowlist`].
    pub fn register_base_url(&mut self, base_url: &str) {
        if let Some(entry) = approved_endpoint_from_base_url(base_url) {
            if is_loopback_face_host(&entry.host) && !self.entries.contains(&entry) {
                self.entries.push(entry);
            }
        }
    }

    fn allows(&self, host: &str, port: u16) -> bool {
        approved_endpoints_contain(&self.entries, host, port)
    }
}

/// Provider-host allowlist: (host, port) pairs of **remote** configured
/// servers — the applied provider base URLs plus the user-configured allowed
/// servers (the explicit user face). Empty = no remote target is approved
/// (fail closed). Loopback bases never land here: local endpoints belong to
/// [`LocalEndpointRegistry`] alone (the user allowlist governs remote servers
/// only), and entries are skipped here as defense in depth.
#[derive(Debug, Clone, Default)]
pub struct ProviderAllowlist {
    entries: Vec<ApprovedEndpoint>,
}

impl ProviderAllowlist {
    /// Add one configured base (host + effective port). Loopback bases are
    /// skipped: they could only ever approve a local endpoint, and that is the
    /// registry's job (see [`LocalEndpointRegistry`]).
    pub fn add_base_url(&mut self, base_url: &str) {
        if let Some(entry) = approved_endpoint_from_base_url(base_url) {
            if is_loopback_face_host(&entry.host) {
                return;
            }
            if !self.entries.contains(&entry) {
                self.entries.push(entry);
            }
        }
    }

    fn allows(&self, host: &str, port: u16) -> bool {
        approved_endpoints_contain(&self.entries, host, port)
    }
}

/// The relay's target-approval policy: registered local endpoints plus the
/// configured provider hosts. Built per `provider_request` call from the
/// supervisor's live gateway endpoint and applied provider env; cloned into
/// the transport so redirect hops re-check against the same policy.
#[derive(Debug, Clone, Default)]
pub struct RelayPolicy {
    /// Registered local endpoints (gateway + wizard faces).
    pub local: LocalEndpointRegistry,
    /// Configured provider hosts (empty = no remote target approved).
    pub providers: ProviderAllowlist,
}

impl RelayPolicy {
    /// Fold one configured provider base into the policy: a loopback base
    /// registers the local endpoint (the registry is the sole authority for
    /// loopback), a remote base extends the provider allowlist. Unparseable
    /// bases approve nothing (fail closed).
    pub fn add_configured_base(&mut self, base_url: &str) {
        let Some(entry) = approved_endpoint_from_base_url(base_url) else {
            return;
        };
        if is_loopback_face_host(&entry.host) {
            self.local.register_base_url(base_url);
        } else {
            self.providers.add_base_url(base_url);
        }
    }

    /// Approve one target: a covering (host, port) in either face passes;
    /// everything else is refused (fail closed) with the face named, so the
    /// webview can tell "save your provider config first" from "wrong port".
    pub fn approve_target(&self, host: &str, port: u16) -> Result<(), String> {
        if self.local.allows(host, port) || self.providers.allows(host, port) {
            return Ok(());
        }
        if is_loopback_face_host(host) {
            Err(format!(
                "provider_request: local endpoint '{host}:{port}' is not registered"
            ))
        } else {
            Err(format!(
                "provider_request: target host '{host}:{port}' is not in the provider allowlist"
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// User allowlist face: the allowed-server bases the user manages explicitly
// (advanced setups with their own relay / service). Persisted through the
// standard stored-doc envelope; the entries feed `ProviderAllowlist` only —
// remote servers, never the local endpoint registry.
// ---------------------------------------------------------------------------

/// Stored-doc identity: the relay's user-configured allowed servers.
pub const ALLOWED_SERVERS_DOC_NAME: &str = "companion-relay-allowed-servers";
/// Allowed-servers stored format version.
pub const ALLOWED_SERVERS_DOC_VERSION: u32 = 1;

fn allowed_servers_doc_compat() -> DocCompat {
    DocCompat::exact(ALLOWED_SERVERS_DOC_NAME, ALLOWED_SERVERS_DOC_VERSION)
}

/// Persisted body of the user allowlist: one entry per allowed server base,
/// exactly as validated on add (see [`validate_allowed_server_base`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AllowedServersDoc {
    /// Allowed server bases (`http(s)://…` or `host:port`), in add order.
    pub servers: Vec<String>,
}

/// Validate one user-configured allowed-server base. Accepted: an `http(s)`
/// base URL with a host, or bare `host:port`. Refused (with the reason):
/// empty/malformed input; other schemes; loopback bases (local endpoints belong
/// to [`LocalEndpointRegistry`] — the user allowlist governs remote servers
/// only); and IP literals in a refused address class (private / link-local /
/// …) — those could never relay, so accepting them would be a fake knob.
pub fn validate_allowed_server_base(base: &str) -> Result<String, String> {
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return Err("allowed server base is empty".to_string());
    }
    let Some(entry) = approved_endpoint_from_base_url(trimmed) else {
        return Err(format!(
            "allowed server base '{trimmed}' is not an http(s) base URL or host:port"
        ));
    };
    if is_loopback_face_host(&entry.host) {
        return Err(format!(
            "allowed server base '{trimmed}' is a local endpoint; local endpoints belong to the local endpoint registry, the allowlist governs remote servers only"
        ));
    }
    if let Ok(ip) = entry.host.parse::<std::net::IpAddr>() {
        gate_address(ip).map_err(|reason| {
            format!("allowed server base '{trimmed}' is a refused address class ({reason})")
        })?;
    }
    Ok(trimmed.to_string())
}

/// The user allowlist's storage face: add / remove / list with strict-open
/// persistence. A malformed / foreign / version-mismatched stored file is
/// **refused** on load (never read-as-default); a missing file is an empty
/// list.
pub struct AllowedServersStore {
    path: PathBuf,
    servers: std::sync::RwLock<Vec<String>>,
}

impl AllowedServersStore {
    /// App-data file for the user allowlist (same directory as the other
    /// desktop config envelopes).
    pub fn allowed_servers_path(logger: &DesktopLogger) -> PathBuf {
        logger
            .log_directory()
            .parent()
            .map(|dir| dir.join("relay-allowed-servers.json"))
            .unwrap_or_else(|| logger.log_directory().join("relay-allowed-servers.json"))
    }

    /// Open (and strictly validate) the store at `path`.
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let loaded: Option<AllowedServersDoc> =
            crate::stored_config::load_config(&path, &allowed_servers_doc_compat())?;
        Ok(Self {
            path,
            servers: std::sync::RwLock::new(loaded.unwrap_or_default().servers),
        })
    }

    /// Open the store at the app-data location.
    pub fn load_from_logger(logger: &DesktopLogger) -> Result<Self, String> {
        Self::load(Self::allowed_servers_path(logger))
    }

    /// The allowed bases, in add order.
    pub fn list(&self) -> Vec<String> {
        self.servers
            .read()
            .map(|servers| servers.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    /// Validate, add (idempotent) and persist one base; returns the new list.
    /// Mutation and persistence happen under one write lock, so concurrent
    /// add/remove calls cannot interleave into a stale file.
    pub fn add(&self, base: &str) -> Result<Vec<String>, String> {
        let validated = validate_allowed_server_base(base)?;
        let mut servers = self
            .servers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !servers.iter().any(|entry| entry == &validated) {
            servers.push(validated);
        }
        self.persist_list(&servers)?;
        Ok(servers.clone())
    }

    /// Remove one base (exact match) and persist; returns the new list.
    /// Removing an absent base is a no-op (idempotent).
    pub fn remove(&self, base: &str) -> Result<Vec<String>, String> {
        let mut servers = self
            .servers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        servers.retain(|entry| entry != base.trim());
        self.persist_list(&servers)?;
        Ok(servers.clone())
    }

    /// Persist one list through the stored-doc envelope (atomic write).
    fn persist_list(&self, servers: &[String]) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs_err::create_dir_all(parent)
                .map_err(|error| format!("allowed servers persist failed: {error}"))?;
        }
        let doc = AllowedServersDoc {
            servers: servers.to_vec(),
        };
        crate::stored_config::save_config(&self.path, &allowed_servers_doc_compat(), &doc)
    }
}

/// URL-level target gate: address classes for IP literals, then target
/// approval for every target. Host names are resolved (gated + pinned) at
/// connect time by `GatedResolver`, once per connection attempt.
fn gate_target_url(target: &reqwest::Url, policy: &RelayPolicy) -> Result<(), String> {
    let host =
        gate_host_str(target).ok_or_else(|| "provider_request: URL has no host".to_string())?;
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        gate_address(ip)
            .map_err(|reason| format!("provider_request: target address refused ({reason})"))?;
    }
    let port = target.port_or_known_default().unwrap_or(0);
    policy.approve_target(&host.to_ascii_lowercase(), port)
}

/// Message prefix for a refused redirect hop (pin-able by tests and visible to
/// the webview as a distinct verdict).
fn redirect_refused(detail: String) -> String {
    format!("provider_request: redirect refused: {detail}")
}

/// Per-hop gate for a redirect target (synchronous face: the redirect policy
/// cannot await). Scheme + literal address classes + the same target approval
/// as the entry gate. Name hops are gated **and pinned** at connect time by
/// `GatedResolver`, so no blocking resolve happens here.
fn gate_redirect_target(target: &reqwest::Url, policy: &RelayPolicy) -> Result<(), String> {
    check_scheme_and_host(target).map_err(redirect_refused)?;
    gate_target_url(target, policy).map_err(redirect_refused)
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
    /// Validated relay target (scheme/host + address-class gated + approved).
    pub url: reqwest::Url,
    /// Validated relay verb.
    pub method: reqwest::Method,
    /// Whitelisted request headers to forward (see `filter_headers`);
    /// constructed by `relay_via` from the caller's map.
    pub headers: HashMap<String, String>,
    /// Optional request body.
    pub body: Option<String>,
    /// Approval policy the transport re-applies to every redirect hop (and
    /// whose address-class face backs the gated resolver).
    pub policy: RelayPolicy,
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

/// Connect-time DNS face: resolve one host **once**, gate every resolved
/// address, and hand back exactly those addresses — the connector uses them
/// as-is, so the checked addresses are the connected addresses (no second
/// resolution). Installed client-wide, so this holds for the first hop and for
/// **every redirect hop** alike (per-hop DNS pinning).
///
/// IP-literal hosts never reach the resolver; they are gated at the URL level
/// (`gate_target_url` / the redirect policy).
#[derive(Debug)]
struct GatedResolver;

impl reqwest::dns::Resolve for GatedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(resolve_and_gate(host))
    }
}

/// Resolve `host` once, gate every resolved address (fail closed), and return
/// exactly the gated addresses as the only connection candidates. Ports are
/// returned as 0: the connector fills in the URL's port (URL ports always
/// win); only the addresses are pinned.
async fn resolve_and_gate(
    host: String,
) -> Result<reqwest::dns::Addrs, Box<dyn std::error::Error + Send + Sync>> {
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
        .await
        .map_err(|error| {
            Box::new(std::io::Error::other(format!(
                "provider_request: target host '{host}' did not resolve: {error}"
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?
        .collect();
    if addrs.is_empty() {
        return Err(Box::new(std::io::Error::other(format!(
            "provider_request: target host '{host}' did not resolve"
        ))));
    }
    gate_resolved_addrs(&host, &addrs).map_err(|detail| {
        Box::new(std::io::Error::other(format!("provider_request: {detail}")))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    Ok(Box::new(
        addrs
            .into_iter()
            .map(|addr| std::net::SocketAddr::new(addr.ip(), 0)),
    ) as reqwest::dns::Addrs)
}

/// Redirect policy for the relay: bounded hops, and every hop re-passes the
/// URL-level gate (scheme + literal address classes + target approval). Name
/// hops are gated and pinned at connect time by [`GatedResolver`], so no
/// blocking resolve happens in this synchronous face. Gate verdicts travel as
/// the error cause chain.
fn redirect_policy(policy: &RelayPolicy) -> reqwest::redirect::Policy {
    let policy = policy.clone();
    reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() > MAX_REDIRECT_HOPS {
            return attempt.error(std::io::Error::other(redirect_refused(
                "too many redirects".to_string(),
            )));
        }
        match gate_redirect_target(attempt.url(), &policy) {
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
            let policy = request.policy.clone();
            let client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .redirect(redirect_policy(&policy))
                // Per-hop DNS pinning: every connection attempt (first hop and
                // each redirect hop) resolves once through the gated resolver
                // and connects to exactly the gated addresses.
                .dns_resolver(std::sync::Arc::new(GatedResolver))
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
    policy: &RelayPolicy,
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<ProviderResponse, String> {
    let secrets = collect_secret_values(headers);
    relay_via(&ReqwestTransport, policy, url, method, headers, body)
        .await
        .map_err(|message| sanitize_error(&message, &secrets))
}

/// Relay one call over `transport`, applying validation and gates in order:
/// scheme/host → method → header whitelist → not-configured gate →
/// URL-level target gate (literal address classes + target approval) →
/// transport (name targets are gated + pinned at connect time by
/// `GatedResolver`). The whitelist runs before the credential gate so
/// "configured" is judged on the headers that can actually reach the far side,
/// and the not-configured gate runs before the target gate so a credential-less
/// outbound probe performs no network activity at all — not even a DNS query.
async fn relay_via<T: RelayTransport>(
    transport: &T,
    policy: &RelayPolicy,
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
    gate_target_url(&target, policy)?;
    let response = transport
        .send(RelayRequest {
            url: target,
            method: verb,
            headers,
            body: body.map(str::to_string),
            policy: policy.clone(),
        })
        .await?;
    Ok(ProviderResponse {
        status: response.status,
        body_text: response.body_text,
        content_type: response.content_type,
    })
}

/// Build the per-call approval policy from the live faces: the supervisor's
/// gateway endpoint (registered local endpoint) and applied provider env
/// `*_URL` base URLs (classified into registry / allowlist by
/// [`RelayPolicy::add_configured_base`]), plus the user-managed allowed-server
/// bases (remote only). Key material never enters the policy: only endpoint
/// URLs do.
async fn relay_policy_from_sidecar(
    supervisor: &crate::backend_supervisor::BackendSupervisor,
    user_allowlist: &AllowedServersStore,
) -> RelayPolicy {
    let mut policy = RelayPolicy {
        local: LocalEndpointRegistry::builtin(),
        providers: ProviderAllowlist::default(),
    };
    let info = supervisor.info().await;
    if let Some(endpoint) = info.endpoint.as_deref() {
        policy.local.register_base_url(endpoint);
    }
    let env = supervisor.provider_env_snapshot().await;
    for (name, value) in env.env_pairs() {
        if name.ends_with("_URL") {
            policy.add_configured_base(value);
        }
    }
    // User allowlist: remote servers only. Loopback entries cannot land here —
    // `validate_allowed_server_base` refuses them at the config face and
    // `ProviderAllowlist::add_base_url` skips them as defense in depth.
    for base in user_allowlist.list() {
        policy.providers.add_base_url(&base);
    }
    policy
}

/// Relay a provider HTTP request on behalf of the webview.
///
/// Arguments mirror the frontend contract: `{ url, method, headers, body }`;
/// the return value is `{ status, body_text, content_type }`. The log line
/// carries a fixed-field summary only — no header values, no body, no URL.
#[tauri::command]
pub async fn provider_request(
    logger: tauri::State<'_, std::sync::Arc<DesktopLogger>>,
    supervisor: tauri::State<'_, std::sync::Arc<crate::backend_supervisor::BackendSupervisor>>,
    allowed_servers: tauri::State<'_, std::sync::Arc<AllowedServersStore>>,
    url: String,
    method: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
) -> Result<ProviderResponse, String> {
    let headers = headers.unwrap_or_default();
    let policy = relay_policy_from_sidecar(&supervisor, &allowed_servers).await;
    let outcome = send_provider_request(&policy, &url, &method, &headers, body.as_deref()).await;

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

/// The user-managed allowed-server bases (UI face: manage list).
#[tauri::command]
pub fn list_allowed_servers(
    store: tauri::State<'_, std::sync::Arc<AllowedServersStore>>,
) -> Vec<String> {
    store.list()
}

/// Add one allowed-server base (validated by [`validate_allowed_server_base`];
/// an invalid base is refused with the reason). Returns the new list.
#[tauri::command]
pub fn add_allowed_server(
    store: tauri::State<'_, std::sync::Arc<AllowedServersStore>>,
    base: String,
) -> Result<Vec<String>, String> {
    store.add(&base)
}

/// Remove one allowed-server base (exact match; idempotent). Returns the new
/// list.
#[tauri::command]
pub fn remove_allowed_server(
    store: tauri::State<'_, std::sync::Arc<AllowedServersStore>>,
    base: String,
) -> Result<Vec<String>, String> {
    store.remove(&base)
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
            &test_policy(),
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
            &test_policy(),
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
        // 演进断言（B2-REG，理由）：目标还须命中 provider allowlist，故测试策略
        // 把该固定公网地址注册为「已配置提供方基址」（等价于已应用的提供方配置）。
        let policy = policy_with_providers(&["https://93.184.216.34/v1"]);
        let response = relay_via(
            &transport,
            &policy,
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
        // 语义未变（产品断言：本地面无凭据也放行）；B2-REG 后「本地面」= 注册的
        // 本地端点——默认网关基址 8080 就是内置注册面之一，故断言原样保留。
        for url in [
            "http://127.0.0.1:8080/v1/models",
            "http://[::1]:8080/v1/models",
            "http://localhost:8080/health",
        ] {
            let transport = FakeTransport::new();
            let response = relay_via(
                &transport,
                &test_policy(),
                url,
                "GET",
                &HashMap::new(),
                None,
            )
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

    // -----------------------------------------------------------------------
    // B2-REG: 本地端点注册面 + 提供方 allowlist (+ 逐跳钉址的 resolver 面)。
    // -----------------------------------------------------------------------

    /// Approval policy for tests: built-in local faces, no provider hosts.
    fn test_policy() -> RelayPolicy {
        RelayPolicy {
            local: LocalEndpointRegistry::builtin(),
            providers: ProviderAllowlist::default(),
        }
    }

    /// Policy with the given configured bases folded in exactly like the
    /// production merge (`add_configured_base`: loopback → registry, remote →
    /// allowlist).
    fn policy_with_providers(bases: &[&str]) -> RelayPolicy {
        let mut policy = test_policy();
        for base in bases {
            policy.add_configured_base(base);
        }
        policy
    }

    /// Unique per-test config file path under the redirected temp dir.
    fn test_config_path(stem: &str) -> PathBuf {
        std::env::temp_dir().join(format!("provider-proxy-{stem}-{}.json", std::process::id()))
    }

    /// Policy with one extra registered local endpoint: test responders play
    /// the registered local face on their ephemeral ports.
    fn policy_with_local(port: u16) -> RelayPolicy {
        let mut policy = test_policy();
        policy
            .local
            .register_base_url(&format!("http://127.0.0.1:{port}"));
        policy
    }

    #[test]
    fn provider_allowlist_is_scoped_to_configured_base_urls() {
        let policy =
            policy_with_providers(&["https://api.example.test/v1", "http://127.0.0.1:8000/v1"]);
        assert!(policy.approve_target("api.example.test", 443).is_ok());
        assert!(
            policy.approve_target("api.example.test", 8443).is_err(),
            "an unconfigured port on a known host is not approved"
        );
        assert!(policy.approve_target("other.test", 443).is_err());
        assert!(
            policy.approve_target("127.0.0.1", 8000).is_ok(),
            "a configured local provider base is approved like any other provider"
        );
        assert!(
            policy.approve_target("127.0.0.1", 8001).is_err(),
            "only the configured port is approved"
        );
    }

    #[tokio::test]
    async fn unregistered_local_endpoints_are_refused() {
        // 对抗用例①：伪装回环未注册端点（本机任意服务端口）一律拒。
        for url in [
            "http://127.0.0.1:9999/",
            "http://localhost:6379/",
            "http://[::1]:22/",
            "http://127.0.0.1:11435/",
            "http://localhost:8081/",
        ] {
            let transport = FakeTransport::new();
            let outcome = relay_via(
                &transport,
                &test_policy(),
                url,
                "GET",
                &configured_headers(),
                None,
            )
            .await;
            let Err(message) = outcome else {
                panic!("{url} must be refused as an unregistered local endpoint");
            };
            assert!(
                message.contains("not registered"),
                "{url} must hit the registration gate: {message}"
            );
            assert_eq!(
                transport.call_count(),
                0,
                "{url} must never reach the transport"
            );
        }
    }

    #[tokio::test]
    async fn public_host_outside_allowlist_is_refused() {
        // 对抗用例②：allowlist 外公网主机一律拒；空 allowlist 时 fail-closed
        // （未配置提供方 = 远程目标全拒，只留注册的本地面）。
        for policy in [
            test_policy(),
            policy_with_providers(&["https://api.allowed.test/v1"]),
        ] {
            let transport = FakeTransport::new();
            let outcome = relay_via(
                &transport,
                &policy,
                "https://93.184.216.34/v1/models",
                "GET",
                &configured_headers(),
                None,
            )
            .await;
            let Err(message) = outcome else {
                panic!("a public host outside the allowlist must be refused");
            };
            assert!(
                message.contains("not in the provider allowlist"),
                "the allowlist verdict must surface: {message}"
            );
            assert_eq!(
                transport.call_count(),
                0,
                "an allowlist miss must never reach the transport"
            );
        }
    }

    #[tokio::test]
    async fn registered_gateway_endpoint_relays() {
        // 活网关端点（动态端点）注册后放行；同端口的回环别名同义；相邻端口仍拒。
        let mut policy = test_policy();
        policy.local.register_base_url("http://127.0.0.1:52719");
        for url in [
            "http://127.0.0.1:52719/v1/models",
            "http://localhost:52719/health",
            "http://[::1]:52719/v1/models",
        ] {
            let transport = FakeTransport::new();
            let response = relay_via(&transport, &policy, url, "GET", &HashMap::new(), None)
                .await
                .expect("the registered gateway face relays");
            assert_eq!(transport.call_count(), 1, "{url} is registered");
            assert_eq!(response.status, 200);
        }
        let transport = FakeTransport::new();
        let outcome = relay_via(
            &transport,
            &policy,
            "http://127.0.0.1:52720/v1/models",
            "GET",
            &HashMap::new(),
            None,
        )
        .await;
        assert!(
            outcome.is_err(),
            "a neighbouring unregistered port must be refused"
        );
        assert_eq!(transport.call_count(), 0);
    }

    #[tokio::test]
    async fn configured_local_provider_base_relays() {
        // 已配置本地提供方（自定端口）经 allowlist 放行——注册面之外的合法本地
        // 用法只能来自已应用的提供方配置，且按端口收口。
        let policy = policy_with_providers(&["http://127.0.0.1:8000/v1"]);
        let transport = FakeTransport::new();
        let response = relay_via(
            &transport,
            &policy,
            "http://127.0.0.1:8000/v1/models",
            "GET",
            &configured_headers(),
            None,
        )
        .await
        .expect("the configured local provider base relays");
        assert_eq!(transport.call_count(), 1);
        assert_eq!(response.status, 200);

        let transport = FakeTransport::new();
        let outcome = relay_via(
            &transport,
            &policy,
            "http://127.0.0.1:8001/v1/models",
            "GET",
            &configured_headers(),
            None,
        )
        .await;
        assert!(outcome.is_err(), "the next port is not the configured base");
        assert_eq!(transport.call_count(), 0);
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
            let outcome = relay_via(
                &transport,
                &test_policy(),
                url,
                "GET",
                &configured_headers(),
                None,
            )
            .await;
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
    async fn allowlisted_public_target_literals_relay() {
        // 演进断言（B2-REG，理由）：公网目标从「类闸放行」收窄为「allowlist 命中才
        // 转发」，故本测试给两个固定公网地址登记提供方基址后钉「公网目标放行」；
        // 反向断言见 `public_host_outside_allowlist_is_refused`。
        let policy = policy_with_providers(&[
            "https://93.184.216.34/v1",
            "http://[2001:4860:4860::8888]/v1",
        ]);
        for url in [
            "https://93.184.216.34/v1/models",
            "http://[2001:4860:4860::8888]/v1/models",
        ] {
            let transport = FakeTransport::new();
            let response = relay_via(&transport, &policy, url, "GET", &configured_headers(), None)
                .await
                .expect("allowlisted public unicast targets relay");
            assert_eq!(
                transport.call_count(),
                1,
                "{url} is public and allowlisted and must relay"
            );
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
    async fn resolver_gates_and_pins_resolved_addresses() {
        // 演进断言（B2-REG，理由）：名字目标的解析从出网前预检移到连接期的
        // `resolve_and_gate`（逐跳钉址的机制面），故「名字解析到本地面」改钉在
        // resolver 层。离线安全：localhost 走 hosts 文件，不出网。
        let addrs = resolve_and_gate("localhost".to_string())
            .await
            .expect("localhost resolves from the hosts file")
            .collect::<Vec<_>>();
        assert!(!addrs.is_empty(), "resolution must yield addresses");
        assert!(
            addrs.iter().all(|addr| gate_address(addr.ip()).is_ok()),
            "every resolved address must be the local face"
        );
        assert!(
            addrs.iter().all(|addr| addr.port() == 0),
            "ports come from the URL; only addresses are pinned"
        );
    }

    #[tokio::test]
    async fn resolver_fails_closed_on_unresolvable_names() {
        let outcome = resolve_and_gate("host.invalid".to_string()).await;
        assert!(
            outcome.is_err(),
            "a name that does not resolve must be refused, not defaulted"
        );
    }

    #[test]
    fn redirect_targets_re_pass_the_gate() {
        // 演进断言（B2-REG，理由）：每一跳除类闸外还过目标审批，故放行集收窄为
        // 「已注册本地面 + allowlist 命中」；未注册回环跳的反向断言新增于此。
        let policy = policy_with_providers(&["https://93.184.216.34/v1"]);
        for url in [
            "http://10.0.0.1/secret",
            "http://169.254.169.254/latest/meta-data",
            "http://[::ffff:192.168.0.1]/x",
            "file:///etc/passwd",
            "http://[fd00::1]/x",
            "http://127.0.0.1:9000/health",
            "https://93.184.216.35/v1/models",
        ] {
            let target = reqwest::Url::parse(url).expect("parseable");
            assert!(
                gate_redirect_target(&target, &policy).is_err(),
                "{url} must be refused as a redirect hop"
            );
        }
        for url in [
            "http://127.0.0.1:8080/health",
            "https://93.184.216.34/v1/models",
        ] {
            let target = reqwest::Url::parse(url).expect("parseable");
            assert!(
                gate_redirect_target(&target, &policy).is_ok(),
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
            &policy_with_providers(&["https://93.184.216.34/v1"]),
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
            &policy_with_providers(&["https://93.184.216.34/v1"]),
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
        // The responder plays the registered local face on its ephemeral port.
        let outcome = send_provider_request(
            &policy_with_local(port),
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
    async fn redirect_to_unregistered_local_endpoint_is_refused() {
        // 对抗用例③：重定向跳到未注册回环端点（伪装本机服务）必须被拒。
        let (target_port, _target_listener) = {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("bind unregistered endpoint");
            (listener.local_addr().expect("addr").port(), listener)
        };
        let (port, handle) = spawn_responder(format!(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{target_port}/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ));
        let outcome = send_provider_request(
            &policy_with_local(port),
            &format!("http://127.0.0.1:{port}/start"),
            "GET",
            &HashMap::new(),
            None,
        )
        .await;
        let _ = handle.join();
        let message = outcome.expect_err("the hop must be refused");
        assert!(
            message.contains("redirect refused") && message.contains("not registered"),
            "the registration verdict must surface: {message}"
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
        // Both responders are registered local faces (the hop must be approved
        // as well as class-clean).
        let mut policy = policy_with_local(entry_port);
        policy
            .local
            .register_base_url(&format!("http://127.0.0.1:{target_port}"));
        let response = send_provider_request(
            &policy,
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
            &policy_with_local(port),
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

    #[tokio::test]
    async fn name_target_relays_through_gated_resolver() {
        // 端到端钉 resolver 接线：名字目标经 `resolve_and_gate` 解析+过闸+钉址后
        // 连接（首跳路径即逐跳路径的同一机制）。localhost 走 hosts 文件，不出网。
        let (port, handle) = spawn_responder(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 11\r\nConnection: close\r\n\r\nname-hop-ok"
                .to_string(),
        );
        let response = send_provider_request(
            &policy_with_local(port),
            &format!("http://localhost:{port}/models"),
            "GET",
            &HashMap::new(),
            None,
        )
        .await
        .expect("a registered local name target relays");
        let _ = handle.join();
        assert_eq!(response.status, 200);
        assert_eq!(response.body_text, "name-hop-ok");
    }

    // -----------------------------------------------------------------------
    // B2-CFG: 用户可配置的允许服务器面（持久 + 策略合并 + 对抗）。
    // -----------------------------------------------------------------------

    #[test]
    fn allowed_servers_persist_round_trip() {
        let path = test_config_path("round-trip");
        let _ = fs_err::remove_file(&path);
        {
            let store =
                AllowedServersStore::load(path.clone()).expect("missing file = empty store");
            assert!(store.list().is_empty(), "no file yet means no entries");
            let added = store
                .add("https://relay.example.test:8443/base")
                .expect("valid base accepted");
            assert_eq!(
                added,
                vec!["https://relay.example.test:8443/base".to_string()]
            );
            store
                .add("relay2.example.test:9000")
                .expect("bare host:port accepted");
            store
                .add("https://relay.example.test:8443/base")
                .expect("adding twice is idempotent");
            assert_eq!(store.list().len(), 2, "idempotent add keeps one entry");
        }
        let reopened = AllowedServersStore::load(path.clone()).expect("stored file reopens");
        assert_eq!(
            reopened.list(),
            vec![
                "https://relay.example.test:8443/base".to_string(),
                "relay2.example.test:9000".to_string()
            ]
        );
        let updated = reopened
            .remove("relay2.example.test:9000")
            .expect("remove persists");
        assert_eq!(
            updated,
            vec!["https://relay.example.test:8443/base".to_string()]
        );
        let final_read = AllowedServersStore::load(path.clone()).expect("reopens after remove");
        assert_eq!(
            final_read.list(),
            vec!["https://relay.example.test:8443/base".to_string()]
        );
        let _ = fs_err::remove_file(&path);
    }

    #[test]
    fn corrupt_allowed_servers_file_is_refused() {
        // 坏档拒开（fail closed）：不回退默认、不读坏就用。
        let path = test_config_path("corrupt");
        fs_err::write(&path, b"{ this is not json").expect("write corrupt file");
        assert!(
            AllowedServersStore::load(path.clone()).is_err(),
            "a malformed stored file must be refused, never read-as-default"
        );
        let _ = fs_err::remove_file(&path);
    }

    #[test]
    fn invalid_allowed_server_bases_are_refused() {
        for base in [
            "",
            "   ",
            "not a url",
            "ftp://relay.example.test:21",
            "file:///etc/passwd",
            "http:///",
            // 回环基址：本地端点归注册面管，用户白名单只收远程
            "localhost:11434",
            "http://127.0.0.1:7890",
            "http://[::1]:7890/",
            "127.0.0.1:7890",
            // 拒绝类地址（私网 / 链路本地）：收了也永远不通 = 假旋钮
            "http://10.0.0.5:8000",
            "http://169.254.169.254/",
            "192.168.0.10:8000",
            // 裸 host（无端口）不成基址
            "relay.example.test",
        ] {
            assert!(
                validate_allowed_server_base(base).is_err(),
                "{base:?} must be refused"
            );
        }
        for base in [
            "https://relay.example.test:8443/base",
            "relay.example.test:8443",
        ] {
            assert!(
                validate_allowed_server_base(base).is_ok(),
                "{base:?} must be accepted"
            );
        }
    }

    #[tokio::test]
    async fn user_allowlist_entries_extend_the_provider_allowlist() {
        // 策略合并：自动推导的提供方基址 + 用户显式加的基址，命中才转发。
        let path = test_config_path("merge");
        let _ = fs_err::remove_file(&path);
        let store = AllowedServersStore::load(path.clone()).expect("store");
        store
            .add("https://relay.example.test:8443/base")
            .expect("valid user base");

        let mut policy = policy_with_providers(&["https://api.example.test/v1"]);
        for base in store.list() {
            policy.providers.add_base_url(&base);
        }
        for url in [
            "https://api.example.test/v1/models",
            "https://relay.example.test:8443/v1/chat",
        ] {
            let transport = FakeTransport::new();
            let response = relay_via(&transport, &policy, url, "GET", &configured_headers(), None)
                .await
                .expect("an approved base relays");
            assert_eq!(transport.call_count(), 1, "{url} is approved");
            assert_eq!(response.status, 200);
        }
        let transport = FakeTransport::new();
        let outcome = relay_via(
            &transport,
            &policy,
            "https://other.example.test/v1",
            "GET",
            &configured_headers(),
            None,
        )
        .await;
        assert!(outcome.is_err(), "an unlisted base must be refused");
        assert_eq!(transport.call_count(), 0);
        let _ = fs_err::remove_file(&path);
    }

    #[tokio::test]
    async fn user_loopback_bases_cannot_bypass_local_endpoint_registration() {
        // 对抗用例：用户白名单出现回环基址（配置面已拒收；这里模拟绕过配置面直接
        // 写入策略层）也不得放行未注册回环目标——回环只归 LocalEndpointRegistry 管。
        let mut policy = test_policy();
        policy.providers.add_base_url("http://127.0.0.1:7890");
        policy.providers.add_base_url("localhost:7890");
        assert!(
            !policy.providers.allows("127.0.0.1", 7890),
            "the allowlist must never approve a loopback endpoint"
        );
        for url in [
            "http://127.0.0.1:7890/v1/models",
            "http://localhost:7890/v1/models",
        ] {
            let transport = FakeTransport::new();
            let outcome =
                relay_via(&transport, &policy, url, "GET", &configured_headers(), None).await;
            let Err(message) = outcome else {
                panic!("{url} must stay refused without local registration");
            };
            assert!(message.contains("not registered"), "{message}");
            assert_eq!(transport.call_count(), 0, "{url} must never relay");
        }
    }

    #[test]
    fn non_loopback_names_must_not_resolve_into_the_local_face() {
        // 对抗用例（解析层）：非本地面名字解析到回环 = 未注册本地目标，必须拒；
        // 否则「加一个指向本机的名字」就能绕过注册面。
        let loopback = ["127.0.0.1:8000"
            .parse::<std::net::SocketAddr>()
            .expect("addr")];
        assert!(
            gate_resolved_addrs("relay.example.test", &loopback).is_err(),
            "a remote-looking name must not resolve into the local face"
        );
        assert!(
            gate_resolved_addrs("localhost", &loopback).is_ok(),
            "the local-face name resolving loopback is the product's local face"
        );
        let public = ["1.2.3.4:8000"
            .parse::<std::net::SocketAddr>()
            .expect("addr")];
        assert!(gate_resolved_addrs("relay.example.test", &public).is_ok());
    }
}
