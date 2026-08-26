//! **apeireth-mcp / hello — client + server 端到端互调示例**
//!
//! **依据**: docs/v2-strategy/05 §Step 2 验收: `cargo run -p apeireth-mcp --example hello` 跑通
//!
//! **演示流程**:
//! 1. 用 `tokio::io::duplex` 建一对内存 pipe
//! 2. server 端: 注册 2 个工具 (echo / add), 作为 tokio task 跑 `run_with_transport`
//! 3. client 端: 调 `initialize` → `list_tools` → `call_tool("echo", ...)` → `call_tool("add", ...)`
//! 4. 打印结果
//!
//! **不依赖子进程** — 用 `MemoryTransport` 跑端到端, 验证协议 / 桥接 / dispatch 全链路.
//!
//! **运行**: `cargo run -p apeireth-mcp --example hello`

use std::sync::Arc;

use apeireth_mcp::transport::MemoryTransport;
use apeireth_mcp::{McpClient, McpServer, ToolDef};
use apeireth_tool_registry::MockSyncTool;
use apeireth_tool_registry::Tool;
use serde_json::{json, Value};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-mcp hello example ===\n");

    // ----- 1. 建 server, 注册 2 个工具 -----
    let mut server = McpServer::new("apeireth-mcp-hello");

    // 1.1 echo 工具 — 直接用 tool-registry 的 MockSyncTool (走桥接)
    let echo_tool: Arc<dyn Tool> = Arc::new(MockSyncTool {
        name: "echo".to_string(),
    });
    server.register_tool_from_arc(Arc::clone(&echo_tool));

    // 1.2 add 工具 — 自定义 handler (从两个 int 求和)
    let add_def = ToolDef {
        name: "add".to_string(),
        description: "Sum two integers (a + b)".to_string(),
        inputSchema: json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer", "description": "first addend"},
                "b": {"type": "integer", "description": "second addend"},
            },
            "required": ["a", "b"],
        }),
    };
    let add_handler = apeireth_mcp::tool_bridge::handler_from_fn(|args: Value| async move {
        let a = args
            .get("a")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing args.a (integer)".to_string())?;
        let b = args
            .get("b")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing args.b (integer)".to_string())?;
        Ok(json!({"sum": a + b, "a": a, "b": b}))
    });
    server.register_tool(add_def, add_handler);

    // ----- 2. 用 tokio::io::duplex 建一对内存 pipe -----
    let (server_side, client_side) = tokio::io::duplex(8192);

    // ----- 3. server 作为后台 task 跑 -----
    let server_task = tokio::spawn(async move {
        let transport = MemoryTransport::new(server_side);
        if let Err(e) = server.run_with_transport(transport).await {
            eprintln!("[server] task failed: {e}");
        }
        Ok::<(), apeireth_mcp::McpError>(())
    });

    // ----- 4. client 端走握手 + 调用 -----
    let mut client = McpClient::with_transport(MemoryTransport::new(client_side));

    println!("→ initialize");
    let info = client.initialize().await?;
    println!(
        "  ✓ serverInfo = {} v{}, protocolVersion = {}",
        info.serverInfo.name, info.serverInfo.version, info.protocolVersion,
    );

    println!("\n→ list_tools");
    let defs = client.list_tools().await?;
    println!("  ✓ {} tool(s) available:", defs.len());
    for d in &defs {
        println!(
            "    - {} ({})",
            d.name,
            if d.description.is_empty() {
                "no description"
            } else {
                d.description.as_str()
            }
        );
    }

    println!("\n→ call_tool echo {{\"msg\": \"hello world\"}}");
    let out = client
        .call_tool("echo", json!({"msg": "hello world"}))
        .await?;
    println!("  ✓ result = {out:#}");

    println!("\n→ call_tool add {{\"a\": 17, \"b\": 25}}");
    let out = client.call_tool("add", json!({"a": 17, "b": 25})).await?;
    println!("  ✓ result = {out:#}");

    // ----- 5. 关闭 client, server EOF 后退出 -----
    drop(client);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
        .await
        .expect("server task did not finish within timeout")
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)??;

    println!("\n=== done ===");
    Ok(())
}
