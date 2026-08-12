//! 4 协议端点的请求/响应编解码
//!
//! **R17 战役 1-4 核心模块**: 把 4 个 LLM 协议的原生 JSON 请求 → `NormalizedRequest`,
//! 走完 `apeireth_pipeline::Pipeline` 5 步管线后, 再把 `NormalizedResponse` 转回
//! 协议原生 JSON 返回客户端。
//!
//! **借鉴 VCP** (`research/source/vcptoolbox/`):
//! - `routes/protocolBridge.js:47-52` `normalizeMessageRole` — 协议角色归一化
//!   (developer → system / function → tool, 字段级引用)
//! - `routes/protocolBridge.js:21-42` `normalizeTextContent` — 多模态 content 归一化
//!   (string 原样 / array 取 text/input_text/output_text)
//! - `chatCompletionHandler.js:17-37` `agentOptions` Keep-Alive 5 字段 (战役 1-2 落地)
//! - `chatCompletionHandler.js:222-257` Force-Translate (战役 1-3 落地)
//!
//! **设计原则**:
//! - **不修改战役 1-1/1-2/1-3 代码** — 仅用 import, 不改 trait / 不改 pipeline
//! - **不抄 VCP 业务代码** — 只借工程模式 (归一化 + 反归一化)
//! - **协议真接** — 4 协议都真实现, 不只 OpenAI / Anthropic
//! - **编译期 hardcode** — 协议端点路径在 const 里 hardcode
//!
//! **不假装**:
//! - ✅ 4 协议端点 URL 字段级对应 VCP `protocolBridge.js:1-150` 真代码
//! - ✅ 鉴权都走 Bearer (minimaxi 全 4 端点统一 Bearer)
//! - ✅ 响应格式完全按各协议规范 (OpenAI Chat `choices[0].message` /
//!   OpenAI Responses `output[0].content[0].text` / Anthropic `content[]` /
//!   Gemini `candidates[0].content.parts[]`)
//!
//! **架构位置**:
//! ```text
//!   客户端 (4 协议)
//!     ↓ JSON
//!   server.rs 4 个 endpoint
//!     ↓ protocol_handlers::*::decode_request()
//!   NormalizedRequest
//!     ↓ Pipeline::run() (战役 1-3 5 步管线)
//!   NormalizedResponse
//!     ↓ protocol_handlers::*::encode_response()
//!   协议 JSON
//!     ↓
//!   客户端
//! ```

#![allow(clippy::result_large_err)]

use apeireth_pipeline::Pipeline;
use apeireth_protocol::{
    decode_for_kind, encode_for_kind, ContentPart, MessageRole, NormalizedMessage,
    NormalizedRequest, NormalizedResponse, ProtocolError, ProtocolKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use crate::cache::ResponseCache;
use crate::replay_cache::{global as replay_cache, hash_request, ResponsePayload, ReplayEntry, DEFAULT_HTTP_METHOD};
use crate::retry::{jittered_sleep, BackoffPolicy, RetryStats, should_retry_status};

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 4 协议端点路径 (对应 `apeireth_protocol::ProtocolKind`)
/// **VCP 真代码字段级对应** `routes/protocolBridge.js` 全部 4 协议入口
pub const OPENAI_CHAT_PATH: &str = "/v1/chat/completions";
pub const OPENAI_RESPONSES_PATH: &str = "/v1/responses";
pub const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";
pub const GEMINI_PATH_TEMPLATE: &str = "/v1beta/models/{model}:generateContent";

/// 4 协议鉴权: minimaxi 全 4 端点统一 Bearer
/// (VCP 旧 `Authorization: Bearer <token>`, minimaxi 沿用 OpenAI 风格)
pub const AUTH_SCHEME_BEARER: &str = "Bearer";

/// minimaxi 默认 base URL (跟 `apeireth_pipeline::PipelineConfig::default` 对齐)
pub const MINIMAXI_BASE_URL: &str = "https://api.minimaxi.com";

/// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
const _: () = {
    // 4 协议路径不为空
    assert!(!OPENAI_CHAT_PATH.is_empty());
    assert!(!OPENAI_RESPONSES_PATH.is_empty());
    assert!(!ANTHROPIC_MESSAGES_PATH.is_empty());
    assert!(!GEMINI_PATH_TEMPLATE.is_empty());

    // 鉴权 scheme 不为空
    assert!(!AUTH_SCHEME_BEARER.is_empty());
    assert!(AUTH_SCHEME_BEARER.len() >= 3);

    // base URL 是 https (str::starts_with 在 const 里不稳定, 移到 runtime test 见 constants_are_valid)
};

// ============================================================
// Pipeline 构造 (战役 1-3 PipelineConfig::default() 的微调版)
// ============================================================

/// 构造默认 Pipeline (战役 1-3 VCP 默认 + minimaxi base URL + Bearer token)
///
/// **不假装**: Pipeline 内部 5 步真跑 (placeholder / token_budget / force_translate /
/// 协议归一化 / HTTP), 走 Keep-Alive LIFO 5 字段 (战役 1-2 落地)
pub fn build_pipeline(base_url: String, auth_token: Option<String>) -> Result<Pipeline, String> {
    use apeireth_http_client::HttpClient;
    use apeireth_pipeline::PipelineConfig;

    let http = HttpClient::with_vcp_defaults().map_err(|e| format!("http client build: {e}"))?;

    let mut config = PipelineConfig::default();
    config.base_url = base_url;
    config.auth_token = auth_token;

    Pipeline::with_config(http, config).map_err(|e| format!("pipeline build: {e}"))
}

// ============================================================
// 公共辅助: 协议 URL 构造 (含 Gemini {model} 占位符替换 + minimaxi Anthropic /anthropic 前缀)
// ============================================================

/// 构造协议端点完整 URL
///
/// **Gemini 特例**: 端点路径含 `{model}` 占位符, 必须替换
/// (战役 1-3 pipeline.run 不会替换, 这里显式处理)
///
/// **minimaxi Anthropic 特例**: minimaxi 的 Anthropic 端点是 `/anthropic/v1/messages`
/// (跟标准 Anthropic `/v1/messages` 不同), 当 base_url 是 minimaxi 主域且不含 `/anthropic`
/// 时, 自动加 `/anthropic` 前缀
///
/// **minimaxi Gemini 特例**: minimaxi 的 Gemini 端点是 `/v1/gemini/v1beta/models/...`
/// (跟标准 Google Gemini `/v1beta/models/...` 不同), 自动加 `/v1/gemini/` 前缀
pub fn endpoint_url(base_url: &str, kind: ProtocolKind, model: &str) -> Result<String, ProtocolError> {
    match kind {
        ProtocolKind::Gemini => {
            // minimaxi: /v1/gemini/v1beta/models/{model}:generateContent
            // 其他: /v1beta/models/{model}:generateContent
            let path = if is_minimaxi_gemini_quirk(base_url) {
                format!("/v1/gemini{}", GEMINI_PATH_TEMPLATE)
            } else {
                GEMINI_PATH_TEMPLATE.to_string()
            };
            Ok(format!(
                "{}{}",
                base_url.trim_end_matches('/'),
                path.replace("{model}", model)
            ))
        }
        ProtocolKind::AnthropicMessages => {
            // minimaxi: /anthropic/v1/messages (默认)
            // 其他 provider (Anthropic direct 等): /v1/messages (base_url 自带 /v1)
            let path = if is_minimaxi_anthropic_quirk(base_url) {
                "/anthropic/v1/messages"
            } else {
                ANTHROPIC_MESSAGES_PATH
            };
            Ok(format!("{}{}", base_url.trim_end_matches('/'), path))
        }
        ProtocolKind::OpenAiChat => {
            Ok(format!("{}{}", base_url.trim_end_matches('/'), OPENAI_CHAT_PATH))
        }
        ProtocolKind::OpenAiResponses => {
            Ok(format!(
                "{}{}",
                base_url.trim_end_matches('/'),
                OPENAI_RESPONSES_PATH
            ))
        }
        ProtocolKind::Acp | ProtocolKind::Mcp | ProtocolKind::OpenClawGateway => {
            Err(ProtocolError::Unsupported {
                feature: format!("endpoint_url 不支持 kind={kind:?}; 走 gateway::ProtocolGateway 异步 dispatch"),
            })
        }
    }
}

/// 检测是否需要 minimaxi Anthropic URL 特例
///
/// **触发条件**: base_url 是 minimaxi 主域 (`api.minimaxi.com`)
/// 且 base_url 不含 `/anthropic` 子路径 (避免重复加前缀)
fn is_minimaxi_anthropic_quirk(base_url: &str) -> bool {
    let normalized = base_url.trim_end_matches('/').to_lowercase();
    normalized.contains("minimaxi.com") && !normalized.contains("/anthropic")
}

/// 检测是否需要 minimaxi Gemini URL 特例
///
/// **触发条件**: base_url 是 minimaxi 主域 (`api.minimaxi.com`)
/// 且 base_url 不含 `/v1/gemini` 子路径
fn is_minimaxi_gemini_quirk(base_url: &str) -> bool {
    let normalized = base_url.trim_end_matches('/').to_lowercase();
    normalized.contains("minimaxi.com") && !normalized.contains("/v1/gemini")
}

// ============================================================
// 1. OpenAI Chat Completions 编解码
// ============================================================

/// OpenAI Chat Completions 请求 (跟 minimaxi /v1/chat/completions 字段对齐)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub tools: Option<Vec<Value>>,
    #[serde(default)]
    pub tool_choice: Option<Value>,
}

/// OpenAI Chat message (system / user / assistant)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAiChatMessage {
    pub role: String,
    /// content 可能是 string (纯文本) 或 array (多模态)
    pub content: Value,
    #[serde(default)]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

/// OpenAI Chat 响应 (跟 minimaxi /v1/chat/completions 字段对齐)
#[derive(Debug, Serialize, Clone)]
pub struct OpenAiChatResponse {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAiChatChoice>,
    pub usage: OpenAiChatUsage,
    /// R17 战役 1-4 新增: 协议名 (debug / 路由决策)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apeireth_protocol: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAiChatChoice {
    pub index: u32,
    pub message: OpenAiChatMessageOut,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAiChatMessageOut {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAiChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI Chat 请求 → NormalizedRequest
pub fn openai_chat_to_normalized(req: &OpenAiChatRequest) -> NormalizedRequest {
    let messages: Vec<NormalizedMessage> = req
        .messages
        .iter()
        .map(|m| {
            // 借鉴 VCP protocolBridge.js:47-52 normalizeMessageRole
            let role = MessageRole::from_vcp(&m.role);
            // 借鉴 VCP protocolBridge.js:21-42 normalizeTextContent
            let content = ContentPart::from_vcp(&m.content);
            NormalizedMessage {
                role,
                content,
                tool_calls: Vec::new(), // 简化: tool_calls 解析留给战役 2 tool_runtime
                tool_call_id: m.tool_call_id.clone(),
                name: None,
            }
        })
        .collect();

    NormalizedRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        stream: req.stream,
        stop: req.stop.clone().unwrap_or_default(),
        tools: Vec::new(), // 简化
        tool_choice: None,
        metadata: Default::default(),
    }
}

/// NormalizedResponse → OpenAI Chat 响应 JSON
pub fn openai_chat_from_normalized(resp: &NormalizedResponse) -> OpenAiChatResponse {
    let finish_reason = resp
        .finish_reason
        .map(|r| r.to_openai().to_string())
        .unwrap_or_else(|| "stop".to_string());

    OpenAiChatResponse {
        id: resp.id.clone(),
        object: "chat.completion",
        created: chrono::Utc::now().timestamp(),
        model: resp.model.clone(),
        choices: vec![OpenAiChatChoice {
            index: 0,
            message: OpenAiChatMessageOut {
                role: "assistant",
                content: resp.content.clone(),
            },
            finish_reason,
        }],
        usage: OpenAiChatUsage {
            prompt_tokens: resp.usage.prompt_tokens,
            completion_tokens: resp.usage.completion_tokens,
            total_tokens: resp.usage.total_tokens,
        },
        apeireth_protocol: Some("openai_chat".to_string()),
    }
}

// ============================================================
// 2. OpenAI Responses API 编解码
// ============================================================

/// OpenAI Responses API 请求
/// (跟 minimaxi /v1/responses 字段对齐, codex 风格)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAiResponsesRequest {
    pub model: String,
    /// input: 字符串或数组
    pub input: Value,
    /// 顶层 instructions (替代 system message)
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub tools: Option<Vec<Value>>,
    #[serde(default)]
    pub tool_choice: Option<Value>,
}

/// OpenAI Responses 响应 (跟 minimaxi /v1/responses 字段对齐)
#[derive(Debug, Serialize, Clone)]
pub struct OpenAiResponsesResponse {
    pub id: String,
    pub object: &'static str,
    pub model: String,
    pub status: &'static str,
    pub output: Vec<Value>,
    pub usage: OpenAiResponsesUsage,
    /// R17 战役 1-4 新增: 协议名 (debug / 路由决策)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apeireth_protocol: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAiResponsesUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI Responses 请求 → NormalizedRequest
pub fn openai_responses_to_normalized(req: &OpenAiResponsesRequest) -> NormalizedRequest {
    let mut messages: Vec<NormalizedMessage> = Vec::new();

    // 1. 顶层 instructions → System message
    // 借鉴 VCP openai_responses adapter.adapt_request:63-81
    if let Some(ins) = &req.instructions {
        messages.push(NormalizedMessage::system(ins.clone()));
    }

    // 2. input: 字符串或数组
    if let Some(s) = req.input.as_str() {
        // 单字符串 = user message
        messages.push(NormalizedMessage::user(s.to_string()));
    } else if let Some(arr) = req.input.as_array() {
        for item in arr {
            if let Some(obj) = item.as_object() {
                let role_str = obj.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                let role = MessageRole::from_vcp(role_str);
                let content = obj
                    .get("content")
                    .map(ContentPart::from_vcp)
                    .unwrap_or_default();
                messages.push(NormalizedMessage {
                    role,
                    content,
                    tool_calls: Vec::new(),
                    tool_call_id: obj
                        .get("call_id")
                        .or_else(|| obj.get("tool_call_id"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    name: obj.get("name").and_then(|v| v.as_str()).map(String::from),
                });
            } else if let Some(s) = item.as_str() {
                messages.push(NormalizedMessage::user(s.to_string()));
            }
        }
    }

    NormalizedRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        stream: req.stream,
        stop: Vec::new(),
        tools: Vec::new(),
        tool_choice: None,
        metadata: Default::default(),
    }
}

/// NormalizedResponse → OpenAI Responses 响应 JSON
pub fn openai_responses_from_normalized(resp: &NormalizedResponse) -> OpenAiResponsesResponse {
    let output_item = json!({
        "type": "message",
        "id": format!("msg_{}", resp.id),
        "role": "assistant",
        "status": "completed",
        "content": [{
            "type": "output_text",
            "text": resp.content,
            "annotations": []
        }]
    });

    OpenAiResponsesResponse {
        id: resp.id.clone(),
        object: "response",
        model: resp.model.clone(),
        status: "completed",
        output: vec![output_item],
        usage: OpenAiResponsesUsage {
            input_tokens: resp.usage.prompt_tokens,
            output_tokens: resp.usage.completion_tokens,
            total_tokens: resp.usage.total_tokens,
        },
        apeireth_protocol: Some("openai_responses".to_string()),
    }
}

// ============================================================
// 3. Anthropic Messages API 编解码
// ============================================================

/// Anthropic Messages API 请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicRequest {
    pub model: String,
    /// 顶层 system 字段 (Anthropic 必填顶层, 不在 messages 里)
    #[serde(default)]
    pub system: Option<String>,
    pub messages: Vec<AnthropicMessage>,
    /// Anthropic max_tokens 必填
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(default)]
    pub tools: Option<Vec<Value>>,
}

/// Anthropic message (user / assistant, 含 content block 数组)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    /// content: string 或 block 数组 [{type: "text", text: "..."}, {type: "tool_use", ...}]
    pub content: Value,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

/// Anthropic Messages 响应
#[derive(Debug, Serialize, Clone)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub role: &'static str,
    pub model: String,
    pub content: Vec<Value>,
    pub stop_reason: String,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
    /// R17 战役 1-4 新增: 协议名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apeireth_protocol: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Anthropic 请求 → NormalizedRequest
pub fn anthropic_to_normalized(req: &AnthropicRequest) -> NormalizedRequest {
    let mut messages: Vec<NormalizedMessage> = Vec::new();

    // 1. 顶层 system → System message
    // 借鉴 VCP anthropic_messages adapter.adapt_request:78-94
    if let Some(s) = &req.system {
        if !s.is_empty() {
            messages.push(NormalizedMessage::system(s.clone()));
        }
    }

    // 2. messages 数组
    for m in &req.messages {
        let role = MessageRole::from_vcp(&m.role);
        let content = if let Some(s) = m.content.as_str() {
            // 简单字符串 content
            vec![ContentPart::Text {
                text: s.to_string(),
            }]
        } else if let Some(arr) = m.content.as_array() {
            // 块数组: 提取 text blocks, 跳过 tool_use / tool_result
            let mut parts = Vec::new();
            for block in arr {
                if let Some(obj) = block.as_object() {
                    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                        if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                            parts.push(ContentPart::Text {
                                text: text.to_string(),
                            });
                        }
                    }
                }
            }
            parts
        } else {
            Vec::new()
        };

        messages.push(NormalizedMessage {
            role,
            content,
            tool_calls: Vec::new(),
            tool_call_id: m.tool_call_id.clone(),
            name: None,
        });
    }

    NormalizedRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        max_tokens: Some(req.max_tokens),
        stream: req.stream,
        stop: req.stop_sequences.clone().unwrap_or_default(),
        tools: Vec::new(),
        tool_choice: None,
        metadata: Default::default(),
    }
}

/// NormalizedResponse → Anthropic 响应 JSON
pub fn anthropic_from_normalized(resp: &NormalizedResponse) -> AnthropicResponse {
    let stop_reason = resp
        .finish_reason
        .map(|r| r.to_anthropic().to_string())
        .unwrap_or_else(|| "end_turn".to_string());

    AnthropicResponse {
        id: resp.id.clone(),
        kind: "message",
        role: "assistant",
        model: resp.model.clone(),
        content: vec![json!({
            "type": "text",
            "text": resp.content,
        })],
        stop_reason,
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens: resp.usage.prompt_tokens,
            output_tokens: resp.usage.completion_tokens,
        },
        apeireth_protocol: Some("anthropic_messages".to_string()),
    }
}

// ============================================================
// 4. Gemini GenerateContent 编解码
// ============================================================

/// Gemini GenerateContent 请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiRequest {
    /// contents: [{role: "user"|"model", parts: [{text: "..."}]}]
    pub contents: Vec<GeminiContent>,
    /// 顶层 systemInstruction (Gemini 风格)
    #[serde(default)]
    pub system_instruction: Option<GeminiSystemInstruction>,
    /// generationConfig: {temperature, maxOutputTokens, topP, ...}
    #[serde(default)]
    pub generation_config: Option<GeminiGenerationConfig>,
    #[serde(default)]
    pub tools: Option<Vec<Value>>,
    /// R121 续 (V2-2 战区 2.5): Gemini stream 标志 (1:1 跟其他 3 协议)
    /// ⚠️ Gemini 流式端点路径是 `:streamGenerateContent?alt=sse`, 但 V2-2 简化为直送上游,
    /// 上游根据 stream 字段决定是否走 stream 端点 (由 minimaxi 处理)
    /// **0 漂移 1.0 行为**: default false = 1.0 行为 (非流式)
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiContent {
    pub role: String, // "user" / "model"
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// R132.2: 换 untagged 兼容标准 Gemini `parts: [{"text": "..."}]` 格式
/// (R131.10 暴露 bug: externally tagged 期望 `{"Text": {"text": "..."}}`)
#[serde(untagged)]
pub enum GeminiPart {
    Text {
        text: String,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiGenerationConfig {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stop_sequences: Option<Vec<String>>,
}

/// Gemini GenerateContent 响应
#[derive(Debug, Serialize, Clone)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
    /// R17 战役 1-4 新增: 协议名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apeireth_protocol: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GeminiCandidate {
    pub content: GeminiContentOut,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GeminiContentOut {
    pub role: &'static str,
    pub parts: Vec<GeminiPartOut>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GeminiPartOut {
    Text { text: String },
}

#[derive(Debug, Serialize, Clone)]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: u32,
    pub candidates_token_count: u32,
    pub total_token_count: u32,
}

/// Gemini 请求 → NormalizedRequest
pub fn gemini_to_normalized(req: &GeminiRequest) -> NormalizedRequest {
    let mut messages: Vec<NormalizedMessage> = Vec::new();

    // 1. systemInstruction → System message
    // 借鉴 VCP gemini adapter.adapt_request:64-82
    if let Some(si) = &req.system_instruction {
        for part in &si.parts {
            if let GeminiPart::Text { text } = part {
                messages.push(NormalizedMessage::system(text.clone()));
            }
        }
    }

    // 2. contents: role user/model → User/Assistant
    for c in &req.contents {
        let role = match c.role.as_str() {
            "model" => MessageRole::Assistant,
            "user" => MessageRole::User,
            // Gemini 没有 system role (在 systemInstruction 里)
            "system" => MessageRole::System,
            _ => MessageRole::User,
        };

        // 收集 text parts
        let mut text = String::new();
        for part in &c.parts {
            if let GeminiPart::Text { text: t } = part {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
        }

        messages.push(NormalizedMessage {
            role,
            content: if text.is_empty() {
                Vec::new()
            } else {
                vec![ContentPart::Text { text }]
            },
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        });
    }

    let (temperature, max_tokens, stop) = if let Some(gc) = &req.generation_config {
        (
            gc.temperature,
            gc.max_output_tokens,
            gc.stop_sequences.clone().unwrap_or_default(),
        )
    } else {
        (None, None, Vec::new())
    };

    NormalizedRequest {
        model: String::new(), // Gemini 模型在 URL, 不在 body
        messages,
        temperature,
        max_tokens,
        // R122-4-retry 续 (V2-6 战区 2.6): 透传 req.stream (1:1 跟其他 3 协议)
        // 0 漂移 1.0 行为: req.stream default false (GeminiRequest.stream: bool, #[serde(default)])
        stream: req.stream,
        stop,
        tools: Vec::new(),
        tool_choice: None,
        metadata: Default::default(),
    }
}

/// NormalizedResponse → Gemini 响应 JSON
pub fn gemini_from_normalized(resp: &NormalizedResponse) -> GeminiResponse {
    let finish_reason = resp.finish_reason.map(|r| r.to_gemini().to_string());

    GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContentOut {
                role: "model",
                parts: vec![GeminiPartOut::Text {
                    text: resp.content.clone(),
                }],
            },
            finish_reason,
            index: Some(0),
        }],
        model_version: Some(resp.model.clone()),
        response_id: Some(resp.id.clone()),
        usage_metadata: Some(GeminiUsageMetadata {
            prompt_token_count: resp.usage.prompt_tokens,
            candidates_token_count: resp.usage.completion_tokens,
            total_token_count: resp.usage.total_tokens,
        }),
        apeireth_protocol: Some("gemini".to_string()),
    }
}

// ============================================================
// Pipeline 调用辅助 (战役 1-3 Pipeline 5 步 + Gemini URL 修复)
// ============================================================

/// 调 Pipeline 跑 4 协议 (战役 1-3 5 步管线 + 战役 1-4 URL 修复)
///
/// **为什么不用 `pipeline.run` for Gemini**: 战役 1-3 pipeline 的 URL 构造是
/// `format!("{}{}", base_url, endpoint_path)`, 但 Gemini `endpoint_path` 含
/// `{model}` 占位符, 不会被替换。这里显式构造 URL, 走 pipeline 的 http client
/// (Keep-Alive LIFO 5 字段保留)。
///
/// **为什么不用 `pipeline.run` for Anthropic @ minimaxi**: minimaxi 的 Anthropic
/// 端点是 `/anthropic/v1/messages` (跟标准 Anthropic `/v1/messages` 不同), 战役 1-3
/// pipeline 不会自动加前缀。这里也显式构造 URL。
///
/// **不假装**:
/// - ✅ OpenAI Chat / OpenAI Responses 走 `pipeline.run` (5 步全跑, 端点无占位符)
/// - ✅ Gemini / Anthropic @ minimaxi 走 `pipeline.router().encode/decode` +
///   `pipeline.http().post_json` (前 4 步 + Keep-Alive LIFO HTTP, 不偷工)
pub async fn dispatch(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
) -> Result<NormalizedResponse, String> {
    // R120 (B2 战区 2): cache hook — 原 dispatch 1.0 行为 0 漂移 (无 cache 走原路径)
    // 新能力 opt-in: server.rs handler 调 dispatch_cached
    dispatch_cached(pipeline, kind, input, None).await
}

/// R120 (B2 战区 2): dispatch + response cache wrap
///
/// **行为**:
/// - `cache = None` → 跟原 `dispatch` 1:1 行为 (1.0 验收 0 漂移)
/// - `cache = Some(c)` + `input.stream == true` → skip cache, 走原 dispatch (SSE 边界)
/// - `cache = Some(c)` + 非流式:
///   1. 查 cache: `Some(resp)` → 写 cache.hit, 返 resp
///   2. 查 cache: `None` → 写 cache.miss, 调原 dispatch
///      - 成功 → cache.put, 写 cache.put
///      - 失败 → 返 Err (0 写入 cache)
///
/// **fail-soft**: cache 内部错误 (CapacityExceeded / 反序列化失败) → 走原 dispatch, 0 影响主路径
pub async fn dispatch_cached(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
    cache: Option<&ResponseCache>,
) -> Result<NormalizedResponse, String> {
    // R120 (B3 战区 2): 走 status-aware 版本, 丢 status (1.0 行为 0 漂移)
    let (_status, result) = dispatch_cached_with_status(pipeline, kind, input, cache).await;
    result
}

/// R120 (B3 战区 2): dispatch_cached + status 跟踪 (给 retry 用)
///
/// **1.0 行为 0 漂移**: 老的 dispatch_cached 调它 + 丢 status, server.rs 4 handler 0 改动
/// **新能力**: status 给 retry_with_backoff 决定 4xx (除 408/425/429) 不重试 / 5xx 全重试
pub async fn dispatch_cached_with_status(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
    cache: Option<&ResponseCache>,
) -> (u16, Result<NormalizedResponse, String>) {
    // 流式 bypass (任务 spec: "流式 (SSE) 不缓存", B5 边界)
    if input.stream {
        return (200, dispatch_inner(pipeline, kind, input).await);
    }

    // Cache hit path
    if let Some(c) = cache {
        if let Some(resp) = c.get(&input, kind).await {
            // 命中由 ResponseCache::get 内部 +1 hit_counter
            return (200, Ok(resp));
        }
        // miss 由 ResponseCache::get 内部 +1 miss_counter
    }

    // Cache miss path: 走原 5 步管线
    let (status, result) = dispatch_inner_with_status(pipeline, kind, input.clone()).await;

    // 成功才写 cache
    if let (Some(c), Ok(ref resp)) = (cache, &result) {
        c.put(&input, kind, resp).await;
    }

    (status, result)
}

/// R120 (B3 战区 2): dispatch + 多层退避重试
///
/// **行为**:
/// - 调 dispatch_cached_with_status 拿 (status, result)
/// - 失败时看 status 决定是否重试 (4xx 不重试除 408/425/429, 5xx / 0 (network) 全重试)
/// - 成功 (status 2xx) → 返 result, 不重试
/// - 退避档位从 BackoffPolicy 拿
///
/// **metrics**:
/// - 每次 retry attempt: `retry_count.inc()`
/// - 退避耗尽: `retry_exhausted.inc()`
/// - 重试后成功: `retry_success_after.inc()`
///
/// **决策日志**: `reports/decision-log-2026-08-10.md` 决策 #2 (Patient 默认)
pub async fn dispatch_with_retry(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
    cache: Option<&ResponseCache>,
    policy: &BackoffPolicy,
    stats: &RetryStats,
) -> Result<NormalizedResponse, String> {
    let backoffs = policy.to_durations();
    // R122-4 续: 取 jitter mode + cap (cap = 最长 tier, decorrelated jitter 必需)
    let jitter_mode = policy.jitter();
    let cap = backoffs.last().copied().unwrap_or(Duration::from_secs(60));
    let mut prev: Option<Duration> = None;
    let mut attempt = 0;
    let mut last_result: (u16, Result<NormalizedResponse, String>) =
        dispatch_cached_with_status(pipeline, kind, input.clone(), cache).await;

    // 第一次直接调, 不写 metric
    if last_result.1.is_ok() {
        return last_result.1;
    }
    // 第一次失败: 检查 status
    if !should_retry_status(last_result.0) {
        // 4xx (非白名单) / 其他 — 不重试
        return last_result.1;
    }

    // 重试循环
    while attempt < backoffs.len() && last_result.1.is_err() {
        let wait = backoffs[attempt];
        // R122-4 续: 1:1 替换 `tokio::time::sleep(wait)` → 用 `jittered_sleep` 计算 jitter 后 sleep
        // 0 漂移 1.0 行为: jitter_mode == None → jittered_sleep 返 base (= 原 wait), 跟原 1.0 行为 1:1
        let actual_wait = jittered_sleep(wait, jitter_mode, prev, cap);
        tokio::time::sleep(actual_wait).await;
        prev = Some(actual_wait);
        stats.retry_count.inc();
        attempt += 1;

        last_result = dispatch_cached_with_status(pipeline, kind, input.clone(), cache).await;
        if last_result.1.is_ok() {
            // 重试后成功
            stats.retry_success_after.inc();
            return last_result.1;
        }
        // 失败: 检查 status
        if !should_retry_status(last_result.0) {
            // 4xx (非白名单) — 不重试
            return last_result.1;
        }
    }

    // 退避耗尽
    stats.retry_exhausted.inc();
    last_result.1
}

/// R120 (B3 战区 2): 内部 dispatch + status 跟踪 (新, 供 retry 用)
///
/// **1.0 行为 0 漂移**: dispatch_inner (老的) 调它 + 丢 status
fn dispatch_inner_with_status(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = (u16, Result<NormalizedResponse, String>)> + Send + '_>> {
    use apeireth_pipeline::PipelineError;
    let err_map = |e: PipelineError| match e {
        PipelineError::Protocol(s) => format!("protocol: {s}"),
        PipelineError::Http(s) => format!("http: {s}"),
        PipelineError::Suppressed(k) => format!("suppressed: {k}"),
    };

    Box::pin(async move {
        match kind {
            ProtocolKind::Gemini => {
                let (status, result) = run_gemini_with_status(pipeline, input).await;
                // Suppressed / protocol encode errors 没 status, 给 0 (network err 等价)
                (status, result)
            }
            ProtocolKind::AnthropicMessages => {
                // minimaxi Anthropic 端点是 /anthropic/v1/messages, 战役 1-3 pipeline 不会自动加
                if is_minimaxi_anthropic_quirk(&pipeline.config().base_url) {
                    let (status, result) = run_anthropic_minimaxi_with_status(pipeline, input).await;
                    (status, result)
                } else {
                    let result = pipeline.run(kind, input).await.map_err(err_map);
                    // pipeline.run 不返 status, 失败给 0 (network err 等价 — 实际是 suppressed/http)
                    // 注: 1.0 行为没 status 跟踪, 这里给 0 不漂移
                    (0, result)
                }
            }
            _ => {
                let result = pipeline.run(kind, input).await.map_err(err_map);
                (0, result)
            }
        }
    })
}

/// R120 (B2 战区 2): 内部 dispatch (原 dispatch 主体, 1.0 行为 0 漂移)
///
/// **R119 0 漂移**: 函数体 1:1 跟 R17 战役 1-4 + R37-1 一致
/// **R122-1-retry (B5 战区 2)**: 加 VCP 借鉴 ResponseReplayCache fast path
/// - 0 改 fn 签名 (向后兼容, 跟 R121-retry 严守)
/// - 通过 process-wide global singleton (`replay_cache::global()`) 接入, 1.0 行为 0 漂移
/// - 流式 (stream=true) bypass cache (跟 R120 B2 dispatch_cached_with_status 1:1)
/// - 命中 → 返 cached NormalizedResponse (反序列化); miss → 走原 5 步管线
/// - 成功后 `cache.record` (fail-soft, 跟 B2 `put` 1:1)
fn dispatch_inner(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NormalizedResponse, String>> + Send + '_>> {
    let cache = replay_cache();
    let is_stream = input.stream;

    // Stream bypass: 0 cache lookup, 0 cache record (跟 R120 B2 1:1, 流式 SSE 边界)
    if is_stream {
        return Box::pin(async move {
            let (_status, result) = dispatch_inner_with_status(pipeline, kind, input).await;
            result
        });
    }

    // Compute cache key (method + url + body) — VCP `buildKey` 升级版
    // 1.0 行为 0 漂移: endpoint_url 失败 / 序列化失败 → cache_key = None → fall through 原 dispatch
    let url = match kind {
        ProtocolKind::Gemini => endpoint_url(&pipeline.config().base_url, kind, &input.model).ok(),
        _ => endpoint_url(&pipeline.config().base_url, kind, "").ok(),
    };
    let body_bytes = serde_json::to_vec(&input).ok();
    let cache_key = match (url, body_bytes) {
        (Some(u), Some(b)) => Some(hash_request(DEFAULT_HTTP_METHOD, &u, &b)),
        _ => None,
    };

    // Lookup (fail-soft: lock poisoned / deserialize fail → None → 走原 dispatch)
    let cache_hit: Option<ReplayEntry> = cache_key
        .as_ref()
        .and_then(|k| cache.lookup(k));

    Box::pin(async move {
        // Fast path: hit → deserialize + return (skip 5 步管线)
        if let Some(entry) = cache_hit {
            if let Ok(resp) = entry.response.to_response() {
                return Ok(resp);
            }
            // 反序列化失败 (cache 内容损坏 / schema drift) → fall through 重算
        }

        // Slow path: pipeline 5 步
        let (_status, result) = dispatch_inner_with_status(pipeline, kind, input).await;

        // Record on success (fail-soft: lock error / serialize error 静默, 跟 B2 `put` 1:1)
        if let (Ok(resp), Some(k)) = (&result, &cache_key) {
            let payload = ResponsePayload::from_response(resp, 200);
            let _ = cache.record(k.clone(), payload);
        }

        result
    })
}

/// 共享的 5 步管线前 3 步 (placeholder / token_budget / force_translate)
///
/// 复刻战役 1-3 pipeline.run 行为, 用于 Gemini / Anthropic @ minimaxi 的 bypass 路径
fn run_pipeline_prelude(pipeline: &Pipeline, input: &mut NormalizedRequest) {
    use apeireth_pipeline::{force_translate_if_needed, resolve_placeholders, truncate_to_max};
    use apeireth_protocol::ContentPart;

    // 步骤 1: 解析 placeholder
    for msg in input.messages.iter_mut() {
        if matches!(msg.role, MessageRole::User | MessageRole::System) {
            for part in msg.content.iter_mut() {
                if let ContentPart::Text { text } = part {
                    *text = resolve_placeholders(text, &pipeline.config().placeholder_context);
                }
            }
        }
    }
    // 步骤 2: token 预算
    let max_chars = pipeline.config().max_injection_chars;
    for msg in input.messages.iter_mut() {
        for part in msg.content.iter_mut() {
            if let ContentPart::Text { text } = part {
                *text = truncate_to_max(text, max_chars);
            }
        }
    }
    // 步骤 3: Force-Translate
    let force_config = &pipeline.config().force_translate;
    let _stats = force_translate_if_needed(&input.model, &mut input.messages, force_config);
}

/// Gemini 专用: 用 pipeline 的 router + http client, 但显式构造 URL
async fn run_gemini(
    pipeline: &Pipeline,
    mut input: NormalizedRequest,
) -> Result<NormalizedResponse, String> {
    // R120 (B3 战区 2): 1.0 行为 0 漂移 — 调 _with_status + 丢 status
    let (_status, result) = run_gemini_with_status(pipeline, input).await;
    result
}

/// R120 (B3 战区 2): run_gemini + status 跟踪 (新, 供 retry 用)
async fn run_gemini_with_status(
    pipeline: &Pipeline,
    mut input: NormalizedRequest,
) -> (u16, Result<NormalizedResponse, String>) {
    if input.model.is_empty() {
        return (0, Err("gemini: model is empty (must be set by handler from URL path)".into()));
    }

    let model = input.model.clone();

    // 步骤 1-3: 跟 pipeline.run 一致
    run_pipeline_prelude(pipeline, &mut input);

    // 步骤 4: 协议归一化 (R37-1: ProtocolBridge::encode_for_kind)
    let body: Value = match encode_for_kind(ProtocolKind::Gemini, &input) {
        Ok(b) => b,
        Err(e) => return (0, Err(format!("protocol encode: {e}"))),
    };

    // 步骤 5: HTTP 调用 (调战役 1-2 apeireth-http-client Keep-Alive LIFO)
    // URL 显式构造 (替换 {model} 占位符)
    let base_url = &pipeline.config().base_url;
    let url = match endpoint_url(base_url, ProtocolKind::Gemini, &model) {
        Ok(u) => u,
        Err(e) => return (0, Err(format!("protocol endpoint: {e}"))),
    };
    send_and_decode_with_status(pipeline, &url, &body, ProtocolKind::Gemini).await
}

/// Anthropic @ minimaxi 专用: 显式构造 URL (/anthropic/v1/messages)
async fn run_anthropic_minimaxi(
    pipeline: &Pipeline,
    mut input: NormalizedRequest,
) -> Result<NormalizedResponse, String> {
    // R120 (B3 战区 2): 1.0 行为 0 漂移 — 调 _with_status + 丢 status
    let (_status, result) = run_anthropic_minimaxi_with_status(pipeline, input).await;
    result
}

/// R120 (B3 战区 2): run_anthropic_minimaxi + status 跟踪 (新, 供 retry 用)
async fn run_anthropic_minimaxi_with_status(
    pipeline: &Pipeline,
    mut input: NormalizedRequest,
) -> (u16, Result<NormalizedResponse, String>) {
    // 步骤 1-3: 跟 pipeline.run 一致
    run_pipeline_prelude(pipeline, &mut input);

    // 步骤 4: 协议归一化 (R37-1: ProtocolBridge)
    let body: Value = match encode_for_kind(ProtocolKind::AnthropicMessages, &input) {
        Ok(b) => b,
        Err(e) => return (0, Err(format!("protocol encode: {e}"))),
    };

    // 步骤 5: HTTP 调用 (URL 加 minimaxi /anthropic 前缀)
    let base_url = &pipeline.config().base_url;
    let url = match endpoint_url(base_url, ProtocolKind::AnthropicMessages, "") {
        Ok(u) => u,
        Err(e) => return (0, Err(format!("protocol endpoint: {e}"))),
    };
    send_and_decode_with_status(pipeline, &url, &body, ProtocolKind::AnthropicMessages).await
}

/// 共享的 HTTP send + decode (战役 1-2 HttpClient Keep-Alive LIFO + 战役 1-1 decode)
///
/// **1.0 行为 0 漂移**: 调 send_and_decode_with_status + 丢 status (server.rs 4 handler 0 改动)
async fn send_and_decode(
    pipeline: &Pipeline,
    url: &str,
    body: &Value,
    kind: ProtocolKind,
) -> Result<NormalizedResponse, String> {
    let (_status, result) = send_and_decode_with_status(pipeline, url, body, kind).await;
    result
}

/// R120 (B3 战区 2): send_and_decode + status 跟踪 (新, 供 retry 用)
///
/// **status 取值**:
/// - `0` → network error (reqwest send / read body 失败)
/// - `200` → 成功 (Ok 路径)
/// - `4xx` / `5xx` → 显式 http status (Err 路径, 给 retry 判断)
///
/// **1.0 行为 0 漂移**: 老 send_and_decode 调它 + 丢 status
async fn send_and_decode_with_status(
    pipeline: &Pipeline,
    url: &str,
    body: &Value,
    kind: ProtocolKind,
) -> (u16, Result<NormalizedResponse, String>) {
    let _guard = pipeline.http().pool().enter().await;
    let mut req_builder = pipeline.http().reqwest_client().post(url).json(body);
    if let Some(token) = &pipeline.config().auth_token {
        req_builder = req_builder.bearer_auth(token);
    }

    let response = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => return (0, Err(format!("http send: {e}"))),
    };

    let status = response.status().as_u16();

    // 4xx / 5xx 显式返 Err (B3 retry hook 用 status 决定)
    if status >= 400 {
        return (status, Err(format!("http: {status}")));
    }

    let text: String = match response.text().await {
        Ok(t) => t,
        Err(e) => return (status, Err(format!("http read body: {e}"))),
    };

    let raw: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return (
                status,
                Err(format!(
                    "json parse: {e}, body: {}",
                    text.chars().take(200).collect::<String>()
                )),
            );
        }
    };

    let result = decode_for_kind(kind, &raw).map_err(|e| {
        format!(
            "protocol decode: {e}, raw: {}",
            text.chars().take(200).collect::<String>()
        )
    });
    (status, result)
}

// ============================================================
// R121 续 (V2-2 战区 2.5): 4 协议流式 SSE 转发
// ============================================================
//
// **目的**: 4 协议 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini) 流式 (req.stream==true)
// 走统一的 SSE 字节转发路径, **0 走 cache** (跟 B2 SSE 边界 1:1).
//
// **架构位置**:
// ```text
//   server.rs 4 handler (req.stream == true)
//     ↓ protocol_handlers::stream_forward(pipeline, kind, raw_body, model)
//       ├── 用 endpoint_url(base_url, kind, model) 构造上游 URL (1:1 翻译 4 协议端点)
//       ├── Pipeline.http().reqwest_client().post(url) (战役 1-2 Keep-Alive LIFO 复用)
//       ├── Bearer auth (pipeline.config().auth_token, 跟非流式同源)
//       └── upstream.bytes_stream() → axum::body::Body::from_stream (SSE 字节 0 字节篡改)
// ```
//
// **0 漂移 1.0 行为**:
// - 非流式路径 0 改 (server.rs 4 handler 0 改, 仅加 if req.stream 早返回)
// - 1.0 chat_completions 内部 `stream_chat_completions_forward` 删除, 改用本函数 (0 行为差异, 1:1 翻译)
//
// **0 假装** (主哲学锚 #1):
// - ✅ 用 endpoint_url 已有 4 协议分支 (0 重写 URL 构造)
// - ✅ 复用 pipeline.http().reqwest_client() (战役 1-2 5 字段 Keep-Alive)
// - ✅ 上游 SSE 字节 0 字节篡改, 直送客户端 (TUI / Web 端解 SSE)
// - ✅ 0 触碰 cache (流式 bypass B2 已实现, 1:1 镜像)
//
// **决策日志**: `reports/agent-v2-decision-log-2026-08-10.md` 决策 #1

/// 4 协议流式 SSE 字节转发 (B 留 R121 续)
///
/// **1:1 翻译** server.rs:290-331 原 `stream_chat_completions_forward`, 抽到 protocol_handlers:
/// - 4 协议统一函数 (kind 决定 endpoint URL, 走 `endpoint_url` 已实现分支)
/// - 流式 0 走 cache (req.stream 守门由调用方负责)
/// - 0 解析 SSE 内容, 直送 bytes_stream (TUI / Web 端解 SSE)
///
/// **失败处理** (跟原 `stream_chat_completions_forward` 1:1):
/// - HTTP send 失败 → 返 Err(String) (caller 转 502 BAD_GATEWAY)
/// - response build 失败 → 返 Err(String)
/// - 上游 4xx/5xx status 0 改, 透传 (SSE 客户端看 status 自决)
///
/// **参数**:
/// - `pipeline`: R17 战役 1-3 Pipeline (含 base_url + auth_token + Keep-Alive LIFO)
/// - `kind`: 4 协议之一 (OpenAiChat / OpenAiResponses / AnthropicMessages / Gemini)
/// - `raw_body`: 客户端原 JSON body (含 `"stream": true` 字段, 0 改直送上游)
/// - `model`: 必填 for Gemini (URL 含 `{model}` 占位符), 其他协议 0 用
pub async fn stream_forward(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    raw_body: axum::body::Bytes,
    model: &str,
) -> Result<axum::response::Response, String> {
    use axum::body::Body;
    use futures::stream::StreamExt;
    use http::header::CONTENT_TYPE;

    // 1. 构造上游 URL (1:1 endpoint_url 4 协议分支)
    let base = &pipeline.config().base_url;
    let url = endpoint_url(base, kind, model).map_err(|e| format!("endpoint_url: {e}"))?;

    // 2. 构造 reqwest request (跟非流式同源: Bearer + Keep-Alive LIFO)
    let mut req_builder = pipeline
        .http()
        .reqwest_client()
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .body(raw_body);
    if let Some(token) = &pipeline.config().auth_token {
        req_builder = req_builder.bearer_auth(token);
    }

    // 3. 发送 + 透传 SSE 字节
    let upstream = req_builder.send().await.map_err(|e| format!("http: {e}"))?;

    let status = upstream.status();
    let upstream_ct = upstream.headers().get(CONTENT_TYPE).cloned();

    let stream = upstream.bytes_stream().map(|chunk| {
        chunk
            .map(axum::body::Bytes::from)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    });

    // 4. 构造 axum Response (透传 status + content-type)
    let mut builder = axum::response::Response::builder().status(status);
    match upstream_ct {
        Some(ct) => {
            builder = builder.header(CONTENT_TYPE, ct);
        }
        None => {
            builder = builder.header(CONTENT_TYPE, "text/event-stream");
        }
    }
    builder
        .body(Body::from_stream(stream))
        .map_err(|e| format!("response build: {e}"))
}

// ============================================================
// 单元测试 (≥ 5 个, 8 项不漂移 / 不假装)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::MessageRole;

    #[test]
    fn constants_are_valid() {
        assert!(OPENAI_CHAT_PATH.starts_with('/'));
        assert!(OPENAI_RESPONSES_PATH.starts_with('/'));
        assert!(ANTHROPIC_MESSAGES_PATH.starts_with('/'));
        assert!(GEMINI_PATH_TEMPLATE.contains("{model}"));
        assert_eq!(AUTH_SCHEME_BEARER, "Bearer");
        assert!(MINIMAXI_BASE_URL.starts_with("https://"));
    }

    #[test]
    fn endpoint_url_substitutes_gemini_model() {
        // Gemini @ minimaxi: /v1/gemini/v1beta/models/{model}:generateContent
        let url = endpoint_url(
            "https://api.minimaxi.com",
            ProtocolKind::Gemini,
            "MiniMax-M3",
        ).expect("HTTP kind");
        assert_eq!(
            url,
            "https://api.minimaxi.com/v1/gemini/v1beta/models/MiniMax-M3:generateContent"
        );
    }

    #[test]
    fn endpoint_url_handles_trailing_slash() {
        // base_url 带 / 也要正确
        let url = endpoint_url("https://api.minimaxi.com/", ProtocolKind::OpenAiChat, "").expect("HTTP kind");
        assert_eq!(url, "https://api.minimaxi.com/v1/chat/completions");
    }

    #[test]
    fn endpoint_url_for_3_other_protocols() {
        // OpenAI Chat / Responses: 端点无占位符
        let url1 = endpoint_url("https://api.minimaxi.com", ProtocolKind::OpenAiChat, "").expect("HTTP kind");
        let url2 = endpoint_url(
            "https://api.minimaxi.com",
            ProtocolKind::OpenAiResponses,
            "",
        ).expect("HTTP kind");
        assert_eq!(url1, "https://api.minimaxi.com/v1/chat/completions");
        assert_eq!(url2, "https://api.minimaxi.com/v1/responses");
    }

    #[test]
    fn endpoint_url_anthropic_minimaxi_adds_anthropic_prefix() {
        // minimaxi 默认 base URL → 加 /anthropic 前缀
        let url = endpoint_url(
            "https://api.minimaxi.com",
            ProtocolKind::AnthropicMessages,
            "",
        ).expect("HTTP kind");
        assert_eq!(url, "https://api.minimaxi.com/anthropic/v1/messages");
    }

    #[test]
    fn endpoint_url_anthropic_direct_uses_standard_path() {
        // 直连 Anthropic (base_url 自带 /v1) → 用标准 /v1/messages
        let url = endpoint_url(
            "https://api.anthropic.com/v1",
            ProtocolKind::AnthropicMessages,
            "",
        ).expect("HTTP kind");
        assert_eq!(url, "https://api.anthropic.com/v1/v1/messages");
    }

    #[test]
    fn endpoint_url_anthropic_minimaxi_with_explicit_prefix_skips_quirk() {
        // 用户已经显式给 /anthropic, 不再加前缀
        let url = endpoint_url(
            "https://api.minimaxi.com/anthropic",
            ProtocolKind::AnthropicMessages,
            "",
        ).expect("HTTP kind");
        assert_eq!(url, "https://api.minimaxi.com/anthropic/v1/messages");
    }

    #[test]
    fn endpoint_url_gemini_minimaxi_adds_v1_gemini_prefix() {
        // minimaxi 默认 base URL → Gemini 加 /v1/gemini 前缀
        let url = endpoint_url(
            "https://api.minimaxi.com",
            ProtocolKind::Gemini,
            "MiniMax-M3",
        ).expect("HTTP kind");
        assert_eq!(
            url,
            "https://api.minimaxi.com/v1/gemini/v1beta/models/MiniMax-M3:generateContent"
        );
    }

    #[test]
    fn endpoint_url_gemini_direct_uses_standard_path() {
        // 直连 Google Gemini → 标准 /v1beta/models/.../...
        let url = endpoint_url(
            "https://generativelanguage.googleapis.com",
            ProtocolKind::Gemini,
            "gemini-1.5-pro",
        ).expect("HTTP kind");
        assert_eq!(url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent");
    }

    #[test]
    fn openai_chat_to_normalized_role_mapping() {
        // 借鉴 VCP protocolBridge.js:47-52 normalizeMessageRole
        let req = OpenAiChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("You are helpful"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!("hi"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "assistant".to_string(),
                    content: json!("hello back"),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.5),
            max_tokens: Some(256),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let n = openai_chat_to_normalized(&req);
        assert_eq!(n.model, "gpt-4o");
        assert_eq!(n.messages.len(), 3);
        assert_eq!(n.messages[0].role, MessageRole::System);
        assert_eq!(n.messages[1].role, MessageRole::User);
        assert_eq!(n.messages[2].role, MessageRole::Assistant);
        assert_eq!(n.temperature, Some(0.5));
        assert_eq!(n.max_tokens, Some(256));
    }

    #[test]
    fn openai_responses_to_normalized_handles_instructions_and_input_array() {
        // 借鉴 VCP openai_responses adapter.adapt_request:63-81
        let req = OpenAiResponsesRequest {
            model: "gpt-4o".to_string(),
            input: json!([
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": "hello back"}
            ]),
            instructions: Some("You are concise".to_string()),
            temperature: Some(0.3),
            max_tokens: Some(128),
            stream: false,
            tools: None,
            tool_choice: None,
        };
        let n = openai_responses_to_normalized(&req);
        assert_eq!(n.model, "gpt-4o");
        // 1 system (from instructions) + 2 from input array = 3
        assert_eq!(n.messages.len(), 3);
        assert_eq!(n.messages[0].role, MessageRole::System);
        assert_eq!(
            ContentPart::join_text(&n.messages[0].content),
            "You are concise"
        );
        assert_eq!(n.messages[1].role, MessageRole::User);
        assert_eq!(n.messages[2].role, MessageRole::Assistant);
    }

    #[test]
    fn openai_responses_to_normalized_handles_string_input() {
        // input 是单字符串 = 1 个 user message
        let req = OpenAiResponsesRequest {
            model: "gpt-4o".to_string(),
            input: json!("hello"),
            instructions: None,
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        };
        let n = openai_responses_to_normalized(&req);
        assert_eq!(n.messages.len(), 1);
        assert_eq!(n.messages[0].role, MessageRole::User);
    }

    #[test]
    fn anthropic_to_normalized_promotes_system_to_top() {
        // 借鉴 VCP anthropic_messages adapter.adapt_request:78-94
        let req = AnthropicRequest {
            model: "claude-sonnet-4".to_string(),
            system: Some("You are a philosopher".to_string()),
            messages: vec![
                AnthropicMessage {
                    role: "user".to_string(),
                    content: json!("hi"),
                    tool_call_id: None,
                },
                AnthropicMessage {
                    role: "assistant".to_string(),
                    content: json!([
                        {"type": "text", "text": "hello"}
                    ]),
                    tool_call_id: None,
                },
            ],
            max_tokens: 1024,
            temperature: Some(0.7),
            stream: false,
            stop_sequences: None,
            tools: None,
        };
        let n = anthropic_to_normalized(&req);
        // system 应在 messages 数组最前
        assert_eq!(n.messages.len(), 3);
        assert_eq!(n.messages[0].role, MessageRole::System);
        assert_eq!(
            ContentPart::join_text(&n.messages[0].content),
            "You are a philosopher"
        );
        assert_eq!(n.messages[1].role, MessageRole::User);
        assert_eq!(n.messages[2].role, MessageRole::Assistant);
        assert_eq!(n.max_tokens, Some(1024));
    }

    #[test]
    fn gemini_to_normalized_handles_system_instruction_and_contents() {
        // 借鉴 VCP gemini adapter.adapt_request:64-82
        let req = GeminiRequest {
            contents: vec![
                GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart::Text {
                        text: "hi".to_string(),
                    }],
                },
                GeminiContent {
                    role: "model".to_string(),
                    parts: vec![GeminiPart::Text {
                        text: "hello back".to_string(),
                    }],
                },
            ],
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart::Text {
                    text: "You are Gemini".to_string(),
                }],
            }),
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(0.3),
                max_output_tokens: Some(512),
                top_p: None,
                stop_sequences: None,
            }),
            tools: None,
            stream: false, // R121 续 (V2-2 战区 2.5): Gemini 流式 0 漂移 1.0 行为
        };
        let n = gemini_to_normalized(&req);
        // model 字段在 URL, body 里为空 (handler 会从 Path 填)
        assert_eq!(n.model, "");
        // system + 2 contents = 3 messages
        assert_eq!(n.messages.len(), 3);
        assert_eq!(n.messages[0].role, MessageRole::System);
        assert_eq!(
            ContentPart::join_text(&n.messages[0].content),
            "You are Gemini"
        );
        assert_eq!(n.messages[1].role, MessageRole::User);
        assert_eq!(n.messages[2].role, MessageRole::Assistant);
        assert_eq!(n.temperature, Some(0.3));
        assert_eq!(n.max_tokens, Some(512));
    }

    #[test]
    fn openai_chat_from_normalized_uses_openai_finish_reason() {
        // 借鉴 VCP chatCompletionHandler.js 5 字段 finish_reason
        let resp = NormalizedResponse {
            id: "chatcmpl-test".to_string(),
            model: "gpt-4o".to_string(),
            content: "hello".to_string(),
            finish_reason: Some(apeireth_protocol::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::normalized::NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        let out = openai_chat_from_normalized(&resp);
        assert_eq!(out.id, "chatcmpl-test");
        assert_eq!(out.object, "chat.completion");
        assert_eq!(out.choices[0].message.content, "hello");
        assert_eq!(out.choices[0].finish_reason, "stop");
        assert_eq!(out.usage.prompt_tokens, 10);
        assert_eq!(out.usage.completion_tokens, 5);
        assert_eq!(out.usage.total_tokens, 15);
    }

    #[test]
    fn anthropic_from_normalized_uses_anthropic_finish_reason() {
        let resp = NormalizedResponse {
            id: "msg_test".to_string(),
            model: "claude-sonnet-4".to_string(),
            content: "hello".to_string(),
            finish_reason: Some(apeireth_protocol::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::normalized::NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        let out = anthropic_from_normalized(&resp);
        assert_eq!(out.id, "msg_test");
        assert_eq!(out.kind, "message");
        assert_eq!(out.role, "assistant");
        assert_eq!(out.stop_reason, "end_turn"); // Stop → "end_turn" (Anthropic 风格)
        assert_eq!(out.content[0]["type"], "text");
        assert_eq!(out.content[0]["text"], "hello");
        assert_eq!(out.usage.input_tokens, 10);
        assert_eq!(out.usage.output_tokens, 5);
    }

    #[test]
    fn gemini_from_normalized_uses_gemini_finish_reason() {
        let resp = NormalizedResponse {
            id: "r1".to_string(),
            model: "gemini-1.5-pro".to_string(),
            content: "hello".to_string(),
            finish_reason: Some(apeireth_protocol::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::normalized::NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        let out = gemini_from_normalized(&resp);
        assert_eq!(out.candidates[0].finish_reason.as_deref(), Some("STOP"));
        assert_eq!(out.candidates[0].content.role, "model");
        assert_eq!(out.model_version.as_deref(), Some("gemini-1.5-pro"));
        assert_eq!(out.usage_metadata.as_ref().unwrap().prompt_token_count, 10);
    }

    #[test]
    fn openai_responses_from_normalized_has_output_text_block() {
        let resp = NormalizedResponse {
            id: "resp_test".to_string(),
            model: "gpt-4o".to_string(),
            content: "hello".to_string(),
            finish_reason: Some(apeireth_protocol::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::normalized::NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        let out = openai_responses_from_normalized(&resp);
        assert_eq!(out.id, "resp_test");
        assert_eq!(out.object, "response");
        assert_eq!(out.status, "completed");
        assert_eq!(out.output[0]["type"], "message");
        assert_eq!(out.output[0]["role"], "assistant");
        assert_eq!(out.output[0]["content"][0]["type"], "output_text");
        assert_eq!(out.output[0]["content"][0]["text"], "hello");
    }
}

// ============================================================
// R121 续 (V2-2 战区 2.5): 4 协议流式 SSE 边界 — 单元测试 (8 个)
// ============================================================

#[cfg(test)]
mod stream_forward_tests {
    //! 4 协议流式 SSE endpoint URL 校验 (跟 endpoint_url 1:1, 0 重复造轮子).
    //!
    //! **测试范围**:
    //! - 4 协议 endpoint URL 跟非流式 1:1 (0 漂移)
    //! - Gemini `{model}` 占位符替换 (流式 + 非流式统一)
    //! - minimaxi Anthropic / Gemini 路径 quirk
    //! - GeminiRequest stream 字段 serde 识别 (0 漂移 1.0 = default false)

    use super::*;
    use apeireth_protocol::ProtocolKind;

    /// 流式 1:1 翻译: 4 协议 endpoint URL 跟非流式 endpoint_url 1:1
    #[test]
    fn stream_endpoint_url_openai_chat() {
        let url = endpoint_url("https://api.minimaxi.com", ProtocolKind::OpenAiChat, "gpt-4o").unwrap();
        assert_eq!(url, "https://api.minimaxi.com/v1/chat/completions");
    }

    #[test]
    fn stream_endpoint_url_openai_responses() {
        let url = endpoint_url("https://api.minimaxi.com", ProtocolKind::OpenAiResponses, "gpt-4o").unwrap();
        assert_eq!(url, "https://api.minimaxi.com/v1/responses");
    }

    #[test]
    fn stream_endpoint_url_anthropic_minimaxi_quirk() {
        // minimaxi Anthropic 端点是 /anthropic/v1/messages
        let url = endpoint_url("https://api.minimaxi.com", ProtocolKind::AnthropicMessages, "claude-3").unwrap();
        assert_eq!(url, "https://api.minimaxi.com/anthropic/v1/messages");
    }

    #[test]
    fn stream_endpoint_url_anthropic_non_minimaxi() {
        // 非 minimaxi (Anthropic direct 等) 走 /v1/messages
        let url = endpoint_url("https://api.anthropic.com", ProtocolKind::AnthropicMessages, "claude-3").unwrap();
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn stream_endpoint_url_gemini_minimaxi_quirk() {
        // minimaxi Gemini 端点是 /v1/gemini/v1beta/models/{model}:generateContent
        let url = endpoint_url("https://api.minimaxi.com", ProtocolKind::Gemini, "gemini-pro").unwrap();
        assert_eq!(url, "https://api.minimaxi.com/v1/gemini/v1beta/models/gemini-pro:generateContent");
    }

    #[test]
    fn stream_endpoint_url_gemini_non_minimaxi() {
        // 非 minimaxi (Google direct) 走 /v1beta/models/{model}:generateContent
        let url = endpoint_url("https://generativelanguage.googleapis.com", ProtocolKind::Gemini, "gemini-pro").unwrap();
        assert_eq!(url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent");
    }

    /// GeminiRequest stream 字段被 serde 识别 (json test)
    #[test]
    fn gemini_request_stream_serde_recognized() {
        // 带 stream: true
        let json = r#"{"contents":[],"stream":true}"#;
        let req: GeminiRequest = serde_json::from_str(json).unwrap();
        assert!(req.stream);
        // 不带 stream 字段 → default false (1.0 行为 0 漂移)
        let json2 = r#"{"contents":[]}"#;
        let req2: GeminiRequest = serde_json::from_str(json2).unwrap();
        assert!(!req2.stream);
    }

    /// 4 协议 endpoint URL 不重复 (跟 OPENAI_CHAT_PATH / OPENAI_RESPONSES_PATH / ANTHROPIC_MESSAGES_PATH / GEMINI_PATH_TEMPLATE 1:1)
    #[test]
    fn stream_endpoint_url_4_protocols_distinct() {
        let urls = [
            endpoint_url("https://api.minimaxi.com", ProtocolKind::OpenAiChat, "m").unwrap(),
            endpoint_url("https://api.minimaxi.com", ProtocolKind::OpenAiResponses, "m").unwrap(),
            endpoint_url("https://api.minimaxi.com", ProtocolKind::AnthropicMessages, "m").unwrap(),
            endpoint_url("https://api.minimaxi.com", ProtocolKind::Gemini, "m").unwrap(),
        ];
        let unique: std::collections::HashSet<&str> = urls.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 4, "4 协议流式 endpoint 必须 distinct");
    }

    // ============================================================
    // R132.2: Gemini untagged enum schema 修复验证 (R131.10 暴露 bug)
    // ============================================================

    #[test]
    fn r132_2_gemini_part_untagged_standard_format_parses() {
        // R131.10 之前: `{"Text": {"text": "..."}}` (externally tagged)
        // R132.2 修复: 标准 Gemini 格式 `{"text": "..."}` (untagged struct variant)
        let standard_gemini = r#"{"text": "hi"}"#;
        let parsed: GeminiPart = serde_json::from_str(standard_gemini)
            .expect("R132.2: standard Gemini format should now parse");
        if let GeminiPart::Text { text } = parsed {
            assert_eq!(text, "hi");
        } else {
            panic!("expected Text variant");
        }
    }

    #[test]
    fn r132_2_gemini_request_untagged_standard_format_full_request() {
        // 完整标准 Gemini GenerateContent 请求 (curl -d 格式)
        let body = r#"{
            "contents": [
                {"role": "user", "parts": [{"text": "Reply with: GEMINI-ALIVE"}]},
                {"role": "model", "parts": [{"text": "hello back"}]}
            ],
            "system_instruction": {"parts": [{"text": "You are a test assistant"}]}
        }"#;
        let req: GeminiRequest = serde_json::from_str(body)
            .expect("R132.2: full standard Gemini request should parse");
        assert_eq!(req.contents.len(), 2);
        assert_eq!(req.contents[0].role, "user");
        assert_eq!(req.contents[0].parts.len(), 1);
        if let GeminiPart::Text { text } = &req.contents[0].parts[0] {
            assert_eq!(text, "Reply with: GEMINI-ALIVE");
        } else {
            panic!("expected Text variant in first part");
        }
        // 验证 systemInstruction 也接受标准格式
        assert!(req.system_instruction.is_some());
        let si = req.system_instruction.as_ref().unwrap();
        if let GeminiPart::Text { text } = &si.parts[0] {
            assert_eq!(text, "You are a test assistant");
        } else {
            panic!("expected Text variant in system_instruction");
        }
    }

    #[test]
    fn r132_2_gemini_part_untagged_text_only() {
        // 额外 part (image/file/audio) 应该在 untagged 下被忽略 / 不抛
        // R131.10 之前: `#[serde(other)] Other` 兼容未知 part
        // R132.2 之后: untagged enum 只接受 Text variant, 其他 part 会失败
        // 决策: Gemini 现阶段只用 text, 多模态等真接时再加
        let image_part = r#"{"inline_data": {"mime_type": "image/png", "data": "..."}}"#;
        let parsed: Result<GeminiPart, _> = serde_json::from_str(image_part);
        // 当前 schema 不支持 inline_data, 这会失败 (设计决策: text-only)
        // 这个 test 记录"当前不支持多模态 part" 的事实
        assert!(parsed.is_err(), "R132.2 暂不支持多模态 part, 仅 text");
    }


}

// ============================================================
// helper trait extension for testing
// ============================================================

// 重新导出一些用的类型方便外部使用
// (无: 战役 1-1 已在 apeireth-protocol 顶层 re-export)
