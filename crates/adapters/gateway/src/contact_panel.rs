//! 联系人合同面 (B3a 批)。
//!
//! 端点 (一张读面, 不另开 `/v1/panel/contacts` 别名):
//! - `GET /v1/agents/contacts` — 联系人列表 (默认**不含**已归档墓碑)
//! - `POST /v1/agents/contacts` — 新建联系人 (身份由**后端铸造**)
//! - `GET /v1/agents/contacts/{contact_id}` — 单个联系人 (归档后仍可读)
//! - `PATCH /v1/agents/contacts/{contact_id}` — 改人设 / 能力投影 / 工作区绑定
//! - `POST /v1/agents/contacts/{contact_id}/archive` — 归档 (软删除, **终态**)
//!
//! 契约七条 (与工作区面同形, 但**不是**同一张面):
//! 1. **身份由后端铸造**: 请求体带 `contact_id` / `agent_id` ⇒ 400 (不是静默忽略)。
//! 2. **归档是终态**: `status=archived` 之后不可复活 (再 PATCH ⇒ 403 `contact_archived`)。
//! 3. **乐观并发**: 写面必须带 `expected_revision`; 缺失 ⇒ 400 且不调端口;
//!    不符 ⇒ 409 `contact_conflict` + `details{expected_revision, actual_revision}`。
//! 4. **读面不含存储位置**: DTO 只有固定字段, 没有路径 / 内部快照 / 宿主私有字段。
//! 5. **404 恒定文案**: 「联系人不存在或不可见」——不泄露存在性; 与 403 (归档) 稳定可判。
//! 6. **绑定只存 id+revision**: 写面校验工作区存在 / 未吊销 / 修订号精确匹配。
//! 7. **allow / deny 都留痕**: 事件 `gateway.contact_action`,
//!    动作词 `contact.create|update|archive|deny`。
//!
//! ⛔ 能力投影 (`capabilities`) 只是**展示投影**, **不是授权依据**; 真正的授权
//! 判定走治理层, 与本面无关 (设计稿 §未决第 5 条)。

use apeireth_core::agent_contact::PersonaText;
use apeireth_core::kernel::ids::ContactId;
use apeireth_runtime::canonical::workspace::WorkspaceStatus;
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

/// 联系人审计事件名 (allow / deny 共用一条事件, 动作词区分)。
pub const CONTACT_AUDIT_EVENT: &str = "gateway.contact_action";
/// 审计动作词：新建联系人。
pub const CONTACT_ACTION_CREATE: &str = "contact.create";
/// 审计动作词：修改联系人。
pub const CONTACT_ACTION_UPDATE: &str = "contact.update";
/// 审计动作词：归档联系人。
pub const CONTACT_ACTION_ARCHIVE: &str = "contact.archive";
/// 审计动作词：拒绝一次联系人动作。
pub const CONTACT_ACTION_DENY: &str = "contact.deny";

/// 404 恒定文案：不区分「不存在」与「不可见」。
const CONTACT_NOT_FOUND_MESSAGE: &str = "联系人不存在或不可见";

// ---------------------------------------------------------------------------
// 读面 DTO（字段写死，不增不删）
// ---------------------------------------------------------------------------

/// 联系人合同态（**与运行时 `AgentStatus` 是两套枚举**，投影时显式映射，不合并）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactStatusDto {
    /// 可用。
    Ready,
    /// 草稿（刚建、还没写人设）。
    Draft,
    /// 已归档（软删除墓碑，终态）。
    Archived,
    /// 暂不可用。
    Unavailable,
}

/// 合同层记忆作用域（DTO 层枚举；到运行时 `MemoryScope` 的翻译属 B3b 装配层）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactMemoryScopeDto {
    /// 记在主体上。
    Subject,
    /// 记在联系人上。
    Contact,
    /// 记在会话上。
    Session,
    /// 不写记忆。
    None,
}

/// 能力**展示投影**（⚠️ 不是授权依据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactCapabilitiesDto {
    /// 展示用：可读。
    pub read: bool,
    /// 展示用：可写。
    pub write: bool,
    /// 展示用：写入需要人工审批。
    pub approval_required: bool,
}

/// 工作区绑定（**只存 id + 修订号**，不存工作区快照）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactWorkspaceBindingDto {
    /// 绑定的工作区身份。
    pub workspace_id: String,
    /// 绑定时该工作区的修订号。
    pub revision: u64,
}

/// 一个联系人的读面投影。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactDto {
    /// 联系人身份（后端铸造）。
    pub contact_id: String,
    /// 该联系人绑定的 agent 身份（后端铸造）。
    pub agent_id: String,
    /// 展示名（标签，不唯一）。
    pub display_name: String,
    /// 副标题（可选）。
    pub subtitle: Option<String>,
    /// 头像引用（可选）。
    pub avatar_ref: Option<String>,
    /// 合同态。
    pub status: ContactStatusDto,
    /// 人设版本号（后端递增）。
    pub persona_revision: u64,
    /// 合同层记忆作用域。
    pub memory_scope: ContactMemoryScopeDto,
    /// 工作区绑定（可空）。
    pub workspace_binding: Option<ContactWorkspaceBindingDto>,
    /// 能力展示投影（不是授权依据）。
    pub capabilities: ContactCapabilitiesDto,
    /// 人设正文（可选）。
    pub persona: Option<String>,
    /// 记录修订号（乐观并发用）。
    pub revision: u64,
    /// 最后写入时间（epoch 毫秒）。
    pub updated_at_ms: u64,
}

/// 联系人列表响应体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactListDto {
    /// 联系人行。
    pub contacts: Vec<ContactDto>,
    /// 行数。
    pub count: u64,
}

/// 单个联系人响应体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactEnvelope {
    /// 联系人。
    pub contact: ContactDto,
}

// ---------------------------------------------------------------------------
// 端口命令
// ---------------------------------------------------------------------------

/// 新建联系人的端口命令（**不含任何身份字段**：身份由后端铸造）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactCreateCommand {
    /// 展示名。
    pub display_name: String,
    /// 副标题。
    pub subtitle: Option<String>,
    /// 头像引用。
    pub avatar_ref: Option<String>,
    /// 已校验的人设文本。
    pub persona: Option<PersonaText>,
    /// 合同层记忆作用域。
    pub memory_scope: Option<ContactMemoryScopeDto>,
    /// 工作区绑定声明。
    pub workspace_binding: Option<ContactBindingCommand>,
}

/// 工作区绑定声明（只存 id + 修订号）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactBindingCommand {
    /// 绑定的工作区身份。
    pub workspace_id: String,
    /// 绑定时该工作区的修订号（必须精确匹配）。
    pub revision: u64,
}

/// 能力展示投影的写入命令。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactCapabilitiesCommand {
    /// 展示用：可读。
    pub read: bool,
    /// 展示用：可写。
    pub write: bool,
    /// 展示用：写入需要人工审批。
    pub approval_required: bool,
}

/// 绑定变更意图：`Keep` = 不动，`Clear` = 解绑，`Bind` = 改绑。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactBindingChange {
    /// 请求里没提绑定 ⇒ 保持原样。
    Keep,
    /// 请求里显式给了 `null` ⇒ 解绑。
    Clear,
    /// 请求里给了绑定声明 ⇒ 改绑（服务端先校验工作区）。
    Bind(ContactBindingCommand),
}

/// 修改联系人的端口命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactUpdateCommand {
    /// 联系人身份。
    pub contact_id: String,
    /// 期望修订号。
    pub expected_revision: u64,
    /// 新人设（`None` = 不动人设）。
    pub persona: Option<PersonaText>,
    /// 新能力展示投影（`None` = 不动）。
    pub capabilities: Option<ContactCapabilitiesCommand>,
    /// 绑定变更。
    pub workspace_binding: ContactBindingChange,
}

/// 归档联系人的端口命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactArchiveCommand {
    /// 联系人身份。
    pub contact_id: String,
    /// 期望修订号。
    pub expected_revision: u64,
}

// ---------------------------------------------------------------------------
// 请求体
// ---------------------------------------------------------------------------

/// `GET /v1/agents/contacts` 查询参数。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ContactListQuery {
    /// 显式带上 `include_archived=true` 才看得到已归档的墓碑。
    #[serde(default)]
    pub include_archived: Option<bool>,
}

/// `POST /v1/agents/contacts` 请求体（**未知字段一律 400**）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactCreateRequest {
    /// 展示名（必填）。
    pub display_name: String,
    /// 副标题（可选）。
    #[serde(default)]
    pub subtitle: Option<String>,
    /// 头像引用（可选）。
    #[serde(default)]
    pub avatar_ref: Option<String>,
    /// 人设正文（可选；校验走领域层 `PersonaText`）。
    #[serde(default)]
    pub persona: Option<String>,
    /// 合同层记忆作用域（可选）。
    #[serde(default)]
    pub memory_scope: Option<ContactMemoryScopeDto>,
    /// 工作区绑定声明（可选）。
    #[serde(default)]
    pub workspace_binding: Option<ContactWorkspaceBindingRequest>,
}

/// 工作区绑定声明（请求面）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactWorkspaceBindingRequest {
    /// 绑定的工作区身份。
    pub workspace_id: String,
    /// 绑定时该工作区的修订号。
    pub revision: u64,
}

/// `PATCH /v1/agents/contacts/{id}` 请求体（**未知字段一律 400**）。
///
/// `workspace_binding` 用**双层 `Option`** 区分三态：字段缺失 = 不动绑定，
/// 显式 `null` = 解绑，给出对象 = 改绑。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactUpdateRequest {
    /// 期望修订号（**必填**）。
    #[serde(default)]
    pub expected_revision: Option<u64>,
    /// 新人设（可选）。
    #[serde(default)]
    pub persona: Option<String>,
    /// 新能力展示投影（可选）。
    #[serde(default)]
    pub capabilities: Option<ContactCapabilitiesRequest>,
    /// 绑定变更（可选）。
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub workspace_binding: Option<Option<ContactWorkspaceBindingRequest>>,
}

/// 能力展示投影（请求面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactCapabilitiesRequest {
    /// 展示用：可读。
    pub read: bool,
    /// 展示用：可写。
    pub write: bool,
    /// 展示用：写入需要人工审批。
    pub approval_required: bool,
}

/// `POST /v1/agents/contacts/{id}/archive` 请求体（**未知字段一律 400**）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactRevisionRequest {
    /// 期望修订号（**必填**）。
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

/// 双层 `Option` 反序列化：区分「字段缺失」与「显式 `null`」。
mod double_option {
    use serde::{Deserialize, Deserializer};
    use std::marker::PhantomData;

    /// 字段缺失 ⇒ `None`；显式 `null` ⇒ `Some(None)`；有值 ⇒ `Some(Some(v))`。
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = Option<Option<T>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("null 或绑定对象")
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
                T::deserialize(deserializer).map(Some).map(Some)
            }

            // JSON 对象走这里（`deserialize_any` 不会把它当 `visit_some`）。
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
                    .map(Some)
                    .map(Some)
            }
        }

        // `deserialize_any` 而不是 `deserialize_option`: 后者把显式 `null`
        // 也报成 `visit_none`, 会把「解绑」和「不动」混成同一态。
        deserializer.deserialize_any(Visitor(PhantomData))
    }
}

// ---------------------------------------------------------------------------
// 错误
// ---------------------------------------------------------------------------

/// 联系人面的错误分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactPanelError {
    /// 不存在或不可见（404，恒定文案）。
    NotFound {
        /// 联系人身份。
        contact_id: String,
    },
    /// 修订号冲突（409）。
    Conflict {
        /// 联系人身份。
        contact_id: String,
        /// 期望修订号。
        expected_revision: u64,
        /// 实际修订号。
        actual_revision: u64,
    },
    /// 已归档（403，终态：不是冲突）。
    Archived {
        /// 联系人身份。
        contact_id: String,
    },
    /// 工作区绑定冲突（409：工作区不存在 / 已吊销 / 修订号不匹配）。
    BindingConflict {
        /// 绑定声明里的工作区身份。
        workspace_id: String,
        /// 稳定的机器可读原因标签。
        reason: String,
    },
    /// 请求不合法（400）。
    InvalidRequest {
        /// 原因（**不含**宿主路径）。
        reason: String,
    },
    /// 服务端故障（500）。
    Internal {
        /// 原因（**不含**宿主路径）。
        reason: String,
    },
}

impl ContactPanelError {
    /// 错误码。
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound { .. } => ErrorCode::ContactNotFound,
            Self::Conflict { .. } => ErrorCode::ContactConflict,
            Self::Archived { .. } => ErrorCode::ContactArchived,
            Self::BindingConflict { .. } => ErrorCode::ContactBindingConflict,
            Self::InvalidRequest { .. } => ErrorCode::InvalidRequest,
            Self::Internal { .. } => ErrorCode::Internal,
        }
    }

    /// HTTP 状态码。
    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } | Self::BindingConflict { .. } => StatusCode::CONFLICT,
            Self::Archived { .. } => StatusCode::FORBIDDEN,
            Self::InvalidRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 审计用的稳定原因标签（**不含**宿主路径）。
    pub fn reason_label(&self) -> String {
        match self {
            Self::NotFound { .. } => "contact_not_found".to_string(),
            Self::Conflict { .. } => "revision_conflict".to_string(),
            Self::Archived { .. } => "contact_archived".to_string(),
            Self::BindingConflict { reason, .. } => format!("binding_conflict:{reason}"),
            Self::InvalidRequest { .. } => "invalid_request".to_string(),
            Self::Internal { .. } => "internal_error".to_string(),
        }
    }

    /// 统一错误帧。
    pub fn into_response(self) -> Response {
        match &self {
            Self::NotFound { .. } => {
                error_response(self.status(), self.code(), CONTACT_NOT_FOUND_MESSAGE, None)
            }
            Self::Conflict {
                expected_revision,
                actual_revision,
                ..
            } => error_response(
                self.status(),
                self.code(),
                format!("联系人修订号冲突: 期望 {expected_revision}, 实际 {actual_revision}"),
                Some(serde_json::json!({
                    "expected_revision": expected_revision,
                    "actual_revision": actual_revision,
                })),
            ),
            Self::Archived { .. } => error_response(
                self.status(),
                self.code(),
                "联系人已归档 (软删除), 不可修改也不可复活",
                None,
            ),
            Self::BindingConflict {
                workspace_id,
                reason,
                ..
            } => error_response(
                self.status(),
                self.code(),
                "工作区绑定冲突: 工作区不存在、已吊销或修订号不匹配",
                Some(serde_json::json!({
                    "workspace_id": workspace_id,
                    "reason": reason,
                })),
            ),
            Self::InvalidRequest { reason } => {
                error_response(self.status(), self.code(), reason.clone(), None)
            }
            // 服务端故障：**只回适配器给的固定文案**（适配器已把宿主路径剥掉），
            // 这里不拼前缀 —— 免得出现「联系人存储故障: 联系人存储故障」这种叠词。
            Self::Internal { reason } => {
                error_response(self.status(), self.code(), reason.clone(), None)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 端口
// ---------------------------------------------------------------------------

/// 联系人面端口（组合根注入真实注册表；未注入 = 501 `unsupported`）。
#[async_trait]
pub trait ContactPanel: Send + Sync {
    /// 列出联系人（`include_archived` 为真时含墓碑）。
    async fn list_contacts(&self, include_archived: bool) -> Result<Vec<ContactDto>, String>;

    /// 取一个联系人（含归档墓碑）。
    async fn get_contact(&self, contact_id: &str) -> Result<ContactDto, ContactPanelError>;

    /// 新建联系人（身份由后端铸造）。
    async fn create_contact(
        &self,
        command: ContactCreateCommand,
    ) -> Result<ContactDto, ContactPanelError>;

    /// 修改联系人。
    async fn update_contact(
        &self,
        command: ContactUpdateCommand,
    ) -> Result<ContactDto, ContactPanelError>;

    /// 归档联系人（软删除，终态）。
    async fn archive_contact(
        &self,
        command: ContactArchiveCommand,
    ) -> Result<ContactDto, ContactPanelError>;
}

// ---------------------------------------------------------------------------
// 处理器
// ---------------------------------------------------------------------------

/// `GET /v1/agents/contacts` — 联系人列表。
///
/// 三态分开报：未装配 (501 `unsupported`)、空列表 (200 + `count: 0`)、
/// 端口故障 (500)。默认**不含**已归档墓碑；`include_archived=true` 才含。
pub async fn list_contacts(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    query: Result<Query<ContactListQuery>, QueryRejection>,
) -> Response {
    if let Err(refusal) = crate::domain_context::request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/agents/contacts",
    ) {
        return refusal;
    }
    let Some(panel) = state.services.contacts.as_ref() else {
        return crate::panels::unsupported("contacts.read");
    };
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => {
            return invalid_request_frame(format!("查询参数无效: {}", rejection.body_text()));
        }
    };
    let include_archived = query.include_archived.unwrap_or(false);
    let rows = match panel.list_contacts(include_archived).await {
        Ok(rows) => rows,
        Err(reason) => return crate::panels::panel_error(reason),
    };
    let mut contacts = Vec::with_capacity(rows.len());
    for row in rows {
        // 面自己守着「默认不含墓碑」这条合同：端口多给了墓碑, 面也不放行
        // (端口与面各守一道, 免得某一侧漏了就泄漏墓碑)。
        if !include_archived && row.status == ContactStatusDto::Archived {
            continue;
        }
        match guard_contact_dto(row) {
            Ok(row) => contacts.push(row),
            Err(response) => return response,
        }
    }
    let count = contacts.len() as u64;
    (StatusCode::OK, Json(ContactListDto { contacts, count })).into_response()
}

/// `GET /v1/agents/contacts/{contact_id}` — 一个联系人。
///
/// 归档后**仍可读**（墓碑是事实，不是消失）。
pub async fn get_contact(
    State(state): State<GatewayState>,
    Path(contact_id): Path<String>,
) -> Response {
    let Some(panel) = state.services.contacts.as_ref() else {
        return crate::panels::unsupported("contacts.read");
    };
    if contact_id.parse::<ContactId>().is_err() {
        return invalid_request_frame("contact_id 格式非法");
    }
    match panel.get_contact(&contact_id).await {
        Ok(dto) => match guard_contact_dto(dto) {
            Ok(dto) => (StatusCode::OK, Json(ContactEnvelope { contact: dto })).into_response(),
            Err(response) => response,
        },
        Err(error) => error.into_response(),
    }
}

/// `POST /v1/agents/contacts` — 新建联系人。
///
/// 身份由**后端铸造**：请求体里出现 `contact_id` / `agent_id` ⇒ 400（不是静默忽略）。
pub async fn create_contact(
    State(state): State<GatewayState>,
    client: Option<Extension<ClientIdentity>>,
    body: Result<Json<ContactCreateRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let Some(panel) = state.services.contacts.as_ref() else {
        return crate::panels::unsupported("contacts.write");
    };
    let client_label = identity_label(client.as_ref());
    let command = match build_create_command(request) {
        Ok(command) => command,
        Err(reason) => {
            record_contact_audit(
                &state,
                CONTACT_ACTION_DENY,
                "/v1/agents/contacts",
                "",
                "deny",
                Some("invalid_request"),
                client_label,
            )
            .await;
            return invalid_request_frame(reason);
        }
    };
    if let Some(binding) = command.workspace_binding.as_ref() {
        if let Err(response) = check_workspace_binding(&state, binding).await {
            record_contact_audit(
                &state,
                CONTACT_ACTION_DENY,
                "/v1/agents/contacts",
                "",
                "deny",
                Some("binding_conflict"),
                client_label,
            )
            .await;
            return response;
        }
    }
    match panel.create_contact(command).await {
        Ok(dto) => {
            let dto = match guard_contact_dto(dto) {
                Ok(dto) => dto,
                Err(response) => return response,
            };
            record_contact_audit(
                &state,
                CONTACT_ACTION_CREATE,
                "/v1/agents/contacts",
                &dto.contact_id,
                "allow",
                None,
                client_label,
            )
            .await;
            (StatusCode::CREATED, Json(ContactEnvelope { contact: dto })).into_response()
        }
        Err(error) => {
            record_contact_audit(
                &state,
                CONTACT_ACTION_DENY,
                "/v1/agents/contacts",
                "",
                "deny",
                Some(&error.reason_label()),
                client_label,
            )
            .await;
            error.into_response()
        }
    }
}

/// `PATCH /v1/agents/contacts/{contact_id}` — 改人设 / 能力投影 / 绑定。
pub async fn update_contact(
    State(state): State<GatewayState>,
    Path(contact_id): Path<String>,
    client: Option<Extension<ClientIdentity>>,
    body: Result<Json<ContactUpdateRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let client_label = identity_label(client.as_ref());
    let persona = match request.persona.map(PersonaText::new).transpose() {
        Ok(persona) => persona,
        Err(error) => {
            record_contact_audit(
                &state,
                CONTACT_ACTION_DENY,
                "/v1/agents/contacts/{contact_id}",
                &contact_id,
                "deny",
                Some("invalid_persona"),
                client_label,
            )
            .await;
            return invalid_request_frame(format!("人设无效: {}", error));
        }
    };
    let binding = match request.workspace_binding {
        None => ContactBindingChange::Keep,
        Some(None) => ContactBindingChange::Clear,
        Some(Some(binding)) => match build_binding_command(binding) {
            Ok(command) => ContactBindingChange::Bind(command),
            Err(reason) => {
                record_contact_audit(
                    &state,
                    CONTACT_ACTION_DENY,
                    "/v1/agents/contacts/{contact_id}",
                    &contact_id,
                    "deny",
                    Some("invalid_request"),
                    client_label,
                )
                .await;
                return invalid_request_frame(reason);
            }
        },
    };
    if persona.is_none()
        && request.capabilities.is_none()
        && matches!(binding, ContactBindingChange::Keep)
    {
        record_contact_audit(
            &state,
            CONTACT_ACTION_DENY,
            "/v1/agents/contacts/{contact_id}",
            &contact_id,
            "deny",
            Some("empty_patch"),
            client_label,
        )
        .await;
        return invalid_request_frame(
            "PATCH 至少要给一项: persona / capabilities / workspace_binding",
        );
    }
    let capabilities = request
        .capabilities
        .map(|capabilities| ContactCapabilitiesCommand {
            read: capabilities.read,
            write: capabilities.write,
            approval_required: capabilities.approval_required,
        });
    apply_contact_mutation(
        &state,
        &contact_id,
        request.expected_revision,
        "/v1/agents/contacts/{contact_id}",
        CONTACT_ACTION_UPDATE,
        ContactMutation::Update {
            persona,
            capabilities,
            binding,
        },
        client_label,
    )
    .await
}

/// `POST /v1/agents/contacts/{contact_id}/archive` — 归档（软删除，终态）。
pub async fn archive_contact(
    State(state): State<GatewayState>,
    Path(contact_id): Path<String>,
    client: Option<Extension<ClientIdentity>>,
    body: Result<Json<ContactRevisionRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(rejection) => {
            return invalid_request_frame(format!("请求体无效: {}", rejection.body_text()));
        }
    };
    let client_label = identity_label(client.as_ref());
    apply_contact_mutation(
        &state,
        &contact_id,
        request.expected_revision,
        "/v1/agents/contacts/{contact_id}/archive",
        CONTACT_ACTION_ARCHIVE,
        ContactMutation::Archive,
        client_label,
    )
    .await
}

/// 写面要执行的动作。
enum ContactMutation {
    /// 改人设 / 能力投影 / 绑定。
    Update {
        /// 新人设。
        persona: Option<PersonaText>,
        /// 新能力投影。
        capabilities: Option<ContactCapabilitiesCommand>,
        /// 绑定变更。
        binding: ContactBindingChange,
    },
    /// 归档。
    Archive,
}

/// 写面公共管线：必填修订号 → 身份格式 → 读现状 → 终态检查 → 修订号比对 → 落动作。
///
/// 修订号比对发生在**写端口之前**：过期调用者永远碰不到写端口，联系人保持
/// 逐字段不变。归档是终态：已归档的联系人再 PATCH ⇒ 403，且不产生任何字段变化。
async fn apply_contact_mutation(
    state: &GatewayState,
    contact_id: &str,
    expected_revision: Option<u64>,
    endpoint: &str,
    action: &str,
    mutation: ContactMutation,
    client_label: &str,
) -> Response {
    let Some(expected_revision) = expected_revision else {
        record_contact_audit(
            state,
            CONTACT_ACTION_DENY,
            endpoint,
            contact_id,
            "deny",
            Some("missing_expected_revision"),
            client_label,
        )
        .await;
        return invalid_request_frame("expected_revision 必填: 写面必须带上读面读到的修订号");
    };
    let Some(panel) = state.services.contacts.as_ref() else {
        return crate::panels::unsupported("contacts.write");
    };
    if contact_id.parse::<ContactId>().is_err() {
        record_contact_audit(
            state,
            CONTACT_ACTION_DENY,
            endpoint,
            contact_id,
            "deny",
            Some("malformed_contact_id"),
            client_label,
        )
        .await;
        return invalid_request_frame("contact_id 格式非法");
    }
    let current = match panel.get_contact(contact_id).await {
        Ok(current) => current,
        Err(error) => {
            record_contact_audit(
                state,
                CONTACT_ACTION_DENY,
                endpoint,
                contact_id,
                "deny",
                Some(&error.reason_label()),
                client_label,
            )
            .await;
            return error.into_response();
        }
    };
    if current.status == ContactStatusDto::Archived {
        record_contact_audit(
            state,
            CONTACT_ACTION_DENY,
            endpoint,
            contact_id,
            "deny",
            Some("contact_archived"),
            client_label,
        )
        .await;
        return ContactPanelError::Archived {
            contact_id: contact_id.to_string(),
        }
        .into_response();
    }
    if current.revision != expected_revision {
        record_contact_audit(
            state,
            CONTACT_ACTION_DENY,
            endpoint,
            contact_id,
            "deny",
            Some("revision_conflict"),
            client_label,
        )
        .await;
        return ContactPanelError::Conflict {
            contact_id: contact_id.to_string(),
            expected_revision,
            actual_revision: current.revision,
        }
        .into_response();
    }
    // 绑定声明要先过工作区事实校验（S10）：不匹配 ⇒ 409 且**不产生任何字段变化**。
    if let ContactMutation::Update {
        binding: ContactBindingChange::Bind(binding),
        ..
    } = &mutation
    {
        if let Err(response) = check_workspace_binding(state, binding).await {
            record_contact_audit(
                state,
                CONTACT_ACTION_DENY,
                endpoint,
                contact_id,
                "deny",
                Some("binding_conflict"),
                client_label,
            )
            .await;
            return response;
        }
    }
    let outcome = match mutation {
        ContactMutation::Update {
            persona,
            capabilities,
            binding,
        } => {
            panel
                .update_contact(ContactUpdateCommand {
                    contact_id: contact_id.to_string(),
                    expected_revision,
                    persona,
                    capabilities,
                    workspace_binding: binding,
                })
                .await
        }
        ContactMutation::Archive => {
            panel
                .archive_contact(ContactArchiveCommand {
                    contact_id: contact_id.to_string(),
                    expected_revision,
                })
                .await
        }
    };
    match outcome {
        Ok(dto) => {
            let dto = match guard_contact_dto(dto) {
                Ok(dto) => dto,
                Err(response) => return response,
            };
            record_contact_audit(
                state,
                action,
                endpoint,
                &dto.contact_id,
                "allow",
                None,
                client_label,
            )
            .await;
            (StatusCode::OK, Json(ContactEnvelope { contact: dto })).into_response()
        }
        Err(error) => {
            record_contact_audit(
                state,
                CONTACT_ACTION_DENY,
                endpoint,
                contact_id,
                "deny",
                Some(&error.reason_label()),
                client_label,
            )
            .await;
            error.into_response()
        }
    }
}

/// 把新建请求折成端口命令（人设校验走领域层 `PersonaText`）。
fn build_create_command(request: ContactCreateRequest) -> Result<ContactCreateCommand, String> {
    if let Some(avatar_ref) = request.avatar_ref.as_deref() {
        if looks_like_path(avatar_ref) {
            return Err("avatar_ref 是引用不是路径: 不得提交绝对路径或带分隔符的路径".to_string());
        }
    }
    let persona = match request.persona.map(PersonaText::new).transpose() {
        Ok(persona) => persona,
        Err(error) => return Err(format!("人设无效: {error}")),
    };
    let workspace_binding = match request.workspace_binding {
        Some(binding) => Some(build_binding_command(binding)?),
        None => None,
    };
    Ok(ContactCreateCommand {
        display_name: request.display_name,
        subtitle: request.subtitle,
        avatar_ref: request.avatar_ref,
        persona,
        memory_scope: request.memory_scope,
        workspace_binding,
    })
}

/// 把绑定声明折成端口命令。
fn build_binding_command(
    binding: ContactWorkspaceBindingRequest,
) -> Result<ContactBindingCommand, String> {
    if binding.workspace_id.trim().is_empty() {
        return Err("workspace_binding.workspace_id 不得为空".to_string());
    }
    Ok(ContactBindingCommand {
        workspace_id: binding.workspace_id,
        revision: binding.revision,
    })
}

/// 校验工作区绑定声明（S10）：工作区必须**存在**、**未吊销**、**修订号精确匹配**。
///
/// ⛔ 绑定**只存 id + 修订号**（设计稿裁决 20）：这里不落任何工作区快照，
/// 只拿**当前事实**做一次比对。工作区注册表没装配 ⇒ **fail-closed**：
/// 无法校验 ⇒ 不接受绑定（不是「先收下再说」）。
async fn check_workspace_binding(
    state: &GatewayState,
    binding: &ContactBindingCommand,
) -> Result<(), Response> {
    let Some(workspaces) = state.services.workspaces.as_ref() else {
        return Err(crate::panels::unsupported("contacts.binding"));
    };
    let workspace = match workspaces.get_workspace(&binding.workspace_id).await {
        Ok(workspace) => workspace,
        Err(error) => {
            let reason = match error {
                crate::workspace_panel::WorkspacePanelError::NotFound { .. } => {
                    "workspace_not_found"
                }
                crate::workspace_panel::WorkspacePanelError::Revoked { .. } => "workspace_revoked",
                crate::workspace_panel::WorkspacePanelError::Conflict { .. } => {
                    "workspace_revision_conflict"
                }
                crate::workspace_panel::WorkspacePanelError::InvalidRequest { .. } => {
                    "workspace_id_malformed"
                }
                crate::workspace_panel::WorkspacePanelError::Internal { .. } => {
                    return Err(error.into_response());
                }
            };
            return Err(binding_conflict_response(&binding.workspace_id, reason));
        }
    };
    if workspace.status == WorkspaceStatus::Revoked {
        return Err(binding_conflict_response(
            &binding.workspace_id,
            "workspace_revoked",
        ));
    }
    if workspace.revision != binding.revision {
        return Err(binding_conflict_response(
            &binding.workspace_id,
            "workspace_revision_mismatch",
        ));
    }
    Ok(())
}

/// 绑定冲突帧（409）：带稳定的机器可读原因标签。
fn binding_conflict_response(workspace_id: &str, reason: &str) -> Response {
    ContactPanelError::BindingConflict {
        workspace_id: workspace_id.to_string(),
        reason: reason.to_string(),
    }
    .into_response()
}

/// 拒掉看起来像路径的值（客户端不得提交绝对路径：设计稿裁决 11）。
fn looks_like_path(value: &str) -> bool {
    if value.starts_with('/') || value.starts_with('\\') {
        return true;
    }
    if value.contains("://") || value.contains('\\') {
        return true;
    }
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic() => true,
        _ => false,
    }
}

/// 拒一个 DTO 里带宿主存储位置的投影。
///
///  offending value **不回显**：帧只说宿主返回了不安全的投影。
fn guard_contact_dto(dto: ContactDto) -> Result<ContactDto, Response> {
    if let Some(avatar_ref) = dto.avatar_ref.as_deref() {
        if looks_like_path(avatar_ref) {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                "联系人投影未通过安全检查 (宿主返回了带路径的头像引用)",
                None,
            ));
        }
    }
    Ok(dto)
}

/// 追加一条可检索的联系人审计行。
///
/// 明细只带身份与判定事实——绝不带路径或人设正文。
async fn record_contact_audit(
    state: &GatewayState,
    action: &str,
    endpoint: &str,
    contact_id: &str,
    decision: &str,
    reason: Option<&str>,
    client_label: &str,
) {
    let contact = if contact_id.is_empty() {
        "-".to_string()
    } else {
        crate::canonical_entry::truncate_label(contact_id)
    };
    let mut detail = format!(
        "contact={contact} action={action} endpoint={endpoint} decision={decision} client={}",
        crate::canonical_entry::truncate_label(client_label)
    );
    if let Some(reason) = reason {
        detail.push_str(&format!(" reason={reason}"));
    }
    if let Some(audit) = state.services.audit_commands.as_ref() {
        audit.append_audit(CONTACT_AUDIT_EVENT, Some(&detail)).await;
        return;
    }
    state
        .observations
        .buffer_audit(CONTACT_AUDIT_EVENT, Some(detail));
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

// ---------------------------------------------------------------------------
// 装配面（路由表 + 能力清单组）
// ---------------------------------------------------------------------------

/// 联系人合同面的路由表（装配点 [`crate::panels::panel_routes`] 一行 `merge`）。
///
/// 路由表跟合同面住在一起：合同面自己带着自己的端点表，装配点不摊开。
/// **不另开** `/v1/panel/contacts` 别名 —— 一张读面，不两套。
pub fn contact_routes() -> axum::Router<GatewayState> {
    axum::Router::new()
        .route("/v1/agents/contacts", axum::routing::get(list_contacts))
        .route("/v1/agents/contacts", axum::routing::post(create_contact))
        .route(
            "/v1/agents/contacts/:contact_id",
            axum::routing::get(get_contact),
        )
        .route(
            "/v1/agents/contacts/:contact_id",
            axum::routing::patch(update_contact),
        )
        .route(
            "/v1/agents/contacts/:contact_id/archive",
            axum::routing::post(archive_contact),
        )
}

/// 能力清单里的联系人组（读/写**分开**登记，与 `workspaces.read`/
/// `workspaces.write` 同口径）。
///
/// 未接线 ⇒ `supported=false` 且 `available=false`（带 `reason`），与工作区面同口径。
pub fn contacts_capability_group(supported: bool) -> serde_json::Value {
    serde_json::json!({
        "name": "contacts",
        "capabilities": [
            crate::panels::cap(
                "contacts.read",
                true,
                false,
                &["list", "get"],
                supported,
                supported,
            ),
            crate::panels::cap(
                "contacts.write",
                false,
                true,
                &["create", "update", "archive"],
                supported,
                supported,
            ),
        ],
    })
}
