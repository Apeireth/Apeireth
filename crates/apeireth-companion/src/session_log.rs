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
//! - `anchor`     — S6 锚定检查点 (不进 surface)
//!
//! 0 假装: 这里实现「日志 + 派生 + 修复」三件机制件; 具体事件内容由调用方 (LLM 循环) 提供.
//!
//! ## S6 审计哈希链: SHA-256 + epoch 锚定 (台账 S6)
//! - **哈希算法**: FNV-1a 64 (非加密可碰撞) → SHA-256。事件带 `hash_era` 字段
//!   (`#[serde(default)]`): 0 = FNV 存量时代, 1 = SHA-256 新时代。旧库事件无此字段
//!   → 反序列化默认 era=0 → 按 FNV 校验; 新写入一律 era=1。
//! - **存量链兼容策略 (如实标注, 非无缝迁移)**: 边界事件的 `prev_hash` 指向 16 位
//!   FNV 遗留哈希, epoch 边界天然记录在链上。**存量段仍只有 FNV 强度** — 不追溯
//!   重哈希; 若需为存量段重锚定, 走「重建链 + 外部存证」迁移窗口 (未实现, 升级路径)。
//! - **锚定**: `with_anchor_every(n)` 每 n 条事件追加一个 kind="anchor" 检查点事件,
//!   payload 含被锚事件的哈希快照 — 供主人侧外部存证 (tamper-evident)。
//!   0 装 PASS: 锚定是哈希检查点非非对称签名 (无密钥管理); 真签名锚定为升级路径。

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde_json::{json, Value};

/// 会话事件 (append-only 日志单元, 带哈希链).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionEvent {
    pub seq: u64,
    pub kind: String, // "user" | "assistant" | "tool" | "anchor"
    pub payload: Value,
    /// 前一条事件哈希 (篡改检测链).
    pub prev_hash: Option<String>,
    /// 哈希时代 (S6): 0 = FNV-1a 64 存量, 1 = SHA-256。
    /// `#[serde(default)]` — 旧库事件无此字段即 era=0, 按存量算法校验。
    #[serde(default)]
    pub hash_era: u8,
}

impl SessionEvent {
    pub fn user(content: impl Into<String>, seq: u64) -> Self {
        Self {
            seq,
            kind: "user".into(),
            payload: json!({"content": content.into()}),
            prev_hash: None,
            hash_era: 1,
        }
    }
    pub fn assistant(content: impl Into<String>, tool_calls: Value, seq: u64) -> Self {
        Self {
            seq,
            kind: "assistant".into(),
            payload: json!({"content": content.into(), "tool_calls": tool_calls}),
            prev_hash: None,
            hash_era: 1,
        }
    }
    pub fn tool(tool_call_id: impl Into<String>, result: Value, seq: u64) -> Self {
        Self {
            seq,
            kind: "tool".into(),
            payload: json!({"tool_call_id": tool_call_id.into(), "result": result}),
            prev_hash: None,
            hash_era: 1,
        }
    }

    /// 合成闭包事件 (崩溃修复): 该 tool_call 结果未知.
    pub fn tool_outcome_unknown(tool_call_id: impl Into<String>, seq: u64) -> Self {
        Self::tool(
            tool_call_id,
            json!({"__outcome__": "TOOL_OUTCOME_UNKNOWN"}),
            seq,
        )
    }

    /// 事件哈希 (S6 按 hash_era 分派): seq|kind|payload|prev_hash.
    /// era=1 → SHA-256 (64 hex); era=0 → FNV-1a 64 (存量校验保留).
    pub fn hash(&self) -> String {
        let prev = self.prev_hash.as_deref().unwrap_or("");
        let joined = format!("{}|{}|{}|{}", self.seq, self.kind, self.payload, prev);
        match self.hash_era {
            0 => format!("{:016x}", fnv1a(joined.as_bytes())),
            _ => sha256_hex(&joined),
        }
    }
}

/// FNV-1a 64 (仅存量 era=0 校验用; 非加密可碰撞, 新事件禁用).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// SHA-256 → 64 位 hex (手写 hex, 不引 hex crate).
fn sha256_hex(s: &str) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(s.as_bytes());
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// 纯函数哈希链校验 (无 DB 依赖, 便于篡改用例测试).
/// 链路: 每条 prev_hash == 前一条 hash (按各自 era); anchor 的 chain_hash == 被锚事件 hash.
pub fn verify_events(evs: &[SessionEvent]) -> (bool, Option<u64>) {
    let mut prev_hash: Option<String> = None;
    for e in evs {
        if e.prev_hash != prev_hash {
            return (false, Some(e.seq));
        }
        if e.kind == "anchor" {
            let covers = e.payload.get("covers").and_then(|v| v.as_u64());
            let claimed = e.payload.get("chain_hash").and_then(|v| v.as_str());
            let actual = covers
                .and_then(|c| evs.iter().find(|x| x.seq == c))
                .map(|x| x.hash());
            if claimed != actual.as_deref() {
                return (false, Some(e.seq));
            }
        }
        prev_hash = Some(e.hash());
    }
    (true, None)
}

/// 会话事件日志: append-only 写真库 + 按序重放 + surface 派生 + 崩溃修复.
pub struct SessionLog {
    store: Arc<SqliteMemoryStore>,
    session_id: String,
    /// S6 锚定间隔: 每 n 条事件追加 anchor 检查点 (0 = 不锚定).
    anchor_every: usize,
}

/// 事件在真库里的 id 前缀 (episodes 表共用, 与 mem-*/reflect-* 区分).
const EVENT_ID_PREFIX: &str = "slog-";

impl SessionLog {
    pub fn new(store: Arc<SqliteMemoryStore>, session_id: impl Into<String>) -> Self {
        Self {
            store,
            session_id: session_id.into(),
            anchor_every: 0,
        }
    }

    /// S6: 每 n 条事件追加锚定检查点 (tamper-evident, 供外部存证).
    pub fn with_anchor_every(mut self, n: usize) -> Self {
        self.anchor_every = n;
        self
    }

    /// 追加一条事件 (seq 自动 = 当前日志长度; 哈希链 prev_hash 自动接).
    /// 达到锚定间隔时追加 kind="anchor" 检查点事件 (payload 含被锚事件哈希快照).
    pub fn append(&self, kind: &str, payload: Value) -> Result<u64, String> {
        let seq = self.append_inner(kind, payload)?;
        if self.anchor_every > 0 && (seq as usize + 1) % self.anchor_every == 0 {
            let covered_hash = self
                .replay()?
                .last()
                .map(|e| e.hash())
                .ok_or_else(|| "锚定失败: 日志为空".to_string())?;
            self.append_inner("anchor", json!({"covers": seq, "chain_hash": covered_hash}))?;
        }
        Ok(seq)
    }

    fn append_inner(&self, kind: &str, payload: Value) -> Result<u64, String> {
        let seq = self.len()? as u64;
        let prev_hash = if seq > 0 {
            self.replay()?.last().map(|e| e.hash())
        } else {
            None
        };
        let ev = SessionEvent {
            seq,
            kind: kind.into(),
            payload,
            prev_hash,
            hash_era: 1,
        };
        self.put(&ev)?;
        Ok(seq)
    }

    /// 校验哈希链: 每条事件的 prev_hash 必须等于前一条的 hash (按各自 hash_era 重算),
    /// anchor 检查点的 chain_hash 必须与被锚事件的哈希一致.
    /// 返回 (ok, 损坏位置 Option<seq>).
    pub fn verify_chain(&self) -> (bool, Option<u64>) {
        let Ok(evs) = self.replay() else {
            return (false, None);
        };
        verify_events(&evs)
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
        Ok(eps
            .iter()
            .filter(|e| e.id.starts_with(EVENT_ID_PREFIX))
            .count())
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

    /// surface 投影: 事件 → 模型可见 messages (OpenAI 形状). anchor 检查点不进 surface.
    pub fn assemble_surface(&self) -> Result<Vec<Value>, String> {
        Ok(self
            .replay()?
            .into_iter()
            .filter(|ev| ev.kind != "anchor")
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
        s.append("user", json!({"content": "任务: 写错题本"}))
            .unwrap();
        s.append("assistant", json!({"content": "先查记忆", "tool_calls": [{"id": "c1", "function": {"name": "recall_memory", "arguments": "{}"}}]})).unwrap();
        s.append(
            "tool",
            json!({"tool_call_id": "c1", "result": {"found": 1}}),
        )
        .unwrap();
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
        assert!(after[2]["content"]
            .as_str()
            .unwrap_or("")
            .contains("TOOL_OUTCOME_UNKNOWN"));
        // 幂等: 再修 0
        assert_eq!(s.repair_interrupted().unwrap(), 0);
    }

    #[test]
    fn repair_skips_closed_tool_calls() {
        let s = SessionLog::new(store(), "me");
        s.append(
            "assistant",
            json!({"content": "调工具", "tool_calls": [{"id": "c1"}]}),
        )
        .unwrap();
        s.append(
            "tool",
            json!({"tool_call_id": "c1", "result": {"ok": true}}),
        )
        .unwrap();
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

    // ===== S6: SHA-256 哈希链 + 锚定 + epoch 兼容 =====

    #[test]
    fn sha256_hash_is_64hex_and_deterministic() {
        let ev = SessionEvent::user("审计事件", 0);
        let h1 = ev.hash();
        let h2 = ev.hash();
        assert_eq!(h1, h2, "确定性");
        assert_eq!(h1.len(), 64, "SHA-256 应为 64 hex (非 16 hex FNV)");
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
        // 内容不同 → 哈希不同 (无碰撞到可测程度)
        let ev2 = SessionEvent::user("审计事件2", 0);
        assert_ne!(ev.hash(), ev2.hash());
    }

    #[test]
    fn verify_chain_detects_tampered_payload() {
        let s = SessionLog::new(store(), "me");
        for c in ["一", "二", "三"] {
            s.append("user", json!({"content": c})).unwrap();
        }
        assert_eq!(s.verify_chain(), (true, None), "未篡改链应通过");
        // 篡改中间事件 (payload 改写, prev_hash 不动) — 模拟攻击者改库
        let mut evs = s.replay().unwrap();
        evs[1].payload = json!({"content": "伪造"});
        let (ok, at) = verify_events(&evs);
        assert!(!ok, "篡改必须被检出");
        assert_eq!(at, Some(2), "损坏位置 = 篡改事件的下一条 (链接断裂处)");
    }

    #[test]
    fn verify_chain_detects_tampered_prev_hash() {
        let s = SessionLog::new(store(), "me");
        s.append("user", json!({"content": "a"})).unwrap();
        s.append("user", json!({"content": "b"})).unwrap();
        let mut evs = s.replay().unwrap();
        evs[1].prev_hash = Some("0".repeat(64)); // 伪造前驱哈希
        let (ok, at) = verify_events(&evs);
        assert!(!ok);
        assert_eq!(at, Some(1));
    }

    #[test]
    fn anchor_every_n_emits_checkpoint_and_verifies() {
        let s = SessionLog::new(store(), "me").with_anchor_every(2);
        s.append("user", json!({"content": "一"})).unwrap();
        s.append("user", json!({"content": "二"})).unwrap(); // 第 2 条 → 触发锚定
        let evs = s.replay().unwrap();
        assert_eq!(evs.len(), 3, "2 事件 + 1 anchor");
        assert_eq!(evs[2].kind, "anchor");
        assert_eq!(evs[2].payload["covers"], json!(1), "锚住第 2 条 (seq=1)");
        assert_eq!(s.verify_chain(), (true, None), "锚定链校验通过");
        // anchor 不进 surface
        assert_eq!(s.assemble_surface().unwrap().len(), 2);
        // 篡改被锚事件 → anchor chain_hash 失配被检出
        let mut evs = s.replay().unwrap();
        evs[1].payload = json!({"content": "伪造"});
        // 攻击者若同步重算链上哈希, anchor 快照仍暴露篡改 (外部存证比对点)
        let (ok, _) = verify_events(&evs);
        assert!(!ok, "锚定快照与链哈希双重防线");
    }

    #[test]
    fn legacy_fnv_epoch_boundary_verified_with_dual_algorithm() {
        // 模拟存量库: era=0 事件按 FNV 链接 (16 hex)
        let ev0 = SessionEvent {
            seq: 0,
            kind: "user".into(),
            payload: json!({"content": "存量"}),
            prev_hash: None,
            hash_era: 0,
        };
        let legacy_hash = ev0.hash();
        assert_eq!(legacy_hash.len(), 16, "存量 era=0 仍 FNV 16 hex");
        let ev1 = SessionEvent {
            seq: 1,
            kind: "user".into(),
            payload: json!({"content": "新时代"}),
            prev_hash: Some(legacy_hash),
            hash_era: 1,
        };
        assert_eq!(
            verify_events(&[ev0.clone(), ev1.clone()]),
            (true, None),
            "epoch 边界链校验通过"
        );
        // serde 兼容: 旧库 JSON 无 hash_era 字段 → 默认 era=0
        let legacy_json =
            r#"{"seq":0,"kind":"user","payload":{"content":"旧事件"},"prev_hash":null}"#;
        let parsed: SessionEvent = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(parsed.hash_era, 0, "缺字段默认存量 era");
        assert_eq!(parsed.hash().len(), 16, "存量事件按 FNV 校验");
        // 篡改存量段同样被检出 (FNV 强度内)
        let mut tampered = ev0.clone();
        tampered.payload = json!({"content": "伪造存量"});
        assert!(!verify_events(&[tampered, ev1]).0);
    }
}
