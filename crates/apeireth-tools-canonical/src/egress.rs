//! Canonical controlled HTTP egress.
//!
//! This module implements **Apeireth-controlled application egress**, not
//! arbitrary child-process network isolation. Those are separate boundaries:
//! this transport validates destinations, pins DNS, and revalidates every
//! redirect hop; [`crate::process`] platform backends decide whether a spawned
//! child is physically denied network access.
//!
//! The transport is intentionally small. It supports GET only, never disables
//! TLS certificate/hostname verification, never follows redirects opaquely,
//! never inherits ambient proxy settings, and always bounds response size and
//! time.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use url::Url;

/// Default total/connect/read timeout for controlled egress.
pub const DEFAULT_EGRESS_TIMEOUT: Duration = Duration::from_secs(30);
/// Default maximum response body bytes.
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
/// Default maximum number of redirect hops.
pub const DEFAULT_MAX_REDIRECTS: usize = 10;

/// Errors produced by the controlled egress boundary.
#[derive(Debug, thiserror::Error)]
pub enum EgressError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("unsupported scheme: {0}")]
    UnsupportedScheme(String),
    #[error("dns resolution failed for {host}: {message}")]
    ResolutionFailed { host: String, message: String },
    #[error("egress destination denied: {reason}")]
    DestinationDenied { reason: String },
    #[error("redirect destination denied: {reason}")]
    RedirectDenied { reason: String },
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("tls failed: {0}")]
    TlsFailed(String),
    #[error("response body exceeded maximum size")]
    ResponseTooLarge,
    #[error("too many redirects")]
    TooManyRedirects,
    #[error("request timed out")]
    Timeout,
    #[error("invalid request header: {0}")]
    InvalidHeader(String),
}

/// Classification of an IP address for egress policy decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgressIpClass {
    Public,
    Loopback,
    Private,
    LinkLocal,
    Multicast,
    Unspecified,
    Broadcast,
}

/// Classify an IP address. IPv4-mapped IPv6 addresses are classified by their
/// embedded IPv4 address so `::ffff:127.0.0.1` cannot bypass IPv4 loopback
/// checks.
pub fn classify_ip(ip: IpAddr) -> EgressIpClass {
    match ip {
        IpAddr::V4(v4) => classify_ipv4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return classify_ipv4(mapped);
            }
            classify_ipv6(v6)
        }
    }
}

/// True when `ip` is permitted by a `PublicInternetOnly` policy.
pub fn is_public_ip(ip: IpAddr) -> bool {
    classify_ip(ip) == EgressIpClass::Public
}

fn classify_ipv4(ip: Ipv4Addr) -> EgressIpClass {
    if ip.is_loopback() {
        EgressIpClass::Loopback
    } else if ip.is_private() {
        EgressIpClass::Private
    } else if ip.is_link_local() {
        EgressIpClass::LinkLocal
    } else if ip.is_multicast() {
        EgressIpClass::Multicast
    } else if ip.is_unspecified() {
        EgressIpClass::Unspecified
    } else if ip.is_broadcast() {
        EgressIpClass::Broadcast
    } else {
        EgressIpClass::Public
    }
}

fn classify_ipv6(ip: Ipv6Addr) -> EgressIpClass {
    if ip.is_loopback() {
        EgressIpClass::Loopback
    } else if ip.is_unique_local() {
        EgressIpClass::Private
    } else if ip.is_unicast_link_local() {
        EgressIpClass::LinkLocal
    } else if ip.is_multicast() {
        EgressIpClass::Multicast
    } else if ip.is_unspecified() {
        EgressIpClass::Unspecified
    } else {
        EgressIpClass::Public
    }
}

/// A single allow-list entry: exact host and optional port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressAllowEntry {
    pub host: String,
    pub port: Option<u16>,
}

impl EgressAllowEntry {
    pub fn new(host: impl Into<String>, port: Option<u16>) -> Self {
        Self {
            host: host.into().to_ascii_lowercase(),
            port,
        }
    }
}

/// An exact-match allow-list for [`EgressPolicy::ExplicitAllowList`].
///
/// Matching is intentionally simple: hostname (or IP literal) is compared
/// case-insensitively and exactly; subdomains and wildcards are not implied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressAllowList {
    entries: HashSet<(String, Option<u16>)>,
}

impl EgressAllowList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow an exact host with an optional explicit port.
    pub fn allow(mut self, host: impl Into<String>, port: Option<u16>) -> Self {
        self.entries
            .insert((host.into().to_ascii_lowercase(), port));
        self
    }

    /// True when `host` matches an entry and the entry either has no port or
    /// the entry port equals `port`.
    pub fn contains(&self, host: &str, port: u16) -> bool {
        let host = host.to_ascii_lowercase();
        self.entries.contains(&(host.clone(), Some(port))) || self.entries.contains(&(host, None))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Canonical egress policy for Apeireth-controlled network requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EgressPolicy {
    /// No destination is allowed.
    DenyAll,
    /// Any public internet destination is allowed; local/private/link-local
    /// destinations are denied. Every resolved address must be public.
    PublicInternetOnly,
    /// Only explicitly listed exact hosts/ports are allowed.
    ExplicitAllowList(EgressAllowList),
    /// Every http/https destination is allowed. Explicit opt-out of
    /// destination protection; reserved for trusted configurations.
    Unrestricted,
}

impl EgressPolicy {
    /// Validate scheme, host, and port before DNS resolution.
    pub fn validate_destination(&self, destination: &EgressDestination) -> Result<(), EgressError> {
        match self {
            Self::DenyAll => Err(EgressError::DestinationDenied {
                reason: "egress policy is DenyAll".into(),
            }),
            Self::PublicInternetOnly => Ok(()),
            Self::ExplicitAllowList(list) => {
                if list.contains(&destination.host, destination.port) {
                    Ok(())
                } else {
                    Err(EgressError::DestinationDenied {
                        reason: format!(
                            "{}:{} is not on the explicit egress allow list",
                            destination.host, destination.port
                        ),
                    })
                }
            }
            Self::Unrestricted => Ok(()),
        }
    }

    /// Validate resolved addresses after DNS resolution.
    ///
    /// `PublicInternetOnly` is conservative: if any resolved address is not
    /// public, the whole destination is denied.
    pub fn validate_resolved(
        &self,
        destination: &EgressDestination,
        addresses: &[SocketAddr],
    ) -> Result<(), EgressError> {
        match self {
            Self::DenyAll => Err(EgressError::DestinationDenied {
                reason: "egress policy is DenyAll".into(),
            }),
            Self::PublicInternetOnly => {
                if addresses.is_empty() {
                    return Err(EgressError::ResolutionFailed {
                        host: destination.host.clone(),
                        message: "no addresses resolved".into(),
                    });
                }
                for addr in addresses {
                    if !is_public_ip(addr.ip()) {
                        return Err(EgressError::DestinationDenied {
                            reason: format!(
                                "resolved address {} for {} is not public",
                                addr.ip(),
                                destination.host
                            ),
                        });
                    }
                }
                Ok(())
            }
            Self::ExplicitAllowList(_) => Ok(()),
            Self::Unrestricted => Ok(()),
        }
    }
}

/// A parsed and normalized destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgressDestination {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    /// Present when the host is already an IP literal.
    pub address: Option<IpAddr>,
}

impl EgressDestination {
    pub fn parse(url: &Url) -> Result<Self, EgressError> {
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(EgressError::UnsupportedScheme(url.scheme().to_string()));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(EgressError::InvalidUrl(
                "url userinfo is not supported in controlled egress".into(),
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| EgressError::InvalidUrl("url has no host".into()))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| EgressError::InvalidUrl("url has no valid port".into()))?;
        let address = host.parse::<IpAddr>().ok();
        Ok(Self {
            scheme: url.scheme().to_string(),
            host: host.to_ascii_lowercase(),
            port,
            address,
        })
    }
}

/// A bounded HTTP response returned by [`ControlledEgress`].
#[derive(Debug, Clone)]
pub struct EgressHttpResponse {
    pub status: u16,
    pub final_url: String,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub body: Vec<u8>,
    pub redirects: usize,
}

#[async_trait]
trait EgressResolver: Send + Sync {
    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, EgressError>;
}

struct TokioResolver;

#[async_trait]
impl EgressResolver for TokioResolver {
    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, EgressError> {
        let addrs = tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| EgressError::ResolutionFailed {
                host: host.to_string(),
                message: e.to_string(),
            })?
            .collect::<Vec<_>>();
        if addrs.is_empty() {
            return Err(EgressError::ResolutionFailed {
                host: host.to_string(),
                message: "no addresses resolved".into(),
            });
        }
        let mut seen = HashSet::new();
        let mut unique = Vec::new();
        for addr in addrs {
            if seen.insert(addr.ip()) {
                unique.push(addr);
            }
        }
        Ok(unique)
    }
}

/// Controlled HTTP GET transport that physically enforces [`EgressPolicy`].
///
/// The transport resolves the destination, validates every resolved address,
/// pins the HTTP client to those validated addresses, disables ambient proxy
/// use, and manually follows redirects so each hop is fully revalidated.
pub struct ControlledEgress {
    policy: EgressPolicy,
    timeout: Duration,
    max_response_bytes: usize,
    max_redirects: usize,
    resolver: Arc<dyn EgressResolver>,
}

impl std::fmt::Debug for ControlledEgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlledEgress")
            .field("policy", &self.policy)
            .field("timeout", &self.timeout)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_redirects", &self.max_redirects)
            .finish_non_exhaustive()
    }
}

impl ControlledEgress {
    pub fn new(policy: EgressPolicy) -> Self {
        Self {
            policy,
            timeout: DEFAULT_EGRESS_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            resolver: Arc::new(TokioResolver),
        }
    }

    /// Set total/connect/read timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum response body size in bytes.
    pub fn with_max_response_bytes(mut self, max_response_bytes: usize) -> Self {
        self.max_response_bytes = max_response_bytes;
        self
    }

    /// Set the maximum number of redirect hops.
    pub fn with_max_redirects(mut self, max_redirects: usize) -> Self {
        self.max_redirects = max_redirects;
        self
    }

    /// The policy this transport enforces.
    pub fn policy(&self) -> &EgressPolicy {
        &self.policy
    }

    /// The configured timeout.
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// The configured maximum response body size.
    pub const fn max_response_bytes(&self) -> usize {
        self.max_response_bytes
    }

    /// The configured maximum number of redirect hops.
    pub const fn max_redirects(&self) -> usize {
        self.max_redirects
    }

    /// Perform a controlled GET request.
    pub async fn get(&self, url: &str) -> Result<EgressHttpResponse, EgressError> {
        self.get_with_headers(url, &[]).await
    }

    /// Perform a controlled GET request with fixed headers.
    ///
    /// Header names and values are validated for CRLF injection and sent on
    /// every hop. Callers must only pass non-sensitive, fixed headers.
    pub async fn get_with_headers(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<EgressHttpResponse, EgressError> {
        validate_headers(headers)?;
        let parsed = Url::parse(url).map_err(|e| EgressError::InvalidUrl(e.to_string()))?;
        let first = EgressDestination::parse(&parsed)?;
        self.policy.validate_destination(&first)?;

        tokio::time::timeout(self.timeout, self.get_inner(parsed, headers))
            .await
            .map_err(|_| EgressError::Timeout)?
    }

    async fn get_inner(
        &self,
        mut current: Url,
        headers: &[(&str, &str)],
    ) -> Result<EgressHttpResponse, EgressError> {
        let mut hops = 0usize;
        loop {
            if hops >= self.max_redirects {
                return Err(EgressError::TooManyRedirects);
            }

            let destination = EgressDestination::parse(&current)?;
            self.policy.validate_destination(&destination)?;
            let addresses = self.resolve(&destination).await?;
            self.policy.validate_resolved(&destination, &addresses)?;

            let response = self
                .send(&destination, &addresses, &current, headers)
                .await?;

            let status = response.status().as_u16();
            if (300..400).contains(&status) {
                if let Some(location) = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                {
                    let next = current
                        .join(location)
                        .map_err(|e| EgressError::RedirectDenied {
                            reason: format!("invalid redirect location: {e}"),
                        })?;
                    if current.scheme() == "https" && next.scheme() == "http" {
                        return Err(EgressError::RedirectDenied {
                            reason: "https to http redirect downgrade is denied".into(),
                        });
                    }
                    current = next;
                    hops += 1;
                    continue;
                }
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(ToOwned::to_owned);
            let content_length = response
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let body = read_limited(response, self.max_response_bytes).await?;
            return Ok(EgressHttpResponse {
                status,
                final_url: current.to_string(),
                content_type,
                content_length,
                body,
                redirects: hops,
            });
        }
    }

    async fn resolve(
        &self,
        destination: &EgressDestination,
    ) -> Result<Vec<SocketAddr>, EgressError> {
        if let Some(ip) = destination.address {
            return Ok(vec![SocketAddr::new(ip, destination.port)]);
        }
        self.resolver
            .resolve(&destination.host, destination.port)
            .await
    }

    async fn send(
        &self,
        destination: &EgressDestination,
        addresses: &[SocketAddr],
        url: &Url,
        headers: &[(&str, &str)],
    ) -> Result<reqwest::Response, EgressError> {
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(self.timeout)
            .read_timeout(self.timeout);

        if destination.address.is_none() {
            builder = builder.resolve_to_addrs(&destination.host, addresses);
        }

        let client = builder
            .build()
            .map_err(|e| EgressError::ConnectionFailed(e.to_string()))?;
        let mut request = client.get(url.clone());
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        request.send().await.map_err(map_reqwest_error)
    }
}
fn validate_headers(headers: &[(&str, &str)]) -> Result<(), EgressError> {
    for (name, value) in headers {
        if name.is_empty() || value.is_empty() {
            return Err(EgressError::InvalidHeader(
                "header name and value must not be empty".into(),
            ));
        }
        if name.contains('\r') || name.contains('\n') {
            return Err(EgressError::InvalidHeader(
                "header name must not contain CR or LF".into(),
            ));
        }
        if value.contains('\r') || value.contains('\n') {
            return Err(EgressError::InvalidHeader(
                "header value must not contain CR or LF".into(),
            ));
        }
    }
    Ok(())
}

fn map_reqwest_error(error: reqwest::Error) -> EgressError {
    if error.is_timeout() {
        EgressError::Timeout
    } else if error.is_connect() {
        EgressError::ConnectionFailed(error.to_string())
    } else {
        let message = error.to_string();
        if message.contains("tls")
            || message.contains("certificate")
            || message.contains("handshake")
        {
            EgressError::TlsFailed(message)
        } else {
            EgressError::ConnectionFailed(message)
        }
    }
}

async fn read_limited(
    response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, EgressError> {
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| EgressError::ConnectionFailed(e.to_string()))?;
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(EgressError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(input: &str) -> Url {
        Url::parse(input).unwrap()
    }

    fn dest(input: &str) -> EgressDestination {
        EgressDestination::parse(&url(input)).unwrap()
    }

    fn allowlist(entries: &[(&str, Option<u16>)]) -> EgressAllowList {
        let mut list = EgressAllowList::new();
        for (host, port) in entries {
            list = list.allow(*host, *port);
        }
        list
    }

    #[test]
    fn policy_deny_all_denies_everything() {
        let policy = EgressPolicy::DenyAll;
        let d = dest("https://example.com");
        assert!(matches!(
            policy.validate_destination(&d),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn policy_public_internet_only_accepts_public_ipv4() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        assert!(policy.validate_destination(&d).is_ok());
        let addrs = [SocketAddr::new("93.184.216.34".parse().unwrap(), 443)];
        assert!(policy.validate_resolved(&d, &addrs).is_ok());
    }

    #[test]
    fn policy_public_internet_only_denies_private_ipv4() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        let addrs = [SocketAddr::new("192.168.1.10".parse().unwrap(), 443)];
        assert!(matches!(
            policy.validate_resolved(&d, &addrs),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn policy_public_internet_only_denies_loopback_ipv4() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        let addrs = [SocketAddr::new("127.0.0.1".parse().unwrap(), 443)];
        assert!(matches!(
            policy.validate_resolved(&d, &addrs),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn policy_public_internet_only_denies_link_local_ipv4() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        let addrs = [SocketAddr::new("169.254.1.1".parse().unwrap(), 443)];
        assert!(matches!(
            policy.validate_resolved(&d, &addrs),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn policy_public_internet_only_denies_ipv6_loopback() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        let addrs = [SocketAddr::new("::1".parse().unwrap(), 443)];
        assert!(matches!(
            policy.validate_resolved(&d, &addrs),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn policy_public_internet_only_denies_ipv6_ula() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        let addrs = [SocketAddr::new("fc00::1".parse().unwrap(), 443)];
        assert!(matches!(
            policy.validate_resolved(&d, &addrs),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn ipv4_mapped_loopback_is_classified_as_loopback() {
        let ip: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        assert_eq!(classify_ip(ip), EgressIpClass::Loopback);
        assert!(!is_public_ip(ip));
    }

    #[test]
    fn ipv4_mapped_private_is_classified_as_private() {
        let ip: IpAddr = "::ffff:10.0.0.1".parse().unwrap();
        assert_eq!(classify_ip(ip), EgressIpClass::Private);
        assert!(!is_public_ip(ip));
    }

    #[test]
    fn mixed_public_private_dns_result_is_denied() {
        let policy = EgressPolicy::PublicInternetOnly;
        let d = dest("https://example.com");
        let addrs = [
            SocketAddr::new("93.184.216.34".parse().unwrap(), 443),
            SocketAddr::new("10.0.0.1".parse().unwrap(), 443),
        ];
        assert!(matches!(
            policy.validate_resolved(&d, &addrs),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn allowlist_exact_match_and_mismatch() {
        let policy = EgressPolicy::ExplicitAllowList(allowlist(&[("example.com", Some(443))]));
        let d = dest("https://example.com");
        assert!(policy.validate_destination(&d).is_ok());

        let d = dest("https://example.com:444");
        assert!(matches!(
            policy.validate_destination(&d),
            Err(EgressError::DestinationDenied { .. })
        ));

        let d = dest("https://evil-example.com");
        assert!(matches!(
            policy.validate_destination(&d),
            Err(EgressError::DestinationDenied { .. })
        ));
    }

    #[test]
    fn allowlist_without_port_matches_any_port() {
        let policy = EgressPolicy::ExplicitAllowList(allowlist(&[("example.com", None)]));
        let d = dest("https://example.com:8443");
        assert!(policy.validate_destination(&d).is_ok());
    }

    #[tokio::test]
    async fn malformed_url_is_rejected() {
        assert!(matches!(
            ControlledEgress::new(EgressPolicy::PublicInternetOnly)
                .get("not a url")
                .await,
            Err(EgressError::InvalidUrl(_))
        ));
    }

    #[tokio::test]
    async fn unsupported_scheme_is_rejected() {
        assert!(matches!(
            ControlledEgress::new(EgressPolicy::PublicInternetOnly)
                .get("file:///etc/passwd")
                .await,
            Err(EgressError::UnsupportedScheme(_))
        ));
    }

    #[test]
    fn url_userinfo_is_rejected() {
        assert!(matches!(
            EgressDestination::parse(&url("https://user:secret@example.com")),
            Err(EgressError::InvalidUrl(_))
        ));
    }

    #[test]
    fn public_internet_only_rejects_local_url_before_connection() {
        let dest = dest("http://127.0.0.1:8080");
        assert_eq!(dest.address, Some("127.0.0.1".parse().unwrap()));
    }

    struct FakeResolver {
        addrs: Vec<SocketAddr>,
    }

    #[async_trait]
    impl EgressResolver for FakeResolver {
        async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<SocketAddr>, EgressError> {
            Ok(self.addrs.clone())
        }
    }

    #[tokio::test]
    async fn pinned_resolver_contacts_validated_address() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok")
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "egress error: {e} source: {:?}",
                        std::error::Error::source(&e)
                    )
                });
        });

        let resolver = Arc::new(FakeResolver {
            addrs: vec![SocketAddr::new("127.0.0.1".parse().unwrap(), port)],
        });
        let transport = ControlledEgress::new(EgressPolicy::ExplicitAllowList(allowlist(&[(
            "pinned.invalid",
            None,
        )])))
        .with_timeout(Duration::from_secs(5))
        .with_max_response_bytes(1024);
        let transport = ControlledEgress {
            resolver,
            ..transport
        };

        let response = transport
            .get(&format!("http://pinned.invalid:{port}/"))
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "egress error: {e} source: {:?}",
                    std::error::Error::source(&e)
                )
            });
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"ok");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn redirect_to_denied_destination_is_rejected_before_contact() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Server that redirects to 127.0.0.1:<same port>/denied. Because the
        // policy only allows `allowed.test`, the redirect must be denied.
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:{}/denied\r\ncontent-length: 0\r\n\r\n",
                        port
                    )
                    .as_bytes(),
                )
                .await
            .unwrap();
        });

        let resolver = Arc::new(FakeResolver {
            addrs: vec![SocketAddr::new("127.0.0.1".parse().unwrap(), port)],
        });
        let transport = ControlledEgress::new(EgressPolicy::ExplicitAllowList(allowlist(&[(
            "allowed.test",
            None,
        )])))
        .with_timeout(Duration::from_secs(5));
        let transport = ControlledEgress {
            resolver,
            ..transport
        };

        let err = transport
            .get(&format!("http://allowed.test:{port}/"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, EgressError::DestinationDenied { .. })
                || matches!(err, EgressError::RedirectDenied { .. })
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn direct_ip_transport_works() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok")
                .await
                .unwrap();
        });
        let transport = ControlledEgress::new(EgressPolicy::ExplicitAllowList(allowlist(&[(
            "127.0.0.1",
            None,
        )])))
        .with_timeout(Duration::from_secs(5));
        let response = transport
            .get(&format!("http://127.0.0.1:{port}/"))
            .await
            .unwrap();
        assert_eq!(response.body, b"ok");
        server.await.unwrap();
    }
}
