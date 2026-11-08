//! `tool.http_write` 集成测试 —— **带鉴权的受控 HTTP 写面**。
//!
//! 全部用例都跑在**本机回环**上（不联网）：一个进程内 HTTP 服务器 + 一份
//! **显式域名白名单** + 一条**显式配置的凭据绑定**。
//!
//! 覆盖的口径：
//!
//! * 正常 POST / PUT / PATCH / DELETE；
//! * 非白名单域名拒绝即帧（**联系之前**就拒）；
//! * 私网地址（SSRF）拒绝；
//! * 动作词表**表外**拒绝；
//! * 请求体超限拒绝 / 响应体超限拒绝；
//! * 凭据**不回显**（结果面 / 冻结载荷 / 审批视图 / Debug 全无明文）；
//! * **对抗**：白名单绕过形态（userinfo 混淆、查询串夹带、后缀拼接）一律拒绝。

use std::sync::Arc;
use std::time::Duration;

use apeireth_plugin::{CredentialResolver, StaticCredentials, ToolCapability};
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::egress::{
    ControlledEgress, EgressAllowList, EgressMethod, EgressPolicy,
};
use apeireth_tools_canonical::http_write::{CredentialBinding, HttpWriteConfig, HttpWriteTool};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

// D26 (2026-10-21): 回环 bind 走共享重试助手 (仅 Windows WSAEACCES 10013 有界重试)。
mod common;

/// 一个**非凭据形状**的凭据值 ⇒ 形状层扫不到它 ⇒ 只有「按值精确遮蔽」能挡住回显。
const SECRET: &str = "value-only-secret-9f3a1c7e";
/// 凭据的**逻辑名**（不是秘密；值才是）。
const CREDENTIAL_NAME: &str = "provider.demo.api_key";

// ============================================================
// 夹具
// ============================================================

/// 绑一个回环监听器并**只接受一次**连接，交给 `handler` 处理。
async fn serve_once<F, Fut>(handler: F) -> (u16, JoinHandle<()>)
where
    F: FnOnce(TcpStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = common::bind_loopback_ephemeral()
        .await
        .expect("bind loopback listener");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        handler(socket).await;
    });
    (port, handle)
}

/// 绑一个回环监听器但**不接收** ⇒ 任何连接都会暴露「请求真的发出去了」。
async fn bind_listener() -> (TcpListener, u16) {
    let listener = common::bind_loopback_ephemeral()
        .await
        .expect("bind loopback listener");
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// 读完整的一个请求（头 + 体），返回 `(头文本, 体字节)`。
async fn read_request(socket: &mut TcpStream) -> (String, Vec<u8>) {
    let mut head: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        let read = socket.read(&mut byte).await.expect("read request head");
        if read == 0 {
            break;
        }
        head.push(byte[0]);
    }
    let head_text = String::from_utf8_lossy(&head).to_string();
    let content_length = head_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        socket
            .read_exact(&mut body)
            .await
            .expect("read request body");
    }
    (head_text, body)
}

/// 回一个响应（带 `content-length` 与 `connection: close`）。
///
/// ⚠️ 写失败**不 panic**：响应体超限时客户端会提前断开 ⇒ 服务端写剩余字节
/// 得到 broken pipe 属**正常**（不是用例失败）。
async fn respond(socket: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = socket.write_all(head.as_bytes()).await;
    let _ = socket.write_all(body).await;
}

fn allow_list(entries: &[(&str, Option<u16>)]) -> EgressAllowList {
    let mut list = EgressAllowList::new();
    for (host, port) in entries {
        list = list.allow(*host, *port);
    }
    list
}

/// 受控通道：显式域名白名单 + 有界的超时 / 请求体 / 响应体。
fn egress(entries: &[(&str, Option<u16>)]) -> ControlledEgress {
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(allow_list(entries)))
        .with_timeout(Duration::from_secs(5))
        .with_max_response_bytes(8 * 1024)
        .with_max_request_bytes(1024)
}

fn resolver_with(secret: &str) -> Arc<dyn CredentialResolver> {
    Arc::new(StaticCredentials::new().with(CREDENTIAL_NAME, secret))
}

/// 写面：白名单 + 凭据链 + 一条绑定到 `127.0.0.1` 的 Bearer 绑定。
fn tool_with_binding(secret: &str) -> HttpWriteTool {
    let config = HttpWriteConfig::new(
        Arc::new(egress(&[("127.0.0.1", None)])),
        resolver_with(secret),
    )
    .expect("the allow list policy is accepted")
    .with_binding(CredentialBinding::bearer(CREDENTIAL_NAME, "127.0.0.1"))
    .expect("the binding is valid");
    HttpWriteTool::new(config)
}

/// 写面：白名单 + 凭据链（**无绑定**）。
fn tool_without_binding(secret: &str) -> HttpWriteTool {
    let config = HttpWriteConfig::new(
        Arc::new(egress(&[("127.0.0.1", None)])),
        resolver_with(secret),
    )
    .expect("the allow list policy is accepted");
    HttpWriteTool::new(config)
}

/// 写面：给一条**自定义**受控通道 + 凭据链。
fn tool_for(egress: ControlledEgress, credentials: Arc<dyn CredentialResolver>) -> HttpWriteTool {
    let config = HttpWriteConfig::new(Arc::new(egress), credentials)
        .expect("the allow list policy is accepted");
    HttpWriteTool::new(config)
}

fn call(arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call_http_write_1".into(),
        name: "http_write".into(),
        arguments,
    }
}

/// 成功结果里的结构化载荷。
fn value_of(result: &ToolResult) -> &serde_json::Value {
    let ToolOutcome::Ok { value } = &result.outcome else {
        panic!("expected a successful result: {}", result.render());
    };
    value
}

/// 失败结果里的消息。
fn error_of(result: &ToolResult) -> String {
    let ToolOutcome::Error { message, .. } = &result.outcome else {
        panic!("expected a failed result: {}", result.render());
    };
    message.clone()
}

// ============================================================
// 正常路径
// ============================================================

#[tokio::test]
async fn a_normal_post_reaches_the_allow_listed_host() {
    let (port, server) = serve_once(|mut socket| async move {
        let (head, body) = read_request(&mut socket).await;
        assert!(
            head.starts_with("POST /"),
            "unexpected request head: {head}"
        );
        assert_eq!(body, b"{\"ping\":true}");
        respond(&mut socket, "200 OK", &[], b"{\"pong\":true}").await;
    })
    .await;

    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{port}/items"),
            "body": "{\"ping\":true}",
        })))
        .await;

    assert!(result.is_ok(), "{}", result.render());
    let value = value_of(&result);
    assert_eq!(value["status"], json!(200));
    assert_eq!(value["method"], json!("POST"));
    assert_eq!(value["bytes"], json!(13));
    // 远端正文按「不可信」包裹 ⇒ 明文里看不到裸正文。
    let body = value["body"].as_str().expect("body is a string");
    assert!(body.contains("pong"), "{body}");
    server.await.unwrap();
}

#[tokio::test]
async fn every_write_method_in_the_closed_vocabulary_works() {
    for (method, expected) in [
        ("post", "POST"),
        ("put", "PUT"),
        ("patch", "PATCH"),
        ("delete", "DELETE"),
    ] {
        let (port, server) = serve_once(|mut socket| async move {
            let (_head, _body) = read_request(&mut socket).await;
            respond(&mut socket, "200 OK", &[], b"ok").await;
        })
        .await;

        let tool = tool_without_binding(SECRET);
        let result = tool
            .invoke(&call(json!({
                "method": method,
                "url": format!("http://127.0.0.1:{port}/items"),
            })))
            .await;
        assert!(result.is_ok(), "{method}: {}", result.render());
        assert_eq!(value_of(&result)["method"], json!(expected));
        let _ = server.await.unwrap();
    }
}

// ============================================================
// 目的地闸
// ============================================================

#[tokio::test]
async fn a_host_outside_the_allow_list_is_refused_before_contact() {
    // 白名单**只**放行 `(127.0.0.1, allowed_port)` ⇒ 另一个端口上的同一台
    // 主机也在表外（端口维度同样是白名单维度）。
    let (_allowed_listener, allowed_port) = bind_listener().await;
    let (denied_listener, denied_port) = bind_listener().await;

    let tool = tool_for(
        egress(&[("127.0.0.1", Some(allowed_port))]),
        resolver_with(SECRET),
    );

    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{denied_port}/items"),
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(
        message.contains("not on the explicit egress allow list"),
        "{message}"
    );
    assert!(
        !message.contains("resolution failed"),
        "拒绝必须发生在目的地判定上: {message}"
    );

    // 表外的端口**从未**被联系。
    let accepted = tokio::time::timeout(Duration::from_millis(200), denied_listener.accept()).await;
    assert!(accepted.is_err(), "表外目的地被联系了");
}

#[tokio::test]
async fn a_host_name_outside_the_allow_list_is_refused_before_contact() {
    let tool = tool_for(egress(&[("127.0.0.1", None)]), resolver_with(SECRET));
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": "http://not-allowed.invalid/items",
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(
        message.contains("not on the explicit egress allow list"),
        "{message}"
    );
}

#[tokio::test]
async fn a_private_address_destination_is_refused() {
    // ⚠️ `localhost` **在白名单里**（主机名目的地）⇒ 它解析到回环地址 ⇒
    //    解析后地址判定（SSRF 纵深）必须拒绝 —— 这正是「表内主机名 ≠ 私网可达」。
    let config = HttpWriteConfig::new(
        Arc::new(egress(&[("localhost", None)])),
        resolver_with(SECRET),
    )
    .expect("the allow list policy is accepted");
    let tool = HttpWriteTool::new(config);

    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": "http://localhost/items",
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(
        message.contains("non-routable address") || message.contains("destination denied"),
        "{message}"
    );
    assert!(
        !message.contains("resolution failed"),
        "私网判定必须发生在解析之后, 不是解析失败: {message}"
    );
}

// ============================================================
// 动作词表（闭合）
// ============================================================

#[tokio::test]
async fn a_method_outside_the_closed_vocabulary_is_refused() {
    for method in ["get", "options", "connect", "trace", "", "POSTX"] {
        let tool = tool_without_binding(SECRET);
        let result = tool
            .invoke(&call(json!({
                "method": method,
                "url": "http://127.0.0.1:1/items",
            })))
            .await;
        assert!(!result.is_ok(), "method {method:?} must be refused");
        let message = error_of(&result);
        assert!(
            message.contains("vocabulary is closed"),
            "method {method:?}: {message}"
        );
    }
}

#[tokio::test]
async fn an_unknown_parameter_is_refused() {
    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": "http://127.0.0.1:1/items",
            "headers": { "x-anything": "1" },
        })))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(
        error_of(&result).contains("unknown http write parameter"),
        "{}",
        result.render()
    );
}

// ============================================================
// 边界（请求体 / 响应体）
// ============================================================

#[tokio::test]
async fn an_oversized_request_body_is_refused_before_contact() {
    let (_listener, port) = bind_listener().await;

    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{port}/items"),
            "body": "x".repeat(4096),
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(message.contains("maximum is 1024 bytes"), "{message}");
}

#[tokio::test]
async fn an_oversized_response_body_is_refused() {
    let (port, server) = serve_once(|mut socket| async move {
        let (_head, _body) = read_request(&mut socket).await;
        // 响应体 16 KiB > 通道上限 8 KiB。
        respond(&mut socket, "200 OK", &[], &vec![b'x'; 16 * 1024]).await;
    })
    .await;

    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{port}/items"),
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(
        message.contains("response body exceeded maximum size"),
        "{message}"
    );
    server.await.unwrap();
}

// ============================================================
// 鉴权
// ============================================================

#[tokio::test]
async fn the_credential_is_actually_sent_and_never_echoed_back() {
    let (port, server) = serve_once(|mut socket| async move {
        let (head, _body) = read_request(&mut socket).await;
        let expected = format!("authorization: bearer {SECRET}");
        let seen = head.to_ascii_lowercase();
        let answer = if seen.contains(&expected) {
            "auth-ok"
        } else {
            "auth-missing"
        };
        respond(&mut socket, "200 OK", &[], answer.as_bytes()).await;
    })
    .await;

    let tool = tool_with_binding(SECRET);
    let call = call(json!({
        "method": "post",
        "url": format!("http://127.0.0.1:{port}/items"),
        "credential_ref": CREDENTIAL_NAME,
    }));

    // 冻结载荷（审批面）里**不得**有凭据明文。
    let frozen = tool
        .freeze_invocation(&call)
        .expect("freeze")
        .expect("a frozen payload");
    let payload = serde_json::to_string(&frozen.payload).expect("serialize payload");
    let display = serde_json::to_string(&frozen.display).expect("serialize display");
    assert!(!payload.contains(SECRET), "冻结载荷里有凭据明文: {payload}");
    assert!(!display.contains(SECRET), "审批视图里有凭据明文: {display}");

    // Debug 也不得泄漏（凭据链不进 Debug）。
    assert!(
        !format!("{:?}", tool).contains(SECRET),
        "Debug 里有凭据明文"
    );

    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(!rendered.contains(SECRET), "结果面里有凭据明文: {rendered}");
    // 鉴权**真的生效**了（服务器收到了 Bearer 头）。
    assert!(rendered.contains("auth-ok"), "{rendered}");
    server.await.unwrap();
}

#[tokio::test]
async fn a_remote_that_echoes_the_credential_back_is_masked_by_value() {
    let (port, server) = serve_once(|mut socket| async move {
        let (_head, _body) = read_request(&mut socket).await;
        // 远端把凭据**原样吐回来** ⇒ 明文绝不能进模型上下文。
        respond(&mut socket, "200 OK", &[], SECRET.as_bytes()).await;
    })
    .await;

    let tool = tool_with_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{port}/items"),
            "credential_ref": CREDENTIAL_NAME,
        })))
        .await;

    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(
        !rendered.contains(SECRET),
        "回显的凭据明文进了结果面: {rendered}"
    );
    let value = value_of(&result);
    assert_eq!(
        value["credential_echo_blocked"],
        json!(true),
        "回显必须可观测: {value}"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn a_credential_reference_must_be_explicitly_configured() {
    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": "http://127.0.0.1:1/items",
            "credential_ref": "not-configured",
        })))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(
        message.contains("not an explicitly configured binding"),
        "{message}"
    );
    assert!(!message.contains(SECRET), "{message}");
}

#[tokio::test]
async fn a_credential_is_never_sent_to_a_host_other_than_its_binding() {
    let (_listener, port) = bind_listener().await;

    // 绑定写的是 `api.example.com` ⇒ 请求打回环 = 换主机 = 拒绝。
    let config = HttpWriteConfig::new(
        Arc::new(egress(&[("127.0.0.1", None)])),
        resolver_with(SECRET),
    )
    .expect("the allow list policy is accepted")
    .with_binding(CredentialBinding::bearer(
        CREDENTIAL_NAME,
        "api.example.com",
    ))
    .expect("the binding is valid");
    let tool = HttpWriteTool::new(config);

    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{port}/items"),
            "credential_ref": CREDENTIAL_NAME,
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(message.contains("bound to host"), "{message}");
    assert!(!message.contains(SECRET), "{message}");
}

// ============================================================
// 对抗：白名单绕过
// ============================================================

#[tokio::test]
async fn allow_list_bypass_attempts_are_all_refused() {
    // 白名单里**只有** `allowed.example` ⇒ 任何真正落到别处的目的地都必须被拒。
    let config = HttpWriteConfig::new(
        Arc::new(egress(&[("allowed.example", None)])),
        resolver_with(SECRET),
    )
    .expect("the allow list policy is accepted");
    let tool = HttpWriteTool::new(config);

    for url in [
        // userinfo 混淆: 真实主机是 `evil.example`。
        "http://allowed.example@evil.example/",
        "http://allowed.example:secret@evil.example/",
        // 查询串夹带白名单域名: 真实主机是 `evil.example`。
        "https://evil.example/?x=allowed.example",
        // 主机名后缀拼接: `allowed.example.evil.example` 不是 `allowed.example`。
        "http://allowed.example.evil.example/",
        // 片段夹带: 片段永远不参与目的地判定。
        "http://evil.example/#allowed.example",
    ] {
        let result = tool
            .invoke(&call(json!({ "method": "post", "url": url })))
            .await;
        assert!(!result.is_ok(), "{url} 必须被拒");
        let message = error_of(&result);
        assert!(
            message.contains("userinfo")
                || message.contains("not on the explicit egress allow list"),
            "{url}: {message}"
        );
        // 拒绝必须发生在**目的地判定**上, 而不是「解析失败」这种偶然失败。
        assert!(
            !message.contains("resolution failed"),
            "{url}: 拒绝点错了: {message}"
        );
    }
}

#[tokio::test]
async fn a_url_with_userinfo_is_refused_without_leaking_the_password() {
    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": "http://user:hunter2@127.0.0.1/items",
        })))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(message.contains("userinfo"), "{message}");
    assert!(!message.contains("hunter2"), "{message}");
}

// ============================================================
// 重定向
// ============================================================

#[tokio::test]
async fn a_write_request_never_follows_a_redirect() {
    // 第二跳的监听器: 若写请求跟了重定向, 一定会连上它。
    let (second_listener, second_port) = bind_listener().await;

    let (port, server) = serve_once(move |mut socket| async move {
        let (_head, _body) = read_request(&mut socket).await;
        respond(
            &mut socket,
            "302 Found",
            &[("location", &format!("http://127.0.0.1:{second_port}/moved"))],
            b"",
        )
        .await;
    })
    .await;

    let tool = tool_without_binding(SECRET);
    let result = tool
        .invoke(&call(json!({
            "method": "post",
            "url": format!("http://127.0.0.1:{port}/items"),
        })))
        .await;

    assert!(!result.is_ok(), "{}", result.render());
    let message = error_of(&result);
    assert!(message.contains("do not follow redirects"), "{message}");

    // 第二跳**从未**被联系。
    let accepted = tokio::time::timeout(Duration::from_millis(200), second_listener.accept()).await;
    assert!(accepted.is_err(), "写请求跟了重定向 ⇒ 第二跳被联系了");
    server.await.unwrap();
}

// ============================================================
// 配置闸
// ============================================================

#[test]
fn the_write_face_refuses_a_policy_without_an_explicit_allow_list() {
    for policy in [
        EgressPolicy::DenyAll,
        EgressPolicy::PublicInternetOnly,
        EgressPolicy::Unrestricted,
    ] {
        let egress = ControlledEgress::new(policy);
        let result = HttpWriteConfig::new(Arc::new(egress), resolver_with(SECRET));
        assert!(result.is_err(), "写面不得在没有域名白名单时构造");
        let message = result.err().expect("the error").to_string();
        assert!(message.contains("explicit domain allow list"), "{message}");
    }
}

#[test]
fn the_method_vocabulary_is_closed() {
    assert_eq!(EgressMethod::ALL.len(), 4);
    for (raw, expected) in [
        ("post", EgressMethod::Post),
        ("PUT", EgressMethod::Put),
        (" patch ", EgressMethod::Patch),
        ("delete", EgressMethod::Delete),
    ] {
        assert_eq!(EgressMethod::parse(raw), Some(expected), "{raw}");
    }
    for raw in ["get", "options", "connect", "trace", "head", "", "postx"] {
        assert_eq!(EgressMethod::parse(raw), None, "{raw} 必须表外");
    }
}
