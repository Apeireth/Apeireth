//! R33-3-1: 3 真接 ResourceServer 端到端 demo (File + Organ + Convention)
//!
//! 跑法: `cargo run -p apeireth-mcp --example resource_servers_demo`
//!
//! **目的**: 不真接 MCP transport (stdio/SSE), 直接在 in-process 跑
//! `resources/list` + `resources/read` JSON-RPC handler, 验证 3 个 server
//! 真接 + Composite 路由正确。

use apeireth_mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use apeireth_mcp::resources::{handle_resources_list, handle_resources_read};
use apeireth_mcp::{
    CompositeResourceServer, ConventionResourceServer, FileResourceServer, OrganResourceServer,
};

fn main() {
    // 1) 起 3 个 server
    let workspace = std::env::current_dir().expect("current_dir");
    let file =
        FileResourceServer::new(&workspace).expect("FileResourceServer 构造 (workspace 必须存在)");
    let organ = OrganResourceServer::new();
    let conv = ConventionResourceServer::new(&workspace);
    let composite = CompositeResourceServer::new()
        .with_file(file)
        .with_organ(organ)
        .with_convention(conv);

    // 2) 模拟 `resources/list` JSON-RPC 请求
    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(apeireth_mcp::protocol::Id::Num(1)),
        method: "resources/list".to_string(),
        params: None,
    };
    let list_resp: JsonRpcResponse = handle_resources_list(&list_req, &composite);
    let list_body = serde_json::to_string_pretty(&list_resp.result.unwrap()).unwrap();
    println!("=== resources/list ===\n{list_body}\n");

    // 3) 模拟 `resources/read` 3 个不同 scheme 的 URI
    let reads = [
        "organ://_all",
        "convention://_summary",
        "convention://_system_prompt_block",
    ];
    for (i, uri) in reads.iter().enumerate() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(apeireth_mcp::protocol::Id::Num(2 + i as i64)),
            method: "resources/read".to_string(),
            params: Some(serde_json::json!({ "uri": uri })),
        };
        let resp = handle_resources_read(&req, &composite);
        let body = serde_json::to_string_pretty(&resp.result.unwrap()).unwrap();
        println!("=== resources/read {uri} ===\n{body}\n");
    }
}
