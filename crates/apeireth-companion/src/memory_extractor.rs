//! `apeireth-companion::memory_extractor` — 通用记忆提炼器 (自动捕获, 替代"加无数机制").
//!
//! 主人洞察 (2026-08-16): 「让它主动存审美偏好不能靠加一个机制又一个机制;
//! 要让它自动智能地捕获记忆, 或在反思期/做梦期自己提炼出关键信息。」
//!
//! 本模块 = **一个通用机制覆盖一切捕获**:
//! - 提炼维度: facts (事实) / preferences (偏好, 含审美/风格/语气) /
//!   commitments (约定承诺) / emotional (情绪信号)
//! - 触发: 对话后节流提炼 + 做梦期批量提炼 (serve 接线)
//! - 写入: facts/commitments → episodes (mem-ex-*); preferences → pref-* 偏好库
//! - 注入: preference_injection() 生成「主人偏好画像」注入对话 — 偏好自动跨场景应用
//!
//! 0 假装: lib 只定义 trait + 写入/注入 (无 LLM 依赖, 同做梦摘要的 trait 策略);
//! LLM 实现由调用方注入 (serve 的 MiniMax 版)。

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 一次提炼的结果 (LLM 输出, JSON 对齐).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractedMemory {
    /// 值得长期记住的事实.
    #[serde(default)]
    pub facts: Vec<String>,
    /// 主人偏好 (审美/风格/语气/交互等) — 写入偏好库, 跨场景自动应用.
    #[serde(default)]
    pub preferences: Vec<String>,
    /// 约定/承诺 (主人与 AI 之间的约定, 时间敏感事项).
    #[serde(default)]
    pub commitments: Vec<String>,
    /// 情绪信号 (一句, 供后续关怀/节律参考).
    #[serde(default)]
    pub emotional: Option<String>,
}

impl ExtractedMemory {
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.preferences.is_empty() && self.commitments.is_empty() && self.emotional.is_none()
    }
}

/// 提炼器 trait (LLM 实现由调用方注入; lib 无 LLM 依赖).
#[async_trait::async_trait]
pub trait MemoryExtractor: Send + Sync {
    /// 输入上下文 (最近对话/记忆文本), 返回结构化提炼.
    async fn extract(&self, context: &str) -> Result<ExtractedMemory, String>;
}

/// 提炼服务: 写入 + 偏好库 + 注入 (纯机制, 无 LLM).
pub struct MemoryExtractionService {
    store: Arc<SqliteMemoryStore>,
}

impl MemoryExtractionService {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }

    /// 把提炼结果静默写入 (facts/commitments → mem-ex-*, preferences → pref-*).
    pub fn apply(&self, ex: &ExtractedMemory) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        for f in &ex.facts {
            if !f.trim().is_empty() {
                self.put(format!("mem-ex-{}", uuid::Uuid::new_v4()), now, f.trim())?;
            }
        }
        for c in &ex.commitments {
            if !c.trim().is_empty() {
                self.put(format!("mem-ex-{}", uuid::Uuid::new_v4()), now, &format!("【约定】{c}"))?;
            }
        }
        for p in &ex.preferences {
            if !p.trim().is_empty() {
                self.put(format!("pref-{}", uuid::Uuid::new_v4()), now, &format!("主人偏好: {p}"))?;
            }
        }
        if let Some(e) = &ex.emotional {
            if !e.trim().is_empty() {
                self.put(format!("mem-ex-{}", uuid::Uuid::new_v4()), now, &format!("【情绪信号】{e}"))?;
            }
        }
        Ok(())
    }

    /// 静默写入一条 (append-only).
    fn put(&self, id: String, ts: i64, content: &str) -> Result<(), String> {
        let ep = CoreEpisode {
            id,
            timestamp: ts,
            role: "assistant".into(),
            content: content.to_string(),
            session_id: "me".into(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    /// 偏好库 → 「主人偏好画像」注入块 (跨场景自动应用的核心).
    pub fn preference_injection(&self) -> String {
        let eps = self.store.recent_episodes("me", 300).unwrap_or_default();
        let prefs: Vec<String> = eps
            .iter()
            .filter(|e| e.id.starts_with("pref-"))
            .map(|e| e.content.clone())
            .collect();
        if prefs.is_empty() {
            return String::new();
        }
        let mut s = String::from("【主人偏好画像】(来自记忆提炼, 做审美/风格/交互类事情时优先沿用):\n");
        for p in prefs.iter().rev().take(8) {
            s.push_str(&format!("  • {}\n", p.chars().take(120).collect::<String>()));
        }
        s
    }

    /// 提炼输入: 最近对话/记忆拼接 (供 LLM 提炼器).
    pub fn recent_context(&self, n: usize) -> String {
        let eps = self.store.recent_episodes("me", n.max(10)).unwrap_or_default();
        let mut parts: Vec<String> = Vec::new();
        for e in eps.iter().rev().take(n) {
            parts.push(format!("[{}] {}", e.role, e.content.chars().take(300).collect::<String>()));
        }
        parts.join("\n")
    }

    /// 提炼结果计数 (诊断).
    pub fn counts(&self) -> Value {
        let eps = self.store.recent_episodes("me", 500).unwrap_or_default();
        json!({
            "mem_ex": eps.iter().filter(|e| e.id.starts_with("mem-ex-")).count(),
            "prefs": eps.iter().filter(|e| e.id.starts_with("pref-")).count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn apply_writes_and_injects_preferences() {
        let s = MemoryExtractionService::new(store());
        let ex = ExtractedMemory {
            facts: vec!["主人周五考高数期中".into()],
            preferences: vec!["唯美写意风格, 深蓝夜空配色".into(), "古风韵味".into()],
            commitments: vec!["周六上午整理错题本".into()],
            emotional: Some("今天有点累但心情平静".into()),
        };
        s.apply(&ex).unwrap();
        // 偏好注入
        let inj = s.preference_injection();
        assert!(inj.contains("唯美写意"), "{inj}");
        assert!(inj.contains("古风"));
        // 计数
        let c = s.counts();
        assert_eq!(c["mem_ex"], json!(3)); // 2 facts/commitments + 1 emotional
        assert_eq!(c["prefs"], json!(2));
        // 空提炼 → 偏好注入空
        let s2 = MemoryExtractionService::new(store());
        assert!(s2.preference_injection().is_empty());
    }

    #[test]
    fn recent_context_has_roles() {
        let s = MemoryExtractionService::new(store());
        let ex = ExtractedMemory {
            facts: vec!["x".into()],
            preferences: vec![],
            commitments: vec![],
            emotional: None,
        };
        s.apply(&ex).unwrap();
        let ctx = s.recent_context(5);
        assert!(ctx.contains("assistant"));
    }
}
