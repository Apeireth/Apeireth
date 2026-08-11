# apeireth-api

> **Apeireth 自研 API 接入平台** — 直连 Anthropic + OpenAI Chat + OpenAI Responses + Gemini 4 协议 + axum HTTP server + minimax 真接.
> **当前状态**: R128 真接 minimax, 4 协议 100% 跑通, 127 lib warnings (历史 missing_docs, 0 effect).
> **默认 base_url**: `https://api.minimaxi.com` (Anthropic 子路径 `/anthropic`).

---

## 公共 API

### LLM Providers

```rust
use apeireth_api::llm::{LlmProvider, LlmRequest, ChatMessage};

let provider = AnthropicCompatibleProvider::new(config)?;
let req = LlmRequest::new("MiniMax-M3", vec![
    ChatMessage::system("You are a Rust assistant"),
    ChatMessage::user("Explain async runtime"),
]);
let resp = provider.complete(req).await?;
// resp.content, resp.usage, resp.finish_reason
```

#### 4 个 LLM provider

| Provider | base URL | Protocol |
|---|---|---|
| `AnthropicCompatibleProvider` | `https://api.minimaxi.com/anthropic` | Anthropic Messages API (x-api-key header) |
| `ApeirethApiProvider` | `https://api.minimaxi.com/v1` | minimax 原生 |
| `OpenAiCompatibleProvider` | `https://api.minimaxi.com/v1` | OpenAI Chat Completions |
| `ScriptedLlmProvider` | mock (test only) | scriptable mock for tests |

#### LlmRequest / LlmResponse

```rust
pub struct LlmRequest {
    pub model: String,           // e.g. "MiniMax-M3"
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,        // 0.0 - 2.0
    pub max_tokens: u32,
    pub trace_id: Option<u64>,
    pub stop: Vec<String>,
}

pub struct LlmResponse {
    pub content: String,
    pub usage: TokenUsage,       // prompt + completion + total
    pub model: String,
    pub provider: String,
    pub finish_reason: String,
    pub latency_ms: u64,
}
```

### HTTP server (axum)

```bash
# 起 server 走 minimax (require APEIRETH_API_KEY env var)
$env:APEIRETH_API_KEY = "<minimax key>"
cargo run -p apeireth-api --example serve
# 默认监听 :8080, 路由: /v1/chat/completions, /v1/responses, /anthropic/v1/messages, /v1/tools/...

# mock backend (不需要 key)
$env:APEIRETH_LLM_BACKEND = "scripted"
cargo run -p apeireth-api --example serve
```

### Examples (R128 真接验证)

| Example | 协议 | Status |
|---|---|---|
| `openai_chat` | OpenAI Chat Completions | ✅ 3 round Keep-Alive LIFO |
| `openai_responses` | OpenAI Responses API | ✅ 1740ms latency |
| `anthropic_hello` | Anthropic Messages | ✅ 3325ms latency |
| `gemini` | Gemini | ✅ |
| `openai_stream` | OpenAI Chat streaming | ✅ |
| `serve` | axum HTTP server | ✅ |
| `v2_smoke` | 9 endpoint e2e (scripted) | ✅ |
| `protocol_handler_demo` | 4 协议 facade | ✅ |

## 依赖

- `apeireth-core` (顶层)
- `apeireth-protocol` (4 协议归一化)
- `apeireth-http-client` (Keep-Alive LIFO)
- `apeireth-pipeline` (5 步管线)
- `apeireth-host` (keyring for API key storage)
- `apeireth-telemetry` (trace + log)

## 验证

```bash
cargo check -p apeireth-api          # 0 errors
cargo run -p apeireth-api --example openai_chat    # 真接 minimax
```

## See also

- [minimax 4 协议验证报告](../../reports/minimax-end-to-end-r128-2026-08-12.md)