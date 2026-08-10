//! **apeireth-mcp / multimodal_mcp_demo — 多模态 dispatcher 模板演示 (R123-4)**
//!
//! **依据**: docs/v2-strategy/07 §2 P2-13 (多模态生成 via MCP, 9 个 Gen 插件)
//!
//! **演示内容**:
//! 1. 列出 9 个 Gen 插件 + 各自 endpoint URL 模板 (0 真连, 仅占位)
//! 2. 演示 6 个 output format + mime type + extension
//! 3. 用 `McpServer::register_tool(def, handler_from_fn(dispatch_multimodal_handler))` 把 multimodal 注册到 mcp
//! 4. client 走 `initialize` → `list_tools` (看到 `multimodal` 工具) → `call_tool("multimodal", ...)` 返 isError=true
//! 5. 0 真接, call 永远 isError=true 提示 "plugin X not connected" (O-5 诚实标缺)
//!
//! **运行**: `cargo run -p apeireth-mcp --example multimodal_mcp_demo`
//!
//! **不漂移 (O-5)**:
//! - ✅ 0 HTTP client 调用 (multimodal dispatcher 永远 Err)
//! - ✅ 0 触碰 11 agent 公共 API 签名
//! - ✅ 0 改 Cargo.toml `[package]` / `[dependencies]`
//! - ❌ 0 真接 9 plugin (R124+ 真接)

use apeireth_mcp::multimodal::{
    dispatch_multimodal_handler, dispatch_multimodal_handler_async, gen_dispatch, multimodal_tool_def, GenPlugin, GenRequest,
    OutputFormat, DEFAULT_SIZE, GEN_PLUGIN_COUNT, OUTPUT_FORMAT_COUNT,
};
use apeireth_mcp::tool_bridge::handler_from_fn;
use apeireth_mcp::transport::MemoryTransport;
use apeireth_mcp::{McpClient, McpServer};
use serde_json::{json, Value};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-mcp multimodal_mcp_demo (R123-4 template) ===\n");

    // ----- 1. 列出 9 个 Gen 插件 + endpoint URL 模板 -----
    println!("→ 9 Gen plugins (per 07 §2 P2-13, VCP Plugin/*Gen*/ 借鉴):");
    for plugin in GenPlugin::ALL.iter() {
        let url = apeireth_mcp::multimodal::plugin_endpoint(*plugin);
        println!(
            "  - {:<13}  →  {}\n      desc: {}",
            plugin.as_str(),
            url,
            plugin.description()
        );
    }
    assert_eq!(GEN_PLUGIN_COUNT, 9, "GEN_PLUGIN_COUNT must be 9");
    println!("  ✓ {} plugins, 0 真接 (template placeholder)\n", GEN_PLUGIN_COUNT);

    // ----- 2. 演示 6 个 output format + mime type + extension -----
    println!("→ 6 output formats (image 3 + video 1 + web 1 + 3d 1):");
    for fmt in OutputFormat::ALL.iter() {
        println!(
            "  - {:<6}  →  mime={:<20} ext={}",
            format!("{:?}", fmt).to_lowercase(),
            fmt.mime_type(),
            fmt.extension()
        );
    }
    assert_eq!(OUTPUT_FORMAT_COUNT, 6, "OUTPUT_FORMAT_COUNT must be 6");
    println!("  ✓ {} formats\n", OUTPUT_FORMAT_COUNT);

    // ----- 3. 演示 GenRequest builder + 序列化 -----
    println!("→ GenRequest demo (Flux + cyberpunk cat + Webp):");
    let req = GenRequest::new(GenPlugin::Flux, "a cyberpunk cat wearing neon")
        .with_seed(42)
        .with_size(512, 768)
        .with_output_format(OutputFormat::Webp);
    println!("  size: {:?} (resolve_size → {:?})", req.size, req.resolve_size());
    let req_json = serde_json::to_string_pretty(&req)?;
    println!("  JSON:\n{req_json}\n");
    assert_eq!(DEFAULT_SIZE, (1024, 1024), "DEFAULT_SIZE must be 1024x1024");

    // ----- 4. gen_dispatch 演示 (O-5 永远 Err) -----
    println!("→ gen_dispatch (template, 0 真接, 永远 Err):");
    match gen_dispatch(req.clone()) {
        Ok(_) => println!("  ✗ UNEXPECTED: dispatch returned Ok (would mean R124 真接)"),
        Err(e) => println!("  ✓ got expected error: {e}"),
    }
    println!();

    // ----- 5. 用 mcp 走端到端: register multimodal tool → list → call -----
    println!("→ mcp end-to-end demo: server + client via MemoryTransport");
    let mut server = McpServer::new("apeireth-mcp-multimodal-demo");
    let def = multimodal_tool_def();
    let handler = handler_from_fn(dispatch_multimodal_handler_async);
    server.register_tool(def, handler);

    let (server_side, client_side) = tokio::io::duplex(8192);
    let server_task = tokio::spawn(async move {
        let transport = MemoryTransport::new(server_side);
        if let Err(e) = server.run_with_transport(transport).await {
            eprintln!("[server] task failed: {e}");
        }
        Ok::<(), apeireth_mcp::McpError>(())
    });

    let mut client = McpClient::with_transport(MemoryTransport::new(client_side));

    // 5.1 initialize
    let info = client.initialize().await?;
    println!("  ✓ initialize: {} v{}", info.serverInfo.name, info.serverInfo.version);

    // 5.2 list_tools — 应该看到 multimodal
    let defs = client.list_tools().await?;
    println!("  ✓ list_tools: {} tool(s) available:", defs.len());
    for d in &defs {
        println!("    - {} ({})", d.name, &d.description[..60.min(d.description.len())]);
    }
    assert!(defs.iter().any(|d| d.name == "multimodal"), "multimodal tool should be listed");

    // 5.3 call_tool("multimodal", ...) — 走 dispatch_multimodal_handler, 返 Err
    println!("\n→ call_tool multimodal (Flux, prompt=cyberpunk cat):");
    let call_args: Value = json!({
        "plugin": "flux",
        "prompt": "a cyberpunk cat wearing neon",
        "seed": 42,
        "size": [512, 768],
        "output_format": "webp"
    });
    // McpClient.call_tool 内部: 如果 isError=true 返 Err(McpError::Tool(msg))
    match client.call_tool("multimodal", call_args).await {
        Ok(out) => println!("  ✗ UNEXPECTED Ok: {out:#}"),
        Err(e) => println!("  ✓ got expected Err: {e}"),
    }
    println!("  (R123-4 template 0 真接, R124+ 真接后这里会返 GenResponse JSON)\n");

    // 5.4 试 9 plugin 全部 call, 全部应返 Err
    println!("→ smoke test: 9 plugin 全部 call (应全部 Err):");
    for plugin in GenPlugin::ALL.iter() {
        let args = json!({"plugin": plugin.as_str(), "prompt": "smoke test"});
        match client.call_tool("multimodal", args).await {
            Ok(_) => println!("  ✗ {plugin:?} returned Ok (UNEXPECTED)"),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not connected") {
                    println!("  ✓ {:<13} → not connected (O-5 标缺)", plugin.as_str());
                } else {
                    println!("  ? {:<13} → other err: {e}", plugin.as_str());
                }
            }
        }
    }

    // ----- 6. 关闭 client, server EOF 后退出 -----
    drop(client);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
        .await
        .expect("server task did not finish within timeout")
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)??;

    println!("\n=== done (R123-4 template, 0 真接, R124+ 真接) ===");
    Ok(())
}
