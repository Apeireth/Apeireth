//! HTTP and native chat adapters for the canonical runtime.
//!
//! This module owns decoding, transport validation, canonical turn
//! construction, runtime invocation, and response encoding. It intentionally
//! owns no provider selection, governance composition, session orchestration,
//! plugin lifecycle, tool dispatch, retry, or agent loop.

use std::sync::Arc;

use apeireth_core::kernel::{SessionId, Timestamp};
use apeireth_protocol::canonical::{ContentPart, NormalizedUsage};
use apeireth_runtime::canonical::{ExecutionTrace, Runtime, RuntimeError, TurnRequest};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

/// Native gateway request. HTTP and CLI transports can both construct it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanonicalChatRequest {
    /// Existing canonical session, or a fresh session when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    /// User input for this turn.
    pub input: String,
    /// Optional model override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// System instruction used only when the session is new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

impl CanonicalChatRequest {
    /// A request containing one user turn.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            session: None,
            input: input.into(),
            model: None,
            system: None,
        }
    }
}

/// Transport-neutral response returned after canonical execution.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalChatResponse {
    /// Stable session used by the full turn.
    pub session: SessionId,
    /// Runtime request identifier.
    pub request: String,
    /// Runtime trace identifier.
    pub trace_id: String,
    /// Final assistant text.
    pub text: String,
    /// Provider capability that served the final round.
    pub served_by: String,
    /// Provider round-trips taken.
    pub rounds: u32,
    /// Canonical token accounting.
    pub usage: NormalizedUsage,
    /// Structured execution metadata; never raw model reasoning.
    pub trace: ExecutionTrace,
}

/// Failure at the gateway adapter boundary.
#[derive(Debug, thiserror::Error)]
pub enum CanonicalEntryError {
    /// Transport input was not meaningful enough to form a turn.
    #[error("invalid chat request: {0}")]
    InvalidRequest(String),
    /// Canonical runtime execution failed.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

/// Invoke the canonical runtime through the real gateway entry adapter.
pub async fn execute_chat(
    runtime: &Runtime,
    request: CanonicalChatRequest,
) -> Result<CanonicalChatResponse, CanonicalEntryError> {
    if request.input.trim().is_empty() {
        return Err(CanonicalEntryError::InvalidRequest(
            "input must not be empty".into(),
        ));
    }

    let session = request.session.unwrap_or_else(SessionId::new);
    let mut turn = TurnRequest::new(session, request.input);
    if let Some(model) = request.model {
        turn = turn.with_model(model);
    }
    if let Some(system) = request.system {
        turn = turn.with_system(system);
    }

    let outcome = runtime.execute(turn).await?;
    Ok(CanonicalChatResponse {
        session: outcome.session,
        request: outcome.request.to_string(),
        trace_id: outcome.trace.trace.to_string(),
        text: outcome.text,
        served_by: outcome.served_by.to_string(),
        rounds: outcome.rounds,
        usage: outcome.usage,
        trace: outcome.trace,
    })
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatRequest {
    model: Option<String>,
    messages: Vec<OpenAiMessage>,
    #[serde(default)]
    session_id: Option<SessionId>,
}

#[derive(Debug, Serialize)]
struct OpenAiChatResponse {
    id: String,
    object: &'static str,
    created: i64,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: NormalizedUsage,
    apeireth: OpenAiExecutionMetadata,
}

#[derive(Debug, Serialize)]
struct OpenAiChoice {
    index: u32,
    message: OpenAiAssistantMessage,
    finish_reason: &'static str,
}

#[derive(Debug, Serialize)]
struct OpenAiAssistantMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiExecutionMetadata {
    session_id: String,
    trace_id: String,
    served_by: String,
    rounds: u32,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

type HttpError = (StatusCode, Json<ErrorBody>);

/// Build the production HTTP router around one long-lived canonical runtime.
pub fn canonical_router(runtime: Arc<Runtime>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/chat", post(native_chat))
        .route("/v1/chat/completions", post(openai_chat))
        .with_state(runtime)
}

/// Serve the canonical gateway until the listener closes.
pub async fn serve_canonical(
    listener: tokio::net::TcpListener,
    runtime: Arc<Runtime>,
) -> std::io::Result<()> {
    axum::serve(listener, canonical_router(runtime)).await
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "execution_owner": "apeireth-runtime::canonical"
    }))
}

async fn native_chat(
    State(runtime): State<Arc<Runtime>>,
    Json(request): Json<CanonicalChatRequest>,
) -> Result<Json<CanonicalChatResponse>, HttpError> {
    let mut request = request;
    let session = request.session.unwrap_or_else(SessionId::new);
    request.session = Some(session);
    execute_chat(runtime.as_ref(), request)
        .await
        .map(Json)
        .map_err(|error| http_error(error, Some(session)))
}

async fn openai_chat(
    State(runtime): State<Arc<Runtime>>,
    Json(request): Json<OpenAiChatRequest>,
) -> Result<Json<OpenAiChatResponse>, HttpError> {
    let session = request.session_id.unwrap_or_else(SessionId::new);
    let input = request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| ContentPart::join_text(&ContentPart::from_legacy_value(&message.content)))
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| {
            http_error(
                CanonicalEntryError::InvalidRequest(
                    "messages must contain a non-empty user message".into(),
                ),
                Some(session),
            )
        })?;
    let system = request
        .messages
        .iter()
        .filter(|message| message.role == "system")
        .map(|message| ContentPart::join_text(&ContentPart::from_legacy_value(&message.content)))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let native = CanonicalChatRequest {
        session: Some(session),
        input,
        model: request.model.clone(),
        system: (!system.is_empty()).then_some(system),
    };
    let outcome = execute_chat(runtime.as_ref(), native)
        .await
        .map_err(|error| http_error(error, Some(session)))?;
    let created = Timestamp::from_clock(runtime.clock().as_ref()).epoch_millis() / 1_000;

    Ok(Json(OpenAiChatResponse {
        id: outcome.request.clone(),
        object: "chat.completion",
        created,
        model: request.model.unwrap_or_default(),
        choices: vec![OpenAiChoice {
            index: 0,
            message: OpenAiAssistantMessage {
                role: "assistant",
                content: outcome.text,
            },
            finish_reason: "stop",
        }],
        usage: outcome.usage,
        apeireth: OpenAiExecutionMetadata {
            session_id: outcome.session.to_string(),
            trace_id: outcome.trace_id,
            served_by: outcome.served_by,
            rounds: outcome.rounds,
        },
    }))
}

fn http_error(error: CanonicalEntryError, session: Option<SessionId>) -> HttpError {
    let status = match &error {
        CanonicalEntryError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
        CanonicalEntryError::Runtime(RuntimeError::Denied { .. }) => StatusCode::FORBIDDEN,
        CanonicalEntryError::Runtime(RuntimeError::ApprovalRequired { .. }) => StatusCode::CONFLICT,
        CanonicalEntryError::Runtime(RuntimeError::NoProvider { .. })
        | CanonicalEntryError::Runtime(RuntimeError::Misconfigured(_)) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        CanonicalEntryError::Runtime(RuntimeError::Provider(_))
        | CanonicalEntryError::Runtime(RuntimeError::ProvidersExhausted { .. }) => {
            StatusCode::BAD_GATEWAY
        }
        CanonicalEntryError::Runtime(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ErrorBody {
            error: error.to_string(),
            session_id: session.map(|id| id.to_string()),
        }),
    )
}
