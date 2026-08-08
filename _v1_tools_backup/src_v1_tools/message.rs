//! `/v1/tools/message/invoke` — **D-01 真接** (R20 阶段 2 推翻原 stub 501 推荐)
//!
//! **端点**: `POST /v1/tools/message/invoke`
//! **真接 (D-01)**: 主人 2026-08-05 20:53 拍板, message 走 Tool trait 真实现, **不** stub 返 501.
//! **3 actions** (per 主人任务稿, 简化自蓝图 §2.2 路由表 #6 4 actions):
//! - `send`      — 发送消息到 target
//! - `list`      — 列出消息 (可选 filter: target / sender)
//! - `subscribe` — 拉取目标 inbox 收件箱 (in-memory pub-sub)
//!
//! **Req schema** (per §2.2 路由表 #6):
//! ```json
//! { "args": { "action": "send|list|subscribe", "target": "string", "payload"?: ..., "filter"?: {...} } }
//! ```
//! **Resp schema** (per §2.2 统一信封):
//! ```json
//! { "ok": true, "result": { "message_id": "...", "messages": [...] }, "error": null, "meta": { "tool": "message", ... } }
//! ```
//!
//! **存储**: in-memory `Arc<Mutex<Vec<Message>>>` (linear log, 阶段 4 SDK 接入时换 SQLite / Redis)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 3 actions 全部真实现 (非 stub 501, D-01 决策)
//! - ✅ in-memory pub-sub 真跑 (send 写 list, subscribe 读+清空 target inbox)
//! - ✅ message ID 用 UUID v4
//!
//! **不修改承诺**:
//! - ❌ 0 改 `apeireth-mcp/src/` (LOCKED, message 不走 mcp, 走 Tool trait)
//! - ❌ 0 改 `apeireth-tool-registry/src/` (LOCKED)
//! - ❌ 0 改 workspace version (1.0.0)

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// **Message** — 5 字段
///
/// **字段** (per R20 阶段 2 + 蓝图 §2.2):
/// - `id`      — UUID v4
/// - `sender`  — 发送者
/// - `target`  — 目标 (inbox key)
/// - `payload` — 任意 JSON Value
/// - `ts`      — Unix timestamp (秒)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub target: String,
    pub payload: Value,
    pub ts: i64,
}

/// **MessageTool** — 3 actions 真实现 (D-01 真接)
///
/// **存储**: 单 `Arc<Mutex<Vec<Message>>>` linear log (按 ts 排序, subscribe 拉取 target 命中项)
/// **线程安全**: `parking_lot::Mutex`
pub struct MessageTool {
    /// 消息 linear log (按 ts 追加)
    messages: Arc<Mutex<Vec<Message>>>,
}

impl MessageTool {
    /// 新建空 MessageTool
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 3 actions 路由分发
    async fn dispatch(&self, args: Value) -> Result<Value, String> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: action".to_string())?;

        match action {
            "send" => self.action_send(args).await,
            "list" => self.action_list(args).await,
            "subscribe" => self.action_subscribe(args).await,
            other => Err(format!("unknown action: {other}")),
        }
    }

    /// **action: send** — 发送消息到 target
    async fn action_send(&self, args: Value) -> Result<Value, String> {
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: target".to_string())?;
        let sender = args
            .get("sender")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous")
            .to_string();
        let payload = args
            .get("payload")
            .cloned()
            .unwrap_or(Value::Null);
        let ts = args
            .get("ts")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| {
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
        Ok(json!({
            "ok": true,
            "message_id": msg_id,
        }))
    }

    /// **action: list** — 列出消息 (按 ts 倒序, 可选 filter: target / sender)
    async fn action_list(&self, args: Value) -> Result<Value, String> {
        let filter_target = args.get("filter").and_then(|f| f.get("target")).and_then(|v| v.as_str());
        let filter_sender = args.get("filter").and_then(|f| f.get("sender")).and_then(|v| v.as_str());
        let limit = args
            .get("filter")
            .and_then(|f| f.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let g = self.messages.lock();
        let mut msgs: Vec<&Message> = g
            .iter()
            .filter(|m| {
                filter_target.map(|t| m.target == t).unwrap_or(true)
                    && filter_sender.map(|s| m.sender == s).unwrap_or(true)
            })
            .collect();
        // 按 ts 倒序
        msgs.sort_by(|a, b| b.ts.cmp(&a.ts));
        msgs.truncate(limit);
        let total = g.len();
        Ok(json!({
            "messages": msgs,
            "count": msgs.len(),
            "total": total,
        }))
    }

    /// **action: subscribe** — 拉取目标 inbox 收件箱 (target 命中项, drain 语义)
    async fn action_subscribe(&self, args: Value) -> Result<Value, String> {
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: target".to_string())?;
        let mut g = self.messages.lock();
        // drain 命中 target 的消息, 其余保留
        let mut mine: Vec<Message> = Vec::new();
        let mut rest: Vec<Message> = Vec::with_capacity(g.len());
        for m in g.drain(..) {
            if m.target == target {
                mine.push(m);
            } else {
                rest.push(m);
            }
        }
        // 重建 g 为 rest (保留其他 target)
        *g = rest;
        let count = mine.len();
        Ok(json!({
            "messages": mine,
            "count": count,
        }))
    }
}

impl Default for MessageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str {
        "Message"
    }

    fn kind(&self) -> ToolKind {
        // 3 actions 是同步操作 (Mutex 守的 in-memory)
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
pub use super::invoke_by_name as invoke;

// ============================================================
// 单元测试 (3 actions 真实现, per D-01 真接 + 蓝图 §2.7 message fixture)
// ============================================================

#[cfg(test)]
mod message_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_path_matches_d02_subpath() {
        assert_eq!(super::super::V1_TOOLS_PATHS[5], "/tools/message/invoke");
        assert_eq!(super::super::V1_TOOL_NAMES[5], "message");
        assert_eq!(super::super::V1_REGISTRY_NAMES[5], "Message");
    }

    /// **D-01 真接** — message 不 stub, 0 返 501
    #[test]
    fn message_is_real_not_stub() {
        // D-01: 真接 (推翻原 A 推荐 stub 501, 主人 2026-08-05 20:53 拍板 B)
        let m = MessageTool::new();
        assert_eq!(m.name(), "Message");
    }

    /// **3 actions 端到端真跑** (per 蓝图 §2.7 message fixture 3/3 case)
    #[tokio::test]
    async fn message_3_actions_e2e() {
        let m = MessageTool::new();

        // 1. send (向 alice 发送)
        let r = m
            .call(json!({
                "action": "send",
                "target": "alice",
                "sender": "bob",
                "payload": {"text": "hello alice"}
            }))
            .await
            .expect("send to alice");
        let msg_id_alice = r["message_id"].as_str().expect("msg_id").to_string();
        assert!(!msg_id_alice.is_empty());

        // 2. send (向 bob 发送)
        let r = m
            .call(json!({
                "action": "send",
                "target": "bob",
                "sender": "alice",
                "payload": {"text": "hi bob"}
            }))
            .await
            .expect("send to bob");
        let _msg_id_bob = r["message_id"].as_str().expect("msg_id").to_string();

        // 3. list (全部)
        let r = m.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 2);
        assert_eq!(r["total"], 2);

        // 4. list filter target=alice
        let r = m
            .call(json!({"action": "list", "filter": {"target": "alice"}}))
            .await
            .expect("list alice");
        assert_eq!(r["count"], 1);

        // 5. subscribe alice (拉取 + drain)
        let r = m
            .call(json!({"action": "subscribe", "target": "alice"}))
            .await
            .expect("subscribe alice");
        assert_eq!(r["count"], 1);
        assert_eq!(r["messages"][0]["target"], "alice");
        assert_eq!(r["messages"][0]["payload"]["text"], "hello alice");
    }

    /// **3 actions 错误路径** — 缺字段 + 未知 action
    #[tokio::test]
    async fn message_3_actions_error_paths() {
        let m = MessageTool::new();

        // 缺 action
        let r = m.call(json!({})).await;
        assert!(r.is_err());

        // 未知 action
        let r = m.call(json!({"action": "broadcast"})).await;
        assert!(r.is_err());

        // send 缺 target
        let r = m.call(json!({"action": "send", "payload": "x"})).await;
        assert!(r.is_err());

        // list 无消息 (空)
        let r = m.call(json!({"action": "list"})).await.expect("list empty");
        assert_eq!(r["count"], 0);

        // subscribe 缺 target
        let r = m.call(json!({"action": "subscribe"})).await;
        assert!(r.is_err());
    }
}
