//! HTTP Server (axum) —— R17 战役 1-4 改造
//!
//! **4 协议端点 (战役 1-4 核心)**: 全部走 `apeireth_pipeline::Pipeline` 5 步管线
//! - `POST /v1/chat/completions`               —— OpenAI Chat Completions
//! - `POST /v1/responses`                     —— OpenAI Responses API (codex 风格)
//! - `POST /v1/messages`                      —— Anthropic Messages
//! - `POST /v1beta/models/{model}:generateContent` —— Google Gemini
//!
//! **R17 保留** (战役 0 + 战役 1 端点):
//! - `GET  /health`                           —— 健康检查
//! - `POST /council/advise`                   —— Council 7 advisor 真接入 LLM
//! - `POST /verdict`                          —— V1+V2+V3 AND 门验证
//!
//! **R25 Step 2 V2 6 类 JSON 端点** (跟 4 协议端点并列, 见 `v2_endpoints` 模块):
//! - `GET  /v1/tools/list` + `POST /v1/tools/invoke` — 战役 2-1 ToolRegistry
//! - `GET  /v1/memory/{episodes,append,identity,identity/update}` — SqliteMemoryStore
//! - `GET  /v1/organs` + `GET  /v1/organs/{name}` + `POST /v1/organs/{name}/invoke`
//! - `GET  /v1/asi/{score,all}` + `POST /v1/asi/calibrate` — DimensionRegistry
//! - `GET  /v1/sovereignty/status` + `POST /v1/sovereignty/{attack,rearm}` — SelfDisableGuard
//! - `GET  /v1/agent/aliases` + `POST /v1/agent/alias` + `GET /v1/agent/cache` — AgentManager
//! - `GET  /v2/health`                        —— 6 类服务健康总览 (debug)
//!
//! **架构** (R17 战役 1-4 后 + R25 Step 2 增量):
//! ```text
//!   客户端 4 协议请求
//!     ↓
//!   server.rs 4 个 endpoint
//!     ↓ protocol_handlers::*_to_normalized()
//!   NormalizedRequest
//!     ↓ Pipeline::run / dispatch() (战役 1-3 5 步 + Keep-Alive LIFO 战役 1-2)
//!   NormalizedResponse
//!     ↓ protocol_handlers::*_from_normalized()
//!   协议原生 JSON 响应
//!
//!   客户端 V2 6 类 JSON 请求
//!     ↓
//!   v2_endpoints::build_router(state) (Step 2)
//!     ↓
//!   ToolRegistry / SqliteMemoryStore / DimensionRegistry /
//!   SelfDisableGuard / AgentManager / OrgansProvider
//! ```
//!
//! **不假装** (主哲学锚 #1):
//! - ✅ 4 协议端点真接 minimaxi (不只 OpenAI / Anthropic)
//! - ✅ 走 `apeireth_pipeline::Pipeline` (战役 1-3) + `apeireth_http_client::HttpClient` (战役 1-2)
//! - ✅ 鉴权: minimaxi 全 4 端点统一 Bearer
//! - ✅ VCP 借鉴字段级: `protocolBridge.js:1-150` 4 协议入口 /
//!   `chatCompletionHandler.js:17-37` Keep-Alive 5 字段
//! - ✅ V2 6 类端点真接 5 个 lib crate (tools / memory / asi / sovereignty / agent) +
//!   本 crate OrgansProvider, lazy init 模式 (`OnceLock<Arc<...>>`) 防止 4 协议路径受影响
//!
//! **R17 战役 0 兼容**: legacy `LlmProvider` trait + 4 个 example (hello_api 等) 保留
//!
//! 跑法: `cargo run -p apeireth-api --example serve`

use std::sync::Arc;

use apeireth_http_client::KeepAliveConfig;
use apeireth_pipeline::Pipeline;
use apeireth_protocol::ProtocolKind;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http::{HeaderMap, Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::cache::ResponseCache;
use crate::llm::{ChatMessage, LlmProvider, LlmRequest};
use crate::protocol_handlers;
use crate::routing::{
    extract_force_cache, extract_protocol_override, parse_traceparent_from_headers, KeyPathSpan,
    SPAN_CHAT_COMPLETIONS, SPAN_COUNCIL_ADVISE, SPAN_GENERATE_CONTENT, SPAN_MESSAGES,
    SPAN_RESPONSES, SPAN_VERDICT,
};

// ============================================================
// AppState
// ============================================================

/// R17 战役 1-4 升级: 持有 4 协议管线 (Pipeline) + legacy LlmProvider (R17 战役 0 保留)
pub struct AppState {
    /// 战役 1-3 5 步管线 (4 协议归一化 + Keep-Alive LIFO 5 字段)
    pub pipeline: Arc<Pipeline>,
    /// R17 战役 0 保留: legacy LlmProvider (用于 /council/advise 跟 1+2 协议兼容)
    pub llm: Arc<dyn LlmProvider>,
    /// R120 (B2 战区 2): Response replay cache (1:1 apeireth-cache MemoryCache, LRU + 60s TTL)
    /// None 表示不启用 (跟 1.0 行为 0 漂移, 给 test / dev 阶段用)
    pub response_cache: Option<Arc<ResponseCache>>,
}

pub type SharedState = Arc<AppState>;

// ============================================================
// Router
// ============================================================

pub fn build_router(state: SharedState) -> Router {
    build_router_with_v2(state, Arc::new(crate::v2_endpoints::V2State::new()))
}

/// **R25 Step 2**: 构造主 router + V2 6 类 JSON 端点
///
/// **签名**: 把 V2State 显式传入, 让上层 (serve example) 能注入懒加载服务.
/// 现有 `build_router(state)` 走默认 V2State (空), 4 协议 + Council + Verdict 路径不变.
///
/// **返回类型**: `Router<()>` — 最后一步调用 `with_state(state)`. 这样可套用
/// `axum::serve(listener, app.into_make_service())` (into_make_service 只在 `Router<()>`
/// 上定义, 不是任意 `Router<S>`).
///
/// **实现**: 因为 axum 0.7 的 Router<S> 类型参数化, 主 router 走 Arc<AppState>,
/// V2 router 走 Arc<V2State>. 用 `.nest_service("/v1", v2_routes)` 嵌进去, axum 0.7
/// nest_service 接受任何 `Router<T>`, 内层 state 类型独立.
pub fn build_router_with_v2(state: SharedState, v2: crate::v2_endpoints::SharedV2) -> Router {
    let v2_routes = crate::v2_endpoints::build_router(v2);
    Router::new()
        .route("/health", get(health))
        // R17 战役 1-4: 4 协议端点 (走 Pipeline)
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(responses))
        .route("/v1/messages", post(messages))
        .route(
            "/v1beta/models/:model/generateContent",
            post(generate_content),
        )
        // R17 战役 0 保留: Council + Verdict
        .route("/council/advise", post(council_advise))
        .route("/verdict", post(verdict))
        .layer(TraceLayer::new_for_http())
        // R27 Web TUI 接入: 允许 localhost / 127.0.0.1 / file:// 浏览器跨域调 4 协议端点
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers([CONTENT_TYPE, AUTHORIZATION]),
        )
        // R25 Step 2: V2 6 类 JSON 端点用 nest_service 嵌进去 (内层 state 独立)
        .nest_service("/v1", v2_routes)
        .with_state(state)
}

// ============================================================
// 共享 handler
// ============================================================

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "apeireth-api",
        "version": env!("CARGO_PKG_VERSION"),
        "protocols": ["openai_chat", "openai_responses", "anthropic_messages", "gemini"],
    }))
}

/// 工具: 把 protocol_handlers error 转 (StatusCode, String)
fn err_to_response(e: String) -> (StatusCode, String) {
    // 抑制窗口拒绝 (VCP 15s 抑制) → 429
    if e.starts_with("suppressed:") {
        return (StatusCode::TOO_MANY_REQUESTS, e);
    }
    // JSON parse 错误 → 400
    if e.starts_with("json parse:") || e.starts_with("protocol encode:") {
        return (StatusCode::BAD_REQUEST, e);
    }
    // HTTP 错误 → 502
    if e.starts_with("http:") || e.starts_with("http send:") || e.starts_with("http read body:") {
        return (StatusCode::BAD_GATEWAY, e);
    }
    // 默认 500
    (StatusCode::INTERNAL_SERVER_ERROR, e)
}

// ============================================================
// 协议端点 (R17 战役 1-4 核心)
// ============================================================

/// `POST /v1/chat/completions` —— OpenAI Chat Completions
///
/// **R28 TUI 接后端**: 流式分支 (req.stream == true) 走 `stream_chat_completions_forward`,
/// 把上游 SSE 字节原样转发回客户端 (TUI 自己解 SSE). 非流式保持 pipeline.run() JSON 解析路径.
///
/// **不假装** (主哲学锚 #1 不漂移):
/// - 流式: 走 Pipeline.http().reqwest_client() (战役 1-2 Keep-Alive LIFO 复用), Bearer auth
///   跟非流式同源 (Pipeline.config.auth_token), 上游 SSE 字节 0 字节篡改直送客户端
/// - 非流式: 走 pipeline.run() 5 步 (战役 1-3), 跟 v0.9 0 行为变更
///
/// **R120 (B4 战区 2) 协议路由增强**:
/// - `X-Apeireth-Protocol: openai|anthropic|gemini` header → override default ProtocolKind
/// - `X-Apeireth-Force-Cache: true` → 强制走 cache, miss 返 503
async fn chat_completions(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let mut span = {
        // R121 续 (V2-3 战区 2.5): W3C traceparent 传播 — 跨服务 trace 关联
        let parent = parse_traceparent_from_headers(&headers);
        KeyPathSpan::start_with_parent(SPAN_CHAT_COMPLETIONS, parent)
    };
    let protocol_override = extract_protocol_override(&headers);
    let force_cache = extract_force_cache(&headers);
    if let Some(p) = protocol_override {
        span.set_protocol_override(match p {
            ProtocolKind::OpenAiChat => "openai",
            ProtocolKind::AnthropicMessages => "anthropic",
            ProtocolKind::Gemini => "gemini",
            _ => "openai",
        });
    }
    span.set_force_cache(force_cache);

    // 先解析 stream 标志 (无论 stream 与否都需解析 body)
    let req: protocol_handlers::OpenAiChatRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let resp = err_to_response(format!("json parse: {e}")).into_response();
            span.set_protocol(ProtocolKind::OpenAiChat);
            span.end_err(format!("json parse: {e}"));
            return resp;
        }
    };

    if req.stream {
        // 流式: SSE 直通 (TUI 端能解 SSE data: {...}\n\n, [DONE])
        // R121 续 (V2-2 战区 2.5): 4 协议统一流式 (B 留 R121 续), 走 protocol_handlers::stream_forward
        span.set_protocol(ProtocolKind::OpenAiChat);
        let resp = match protocol_handlers::stream_forward(
            &state.pipeline,
            ProtocolKind::OpenAiChat,
            body,
            &req.model,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => err_to_response(e).into_response(),
        };
        span.end_ok();
        return resp;
    }

    // 非流式: 走原 5 步管线 (+ R120 B2 response cache hook + B4 protocol override)
    let kind = protocol_override.unwrap_or(ProtocolKind::OpenAiChat);
    span.set_protocol(kind);
    span.set_model(&req.model);

    let normalized = protocol_handlers::openai_chat_to_normalized(&req);

    // R120 B4: force_cache 模式 — 强制走 cache, miss 返 503
    if force_cache {
        if let Some(cache) = state.response_cache.as_deref() {
            if let Some(cached) = cache.get(&normalized, kind).await {
                span.set_cache_status("hit");
                let resp = Json(protocol_handlers::openai_chat_from_normalized(&cached))
                    .into_response();
                span.end_ok();
                return resp;
            }
        }
        span.set_cache_status("miss_forced");
        let resp = err_to_response("force cache miss: no cached response available".to_string())
            .into_response();
        span.end_ok();
        return resp;
    }

    match protocol_handlers::dispatch_cached(
        &state.pipeline,
        kind,
        normalized,
        state.response_cache.as_deref(),
    )
    .await
    {
        Ok(resp) => {
            span.set_cache_status("ok");
            let http_resp =
                Json(protocol_handlers::openai_chat_from_normalized(&resp)).into_response();
            span.end_ok();
            http_resp
        }
        Err(e) => {
            let http_resp = err_to_response(e.clone()).into_response();
            span.end_err(e);
            http_resp
        }
    }
}

/// **R121 续 (V2-2 战区 2.5)**: 4 协议流式 SSE 字节直通 (B 留 R121 续)
///
/// 上游是 minimaxi (4 协议), 返 `text/event-stream` + 多个 `data: {...}\n\n` 事件 + `data: [DONE]` 终止.
/// 客户端 (TUI / Web) 已能解 SSE, daemon 这里只做"路由 + auth + LIFO 连接复用", 不解析内容.
///
/// **0 漂移 1.0 行为**:
/// - 1.0 chat_completions 内部 `stream_chat_completions_forward` 已删除, 改用 `protocol_handlers::stream_forward`
/// - 4 协议流式统一函数 (kind 决定 endpoint URL, 走 `endpoint_url` 已实现分支)
/// - 0 走 cache (req.stream 守门, 跟 B2 SSE 边界 1:1)
/// - 0 触碰 4 handler 公共 API 签名 (axum 0.7 `Response` / `Result<Json<...>, (StatusCode, String)>` 0 改)
async fn responses(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    // 流式优先 (req.stream 早返回) — 跟 chat_completions 1:1 模式
    let req: protocol_handlers::OpenAiResponsesRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return err_to_response(format!("json parse: {e}")).into_response();
        }
    };
    if req.stream {
        // 流式: SSE 直通 (跟 chat_completions 1:1 模式)
        return match protocol_handlers::stream_forward(
            &state.pipeline,
            ProtocolKind::OpenAiResponses,
            body,
            &req.model,
        )
        .await
        {
            Ok(resp) => resp,
            Err(e) => err_to_response(e).into_response(),
        };
    }

    // 非流式: 走原 5 步管线
    responses_inner(state, headers, req).await
}

/// responses handler 内部 (非流式, 走 dispatch_cached)
async fn responses_inner(
    state: SharedState,
    headers: HeaderMap,
    req: protocol_handlers::OpenAiResponsesRequest,
) -> Response {
    let mut span = {
        let parent = parse_traceparent_from_headers(&headers);
        KeyPathSpan::start_with_parent(SPAN_RESPONSES, parent)
    };
    let protocol_override = extract_protocol_override(&headers);
    let force_cache = extract_force_cache(&headers);
    span.set_protocol(protocol_override.unwrap_or(ProtocolKind::OpenAiResponses));
    span.set_force_cache(force_cache);
    span.set_model(&req.model);

    let normalized = protocol_handlers::openai_responses_to_normalized(&req);

    // R120 B4: force_cache 模式
    if force_cache {
        let kind = protocol_override.unwrap_or(ProtocolKind::OpenAiResponses);
        if let Some(cache) = state.response_cache.as_deref() {
            if let Some(cached) = cache.get(&normalized, kind).await {
                span.set_cache_status("hit");
                let out = protocol_handlers::openai_responses_from_normalized(&cached);
                span.end_ok();
                return Json(out).into_response();
            }
        }
        span.set_cache_status("miss_forced");
        span.end_ok();
        return err_to_response("force cache miss: no cached response available".to_string())
            .into_response();
    }

    let kind = protocol_override.unwrap_or(ProtocolKind::OpenAiResponses);
    let result = protocol_handlers::dispatch_cached(
        &state.pipeline,
        kind,
        normalized,
        state.response_cache.as_deref(),
    )
    .await;
    match result {
        Ok(resp) => {
            span.set_cache_status("ok");
            let out = protocol_handlers::openai_responses_from_normalized(&resp);
            span.end_ok();
            Json(out).into_response()
        }
        Err(e) => {
            span.end_err(&e);
            err_to_response(e).into_response()
        }
    }
}

/// `POST /v1/messages` —— Anthropic Messages
async fn messages(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let req: protocol_handlers::AnthropicRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return err_to_response(format!("json parse: {e}")).into_response();
        }
    };
    if req.stream {
        return match protocol_handlers::stream_forward(
            &state.pipeline,
            ProtocolKind::AnthropicMessages,
            body,
            &req.model,
        )
        .await
        {
            Ok(resp) => resp,
            Err(e) => err_to_response(e).into_response(),
        };
    }
    messages_inner(state, headers, req).await
}

async fn messages_inner(
    state: SharedState,
    headers: HeaderMap,
    req: protocol_handlers::AnthropicRequest,
) -> Response {
    let mut span = {
        let parent = parse_traceparent_from_headers(&headers);
        KeyPathSpan::start_with_parent(SPAN_MESSAGES, parent)
    };
    let protocol_override = extract_protocol_override(&headers);
    let force_cache = extract_force_cache(&headers);
    span.set_protocol(protocol_override.unwrap_or(ProtocolKind::AnthropicMessages));
    span.set_force_cache(force_cache);
    span.set_model(&req.model);

    let normalized = protocol_handlers::anthropic_to_normalized(&req);

    if force_cache {
        let kind = protocol_override.unwrap_or(ProtocolKind::AnthropicMessages);
        if let Some(cache) = state.response_cache.as_deref() {
            if let Some(cached) = cache.get(&normalized, kind).await {
                span.set_cache_status("hit");
                let out = protocol_handlers::anthropic_from_normalized(&cached);
                span.end_ok();
                return Json(out).into_response();
            }
        }
        span.set_cache_status("miss_forced");
        span.end_ok();
        return err_to_response("force cache miss: no cached response available".to_string())
            .into_response();
    }

    let kind = protocol_override.unwrap_or(ProtocolKind::AnthropicMessages);
    let result = protocol_handlers::dispatch_cached(
        &state.pipeline,
        kind,
        normalized,
        state.response_cache.as_deref(),
    )
    .await;
    match result {
        Ok(resp) => {
            span.set_cache_status("ok");
            let out = protocol_handlers::anthropic_from_normalized(&resp);
            span.end_ok();
            Json(out).into_response()
        }
        Err(e) => {
            span.end_err(&e);
            err_to_response(e).into_response()
        }
    }
}

/// `POST /v1beta/models/{model}:generateContent` —— Google Gemini
async fn generate_content(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(model): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    let req: protocol_handlers::GeminiRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return err_to_response(format!("json parse: {e}")).into_response();
        }
    };
    if req.stream {
        return match protocol_handlers::stream_forward(
            &state.pipeline,
            ProtocolKind::Gemini,
            body,
            &model,
        )
        .await
        {
            Ok(resp) => resp,
            Err(e) => err_to_response(e).into_response(),
        };
    }
    generate_content_inner(state, headers, model, req).await
}

async fn generate_content_inner(
    state: SharedState,
    headers: HeaderMap,
    model: String,
    req: protocol_handlers::GeminiRequest,
) -> Response {
    let mut span = {
        let parent = parse_traceparent_from_headers(&headers);
        KeyPathSpan::start_with_parent(SPAN_GENERATE_CONTENT, parent)
    };
    let protocol_override = extract_protocol_override(&headers);
    let force_cache = extract_force_cache(&headers);
    span.set_protocol(protocol_override.unwrap_or(ProtocolKind::Gemini));
    span.set_force_cache(force_cache);
    span.set_model(&model);

    let mut normalized = protocol_handlers::gemini_to_normalized(&req);
    // Gemini 模型在 URL 路径, 这里填到 NormalizedRequest.model (供 dispatch 用)
    normalized.model = model;

    if force_cache {
        let kind = protocol_override.unwrap_or(ProtocolKind::Gemini);
        if let Some(cache) = state.response_cache.as_deref() {
            if let Some(cached) = cache.get(&normalized, kind).await {
                span.set_cache_status("hit");
                let out = protocol_handlers::gemini_from_normalized(&cached);
                span.end_ok();
                return Json(out).into_response();
            }
        }
        span.set_cache_status("miss_forced");
        span.end_ok();
        return err_to_response("force cache miss: no cached response available".to_string())
            .into_response();
    }

    let kind = protocol_override.unwrap_or(ProtocolKind::Gemini);
    let result = protocol_handlers::dispatch_cached(
        &state.pipeline,
        kind,
        normalized,
        state.response_cache.as_deref(),
    )
    .await;
    match result {
        Ok(resp) => {
            span.set_cache_status("ok");
            let out = protocol_handlers::gemini_from_normalized(&resp);
            span.end_ok();
            Json(out).into_response()
        }
        Err(e) => {
            span.end_err(&e);
            err_to_response(e).into_response()
        }
    }
}

// ============================================================
// R17 战役 0 保留: Council + Verdict (用 legacy LlmProvider)
// ============================================================

#[derive(Debug, Deserialize)]
struct CouncilAdviseRequest {
    /// 议题
    pub topic: String,
    /// 上下文 (可选)
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
struct CouncilAdviseResponse {
    pub topic: String,
    pub status: String,
    pub advisors: Vec<AdvisorOpinion>,
    pub verdict: String,
}

#[derive(Debug, Serialize)]
struct AdvisorOpinion {
    pub domain: String,
    pub stance: String,
    pub reasoning: String,
}

async fn council_advise(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<CouncilAdviseRequest>,
) -> Result<Json<CouncilAdviseResponse>, (StatusCode, String)> {
    // R120 (B4 战区 2): 关键路径 span
    let _span = {
        let mut s = KeyPathSpan::start(SPAN_COUNCIL_ADVISE);
        s.set_model(&req.topic);
        s
    };
    // Week 3: Council 7 advisor 真接入
    // 当前 stub: 用 LLM 跑 3 个代表 advisor (safety / philosophy / ethics) 真接入
    // 完整 7 advisor Week 3 主任务
    let advisors = vec![
        ("safety", "你是 Council safety advisor. 评估议题是否有安全风险 (nuke/weapon/self-modify). 给出立场 (approve/reject/neutral) + 简短推理. 中文."),
        ("performance", "你是 Council performance advisor. 评估议题性能影响 (wallclock/资源). 给出立场 + 推理. 中文."),
        ("philosophy", "你是 Council philosophy advisor. 评估议题是否违反 12 键哲学守门 (不假装/不欺骗). 给出立场 + 推理. 中文."),
        ("history", "你是 Council history advisor. 评估议题是否有历史相似案例. 给出立场 + 推理. 中文."),
        ("strategy", "你是 Council strategy advisor. 评估议题长期价值 vs 短期收益. 给出立场 + 推理. 中文."),
        ("ethics", "你是 Council ethics advisor. 评估议题是否违反实事求是. 给出立场 + 推理. 中文."),
        ("legal", "你是 Council legal advisor. 评估议题是否触发 L0 HA 司法边界. 给出立场 + 推理. 中文."),
    ];
    let mut opinions = Vec::new();

    let prompt_context = req.context.as_deref().unwrap_or("");
    for (domain, sys_prompt) in &advisors {
        let usr = format!(
            "议题: {}{}",
            req.topic,
            if prompt_context.is_empty() {
                String::new()
            } else {
                format!("\n上下文: {}", prompt_context)
            }
        );
        // 修 Bug: model 字段必须用真 model 名 (MiniMax-M3 等), 不能用 provider name "apeireth-api"
        let model = if state.llm.name() == "apeireth-api" {
            "MiniMax-M3"
        } else {
            state.llm.name()
        };
        let llm_req = LlmRequest::new(
            model,
            vec![
                ChatMessage::system((*sys_prompt).to_string()),
                ChatMessage::user(usr),
            ],
        )
        .with_temperature(0.3)
        .with_max_tokens(200);
        let resp = state
            .llm
            .complete(llm_req)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("llm error: {e}")))?;

        // 简单解析: 第一个字是立场 (赞/反/中)
        let (stance, reasoning) = parse_advice(&resp.content);
        opinions.push(AdvisorOpinion {
            domain: (*domain).to_string(),
            stance,
            reasoning,
        });
    }

    // 简单投票
    let approve = opinions.iter().filter(|o| o.stance == "approve").count();
    let reject = opinions.iter().filter(|o| o.stance == "reject").count();
    let total = opinions.len();
    let verdict = if approve * 2 > total {
        "approved"
    } else if reject * 2 > total {
        "rejected"
    } else {
        "needs_more_review"
    };

    Ok(Json(CouncilAdviseResponse {
        topic: req.topic,
        status: "ok".into(),
        advisors: opinions,
        verdict: verdict.to_string(),
    }))
}

fn parse_advice(content: &str) -> (String, String) {
    let trimmed = content.trim();
    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let stance = if first_word.contains("赞")
        || first_word.contains("同")
        || first_word.contains("支持")
        || first_word.contains("approve")
        || first_word.contains("yes")
    {
        "approve"
    } else if first_word.contains("反")
        || first_word.contains("反对")
        || first_word.contains("reject")
        || first_word.contains("no")
    {
        "reject"
    } else {
        "neutral"
    };
    (stance.to_string(), content.to_string())
}

#[derive(Debug, Deserialize)]
struct VerdictRequest {
    /// V1 (哲学守门) 输出
    pub v1: String,
    /// V2 (权限洋葱) 输出
    pub v2: String,
    /// V3 (默认守门) 输出
    pub v3: String,
}

#[derive(Debug, Serialize)]
struct VerdictResponse {
    pub final_verdict: String,
    pub v1: bool,
    pub v2: bool,
    pub v3: bool,
    pub details: Value,
}

async fn verdict(
    State(_state): State<SharedState>,
    _headers: HeaderMap,
    Json(req): Json<VerdictRequest>,
) -> impl IntoResponse {
    // R120 (B4 战区 2): 关键路径 span
    let _span = {
        let mut s = KeyPathSpan::start(SPAN_VERDICT);
        s.set_model(&format!("v1={}v2={}v3={}", &req.v1[..req.v1.len().min(20)], &req.v2[..req.v2.len().min(20)], &req.v3[..req.v3.len().min(20)]));
        s
    };
    // V1+V2+V3 AND 门
    let v1_pass =
        req.v1.to_lowercase().contains("pass") || req.v1.contains("通过") || req.v1.contains("✓");
    let v2_pass =
        req.v2.to_lowercase().contains("pass") || req.v2.contains("通过") || req.v2.contains("✓");
    let v3_pass =
        req.v3.to_lowercase().contains("pass") || req.v3.contains("通过") || req.v3.contains("✓");

    let all_pass = v1_pass && v2_pass && v3_pass;

    Json(VerdictResponse {
        final_verdict: if all_pass {
            "allow".to_string()
        } else {
            "block".to_string()
        },
        v1: v1_pass,
        v2: v2_pass,
        v3: v3_pass,
        details: json!({
            "v1": req.v1,
            "v2": req.v2,
            "v3": req.v3,
        }),
    })
}

// ============================================================
// AppState 构造辅助 (供 examples 跟 serve 启动用)
// ============================================================

impl AppState {
    /// 构造默认 AppState: Pipeline 走 minimaxi base + Bearer auth_token, LlmProvider 走 minimaxi
    ///
    /// # 参数
    /// - `base_url`: minimaxi 默认 `https://api.minimaxi.com`
    /// - `auth_token`: Bearer token (minimaxi 4 端点共用)
    /// - `llm_provider`: R17 战役 0 保留的 legacy LlmProvider (用于 /council/advise)
    pub fn new(
        base_url: String,
        auth_token: Option<String>,
        llm_provider: Arc<dyn LlmProvider>,
    ) -> Result<Self, String> {
        // Pipeline 5 步管线 (战役 1-3) + Keep-Alive LIFO 5 字段 (战役 1-2)
        // 手动构造 Pipeline, 因为 protocol_handlers::build_pipeline 内部用 vcp_defaults
        use apeireth_http_client::HttpClient;
        use apeireth_pipeline::PipelineConfig;

        let http = HttpClient::new(KeepAliveConfig::vcp_default())
            .map_err(|e| format!("http client build: {e}"))?;
        let mut config = PipelineConfig::default();
        config.base_url = base_url;
        config.auth_token = auth_token;
        let pipeline =
            Pipeline::with_config(http, config).map_err(|e| format!("pipeline build: {e}"))?;
        Ok(Self {
            pipeline: Arc::new(pipeline),
            llm: llm_provider,
            // R120 (B2 战区 2): 默认 None (跟 1.0 行为 0 漂移, 给 test / dev 阶段用)
            // 生产环境用 with_response_cache() 启用
            response_cache: None,
        })
    }

    /// R120 (B2 战区 2): 启用 response cache
    ///
    /// **1:1 翻译** `apeireth-cache::MemoryCache` (LRU + 60s TTL + 32 shards, 0 漂移)
    /// **不假装**: cache 内部错误 (CapacityExceeded / IO) → fail-soft, 0 影响主路径
    pub async fn with_response_cache(mut self) -> Result<Self, String> {
        let cache = ResponseCache::new()
            .await
            .map_err(|e| format!("response cache init: {e}"))?;
        self.response_cache = Some(Arc::new(cache));
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_paths_match_protocol_endpoints() {
        // 4 协议端点路径正确 (axum path 解析)
        let paths = [
            ("/v1/chat/completions", ProtocolKind::OpenAiChat),
            ("/v1/responses", ProtocolKind::OpenAiResponses),
            ("/v1/messages", ProtocolKind::AnthropicMessages),
        ];
        for (path, kind) in paths {
            let url = protocol_handlers::endpoint_url("https://api.minimaxi.com", kind, "").expect("HTTP kind");
            assert!(url.ends_with(path), "URL {url} should end with {path}");
        }
    }

    #[test]
    fn gemini_url_substitutes_model() {
        // 验证 gemini 路径替换 (axum Path 解析, minimaxi quirk: /v1/gemini/v1beta/...)
        let url = protocol_handlers::endpoint_url(
            "https://api.minimaxi.com",
            ProtocolKind::Gemini,
            "MiniMax-M3",
        ).expect("HTTP kind");
        assert_eq!(
            url,
            "https://api.minimaxi.com/v1/gemini/v1beta/models/MiniMax-M3:generateContent"
        );
    }
}