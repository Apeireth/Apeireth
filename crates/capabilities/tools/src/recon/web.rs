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
    use std::sync::mpsc::Receiver;

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

    /// 回环 fixture：接受一次连接、读走请求头、回固定字节。
    ///
    /// `port` 是 fixture 端口，`request` 带回客户端发来的请求原文，`handle`
    /// 用于收尾。全部落在 127.0.0.1，**不触外网**。
    struct Fixture {
        port: u16,
        request: Receiver<String>,
        handle: std::thread::JoinHandle<()>,
    }

    impl Fixture {
        /// 取回客户端发来的请求原文（最多等 5 秒）。
        fn request_text(&self) -> String {
            self.request
                .recv_timeout(Duration::from_secs(5))
                .unwrap_or_default()
        }

        /// 收尾：等服务端线程退出。
        fn join(self) {
            self.handle.join().ok();
        }
    }

    fn serve_bytes(response: Vec<u8>) -> Fixture {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                let mut request = Vec::new();
                let mut buffer = [0u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => request.extend_from_slice(&buffer[..count]),
                    }
                }
                let _ = tx.send(String::from_utf8_lossy(&request).to_string());
                let _ = stream.write_all(&response);
                let _ = stream.flush();
            }
        });
        Fixture {
            port,
            request: rx,
            handle,
        }
    }

    fn serve_once(response: &'static str) -> (u16, std::thread::JoinHandle<()>) {
        let fixture = serve_bytes(response.as_bytes().to_vec());
        (fixture.port, fixture.handle)
    }

    /// 断言监听器上没有任何待接受的连接 —— 即：这条路径**没有发出请求**。
    fn assert_no_pending_connection(listener: &TcpListener) {
        listener.set_nonblocking(true).expect("nonblocking");
        match listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Ok((stream, _)) => {
                drop(stream);
                panic!("拒绝路径不得发出任何连接");
            }
            Err(error) => panic!("accept 失败: {error}"),
        }
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

    // ── 拒绝面：全部在**发请求之前**收场 ────────────────────────────────────

    /// 空范围（未授权 / 已授权但清单全空）⇒ 全拒，且**没有发出任何连接**。
    #[tokio::test]
    async fn empty_scope_denies_every_target_without_connecting() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let url = format!("http://127.0.0.1:{port}/");

        let error = probe(&url, &Scope::empty("empty"), &WebProbeOptions::default())
            .await
            .expect_err("未授权范围必须拒绝");
        assert!(
            matches!(error, WebError::Scope(ScopeError::NotAuthorized(_))),
            "{error:?}"
        );

        let mut authorized_but_empty = Scope::empty("empty-but-authorized");
        authorized_but_empty.authorized = true;
        let error = probe(&url, &authorized_but_empty, &WebProbeOptions::default())
            .await
            .expect_err("清单全空必须拒绝");
        assert!(
            matches!(error, WebError::Scope(ScopeError::NotAuthorized(_))),
            "{error:?}"
        );

        assert_no_pending_connection(&listener);
    }

    /// 非 http(s) 协议（file / ftp / gopher / data）⇒ 拒绝，且**没有连接**。
    #[tokio::test]
    async fn non_http_schemes_are_rejected_without_connecting() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let scope = scope_for_host("127.0.0.1");

        for url in [
            "file:///etc/passwd".to_string(),
            format!("ftp://127.0.0.1:{port}/x"),
            format!("gopher://127.0.0.1:{port}/x"),
            "data:text/plain,hello".to_string(),
        ] {
            let error = probe(&url, &scope, &WebProbeOptions::default())
                .await
                .expect_err("非 http(s) 必须拒绝");
            match error {
                WebError::InvalidUrl(ref payload) => assert_eq!(payload, &url),
                other => panic!("期望 InvalidUrl，实际 {other:?}"),
            }
        }

        assert_no_pending_connection(&listener);
    }

    /// 畸形 URL ⇒ 拒绝（不是 panic，也不是静默空结果）。
    #[tokio::test]
    async fn malformed_urls_are_rejected() {
        let scope = scope_for_host("127.0.0.1");
        for url in ["", "http://", "not a url", "://missing-scheme"] {
            let error = probe(url, &scope, &WebProbeOptions::default())
                .await
                .expect_err("畸形 URL 必须拒绝");
            assert!(matches!(error, WebError::InvalidUrl(_)), "{url}: {error:?}");
        }
    }

    /// 越界目标（IP 走网段、域名走域名两条路）⇒ Scope 拒绝，且**没有连接**。
    #[tokio::test]
    async fn out_of_scope_targets_are_rejected_without_connecting() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();

        // IP 路：授权网段不含回环地址。
        let mut ip_scope = Scope::empty("cidr-only");
        ip_scope.authorized = true;
        ip_scope.cidrs = vec!["10.0.0.1/32".to_string()];
        let error = probe(
            &format!("http://127.0.0.1:{port}/"),
            &ip_scope,
            &WebProbeOptions::default(),
        )
        .await
        .expect_err("越界 IP 必须拒绝");
        assert!(
            matches!(error, WebError::Scope(ScopeError::Outside(_))),
            "{error:?}"
        );

        // 域名路：主机名不是 IP ⇒ 必须落到域名档（授权清单里没有它 ⇒ 越界）。
        let mut domain_scope = Scope::empty("domain-only");
        domain_scope.authorized = true;
        domain_scope.domains = vec!["example.com".to_string()];
        let error = probe(
            "http://host.corp.example/",
            &domain_scope,
            &WebProbeOptions::default(),
        )
        .await
        .expect_err("越界域名必须拒绝");
        assert!(
            matches!(error, WebError::Scope(ScopeError::Outside(_))),
            "{error:?}"
        );

        assert_no_pending_connection(&listener);
    }

    /// IPv6 字面量主机不得绕过授权：未授权时必须拒绝，且**不得发出任何请求**
    /// （若真发过请求，结果会是 `Request` 或 `Ok`，不会是 `Scope`）。
    ///
    /// 现状注记：`Url::host_str()` 对 IPv6 返回带方括号的 `[::1]`，因此 IPv6
    /// 字面量落到**域名**档，授权清单里的 IPv6 网段对该工具不可达 —— 方向仍是
    /// fail-closed（永远拒），但口径缺口已登记在交付报告里；此处只钉「不得绕过
    /// 授权」这一条不变量。
    #[tokio::test]
    async fn ipv6_literals_never_bypass_the_authorization() {
        let mut scope = Scope::empty("v6");
        scope.authorized = true;
        scope.cidrs = vec!["10.0.0.1/32".to_string()];

        let error = probe("http://[::1]:8080/", &scope, &WebProbeOptions::default())
            .await
            .expect_err("未授权的 IPv6 目标必须拒绝");
        assert!(matches!(error, WebError::Scope(_)), "{error:?}");
    }

    /// 授权通过 ⇒ 进入传输层；传输失败必须映射成结构化 `Request` 错误
    /// （不是 panic，也不是静默空结果）。
    #[tokio::test]
    async fn transport_failure_maps_to_structured_request_error() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener); // 端口无人监听 ⇒ 连接被拒

        let outcome = probe(
            &format!("http://127.0.0.1:{port}/"),
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await;

        match outcome {
            Err(WebError::Request(message)) => {
                assert!(!message.trim().is_empty(), "拒绝帧必须带可读原因");
            }
            Err(WebError::Scope(_)) => panic!("授权内的目标不得被判越界"),
            Err(other) => panic!("期望 Request 错误，实际 {other:?}"),
            // 端口被别的进程抢走时可能拿到响应：仍证明 scope 已放行到传输层。
            Ok(_) => {}
        }
    }

    /// 对端不应答 ⇒ 超时收场，映射成结构化 `Request` 错误。
    #[tokio::test]
    async fn stalled_target_maps_to_structured_timeout_error() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buffer = [0u8; 512];
                let _ = stream.read(&mut buffer);
                // 收下请求但不应答：客户端只能靠超时收场。
                std::thread::sleep(Duration::from_millis(800));
            }
        });

        let options = WebProbeOptions {
            timeout: Duration::from_millis(150),
            ..WebProbeOptions::default()
        };
        let error = probe(
            &format!("http://127.0.0.1:{port}/"),
            &scope_for_host("127.0.0.1"),
            &options,
        )
        .await
        .expect_err("无响应必须失败");
        assert!(matches!(error, WebError::Request(_)), "{error:?}");
        handle.join().ok();
    }

    // ── 观察面：响应头装配、重定向、正文边界 ─────────────────────────────────

    /// 响应头装配：被关注的安全头逐个出现在 map 里（缺失记 `None`）。
    #[tokio::test]
    async fn response_headers_are_assembled_into_the_probe() {
        let fixture = serve_bytes(
            b"HTTP/1.1 200 OK\r\n\
              Server: fixture/1.0\r\n\
              Content-Type: text/html; charset=utf-8\r\n\
              Strict-Transport-Security: max-age=31536000\r\n\
              Content-Length: 0\r\n\
              \r\n"
                .to_vec(),
        );
        let url = format!("http://127.0.0.1:{}/", fixture.port);
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
            probe.content_type.as_deref(),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(probe.security_headers.len(), SECURITY_HEADERS.len());
        assert_eq!(
            probe.security_headers.get("strict-transport-security"),
            Some(&Some("max-age=31536000".to_string()))
        );
        assert_eq!(probe.security_headers.get("x-frame-options"), Some(&None));
        fixture.join();
    }

    /// 出网请求的形状：一次 GET、带 UA；URL 归一化后回填到输出。
    #[tokio::test]
    async fn request_is_a_single_get_with_a_user_agent() {
        let fixture = serve_bytes(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec());
        let url = format!("http://127.0.0.1:{}", fixture.port);
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("probe");

        assert_eq!(probe.url, format!("http://127.0.0.1:{}/", fixture.port));
        let request = fixture.request_text().to_lowercase();
        assert!(request.starts_with("get / http/1.1"), "{request}");
        assert!(request.contains("user-agent:"), "{request}");
        fixture.join();
    }

    /// 重定向不跟随：3xx 原样作为观察面返回，跳板端口从未被连接。
    #[tokio::test]
    async fn redirects_are_surfaced_and_never_followed() {
        let hop = TcpListener::bind("127.0.0.1:0").expect("bind");
        let hop_port = hop.local_addr().expect("addr").port();

        let fixture = serve_bytes(
            format!(
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{hop_port}/next\r\n\
                 Content-Length: 0\r\n\r\n"
            )
            .into_bytes(),
        );
        let url = format!("http://127.0.0.1:{}/start", fixture.port);
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("probe");

        assert_eq!(probe.status, 302);
        assert_eq!(probe.url, url);
        assert!(
            probe
                .limitations
                .iter()
                .any(|item| item.contains("redirect")),
            "{:?}",
            probe.limitations
        );
        // 跳板端口没有被连接 ⇒ 没有跟随重定向。
        assert_no_pending_connection(&hop);
        fixture.join();
    }

    /// 响应体不进输出：只读头，超长响应不会撑大输出面。
    #[tokio::test]
    async fn response_body_is_never_surfaced() {
        let body = "MARKER-BODY ".repeat(4096);
        let fixture = serve_bytes(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .into_bytes(),
        );
        let url = format!("http://127.0.0.1:{}/", fixture.port);
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("probe");

        assert_eq!(probe.status, 200);
        let rendered = serde_json::to_string(&probe).expect("serialize");
        assert!(!rendered.contains("MARKER-BODY"), "响应体不得进入输出");
        assert!(rendered.len() < 4096, "输出必须是有界的");
        assert!(
            probe.limitations.iter().any(|item| item.contains("body")),
            "{:?}",
            probe.limitations
        );
        fixture.join();
    }

    /// 非 2xx 不是错误：状态码作为观察面返回（结构化），不是拒绝也不是空结果。
    #[tokio::test]
    async fn non_2xx_status_is_reported_as_an_observation() {
        let fixture = serve_bytes(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec());
        let url = format!("http://127.0.0.1:{}/missing", fixture.port);
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("4xx 是观察面，不是错误");

        assert_eq!(probe.status, 404);
        assert_eq!(probe.security_headers.len(), SECURITY_HEADERS.len());
        fixture.join();
    }

    /// 技术栈线索：Server 与 X-Powered-By 同值时只记一次。
    #[tokio::test]
    async fn technologies_are_deduplicated() {
        let fixture = serve_bytes(
            b"HTTP/1.1 200 OK\r\n\
              Server: fixture/1.0\r\n\
              X-Powered-By: fixture/1.0\r\n\
              Content-Length: 0\r\n\
              \r\n"
                .to_vec(),
        );
        let url = format!("http://127.0.0.1:{}/", fixture.port);
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("probe");

        assert_eq!(probe.technologies, vec!["fixture/1.0".to_string()]);
        fixture.join();
    }

    /// 非 UTF-8 的响应头值读不出来时按「缺失」处理（不 panic、不乱码）。
    #[tokio::test]
    async fn unreadable_header_values_are_treated_as_absent() {
        let fixture = serve_bytes(
            b"HTTP/1.1 200 OK\r\nServer: \xff\xfe\r\nContent-Length: 0\r\n\r\n".to_vec(),
        );
        let url = format!("http://127.0.0.1:{}/", fixture.port);
        let probe = probe(
            &url,
            &scope_for_host("127.0.0.1"),
            &WebProbeOptions::default(),
        )
        .await
        .expect("probe");

        assert_eq!(probe.server, None);
        assert!(probe.technologies.is_empty());
        fixture.join();
    }
}
