//! R72: MCP subscriptions protocol — resources/subscribe + resources/unsubscribe (push 模式)
//!
//! **MCP 协议 (per modelcontextprotocol/specification 2025-06-18 §resources/subscribe)**:
//! - `resources/subscribe` — 客户端订阅 server 端 resource URI 变化, server 端主动 push `notifications/resources/updated`
//! - `resources/unsubscribe` — 客户端取消订阅
//! - `notifications/resources/updated` — server 端推送 (notification 形式, 无 id)
//!
//! **Apeireth 真接 (本 module)**:
//! - `Subscription` — URI + client id (String, e.g. UUID)
//! - `SubscriptionManager` — HashMap<URI, HashSet<client_id>> + 锁保护
//! - `handle_resources_subscribe(req) -> JsonRpcResponse`
//! - `handle_resources_unsubscribe(req) -> JsonRpcResponse`
//! - `build_resource_updated_notification(uri) -> JsonRpcRequest` — push notification 构造
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `resources.rs` / `protocol.rs` / `ResourceServer` (R33-3 R33-3-1 LOCKED)
//! - 0 引入 async (sync 路径, push notification 由 caller 异步分发)
//! - 0 业务耦合 (apeireth-mcp 0 依赖 tui/api, 任意 server impl 都能挂)
//!
//! **借鉴锚 (S-1)**:
//! - MCP spec 2025-06-18 §resources/subscribe (push 模式)
//! - LSP `workspace/didChangeWatchedFiles` notification 模式 (push 1:1)
//! - GraphQL subscriptions (`graphql-ws` protocol SUBSCRIBE frame 借鉴)

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::protocol::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// MCP 错误码 (per MCP spec, -32000 ~ -32099 范围 server-define)
pub const SUBSCRIBE_INVALID_URI: i32 = -32020;
pub const SUBSCRIBE_NOT_FOUND: i32 = -32021;
pub const SUBSCRIBE_ALREADY_SUBSCRIBED: i32 = -32022;

/// 单个 subscription
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    /// 订阅的 resource URI
    pub uri: String,
    /// 客户端 ID
    pub client_id: String,
    /// 订阅创建时间 (ms since epoch)
    pub created_at_unix_ms: u64,
}

impl Subscription {
    pub fn new(uri: impl Into<String>, client_id: impl Into<String>) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            uri: uri.into(),
            client_id: client_id.into(),
            created_at_unix_ms: now_ms,
        }
    }
}

/// SubscriptionManager — 维护 URI -> client_id 集合
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

    pub fn subscribe(&self, uri: &str, client_id: &str) -> Result<(), JsonRpcError> {
        if uri.is_empty() {
            return Err(JsonRpcError::new(SUBSCRIBE_INVALID_URI, "uri empty"));
        }
        if client_id.is_empty() {
            return Err(JsonRpcError::new(SUBSCRIBE_INVALID_URI, "client_id empty"));
        }
        let mut map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
        let entry = map.entry(uri.to_string()).or_insert_with(HashSet::new);
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
        let mut map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
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
        let map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
        map.get(uri)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn unsubscribe_client(&self, client_id: &str) -> usize {
        let mut map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
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
        let map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
        map.len()
    }

    pub fn subscription_count(&self) -> usize {
        let map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
        map.values().map(|s| s.len()).sum()
    }

    pub fn uris(&self) -> Vec<String> {
        let map = self
            .inner
            .lock()
            .expect("subscription manager mutex poisoned");
        let mut keys: Vec<String> = map.keys().cloned().collect();
        keys.sort();
        keys
    }
}

/// 处理 `resources/subscribe` 请求
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
    let uri = match params.get("uri").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => {
            return JsonRpcResponse::err(
                req.id.clone(),
                JsonRpcError::new(SUBSCRIBE_INVALID_URI, "params.uri missing or not string"),
            );
        }
    };
    let client_id = match params.get("client_id").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => format!("anon-{}", std::process::id()),
    };
    match mgr.subscribe(&uri, &client_id) {
        Ok(()) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({ "subscribed": true, "uri": uri, "client_id": client_id }),
        ),
        Err(e) => JsonRpcResponse::err(req.id.clone(), JsonRpcError::new(e.code, e.message)),
    }
}

/// 处理 `resources/unsubscribe` 请求
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
    let uri = match params.get("uri").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => {
            return JsonRpcResponse::err(
                req.id.clone(),
                JsonRpcError::new(SUBSCRIBE_INVALID_URI, "params.uri missing or not string"),
            );
        }
    };
    let client_id = match params.get("client_id").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => format!("anon-{}", std::process::id()),
    };
    match mgr.unsubscribe(&uri, &client_id) {
        Ok(()) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({ "unsubscribed": true, "uri": uri, "client_id": client_id }),
        ),
        Err(e) => JsonRpcResponse::err(req.id.clone(), JsonRpcError::new(e.code, e.message)),
    }
}

/// 构造 `notifications/resources/updated` notification (server 端 push)
pub fn build_resource_updated_notification(uri: &str) -> JsonRpcRequest {
    JsonRpcRequest::notification(
        "notifications/resources/updated",
        Some(json!({ "uri": uri })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let e1 = m.subscribe("", "c1").unwrap_err();
        assert_eq!(e1.code, SUBSCRIBE_INVALID_URI);
        let e2 = m.subscribe("uri", "").unwrap_err();
        assert_eq!(e2.code, SUBSCRIBE_INVALID_URI);
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
        assert_eq!(m.subscription_count(), 1);
        assert_eq!(m.subscribers("file:///a.rs"), vec!["c2".to_string()]);
    }

    #[test]
    fn unsubscribe_unknown_returns_error() {
        let m = SubscriptionManager::new();
        let err = m.unsubscribe("file:///a.rs", "c1").unwrap_err();
        assert_eq!(err.code, SUBSCRIBE_NOT_FOUND);
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
    fn subscribers_unknown_uri_returns_empty() {
        let m = SubscriptionManager::new();
        assert!(m.subscribers("file:///nope.rs").is_empty());
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
        assert_eq!(r.get("client_id").and_then(|v| v.as_str()), Some("c1"));
        assert_eq!(m.uri_count(), 1);
    }

    #[test]
    fn handle_subscribe_missing_params_returns_error() {
        let m = SubscriptionManager::new();
        let req = JsonRpcRequest::new("resources/subscribe", None, Id::Num(2));
        let resp = handle_resources_subscribe(&req, &m);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, SUBSCRIBE_INVALID_URI);
    }

    #[test]
    fn handle_subscribe_missing_uri_returns_error() {
        let m = SubscriptionManager::new();
        let req = JsonRpcRequest::new(
            "resources/subscribe",
            Some(json!({ "client_id": "c1" })),
            Id::Num(3),
        );
        let resp = handle_resources_subscribe(&req, &m);
        assert!(resp.error.is_some());
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
    fn handle_unsubscribe_unknown_returns_error() {
        let m = SubscriptionManager::new();
        let req = JsonRpcRequest::new(
            "resources/unsubscribe",
            Some(json!({ "uri": "file:///a.rs", "client_id": "c1" })),
            Id::Num(5),
        );
        let resp = handle_resources_unsubscribe(&req, &m);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, SUBSCRIBE_NOT_FOUND);
    }

    #[test]
    fn build_resource_updated_notification_basic() {
        let n = build_resource_updated_notification("file:///a.rs");
        assert_eq!(n.method, "notifications/resources/updated");
        assert!(n.id.is_none(), "notification must have id = None");
        let params = n.params.expect("params");
        assert_eq!(
            params.get("uri").and_then(|v| v.as_str()),
            Some("file:///a.rs")
        );
    }

    #[test]
    fn subscription_serialize_round_trip() {
        let s = Subscription::new("file:///a.rs", "c1");
        let j = serde_json::to_string(&s).unwrap();
        let r: Subscription = serde_json::from_str(&j).unwrap();
        assert_eq!(s, r);
    }
}
