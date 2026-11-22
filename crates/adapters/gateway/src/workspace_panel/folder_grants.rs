//! Paired native host intake for read-only temporary folder grants.

use std::path::PathBuf;

use apeireth_runtime::canonical::workspace::{
    DesktopFolderIdentity, DESKTOP_FOLDER_LEASE_SECONDS, DESKTOP_GRANTED_WORKSPACE_SOURCE,
};
use axum::extract::{DefaultBodyLimit, FromRequest, Request};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::middleware::{self, Next};
use axum::Router;

use super::*;

const ROUTE: &str = "/v1/admin/desktop-folder-grants";
const TOKEN_HEADER: &str = "x-apeireth-native-folder-grant";

/// Credential shared only by the native desktop launcher and its managed gateway process.
pub struct NativeFolderGrantToken([u8; 64]);

impl NativeFolderGrantToken {
    /// Accept the launcher's canonical 64-digit hexadecimal credential.
    pub fn parse(value: &str) -> Option<Self> {
        let bytes: [u8; 64] = value.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(*byte, b'a'..=b'f'))
            .then_some(Self(bytes))
    }

    /// Compare a presented candidate without exposing the credential.
    pub fn matches(&self, candidate: &str) -> bool {
        crate::client_tokens::constant_time_eq(&self.0, candidate.as_bytes())
    }
}

/// Evidence from the native chooser. Filesystem authority is rechecked by the host.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeFolderGrant {
    /// User-visible workspace label.
    pub display_name: String,
    /// Absolute directory supplied by the native host.
    pub directory: PathBuf,
    /// Stable identity from the chooser's open directory handle.
    pub directory_identity: DesktopFolderIdentity,
}

fn authorized_panel<'a>(
    state: &'a GatewayState,
    client: Option<&Extension<ClientIdentity>>,
    domain: Option<&Extension<DomainContext>>,
    headers: &HeaderMap,
) -> Option<&'a dyn WorkspacePanel> {
    if !managed_workspace_operator(state, client, domain) || headers.contains_key(header::ORIGIN) {
        return None;
    }
    let panel = state.services.workspaces.as_deref()?;
    let expected = panel.native_folder_grant_token()?;
    let values = headers.get_all(TOKEN_HEADER);
    let mut values = values.iter();
    let presented = values.next()?.to_str().ok()?;
    (values.next().is_none() && expected.matches(presented)).then_some(panel)
}

async fn denied(state: &GatewayState, client_label: &str) -> Response {
    record_workspace_audit(
        state,
        WORKSPACE_ACTION_DENY,
        ROUTE,
        "",
        "deny",
        Some("native_folder_authority_required"),
        client_label,
    )
    .await;
    error_response(
        StatusCode::FORBIDDEN,
        ErrorCode::PermissionDenied,
        "临时目录授权需要受管桌面的原生授权凭据。",
        None,
    )
}

async fn capability(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    headers: HeaderMap,
) -> Response {
    if authorized_panel(&state, client.as_ref(), domain.as_ref(), &headers).is_none() {
        return denied(&state, identity_label(client.as_ref())).await;
    }
    Json(serde_json::json!({
        "schema_version": 1,
        "permission_scope": "read",
        "lease_seconds": DESKTOP_FOLDER_LEASE_SECONDS,
    }))
    .into_response()
}

async fn issue(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    mut request: Request,
) -> Response {
    let client_label = identity_label(client.as_ref());
    let Some(panel) = authorized_panel(&state, client.as_ref(), domain.as_ref(), request.headers())
    else {
        return denied(&state, client_label).await;
    };
    request.headers_mut().remove(TOKEN_HEADER);
    let Ok(Json(command)) = Json::<NativeFolderGrant>::from_request(request, &state).await else {
        return invalid_request_frame("临时目录授权请求格式无效。");
    };
    if !command.directory.is_absolute() {
        return invalid_request_frame("临时目录授权需要原生选择的绝对目录。");
    }
    match panel.grant_native_folder(command).await {
        Ok(dto) => {
            let dto = match guard_workspace_dto(dto) {
                Ok(dto) if is_temporary_receipt(&dto) => dto,
                Ok(_) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ErrorCode::Internal,
                        "临时目录授权记录未能确认。",
                        None,
                    );
                }
                Err(response) => return response,
            };
            record_workspace_audit(
                &state,
                WORKSPACE_ACTION_CREATE,
                ROUTE,
                &dto.workspace_id,
                "allow",
                Some("native_temporary_grant"),
                client_label,
            )
            .await;
            (
                StatusCode::CREATED,
                Json(WorkspaceEnvelope { workspace: dto }),
            )
                .into_response()
        }
        Err(error) => {
            record_workspace_audit(
                &state,
                WORKSPACE_ACTION_DENY,
                ROUTE,
                "",
                "deny",
                Some(error.reason_label()),
                client_label,
            )
            .await;
            error.into_response()
        }
    }
}

fn is_temporary_receipt(dto: &WorkspaceDto) -> bool {
    dto.workspace_id.parse::<WorkspaceId>().is_ok()
        && is_live_temporary(dto)
        && dto.permission_scope == WorkspacePermissionScope::Read
        && dto.revision == 0
        && !dto.can_write
        && !dto.can_create_temporary
        && dto
            .expires_at
            .and_then(|expires_at| expires_at.checked_sub(dto.updated_at_ms))
            == Some((DESKTOP_FOLDER_LEASE_SECONDS * 1000) as i64)
}

pub(super) fn is_live_temporary(dto: &WorkspaceDto) -> bool {
    dto.root_kind == WorkspaceRootKind::Temporary
        && dto.source == DESKTOP_GRANTED_WORKSPACE_SOURCE
        && dto.status == WorkspaceStatus::Ready
        && dto.can_select
}

async fn protect(mut request: Request, next: Next) -> Response {
    for (name, value) in request.headers_mut().iter_mut() {
        if name == TOKEN_HEADER {
            value.set_sensitive(true);
        }
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

pub(crate) fn routes() -> Router<GatewayState> {
    Router::new()
        .route(ROUTE, axum::routing::get(capability).post(issue))
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::from_fn(protect))
}
