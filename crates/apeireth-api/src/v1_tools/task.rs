//! `/v1/tools/task/invoke` — **R20 阶段 4 估补** (6 actions, 5 K-1 强校验, storage 抽象)
//!
//! **6 actions** (per R20 阶段 4 任务规范):
//! - `list` — 全量列 (可按 status / priority / assignee 过滤)
//! - `create` — 新建 (id 内部 UUID v4)
//! - `update` — 更新 (id 由 caller 传, 部分字段更新)
//! - `delete` — 删
//! - `get` — 单查
//! - `complete` — 标完成 (status: pending → done, 返 done_at ts)
//!
//! **7 实体字段** (per R20 阶段 4 任务规范):
//! - `id` (UUID v4) / `title` (K-1 非空) / `description` (可选) /
//!   `status` (K-1 枚举: pending / in_progress / done / cancelled) /
//!   `priority` (K-1 0-3: low/med/high/urgent) /
//!   `due_date` (K-1 ISO 8601 date) / `assignee` (K-1 邮箱)
//!
//! **5 K-1 强校验** (per R20 阶段 4 任务规范):
//! 1. **title 非空** — 拒空字符串
//! 2. **status 枚举** — 4 值: pending / in_progress / done / cancelled
//! 3. **priority 0-3** — 0/1/2/3 (low/med/high/urgent)
//! 4. **due_date ISO 8601** — YYYY-MM-DD (date, 不接 datetime, 简化版)
//! 5. **assignee 邮箱** — 复用 `calendar::validate_email`
//!
//! **存储**: `Arc<dyn EntityStorage<Task>>` — 默认 `InMemoryStorage`
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 6 actions 全部真实现 (非 stub 501)
//! - ✅ 5 K-1 强校验真跑
//! - ✅ storage trait 真用
//! - ✅ task ID 用 UUID v4
//!
//! **6 哲学锚穿透**:
//! - 锚 #1: 6 actions 真接, 5 K-1 真跑
//! - 锚 #2: `TASK_ACTIONS_COUNT = 6` const assert
//! - 锚 #3: `#![deny(unsafe_code)]` 继承
//! - 锚 #4: storage trait NotFound 真返
//! - 锚 #5: 复用 calendar 5 工具校验函数
//! - 锚 #6: 5 K-1 强校验
//!
//! **8 项不修改承诺**:
//! - ❌ 不改 LOCKED crate
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方 DB / chrono 之外日期库
//! - ❌ 不假装支持完整 ISO 8601 (datetime / timezone, 留 R21)
//! - ❌ 不假装 task 已支持子任务 / 依赖 (留 R21)
//! - ❌ 不破坏 24 LOCKED crate
//! - ❌ 不假装 complete 是副作用触发器 (仅改 status, 留 R21 接 WebSocket 通知)

#![deny(unsafe_code)]

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use super::storage::{EntityStorage, InMemoryStorage};

/// **K-1-5 assignee 邮箱格式校验** — 简化版 (per RFC 5321 字面字符集)
///
/// 内联在本文件 (不依赖 LOCKED `super::calendar::validate_email`, 守 8 项不修改承诺).
/// 真值 (R21 估补): 完整 RFC 5321 / 5322 + IDN 域名.
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("email must not be empty".to_string());
    }
    if email.len() > 254 {
        return Err(format!("email too long ({} > 254)", email.len()));
    }
    let at_pos = email
        .find('@')
        .ok_or_else(|| format!("email missing '@': {email}"))?;
    if at_pos == 0 {
        return Err(format!("email local part empty: {email}"));
    }
    if at_pos == email.len() - 1 {
        return Err(format!("email domain part empty: {email}"));
    }
    let (local, domain) = email.split_at(at_pos);
    let domain = &domain[1..];
    if local.is_empty() || local.contains('@') {
        return Err(format!("email local part invalid: {email}"));
    }
    if !domain.contains('.') {
        return Err(format!("email domain missing '.': {email}"));
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(format!("email domain has leading/trailing '.': {email}"));
    }
    for c in email.chars() {
        if !c.is_ascii() {
            return Err(format!("email non-ASCII char: {email}"));
        }
    }
    Ok(())
}

// ============================================================
// 实体 + 7 字段 + status 枚举
// ============================================================

/// **TaskStatus** — task 4 状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub const ALL: [TaskStatus; 4] = [
        TaskStatus::Pending,
        TaskStatus::InProgress,
        TaskStatus::Done,
        TaskStatus::Cancelled,
    ];

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "done" => Some(Self::Done),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

/// **Task** — task 7 字段实体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: String, // TaskStatus 字符串表示, 序列化用 snake_case
    pub priority: u8,
    #[serde(default)]
    pub due_date: String, // ISO 8601 date (YYYY-MM-DD), 可选
    #[serde(default)]
    pub assignee: String, // 邮箱, 可选
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub done_at: Option<i64>,
}

// ============================================================
// K-1 强校验 (5 类)
// ============================================================

/// **K-1-2 status 枚举** — 必须在 4 值内
pub fn validate_status(s: &str) -> Result<TaskStatus, String> {
    TaskStatus::from_str(s).ok_or_else(|| {
        format!(
            "K-1 violation: status must be one of pending/in_progress/done/cancelled, got '{s}'"
        )
    })
}

/// **K-1-3 priority 0-3**
pub fn validate_priority(p: i64) -> Result<u8, String> {
    if p < 0 || p > 3 {
        return Err(format!("K-1 violation: priority must be 0-3, got {p}"));
    }
    Ok(p as u8)
}

/// **K-1-4 due_date ISO 8601 date** — YYYY-MM-DD (简化版, 不接 datetime / timezone)
pub fn validate_due_date(d: &str) -> Result<(), String> {
    if d.is_empty() {
        return Ok(()); // due_date 可选, 空视为"未设"
    }
    // 长度必须 10 (YYYY-MM-DD)
    if d.len() != 10 {
        return Err(format!(
            "K-1 violation: due_date must be YYYY-MM-DD (len 10), got '{d}' (len {})",
            d.len()
        ));
    }
    // 位置 4 和 7 必须是 -
    let bytes = d.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(format!(
            "K-1 violation: due_date must be YYYY-MM-DD (positions 4,7 = '-'), got '{d}'"
        ));
    }
    // 其它位置必须数字
    for (i, b) in bytes.iter().enumerate() {
        if i == 4 || i == 7 {
            continue;
        }
        if !b.is_ascii_digit() {
            return Err(format!(
                "K-1 violation: due_date non-digit at position {i}: '{d}'"
            ));
        }
    }
    // 月 01-12, 日 01-31 (简化, 不查闰年)
    let month: u32 = d[5..7].parse().unwrap_or(0);
    let day: u32 = d[8..10].parse().unwrap_or(0);
    if !(1..=12).contains(&month) {
        return Err(format!(
            "K-1 violation: due_date month {month} not 01-12: '{d}'"
        ));
    }
    if !(1..=31).contains(&day) {
        return Err(format!(
            "K-1 violation: due_date day {day} not 01-31: '{d}'"
        ));
    }
    Ok(())
}

// ============================================================
// TaskTool — 6 actions + storage 抽象
// ============================================================

/// **TaskTool** — 6 actions (list/create/update/delete/get/complete)
pub struct TaskTool {
    storage: Arc<dyn EntityStorage<Task>>,
}

impl TaskTool {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(InMemoryStorage::<Task>::new("task")),
        }
    }

    pub fn with_storage(storage: Arc<dyn EntityStorage<Task>>) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &Arc<dyn EntityStorage<Task>> {
        &self.storage
    }

    /// **dispatch** — 6 actions 路由
    async fn dispatch(&self, args: Value) -> Result<Value, String> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: action".to_string())?;
        match action {
            "list" => self.action_list(args).await,
            "create" => self.action_create(args).await,
            "update" => self.action_update(args).await,
            "delete" => self.action_delete(args).await,
            "get" => self.action_get(args).await,
            "complete" => self.action_complete(args).await,
            other => Err(format!("unknown action: {other}")),
        }
    }

    async fn action_list(&self, args: Value) -> Result<Value, String> {
        let filter_status = args
            .get("filter")
            .and_then(|f| f.get("status"))
            .and_then(|v| v.as_str());
        let filter_priority = args
            .get("filter")
            .and_then(|f| f.get("priority"))
            .and_then(|v| v.as_i64());
        let filter_assignee = args
            .get("filter")
            .and_then(|f| f.get("assignee"))
            .and_then(|v| v.as_str());
        let all = self
            .storage
            .list()
            .await
            .map_err(|e| format!("list failed: {e}"))?;
        let tasks: Vec<&Task> = all
            .iter()
            .filter(|t| {
                filter_status.map(|s| t.status == s).unwrap_or(true)
                    && filter_priority
                        .map(|p| i64::from(t.priority) == p)
                        .unwrap_or(true)
                    && filter_assignee.map(|a| t.assignee == a).unwrap_or(true)
            })
            .collect();
        Ok(json!({
            "tasks": tasks,
            "count": tasks.len(),
        }))
    }

    async fn action_create(&self, args: Value) -> Result<Value, String> {
        // 7 字段提取 + 5 K-1 强校验
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: title".to_string())?;
        // K-1-1: title 非空
        if title.is_empty() {
            return Err("K-1 violation: title must not be empty".to_string());
        }
        if title.len() > 512 {
            return Err(format!("title too long ({} > 512)", title.len()));
        }

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let status_str = args
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending");
        // K-1-2: status 枚举
        let _status = validate_status(status_str)?;

        let priority_i = args.get("priority").and_then(|v| v.as_i64()).unwrap_or(1);
        // K-1-3: priority 0-3
        let priority = validate_priority(priority_i)?;

        let due_date = args
            .get("due_date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // K-1-4: due_date ISO 8601
        validate_due_date(&due_date)?;

        let assignee = args
            .get("assignee")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // K-1-5: assignee 邮箱 (非空时)
        if !assignee.is_empty() {
            validate_email(&assignee)?;
        }

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let t = Task {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            description,
            status: status_str.to_string(),
            priority,
            due_date,
            assignee,
            created_at,
            done_at: None,
        };
        let task_id = t.id.clone();
        self.storage
            .upsert(t)
            .await
            .map_err(|e| format!("create failed: {e}"))?;
        Ok(json!({ "task_id": task_id, "ok": true }))
    }

    async fn action_update(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;

        let mut existing = self
            .storage
            .get(id)
            .await
            .map_err(|e| format!("update failed: {e}"))?;

        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            if title.is_empty() {
                return Err("K-1 violation: title must not be empty".to_string());
            }
            existing.title = title.to_string();
        }
        if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
            existing.description = description.to_string();
        }
        if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
            let _ = validate_status(status_str)?;
            existing.status = status_str.to_string();
            // done → 设 done_at; 非 done → 清 done_at
            if status_str == "done" && existing.done_at.is_none() {
                existing.done_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                );
            } else if status_str != "done" {
                existing.done_at = None;
            }
        }
        if let Some(priority_i) = args.get("priority").and_then(|v| v.as_i64()) {
            existing.priority = validate_priority(priority_i)?;
        }
        if let Some(due_date) = args.get("due_date").and_then(|v| v.as_str()) {
            validate_due_date(due_date)?;
            existing.due_date = due_date.to_string();
        }
        if let Some(assignee) = args.get("assignee").and_then(|v| v.as_str()) {
            if !assignee.is_empty() {
                validate_email(assignee)?;
            }
            existing.assignee = assignee.to_string();
        }

        self.storage
            .upsert(existing)
            .await
            .map_err(|e| format!("update failed: {e}"))?;
        Ok(json!({ "ok": true, "updated": id }))
    }

    async fn action_delete(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;
        let removed = self
            .storage
            .delete(id)
            .await
            .map_err(|e| format!("delete failed: {e}"))?;
        if !removed {
            return Err(format!("task not found: {id}"));
        }
        Ok(json!({ "ok": true, "deleted": id }))
    }

    async fn action_get(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;
        let t = self
            .storage
            .get(id)
            .await
            .map_err(|e| format!("get failed: {e}"))?;
        Ok(json!({ "task": t }))
    }

    async fn action_complete(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;
        let mut existing = self
            .storage
            .get(id)
            .await
            .map_err(|e| format!("complete failed: {e}"))?;
        let done_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        existing.status = "done".to_string();
        existing.done_at = Some(done_at);
        self.storage
            .upsert(existing)
            .await
            .map_err(|e| format!("complete failed: {e}"))?;
        Ok(json!({ "ok": true, "id": id, "status": "done", "done_at": done_at }))
    }
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "Task"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default_for_kind(ToolKind::Sync)
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        self.dispatch(args).await
    }
}

pub use super::invoke_by_name as invoke;

// ============================================================
// 编译期 hardcode
// ============================================================

/// **6 actions** (per R20 阶段 4 任务规范)
pub const TASK_ACTIONS: [&str; 6] = ["list", "create", "update", "delete", "get", "complete"];

/// **5 K-1 强校验** (per R20 阶段 4 任务规范)
pub const TASK_K1_CHECKS: [&str; 5] = [
    "title_not_empty",
    "status_enum",
    "priority_0_3",
    "due_date_iso8601",
    "assignee_email",
];

const _: () = {
    assert!(TASK_ACTIONS.len() == 6, "6 actions");
    assert!(TASK_K1_CHECKS.len() == 5, "5 K-1 强校验");
    assert!(TaskStatus::ALL.len() == 4, "4 task status 枚举");
};

// ============================================================
// 单元测试 (6 测试: 6 actions e2e + 5 K-1 校验 + 编译期 + 错误路径)
// ============================================================

#[cfg(test)]
mod task_tests {
    use super::*;
    use serde_json::json;

    /// 1. 6 actions e2e
    #[tokio::test]
    async fn task_6_actions_e2e() {
        let t = TaskTool::new();
        // 1. list empty
        let r = t.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 0);

        // 2. create
        let r = t
            .call(json!({
                "action": "create",
                "title": "Write spec",
                "description": "R20 阶段 4 spec",
                "status": "pending",
                "priority": 2,
                "due_date": "2026-12-31",
                "assignee": "alice@example.com"
            }))
            .await
            .expect("create");
        let tid = r["task_id"].as_str().expect("task_id").to_string();

        // 3. get
        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get");
        assert_eq!(r["task"]["title"], "Write spec");
        assert_eq!(r["task"]["status"], "pending");
        assert_eq!(r["task"]["priority"], 2);

        // 4. update (改 priority 2 → 3)
        let r = t
            .call(json!({
                "action": "update", "id": tid,
                "priority": 3, "status": "in_progress"
            }))
            .await
            .expect("update");
        assert_eq!(r["ok"], true);
        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get after update");
        assert_eq!(r["task"]["priority"], 3);
        assert_eq!(r["task"]["status"], "in_progress");

        // 5. complete
        let r = t
            .call(json!({"action": "complete", "id": tid}))
            .await
            .expect("complete");
        assert_eq!(r["status"], "done");
        assert!(r["done_at"].as_i64().unwrap() > 0);
        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get after complete");
        assert_eq!(r["task"]["status"], "done");
        assert!(r["task"]["done_at"].as_i64().is_some());

        // 6. delete
        let r = t
            .call(json!({"action": "delete", "id": tid}))
            .await
            .expect("delete");
        assert_eq!(r["deleted"], tid);
    }

    /// 2. K-1-1/2/3 强校验: title / status / priority
    #[tokio::test]
    async fn task_k1_title_status_priority() {
        let t = TaskTool::new();
        // title 空
        let r = t.call(json!({"action": "create", "title": ""})).await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("title"));
        // status 不在枚举
        let r = t
            .call(json!({"action": "create", "title": "x", "status": "weird"}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("status"));
        // priority 越界
        let r = t
            .call(json!({"action": "create", "title": "x", "priority": 99}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("priority"));
        // priority 负
        let r = t
            .call(json!({"action": "create", "title": "x", "priority": -1}))
            .await;
        assert!(r.is_err());
    }

    /// 3. K-1-4 强校验: due_date ISO 8601
    #[tokio::test]
    async fn task_k1_due_date_iso8601() {
        let t = TaskTool::new();
        // 缺 -
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "20261231"}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("due_date"));
        // 月超界
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-13-01"}))
            .await;
        assert!(r.is_err());
        // 日超界
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-12-32"}))
            .await;
        assert!(r.is_err());
        // 含 datetime (留 R21)
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-12-31T10:00:00"}))
            .await;
        assert!(r.is_err());
        // 合法通过
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-12-31"}))
            .await
            .expect("create iso date");
        assert!(r["task_id"].is_string());
    }

    /// 4. K-1-5 强校验: assignee 邮箱
    #[tokio::test]
    async fn task_k1_assignee_email() {
        let t = TaskTool::new();
        // 不传 assignee (可选, 默认空)
        let r = t
            .call(json!({"action": "create", "title": "x"}))
            .await
            .expect("create no assignee");
        assert!(r["task_id"].is_string());
        // 空字符串 assignee (K-1 跳过)
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": ""}))
            .await
            .expect("create empty assignee");
        assert!(r["task_id"].is_string());
        // 非空 + 错格式
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": "bad"}))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("email") || err_msg.contains("@"));
        // 合法
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": "a@example.com"}))
            .await
            .expect("create good assignee");
        assert!(r["task_id"].is_string());
    }

    /// 5. list filter + complete 幂等
    #[tokio::test]
    async fn task_list_filter_and_complete_idempotent() {
        let t = TaskTool::new();
        // 创 3 个 task
        for (title, status, prio) in [
            ("a", "pending", 1),
            ("b", "in_progress", 2),
            ("c", "done", 3),
        ] {
            t.call(json!({
                "action": "create", "title": title, "status": status, "priority": prio
            }))
            .await
            .expect("create");
        }
        // filter status=pending
        let r = t
            .call(json!({"action": "list", "filter": {"status": "pending"}}))
            .await
            .expect("list pending");
        assert_eq!(r["count"], 1);
        // filter priority>=2 (用 priority=2 filter)
        let r = t
            .call(json!({"action": "list", "filter": {"priority": 2}}))
            .await
            .expect("list prio 2");
        assert_eq!(r["count"], 1);

        // complete 幂等: 重复调 complete, done_at 不变 (或重新设, 都合法)
        let r = t
            .call(json!({"action": "create", "title": "d"}))
            .await
            .expect("create d");
        let did = r["task_id"].as_str().unwrap().to_string();
        let r1 = t
            .call(json!({"action": "complete", "id": did}))
            .await
            .expect("complete 1");
        let r2 = t
            .call(json!({"action": "complete", "id": did}))
            .await
            .expect("complete 2");
        assert_eq!(r1["status"], "done");
        assert_eq!(r2["status"], "done");
    }

    /// 6. 编译期 hardcode + 错误路径 + K-1 校验函数单测
    #[tokio::test]
    async fn task_constants_errors_and_validators() {
        // 编译期
        assert_eq!(TASK_ACTIONS.len(), 6);
        assert_eq!(TASK_K1_CHECKS.len(), 5);
        assert_eq!(TASK_ACTIONS[0], "list");
        assert_eq!(TASK_ACTIONS[5], "complete");
        assert_eq!(TASK_K1_CHECKS[0], "title_not_empty");
        assert_eq!(TASK_K1_CHECKS[4], "assignee_email");

        // 错误路径: 缺 action / 缺 id (get/update/delete/complete) / 不存在 id
        let t = TaskTool::new();
        assert!(t.call(json!({})).await.is_err());
        assert!(t.call(json!({"action": "get"})).await.is_err());
        assert!(t
            .call(json!({"action": "update", "title": "x"}))
            .await
            .is_err());
        assert!(t
            .call(json!({"action": "delete", "id": "x"}))
            .await
            .is_err());
        assert!(t
            .call(json!({"action": "complete", "id": "x"}))
            .await
            .is_err());

        // K-1 校验函数
        assert!(validate_status("pending").is_ok());
        assert!(validate_status("done").is_ok());
        assert!(validate_status("weird").is_err());
        assert!(validate_priority(0).is_ok());
        assert!(validate_priority(3).is_ok());
        assert!(validate_priority(4).is_err());
        assert!(validate_due_date("").is_ok()); // 空可选
        assert!(validate_due_date("2026-12-31").is_ok());
        assert!(validate_due_date("2026-13-01").is_err());
        assert!(validate_due_date("20261231").is_err());
        assert!(validate_due_date("2026-12-31T10:00:00").is_err());
    }
}
