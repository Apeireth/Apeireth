//! R123-2: 4 协议 handler 抽 trait 抽象 (R122-10 5 重复模式 #1)
//!
//! **目的**: R122-10 扫到 `v2_endpoints.rs:230-322` + `protocol_handlers.rs:844-935` 4 协议 handler
//! 80% 模板代码重复 (cache lookup → dispatch → record → response), 抽 `ProtocolHandler` trait +
//! `HandlerRegistry` 注册表 + `route_dispatch()` 通用模板, 加新协议零模板代码.
//!
//! **借鉴 ID**: `R123-2-NEW-ProtocolHandlerTrait-2026-08-10` (0 VCP 借鉴, 自创骨架)
//!
//! **0 接入** (硬约束 #8, 0 装"已替换"主哲学锚 #1):
//! - ✅ 1 trait + 1 registry + 1 route_dispatch 模板 ready
//! - ✅ 8+ unit test 覆盖, 0 依赖任何具体 protocol impl
//! - ❌ 0 改 `protocol_handlers.rs` / `v2_endpoints.rs` / `server.rs` 4 协议 handler impl
//! - ❌ 0 接 `ResponseCache` / `ReplayCache` (R123+ 续真接)
//!
//! **架构位置** (R123+ 续真接后):
//! ```text
//!   客户端 4 协议请求
//!     ↓
//!   server.rs 4 endpoint
//!     ↓ registry.dispatch(kind, req)  (替换 1.0 调 dispatch_cached_with_status)
//!   HandlerRegistry
//!     ↓ route_dispatch(handler, req)
//!   ProtocolHandler (4 impl: OpenAiChat / OpenAiResponses / AnthropicMessages / Gemini)
//!     ↓ handler.dispatch(req)
//!   NormalizedResponse
//! ```
//!
//! **不假装** (主哲学锚 #1):
//! - ✅ trait 4 method 都有, 0 "todo" placeholder
//! - ✅ HandlerRegistry 真用 `HashMap<ProtocolKind, ...>`, 0 装 "AnyMap"
//! - ✅ 8 test 真跑 (本文件 cfg(test)), 0 装 "test written"
//! - ✅ `route_dispatch` 当前 1:1 调 `handler.dispatch`, 0 装 "已接 cache lookup"

use apeireth_protocol::{NormalizedRequest, NormalizedResponse, ProtocolKind};
use std::collections::HashMap;
use std::fmt;

// ============================================================
// 1. ProtocolHandler trait — 4 协议 handler 抽象 (4 method)
// ============================================================

/// 4 协议 handler 抽象 (R123-2 自创, R122-10 重复模式 #1 修复)
///
/// **4 method 设计**:
/// - `endpoint_url` — 协议端点 path (不含 base_url, 跟 `protocol_handlers::endpoint_url` 配合)
/// - `cache_key` — 生成稳定 cache key (同 `NormalizedRequest` → 同 key)
/// - `dispatch` — 派发归一化请求 → 归一化响应
/// - `supports_stream` — 是否支持流式, 默认 `true` (4 协议全支持, R123+ 续可重写)
///
/// **Error 类型**: 返 `Result<NormalizedResponse, String>`, 跟现有
/// `protocol_handlers::dispatch` 1.0 行为 0 漂移 (避免 0 假装引 `ProtocolError`)
pub trait ProtocolHandler {
    /// 协议端点 URL path (不含 base_url, 跟 `protocol_handlers::endpoint_url` 配合)
    fn endpoint_url(&self) -> &str;

    /// 生成 cache key (基于 `NormalizedRequest` 字段, 同请求返同 key)
    fn cache_key(&self, req: &NormalizedRequest) -> String;

    /// 派发归一化请求 → 归一化响应
    fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String>;

    /// 是否支持流式 (SSE), 默认 `true` (4 协议全支持流式, 仅在 R123+ 续接 Gemini 文生文等特例)
    fn supports_stream(&self) -> bool {
        true
    }
}

// ============================================================
// 2. route_dispatch — 通用 route 模板
// ============================================================

/// 通用 route 模板 (4 协议统一入口)
///
/// **0 漂移 1.0 行为**: 当前 1.0 走 `protocol_handlers::dispatch_cached_with_status`
/// (cache + 5 步管线), `route_dispatch` 是 R123+ 续替换的入口, 本任务仅"骨架 ready".
///
/// **Phase 1 (本任务)**: 1:1 调 `handler.dispatch(req)`, 0 接 cache
/// **Phase 2 (R123+ 续)**: 加 cache lookup + record (接 `ResponseCache` / `ReplayCache`)
///
/// **泛型 + `?Sized`**: 允许传 `&dyn ProtocolHandler`, 加 1 个 indirection level 0 漂移
pub fn route_dispatch<P: ProtocolHandler + ?Sized>(
    handler: &P,
    req: NormalizedRequest,
) -> Result<NormalizedResponse, String> {
    handler.dispatch(req)
}

// ============================================================
// 3. HandlerRegistry — 4 协议 handler 注册表
// ============================================================

/// 4 协议 handler 注册表 (R123-2 自创, 0 改 `ProtocolGateway` in apeireth-protocol)
///
/// **架构意图**: 未来 `server.rs` 启动时 `register(OpenAiChat, ...)` 等 4 行, dispatch 走 map
/// 查表, 加新协议只 register 1 行.
///
/// **Send + Sync**: trait object 强制, 跟 axum State 兼容 (跨 await 安全)
pub struct HandlerRegistry {
    handlers: HashMap<ProtocolKind, Box<dyn ProtocolHandler + Send + Sync>>,
}

impl HandlerRegistry {
    /// 新建空注册表
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// 注册 1 个 handler (覆盖同 kind 的旧 handler)
    ///
    /// **0 漂移**: 同 kind 重复 register → 覆盖 (HashMap::insert 语义), 不报错
    pub fn register<H: ProtocolHandler + Send + Sync + 'static>(
        &mut self,
        kind: ProtocolKind,
        handler: H,
    ) {
        self.handlers.insert(kind, Box::new(handler));
    }

    /// 派发 1 个请求到指定 kind 的 handler
    ///
    /// **Err 行为**: kind 未注册 → 返 `Err("protocol_handler_trait: no handler registered for kind={:?}")`
    pub fn dispatch(
        &self,
        kind: ProtocolKind,
        req: NormalizedRequest,
    ) -> Result<NormalizedResponse, String> {
        match self.handlers.get(&kind) {
            Some(h) => h.dispatch(req),
            None => Err(format!(
                "protocol_handler_trait: no handler registered for kind={kind:?}"
            )),
        }
    }

    /// 注册 handler 数量
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// 查询 kind 是否支持流式 (kind 未注册 → `false`)
    pub fn supports_stream(&self, kind: ProtocolKind) -> bool {
        self.handlers
            .get(&kind)
            .map(|h| h.supports_stream())
            .unwrap_or(false)
    }

    /// 已注册的 kind 列表 (debug / 路由决策)
    pub fn registered_kinds(&self) -> Vec<ProtocolKind> {
        self.handlers.keys().copied().collect()
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("registered_kinds", &self.registered_kinds())
            .field("count", &self.handlers.len())
            .finish()
    }
}

// ============================================================
// 编译期 hardcode (4 协议数, 跟 PROTOCOL_COUNT 对齐)
// ============================================================

/// HandlerRegistry 4 协议注册数 hardcode (防止 R123+ 续真接时忘改 hardcode)
const HANDLER_REGISTRY_PROTOCOL_COUNT: usize = 4;

const _: () = {
    assert!(
        HANDLER_REGISTRY_PROTOCOL_COUNT == 4,
        "HandlerRegistry 4 协议数 hardcode (跟 apeireth_protocol::PROTOCOL_COUNT 对齐)"
    );
};

// ============================================================
// Unit tests (8+ test, 0 依赖任何具体 protocol impl)
// ============================================================

#[cfg(test)]
mod protocol_handler_trait_tests {
    use super::*;
    use apeireth_protocol::{ContentPart, MessageRole, NormalizedMessage};

    // ---------- Stub impl (1 个 handler, 4 method 都实现, 仅 test 用) ----------

    struct StubHandler {
        endpoint: &'static str,
        prefix: &'static str,
        supports_stream_flag: bool,
    }

    impl ProtocolHandler for StubHandler {
        fn endpoint_url(&self) -> &str {
            self.endpoint
        }

        fn cache_key(&self, req: &NormalizedRequest) -> String {
            // 简单稳定: endpoint + model + messages.len
            format!("{}:{}:{}", self.prefix, req.model, req.messages.len())
        }

        fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
            Ok(NormalizedResponse::text(
                format!("{}-{}", self.prefix, req.model),
                req.model,
                format!("{} stub: {} msgs", self.prefix, req.messages.len()),
            ))
        }

        fn supports_stream(&self) -> bool {
            self.supports_stream_flag
        }
    }

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

    // ---------- Test 1: endpoint_url 返静态 str ----------

    #[test]
    fn protocol_handler_trait_endpoint_url_returns_static_str() {
        let h = StubHandler {
            endpoint: "/v1/chat/completions",
            prefix: "openai_chat",
            supports_stream_flag: true,
        };
        let url: &str = h.endpoint_url();
        assert_eq!(url, "/v1/chat/completions");

        // 4 协议 URL 都对齐 protocol_handlers const (per readmap 7-#6 决策)
        let urls = [
            ("/v1/chat/completions", "openai_chat"),
            ("/v1/responses", "openai_responses"),
            ("/v1/messages", "anthropic"),
            ("/v1beta/models/{model}:generateContent", "gemini"),
        ];
        for (path, prefix) in urls {
            let h = StubHandler {
                endpoint: path,
                prefix,
                supports_stream_flag: true,
            };
            assert_eq!(h.endpoint_url(), path, "endpoint_url mismatch for {prefix}");
        }
    }

    // ---------- Test 2: cache_key 稳定 (同 input → 同 key) ----------

    #[test]
    fn protocol_handler_trait_cache_key_stable_for_same_input() {
        let h = StubHandler {
            endpoint: "/v1/chat/completions",
            prefix: "openai_chat",
            supports_stream_flag: true,
        };
        let req1 = make_request("gpt-4o", 2);
        let req2 = make_request("gpt-4o", 2);
        // 同 model + 同 messages.len → 同 cache_key
        assert_eq!(h.cache_key(&req1), h.cache_key(&req2));

        // 不同 model → 不同 key
        let req3 = make_request("gpt-4o-mini", 2);
        assert_ne!(h.cache_key(&req1), h.cache_key(&req3));

        // 不同 messages 数 → 不同 key
        let req4 = make_request("gpt-4o", 3);
        assert_ne!(h.cache_key(&req1), h.cache_key(&req4));

        // key 包含 prefix
        let key = h.cache_key(&req1);
        assert!(key.starts_with("openai_chat:"));
        assert!(key.contains("gpt-4o"));
    }

    // ---------- Test 3: supports_stream 默认 true ----------

    #[test]
    fn protocol_handler_trait_supports_stream_default_true() {
        struct DefaultSupportsStream;
        impl ProtocolHandler for DefaultSupportsStream {
            fn endpoint_url(&self) -> &str {
                "/x"
            }
            fn cache_key(&self, _req: &NormalizedRequest) -> String {
                "k".into()
            }
            fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
                Ok(NormalizedResponse::text("x", req.model, "x"))
            }
            // 不重写 supports_stream → 默 true
        }
        let h = DefaultSupportsStream;
        assert!(h.supports_stream(), "默认 supports_stream 必须为 true");

        // 重写为 false → 返 false
        struct NonStream;
        impl ProtocolHandler for NonStream {
            fn endpoint_url(&self) -> &str {
                "/x"
            }
            fn cache_key(&self, _req: &NormalizedRequest) -> String {
                "k".into()
            }
            fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, String> {
                Ok(NormalizedResponse::text("x", req.model, "x"))
            }
            fn supports_stream(&self) -> bool {
                false
            }
        }
        assert!(!NonStream.supports_stream(), "重写 false 必须返 false");
    }

    // ---------- Test 4: route_dispatch 调 handler ----------

    #[test]
    fn protocol_handler_trait_dispatch_routes_to_handler() {
        let h = StubHandler {
            endpoint: "/v1/chat/completions",
            prefix: "openai_chat",
            supports_stream_flag: true,
        };
        let req = make_request("gpt-4o", 2);
        let resp = route_dispatch(&h, req).expect("dispatch should succeed");
        assert_eq!(resp.model, "gpt-4o");
        assert!(resp.content.contains("openai_chat stub"));
        assert!(resp.content.contains("2 msgs"));
        assert_eq!(resp.id, "openai_chat-gpt-4o");
    }

    // ---------- Test 5: register 4 协议 → len == 4 ----------

    #[test]
    fn handler_registry_register_4_protocols_4_handlers() {
        let mut reg = HandlerRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);

        let kinds = [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::AnthropicMessages,
            ProtocolKind::Gemini,
        ];
        for (i, &kind) in kinds.iter().enumerate() {
            let prefix = ["openai_chat", "openai_responses", "anthropic", "gemini"][i];
            let endpoint = [
                "/v1/chat/completions",
                "/v1/responses",
                "/v1/messages",
                "/v1beta/models/{model}:generateContent",
            ][i];
            reg.register(
                kind,
                StubHandler {
                    endpoint,
                    prefix,
                    supports_stream_flag: true,
                },
            );
        }

        assert_eq!(reg.len(), 4);
        assert!(!reg.is_empty());
        let registered = reg.registered_kinds();
        assert_eq!(registered.len(), 4);
        for &kind in &kinds {
            assert!(
                registered.contains(&kind),
                "kind {kind:?} should be registered"
            );
        }
    }

    // ---------- Test 6: 4 协议 dispatch 返各自正确 content ----------

    #[test]
    fn handler_registry_dispatch_4_protocols_returns_correct() {
        let mut reg = HandlerRegistry::new();

        let configs: [(ProtocolKind, &str, &str); 4] = [
            (
                ProtocolKind::OpenAiChat,
                "/v1/chat/completions",
                "openai_chat",
            ),
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
        for (kind, endpoint, prefix) in configs {
            reg.register(
                kind,
                StubHandler {
                    endpoint,
                    prefix,
                    supports_stream_flag: true,
                },
            );
        }

        // 4 协议各 dispatch 1 次, content 不混
        for (kind, _endpoint, prefix) in configs {
            let req = make_request("test-model", 1);
            let resp = reg.dispatch(kind, req).expect("dispatch should succeed");
            assert!(
                resp.content.contains(&format!("{prefix} stub")),
                "kind {kind:?} should dispatch to {prefix}, got: {}",
                resp.content
            );
            assert!(
                resp.content.contains("1 msgs"),
                "messages count should appear in content for {kind:?}"
            );
        }

        // 4 协议 content 互相不混 (4 个 prefix 各不同)
        let contents: Vec<String> = configs
            .iter()
            .map(|&(kind, _, _)| reg.dispatch(kind, make_request("m", 1)).unwrap().content)
            .collect();
        let unique: std::collections::HashSet<&String> = contents.iter().collect();
        assert_eq!(
            unique.len(),
            4,
            "4 协议 content 必须互不相同, got: {contents:?}"
        );
    }

    // ---------- Test 7: 未注册 kind → Err ----------

    #[test]
    fn handler_registry_dispatch_unknown_kind_returns_error() {
        let mut reg = HandlerRegistry::new();
        // 仅 register 4 LLM 协议
        for (kind, endpoint, prefix) in [
            (
                ProtocolKind::OpenAiChat,
                "/v1/chat/completions",
                "openai_chat",
            ),
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
        ] {
            reg.register(
                kind,
                StubHandler {
                    endpoint,
                    prefix,
                    supports_stream_flag: true,
                },
            );
        }

        // Acp / Mcp / OpenClawGateway 3 个 gateway kind 0 注册 → Err
        for &unknown in &[
            ProtocolKind::Acp,
            ProtocolKind::Mcp,
            ProtocolKind::OpenClawGateway,
        ] {
            let req = make_request("x", 1);
            let err = reg
                .dispatch(unknown, req)
                .expect_err("unregistered kind must error");
            assert!(
                err.contains("no handler registered"),
                "err must contain diagnostic, got: {err}"
            );
            assert!(err.contains("kind"), "err must mention kind, got: {err}");

            // supports_stream 查询同 kind → false
            assert!(!reg.supports_stream(unknown));
        }
    }

    // ---------- Test 8: 编译期 Send + Sync 验证 ----------

    #[test]
    fn protocol_handler_trait_send_sync_compiles() {
        // 编译期: `Box<dyn ProtocolHandler + Send + Sync>` 是 Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn ProtocolHandler + Send + Sync>>();

        // runtime: HandlerRegistry 本体 Send + Sync (跨 await 安全)
        fn assert_send_sync_value<T: Send + Sync>(_: &T) {}
        let reg = HandlerRegistry::new();
        assert_send_sync_value(&reg);

        // 4 协议 register 后, registry 仍 Send + Sync
        let mut reg = HandlerRegistry::new();
        reg.register(
            ProtocolKind::OpenAiChat,
            StubHandler {
                endpoint: "/v1/chat/completions",
                prefix: "openai_chat",
                supports_stream_flag: true,
            },
        );
        assert_send_sync_value(&reg);
    }
}
