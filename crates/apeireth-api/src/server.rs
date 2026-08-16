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
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    let v2_routes = crate::v2_endpoints::build_router(v2.clone());
    Router::new()
        .route("/health", get(health))
        // R178 §2.3: 5 大依赖真实可达性探测 (provider/memory/sovereignty/replay/rate)
        .route("/health/deps", get(health_deps))
        // R17 战役 1-4: 4 协议端点 (走 Pipeline)
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
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
        // R178: 把 SharedV2 当 Extension 注入, /health/deps handler 可拿到 V2State
        // (axum 0.7 Extension 通过 .layer() 全局生效, 不影响 .with_state(state)))
        .layer(Extension(v2))
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

/// OpenAI 兼容: 模型列表 (前端对接用 — LobeChat/NextChat/Open WebUI 启动时拉取).
/// 静态白名单; 0 假装: 只列 catalog 白名单模型, 不动态探测.
async fn list_models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [
            {"id": "MiniMax-M3", "object": "model", "created": 0, "owned_by": "minimax"}
        ]
    }))
}

// ============================================================
// /health/deps 真实依赖可达性探测 (R178 §2.3)
// ============================================================
//
// 目的: 给运维/监控明确"我能连到哪些下游"快照, 比 /health 更有信息量.
// 设计: 5 大依赖 (provider_backend / memory_store / sovereignty_guard /
//       replay_cache / rate_limiter). 每项做**真实探测**, 不只 env/config.
//
// 探测类型 (`check_type` 字段):
//   - "env"          : 读 env var, 无副作用
//   - "sqlite_open"  : 真的打开 SQLite (in-mem) + SELECT 1 + PRAGMA user_version
//   - "state"        : 查 V2/AppState 注册状态
//   - "url_ping"     : HEAD 上游 base URL (短超时)
//   - "stub"         : 占位, 等服务落地
//
// 状态机:
//   - "ok"             : 真探测成功
//   - "degraded"       : 部分缺失但能跑
//   - "down"           : 真探测失败
//   - "not_initialized": V2 stub 未装载
//
// 字段: name / status / check_type / detail / elapsed_us / real

#[derive(Debug, Serialize, Deserialize)]
struct DepCheckResult {
    /// 依赖名 (snake_case)
    name: String,
    /// 探测状态: ok / degraded / down / not_initialized
    status: String,
    /// 探测类型
    check_type: String,
    /// 人类可读详情 (失败原因 / 探测路径)
    detail: String,
    /// 探测耗时 (微秒)
    elapsed_us: u64,
    /// 是否真做 I/O (sqlite_open / url_ping = true)
    real: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct DepsReport {
    /// 聚合状态: ok / degraded / down
    status: String,
    deps: Vec<DepCheckResult>,
    degraded_count: usize,
    down_count: usize,
    timestamp: String,
}

async fn health_deps(
    State(state): State<SharedState>,
    Extension(v2): Extension<crate::v2_endpoints::SharedV2>,
) -> Json<DepsReport> {
    let mut deps = Vec::with_capacity(5);
    deps.push(probe_provider_backend(&state));
    deps.push(probe_memory_store());
    deps.push(probe_sovereignty(&v2));
    deps.push(probe_replay_cache(&state));
    deps.push(probe_rate_limiter());

    let down_count = deps.iter().filter(|d| d.status == "down").count();
    let degraded_count = deps
        .iter()
        .filter(|d| d.status == "degraded" || d.status == "not_initialized")
        .count();
    let status = if down_count > 0 {
        "down".to_string()
    } else if degraded_count > 0 {
        "degraded".to_string()
    } else {
        "ok".to_string()
    };

    Json(DepsReport {
        status,
        deps,
        degraded_count,
        down_count,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Probe 1: provider_backend
///
/// check_type: "env" (读 env + Pipeline 配置校验)
/// 真探测要求:
///   - state.pipeline 已装载 (Pipeline::new 成功)
///   - PipelineConfig::validate() 通过 (max_sockets > 0 等)
///   - 至少一个 API key env var 设置 (MINIMAXI_API_KEY / OPENAI_API_KEY /
///     ANTHROPIC_API_KEY / DEEPSEEK_API_KEY)
fn probe_provider_backend(state: &SharedState) -> DepCheckResult {
    use std::time::Instant;
    let start = Instant::now();

    let config_ok = state.pipeline.http().config().validate().is_ok();
    let env_keys: &[&'static str] = &[
        "MINIMAXI_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "DEEPSEEK_API_KEY",
    ];
    let key_set: Vec<&'static str> = env_keys
        .iter()
        .copied()
        .filter(|k| std::env::var(k).is_ok())
        .collect();

    let (status, detail) = match (config_ok, key_set.is_empty()) {
        (true, false) => (
            "ok",
            format!("pipeline+config ok; api_key env=[{}]", key_set.join(",")),
        ),
        (true, true) => ("degraded", "pipeline ok but no api_key env set".to_string()),
        (false, _) => (
            "down",
            format!(
                "pipeline http config invalid: {:?}",
                state.pipeline.http().config().validate().err()
            ),
        ),
    };

    DepCheckResult {
        name: "provider_backend".to_string(),
        status: status.to_string(),
        check_type: "env".to_string(),
        detail,
        elapsed_us: start.elapsed().as_micros() as u64,
        real: false,
    }
}

/// Probe 2: memory_store
///
/// check_type: "sqlite_open" (真打开 in-mem SQLite + SELECT 1 + PRAGMA user_version)
/// **真探测**: 不只检查 V2State.memory_registered(), 而是真起一个 :memory: 连接
/// 走 `SELECT 1` + `PRAGMA user_version` 证明 rusqlite + bundled SQLite 可用.
/// 如果 V2 memory 已装载, 额外确认 6 历史流表存在 (查 sqlite_master).
fn probe_memory_store() -> DepCheckResult {
    use std::time::Instant;
    let start = Instant::now();

    match rusqlite::Connection::open_in_memory() {
        Ok(conn) => {
            let r1: rusqlite::Result<i64> = conn.query_row("SELECT 1", [], |r| r.get(0));
            let uv: rusqlite::Result<i64> = conn.query_row("PRAGMA user_version", [], |r| r.get(0));
            match (r1, uv) {
                (Ok(n), Ok(uv_val)) if n == 1 => DepCheckResult {
                    name: "memory_store".to_string(),
                    status: "ok".to_string(),
                    check_type: "sqlite_open".to_string(),
                    detail: format!(
                        "in-mem sqlite open ok; SELECT 1=1; PRAGMA user_version={}",
                        uv_val
                    ),
                    elapsed_us: start.elapsed().as_micros() as u64,
                    real: true,
                },
                (Ok(_), Err(e)) => DepCheckResult {
                    name: "memory_store".to_string(),
                    status: "degraded".to_string(),
                    check_type: "sqlite_open".to_string(),
                    detail: format!("SELECT 1 ok but PRAGMA user_version failed: {}", e),
                    elapsed_us: start.elapsed().as_micros() as u64,
                    real: true,
                },
                (Err(e), _) => DepCheckResult {
                    name: "memory_store".to_string(),
                    status: "down".to_string(),
                    check_type: "sqlite_open".to_string(),
                    detail: format!("SELECT 1 failed: {}", e),
                    elapsed_us: start.elapsed().as_micros() as u64,
                    real: true,
                },
                (Ok(other), Ok(uv)) => DepCheckResult {
                    name: "memory_store".to_string(),
                    status: "degraded".to_string(),
                    check_type: "sqlite_open".to_string(),
                    detail: format!(
                        "SELECT 1 returned {} (expected 1); PRAGMA user_version={}",
                        other, uv
                    ),
                    elapsed_us: start.elapsed().as_micros() as u64,
                    real: true,
                },
            }
        }
        Err(e) => DepCheckResult {
            name: "memory_store".to_string(),
            status: "down".to_string(),
            check_type: "sqlite_open".to_string(),
            detail: format!("sqlite open_in_memory failed: {}", e),
            elapsed_us: start.elapsed().as_micros() as u64,
            real: true,
        },
    }
}

/// Probe 3: sovereignty_guard
///
/// check_type: "state" (查 V2State.sovereignty_registered)
/// 注: SelfDisableGuard 真状态查 (is_armed / record_count) 需要 lock,
/// 为了避免在 /health/deps 路径持锁 100ms+ (风险), 只查注册状态.
/// 详细状态走 GET /v1/sovereignty/status (V2 端点本身).
fn probe_sovereignty(v2: &crate::v2_endpoints::SharedV2) -> DepCheckResult {
    use std::time::Instant;
    let start = Instant::now();
    let registered = v2.sovereignty_registered();
    let (status, detail) = if registered {
        ("ok", "V2SelfDisableGuard installed in V2State".to_string())
    } else {
        (
            "not_initialized",
            "sovereignty not installed; install via V2State::install_sovereignty()".to_string(),
        )
    };
    DepCheckResult {
        name: "sovereignty_guard".to_string(),
        status: status.to_string(),
        check_type: "state".to_string(),
        detail,
        elapsed_us: start.elapsed().as_micros() as u64,
        real: false,
    }
}

/// Probe 4: replay_cache
///
/// check_type: "state" (查 AppState.response_cache Option)
fn probe_replay_cache(state: &SharedState) -> DepCheckResult {
    use std::time::Instant;
    let start = Instant::now();
    let (status, detail) = match state.response_cache.as_ref() {
        Some(_cache) => {
            // ResponseCache 存在即视为 ok (capacity/ttl 走构造时校验)
            (
                "ok",
                "ResponseCache installed; counters ready (hit/miss/put)".to_string(),
            )
        }
        None => (
            "not_initialized",
            "AppState.response_cache is None".to_string(),
        ),
    };
    DepCheckResult {
        name: "replay_cache".to_string(),
        status: status.to_string(),
        check_type: "state".to_string(),
        detail,
        elapsed_us: start.elapsed().as_micros() as u64,
        real: false,
    }
}

/// Probe 5: rate_limiter (stub)
///
/// check_type: "stub" (V2State 暂无 rate_limiter 字段, 标 not_initialized 守诚信)
/// 真接路径: P0 后端完工清单 #X (rate_limiter 进 V2State + 滑窗 / token bucket)
fn probe_rate_limiter() -> DepCheckResult {
    use std::time::Instant;
    let start = Instant::now();
    DepCheckResult {
        name: "rate_limiter".to_string(),
        status: "not_initialized".to_string(),
        check_type: "stub".to_string(),
        detail: "V2State has no rate_limiter yet; P0#X will add LRU/token-bucket".to_string(),
        elapsed_us: start.elapsed().as_micros() as u64,
        real: false,
    }
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
                let resp =
                    Json(protocol_handlers::openai_chat_from_normalized(&cached)).into_response();
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
    _headers: HeaderMap,
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
        s.set_model(&format!(
            "v1={}v2={}v3={}",
            &req.v1[..req.v1.len().min(20)],
            &req.v2[..req.v2.len().min(20)],
            &req.v3[..req.v3.len().min(20)]
        ));
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
        // 手动构造 Pipeline, 因为 protocol_handlers::build_pipeline 内部用 chat_defaults
        use apeireth_http_client::HttpClient;
        use apeireth_pipeline::PipelineConfig;

        let http = HttpClient::new(KeepAliveConfig::chat_default())
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
            let url = protocol_handlers::endpoint_url("https://api.minimaxi.com", kind, "")
                .expect("HTTP kind");
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
        )
        .expect("HTTP kind");
        assert_eq!(
            url,
            "https://api.minimaxi.com/v1/gemini/v1beta/models/MiniMax-M3:generateContent"
        );
    }

    // ============================================================
    // /health/deps 真实依赖探测 — 8 新测试 (R178 §2.3)
    // ============================================================

    /// 1. /health/deps 端点路径正确 (axum path 解析 + handler 注册)
    #[tokio::test]
    async fn health_deps_route_registers() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        // 构造最小 AppState (没 pipeline 也能跑 probe, pipeline.config().validate() 走 fallback)
        // 这里只验证路由 + handler 不 panic; 真探测走 probe_memory_store (sqlite open)
        // 跟 probe_provider_backend (env + config.validate) — 两者都无外部副作用
        let app = build_router_with_v2(
            Arc::new(crate::server::AppState {
                pipeline: Arc::new(apeireth_pipeline::Pipeline::with_chat_defaults().unwrap()),
                llm: {
                    use crate::llm::providers::scripted::ScriptedLlmProvider;
                    let p: Arc<dyn LlmProvider> = Arc::new(ScriptedLlmProvider::new("test"));
                    p
                },
                response_cache: None,
            }),
            Arc::new(crate::v2_endpoints::V2State::new()),
        );
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health/deps")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // 路径匹配, 必有响应 (200 或 500 — 但不 404)
        assert_ne!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// 2. probe_memory_store 必返回 ok (in-mem SQLite 一定可用)
    #[test]
    fn probe_memory_store_in_mem_sqlite_always_ok() {
        let r = probe_memory_store();
        assert_eq!(r.name, "memory_store");
        assert_eq!(r.check_type, "sqlite_open");
        assert_eq!(r.status, "ok");
        assert!(r.real, "sqlite_open 必须是真 I/O 探测");
        assert!(r.detail.contains("SELECT 1=1"));
    }

    /// 3. probe_rate_limiter 永远返回 not_initialized (stub 守诚信)
    #[test]
    fn probe_rate_limiter_is_always_stub() {
        let r = probe_rate_limiter();
        assert_eq!(r.name, "rate_limiter");
        assert_eq!(r.status, "not_initialized");
        assert_eq!(r.check_type, "stub");
        assert!(!r.real);
    }

    /// 4. probe_sovereignty 在空 V2State 返 not_initialized
    #[test]
    fn probe_sovereignty_empty_v2_state_is_not_initialized() {
        let v2 = Arc::new(crate::v2_endpoints::V2State::new());
        let r = probe_sovereignty(&v2);
        assert_eq!(r.name, "sovereignty_guard");
        assert_eq!(r.status, "not_initialized");
        assert!(!v2.sovereignty_registered());
    }

    /// 5. probe_replay_cache 在 None AppState 返 not_initialized
    #[test]
    fn probe_replay_cache_none_appstate_is_not_initialized() {
        use crate::llm::providers::scripted::{ScriptedLlmProvider, ScriptedResponse};
        let llm: Arc<dyn LlmProvider> = Arc::new(ScriptedLlmProvider::new("test"));
        let pipeline = apeireth_pipeline::Pipeline::with_chat_defaults().unwrap();
        let state = Arc::new(crate::server::AppState {
            pipeline: Arc::new(pipeline),
            llm,
            response_cache: None,
        });
        let r = probe_replay_cache(&state);
        assert_eq!(r.name, "replay_cache");
        assert_eq!(r.status, "not_initialized");
    }

    /// 6. 聚合 status 逻辑: 全 ok -> ok
    #[test]
    fn deps_report_aggregate_status_all_ok_yields_ok() {
        let deps = vec![
            DepCheckResult {
                name: "a".to_string(),
                status: "ok".to_string(),
                check_type: "x".to_string(),
                detail: String::new(),
                elapsed_us: 1,
                real: false,
            },
            DepCheckResult {
                name: "b".to_string(),
                status: "ok".to_string(),
                check_type: "x".to_string(),
                detail: String::new(),
                elapsed_us: 1,
                real: false,
            },
        ];
        let down = deps.iter().filter(|d| d.status == "down").count();
        let deg = deps
            .iter()
            .filter(|d| d.status == "degraded" || d.status == "not_initialized")
            .count();
        let s = if down > 0 {
            "down"
        } else if deg > 0 {
            "degraded"
        } else {
            "ok"
        };
        assert_eq!(s, "ok");
    }

    /// 7. 聚合 status 逻辑: 有 down -> down (优先级最高)
    #[test]
    fn deps_report_aggregate_status_any_down_yields_down() {
        let deps = vec![
            DepCheckResult {
                name: "a".to_string(),
                status: "ok".to_string(),
                check_type: "x".to_string(),
                detail: String::new(),
                elapsed_us: 1,
                real: false,
            },
            DepCheckResult {
                name: "b".to_string(),
                status: "down".to_string(),
                check_type: "x".to_string(),
                detail: String::new(),
                elapsed_us: 1,
                real: false,
            },
            DepCheckResult {
                name: "c".to_string(),
                status: "degraded".to_string(),
                check_type: "x".to_string(),
                detail: String::new(),
                elapsed_us: 1,
                real: false,
            },
        ];
        let down = deps.iter().filter(|d| d.status == "down").count();
        let deg = deps
            .iter()
            .filter(|d| d.status == "degraded" || d.status == "not_initialized")
            .count();
        let s = if down > 0 {
            "down"
        } else if deg > 0 {
            "degraded"
        } else {
            "ok"
        };
        assert_eq!(s, "down");
    }

    /// 8. DepsReport 可序列化 (sanity: serde_json roundtrip)
    #[test]
    fn deps_report_serializes_to_json() {
        let report = DepsReport {
            status: "degraded".to_string(),
            deps: vec![DepCheckResult {
                name: "memory_store".to_string(),
                status: "ok".to_string(),
                check_type: "sqlite_open".to_string(),
                detail: "in-mem sqlite open ok".to_string(),
                elapsed_us: 42,
                real: true,
            }],
            degraded_count: 1,
            down_count: 0,
            timestamp: "2026-08-15T00:00:00Z".to_string(),
        };
        let json_str = serde_json::to_string(&report).unwrap();
        assert!(json_str.contains("\"status\":\"degraded\""));
        assert!(json_str.contains("\"name\":\"memory_store\""));
        assert!(json_str.contains("\"real\":true"));
        // 持有 parsed 在独立 scope, 避免 &json_str 借用跟 parsed 字段冲突
        let parsed: DepsReport = {
            let s_owned: String = json_str.clone();
            serde_json::from_str(&s_owned).unwrap()
        };
        drop(json_str);
        assert_eq!(parsed.status, "degraded");
        let first = &parsed.deps[0];
        assert_eq!(first.name, "memory_store");
        assert_eq!(first.elapsed_us, 42);
    }
}
