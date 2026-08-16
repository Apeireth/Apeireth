//! `apeireth-companion::session_log` — 事件溯源会话底座 (吸收 DSH session 设计, Rust 重写).
//!
//! 核心思想 (DSH 第 1 项吸收):
//! - **append-only 事件日志 = 唯一真相**: 会话历史不是二次存储, 是从日志**派生**的
//! - **surface 投影**: `assemble_surface()` 按序重放事件 → 模型可见的 messages
//! - **崩溃修复**: 检测「缺闭包」的 turn (assistant 发起了 tool_call 但无 tool/result),
//!   合成 `TOOL_OUTCOME_UNKNOWN` 闭包事件, 让 transcript 重新合法
//! - 持久化: 事件以 episodes 写入真 SQLite (kind 前缀标记), 可跨进程恢复
//!
//! 事件类型:
//! - `user`       — 用户消息
//! - `assistant`  — 模型消息 (含 tool_calls)
//! - `tool`       — 工具结果 (tool_call_id)
//!
//! 0 假装: 这里实现「日志 + 派生 + 修复」三件机制件; 具体事件内容由调用方 (LLM 循环) 提供.

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde_json::{json, Value};

/// 会话事件 (append-only 日志单元, 带哈希链).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionEvent {
    pub seq: u64,
    pub kind: String, // "user" | "assistant" | "tool"
    pub payload: Value,
    /// 前一条事件哈希 (FNV-1a 64, 篡改检测链).
    pub prev_hash: Option<String>,
}

impl SessionEvent {
    pub fn user(content: impl Into<String>, seq: u64) -> Self {
        Self { seq, kind: "user".into(), payload: json!({"content": content.into()}), prev_hash: None }
    }
    pub fn assistant(content: impl Into<String>, tool_calls: Value, seq: u64) -> Self {
        Self { seq, kind: "assistant".into(), payload: json!({"content": content.into(), "tool_calls": tool_calls}), prev_hash: None }
    }
    pub fn tool(tool_call_id: impl Into<String>, result: Value, seq: u64) -> Self {
        Self { seq, kind: "tool".into(), payload: json!({"tool_call_id": tool_call_id.into(), "result": result}), prev_hash: None }
    }

    /// 合成闭包事件 (崩溃修复): 该 tool_call 结果未知.
    pub fn tool_outcome_unknown(tool_call_id: impl Into<String>, seq: u64) -> Self {
        Self::tool(tool_call_id, json!({"__outcome__": "TOOL_OUTCOME_UNKNOWN"}), seq)
    }

    /// FNV-1a 64 哈希 (确定性, 无新依赖): seq|kind|payload|prev_hash.
    pub fn hash(&self) -> String {
        fn fnv1a(bytes: &[u8]) -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in bytes {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        }
        let prev = self.prev_hash.as_deref().unwrap_or("");
        let joined = format!("{}|{}|{}|{}", self.seq, self.kind, self.payload, prev);
        format!("{:016x}", fnv1a(joined.as_bytes()))
    }
}

/// 会话事件日志: append-only 写真库 + 按序重放 + surface 派生 + 崩溃修复.
pub struct SessionLog {
    store: Arc<SqliteMemoryStore>,
    session_id: String,
}

/// 事件在真库里的 id 前缀 (episodes 表共用, 与 mem-*/reflect-* 区分).
const EVENT_ID_PREFIX: &str = "slog-";

impl SessionLog {
    pub fn new(store: Arc<SqliteMemoryStore>, session_id: impl Into<String>) -> Self {
        Self { store, session_id: session_id.into() }
    }

    /// 追加一条事件 (seq 自动 = 当前日志长度; 哈希链 prev_hash 自动接).
    pub fn append(&self, kind: &str, payload: Value) -> Result<u64, String> {
        let seq = self.len()? as u64;
        let prev_hash = if seq > 0 {
            self.replay()?.last().map(|e| e.hash())
        } else {
            None
        };
        let ev = SessionEvent { seq, kind: kind.into(), payload, prev_hash };
        self.put(&ev)?;
        Ok(seq)
    }

    /// 校验哈希链: 每条事件的 prev_hash 必须等于前一条的 hash.
    /// 返回 (ok, 损坏位置 Option<seq>).
    pub fn verify_chain(&self) -> (bool, Option<u64>) {
        let evs = match self.replay() {
            Ok(e) => e,
            Err(_) => return (false, None),
        };
        let mut prev_hash: Option<String> = None;
        for e in &evs {
            if e.prev_hash != prev_hash {
                return (false, Some(e.seq));
            }
            prev_hash = Some(e.hash());
        }
        (true, None)
    }

    fn put(&self, ev: &SessionEvent) -> Result<(), String> {
        let ep = CoreEpisode {
            id: format!("{EVENT_ID_PREFIX}{}", ev.seq),
            timestamp: chrono::Utc::now().timestamp(),
            role: "system".into(),
            content: serde_json::to_string(ev).map_err(|e| e.to_string())?,
            session_id: self.session_id.clone(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    /// 当前日志长度 (事件数).
    pub fn len(&self) -> Result<usize, String> {
        let eps = self
            .store
            .recent_episodes(&self.session_id, 1000)
            .map_err(|e| e.to_string())?;
        Ok(eps.iter().filter(|e| e.id.starts_with(EVENT_ID_PREFIX)).count())
    }

    /// 按 seq 排序重放全部事件 (日志 = 唯一真相).
    pub fn replay(&self) -> Result<Vec<SessionEvent>, String> {
        let eps = self
            .store
            .recent_episodes(&self.session_id, 1000)
            .map_err(|e| e.to_string())?;
        let mut evs: Vec<SessionEvent> = Vec::new();
        for e in eps.iter().filter(|e| e.id.starts_with(EVENT_ID_PREFIX)) {
            if let Ok(ev) = serde_json::from_str::<SessionEvent>(&e.content) {
                evs.push(ev);
            }
        }
        evs.sort_by_key(|e| e.seq);
        Ok(evs)
    }

    /// surface 投影: 事件 → 模型可见 messages (OpenAI 形状).
    pub fn assemble_surface(&self) -> Result<Vec<Value>, String> {
        Ok(self
            .replay()?
            .into_iter()
            .map(|ev| match ev.kind.as_str() {
                "user" => json!({"role": "user", "content": ev.payload.get("content").cloned().unwrap_or(json!(null))}),
                "assistant" => json!({
                    "role": "assistant",
                    "content": ev.payload.get("content").cloned().unwrap_or(json!(null)),
                    "tool_calls": ev.payload.get("tool_calls").cloned().unwrap_or(json!([])),
                }),
                "tool" => json!({
                    "role": "tool",
                    "tool_call_id": ev.payload.get("tool_call_id").cloned().unwrap_or(json!(null)),
                    "content": serde_json::to_string(ev.payload.get("result").unwrap_or(&json!(null))).unwrap_or_default(),
                }),
                _ => json!({"role": "system", "content": format!("unknown event {}", ev.kind)}),
            })
            .collect())
    }

    /// 崩溃修复: 扫描缺闭包的 assistant tool_call, 补合成 TOOL_OUTCOME_UNKNOWN 事件.
    /// 返回修复条数; 幂等 (已闭合的调用跳过).
    pub fn repair_interrupted(&self) -> Result<usize, String> {
        let evs = self.replay()?;
        let mut open: Vec<String> = Vec::new(); // 未闭合的 tool_call_id
        let mut fixed = 0usize;
        let mut seq = evs.len() as u64;
        for ev in &evs {
            match ev.kind.as_str() {
                "assistant" => {
                    if let Some(calls) = ev.payload.get("tool_calls").and_then(|c| c.as_array()) {
                        for c in calls {
                            if let Some(id) = c.get("id").and_then(|v| v.as_str()) {
                                open.push(id.to_string());
                            }
                        }
                    }
                }
                "tool" => {
                    if let Some(id) = ev.payload.get("tool_call_id").and_then(|v| v.as_str()) {
                        open.retain(|o| o != id);
                    }
                }
                _ => {}
            }
        }
        for id in open {
            let close = SessionEvent::tool_outcome_unknown(&id, seq);
            self.put(&close)?;
            seq += 1;
            fixed += 1;
        }
        Ok(fixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn append_replay_surface_round_trip() {
        let s = SessionLog::new(store(), "me");
        s.append("user", json!({"content": "任务: 写错题本"})).unwrap();
        s.append("assistant", json!({"content": "先查记忆", "tool_calls": [{"id": "c1", "function": {"name": "recall_memory", "arguments": "{}"}}]})).unwrap();
        s.append("tool", json!({"tool_call_id": "c1", "result": {"found": 1}})).unwrap();
        assert_eq!(s.len().unwrap(), 3);
        let surface = s.assemble_surface().unwrap();
        assert_eq!(surface.len(), 3);
        assert_eq!(surface[0]["role"], json!("user"));
        assert_eq!(surface[1]["role"], json!("assistant"));
        assert_eq!(surface[2]["tool_call_id"], json!("c1"));
        assert_eq!(surface[2]["content"], "{\"found\":1}");
    }

    #[test]
    fn repair_closes_interrupted_turn() {
        let s = SessionLog::new(store(), "me");
        s.append("user", json!({"content": "继续"})).unwrap();
        // assistant 发起 tool_call 但无结果 (崩溃现场)
        s.append("assistant", json!({"content": "调工具", "tool_calls": [{"id": "c9", "function": {"name": "FileOperator", "arguments": "{}"}}]})).unwrap();
        // 修复前 surface 有悬空 tool_call
        let before = s.assemble_surface().unwrap();
        assert_eq!(before.len(), 2);
        let n = s.repair_interrupted().unwrap();
        assert_eq!(n, 1, "应合成 1 个闭包事件");
        let after = s.assemble_surface().unwrap();
        assert_eq!(after.len(), 3, "修复后 surface 应含闭包 tool 事件");
        assert!(after[2]["content"].as_str().unwrap_or("").contains("TOOL_OUTCOME_UNKNOWN"));
        // 幂等: 再修 0
        assert_eq!(s.repair_interrupted().unwrap(), 0);
    }

    #[test]
    fn repair_skips_closed_tool_calls() {
        let s = SessionLog::new(store(), "me");
        s.append("assistant", json!({"content": "调工具", "tool_calls": [{"id": "c1"}]})).unwrap();
        s.append("tool", json!({"tool_call_id": "c1", "result": {"ok": true}})).unwrap();
        assert_eq!(s.repair_interrupted().unwrap(), 0, "已闭合不应修复");
    }

    #[test]
    fn replay_orders_by_seq() {
        let s = SessionLog::new(store(), "me");
        // 乱序写入 (seq 由 append 分配, 顺序追加)
        s.append("user", json!({"content": "一"})).unwrap();
        s.append("user", json!({"content": "二"})).unwrap();
        let evs = s.replay().unwrap();
        assert_eq!(evs[0].seq, 0);
        assert_eq!(evs[1].seq, 1);
        assert_eq!(evs[0].payload["content"], json!("一"));
    }
}
