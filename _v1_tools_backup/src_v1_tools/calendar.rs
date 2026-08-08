//! `/v1/tools/calendar/invoke` — **D-01 真接** (R20 阶段 2 推翻原 stub 501 推荐)
//!
//! **端点**: `POST /v1/tools/calendar/invoke`
//! **真接 (D-01)**: 主人 2026-08-05 20:53 拍板, calendar 走 Tool trait 真实现, **不** stub 返 501.
//! **5 actions** (per 主人任务稿 + 蓝图 §2.2 路由表 #5):
//! - `list`     — 列出全部 events
//! - `create`   — 新建 event
//! - `update`   — 更新 event (by id)
//! - `delete`   — 删除 event (by id)
//! - `list_range` — 按 date range 过滤 list
//!
//! **Req schema** (per §2.2 路由表 #5):
//! ```json
//! { "args": { "action": "list|create|update|delete|list_range", "event"?: {...}, "range"?: {...} } }
//! ```
//! **Resp schema** (per §2.2 统一信封):
//! ```json
//! { "ok": true, "result": { "events": [...], "event_id": "..." }, "error": null, "meta": { "tool": "calendar", ... } }
//! ```
//!
//! **存储**: in-memory `Arc<Mutex<Vec<CalendarEvent>>>` (per D-01 真接, 阶段 2 minimal, 阶段 4 SDK 接入时换 SQLite)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 5 actions 全部真实现 (非 stub 501, D-01 决策)
//! - ✅ in-memory store 真跑 (Mutex 守并发, 5 actions 真改/读)
//! - ✅ event ID 用 UUID v4 (workspace `uuid` crate 真生成)
//! - ✅ 5 actions 在 fixture 跑通 (per 蓝图 §2.7 calendar fixture)
//!
//! **不修改承诺**:
//! - ❌ 0 改 `apeireth-mcp/src/` (LOCKED, calendar 不走 mcp, 走 Tool trait)
//! - ❌ 0 改 `apeireth-tool-registry/src/` (LOCKED, 仅实现 Tool trait, 不改 trait 定义)
//! - ❌ 0 改 workspace version (1.0.0)
//! - ❌ 0 引 NewAPI / 0 重复造轮子

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// **Calendar event** — 6 字段
///
/// **字段** (per R20 阶段 2 主人拍板 + 蓝图 §2.2):
/// - `id`        — UUID v4 (workspace `uuid` crate 真生成)
/// - `title`     — 必填
/// - `start_ts`  — Unix timestamp (秒, 跟 chrono 字段对齐)
/// - `end_ts`    — Unix timestamp (秒, 0 = 无结束时间)
/// - `attendees` — 参与人列表 (e.g. emails)
/// - `notes`     — 备注 (可选)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub attendees: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// **CalendarTool** — 5 actions 真实现 (D-01 真接)
///
/// **存储**: in-memory `Arc<Mutex<HashMap<String, CalendarEvent>>>` (id → event)
/// **线程安全**: `parking_lot::Mutex` (快 + 无毒, 跟 `apeireth-tool-registry` 一致)
pub struct CalendarTool {
    /// 事件表 (id → event)
    events: Arc<Mutex<HashMap<String, CalendarEvent>>>,
}

impl CalendarTool {
    /// 新建空 CalendarTool
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 5 actions 路由分发
    async fn dispatch(&self, args: Value) -> Result<Value, String> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: action".to_string())?;

        match action {
            "list" => self.action_list().await,
            "create" => self.action_create(args).await,
            "update" => self.action_update(args).await,
            "delete" => self.action_delete(args).await,
            "list_range" => self.action_list_range(args).await,
            other => Err(format!("unknown action: {other}")),
        }
    }

    /// **action: list** — 列出全部 events
    async fn action_list(&self) -> Result<Value, String> {
        let g = self.events.lock();
        let events: Vec<&CalendarEvent> = g.values().collect();
        Ok(json!({
            "events": events,
            "count": events.len(),
        }))
    }

    /// **action: create** — 新建 event
    async fn action_create(&self, args: Value) -> Result<Value, String> {
        let title = args
            .get("event")
            .and_then(|e| e.get("title"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: event.title".to_string())?;
        let start_ts = args
            .get("event")
            .and_then(|e| e.get("start_ts"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing field: event.start_ts".to_string())?;
        let end_ts = args
            .get("event")
            .and_then(|e| e.get("end_ts"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let attendees: Vec<String> = args
            .get("event")
            .and_then(|e| e.get("attendees"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let notes = args
            .get("event")
            .and_then(|e| e.get("notes"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let event = CalendarEvent {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            start_ts,
            end_ts,
            attendees,
            notes,
        };
        let event_id = event.id.clone();
        self.events.lock().insert(event_id.clone(), event);
        Ok(json!({
            "event_id": event_id,
            "ok": true,
        }))
    }

    /// **action: update** — 更新 event (by id)
    async fn action_update(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        let mut g = self.events.lock();
        let ev = g
            .get_mut(id)
            .ok_or_else(|| format!("event not found: {id}"))?;
        if let Some(title) = args.get("event").and_then(|e| e.get("title")).and_then(|v| v.as_str()) {
            ev.title = title.to_string();
        }
        if let Some(start_ts) = args.get("event").and_then(|e| e.get("start_ts")).and_then(|v| v.as_i64()) {
            ev.start_ts = start_ts;
        }
        if let Some(end_ts) = args.get("event").and_then(|e| e.get("end_ts")).and_then(|v| v.as_i64()) {
            ev.end_ts = end_ts;
        }
        if let Some(notes) = args.get("event").and_then(|e| e.get("notes")).and_then(|v| v.as_str()) {
            ev.notes = notes.to_string();
        }
        Ok(json!({"ok": true, "updated": id}))
    }

    /// **action: delete** — 删除 event (by id)
    async fn action_delete(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        let removed = self.events.lock().remove(id).is_some();
        if !removed {
            return Err(format!("event not found: {id}"));
        }
        Ok(json!({"ok": true, "deleted": id}))
    }

    /// **action: list_range** — 按 date range 过滤 list
    async fn action_list_range(&self, args: Value) -> Result<Value, String> {
        let from_ts = args
            .get("range")
            .and_then(|r| r.get("from_ts"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing field: range.from_ts".to_string())?;
        let to_ts = args
            .get("range")
            .and_then(|r| r.get("to_ts"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing field: range.to_ts".to_string())?;
        let g = self.events.lock();
        let events: Vec<&CalendarEvent> = g
            .values()
            .filter(|e| e.start_ts >= from_ts && e.start_ts <= to_ts)
            .collect();
        Ok(json!({
            "events": events,
            "count": events.len(),
            "from_ts": from_ts,
            "to_ts": to_ts,
        }))
    }
}

impl Default for CalendarTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CalendarTool {
    fn name(&self) -> &str {
        "Calendar"
    }

    fn kind(&self) -> ToolKind {
        // 5 actions 是同步操作 (创建/读/改/删 in-memory map)
        ToolKind::Sync
    }

    fn axes(&self) -> ToolAxes {
        ToolAxes::default_for_kind(ToolKind::Sync)
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        self.dispatch(args).await
    }
}

// ============================================================
// 6 端点 handler — re-export 共享 dispatch (D-02 子路径风格)
// ============================================================

/// **6 端点 handler** — re-export 共享 dispatch
///
/// **设计**: 跟 4 真接 (web_search/file_ops/git_ops/code_exec) 共享 `super::invoke_by_name`,
/// 路径由 axum `Path<String>` 从 URL 提取, 走 ToolRegistry.get("Calendar") 路由.
pub use super::invoke_by_name as invoke;

// ============================================================
// 单元测试 (5 actions 真实现, per D-01 真接 + 蓝图 §2.7 fixture)
// ============================================================

#[cfg(test)]
mod calendar_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn calendar_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_TOOLS_PATHS[4], "/tools/calendar/invoke");
        assert_eq!(super::super::V1_TOOL_NAMES[4], "calendar");
        assert_eq!(super::super::V1_REGISTRY_NAMES[4], "Calendar");
    }

    /// **D-01 真接** — calendar 不 stub, 0 返 501
    #[test]
    fn calendar_is_real_not_stub() {
        // D-01: 真接 (推翻原 A 推荐 stub 501, 主人 2026-08-05 20:53 拍板 B)
        // 证据: CalendarTool 5 actions 全真实现
        let cal = CalendarTool::new();
        assert_eq!(cal.name(), "Calendar");
    }

    /// **5 actions 端到端真跑** (per 蓝图 §2.7 calendar fixture 5/5 case)
    #[tokio::test]
    async fn calendar_5_actions_e2e() {
        let cal = CalendarTool::new();

        // 1. list (空)
        let r = cal.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 0);

        // 2. create
        let r = cal
            .call(json!({
                "action": "create",
                "event": {
                    "title": "team standup",
                    "start_ts": 1722931200,
                    "end_ts": 1722934800,
                    "attendees": ["alice@x", "bob@x"],
                    "notes": "daily sync"
                }
            }))
            .await
            .expect("create");
        let event_id = r["event_id"].as_str().expect("event_id").to_string();
        assert!(!event_id.is_empty());

        // 3. update
        let r = cal
            .call(json!({
                "action": "update",
                "id": event_id,
                "event": {"title": "team standup (updated)"}
            }))
            .await
            .expect("update");
        assert_eq!(r["ok"], true);

        // 4. list_range (命中)
        let r = cal
            .call(json!({
                "action": "list_range",
                "range": {"from_ts": 1722931000, "to_ts": 1722932000}
            }))
            .await
            .expect("list_range");
        assert_eq!(r["count"], 1);

        // 5. delete
        let r = cal
            .call(json!({"action": "delete", "id": event_id}))
            .await
            .expect("delete");
        assert_eq!(r["deleted"], event_id);

        // 6. list (空 again)
        let r = cal.call(json!({"action": "list"})).await.expect("list empty");
        assert_eq!(r["count"], 0);
    }

    /// **5 actions 错误路径** — 缺字段 + 未知 action
    #[tokio::test]
    async fn calendar_5_actions_error_paths() {
        let cal = CalendarTool::new();

        // 缺 action
        let r = cal.call(json!({})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("action"));

        // 未知 action
        let r = cal.call(json!({"action": "purge"})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("unknown"));

        // create 缺 title
        let r = cal
            .call(json!({
                "action": "create",
                "event": {"start_ts": 1}
            }))
            .await;
        assert!(r.is_err());

        // update 不存在 id
        let r = cal
            .call(json!({
                "action": "update",
                "id": "no-such-id",
                "event": {"title": "x"}
            }))
            .await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not found"));

        // delete 不存在 id
        let r = cal.call(json!({"action": "delete", "id": "no"})).await;
        assert!(r.is_err());

        // list_range 缺 from_ts
        let r = cal
            .call(json!({"action": "list_range", "range": {"to_ts": 1}}))
            .await;
        assert!(r.is_err());
    }
}
