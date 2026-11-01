//! W3-② 用户管理面 (HTTP + CLI **双入口同源**): [`UserAdminService`] 是唯一
//! UserTable 操作语义, 两个入口共用它、改同一份 `users.json`。
//!
//! * HTTP 面 (V2-b 域面开放与管理端点, 供 V1-b 面板消费):
//!   `GET /v1/panel/users` (列表) / `POST /v1/admin/users` (建户) /
//!   `PATCH /v1/admin/users/{user_id}` (改角色) / `DELETE /v1/admin/users/{user_id}`
//!   (删户) —— 全部在 W2-② 令牌门与 W3-② 角色门 [`UserCapability::AdminControl`]
//!   (owner 档) 之内, 每次管理动作落一行 `gateway.user_action` 审计;
//! * CLI 面 (`apeireth user add|list|role|remove`, `crates/adapters/cli/src/user_admin.rs`)
//!   调同一个 [`UserAdminService`] —— 引导口径 / 末 owner 保护 / 发令牌序一处实现,
//!   两个入口的行为逐字同源。
//!
//! 坏输入防御 (fail-closed, 全入口同一把闸):
//!
//! * **user_id 路径安全** —— 复用 W3-① [`DomainUserId`] 的目录安全校验
//!   (1..=64 字节、`[a-z0-9_-]`、首字符小写字母或数字): user_id 会成为域引用
//!   缺省值 (`users/<user_id>/` 目录名), 不安全的 id 一律拒绝, 路径穿越 (`../`)、
//!   路径分隔符、大小写混淆天然被拒;
//! * **角色词表冻结** —— 只认 `owner|member|guest` ([`UserRole::parse`]), 未知
//!   角色拒, 不就近映射;
//! * **坏配置拒开** —— 坏 `users.json` / 令牌↔用户绑定不一致一律拒绝操作,
//!   0 静默回退空表 (否则会走引导口绕过 owner 门);
//! * **发令牌序 (问责链不得断)** —— 落盘前过 [`validate_client_user_bindings`]:
//!   删用户前必须先摘令牌绑定, 否则拒绝落盘。
//!
//! 破坏性不变量: **末 owner 不可删/不可降级** ([`UserTable`] 拒)。

use std::path::{Path, PathBuf};

use apeireth_core::user_domain::DomainUserId;
use apeireth_governance::{UserAccessError, UserRecord, UserRole, UserTable};
use axum::extract::{Extension, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{
    record_user_action, validate_client_user_bindings, ClientTokenTable, UserContext,
};
use crate::panels::GatewayState;

/// 用户管理行 (稳定合同字段; 改名须同步 `docs/gateway-api-contract.md` 与
/// `docs/02-guides/frontend-data-contract.md`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAdminDto {
    /// 用户 id (问责署名键; 目录安全词面, 见 [`DomainUserId`])。
    pub user_id: String,
    /// 冻结词表角色 (`owner`/`member`/`guest`)。
    pub role: String,
    /// 域引用 (W3-① 域目录的宽接口; 缺省 = 与 user_id 同名的域)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// 绑定客户端名 (全表唯一归属; 空 = 尚未发令牌)。
    pub client_names: Vec<String>,
    /// 绑定客户端数 (`client_names.len()`, 面板"客户端数"列)。
    pub client_count: usize,
}

impl UserAdminDto {
    fn from_record(record: &UserRecord) -> Self {
        Self {
            user_id: record.user_id.clone(),
            role: record.role.clone(),
            domain: record.domain.clone(),
            client_count: record.client_names.len(),
            client_names: record.client_names.clone(),
        }
    }
}

/// `POST /v1/admin/users` 请求体 (字段合同见契约文档)。
#[derive(Debug, Clone, Deserialize)]
pub struct UserCreateBody {
    /// 用户 id (目录安全词面)。
    pub user_id: String,
    /// 角色词面 (冻结词表)。
    pub role: String,
    /// 绑定客户端名 (可空)。
    #[serde(default)]
    pub client_names: Vec<String>,
    /// 域引用 (可空; 引用词面同目录安全规则)。
    #[serde(default)]
    pub domain: Option<String>,
}

/// `PATCH /v1/admin/users/{user_id}` 请求体 (只开改角色一门)。
#[derive(Debug, Clone, Deserialize)]
pub struct UserRolePatchBody {
    /// 新角色词面 (冻结词表)。
    pub role: String,
}

/// 用户管理面缺陷 (HTTP 状态映射见 [`UserAdminError::http_status`])。
///
/// `message` 携带真因 (CLI 直接呈现, HTTP 进错误帧 `message` 字段)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserAdminError {
    /// 坏输入 (id 词面越界 / 未知角色 / 空字段): 400 `invalid_request`。
    Invalid(String),
    /// 目标用户不在表内: 404 `not_found`。
    NotFound(String),
    /// 状态冲突 (重复 user_id / 绑定歧义 / 末 owner 保护 / 问责链断): 409 `conflict`。
    Conflict(String),
    /// 配置/存储拒开 (坏 `users.json` / IO): 500 `runtime_error`。
    Storage(String),
}

impl UserAdminError {
    /// 真因文本 (审计/CLI/错误帧)。
    pub fn message(&self) -> &str {
        match self {
            Self::Invalid(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Storage(message) => message,
        }
    }

    /// HTTP 状态 (400/404/409/500, 与契约文档错误模型一致)。
    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 线级错误码 (契约文档冻结词表)。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid_request",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Storage(_) => "runtime_error",
        }
    }
}

impl std::fmt::Display for UserAdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for UserAdminError {}

impl From<UserAccessError> for UserAdminError {
    fn from(error: UserAccessError) -> Self {
        match &error {
            UserAccessError::UnknownRole { .. }
            | UserAccessError::RoleUnprovisioned { .. }
            | UserAccessError::InvalidIdentity { .. } => Self::Invalid(error.to_string()),
            UserAccessError::UserNotFound { .. } => Self::NotFound(error.to_string()),
            UserAccessError::DuplicateUserId { .. }
            | UserAccessError::DuplicateClientBinding { .. }
            | UserAccessError::KeepLastOwner { .. }
            | UserAccessError::BindingMismatch { .. } => Self::Conflict(error.to_string()),
            UserAccessError::Stored(_) => Self::Storage(error.to_string()),
        }
    }
}

/// 共享 UserTable 服务 (**双入口同源**): 每个操作都以数据目录的 `users.json`
/// 为唯一事实 (操作前重读、校验后落盘), 所以 CLI 进程与网关进程的管理动作
/// 互相可见、语义逐字一致。
///
/// 每次操作都重读令牌面 (env 单令牌 + `client-tokens.json`, 与 W2-② 令牌族
/// 同口径) 做发令牌序校验; 令牌/角色门的**运行期**判定仍按 W2-②/W3-② 装载期
/// 快照语义 (新绑定随下次启动生效), 两者的边界见契约文档。
#[derive(Debug, Clone)]
pub struct UserAdminService {
    data_dir: PathBuf,
}

impl UserAdminService {
    /// 以数据目录构造 (目录里的 `users.json` 是唯一事实)。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// 数据目录。
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 当前用户表 (缺失 = 空表; 坏表拒开)。
    pub fn load(&self) -> Result<UserTable, UserAdminError> {
        UserTable::load(&self.data_dir)
            .map(|table| table.unwrap_or_default())
            .map_err(|error| {
                UserAdminError::Storage(format!("user table refused to open: {error}"))
            })
    }

    /// 用户管理行全表 (文件顺序)。
    pub fn list(&self) -> Result<Vec<UserAdminDto>, UserAdminError> {
        Ok(self
            .load()?
            .users
            .iter()
            .map(UserAdminDto::from_record)
            .collect())
    }

    /// 建户: 词面校验 + 引导口径 + 入表 + 发令牌序校验 + 落盘。
    ///
    /// 引导口径 (fail-closed, 不锁死): 用户表为空时首个用户**必须是 owner**
    /// (否则用户表永久失主); 表非空后的 owner 门由入口面把守 (CLI 的
    /// `require_manager` / HTTP 的 [`UserCapability::AdminControl`] 角色门)。
    pub fn add(
        &self,
        user_id: &str,
        role: &str,
        client_names: Vec<String>,
        domain: Option<&str>,
    ) -> Result<UserAdminDto, UserAdminError> {
        let user_id = checked_id(user_id)?;
        let role = parse_role(role)?;
        if let Some(domain) = domain {
            checked_domain_ref(domain)?;
        }
        let mut users = self.load()?;
        if users.users.is_empty() && role != UserRole::Owner {
            return Err(UserAdminError::Invalid(
                "the first user must be owner (the user table must start with an owner)"
                    .to_string(),
            ));
        }
        let record = UserRecord {
            user_id,
            role: role.as_str().to_string(),
            client_names,
            domain: domain.map(str::to_string),
        };
        users.add(record)?;
        self.commit(&users)?;
        Ok(UserAdminDto::from_record(
            users.users.last().expect("just appended"),
        ))
    }

    /// 改角色 (词表冻结; 末 owner 不可降级)。
    pub fn set_role(&self, user_id: &str, role: &str) -> Result<UserAdminDto, UserAdminError> {
        let user_id = checked_id(user_id)?;
        let role = parse_role(role)?;
        let mut users = self.load()?;
        users.set_role(&user_id, role)?;
        self.commit(&users)?;
        let record = users
            .users
            .iter()
            .find(|record| record.user_id == user_id)
            .expect("set_role just succeeded");
        Ok(UserAdminDto::from_record(record))
    }

    /// 删户 (末 owner 不可删; 令牌仍引用的用户删不得 —— 先摘绑定后删用户)。
    pub fn remove(&self, user_id: &str) -> Result<UserAdminDto, UserAdminError> {
        let user_id = checked_id(user_id)?;
        let mut users = self.load()?;
        let removed = users.remove(&user_id)?;
        self.commit(&users)?;
        Ok(UserAdminDto::from_record(&removed))
    }

    /// 落盘前的最后闸: 令牌表↔用户表绑定一致性 (问责链不得断) + 持久化。
    fn commit(&self, users: &UserTable) -> Result<(), UserAdminError> {
        let tokens = ClientTokenTable::load(Some(&self.data_dir)).map_err(|error| {
            UserAdminError::Storage(format!("client token table refused to open: {error}"))
        })?;
        validate_client_user_bindings(&tokens, users)?;
        users.save(&self.data_dir).map_err(|error| {
            UserAdminError::Storage(format!("user table refused to save: {error}"))
        })
    }
}

/// user_id 词面校验 (复用 W3-① [`DomainUserId`] 目录安全规则)。
fn checked_id(raw: &str) -> Result<String, UserAdminError> {
    DomainUserId::new(raw.trim())
        .map(|id| id.as_str().to_string())
        .map_err(|error| UserAdminError::Invalid(error.to_string()))
}

/// V2-c 管理动作审计行的**目标域**标注 (`domain=<id>` 记号): 域引用缺省 =
/// user_id 同名域 (与域解析链同一缺省语义); 词面未过 [`DomainUserId`] 目录
/// 安全闸 = 不标注 (fail-closed, 未核实值不进审计行)。
fn target_domain_label(domain: Option<&str>, user_id: &str) -> Option<String> {
    checked_domain_ref(domain.unwrap_or(user_id)).ok()
}

/// 目标用户的域标注 (查表取该用户**实际**域引用, 缺省 user_id 同名域;
/// 查无/表坏 = 不标注, fail-closed)。
fn target_user_domain(service: &UserAdminService, user_id: &str) -> Option<String> {
    let table = service.load().ok()?;
    let record = table
        .users
        .into_iter()
        .find(|record| record.user_id == user_id)?;
    Some(record.domain.unwrap_or(record.user_id))
}

/// 域引用词面校验 (域引用缺省落 `users/<id>/` 目录, 同一把路径安全闸)。
fn checked_domain_ref(raw: &str) -> Result<String, UserAdminError> {
    checked_id(raw)
}

/// 角色词表冻结: 只认 `owner|member|guest`。
fn parse_role(raw: &str) -> Result<UserRole, UserAdminError> {
    UserRole::parse(raw).map_err(|error| UserAdminError::Invalid(error.to_string()))
}

// ---------------------------------------------------------------------------
// HTTP 面 (过令牌门 + AdminControl 角色门; 路由挂载见 panels.rs / canonical_entry.rs)
// ---------------------------------------------------------------------------

/// `GET /v1/panel/users` —— 用户管理列表 (id/角色/域引用/客户端数)。
pub async fn list_users(State(state): State<GatewayState>) -> Response {
    let Some(service) = &state.services.user_admin else {
        return crate::panels::unsupported("users.admin.read");
    };
    match service.list() {
        Ok(users) => {
            let count = users.len();
            (
                StatusCode::OK,
                Json(serde_json::json!({ "users": users, "count": count })),
            )
                .into_response()
        }
        Err(error) => error_response(&error),
    }
}

/// `POST /v1/admin/users` —— 建户 (owner 档)。
pub async fn create_user(
    State(state): State<GatewayState>,
    Extension(user): Extension<UserContext>,
    Json(body): Json<UserCreateBody>,
) -> Response {
    let Some(service) = &state.services.user_admin else {
        return crate::panels::unsupported("users.admin.write");
    };
    let endpoint = "/v1/admin/users".to_string();
    match service.add(
        &body.user_id,
        &body.role,
        body.client_names.clone(),
        body.domain.as_deref(),
    ) {
        Ok(created) => {
            // V2-c 域标注: 管理动作 = 目标域 (新建用户的域引用, 缺省同名域)。
            let domain_label = target_domain_label(created.domain.as_deref(), &created.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "user_add",
                &endpoint,
                true,
                domain_label.as_deref(),
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({ "user": created })),
            )
                .into_response()
        }
        Err(error) => {
            // V2-c 域标注: 失败行按意图目标标注 (域引用缺省同名域)。
            let domain_label = target_domain_label(body.domain.as_deref(), &body.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "user_add",
                &endpoint,
                false,
                domain_label.as_deref(),
            );
            error_response(&error)
        }
    }
}

/// `PATCH /v1/admin/users/{user_id}` —— 改角色 (owner 档; user_id 过路径安全闸)。
pub async fn patch_user(
    State(state): State<GatewayState>,
    Extension(user): Extension<UserContext>,
    AxumPath(user_id): AxumPath<String>,
    Json(body): Json<UserRolePatchBody>,
) -> Response {
    let Some(service) = &state.services.user_admin else {
        return crate::panels::unsupported("users.admin.write");
    };
    let endpoint = format!("/v1/admin/users/{user_id}");
    match service.set_role(&user_id, &body.role) {
        Ok(updated) => {
            // V2-c 域标注: 管理动作 = 目标域 (该用户的实际域引用)。
            let domain_label = target_domain_label(updated.domain.as_deref(), &updated.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "user_role_change",
                &endpoint,
                true,
                domain_label.as_deref(),
            );
            (StatusCode::OK, Json(serde_json::json!({ "user": updated }))).into_response()
        }
        Err(error) => {
            // V2-c 域标注: 失败行按查表的目标域标注 (查无 = 不标注)。
            let domain_label = target_user_domain(service, &user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "user_role_change",
                &endpoint,
                false,
                domain_label.as_deref(),
            );
            error_response(&error)
        }
    }
}

/// `DELETE /v1/admin/users/{user_id}` —— 删户 (owner 档; 末 owner 保护)。
pub async fn delete_user(
    State(state): State<GatewayState>,
    Extension(user): Extension<UserContext>,
    AxumPath(user_id): AxumPath<String>,
) -> Response {
    let Some(service) = &state.services.user_admin else {
        return crate::panels::unsupported("users.admin.write");
    };
    let endpoint = format!("/v1/admin/users/{user_id}");
    match service.remove(&user_id) {
        Ok(removed) => {
            // V2-c 域标注: 管理动作 = 目标域 (被删用户的实际域引用)。
            let domain_label = target_domain_label(removed.domain.as_deref(), &removed.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "user_remove",
                &endpoint,
                true,
                domain_label.as_deref(),
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "deleted": true, "user_id": removed.user_id })),
            )
                .into_response()
        }
        Err(error) => {
            // V2-c 域标注: 失败行按查表的目标域标注 (查无 = 不标注)。
            let domain_label = target_user_domain(service, &user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "user_remove",
                &endpoint,
                false,
                domain_label.as_deref(),
            );
            error_response(&error)
        }
    }
}

/// 管理面错误帧 (与面板族同形: `{"error": {code, message, solution}}`)。
pub(crate) fn error_response(error: &UserAdminError) -> Response {
    let solution = match error {
        UserAdminError::Invalid(_) => {
            "检查参数后重试 (user_id 限 [a-z0-9_-] 词面, 角色限 owner|member|guest)"
        }
        UserAdminError::NotFound(_) => "回到列表重新选择",
        UserAdminError::Conflict(_) => {
            "刷新列表后重试; 冲突持续请检查用户表与令牌绑定 (发令牌序: 先摘绑定后删用户)"
        }
        UserAdminError::Storage(_) => {
            "重启 Companion 后重试, 若复现请提交日志 (坏用户表拒开, 0 静默回退)"
        }
    };
    crate::panels::admin_error(error.http_status(), error.code(), error.message(), solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-gateway-user-admin-{tag}-{}",
            std::process::id()
        ));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn tokens(dir: &Path, entries: &[(&str, &str, Option<&str>)]) {
        use crate::client_tokens::{
            ClientTokenConfig, ClientTokenSpec, CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION,
            CLIENT_TOKENS_FILE,
        };
        use apeireth_core::stored_doc::StoredDoc;
        let clients = entries
            .iter()
            .map(|(name, token, user_id)| ClientTokenSpec {
                name: (*name).to_string(),
                token: (*token).to_string(),
                user_id: user_id.map(str::to_string),
            })
            .collect();
        let doc = StoredDoc::new(
            CLIENT_TOKENS_DOC_NAME,
            CLIENT_TOKENS_DOC_VERSION,
            vec![CLIENT_TOKENS_DOC_VERSION],
            ClientTokenConfig::new(clients),
        );
        fs_err::write(
            dir.join(CLIENT_TOKENS_FILE),
            serde_json::to_vec_pretty(&doc).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn unsafe_user_ids_are_refused_before_any_lookup() {
        let service = UserAdminService::new(temp_dir("id-safety"));
        let too_long = "x".repeat(65);
        for bad in [
            "../escape",
            "..",
            "a/b",
            "a\\b",
            "A-1",
            "-lead",
            "",
            "   ",
            too_long.as_str(),
        ] {
            let error = service
                .add(bad, "owner", vec![], None)
                .expect_err(&format!("{bad:?} 必须拒"));
            assert!(
                matches!(error, UserAdminError::Invalid(_)),
                "{bad:?}: {error}"
            );
            let error = service
                .set_role(bad, "member")
                .expect_err(&format!("{bad:?} 必须拒"));
            assert!(
                matches!(error, UserAdminError::Invalid(_)),
                "{bad:?}: {error}"
            );
            let error = service.remove(bad).expect_err(&format!("{bad:?} 必须拒"));
            assert!(
                matches!(error, UserAdminError::Invalid(_)),
                "{bad:?}: {error}"
            );
        }
    }

    #[test]
    fn the_service_refuses_unknown_roles_and_unsafe_domain_refs() {
        let dir = temp_dir("roles");
        tokens(
            &dir,
            &[("tui", "tok-owner-0123456789abcdef", Some("owner-1"))],
        );
        let service = UserAdminService::new(&dir);
        service
            .add("owner-1", "owner", vec!["tui".to_string()], None)
            .unwrap();

        let error = service
            .add("u-2", "admin", vec![], None)
            .expect_err("未知角色必须拒");
        assert!(matches!(error, UserAdminError::Invalid(_)), "{error}");
        let error = service
            .add("u-2", "owner", vec![], Some("../etc"))
            .expect_err("越界域引用必须拒");
        assert!(matches!(error, UserAdminError::Invalid(_)), "{error}");
    }

    #[test]
    fn the_first_user_must_be_owner_and_the_last_owner_is_protected() {
        let dir = temp_dir("invariants");
        tokens(
            &dir,
            &[("tui", "tok-owner-0123456789abcdef", Some("owner-1"))],
        );
        let service = UserAdminService::new(&dir);

        let error = service
            .add("member-1", "member", vec![], None)
            .expect_err("首用户非 owner 必须拒");
        assert!(
            error.message().contains("first user must be owner"),
            "{error}"
        );
        service
            .add("owner-1", "owner", vec!["tui".to_string()], None)
            .unwrap();

        let error = service
            .set_role("owner-1", "member")
            .expect_err("末 owner 不可降");
        assert!(matches!(error, UserAdminError::Conflict(_)), "{error}");
        let error = service.remove("owner-1").expect_err("末 owner 不可删");
        assert!(matches!(error, UserAdminError::Conflict(_)), "{error}");
    }

    #[test]
    fn removing_a_user_still_bound_by_a_token_is_refused() {
        let dir = temp_dir("bindings");
        // 发令牌序: 令牌声明 `user_id` 的, 先建用户再写绑定 (悬空绑定拒开)。
        tokens(
            &dir,
            &[("tui", "tok-owner-0123456789abcdef", Some("owner-1"))],
        );
        let service = UserAdminService::new(&dir);
        service
            .add("owner-1", "owner", vec!["tui".to_string()], None)
            .unwrap();
        service
            .add("member-1", "member", vec!["phone".to_string()], None)
            .unwrap();
        tokens(
            &dir,
            &[
                ("tui", "tok-owner-0123456789abcdef", Some("owner-1")),
                ("phone", "tok-member-0123456789abcdef", Some("member-1")),
            ],
        );

        let error = service
            .remove("member-1")
            .expect_err("令牌仍引用的用户删不得");
        assert!(matches!(error, UserAdminError::Conflict(_)), "{error}");

        // 先摘令牌绑定, 再删用户 (发令牌序)。
        tokens(
            &dir,
            &[("tui", "tok-owner-0123456789abcdef", Some("owner-1"))],
        );
        let removed = service.remove("member-1").expect("摘绑定后可删");
        assert_eq!(removed.user_id, "member-1");
    }
}
