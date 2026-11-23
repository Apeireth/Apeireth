//! Open a desktop contact session before it has any messages.

use super::*;
use crate::contact_panel::ContactStatusDto;
use apeireth_runtime::canonical::{RuntimeError, Session, SessionEventKind, SessionManager};
use serde_json::json;

pub(super) const ROUTE: &str = "/v1/sessions/:session_id/contact-session";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenContactSession {
    contact_id: ContactId,
    context_key: String,
    incarnation: String,
    #[serde(default)]
    require_existing: bool,
}

pub(crate) fn supports_scope(
    state: &GatewayState,
    identity: Option<&Extension<ClientIdentity>>,
    domain: Option<&Extension<DomainContext>>,
) -> bool {
    identity.is_some_and(|Extension(identity)| identity.authenticated)
        && (!state.user_access.is_active()
            || crate::model_connection::is_managed_single_operator(state))
        && !domain.is_some_and(|Extension(domain)| domain.is_non_default())
}

pub(crate) fn scope_refused() -> Response {
    ErrorFrame::response(
        StatusCode::FORBIDDEN,
        ErrorCode::PermissionDenied,
        "联系人文字会话需要认证的单操作者运行时。",
    )
}

pub(super) async fn open(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    Path(session): Path<String>,
    body: Result<Json<OpenContactSession>, JsonRejection>,
) -> Response {
    if !supports_scope(&state, identity.as_ref(), domain.as_ref()) {
        return scope_refused();
    }
    let (Ok(session_id), Ok(Json(request))) = (SessionId::from_str(&session), body) else {
        return invalid_request_frame("会话标识或联系人会话请求无效。");
    };
    if session != session_id.to_string()
        || request.context_key.len() != 64
        || !request
            .context_key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || request.incarnation.trim().is_empty()
        || request.incarnation.len() > 128
        || request.incarnation.chars().any(char::is_control)
    {
        return invalid_request_frame("会话标识或宿主身份格式无效。");
    }

    let contact = request.contact_id.to_string();
    let label = identity_label(identity.as_ref());
    let result = open_locked(&state, session_id, &contact, &request).await;
    match result {
        Ok(created) => {
            if created {
                record_session_binding_audit(
                    &state,
                    SESSION_BINDING_ACTION_CREATE,
                    &session,
                    &contact,
                    "allow",
                    Some("contact_session"),
                    label,
                )
                .await;
            }
            (
                [(axum::http::header::CACHE_CONTROL, "no-store")],
                Json(json!({
                    "schema": 1, "session_id": session, "contact_id": contact,
                    "context_key": request.context_key, "incarnation": request.incarnation
                })),
            )
                .into_response()
        }
        Err((reason, response)) => deny(&state, &session, &contact, reason, label, response).await,
    }
}

async fn open_locked(
    state: &GatewayState,
    session_id: SessionId,
    contact: &str,
    request: &OpenContactSession,
) -> Result<bool, (&'static str, Response)> {
    let (Some(panel), Some(contacts), Some(workspaces)) = (
        state.services.contacts.as_ref(),
        state.runtime.contact_registry(),
        state.runtime.workspace_registry(),
    ) else {
        return Err((
            "registry_not_installed",
            crate::panels::unsupported("session_agent_binding.open_contact"),
        ));
    };
    let manager = state.runtime.sessions();
    let _permit = manager.write_permit(session_id).await;
    let row = panel
        .get_contact(contact)
        .await
        .map_err(|e| ("contact_read_failed", e.into_response()))?;
    match row.status {
        ContactStatusDto::Archived => {
            return Err((
                "contact_archived",
                ErrorFrame::response(
                    StatusCode::FORBIDDEN,
                    ErrorCode::ContactArchived,
                    "联系人已归档。",
                ),
            ))
        }
        ContactStatusDto::Unavailable => {
            return Err((
                "contact_unavailable",
                ErrorFrame::response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::Internal,
                    "联系人暂不可用。",
                ),
            ));
        }
        ContactStatusDto::Ready | ContactStatusDto::Draft => (),
    }
    let Some(bound) = row.workspace_binding else {
        return Err((
            "workspace_required",
            error_response(
                StatusCode::CONFLICT,
                ErrorCode::ContactBindingConflict,
                "请先为联系人登记工作区。",
                Some(json!({"reason":"workspace_required"})),
            ),
        ));
    };
    let invalid_identity = || {
        (
            "registry_invalid",
            ErrorFrame::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                "联系人登记身份无法校验。",
            ),
        )
    };
    if row.contact_id != contact {
        return Err(invalid_identity());
    }
    let claim = AgentBindingRequest {
        contact_id: request.contact_id,
        agent_id: row.agent_id.parse().map_err(|_| invalid_identity())?,
        persona_revision: row.persona_revision,
        workspace_id: bound.workspace_id.parse().map_err(|_| invalid_identity())?,
        workspace_revision: bound.revision,
        context_key: Some(request.context_key.clone()),
        incarnation: Some(request.incarnation.clone()),
    };
    let clock = manager.clock().as_ref();
    let binding = resolve_agent_binding(
        &claim,
        contacts.as_ref(),
        workspaces.as_ref(),
        Timestamp::from_clock(clock).epoch_millis() as u64,
    )
    .map_err(|error| {
        let (status, code, message) = domain_error_frame(&error);
        (
            domain_reason_label(&error),
            ErrorFrame::response(status, code, message),
        )
    })?;
    let storage_failure = |_| {
        (
            "session_store_error",
            ErrorFrame::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                "联系人会话无法保存或核对。",
            ),
        )
    };
    if let Some(existing) = manager.load(&session_id).await.map_err(storage_failure)? {
        let same = existing.agent_binding().is_some_and(|saved| {
            saved.contact_id() == binding.contact_id()
                && saved.agent_id() == binding.agent_id()
                && saved.persona_revision() == binding.persona_revision()
                && saved.workspace_id() == binding.workspace_id()
                && saved.workspace_revision() == binding.workspace_revision()
                && saved.contact_workspace_revision() == binding.contact_workspace_revision()
                && saved.context_key() == binding.context_key()
                && saved.incarnation().is_some()
        });
        if same
            && !existing
                .events
                .iter()
                .any(|event| matches!(event.event, SessionEventKind::SessionEnded))
        {
            return Ok(false);
        }
        return Err((
            "contact_session_conflict",
            error_response(
                StatusCode::CONFLICT,
                ErrorCode::SessionBindingConflict,
                "原会话的联系人身份或登记已变化。",
                Some(json!({"reason":"contact_session_conflict"})),
            ),
        ));
    }

    if request.require_existing {
        return Err((
            "contact_session_missing",
            error_response(
                StatusCode::CONFLICT,
                ErrorCode::SessionBindingConflict,
                "运行时未找到之前的联系人会话。",
                Some(json!({"reason":"contact_session_missing"})),
            ),
        ));
    }
    let mut session = Session::new(session_id, clock);
    session.set_agent_binding(binding, clock);
    manager.save(&session).await.map_err(storage_failure)?;
    Ok(true)
}

pub(crate) async fn verify_contact(
    manager: &SessionManager,
    session: SessionId,
    expected_contact: ContactId,
) -> Result<(), RuntimeError> {
    if manager.load(&session).await?.is_some_and(|session| {
        session
            .agent_binding()
            .is_some_and(|binding| binding.contact_id() == expected_contact)
    }) {
        return Ok(());
    }
    Err(RuntimeError::AgentBindingRejected {
        session,
        reason: "原会话未绑定此联系人。".into(),
    })
}
