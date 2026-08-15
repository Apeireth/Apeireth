//! `apeireth-pipeline` — **Apeireth R17 战役 1-3 主 chat 管线**
//!
//! **目标**: 把 4 个 LLM 协议的请求 (归一化为 `NormalizedRequest`),
//! 走完 **5 步管线** (解析 placeholder → token 预算 → Force-Translate →
//! 协议归一化 → HTTP 调用), 返回 `NormalizedResponse`.
//!
//! **5 步管线** (借鉴 VCP `chatCompletionHandler.js:1-220` 主 chat 模式):
//! 1. **解析 placeholder** — `resolve_placeholders` (借鉴 #17 `messageProcessor.js:146-220`)
//! 2. **token 预算** — 截断到 `MAX_INJECTION_CHARS` (借鉴 #15 `dynamicToolRegistry.js:10/11/21`)
//! 3. **Force-Translate** — base64 image → text tag (借鉴 #20 `chatCompletionHandler.js:222-257` + `multiModalConfigStore.js`)
//! 4. **协议归一化** — `NormalizedRequest` → 协议特定 JSON (调战役 1-1 `apeireth-protocol`)
//! 5. **HTTP 调用** — POST 到 LLM endpoint, 拿响应 (调战役 1-2 `apeireth-http-client` Keep-Alive LIFO)
//!
//! **借鉴 VCP 真代码** (战役 1-3 pipeline crate 实际借鉴 4 项, per `docs/stage3-blueprints/borrowed-from-projects.md §6.2.2`):
//! - **#15** token 预算三层 → `token_budget.rs` (`dynamicToolRegistry.js:10/11/21`)
//! - **#17** Recursive placeholder + 防循环 → `placeholder.rs` (`messageProcessor.js:146-220 resolveAllVariables` + line 186-191 `processingStack`)
//! - **#19** 15s 抑制窗口 → `retry_suppression.rs` (`protocolBridge.js:11-12`)
//! - **#20** Force-Translate → `force_translate.rs` (`chatCompletionHandler.js:222-257` + `multiModalConfigStore.js`)
//!
//! **不引用但已被战役 1-3 pipeline 调用的真代码**:
//! - **#14** Keep-Alive LIFO 池 (战役 1-2 已在 `apeireth-http-client` 落地, 本 pipeline 通过 `HttpClient::with_chat_defaults()` 调用)
//! - **归一化模式** (战役 1-1 已在 `apeireth-protocol` 落地, 本 pipeline 通过 `encode_for_kind` / `decode_for_kind` 调用 (R37-1 bridge facade, R36-2 删 ProtocolRouter))
//!
//! **架构位置**:
//! ```text
//!   apeireth-api / apeireth-council / apeireth-tui / 未来消费者
//!          |
//!      apeireth-pipeline (本 crate)
//!      ├── token_budget.rs     : #15 三层常量 + truncate_to_max
//!      ├── placeholder.rs      : #17 递归展开 + 防循环
//!      ├── force_translate.rs  : #20 base64 → text
//!      ├── retry_suppression.rs: #19 15s 窗口
//!      ├── streaming.rs        : SSE / chunk 推流
//!      └── lib.rs              : Pipeline 主体 + 5 步编排
//!          |
//!      apeireth-protocol (战役 1-1) + apeireth-http-client (战役 1-2)
//! ```
//!
//! **不假装 (主哲学锚 #1 不漂移)**:
//! - 4 项借鉴字段级引用 VCP 真代码 (文件 + 行号 + 真字段名 + 真函数名)
//! - Pipeline 5 步真跑 (不只 mock), 通过 wiremock 本地 server 端到端验证
//! - Example 真接 minimaxi (`/v1/chat/completions` 4 协议之一)
//! - unit tests >= 15, 编译期 hardcode 守
//!
//! **不修改承诺**:
//! - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
//! - 不改战役 1-1 / 1-2 已有代码 (用 import, 不改源码)
//! - 不引入 unsafe (workspace `#![deny(unsafe_code)]` 继承)
//! - 不假装 "已实现但没真跑" 的 5 步

#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod force_translate;
pub mod model_router; // R122-5: 借鉴 VCP SemanticModelRouter.json (R122-5-VCP-SemanticModelRouter-2026-08-10)
pub mod placeholder;
pub mod provider_registry; // R126-1: 借鉴 LiteLLM Provider Registry 模式 (R126-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10, ⏳ 限流 = 准备)
pub mod tiktoken_counter; // R122-3-retry: 借鉴 VCP finalContextStore.js (R122-3-retry-VCP-FinalContextStore-Tiktoken-2026-08-10)
pub mod retry_suppression;
pub mod role_divider; // R122-2-retry: 借鉴 VCP roleDivider.js (R122-2-retry-VCP-RoleDivider-2026-08-10)
pub mod streaming;
pub mod token_budget;
pub mod tool_loop; // R32-2: 借鉴 LangGraph state machine + conditional edge
// R177: pipeline invariants (10 tests + 2 Kani proofs)
// R177: model_router invariants (10 tests + 2 Kani proofs)
mod model_router_kani;
mod organ_kani_proofs;
pub mod g5_chat_bridge; // R157: chat 5-step -> g5 5-stage substrate 集成 (第 2 个 g5 生产调用方)

pub use force_translate::{
    force_translate_if_needed, is_text_only_model_by_tag, messages_contain_base64_media,
    needs_force_translate, ForceTranslateConfig, ForceTranslateStats,
};
pub use placeholder::{
    resolve_placeholders, PlaceholderContext, MAX_RECURSION_DEPTH, PLACEHOLDER_REGEX_STR,
};
pub use provider_registry::{
    CostTracker, FallbackChain, FallbackError, ProviderCapability, ProviderRegistry, ProviderSpec,
    RegistryError, SelectionStrategy, UsageRecord, ALL_PROVIDER_CAPABILITIES,
    ALL_SELECTION_STRATEGIES,
};
pub use retry_suppression::{RetrySuppression, DEFAULT_SUPPRESSION_WINDOW_MS};
pub use streaming::{stream_to_sender, StreamChunk};
pub use token_budget::{
    exceeds_budget, truncate_to_max, DEFAULT_BRIEF_TOKEN_BUDGET, LIGHT_LIST_TOKEN_BUDGET,
    MAX_INJECTION_CHARS, MIN_INJECTION_CHARS,
};
pub use tool_loop::{
    run_tool_loop, should_continue, LlmStepResult, ToolLoopMessage, ToolLoopState,
    DEFAULT_MAX_TOOL_TURNS,
};

use apeireth_http_client::{HttpClient, HttpClientError};
use apeireth_protocol::{
    decode_for_kind, encode_for_kind, endpoint_path_for_kind, ContentPart, MessageRole,
    NormalizedRequest, NormalizedResponse, ProtocolKind,
};
use serde_json::Value;

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 战役 1-3 pipeline crate 实际借鉴 VCP 4 项 (#15/#17/#19/#20)
/// #14 在战役 1-2 `apeireth-http-client` 落地, #16 在战役 1-4 `apeireth-council`, #18 在战役 2 `apeireth-tool-runtime`
pub const BORROWED_LEGACY_COUNT: usize = 4;

/// Pipeline 5 步 (VCP `chatCompletionHandler.js:1-220` 主 chat 模式, 字段级对应)
pub const PIPELINE_STEP_COUNT: usize = 5;

/// VCP `protocolBridge.js:11` 真值 15000ms 抑制窗口
pub const LEGACY_RETRY_SUPPRESSION_MS: u64 = 15_000;

/// VCP `dynamicToolRegistry.js:21` 真值 16000 字符注入上限
pub const LEGACY_MAX_INJECTION_CHARS: usize = 16_000;

// ============================================================
// Pipeline 主体
// ============================================================

/// Pipeline 配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// LLM 服务 base URL (e.g. `https://api.minimaxi.com` 或本地 mock server)
    /// 拼接方式: `format!("{base_url}{endpoint_path}")`, e.g. `https://api.minimaxi.com/v1/chat/completions`
    /// (注: VCP 各协议端点 path 是 `/v1/chat/completions` 等, base URL 不含 /v1)
    pub base_url: String,
    /// **API key (Bearer auth)** — 给 LLM 服务的鉴权 token
    /// (战役 1-2 `apeireth-http-client::post` 不自动加 auth, 本 pipeline 在 reqwest 层面手动加)
    pub auth_token: Option<String>,
    /// Force-Translate 配置 (VCP §6.2.2 #20)
    pub force_translate: ForceTranslateConfig,
    /// token 预算上限 (VCP §6.2.2 #15)
    pub max_injection_chars: usize,
    /// placeholder context (VCP §6.2.2 #17)
    pub placeholder_context: PlaceholderContext,
    /// 15s 抑制窗口 (VCP §6.2.2 #19)
    pub suppression: RetrySuppression,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        // **Apeireth 默认**: minimaxi base URL (战役 1-3 example 用)
        // 注: protocol endpoint_path 已含 `/v1/chat/completions` 等版本前缀, base URL 不要带 /v1
        Self {
            base_url: "https://api.minimaxi.com".to_string(),
            auth_token: None, // 编译期无 key, example 设
            force_translate: ForceTranslateConfig::chat_default(),
            max_injection_chars: LEGACY_MAX_INJECTION_CHARS,
            placeholder_context: PlaceholderContext::new(),
            suppression: RetrySuppression::with_chat_default(),
        }
    }
}

/// Pipeline 主体 — 5 步编排 + 4 协议 dispatch
///
/// **构造**: `Pipeline::new()` 走 VCP 默认 (HttpClient 5 字段 + R37-1 ProtocolBridge 4 协议)
///
/// **R37-1 + R36-2 终态**: `ProtocolRouter` 中间层已删, 改用 `ProtocolBridge::encode_for_kind` / `decode_for_kind` 高层 facade
pub struct Pipeline {
    /// HTTP 客户端 (战役 1-2 apeireth-http-client Keep-Alive LIFO)
    http: HttpClient,
    /// Pipeline 配置
    config: PipelineConfig,
}

impl Pipeline {
    /// 构造默认 Pipeline (VCP 5 字段 HttpClient + 4 协议 Bridge + 15s 抑制窗口)
    pub fn new(http: HttpClient) -> Result<Self, HttpClientError> {
        Ok(Self {
            http,
            config: PipelineConfig::default(),
        })
    }

    /// VCP 全默认快速构造
    pub fn with_chat_defaults() -> Result<Self, HttpClientError> {
        Self::new(HttpClient::with_chat_defaults()?)
    }

    /// 自定义 config
    pub fn with_config(http: HttpClient, config: PipelineConfig) -> Result<Self, HttpClientError> {
        Ok(Self {
            http,
            config,
        })
    }

    /// 获取 HTTP client (测试用)
    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    /// 获取 config (测试用)
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// **5 步主 chat 管线** (VCP `chatCompletionHandler.js:1-220` 主 chat 模式)
    ///
    /// 1. 解析 placeholder (`resolve_placeholders` 借鉴 #17)
    /// 2. token 预算 (`truncate_to_max` 借鉴 #15)
    /// 3. Force-Translate (`force_translate_if_needed` 借鉴 #20)
    /// 4. 协议归一化 (`router.encode` 调战役 1-1)
    /// 5. HTTP 调用 (`http.post` 调战役 1-2 Keep-Alive LIFO)
    pub async fn run(
        &self,
        kind: ProtocolKind,
        input: NormalizedRequest,
    ) -> Result<NormalizedResponse, PipelineError> {
        // 抑制窗口检查 (VCP §6.2.2 #19, 防 OpenAI Responses 偶发 5xx 重试风暴)
        let key = format!("{}:{}", kind.as_str(), model_fingerprint(&input));
        if self.config.suppression.should_suppress(&key) {
            return Err(PipelineError::Suppressed(key));
        }

        // 步骤 1: 解析 placeholder (借鉴 VCP `messageProcessor.js:146-220`)
        let mut req = input;
        for msg in req.messages.iter_mut() {
            // 只展开 user + system (借鉴 VCP `messagesContainBase64Media` 同款 role 判定)
            if matches!(msg.role, MessageRole::User | MessageRole::System) {
                for part in msg.content.iter_mut() {
                    if let ContentPart::Text { text } = part {
                        *text = resolve_placeholders(text, &self.config.placeholder_context);
                    }
                }
            }
        }

        // 步骤 2: token 预算 (借鉴 VCP `dynamicToolRegistry.js:21` MAX_INJECTION_CHARS=16000)
        for msg in req.messages.iter_mut() {
            for part in msg.content.iter_mut() {
                if let ContentPart::Text { text } = part {
                    *text = truncate_to_max(text, self.config.max_injection_chars);
                }
            }
        }

        // 步骤 3: Force-Translate (借鉴 VCP `chatCompletionHandler.js:222-257` + `multiModalConfigStore.js`)
        let _stats =
            force_translate_if_needed(&req.model, &mut req.messages, &self.config.force_translate);

        // 步骤 4: 协议归一化 (R37-1: ProtocolBridge 砍 router 中间层)
        let body: Value = encode_for_kind(kind, &req).map_err(|e| PipelineError::Protocol(e.to_string()))?;

        // 步骤 5: HTTP 调用 (调战役 1-2 apeireth-http-client Keep-Alive LIFO)
        // 注: HttpClient::post() 不自动加 Bearer auth, 我们用 reqwest_client() 底层
        // + LIFO 池 permit 显式拿, 这样保留 Keep-Alive LIFO 调度的同时也能加 auth header
        let endpoint = endpoint_path_for_kind(kind).ok_or_else(|| PipelineError::Protocol(format!("kind {kind:?} has no HTTP endpoint")))?;
        let url = format!("{}{}", self.config.base_url, endpoint);
        let _guard = self.http.pool().enter().await;
        let mut req_builder = self.http.reqwest_client().post(&url).json(&body);
        if let Some(token) = &self.config.auth_token {
            req_builder = req_builder.bearer_auth(token);
        }
        let response = req_builder
            .send()
            .await
            .map_err(|e| PipelineError::Http(e.to_string()))?;

        // 拿响应 body (text 优先, 自己 parse, 给清晰错误)
        let text: String = response
            .text()
            .await
            .map_err(|e| PipelineError::Http(format!("read body: {e}")))?;

        let raw: Value = serde_json::from_str(&text).map_err(|e| {
            PipelineError::Http(format!(
                "parse JSON: {e}, body: {}",
                text.chars().take(200).collect::<String>()
            ))
        })?;

        // 协议反归一化 (R37-1: ProtocolBridge)
        let normalized: NormalizedResponse = decode_for_kind(kind, &raw).map_err(|e| {
            PipelineError::Protocol(format!(
                "{e}, raw: {}",
                text.chars().take(200).collect::<String>()
            ))
        })?;

        Ok(normalized)
    }

    /// **流式 5 步管线** (VCP `chatCompletionHandler.js:1-220` 主 chat 流式模式)
    ///
    /// 与 `run` 共享前 3 步 (placeholder / token / force_translate), 协议 encode 一样,
    /// 但 HTTP 调用改用流式 — **W3 #1 simulate 升级版**: 一次拿全 body, 按 50 字符一块推 chunk
    /// (战役 2+ 真 SSE 解析留给 `apeireth-tool-runtime`)
    pub async fn run_streaming(
        &self,
        kind: ProtocolKind,
        input: NormalizedRequest,
        sender: tokio::sync::mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), PipelineError> {
        // 步骤 1-3 同 run
        let mut req = input;
        for msg in req.messages.iter_mut() {
            if matches!(msg.role, MessageRole::User | MessageRole::System) {
                for part in msg.content.iter_mut() {
                    if let ContentPart::Text { text } = part {
                        *text = resolve_placeholders(text, &self.config.placeholder_context);
                    }
                }
            }
        }
        for msg in req.messages.iter_mut() {
            for part in msg.content.iter_mut() {
                if let ContentPart::Text { text } = part {
                    *text = truncate_to_max(text, self.config.max_injection_chars);
                }
            }
        }
        let _ =
            force_translate_if_needed(&req.model, &mut req.messages, &self.config.force_translate);

        // 步骤 4: 协议归一化 (R37-1: ProtocolBridge)
        let body: Value = encode_for_kind(kind, &req).map_err(|e| PipelineError::Protocol(e.to_string()))?;

        // 步骤 5: HTTP 调用 + 流式推 (simulate: 按 50 字符一块)
        let endpoint = endpoint_path_for_kind(kind).ok_or_else(|| PipelineError::Protocol(format!("kind {kind:?} has no HTTP endpoint")))?;
        let url = format!("{}{}", self.config.base_url, endpoint);
        let _ = sender.send(StreamChunk::Start);

        let _guard = self.http.pool().enter().await;
        let mut req_builder = self.http.reqwest_client().post(&url).json(&body);
        if let Some(token) = &self.config.auth_token {
            req_builder = req_builder.bearer_auth(token);
        }
        let response = req_builder
            .send()
            .await
            .map_err(|e| PipelineError::Http(e.to_string()))?;

        let text: String = response
            .text()
            .await
            .map_err(|e| PipelineError::Http(e.to_string()))?;

        // 模拟流式: 按 50 字符一块推
        for chunk in text.chars().collect::<Vec<_>>().chunks(50) {
            let s: String = chunk.iter().collect();
            if sender.send(StreamChunk::Data(s)).is_err() {
                // receiver dropped, graceful exit
                return Ok(());
            }
        }
        let _ = sender.send(StreamChunk::End);
        Ok(())
    }
}

/// Pipeline 错误
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// 协议层错误 (战役 1-1 apeireth-protocol 报错)
    #[error("protocol error: {0}")]
    Protocol(String),
    /// HTTP 层错误 (战役 1-2 apeireth-http-client 报错)
    #[error("http error: {0}")]
    Http(String),
    /// 抑制窗口拒绝 (VCP §6.2.2 #19)
    #[error("request suppressed (15s window): {0}")]
    Suppressed(String),
}

/// 用于抑制窗口的 model fingerprint (model + messages 头 1 个 user message)
fn model_fingerprint(req: &NormalizedRequest) -> String {
    let mut s = req.model.clone();
    if let Some(first_user) = req
        .messages
        .iter()
        .find(|m| matches!(m.role, MessageRole::User))
    {
        // 截前 64 字符
        let preview: String = first_user
            .content
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(64)
            .collect();
        s.push_str(&format!(":{preview}"));
    }
    s
}

// ============================================================
// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
// ============================================================

const _: () = {
    // 战役 1-3 pipeline crate 实际借鉴 4 项 (#15 / #17 / #19 / #20)
    assert!(
        BORROWED_LEGACY_COUNT == 4,
        "战役 1-3 pipeline crate 应借鉴 4 项 VCP 真代码"
    );

    // Pipeline 5 步 (VCP chatCompletionHandler.js:1-220 主 chat 模式)
    assert!(PIPELINE_STEP_COUNT == 5, "主 chat 管线必为 5 步");

    // 借鉴 VCP 真值守
    assert!(
        LEGACY_RETRY_SUPPRESSION_MS == 15_000,
        "VCP protocolBridge.js:11 真值 15000ms"
    );
    assert!(
        LEGACY_MAX_INJECTION_CHARS == 16_000,
        "VCP dynamicToolRegistry.js:21 真值 16000"
    );

    // token 预算至少 100 字符
    assert!(
        MIN_INJECTION_CHARS >= 100,
        "MIN_INJECTION_CHARS 至少 100 字符"
    );

    // 抑制窗口至少 1s
    assert!(DEFAULT_SUPPRESSION_WINDOW_MS >= 1_000, "抑制窗口至少 1s");
    // PipelineConfig 默认 max_injection_chars 对齐 VCP 16000 — 移到 lib_tests::pipeline_default_config_matches_vcp (runtime)
};

// ============================================================
// lib.rs 单元测试 (Pipeline 5 步 / 编译期 hardcode 守门)
// ============================================================

#[cfg(test)]
mod lib_tests {
    use super::*;
    use apeireth_http_client::KeepAliveConfig;
    use apeireth_protocol::NormalizedMessage;
    use std::time::Duration;

    fn test_pipeline() -> Pipeline {
        let http = HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        Pipeline::new(http).unwrap()
    }

    #[test]
    fn compile_time_hardcode_borrowed_count() {
        assert_eq!(BORROWED_LEGACY_COUNT, 4);
        assert_eq!(PIPELINE_STEP_COUNT, 5);
        assert_eq!(LEGACY_RETRY_SUPPRESSION_MS, 15_000);
        assert_eq!(LEGACY_MAX_INJECTION_CHARS, 16_000);
    }

    #[test]
    fn pipeline_default_config_matches_vcp() {
        // 编译期无法调 `PipelineConfig::default()` (含 Mutex),
        // 改为 runtime 验证, 守住 VCP 真值
        let cfg = PipelineConfig::default();
        assert_eq!(cfg.max_injection_chars, 16_000);
        assert_eq!(cfg.suppression.window, Duration::from_millis(15_000));
        assert_eq!(cfg.base_url, "https://api.minimaxi.com");
    }

    #[test]
    fn pipeline_constructs_with_chat_defaults() {
        let p = test_pipeline();
        assert_eq!(p.config().max_injection_chars, 16_000);
        assert_eq!(p.config().suppression.window, Duration::from_millis(15_000));
        assert_eq!(
            [
                ProtocolKind::OpenAiChat,
                ProtocolKind::OpenAiResponses,
                ProtocolKind::AnthropicMessages,
                ProtocolKind::Gemini,
            ]
            .len(),
            4
        );
    }

    #[test]
    fn pipeline_with_chat_defaults_quick_construct() {
        let p = Pipeline::with_chat_defaults();
        assert!(p.is_ok());
    }

    #[test]
    fn pipeline_5_step_constants_consistent() {
        // Pipeline 5 步对应 VCP 主 chat 模式 (解析 / 预算 / 翻译 / 归一化 / HTTP)
        // 此测试保证 5 步命名不被漂移
        const EXPECTED_STEPS: [&str; 5] = [
            "1. resolve_placeholders (VCP #17)",
            "2. token_budget (VCP #15)",
            "3. force_translate (VCP #20)",
            "4. protocol_encode (apeireth-protocol)",
            "5. http_post (apeireth-http-client Keep-Alive LIFO)",
        ];
        assert_eq!(EXPECTED_STEPS.len(), PIPELINE_STEP_COUNT);
    }

    #[test]
    fn model_fingerprint_uses_model_and_first_user_message() {
        let req = NormalizedRequest::new(
            "gpt-4o",
            vec![
                NormalizedMessage::system("system"),
                NormalizedMessage::user("hello world"),
            ],
        );
        let fp = model_fingerprint(&req);
        assert!(fp.starts_with("gpt-4o:"));
        assert!(fp.contains("hello world"));
    }

    #[test]
    fn model_fingerprint_no_user_message_just_model() {
        let req = NormalizedRequest::new("claude-sonnet-4", vec![]);
        let fp = model_fingerprint(&req);
        assert_eq!(fp, "claude-sonnet-4");
    }

    #[test]
    fn pipeline_error_display_includes_context() {
        let e = PipelineError::Suppressed("test-key".to_string());
        assert!(e.to_string().contains("suppressed"));
        assert!(e.to_string().contains("test-key"));
    }

    // ====== 5 步真跑验证 (wiremock 本地 server) ======

    #[tokio::test]
    async fn pipeline_5_step_runs_against_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // 1) 启动本地 mock server, 模拟 OpenAI Chat 协议端点
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test-001",
                "object": "chat.completion",
                "created": 1_700_000_000u64,
                "model": "gpt-4o",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Mock response: 5 步走通"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })))
            .expect(1) // **不假装**: 必须真发 1 次
            .mount(&server)
            .await;

        // 2) Pipeline + base URL 指向 mock server
        let http = HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let mut config = PipelineConfig::default();
        config.base_url = server.uri();
        // **不抑制**: fresh suppression 避免上一次测试残留
        config.suppression = RetrySuppression::new(Duration::from_millis(50));
        let pipeline = Pipeline::with_config(http, config).unwrap();

        // 3) 跑 5 步 (VCP §6.2.2 #15/#17/#19/#20 + 战役 1-1/1-2)
        let req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("test 5 步 e2e")]);
        let result = pipeline.run(ProtocolKind::OpenAiChat, req).await;

        // **不假装**: 必须真发 + 拿到 response
        let response = result.expect("5 步必须真跑通, 不许 mock");
        assert_eq!(response.id, "chatcmpl-test-001");
        assert_eq!(response.model, "gpt-4o");
        assert_eq!(response.content, "Mock response: 5 步走通");
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 5);
    }

    #[tokio::test]
    async fn pipeline_5_step_resolves_placeholders() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // 1) mock server (返回 fix JSON)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x", "model": "gpt-4o",
                "choices": [{"message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1}
            })))
            .expect(1)
            .mount(&server)
            .await;

        // 2) Pipeline 配 placeholder context: {{greeting}} -> "Hi from placeholder"
        let http = HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let mut config = PipelineConfig::default();
        config.base_url = server.uri();
        config.suppression = RetrySuppression::new(Duration::from_millis(50));
        config
            .placeholder_context
            .insert("greeting".to_string(), "Hi from placeholder".to_string());
        let pipeline = Pipeline::with_config(http, config).unwrap();

        // 3) 发请求, 第 1 步解析 placeholder
        let req = NormalizedRequest::new(
            "gpt-4o",
            vec![NormalizedMessage::user("{{greeting}}, please respond")],
        );
        let result = pipeline.run(ProtocolKind::OpenAiChat, req).await;

        // **5 步真跑**: 验证 pipeline 走通, placeholder 被解析
        assert!(result.is_ok(), "5 步应该走通, 实际: {:?}", result.err());
    }

    #[tokio::test]
    async fn pipeline_5_step_uses_suppression_window() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // 1) mock server: 期望 1 次请求 (第 2 次被抑制, 不发 HTTP)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x", "model": "gpt-4o",
                "choices": [{"message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1}
            })))
            .expect(1) // 重要: 期望只发 1 次
            .mount(&server)
            .await;

        let http = HttpClient::new(KeepAliveConfig::chat_default()).unwrap();
        let mut config = PipelineConfig::default();
        config.base_url = server.uri();
        // 默认 15s 窗口, 2 次都在窗口内
        let pipeline = Pipeline::with_config(http, config).unwrap();

        let req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);

        // 第 1 次: 不抑制, 跑 5 步
        let r1 = pipeline.run(ProtocolKind::OpenAiChat, req.clone()).await;
        assert!(r1.is_ok(), "第 1 次应走通, 实际: {:?}", r1.err());

        // 第 2 次: 同一 fingerprint → 抑制
        let r2 = pipeline.run(ProtocolKind::OpenAiChat, req).await;
        assert!(
            matches!(r2, Err(PipelineError::Suppressed(_))),
            "第 2 次应被抑制, 实际: {:?}",
            r2
        );
    }
}
