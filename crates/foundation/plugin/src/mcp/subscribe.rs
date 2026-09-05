//! In-memory MCP subscription brokers.
//!
//! Donor: `legacy/donor/apeireth-mcp/src/{subscriptions,tool_subscriptions}.rs`.
//!
//! Sync, lock-protected maps. Push delivery is the caller's job — this
//! module only tracks who subscribed and builds notification envelopes.
//! Not a second event bus: no runtime wiring, no I/O.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::mcp::jsonrpc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// Empty URI / client id.
///
/// Donor `subscriptions.rs` used −32020. That collides with
/// [`crate::mcp::prompt::PROMPT_NOT_FOUND`]. Kept as-is for donor fidelity;
/// codes are namespaced by method, not globally unique in this library.
pub const SUBSCRIBE_INVALID_URI: i32 = -32020;
/// No such subscription.
pub const SUBSCRIBE_NOT_FOUND: i32 = -32021;
/// Duplicate subscribe.
pub const SUBSCRIBE_ALREADY_SUBSCRIBED: i32 = -32022;

/// Empty tool name + client id.
pub const TOOL_SUBSCRIBE_INVALID_NAME: i32 = -32030;
/// No such tool subscription.
pub const TOOL_SUBSCRIBE_NOT_FOUND: i32 = -32031;
/// Duplicate tool subscribe.
pub const TOOL_SUBSCRIBE_ALREADY: i32 = -32032;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// One resource subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub uri: String,
    pub client_id: String,
    pub created_at_unix_ms: u64,
}

impl Subscription {
    pub fn new(uri: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            client_id: client_id.into(),
            created_at_unix_ms: now_ms(),
        }
    }
}

/// URI → set of client ids.
#[derive(Debug)]
pub struct SubscriptionManager {
    inner: Mutex<HashMap<String, HashSet<String>>>,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, HashSet<String>>> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn subscribe(&self, uri: &str, client_id: &str) -> Result<(), JsonRpcError> {
        if uri.is_empty() {
            return Err(JsonRpcError::new(SUBSCRIBE_INVALID_URI, "uri empty"));
        }
        if client_id.is_empty() {
            return Err(JsonRpcError::new(SUBSCRIBE_INVALID_URI, "client_id empty"));
        }
        let mut map = self.lock();
        let entry = map.entry(uri.to_string()).or_default();
        if entry.contains(client_id) {
            return Err(JsonRpcError::new(
                SUBSCRIBE_ALREADY_SUBSCRIBED,
                format!("client `{client_id}` already subscribed to `{uri}`"),
            ));
        }
        entry.insert(client_id.to_string());
        Ok(())
    }

    pub fn unsubscribe(&self, uri: &str, client_id: &str) -> Result<(), JsonRpcError> {
        let mut map = self.lock();
        if let Some(entry) = map.get_mut(uri) {
            if entry.remove(client_id) {
                if entry.is_empty() {
                    map.remove(uri);
                }
                return Ok(());
            }
        }
        Err(JsonRpcError::new(
            SUBSCRIBE_NOT_FOUND,
            format!("no subscription for client `{client_id}` on `{uri}`"),
        ))
    }

    pub fn subscribers(&self, uri: &str) -> Vec<String> {
        let map = self.lock();
        map.get(uri)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Drop every subscription for `client_id`. Returns how many URIs were hit.
    pub fn unsubscribe_client(&self, client_id: &str) -> usize {
        let mut map = self.lock();
        let mut removed = 0;
        let uris: Vec<String> = map.keys().cloned().collect();
        for uri in uris {
            if let Some(entry) = map.get_mut(&uri) {
                if entry.remove(client_id) {
                    removed += 1;
                    if entry.is_empty() {
                        map.remove(&uri);
                    }
                }
            }
        }
        removed
    }

    pub fn uri_count(&self) -> usize {
        self.lock().len()
    }

    pub fn subscription_count(&self) -> usize {
        self.lock().values().map(|s| s.len()).sum()
    }

    pub fn uris(&self) -> Vec<String> {
        let map = self.lock();
        let mut keys: Vec<String> = map.keys().cloned().collect();
        keys.sort();
        keys
    }
}

fn client_id_from_params(params: &serde_json::Value) -> String {
    params
        .get("client_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "anon".to_string())
}

/// Handle `resources/subscribe`.
pub fn handle_resources_subscribe(
    req: &JsonRpcRequest,
    mgr: &SubscriptionManager,
) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(SUBSCRIBE_INVALID_URI, "params missing"),
        );
    };
    let Some(uri) = params.get("uri").and_then(|v| v.as_str()) else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(SUBSCRIBE_INVALID_URI, "params.uri missing or not string"),
        );
    };
    let client_id = client_id_from_params(params);
    match mgr.subscribe(uri, &client_id) {
        Ok(()) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({ "subscribed": true, "uri": uri, "client_id": client_id }),
        ),
        Err(e) => JsonRpcResponse::err(req.id.clone(), e),
    }
}

/// Handle `resources/unsubscribe`.
pub fn handle_resources_unsubscribe(
    req: &JsonRpcRequest,
    mgr: &SubscriptionManager,
) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(SUBSCRIBE_INVALID_URI, "params missing"),
        );
    };
    let Some(uri) = params.get("uri").and_then(|v| v.as_str()) else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(SUBSCRIBE_INVALID_URI, "params.uri missing or not string"),
        );
    };
    let client_id = client_id_from_params(params);
    match mgr.unsubscribe(uri, &client_id) {
        Ok(()) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({ "unsubscribed": true, "uri": uri, "client_id": client_id }),
        ),
        Err(e) => JsonRpcResponse::err(req.id.clone(), e),
    }
}

/// `notifications/resources/updated` (no id).
pub fn build_resource_updated_notification(uri: &str) -> JsonRpcRequest {
    JsonRpcRequest::notification(
        "notifications/resources/updated",
        Some(json!({ "uri": uri })),
    )
}

/// Tool event kinds a client can filter on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolEventKind {
    ListChanged,
    Progress,
    Completed,
    Failed,
}

/// One tool event.
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
        Self {
            kind,
            tool_name: tool_name.into(),
            message: message.into(),
            progress: None,
            timestamp_unix_ms: now_ms(),
        }
    }

    pub fn with_progress(mut self, pct: u8) -> Self {
        self.progress = Some(pct.min(100));
        self
    }
}

/// One tool subscription. Empty `tool_name` means global.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSubscription {
    pub tool_name: String,
    pub client_id: String,
    pub created_at_unix_ms: u64,
    pub event_filter: Option<HashSet<ToolEventKind>>,
}

impl ToolSubscription {
    pub fn new(tool_name: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            client_id: client_id.into(),
            created_at_unix_ms: now_ms(),
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

/// tool_name (empty = global) → client_id → subscription.
#[derive(Debug)]
pub struct ToolEventBroker {
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

    fn lock(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<String, HashMap<String, ToolSubscription>>> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn subscribe(&self, sub: ToolSubscription) -> Result<(), JsonRpcError> {
        if sub.tool_name.is_empty() && sub.client_id.is_empty() {
            return Err(JsonRpcError::new(
                TOOL_SUBSCRIBE_INVALID_NAME,
                "both tool_name and client_id empty",
            ));
        }
        let mut map = self.lock();
        let entry = map.entry(sub.tool_name.clone()).or_default();
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
        let mut map = self.lock();
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

    /// Matching `(tool_name, client_id)` pairs. Global (`""`) is consulted first.
    pub fn dispatch_event(&self, event: &ToolEvent) -> Vec<(String, String)> {
        let map = self.lock();
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
        self.lock().len()
    }

    pub fn total_subscriptions(&self) -> usize {
        self.lock().values().map(|s| s.len()).sum()
    }
}

/// Handle `tools/subscribe`.
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
    let client_id = client_id_from_params(params);
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
        Err(e) => JsonRpcResponse::err(resp_id, e),
    }
}

/// Handle `tools/unsubscribe`.
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
    let client_id = client_id_from_params(params);
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
        Err(e) => JsonRpcResponse::err(resp_id, e),
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
    use crate::mcp::jsonrpc::Id;

    #[test]
    fn subscription_new_basic() {
        let s = Subscription::new("file:///x.rs", "client-1");
        assert_eq!(s.uri, "file:///x.rs");
        assert_eq!(s.client_id, "client-1");
        assert!(s.created_at_unix_ms > 0);
    }

    #[test]
    fn manager_new_empty() {
        let m = SubscriptionManager::new();
        assert_eq!(m.uri_count(), 0);
        assert_eq!(m.subscription_count(), 0);
        assert!(m.uris().is_empty());
    }

    #[test]
    fn subscribe_single_uri_single_client() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        assert_eq!(m.uri_count(), 1);
        assert_eq!(m.subscription_count(), 1);
        assert_eq!(m.subscribers("file:///a.rs"), vec!["c1".to_string()]);
    }

    #[test]
    fn subscribe_single_uri_multiple_clients() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        m.subscribe("file:///a.rs", "c2").unwrap();
        assert_eq!(m.uri_count(), 1);
        assert_eq!(m.subscription_count(), 2);
        let mut subs = m.subscribers("file:///a.rs");
        subs.sort();
        assert_eq!(subs, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[test]
    fn subscribe_duplicate_rejected() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        let err = m.subscribe("file:///a.rs", "c1").unwrap_err();
        assert_eq!(err.code, SUBSCRIBE_ALREADY_SUBSCRIBED);
    }

    #[test]
    fn subscribe_invalid_uri_or_client_rejected() {
        let m = SubscriptionManager::new();
        assert_eq!(
            m.subscribe("", "c1").unwrap_err().code,
            SUBSCRIBE_INVALID_URI
        );
        assert_eq!(
            m.subscribe("uri", "").unwrap_err().code,
            SUBSCRIBE_INVALID_URI
        );
    }

    #[test]
    fn unsubscribe_removes_entry() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        m.unsubscribe("file:///a.rs", "c1").unwrap();
        assert_eq!(m.uri_count(), 0);
        assert_eq!(m.subscription_count(), 0);
    }

    #[test]
    fn unsubscribe_keeps_other_clients() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        m.subscribe("file:///a.rs", "c2").unwrap();
        m.unsubscribe("file:///a.rs", "c1").unwrap();
        assert_eq!(m.uri_count(), 1);
        assert_eq!(m.subscribers("file:///a.rs"), vec!["c2".to_string()]);
    }

    #[test]
    fn unsubscribe_unknown_returns_error() {
        let m = SubscriptionManager::new();
        assert_eq!(
            m.unsubscribe("file:///a.rs", "c1").unwrap_err().code,
            SUBSCRIBE_NOT_FOUND
        );
    }

    #[test]
    fn unsubscribe_client_removes_all() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        m.subscribe("file:///b.rs", "c1").unwrap();
        m.subscribe("file:///c.rs", "c2").unwrap();
        let removed = m.unsubscribe_client("c1");
        assert_eq!(removed, 2);
        assert_eq!(m.uri_count(), 1);
        assert_eq!(m.subscribers("file:///c.rs"), vec!["c2".to_string()]);
    }

    #[test]
    fn handle_subscribe_basic() {
        let m = SubscriptionManager::new();
        let req = JsonRpcRequest::new(
            "resources/subscribe",
            Some(json!({ "uri": "file:///a.rs", "client_id": "c1" })),
            Id::Num(1),
        );
        let resp = handle_resources_subscribe(&req, &m);
        assert!(resp.error.is_none());
        let r = resp.result.expect("result");
        assert_eq!(r.get("subscribed").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(m.uri_count(), 1);
    }

    #[test]
    fn handle_subscribe_missing_params_returns_error() {
        let m = SubscriptionManager::new();
        let req = JsonRpcRequest::new("resources/subscribe", None, Id::Num(2));
        let resp = handle_resources_subscribe(&req, &m);
        assert_eq!(resp.error.unwrap().code, SUBSCRIBE_INVALID_URI);
    }

    #[test]
    fn handle_unsubscribe_basic() {
        let m = SubscriptionManager::new();
        m.subscribe("file:///a.rs", "c1").unwrap();
        let req = JsonRpcRequest::new(
            "resources/unsubscribe",
            Some(json!({ "uri": "file:///a.rs", "client_id": "c1" })),
            Id::Num(4),
        );
        let resp = handle_resources_unsubscribe(&req, &m);
        assert!(resp.error.is_none());
        assert_eq!(m.uri_count(), 0);
    }

    #[test]
    fn build_resource_updated_notification_basic() {
        let n = build_resource_updated_notification("file:///a.rs");
        assert_eq!(n.method, "notifications/resources/updated");
        assert!(n.id.is_none());
        let params = n.params.expect("params");
        assert_eq!(
            params.get("uri").and_then(|v| v.as_str()),
            Some("file:///a.rs")
        );
    }

    #[test]
    fn tool_event_with_progress_clamped() {
        let e = ToolEvent::new(ToolEventKind::Progress, "x", "y").with_progress(150);
        assert_eq!(e.progress, Some(100));
    }

    #[test]
    fn tool_subscription_matches_specific_and_global() {
        let s = ToolSubscription::new("long-task", "c1");
        let e1 = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        let e2 = ToolEvent::new(ToolEventKind::Progress, "other-task", "running");
        assert!(s.matches(&e1));
        assert!(!s.matches(&e2));
        let g = ToolSubscription::new("", "c1");
        assert!(g.matches(&e1));
    }

    #[test]
    fn tool_subscription_with_filter() {
        let s = ToolSubscription::new("long-task", "c1")
            .with_filter([ToolEventKind::Progress, ToolEventKind::Completed]);
        assert!(s.matches(&ToolEvent::new(
            ToolEventKind::Progress,
            "long-task",
            "running"
        )));
        assert!(!s.matches(&ToolEvent::new(ToolEventKind::Failed, "long-task", "error")));
    }

    #[test]
    fn broker_subscribe_duplicate_and_empty_rejected() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("long-task", "c1"))
            .unwrap();
        assert_eq!(
            b.subscribe(ToolSubscription::new("long-task", "c1"))
                .unwrap_err()
                .code,
            TOOL_SUBSCRIBE_ALREADY
        );
        assert_eq!(
            b.subscribe(ToolSubscription::new("", "")).unwrap_err().code,
            TOOL_SUBSCRIBE_INVALID_NAME
        );
    }

    #[test]
    fn broker_dispatch_event_global_and_specific() {
        let b = ToolEventBroker::new();
        b.subscribe(ToolSubscription::new("", "c1")).unwrap();
        b.subscribe(ToolSubscription::new("long-task", "c2"))
            .unwrap();
        let event = ToolEvent::new(ToolEventKind::Progress, "long-task", "running");
        let mut matched = b.dispatch_event(&event);
        matched.sort();
        assert_eq!(matched.len(), 2);
        let none = b.dispatch_event(&ToolEvent::new(
            ToolEventKind::Progress,
            "other-task",
            "running",
        ));
        assert_eq!(none.len(), 1); // global still matches
        assert_eq!(none[0].1, "c1");
    }

    #[test]
    fn broker_dispatch_event_with_filter() {
        let b = ToolEventBroker::new();
        b.subscribe(
            ToolSubscription::new("long-task", "c1").with_filter([ToolEventKind::Completed]),
        )
        .unwrap();
        assert!(b
            .dispatch_event(&ToolEvent::new(
                ToolEventKind::Progress,
                "long-task",
                "running"
            ))
            .is_empty());
        assert_eq!(
            b.dispatch_event(&ToolEvent::new(
                ToolEventKind::Completed,
                "long-task",
                "done"
            ))
            .len(),
            1
        );
    }

    #[test]
    fn handle_tools_subscribe_with_filter() {
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
    fn handle_tools_unsubscribe_basic() {
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
        assert_eq!(b.uri_count(), 0);
    }

    #[test]
    fn build_progress_notification_clamps_to_100() {
        let n = build_tool_progress_notification("x", 200, "msg");
        assert!(n.id.is_none());
        let p = n.params.expect("params");
        assert_eq!(p.get("progress").and_then(|v| v.as_u64()), Some(100));
    }

    #[test]
    fn build_list_changed_and_completed() {
        let n = build_tool_list_changed_notification();
        assert_eq!(n.method, "notifications/tools/list_changed");
        let c = build_tool_completed_notification("long-task", "result: 42");
        assert_eq!(c.method, "notifications/tools/completed");
        assert_eq!(c.params.unwrap()["result_summary"], "result: 42");
    }
}
