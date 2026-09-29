//! 会话切分 (协议 §2.1)。
//!
//! `LocomoSource` 的 `docs_from_sessions` 是私有函数, 未暴露「会话 → dia_id」映射,
//! 故按协议在 research/b1 内实现。关键事实 (实测): locomo10.json 的 10 个顶层元素 =
//! 10 个会话 (分析单元), 但**同一 dia_id 会跨会话重复** (如 D1:1 同时出现在会话 0/1 的
//! session_N 快照里)。因此「会话归属」按**全局首次出现**判定 —— 与 research/runners
//! `docs_from_sessions` 的全局去重语义一致 —— 从而满足自检「每会话唯一轮次数之和 = 1033」。
//!
//! 结构说明 (对齐 research/runners 的 LocomoSession):
//!   - 每个顶层会话的 `conversation` 含 `speaker_a` / `speaker_b` /
//!     `session_N_date_time` / `session_N` (快照重复携带历史轮次);
//!   - `qa` 为该会话的全部 QA (evidence = 引用的 dia_id)。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// 会话内一个去重后的轮次。
#[derive(Debug, Clone)]
pub struct SessionTurn {
    pub dia_id: String,
    pub speaker: String,
    pub text: String,
    pub date: String,
    /// 全局首次出现序号 (0-based, 新者更大), 与 LocomoSource::docs() 的 created_turn 对齐。
    pub created_turn: usize,
}

/// 一个顶层会话的切分结果。
#[derive(Debug, Clone)]
pub struct SessionSlice {
    pub index: usize,
    /// 会话内全部轮次条目数 (含 session_N 快照重复历史)。
    pub n_raw: usize,
    /// 去重后、归属本会话的唯一轮次 (全局首次出现)。
    pub turns: Vec<SessionTurn>,
    /// 本会话 QA 的 evidence → dia_id 引用计数 (activation_energy 分子, 协议 §2.2)。
    pub evidence_citations: HashMap<String, usize>,
}

impl SessionSlice {
    /// 归属本会话的唯一轮次数 n_s。
    pub fn n_kept(&self) -> usize {
        self.turns.len()
    }
}

/// 从文件切分会话。
pub fn split_sessions(path: &Path) -> Vec<SessionSlice> {
    let text = fs::read_to_string(path).expect("read locomo10.json");
    split_sessions_str(&text)
}

/// 从 JSON 文本切分会话 (测试用 fixture 入口)。
pub fn split_sessions_str(text: &str) -> Vec<SessionSlice> {
    let sessions: Vec<serde_json::Value> =
        serde_json::from_str(text).expect("parse locomo10.json as JSON array");
    let mut seen: HashSet<String> = HashSet::new();
    sessions
        .into_iter()
        .enumerate()
        .map(|(idx, s)| split_one(idx, &s, &mut seen))
        .collect()
}

fn split_one(index: usize, session: &serde_json::Value, seen: &mut HashSet<String>) -> SessionSlice {
    let conversation = session.get("conversation").and_then(|v| v.as_object());
    let qa = session.get("qa").and_then(|v| v.as_array());

    let mut n_raw = 0usize;
    let mut turns: Vec<SessionTurn> = Vec::new();

    if let Some(obj) = conversation {
        let mut keys: Vec<&String> = obj
            .keys()
            .filter(|k| k.starts_with("session_") && !k.ends_with("date_time"))
            .collect();
        keys.sort_by_key(|k| k.trim_start_matches("session_").parse::<u64>().unwrap_or(0));
        for k in keys {
            let date = obj
                .get(&format!("{k}_date_time"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(arr) = obj.get(k).and_then(|v| v.as_array()) {
                for t in arr {
                    n_raw += 1;
                    let dia_id = t.get("dia_id").and_then(|v| v.as_str()).unwrap_or("");
                    if dia_id.is_empty() {
                        continue;
                    }
                    // 全局首次出现才归属本会话 (跨会话重复的 dia_id 只归属第一次出现的会话)。
                    if seen.insert(dia_id.to_string()) {
                        let speaker = t
                            .get("speaker")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let text = t
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        turns.push(SessionTurn {
                            dia_id: dia_id.to_string(),
                            speaker,
                            text,
                            date: date.clone(),
                            created_turn: seen.len() - 1,
                        });
                    }
                }
            }
        }
    }

    // evidence_citation_count = 本会话内 evidence 包含此 dia_id 的 QA 条数。
    // 同一 QA 内重复引用同一 dia_id 只计 1 条。
    let mut evidence_citations: HashMap<String, usize> = HashMap::new();
    if let Some(qa_arr) = qa {
        for q in qa_arr {
            if let Some(ev) = q.get("evidence").and_then(|v| v.as_array()) {
                let mut counted: HashSet<String> = HashSet::new();
                for e in ev {
                    if let Some(id) = e.as_str() {
                        if counted.insert(id.to_string()) {
                            *evidence_citations.entry(id.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    SessionSlice {
        index,
        n_raw,
        turns,
        evidence_citations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 小 fixture: 1 个顶层会话, 2 个快照, 快照 2 重复携带历史轮次 + 新增 1 轮。
    const FIXTURE: &str = r#"[
      {
        "conversation": {
          "speaker_a": "Alice",
          "speaker_b": "Bob",
          "session_1_date_time": "2026-01-01",
          "session_1": [
            {"dia_id": "D1:1", "speaker": "Alice", "text": "hi"},
            {"dia_id": "D1:2", "speaker": "Bob", "text": "hello"}
          ],
          "session_2_date_time": "2026-01-01",
          "session_2": [
            {"dia_id": "D1:1", "speaker": "Alice", "text": "hi"},
            {"dia_id": "D1:2", "speaker": "Bob", "text": "hello"},
            {"dia_id": "D1:3", "speaker": "Alice", "text": "question?"}
          ]
        },
        "qa": [
          {"question": "q1", "evidence": ["D1:1", "D1:3"], "answer": null}
        ]
      }
    ]"#;

    #[test]
    fn session_split_dedups_and_counts_evidence() {
        let slices = split_sessions_str(FIXTURE);
        assert_eq!(slices.len(), 1);
        let s = &slices[0];
        // 5 条原始条目 (2 + 3), 去重后 3 个唯一 dia_id。
        assert_eq!(s.n_raw, 5);
        assert_eq!(s.n_kept(), 3);
        assert_eq!(
            s.turns.iter().map(|t| t.dia_id.as_str()).collect::<Vec<_>>(),
            vec!["D1:1", "D1:2", "D1:3"]
        );
        // D1:1 被 1 条 QA 引用; D1:2 未被引用。
        assert_eq!(s.evidence_citations.get("D1:1"), Some(&1));
        assert_eq!(s.evidence_citations.get("D1:2"), None);
        // 全局首次出现序号 0/1/2。
        assert_eq!(s.turns[2].created_turn, 2);
    }

    #[test]
    fn session_split_assigns_cross_session_duplicate_to_first_appearance() {
        // 会话 1 重复携带 A:2 (已在会话 0 出现), 并新增 B:1。
        let two = r#"[
          {"conversation": {"session_1": [
            {"dia_id":"A:1","speaker":"x","text":"t"},
            {"dia_id":"A:2","speaker":"x","text":"t"}
          ]}, "qa": []},
          {"conversation": {"session_1": [
            {"dia_id":"A:2","speaker":"x","text":"t"},
            {"dia_id":"B:1","speaker":"y","text":"t"}
          ]}, "qa": []}
        ]"#;
        let slices = split_sessions_str(two);
        assert_eq!(slices.len(), 2);
        // 会话 0: A:1 + A:2; 会话 1: 仅 B:1 (A:2 已归属会话 0)。
        assert_eq!(slices[0].n_kept(), 2);
        assert_eq!(slices[1].n_kept(), 1);
        assert_eq!(slices[1].n_raw, 2);
        // 自检: 每会话唯一轮次之和 = 全局唯一 = 3。
        assert_eq!(slices.iter().map(|s| s.n_kept()).sum::<usize>(), 3);
        // 会话 0 内 A:2 的全局序号 = 1。
        assert_eq!(slices[0].turns[1].created_turn, 1);
    }
}
