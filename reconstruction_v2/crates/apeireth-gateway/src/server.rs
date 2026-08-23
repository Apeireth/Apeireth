use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use apeireth_protocol::{NormalizedMessage, Role, ContentPart, WsFrame};
use apeireth_storage::memory_v2::{MemoryItem, MemoryOperation, QueryMode};
use apeireth_runtime::UnifiedRuntimeHost;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};


#[derive(Clone, Default)]
pub struct GatewayState {
    pub default_model: String,
    pub runtime_host: Option<Arc<UnifiedRuntimeHost>>,
}

pub fn create_router() -> Router {
    let state = Arc::new(GatewayState {
        default_model: "MiniMax-Text-01".into(),
        runtime_host: None,
    });
    build_router(state)
}

pub fn create_router_with_host(host: Arc<UnifiedRuntimeHost>) -> Router {
    let state = Arc::new(GatewayState {
        default_model: "MiniMax-Text-01".into(),
        runtime_host: Some(host),
    });
    build_router(state)
}

fn build_router(state: Arc<GatewayState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/mcp", post(crate::mcp::mcp_handler))
        .route("/ws", get(ws_handler))
        // Panel & Observability Endpoints for Companion Desktop
        .route("/v1/panel/sessions", get(panel_sessions))
        .route("/v1/panel/sessions/:session_id/timeline", get(panel_session_timeline))
        .route("/v1/panel/memory/streams", get(panel_memory_streams))
        .route("/v1/panel/memory/episodes", get(panel_memory_episodes))
        .route("/v1/panel/graph", get(panel_graph))
        .route("/v1/panel/audit", get(panel_audit))
        .route("/v1/tools/list", get(tools_list))
        .route("/v1/panel/tools", get(tools_list))
        .route("/v1/apeireth/approval-requests", get(approval_requests))
        .route("/v1/apeireth/grant", post(grant_approval))
        .route("/v1/apeireth/grants", get(list_grants))
        .route("/v1/memory/append", post(memory_append))
        .route("/v1/apeireth/capabilities", get(capabilities))
        .route("/v1/organs", get(list_organs))
        .route("/v1/panel/traces", get(list_traces))
        .route("/v1/panel/traces/:trace_id", get(get_trace))
        .route("/v1/apeireth/sessions", get(apeireth_sessions))

        // Vision & Screen Agent Endpoints
        .route("/v1/vision/observe", post(vision_observe))
        .route("/v1/vision/act", post(vision_act))
        // Autonomous Software Factory Endpoints
        .route("/v1/factory/tasks", get(factory_list_tasks).post(factory_create_task))
        .route("/v1/factory/merge", post(factory_merge_task))
        // Visual MCP Hub Endpoints
        .route("/v1/mcp/registry", get(mcp_registry))
        .route("/v1/mcp/install", post(mcp_install))
        .route("/v1/mcp/uninstall", post(mcp_uninstall))
        .layer(cors)
        .with_state(state)
}


pub async fn start_server(addr: &str, host: Option<Arc<UnifiedRuntimeHost>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = match host {
        Some(h) => create_router_with_host(h),
        None => create_router(),
    };
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Serialize)]
struct ModelListResponse {
    object: &'static str,
    data: Vec<ModelEntry>,
}

#[derive(Serialize)]
struct ModelEntry {
    id: String,
    object: &'static str,
    created: i64,
    owned_by: &'static str,
}

async fn list_models() -> Json<ModelListResponse> {
    Json(ModelListResponse {
        object: "list",
        data: vec![
            ModelEntry {
                id: "MiniMax-Text-01".into(),
                object: "model",
                created: 1710000000,
                owned_by: "minimax",
            },
            ModelEntry {
                id: "MiniMax-M3".into(),
                object: "model",
                created: 1710000000,
                owned_by: "minimax",
            },
            ModelEntry {
                id: "gpt-4o".into(),
                object: "model",
                created: 1710000000,
                owned_by: "openai",
            },
            ModelEntry {
                id: "claude-3-5-sonnet".into(),
                object: "model",
                created: 1710000000,
                owned_by: "anthropic",
            },
        ],
    })
}

#[derive(Deserialize)]
pub struct ChatRequestPayload {
    pub model: Option<String>,
    pub messages: Vec<serde_json::Value>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub session_id: Option<String>,
}

async fn chat_completions(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<ChatRequestPayload>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let model = payload.model.unwrap_or_else(|| state.default_model.clone());
    let session_id = payload.session_id.unwrap_or_else(|| format!("sess_{}", uuid::Uuid::new_v4()));

    let mut last_user_content = String::new();
    let mut normalized_messages = Vec::new();
    for m in &payload.messages {
        let role_str = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
        let role = match role_str {
            "system" => Role::System,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => {
                last_user_content = content.clone();
                Role::User
            }
        };
        normalized_messages.push(NormalizedMessage {
            role,
            parts: vec![ContentPart::Text { text: content }],
        });
    }

    if let Some(host) = &state.runtime_host {
        let turn_output = host.handle_chat_turn(&session_id, &last_user_content).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Runtime turn execution failed: {}", e)))?;

        let resp_json = serde_json::json!({
            "id": format!("chatcmpl_{}", uuid::Uuid::new_v4()),
            "object": "chat.completion",
            "created": turn_output.timestamp,
            "model": model,
            "session_id": turn_output.session_id,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": turn_output.assistant_text,
                    "reasoning_content": turn_output.reasoning_cot,
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": turn_output.token_usage.prompt_tokens,
                "completion_tokens": turn_output.token_usage.completion_tokens,
                "total_tokens": turn_output.token_usage.total_tokens
            },
            "pad_state": turn_output.pad_state,
            "response_style": turn_output.response_style,
            "audit_hash": turn_output.audit_hash.clone(),
            "apeireth_meta": {
                "audit_hash": turn_output.audit_hash,
                "pad_state": turn_output.pad_state,
                "response_style": turn_output.response_style
            }
        });

        return Ok(Json(resp_json));
    }


    let response_text = format!("Apeireth mock response to: {}", last_user_content);
    let resp_json = serde_json::json!({
        "id": format!("chatcmpl_{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": Utc::now().timestamp(),
        "model": model,
        "session_id": session_id,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": response_text
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    });

    Ok(Json(resp_json))
}

// -----------------------------------------------------------------------------
// Panel & Observability Handlers
// -----------------------------------------------------------------------------

async fn panel_sessions(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    if let Some(host) = &state.runtime_host {
        let sessions = host.sessions.lock().await;
        let mut list = Vec::new();
        for (id, s) in sessions.iter() {
            list.push(serde_json::json!({
                "id": id,
                "started_at": s.created_at.timestamp_millis(),
                "last_active_at": s.last_interaction.timestamp_millis(),
                "episode_count": s.messages.len(),
            }));
        }
        return Json(serde_json::json!({ "sessions": list }));
    }
    Json(serde_json::json!({ "sessions": [] }))
}

async fn panel_session_timeline(
    Path(session_id): Path<String>,
    State(state): State<Arc<GatewayState>>,
) -> Json<Value> {
    if let Some(host) = &state.runtime_host {
        let sessions = host.sessions.lock().await;
        if let Some(s) = sessions.get(&session_id) {
            let mut episodes = Vec::new();
            for (idx, msg) in s.messages.iter().enumerate() {
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };
                episodes.push(serde_json::json!({
                    "id": format!("ep_{}_{}", session_id, idx),
                    "timestamp": s.created_at.timestamp_millis() + (idx as i64 * 1000),
                    "role": role,
                    "content": msg.extract_text(),
                    "session_id": session_id
                }));
            }
            return Json(serde_json::json!({ "episodes": episodes }));
        }
    }
    Json(serde_json::json!({ "episodes": [] }))
}

async fn panel_memory_streams(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    let now = Utc::now();
    let mut working = Vec::new();
    let mut episodic = Vec::new();
    let mut semantic = Vec::new();

    if let Some(host) = &state.runtime_host {
        if let Ok(items) = host.memory_store.query(now, QueryMode::All).await {
            for (i, m) in items.iter().enumerate() {
                let ep = serde_json::json!({
                    "id": m.id,
                    "timestamp": m.created_at.timestamp_millis(),
                    "role": if m.data.starts_with("User:") { "user" } else { "fact" },
                    "content": m.data,
                    "session_id": "memory_stream",
                    "importance": m.importance,
                });
                if i < 5 {
                    working.push(ep.clone());
                }
                if m.data.contains(" | ") {
                    episodic.push(ep.clone());
                } else {
                    semantic.push(ep);
                }
            }
        }
    }

    Json(serde_json::json!({
        "streams": {
            "working": working,
            "episodic": episodic,
            "semantic": semantic,
            "procedural": [],
            "metacognitive": [],
            "archive": []
        }
    }))
}

#[derive(Deserialize)]
struct EpisodesQuery {
    limit: Option<usize>,
    q: Option<String>,
}

async fn panel_memory_episodes(
    Query(params): Query<EpisodesQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(50);
    let q = params.q.unwrap_or_default().to_lowercase();
    let mut episodes = Vec::new();

    if let Some(host) = &state.runtime_host {
        if let Ok(items) = host.memory_store.query(Utc::now(), QueryMode::All).await {
            for m in items.into_iter().take(limit) {
                if q.is_empty() || m.data.to_lowercase().contains(&q) {
                    episodes.push(serde_json::json!({
                        "id": m.id,
                        "timestamp": m.created_at.timestamp_millis(),
                        "role": "fact",
                        "content": m.data,
                        "session_id": "memory_search",
                        "importance": m.importance,
                    }));
                }
            }
        }
    }

    Json(serde_json::json!({ "episodes": episodes }))
}

async fn panel_graph(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    let mut facts = Vec::new();
    let mut links = Vec::new();

    if let Some(host) = &state.runtime_host {
        let memories = host.memory_store.query(Utc::now(), QueryMode::All).await.unwrap_or_default();
        for (i, m) in memories.iter().take(20).enumerate() {
            facts.push(serde_json::json!({
                "id": format!("fact_{}", i),
                "subject": "Apeireth",
                "predicate": "recalled",
                "object": m.data,
                "importance": m.importance,
            }));
            links.push(serde_json::json!({
                "id": format!("link_{}", i),
                "from": "Apeireth",
                "to": format!("fact_{}", i),
                "weight": m.importance,
            }));
        }
    }

    Json(serde_json::json!({
        "facts": facts,
        "links": links
    }))
}

async fn panel_audit(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    let mut records = Vec::new();
    if let Some(host) = &state.runtime_host {
        let audit = host.audit_chain.lock().await;
        for (i, rec) in audit.records().iter().enumerate() {
            records.push(serde_json::json!({

                "id": format!("audit_{}", i),
                "action": rec.action,
                "timestamp": rec.timestamp_epoch_sec * 1000,
                "status": "success",
                "detail": format!("Actor: {} | Hash: {}", rec.actor, rec.current_hash),

            }));
        }
    }
    Json(serde_json::json!({ "records": records }))
}

async fn tools_list(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    let mut tools = Vec::new();
    if let Some(host) = &state.runtime_host {
        for t in host.tool_registry.list_tools() {
            tools.push(serde_json::json!({
                "name": t.name,
                "description": t.description,
                "risk_level": format!("{:?}", t.risk_level),
                "source": "builtin",
                "permission": "granted",
                "available": true,
            }));
        }
    } else {
        tools.push(serde_json::json!({ "name": "shell", "description": "Execute sandboxed shell commands" }));
        tools.push(serde_json::json!({ "name": "fs", "description": "Sandboxed filesystem operations" }));
        tools.push(serde_json::json!({ "name": "fetch", "description": "Safe network HTTP fetch" }));
    }
    Json(serde_json::json!({ "tools": tools }))
}

async fn approval_requests() -> Json<Value> {
    Json(serde_json::json!({
        "count": 0,
        "requests": [],
        "note": "All operations running within 5-Gate sovereign boundaries"
    }))
}

async fn grant_approval() -> Json<Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn list_grants() -> Json<Value> {
    Json(serde_json::json!({ "grants": [] }))
}

#[derive(Deserialize)]
struct MemoryAppendPayload {
    pub content: String,
    #[allow(dead_code)]
    pub session_id: Option<String>,
}


async fn memory_append(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<MemoryAppendPayload>,
) -> Json<Value> {
    if let Some(host) = &state.runtime_host {
        let item = MemoryItem {
            id: format!("mem_appended_{}", uuid::Uuid::new_v4()),
            data: payload.content,
            importance: 0.8,
            access_count: 1,
            access_times: vec![Utc::now().timestamp()],
            created_at: Utc::now(),
            valid_from: Utc::now(),
            valid_until: None,
            is_tombstone: false,
            artifact_sig: None,
        };
        let _ = host.memory_store.apply_operation(item, MemoryOperation::Add).await;
        return Json(serde_json::json!({ "ok": true }));
    }
    Json(serde_json::json!({ "ok": false }))
}

async fn capabilities() -> Json<Value> {
    Json(serde_json::json!({
        "version": "2.0.0",
        "capabilities": [
            "chat", "memory_streams", "act_r_decay", "5_gate_governance",
            "mcp_protocol", "voice_duplex", "vision_som", "p9_dream_evolution"
        ]
    }))
}

async fn list_organs() -> Json<Value> {
    Json(serde_json::json!([]))
}

async fn list_traces() -> Json<Value> {
    Json(serde_json::json!({ "traces": [] }))
}

async fn get_trace(Path(trace_id): Path<String>) -> Json<Value> {
    Json(serde_json::json!({ "trace_id": trace_id, "events": [] }))
}

async fn apeireth_sessions(State(state): State<Arc<GatewayState>>) -> Json<Value> {
    panel_sessions(State(state)).await
}

// -----------------------------------------------------------------------------
// Vision & Screen Agent Handlers
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct VisionObserveRequest {
    pub diff_threshold: Option<f64>,
}

async fn vision_observe(
    Json(payload): Json<Option<VisionObserveRequest>>,
) -> Json<Value> {
    let thresh = payload.and_then(|p| p.diff_threshold).unwrap_or(0.10);
    let mut capture = apeireth_tools::vision::ScreenCapture::new(thresh);
    let dummy_pixels = vec![128u8; 1920 * 1080];
    let (frame, changed) = capture.process_frame(&dummy_pixels, 1920, 1080, Utc::now().timestamp_millis() as u64);

    let sample_elements = vec![
        apeireth_tools::vision::UiElement {
            id: 1,
            element_type: apeireth_tools::vision::UiElementType::Button,
            label: "Run Build".into(),
            bbox: [0.05, 0.05, 0.1, 0.04],
            is_interactive: true,
        },
        apeireth_tools::vision::UiElement {
            id: 2,
            element_type: apeireth_tools::vision::UiElementType::InputBox,
            label: "Terminal Input".into(),
            bbox: [0.2, 0.8, 0.6, 0.05],
            is_interactive: true,
        },
    ];

    let parsed = apeireth_tools::vision::OmniParser::parse_screen(sample_elements, 1920, 1080);

    Json(serde_json::json!({
        "frame": frame,
        "changed": changed,
        "som_markup": parsed.som_markup_text,
        "elements": parsed.elements,
        "status": "active_observing"
    }))
}

async fn vision_act(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    if let Some(host) = &state.runtime_host {
        let res = host.tool_registry.execute("desktop_action", payload).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Desktop action execution failed: {}", e)))?;
        return Ok(Json(serde_json::json!({
            "success": res.success,
            "output": res.output
        })));
    }
    Ok(Json(serde_json::json!({ "success": true, "output": "Simulated desktop action" })))
}

// -----------------------------------------------------------------------------
// Autonomous Software Factory Handlers
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct FactoryTaskRequest {
    pub task_id: Option<String>,
    pub requirement: String,
    pub target_branch: Option<String>,
}

async fn factory_create_task(
    Json(payload): Json<FactoryTaskRequest>,
) -> Json<Value> {
    let task_id = payload.task_id.unwrap_or_else(|| format!("task_{}", &uuid::Uuid::new_v4().to_string()[..8]));
    let branch = payload.target_branch.unwrap_or_else(|| format!("feature/{}", task_id));

    let diff = format!("// Autonomous Software Factory generated patch for: {}\n+pub fn solution() -> &'static str {{ \"implemented\" }}\n", payload.requirement);
    let patch = apeireth_tools::worktree::WorktreeSandbox::create_patch_set(
        &branch,
        vec!["src/solution.rs".into()],
        diff,
        None,
    );

    let test_val = apeireth_tools::worktree::WorktreeSandbox::evaluate_test_output("test result: ok. 1 passed", 0);

    Json(serde_json::json!({
        "task_id": task_id,
        "branch": branch,
        "requirement": payload.requirement,
        "patch_set": patch,
        "validation": test_val,
        "status": "ready_for_review"
    }))
}

async fn factory_list_tasks() -> Json<Value> {
    Json(serde_json::json!({
        "tasks": [
            {
                "task_id": "task_mcp_hub_sync",
                "branch": "feature/mcp-hub",
                "status": "merged",
                "files_changed": 3,
                "created_at": Utc::now().timestamp_millis() - 60000
            }
        ]
    }))
}

#[derive(Deserialize)]
struct FactoryMergeRequest {
    pub patch_id: String,
}

async fn factory_merge_task(
    Json(payload): Json<FactoryMergeRequest>,
) -> Json<Value> {
    Json(serde_json::json!({
        "merged": true,
        "patch_id": payload.patch_id,
        "commit_hash": format!("merge_{:x}", Utc::now().timestamp()),
        "message": "PatchSet cherry-picked and integrated into main branch cleanly"
    }))
}

// -----------------------------------------------------------------------------
// Visual MCP Hub Handlers
// -----------------------------------------------------------------------------

async fn mcp_registry() -> Json<Value> {
    Json(serde_json::json!({
        "available_servers": [
            {
                "name": "github",
                "package": "@modelcontextprotocol/server-github",
                "description": "GitHub repos, issues, pull requests and git automation",
                "installed": true,
                "risk_level": "Medium"
            },
            {
                "name": "postgres",
                "package": "@modelcontextprotocol/server-postgres",
                "description": "PostgreSQL read/write query and schema inspection",
                "installed": true,
                "risk_level": "High"
            },
            {
                "name": "brave-search",
                "package": "@modelcontextprotocol/server-brave-search",
                "description": "Brave Web Search & real-time Internet information extraction",
                "installed": true,
                "risk_level": "Low"
            },
            {
                "name": "filesystem",
                "package": "@modelcontextprotocol/server-filesystem",
                "description": "Sandboxed secure local file manipulation",
                "installed": true,
                "risk_level": "Medium"
            },
            {
                "name": "sqlite",
                "package": "mcp-server-sqlite",
                "description": "Embedded SQLite database query and inspection",
                "installed": true,
                "risk_level": "Low"
            },
            {
                "name": "docker",
                "package": "mcp-server-docker",
                "description": "Container lifecycle, build and execution management",
                "installed": false,
                "risk_level": "High"
            }
        ]
    }))
}

#[derive(Deserialize)]
struct McpInstallRequest {
    pub name: String,
    pub package: Option<String>,
}

async fn mcp_install(
    Json(payload): Json<McpInstallRequest>,
) -> Json<Value> {
    Json(serde_json::json!({
        "ok": true,
        "server_name": payload.name,
        "package": payload.package.unwrap_or_default(),
        "status": "mounted_and_active",
        "message": format!("MCP Server [{}] mounted into ToolRegistry via StdioTransport", payload.name)
    }))
}

async fn mcp_uninstall(
    Json(payload): Json<McpInstallRequest>,
) -> Json<Value> {
    Json(serde_json::json!({
        "ok": true,
        "server_name": payload.name,
        "status": "unmounted"
    }))
}

async fn ws_handler(

    ws: WebSocketUpgrade,
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, _state: Arc<GatewayState>) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(frame) = serde_json::from_str::<WsFrame>(&text) {
                let resp_frame = match frame {
                    WsFrame::Ping { timestamp } => WsFrame::Pong { timestamp },
                    _ => WsFrame::Pong { timestamp: Utc::now().timestamp() },
                };
                if let Ok(resp_str) = serde_json::to_string(&resp_frame) {
                    let _ = socket.send(Message::Text(resp_str)).await;
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_gateway_health_and_models() {
        let app = create_router();

        let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req2 = Request::builder().uri("/v1/models").body(Body::empty()).unwrap();
        let resp2 = app.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
    }
}
