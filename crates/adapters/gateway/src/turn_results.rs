//! Read-only recovery of a precisely identified turn from its durable session log.
//! No execution, transcript rewriting, latest-turn fallback or inferred usage.

use std::collections::HashSet;

use apeireth_core::kernel::{RequestId, SessionId, TraceId};
use apeireth_governance::UserRole;
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::canonical::{LogEntry, Session, SessionEventKind};
use axum::extract::{rejection::QueryRejection, Extension, Path, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client_tokens::UserContext;
use crate::domain_context::{request_scope, DomainContext};
use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::panels::GatewayState;

const ENDPOINT: &str = "/v1/sessions/:session_id/turn-result";

pub(crate) fn routes() -> axum::Router<GatewayState> {
    axum::Router::new()
        .route(
            "/v1/sessions/:session_id/turns",
            axum::routing::get(history::get_session_turns),
        )
        .route(
            "/v1/sessions/:session_id/turn-result",
            axum::routing::get(get_turn_result),
        )
        .route(
            provider_context::ENDPOINT,
            axum::routing::get(provider_context::read),
        )
}

mod history;
mod provider_context;
pub(crate) use history::get_session_turns;

/// Outside the token gate so refusal frames cannot become cached results either.
pub(crate) async fn protect_read_response(request: Request, next: Next) -> Response {
    let sensitive = request
        .uri()
        .path()
        .strip_prefix("/v1/sessions/")
        .is_some_and(|path| {
            path.ends_with("/turn-result")
                || path.ends_with("/turns")
                || path.ends_with("/provider-context")
        });
    let mut response = next.run(request).await;
    if sensitive {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TurnResultQuery {
    input_sha256: String,
    request: Option<String>,
    trace_id: Option<String>,
}

struct Selector {
    digest: String,
    request: Option<RequestId>,
    trace: Option<TraceId>,
}

impl TryFrom<TurnResultQuery> for Selector {
    type Error = ();

    fn try_from(query: TurnResultQuery) -> Result<Self, Self::Error> {
        if query.input_sha256.len() != 64
            || !query
                .input_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(());
        }
        Ok(Self {
            digest: query.input_sha256,
            request: query
                .request
                .map(|id| id.parse())
                .transpose()
                .map_err(|_| ())?,
            trace: query
                .trace_id
                .map(|id| id.parse())
                .transpose()
                .map_err(|_| ())?,
        })
    }
}

#[derive(Debug, Serialize)]
struct TurnResult {
    schema: u8,
    source: &'static str,
    session: SessionId,
    input_sha256: String,
    state: &'static str,
    request: RequestId,
    trace_id: TraceId,
    text: Option<String>,
    usage: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recorded_at: Option<i64>,
}

#[derive(Debug, PartialEq)]
enum ProjectionError {
    NotFound,
    Unverifiable,
    Conflict,
}

fn failure(status: StatusCode, code: ErrorCode, message: &'static str) -> Response {
    ErrorFrame::response(status, code, message)
}

fn not_found() -> Response {
    failure(
        StatusCode::NOT_FOUND,
        ErrorCode::SessionNotFound,
        "no matching turn result",
    )
}

pub(crate) async fn get_turn_result(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    user: Option<Extension<UserContext>>,
    Path(session): Path<String>,
    query: Result<Query<TurnResultQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let session = authorized_session_id(&state, user.as_ref().map(|u| &u.0), &session)?;
    let selector = query
        .ok()
        .and_then(|Query(q)| Selector::try_from(q).ok())
        .ok_or_else(|| {
            failure(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "invalid turn result selector",
            )
        })?;
    let record =
        load_scoped_session(&state, domain.as_ref().map(|d| &d.0), session, ENDPOINT).await?;
    match project(&record, &selector) {
        Ok(body) => Ok(Json(body).into_response()),
        Err(ProjectionError::NotFound | ProjectionError::Unverifiable) => Err(not_found()),
        Err(ProjectionError::Conflict) => Err(failure(
            StatusCode::CONFLICT,
            ErrorCode::InvalidRequest,
            "turn result selector is ambiguous or conflicts with recorded identity",
        )),
    }
}

fn authorized_session_id(
    state: &GatewayState,
    user: Option<&UserContext>,
    session: &str,
) -> Result<SessionId, Response> {
    if state.user_access.is_active() && user.as_ref().and_then(|u| u.user_id()).is_none() {
        return Err(failure(
            StatusCode::FORBIDDEN,
            ErrorCode::PermissionDenied,
            "turn result recovery requires a bound user",
        ));
    }
    let session = session.parse::<SessionId>().map_err(|_| {
        failure(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "invalid session identity",
        )
    })?;
    if state.user_access.is_active() {
        let user = user.expect("bound user checked above");
        if user.role() != Some(UserRole::Owner)
            && state.user_access.session_user(session).as_deref() != user.user_id()
        {
            return Err(not_found());
        }
    }
    Ok(session)
}

async fn load_scoped_session(
    state: &GatewayState,
    domain: Option<&DomainContext>,
    session: SessionId,
    endpoint: &'static str,
) -> Result<Session, Response> {
    let scope = request_scope(state, domain, endpoint)?;
    let manager = scope
        .manager()
        .map(|m| m.as_ref())
        .unwrap_or(state.runtime.sessions());
    // A snapshot read does not take the write permit of an executing turn.
    manager
        .load(&session)
        .await
        .map_err(|_| {
            failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                "session record could not be read",
            )
        })?
        .ok_or_else(not_found)
}

fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn plain_text(entry: &LogEntry, role: MessageRole) -> Option<&str> {
    let SessionEventKind::MessageAppended { message, .. } = &entry.event else {
        return None;
    };
    if message.role != role
        || !message.tool_calls.is_empty()
        || message.tool_call_id.is_some()
        || message.name.is_some()
    {
        return None;
    }
    match message.content.as_slice() {
        [ContentPart::Text { text }] => Some(text),
        _ => None,
    }
}

fn validate(record: &Session) -> Result<usize, ProjectionError> {
    let prefix = record.inherited_event_count.unwrap_or(0);
    if prefix > record.log.len() {
        return Err(ProjectionError::Unverifiable);
    }
    let mut messages = 0;
    let mut events = record.events.iter();
    let mut requests = HashSet::new();
    let mut traces = HashSet::new();
    for entry in &record.log {
        if let SessionEventKind::MessageAppended { seq, message } = &entry.event {
            if *seq != messages
                || record.messages.get(*seq) != Some(message)
                || entry.request.is_some()
                || entry.trace.is_some()
            {
                return Err(ProjectionError::Unverifiable);
            }
            messages += 1;
        }
        match (entry.request, entry.trace) {
            (Some(request), Some(trace)) => {
                let fact = events.next().ok_or(ProjectionError::Unverifiable)?;
                if fact.request != request
                    || fact.trace != trace
                    || fact.at != entry.at
                    || fact.event != entry.event
                {
                    return Err(ProjectionError::Unverifiable);
                }
                if matches!(entry.event, SessionEventKind::TurnStarted)
                    && (!requests.insert(request) || !traces.insert(trace))
                {
                    return Err(ProjectionError::Unverifiable);
                }
            }
            (None, None)
                if !matches!(
                    entry.event,
                    SessionEventKind::TurnStarted | SessionEventKind::TurnCompleted { .. }
                ) => {}
            _ => return Err(ProjectionError::Unverifiable),
        }
    }
    if messages != record.messages.len() || events.next().is_some() {
        return Err(ProjectionError::Unverifiable);
    }
    committed::validate_markers(record, prefix)?;
    Ok(prefix)
}

fn project(record: &Session, selector: &Selector) -> Result<TurnResult, ProjectionError> {
    let prefix = validate(record)?;
    let starts: Vec<_> = record
        .log
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.event, SessionEventKind::TurnStarted))
        .collect();
    let mut candidates = Vec::new();
    for (position, (index, start)) in starts.iter().enumerate() {
        if *index <= prefix {
            continue;
        }
        let Some(input) = committed::input_text(record, *index) else {
            continue;
        };
        let request = start.request.ok_or(ProjectionError::Unverifiable)?;
        let trace = start.trace.ok_or(ProjectionError::Unverifiable)?;
        let input_digest = digest(input);
        if selector.request == Some(request) && input_digest != selector.digest {
            return Err(ProjectionError::Conflict);
        }
        if input_digest == selector.digest {
            let end = starts.get(position + 1).map_or(record.log.len(), |(i, _)| {
                committed::end_before_next_start(record, *i)
            });
            candidates.push((*index, end, request, trace));
        }
    }
    if candidates.is_empty() {
        return Err(ProjectionError::NotFound);
    }
    if candidates.len() > 1 && selector.request.is_none() {
        return Err(ProjectionError::Conflict);
    }
    if let Some(request) = selector.request {
        candidates.retain(|(_, _, id, _)| *id == request);
    }
    if candidates.len() != 1 {
        return Err(ProjectionError::Conflict);
    }
    let (start, end, request, trace) = candidates[0];
    if selector.trace.is_some_and(|id| id != trace) {
        return Err(ProjectionError::Conflict);
    }
    let mut result = TurnResult {
        schema: 1,
        source: "session_log",
        session: record.id,
        input_sha256: selector.digest.clone(),
        state: "unconfirmed",
        request,
        trace_id: trace,
        text: None,
        usage: None,
        recorded_at: None,
    };
    if let Some((text, at)) = completion(record, start, end, request, trace)? {
        result.state = "completed";
        result.text = Some(text.to_string());
        result.recorded_at = Some(at);
    }
    Ok(result)
}

/// Only the original assistant bound by a verified commit marker, or the strict
/// adjacent legacy message, can supply a matching durable completion.
fn completion(
    record: &Session,
    start: usize,
    end: usize,
    request: RequestId,
    trace: TraceId,
) -> Result<Option<(&str, i64)>, ProjectionError> {
    let mut completed = record.log[start + 1..end]
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.event, SessionEventKind::TurnCompleted { .. }));
    let first = completed.next();
    if completed.next().is_some() {
        return Err(ProjectionError::Conflict);
    }
    if let Some((offset, completed)) = first {
        if completed.request != Some(request)
            || completed.trace != Some(trace)
            || record.log[start + 1..end].iter().any(|e| {
                e.request.is_some_and(|id| id != request) || e.trace.is_some_and(|id| id != trace)
            })
        {
            return Err(ProjectionError::Unverifiable);
        }
        let text = committed::response_text(record, start + 1 + offset)
            .ok_or(ProjectionError::Unverifiable)?;
        return Ok(Some((text, completed.at.epoch_millis())));
    }
    Ok(None)
}

mod committed;
#[cfg(test)]
mod http_tests;
#[cfg(test)]
mod tests;
