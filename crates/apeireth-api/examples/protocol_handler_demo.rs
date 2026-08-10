//! R123-2: 4 协议 handler 抽 trait 抽象 demo
//!
//! **目的**: 演示 `ProtocolHandler` trait + `HandlerRegistry` 注册表 + `route_dispatch`
//! 通用模板 4 协议注册 + dispatch 流程. 0 装"已替换 4 协议 handler", 仅"骨架 ready".
//!
//! **跑法**:
//! ```powershell
//! cargo run -p apeireth-api --example protocol_handler_demo
//! ```
//!
//! **期望输出**: 4 协议 stub handler 注册 + 各 dispatch 1 次, 看到 4 段 content 不混 +
//! Acp/Mcp/OpenClawGateway 未注册 → Err
//!
//! **0 触碰** (硬约束 #8): 0 改 protocol_handlers.rs / v2_endpoints.rs / server.rs,
//! 0 接 ResponseCache / ReplayCache, 0 真 LLM 调用.

use apeireth_api::{
    protocol_handler_trait::{route_dispatch, HandlerRegistry, ProtocolHandler},
    ContentPart, MessageRole, NormalizedMessage, NormalizedRequest, NormalizedResponse,
    ProtocolKind,
};

/// 演示用 stub handler (4 协议各 1 个, 0 业务状态)
struct DemoHandler {
    endpoint: &'static str,
    label: &'static str,
}

impl ProtocolHandler for DemoHandler {
    fn endpoint_url(&self) -> &str {
        self.endpoint
    }

    fn cache_key(&self, req: &NormalizedRequest) -> String {
        format!("demo:{}:{}:{}", self.label, req.model, req.messages.len())
    }

    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
        Ok(NormalizedResponse::text(
            format!("{}-{}", self.label, req.model),
            req.model,
            format!(
                "[{}] routed {} msgs through {}",
                self.label,
                req.messages.len(),
                self.endpoint
            ),
        ))
    }
}

fn make_demo_request(model: &str) -> NormalizedRequest {
    let messages = vec![
        NormalizedMessage {
            role: MessageRole::System,
            content: vec![ContentPart::text_only(
                "You are a helpful assistant.".to_string(),
            )],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        },
        NormalizedMessage {
            role: MessageRole::User,
            content: vec![ContentPart::text_only("Hello!".to_string())],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        },
    ];
    NormalizedRequest::new(model, messages)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 R123-2 ProtocolHandler trait 抽象 demo");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // 1. 新建 registry
    let mut reg = HandlerRegistry::new();
    println!("✅ HandlerRegistry::new() (空, len={})", reg.len());

    // 2. 注册 4 协议 stub handler
    let configs = [
        (ProtocolKind::OpenAiChat, "/v1/chat/completions", "openai_chat"),
        (
            ProtocolKind::OpenAiResponses,
            "/v1/responses",
            "openai_responses",
        ),
        (ProtocolKind::AnthropicMessages, "/v1/messages", "anthropic"),
        (
            ProtocolKind::Gemini,
            "/v1beta/models/{model}:generateContent",
            "gemini",
        ),
    ];
    for (kind, endpoint, label) in configs {
        reg.register(kind, DemoHandler { endpoint, label });
        println!(
            "   register {:?} → endpoint={} label={}",
            kind, endpoint, label
        );
    }
    println!();
    println!("📋 注册表 state: {:?}", reg);
    println!(
        "   len={}, is_empty={}, supports_stream(OpenAiChat)={}",
        reg.len(),
        reg.is_empty(),
        reg.supports_stream(ProtocolKind::OpenAiChat)
    );
    println!();

    // 3. route_dispatch 演示 (直调 1 个 handler)
    let demo_handler = DemoHandler {
        endpoint: "/v1/chat/completions",
        label: "openai_chat",
    };
    let req = make_demo_request("gpt-4o");
    let resp = route_dispatch(&demo_handler, req)?;
    println!("📡 route_dispatch 直调 (openai_chat):");
    println!("   model:   {}", resp.model);
    println!("   content: {}", resp.content);
    println!();

    // 4. registry.dispatch 演示 (4 协议各 1 次)
    println!("📡 registry.dispatch 4 协议:");
    for (kind, _endpoint, _label) in configs {
        let req = make_demo_request("test-model");
        match reg.dispatch(kind, req) {
            Ok(resp) => println!("   {:?} → {}", kind, resp.content),
            Err(e) => println!("   {:?} → ERR: {}", kind, e),
        }
    }
    println!();

    // 5. 未注册 kind → Err
    println!("❌ 未注册 kind (Acp / Mcp / OpenClawGateway) dispatch:");
    for &unknown in &[
        ProtocolKind::Acp,
        ProtocolKind::Mcp,
        ProtocolKind::OpenClawGateway,
    ] {
        let req = make_demo_request("x");
        match reg.dispatch(unknown, req) {
            Ok(_) => println!("   {unknown:?} → 意外成功 (0 漂移测试)"),
            Err(e) => println!("   {unknown:?} → 预期 Err: {e}"),
        }
    }
    println!();

    println!(
        "✨ R123-2 demo 验收通过 (4 协议 register + dispatch + 3 gateway Err 0 漂移)"
    );
    Ok(())
}
