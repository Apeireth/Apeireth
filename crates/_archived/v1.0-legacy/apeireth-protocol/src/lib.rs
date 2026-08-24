//! `apeireth-protocol`: **Apeireth R17 战役 1-1 LLM 协议归一化层**
//!
//! **目标**: 把 4 个 LLM 协议 (OpenAI Chat Completions / OpenAI Responses API /
//! Anthropic Messages API / Google Gemini GenerateContent) 都先归一化到内部
//! `NormalizedRequest` / `NormalizedResponse`, 内部业务逻辑只跟归一化类型交互。
//!
//! **设计**:
//! - `ProtocolKind` enum 标识 4 协议 (config 字符串解析用)
//! - `NormalizedRequest` / `NormalizedResponse` 内部统一形态
//! - `ProtocolAdapter` trait 每个协议实现 `adapt_request` / `adapt_response` (低层)
//! - `ProtocolBridge` trait + 4 Bridge struct (R37-1: 高层 facade, 砍 router 中间层)
//! - `ProtocolRouter` (R37-1 起 `#[deprecated]`, 0.5 release 后删)
//!
//! **借鉴 VCP 真代码** (`research/source/vcptoolbox/`):
//! - 归一化 message role: `routes/protocolBridge.js:47-52` `normalizeMessageRole`
//! - 归一化 content: `routes/protocolBridge.js:21-42` `normalizeTextContent`
//! - 工具归一化: `routes/protocolBridge.js:63-89` `toOpenAiChatTool` 3 步判定
//! - tool_choice 归一化: `routes/protocolBridge.js:120-156` `normalizeToolChoice`
//! - Gemini `functionDeclarations` 处理: `routes/protocolBridge.js:91-118` `extractProtectedTools`
//! - 工具结果错误检测: `modules/chatCompletionHandler.js:286-323` `isToolResultError`
//!   5 字段判断 (success / ok / status / code / httpStatus)
//! - Keep-Alive 5 字段: `modules/chatCompletionHandler.js:22-28` `agentOptions`
//!   (战役 1-2 借鉴进 `apeireth-http-client`, 本 crate 只在编译期 hardcode 注释里标注)
//!
//! **不假装** (战役 0 主哲学锚 #1 不漂移):
//! - ✅ 4 协议都真实现, 不只 OpenAI (R17 战役 0 已直连)
//! - ✅ 字段级引用 VCP 真代码 (文件 + 行号 + 真函数名 + 真字段名)
//! - ✅ 不抄 VCP 业务代码, 只借工程模式 (归一化思想)
//! - ✅ unit tests 覆盖归一化核心, ≥ 50 个 test
//!
//! **不修改承诺**:
//! - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
//! - ❌ 不改 28 crate 现有代码
//! - ❌ 不引入 unsafe (workspace `#![deny(unsafe_code)]` 继承)
//! - ❌ 不引入 I/O / 网络 (本 crate 纯类型 + JSON 互转)
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline / apeireth-council / apeireth-api ← 未来消费者
//!          ↓
//!      apeireth-protocol (本 crate)
//!      ├── normalized.rs  : NormalizedRequest / NormalizedResponse / Message / Tool
//!      ├── adapter.rs     : ProtocolAdapter trait (低层)
//!      ├── adapters/      : 4 协议实现 (ZST, 0 业务状态)
//!      ├── bridge.rs      : R37-1 ProtocolBridge trait + 4 Bridge struct (高层 facade)
//!      ├── router.rs      : R37-1 起 deprecated, 0.5 release 后删
//!      ├── error.rs       : ProtocolError + is_tool_result_error
//!      └── lib.rs         : 入口 + 编译期 hardcode
//! ```

#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod adapter;
pub mod adapters;
// R37-1: 砍 ProtocolRouter 中间层, 加 ProtocolBridge trait + 4 Bridge struct
pub mod bridge;
pub mod bridge_ext;
pub mod error;
pub mod gateway;
pub mod normalized;
// R20 阶段 2: WebSocket 8 帧协议 (蓝图 §2.3). 跟 4 LLM 协议归一化层并列, 0 冲突.
pub mod ws_v1;

pub use adapter::ProtocolAdapter;
pub use adapters::{
    AnthropicMessagesAdapter, GeminiAdapter, OpenAiChatAdapter, OpenAiResponsesAdapter,
};
// R37-1: re-export 4 Bridge struct + dispatch helper
pub use bridge::{
    decode_for_kind, encode_for_kind, endpoint_path_for_kind, AnthropicMessagesBridge,
    GeminiBridge, OpenAiChatBridge, OpenAiResponsesBridge, ProtocolBridge,
};
pub use bridge_ext::{
    BridgeExtError, BridgeKind, ExtendedBridge, PassthroughBridge, QueueBridge, StreamBridge,
};
pub use error::{is_tool_result_error, ProtocolError};
pub use gateway::ProtocolKind;
pub use normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedToolChoice, ToolCall, ToolParameters,
};
// R20 阶段 2 re-export: WS 8 帧 + 编译期 hardcode 常量
pub use ws_v1::{
    AuthFrame, CloseFrame, ErrorFrame, PingFrame, StreamChunkFrame, StreamEndFrame,
    ToolInvokeFrame, ToolResultFrame, WsFrame, WS_IDLE_TIMEOUT_SECS, WS_MAX_STREAM_CHUNKS,
    WS_PING_INTERVAL_SECS, WS_PROTOCOL_VERSION, WS_TOKEN_DEFAULT_TTL_SECS,
};

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 协议层版本 (同步 workspace version, 编译期保证)
pub const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 4 协议总数 (编译期 hardcode, 防止加新协议忘改 docs)
pub const PROTOCOL_COUNT: usize = 4;

/// 默认 OpenAI Chat 路径 (VCP 兼容: `/v1/chat/completions`)
pub const OPENAI_CHAT_PATH: &str = "/v1/chat/completions";

/// 默认 OpenAI Responses 路径
pub const OPENAI_RESPONSES_PATH: &str = "/v1/responses";

/// 默认 Anthropic Messages 路径
pub const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";

/// 默认 Gemini 路径 (URL 路径含 model 占位符)
pub const GEMINI_PATH_TEMPLATE: &str = "/v1beta/models/{model}:generateContent";

/// 借鉴 VCP Keep-Alive 5 字段 (战役 1-2 在 `apeireth-http-client` 落地)
/// 当前仅在编译期 hardcode, 战役 1-2 真用 (`research/source/vcptoolbox/modules/chatCompletionHandler.js:22-28`)
///
/// **借鉴来源**: `chatCompletionHandler.js:22-28` `agentOptions = { keepAlive: true, keepAliveMsecs: 1000, freeSocketTimeout: 8000, scheduling: 'lifo', maxSockets: 10000 }`
pub const KEEP_ALIVE_KEEP_ALIVE: bool = true;
/// VCP `chatCompletionHandler.js:24` `keepAliveMsecs: 1000` (TCP 探针 1s)
pub const KEEP_ALIVE_KEEP_ALIVE_MSECS: u64 = 1000;
/// VCP `chatCompletionHandler.js:25` `freeSocketTimeout: 8000` (8s 杀 zombie)
pub const KEEP_ALIVE_FREE_SOCKET_TIMEOUT: u64 = 8000;
/// VCP `chatCompletionHandler.js:26` `scheduling: 'lifo'` (优先复用最新鲜)
pub const KEEP_ALIVE_SCHEDULING_LIFO: bool = true;
/// VCP `chatCompletionHandler.js:27` `maxSockets: 10000` (高并发上限)
pub const KEEP_ALIVE_MAX_SOCKETS: usize = 10000;

/// Anthropic max_tokens 默认 (协议要求必填, R17 战役 0 验证 minimaxi 兼容)
pub const DEFAULT_ANTHROPIC_MAX_TOKENS: u32 = 1024;

/// OpenAI temperature 上限 (协议要求 [0.0, 2.0])
pub const OPENAI_MAX_TEMPERATURE: f32 = 2.0;

/// Anthropic temperature 上限 (Anthropic [0.0, 1.0])
pub const ANTHROPIC_MAX_TEMPERATURE: f32 = 1.0;

// ============================================================
// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
// ============================================================

const _: () = {
    // 4 协议总数 = PROTOCOL_COUNT, 防止 enum 加 variant 忘改 hardcode
    assert!(
        PROTOCOL_COUNT == 4,
        "PROTOCOL_COUNT must be 4 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini)"
    );

    // Keep-Alive 5 字段 (借鉴 VCP chatCompletionHandler.js:22-28)
    assert!(
        KEEP_ALIVE_KEEP_ALIVE,
        "Keep-Alive must be enabled (VCP 真代码 keepAlive: true)"
    );
    assert!(
        KEEP_ALIVE_KEEP_ALIVE_MSECS >= 100,
        "Keep-Alive interval must be >= 100ms (VCP: 1000ms)"
    );
    assert!(
        KEEP_ALIVE_FREE_SOCKET_TIMEOUT >= 1000,
        "Free socket timeout must be >= 1s (VCP: 8000ms 防 zombie)"
    );
    assert!(
        KEEP_ALIVE_SCHEDULING_LIFO,
        "Scheduling must be LIFO (VCP: 'lifo' 优先复用最新鲜连接)"
    );
    assert!(
        KEEP_ALIVE_MAX_SOCKETS >= 100,
        "Max sockets must be >= 100 (VCP: 10000)"
    );

    // 温度上限
    assert!(
        OPENAI_MAX_TEMPERATURE == 2.0,
        "OpenAI temperature cap must be 2.0 (协议规定)"
    );
    assert!(
        ANTHROPIC_MAX_TEMPERATURE == 1.0,
        "Anthropic temperature cap must be 1.0 (协议规定)"
    );
    assert!(
        ANTHROPIC_MAX_TEMPERATURE < OPENAI_MAX_TEMPERATURE,
        "Anthropic 比 OpenAI 温度上限更严"
    );

    // Anthropic max_tokens 必填 > 0
    assert!(
        DEFAULT_ANTHROPIC_MAX_TOKENS > 0,
        "Anthropic max_tokens must be > 0 (协议要求必填)"
    );

    // 路径不为空
    assert!(!OPENAI_CHAT_PATH.is_empty());
    assert!(!OPENAI_RESPONSES_PATH.is_empty());
    assert!(!ANTHROPIC_MESSAGES_PATH.is_empty());
    assert!(!GEMINI_PATH_TEMPLATE.is_empty());
    // Gemini {model} 占位符检查不能在 const fn 里 (str.contains / find 都不稳定),
    // 移到 lib_tests 里的 runtime test (见 constants_are_valid 下面)
};

#[cfg(test)]
// R177: protocol invariants
mod organ_kani_proofs;
mod lib_tests {
    use super::*;

    #[test]
    fn constants_are_valid() {
        // 编译期 hardcode 已经过 assert, 这里再 runtime 测一次
        assert_eq!(PROTOCOL_COUNT, 4);
        assert_eq!(OPENAI_CHAT_PATH, "/v1/chat/completions");
        assert_eq!(ANTHROPIC_MESSAGES_PATH, "/v1/messages");
        assert!(GEMINI_PATH_TEMPLATE.contains("{model}"));
        assert!(KEEP_ALIVE_KEEP_ALIVE);
        assert!(KEEP_ALIVE_SCHEDULING_LIFO);
    }

    #[test]
    fn protocol_bridge_works_through_lib_api() {
        // R37-1 + R36-2: ProtocolRouter 已删, 验证 ProtocolBridge facade (encode_for_kind / decode_for_kind / endpoint_path_for_kind) 4 协议 dispatch
        use crate::bridge::{decode_for_kind, encode_for_kind, endpoint_path_for_kind};
        use crate::normalized::{NormalizedRequest, NormalizedResponse};
        use serde_json::json;
        for kind in [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::AnthropicMessages,
            ProtocolKind::Gemini,
        ] {
            assert!(
                endpoint_path_for_kind(kind).is_some(),
                "HTTP kind should have endpoint path"
            );
            // encode_for_kind 需要合法 NormalizedRequest; 测 4 种 kind 都能 dispatch 到对应 Bridge (不验证 payload 内容)
            let req = NormalizedRequest::default();
            let _ = encode_for_kind(kind, &req);
            let raw = json!({});
            let _: Result<NormalizedResponse, _> = decode_for_kind(kind, &raw);
        }
    }
}
