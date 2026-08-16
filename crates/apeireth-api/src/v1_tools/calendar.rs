//! `/v1/tools/calendar/invoke` — **D-01 真接** (5 actions, in-memory)
//!
//! **D-01 真接**: 主人 2026-08-05 20:53 拍板, calendar 走 Tool trait 真实现, **不** stub 501.
//! **5 actions**: list / create / update / delete / list_range
//! **存储**: in-memory `Arc<Mutex<HashMap<String, CalendarEvent>>>` (UUID v4 event_id)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 5 actions 全部真实现 (非 stub 501, D-01 决策)
//! - ✅ in-memory store 真跑 (Mutex 守并发, 5 actions 真改/读)
//! - ✅ event ID 用 UUID v4 (workspace `uuid` crate 真生成)
//!
//! **不修改承诺**:
//! - ❌ 0 改 `apeireth-mcp/src/` (LOCKED, calendar 不走 mcp, 走 Tool trait)
//! - ❌ 0 改 `apeireth-tool-registry/src/` (LOCKED, 仅实现 Tool trait, 不改 trait 定义)
//! - ❌ 0 改 workspace version (1.0.0)

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

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

pub struct CalendarTool {
    events: Arc<Mutex<HashMap<String, CalendarEvent>>>,
}

impl CalendarTool {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

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

    async fn action_list(&self) -> Result<Value, String> {
        let g = self.events.lock();
        let events: Vec<&CalendarEvent> = g.values().collect();
        Ok(json!({ "events": events, "count": events.len() }))
    }

    async fn action_create(&self, args: Value) -> Result<Value, String> {
        let event = args
            .get("event")
            .ok_or_else(|| "missing field: event".to_string())?;
        let title = event
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: event.title".to_string())?;
        let start_ts = event
            .get("start_ts")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing field: event.start_ts".to_string())?;
        let end_ts = event.get("end_ts").and_then(|v| v.as_i64()).unwrap_or(0);
        let attendees: Vec<String> = event
            .get("attendees")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let notes = event
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let ev = CalendarEvent {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            start_ts,
            end_ts,
            attendees,
            notes,
        };
        let event_id = ev.id.clone();
        self.events.lock().insert(event_id.clone(), ev);
        Ok(json!({ "event_id": event_id, "ok": true }))
    }

    async fn action_update(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        let mut g = self.events.lock();
        let ev = g
            .get_mut(id)
            .ok_or_else(|| format!("event not found: {id}"))?;
        if let Some(title) = args
            .get("event")
            .and_then(|e| e.get("title"))
            .and_then(|v| v.as_str())
        {
            ev.title = title.to_string();
        }
        if let Some(start_ts) = args
            .get("event")
            .and_then(|e| e.get("start_ts"))
            .and_then(|v| v.as_i64())
        {
            ev.start_ts = start_ts;
        }
        if let Some(end_ts) = args
            .get("event")
            .and_then(|e| e.get("end_ts"))
            .and_then(|v| v.as_i64())
        {
            ev.end_ts = end_ts;
        }
        if let Some(notes) = args
            .get("event")
            .and_then(|e| e.get("notes"))
            .and_then(|v| v.as_str())
        {
            ev.notes = notes.to_string();
        }
        Ok(json!({ "ok": true, "updated": id }))
    }

    async fn action_delete(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        let removed = self.events.lock().remove(id).is_some();
        if !removed {
            return Err(format!("event not found: {id}"));
        }
        Ok(json!({ "ok": true, "deleted": id }))
    }

    async fn action_list_range(&self, args: Value) -> Result<Value, String> {
        let range = args
            .get("range")
            .ok_or_else(|| "missing field: range".to_string())?;
        let from_ts = range
            .get("from_ts")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing field: range.from_ts".to_string())?;
        let to_ts = range
            .get("to_ts")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing field: range.to_ts".to_string())?;
        let g = self.events.lock();
        let events: Vec<&CalendarEvent> = g
            .values()
            .filter(|e| e.start_ts >= from_ts && e.start_ts <= to_ts)
            .collect();
        Ok(json!({ "events": events, "count": events.len(), "from_ts": from_ts, "to_ts": to_ts }))
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

#[cfg(test)]
mod calendar_tests {
    use super::*;

    #[test]
    fn calendar_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_REGISTRY_NAMES[4], "Calendar");
    }

    /// D-01 真接 — 5 actions 端到端
    #[tokio::test]
    async fn calendar_5_actions_e2e() {
        let cal = CalendarTool::new();
        // 1. list empty
        let r = cal.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 0);
        // 2. create
        let r = cal.call(json!({
            "action": "create",
            "event": {"title": "standup", "start_ts": 1722931200, "end_ts": 1722934800, "attendees": ["a@x"]}
        })).await.expect("create");
        let event_id = r["event_id"].as_str().expect("event_id").to_string();
        // 3. update
        let r = cal
            .call(json!({
                "action": "update", "id": event_id,
                "event": {"title": "standup (updated)"}
            }))
            .await
            .expect("update");
        assert_eq!(r["ok"], true);
        // 4. list_range
        let r = cal
            .call(json!({
                "action": "list_range", "range": {"from_ts": 1722931000, "to_ts": 1722932000}
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
    }
}
