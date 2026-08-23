use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use std::sync::Arc;
use apeireth_tools::mcp::{JsonRpcRequest, McpServer};

use apeireth_tools::ToolRegistry;
use crate::server::GatewayState;

pub async fn mcp_handler(
    State(state): State<Arc<GatewayState>>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let server = match state.runtime_host {
        Some(ref host) => McpServer::with_info(
            host.tool_registry.clone(),
            "apeireth-gateway-mcp",
            "2.0.0",
        ),
        None => {
            let registry = Arc::new(ToolRegistry::new());
            McpServer::with_info(registry, "apeireth-gateway-mcp", "2.0.0")
        }
    };

    let resp = server.handle_request(req).await;
    Json(resp)
}
