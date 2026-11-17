//! 会话 agent 绑定合同面 (B3b-1 批)。
//!
//! 端点 (一张面, 不另开别名):
//! - `GET /v1/sessions/{session_id}/agent-binding` — 读这张会话当前绑定的 agent
//! - `POST /v1/sessions/{session_id}/agent-binding` — 绑定 / 更新 / 解绑
//!
//! 契约:
//! 1. **读面只回显已存事实**: GET 回会话上存着的那份绑定 (9 个字段) + 外层三项;
//!    未绑定 ⇒ `bound=false` 且 `binding=null`。读面**不生成**任何字段。
//! 2. **写面必须带 `expected_revision`**: 缺失 ⇒ 400 且**不碰会话**; 不符 ⇒ 409
//!    `session_binding_conflict` (骑会话自己的修订号, 不是绑定的修订号)。
//! 3. **解绑要显式 `null`**: 请求体写 `contact_id: null` ⇒ 解绑; **字段缺失** ⇒
//!    400 (不猜客户端意图)。
//! 4. **绑定要过注册表**: 联系人 / 工作区必须存在、未归档 / 未吊销, 且修订号精确
//!    匹配 —— 判定交给领域层, 面只翻译错误。
//! 5. **闸读的是同一份事实**: 面与回合闸共用**同一个**注册表实例 (组合根开一份,
//!    同时注入运行时与本面) ⇒ 闸看到的永远是这张面刚写下的事实。
//! 6. **allow / deny 都留痕**: 事件 `gateway.session_binding_action`, 动作词
//!    `session_binding.create|update|unbind|deny`。
//!
//! ⚠️ `context_key` 与 `incarnation` 是**宿主持有的回显字段**: 面既不生成它们,
//! 也不改写它们 —— 更新时省略即沿用已存的那份, 给了不同值则由领域层判冲突。

use std::str::FromStr;

use apeireth_core::kernel::ids::{ContactAgentId, ContactId, SessionId, WorkspaceId};
use apeireth_core::kernel::Timestamp;
use apeireth_runtime::canonical::agent_binding::{
    rebind_agent_binding, resolve_agent_binding, AgentBinding, AgentBindingError,
    AgentBindingRequest,
};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{identity_label, ClientIdentity};
use crate::domain_context::DomainContext;
use crate::error_frame::{ErrorCode, ErrorEnvelope, ErrorFrame};
use crate::panels::GatewayState;

/// 会话绑定审计事件名 (allow / deny 共用一条事件, 动作词区分)。
pub const SESSION_BINDING_AUDIT_EVENT: &str = "gateway.session_binding_action";
/// 审计动作词：新建绑定。
pub const SESSION_BINDING_ACTION_CREATE: &str = "session_binding.create";
/// 审计动作词：更新绑定。
pub const SESSION_BINDING_ACTION_UPDATE: &str = "session_binding.update";
/// 审计动作词：解绑。
pub const SESSION_BINDING_ACTION_UNBIND: &str = "session_binding.unbind";
/// 审计动作词：拒绝一次绑定动作。
pub const SESSION_BINDING_ACTION_DENY: &str = "session_binding.deny";

/// 端点标签 (进审计明细)。
const SESSION_BINDING_ENDPOINT: &str = "/v1/sessions/:session_id/agent-binding";
/// 404 恒定文案：不区分「不存在」与「不可见」。
const CONTACT_NOT_FOUND_MESSAGE: &str = "联系人不存在或不可见";

// ---------------------------------------------------------------------------
// 读面 DTO（字段写死，不增不删）
// ---------------------------------------------------------------------------

/// 一份会话绑定的读面投影（9 个字段，与运行时 `AgentBinding` 同名同义）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBindingDto {
    /// 绑定的联系人身份。
    pub contact_id: String,
    /// 联系人背后的 agent 身份。
    pub agent_id: String,
    /// 绑定时人设的修订号。
    pub persona_revision: u64,
    /// 绑定的工作区身份。
    pub workspace_id: String,
    /// 绑定时工作区的修订号。
    pub workspace_revision: u64,
    /// 宿主持有的上下文键（只透传，面不生成）。
    pub context_key: Option<String>,
    /// 宿主持有的化身标记（只透传，面不生成）。
    pub incarnation: Option<String>,
    /// 绑定自身的修订号（不是会话的修订号）。
    pub revision: u64,
    /// 绑定最后一次落定的时刻（Unix 毫秒）。
    pub updated_at_ms: u64,
}

impl AgentBindingDto {
    fn from_binding(binding: &AgentBinding) -> Self {
        Self {
            contact_id: binding.contact_id().to_string(),
            agent_id: binding.agent_id().to_string(),
            persona_revision: binding.persona_revision(),
            workspace_id: binding.workspace_id().to_string(),
            workspace_revision: binding.workspace_revision(),
            context_key: binding.context_key().map(str::to_string),
            incarnation: binding.incarnation().map(str::to_string),
            revision: binding.revision(),
            updated_at_ms: binding.updated_at_ms(),
        }
    }
}

/// 会话绑定的读面回答（GET 与 POST 回读同形）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBindingDto {
    /// 这张会话的身份。
    pub session_id: String,
    /// 是否已绑定。
    pub bound: bool,
    /// 已绑定的内容；未绑定 ⇒ `null`。
    pub binding: Option<AgentBindingDto>,
    /// 会话当前的修订号（下次写面要带的就是它）。
    pub session_revision: u64,
}

impl SessionBindingDto {
    fn from_session(session_id: &str, session: &apeireth_runtime::canonical::Session) -> Self {
        let binding = session.agent_binding().map(AgentBindingDto::from_binding);
        Self {
            session_id: session_id.to_string(),
            bound: binding.is_some(),
            binding,
            session_revision: session.revision,
        }
    }
}

/// 写面请求体。
///
/// `contact_id` 用双层 `Option` 区分三态：**缺失**（⇒ 400）/ **显式 `null`**
/// （⇒ 解绑）/ **有值**（⇒ 绑定或更新）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBindingWriteRequest {
    /// 会话修订号 (CAS): 缺失 ⇒ 400，不符 ⇒ 409。
    #[serde(default)]
    pub expected_revision: Option<u64>,
    /// 要绑定的联系人；显式 `null` ⇒ 解绑。
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub contact_id: Option<Option<String>>,
    /// 联系人背后的 agent 身份（调用方声称的事实）。
    #[serde(default)]
    pub agent_id: Option<String>,
    /// 声称的人设修订号。
    #[serde(default)]
    pub persona_revision: Option<u64>,
    /// 要绑定的工作区身份。
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// 声称的工作区修订号。
    #[serde(default)]
    pub workspace_revision: Option<u64>,
    /// 宿主持有的上下文键（只透传）。
    #[serde(default)]
    pub context_key: Option<String>,
    /// 宿主持有的化身标记（只透传）。
    #[serde(default)]
    pub incarnation: Option<String>,
}

/// 双层 `Option` 反序列化：区分「字段缺失」与「显式 `null`」。
///
/// 本面只反序列化 `contact_id`（字符串），故实现按字符串定死：
/// 缺失 ⇒ `None`（不动）；显式 `null` ⇒ `Some(None)`（解绑）；字符串 ⇒ `Some(Some(v))`。
mod double_option {
    use serde::{Deserialize, Deserializer};

    /// 字段缺失 ⇒ `None`；显式 `null` ⇒ `Some(None)`；有值 ⇒ `Some(Some(v))`。
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Option<Option<String>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("null 或字符串")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Some(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                String::deserialize(deserializer).map(Some).map(Some)
            }

            // JSON 字符串走这里（`deserialize_any` 不会把它当 `visit_some`）。
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Some(Some(value.to_owned())))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Some(Some(value)))
            }
        }

        // `deserialize_any` 而不是 `deserialize_option`: 后者把显式 `null`
        // 也报成 `visit_none`, 会把「解绑」和「不动」混成同一态。
        deserializer.deserialize_any(Visitor)
    }
}

// ---------------------------------------------------------------------------
// 处理器
// ---------------------------------------------------------------------------

/// `GET /v1/sessions/{session_id}/agent-binding` — 读这张会话的绑定。
///
/// 未绑定 ⇒ 200 + `bound=false` 且 `binding=null`（不是 404：会话在，只是没绑定）。
pub async fn get_session_binding(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
) -> Response {
    if let Err(refusal) = crate::domain_context::request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        SESSION_BINDING_ENDPOINT,
    ) {
        return refusal;
    }
    let Ok(session_id) = SessionId::from_str(&session_id) else {
        return invalid_request_frame("session_id 格式非法");
    };
    let manager = match crate::session_settings::session_manager(&state, domain.as_ref()) {
        Ok(manager) => manager,
        Err(refusal) => return refusal.into_response(),
    };
    let loaded = match manager.load(&session_id).await {
        Ok(loaded) => loaded,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                format!("会话读取失败: {error}"),
                None,
            )
        }
    };
    let Some(session) = loaded else {
        return session_not_found_frame(&session_id.to_string());
    };
    (
        StatusCode::OK,
        Json(SessionBindingDto::from_session(
            &session_id.to_string(),
            &session,
        )),
    )
        .into_response()
}

/// `POST /v1/sessions/{session_id}/agent-binding` — 绑定 / 更新 / 解绑。
///
/// 写面三态：`contact_id` 有值 ⇒ 绑定或更新；显式 `null` ⇒ 解绑；字段缺失 ⇒ 400。
pub async fn post_session_binding(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
    body: Result<Json<SessionBindingWriteRequest>, JsonRejection>,
) -> Response {
    if let Err(refusal) = crate::domain_context::request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        SESSION_BINDING_ENDPOINT,
    ) {
        return refusal;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let client_label = identity_label(client.as_ref());
    let Ok(session_id) = SessionId::from_str(&session_id) else {
        return deny(
            &state,
            "",
            "",
            "malformed_session_id",
            client_label,
            invalid_request_frame("session_id 格式非法"),
        )
        .await;
    };
    let session_label = session_id.to_string();
    let Some(expected_revision) = request.expected_revision else {
        return deny(
            &state,
            &session_label,
            "",
            "missing_expected_revision",
            client_label,
            invalid_request_frame("expected_revision 必填: 写面必须带上读面读到的会话修订号"),
        )
        .await;
    };
    let Some(contact_id) = request.contact_id.clone() else {
        return deny(
            &state,
            &session_label,
            "",
            "missing_contact_id",
            client_label,
            invalid_request_frame("contact_id 必填: 想解绑就显式写 contact_id: null"),
        )
        .await;
    };
    let manager = match crate::session_settings::session_manager(&state, domain.as_ref()) {
        Ok(manager) => manager,
        Err(refusal) => return refusal.into_response(),
    };
    let loaded = match manager.load(&session_id).await {
        Ok(loaded) => loaded,
        Err(error) => {
            return deny(
                &state,
                &session_label,
                "",
                "session_store_error",
                client_label,
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::Internal,
                    format!("会话读取失败: {error}"),
                    None,
                ),
            )
            .await
        }
    };
    let Some(session) = loaded else {
        return deny(
            &state,
            &session_label,
            "",
            "session_not_found",
            client_label,
            session_not_found_frame(&session_label),
        )
        .await;
    };
    if session.revision != expected_revision {
        return deny(
            &state,
            &session_label,
            "",
            "session_revision_conflict",
            client_label,
            session_conflict_frame(expected_revision, session.revision),
        )
        .await;
    }

    let clock = manager.clock().as_ref();
    let now_ms = Timestamp::from_clock(clock).epoch_millis() as u64;
    let Some(contact) = contact_id else {
        return unbind_session(
            &state,
            manager,
            session_id,
            &session_label,
            expected_revision,
            client_label,
        )
        .await;
    };
    bind_session(
        &state,
        manager,
        session_id,
        &session_label,
        expected_revision,
        contact,
        request,
        clock,
        now_ms,
        client_label,
    )
    .await
}

/// 解绑: 显式 `contact_id: null`。
async fn unbind_session(
    state: &GatewayState,
    manager: &apeireth_runtime::canonical::SessionManager,
    session_id: SessionId,
    session_label: &str,
    expected_revision: u64,
    client_label: &str,
) -> Response {
    let clock = manager.clock().as_ref();
    let updated = match manager
        .update(session_id, |session| {
            if session.revision != expected_revision {
                return Err(BindingFailure::SessionConflict {
                    expected: expected_revision,
                    actual: session.revision,
                });
            }
            if session.agent_binding().is_none() {
                return Err(BindingFailure::NotBound);
            }
            session.clear_agent_binding(clock);
            Ok(SessionBindingDto::from_session(session_label, session))
        })
        .await
    {
        Ok(updated) => updated,
        Err(error) => {
            return deny(
                state,
                session_label,
                "",
                "session_store_error",
                client_label,
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::Internal,
                    format!("会话写入失败: {error}"),
                    None,
                ),
            )
            .await
        }
    };
    let Some((result, _admission)) = updated else {
        return deny(
            state,
            session_label,
            "",
            "session_not_found",
            client_label,
            session_not_found_frame(session_label),
        )
        .await;
    };
    match result {
        Ok(dto) => {
            record_session_binding_audit(
                state,
                SESSION_BINDING_ACTION_UNBIND,
                session_label,
                "",
                "allow",
                None,
                client_label,
            )
            .await;
            (StatusCode::OK, Json(dto)).into_response()
        }
        Err(failure) => {
            deny(
                state,
                session_label,
                "",
                failure.reason_label(),
                client_label,
                failure.into_response(),
            )
            .await
        }
    }
}

/// 绑定或更新: 领域层判身份, 面只翻译错误。
#[allow(clippy::too_many_arguments)]
async fn bind_session(
    state: &GatewayState,
    manager: &apeireth_runtime::canonical::SessionManager,
    session_id: SessionId,
    session_label: &str,
    expected_revision: u64,
    contact: String,
    request: SessionBindingWriteRequest,
    clock: &dyn apeireth_core::kernel::Clock,
    now_ms: u64,
    client_label: &str,
) -> Response {
    let Ok(contact_id) = contact.parse::<ContactId>() else {
        return deny(
            state,
            session_label,
            "",
            "malformed_contact_id",
            client_label,
            invalid_request_frame("contact_id 格式非法"),
        )
        .await;
    };
    let (Some(agent_id), Some(persona_revision), Some(workspace_id), Some(workspace_revision)) = (
        request.agent_id.clone(),
        request.persona_revision,
        request.workspace_id.clone(),
        request.workspace_revision,
    ) else {
        return deny(
            state,
            session_label,
            &contact,
            "incomplete_binding_request",
            client_label,
            invalid_request_frame(
                "绑定必须给出 agent_id / persona_revision / workspace_id / workspace_revision",
            ),
        )
        .await;
    };
    let Ok(agent_id) = agent_id.parse::<ContactAgentId>() else {
        return deny(
            state,
            session_label,
            &contact,
            "malformed_agent_id",
            client_label,
            invalid_request_frame("agent_id 格式非法"),
        )
        .await;
    };
    let Ok(workspace_id) = workspace_id.parse::<WorkspaceId>() else {
        return deny(
            state,
            session_label,
            &contact,
            "malformed_workspace_id",
            client_label,
            invalid_request_frame("workspace_id 格式非法"),
        )
        .await;
    };
    // 注册表没装配 ⇒ 这张面无从核对事实 ⇒ 拒 (不写半份绑定)。
    let (Some(contacts), Some(workspaces)) = (
        state.runtime.contact_registry(),
        state.runtime.workspace_registry(),
    ) else {
        return deny(
            state,
            session_label,
            &contact,
            "registry_not_installed",
            client_label,
            crate::panels::unsupported("session_agent_binding.write"),
        )
        .await;
    };
    let contacts = contacts.as_ref();
    let workspaces = workspaces.as_ref();
    let claimed = AgentBindingRequest {
        contact_id,
        agent_id,
        persona_revision,
        workspace_id,
        workspace_revision,
        context_key: request.context_key.clone(),
        incarnation: request.incarnation.clone(),
    };
    let updated = match manager
        .update(session_id, |session| {
            if session.revision != expected_revision {
                return Err(BindingFailure::SessionConflict {
                    expected: expected_revision,
                    actual: session.revision,
                });
            }
            let (binding, action) = match session.agent_binding() {
                Some(existing) => {
                    // 更新: 回显字段省略即沿用已存的那份 (面不生成, 只透传)。
                    let mut effective = claimed.clone();
                    if effective.context_key.is_none() {
                        effective.context_key = existing.context_key().map(str::to_string);
                    }
                    if effective.incarnation.is_none() {
                        effective.incarnation = existing.incarnation().map(str::to_string);
                    }
                    let bound = rebind_agent_binding(
                        existing,
                        &effective,
                        existing.revision(),
                        contacts,
                        workspaces,
                        now_ms,
                    );
                    (bound, SESSION_BINDING_ACTION_UPDATE)
                }
                None => (
                    resolve_agent_binding(&claimed, contacts, workspaces, now_ms),
                    SESSION_BINDING_ACTION_CREATE,
                ),
            };
            match binding {
                Ok(binding) => {
                    session.set_agent_binding(binding, clock);
                    Ok((
                        SessionBindingDto::from_session(session_label, session),
                        action,
                    ))
                }
                Err(error) => Err(BindingFailure::Domain(error)),
            }
        })
        .await
    {
        Ok(updated) => updated,
        Err(error) => {
            return deny(
                state,
                session_label,
                &contact,
                "session_store_error",
                client_label,
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::Internal,
                    format!("会话写入失败: {error}"),
                    None,
                ),
            )
            .await
        }
    };
    let Some((result, _admission)) = updated else {
        return deny(
            state,
            session_label,
            &contact,
            "session_not_found",
            client_label,
            session_not_found_frame(session_label),
        )
        .await;
    };
    match result {
        Ok((dto, action)) => {
            record_session_binding_audit(
                state,
                action,
                session_label,
                &contact,
                "allow",
                None,
                client_label,
            )
            .await;
            (StatusCode::OK, Json(dto)).into_response()
        }
        Err(failure) => {
            deny(
                state,
                session_label,
                &contact,
                failure.reason_label(),
                client_label,
                failure.into_response(),
            )
            .await
        }
    }
}

/// 写面内部的一次失败 (要翻译成帧, 也要留痕)。
enum BindingFailure {
    /// 会话修订号已被别人推进 (CAS 失败)。
    SessionConflict { expected: u64, actual: u64 },
    /// 领域层拒绝了这次绑定。
    Domain(AgentBindingError),
    /// 会话当前没有绑定, 无可解绑。
    NotBound,
}

impl BindingFailure {
    fn reason_label(&self) -> &'static str {
        match self {
            BindingFailure::SessionConflict { .. } => "session_revision_conflict",
            BindingFailure::Domain(error) => domain_reason_label(error),
            BindingFailure::NotBound => "not_bound",
        }
    }

    fn into_response(self) -> Response {
        match self {
            BindingFailure::SessionConflict { expected, actual } => {
                session_conflict_frame(expected, actual)
            }
            BindingFailure::Domain(error) => {
                let (status, code, message) = domain_error_frame(&error);
                error_response(status, code, message, None)
            }
            BindingFailure::NotBound => invalid_request_frame("会话当前没有绑定: 无可解绑"),
        }
    }
}

/// 领域层绑定错误 ⇒ 审计标签。
fn domain_reason_label(error: &AgentBindingError) -> &'static str {
    match error {
        AgentBindingError::ContactNotFound(_) => "contact_not_found",
        AgentBindingError::ContactArchived(_) => "contact_archived",
        AgentBindingError::PersonaRevisionMismatch { .. } => "persona_revision_mismatch",
        AgentBindingError::WorkspaceNotFound(_) => "workspace_not_found",
        AgentBindingError::WorkspaceRevoked(_) => "workspace_revoked",
        AgentBindingError::WorkspaceRevisionMismatch { .. } => "workspace_revision_mismatch",
        AgentBindingError::BindingConflict { .. } => "binding_conflict",
        AgentBindingError::InvalidRequest { .. } => "invalid_request",
    }
}

/// 领域层绑定错误 ⇒ 一帧 (状态 / 码 / 文案)。
fn domain_error_frame(error: &AgentBindingError) -> (StatusCode, ErrorCode, String) {
    match error {
        AgentBindingError::ContactNotFound(_) => (
            StatusCode::NOT_FOUND,
            ErrorCode::ContactNotFound,
            CONTACT_NOT_FOUND_MESSAGE.to_string(),
        ),
        AgentBindingError::ContactArchived(_) => (
            StatusCode::FORBIDDEN,
            ErrorCode::ContactArchived,
            "联系人已归档 (软删除): 不可绑定".to_string(),
        ),
        AgentBindingError::PersonaRevisionMismatch { expected, actual } => (
            StatusCode::CONFLICT,
            ErrorCode::ContactConflict,
            format!("人设修订号不匹配: 期望 {expected}, 实际 {actual}"),
        ),
        AgentBindingError::WorkspaceNotFound(_) => (
            StatusCode::NOT_FOUND,
            ErrorCode::WorkspaceNotFound,
            "工作区不存在".to_string(),
        ),
        AgentBindingError::WorkspaceRevoked(_) => (
            StatusCode::CONFLICT,
            ErrorCode::WorkspaceRevoked,
            "工作区已吊销".to_string(),
        ),
        AgentBindingError::WorkspaceRevisionMismatch { expected, actual } => (
            StatusCode::CONFLICT,
            ErrorCode::WorkspaceConflict,
            format!("工作区修订号不匹配: 期望 {expected}, 实际 {actual}"),
        ),
        AgentBindingError::BindingConflict { expected, actual } => (
            StatusCode::CONFLICT,
            ErrorCode::ContactConflict,
            format!("绑定冲突: 期望 {expected}, 实际 {actual}"),
        ),
        AgentBindingError::InvalidRequest { reason } => (
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            format!("绑定请求无效: {reason}"),
        ),
    }
}

// ---------------------------------------------------------------------------
// 审计 / 错误帧 / 装配
// ---------------------------------------------------------------------------

/// 留痕一次绑定动作 (allow / deny 同事件, 动作词区分)。
///
/// 明细只带身份与判定事实 —— 绝不带路径或人设正文。
async fn record_session_binding_audit(
    state: &GatewayState,
    action: &str,
    session_id: &str,
    contact_id: &str,
    decision: &str,
    reason: Option<&str>,
    client_label: &str,
) {
    let session = if session_id.is_empty() {
        "-".to_string()
    } else {
        crate::canonical_entry::truncate_label(session_id)
    };
    let contact = if contact_id.is_empty() {
        "-".to_string()
    } else {
        crate::canonical_entry::truncate_label(contact_id)
    };
    let mut detail = format!(
        "session={session} contact={contact} action={action} endpoint={SESSION_BINDING_ENDPOINT} decision={decision} client={}",
        crate::canonical_entry::truncate_label(client_label)
    );
    if let Some(reason) = reason {
        detail.push_str(&format!(" reason={reason}"));
    }
    if let Some(audit) = state.services.audit_commands.as_ref() {
        audit
            .append_audit(SESSION_BINDING_AUDIT_EVENT, Some(&detail))
            .await;
        return;
    }
    state
        .observations
        .buffer_audit(SESSION_BINDING_AUDIT_EVENT, Some(detail));
}

/// 拒绝一次绑定动作：先留痕, 再回帧 (allow / deny 都留痕)。
async fn deny(
    state: &GatewayState,
    session_id: &str,
    contact_id: &str,
    reason: &str,
    client_label: &str,
    response: Response,
) -> Response {
    record_session_binding_audit(
        state,
        SESSION_BINDING_ACTION_DENY,
        session_id,
        contact_id,
        "deny",
        Some(reason),
        client_label,
    )
    .await;
    response
}

/// 统一错误帧（可带结构化 details）。
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

/// 400 帧（统一形状）。
fn invalid_request_frame(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        ErrorCode::InvalidRequest,
        message,
        None,
    )
}

/// 404 帧：会话不存在。
fn session_not_found_frame(session_id: &str) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        ErrorCode::SessionNotFound,
        format!("session {session_id} does not exist"),
        None,
    )
}

/// 409 帧：会话修订号已被别人推进。
fn session_conflict_frame(expected: u64, actual: u64) -> Response {
    error_response(
        StatusCode::CONFLICT,
        ErrorCode::SessionBindingConflict,
        format!("会话修订号已变: 期望 {expected}, 实际 {actual}"),
        Some(serde_json::json!({
            "expected_revision": expected,
            "actual_revision": actual,
        })),
    )
}

/// 会话绑定合同面的路由表（装配点 [`crate::panels::panel_routes`] 一行 `merge`）。
pub fn session_binding_routes() -> axum::Router<GatewayState> {
    axum::Router::new()
        .route(
            "/v1/sessions/:session_id/agent-binding",
            axum::routing::get(get_session_binding),
        )
        .route(
            "/v1/sessions/:session_id/agent-binding",
            axum::routing::post(post_session_binding),
        )
}

/// 能力清单里的会话绑定组（读/写**分开**登记）。
///
/// 读面只读会话 ⇒ 恒可用；`bind` 需要联系人/工作区注册表 ⇒ 随注册表装配状态登记；
/// `unbind` 只清会话上的绑定 ⇒ 恒可用。
pub fn session_binding_capability_group(registry_installed: bool) -> serde_json::Value {
    serde_json::json!({
        "name": "session_agent_binding",
        "capabilities": [
            crate::capabilities::cap(
                "session_agent_binding.read",
                true,
                false,
                &["get"],
                true,
                true,
            ),
            crate::capabilities::cap(
                "session_agent_binding.bind",
                false,
                true,
                &["bind"],
                registry_installed,
                registry_installed,
            ),
            crate::capabilities::cap(
                "session_agent_binding.unbind",
                false,
                true,
                &["unbind"],
                true,
                true,
            ),
        ],
    })
}
