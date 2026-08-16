//! R80: MCP tools/subscribe — 双向 push 模式 (server → client 推 ToolCallRequest)
//!
//! **目标**: R72 加了 resources/subscribe (server push resource 变化), R80 加 tools/subscribe
//! 双向 push 模式 — server 端主动 push `notifications/tools/list_changed` (server tool list 变化)
//! + 客户端订阅 server 端事件流 (e.g. long-running task 完成通知).
//!
//! **MCP 协议 (per modelcontextprotocol/specification 2025-06-18 §tools/subscribe)**:
//! - `tools/subscribe` — 客户端订阅 server 端 tool 事件 (e.g. long-running task completion)
//! - `notifications/tools/list_changed` — server 端 push 通知 (server tool list 变化)
//! - `notifications/tools/progress` — server 端 push 进度 (per Cursor IDE MCP 实践)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 tools.rs / subscriptions.rs / protocol.rs / ResourceServer / ToolServer (LOCKED)
//! - 0 引入 async (sync 路径; push notification 由 caller 异步分发)
//! - 0 业务耦合

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::protocol::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

pub const TOOL_SUBSCRIBE_INVALID_NAME: i32 = -32030;
pub const TOOL_SUBSCRIBE_NOT_FOUND: i32 = -32031;
pub const TOOL_SUBSCRIBE_ALREADY: i32 = -32032;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolEventKind {
    ListChanged,
    Progress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolEvent {
    pub kind: ToolEventKind,
    pub tool_name: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    pub timestamp_unix_ms: u64,
}

impl ToolEvent {
    pub fn new(
        kind: ToolEventKind,
        tool_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            kind,
            tool_name: tool_name.into(),
            message: message.into(),
            progress: None,
            timestamp_unix_ms: now_ms,
        }
    }
    pub fn with_progress(mut self, pct: u8) -> Self {
        self.progress = Some(pct.min(100));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSubscription {
    pub tool_name: String,
    pub client_id: String,
    pub created_at_unix_ms: u64,
    pub event_filter: Option<HashSet<ToolEventKind>>,
}

impl ToolSubscription {
    pub fn new(tool_name: impl Into<String>, client_id: impl Into<String>) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            tool_name: tool_name.into(),
            client_id: client_id.into(),
            created_at_unix_ms: now_ms,
            event_filter: None,
        }
    }
    pub fn with_filter(mut self, kinds: impl IntoIterator<Item = ToolEventKind>) -> Self {
        self.event_filter = Some(kinds.into_iter().collect());
        self
    }
    pub fn matches(&self, event: &ToolEvent) -> bool {
        if !self.tool_name.is_empty() && self.tool_name != event.tool_name {
            return false;
        }
        if let Some(filter) = &self.event_filter {
            if !filter.contains(&event.kind) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct ToolEventBroker {
    /// tool_name (空字符串 = 全局) → client_id → ToolSubscription
    inner: Mutex<HashMap<String, HashMap<String, ToolSubscription>>>,
}

impl Default for ToolEventBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolEventBroker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self, sub: ToolSubscription) -> Result<(), JsonRpcError> {
        if sub.tool_name.is_empty() && sub.client_id.is_empty() {
            return Err(JsonRpcError::new(
                TOOL_SUBSCRIBE_INVALID_NAME,
                "both tool_name and client_id empty",
            ));
        }
        let mut map = self.inner.lock().expect("tool event broker mutex poisoned");
        let entry = map
            .entry(sub.tool_name.clone())
            .or_insert_with(HashMap::new);
        if entry.contains_key(&sub.client_id) {
            return Err(JsonRpcError::new(
                TOOL_SUBSCRIBE_ALREADY,
                format!(
                    "client `{}` already subscribed to `{}`",
                    sub.client_id, sub.tool_name
                ),
            ));
        }
        entry.insert(sub.client_id.clone(), sub);
        Ok(())
    }

    pub fn unsubscribe(&self, tool_name: &str, client_id: &str) -> Result<(), JsonRpcError> {
        let mut map = self.inner.lock().expect("tool event broker mutex poisoned");
        if let Some(entry) = map.get_mut(tool_name) {
            if entry.remove(client_id).is_some() {
                if entry.is_empty() {
                    map.remove(tool_name);
                }
                return Ok(());
            }
        }
        Err(JsonRpcError::new(
            TOOL_SUBSCRIBE_NOT_FOUND,
            format!("no subscription for client `{client_id}` on `{tool_name}`"),
        ))
    }

    /// 分发事件 — 返所有匹配 subscription 的 (tool_name, client_id) 对
    pub fn dispatch_event(&self, event: &ToolEvent) -> Vec<(String, String)> {
        let map = self.inner.lock().expect("tool event broker mutex poisoned");
        let mut matched = Vec::new();
        if let Some(global) = map.get("") {
            for (client_id, sub) in global {
                if sub.matches(event) {
                    matched.push((String::new(), client_id.clone()));
                }
            }
        }
        if !event.tool_name.is_empty() {
            if let Some(entry) = map.get(&event.tool_name) {
                for (client_id, sub) in entry {
                    if sub.matches(event) {
                        matched.push((event.tool_name.clone(), client_id.clone()));
                    }
                }
            }
        }
        matched
    }

    pub fn uri_count(&self) -> usize {
        let map = self.inner.lock().expect("tool event broker mutex poisoned");
        map.len()
    }
    pub fn total_subscriptions(&self) -> usize {
        let map = self.inner.lock().expect("tool event broker mutex poisoned");
        map.values().map(|s| s.len()).sum()
    }
}

pub fn handle_tools_subscribe(req: &JsonRpcRequest, broker: &ToolEventBroker) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(TOOL_SUBSCRIBE_INVALID_NAME, "params missing"),
        );
    };
    let tool_name = params
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let client_id = match params.get("client_id").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => format!("anon-{}", std::process::id()),
    };
    let mut sub = ToolSubscription::new(tool_name, &client_id);
    if let Some(filter_arr) = params.get("event_filter").and_then(|v| v.as_array()) {
        let kinds: Vec<ToolEventKind> = filter_arr
            .iter()
            .filter_map(|v| match v.as_str()? {
                "list_changed" => Some(ToolEventKind::ListChanged),
                "progress" => Some(ToolEventKind::Progress),
                "completed" => Some(ToolEventKind::Completed),
                "failed" => Some(ToolEventKind::Failed),
                _ => None,
            })
            .collect();
        if !kinds.is_empty() {
            sub = sub.with_filter(kinds);
        }
    }
    let resp_id = req.id.clone();
    match broker.subscribe(sub) {
        Ok(()) => JsonRpcResponse::ok(
            resp_id,
            json!({
                "subscribed": true,
                "tool_name": tool_name,
                "client_id": client_id,
            }),
        ),
        Err(e) => JsonRpcResponse::err(resp_id, JsonRpcError::new(e.code, e.message)),
    }
}

pub fn handle_tools_unsubscribe(req: &JsonRpcRequest, broker: &ToolEventBroker) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(TOOL_SUBSCRIBE_INVALID_NAME, "params missing"),
        );
    };
    let tool_name = params
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let client_id = match params.get("client_id").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => format!("anon-{}", std::process::id()),
    };
    let resp_id = req.id.clone();
    match broker.unsubscribe(tool_name, &client_id) {
        Ok(()) => JsonRpcResponse::ok(
            resp_id,
            json!({
                "unsubscribed": true,
                "tool_name": tool_name,
                "client_id": client_id,
            }),
        ),
        Err(e) => JsonRpcResponse::err(resp_id, JsonRpcError::new(e.code, e.message)),
    }
}

pub fn build_tool_list_changed_notification() -> JsonRpcRequest {
    JsonRpcRequest::notification("notifications/tools/list_changed", Some(json!({})))
}

pub fn build_tool_progress_notification(
    tool_name: &str,
    progress: u8,
    message: &str,
) -> JsonRpcRequest {
    JsonRpcRequest::notification(
        "notifications/tools/progress",
        Some(json!({
            "tool_name": tool_name,
            "progress": progress.min(100),
            "message": message,
        })),
    )
}

pub fn build_tool_completed_notification(tool_name: &str, result_summary: &str) -> JsonRpcRequest {
    JsonRpcRequest::notification(
        "notifications/tools/completed",
        Some(json!({
            "tool_name": tool_name,
            "result_summary": result_summary,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_event_kind_serialize_round_trip() {
        for kind in [
            ToolEventKind::ListChanged,
            ToolEventKind::Progress,
            ToolEventKind::Completed,
            ToolEventKind::Failed,
        ] {
            let j = serde_json::to_string(&kind).unwrap();
            let r: ToolEventKind = serde_json::from_str(&j).unwrap();
            assert_eq!(r, kind);
        }
    }

    #[test]
    fn tool_event_new_basic() {
        let e = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        assert_eq!(e.kind, ToolEventKind::Progress);
        assert_eq!(e.tool_name, "long-task");
        assert!(e.timestamp_unix_ms > 0);
        assert!(e.progress.is_none());
    }

    #[test]
    fn tool_event_with_progress_clamped() {
        let e = ToolEvent::new(ToolEventKind::Progress, "x", "y").with_progress(150);
        assert_eq!(e.progress, Some(100), "must clamp to 100");
    }

    #[test]
    fn tool_subscription_matches_specific_tool() {
        let s = ToolSubscription::new("long-task", "c1");
        let e1 = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        let e2 = ToolEvent::new(ToolEventKind::Progress, "other-task", "running");
        assert!(s.matches(&e1));
        assert!(!s.matches(&e2));
    }

    #[test]
    fn tool_subscription_matches_global() {
        let s = ToolSubscription::new("", "c1");
        let e1 = ToolEvent::new(ToolEventKind::Progress, "any-task", "running");
        assert!(s.matches(&e1));
    }

    #[test]
    fn tool_subscription_with_filter() {
        let s = ToolSubscription::new("long-task", "c1")
            .with_filter([ToolEventKind::Progress, ToolEventKind::Completed]);
        let e_progress = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        let e_completed = ToolEvent::new(ToolEventKind::Completed, "long-task", "done");
        let e_failed = ToolEvent::new(ToolEventKind::Failed, "long-task", "error");
        assert!(s.matches(&e_progress));
        assert!(s.matches(&e_completed));
        assert!(!s.matches(&e_failed));
    }

    #[test]
    fn broker_new_empty() {
        let b = ToolEventBroker::new();
        assert_eq!(b.uri_count(), 0);
        assert_eq!(b.total_subscriptions(), 0);
    }

    #[test]
    fn broker_subscribe_single() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        assert_eq!(b.uri_count(), 1);
        assert_eq!(b.total_subscriptions(), 1);
    }

    #[test]
    fn broker_subscribe_duplicate_rejected() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        let err = b
            .subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap_err();
        assert_eq!(err.code, TOOL_SUBSCRIBE_ALREADY);
    }

    #[test]
    fn broker_subscribe_empty_both_rejected() {
        let b = ToolEventBroker::new();
        let err = b.subscribe(ToolSubscription::new("", "")).unwrap_err();
        assert_eq!(err.code, TOOL_SUBSCRIBE_INVALID_NAME);
    }

    #[test]
    fn broker_unsubscribe_basic() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        b.unsubscribe("long-task", "c1").unwrap();
        assert_eq!(b.uri_count(), 0);
    }

    #[test]
    fn broker_unsubscribe_unknown_returns_error() {
        let b = ToolEventBroker::new();
        let err = b.unsubscribe("nope", "c1").unwrap_err();
        assert_eq!(err.code, TOOL_SUBSCRIBE_NOT_FOUND);
    }

    #[test]
    fn broker_dispatch_event_global_match() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("", "c1")).unwrap();
        let event = ToolEvent::new(ToolEventKind::Progress, "any-task", "running");
        let matched = b.dispatch_event(&event);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].1, "c1");
    }

    #[test]
    fn broker_dispatch_event_specific_match() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        let event = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        let matched = b.dispatch_event(&event);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn broker_dispatch_event_no_match() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        let event = ToolEvent::new(ToolEventKind::Progress, "other-task", "running");
        let matched = b.dispatch_event(&event);
        assert!(matched.is_empty());
    }

    #[test]
    fn broker_dispatch_event_with_filter() {
        let b = ToolEventBroker::new();
        b.subscribe(
            ToolSubscription::new("long-task", "c1").with_filter([ToolEventKind::Completed]),
        )
        .unwrap();
        let e_progress = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        let e_completed = ToolEvent::new(ToolEventKind::Completed, "long-task", "done");
        assert!(b.dispatch_event(&e_progress).is_empty());
        assert_eq!(b.dispatch_event(&e_completed).len(), 1);
    }

    #[test]
    fn handle_subscribe_basic() {
        let b = ToolEventBroker::new();
        let req = JsonRpcRequest::new(
            "tools/subscribe",
            Some(json!({ "tool_name": "long-task", "client_id": "c1" })),
            Id::Num(1),
        );
        let resp = handle_tools_subscribe(&req, &b);
        assert!(resp.error.is_none());
        assert_eq!(b.uri_count(), 1);
    }

    #[test]
    fn handle_subscribe_with_filter() {
        let b = ToolEventBroker::new();
        let req = JsonRpcRequest::new(
            "tools/subscribe",
            Some(json!({
                "tool_name": "long-task",
                "client_id": "c1",
                "event_filter": ["progress", "completed"],
            })),
            Id::Num(1),
        );
        let resp = handle_tools_subscribe(&req, &b);
        assert!(resp.error.is_none());
        assert_eq!(b.uri_count(), 1);
    }

    #[test]
    fn handle_subscribe_missing_params_returns_error() {
        let b = ToolEventBroker::new();
        let req = JsonRpcRequest::new("tools/subscribe", None, Id::Num(1));
        let resp = handle_tools_subscribe(&req, &b);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, TOOL_SUBSCRIBE_INVALID_NAME);
    }

    #[test]
    fn handle_unsubscribe_basic() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        let req = JsonRpcRequest::new(
            "tools/unsubscribe",
            Some(json!({ "tool_name": "long-task", "client_id": "c1" })),
            Id::Num(2),
        );
        let resp = handle_tools_unsubscribe(&req, &b);
        assert!(resp.error.is_none());
    }

    #[test]
    fn build_list_changed_notification_basic() {
        let n = build_tool_list_changed_notification();
        assert_eq!(n.method, "notifications/tools/list_changed");
        assert!(n.id.is_none());
    }

    #[test]
    fn build_progress_notification_basic() {
        let n = build_tool_progress_notification("long-task", 50, "halfway");
        assert_eq!(n.method, "notifications/tools/progress");
        assert!(n.id.is_none());
        let p = n.params.expect("params");
        assert_eq!(p.get("progress").and_then(|v| v.as_u64()), Some(50));
        assert_eq!(
            p.get("tool_name").and_then(|v| v.as_str()),
            Some("long-task")
        );
        assert_eq!(p.get("message").and_then(|v| v.as_str()), Some("halfway"));
    }

    #[test]
    fn build_progress_notification_clamps_to_100() {
        let n = build_tool_progress_notification("x", 200, "msg");
        let p = n.params.expect("params");
        assert_eq!(p.get("progress").and_then(|v| v.as_u64()), Some(100));
    }

    #[test]
    fn build_completed_notification_basic() {
        let n = build_tool_completed_notification("long-task", "result: 42");
        assert_eq!(n.method, "notifications/tools/completed");
        assert!(n.id.is_none());
        let p = n.params.expect("params");
        assert_eq!(
            p.get("tool_name").and_then(|v| v.as_str()),
            Some("long-task")
        );
        assert_eq!(
            p.get("result_summary").and_then(|v| v.as_str()),
            Some("result: 42")
        );
    }
}
