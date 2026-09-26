//! # lark 审批面 (审批实例 wire 契约)
//!
//! 平台审批端点: `GET /approval/v4/instances/{instance_id}`。
//!
//! ## wire 契约 (严格形状 + 未知字段容错)
//!
//! - 响应 `data`: `{"instance": {...}}`, 未知字段忽略;
//! - 实例必填: `approval_code` (非空) + `status` (闭合取值) + `user_open_id` (`ou_` 前缀);
//! - `form[]` 条目必填: `id` / `type` / `value`;
//! - `tasks[]` 条目必填: `instance_id` / `approver_open_id` / `status` (闭合取值);
//! - 时间字段 (`start_time` / `end_time` / `action_time`) 为 RFC3339 字符串。
//!
//! ## 字段校验 (响应映射时完成)
//!
//! - K-1 #4 `user_open_id` / `approver_open_id` (每项);
//! - 状态未知取值 = 永久错误 (闭合枚举 [`InstanceStatus`] / [`TaskStatus`]);
//! - 时间区间: 若两端都给出, `end_time > start_time`。

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;
use crate::lark::http::{encode_path_segment, ApiRequest};

// ============================================================================
// §1 InstanceStatus (5 variant 闭合枚举) / TaskStatus (3 variant 闭合枚举)
// ============================================================================

/// 审批实例状态 (5 variant 闭合枚举)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// 审批中 (`pending`)。
    #[default]
    Pending,
    /// 已通过 (`approved`)。
    Approved,
    /// 已拒绝 (`rejected`)。
    Rejected,
    /// 已撤回 (`withdrawn`)。
    Withdrawn,
    /// 已转交 (`transferred`)。
    Transferred,
}

impl InstanceStatus {
    /// 5 状态 hardcode 常量。
    pub const COUNT: usize = 5;

    /// wire 字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceStatus::Pending => "pending",
            InstanceStatus::Approved => "approved",
            InstanceStatus::Rejected => "rejected",
            InstanceStatus::Withdrawn => "withdrawn",
            InstanceStatus::Transferred => "transferred",
        }
    }

    /// 从 wire 字符串解析 (未知值 → `None`, 调用方按永久错误处理)。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(InstanceStatus::Pending),
            "approved" => Some(InstanceStatus::Approved),
            "rejected" => Some(InstanceStatus::Rejected),
            "withdrawn" => Some(InstanceStatus::Withdrawn),
            "transferred" => Some(InstanceStatus::Transferred),
            _ => None,
        }
    }
}

impl std::fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 审批任务状态 (3 variant 闭合枚举)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待审批 (`pending`)。
    #[default]
    Pending,
    /// 已通过 (`approved`)。
    Approved,
    /// 已拒绝 (`rejected`)。
    Rejected,
}

impl TaskStatus {
    /// 3 状态 hardcode 常量。
    pub const COUNT: usize = 3;

    /// wire 字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Approved => "approved",
            TaskStatus::Rejected => "rejected",
        }
    }

    /// 从 wire 字符串解析 (未知值 → `None`)。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskStatus::Pending),
            "approved" => Some(TaskStatus::Approved),
            "rejected" => Some(TaskStatus::Rejected),
            _ => None,
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: 3 TaskStatus variant。
pub const SUPPORTED_TASK_STATUSES: &[TaskStatus] = &[
    TaskStatus::Pending,
    TaskStatus::Approved,
    TaskStatus::Rejected,
];
const _: () = assert!(SUPPORTED_TASK_STATUSES.len() == 3);

/// 编译期守门别名。
pub const TASK_STATUS_COUNT: usize = TaskStatus::COUNT;

/// Pending 守门别名 (测试便捷)。
pub const TASK_STATUS_PENDING: TaskStatus = TaskStatus::Pending;

// ============================================================================
// §2 ApprovalInstance / ApprovalFormField / ApprovalTask (领域实体)
// ============================================================================

/// 审批实例 (领域实体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalInstance {
    /// 实例 ID (平台颁发后才有)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// 审批定义 code (非空)。
    pub approval_code: String,
    /// 实例状态 (闭合枚举)。
    pub status: InstanceStatus,
    /// 发起人 open_id (K-1 #4)。
    pub user_open_id: String,
    /// 表单数据。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form: Vec<ApprovalFormField>,
    /// 审批任务列表 (多人/多级审批)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<ApprovalTask>,
    /// 开始时间 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<SystemTime>,
    /// 结束时间 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<SystemTime>,
}

/// 审批表单字段 (`form[].{id,type,value}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalFormField {
    /// 字段 ID。
    pub id: String,
    /// 字段类型 (e.g. "input" / "number" / "date" / "textarea")。
    #[serde(rename = "type")]
    pub field_type: String,
    /// 字段值 (字符串)。
    pub value: String,
}

impl ApprovalInstance {
    /// 创建新审批实例 (approval_code 非空 + user_open_id K-1 #4)。
    pub fn new(approval_code: String, user_open_id: String) -> Result<Self, LarkError> {
        if approval_code.trim().is_empty() {
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

    /// 字段校验 (approval_code / user_open_id / 时间区间 / 任务逐项)。
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.approval_code.trim().is_empty() {
            return Err(LarkError::Other("approval_code is empty".to_string()));
        }
        LarkError::validate_open_id(&self.user_open_id)?;
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            if end <= start {
                return Err(LarkError::Other(
                    "approval instance: end_time must be > start_time".to_string(),
                ));
            }
        }
        for task in &self.tasks {
            task.validate()?;
        }
        Ok(())
    }

    /// 追加表单字段。
    pub fn with_form_field(mut self, id: String, field_type: String, value: String) -> Self {
        self.form.push(ApprovalFormField {
            id,
            field_type,
            value,
        });
        self
    }
}

/// 审批任务 (领域实体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalTask {
    /// 任务 ID (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// 关联实例 ID (非空)。
    pub instance_id: String,
    /// 审批人 open_id (K-1 #4)。
    pub approver_open_id: String,
    /// 任务状态 (闭合枚举)。
    pub status: TaskStatus,
    /// 审批意见 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 审批时间 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_time: Option<SystemTime>,
}

impl ApprovalTask {
    /// 创建新审批任务 (instance_id 非空 + approver_open_id K-1 #4)。
    pub fn new(instance_id: String, approver_open_id: String) -> Result<Self, LarkError> {
        if instance_id.trim().is_empty() {
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

    /// 字段校验 (instance_id / approver_open_id)。
    pub fn validate(&self) -> Result<(), LarkError> {
        if self.instance_id.trim().is_empty() {
            return Err(LarkError::Other("instance_id is empty".to_string()));
        }
        LarkError::validate_open_id(&self.approver_open_id)?;
        Ok(())
    }
}

// ============================================================================
// §3 wire 契约 (请求构造 + 响应映射)
// ============================================================================

/// 实例查询请求 (`GET /approval/v4/instances/{instance_id}`)。
///
/// instance_id 为空 = 永久错误 (调用方字段校验)。
pub fn build_instance_request(instance_id: &str) -> Result<ApiRequest, LarkError> {
    if instance_id.trim().is_empty() {
        return Err(LarkError::Other("instance_id is empty".to_string()));
    }
    Ok(ApiRequest::get(format!(
        "/approval/v4/instances/{}",
        encode_path_segment(instance_id.trim())
    )))
}

/// 实例响应 `data` 载荷 (未知字段容错)。
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalInstanceData {
    /// 实例对象 (必填)。
    pub instance: ApprovalInstanceWire,
}

/// 实例 wire 形状 (必填: `approval_code` / `status` / `user_open_id`)。
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalInstanceWire {
    /// 实例 ID (可选)。
    #[serde(default)]
    pub instance_id: Option<String>,
    /// 审批定义 code (必填)。
    pub approval_code: String,
    /// 状态 (必填, 闭合取值)。
    pub status: String,
    /// 发起人 open_id (必填)。
    pub user_open_id: String,
    /// 表单 (可选)。
    #[serde(default)]
    pub form: Vec<ApprovalFormFieldWire>,
    /// 任务 (可选)。
    #[serde(default)]
    pub tasks: Vec<ApprovalTaskWire>,
    /// 开始时间 (可选, RFC3339)。
    #[serde(default)]
    pub start_time: Option<String>,
    /// 结束时间 (可选, RFC3339)。
    #[serde(default)]
    pub end_time: Option<String>,
}

/// 表单字段 wire 形状 (必填: `id` / `type` / `value`)。
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalFormFieldWire {
    /// 字段 ID。
    pub id: String,
    /// 字段类型。
    #[serde(rename = "type")]
    pub field_type: String,
    /// 字段值。
    pub value: String,
}

/// 任务 wire 形状 (必填: `instance_id` / `approver_open_id` / `status`)。
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalTaskWire {
    /// 任务 ID (可选)。
    #[serde(default)]
    pub task_id: Option<String>,
    /// 关联实例 ID。
    pub instance_id: String,
    /// 审批人 open_id。
    pub approver_open_id: String,
    /// 状态 (闭合取值)。
    pub status: String,
    /// 意见 (可选)。
    #[serde(default)]
    pub comment: Option<String>,
    /// 审批时间 (可选, RFC3339)。
    #[serde(default)]
    pub action_time: Option<String>,
}

/// wire → 领域 (状态闭合解析 + K-1 #4 + 时间区间)。
pub fn map_instance(wire: &ApprovalInstanceWire) -> Result<ApprovalInstance, LarkError> {
    let status = InstanceStatus::parse(&wire.status).ok_or_else(|| {
        LarkError::Other(format!(
            "malformed approval instance: unknown status value '{}'",
            wire.status
        ))
    })?;
    let mut instance =
        ApprovalInstance::new(wire.approval_code.clone(), wire.user_open_id.clone())?;
    instance.instance_id = wire.instance_id.clone();
    instance.status = status;
    instance.start_time = wire
        .start_time
        .as_deref()
        .map(|s| parse_time(s, "start_time"))
        .transpose()?;
    instance.end_time = wire
        .end_time
        .as_deref()
        .map(|s| parse_time(s, "end_time"))
        .transpose()?;
    instance.form = wire
        .form
        .iter()
        .map(|f| ApprovalFormField {
            id: f.id.clone(),
            field_type: f.field_type.clone(),
            value: f.value.clone(),
        })
        .collect();
    instance.tasks = wire
        .tasks
        .iter()
        .map(map_task)
        .collect::<Result<Vec<_>, _>>()?;
    instance.validate()?;
    Ok(instance)
}

fn map_task(wire: &ApprovalTaskWire) -> Result<ApprovalTask, LarkError> {
    let status = TaskStatus::parse(&wire.status).ok_or_else(|| {
        LarkError::Other(format!(
            "malformed approval task: unknown status value '{}'",
            wire.status
        ))
    })?;
    let mut task = ApprovalTask::new(wire.instance_id.clone(), wire.approver_open_id.clone())?;
    task.task_id = wire.task_id.clone();
    task.status = status;
    task.comment = wire.comment.clone();
    task.action_time = wire
        .action_time
        .as_deref()
        .map(|s| parse_time(s, "action_time"))
        .transpose()?;
    task.validate()?;
    Ok(task)
}

/// RFC3339 → SystemTime (严格; 失败 = 永久错误)。
fn parse_time(value: &str, field: &'static str) -> Result<SystemTime, LarkError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|dt| dt.with_timezone(&Utc))
        .map(|dt| dt.into())
        .map_err(|_| {
            LarkError::Other(format!(
                "malformed approval payload: {field} is not RFC3339 (len={})",
                value.len()
            ))
        })
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_enums_round_trip() {
        assert_eq!(InstanceStatus::COUNT, 5);
        assert_eq!(TaskStatus::COUNT, 3);
        for s in [
            InstanceStatus::Pending,
            InstanceStatus::Approved,
            InstanceStatus::Rejected,
            InstanceStatus::Withdrawn,
            InstanceStatus::Transferred,
        ] {
            assert_eq!(InstanceStatus::parse(s.as_str()), Some(s));
        }
        for s in [
            TaskStatus::Pending,
            TaskStatus::Approved,
            TaskStatus::Rejected,
        ] {
            assert_eq!(TaskStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(InstanceStatus::parse("bogus"), None);
        assert_eq!(TaskStatus::parse("bogus"), None);
    }

    #[test]
    fn approval_instance_construction_and_validation() {
        let inst = ApprovalInstance::new(
            "approval_code_xxx".to_string(),
            "ou_user1234567890abcdef".to_string(),
        )
        .expect("valid");
        assert_eq!(inst.status, InstanceStatus::Pending);
        assert!(inst.validate().is_ok());
        assert!(
            ApprovalInstance::new(String::new(), "ou_user1234567890abcdef".to_string()).is_err()
        );
        assert!(ApprovalInstance::new("code".to_string(), "invalid".to_string()).is_err());
    }

    #[test]
    fn approval_task_construction_and_validation() {
        let task = ApprovalTask::new(
            "instance_001".to_string(),
            "ou_approver1234567890abcdef".to_string(),
        )
        .expect("valid");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.validate().is_ok());
        assert!(
            ApprovalTask::new(String::new(), "ou_approver1234567890abcdef".to_string()).is_err()
        );
        assert!(ApprovalTask::new("instance_001".to_string(), "invalid".to_string()).is_err());
    }

    #[test]
    fn approval_instance_with_form_field() {
        let inst = ApprovalInstance::new(
            "approval_code_xxx".to_string(),
            "ou_user1234567890abcdef".to_string(),
        )
        .expect("valid")
        .with_form_field(
            "reason".to_string(),
            "textarea".to_string(),
            "出差".to_string(),
        );
        assert_eq!(inst.form.len(), 1);
        assert_eq!(inst.form[0].id, "reason");
    }

    // ---- wire 契约 ----

    #[test]
    fn build_instance_request_shapes_path() {
        assert!(build_instance_request("").is_err());
        assert!(build_instance_request("  ").is_err());
        let req = build_instance_request("instance_001").expect("request");
        assert_eq!(req.path, "/approval/v4/instances/instance_001");
    }

    #[test]
    fn wire_instance_maps_full_payload_with_unknown_fields() {
        let data: ApprovalInstanceData = serde_json::from_value(serde_json::json!({
            "instance": {
                "instance_id": "inst_001",
                "approval_code": "approval_code_xxx",
                "status": "approved",
                "user_open_id": "ou_user1234567890abcdef",
                "form": [{"id": "reason", "type": "textarea", "value": "出差"}],
                "tasks": [{
                    "task_id": "task_1",
                    "instance_id": "inst_001",
                    "approver_open_id": "ou_approver1234567890abcdef",
                    "status": "approved",
                    "comment": "ok",
                    "action_time": "2026-08-05T11:00:00Z"
                }],
                "start_time": "2026-08-05T10:00:00Z",
                "end_time": "2026-08-05T12:00:00Z",
                "future_field": {"x": 1}
            }
        }))
        .expect("parse");
        let inst = map_instance(&data.instance).expect("map");
        assert_eq!(inst.status, InstanceStatus::Approved);
        assert_eq!(inst.form.len(), 1);
        assert_eq!(inst.tasks.len(), 1);
        assert_eq!(inst.tasks[0].status, TaskStatus::Approved);
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn wire_instance_rejects_unknown_status_and_missing_fields() {
        // 未知状态 → 永久错误 (闭合枚举)
        let data: ApprovalInstanceData = serde_json::from_value(serde_json::json!({
            "instance": {
                "approval_code": "c",
                "status": "exploded",
                "user_open_id": "ou_user1234567890abcdef"
            }
        }))
        .expect("parse");
        assert!(matches!(
            map_instance(&data.instance),
            Err(LarkError::Other(_))
        ));

        // 缺 status → serde 拒 (严格必填)
        let result = serde_json::from_value::<ApprovalInstanceData>(serde_json::json!({
            "instance": {"approval_code": "c", "user_open_id": "ou_user1234567890abcdef"}
        }));
        assert!(result.is_err());
    }

    #[test]
    fn wire_instance_rejects_bad_open_id_and_bad_time_range() {
        // 发起人 open_id 非法 → K-1 #4
        let data: ApprovalInstanceData = serde_json::from_value(serde_json::json!({
            "instance": {"approval_code": "c", "status": "pending", "user_open_id": "nope"}
        }))
        .expect("parse");
        assert!(matches!(
            map_instance(&data.instance),
            Err(LarkError::OpenIdInvalid(_))
        ));

        // end <= start → 永久错误
        let data: ApprovalInstanceData = serde_json::from_value(serde_json::json!({
            "instance": {
                "approval_code": "c",
                "status": "pending",
                "user_open_id": "ou_user1234567890abcdef",
                "start_time": "2026-08-05T12:00:00Z",
                "end_time": "2026-08-05T10:00:00Z"
            }
        }))
        .expect("parse");
        assert!(matches!(
            map_instance(&data.instance),
            Err(LarkError::Other(_))
        ));

        // 任务 open_id 非法 → K-1 #4 (逐项校验)
        let data: ApprovalInstanceData = serde_json::from_value(serde_json::json!({
            "instance": {
                "approval_code": "c",
                "status": "pending",
                "user_open_id": "ou_user1234567890abcdef",
                "tasks": [{"instance_id": "i", "approver_open_id": "bad", "status": "pending"}]
            }
        }))
        .expect("parse");
        assert!(matches!(
            map_instance(&data.instance),
            Err(LarkError::OpenIdInvalid(_))
        ));
    }
}
