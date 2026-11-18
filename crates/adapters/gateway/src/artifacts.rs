//! Managed text transport over the same version service used by governed tools.
//! This module owns no files, version store, actor registry or edit engine.

use std::sync::Arc;

use apeireth_protocol::artifacts::{
    valid_artifact_uuid, ArtifactAccess, ArtifactError, ArtifactHistoryQuery, ArtifactListQuery,
    ArtifactReceipt, ArtifactService, CommitArtifactRequest, CreateArtifactRequest,
    RestoreArtifactRequest, MAX_ARTIFACT_CONTENT_BYTES, MAX_ARTIFACT_PAGE_LIMIT,
    MAX_ARTIFACT_TITLE_BYTES, MAX_ARTIFACT_WORK_BYTES,
};
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Extension, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::domain_context::DomainContext;
use crate::error_frame::{ErrorCode, ErrorEnvelope, ErrorFrame};
use crate::panels::GatewayState;

const JSON_BODY_LIMIT: usize = 2 * 1024 * 1024;

pub(crate) fn routes() -> Router<GatewayState> {
    Router::new()
        .route("/v1/artifacts/capabilities", get(capabilities))
        .route("/v1/artifacts", get(list).post(create))
        .route("/v1/artifacts/operations/:operation_id", get(operation))
        .route("/v1/artifacts/:artifact_id", get(read_head))
        .route(
            "/v1/artifacts/:artifact_id/revisions",
            get(history).post(commit),
        )
        .route(
            "/v1/artifacts/:artifact_id/revisions/:revision_id",
            get(read_revision),
        )
        .route(
            "/v1/artifacts/:artifact_id/revisions/:revision_id/export",
            get(export),
        )
        .route("/v1/artifacts/:artifact_id/restore", post(restore))
        // A 256 KiB UTF-8 string can expand sixfold in JSON. Bound the
        // transport separately from the service's actual content-byte limit.
        .layer(DefaultBodyLimit::max(JSON_BODY_LIMIT))
}

/// Compose the panel surface with the artifact routes.
pub(crate) fn routes_with_panels() -> Router<GatewayState> {
    crate::panels::panel_routes().merge(routes())
}

/// Kept outside authentication so even early refusals cannot be cached.
pub(crate) async fn protect_response(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let sensitive = path == "/v1/artifacts" || path.starts_with("/v1/artifacts/");
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
struct EmptyQuery {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreQuery {
    store_id: String,
}

fn failure(status: StatusCode, kind: &'static str, message: &'static str) -> Response {
    (
        status,
        Json(ErrorEnvelope {
            error: ErrorFrame::new(ErrorCode::InvalidRequest, message)
                .with_details(json!({"artifact_error": kind})),
        }),
    )
        .into_response()
}

fn invalid() -> Response {
    failure(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "invalid managed artifact request",
    )
}

fn error(error: ArtifactError) -> Response {
    let (status, kind, message) = match error {
        ArtifactError::Invalid(_) => return invalid(),
        ArtifactError::StoreMismatch => (
            StatusCode::CONFLICT,
            "store_mismatch",
            "managed artifact store identity changed",
        ),
        ArtifactError::NotFound => (
            StatusCode::NOT_FOUND,
            "not_found",
            "managed artifact resource not found",
        ),
        ArtifactError::ScopeUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "scope_unavailable",
            "managed artifacts require the single-operator scope",
        ),
        ArtifactError::Capacity => (
            StatusCode::SERVICE_UNAVAILABLE,
            "capacity",
            "managed artifact capacity reached",
        ),
        ArtifactError::OperationMismatch => (
            StatusCode::CONFLICT,
            "operation_mismatch",
            "operation identity is bound to a different managed artifact request",
        ),
        ArtifactError::Storage(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage",
            "managed artifact storage is unavailable",
        ),
    };
    failure(status, kind, message)
}

fn service(
    state: &GatewayState,
    domain: Option<&Extension<DomainContext>>,
) -> Result<Arc<dyn ArtifactService>, Response> {
    // The first service has one explicitly configured local namespace. Never
    // silently project an authenticated multi-user/domain request into it.
    if state.user_access.is_active() || domain.is_some_and(|d| d.is_non_default()) {
        return Err(error(ArtifactError::ScopeUnavailable));
    }
    state.services.artifacts.clone().ok_or_else(|| {
        failure(
            StatusCode::NOT_IMPLEMENTED,
            "unsupported",
            "managed artifact service is not configured",
        )
    })
}

fn query<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, Response> {
    query.map(|Query(value)| value).map_err(|_| invalid())
}

fn path<T>(path: Result<Path<T>, PathRejection>) -> Result<T, Response> {
    path.map(|Path(value)| value).map_err(|_| invalid())
}

fn body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    body.map(|Json(value)| value).map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            failure(
                StatusCode::PAYLOAD_TOO_LARGE,
                "invalid_request",
                "managed artifact JSON body exceeds the transport limit",
            )
        } else {
            invalid()
        }
    })
}

fn uuid(value: &str) -> Result<(), Response> {
    valid_artifact_uuid(value).then_some(()).ok_or_else(invalid)
}

fn work(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ARTIFACT_WORK_BYTES
        && !value.chars().any(char::is_control)
}

fn limit(value: Option<u8>) -> Result<(), Response> {
    value
        .is_none_or(|limit| (1..=MAX_ARTIFACT_PAGE_LIMIT).contains(&limit))
        .then_some(())
        .ok_or_else(invalid)
}

fn receipt(receipt: ArtifactReceipt) -> Response {
    match receipt.outcome.as_str() {
        "committed" => Json(receipt).into_response(),
        "conflict" => (
            StatusCode::CONFLICT,
            Json(ErrorEnvelope {
                error: ErrorFrame::new(
                    ErrorCode::InvalidRequest,
                    "managed artifact head no longer matches the expected version",
                )
                .with_details(json!({"artifact_error":"head_conflict","receipt":receipt})),
            }),
        )
            .into_response(),
        _ => error(ArtifactError::Storage("invalid receipt outcome".into())),
    }
}

async fn capabilities(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    input: Result<Query<EmptyQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    query(input)?;
    service
        .capabilities(ArtifactAccess::local_human())
        .await
        .map(|value| Json(value).into_response())
        .map_err(error)
}

async fn list(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    input: Result<Query<ArtifactListQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let input = query(input)?;
    uuid(&input.store_id)?;
    if !work(&input.work_id) {
        return Err(invalid());
    }
    if let Some(after) = &input.after {
        uuid(after)?;
    }
    limit(input.limit)?;
    service
        .list(ArtifactAccess::local_human(), input)
        .await
        .map(|value| Json(value).into_response())
        .map_err(error)
}

async fn read_head(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<String>, PathRejection>,
    input: Result<Query<StoreQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let artifact = path(resource)?;
    let input = query(input)?;
    uuid(&artifact)?;
    uuid(&input.store_id)?;
    service
        .read(
            ArtifactAccess::local_human(),
            input.store_id,
            artifact,
            None,
        )
        .await
        .map(|value| Json(value).into_response())
        .map_err(error)
}

async fn read_revision(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<(String, String)>, PathRejection>,
    input: Result<Query<StoreQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let (artifact, revision) = path(resource)?;
    let input = query(input)?;
    uuid(&artifact)?;
    uuid(&revision)?;
    uuid(&input.store_id)?;
    service
        .read(
            ArtifactAccess::local_human(),
            input.store_id,
            artifact,
            Some(revision),
        )
        .await
        .map(|value| Json(value).into_response())
        .map_err(error)
}

async fn history(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<String>, PathRejection>,
    input: Result<Query<ArtifactHistoryQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let artifact = path(resource)?;
    let input = query(input)?;
    uuid(&artifact)?;
    uuid(&input.store_id)?;
    limit(input.limit)?;
    service
        .history(ArtifactAccess::local_human(), artifact, input)
        .await
        .map(|value| Json(value).into_response())
        .map_err(error)
}

async fn operation(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<String>, PathRejection>,
    input: Result<Query<StoreQuery>, QueryRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let operation = path(resource)?;
    let input = query(input)?;
    uuid(&operation)?;
    uuid(&input.store_id)?;
    // A previously persisted conflict is a successful lookup, not a new CAS.
    service
        .operation(ArtifactAccess::local_human(), input.store_id, operation)
        .await
        .map(|value| Json(value).into_response())
        .map_err(error)
}

async fn create(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    input: Result<Json<CreateArtifactRequest>, JsonRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let input = body(input)?;
    uuid(&input.store_id)?;
    uuid(&input.operation_id)?;
    if let Some(session) = &input.conversation_session_id {
        uuid(session)?;
    }
    if !work(&input.work_id)
        || input.title.trim().is_empty()
        || input.title.len() > MAX_ARTIFACT_TITLE_BYTES
        || !matches!(input.media_type.as_str(), "text/plain" | "text/markdown")
        || input.content.len() > MAX_ARTIFACT_CONTENT_BYTES
    {
        return Err(invalid());
    }
    service
        .create(ArtifactAccess::local_human(), input)
        .await
        .map(receipt)
        .map_err(error)
}

async fn commit(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<String>, PathRejection>,
    input: Result<Json<CommitArtifactRequest>, JsonRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let artifact = path(resource)?;
    let input = body(input)?;
    for value in [
        &artifact,
        &input.store_id,
        &input.operation_id,
        &input.expected_head,
    ] {
        uuid(value)?;
    }
    if input.content.len() > MAX_ARTIFACT_CONTENT_BYTES {
        return Err(invalid());
    }
    service
        .commit(ArtifactAccess::local_human(), artifact, input)
        .await
        .map(receipt)
        .map_err(error)
}

async fn restore(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<String>, PathRejection>,
    input: Result<Json<RestoreArtifactRequest>, JsonRejection>,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let artifact = path(resource)?;
    let input = body(input)?;
    for value in [
        &artifact,
        &input.store_id,
        &input.operation_id,
        &input.expected_head,
        &input.revision_id,
    ] {
        uuid(value)?;
    }
    service
        .restore(ArtifactAccess::local_human(), artifact, input)
        .await
        .map(receipt)
        .map_err(error)
}

async fn export(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    resource: Result<Path<(String, String)>, PathRejection>,
    input: Result<Query<StoreQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    let service = service(&state, domain.as_ref())?;
    let (artifact, revision) = path(resource)?;
    let input = query(input)?;
    for value in [&artifact, &revision, &input.store_id] {
        uuid(value)?;
    }
    if headers.contains_key(header::RANGE) || headers.contains_key(header::IF_RANGE) {
        return Err(invalid());
    }
    let document = service
        .read(
            ArtifactAccess::local_human(),
            input.store_id,
            artifact,
            Some(revision),
        )
        .await
        .map_err(error)?;
    let (content_type, extension) = match document.artifact.media_type.as_str() {
        "text/plain" => ("text/plain; charset=utf-8", "txt"),
        "text/markdown" => ("text/markdown; charset=utf-8", "md"),
        _ => return Err(error(ArtifactError::Storage("invalid media type".into()))),
    };
    let filename = format!(
        "attachment; filename=\"artifact-{}.{}\"",
        document.artifact.artifact_id, extension
    );
    let disposition = HeaderValue::from_str(&filename)
        .map_err(|_| error(ArtifactError::Storage("invalid export identity".into())))?;
    let entity_tag = HeaderValue::from_str(&format!("\"{}\"", document.revision.content_sha256))
        .map_err(|_| error(ArtifactError::Storage("invalid export digest".into())))?;
    let mut response = document.revision.content.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, disposition);
    response.headers_mut().insert(header::ETAG, entity_tag);
    Ok(response)
}

#[cfg(test)]
mod tests;
