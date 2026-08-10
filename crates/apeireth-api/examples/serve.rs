//! `serve` — 启动 apeireth-api HTTP server (R17 战役 1-4 升级)
//!
//! R17 战役 1-4 升级: 4 协议端点 (走 `apeireth_pipeline::Pipeline` 5 步管线 +
//! `apeireth_http_client::HttpClient` Keep-Alive LIFO 5 字段 +
//! `apeireth_protocol::ProtocolBridge` 4 协议 facade (R37-1, R36-2 删 ProtocolRouter))
//!
//! 跑法: `cargo run -p apeireth-api --example serve`
//!
//! 需要 env vars:
//! - `APEIRETH_API_KEY` (真 minimaxi LLM key) 或 `APEIRETH_LLM_BACKEND=scripted` (mock)
//! - `APEIRETH_API_URL` (可选, base URL 默认 `https://api.minimaxi.com`)
//! - `PORT` (默认 8080)

use std::sync::Arc;

use apeireth_api::{
    llm::{
        providers::scripted::{ScriptedLlmProvider, ScriptedResponse},
        ApeirethApiConfig, ApeirethApiProvider, LlmProvider,
    },
    protocol_handlers,
    server::{build_router, AppState},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let base_url = std::env::var("APEIRETH_API_URL")
        .unwrap_or_else(|_| protocol_handlers::MINIMAXI_BASE_URL.to_string());

    // 1. 构造 LLM provider (real via APEIRETH_API_KEY 或 scripted fallback)
    let llm: Arc<dyn LlmProvider> = match std::env::var("APEIRETH_LLM_BACKEND").as_deref() {
        Ok("scripted") | Ok("mock") => {
            tracing::info!("Using ScriptedLlmProvider (mock backend)");
            let scripted = ScriptedLlmProvider::new("scripted-mock")
                .with_script("hello", ScriptedResponse::new("hi from scripted mock"))
                .with_script("safe", ScriptedResponse::new("approve: 议题安全"))
                .with_script("danger", ScriptedResponse::new("reject: 检测到风险"));
            Arc::new(scripted)
        }
        _ => {
            let llm_config = ApeirethApiConfig::from_env()?;
            let real = ApeirethApiProvider::new(llm_config)?;
            tracing::info!("Using real LLM provider: {}", real.name());
            Arc::new(real)
        }
    };

    // 2. 构造 Pipeline (战役 1-3 5 步管线 + 战役 1-2 Keep-Alive LIFO 5 字段)
    let auth_token = std::env::var("APEIRETH_API_KEY").ok();
    let pipeline = protocol_handlers::build_pipeline(base_url.clone(), auth_token.clone())?;
    let pipeline = Arc::new(pipeline);

    // 3. 共享 state (R17 战役 1-4: 4 协议 pipeline + 1 legacy LlmProvider for /council/advise)
    let state = Arc::new(AppState {
        pipeline,
        llm,
        // R120 (B2 战区 2): serve 默认开启 cache (生产用)
        // response_cache: None,  // 改这里关 cache
        response_cache: None, // 临时: keep 1.0 行为 0 漂移
    });

    // 4. 启动 server
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("apeireth-api listening on http://0.0.0.0:{port}");

    println!("✅ Apeireth 自研 API 接入平台 HTTP server (R17 战役 1-4)");
    println!("   base_url: {base_url}");
    println!(
        "   auth:     {}",
        if auth_token.is_some() {
            "Bearer token"
        } else {
            "no token"
        }
    );
    println!();
    println!("   GET  /health");
    println!("   POST /v1/chat/completions          (OpenAI Chat Completions)");
    println!("   POST /v1/responses                (OpenAI Responses API / codex)");
    println!("   POST /v1/messages                 (Anthropic Messages)");
    println!("   POST /v1beta/models/{{model}}:generateContent  (Google Gemini)");
    println!("   POST /council/advise              (R17 战役 0 保留)");
    println!("   POST /verdict                     (R17 战役 0 保留)");
    println!();
    println!("❌ /channels endpoint 已砍掉 (R17 去掉 NewAPI channel 概念)");

    axum::serve(listener, app).await?;
    Ok(())
}

