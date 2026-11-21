//! Managed desktop startup settings for three existing capabilities.

use std::sync::Arc;

use axum::extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain_context::DomainContext;
use crate::panels::GatewayState;
use crate::{ClientIdentity, GatewayTokenGate};

const PATH: &str = "/v1/admin/desktop-capabilities";

/// The complete allowlist of startup switches supported by the managed host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopCapabilitySettings {
    /// Register controlled project file tools behind existing workspace checks.
    pub project_file_tools: bool,
    /// Learn explicitly stated preferences for future turns.
    pub preference_learning: bool,
    /// Register search of the local skill shelf.
    pub skill_search: bool,
}

impl Default for DesktopCapabilitySettings {
    fn default() -> Self {
        Self {
            project_file_tools: true,
            preference_learning: true,
            skill_search: false,
        }
    }
}

/// The persisted request and actual running assembly, independently reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DesktopCapabilityView {
    pub schema_version: u32,
    pub revision: u64,
    pub configured: DesktopCapabilitySettings,
    pub effective: DesktopCapabilitySettings,
    pub restart_required: bool,
}

/// One explicit compare-and-swap of the configured switches.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopCapabilityPatch {
    pub expected_revision: u64,
    pub configured: DesktopCapabilitySettings,
}

/// Expected errors carry no paths or raw request values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopCapabilityError {
    Conflict,
    Storage,
}

/// File storage and assembly readback are owned by the CLI composition root.
pub trait DesktopCapabilityService: Send + Sync {
    fn read(&self) -> Result<DesktopCapabilityView, DesktopCapabilityError>;
    fn save(
        &self,
        patch: DesktopCapabilityPatch,
    ) -> Result<DesktopCapabilityView, DesktopCapabilityError>;
}

fn error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({"error":{"code":code,"message":message}})),
    )
        .into_response()
}

fn response(
    result: Result<Result<DesktopCapabilityView, DesktopCapabilityError>, tokio::task::JoinError>,
) -> Response {
    match result {
        Ok(Ok(view)) => Json(view).into_response(),
        Ok(Err(DesktopCapabilityError::Conflict)) => error(
            StatusCode::CONFLICT,
            "capability_revision_conflict",
            "能力配置已被修改，请刷新后再试。",
        ),
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "profile_unavailable",
            "无法保存或读取受管能力配置。",
        ),
    }
}

fn authorize(
    state: &GatewayState,
    client: Option<&Extension<ClientIdentity>>,
    domain: Option<&Extension<DomainContext>>,
) -> Result<Arc<dyn DesktopCapabilityService>, Response> {
    if !crate::model_connection::is_managed_single_operator(state)
        || domain.is_some_and(|Extension(domain)| domain.is_non_default())
        || !client.is_some_and(|Extension(identity)| {
            identity.authenticated && identity.client == GatewayTokenGate::ENV_FACE_CLIENT_NAME
        })
    {
        return Err(error(
            StatusCode::FORBIDDEN,
            "single_operator_required",
            "能力配置仅用于认证的受管桌面单操作者。",
        ));
    }
    state.services.desktop_capabilities.clone().ok_or_else(|| {
        error(
            StatusCode::NOT_IMPLEMENTED,
            "unsupported",
            "当前运行时未装配能力配置。",
        )
    })
}

async fn read(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
) -> Response {
    let service = match authorize(&state, client.as_ref(), domain.as_ref()) {
        Ok(service) => service,
        Err(error) => return error,
    };
    response(tokio::task::spawn_blocking(move || service.read()).await)
}

async fn save(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    body: Result<Json<DesktopCapabilityPatch>, JsonRejection>,
) -> Response {
    let service = match authorize(&state, client.as_ref(), domain.as_ref()) {
        Ok(service) => service,
        Err(error) => return error,
    };
    let Ok(Json(patch)) = body else {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "能力配置字段无效。",
        );
    };
    response(tokio::task::spawn_blocking(move || service.save(patch)).await)
}

pub(crate) fn routes() -> Router<GatewayState> {
    Router::new()
        .route(PATH, get(read).post(save))
        .layer(DefaultBodyLimit::max(1024))
}

pub(crate) async fn protect_response(request: Request, next: Next) -> Response {
    let scoped = request.uri().path() == PATH;
    let mut response = next.run(request).await;
    if scoped {
        let headers = response.headers_mut();
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        );
    }
    response
}
