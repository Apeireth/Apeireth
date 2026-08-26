//! `config_demo` — TOML 配置驱动演示
//!
//! 不需要 API key (用 ScriptedLlmProvider mock)
//!
//! 跑法: cargo run -p apeireth-api --example config_demo

use apeireth_api::llm::{ChatMessage, LlmConfig, LlmProvider, LlmRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 通用 API 扩展平台 — config_demo");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let toml_content = r#"
[providers.primary]
type = "scripted"
api_key_env = "APEIRETH_LLM_NO_KEY"
models = ["MiniMax-M3"]
scripts = { "hello" = "hi from PRIMARY", "goodbye" = "bye from PRIMARY" }
default_response = "[primary] default"

[providers.secondary]
type = "scripted"
api_key_env = "APEIRETH_LLM_NO_KEY"
models = ["gpt-4o"]
scripts = { "hello" = "hi from SECONDARY" }
default_response = "[secondary] default"

[router]
fallback_order = ["primary", "secondary"]
    "#;

    println!("\n📋 TOML 配置:");
    println!("{}", toml_content);

    let config = LlmConfig::from_str(toml_content)?;
    println!("✅ 解析成功:");
    println!("   providers: {}", config.providers.len());
    println!("   fallback_order: {:?}", config.router.fallback_order);

    std::env::set_var("APEIRETH_LLM_NO_KEY", "placeholder");

    let router = config.build_router()?;
    println!("\n🔗 Router 状态:");
    println!("   provider count: {}", router.provider_count());
    println!("   provider names: {:?}", router.provider_names());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 测试 1: 'hello' (primary 命中)");
    let req = LlmRequest::new("MiniMax-M3", vec![ChatMessage::user("hello world")]);
    let resp = router.complete(req).await?;
    println!("✅ 响应: {}", resp.content);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 测试 2: model='gpt-4o' (只有 secondary 支持)");
    let req = LlmRequest::new("gpt-4o", vec![ChatMessage::user("hello")]);
    let resp = router.complete(req).await?;
    println!("✅ 响应: {}", resp.content);

    println!("\n✨ config_demo 跑通");

    Ok(())
}
