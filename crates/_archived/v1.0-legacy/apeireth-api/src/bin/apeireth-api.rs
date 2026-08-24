//! `apeireth-api` 二进制入口 (R27 C 方案: 独立 server 进程)
//!
//! 让 `cargo run -p apeireth-api` 直接起 daemon, 多前端共享:
//! - TUI: 连 http://127.0.0.1:8080/v1 (onboarding 提示)
//! - Web 前端 / 桌面 App: 连同 server, 多端可同时操作同一会话状态
//!
//! 复用 `examples/serve.rs` 逻辑: 4 协议端点 + V2 6 类 JSON + Keep-Alive LIFO 5 步管线.
//! 跑法: `cargo run -p apeireth-api --release`
//! 关掉: Ctrl+C (前台) 或杀进程
//!
//! env vars:
//! - `APEIRETH_PORT` (默认 8080)
//! - `APEIRETH_API_URL` (默认 https://api.minimaxi.com)
//! - `APEIRETH_API_KEY` (真 LLM key, 必填除非 APEIRETH_LLM_BACKEND=scripted)
//! - `APEIRETH_LLM_BACKEND=scripted` (mock 模式, 无需 key)

use std::sync::Arc;

use apeireth_api::{
    llm::{
        providers::scripted::{ScriptedLlmProvider, ScriptedResponse},
        ApeirethApiConfig, ApeirethApiProvider, LlmConfig, LlmProvider, SemanticRouter,
    },
    protocol_handlers,
    server::{build_router_with_v2, AppState},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("APEIRETH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let base_url = std::env::var("APEIRETH_API_URL")
        .unwrap_or_else(|_| protocol_handlers::MINIMAXI_BASE_URL.to_string());

    // R30 U7: 三档启动路径
    // 1) APEIRETH_LLM_CONFIG env 指向 toml → 走 LlmConfig.build_semantic_router (按 description 余弦相似度选 route)
    // 2) APEIRETH_LLM_BACKEND=scripted → 单 ScriptedLlmProvider mock
    // 3) 默认 → 单 ApeirethApiProvider (兼容老行为)
    let llm: Arc<dyn LlmProvider> = if let Ok(config_path) = std::env::var("APEIRETH_LLM_CONFIG") {
        let config =
            LlmConfig::from_file(&config_path).map_err(|e| format!("read {config_path}: {e}"))?;
        if let Some(sr) = config.build_semantic_router()? {
            tracing::info!(
                "Using SemanticRouter from {config_path} ({} routes)",
                config
                    .semantic_routes
                    .as_ref()
                    .map(|c| c.routes.len())
                    .unwrap_or(0)
            );
            println!(
                "   llm:      SemanticRouter ({} routes, from {})",
                config
                    .semantic_routes
                    .as_ref()
                    .map(|c| c.routes.len())
                    .unwrap_or(0),
                config_path
            );
            Arc::new(sr) as Arc<dyn LlmProvider>
        } else {
            // 配置了 toml 但没 semantic_routes, fallback 用 build_router
            let router = config.build_router()?;
            tracing::info!(
                "Using MultiLlmRouter from {config_path} ({} providers)",
                router.provider_count()
            );
            println!(
                "   llm:      MultiLlmRouter ({} providers, from {})",
                router.provider_count(),
                config_path
            );
            Arc::new(router) as Arc<dyn LlmProvider>
        }
    } else {
        match std::env::var("APEIRETH_LLM_BACKEND").as_deref() {
            Ok("scripted") | Ok("mock") => {
                tracing::info!("Using ScriptedLlmProvider (mock backend)");
                let scripted = ScriptedLlmProvider::new("scripted-mock")
                    .with_script("hello", ScriptedResponse::new("hi from scripted mock"))
                    .with_script("safe", ScriptedResponse::new("approve: 议题安全"))
                    .with_script("danger", ScriptedResponse::new("reject: 检测到风险"));
                println!("   llm:      ScriptedLlmProvider (mock)");
                Arc::new(scripted) as Arc<dyn LlmProvider>
            }
            _ => {
                let llm_config = ApeirethApiConfig::from_env()?;
                let real = ApeirethApiProvider::new(llm_config)?;
                tracing::info!("Using real LLM provider: {}", real.name());
                println!("   llm:      {} (real upstream)", real.name());
                Arc::new(real) as Arc<dyn LlmProvider>
            }
        }
    };

    let auth_token = std::env::var("APEIRETH_API_KEY").ok();
    let pipeline = protocol_handlers::build_pipeline(base_url.clone(), auth_token.clone())?;
    let pipeline = Arc::new(pipeline);

    let state = Arc::new(AppState {
        pipeline,
        llm,
        // R120 (B2 战区 2): 默认 None (1.0 行为 0 漂移)
        // 启用用 AppState::with_response_cache().await
        response_cache: None,
    });

    // R30 P0: Initialize V2 6 类服务 — Tools 是 AI 触手, 必须开
    // register_all 把 4 真工具 (WebSearch / FileOperator / Git / ShellExec) 塞进 ToolRegistry
    let v2_state = Arc::new(apeireth_api::v2_endpoints::V2State::new());
    {
        let registry = Arc::new(apeireth_tool_registry::ToolRegistry::new());
        match apeireth_tools::register_all(&registry) {
            Ok(()) => {
                let names = apeireth_tools::registered_tool_names();
                tracing::info!(names = ?names, "V2 tools registered");
                println!(
                    "   tools:    {} registered ({})",
                    names.len(),
                    names.join(", ")
                );
                v2_state.install_tools(registry);
            }
            Err(e) => eprintln!("[apeireth-api] WARN: register_all failed: {e}"),
        }
    }

    let app = build_router_with_v2(state, v2_state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("apeireth-api listening on http://0.0.0.0:{port}");

    println!("Apeireth 自研 API 接入平台 HTTP server (R27 C 方案: 独立 daemon)");
    println!("   listen:    http://0.0.0.0:{port}");
    println!("   base_url:  {base_url}");
    println!(
        "   auth:      {}",
        if auth_token.is_some() {
            "Bearer token"
        } else {
            "no token"
        }
    );
    println!();
    println!("   多前端可同时连这个 server (TUI / Web / 桌面 App)");
    println!();
    println!("   走 TUI:  Base URL 填 http://127.0.0.1:{port}/v1");
    println!();
    println!("   GET  /health");
    println!("   POST /v1/chat/completions          (OpenAI Chat Completions)");
    println!("   POST /v1/responses                (OpenAI Responses API / codex)");
    println!("   POST /v1/messages                 (Anthropic Messages)");
    println!("   POST /v1beta/models/{{model}}:generateContent  (Google Gemini)");
    println!("   POST /council/advise              (R17 战役 0 保留)");
    println!("   POST /verdict                     (R17 战役 0 保留)");
    println!("   GET  /v1/tools/list               (R30 P0: AI 真工具注册表)");
    println!("   POST /v1/tools/invoke              (R30 P0: AI 调用 FileOperator/Git/ShellExec/WebSearch)");
    println!();
    println!("   启动模式:");
    println!("     默认: 1 个 apeireth-api provider (兼容老行为)");
    println!("     APEIRETH_LLM_BACKEND=scripted  1 个 mock (无 key)");
    println!("     APEIRETH_LLM_CONFIG=path.toml  N providers + 余弦相似度语义路由");
    println!();
    println!("   Ctrl+C 退出");

    axum::serve(listener, app).await?;
    Ok(())
}
