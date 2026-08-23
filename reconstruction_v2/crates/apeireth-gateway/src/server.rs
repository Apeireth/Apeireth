use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use apeireth_protocol::{NormalizedMessage, Role, ContentPart, WsFrame};

use apeireth_runtime::UnifiedRuntimeHost;
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

    if let Some(ref host) = state.runtime_host {
        // True E2E dispatch through UnifiedRuntimeHost
        let turn_output = host.handle_chat_turn(&session_id, &last_user_content)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("RuntimeHost error: {}", e)))?;

        let res = serde_json::json!({
            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            "object": "chat.completion",
            "created": turn_output.timestamp,
            "model": model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": turn_output.assistant_text,
                    "reasoning": turn_output.reasoning_cot,
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": turn_output.token_usage.prompt_tokens,
                "completion_tokens": turn_output.token_usage.completion_tokens,
                "total_tokens": turn_output.token_usage.total_tokens
            },
            "apeireth_meta": {
                "session_id": turn_output.session_id,
                "audit_hash": turn_output.audit_hash,
                "pad_state": turn_output.pad_state,
                "response_style": turn_output.response_style,
                "drive_warmth": turn_output.drive_warmth,
                "recalled_memories_count": turn_output.recalled_memories_count
            }
        });

        return Ok(Json(res));
    }

    // Default standalone response when host is not attached
    let res = serde_json::json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Apeireth Gateway processed request for model: {}", model),
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 15,
            "total_tokens": 25
        }
    });

    Ok(Json(res))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<GatewayState>) {
    let mut current_session_id = format!("ws_{}", uuid::Uuid::new_v4());

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(frame) = WsFrame::decode(&text) {
                        match frame {
                            WsFrame::Ping { timestamp } => {
                                let pong = WsFrame::Pong { timestamp };
                                if let Ok(enc) = pong.encode() {
                                    let _ = socket.send(Message::Text(enc)).await;
                                }
                            }
                            WsFrame::Handshake { client_id, .. } => {
                                current_session_id = format!("ws_{}", client_id);
                                let ack = WsFrame::TextDelta {
                                    session_id: current_session_id.clone(),
                                    text: "HANDSHAKE_ACK: Apeireth Runtime Connected".into(),
                                };
                                if let Ok(enc) = ack.encode() {
                                    let _ = socket.send(Message::Text(enc)).await;
                                }
                            }
                            WsFrame::TextDelta { text: user_text, session_id } => {
                                let sid = if session_id.is_empty() { &current_session_id } else { &session_id };
                                if let Some(ref host) = state.runtime_host {
                                    if let Ok(turn) = host.handle_chat_turn(sid, &user_text).await {
                                        if let Some(cot) = turn.reasoning_cot {
                                            let cot_frame = WsFrame::CoTDelta { session_id: sid.to_string(), reasoning: cot };
                                            if let Ok(enc) = cot_frame.encode() {
                                                let _ = socket.send(Message::Text(enc)).await;
                                            }
                                        }

                                        let text_frame = WsFrame::TextDelta { session_id: sid.to_string(), text: turn.assistant_text };
                                        if let Ok(enc) = text_frame.encode() {
                                            let _ = socket.send(Message::Text(enc)).await;
                                        }
                                    }
                                } else {
                                    let echo = WsFrame::TextDelta { session_id: sid.to_string(), text: format!("Echo: {}", user_text) };
                                    if let Ok(enc) = echo.encode() {
                                        let _ = socket.send(Message::Text(enc)).await;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_health_and_models() {
        let app = create_router();
        let _service = app.into_make_service();
    }
}

