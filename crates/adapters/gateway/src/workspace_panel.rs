//! Workspace contract face (A2 batch): identity + safe display, never a root path.
//!
//! Five endpoints (one panel read face, one write face):
//! - `GET  /v1/panel/workspaces`                → 200 `{"workspaces":[...],"count":n}`
//! - `GET  /v1/workspaces/{workspace_id}`       → 200 `{"workspace":{...}}`
//! - `POST /v1/workspaces`                      → 201 `{"workspace":{...}}`
//! - `POST /v1/workspaces/{workspace_id}/select` → 200 `{"workspace":{...}}`
//! - `POST /v1/workspaces/{workspace_id}/close`  → 200 `{"workspace":{...}}`
//!
//! Contract rules this module owns:
//! 1. **No leak** — the port hands back a DTO (there is no root-path field at
//!    all), and the gateway re-checks `root_display` against the leaf-name rule
//!    before serializing. A value that is not a leaf name is refused with a
//!    500 frame that does **not** echo the value back.
//! 2. **A client request is not an authorization** — the create body has no
//!    root field (`deny_unknown_fields` ⇒ posting `root` is a 400), the managed
//!    root belongs to the host, and the effective permission scope is the
//!    host's confirmed value, not the requested one.
//! 3. **Optimistic concurrency** — the write faces require `expected_revision`;
//!    missing ⇒ 400 `invalid_request`; stale ⇒ 409 `workspace_conflict` and the
//!    mutating call is **not** made, so the workspace stays byte-for-byte equal.
//! 4. **Revoked is not a conflict** — a revoked workspace is 403
//!    `workspace_revoked` (state unavailable, not a concurrency clash).
//! 5. **Retrievable audit** — create / select / close / every denial append an
//!    audit row (`gateway.workspace_action`) whose detail carries only the
//!    workspace id, the action word, the endpoint, the decision and the reason.
//!    No absolute root path ever reaches an audit row.

use std::sync::Arc;

use apeireth_core::kernel::ids::WorkspaceId;
use apeireth_runtime::canonical::workspace::{
    WorkspacePermissionScope, WorkspaceRootKind, WorkspaceStatus, MAX_ROOT_DISPLAY_BYTES,
};
use async_trait::async_trait;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{identity_label, ClientIdentity};
use crate::domain_context::DomainContext;
use crate::error_frame::{ErrorCode, ErrorEnvelope, ErrorFrame};
use crate::panels::GatewayState;

/// Audit event name for every workspace action (create / select / close / deny).
pub const WORKSPACE_AUDIT_EVENT: &str = "gateway.workspace_action";

/// Action word recorded when a workspace is created.
pub const WORKSPACE_ACTION_CREATE: &str = "workspace.create";
/// Action word recorded when a workspace is selected.
pub const WORKSPACE_ACTION_SELECT: &str = "workspace.select";
/// Action word recorded when a workspace is closed.
pub const WORKSPACE_ACTION_CLOSE: &str = "workspace.close";
/// Action word recorded when a workspace action is refused.
pub const WORKSPACE_ACTION_DENY: &str = "workspace.deny";

/// Default row cap of the workspace list face (contract §1 list convention).
pub const DEFAULT_WORKSPACE_LIMIT: usize = 50;
/// Hard row cap of the workspace list face.
pub const MAX_WORKSPACE_LIMIT: usize = 500;

/// The wire shape of one workspace.
///
/// The field set is the frozen front-end draft: identity, a safe display name,
/// the host-confirmed scope, the optimistic-concurrency revision and the
/// capability facts. There is **no** root path field — the read face cannot
/// leak one even by accident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDto {
    /// Stable workspace identity (UUID).
    pub workspace_id: String,
    /// Human label chosen by the host operator.
    pub display_name: String,
    /// Which kind of managed root the workspace owns.
    pub root_kind: WorkspaceRootKind,
    /// Safe display name of the root: the leaf name only (no parent directory,
    /// no drive letter, no separator).
    pub root_display: String,
    /// The host-confirmed permission scope.
    pub permission_scope: WorkspacePermissionScope,
    /// Optimistic-concurrency revision; bumped when the root, the scope or the
    /// revoked state changes.
    pub revision: u64,
    /// Lifecycle state.
    pub status: WorkspaceStatus,
    /// Whether the workspace may still be selected.
    pub can_select: bool,
    /// Whether a temporary workspace may still be created from it.
    pub can_create_temporary: bool,
    /// Whether the contract permits writes (not a human approval).
    pub can_write: bool,
    /// Expiry instant in epoch milliseconds, when the workspace expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// Which host surface produced the workspace.
    pub source: String,
    /// Last mutation instant in epoch milliseconds.
    pub updated_at_ms: i64,
}

/// The list envelope: rows plus the row count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceListDto {
    /// The bounded rows of this response.
    pub workspaces: Vec<WorkspaceDto>,
    /// How many rows this response carries.
    pub count: u64,
}

/// The single-workspace envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEnvelope {
    /// The workspace this response is about.
    pub workspace: WorkspaceDto,
}

/// What the host is asked to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreateCommand {
    /// Human label for the new workspace.
    pub display_name: String,
    /// Which kind of root the client asked for (the host may refuse kinds it
    /// does not assemble).
    pub root_kind: WorkspaceRootKind,
    /// The **requested** scope; the host confirms it before it lands.
    pub permission_scope: WorkspacePermissionScope,
    /// Requested expiry in epoch milliseconds (temporary roots only).
    pub expires_at: Option<i64>,
}

/// Why a workspace action failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspacePanelError {
    /// The workspace does not exist (or is not visible to this caller).
    NotFound {
        /// The requested workspace id.
        workspace_id: String,
    },
    /// The caller's `expected_revision` does not match the current revision.
    Conflict {
        /// The workspace the conflict happened on.
        workspace_id: String,
        /// The revision the caller expected.
        expected_revision: u64,
        /// The revision the workspace actually carries.
        actual_revision: u64,
    },
    /// The workspace was revoked; it can no longer be selected or closed.
    Revoked {
        /// The revoked workspace id.
        workspace_id: String,
    },
    /// The request itself is not acceptable (bad id, bad scope, refused kind).
    InvalidRequest {
        /// Why the request was refused.
        reason: String,
    },
    /// The host failed in a way the caller cannot fix.
    Internal {
        /// What went wrong inside the host.
        reason: String,
    },
}

impl WorkspacePanelError {
    /// The machine-readable contract code of this failure.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound { .. } => ErrorCode::WorkspaceNotFound,
            Self::Conflict { .. } => ErrorCode::WorkspaceConflict,
            Self::Revoked { .. } => ErrorCode::WorkspaceRevoked,
            Self::InvalidRequest { .. } => ErrorCode::InvalidRequest,
            Self::Internal { .. } => ErrorCode::Internal,
        }
    }

    /// The HTTP status of this failure.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::Revoked { .. } => StatusCode::FORBIDDEN,
            Self::InvalidRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// A short machine-ish reason token for the audit row.
    pub fn reason_label(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "workspace_not_found",
            Self::Conflict { .. } => "revision_conflict",
            Self::Revoked { .. } => "workspace_revoked",
            Self::InvalidRequest { .. } => "invalid_request",
            Self::Internal { .. } => "internal_error",
        }
    }

    /// Render this failure as the unified gateway error frame.
    ///
    /// The not-found message is constant on purpose: a caller must not be able
    /// to tell "does not exist" from "exists but belongs to somebody else".
    pub fn into_response(&self) -> Response {
        let message = match self {
            Self::NotFound { .. } => "工作区不存在或不可见".to_string(),
            Self::Conflict {
                expected_revision,
                actual_revision,
                ..
            } => format!("工作区修订号冲突: 期望 {expected_revision}, 实际 {actual_revision}"),
            Self::Revoked { .. } => "工作区已吊销, 不可再选择或关闭".to_string(),
            Self::InvalidRequest { reason } => reason.clone(),
            Self::Internal { reason } => reason.clone(),
        };
        let details = match self {
            Self::Conflict {
                expected_revision,
                actual_revision,
                ..
            } => Some(serde_json::json!({
                "expected_revision": expected_revision,
                "actual_revision": actual_revision,
            })),
            _ => None,
        };
        error_response(self.status(), self.code(), message, details)
    }
}

/// The bounded-context port the composition root implements.
///
/// The port speaks DTOs, never domain rows: the gateway therefore has no field
/// that could carry an absolute root path.
#[async_trait]
pub trait WorkspacePanel: Send + Sync {
    /// List the bounded set of workspaces the host knows about.
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceDto>, String>;

    /// Read one workspace.
    async fn get_workspace(&self, workspace_id: &str) -> Result<WorkspaceDto, WorkspacePanelError>;

    /// Create one workspace under the host's managed root.
    async fn create_workspace(
        &self,
        command: WorkspaceCreateCommand,
    ) -> Result<WorkspaceDto, WorkspacePanelError>;

    /// Select one workspace at an expected revision.
    async fn select_workspace(
        &self,
        workspace_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError>;

    /// Close (revoke) one workspace at an expected revision.
    async fn close_workspace(
        &self,
        workspace_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError>;
}

/// The create body of `POST /v1/workspaces`.
///
/// The body carries **no** root path: the managed root is a host fact. Unknown
/// fields are refused, so a client that tries to push `root` in gets a 400
/// instead of a silently ignored path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCreateRequest {
    /// Human label for the new workspace.
    pub display_name: String,
    /// Which kind of root the client asks for (default: `managed`).
    #[serde(default)]
    pub root_kind: Option<WorkspaceRootKind>,
    /// The **requested** scope (default: `read` — never escalated by default).
    #[serde(default)]
    pub permission_scope: Option<WorkspacePermissionScope>,
    /// Requested expiry in epoch milliseconds (temporary roots only).
    #[serde(default)]
    pub expires_at: Option<i64>,
}

/// The body of the two write faces: the revision the caller saw.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRevisionRequest {
    /// The revision the caller believes the workspace carries.
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

/// Query of the workspace list face.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct WorkspaceListQuery {
    /// Row cap for this response.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Which mutation the write face performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceMutation {
    Select,
    Close,
}

/// `GET /v1/panel/workspaces` — the bounded workspace list.
///
/// Three states are distinguished on purpose: not assembled (501 `unsupported`),
/// empty (200 with an empty array and `count: 0`) and unavailable (500).
pub async fn list_workspaces(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    query: Result<Query<WorkspaceListQuery>, QueryRejection>,
) -> Response {
    if let Err(refusal) = crate::domain_context::request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/panel/workspaces",
    ) {
        return refusal;
    }
    let Some(panel) = state.services.workspaces.as_ref() else {
        return crate::panels::unsupported("workspaces.read");
    };
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => {
            return invalid_request_frame(format!("查询参数无效: {}", rejection.body_text()));
        }
    };
    let limit = match query.limit {
        None => DEFAULT_WORKSPACE_LIMIT,
        Some(limit) if limit == 0 || limit > MAX_WORKSPACE_LIMIT => {
            return invalid_request_frame(format!("limit 必须在 1..={MAX_WORKSPACE_LIMIT} 之间"));
        }
        Some(limit) => limit,
    };
    let rows = match panel.list_workspaces().await {
        Ok(rows) => rows,
        Err(reason) => return crate::panels::panel_error(reason),
    };
    let mut workspaces = Vec::with_capacity(rows.len().min(limit));
    for row in rows.into_iter().take(limit) {
        match guard_workspace_dto(row) {
            Ok(row) => workspaces.push(row),
            Err(response) => return response,
        }
    }
    let count = workspaces.len() as u64;
    (StatusCode::OK, Json(WorkspaceListDto { workspaces, count })).into_response()
}

/// `GET /v1/workspaces/{workspace_id}` — one workspace.
pub async fn get_workspace(
    State(state): State<GatewayState>,
    Path(workspace_id): Path<String>,
) -> Response {
    let Some(panel) = state.services.workspaces.as_ref() else {
        return crate::panels::unsupported("workspaces.read");
    };
    if workspace_id.parse::<WorkspaceId>().is_err() {
        return invalid_request_frame("workspace_id 格式非法");
    }
    match panel.get_workspace(&workspace_id).await {
        Ok(dto) => match guard_workspace_dto(dto) {
            Ok(dto) => (StatusCode::OK, Json(WorkspaceEnvelope { workspace: dto })).into_response(),
            Err(response) => response,
        },
        Err(error) => error.into_response(),
    }
}

/// `POST /v1/workspaces` — create one workspace under the host's managed root.
pub async fn create_workspace(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    body: Result<Json<WorkspaceCreateRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let Some(panel) = state.services.workspaces.as_ref() else {
        return crate::panels::unsupported("workspaces.write");
    };
    let command = WorkspaceCreateCommand {
        display_name: request.display_name,
        root_kind: request.root_kind.unwrap_or(WorkspaceRootKind::Managed),
        permission_scope: request
            .permission_scope
            .unwrap_or(WorkspacePermissionScope::Read),
        expires_at: request.expires_at,
    };
    let client_label = identity_label(client.as_ref());
    match panel.create_workspace(command).await {
        Ok(dto) => {
            let dto = match guard_workspace_dto(dto) {
                Ok(dto) => dto,
                Err(response) => return response,
            };
            record_workspace_audit(
                &state,
                WORKSPACE_ACTION_CREATE,
                "/v1/workspaces",
                &dto.workspace_id,
                "allow",
                None,
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
                "/v1/workspaces",
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

/// `POST /v1/workspaces/{workspace_id}/select` — select one workspace.
pub async fn select_workspace(
    State(state): State<GatewayState>,
    Path(workspace_id): Path<String>,
    client: Option<Extension<ClientIdentity>>,
    body: Result<Json<WorkspaceRevisionRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let client_label = identity_label(client.as_ref());
    apply_workspace_mutation(
        &state,
        &workspace_id,
        request.expected_revision,
        "/v1/workspaces/{workspace_id}/select",
        WORKSPACE_ACTION_SELECT,
        WorkspaceMutation::Select,
        client_label,
    )
    .await
}

/// `POST /v1/workspaces/{workspace_id}/close` — close (revoke) one workspace.
pub async fn close_workspace(
    State(state): State<GatewayState>,
    Path(workspace_id): Path<String>,
    client: Option<Extension<ClientIdentity>>,
    body: Result<Json<WorkspaceRevisionRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let client_label = identity_label(client.as_ref());
    apply_workspace_mutation(
        &state,
        &workspace_id,
        request.expected_revision,
        "/v1/workspaces/{workspace_id}/close",
        WORKSPACE_ACTION_CLOSE,
        WorkspaceMutation::Close,
        client_label,
    )
    .await
}

/// Shared write-face pipeline: required revision, host lookup, revoked check,
/// revision check, then the mutation itself.
///
/// The revision check happens **before** the mutating call, so a stale caller
/// never reaches the host and the workspace stays byte-for-byte equal.
async fn apply_workspace_mutation(
    state: &GatewayState,
    workspace_id: &str,
    expected_revision: Option<u64>,
    endpoint: &str,
    action: &str,
    mutation: WorkspaceMutation,
    client_label: &str,
) -> Response {
    let Some(expected_revision) = expected_revision else {
        record_workspace_audit(
            state,
            WORKSPACE_ACTION_DENY,
            endpoint,
            workspace_id,
            "deny",
            Some("missing_expected_revision"),
            client_label,
        )
        .await;
        return invalid_request_frame("expected_revision 必填: 写面必须带上读面读到的修订号");
    };
    let Some(panel) = state.services.workspaces.as_ref() else {
        return crate::panels::unsupported("workspaces.write");
    };
    if workspace_id.parse::<WorkspaceId>().is_err() {
        record_workspace_audit(
            state,
            WORKSPACE_ACTION_DENY,
            endpoint,
            workspace_id,
            "deny",
            Some("malformed_workspace_id"),
            client_label,
        )
        .await;
        return invalid_request_frame("workspace_id 格式非法");
    }
    let current = match panel.get_workspace(workspace_id).await {
        Ok(current) => current,
        Err(error) => {
            record_workspace_audit(
                state,
                WORKSPACE_ACTION_DENY,
                endpoint,
                workspace_id,
                "deny",
                Some(error.reason_label()),
                client_label,
            )
            .await;
            return error.into_response();
        }
    };
    if current.status == WorkspaceStatus::Revoked {
        record_workspace_audit(
            state,
            WORKSPACE_ACTION_DENY,
            endpoint,
            workspace_id,
            "deny",
            Some("workspace_revoked"),
            client_label,
        )
        .await;
        return error_response(
            StatusCode::FORBIDDEN,
            ErrorCode::WorkspaceRevoked,
            "工作区已吊销, 不可再选择或关闭",
            None,
        );
    }
    if current.revision != expected_revision {
        record_workspace_audit(
            state,
            WORKSPACE_ACTION_DENY,
            endpoint,
            workspace_id,
            "deny",
            Some("revision_conflict"),
            client_label,
        )
        .await;
        return error_response(
            StatusCode::CONFLICT,
            ErrorCode::WorkspaceConflict,
            format!(
                "工作区修订号冲突: 期望 {expected_revision}, 实际 {}",
                current.revision
            ),
            Some(serde_json::json!({
                "expected_revision": expected_revision,
                "actual_revision": current.revision,
            })),
        );
    }
    let outcome = match mutation {
        WorkspaceMutation::Select => {
            panel
                .select_workspace(workspace_id, expected_revision)
                .await
        }
        WorkspaceMutation::Close => panel.close_workspace(workspace_id, expected_revision).await,
    };
    match outcome {
        Ok(dto) => {
            let dto = match guard_workspace_dto(dto) {
                Ok(dto) => dto,
                Err(response) => return response,
            };
            record_workspace_audit(
                state,
                action,
                endpoint,
                &dto.workspace_id,
                "allow",
                None,
                client_label,
            )
            .await;
            (StatusCode::OK, Json(WorkspaceEnvelope { workspace: dto })).into_response()
        }
        Err(error) => {
            record_workspace_audit(
                state,
                WORKSPACE_ACTION_DENY,
                endpoint,
                workspace_id,
                "deny",
                Some(error.reason_label()),
                client_label,
            )
            .await;
            error.into_response()
        }
    }
}

/// Refuse a workspace whose `root_display` is not a safe leaf name.
///
/// The offending value is **not** echoed back: the frame only says the host
/// returned an unsafe display name.
fn guard_workspace_dto(dto: WorkspaceDto) -> Result<WorkspaceDto, Response> {
    if !root_display_is_safe(&dto.root_display) {
        return Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            "工作区展示名未通过安全检查 (宿主返回了不安全的工作区根展示名)",
            None,
        ));
    }
    Ok(dto)
}

/// A safe `root_display` is a leaf name: non-empty, bounded, and free of
/// separators, drive letters and control characters.
fn root_display_is_safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ROOT_DISPLAY_BYTES
        && !value
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':') || character.is_control())
}

/// Append one retrievable audit row for a workspace action.
///
/// The detail carries only identity and decision facts — never a root path.
async fn record_workspace_audit(
    state: &GatewayState,
    action: &str,
    endpoint: &str,
    workspace_id: &str,
    decision: &str,
    reason: Option<&str>,
    client_label: &str,
) {
    let workspace = if workspace_id.is_empty() {
        "-".to_string()
    } else {
        crate::canonical_entry::truncate_label(workspace_id)
    };
    let mut detail = format!(
        "workspace={workspace} action={action} endpoint={endpoint} decision={decision} client={}",
        crate::canonical_entry::truncate_label(client_label)
    );
    if let Some(reason) = reason {
        detail.push_str(&format!(" reason={reason}"));
    }
    if let Some(audit) = state.services.audit_commands.as_ref() {
        audit
            .append_audit(WORKSPACE_AUDIT_EVENT, Some(&detail))
            .await;
        return;
    }
    state
        .observations
        .buffer_audit(WORKSPACE_AUDIT_EVENT, Some(detail));
}

/// The unified error frame with optional structured details.
fn error_response(
    status: StatusCode,
    code: ErrorCode,
    message: impl Into<String>,
    details: Option<serde_json::Value>,
) -> Response {
    let mut frame = ErrorFrame::new(code, message);
    if let Some(details) = details {
        frame = frame.with_details(details);
    }
    (status, Json(ErrorEnvelope { error: frame })).into_response()
}

/// A 400 frame in the unified shape.
fn invalid_request_frame(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        ErrorCode::InvalidRequest,
        message,
        None,
    )
}
