use std::sync::Arc;
use apeireth_runtime::UnifiedRuntimeHost;
use apeireth_gateway::server::start_server;

fn get_default_api_key() -> String {
    let key_file = r"C:\Users\31683\apikey-ultra.txt";
    if let Ok(key) = std::fs::read_to_string(key_file) {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    std::env::var("MINIMAX_API_KEY").unwrap_or_else(|_| "sk-dummy".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bind_addr = std::env::var("APEIRETH_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let db_path = std::env::var("APEIRETH_DB").unwrap_or_else(|_| "apeireth_gateway.db".to_string());
    let api_key = get_default_api_key();

    println!("===============================================================================");
    println!("             APEIRETH 2.0 LIVING COMPANION GATEWAY SERVER                      ");
    println!("===============================================================================");
    println!("Initializing UnifiedRuntimeHost with ACT-R Memory, 5-Gate Governance & MCP...");
    
    let host = Arc::new(UnifiedRuntimeHost::new(api_key, &db_path).await?);
    println!("✓ UnifiedRuntimeHost initialized successfully.");
    println!("✓ Database attached: {}", db_path);
    println!("✓ Endpoints mounted:");
    println!("  - REST Health:            GET  http://{}/health", bind_addr);
    println!("  - Model List:             GET  http://{}/v1/models", bind_addr);
    println!("  - OpenAI Compatible Chat: POST http://{}/v1/chat/completions", bind_addr);
    println!("  - Anthropic Standard MCP: POST http://{}/mcp", bind_addr);
    println!("  - Full-Duplex WebSocket:  GET  ws://{}/ws", bind_addr);
    println!("-------------------------------------------------------------------------------");
    println!("Apeireth Gateway listening on http://{} (Press Ctrl+C to stop)", bind_addr);
    println!("===============================================================================");

    start_server(&bind_addr, Some(host)).await
}
