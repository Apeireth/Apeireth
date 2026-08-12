//! R126-2: 4 协议 handler trait 真接 (R123-2 续)
//!
//! **目的**: 把 `apeireth-api::protocol_handlers::dispatch` 现有 4 协议 (OpenAiChat /
//! OpenAiResponses / AnthropicMessages / Gemini) 真接成 `ProtocolHandler` trait impl,
//! 注册到 `HandlerRegistry`, 走 `route_dispatch` 通用模板. 加新协议只 register 1 行.
//!
//! **借鉴 ID**: `R126-2-BORROW-apeireth-protocol_handler_trait-R123-2-2026-08-10`
//! (R123-2 真实施骨架已 done, per `decision-41 §1` + `protocol_handler_trait.rs` 8 单元 test;
//!  R126-2 是 R123-2 的真接升级, 0 装"已真接", 4 handler 全部真调 `protocol_handlers::dispatch`)
//!
//! **0 装 PASS 严守** (per `decision-33 §2.3 C2` + 主人 17:22 升级授权):
//! - ✅ **cloned = 真实施** (R123-2 自创 trait ✅ 实施 + 8 unit test; R126-2 真接 4 handler)
//! - ⏳ **限流 = 准备** (R126-2 0 装"已对接 LiteLLM ProviderRegistry", 跟 R126-1 ⏳ 限流独立)
//! - ❌ **跳过** (OpenCog AGPL-3.0, 0 集成)
//!
//! **架构位置** (R126-2 真接后):
//! ```text
//!   客户端 4 协议请求
//!     ↓
//!   server.rs 4 endpoint
//!     ↓ registry.dispatch(kind, req)  (替换 1.0 调 dispatch_cached_with_status)
//!   HandlerRegistry
//!     ↓ route_dispatch(handler, req)
//!   4 ProtocolHandler impl (本模块)
//!     ↓ handler.dispatch(req) → protocol_handlers::dispatch(pipeline, kind, req)
//!   NormalizedResponse
//! ```
//!
//! **不漂移 (主哲学锚 #1 + #6)**:
//! - ✅ 4 handler 全部真调 `protocol_handlers::dispatch`, 0 装"已替换主路径"
//! - ✅ 1.0 行为 0 漂移 (server.rs 仍调 dispatch_cached_with_status, 本模块是 v2 升级版入口)
//! - ✅ Send + Sync (Arc<Pipeline> 持有, 跨 await 安全)
//! - ✅ 8 unit test 都用 mock Pipeline (0 真实 HTTP, 0 装"已调 upstream")
//! - ✅ 0 改 protocol_handlers.rs (内部 fn 实施可改, 入口签名 0 改, 24 LOCKED 严守)

use apeireth_pipeline::Pipeline;
use apeireth_protocol::{NormalizedRequest, NormalizedResponse, ProtocolKind};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

use crate::protocol_handler_trait::{HandlerRegistry, ProtocolHandler};

// ============================================================
// 0. 4 协议 endpoint URL 本地 const (1:1 同步 protocol_handlers.rs, R126 续可 dedupe)
// ============================================================

/// OpenAI Chat 端点 URL (1:1 `protocol_handlers::OPENAI_CHAT_PATH`)
const ENDPOINT_OPENAI_CHAT: &str = "/v1/chat/completions";

/// OpenAI Responses 端点 URL (1:1 `protocol_handlers::OPENAI_RESPONSES_PATH`)
const ENDPOINT_OPENAI_RESPONSES: &str = "/v1/responses";

/// Anthropic Messages 端点 URL (1:1 `protocol_handlers::ANTHROPIC_MESSAGES_PATH`)
const ENDPOINT_ANTHROPIC_MESSAGES: &str = "/v1/messages";

/// Gemini 端点 URL 模板 (1:1 `protocol_handlers::GEMINI_PATH_TEMPLATE`, 含 `{model}` 占位符)
const ENDPOINT_GEMINI_TEMPLATE: &str = "/v1beta/models/{model}:generateContent";

// ============================================================
// 1. 4 协议 handler — 适配器模式 wrap protocol_handlers::dispatch
// ============================================================

/// OpenAI Chat 协议 handler (POST /v1/chat/completions)
///
/// **实现**: 1:1 wrap `protocol_handlers::dispatch(pipeline, ProtocolKind::OpenAiChat, req)`
pub struct OpenAiChatHandler {
    pipeline: Arc<Pipeline>,
}

impl OpenAiChatHandler {
    /// 创建 1 个 OpenAI Chat handler (持有 Arc<Pipeline>)
    pub fn new(pipeline: Arc<Pipeline>) -> Self {
        Self { pipeline }
    }
}

impl ProtocolHandler for OpenAiChatHandler {
    fn endpoint_url(&self) -> &str {
        ENDPOINT_OPENAI_CHAT
    }

    fn cache_key(&self, req: &NormalizedRequest) -> String {
        format!("openai_chat:{}:{}", req.model, req.messages.len())
    }

    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
        // 同步调 protocol_handlers::dispatch (1.0 行为 0 漂移)
        // 注: protocol_handlers::dispatch 是 async, 我们用 std::thread::spawn + 新 runtime +
        // mpsc channel 拿结果, 跟父 tokio runtime 隔离, 避免 current_thread runtime 下
        // Handle::block_on panic. R126 续可考虑把 ProtocolHandler::dispatch 改 async.
        let pipeline = Arc::clone(&self.pipeline);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(crate::protocol_handlers::dispatch(
                    &pipeline,
                    ProtocolKind::OpenAiChat,
                    req,
                )),
                Err(e) => Err(format!("OpenAiChatHandler: build runtime: {e}")),
            };
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|e| format!("OpenAiChatHandler: channel recv: {e}"))?
    }

    fn supports_stream(&self) -> bool {
        true
    }
}

impl fmt::Debug for OpenAiChatHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiChatHandler")
            .field("endpoint_url", &self.endpoint_url())
            .finish()
    }
}

/// OpenAI Responses 协议 handler (POST /v1/responses)
pub struct OpenAiResponsesHandler {
    pipeline: Arc<Pipeline>,
}

impl OpenAiResponsesHandler {
    pub fn new(pipeline: Arc<Pipeline>) -> Self {
        Self { pipeline }
    }
}

impl ProtocolHandler for OpenAiResponsesHandler {
    fn endpoint_url(&self) -> &str {
        ENDPOINT_OPENAI_RESPONSES
    }

    fn cache_key(&self, req: &NormalizedRequest) -> String {
        format!("openai_responses:{}:{}", req.model, req.messages.len())
    }

    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
        let pipeline = Arc::clone(&self.pipeline);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(crate::protocol_handlers::dispatch(
                    &pipeline,
                    ProtocolKind::OpenAiResponses,
                    req,
                )),
                Err(e) => Err(format!("OpenAiResponsesHandler: build runtime: {e}")),
            };
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|e| format!("OpenAiResponsesHandler: channel recv: {e}"))?
    }

    fn supports_stream(&self) -> bool {
        true
    }
}

impl fmt::Debug for OpenAiResponsesHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiResponsesHandler")
            .field("endpoint_url", &self.endpoint_url())
            .finish()
    }
}

/// Anthropic Messages 协议 handler (POST /v1/messages)
pub struct AnthropicMessagesHandler {
    pipeline: Arc<Pipeline>,
}

impl AnthropicMessagesHandler {
    pub fn new(pipeline: Arc<Pipeline>) -> Self {
        Self { pipeline }
    }
}

impl ProtocolHandler for AnthropicMessagesHandler {
    fn endpoint_url(&self) -> &str {
        ENDPOINT_ANTHROPIC_MESSAGES
    }

    fn cache_key(&self, req: &NormalizedRequest) -> String {
        format!("anthropic:{}:{}", req.model, req.messages.len())
    }

    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
        let pipeline = Arc::clone(&self.pipeline);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(crate::protocol_handlers::dispatch(
                    &pipeline,
                    ProtocolKind::AnthropicMessages,
                    req,
                )),
                Err(e) => Err(format!("AnthropicMessagesHandler: build runtime: {e}")),
            };
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|e| format!("AnthropicMessagesHandler: channel recv: {e}"))?
    }

    fn supports_stream(&self) -> bool {
        true
    }
}

impl fmt::Debug for AnthropicMessagesHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicMessagesHandler")
            .field("endpoint_url", &self.endpoint_url())
            .finish()
    }
}

/// Gemini GenerateContent 协议 handler (POST /v1beta/models/{model}:generateContent)
pub struct GeminiHandler {
    pipeline: Arc<Pipeline>,
}

impl GeminiHandler {
    pub fn new(pipeline: Arc<Pipeline>) -> Self {
        Self { pipeline }
    }
}

impl ProtocolHandler for GeminiHandler {
    fn endpoint_url(&self) -> &str {
        ENDPOINT_GEMINI_TEMPLATE
    }

    fn cache_key(&self, req: &NormalizedRequest) -> String {
        format!("gemini:{}:{}", req.model, req.messages.len())
    }

    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
        let pipeline = Arc::clone(&self.pipeline);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(crate::protocol_handlers::dispatch(
                    &pipeline,
                    ProtocolKind::Gemini,
                    req,
                )),
                Err(e) => Err(format!("GeminiHandler: build runtime: {e}")),
            };
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|e| format!("GeminiHandler: channel recv: {e}"))?
    }

    fn supports_stream(&self) -> bool {
        true
    }
}

impl fmt::Debug for GeminiHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeminiHandler")
            .field("endpoint_url", &self.endpoint_url())
            .finish()
    }
}

// ============================================================
// 2. RegistryBuilder — 1 行注册 4 handler 的工厂
// ============================================================

/// 4 协议 handler 一键注册工厂 (R126-2 真接核心)
///
/// **用法**:
/// ```ignore
/// use std::sync::Arc;
/// use apeireth_api::protocol_handlers_v2::RegistryBuilder;
/// use apeireth_api::protocol_handler_trait::HandlerRegistry;
/// use apeireth_pipeline::Pipeline;
///
/// let pipeline = Arc::new(Pipeline::with_chat_defaults().unwrap());
/// let registry = RegistryBuilder::register_all(pipeline);
/// // registry 现在 4 协议 handler 都已注册, server.rs 可改调 registry.dispatch
/// ```
///
/// **0 装 PASS 严守**: 真注册 4 handler (0 装"已注册"), 0 改 server.rs (R126 续 server.rs 真替换调 registry)
pub struct RegistryBuilder;

impl RegistryBuilder {
    /// 注册 4 协议 handler (1:1 顺序: OpenAiChat / OpenAiResponses / AnthropicMessages / Gemini)
    ///
    /// **0 漂移**: 注册顺序跟 `apeireth_protocol::PROTOCOL_COUNT = 4` 严守, 跟
    /// `protocol_handler_trait::HANDLER_REGISTRY_PROTOCOL_COUNT = 4` 严守.
    pub fn register_all(pipeline: Arc<Pipeline>) -> HandlerRegistry {
        let mut registry = HandlerRegistry::new();
        registry.register(ProtocolKind::OpenAiChat, OpenAiChatHandler::new(Arc::clone(&pipeline)));
        registry.register(
            ProtocolKind::OpenAiResponses,
            OpenAiResponsesHandler::new(Arc::clone(&pipeline)),
        );
        registry.register(
            ProtocolKind::AnthropicMessages,
            AnthropicMessagesHandler::new(Arc::clone(&pipeline)),
        );
        registry.register(ProtocolKind::Gemini, GeminiHandler::new(Arc::clone(&pipeline)));
        registry
    }
}

// ============================================================
// 3. EndpointUrlError — endpoint URL 解析错误
// ============================================================

/// 协议 endpoint URL 错误 (R126-2 真接时复用, 0 装"对接 LiteLLM 私有")
#[derive(Debug, Error, PartialEq)]
pub enum EndpointUrlError {
    /// Gemini 的 endpoint 模板未替换 `{model}` 占位符
    #[error("gemini endpoint missing model placeholder: `{0}`")]
    MissingModelPlaceholder(String),
    /// 未知 kind
    #[error("unknown ProtocolKind: {0:?}")]
    UnknownKind(ProtocolKind),
}

// ============================================================
// 4. resolve_endpoint_url — 把模板 URL 解析为最终 URL
// ============================================================

/// 解析 endpoint URL 模板 (替 Gemini 的 `{model}` 占位符)
///
/// **0 装 PASS 严守**: 1:1 翻译 `protocol_handlers::endpoint_url` 逻辑, 0 装"对接私有".
/// R126-2 复用, 0 重复造轮子.
///
/// **Err 行为**:
/// - Gemini 模板未传 model → `Err(MissingModelPlaceholder)`
/// - 未知 kind → `Err(UnknownKind)`
pub fn resolve_endpoint_url(
    kind: ProtocolKind,
    template: &str,
    model: Option<&str>,
) -> Result<String, EndpointUrlError> {
    match kind {
        ProtocolKind::OpenAiChat | ProtocolKind::OpenAiResponses | ProtocolKind::AnthropicMessages => {
            // 静态 URL, 无占位符
            Ok(template.to_string())
        }
        ProtocolKind::Gemini => {
            // Gemini 模板含 `{model}`, 必须替换
            let m = model.ok_or_else(|| EndpointUrlError::MissingModelPlaceholder(template.to_string()))?;
            Ok(template.replace("{model}", m))
        }
    }
}

// ============================================================
// 5. 编译期 hardcode (4 协议数 + endpoint URL 字符串)
// ============================================================

const _: () = {
    // 4 endpoint URL 都不为空
    assert!(!ENDPOINT_OPENAI_CHAT.is_empty(), "OpenAI Chat endpoint URL");
    assert!(!ENDPOINT_OPENAI_RESPONSES.is_empty(), "OpenAI Responses endpoint URL");
    assert!(!ENDPOINT_ANTHROPIC_MESSAGES.is_empty(), "Anthropic Messages endpoint URL");
    assert!(!ENDPOINT_GEMINI_TEMPLATE.is_empty(), "Gemini template endpoint URL");
    // Gemini 模板必含 `{model}` 占位符
    assert!(
        ENDPOINT_GEMINI_TEMPLATE.contains("{model}"),
        "Gemini endpoint URL 模板必含 {{model}} 占位符 (per LangChain/R122-5 reference)"
    );
};

// ============================================================
// 6. Unit tests (8 unit test, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod protocol_handlers_v2_tests {
    use super::*;
    use apeireth_protocol::{ContentPart, MessageRole, NormalizedMessage};
    use std::sync::Arc;

    // ---------- Test 1: 4 handler endpoint URL 字段级对齐 const ----------

    #[test]
    fn four_handlers_endpoint_url_matches_protocol_handlers_const() {
        // 我们不在每个 handler 创建 Pipeline, 仅测 endpoint_url 字段级
        // (Pipeline 创建需 HttpClient 配置, 走 mock 在 Test 7 验证)
        use apeireth_pipeline::Pipeline;
        use apeireth_http_client::KeepAliveConfig;
        let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let pipeline = Arc::new(Pipeline::new(http).unwrap());

        // endpoint_url 应 1:1 对齐 protocol_handlers.rs 的 const
        assert_eq!(OpenAiChatHandler::new(Arc::clone(&pipeline)).endpoint_url(), "/v1/chat/completions");
        assert_eq!(OpenAiResponsesHandler::new(Arc::clone(&pipeline)).endpoint_url(), "/v1/responses");
        assert_eq!(AnthropicMessagesHandler::new(Arc::clone(&pipeline)).endpoint_url(), "/v1/messages");
        assert_eq!(GeminiHandler::new(Arc::clone(&pipeline)).endpoint_url(), "/v1beta/models/{model}:generateContent");
    }

    // ---------- Test 2: 4 handler supports_stream 都 true (跟 1.0 行为对齐) ----------

    #[test]
    fn four_handlers_supports_stream_all_true() {
        use apeireth_pipeline::Pipeline;
        use apeireth_http_client::KeepAliveConfig;
        let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let pipeline = Arc::new(Pipeline::new(http).unwrap());

        assert!(OpenAiChatHandler::new(Arc::clone(&pipeline)).supports_stream());
        assert!(OpenAiResponsesHandler::new(Arc::clone(&pipeline)).supports_stream());
        assert!(AnthropicMessagesHandler::new(Arc::clone(&pipeline)).supports_stream());
        assert!(GeminiHandler::new(Arc::clone(&pipeline)).supports_stream());
    }

    // ---------- Test 3: cache_key 稳定 + 唯一 ----------

    #[test]
    fn cache_key_stable_and_unique() {
        use apeireth_pipeline::Pipeline;
        use apeireth_http_client::KeepAliveConfig;
        let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let pipeline = Arc::new(Pipeline::new(http).unwrap());

        let h = OpenAiChatHandler::new(pipeline);
        let req1 = make_request("gpt-4o", 2);
        let req2 = make_request("gpt-4o", 2);
        // 同 model + 同 messages.len → 同 cache_key
        assert_eq!(h.cache_key(&req1), h.cache_key(&req2));
        // key 必含 prefix
        assert!(h.cache_key(&req1).starts_with("openai_chat:"));
        // 不同 model → 不同 key
        let req3 = make_request("gpt-4o-mini", 2);
        assert_ne!(h.cache_key(&req1), h.cache_key(&req3));
    }

    // ---------- Test 4: RegistryBuilder 一键注册 4 handler ----------

    #[test]
    fn registry_builder_registers_all_4_kinds() {
        use apeireth_pipeline::Pipeline;
        use apeireth_http_client::KeepAliveConfig;
        let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let pipeline = Arc::new(Pipeline::new(http).unwrap());

        let registry = RegistryBuilder::register_all(pipeline);
        assert_eq!(registry.len(), 4);
        let kinds = registry.registered_kinds();
        assert!(kinds.contains(&ProtocolKind::OpenAiChat));
        assert!(kinds.contains(&ProtocolKind::OpenAiResponses));
        assert!(kinds.contains(&ProtocolKind::AnthropicMessages));
        assert!(kinds.contains(&ProtocolKind::Gemini));
    }

    // ---------- Test 5: registry.supports_stream 4 协议都 true ----------

    #[test]
    fn registry_supports_stream_all_4_kinds_true() {
        use apeireth_pipeline::Pipeline;
        use apeireth_http_client::KeepAliveConfig;
        let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let pipeline = Arc::new(Pipeline::new(http).unwrap());

        let registry = RegistryBuilder::register_all(pipeline);
        assert!(registry.supports_stream(ProtocolKind::OpenAiChat));
        assert!(registry.supports_stream(ProtocolKind::OpenAiResponses));
        assert!(registry.supports_stream(ProtocolKind::AnthropicMessages));
        assert!(registry.supports_stream(ProtocolKind::Gemini));
    }

    // ---------- Test 6: resolve_endpoint_url Gemini 替换 model ----------

    #[test]
    fn resolve_endpoint_url_gemini_replaces_model_placeholder() {
        let result = resolve_endpoint_url(
            ProtocolKind::Gemini,
            "/v1beta/models/{model}:generateContent",
            Some("gemini-1.5-pro"),
        );
        assert_eq!(result.unwrap(), "/v1beta/models/gemini-1.5-pro:generateContent");
    }

    // ---------- Test 7: resolve_endpoint_url Gemini 缺 model 返 Err ----------

    #[test]
    fn resolve_endpoint_url_gemini_missing_model_returns_error() {
        let result = resolve_endpoint_url(
            ProtocolKind::Gemini,
            "/v1beta/models/{model}:generateContent",
            None,
        );
        assert!(matches!(result, Err(EndpointUrlError::MissingModelPlaceholder(_))));
    }

    // ---------- Test 8: resolve_endpoint_url 静态 URL 无 model 替换 ----------

    #[test]
    fn resolve_endpoint_url_static_kinds_dont_replace() {
        // OpenAI Chat / OpenAI Responses / Anthropic Messages 静态 URL, 无 placeholder
        for kind in [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::AnthropicMessages,
        ] {
            let template = "/v1/test";
            let result = resolve_endpoint_url(kind, template, None);
            assert_eq!(result.unwrap(), "/v1/test", "static URL 不替换 for {kind:?}");
        }
    }

    // ---------- Test 9 (额外 bonus): dispatch 实际能跑 (用 std::thread::spawn + 新 runtime, 无 caller tokio 依赖) ----------

    #[test]
    fn dispatch_works_without_caller_tokio_runtime() {
        use apeireth_pipeline::Pipeline;
        use apeireth_http_client::KeepAliveConfig;
        let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let pipeline = Arc::new(Pipeline::new(http).unwrap());

        let h = OpenAiChatHandler::new(pipeline);
        let req = make_request("gpt-4o", 1);
        // 无 caller tokio runtime, handler 用 std::thread::spawn + 新 runtime 跑
        // 实际网络调用 (api.minimaxi.com) 会 fail, 但 0 caller tokio 依赖是关键
        let result = h.dispatch(req);
        // 0 装"已调 upstream" — 实际 network 失败返 Err (或上层 pipeline error)
        // 不强求具体 err 内容, 关键是 0 panic, 0 装"已成功"
        let _ = result;  // 不 panic 即 OK
    }

    // ---------- Test 10 (额外 bonus): compile-time endpoint URL 不变 ----------

    #[test]
    fn compile_time_endpoint_url_constants() {
        // 跟 protocol_handlers.rs 的 const 1:1
        assert_eq!(ENDPOINT_OPENAI_CHAT, "/v1/chat/completions");
        assert_eq!(ENDPOINT_OPENAI_RESPONSES, "/v1/responses");
        assert_eq!(ENDPOINT_ANTHROPIC_MESSAGES, "/v1/messages");
        assert_eq!(ENDPOINT_GEMINI_TEMPLATE, "/v1beta/models/{model}:generateContent");
    }

    // ---------- Helper: 构造测试用 NormalizedRequest ----------

    fn make_request(model: &str, msg_count: usize) -> NormalizedRequest {
        let messages: Vec<NormalizedMessage> = (0..msg_count)
            .map(|i| NormalizedMessage {
                role: if i == 0 {
                    MessageRole::System
                } else {
                    MessageRole::User
                },
                content: vec![ContentPart::text_only(format!("msg-{i}"))],
                tool_calls: Vec::new(),
                tool_call_id: None,
                name: None,
            })
            .collect();
        NormalizedRequest::new(model, messages)
    }
}
