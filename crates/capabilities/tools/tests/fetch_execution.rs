//! M3A controlled Fetch integration tests.
//!
//! Every test uses an in-process local loopback server plus an explicit
//! allow-list policy (or PublicInternetOnly for denial-before-contact). No
//! Internet access is required.

use std::sync::Arc;
use std::time::Duration;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::{
    ControlledEgress, EgressAllowList, EgressPolicy, FetchConfig, FetchTool,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// Bind a loopback listener and spawn a server that accepts one connection and
/// hands it to `handler`. Returns the port and the spawned task.
async fn serve_once<F, Fut>(handler: F) -> (u16, JoinHandle<()>)
where
    F: FnOnce(TcpStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        handler(socket).await;
    });
    (port, handle)
}

/// Bind a loopback listener and return it without accepting. Used to prove a
/// fetch was denied before any TCP connection was made.
async fn bind_listener() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

fn call_url(url: impl Into<String>) -> ToolCall {
    ToolCall {
        id: "call_fetch_1".into(),
        name: "fetch".into(),
        arguments: json!({ "url": url.into() }),
    }
}

fn allowlist_egress() -> ControlledEgress {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
        .with_timeout(Duration::from_secs(5))
        .with_max_response_bytes(64 * 1024)
}

fn fetch_tool() -> FetchTool {
    FetchTool::new(FetchConfig::new(Arc::new(allowlist_egress())))
}

async fn write_http(socket: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    socket.write_all(head.as_bytes()).await.unwrap();
    socket.write_all(body).await.unwrap();
}

async fn read_request_head(socket: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = socket.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_ascii_lowercase()
}

fn value_of(result: ToolResult) -> serde_json::Value {
    match result.outcome {
        ToolOutcome::Ok { value } => value,
        ToolOutcome::Error { message, .. } => panic!("expected ok outcome, got error: {message}"),
    }
}

#[tokio::test]
async fn basic_text_fetch_returns_status_final_url_content_type_and_body() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain; charset=utf-8")],
            b"hello fetch",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/hello")))
        .await;

    let value = value_of(result);
    assert_eq!(value["status"], 200);
    assert_eq!(value["url"], format!("http://127.0.0.1:{port}/hello"));
    assert_eq!(value["content_type"], "text/plain; charset=utf-8");
    assert_eq!(value["body"], "hello fetch");
    assert_eq!(value["bytes"], 11);
    assert_eq!(value["redirects"], 0);
    server.await.unwrap();
}

#[tokio::test]
async fn json_fetch_returns_raw_body_text() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "application/json")],
            br#"{"ok":true}"#,
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/data.json")))
        .await;

    let value = value_of(result);
    assert_eq!(value["body"], r#"{"ok":true}"#);
    assert_eq!(value["content_type"], "application/json");
    server.await.unwrap();
}

#[tokio::test]
async fn html_fetch_returns_raw_html_without_stripping() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/html; charset=utf-8")],
            b"<html><body>hello</body></html>",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/page.html")))
        .await;

    let value = value_of(result);
    assert_eq!(value["body"], "<html><body>hello</body></html>");
    server.await.unwrap();
}

#[tokio::test]
async fn http_404_is_a_factual_fetch_result_not_an_infrastructure_error() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "404 Not Found",
            &[("content-type", "text/plain")],
            b"missing",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/missing")))
        .await;

    assert!(
        result.is_ok(),
        "404 must be a successful fetch result: {}",
        result.render()
    );
    let value = value_of(result);
    assert_eq!(value["status"], 404);
    assert_eq!(value["body"], "missing");
    server.await.unwrap();
}

#[tokio::test]
async fn http_500_is_a_factual_fetch_result_not_an_infrastructure_error() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "500 Internal Server Error",
            &[("content-type", "text/plain")],
            b"boom",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/fail")))
        .await;

    assert!(
        result.is_ok(),
        "500 must be a successful fetch result: {}",
        result.render()
    );
    let value = value_of(result);
    assert_eq!(value["status"], 500);
    assert_eq!(value["body"], "boom");
    server.await.unwrap();
}

#[tokio::test]
async fn private_loopback_is_denied_under_public_internet_only_before_contact() {
    let (listener, port) = bind_listener().await;

    let egress = ControlledEgress::new(EgressPolicy::PublicInternetOnly)
        .with_timeout(Duration::from_secs(2));
    let tool = FetchTool::new(FetchConfig::new(Arc::new(egress)));

    let result = tool
        .invoke(&call_url(format!("http://127.0.0.1:{port}/private")))
        .await;

    assert!(!result.is_ok(), "PublicInternetOnly must deny loopback");
    assert!(result.render().contains("denied"), "{}", result.render());

    // No TCP connection may have arrived.
    let no_contact = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
    assert!(
        no_contact.is_err(),
        "fetch must be denied before contacting the target"
    );
}

#[tokio::test]
async fn explicit_allowlist_allows_local_server_through_the_same_transport() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"allowed",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/allowed")))
        .await;

    let value = value_of(result);
    assert_eq!(value["body"], "allowed");
    server.await.unwrap();
}

#[tokio::test]
async fn redirect_to_allowed_target_returns_final_content_and_redirect_count() {
    let (port_b, server_b) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"redirected content",
        )
        .await;
    })
    .await;

    let (port_a, server_a) = serve_once(move |mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "302 Found",
            &[("location", &format!("http://127.0.0.1:{port_b}/final"))],
            b"",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port_a}/start")))
        .await;

    let value = value_of(result);
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"], "redirected content");
    assert_eq!(value["redirects"], 1);
    assert_eq!(value["url"], format!("http://127.0.0.1:{port_b}/final"));
    server_a.await.unwrap();
    server_b.await.unwrap();
}

#[tokio::test]
async fn redirect_to_denied_target_is_rejected_before_contact() {
    let (denied_listener, denied_port) = bind_listener().await;

    let (port_a, server_a) = serve_once(move |mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "302 Found",
            &[(
                "location",
                &format!("http://localhost:{denied_port}/denied"),
            )],
            b"",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port_a}/start")))
        .await;

    assert!(!result.is_ok());
    assert!(result.render().contains("denied"), "{}", result.render());

    let no_contact =
        tokio::time::timeout(Duration::from_millis(300), denied_listener.accept()).await;
    assert!(
        no_contact.is_err(),
        "redirect target must not be contacted when denied"
    );
    server_a.await.unwrap();
}

#[tokio::test]
async fn redirect_loop_is_bounded_by_the_transport() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        for _ in 0..10 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_request_head(&mut socket).await;
            write_http(
                &mut socket,
                "302 Found",
                &[("location", &format!("http://127.0.0.1:{port}/loop"))],
                b"",
            )
            .await;
        }
    });

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/loop")))
        .await;

    assert!(!result.is_ok());
    assert!(result.render().contains("redirect"), "{}", result.render());
    server.await.unwrap();
}

#[tokio::test]
async fn response_size_limit_is_enforced() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            &vec![b'a'; 256],
        )
        .await;
    })
    .await;

    let egress = ControlledEgress::new(EgressPolicy::ExplicitAllowList(
        EgressAllowList::new().allow("127.0.0.1", None),
    ))
    .with_timeout(Duration::from_secs(5))
    .with_max_response_bytes(32);
    let tool = FetchTool::new(FetchConfig::new(Arc::new(egress)));

    let result = tool
        .invoke(&call_url(format!("http://127.0.0.1:{port}/large")))
        .await;

    assert!(!result.is_ok());
    assert!(result.render().contains("exceeded"), "{}", result.render());
    server.await.unwrap();
}

#[tokio::test]
async fn timeout_is_bounded_and_reported_as_retryable() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        tokio::time::sleep(Duration::from_millis(800)).await;
        let _ = write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"late",
        )
        .await;
    })
    .await;

    let egress = ControlledEgress::new(EgressPolicy::ExplicitAllowList(
        EgressAllowList::new().allow("127.0.0.1", None),
    ))
    .with_timeout(Duration::from_millis(100));
    let tool = FetchTool::new(FetchConfig::new(Arc::new(egress)));

    let result = tool
        .invoke(&call_url(format!("http://127.0.0.1:{port}/slow")))
        .await;

    assert!(!result.is_ok());
    assert!(
        result.outcome.is_retryable(),
        "timeout should be retryable: {}",
        result.render()
    );
    server.abort();
}

#[tokio::test]
async fn unsupported_binary_media_is_not_dumped_as_base64() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "image/png")],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/image.png")))
        .await;

    assert!(!result.is_ok());
    assert!(
        result.render().contains("unsupported media type"),
        "{}",
        result.render()
    );
    assert!(
        !result.render().contains("89504e47"),
        "binary must not be base64-dumped"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn missing_content_type_with_valid_utf8_is_accepted_as_text() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(&mut socket, "200 OK", &[], b"hello no content type").await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/noct")))
        .await;

    let value = value_of(result);
    assert_eq!(value["body"], "hello no content type");
    assert_eq!(value["content_type"], serde_json::Value::Null);
    server.await.unwrap();
}

#[tokio::test]
async fn missing_content_type_with_binary_bytes_is_rejected() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(&mut socket, "200 OK", &[], &[0xff, 0xfe, 0x00]).await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/binary-noct")))
        .await;

    assert!(!result.is_ok());
    assert!(
        result.render().contains("NUL") || result.render().contains("not valid UTF-8"),
        "{}",
        result.render()
    );
    server.await.unwrap();
}

#[tokio::test]
async fn userinfo_url_is_rejected_without_contacting_or_leaking_password() {
    let (listener, port) = bind_listener().await;

    let result = fetch_tool()
        .invoke(&call_url(format!(
            "http://user:secret@127.0.0.1:{port}/private"
        )))
        .await;

    assert!(!result.is_ok());
    assert!(result.render().contains("userinfo"), "{}", result.render());
    assert!(!result.render().contains("secret"), "{}", result.render());

    let no_contact = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
    assert!(
        no_contact.is_err(),
        "userinfo URL must be rejected before contact"
    );
}

#[tokio::test]
async fn fetch_does_not_send_auth_cookie_or_proxy_headers() {
    let (port, server) = serve_once(|mut socket| async move {
        let head = read_request_head(&mut socket).await;
        assert!(
            !head.contains("authorization"),
            "fetch must not send Authorization: {head}"
        );
        assert!(
            !head.contains("cookie"),
            "fetch must not send Cookie: {head}"
        );
        assert!(
            !head.contains("proxy-authorization"),
            "fetch must not send Proxy-Authorization: {head}"
        );
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"headers clean",
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/headers")))
        .await;

    let value = value_of(result);
    assert_eq!(value["body"], "headers clean");
    server.await.unwrap();
}

#[tokio::test]
async fn html_fetch_adds_title_text_and_accessibility_without_changing_body() {
    let html = b"<html><head><title>My Page</title></head><body><h1>Hello</h1><button>OK</button></body></html>".to_vec();
    let (port, server) = serve_once(move |mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/html; charset=utf-8")],
            &html,
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/page.html")))
        .await;

    let value = value_of(result);
    assert_eq!(
        value["body"],
        "<html><head><title>My Page</title></head><body><h1>Hello</h1><button>OK</button></body></html>"
    );
    assert_eq!(value["title"], "My Page");
    let text = value["text"].as_str().unwrap_or("");
    assert!(text.contains("Hello"), "{text}");
    assert!(text.contains("OK"), "{text}");
    let snap = value["accessibility"].as_str().unwrap_or("");
    assert!(snap.contains("heading"), "{snap}");
    assert!(snap.contains("button"), "{snap}");
    assert!(snap.contains("[ref="), "{snap}");
    server.await.unwrap();
}

#[tokio::test]
async fn rate_limiter_rejects_over_limit_without_a_second_contact() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"first",
        )
        .await;
    })
    .await;

    let tool = FetchTool::new(
        FetchConfig::new(Arc::new(allowlist_egress()))
            .with_rate_limiting_config(1, Duration::from_secs(60)),
    );
    let url = format!("http://127.0.0.1:{port}/once");

    let first = tool.invoke(&call_url(&url)).await;
    let value = value_of(first);
    assert_eq!(value["body"], "first");

    let second = tool.invoke(&call_url(&url)).await;
    assert!(!second.is_ok(), "second request must be rate-limited");
    let rendered = second.render();
    assert!(rendered.contains("rate limited"), "{rendered}");
    assert!(
        second.outcome.is_retryable(),
        "rate-limit should be retryable: {rendered}"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn response_cache_serves_the_second_request_without_contacting() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"cached-body",
        )
        .await;
    })
    .await;

    let config = FetchConfig::new(Arc::new(allowlist_egress()))
        .with_response_cache(Duration::from_secs(60));
    let cache = config.response_cache().cloned();
    let tool = FetchTool::new(config);
    let url = format!("http://127.0.0.1:{port}/cached");

    let first = tool.invoke(&call_url(&url)).await;
    assert_eq!(value_of(first)["body"], "cached-body");

    let second = tool.invoke(&call_url(&url)).await;
    assert_eq!(value_of(second)["body"], "cached-body");

    let stats = cache.expect("cache enabled").stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    server.await.unwrap();
}
