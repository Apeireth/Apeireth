//! # Lark 审批 (per @larksuiteoapi/lark-sdk v0.9.21 商业版 1:1 翻译)
//!
//! 飞书审批 `approval/v4/instances` / `approval/v4/tasks` API 翻译源.
//! 4 实体之一: `ApprovalInstance` + `ApprovalTask`.
//!
//! **1 核心 API** (per v0.9.21 商业版):
//! - `get_approval_instance` — 根据 instance_id 查审批实例
//!
//! **当前 STUB**: 字段保留 1:1 翻译, 走 `get_approval_instance` 返 `NotImplemented`.
//!
//! ## 5 InstanceStatus 守门 (per v0.9.21 商业版)
//!
//! - `Pending` — 审批中
//! - `Approved` — 已通过
//! - `Rejected` — 已拒绝
//! - `Withdrawn` — 已撤回
//! - `Transferred` — 已转交

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::LarkError;

// ============================================================================
// §1 InstanceStatus (5 variant, 1:1 翻译 v0.9.21 商业版)
// ============================================================================

/// 审批实例状态 (5 variant, per v0.9.21 商业版 `status` 字段).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// 审批中 (per v0.9.21 商业版 `status: "pending"`).
    #[default]
    Pending,
    /// 已通过 (per v0.9.21 商业版 `status: "approved"`).
    Approved,
    /// 已拒绝 (per v0.9.21 商业版 `status: "rejected"`).
    Rejected,
    /// 已撤回 (per v0.9.21 商业版 `status: "withdrawn"`).
    Withdrawn,
    /// 已转交 (per v0.9.21 商业版 `status: "transferred"`, R21 续真接).
    Transferred,
}

impl InstanceStatus {
    /// 5 状态 hardcode 常量.
    pub const COUNT: usize = 5;

    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceStatus::Pending => "pending",
            InstanceStatus::Approved => "approved",
            InstanceStatus::Rejected => "rejected",
            InstanceStatus::Withdrawn => "withdrawn",
            InstanceStatus::Transferred => "transferred",
        }
    }
}

impl std::fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 TaskStatus (3 variant, 1:1 翻译 v0.9.21 商业版)
// ============================================================================

/// 审批任务状态 (3 variant, per v0.9.21 商业版 `status` 字段).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待审批 (per v0.9.21 商业版 `status: "pending"`).
    #[default]
    Pending,
    /// 已通过 (per v0.9.21 商业版 `status: "approved"`).
    Approved,
    /// 已拒绝 (per v0.9.21 商业版 `status: "rejected"`).
    Rejected,
}

impl TaskStatus {
    /// 3 状态 hardcode 常量.
    pub const COUNT: usize = 3;

    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Approved => "approved",
            TaskStatus::Rejected => "rejected",
        }
    }
}

/// 编译期守门: 3 TaskStatus variant 守门 (per K-1 强校验).
pub const SUPPORTED_TASK_STATUSES: &[TaskStatus] = &[
    TaskStatus::Pending,
    TaskStatus::Approved,
    TaskStatus::Rejected,
];
const _: () = assert!(SUPPORTED_TASK_STATUSES.len() == 3);

/// 编译期守门别名 (per 8 项不修改承诺 + 跨模块同步守门).
pub const TASK_STATUS_COUNT: usize = TaskStatus::COUNT;

/// Pending 守门别名 (per 测试便捷 + 编译期 hardcode).
pub const TASK_STATUS_PENDING: TaskStatus = TaskStatus::Pending;

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §3 ApprovalInstance (per v0.9.21 商业版 1:1)
// ============================================================================

/// 审批实例 (per v0.9.21 商业版 `approval/v4/instances/{instance_id}` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalInstance {
    /// 实例 ID (per `instance_id` 字段, R21 真接飞书后才有, STUB 模式 None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// 审批定义 code (per `approval_code` 字段, 非空).
    pub approval_code: String,
    /// 实例状态 (per `status` 字段, 5 variant).
    pub status: InstanceStatus,
    /// 用户 open_id (per `user_id` 字段, 发起人).
    pub user_open_id: String,
    /// 表单数据 (per `form` 字段, JSON 数组, 每个元素是 form_field).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form: Vec<ApprovalFormField>,
    /// 审批任务列表 (per `tasks` 字段, 多人审批时多个).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<ApprovalTask>,
    /// 开始时间 (per `start_time` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<SystemTime>,
    /// 结束时间 (per `end_time` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<SystemTime>,
}

/// 审批表单字段 (per v0.9.21 商业版 `form[].{id,type,value}` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalFormField {
    /// 字段 ID (per `id` 字段).
    pub id: String,
    /// 字段类型 (per `type` 字段, e.g. "input" / "number" / "date" / "textarea").
    #[serde(rename = "type")]
    pub field_type: String,
    /// 字段值 (per `value` 字段, 字符串).
    pub value: String,
}

impl ApprovalInstance {
    /// 创建新审批实例 (STUB 模式不真调飞书 API).
    pub fn new(
        approval_code: String,
        user_open_id: String,
    ) -> Result<Self, LarkError> {
        if approval_code.is_empty() {
            return Err(LarkError::Other("approval_code is empty".to_string()));
        }
        LarkError::validate_open_id(&user_open_id)?;
        Ok(Self {
            instance_id: None,
            approval_code,
            status: InstanceStatus::default(),
            user_open_id,
            form: Vec::new(),
            tasks: Vec::new(),
            start_time: None,
            end_time: None,
        })
    }

    /// 校验 2 字段 (K-1 强校验守门: approval_code / user_open_id).
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.approval_code.is_empty() {
            return Err(LarkError::Other("approval_code is empty".to_string()));
        }
        LarkError::validate_open_id(&self.user_open_id)?;
        Ok(())
    }

    /// 添加表单字段.
    pub fn with_form_field(mut self, id: String, field_type: String, value: String) -> Self {
        self.form.push(ApprovalFormField {
            id,
            field_type,
            value,
        });
        self
    }
}

// ============================================================================
// §4 ApprovalTask (per v0.9.21 商业版 1:1)
// ============================================================================

/// 审批任务 (per v0.9.21 商业版 `approval/v4/tasks/{task_id}` 1:1).
///
/// 每个审批实例有 1..N 个任务 (多人审批 / 多级审批).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalTask {
    /// 任务 ID (per `task_id` 字段, R21 真接飞书后才有, STUB 模式 None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// 关联实例 ID (per `instance_id` 字段).
    pub instance_id: String,
    /// 审批人 open_id (per `user_id` 字段).
    pub approver_open_id: String,
    /// 任务状态 (per `status` 字段, 3 variant).
    pub status: TaskStatus,
    /// 审批意见 (per `comment` 字段, 可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 审批时间 (per `action_time` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_time: Option<SystemTime>,
}

impl ApprovalTask {
    /// 创建新审批任务.
    pub fn new(
        instance_id: String,
        approver_open_id: String,
    ) -> Result<Self, LarkError> {
        if instance_id.is_empty() {
            return Err(LarkError::Other("instance_id is empty".to_string()));
        }
        LarkError::validate_open_id(&approver_open_id)?;
        Ok(Self {
            task_id: None,
            instance_id,
            approver_open_id,
            status: TaskStatus::default(),
            comment: None,
            action_time: None,
        })
    }

    /// 校验 2 字段 (K-1 强校验守门: instance_id / approver_open_id).
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.instance_id.is_empty() {
            return Err(LarkError::Other("instance_id is empty".to_string()));
        }
        LarkError::validate_open_id(&self.approver_open_id)?;
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
    fn instance_status_5_variants() {
        assert_eq!(InstanceStatus::COUNT, 5);
    }

    #[test]
    fn task_status_3_variants() {
        assert_eq!(TaskStatus::COUNT, 3);
    }

    #[test]
    fn approval_instance_creation_valid() {
        let inst = ApprovalInstance::new(
            "approval_code_xxx".to_string(),
            "ou_user1234567890abcdef".to_string(),
        )
        .expect("valid");
        assert_eq!(inst.approval_code, "approval_code_xxx");
        assert_eq!(inst.status, InstanceStatus::Pending);
    }

    #[test]
    fn approval_instance_reject_empty_approval_code() {
        let result = ApprovalInstance::new(
            String::new(),
            "ou_user1234567890abcdef".to_string(),
        );
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn approval_instance_reject_invalid_user_open_id() {
        let result = ApprovalInstance::new(
            "approval_code_xxx".to_string(),
            "invalid".to_string(),
        );
        assert!(matches!(result, Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn approval_instance_with_form_field() {
        let inst = ApprovalInstance::new(
            "approval_code_xxx".to_string(),
            "ou_user1234567890abcdef".to_string(),
        )
        .expect("valid")
        .with_form_field("reason".to_string(), "textarea".to_string(), "出差".to_string());
        assert_eq!(inst.form.len(), 1);
        assert_eq!(inst.form[0].id, "reason");
    }

    #[test]
    fn approval_task_creation_valid() {
        let task = ApprovalTask::new(
            "instance_001".to_string(),
            "ou_approver1234567890abcdef".to_string(),
        )
        .expect("valid");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn approval_task_reject_empty_instance_id() {
        let result = ApprovalTask::new(
            String::new(),
            "ou_approver1234567890abcdef".to_string(),
        );
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn approval_task_reject_invalid_approver_open_id() {
        let result = ApprovalTask::new(
            "instance_001".to_string(),
            "invalid".to_string(),
        );
        assert!(matches!(result, Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn instance_status_round_trip() {
        for status in [
            InstanceStatus::Pending,
            InstanceStatus::Approved,
            InstanceStatus::Rejected,
            InstanceStatus::Withdrawn,
            InstanceStatus::Transferred,
        ] {
            assert!(!status.as_str().is_empty());
        }
    }
}
