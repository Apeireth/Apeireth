//! `apeireth-companion::continuation` — 续行快照: 工具调用点保存 LLM 上下文, 之后可恢复续跑.
//!
//! 吸收 NemesisBot 续行快照思想重写:
//! - 每轮 LLM+工具循环前保存快照 (messages + 挂起工具调用 + 轮次), 原子写 (tmp+rename)
//! - 崩溃/重启后 `consume` 恢复 → 追加真实工具结果 → 继续跑 (多轮 function calling 的断点续传)
//! - 消费后删除 (每个快照一次性)
//!
//! 0 假装: 这是「持久化 + 恢复」的机制件; 真 LLM 循环 (发请求) 由调用方
//! (example/daemon) 提供 — lib 不依赖 `apeireth-api` (同 judicator 的 trait 策略).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 挂起的工具调用 (异步等待回调 / 崩溃时未完成的那一步).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingToolCall {
    pub tool_name: String,
    pub args: Value,
    pub call_id: String,
}

/// 续行快照: 一次可恢复的 LLM 上下文.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationSnapshot {
    pub id: String,
    pub session_id: String,
    /// LLM 上下文消息 (OpenAI 形状; 每轮保存, 恢复后追加真实结果继续).
    pub messages: Vec<Value>,
    /// 挂起的工具调用 (None = 非异步断点).
    pub pending_tool_call: Option<PendingToolCall>,
    pub saved_at_ms: i64,
    pub turn: u64,
}

/// 续行快照存储: 目录 + 原子写 (tmp+rename), 按 id 存 `{id}.json`.
pub struct ContinuationStore {
    dir: PathBuf,
}

impl ContinuationStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// 原子保存: 先写 tmp, rename 覆盖 (崩溃安全).
    pub fn save(&self, snap: &ContinuationSnapshot) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("创建快照目录失败: {e}"))?;
        let tmp = self.dir.join(format!("{}.tmp-{}", snap.id, uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(snap)
            .map_err(|e| format!("快照序列化失败: {e}"))?;
        std::fs::write(&tmp, bytes).map_err(|e| format!("写 tmp 失败: {e}"))?;
        std::fs::rename(&tmp, self.path_for(&snap.id))
            .map_err(|e| format!("原子提交失败: {e}"))?;
        Ok(())
    }

    pub fn exists(&self, id: &str) -> bool {
        self.path_for(id).exists()
    }

    pub fn load(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let bytes = std::fs::read(self.path_for(id))
            .map_err(|e| format!("读快照 {id} 失败: {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("解析快照 {id} 失败: {e}"))
    }

    /// 消费 (load + 删除): 快照一次性.
    pub fn consume(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let snap = self.load(id)?;
        std::fs::remove_file(self.path_for(id))
            .map_err(|e| format!("删除快照 {id} 失败: {e}"))?;
        Ok(snap)
    }

    /// 列出全部快照 id.
    pub fn list(&self) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        rd.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snap(id: &str, turn: u64) -> ContinuationSnapshot {
        ContinuationSnapshot {
            id: id.into(),
            session_id: "me".into(),
            messages: vec![
                json!({"role": "user", "content": format!("第{turn}轮问题")}),
                json!({"role": "assistant", "content": "思考中", "tool_calls": [{"id": "c1", "function": {"name": "FileOperator", "arguments": "{}"}}]}),
            ],
            pending_tool_call: Some(PendingToolCall {
                tool_name: "FileOperator".into(),
                args: json!({"op": "write", "path": "x"}),
                call_id: "c1".into(),
            }),
            saved_at_ms: turn as i64 * 1000,
            turn,
        }
    }

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "apeireth-continuation-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn save_load_round_trip_preserves_everything() {
        let store = ContinuationStore::new(tmp_dir("rt"));
        let s = snap("snap-1", 3);
        store.save(&s).unwrap();
        assert!(store.exists("snap-1"));
        let loaded = store.load("snap-1").unwrap();
        assert_eq!(loaded.id, "snap-1");
        assert_eq!(loaded.turn, 3);
        assert_eq!(loaded.messages.len(), 2);
        let p = loaded.pending_tool_call.unwrap();
        assert_eq!(p.tool_name, "FileOperator");
        assert_eq!(p.call_id, "c1");
        // 原子性: 无 tmp 残留
        let left: Vec<_> = std::fs::read_dir(store.dir.as_path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(left, vec!["snap-1.json"], "不应有 tmp 残留: {left:?}");
    }

    #[test]
    fn consume_loads_and_deletes() {
        let store = ContinuationStore::new(tmp_dir("consume"));
        store.save(&snap("s2", 1)).unwrap();
        let s = store.consume("s2").unwrap();
        assert_eq!(s.id, "s2");
        assert!(!store.exists("s2"));
        assert!(store.list().is_empty());
    }

    #[test]
    fn crash_recovery_resumes_from_last_snapshot() {
        // 模拟: 进程跑第 1 轮 → save → 崩溃 → 新进程 (同目录新 store) load → 追加 → 继续
        let dir = tmp_dir("crash");
        let store1 = ContinuationStore::new(&dir);
        store1.save(&snap("s3", 1)).unwrap();
        drop(store1); // "崩溃"

        let store2 = ContinuationStore::new(&dir); // "重启"
        assert!(store2.exists("s3"));
        let mut recovered = store2.consume("s3").unwrap();
        assert_eq!(recovered.turn, 1);
        // 恢复后追加真实工具结果, 继续下一轮
        recovered.messages.push(json!({"role": "tool", "tool_call_id": "c1", "content": "写入成功"}));
        recovered.turn = 2;
        store2.save(&recovered).unwrap();
        let final_snap = store2.load("s3").unwrap();
        assert_eq!(final_snap.turn, 2);
        assert_eq!(final_snap.messages.len(), 3, "上下文应累积恢复");
        assert_eq!(final_snap.messages[2]["role"], "tool");
    }

    #[test]
    fn load_missing_returns_error() {
        let store = ContinuationStore::new(tmp_dir("missing"));
        assert!(store.load("nope").is_err());
    }
}
