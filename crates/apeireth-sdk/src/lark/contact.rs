//! # Lark 通讯录 (per @larksuiteoapi/lark-sdk v0.9.21 商业版 1:1 翻译)
//!
//! 飞书通讯录 `contact/v3/users` / `contact/v3/departments` API 翻译源.
//! 4 实体之一: `User` + `Department`.
//!
//! **2 核心 API** (per v0.9.21 商业版):
//! - `get_user` — 根据 user_id / open_id / email / mobile 查 user
//! - `get_department` — 根据 department_id 查 department
//!
//! **当前 STUB**: 字段保留 1:1 翻译, 走 `get_user` / `get_department` 返 `NotImplemented`.
//!
//! ## 4 User ID 类型守门 (per v0.9.21 商业版 `user_id_type` 字段)
//!
//! - `OpenId` — `ou_xxx` (K-1 #4 强校验)
//! - `UnionId` — `on_xxx`
//! - `UserId` — `user_xxx` (R21 续真接时支持)
//! - `Email` — RFC 5322 (K-1 #5 强校验)
//!
//! ## 6 K-1 强校验触点
//!
//! - `open_id` (K-1 #4)
//! - `email` (K-1 #5)
//! - `mobile` (K-1 #6)

use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;

// ============================================================================
// §1 UserIdType (4 variant, 1:1 翻译 v0.9.21 商业版 `user_id_type` enum)
// ============================================================================

/// 用户 ID 类型 (4 variant, per v0.9.21 商业版 `user_id_type` 字段).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserIdType {
    /// Open ID (per `open_id`, K-1 #4 强校验 `ou_` 前缀).
    #[default]
    OpenId,
    /// Union ID (per `union_id`, `on_` 前缀).
    UnionId,
    /// User ID (per `user_id`, 飞书 user_id 字段).
    UserId,
    /// Email (per `email`, K-1 #5 RFC 5322 强校验).
    Email,
}

impl UserIdType {
    /// 字符串 (1:1 翻译 v0.9.21 商业版 `user_id_type` snake_case).
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
// §2 User (per v0.9.21 商业版 1:1)
// ============================================================================

/// 用户 (per v0.9.21 商业版 `contact/v3/users/{user_id}` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Open ID (per `open_id` 字段, K-1 #4 强校验 `ou_` 前缀).
    pub open_id: String,
    /// Union ID (per `union_id` 字段, 可选, 跨租户).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_id: Option<String>,
    /// User ID (per `user_id` 字段, 可选, 租户内 user_id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 用户名 (per `name` 字段, 非空).
    pub name: String,
    /// 英文名 (per `en_name` 字段, 可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub en_name: Option<String>,
    /// 昵称 (per `nickname` 字段, 可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// 邮箱 (per `email` 字段, K-1 #5 强校验).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 手机 (per `mobile` 字段, K-1 #6 强校验 E.164).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    /// 头像 URL (per `avatar` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 部门 ID 列表 (per `department_ids` 字段, R21 续真接).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub department_ids: Vec<String>,
    /// 工号 (per `employee_no` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employee_no: Option<String>,
    /// 职位 (per `job_title` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_title: Option<String>,
    /// 是否激活 (per `is_activated` 字段).
    #[serde(default = "default_true")]
    pub is_activated: bool,
}

fn default_true() -> bool {
    true
}

impl User {
    /// 创建新 user (STUB 模式不真调飞书 API).
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

    /// 校验 3 字段 (K-1 强校验守门: open_id / email / mobile).
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

    /// 用 email 构造.
    pub fn with_email(mut self, email: String) -> Result<Self, LarkError> {
        LarkError::validate_email(&email)?;
        self.email = Some(email);
        Ok(self)
    }

    /// 用 mobile 构造.
    pub fn with_mobile(mut self, mobile: String) -> Result<Self, LarkError> {
        LarkError::validate_mobile(&mobile)?;
        self.mobile = Some(mobile);
        Ok(self)
    }
}

// ============================================================================
// §3 UserQuery (per get_user 1:1)
// ============================================================================

/// 用户查询参数 (per v0.9.21 商业版 `contact/v3/users/{user_id}?user_id_type=...` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserQuery {
    /// 用户 ID (per `user_id` path 参数).
    pub user_id: String,
    /// ID 类型 (per `user_id_type` query 参数).
    pub user_id_type: UserIdType,
    /// 部门 ID 类型 (per `department_id_type` query 参数, 默认 `open_department_id`).
    #[serde(default = "default_dept_id_type")]
    pub department_id_type: String,
}

fn default_dept_id_type() -> String {
    "open_department_id".to_string()
}

impl UserQuery {
    /// 创建查询 (per user_id_type 走 K-1 强校验).
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
                if user_id.is_empty() {
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
// §4 Department (per v0.9.21 商业版 1:1)
// ============================================================================

/// 部门 (per v0.9.21 商业版 `contact/v3/departments/{department_id}` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Department {
    /// 部门 Open ID (per `open_department_id` 字段, K-1 强校验).
    pub open_department_id: String,
    /// 部门名称 (per `name` 字段, 非空).
    pub name: String,
    /// 父部门 ID (per `parent_department_id` 字段, 顶层为 "0").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_department_id: Option<String>,
    /// 部门 leader 的 open_id 列表 (per `leaders` 字段).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leader_open_ids: Vec<String>,
    /// 成员数量 (per `member_count` 字段, R21 真接飞书后才有).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    /// 子部门数量 (per `sub_department_count` 字段, R21 真接飞书后才有).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_department_count: Option<u32>,
    /// 部门排序 (per `order` 字段, 数值越小越靠前).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    /// 状态 (per `status` 字段, `active` / `deleted`).
    #[serde(default = "default_dept_status")]
    pub status: String,
}

fn default_dept_status() -> String {
    "active".to_string()
}

impl Department {
    /// 创建新 department.
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

    /// 校验 2 字段 (K-1 强校验守门: open_department_id / name).
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
        Ok(())
    }
}

// ============================================================================
// §5 单元测试
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
    fn user_reject_invalid_open_id() {
        let result = User::new("invalid".to_string(), "Alice".to_string());
        assert!(matches!(result, Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn user_reject_empty_name() {
        let result = User::new("ou_user1234567890abcdef".to_string(), String::new());
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn user_with_email() {
        let user = User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
            .expect("valid")
            .with_email("alice@example.com".to_string())
            .expect("valid email");
        assert_eq!(user.email.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn user_with_email_rejects_invalid() {
        let result = User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
            .expect("valid")
            .with_email("not-an-email".to_string());
        assert!(matches!(result, Err(LarkError::EmailInvalid(_))));
    }

    #[test]
    fn user_with_mobile() {
        let user = User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
            .expect("valid")
            .with_mobile("+8613800138000".to_string())
            .expect("valid mobile");
        assert_eq!(user.mobile.as_deref(), Some("+8613800138000"));
    }

    #[test]
    fn user_with_mobile_rejects_invalid() {
        let result = User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
            .expect("valid")
            .with_mobile("13800138000".to_string());
        assert!(matches!(result, Err(LarkError::MobileInvalid(_))));
    }

    #[test]
    fn user_validate_full() {
        let mut user = User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string())
            .expect("valid")
            .with_email("alice@example.com".to_string())
            .expect("valid")
            .with_mobile("+8613800138000".to_string())
            .expect("valid");
        user.department_ids = vec!["od_dept123".to_string()];
        assert!(user.validate().is_ok());
    }

    #[test]
    fn user_query_open_id() {
        let q = UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId)
            .expect("valid");
        assert_eq!(q.user_id_type, UserIdType::OpenId);
    }

    #[test]
    fn user_query_email() {
        let q = UserQuery::new("user@example.com".to_string(), UserIdType::Email).expect("valid");
        assert_eq!(q.user_id_type, UserIdType::Email);
    }

    #[test]
    fn user_query_email_rejects_invalid() {
        let result = UserQuery::new("not-email".to_string(), UserIdType::Email);
        assert!(matches!(result, Err(LarkError::EmailInvalid(_))));
    }

    #[test]
    fn department_creation_valid() {
        let dept = Department::new("od_dept123".to_string(), "工程部".to_string()).expect("valid");
        assert_eq!(dept.name, "工程部");
    }

    #[test]
    fn department_reject_empty_id() {
        let result = Department::new(String::new(), "工程部".to_string());
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn department_reject_empty_name() {
        let result = Department::new("od_dept123".to_string(), String::new());
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn department_validate_leaders() {
        let mut dept =
            Department::new("od_dept123".to_string(), "工程部".to_string()).expect("valid");
        dept.leader_open_ids = vec!["ou_leader123".to_string()];
        assert!(dept.validate().is_ok());
    }

    #[test]
    fn department_validate_rejects_bad_leader() {
        let mut dept =
            Department::new("od_dept123".to_string(), "工程部".to_string()).expect("valid");
        dept.leader_open_ids = vec!["invalid".to_string()];
        assert!(matches!(dept.validate(), Err(LarkError::OpenIdInvalid(_))));
    }
}
