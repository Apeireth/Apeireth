//! `/v1/tools/message/invoke` — **D-01 真接** (3 actions, in-memory)
//!
//! **D-01 真接**: 主人 2026-08-05 20:53 拍板, message 走 Tool trait 真实现, **不** stub 501.
//! **3 actions**: send / list / subscribe (per 主人任务稿简化)
//! **存储**: in-memory `Arc<Mutex<Vec<Message>>>` linear log
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 3 actions 全部真实现
//! - ✅ in-memory pub-sub 真跑 (send 写 list, subscribe 拉 + drain target 命中)
//! - ✅ message ID 用 UUID v4
//!
//! **不修改承诺**: ❌ 0 改 LOCKED crate, ❌ 0 改 workspace version

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub target: String,
    pub payload: Value,
    pub ts: i64,
}

pub struct MessageTool {
    messages: Arc<Mutex<Vec<Message>>>,
}

impl MessageTool {
    pub fn new() -> Self {
        Self { messages: Arc::new(Mutex::new(Vec::new())) }
    }

    async fn dispatch(&self, args: Value) -> Result<Value, String> {
        let action = args.get("action").and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: action".to_string())?;
        match action {
            "send" => self.action_send(args).await,
            "list" => self.action_list(args).await,
            "subscribe" => self.action_subscribe(args).await,
            other => Err(format!("unknown action: {other}")),
        }
    }

    async fn action_send(&self, args: Value) -> Result<Value, String> {
        let target = args.get("target").and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: target".to_string())?;
        let sender = args.get("sender").and_then(|v| v.as_str())
            .unwrap_or("anonymous").to_string();
        let payload = args.get("payload").cloned().unwrap_or(Value::Null);
        let ts = args.get("ts").and_then(|v| v.as_i64()).unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
        let msg = Message {
            id: Uuid::new_v4().to_string(),
            sender,
            target: target.to_string(),
            payload,
            ts,
        };
        let msg_id = msg.id.clone();
        self.messages.lock().push(msg);
        Ok(json!({ "ok": true, "message_id": msg_id }))
    }

    async fn action_list(&self, args: Value) -> Result<Value, String> {
        let filter_target = args.get("filter").and_then(|f| f.get("target")).and_then(|v| v.as_str());
        let filter_sender = args.get("filter").and_then(|f| f.get("sender")).and_then(|v| v.as_str());
        let limit = args.get("filter").and_then(|f| f.get("limit")).and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;
        let g = self.messages.lock();
        let mut msgs: Vec<&Message> = g.iter()
            .filter(|m| {
                filter_target.map(|t| m.target == t).unwrap_or(true)
                    && filter_sender.map(|s| m.sender == s).unwrap_or(true)
            })
            .collect();
        msgs.sort_by(|a, b| b.ts.cmp(&a.ts));
        msgs.truncate(limit);
        let total = g.len();
        Ok(json!({ "messages": msgs, "count": msgs.len(), "total": total }))
    }

    async fn action_subscribe(&self, args: Value) -> Result<Value, String> {
        let target = args.get("target").and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: target".to_string())?;
        let mut g = self.messages.lock();
        // drain 命中 target 的消息, 其余保留
        let mut mine: Vec<Message> = Vec::new();
        let mut rest: Vec<Message> = Vec::with_capacity(g.len());
        for m in g.drain(..) {
            if m.target == target { mine.push(m); } else { rest.push(m); }
        }
        *g = rest;
        let count = mine.len();
        Ok(json!({ "messages": mine, "count": count }))
    }
}

impl Default for MessageTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str { "Message" }
    fn kind(&self) -> ToolKind { ToolKind::Sync }
    fn axes(&self) -> ToolAxes { ToolAxes::default_for_kind(ToolKind::Sync) }
    async fn call(&self, args: Value) -> Result<Value, String> { self.dispatch(args).await }
}

pub use super::invoke_by_name as invoke;

#[cfg(test)]
mod message_tests {
    use super::*;

    #[test]
    fn message_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_REGISTRY_NAMES[5], "Message");
    }

    /// D-01 真接 — 3 actions 端到端
    #[tokio::test]
    async fn message_3_actions_e2e() {
        let m = MessageTool::new();
        // 1. send to alice
        let r = m.call(json!({
            "action": "send", "target": "alice", "sender": "bob",
            "payload": {"text": "hello"}
        })).await.expect("send alice");
        assert!(r["message_id"].is_string());
        // 2. send to bob
        let r = m.call(json!({
            "action": "send", "target": "bob", "sender": "alice",
            "payload": {"text": "hi bob"}
        })).await.expect("send bob");
        // 3. list
        let r = m.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 2);
        // 4. subscribe alice (drain)
        let r = m.call(json!({"action": "subscribe", "target": "alice"})).await.expect("sub alice");
        assert_eq!(r["count"], 1);
        // 5. list again (alice 已被 drain, bob 保留)
        let r = m.call(json!({"action": "list"})).await.expect("list 2");
        assert_eq!(r["count"], 1);
        assert_eq!(r["messages"][0]["target"], "bob");
    }
}
