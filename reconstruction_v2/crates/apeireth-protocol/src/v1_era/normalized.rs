//! 归一化请求/响应类型 (战役 1-1 核心)
//!
//! **设计目标**: 4 个 LLM 协议 (OpenAI Chat / OpenAI Responses / Anthropic Messages
//! / Gemini GenerateContent) 都先转成 [`NormalizedRequest`] / [`NormalizedResponse`],
//! 内部逻辑 (chat pipeline / routing / context management) 只跟归一化类型交互。
//!
//! **借鉴 VCP 真代码** (`research/source/vcptoolbox/`):
//! - 归一化 message role: `routes/protocolBridge.js:47-52` `normalizeMessageRole`
//!   (developer → system, tool / system / user / assistant 原样, 其他 → user)
//! - 归一化 content: `routes/protocolBridge.js:21-42` `normalizeTextContent`
//!   (string 原样, array 取 text / input_text / output_text)
//! - 元数据透传: `routes/protocolBridge.js:91-118` `extractProtectedTools` (Gemini
//!   functionDeclarations + legacy functions 前向传递, 不进入 messages / RAG)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================
// MessageRole (归一化角色)
// ============================================================

/// 归一化消息角色。
///
/// **借鉴** VCP `routes/protocolBridge.js:47-52` `normalizeMessageRole`:
/// - `developer` → `System` (OpenAI Responses API 引入了 developer role 跟 system 等价)
/// - `system` / `user` / `assistant` / `tool` → 原样
/// - 其他 (e.g. `function`) → `Tool` (按 VCP 真代码语义降级)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// 系统消息 (Anthropic 是顶层 system 字段, OpenAI 是 messages 第一个)
    System,
    /// 用户消息
    User,
    /// 助手消息 (历史回复)
    Assistant,
    /// 工具结果 (OpenAI tool / Anthropic tool_result / Gemini functionResponse)
    Tool,
}

impl MessageRole {
    /// 从字符串归一化 (借鉴 `protocolBridge.js:47-52`)
    pub fn from_legacy_value(s: &str) -> Self {
        match s {
            "system" | "developer" => Self::System,
            "user" => Self::User,
            "assistant" => Self::Assistant,
            "tool" | "function" => Self::Tool,
            _ => Self::User, // VCP 默认 fallback 是 user
        }
    }
}

// ============================================================
// ContentPart (多模态内容)
// ============================================================

/// 多模态内容部分。
///
/// **借鉴** VCP `routes/protocolBridge.js:21-42` `normalizeTextContent`:
/// - `text` (OpenAI Chat / Anthropic text)
/// - `input_text` (OpenAI Responses)
/// - `output_text` (OpenAI Responses assistant 角色)
/// - `image_url` (OpenAI / Gemini inline base64 or URL)
/// - 其他 (audio / video) — 战役 1-1 不实现,保留 placeholder
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// 纯文本
    Text {
        /// 文本内容
        text: String,
    },
    /// 图像 URL 或 base64 (data URL)
    ImageUrl {
        /// URL (`https://...` 或 `data:image/png;base64,...`)
        url: String,
        /// 可选 detail (OpenAI: low / high / auto)
        detail: Option<String>,
    },
}

impl ContentPart {
    /// 提取纯文本 (借鉴 `normalizeTextContent`)
    pub fn text_only(s: impl Into<String>) -> Self {
        Self::Text { text: s.into() }
    }

    /// 从 VCP 风格 raw content (string or array) 归一化
    pub fn from_legacy_value(raw: &serde_json::Value) -> Vec<Self> {
        if let Some(s) = raw.as_str() {
            return vec![Self::Text {
                text: s.to_string(),
            }];
        }
        if let Some(arr) = raw.as_array() {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                if let Some(s) = item.as_str() {
                    out.push(Self::Text {
                        text: s.to_string(),
                    });
                    continue;
                }
                if let Some(obj) = item.as_object() {
                    let t = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match t {
                        "text" | "input_text" | "output_text" => {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                out.push(Self::Text {
                                    text: text.to_string(),
                                });
                            }
                        }
                        "image_url" => {
                            if let Some(image_url) = obj.get("image_url") {
                                if let Some(url) = image_url.get("url").and_then(|v| v.as_str()) {
                                    let detail = image_url
                                        .get("detail")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    out.push(Self::ImageUrl {
                                        url: url.to_string(),
                                        detail,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            return out;
        }
        Vec::new()
    }

    /// 合并多个 part 为纯文本 (空分隔符, 借鉴 VCP join('\n'))
    pub fn join_text(parts: &[Self]) -> String {
        parts
            .iter()
            .filter_map(|p| match p {
                Self::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ============================================================
// NormalizedMessage
// ============================================================

/// 归一化消息。
///
/// **字段含义** (借鉴 VCP 真代码语义):
/// - `role`: 4 种 (System / User / Assistant / Tool)
/// - `content`: 多模态 part 列表 (空 = tool call 无 content)
/// - `tool_calls`: assistant 想调用的工具 (OpenAI tool_calls / Anthropic tool_use /
///   Gemini functionCall 归一化)
/// - `tool_call_id`: tool 角色时的关联 id (OpenAI tool_call_id / Anthropic tool_use_id)
/// - `name`: 可选 (Anthropic tool_result 需要 name)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedMessage {
    /// 角色
    pub role: MessageRole,
    /// 多模态内容
    pub content: Vec<ContentPart>,
    /// 助手想调用的工具 (仅 Assistant 角色)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// 工具调用 id (Tool 角色时, 关联到 assistant 的 tool_calls)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 可选 name (Anthropic tool_result 需要)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl NormalizedMessage {
    /// 创建 system 消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: vec![ContentPart::text_only(content)],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 user 消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![ContentPart::text_only(content)],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 assistant 消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: vec![ContentPart::text_only(content)],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 assistant + tool_calls 消息
    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        let content_str: String = content.into();
        Self {
            role: MessageRole::Assistant,
            content: if content_str.is_empty() {
                Vec::new()
            } else {
                vec![ContentPart::text_only(content_str)]
            },
            tool_calls,
            tool_call_id: None,
            name: None,
        }
    }

    /// 创建 tool 响应消息
    pub fn tool_result(
        tool_call_id: impl Into<String>,
        name: Option<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: vec![ContentPart::text_only(content)],
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            name,
        }
    }
}

// ============================================================
// Tool / ToolChoice (工具归一化)
// ============================================================

/// 函数工具参数 (JSON Schema 风格)
pub type ToolParameters = serde_json::Map<String, serde_json::Value>;

/// 归一化工具 (复刻 VCP `toOpenAiChatTool` 归一化结果)。
///
/// **借鉴** VCP `routes/protocolBridge.js:63-89` `toOpenAiChatTool`:
/// - 优先 `tool.type === 'function' && tool.function.name` (OpenAI Chat 风格)
/// - 退化 `tool.name` (Anthropic / Gemini 风格, 无 function 包装)
/// - `parameters` 从 `function.parameters` / `function.input_schema` / `parameters` /
///   `input_schema` / `schema` 任一字段取
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedTool {
    /// 工具名 (必填)
    pub name: String,
    /// 可选描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 参数 (JSON Schema 风格 object)
    #[serde(default, skip_serializing_if = "ToolParameters::is_empty")]
    pub parameters: ToolParameters,
    /// 是否严格 (OpenAI 严格模式, R17 留空)
    #[serde(default, skip_serializing_if = "is_false")]
    pub strict: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl NormalizedTool {
    /// 创建工具
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置参数
    pub fn with_parameters(mut self, params: ToolParameters) -> Self {
        self.parameters = params;
        self
    }
}

/// 归一化 tool_choice (借鉴 VCP `normalizeToolChoice` 真代码)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NormalizedToolChoice {
    /// 让 LLM 自动决定
    Auto,
    /// 不允许调工具 (VCP NONE / OpenAI "none" / Anthropic 不支持)
    None,
    /// 必须调工具 (VCP ANY with 0 or 2+ allowed names)
    Required,
    /// 必须调指定工具 (VCP ANY with 1 allowed name / OpenAI specific function)
    Specific {
        /// 工具名
        name: String,
    },
}

// ============================================================
// ToolCall (助手想调用的工具)
// ============================================================

/// 助手想调用的工具 (assistant message 的 tool_calls 字段)。
///
/// **借鉴** VCP 协议归一化:
/// - OpenAI: `{id, type: "function", function: {name, arguments (JSON string)}}`
/// - Anthropic: `{id: "toolu_xxx", name, input (JSON object)}`
/// - Gemini: `{name, args (JSON object)}` (无 id, 用 `functionCall` 字段)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用 id (Anthropic `toolu_xxx` / OpenAI `call_xxx` / Gemini 无, 用 name 替代)
    pub id: String,
    /// 工具名
    pub name: String,
    /// 参数 (JSON object 风格, 内部统一 object, 序列化时按协议决定 string vs object)
    pub arguments: serde_json::Value,
}

// ============================================================
// FinishReason / Usage
// ============================================================

/// 归一化完成原因。
///
/// **协议映射**:
/// - OpenAI: `stop` / `length` / `tool_calls` / `content_filter` / `function_call`
/// - Anthropic: `end_turn` / `max_tokens` / `stop_sequence` / `tool_use`
/// - Gemini: `STOP` / `MAX_TOKENS` / `SAFETY` / `RECITATION` / `OTHER` / `FINISH_REASON_UNSPECIFIED`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedFinishReason {
    /// 正常结束 (OpenAI "stop" / Anthropic "end_turn" / Gemini "STOP")
    Stop,
    /// 达到 max_tokens (OpenAI "length" / Anthropic "max_tokens" / Gemini "MAX_TOKENS")
    Length,
    /// 助手调用工具 (OpenAI "tool_calls" / Anthropic "tool_use")
    ToolCalls,
    /// 内容过滤 (OpenAI "content_filter" / Gemini "SAFETY")
    ContentFilter,
    /// 停止序列 (Anthropic "stop_sequence")
    StopSequence,
    /// 其他 (Gemini "OTHER" / "RECITATION" / 未知)
    Other,
}

impl NormalizedFinishReason {
    /// 从 OpenAI 风格字符串归一化
    pub fn from_openai(s: &str) -> Self {
        match s {
            "stop" => Self::Stop,
            "length" => Self::Length,
            "tool_calls" | "function_call" => Self::ToolCalls,
            "content_filter" => Self::ContentFilter,
            "stop_sequence" => Self::StopSequence,
            _ => Self::Other,
        }
    }

    /// 从 Anthropic 风格字符串归一化
    pub fn from_anthropic(s: &str) -> Self {
        match s {
            "end_turn" => Self::Stop,
            "max_tokens" => Self::Length,
            "stop_sequence" => Self::StopSequence,
            "tool_use" => Self::ToolCalls,
            _ => Self::Other,
        }
    }

    /// 从 Gemini 风格字符串归一化
    pub fn from_gemini(s: &str) -> Self {
        match s {
            "STOP" => Self::Stop,
            "MAX_TOKENS" => Self::Length,
            "SAFETY" => Self::ContentFilter,
            "RECITATION" | "OTHER" | "FINISH_REASON_UNSPECIFIED" => Self::Other,
            _ => Self::Other,
        }
    }

    /// 转 OpenAI 风格字符串
    pub fn to_openai(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ToolCalls => "tool_calls",
            Self::ContentFilter => "content_filter",
            Self::StopSequence => "stop_sequence",
            Self::Other => "other",
        }
    }

    /// 转 Anthropic 风格字符串
    pub fn to_anthropic(self) -> &'static str {
        match self {
            Self::Stop => "end_turn",
            Self::Length => "max_tokens",
            Self::ToolCalls => "tool_use",
            Self::StopSequence => "stop_sequence",
            Self::ContentFilter => "end_turn", // Anthropic 没有 content_filter
            Self::Other => "end_turn",
        }
    }

    /// 转 Gemini 风格字符串
    pub fn to_gemini(self) -> &'static str {
        match self {
            Self::Stop => "STOP",
            Self::Length => "MAX_TOKENS",
            Self::ToolCalls => "STOP", // Gemini 不区分 tool_use, 仍按 STOP 处理
            Self::ContentFilter => "SAFETY",
            Self::StopSequence => "STOP",
            Self::Other => "OTHER",
        }
    }
}

/// 归一化 token 使用
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NormalizedUsage {
    /// 输入 token
    pub prompt_tokens: u32,
    /// 输出 token
    pub completion_tokens: u32,
    /// 总计
    pub total_tokens: u32,
}

impl NormalizedUsage {
    /// 创建
    pub fn new(prompt: u32, completion: u32) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }
}

// ============================================================
// NormalizedRequest / NormalizedResponse
// ============================================================

/// 归一化请求 (4 协议入参统一形态)。
///
/// **借鉴** VCP `routes/protocolBridge.js:91-118` `extractProtectedTools`:
/// 工具字段是**受保护**的 (不进 messages / RAG),只在请求转发前附加。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRequest {
    /// 模型名 (e.g. "gpt-4o" / "claude-sonnet-4" / "gemini-1.5-pro")
    pub model: String,
    /// 对话消息 (system 必须放第一个, 借鉴 VCP `consumeVcpToolUseForbiddenPlaceholder`
    /// 只扫首段连续 system)
    pub messages: Vec<NormalizedMessage>,
    /// 温度 0.0 - 2.0
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 最大输出 token
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 是否流式 (SSE)
    #[serde(default)]
    pub stream: bool,
    /// 停止词
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// 工具列表 (归一化, **借鉴 VCP extractProtectedTools 不进 messages 原则**)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<NormalizedTool>,
    /// 工具选择策略
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<NormalizedToolChoice>,
    /// 元数据透传 (借鉴 VCP `__oneRingMeta` 思路, 任何返回新数组的步骤保留)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl NormalizedRequest {
    /// 创建简单 request (model + messages)
    pub fn new(model: impl Into<String>, messages: Vec<NormalizedMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            stream: false,
            stop: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
            metadata: BTreeMap::new(),
        }
    }
}

impl Default for NormalizedRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: false,
            stop: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// 归一化响应 (4 协议出参统一形态)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedResponse {
    /// 响应 id (OpenAI `chatcmpl-xxx` / Anthropic `msg_xxx` / Gemini 自行生成)
    pub id: String,
    /// 实际模型 (server 可能 normalize, e.g. "gpt-4o-2024-08-06")
    pub model: String,
    /// 文本内容
    pub content: String,
    /// 完成原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<NormalizedFinishReason>,
    /// Token 使用
    pub usage: NormalizedUsage,
    /// 工具调用 (assistant 想调的)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// 协议特定原始字段 (debug / 透传)
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub raw_metadata: serde_json::Map<String, serde_json::Value>,
}

impl NormalizedResponse {
    /// 创建简单响应
    pub fn text(
        id: impl Into<String>,
        model: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            content: content.into(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_role_from_legacy_value_developer_to_system() {
        // 借鉴 VCP protocolBridge.js:47-52: developer → system
        assert_eq!(
            MessageRole::from_legacy_value("developer"),
            MessageRole::System
        );
        assert_eq!(
            MessageRole::from_legacy_value("system"),
            MessageRole::System
        );
        assert_eq!(MessageRole::from_legacy_value("user"), MessageRole::User);
        assert_eq!(
            MessageRole::from_legacy_value("assistant"),
            MessageRole::Assistant
        );
        assert_eq!(MessageRole::from_legacy_value("tool"), MessageRole::Tool);
        assert_eq!(
            MessageRole::from_legacy_value("function"),
            MessageRole::Tool
        );
        assert_eq!(MessageRole::from_legacy_value("unknown"), MessageRole::User);
    }

    #[test]
    fn content_part_from_legacy_string() {
        // 借鉴 VCP protocolBridge.js:21-42: string 原样
        let raw = serde_json::json!("Hello");
        let parts = ContentPart::from_legacy_value(&raw);
        assert_eq!(parts.len(), 1);
        assert_eq!(ContentPart::join_text(&parts), "Hello");
    }

    #[test]
    fn content_part_from_legacy_value_array_text_types() {
        // 借鉴 VCP protocolBridge.js:31-34: text / input_text / output_text 都归一化
        let raw = serde_json::json!([
            {"type": "text", "text": "A"},
            {"type": "input_text", "text": "B"},
            {"type": "output_text", "text": "C"},
        ]);
        let parts = ContentPart::from_legacy_value(&raw);
        assert_eq!(parts.len(), 3);
        assert_eq!(ContentPart::join_text(&parts), "A\nB\nC");
    }

    #[test]
    fn content_part_from_legacy_value_image_url() {
        // 借鉴 VCP: image_url 单独处理
        let raw = serde_json::json!([
            {"type": "text", "text": "see "},
            {"type": "image_url", "image_url": {"url": "https://x.com/a.png", "detail": "high"}},
        ]);
        let parts = ContentPart::from_legacy_value(&raw);
        assert_eq!(parts.len(), 2);
        match &parts[1] {
            ContentPart::ImageUrl { url, detail } => {
                assert_eq!(url, "https://x.com/a.png");
                assert_eq!(detail.as_deref(), Some("high"));
            }
            _ => panic!("expected ImageUrl"),
        }
    }

    #[test]
    fn content_part_from_legacy_value_empty() {
        let raw = serde_json::json!(null);
        let parts = ContentPart::from_legacy_value(&raw);
        assert!(parts.is_empty());
        let raw = serde_json::json!({"type": "unknown"});
        let parts = ContentPart::from_legacy_value(&raw);
        assert!(parts.is_empty());
    }

    #[test]
    fn finish_reason_round_trip_all_protocols() {
        // 分协议 round-trip:
        // - OpenAI Chat 6 全部 round-trip OK
        // - Anthropic 没有 content_filter, 跳过 ContentFilter
        // - Gemini 没有 native ToolCalls / StopSequence, 跳过

        for r in [
            NormalizedFinishReason::Stop,
            NormalizedFinishReason::Length,
            NormalizedFinishReason::ToolCalls,
            NormalizedFinishReason::ContentFilter,
            NormalizedFinishReason::StopSequence,
            NormalizedFinishReason::Other,
        ] {
            assert_eq!(
                NormalizedFinishReason::from_openai(r.to_openai()),
                r,
                "OpenAI round-trip {:?} failed",
                r
            );
        }

        for r in [
            NormalizedFinishReason::Stop,
            NormalizedFinishReason::Length,
            NormalizedFinishReason::ToolCalls,
            NormalizedFinishReason::StopSequence,
        ] {
            assert_eq!(
                NormalizedFinishReason::from_anthropic(r.to_anthropic()),
                r,
                "Anthropic round-trip {:?} failed",
                r
            );
        }
        // ContentFilter → Anthropic "end_turn" → Stop (退化, 这是预期)
        // Other → Anthropic "end_turn" → Stop (退化, Anthropic 没有 unknown 概念)

        for r in [
            NormalizedFinishReason::Stop,
            NormalizedFinishReason::Length,
            NormalizedFinishReason::ContentFilter,
            NormalizedFinishReason::Other,
        ] {
            assert_eq!(
                NormalizedFinishReason::from_gemini(r.to_gemini()),
                r,
                "Gemini round-trip {:?} failed",
                r
            );
        }
        // ToolCalls / StopSequence → Gemini "STOP" → Stop (退化, 协议差异)
    }

    #[test]
    fn usage_total_is_sum() {
        let u = NormalizedUsage::new(10, 20);
        assert_eq!(u.total_tokens, 30);
    }

    #[test]
    fn normalized_request_new_defaults() {
        let r = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        assert_eq!(r.model, "gpt-4o");
        assert_eq!(r.messages.len(), 1);
        assert!(r.temperature.is_none());
        assert!(r.max_tokens.is_none());
        assert!(!r.stream);
        assert!(r.stop.is_empty());
        assert!(r.tools.is_empty());
        assert!(r.tool_choice.is_none());
        assert!(r.metadata.is_empty());
    }

    #[test]
    fn normalized_message_assistant_with_tool_calls() {
        let tc = ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"city": "Beijing"}),
        };
        let m = NormalizedMessage::assistant_with_tool_calls("", vec![tc.clone()]);
        assert_eq!(m.role, MessageRole::Assistant);
        assert!(m.content.is_empty());
        assert_eq!(m.tool_calls.len(), 1);
        assert_eq!(m.tool_calls[0], tc);
    }

    #[test]
    fn normalized_message_tool_result() {
        let m = NormalizedMessage::tool_result("call_1", Some("get_weather".into()), "sunny");
        assert_eq!(m.role, MessageRole::Tool);
        assert_eq!(m.tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(m.name.as_deref(), Some("get_weather"));
        assert_eq!(ContentPart::join_text(&m.content), "sunny");
    }
}
