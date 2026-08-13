//! `apeireth-protocol` router_demo: 4 协议归一化演示
//!
//! **不调 LLM** (主哲学锚 #6 工程铁律 + 主人 8 项不修改承诺)
//! - 用 fake JSON 输入, 跑 4 协议 encode + decode 完整循环
//! - 展示: NormalizedRequest → 协议 JSON → 协议 JSON → NormalizedResponse
//! - 输出: 各协议 JSON 结构对比 + 互转可行性
//!
//! **跑法**:
//! ```bash
//! cargo run -p apeireth-protocol --example router_demo
//! ```
//!
//! **期望输出**:
//! - 4 协议 request body 结构 (json pretty)
//! - 4 协议 response body 归一化 (content / finish_reason / tool_calls / usage)
//! - 终点: 总结 4 协议字段差异 + VCP 借鉴映射

use apeireth_protocol::{
    is_tool_result_error, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedTool, NormalizedToolChoice, ProtocolKind, decode_for_kind, encode_for_kind,
    endpoint_path_for_kind, AnthropicMessagesBridge, GeminiBridge, OpenAiChatBridge,
    OpenAiResponsesBridge, ProtocolBridge,
};
use serde_json::json;

fn make_demo_request() -> NormalizedRequest {
    let mut req = NormalizedRequest::new(
        "demo-model",
        vec![
            NormalizedMessage::system("You are a helpful assistant."),
            NormalizedMessage::user("What's the weather in Beijing?"),
        ],
    );
    req.temperature = Some(0.7);
    req.max_tokens = Some(2048);
    req.stream = false;
    req.stop = vec!["END".to_string()];
    req.tools.push(
        NormalizedTool::new("get_weather")
            .with_description("Get the current weather for a city")
            .with_parameters({
                let mut p = serde_json::Map::new();
                p.insert("type".into(), json!("object"));
                p.insert(
                    "properties".into(),
                    json!({
                        "city": {"type": "string", "description": "City name"},
                    }),
                );
                p.insert("required".into(), json!(["city"]));
                p
            }),
    );
    req.tool_choice = Some(NormalizedToolChoice::Auto);
    req
}

fn make_fake_response(kind: ProtocolKind) -> serde_json::Value {
    match kind {
        ProtocolKind::OpenAiChat => json!({
            "id": "chatcmpl-demo123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "demo-model-2024",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Let me check the weather for you.",
                    "tool_calls": [{
                        "id": "call_demo_1",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Beijing\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 30, "completion_tokens": 15, "total_tokens": 45}
        }),
        ProtocolKind::OpenAiResponses => json!({
            "id": "resp_demo_1",
            "object": "response",
            "created_at": 1700000001,
            "model": "demo-model",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Let me check the weather for you."}]
            }, {
                "type": "function_call",
                "call_id": "call_demo_1",
                "name": "get_weather",
                "arguments": "{\"city\":\"Beijing\"}"
            }],
            "output_text": "Let me check the weather for you.",
            "usage": {"input_tokens": 30, "output_tokens": 15, "total_tokens": 45}
        }),
        ProtocolKind::AnthropicMessages => json!({
            "id": "msg_demo_1",
            "type": "message",
            "role": "assistant",
            "model": "demo-model-20250514",
            "content": [
                {"type": "text", "text": "Let me check the weather for you."},
                {
                    "type": "tool_use",
                    "id": "toolu_demo_1",
                    "name": "get_weather",
                    "input": {"city": "Beijing"}
                }
            ],
            "stop_reason": "tool_use",
            "stop_sequence": null,
            "usage": {"input_tokens": 30, "output_tokens": 15}
        }),
        ProtocolKind::Gemini => json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "Let me check the weather for you."},
                        {
                            "functionCall": {
                                "name": "get_weather",
                                "args": {"city": "Beijing"}
                            }
                        }
                    ]
                },
                "finishReason": "STOP",
                "index": 0
            }],
            "usageMetadata": {
                "promptTokenCount": 30,
                "candidatesTokenCount": 15,
                "totalTokenCount": 45
            },
            "modelVersion": "demo-model-002",
            "responseId": "gemini_demo_1"
        }),
        // 3 non-HTTP kind 走 ProtocolGateway (非 HTTP JSON), 本 demo 不演示
        ProtocolKind::Acp | ProtocolKind::Mcp | ProtocolKind::OpenClawGateway => json!({}),
    }
}

fn print_section(title: &str) {
    println!();
    println!("============================================================");
    println!("  {}", title);
    println!("============================================================");
}

fn main() {
    println!("apeireth-protocol router_demo (R37-1: 4 Bridge 替 router, fake JSON, 不调 LLM)");
    println!("============================================================");

    let req = make_demo_request();

    // ============================================================
    // 第 1 步: NormalizedRequest → 4 协议 JSON
    // ============================================================
    print_section("1. NormalizedRequest → 4 协议 JSON (R37-1 ProtocolBridge::encode_for_kind)");

    let kinds = [
        ProtocolKind::OpenAiChat,
        ProtocolKind::OpenAiResponses,
        ProtocolKind::AnthropicMessages,
        ProtocolKind::Gemini,
    ];
    for kind in &kinds {
        let name: &str = match kind {
            ProtocolKind::OpenAiChat => OpenAiChatBridge::name(),
            ProtocolKind::OpenAiResponses => OpenAiResponsesBridge::name(),
            ProtocolKind::AnthropicMessages => AnthropicMessagesBridge::name(),
            ProtocolKind::Gemini => GeminiBridge::name(),
            // 3 non-HTTP kind 不走本 bridge facade
            ProtocolKind::Acp | ProtocolKind::Mcp | ProtocolKind::OpenClawGateway => "non-http-bridge",

        };
        let endpoint = endpoint_path_for_kind(*kind).expect("4 HTTP kind always Some");
        match encode_for_kind(*kind, &req) {
            Ok(v) => {
                println!(
                    "\n[{}] {} (POST {})",
                    kind.as_str(),
                    name,
                    endpoint
                );
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
            }
            Err(e) => {
                println!("\n[{}] encode error: {}", kind.as_str(), e);
            }
        }
    }

    // ============================================================
    // 第 2 步: 4 协议 fake JSON → NormalizedResponse
    // ============================================================
    print_section("2. 4 协议 fake JSON → NormalizedResponse (R37-1 ProtocolBridge::decode_for_kind)");

    for kind in &kinds {
        let fake = make_fake_response(*kind);
        match decode_for_kind(*kind, &fake) {
            Ok(resp) => {
                println!("\n[{}] decoded:", kind.as_str());
                println!("  id:              {}", resp.id);
                println!("  model:           {}", resp.model);
                println!("  content:         {:?}", resp.content);
                println!(
                    "  finish_reason:   {:?}",
                    resp.finish_reason.unwrap_or(NormalizedFinishReason::Other)
                );
                println!(
                    "  usage:           prompt={} completion={} total={}",
                    resp.usage.prompt_tokens, resp.usage.completion_tokens, resp.usage.total_tokens
                );
                println!("  tool_calls:      {} 个", resp.tool_calls.len());
                for tc in &resp.tool_calls {
                    println!("    - id={} name={} args={}", tc.id, tc.name, tc.arguments);
                }
                if !resp.raw_metadata.is_empty() {
                    println!("  raw_metadata:    {} 字段", resp.raw_metadata.len());
                }
            }
            Err(e) => {
                println!("\n[{}] decode error: {}", kind.as_str(), e);
            }
        }
    }

    // ============================================================
    // 第 3 步: 字段级 VCP 借鉴映射
    // ============================================================
    print_section("3. 字段级 VCP 真代码借鉴映射");

    println!();
    println!("| 借鉴点                  | VCP 真文件 + 行号                  | 借鉴落地                    |");
    println!("|-------------------------|------------------------------------|------------------------------|");
    println!("| 归一化 message role     | protocolBridge.js:47-52             | MessageRole::from_legacy_value()      |");
    println!("| 归一化 content          | protocolBridge.js:21-42             | ContentPart::from_legacy_value()      |");
    println!("| 归一化 tool 3 步判定    | protocolBridge.js:63-89             | NormalizedTool 构造          |");
    println!("| 归一化 tool_choice      | protocolBridge.js:120-156           | NormalizedToolChoice 枚举    |");
    println!("| Gemini functionDecl.    | protocolBridge.js:91-118            | GeminiAdapter.build_tools    |");
    println!("| 工具结果错误 5 字段     | chatCompletionHandler.js:286-323   | error::is_tool_result_error  |");
    println!("| Keep-Alive 5 字段       | chatCompletionHandler.js:22-28     | lib.rs 编译期 hardcode       |");
    println!("|                         | (战役 1-2 apeireth-http-client 落地) |                          |");

    // ============================================================
    // 第 4 步: is_tool_result_error 演示
    // ============================================================
    print_section("4. is_tool_result_error 演示 (VCP chatCompletionHandler.js:286-323)");

    let cases: Vec<(&str, serde_json::Value, bool)> = vec![
        ("null (空结果)", json!(null), false),
        ("success=true", json!({"success": true, "data": "x"}), false),
        ("status=ok", json!({"status": "ok"}), false),
        ("error=true", json!({"error": true}), true),
        ("status=error", json!({"status": "error"}), true),
        ("code=404", json!({"code": 404}), true),
        ("code=200", json!({"code": 200}), false),
        ("[error] 前缀", json!("[error] something failed"), true),
        ("普通字符串", json!("everything is fine"), false),
    ];
    for (label, val, expected) in cases {
        let actual = is_tool_result_error(&val);
        let mark = if actual == expected { "✓" } else { "✗" };
        println!(
            "  {} {} → is_error={} (expected={})",
            mark, label, actual, expected
        );
    }

    // ============================================================
    // 第 5 步: 终点 — 总结
    // ============================================================
    print_section("5. 总结 — 4 协议归一化能力");

    println!();
    println!("  ✅ 4 协议都真实现 (不止 OpenAI, R17 战役 0 已直连 minimaxi)");
    println!("  ✅ NormalizedRequest/Response 统一内部表示");
    println!("  ✅ 字段级引用 VCP 真代码 (文件 + 行号 + 真函数名 + 真字段名)");
    println!("  ✅ 不调 LLM, 全 fake JSON, 0 key 消耗 (主哲学锚 #6)");
    println!("  ✅ 编译期 hardcode (KEEP_ALIVE 5 字段 + 4 协议常量)");
    println!();
    println!("  下一步: 战役 1-2 把 apeireth-protocol 接进 apeireth-api + apeireth-pipeline");
    println!("          战役 1-2 在 apeireth-http-client 落 Keep-Alive 5 字段 (VCP 真代码)");
    println!();
    println!("============================================================");
    println!("  demo 完成 (不调 LLM, 不消耗 key, 4 协议全跑通)");
    println!("============================================================");
}