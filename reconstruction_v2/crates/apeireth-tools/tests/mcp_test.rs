use std::sync::Arc;
use apeireth_tools::ToolRegistry;
use apeireth_tools::builtin::shell::ShellTool;
use apeireth_tools::builtin::filesystem::FilesystemTool;
use apeireth_tools::mcp::{McpServer, McpClient, MemoryTransport};

#[tokio::test]
async fn test_mcp_client_server_roundtrip_and_tool_adapter() {
    // 1. Setup Server with Native Apeireth Tools
    let server_registry = ToolRegistry::new();
    server_registry.register(Arc::new(ShellTool::new()));

    server_registry.register(Arc::new(FilesystemTool::new()));

    let server = Arc::new(McpServer::with_info(
        Arc::new(server_registry),
        "apeireth-core-host",
        "2.0.0",
    ));

    // 2. Setup In-Memory Direct Channel Transport Pair
    let (transport, mut req_rx, resp_tx) = MemoryTransport::pair(32);

    // Spawn server request processing loop
    let server_clone = server.clone();
    tokio::spawn(async move {
        while let Some(req) = req_rx.recv().await {
            let resp = server_clone.handle_request(req).await;
            if resp_tx.send(resp).await.is_err() {
                break;
            }
        }
    });

    // 3. Setup MCP Client
    let client = Arc::new(McpClient::new(Arc::new(transport)));

    // 4. Test MCP Handshake (initialize)
    let init_res = client.initialize("test-agent", "0.1.0").await.expect("Initialize failed");
    assert_eq!(init_res.protocol_version, "2024-11-05");
    assert_eq!(init_res.server_info.name, "apeireth-core-host");
    assert!(init_res.capabilities.tools.is_some());

    // 5. Test Tools Discovery (tools/list)
    let tools = client.list_tools().await.expect("tools/list failed");
    assert!(tools.len() >= 2);
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"shell".to_string()));
    assert!(tool_names.contains(&"filesystem".to_string()));



    // 6. Test Direct Tool Call (tools/call)
    let call_res = client.call_tool(
        "shell",
        serde_json::json!({
            "command": "echo 'Hello from MCP Protocol'"
        })
    ).await.expect("tools/call failed");
    assert!(!call_res.is_error);
    assert!(!call_res.content.is_empty());

    // 7. Test Resources Discovery (resources/list)
    let resources = client.list_resources().await.expect("resources/list failed");
    assert!(resources.iter().any(|r| r.uri == "apeireth://memory/act-r"));

    // 8. Test Dynamic Tool Adapter & Auto-Registration into secondary Agent ToolRegistry
    let mut agent_registry = ToolRegistry::new();
    let registered_count = McpClient::discover_and_register_tools(client.clone(), &mut agent_registry).await.expect("Discover failed");
    assert_eq!(registered_count, tools.len());

    // Execute through secondary ToolRegistry via McpToolAdapter
    let agent_exec_res = agent_registry.execute(
        "shell",
        serde_json::json!({
            "command": "echo 'Executed through McpToolAdapter'"
        })
    ).await.expect("Agent execution failed");

    assert!(agent_exec_res.success);
    assert!(agent_exec_res.output.contains("Executed through McpToolAdapter"));
}
