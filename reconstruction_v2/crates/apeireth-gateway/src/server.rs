use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use apeireth_protocol::{NormalizedRequest, NormalizedMessage, Role, ContentPart, WsFrame};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};


#[derive(Clone, Default)]
pub struct GatewayState {
    pub default_model: String,
}

pub fn create_router() -> Router {
    let state = Arc::new(GatewayState {
        default_model: "MiniMax-Text-01".into(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

pub async fn start_server(addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router();
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
}

async fn chat_completions(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<ChatRequestPayload>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let model = payload.model.unwrap_or_else(|| state.default_model.clone());

    let mut normalized_messages = Vec::new();
    for m in payload.messages {
        let role = match m.get("role").and_then(|r| r.as_str()).unwrap_or("user") {
            "system" => Role::System,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::User,
        };
        let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
        normalized_messages.push(NormalizedMessage {
            role,
            parts: vec![ContentPart::Text { text: content }],
        });
    }

    let _req = NormalizedRequest {
        model: model.clone(),
        messages: normalized_messages,
        temperature: payload.temperature,
        max_tokens: payload.max_tokens,
        tools: None,
        stream: payload.stream.unwrap_or(false),
    };


    // Return unified OpenAI-compatible chat completion JSON response
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
    State(_state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
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
                                let welcome = WsFrame::TextDelta {
                                    session_id: client_id,
                                    text: "Connected to Apeireth 2.0 Gateway WebSocket".into(),
                                };
                                if let Ok(enc) = welcome.encode() {
                                    let _ = socket.send(Message::Text(enc)).await;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_gateway_health_and_models() {
        let app = create_router();

        // 1. Health check test
        let res = app.clone()
            .oneshot(Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 2. Models list test
        let res2 = app
            .oneshot(Request::builder().uri("/v1/models").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
    }
}

