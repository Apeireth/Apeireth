//! Narrow managed-desktop configuration over the composition root's keyring.
//! Configuration presence is not a claim of provider connectivity.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{header, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain_context::DomainContext;
use crate::panels::GatewayState;
use crate::{ClientIdentity, GatewayTokenGate};

/// Secret-free configuration and credential availability, never connectivity.
#[derive(Clone, Debug, Serialize)]
pub struct ModelConnectionView {
    pub schema_version: u32,
    pub provider: &'static str,
    pub base_url: String,
    pub model: String,
    pub credential_state: CredentialState,
    pub storage: &'static str,
    pub endpoint_bound: bool,
    pub restart_required: bool,
}

/// Availability is read from the real credential backend, not a masked cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialState {
    Missing,
    Available,
    Unavailable,
}

/// Saving schedules the configuration for the next ordinary runtime start.
/// Deliberately no Debug/Serialize: raw credentials never enter diagnostics.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConnectionPatch {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl ModelConnectionPatch {
    /// Shared validation for HTTP, persistent reads and the composition root.
    pub fn valid(&self) -> bool {
        let Ok(uri) = self.base_url.parse::<Uri>() else {
            return false;
        };
        let scheme = uri.scheme_str();
        let host = uri.host().unwrap_or_default();
        let safe_http =
            scheme == Some("http") && matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1");
        let authority = uri
            .authority()
            .map(|value| value.as_str())
            .unwrap_or_default();
        let canonical: String = self
            .model
            .to_ascii_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        self.base_url.len() <= 2048
            && !self.base_url.contains(['?', '#'])
            && !authority.contains('@')
            && !host.is_empty()
            && (scheme == Some("https") || safe_http)
            && !self.base_url.chars().any(char::is_whitespace)
            && !self.model.is_empty()
            && self.model.len() <= 128
            && self
                .model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
            && apeireth_core::kernel::ModelId::new(canonical).is_ok()
            && self.api_key.as_ref().is_none_or(|key| {
                !key.trim().is_empty() && key.len() <= 4096 && !key.chars().any(char::is_control)
            })
    }
}

/// Errors intentionally carry no external error strings or supplied values.
#[derive(Clone, Copy, Debug)]
pub enum ModelConnectionError {
    Invalid,
    Scope,
    Storage,
    CredentialUnavailable,
    EndpointChangeNeedsKey,
}

/// OS credential and non-secret profile authority supplied by the CLI root.
pub trait ModelConnectionService: Send + Sync {
    fn read(&self) -> Result<ModelConnectionView, ModelConnectionError>;
    fn save(
        &self,
        patch: ModelConnectionPatch,
    ) -> Result<ModelConnectionView, ModelConnectionError>;
}

/// An explicit single local operator, still authorized by the ordinary role gate.
/// No user file or implicit multi-user-to-default-store conversion is involved.
pub fn managed_operator_access() -> crate::UserAccess {
    crate::UserAccess::from_table(apeireth_governance::UserTable::new(vec![
        apeireth_governance::UserRecord {
            user_id: "default".into(),
            role: "owner".into(),
            client_names: vec![crate::GatewayTokenGate::ENV_FACE_CLIENT_NAME.into()],
            domain: Some("default".into()),
        },
    ]))
}

pub(crate) fn is_managed_single_operator(state: &GatewayState) -> bool {
    state.services.model_connection.is_some()
        && state
            .token_gate
            .enforced_token()
            .is_some_and(|token| token.len() >= 32)
        && state.client_tokens.is_empty()
        && state.user_access.table() == managed_operator_access().table()
}

pub(crate) fn is_authenticated_managed_operator(
    state: &GatewayState,
    client: Option<&ClientIdentity>,
    domain: Option<&DomainContext>,
) -> bool {
    is_managed_single_operator(state)
        && !domain.is_some_and(DomainContext::is_non_default)
        && client.is_some_and(|identity| {
            identity.authenticated && identity.client == GatewayTokenGate::ENV_FACE_CLIENT_NAME
        })
}

fn failure(error: ModelConnectionError) -> Response {
    let (status, code, message) = match error {
        ModelConnectionError::Invalid => (
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "模型连接设置无效。",
        ),
        ModelConnectionError::Scope => (
            StatusCode::FORBIDDEN,
            "single_operator_required",
            "此入口仅用于受管单用户运行时。",
        ),
        ModelConnectionError::Storage => (
            StatusCode::SERVICE_UNAVAILABLE,
            "profile_unavailable",
            "模型连接档案无法保存或读取。",
        ),
        ModelConnectionError::CredentialUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "credential_unavailable",
            "系统凭据管理器无法完成操作，设置未保存。",
        ),
        ModelConnectionError::EndpointChangeNeedsKey => (
            StatusCode::CONFLICT,
            "endpoint_change_needs_key",
            "变更端点需要同时重新填写属于该端点的密钥。",
        ),
    };
    reply(
        status,
        serde_json::json!({"error":{"code":code,"message":message}}),
    )
}

fn reply(status: StatusCode, value: impl Serialize) -> Response {
    (
        status,
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Json(value),
    )
        .into_response()
}

fn service(state: &GatewayState) -> Result<Arc<dyn ModelConnectionService>, Response> {
    state.services.model_connection.clone().ok_or_else(|| {
        reply(StatusCode::NOT_IMPLEMENTED, serde_json::json!({"error":{"code":"unsupported","message":"未装配受管模型连接入口。"}}))
    })
}

async fn read(State(state): State<GatewayState>) -> Response {
    let service = match service(&state) {
        Ok(service) => service,
        Err(error) => return error,
    };
    match tokio::task::spawn_blocking(move || service.read()).await {
        Ok(Ok(view)) => reply(StatusCode::OK, view),
        Ok(Err(error)) => failure(error),
        Err(_) => failure(ModelConnectionError::Storage),
    }
}

async fn save(
    State(state): State<GatewayState>,
    body: Result<Json<ModelConnectionPatch>, JsonRejection>,
) -> Response {
    let Ok(Json(patch)) = body else {
        return failure(ModelConnectionError::Invalid);
    };
    if !patch.valid() {
        return failure(ModelConnectionError::Invalid);
    }
    let service = match service(&state) {
        Ok(service) => service,
        Err(error) => return error,
    };
    match tokio::task::spawn_blocking(move || service.save(patch)).await {
        Ok(Ok(view)) => reply(
            StatusCode::OK,
            serde_json::json!({"schema_version":1,"ok":true,"restart_required":view.restart_required}),
        ),
        Ok(Err(error)) => failure(error),
        Err(_) => failure(ModelConnectionError::Storage),
    }
}

pub(crate) fn routes() -> Router<GatewayState> {
    Router::new()
        .route("/v1/admin/model-connection", get(read).post(save))
        .layer(DefaultBodyLimit::max(8192))
}

pub(crate) async fn protect_response(request: Request, next: Next) -> Response {
    let sensitive = request.uri().path() == "/v1/admin/model-connection";
    let mut response = next.run(request).await;
    if sensitive {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );
        response.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_remote_plaintext_userinfo_and_unbounded_or_ambiguous_fields() {
        let patch = |url: &str, model: &str| ModelConnectionPatch {
            base_url: url.into(),
            model: model.into(),
            api_key: None,
        };
        assert!(patch("https://model.example/v1", "vendor/Model-A").valid());
        assert!(patch("http://127.0.0.1:1200/v1", "model-a").valid());
        for url in [
            "http://model.example/v1",
            "https://secret@model.example/v1",
            "https://model.example/v1?key=secret",
            "https://model.example/#secret",
        ] {
            assert!(!patch(url, "model").valid());
        }
        assert!(!patch("https://model.example/v1", "model-a,model-b").valid());
        for model in ["1model", ".m", "m..x", "/m"] {
            assert!(!patch("https://model.example/v1", model).valid());
        }
        assert!(serde_json::from_str::<ModelConnectionPatch>(
            r#"{"base_url":"https://model.example","model":"m","unexpected":"raw-secret"}"#
        )
        .is_err());
    }
}
