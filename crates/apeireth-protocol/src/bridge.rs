//! R37-1: ProtocolBridge trait + 4 Bridge struct — 砍 ProtocolRouter 那一层
//!
//! **R34 架构调研 #4 (5 候选 ROI 排)**: "4 个 `ProtocolAdapter` 直接实现 `ProtocolBridge` trait, 砍 `ProtocolRouter`".
//!
//! **之前 (砍前)**:
//! ```text
//!   调用方:  router.encode(ProtocolKind::OpenAiChat, req)
//!           ↓
//!       ProtocolRouter { _openai_chat: OpenAiChatAdapter, ... }
//!           ↓ match kind
//!       &OpenAiChatAdapter
//!           ↓ .adapt_request
//!       Value
//! ```
//! 2 层间接, 调用方要记 ProtocolKind enum, router ZST 持有 4 个 ZST 字段.
//!
//! **之后 (砍后, R37-1)**:
//! ```text
//!   调用方:  OpenAiChatBridge::encode(req)
//!           ↓ (ZST, 编译期 inline)
//!       Value
//! ```
//! 1 层, 0 router 字段, 0 enum match (类型系统保证 dispatch).
//!
//! **设计**:
//! - `ProtocolBridge` trait 用 **associated function** (非 `&self` method), 强制 ZST 调用,
//!   编译期 inline, 0 虚调用, 0 堆分配.
//! - 4 个 Bridge struct 都是 unit struct (`struct OpenAiChatBridge;`), zero-sized.
//! - 内部 delegate 给现有 `ProtocolAdapter` impl (adapter 代码 0 漂移, 0 重复造轮子).
//!
//! **不漂移 (主哲学锚 #1)**:
//! - `ProtocolAdapter` trait 保留 (4 个 adapter 真实现不删), 0 触碰
//! - `ProtocolKind` enum 保留 (config 字符串解析还要用), 0 删
//! - `ProtocolRouter` 标 `#[deprecated]`, 0.5 release 周期后再删 (R37-1.2 后续, 不在本 R 强删防 breaking)
//! - pipeline / ws_v1 / examples 内部 caller 全改, 公共 API 0 breaking

use crate::adapter::ProtocolAdapter;
use crate::adapters::{
    AnthropicMessagesAdapter, GeminiAdapter, OpenAiChatAdapter, OpenAiResponsesAdapter,
};
use crate::error::ProtocolError;
use crate::normalized::{NormalizedRequest, NormalizedResponse};
use serde_json::Value;

/// 协议 bridge 共同 trait (高层 facade, 砍 router 的中间层)
///
/// **设计**: 用 associated function (非 `&self` method), 强制 zero-sized 调用,
/// 编译期 inline, 0 虚调用. 调用方写 `OpenAiChatBridge::encode(req)` 即可, 0 enum 介入.
pub trait ProtocolBridge {
    /// Bridge 协议名 (e.g. "openai_chat", 跟 `ProtocolKind::as_str()` 一致)
    fn name() -> &'static str;

    /// 协议端点路径 (POST, e.g. "/v1/chat/completions")
    fn endpoint_path() -> &'static str;

    /// 归一化请求 → 协议特定 JSON
    fn encode(req: &NormalizedRequest) -> Result<Value, ProtocolError>;

    /// 协议特定 JSON → 归一化响应
    fn decode(raw: &Value) -> Result<NormalizedResponse, ProtocolError>;
}

// ============================================================
// 4 Bridge struct — 全部 ZST, 编译期 inline
// ============================================================

/// OpenAI Chat Completions bridge (ZST)
pub struct OpenAiChatBridge;

impl ProtocolBridge for OpenAiChatBridge {
    fn name() -> &'static str {
        "openai_chat"
    }
    fn endpoint_path() -> &'static str {
        OpenAiChatAdapter::new().endpoint_path()
    }
    fn encode(req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        OpenAiChatAdapter::new().adapt_request(req)
    }
    fn decode(raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        OpenAiChatAdapter::new().adapt_response(raw)
    }
}

/// OpenAI Responses API bridge (ZST)
pub struct OpenAiResponsesBridge;

impl ProtocolBridge for OpenAiResponsesBridge {
    fn name() -> &'static str {
        "openai_responses"
    }
    fn endpoint_path() -> &'static str {
        OpenAiResponsesAdapter::new().endpoint_path()
    }
    fn encode(req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        OpenAiResponsesAdapter::new().adapt_request(req)
    }
    fn decode(raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        OpenAiResponsesAdapter::new().adapt_response(raw)
    }
}

/// Anthropic Messages bridge (ZST)
pub struct AnthropicMessagesBridge;

impl ProtocolBridge for AnthropicMessagesBridge {
    fn name() -> &'static str {
        "anthropic_messages"
    }
    fn endpoint_path() -> &'static str {
        AnthropicMessagesAdapter::new().endpoint_path()
    }
    fn encode(req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        AnthropicMessagesAdapter::new().adapt_request(req)
    }
    fn decode(raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        AnthropicMessagesAdapter::new().adapt_response(raw)
    }
}

/// Google Gemini GenerateContent bridge (ZST)
pub struct GeminiBridge;

impl ProtocolBridge for GeminiBridge {
    fn name() -> &'static str {
        "gemini"
    }
    fn endpoint_path() -> &'static str {
        GeminiAdapter::new().endpoint_path()
    }
    fn encode(req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        GeminiAdapter::new().adapt_request(req)
    }
    fn decode(raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        GeminiAdapter::new().adapt_response(raw)
    }
}

// ============================================================
// ProtocolKind → Bridge dispatch helper (消除调用方 enum match 样板)
// ============================================================

use crate::gateway::ProtocolKind;

/// 把 `ProtocolKind` 路由到对应 Bridge, 给调用方 `match kind` 样板代码
///
/// 用法 (R37-1 后, 替代 `router.encode(kind, req)`):
/// ```ignore
/// let raw = match kind {
///     ProtocolKind::OpenAiChat => OpenAiChatBridge::encode(req),
///     ProtocolKind::OpenAiResponses => OpenAiResponsesBridge::encode(req),
///     ProtocolKind::AnthropicMessages => AnthropicMessagesBridge::encode(req),
///     ProtocolKind::Gemini => GeminiBridge::encode(req),
/// };
/// ```
///
/// **设计取舍**: trait 没法用 `ProtocolKind` 做类型参数 (异构), 所以保留 match.
pub fn encode_for_kind(
    kind: ProtocolKind,
    req: &NormalizedRequest,
) -> Result<Value, ProtocolError> {
    match kind {
        ProtocolKind::OpenAiChat => OpenAiChatBridge::encode(req),
        ProtocolKind::OpenAiResponses => OpenAiResponsesBridge::encode(req),
        ProtocolKind::AnthropicMessages => AnthropicMessagesBridge::encode(req),
        ProtocolKind::Gemini => GeminiBridge::encode(req),
        // ACP / MCP / OpenClawGateway 是本地服务桥 (非 HTTP JSON 互转),
        // 走 `gateway::ProtocolGateway` 异步 dispatch, 不经本 facade.
        ProtocolKind::Acp | ProtocolKind::Mcp | ProtocolKind::OpenClawGateway => {
            Err(ProtocolError::Unsupported {
                feature: format!("ProtocolBridge facade 不支持 kind={kind:?}; 走 gateway::ProtocolGateway 异步 dispatch"),
            })
        }
    }
}

/// 同理, decode
pub fn decode_for_kind(
    kind: ProtocolKind,
    raw: &Value,
) -> Result<NormalizedResponse, ProtocolError> {
    match kind {
        ProtocolKind::OpenAiChat => OpenAiChatBridge::decode(raw),
        ProtocolKind::OpenAiResponses => OpenAiResponsesBridge::decode(raw),
        ProtocolKind::AnthropicMessages => AnthropicMessagesBridge::decode(raw),
        ProtocolKind::Gemini => GeminiBridge::decode(raw),
        ProtocolKind::Acp | ProtocolKind::Mcp | ProtocolKind::OpenClawGateway => {
            Err(ProtocolError::Unsupported {
                feature: format!("ProtocolBridge facade 不支持 kind={kind:?}; 走 gateway::ProtocolGateway 异步 dispatch"),
            })
        }
    }
}

/// endpoint_path for kind (替代 `router.adapter(kind).endpoint_path()`)
///
/// **返回**: `Some(&'static str)` for 4 HTTP LLM kind (有 endpoint path);
/// `None` for ACP / MCP / OpenClawGateway (本地服务桥, 0 HTTP endpoint).
pub fn endpoint_path_for_kind(kind: ProtocolKind) -> Option<&'static str> {
    match kind {
        ProtocolKind::OpenAiChat => Some(OpenAiChatBridge::endpoint_path()),
        ProtocolKind::OpenAiResponses => Some(OpenAiResponsesBridge::endpoint_path()),
        ProtocolKind::AnthropicMessages => Some(AnthropicMessagesBridge::endpoint_path()),
        ProtocolKind::Gemini => Some(GeminiBridge::endpoint_path()),
        ProtocolKind::Acp | ProtocolKind::Mcp | ProtocolKind::OpenClawGateway => None,
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use crate::normalized::{NormalizedMessage, NormalizedRequest};

    fn sample_req() -> NormalizedRequest {
        NormalizedRequest::new("test-model", vec![NormalizedMessage::user("hi")])
    }

    #[test]
    fn openai_chat_bridge_name_and_endpoint() {
        assert_eq!(OpenAiChatBridge::name(), "openai_chat");
        assert_eq!(OpenAiChatBridge::endpoint_path(), "/v1/chat/completions");
    }

    #[test]
    fn openai_responses_bridge_name_and_endpoint() {
        assert_eq!(OpenAiResponsesBridge::name(), "openai_responses");
        // endpoint path 跟 adapter 一致
        assert!(!OpenAiResponsesBridge::endpoint_path().is_empty());
    }

    #[test]
    fn anthropic_bridge_name_and_endpoint() {
        assert_eq!(AnthropicMessagesBridge::name(), "anthropic_messages");
        assert!(!AnthropicMessagesBridge::endpoint_path().is_empty());
    }

    #[test]
    fn gemini_bridge_name_and_endpoint() {
        assert_eq!(GeminiBridge::name(), "gemini");
        assert!(!GeminiBridge::endpoint_path().is_empty());
    }

    #[test]
    fn openai_chat_bridge_encode_decode_round_trip() {
        let req = sample_req();
        let raw = OpenAiChatBridge::encode(&req).unwrap();
        assert!(raw.is_object());
        assert!(raw.get("messages").is_some());
        assert_eq!(raw.get("model").and_then(|v| v.as_str()), Some("test-model"));
        // decode 模拟响应
        let fake_resp = serde_json::json!({
            "id": "x", "model": "test-model",
            "choices": [{"message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });
        let resp = OpenAiChatBridge::decode(&fake_resp).unwrap();
        assert_eq!(resp.content, "hi");
    }

    #[test]
    fn gemini_bridge_encode_decode_round_trip() {
        let req = sample_req();
        let raw = GeminiBridge::encode(&req).unwrap();
        assert!(raw.is_object());
        let fake_resp = serde_json::json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "hi"}]},
                "finishReason": "STOP"
            }],
            "modelVersion": "gemini-1.5-pro",
            "responseId": "r1"
        });
        let resp = GeminiBridge::decode(&fake_resp).unwrap();
        assert_eq!(resp.content, "hi");
    }

    #[test]
    fn encode_for_kind_dispatches_all_4() {
        let req = sample_req();
        // 4 种 kind 都能 dispatch (Anthropic 因 max_tokens 必填失败, 预期)
        for k in [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::Gemini,
        ] {
            let r = encode_for_kind(k, &req);
            assert!(r.is_ok(), "{:?} should encode ok", k);
        }
        // Anthropic 必填 max_tokens
        let r = encode_for_kind(ProtocolKind::AnthropicMessages, &req);
        assert!(r.is_err());
    }

    #[test]
    fn decode_for_kind_dispatches_all_4() {
        // 各协议的真响应都能 decode
        let openai = serde_json::json!({
            "id": "x", "model": "gpt-4o",
            "choices": [{"message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });
        let anthropic = serde_json::json!({
            "id": "msg_x", "type": "message", "role": "assistant",
            "model": "claude-sonnet-4",
            "content": [{"type": "text", "text": "hi"}],
            "stop_reason": "end_turn", "usage": {}
        });
        let gemini = serde_json::json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "hi"}]},
                "finishReason": "STOP"
            }],
            "modelVersion": "gemini-1.5-pro",
            "responseId": "r1"
        });
        let responses = serde_json::json!({
            "id": "resp_x", "object": "response", "model": "gpt-4o",
            "status": "completed",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "hi"}]}],
            "usage": {"input_tokens": 1, "output_tokens": 1}
        });
        let cases = [
            (ProtocolKind::OpenAiChat, &openai),
            (ProtocolKind::OpenAiResponses, &responses),
            (ProtocolKind::AnthropicMessages, &anthropic),
            (ProtocolKind::Gemini, &gemini),
        ];
        for (k, raw) in cases {
            let r = decode_for_kind(k, raw);
            assert!(r.is_ok(), "{:?} decode failed", k);
            assert_eq!(r.unwrap().content, "hi");
        }
    }

    #[test]
    fn endpoint_path_for_kind_distinct() {
        let paths: Vec<&str> = [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::AnthropicMessages,
            ProtocolKind::Gemini,
        ]
        .iter()
        .filter_map(|k| endpoint_path_for_kind(*k)).collect();
        let unique: std::collections::HashSet<&str> = paths.iter().copied().collect();
        assert_eq!(unique.len(), 4, "endpoints not unique: {:?}", paths);
    }

    #[test]
    fn bridges_are_zero_sized() {
        // ZST 编译期断言: sizeof 必须为 0
        assert_eq!(std::mem::size_of::<OpenAiChatBridge>(), 0);
        assert_eq!(std::mem::size_of::<OpenAiResponsesBridge>(), 0);
        assert_eq!(std::mem::size_of::<AnthropicMessagesBridge>(), 0);
        assert_eq!(std::mem::size_of::<GeminiBridge>(), 0);
    }
}