//! **apeireth-mcp / 多 transport 集成测试**
//!
//! **目标**: 验证 3 种 transport (Memory / SSE / HTTP-streamable) 都能跑通 JSON-RPC 端到端。
//!
//! **方法**:
//! - Memory: `tokio::io::duplex` + `McpClient::with_transport` + `McpServer::run_with_transport`
//! - SSE / HTTP-streamable: `tokio::net::TcpListener` 起 mock HTTP server
//!
//! **不假装**: stdio 子进程模式不测 (spawn 不可控, 已在 hello example 验证);
//!   SSE / HTTP-streamable 用真 TCP server, wire-level JSON 字段断言。

use apeireth_mcp::transport::{
    connect, HttpStreamableTransport, MemoryTransport, SseTransport, Transport, TransportKind,
};
use apeireth_mcp::{McpClient, McpServer, ServerInfo};
use apeireth_tool_registry::{Tool, ToolAxes, ToolKind, ToolRegistry};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

// ============================================================
// 共享: 1 个 echo + 1 个 add tool
// ============================================================

struct EchoTool;
#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        Ok(json!({"echo": args}))
    }
}

struct AddTool;
#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str {
        "add"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let a = args["a"].as_i64().unwrap_or(0);
        let b = args["b"].as_i64().unwrap_or(0);
        Ok(json!({"a": a, "b": b, "sum": a + b}))
    }
}

fn build_registry() -> Arc<ToolRegistry> {
    let reg = ToolRegistry::new();
    reg.register("echo".to_string(), Arc::new(EchoTool));
    reg.register("add".to_string(), Arc::new(AddTool));
    Arc::new(reg)
}

// ============================================================
// 测试 1: Memory Transport 端到端
// ============================================================

#[tokio::test]
async fn memory_transport_end_to_end() {
    let (a, b) = tokio::io::duplex(4096);
    let client_t = MemoryTransport::new(a);
    let server_t = MemoryTransport::new(b);

    let registry = build_registry();
    let server = McpServer::from_registry("mcp-test", registry);
    let server_handle = tokio::spawn(async move {
        server.run_with_transport(server_t).await.unwrap();
    });

    let mut client = McpClient::with_transport(client_t);
    let info = client.initialize().await.unwrap();
    assert_eq!(info.serverInfo.name, "mcp-test");
    assert_eq!(info.protocolVersion, "2025-03-26");
    assert!(
        info.capabilities.tools.is_some(),
        "tools capability present"
    );

    let tools = client.list_tools().await.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"echo"), "should list echo tool");
    assert!(names.contains(&"add"), "should list add tool");

    let r = client
        .call_tool("echo", json!({"msg": "hi"}))
        .await
        .unwrap();
    assert_eq!(r["echo"]["msg"], "hi");

    let r = client
        .call_tool("add", json!({"a": 7, "b": 13}))
        .await
        .unwrap();
    assert_eq!(r["sum"], 20);

    drop(client); // 关闭 transport
    server_handle.await.unwrap();
}

// ============================================================
// 测试 2: SSE Transport wire-level (用 mock TCP server)
// ============================================================

async fn run_mock_sse_server(listener: TcpListener) {
    let (mut sock, _) = listener.accept().await.unwrap();
    let mut reader = BufReader::new(&mut sock);

    // 读 HTTP 请求头 (到空行)
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.unwrap() == 0 {
            return;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
    }

    // 写 SSE 响应头
    sock.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\r\n",
    )
    .await
    .unwrap();

    // 推 endpoint 帧
    sock.write_all(b"event: endpoint\ndata: /messages?sessionId=test123\n\n")
        .await
        .unwrap();
    sock.flush().await.unwrap();

    // 推 mock initialize 响应
    sock.write_all(b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-03-26\",\"serverInfo\":{\"name\":\"mock-sse\",\"version\":\"1.0.0\"},\"capabilities\":{\"tools\":{\"listChanged\":false}}}}\n\n").await.unwrap();
    sock.flush().await.unwrap();

    // 推 mock tools/list 响应
    sock.write_all(b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\",\"description\":\"E\",\"inputSchema\":{}}]}}\n\n").await.unwrap();
    sock.flush().await.unwrap();

    // 保持连接一会儿
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = sock.shutdown().await;
}

#[tokio::test]
async fn sse_transport_end_to_end_via_mock_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        run_mock_sse_server(listener).await;
    });

    let url = format!("http://{addr}/sse");
    let mut client_t = SseTransport::connect(&url).await.unwrap();

    // 验证 endpoint 已获取 (从首帧 event:endpoint 解析)
    let ep = client_t.endpoint_url().await;
    assert!(ep.is_some(), "endpoint should be set from SSE stream");
    assert!(
        ep.as_ref().unwrap().contains("sessionId=test123"),
        "endpoint URL should contain session id"
    );

    // 收第 1 帧 message (mock initialize 响应)
    let line = client_t.recv().await.unwrap();
    assert!(line.is_some(), "should receive first message");
    let v: Value = serde_json::from_str(&line.unwrap()).unwrap();
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["id"], 1);
    assert_eq!(v["result"]["serverInfo"]["name"], "mock-sse");

    // 收第 2 帧 message (mock tools/list 响应)
    let line = client_t.recv().await.unwrap();
    assert!(line.is_some());
    let v: Value = serde_json::from_str(&line.unwrap()).unwrap();
    assert_eq!(v["id"], 2);
    assert_eq!(v["result"]["tools"][0]["name"], "echo");

    client_t.close().await.unwrap();
    server_handle.await.unwrap();
}

// ============================================================
// 测试 3: HTTP Streamable Transport wire-level (mock TCP server)
// ============================================================

async fn run_mock_http_server(listener: TcpListener) {
    use tokio::io::AsyncReadExt;
    let (mut sock, _) = listener.accept().await.unwrap();
    let mut reader = BufReader::new(&mut sock);

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.unwrap() == 0 {
            return;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    // 读 body
    let mut body_buf = vec![0u8; content_length];
    reader.read_exact(&mut body_buf).await.unwrap();
    let body_str = String::from_utf8_lossy(&body_buf);

    let req: Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
    let id = req["id"].as_i64().unwrap_or(0);
    let method = req["method"].as_str().unwrap_or("");

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-03-26",
            "serverInfo": {"name": "mock-http", "version": "1.0.0"},
            "capabilities": {"tools": {"listChanged": false}}
        }),
        "tools/list" => json!({
            "tools": [
                {"name": "echo", "description": "Echo", "inputSchema": {}},
                {"name": "add", "description": "Add", "inputSchema": {}}
            ]
        }),
        "tools/call" => {
            let params = &req["params"];
            let tool_name = params["name"].as_str().unwrap_or("");
            match tool_name {
                "echo" => json!({"echo": "echoed"}),
                "add" => {
                    let a = params["arguments"]["a"].as_i64().unwrap_or(0);
                    let b = params["arguments"]["b"].as_i64().unwrap_or(0);
                    json!({"sum": a + b})
                }
                _ => json!({"err": "unknown tool"}),
            }
        }
        _ => json!({}),
    };

    let resp_body = json!({ "jsonrpc": "2.0", "id": id, "result": result });
    let resp_str = resp_body.to_string();
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        resp_str.len(),
        resp_str
    );
    sock.write_all(http.as_bytes()).await.unwrap();
    sock.flush().await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let _ = sock.shutdown().await;
}

#[tokio::test]
async fn http_streamable_transport_end_to_end_via_mock_server() {
    // 启 1 个 mock server (每个 server 只 accept 一个连接)
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { run_mock_http_server(l).await });

    let endpoint = format!("http://{addr}/mcp");
    let mut client_t = HttpStreamableTransport::connect(endpoint).unwrap();

    // initialize
    client_t
        .send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .unwrap();
    let line = client_t.recv().await.unwrap();
    assert!(line.is_some(), "initialize should get response");
    let v: Value = serde_json::from_str(&line.unwrap()).unwrap();
    assert_eq!(v["result"]["serverInfo"]["name"], "mock-http");

    client_t.close().await.unwrap();
}

// ============================================================
// 测试 4: TransportKind::connect 工厂 — HTTPStreamable 路径
// ============================================================

#[tokio::test]
async fn transport_kind_factory_http_streamable() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let server_h = tokio::spawn(async move { run_mock_http_server(l).await });

    let mut boxed: Box<dyn Transport> = connect(TransportKind::HttpStreamable {
        url: format!("http://{addr}/mcp"),
    })
    .await
    .unwrap();

    boxed
        .send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .unwrap();
    let line = boxed.recv().await.unwrap();
    assert!(line.is_some());
    let v: Value = serde_json::from_str(&line.unwrap()).unwrap();
    assert_eq!(v["id"], 1);
    assert_eq!(v["result"]["serverInfo"]["name"], "mock-http");

    boxed.close().await.unwrap();
    server_h.await.unwrap();
}

// ============================================================
// 测试 5: TransportKind 所有枚举变体可构造 (字段级)
// ============================================================

#[test]
fn transport_kind_variants_construct() {
    let _ = TransportKind::Stdio {
        cmd: "node".into(),
        args: vec!["server.js".into()],
    };
    let _ = TransportKind::StdioCurrent;
    let _ = TransportKind::Sse {
        url: "https://example.com/sse".into(),
    };
    let _ = TransportKind::HttpStreamable {
        url: "https://example.com/mcp".into(),
    };
    let _ = TransportKind::Memory;
}

// ============================================================
// 测试 6: TransportKind::Memory 工厂应返回 NotImplemented
// ============================================================

#[tokio::test]
async fn transport_kind_memory_factory_errors() {
    let r = connect(TransportKind::Memory).await;
    assert!(r.is_err());
}

// ============================================================
// 测试 7: ServerInfo 字段 (字段级参考 MCP 2025-03-26 §Initialize)
// ============================================================

#[test]
fn serverinfo_fields_match_mcp_spec() {
    let info = ServerInfo::for_server("test");
    assert_eq!(info.serverInfo.name, "test");
    assert_eq!(info.protocolVersion, "2025-03-26");
    assert!(info.capabilities.tools.is_some());

    // 直接构造验证字段独立性
    let custom = ServerInfo::for_server("custom");
    assert_eq!(custom.serverInfo.name, "custom");
    assert_eq!(custom.serverInfo.version.len() > 0, true);
}

// ============================================================
// 测试 8: SSE Transport endpoint URL absolutize (relative path)
// ============================================================

#[tokio::test]
async fn sse_transport_absolutize_endpoint_in_mock() {
    // Mock server 给相对路径 endpoint
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(&mut sock);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await.unwrap() == 0 {
                return;
            }
            if line == "\r\n" || line == "\n" {
                break;
            }
        }
        // 写 SSE 头 + endpoint 帧 (相对路径)
        sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .await
            .unwrap();
        sock.write_all(b"event: endpoint\ndata: /relative/endpoint\n\n")
            .await
            .unwrap();
        sock.flush().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = sock.shutdown().await;
    });

    let url = format!("http://{addr}/sse");
    let client_t = SseTransport::connect(&url).await.unwrap();
    let ep = client_t.endpoint_url().await;
    assert!(ep.is_some());
    // 相对路径应当被绝对化到 base URL origin
    let ep_str = ep.unwrap();
    assert!(
        ep_str.contains("relative/endpoint"),
        "relative endpoint should be absolutized: {ep_str}"
    );
    assert!(ep_str.starts_with("http://"), "should have http scheme");

    server_handle.await.unwrap();
}

// ============================================================
// 测试 9: HTTPStreamable Mcp-Session-Id 字段级 (MCP 2025-03-26 §Streamable HTTP)
// ============================================================

async fn run_mock_http_server_with_session(listener: TcpListener) {
    use tokio::io::AsyncReadExt;
    let (mut sock, _) = listener.accept().await.unwrap();
    let mut reader = BufReader::new(&mut sock);
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.unwrap() == 0 {
            return;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body_buf = vec![0u8; content_length];
    reader.read_exact(&mut body_buf).await.unwrap();

    let resp_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"protocolVersion": "2025-03-26", "serverInfo": {"name": "s", "version": "1"}, "capabilities": {}}
    });
    let resp_str = resp_body.to_string();
    // 服务端回 Mcp-Session-Id 头
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: sess-xyz-789\r\nContent-Length: {}\r\n\r\n{}",
        resp_str.len(),
        resp_str
    );
    sock.write_all(http.as_bytes()).await.unwrap();
    sock.flush().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let _ = sock.shutdown().await;
}

#[tokio::test]
async fn http_streamable_session_id_header() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let server_h = tokio::spawn(async move { run_mock_http_server_with_session(l).await });

    let mut client_t = HttpStreamableTransport::connect(format!("http://{addr}/mcp")).unwrap();
    assert!(client_t.session_id().await.is_none());

    client_t
        .send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .unwrap();
    let _ = client_t.recv().await.unwrap();

    // server 应自动 set session id
    let sid = client_t.session_id().await;
    assert!(
        sid.is_some(),
        "session id should be set after Initialize response"
    );
    assert_eq!(sid.unwrap(), "sess-xyz-789");

    client_t.close().await.unwrap();
    server_h.await.unwrap();
}
