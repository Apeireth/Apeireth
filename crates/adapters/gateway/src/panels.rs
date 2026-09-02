//! Panel introspection surface (`/v1/panel/*`, `/v1/tools/list`, `/v1/apeireth/capabilities`).
//!
//! The gateway owns HTTP shape and transport only; concrete data access is
//! supplied by the composition root through [`PanelData`]. When no panel data
//! is configured (e.g. embedded tests), every panel route answers
//! `501 unsupported` with the canonical error body — the frontend treats that
//! as honest degradation, never as a transport failure.
//!
//! Response shapes follow `docs/gateway-api-contract.md` §4-§9 and mirror the
//! desktop types in `frontend/companion-desktop/src/lib/types.ts`.

use std::sync::Arc;

use apeireth_runtime::canonical::Runtime;
use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

/// Shared router state: the runtime, optional panel backends, and the
/// gateway-level SSE event bus.
#[derive(Clone)]
pub struct GatewayState {
    pub runtime: Arc<Runtime>,
    pub panels: Option<Arc<dyn PanelData>>,
    pub events: crate::events::EventBus,
}

// ---------------------------------------------------------------------------
// DTOs (stable contract — do not rename fields without updating the contract doc)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummaryDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: usize,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpanDto {
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub kind: String,
    pub actor: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummaryDto {
    pub trace_id: String,
    pub span_count: usize,
    pub started_at: i64,
    pub root_span: TraceSpanDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDetailDto {
    pub trace_id: String,
    pub spans: Vec<TraceSpanDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDto {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args_schema: Option<serde_json::Value>,
    pub source: String,
    pub permission: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDto {
    pub ts: i64,
    pub event: String,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpisodeDto {
    pub id: String,
    /// Epoch milliseconds (adapter converts the core seconds representation).
    pub timestamp: i64,
    pub role: String,
    pub content: String,
    pub session_id: String,
    /// Omitted when the backend does not store the field (rc honesty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpisodeMutationDto {
    pub ok: bool,
    pub rev: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdgeDto {
    pub from: String,
    pub to: String,
    pub weight: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryGraphDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantDto {
    /// Stable permission label, e.g. `execute_tool:tool.repo`.
    pub permission: String,
    /// Capability id this grant governs, e.g. `tool.repo`.
    pub capability: String,
    /// Omitted: the canonical policy does not timestamp grants (rc honesty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantMutationDto {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrganDto {
    /// Stable organ code, e.g. `W1`.
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// PanelData — the composition-root contract
// ---------------------------------------------------------------------------

/// Read/write handles for the introspection panels.
///
/// Implementations live in the composition root (the CLI adapts its session
/// store, tool catalog and trace/audit archives). All list methods return
/// newest-first. Errors are rendered as `500 runtime_error` with the message
/// as-is; implementations must not leak secrets into error strings.
#[async_trait]
pub trait PanelData: Send + Sync {
    /// Session summaries, most recently updated first.
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, String>;

    /// Tool catalog with permission annotations.
    async fn list_tools(&self) -> Result<Vec<ToolDto>, String>;

    /// Recent trace summaries, newest first.
    async fn list_traces(&self, limit: usize) -> Result<Vec<TraceSummaryDto>, String>;

    /// Full span list for one trace, or `None` when unknown.
    async fn trace_detail(&self, trace_id: &str) -> Result<Option<TraceDetailDto>, String>;

    /// Recent audit events, newest first.
    async fn list_audit(&self, limit: usize) -> Result<Vec<AuditDto>, String>;

    /// Best-effort audit append (chat/approval lifecycle). Never fails the turn.
    async fn append_audit(&self, event: &str, detail: Option<&str>);

    /// Best-effort trace archive append after a completed turn.
    async fn append_trace(&self, trace_id: &str, spans: Vec<TraceSpanDto>);

    /// Whether the memory introspection surface is available. Drives the
    /// capability manifest; defaults to `false` so a minimal embedder stays
    /// honest about what it can serve.
    fn supports_memory(&self) -> bool {
        false
    }

    /// Episodes, newest first, optionally filtered by session and/or a
    /// case-insensitive content query. Tombstoned episodes are omitted.
    async fn list_episodes(
        &self,
        session: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EpisodeDto>, String>;

    /// Append one episode and return its DTO (rev starts at 0).
    async fn append_episode(&self, session: &str, role: &str, content: &str)
        -> Result<EpisodeDto, String>;

    /// Gateway-level protect/unprotect/forget flags with optimistic revision
    /// checks. `expected_rev` must equal the current revision, otherwise a
    /// conflict error is returned and nothing changes.
    async fn protect_episode(&self, id: &str, expected_rev: u64) -> Result<EpisodeMutationDto, String>;
    async fn unprotect_episode(&self, id: &str, expected_rev: u64)
        -> Result<EpisodeMutationDto, String>;
    async fn forget_episode(
        &self,
        id: &str,
        expected_rev: u64,
        reason: Option<&str>,
    ) -> Result<EpisodeMutationDto, String>;

    /// Memory graph (v1 semantics: session nodes + episode nodes with
    /// containment edges derived from real stored data).
    async fn memory_graph(&self) -> Result<MemoryGraphDto, String>;

    /// Whether the permissions introspection surface (grants list / hot
    /// revoke) is available. Defaults to `false`.
    fn supports_permissions(&self) -> bool {
        false
    }

    /// Current grants, deterministic order.
    async fn list_grants(&self) -> Result<Vec<GrantDto>, String>;

    /// Revoke the grant governing `capability`. Session-scoped hot change:
    /// process restart restores the default policy.
    async fn revoke_grant(&self, capability: &str) -> Result<GrantMutationDto, String>;

    /// Whether an organ catalog is available. Defaults to `false`.
    fn supports_organs(&self) -> bool {
        false
    }

    /// The organ catalog (production default: organs chain disabled).
    async fn list_organs(&self) -> Result<Vec<OrganDto>, String>;
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Panel routes without attached state — merge into the canonical router
/// before its final `.with_state(...)`.
pub fn panel_routes() -> Router<GatewayState> {
    Router::new()
        .route("/v1/panel/sessions", get(list_sessions))
        .route("/v1/panel/traces", get(list_traces))
        .route("/v1/panel/traces/:trace_id", get(trace_detail))
        .route("/v1/tools/list", get(list_tools))
        .route("/v1/panel/audit", get(list_audit))
        .route("/v1/apeireth/capabilities", get(capabilities))
        .route("/v1/panel/memory/episodes", get(list_episodes))
        .route("/v1/memory/append", axum::routing::post(append_episode))
        .route(
            "/v1/apeireth/memory/episodes/:id/forget",
            axum::routing::post(forget_episode),
        )
        .route(
            "/v1/apeireth/memory/episodes/:id/protect",
            axum::routing::post(protect_episode),
        )
        .route(
            "/v1/apeireth/memory/episodes/:id/unprotect",
            axum::routing::post(unprotect_episode),
        )
        .route("/v1/panel/graph", get(memory_graph))
        .route("/v1/panel/grants", get(list_grants))
        .route(
            "/v1/panel/grants/revoke",
            axum::routing::post(revoke_grant),
        )
        .route("/v1/organs", get(list_organs))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unsupported(what: &str) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": {
                "code": "unsupported",
                "message": format!("{what} 不支持: 当前运行时未实现该内省 API (Apeireth 2.0 canonical gateway)")
            }
        })),
    )
        .into_response()
}

fn panel_error(message: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": { "code": "runtime_error", "message": message } })),
    )
        .into_response()
}

fn limit_of(query: Option<usize>) -> usize {
    query.unwrap_or(50).clamp(1, 500)
}

#[derive(Debug, Default, Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_sessions(State(state): State<GatewayState>) -> Response {
    let Some(panels) = &state.panels else {
        return unsupported("sessions.read");
    };
    match panels.list_sessions().await {
        Ok(sessions) => (StatusCode::OK, Json(serde_json::json!({ "sessions": sessions }))).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_tools(State(state): State<GatewayState>) -> Response {
    let Some(panels) = &state.panels else {
        return unsupported("tools.list");
    };
    match panels.list_tools().await {
        Ok(tools) => (StatusCode::OK, Json(serde_json::json!({ "tools": tools }))).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_traces(State(state): State<GatewayState>, Query(q): Query<LimitQuery>) -> Response {
    let Some(panels) = &state.panels else {
        return unsupported("trace.read");
    };
    match panels.list_traces(limit_of(q.limit)).await {
        Ok(traces) => (StatusCode::OK, Json(serde_json::json!({ "traces": traces }))).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn trace_detail(State(state): State<GatewayState>, Path(trace_id): Path<String>) -> Response {
    let Some(panels) = &state.panels else {
        return unsupported("trace.read");
    };
    match panels.trace_detail(&trace_id).await {
        Ok(Some(detail)) => (StatusCode::OK, Json(detail)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": { "code": "not_found", "message": format!("trace {trace_id} not found") } })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_audit(State(state): State<GatewayState>, Query(q): Query<LimitQuery>) -> Response {
    let Some(panels) = &state.panels else {
        return unsupported("audit.read");
    };
    match panels.list_audit(limit_of(q.limit)).await {
        Ok(events) => (StatusCode::OK, Json(serde_json::json!({ "events": events }))).into_response(),
        Err(e) => panel_error(e),
    }
}

// ---------------------------------------------------------------------------
// Memory introspection
// ---------------------------------------------------------------------------

fn memory_unavailable(state: &GatewayState) -> Option<Response> {
    let Some(panels) = &state.panels else {
        return Some(unsupported("memory.read"));
    };
    if !panels.supports_memory() {
        return Some(unsupported("memory.read"));
    }
    None
}

#[derive(Debug, Default, Deserialize)]
struct EpisodeQuery {
    limit: Option<usize>,
    q: Option<String>,
    session: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpisodeAppendRequest {
    session: String,
    content: String,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpisodeMutationRequest {
    expected_rev: u64,
    #[serde(default)]
    reason: Option<String>,
}

async fn list_episodes(
    State(state): State<GatewayState>,
    Query(q): Query<EpisodeQuery>,
) -> Response {
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels
        .list_episodes(q.session.as_deref(), q.q.as_deref(), limit_of(q.limit))
        .await
    {
        Ok(episodes) => {
            (StatusCode::OK, Json(serde_json::json!({ "episodes": episodes }))).into_response()
        }
        Err(e) => panel_error(e),
    }
}

async fn append_episode(
    State(state): State<GatewayState>,
    Json(request): Json<EpisodeAppendRequest>,
) -> Response {
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    if request.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": { "code": "invalid_request", "message": "content must not be empty" } })),
        )
            .into_response();
    }
    let panels = state.panels.as_ref().expect("checked above");
    let role = request.role.as_deref().unwrap_or("user");
    match panels.append_episode(&request.session, role, &request.content).await {
        Ok(episode) => (StatusCode::CREATED, Json(episode)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn protect_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeMutationRequest>,
) -> Response {
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels.protect_episode(&id, request.expected_rev).await {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn unprotect_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeMutationRequest>,
) -> Response {
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels.unprotect_episode(&id, request.expected_rev).await {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn forget_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeMutationRequest>,
) -> Response {
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels
        .forget_episode(&id, request.expected_rev, request.reason.as_deref())
        .await
    {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn memory_graph(State(state): State<GatewayState>) -> Response {
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels.memory_graph().await {
        Ok(graph) => (StatusCode::OK, Json(graph)).into_response(),
        Err(e) => panel_error(e),
    }
}

// ---------------------------------------------------------------------------
// Permissions (grants / hot revoke) and organs
// ---------------------------------------------------------------------------

fn permissions_unavailable(state: &GatewayState) -> Option<Response> {
    let Some(panels) = &state.panels else {
        return Some(unsupported("permissions.grants.read"));
    };
    if !panels.supports_permissions() {
        return Some(unsupported("permissions.grants.read"));
    }
    None
}

#[derive(Debug, Deserialize)]
struct RevokeGrantRequest {
    capability: String,
}

async fn list_grants(State(state): State<GatewayState>) -> Response {
    if let Some(response) = permissions_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels.list_grants().await {
        Ok(grants) => (StatusCode::OK, Json(serde_json::json!({ "grants": grants }))).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn revoke_grant(
    State(state): State<GatewayState>,
    Json(request): Json<RevokeGrantRequest>,
) -> Response {
    if let Some(response) = permissions_unavailable(&state) {
        return response;
    }
    let panels = state.panels.as_ref().expect("checked above");
    match panels.revoke_grant(&request.capability).await {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_organs(State(state): State<GatewayState>) -> Response {
    let Some(panels) = &state.panels else {
        return unsupported("organs.list");
    };
    if !panels.supports_organs() {
        return unsupported("organs.list");
    }
    match panels.list_organs().await {
        Ok(organs) => (StatusCode::OK, Json(serde_json::json!({ "organs": organs }))).into_response(),
        Err(e) => panel_error(e),
    }
}

// ---------------------------------------------------------------------------
// Capability manifest
// ---------------------------------------------------------------------------

fn cap(id: &str, read: bool, write: bool, ops: &[&str], supported: bool, available: bool) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "supported": supported,
        "read": read,
        "write": write,
        "version": 1,
        "operations": ops,
        "available": available,
    })
}

async fn capabilities(State(state): State<GatewayState>) -> Response {
    let panels = state.panels.is_some();
    let memory_supported = state
        .panels
        .as_ref()
        .map(|p| p.supports_memory())
        .unwrap_or(false);
    let permissions_supported = state
        .panels
        .as_ref()
        .map(|p| p.supports_permissions())
        .unwrap_or(false);
    let organs_supported = state
        .panels
        .as_ref()
        .map(|p| p.supports_organs())
        .unwrap_or(false);

    let memory_ids = [
        ("memory.read", true, true),
        ("memory.write", true, true),
        ("memory.forget", false, true),
        ("memory.protect", false, true),
        ("memory.unprotect", false, true),
    ];
    let memory = memory_ids
        .iter()
        .map(|(id, read, write)| {
            cap(id, *read, *write, &["list", "append"], memory_supported, memory_supported)
        })
        .chain(std::iter::once(cap(
            "memory.graph.read",
            true,
            false,
            &["graph"],
            memory_supported,
            memory_supported,
        )))
        .collect::<Vec<_>>();

    let manifest = serde_json::json!({
        "schema_version": 1,
        "runtime": { "service": "apeireth-gateway-2.0", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": [
            { "name": "health", "capabilities": [ cap("health", true, false, &["check"], true, true) ] },
            { "name": "models", "capabilities": [ cap("models.list", true, false, &["list"], true, true) ] },
            { "name": "chat", "capabilities": [ cap("chat.completions", true, true, &["stream"], true, true) ] },
            { "name": "sessions", "capabilities": [ cap("sessions.read", true, false, &["list"], panels, panels) ] },
            { "name": "memory", "capabilities": memory },
            { "name": "tools", "capabilities": [ cap("tools.list", true, false, &["list"], panels, panels) ] },
            { "name": "permissions", "capabilities": [
                cap("permissions.approval.read", true, false, &["list"], true, true),
                cap("permissions.grants.read", true, false, &["list"], permissions_supported, permissions_supported),
                cap("permissions.revoke", false, true, &["revoke"], permissions_supported, permissions_supported),
            ] },
            { "name": "organs", "capabilities": [
                cap("organs.list", true, false, &["list"], organs_supported, organs_supported),
            ] },
            { "name": "trace", "capabilities": [ cap("trace.read", true, false, &["list", "detail"], panels, panels) ] },
            { "name": "audit", "capabilities": [ cap("audit.read", true, false, &["list"], panels, panels) ] },
            { "name": "activity", "capabilities": [ cap("activity.sse", true, false, &["subscribe"], true, true) ] },
        ]
    });
    (StatusCode::OK, Json(manifest)).into_response()
}
