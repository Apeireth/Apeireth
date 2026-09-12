//! HTTP and native chat adapters for the canonical runtime.
//!
//! This module owns decoding, transport validation, canonical turn
//! construction, runtime invocation, and response encoding. It intentionally
//! owns no provider selection, governance composition, session orchestration,
//! plugin lifecycle, tool dispatch, retry, or agent loop.

use std::sync::Arc;

use apeireth_core::kernel::{ApprovalId, RequestId, SessionId, Timestamp};
use apeireth_protocol::canonical::{ContentPart, NormalizedUsage};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, ExecutionTrace, PendingApprovalView, Runtime,
    RuntimeError, TraceEvent, TurnOutcome, TurnRequest,
};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::events::{events_handler, EventBus, GatewayEvent};
use crate::panels::{panel_routes, GatewayServices, GatewayState, PanelData};

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
    /// Product-facing execution events derived from the trace.
    ///
    /// Desktop may render these. It must not execute tools from them.
    pub events: Vec<CanonicalExecutionEvent>,
}

/// Minimal product-facing execution event.
///
/// These are observations of the Main Loop. They are not a tool-call protocol
/// and they never authorize the client to run a capability.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CanonicalExecutionEvent {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<u32>,
}

fn events_from_trace(trace: &ExecutionTrace) -> Vec<CanonicalExecutionEvent> {
    let mut events = Vec::new();
    for entry in &trace.entries {
        match &entry.event {
            TraceEvent::CapabilityDispatched {
                capability,
                tool_call_id,
                round,
            } => events.push(CanonicalExecutionEvent {
                event: "tool_started".into(),
                tool_name: Some(capability.to_string()),
                capability_id: Some(capability.to_string()),
                tool_call_id: Some(tool_call_id.clone()),
                succeeded: None,
                approval_id: None,
                round: Some(*round),
            }),
            TraceEvent::CapabilityCompleted {
                capability,
                tool_call_id,
                succeeded,
                round,
            } => events.push(CanonicalExecutionEvent {
                event: if *succeeded {
                    "tool_completed".into()
                } else {
                    "tool_failed".into()
                },
                tool_name: Some(capability.to_string()),
                capability_id: Some(capability.to_string()),
                tool_call_id: Some(tool_call_id.clone()),
                succeeded: Some(*succeeded),
                approval_id: None,
                round: Some(*round),
            }),
            TraceEvent::ApprovalRequested {
                approval_id,
                capability,
                tool_call_id,
                round,
            } => events.push(CanonicalExecutionEvent {
                event: "approval_required".into(),
                tool_name: Some(capability.to_string()),
                capability_id: Some(capability.to_string()),
                tool_call_id: Some(tool_call_id.clone()),
                succeeded: None,
                approval_id: Some(approval_id.to_string()),
                round: Some(*round),
            }),
            _ => {}
        }
    }
    events
}

/// Transport-neutral pending-approval payload. Exposes identity and safe
/// metadata only; it never includes the executable frozen payload.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalPendingApproval {
    pub session: SessionId,
    pub approval_id: ApprovalId,
    pub request: String,
    pub trace_id: String,
    pub capability_id: String,
    pub tool_name: String,
    pub governance_hook: String,
    pub governance_reason: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

impl From<PendingApprovalView> for CanonicalPendingApproval {
    fn from(view: PendingApprovalView) -> Self {
        Self {
            session: view.session_id,
            approval_id: view.approval_id,
            request: view.request_id.to_string(),
            trace_id: view.trace_id.to_string(),
            capability_id: view.capability_id.to_string(),
            tool_name: view.tool_name,
            governance_hook: view.governance_hook,
            governance_reason: view.governance_reason,
            created_at: view.created_at,
            expires_at: view.expires_at,
        }
    }
}

/// Result of a canonical chat or approval-resume call.
#[derive(Debug, Clone)]
pub enum CanonicalChatOutcome {
    /// The turn completed.
    Completed(CanonicalChatResponse),
    /// The turn is paused for human approval. The `ApprovalId` is retained.
    PendingApproval(CanonicalPendingApproval),
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

/// Request to resolve one pending approval.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanonicalApprovalRequest {
    pub session: SessionId,
    pub approval: ApprovalId,
    /// `approve`, `reject`, or `cancel`.
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Invoke the canonical runtime through the real gateway entry adapter.
pub async fn execute_chat(
    runtime: &Runtime,
    request: CanonicalChatRequest,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
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

    Ok(turn_outcome_to_chat(runtime.execute_outcome(turn).await?))
}

/// Resolve a pending approval through the canonical runtime API.
pub async fn resolve_approval(
    runtime: &Runtime,
    request: CanonicalApprovalRequest,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
    let decision = parse_approval_decision(&request.decision, request.reason.clone())?;
    match runtime
        .resolve_approval(request.session, request.approval, decision)
        .await?
    {
        ApprovalResolution::Resumed(outcome) => Ok(turn_outcome_to_chat(outcome)),
        ApprovalResolution::AlreadyResolved { status } => Err(CanonicalEntryError::InvalidRequest(
            format!("approval already resolved: {status:?}"),
        )),
        ApprovalResolution::ExecutionInterrupted { approval_id } => {
            Err(CanonicalEntryError::InvalidRequest(format!(
                "approval {approval_id} was interrupted and must not be retried automatically"
            )))
        }
        ApprovalResolution::Expired => Err(CanonicalEntryError::InvalidRequest(
            "approval expired before it was resolved".into(),
        )),
        ApprovalResolution::NotFound => Err(CanonicalEntryError::InvalidRequest(
            "approval was not found for this session".into(),
        )),
    }
}

fn turn_outcome_to_chat(outcome: TurnOutcome) -> CanonicalChatOutcome {
    match outcome {
        TurnOutcome::Completed(response) => {
            CanonicalChatOutcome::Completed(CanonicalChatResponse {
                session: response.session,
                request: response.request.to_string(),
                trace_id: response.trace.trace.to_string(),
                text: response.text,
                served_by: response.served_by.to_string(),
                rounds: response.rounds,
                usage: response.usage,
                trace: response.trace.clone(),
                events: events_from_trace(&response.trace),
            })
        }
        TurnOutcome::PendingApproval(view) => {
            CanonicalChatOutcome::PendingApproval(CanonicalPendingApproval::from(view))
        }
    }
}

fn parse_approval_decision(
    decision: &str,
    reason: Option<String>,
) -> Result<ApprovalDecision, CanonicalEntryError> {
    match decision.trim().to_ascii_lowercase().as_str() {
        "approve" => Ok(ApprovalDecision::Approve),
        "reject" => Ok(ApprovalDecision::Reject { reason }),
        "cancel" => Ok(ApprovalDecision::Cancel { reason }),
        other => Err(CanonicalEntryError::InvalidRequest(format!(
            "unknown approval decision {other:?}; expected approve, reject, or cancel"
        ))),
    }
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
    #[serde(default)]
    stream: bool,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    events: Vec<CanonicalExecutionEvent>,
    /// Present on the final streaming chunk (the non-stream body carries
    /// `usage` at the top level instead).
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<NormalizedUsage>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamChunk {
    id: String,
    object: &'static str,
    created: i64,
    model: String,
    choices: Vec<OpenAiStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    apeireth: Option<OpenAiExecutionMetadata>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamChoice {
    index: u32,
    delta: OpenAiStreamDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelListItem {
    id: String,
    object: &'static str,
    created: i64,
    owned_by: String,
}

#[derive(Debug, Serialize)]
struct ModelListResponse {
    object: &'static str,
    data: Vec<ModelListItem>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorFrame,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

type HttpError = (StatusCode, Json<ErrorBody>);

/// Assemble the shared gateway state (runtime + optional panels + event bus).
pub fn build_gateway_state(
    runtime: Arc<Runtime>,
    panels: Option<Arc<dyn PanelData>>,
) -> GatewayState {
    build_gateway_state_with_services(runtime, GatewayServices::from_panel(panels))
}

/// Assemble gateway state from bounded-context ports supplied by the
/// composition root. This is the production path; the legacy `PanelData`
/// adapter above exists only for compatibility with older embedders/tests.
pub fn build_gateway_state_with_services(
    runtime: Arc<Runtime>,
    services: GatewayServices,
) -> GatewayState {
    let events = EventBus::default();
    let observations = Arc::new(crate::events::RuntimeObservationSink::new(
        services.trace_commands.clone(),
        services.audit_commands.clone(),
    ));
    runtime.set_event_sink(Arc::new(
        apeireth_runtime::canonical::CompositeRuntimeEventSink::new(vec![
            Arc::new(events.clone()),
            observations.clone(),
        ]),
    ));
    GatewayState {
        runtime,
        services,
        events,
        observations,
    }
}

/// Build the production HTTP router around one long-lived canonical runtime.
///
/// Panel routes answer `501 unsupported` while no [`PanelData`] is attached —
/// see [`canonical_router_with_panels`].
pub fn canonical_router(runtime: Arc<Runtime>) -> Router {
    canonical_router_with_services(runtime, GatewayServices::default())
}

/// Build the production router with optional panel/introspection backends.
pub fn canonical_router_with_panels(
    runtime: Arc<Runtime>,
    panels: Option<Arc<dyn PanelData>>,
) -> Router {
    canonical_router_with_state(build_gateway_state(runtime, panels))
}

/// Build the production router over explicit bounded-context gateway ports.
pub fn canonical_router_with_services(runtime: Arc<Runtime>, services: GatewayServices) -> Router {
    canonical_router_with_state(build_gateway_state_with_services(runtime, services))
}

/// Build the production router over an explicit [`GatewayState`] (lets tests
/// keep a handle on the event bus).
pub fn canonical_router_with_state(state: GatewayState) -> Router {
    Router::<GatewayState>::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/providers", get(list_providers))
        .route("/v1/runtime/snapshot", get(runtime_snapshot))
        .route("/v1/apeireth/runtime/snapshot", get(runtime_snapshot))
        .route("/v1/chat", post(native_chat))
        .route("/v1/chat/completions", post(openai_chat))
        .route("/v1/approvals", get(list_pending_approvals))
        .route("/v1/approvals/resolve", post(native_resolve_approval))
        .route("/v1/apeireth/events", get(events_handler))
        .merge(panel_routes())
        // CORS is mandatory, not optional: the desktop WebView is a distinct
        // origin (tauri://localhost / http://tauri.localhost), and browsers
        // enforce cross-origin policy even against loopback addresses. Without
        // this layer every UI fetch fails with "backend unreachable or CORS
        // refused" while curl probes keep passing — a real-world 2026-09-28
        // failure that curl-only E2E could not see.
        //
        // The gateway binds 127.0.0.1 by default, so permissive CORS does not
        // widen network exposure. Deployments that intentionally expose the
        // gateway must replace this with an explicit trusted-origin policy.
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state)
}

/// Serve the canonical gateway until the listener closes.
pub async fn serve_canonical(
    listener: tokio::net::TcpListener,
    runtime: Arc<Runtime>,
    panels: Option<Arc<dyn PanelData>>,
) -> std::io::Result<()> {
    serve_canonical_with_services(listener, runtime, GatewayServices::from_panel(panels)).await
}

/// Serve the canonical gateway over explicit bounded-context gateway ports.
pub async fn serve_canonical_with_services(
    listener: tokio::net::TcpListener,
    runtime: Arc<Runtime>,
    services: GatewayServices,
) -> std::io::Result<()> {
    let state = build_gateway_state_with_services(runtime, services);
    let endpoint = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    state.events.publish(GatewayEvent::new(
        "backend_ready",
        serde_json::json!({ "service": "apeireth-gateway-2.0", "endpoint": endpoint }),
    ));
    axum::serve(listener, canonical_router_with_state(state)).await
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "execution_owner": "apeireth-runtime::canonical"
    }))
}

async fn list_models(State(state): State<GatewayState>) -> Json<ModelListResponse> {
    let created = Timestamp::from_clock(state.runtime.clock().as_ref()).epoch_millis() / 1_000;
    // Dedupe by model id: two providers can serve the same canonical model over
    // different wires (the anthropic plugin defaults to MiniMax's
    // Anthropic-compatible gateway, so both it and the native minimax plugin
    // advertise `minimax-m3`). The UI model list needs one entry per id; model
    // resolution by the runtime is unaffected by this display layer.
    let mut seen = std::collections::HashSet::new();
    let data = state
        .runtime
        .providers()
        .model_descriptors()
        .into_iter()
        .filter(|model| seen.insert(model.id.to_string()))
        .map(|model| ModelListItem {
            id: model.id.to_string(),
            object: "model",
            created,
            owned_by: model.provider.to_string(),
        })
        .collect();
    Json(ModelListResponse {
        object: "list",
        data,
    })
}

async fn list_providers(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "object": "list",
        "data": state.runtime.snapshot().providers,
    }))
}

async fn runtime_snapshot(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    Json(serde_json::json!(state.runtime.snapshot()))
}

async fn native_chat(
    State(state): State<GatewayState>,
    Json(request): Json<CanonicalChatRequest>,
) -> Result<Response, HttpError> {
    let mut request = request;
    let session = request.session.unwrap_or_else(SessionId::new);
    request.session = Some(session);
    let outcome = execute_chat(state.runtime.as_ref(), request).await;
    state.observations.flush().await;
    let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
    publish_turn_delta(&state.events, &outcome);
    Ok(chat_http_response(outcome))
}

/// Emit the final assistant text as a transport delta. Lifecycle semantics are
/// emitted by the RuntimeEventSink; this helper does not infer them.
fn publish_turn_delta(bus: &EventBus, outcome: &CanonicalChatOutcome) {
    if let CanonicalChatOutcome::Completed(response) = outcome {
        // v1 honesty: the runtime completes a turn before the gateway can
        // encode it, so `turn_delta` carries the final text as ONE delta.
        bus.publish(GatewayEvent::new(
            "turn_delta",
            serde_json::json!({ "session": response.session, "text": response.text }),
        ));
    }
}

async fn native_resolve_approval(
    State(state): State<GatewayState>,
    Json(request): Json<CanonicalApprovalRequest>,
) -> Result<Response, HttpError> {
    let session = request.session;
    let outcome = resolve_approval(state.runtime.as_ref(), request).await;
    state.observations.flush().await;
    let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
    publish_turn_delta(&state.events, &outcome);
    Ok(chat_http_response(outcome))
}

#[derive(Debug, Deserialize)]
struct ApprovalInboxQuery {
    session: SessionId,
}

#[derive(Debug, Serialize)]
struct ApprovalInboxResponse {
    session: SessionId,
    approvals: Vec<CanonicalPendingApproval>,
}

async fn list_pending_approvals(
    State(state): State<GatewayState>,
    Query(query): Query<ApprovalInboxQuery>,
) -> Result<Json<ApprovalInboxResponse>, HttpError> {
    let approvals = state
        .runtime
        .pending_approvals(query.session)
        .await
        .map_err(|error| http_error(CanonicalEntryError::Runtime(error), Some(query.session)))?
        .into_iter()
        .map(CanonicalPendingApproval::from)
        .collect();
    Ok(Json(ApprovalInboxResponse {
        session: query.session,
        approvals,
    }))
}

fn chat_http_response(outcome: CanonicalChatOutcome) -> Response {
    match outcome {
        CanonicalChatOutcome::Completed(response) => {
            (StatusCode::OK, Json(response)).into_response()
        }
        CanonicalChatOutcome::PendingApproval(pending) => {
            (StatusCode::ACCEPTED, Json(pending)).into_response()
        }
    }
}

async fn openai_chat(
    State(state): State<GatewayState>,
    Json(request): Json<OpenAiChatRequest>,
) -> Result<Response, HttpError> {
    let is_stream = request.stream;
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
    let model_name = request.model.unwrap_or_default();

    // True incremental streaming: deltas flow from the provider through the
    // canonical loop to the client as they arrive. Non-streaming providers
    // fall back inside the runtime (one final delta), so this branch is
    // correct for every provider.
    if is_stream {
        return openai_chat_streaming(&state, native, model_name).await;
    }

    let outcome = execute_chat(state.runtime.as_ref(), native).await;
    state.observations.flush().await;
    let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
    let outcome = match outcome {
        CanonicalChatOutcome::Completed(completed) => completed,
        CanonicalChatOutcome::PendingApproval(pending) => {
            return Ok((StatusCode::ACCEPTED, Json(pending)).into_response());
        }
    };
    let created = Timestamp::from_clock(state.runtime.clock().as_ref()).epoch_millis() / 1_000;
    let events = outcome.events.clone();
    publish_turn_delta(
        &state.events,
        &CanonicalChatOutcome::Completed(outcome.clone()),
    );

    Ok(Json(OpenAiChatResponse {
        id: outcome.request.clone(),
        object: "chat.completion",
        created,
        model: model_name,
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
            events,
            usage: None,
        },
    })
    .into_response())
}

/// Run the canonical turn with incremental delta forwarding and write an
/// OpenAI-spec SSE response: role chunk → one chunk per content delta →
/// final chunk (finish_reason + apeireth metadata + usage) → `[DONE]`.
/// A pending approval or a runtime error also terminates the stream with an
/// explicit final frame, so the client never hangs.
async fn openai_chat_streaming(
    state: &GatewayState,
    native: CanonicalChatRequest,
    model_name: String,
) -> Result<Response, HttpError> {
    let session = native
        .session
        .expect("canonical chat request always carries a session");
    let stream_id = RequestId::new().to_string();
    let created = Timestamp::from_clock(state.runtime.clock().as_ref()).epoch_millis() / 1_000;

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(64);
    let delta_tx = tx.clone();
    let delta_stream_id = stream_id.clone();
    let delta_model = model_name.clone();
    let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |delta: String| {
        // Wrap each delta in its own OpenAI-spec SSE chunk frame; the channel
        // carries only complete, pre-serialized frames.
        let chunk = OpenAiStreamChunk {
            id: delta_stream_id.clone(),
            object: "chat.completion.chunk",
            created,
            model: delta_model.clone(),
            choices: vec![OpenAiStreamChoice {
                index: 0,
                delta: OpenAiStreamDelta {
                    role: None,
                    content: Some(delta),
                },
                finish_reason: None,
            }],
            apeireth: None,
        };
        if let Ok(json) = serde_json::to_string(&chunk) {
            // try_send: a vanished reader must never block the canonical loop.
            let _ = delta_tx.try_send(format!("data: {json}\n\n"));
        }
    });

    let runtime = Arc::clone(&state.runtime);
    let observations = Arc::clone(&state.observations);
    let events = state.events.clone();
    let mut turn = TurnRequest::new(session, native.input);
    if let Some(model) = native.model {
        turn = turn.with_model(model);
    }
    if let Some(system) = native.system {
        turn = turn.with_system(system);
    }
    tokio::spawn(async move {
        let role_chunk = OpenAiStreamChunk {
            id: stream_id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model_name.clone(),
            choices: vec![OpenAiStreamChoice {
                index: 0,
                delta: OpenAiStreamDelta {
                    role: Some("assistant"),
                    content: None,
                },
                finish_reason: None,
            }],
            apeireth: None,
        };
        if let Ok(json) = serde_json::to_string(&role_chunk) {
            let _ = tx.send(format!("data: {json}\n\n")).await;
        }

        let outcome = runtime.execute_outcome_streaming(turn, sink).await;
        observations.flush().await;

        match outcome {
            Ok(TurnOutcome::Completed(response)) => {
                let chat_outcome = turn_outcome_to_chat(TurnOutcome::Completed(response.clone()));
                publish_turn_delta(&events, &chat_outcome);
                let final_chunk = OpenAiStreamChunk {
                    id: stream_id.clone(),
                    object: "chat.completion.chunk",
                    created,
                    model: model_name.clone(),
                    choices: vec![OpenAiStreamChoice {
                        index: 0,
                        delta: OpenAiStreamDelta {
                            role: None,
                            content: None,
                        },
                        finish_reason: Some("stop"),
                    }],
                    apeireth: Some(OpenAiExecutionMetadata {
                        session_id: response.session.to_string(),
                        trace_id: response.trace.trace.to_string(),
                        served_by: response.served_by.to_string(),
                        rounds: response.rounds,
                        events: events_from_trace(&response.trace),
                        usage: Some(response.usage),
                    }),
                };
                if let Ok(json) = serde_json::to_string(&final_chunk) {
                    let _ = tx.send(format!("data: {json}\n\ndata: [DONE]\n\n")).await;
                }
            }
            Ok(TurnOutcome::PendingApproval(view)) => {
                let pending = CanonicalPendingApproval::from(view);
                let frame = serde_json::json!({
                    "id": stream_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model_name,
                    "choices": [{"index": 0, "delta": {}, "finish_reason": "approval_required"}],
                    "apeireth": { "pending_approval": pending },
                });
                if let Ok(json) = serde_json::to_string(&frame) {
                    let _ = tx.send(format!("data: {json}\n\ndata: [DONE]\n\n")).await;
                }
            }
            Err(error) => {
                let (_, code) = classify_runtime_error(&error);
                let frame = serde_json::json!({
                    "error": ErrorFrame::new(code, error.to_string()),
                    "session_id": session.to_string(),
                });
                if let Ok(json) = serde_json::to_string(&frame) {
                    let _ = tx.send(format!("data: {json}\n\ndata: [DONE]\n\n")).await;
                }
            }
        }
    });

    let stream =
        tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok::<String, std::convert::Infallible>);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .body(axum::body::Body::from_stream(stream))
        .map_err(|e| {
            http_error(
                CanonicalEntryError::InvalidRequest(e.to_string()),
                Some(session),
            )
        })?)
}

fn http_error(error: CanonicalEntryError, session: Option<SessionId>) -> HttpError {
    let (status, code) = classify_entry_error(&error);
    (
        status,
        Json(ErrorBody {
            error: ErrorFrame::new(code, error.to_string()),
            session_id: session.map(|id| id.to_string()),
        }),
    )
}

fn classify_entry_error(error: &CanonicalEntryError) -> (StatusCode, ErrorCode) {
    match error {
        CanonicalEntryError::InvalidRequest(_) => {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest)
        }
        CanonicalEntryError::Runtime(runtime) => classify_runtime_error(runtime),
    }
}

fn classify_runtime_error(error: &RuntimeError) -> (StatusCode, ErrorCode) {
    match error {
        RuntimeError::Provider(provider) => (
            StatusCode::BAD_GATEWAY,
            provider_code(&provider.to_string()),
        ),
        RuntimeError::ProvidersExhausted { source, .. } => (
            StatusCode::BAD_GATEWAY,
            provider_code(&source.to_string()),
        ),
        RuntimeError::NoProvider { .. } | RuntimeError::NoHealthyProvider { .. } => {
            (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::ProviderUnreachable)
        }
        RuntimeError::Misconfigured(_) => (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::Internal),
        RuntimeError::Denied { .. } => (StatusCode::FORBIDDEN, ErrorCode::InvalidRequest),
        RuntimeError::ApprovalRequired { .. } | RuntimeError::SessionApprovalPending { .. } => {
            (StatusCode::CONFLICT, ErrorCode::InvalidRequest)
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal),
    }
}

/// Classify a provider failure from its human-readable rendering.
///
/// The gateway crate has no regular dependency on `apeireth_plugin` (where
/// `ProviderError` is declared), so it cannot destructure the error type. It
/// reads the stable `Display` wording instead and preserves the full text in
/// the frame `message`.
fn provider_code(text: &str) -> ErrorCode {
    let lower = text.to_ascii_lowercase();
    if lower.contains("rate limited") {
        ErrorCode::RateLimited
    } else if lower.contains("timed out") || lower.contains("network error") {
        ErrorCode::ProviderUnreachable
    } else if lower.contains("authentication failed") {
        if lower.contains("missing") {
            ErrorCode::AuthMissingKey
        } else {
            ErrorCode::AuthInvalidKey
        }
    } else {
        ErrorCode::ProviderError
    }
}
