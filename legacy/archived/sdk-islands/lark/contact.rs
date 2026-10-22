//! # lark 通讯录面 (用户 / 部门 wire 契约)
//!
//! 平台通讯录端点:
//! - `GET /contact/v3/users/{user_id}?user_id_type=...&department_id_type=...`
//! - `GET /contact/v3/departments/{department_id}?department_id_type=open_department_id`
//!
//! ## wire 契约 (严格形状 + 未知字段容错)
//!
//! - 响应 `data`: `{"user": {...}}` / `{"department": {...}}`, 未知字段忽略。
//! - 用户必填: `open_id` (`ou_` 前缀) + `name` (非空);
//! - 部门必填: `open_department_id` (非空) + `name` (非空);
//! - `status` 取值闭合 (`active` / `deleted`), 未知取值 = 永久错误。
//!
//! ## 字段校验 (响应映射时完成)
//!
//! - K-1 #4 `open_id` / K-1 #5 `email` / K-1 #6 `mobile` (若有);
//! - `leader_open_ids` 每项 K-1 #4;
//! - ID 类型口径按 [`UserQuery::user_id_type`] 闭合校验。

use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;
use crate::lark::http::{encode_path_segment, ApiRequest};

// ============================================================================
// §1 UserIdType (4 variant 闭合枚举)
// ============================================================================

/// 用户 ID 类型 (4 variant 闭合枚举, wire `user_id_type`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserIdType {
    /// Open ID (`ou_` 前缀, K-1 #4)。
    #[default]
    OpenId,
    /// Union ID (`on_` 前缀)。
    UnionId,
    /// User ID (租户内 user_id)。
    UserId,
    /// Email (K-1 #5)。
    Email,
}

impl UserIdType {
    /// wire 字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            UserIdType::OpenId => "open_id",
            UserIdType::UnionId => "union_id",
            UserIdType::UserId => "user_id",
            UserIdType::Email => "email",
        }
    }
}

impl std::fmt::Display for UserIdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 User (领域实体)
// ============================================================================

/// 用户 (领域实体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Open ID (K-1 #4)。
    pub open_id: String,
    /// Union ID (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_id: Option<String>,
    /// 租户内 User ID (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 用户名 (非空)。
    pub name: String,
    /// 英文名 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub en_name: Option<String>,
    /// 昵称 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// 邮箱 (K-1 #5)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 手机 (K-1 #6, E.164)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    /// 头像 URL (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 部门 ID 列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub department_ids: Vec<String>,
    /// 工号 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employee_no: Option<String>,
    /// 职位 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_title: Option<String>,
    /// 是否激活 (默认 true)。
    #[serde(default = "default_true")]
    pub is_activated: bool,
}

fn default_true() -> bool {
    true
}

impl User {
    /// 创建新用户 (K-1 #4 + name 非空)。
    pub fn new(open_id: String, name: String) -> Result<Self, LarkError> {
        LarkError::validate_open_id(&open_id)?;
        if name.trim().is_empty() {
            return Err(LarkError::Other("user name is empty".to_string()));
        }
        Ok(Self {
            open_id,
            union_id: None,
            user_id: None,
            name,
            en_name: None,
            nickname: None,
            email: None,
            mobile: None,
            avatar_url: None,
            department_ids: Vec::new(),
            employee_no: None,
            job_title: None,
            is_activated: true,
        })
    }

    /// 字段校验 (K-1 #4/#5/#6 + name 非空)。
    pub fn validate(&self) -> Result<(), LarkError> {
        LarkError::validate_open_id(&self.open_id)?;
        if let Some(email) = &self.email {
            LarkError::validate_email(email)?;
        }
        if let Some(mobile) = &self.mobile {
            LarkError::validate_mobile(mobile)?;
        }
        if self.name.trim().is_empty() {
            return Err(LarkError::Other("user name is empty".to_string()));
        }
        Ok(())
    }

    /// 设置邮箱 (K-1 #5)。
    pub fn with_email(mut self, email: String) -> Result<Self, LarkError> {
        LarkError::validate_email(&email)?;
        self.email = Some(email);
        Ok(self)
    }

    /// 设置手机 (K-1 #6)。
    pub fn with_mobile(mut self, mobile: String) -> Result<Self, LarkError> {
        LarkError::validate_mobile(&mobile)?;
        self.mobile = Some(mobile);
        Ok(self)
    }
}

// ============================================================================
// §3 UserQuery (用户查询参数)
// ============================================================================

/// 用户查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserQuery {
    /// 用户 ID (path 参数, 口径由 `user_id_type` 决定)。
    pub user_id: String,
    /// ID 类型 (query 参数)。
    pub user_id_type: UserIdType,
    /// 部门 ID 类型 (query 参数, 默认 `open_department_id`)。
    #[serde(default = "default_dept_id_type")]
    pub department_id_type: String,
}

fn default_dept_id_type() -> String {
    "open_department_id".to_string()
}

impl UserQuery {
    /// 创建查询 (user_id 按 user_id_type 闭合校验)。
    pub fn new(user_id: String, user_id_type: UserIdType) -> Result<Self, LarkError> {
        match user_id_type {
            UserIdType::OpenId => LarkError::validate_open_id(&user_id)?,
            UserIdType::Email => LarkError::validate_email(&user_id)?,
            UserIdType::UnionId => {
                if user_id.is_empty() || !user_id.starts_with("on_") {
                    return Err(LarkError::Other(format!(
                        "union_id invalid: {user_id} (expected 'on_' prefix)"
                    )));
                }
            }
            UserIdType::UserId => {
                if user_id.trim().is_empty() {
                    return Err(LarkError::Other("user_id is empty".to_string()));
                }
            }
        }
        Ok(Self {
            user_id,
            user_id_type,
            department_id_type: default_dept_id_type(),
        })
    }
}

// ============================================================================
// §4 Department (领域实体)
// ============================================================================

/// 部门状态闭合取值 (`active` / `deleted`)。
pub const DEPARTMENT_STATUSES: &[&str] = &["active", "deleted"];

/// 部门 (领域实体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Department {
    /// 部门 Open ID (非空)。
    pub open_department_id: String,
    /// 部门名称 (非空)。
    pub name: String,
    /// 父部门 ID (顶层为 "0")。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_department_id: Option<String>,
    /// 部门 leader open_id 列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leader_open_ids: Vec<String>,
    /// 成员数量 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    /// 子部门数量 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_department_count: Option<u32>,
    /// 部门排序 (数值越小越靠前)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    /// 状态 (`active` / `deleted`)。
    #[serde(default = "default_dept_status")]
    pub status: String,
}

fn default_dept_status() -> String {
    "active".to_string()
}

impl Department {
    /// 创建新部门 (id / name 非空)。
    pub fn new(open_department_id: String, name: String) -> Result<Self, LarkError> {
        if open_department_id.trim().is_empty() {
            return Err(LarkError::Other("open_department_id is empty".to_string()));
        }
        if name.trim().is_empty() {
            return Err(LarkError::Other("department name is empty".to_string()));
        }
        Ok(Self {
            open_department_id,
            name,
            parent_department_id: None,
            leader_open_ids: Vec::new(),
            member_count: None,
            sub_department_count: None,
            order: None,
            status: default_dept_status(),
        })
    }

    /// 字段校验 (id / name 非空 + leaders K-1 #4 + status 闭合取值)。
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.open_department_id.trim().is_empty() {
            return Err(LarkError::Other("open_department_id is empty".to_string()));
        }
        if self.name.trim().is_empty() {
            return Err(LarkError::Other("department name is empty".to_string()));
        }
        for leader in &self.leader_open_ids {
            LarkError::validate_open_id(leader)?;
        }
        if !DEPARTMENT_STATUSES.contains(&self.status.as_str()) {
            return Err(LarkError::Other(format!(
                "unknown department status: {}",
                self.status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// §5 wire 契约 (请求构造 + 响应映射)
// ============================================================================

/// 用户查询请求 (`GET /contact/v3/users/{user_id}`)。
pub fn build_user_request(query: &UserQuery) -> ApiRequest {
    ApiRequest::get(format!(
        "/contact/v3/users/{}",
        encode_path_segment(&query.user_id)
    ))
    .with_query("user_id_type", query.user_id_type.as_str())
    .with_query("department_id_type", query.department_id_type.clone())
}

/// 用户响应 `data` 载荷 (未知字段容错)。
#[derive(Debug, Clone, Deserialize)]
pub struct UserData {
    /// 用户对象 (必填)。
    pub user: UserWire,
}

/// 用户 wire 形状 (必填: `open_id` / `name`)。
#[derive(Debug, Clone, Deserialize)]
pub struct UserWire {
    /// Open ID (必填)。
    pub open_id: String,
    /// Union ID (可选)。
    #[serde(default)]
    pub union_id: Option<String>,
    /// User ID (可选)。
    #[serde(default)]
    pub user_id: Option<String>,
    /// 用户名 (必填)。
    pub name: String,
    /// 英文名 (可选)。
    #[serde(default)]
    pub en_name: Option<String>,
    /// 昵称 (可选)。
    #[serde(default)]
    pub nickname: Option<String>,
    /// 邮箱 (可选)。
    #[serde(default)]
    pub email: Option<String>,
    /// 手机 (可选)。
    #[serde(default)]
    pub mobile: Option<String>,
    /// 头像 URL (可选)。
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// 部门 ID 列表 (可选)。
    #[serde(default)]
    pub department_ids: Vec<String>,
    /// 工号 (可选)。
    #[serde(default)]
    pub employee_no: Option<String>,
    /// 职位 (可选)。
    #[serde(default)]
    pub job_title: Option<String>,
    /// 是否激活 (可选, 默认 true)。
    #[serde(default)]
    pub is_activated: Option<bool>,
}

/// wire → 领域 (K-1 强校验 + name 非空)。
pub fn map_user(wire: &UserWire) -> Result<User, LarkError> {
    let mut user = User::new(wire.open_id.clone(), wire.name.clone())?;
    user.union_id = wire.union_id.clone();
    user.user_id = wire.user_id.clone();
    user.en_name = wire.en_name.clone();
    user.nickname = wire.nickname.clone();
    user.email = wire.email.clone();
    user.mobile = wire.mobile.clone();
    user.avatar_url = wire.avatar_url.clone();
    user.department_ids = wire.department_ids.clone();
    user.employee_no = wire.employee_no.clone();
    user.job_title = wire.job_title.clone();
    user.is_activated = wire.is_activated.unwrap_or(true);
    user.validate()?;
    Ok(user)
}

/// 部门查询请求 (`GET /contact/v3/departments/{department_id}`)。
///
/// department_id 为空 = 永久错误 (调用方字段校验)。
pub fn build_department_request(department_id: &str) -> Result<ApiRequest, LarkError> {
    if department_id.trim().is_empty() {
        return Err(LarkError::Other("department_id is empty".to_string()));
    }
    Ok(ApiRequest::get(format!(
        "/contact/v3/departments/{}",
        encode_path_segment(department_id.trim())
    ))
    .with_query("department_id_type", "open_department_id"))
}

/// 部门响应 `data` 载荷 (未知字段容错)。
#[derive(Debug, Clone, Deserialize)]
pub struct DepartmentData {
    /// 部门对象 (必填)。
    pub department: DepartmentWire,
}

/// 部门 wire 形状 (必填: `open_department_id` / `name`)。
#[derive(Debug, Clone, Deserialize)]
pub struct DepartmentWire {
    /// 部门 Open ID (必填)。
    pub open_department_id: String,
    /// 部门名称 (必填)。
    pub name: String,
    /// 父部门 ID (可选)。
    #[serde(default)]
    pub parent_department_id: Option<String>,
    /// leader open_id 列表 (可选)。
    #[serde(default)]
    pub leader_open_ids: Vec<String>,
    /// 成员数量 (可选)。
    #[serde(default)]
    pub member_count: Option<u32>,
    /// 子部门数量 (可选)。
    #[serde(default)]
    pub sub_department_count: Option<u32>,
    /// 排序 (可选)。
    #[serde(default)]
    pub order: Option<i32>,
    /// 状态 (可选; 未知取值 = 永久错误)。
    #[serde(default)]
    pub status: Option<String>,
}

/// wire → 领域 (id/name 非空 + leaders K-1 #4 + status 闭合取值)。
pub fn map_department(wire: &DepartmentWire) -> Result<Department, LarkError> {
    let mut dept = Department::new(wire.open_department_id.clone(), wire.name.clone())?;
    dept.parent_department_id = wire.parent_department_id.clone();
    dept.leader_open_ids = wire.leader_open_ids.clone();
    dept.member_count = wire.member_count;
    dept.sub_department_count = wire.sub_department_count;
    dept.order = wire.order;
    if let Some(status) = &wire.status {
        dept.status = status.clone();
    }
    dept.validate()?;
    Ok(dept)
}

// ============================================================================
// §6 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_creation_valid() {
        let user =
            User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string()).expect("valid");
        assert_eq!(user.open_id, "ou_user1234567890abcdef");
        assert_eq!(user.name, "Alice");
        assert!(user.is_activated);
    }

    #[test]
    fn user_rejects_invalid_input() {
        assert!(matches!(
            User::new("invalid".to_string(), "Alice".to_string()),
            Err(LarkError::OpenIdInvalid(_))
        ));
        assert!(matches!(
            User::new("ou_user1234567890abcdef".to_string(), String::new()),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn user_with_email_and_mobile() {
        let user = User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
            .expect("valid")
            .with_email("alice@example.com".to_string())
            .expect("valid email")
            .with_mobile("+8613800138000".to_string())
            .expect("valid mobile");
        assert_eq!(user.email.as_deref(), Some("alice@example.com"));
        assert_eq!(user.mobile.as_deref(), Some("+8613800138000"));
        assert!(user.validate().is_ok());

        assert!(matches!(
            User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
                .expect("valid")
                .with_email("not-an-email".to_string()),
            Err(LarkError::EmailInvalid(_))
        ));
        assert!(matches!(
            User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
                .expect("valid")
                .with_mobile("13800138000".to_string()),
            Err(LarkError::MobileInvalid(_))
        ));
    }

    #[test]
    fn user_query_id_type_validation() {
        let q = UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId)
            .expect("valid");
        assert_eq!(q.user_id_type, UserIdType::OpenId);
        assert_eq!(q.department_id_type, "open_department_id");

        let q = UserQuery::new("user@example.com".to_string(), UserIdType::Email).expect("valid");
        assert_eq!(q.user_id_type, UserIdType::Email);
        assert!(matches!(
            UserQuery::new("not-email".to_string(), UserIdType::Email),
            Err(LarkError::EmailInvalid(_))
        ));
        assert!(UserQuery::new("plain".to_string(), UserIdType::UserId).is_ok());
        assert!(UserQuery::new(String::new(), UserIdType::UserId).is_err());
    }

    #[test]
    fn department_creation_and_validation() {
        let mut dept =
            Department::new("od_dept123".to_string(), "工程部".to_string()).expect("valid");
        assert_eq!(dept.name, "工程部");
        assert!(dept.validate().is_ok());
        dept.leader_open_ids = vec!["ou_leader123".to_string()];
        assert!(dept.validate().is_ok());
        dept.leader_open_ids = vec!["invalid".to_string()];
        assert!(matches!(dept.validate(), Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn department_rejects_empty_fields() {
        assert!(Department::new(String::new(), "x".to_string()).is_err());
        assert!(Department::new("od_dept123".to_string(), String::new()).is_err());
    }

    // ---- wire 契约 ----

    #[test]
    fn build_user_request_encodes_path_and_query() {
        let q = UserQuery::new("user@example.com".to_string(), UserIdType::Email).expect("valid");
        let req = build_user_request(&q);
        assert_eq!(req.path, "/contact/v3/users/user%40example.com");
        assert!(req.query.contains(&("user_id_type".into(), "email".into())));
    }

    #[test]
    fn user_wire_maps_with_unknown_field_tolerance() {
        let data: UserData = serde_json::from_value(serde_json::json!({
            "user": {
                "open_id": "ou_user1234567890abcdef",
                "name": "Alice",
                "email": "alice@example.com",
                "future_field": 1
            },
            "extra": true
        }))
        .expect("parse");
        let user = map_user(&data.user).expect("map");
        assert_eq!(user.name, "Alice");
        assert!(user.is_activated, "缺省 is_activated = true");
    }

    #[test]
    fn user_wire_rejects_bad_identity_fields() {
        // 缺 name → serde 拒 (严格必填)
        let result = serde_json::from_value::<UserData>(serde_json::json!({
            "user": {"open_id": "ou_user1234567890abcdef"}
        }));
        assert!(result.is_err());

        // 非法 open_id → K-1 #4 拒
        let data: UserData = serde_json::from_value(serde_json::json!({
            "user": {"open_id": "bogus", "name": "Alice"}
        }))
        .expect("parse");
        assert!(matches!(
            map_user(&data.user),
            Err(LarkError::OpenIdInvalid(_))
        ));

        // 非法 email → K-1 #5 拒
        let data: UserData = serde_json::from_value(serde_json::json!({
            "user": {"open_id": "ou_user1234567890abcdef", "name": "Alice", "email": "nope"}
        }))
        .expect("parse");
        assert!(matches!(
            map_user(&data.user),
            Err(LarkError::EmailInvalid(_))
        ));
    }

    #[test]
    fn department_wire_maps_and_enforces_closed_status() {
        let data: DepartmentData = serde_json::from_value(serde_json::json!({
            "department": {
                "open_department_id": "od_dept123",
                "name": "工程部",
                "status": "deleted",
                "future_field": 2
            }
        }))
        .expect("parse");
        let dept = map_department(&data.department).expect("map");
        assert_eq!(dept.status, "deleted");

        // 未知 status 取值 → 永久错误 (闭合取值表)
        let data: DepartmentData = serde_json::from_value(serde_json::json!({
            "department": {
                "open_department_id": "od_dept123",
                "name": "工程部",
                "status": "frozen"
            }
        }))
        .expect("parse");
        assert!(matches!(
            map_department(&data.department),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn department_request_rejects_empty_id() {
        assert!(build_department_request("").is_err());
        assert!(build_department_request("   ").is_err());
        let req = build_department_request("od_dept123").expect("request");
        assert_eq!(req.path, "/contact/v3/departments/od_dept123");
    }
}
