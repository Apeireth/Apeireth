//! `web.probe`：HTTP 指纹与安全头探测。
//!
//! 边界：只发一次 GET、不跟随重定向、不读响应体；主机必须落在授权 Scope 内
//! （IP 走网段校验，域名走域名校验）。TLS 版本/证书链不在本工具范围内，
//! 明确写进 limitations。

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::scope::{Scope, ScopeError};

/// 关注的安全响应头（小写）。
const SECURITY_HEADERS: &[&str] = &[
    "strict-transport-security",
    "content-security-policy",
    "x-frame-options",
    "x-content-type-options",
    "referrer-policy",
    "permissions-policy",
    "cross-origin-opener-policy",
    "cross-origin-resource-policy",
];

/// 一次 HTTP 指纹观察。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebProbe {
    /// 请求的 URL。
    pub url: String,
    /// 状态码。
    pub status: u16,
    /// `Server` 头。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// `Content-Type` 头。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// 安全头：存在时给出值，缺失时值为 `None`。
    pub security_headers: BTreeMap<String, Option<String>>,
    /// 技术栈线索（来自 Server / X-Powered-By）。
    pub technologies: Vec<String>,
    /// 显式局限。
    pub limitations: Vec<String>,
}

/// 探测参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebProbeOptions {
    /// 总超时。
    pub timeout: Duration,
    /// User-Agent。
    pub user_agent: &'static str,
}

impl Default for WebProbeOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            user_agent: "Apeireth-Recon/0.1",
        }
    }
}

/// 探测错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WebError {
    /// 目标越界。
    #[error(transparent)]
    Scope(#[from] ScopeError),
    /// URL 非法。
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    /// 请求失败（网络 / TLS / 超时）。
    #[error("request failed: {0}")]
    Request(String),
}

/// 探测一个 URL。
pub async fn probe(
    url: &str,
    scope: &Scope,
    options: &WebProbeOptions,
) -> Result<WebProbe, WebError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| WebError::InvalidUrl(url.to_string()))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(WebError::InvalidUrl(url.to_string()));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| WebError::InvalidUrl(url.to_string()))?;
    match host.parse::<IpAddr>() {
        Ok(address) => scope.check_address(address)?,
        Err(_) => scope.check_domain(host)?,
    }

    let client = reqwest::Client::builder()
        .timeout(options.timeout)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(options.user_agent)
        .build()
        .map_err(|error| WebError::Request(error.to_string()))?;

    let response = client
        .get(parsed.clone())
        .send()
        .await
        .map_err(|error| WebError::Request(error.to_string()))?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();

    let mut security_headers = BTreeMap::new();
    for name in SECURITY_HEADERS {
        let value = headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        security_headers.insert((*name).to_string(), value);
    }

    let header_text = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    };
    let server = header_text("server");
    let content_type = header_text("content-type");

    let powered_by = header_text("x-powered-by");
    let mut technologies = Vec::new();
    for hint in [server.as_deref(), powered_by.as_deref()]
        .into_iter()
        .flatten()
    {
        let trimmed = hint.trim();
        if !trimmed.is_empty() && !technologies.iter().any(|seen| seen == trimmed) {
            technologies.push(trimmed.to_string());
        }
    }

    Ok(WebProbe {
        url: parsed.to_string(),
        status,
        server,
        content_type,
        security_headers,
        technologies,
        limitations: vec![
            "redirects are not followed".to_string(),
            "response body is not read".to_string(),
            "TLS version and certificate chain are not inspected".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn scope_for_host(host: &str) -> Scope {
        let mut scope = Scope::empty("test");
        scope.authorized = true;
        if host.parse::<IpAddr>().is_ok() {
            scope.cidrs = vec![format!("{host}/32")];
        } else {
            scope.domains = vec![host.to_string()];
        }
        scope
    }

    fn serve_once(response: &'static str) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (port, handle)
    }

    #[tokio::test]
    async fn probes_status_and_security_headers() {
        let (port, handle) = serve_once(
            "HTTP/1.1 200 OK\r\nServer: fixture/1.0\r\nContent-Type: text/html\r\n\
             Strict-Transport-Security: max-age=31536000\r\nContent-Length: 0\r\n\r\n",
        );
        let url = format!("http://127.0.0.1:{port}/");
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("probe");

        assert_eq!(probe.status, 200);
        assert_eq!(probe.server.as_deref(), Some("fixture/1.0"));
        assert_eq!(
            probe.security_headers.get("strict-transport-security"),
            Some(&Some("max-age=31536000".to_string()))
        );
        assert_eq!(
            probe.security_headers.get("content-security-policy"),
            Some(&None)
        );
        assert!(probe.technologies.iter().any(|t| t.contains("fixture")));
        handle.join().ok();
    }

    #[tokio::test]
    async fn out_of_scope_host_is_rejected() {
        let error = probe(
            "http://example.com/",
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect_err("must be denied");
        assert!(matches!(error, WebError::Scope(_)));
    }

    #[tokio::test]
    async fn unsupported_scheme_is_rejected() {
        let error = probe(
            "file:///etc/passwd",
            &scope_for_host("localhost"),
            &WebProbeOptions::default(),
        )
        .await
        .expect_err("must be denied");
        assert!(matches!(error, WebError::InvalidUrl(_)));
    }
}
