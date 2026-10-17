//! W3-① 域管理面 (HTTP, V2-b 域面开放与管理端点): [`DomainAdminService`] 是
//! [`DomainRegistry`] 的开域/列域口, 供 V1-b 面板消费。
//!
//! * `GET /v1/panel/domains` —— 域列表 + 计数 (域计数 + 每域用户引用计数);
//! * `POST /v1/admin/domains` —— 开域 (owner 档)。
//!
//! 两端点都在 W2-② 令牌门与 W3-② 角色门 [`UserCapability::AdminControl`]
//! (owner 档) 之内; 开域动作落一行 `gateway.user_action` 审计。坏域配置
//! (畸形域文档 / 目录名不是合法 id) 一律**拒开** ([`UserDomainError`] 的
//! 拒开语义), 不静默降级; `default` 域 id 由「采用」语义保留, 拒绝再建。

use apeireth_core::user_domain::{
    DomainRegistry, DomainRole, DomainUserId, UserDomain, UserDomainError,
};
use apeireth_governance::{UserRole, UserTable};
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{record_user_action, UserContext};
use crate::panels::GatewayState;

/// 域管理行 (稳定合同字段; 改名须同步两份契约文档)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainAdminDto {
    /// 域 id (目录安全词面, 与 `users/<id>/` 目录名一致)。
    pub user_id: String,
    /// 显示名 (1..=128 字符, 无控制字符)。
    pub display_name: String,
    /// 创建时刻 (epoch 毫秒)。
    pub created_at: i64,
    /// 域角色词面 (`owner`/`member`/`guest`, 与治理层冻结词表同形)。
    pub role: String,
    /// 用户引用计数 (用户表中域引用落到该域的用户数; 缺省引用 = 与 user_id
    /// 同名的域)。用户管理面未接线时**显式缺省** (不编 0)。
    pub user_count: Option<usize>,
}

/// `POST /v1/admin/domains` 请求体 (字段合同见契约文档)。
#[derive(Debug, Clone, Deserialize)]
pub struct DomainCreateBody {
    /// 域 id (目录安全词面)。
    pub user_id: String,
    /// 显示名。
    pub display_name: String,
    /// 域角色词面 (缺省 `owner` = 开域即域主)。
    #[serde(default)]
    pub role: Option<String>,
}

/// 域管理面服务 (数据根上的 [`DomainRegistry`] 门面)。
#[derive(Debug, Clone)]
pub struct DomainAdminService {
    registry: DomainRegistry,
}

impl DomainAdminService {
    /// 以域注册表构造 (组合根显式注入, 同一数据根与域平面共用)。
    pub fn new(registry: DomainRegistry) -> Self {
        Self { registry }
    }

    /// 域注册表。
    pub fn registry(&self) -> &DomainRegistry {
        &self.registry
    }

    /// 域列表 (id 序) + 每域用户引用计数 (`user_counts` = `None` 时计数缺省)。
    pub fn list(
        &self,
        user_counts: Option<&UserTable>,
    ) -> Result<Vec<DomainAdminDto>, UserDomainError> {
        let mut rows = Vec::new();
        for id in self.registry.domain_ids()? {
            let domain = self.registry.load(&id)?;
            rows.push(to_dto(&domain, user_counts));
        }
        Ok(rows)
    }

    /// 开域 (坏 id / 坏显示名拒, 已存在拒, `default` 保留拒)。
    pub fn create(
        &self,
        user_id: &str,
        display_name: &str,
        role: DomainRole,
        user_counts: Option<&UserTable>,
    ) -> Result<DomainAdminDto, UserDomainError> {
        let domain = self.registry.create(user_id, display_name, role)?;
        Ok(to_dto(&domain, user_counts))
    }
}

/// 域行 → DTO (计数 = 用户表 `domain` 引用缺省回落 user_id 同名域的口径)。
fn to_dto(domain: &UserDomain, user_counts: Option<&UserTable>) -> DomainAdminDto {
    DomainAdminDto {
        user_id: domain.user_id.as_str().to_string(),
        display_name: domain.display_name.clone(),
        created_at: domain.created_at,
        role: domain.role_str().to_string(),
        user_count: user_counts.map(|table| count_user_refs(table, &domain.user_id)),
    }
}

/// 用户引用计数: `UserRecord.domain` 缺省 = 与 user_id 同名的域 (W3-① 域解析链)。
fn count_user_refs(table: &UserTable, domain_id: &DomainUserId) -> usize {
    table
        .users
        .iter()
        .filter(|record| {
            record
                .domain
                .as_deref()
                .unwrap_or(record.user_id.as_str())
                .trim()
                == domain_id.as_str()
        })
        .count()
}

/// V2-c 管理动作审计行的**目标域**标注 (开域动作 = 被开的域 id; 词面未过
/// [`DomainUserId`] 目录安全闸 = 不标注, fail-closed, 未核实值不进审计行)。
fn target_domain_label(domain_id: &str) -> Option<String> {
    DomainUserId::new(domain_id.trim())
        .ok()
        .map(|id| id.as_str().to_string())
}

/// 域角色词面 → 域角色 (过治理层冻结词表 [`UserRole::parse`], 词表口径唯一)。
fn parse_domain_role(raw: Option<&str>) -> Result<DomainRole, UserDomainError> {
    let role = UserRole::parse(raw.unwrap_or(UserRole::Owner.as_str())).map_err(|error| {
        UserDomainError::Invalid(format!("user domain role is refused: {error}"))
    })?;
    Ok(match role {
        UserRole::Owner => DomainRole::Owner,
        UserRole::Member => DomainRole::Member,
        UserRole::Guest => DomainRole::Guest,
    })
}

// ---------------------------------------------------------------------------
// HTTP 面 (过令牌门 + AdminControl 角色门)
// ---------------------------------------------------------------------------

/// `GET /v1/panel/domains` —— 域列表 + 计数。
pub async fn list_domains(State(state): State<GatewayState>) -> Response {
    let Some(service) = &state.services.domain_admin else {
        return crate::panels::unsupported("domains.admin.read");
    };
    let user_counts = match domain_user_counts(&state) {
        Ok(counts) => counts,
        Err(error) => return error,
    };
    match service.list(user_counts.as_ref()) {
        Ok(domains) => {
            let count = domains.len();
            (
                StatusCode::OK,
                Json(serde_json::json!({ "domains": domains, "count": count })),
            )
                .into_response()
        }
        Err(error) => domain_error_response(&error),
    }
}

/// `POST /v1/admin/domains` —— 开域 (owner 档)。
pub async fn create_domain(
    State(state): State<GatewayState>,
    Extension(user): Extension<UserContext>,
    Json(body): Json<DomainCreateBody>,
) -> Response {
    let Some(service) = &state.services.domain_admin else {
        return crate::panels::unsupported("domains.admin.write");
    };
    let endpoint = "/v1/admin/domains".to_string();
    let role = match parse_domain_role(body.role.as_deref()) {
        Ok(role) => role,
        Err(error) => {
            // V2-c 域标注: 管理动作 = 目标域 (意图开的域)。
            let domain_label = target_domain_label(&body.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "domain_create",
                &endpoint,
                false,
                domain_label.as_deref(),
            );
            return domain_error_response(&error);
        }
    };
    let user_counts = match domain_user_counts(&state) {
        Ok(counts) => counts,
        Err(error) => return error,
    };
    match service.create(
        &body.user_id,
        &body.display_name,
        role,
        user_counts.as_ref(),
    ) {
        Ok(created) => {
            // V2-c 域标注: 管理动作 = 目标域 (被开的域)。
            let domain_label = target_domain_label(&created.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "domain_create",
                &endpoint,
                true,
                domain_label.as_deref(),
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({ "domain": created })),
            )
                .into_response()
        }
        Err(error) => {
            // V2-c 域标注: 管理动作 = 目标域 (意图开的域)。
            let domain_label = target_domain_label(&body.user_id);
            record_user_action(
                Some(state.observations.as_ref()),
                user.user_id(),
                user.role(),
                "domain_create",
                &endpoint,
                false,
                domain_label.as_deref(),
            );
            domain_error_response(&error)
        }
    }
}

/// 用户引用计数的取数口: 用户管理面未接线 = 计数缺省 (`None`, 不编 0);
/// 用户表拒开 = 整面拒开 (坏配置拒开, fail-closed)。
fn domain_user_counts(state: &GatewayState) -> Result<Option<UserTable>, Response> {
    let Some(service) = &state.services.user_admin else {
        return Ok(None);
    };
    match service.load() {
        Ok(table) => Ok(Some(table)),
        Err(error) => Err(crate::user_admin::error_response(&error)),
    }
}

/// 域面错误帧 ([`UserDomainError::http_status`] 钉死的映射)。
fn domain_error_response(error: &UserDomainError) -> Response {
    let (status, code, solution) = match error {
        UserDomainError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", "回到列表重新选择"),
        UserDomainError::Forbidden(_) | UserDomainError::Refused(_) => (
            StatusCode::FORBIDDEN,
            "permission_denied",
            "域管理需要 owner 角色; 越域访问一律拒绝",
        ),
        UserDomainError::Invalid(_) => (
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "检查参数后重试 (域 id 限 [a-z0-9_-] 词面, 显示名 1..=128 字符)",
        ),
        UserDomainError::AlreadyExists(_) => (
            StatusCode::CONFLICT,
            "conflict",
            "该域已存在; 回到列表查看或换一个域 id",
        ),
        UserDomainError::Io(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "runtime_error",
            "重启 Companion 后重试, 若复现请提交日志",
        ),
    };
    crate::panels::admin_error(status, code, error.reason(), solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-gateway-domain-admin-{tag}-{}",
            std::process::id()
        ));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn domain_listing_counts_user_references_with_the_fallback_rule() {
        let dir = temp_dir("counts");
        let service = DomainAdminService::new(DomainRegistry::open(&dir).unwrap());
        service
            .create("alice", "Alice", DomainRole::Owner, None)
            .unwrap();
        let users = UserTable::new(vec![
            apeireth_governance::UserRecord {
                user_id: "alice".to_string(),
                role: "owner".to_string(),
                client_names: vec![],
                domain: None,
            },
            apeireth_governance::UserRecord {
                user_id: "bob".to_string(),
                role: "member".to_string(),
                client_names: vec![],
                domain: Some("alice".to_string()),
            },
            apeireth_governance::UserRecord {
                user_id: "carol".to_string(),
                role: "guest".to_string(),
                client_names: vec![],
                domain: Some("unopened".to_string()),
            },
        ]);
        let rows = service.list(Some(&users)).unwrap();
        let alice = rows
            .iter()
            .find(|row| row.user_id == "alice")
            .expect("alice domain");
        assert_eq!(alice.user_count, Some(2), "缺省引用回落 user_id 同名域");
        let default = rows
            .iter()
            .find(|row| row.user_id == "default")
            .expect("adopted default domain");
        assert_eq!(default.user_count, Some(0));
    }

    #[test]
    fn the_default_domain_id_is_reserved_and_duplicates_refuse() {
        let dir = temp_dir("reserved");
        let service = DomainAdminService::new(DomainRegistry::open(&dir).unwrap());
        let error = service
            .create("default", "X", DomainRole::Owner, None)
            .expect_err("default 保留拒");
        assert!(matches!(error, UserDomainError::Invalid(_)), "{error}");
        service
            .create("alice", "Alice", DomainRole::Owner, None)
            .unwrap();
        let error = service
            .create("alice", "Alice again", DomainRole::Owner, None)
            .expect_err("重复域拒");
        assert!(
            matches!(error, UserDomainError::AlreadyExists(_)),
            "{error}"
        );
    }
}
