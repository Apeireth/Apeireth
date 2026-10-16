//! Session-scoped settings surface.
//!
//! `GET /v1/sessions/{session_id}/settings` reads the durable session settings;
//! `PATCH /v1/sessions/{session_id}/settings` applies a partial update;
//! `DELETE /v1/sessions/{session_id}` removes the stored session record.
//!
//! These endpoints go through the runtime's own [`SessionManager`], so they act
//! on exactly the same durable session the agent loop reads at the start of a
//! turn. A `model` of `null` in a PATCH resets the session back to the global
//! default model.
//!
//! W2-③ 并发写语义: PATCH / DELETE 是会话账本写者, 与在途回合走**同一把会话
//! 写闸** (先到先服务排队) —— 读-改-写全程持证, 与回合写零交叠 (设置补丁不会
//! 覆写回合刚追加的消息, 回合也不会吃掉设置改动); 每次写落
//! `gateway.session_client` 审计行 (谁改了哪个会话)。

use std::str::FromStr;

use apeireth_core::kernel::SessionId;
use apeireth_core::user_domain::UserDomainError;
use apeireth_runtime::canonical::{PermissionPreset, SessionManager, SessionSettings};
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{identity_label, record_session_client, ClientIdentity};
use crate::domain_context::DomainContext;
use crate::panels::GatewayState;

/// Full session-settings projection returned by both endpoints.
#[derive(Debug, Serialize)]
pub struct SessionSettingsBody {
    pub session_id: String,
    pub model: Option<String>,
    pub permission_preset: PermissionPreset,
    pub approval_remember: bool,
}

impl SessionSettingsBody {
    fn from_session(session_id: SessionId, settings: &SessionSettings) -> Self {
        Self {
            session_id: session_id.to_string(),
            model: settings.model.clone(),
            permission_preset: settings.permission_preset,
            approval_remember: settings.approval_remember,
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
    #[serde(default)]
    pub approval_remember: Option<bool>,
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

fn delete_failed(message: String) -> SettingsError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: ErrorDetail {
                message,
                code: "session_delete_failed".into(),
                solution: "retry; if it persists check the gateway storage log".into(),
            },
        }),
    )
}

/// W3-①: 本域拒开帧 (403 `domain_refused`) —— 真因如实 (本域信息, 不涉他域)。
fn domain_refused(error: &UserDomainError) -> SettingsError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            error: ErrorDetail {
                message: format!("the request's user domain is not open: {}", error.reason()),
                code: "domain_refused".into(),
                solution: "check users/<id>/domain.json (bad domain config refuses to open)".into(),
            },
        }),
    )
}

/// W3-①: 按请求域上下文取会话写闸 (**每域恰一把**)。
///
/// - 域面未接线 / 无域上下文 = 兼容路径: 运行时那把 (默认域原位语义);
/// - 已接线 = 域平面按域路由; 默认域与运行时**同一把** (W2-③ 同会话写串行
///   承诺不拆);
/// - 资源级跨域不需要专门拒帧: 本域写闸查不到他域会话, 一律走
///   [`session_not_found`] —— 与真不存在**同帧** (存在性不泄露)。
fn session_manager<'s>(
    state: &'s GatewayState,
    domain: Option<&Extension<DomainContext>>,
) -> Result<&'s SessionManager, SettingsError> {
    let Some(Extension(context)) = domain else {
        return Ok(state.runtime.sessions());
    };
    let Some(plane) = state.domains.as_ref() else {
        return Ok(state.runtime.sessions());
    };
    match &context.domain {
        None => Ok(state.runtime.sessions()),
        Some(domain) => plane
            .sessions(domain)
            .map(|manager| manager.as_ref())
            .map_err(|error| domain_refused(&error)),
    }
}

pub(crate) async fn get_session_settings(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    let loaded = session_manager(&state, domain.as_ref())?
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
    identity: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
    Json(request): Json<PatchSessionSettingsRequest>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    // W2-③: 读-改-写全程持会话写闸 (与在途回合零交叠, 不丢任何一方的写);
    // W3-①: 写闸按请求域路由 (每域恰一把)。
    let updated = session_manager(&state, domain.as_ref())?
        .update(session_id, |session| {
            if let Some(model) = request.model {
                session.settings.model = model;
            }
            if let Some(preset) = request.permission_preset {
                session.settings.permission_preset = preset;
            }
            if let Some(remember) = request.approval_remember {
                session.settings.approval_remember = remember;
            }
            SessionSettingsBody::from_session(session_id, &session.settings)
        })
        .await
        .map_err(|error| invalid_request(error.to_string()))?;
    let Some((body, admission)) = updated else {
        return Err(session_not_found(&session_id.to_string()));
    };
    record_session_client(
        Some(state.observations.as_ref()),
        session_id,
        identity_label(identity.as_ref()),
        "settings.patch",
        Some(admission),
    );

    Ok(Json(body).into_response())
}

pub(crate) async fn delete_session(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    // W2-③: 删除同为会话账本写 —— 过同一把写闸, 不在回合半途抽走账本;
    // W3-①: 写闸按请求域路由 (每域恰一把)。
    let manager = session_manager(&state, domain.as_ref())?;
    let permit = manager.write_permit(session_id).await;
    let deleted = manager
        .delete(&session_id)
        .await
        .map_err(|error| delete_failed(error.to_string()))?;
    if !deleted {
        return Err(session_not_found(&session_id.to_string()));
    }
    record_session_client(
        Some(state.observations.as_ref()),
        session_id,
        identity_label(identity.as_ref()),
        "session.delete",
        Some(permit.admission()),
    );
    drop(permit);

    Ok(Json(serde_json::json!({ "deleted": true })).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::system_clock;
    use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, Session, SessionStore};
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::{delete, get, patch};
    use axum::Router;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_router(store: Arc<dyn SessionStore>) -> Router {
        let runtime = Runtime::builder()
            .with_session_store(store)
            .build()
            .await
            .unwrap();
        let presence = crate::presence::PresenceService::new(crate::events::EventBus::default());
        let presence_stream = Arc::new(
            presence
                .keepalive()
                .expect("fresh presence surface cannot be shut down"),
        );
        let events = crate::events::EventBus::default();
        let events_stream = Arc::new(
            events
                .keepalive()
                .expect("fresh events surface cannot be shut down"),
        );
        let observations = Arc::new(crate::events::RuntimeObservationSink::new(None, None));
        let observations_stream = Arc::new(
            observations
                .keepalive()
                .expect("fresh flush surface cannot be shut down"),
        );
        let state = GatewayState {
            runtime: Arc::new(runtime),
            services: crate::panels::GatewayServices::default(),
            events,
            events_stream,
            observations,
            observations_stream,
            presence,
            presence_stream,
            hot_config: Arc::new(std::sync::RwLock::new(
                crate::admin::GatewayRuntimeConfig::from_env(),
            )),
            token_gate: crate::canonical_entry::GatewayTokenGate::from_env(),
            client_tokens: crate::client_tokens::ClientTokenTable::default(),
            user_access: crate::client_tokens::UserAccess::default(),
            domains: None,
            domain_runtimes: None,
            shutdown: Arc::new(crate::shutdown::ShutdownTrigger::default()),
        };
        Router::<GatewayState>::new()
            .route(
                "/v1/sessions/:session_id/settings",
                get(get_session_settings).patch(patch_session_settings),
            )
            .route("/v1/sessions/:session_id", delete(delete_session))
            .with_state(state)
    }

    async fn save_session(store: &Arc<dyn SessionStore>, id: SessionId) {
        let clock = system_clock();
        store.save(&Session::new(id, clock.as_ref())).await.unwrap();
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
        assert_eq!(body["approval_remember"], false);
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
                        r#"{"model":"anthropic/claude-3-5","permission_preset":"full","approval_remember":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["model"], "anthropic/claude-3-5");
        assert_eq!(body["permission_preset"], "full");
        assert_eq!(body["approval_remember"], true);
    }

    #[tokio::test]
    async fn patch_approval_remember_is_partial_and_preserves_other_fields() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        // Set model + preset first.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"some/model","permission_preset":"full"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then toggle only approval_remember; model and preset must survive.
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approval_remember":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["approval_remember"], true);
        assert_eq!(body["model"], "some/model");
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
        assert_eq!(body["approval_remember"], false);
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
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&sid.to_string()));
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

    #[tokio::test]
    async fn delete_removes_an_existing_session_and_reports_deletion() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store.clone()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/sessions/{sid}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["deleted"], true);
        assert!(
            store.load(&sid).await.unwrap().is_none(),
            "the store must no longer hold the deleted session"
        );
    }

    #[tokio::test]
    async fn delete_unknown_session_returns_session_not_found_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;
        let sid = SessionId::new();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/sessions/{sid}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "session_not_found");
    }

    #[tokio::test]
    async fn delete_malformed_session_id_returns_invalid_request_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/sessions/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "invalid_request");
    }

    /// One settings PATCH against the HTTP surface, then the read-back GET.
    async fn patch_then_get(
        app: &Router,
        sid: SessionId,
        patch: &str,
    ) -> (serde_json::Value, serde_json::Value) {
        let patched = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(patch.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patched.status(), StatusCode::OK);
        let read_back = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(read_back.status(), StatusCode::OK);
        (json_body(patched).await, json_body(read_back).await)
    }

    #[tokio::test]
    async fn patch_permission_preset_roundtrips_each_tier_through_read_back() {
        // Select a tier, persist it, read the same tier back — for every tier and
        // through every transition (read_only → standard → full), so a changed
        // tier must never round-trip distorted.
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        for tier in ["read_only", "standard", "full"] {
            let body = format!(r#"{{"permission_preset":"{tier}"}}"#);
            let (patched, read_back) = patch_then_get(&app, sid, &body).await;
            assert_eq!(patched["permission_preset"], tier);
            assert_eq!(
                read_back["permission_preset"], tier,
                "tier {tier} must read back unchanged after being persisted"
            );
            assert_eq!(read_back["session_id"], sid.to_string());
        }
    }

    #[tokio::test]
    async fn permission_preset_survives_router_rebuild_over_the_same_store() {
        // A gateway restart rebuilds every HTTP surface from scratch while the
        // durable store keeps the rows. The stored tier must come back as itself,
        // not fall back to the default tier.
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;

        let (patched, _) = patch_then_get(
            &test_router(store.clone()).await,
            sid,
            r#"{"permission_preset":"full","approval_remember":true}"#,
        )
        .await;
        assert_eq!(patched["permission_preset"], "full");

        let restarted = test_router(store).await;
        let response = restarted
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
        assert_eq!(body["permission_preset"], "full");
        assert_eq!(body["approval_remember"], true);
    }
}
