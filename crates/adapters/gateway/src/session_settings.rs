//! Session-scoped settings surface.
//!
//! `GET /v1/sessions/{session_id}/settings` reads the durable session settings;
//! `PATCH /v1/sessions/{session_id}/settings` applies a partial update.
//!
//! These endpoints go through the runtime's own [`SessionManager`], so they act
//! on exactly the same durable session the agent loop reads at the start of a
//! turn. A `model` of `null` in a PATCH resets the session back to the global
//! default model.

use std::str::FromStr;

use apeireth_core::kernel::SessionId;
use apeireth_runtime::canonical::{PermissionPreset, SessionSettings};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::panels::GatewayState;

/// Full session-settings projection returned by both endpoints.
#[derive(Debug, Serialize)]
pub struct SessionSettingsBody {
    pub session_id: String,
    pub model: Option<String>,
    pub permission_preset: PermissionPreset,
}

impl SessionSettingsBody {
    fn from_session(session_id: SessionId, settings: &SessionSettings) -> Self {
        Self {
            session_id: session_id.to_string(),
            model: settings.model.clone(),
            permission_preset: settings.permission_preset,
        }
    }
}

/// Partial PATCH body. `model: null` resets the session to the global default;
/// a missing field leaves it unchanged.
#[derive(Debug, Deserialize)]
pub struct PatchSessionSettingsRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub model: Option<Option<String>>,
    #[serde(default)]
    pub permission_preset: Option<PermissionPreset>,
}

/// Deserialize a present-but-null field as `Some(None)` so `Option<Option<T>>`
/// can distinguish "field missing" from "field explicitly reset to null".
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

/// Unified error contract for this surface.
#[derive(Debug, Serialize)]
pub(crate) struct ErrorDetail {
    pub(crate) message: String,
    pub(crate) code: String,
    pub(crate) solution: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorBody {
    pub(crate) error: ErrorDetail,
}

type SettingsError = (StatusCode, Json<ErrorBody>);

fn session_not_found(session_id: &str) -> SettingsError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: ErrorDetail {
                message: format!("session {session_id} does not exist"),
                code: "session_not_found".into(),
                solution: "create the session first or check the session id".into(),
            },
        }),
    )
}

fn invalid_request(message: String) -> SettingsError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: ErrorDetail {
                message,
                code: "invalid_request".into(),
                solution: "check the request body or session id and retry".into(),
            },
        }),
    )
}

pub(crate) async fn get_session_settings(
    State(state): State<GatewayState>,
    Path(session_id): Path<String>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    let loaded = state
        .runtime
        .sessions()
        .load(&session_id)
        .await
        .map_err(|error| invalid_request(error.to_string()))?;
    let Some(session) = loaded else {
        return Err(session_not_found(&session_id.to_string()));
    };

    Ok(Json(SessionSettingsBody::from_session(
        session_id,
        &session.settings,
    ))
    .into_response())
}

pub(crate) async fn patch_session_settings(
    State(state): State<GatewayState>,
    Path(session_id): Path<String>,
    Json(request): Json<PatchSessionSettingsRequest>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    let loaded = state
        .runtime
        .sessions()
        .load(&session_id)
        .await
        .map_err(|error| invalid_request(error.to_string()))?;
    let Some(mut session) = loaded else {
        return Err(session_not_found(&session_id.to_string()));
    };

    if let Some(model) = request.model {
        session.settings.model = model;
    }
    if let Some(preset) = request.permission_preset {
        session.settings.permission_preset = preset;
    }

    state
        .runtime
        .sessions()
        .save(&session)
        .await
        .map_err(|error| invalid_request(error.to_string()))?;

    Ok(Json(SessionSettingsBody::from_session(
        session_id,
        &session.settings,
    ))
    .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::system_clock;
    use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, Session, SessionStore};
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::{get, patch};
    use axum::Router;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_router(store: Arc<dyn SessionStore>) -> Router {
        let runtime = Runtime::builder()
            .with_session_store(store)
            .build()
            .await
            .unwrap();
        let state = GatewayState {
            runtime: Arc::new(runtime),
            services: crate::panels::GatewayServices::default(),
            events: crate::events::EventBus::default(),
            observations: Arc::new(crate::events::RuntimeObservationSink::new(None, None)),
            hot_config: Arc::new(std::sync::RwLock::new(
                crate::admin::GatewayRuntimeConfig::from_env(),
            )),
        };
        Router::<GatewayState>::new()
            .route(
                "/v1/sessions/:session_id/settings",
                get(get_session_settings).patch(patch_session_settings),
            )
            .with_state(state)
    }

    async fn save_session(store: &Arc<dyn SessionStore>, id: SessionId) {
        let clock = system_clock();
        store
            .save(&Session::new(id, clock.as_ref()))
            .await
            .unwrap();
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn get_returns_default_settings_for_an_existing_session() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["session_id"], sid.to_string());
        assert_eq!(body["model"], serde_json::Value::Null);
        assert_eq!(body["permission_preset"], "standard");
    }

    #[tokio::test]
    async fn patch_updates_settings_and_returns_full_projection() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"anthropic/claude-3-5","permission_preset":"full"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["model"], "anthropic/claude-3-5");
        assert_eq!(body["permission_preset"], "full");
    }

    #[tokio::test]
    async fn patch_model_null_resets_to_global_default() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store.clone()).await;

        // Set a model first.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"some/model"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then reset it with an explicit null.
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":null}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["model"], serde_json::Value::Null);
        assert_eq!(body["permission_preset"], "standard");
    }

    #[tokio::test]
    async fn missing_session_returns_session_not_found_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;
        let sid = SessionId::new();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "session_not_found");
        assert!(body["error"]["message"].as_str().unwrap().contains(&sid.to_string()));
        assert!(body["error"]["solution"].is_string());
    }

    #[tokio::test]
    async fn malformed_session_id_returns_invalid_request_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/sessions/not-a-uuid/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "invalid_request");
    }
}
