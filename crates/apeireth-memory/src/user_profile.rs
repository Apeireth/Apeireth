//! 用户画像提取 (UserProfile) — apeireth-memory 内部统计画像路径
//!
//! ## 现状 (2026-08 审计): 仍是有效路径, 但已不是"偏好注入"主路径
//! - **仍是有效路径**: `UserProfile` / `ProfileExtractor` 是 apeireth-memory
//!   公开 API 的一部分 (`extract_user_profile`), 由 `semantic.rs` /
//!   `semantic_persist.rs` 实现, 并有 e2e 集成测试覆盖
//!   (tests/semantic_pipeline_e2e.rs).
//! - **已被取代的角色**: 对话偏好注入已由 `apeireth-companion` 的偏好库接管
//!   (`memory_extractor::preference_injection()` + `pref-*` episodes,
//!   见 crates/apeireth-companion/src/memory_extractor.rs)。全 workspace grep
//!   显示**没有任何外部 crate** 引用 `UserProfile` / `ProfileExtractor` /
//!   `extract_user_profile` — 偏好注入不走本模块。
//! - 结论: 本模块保留为 memory 层的"画像统计/提取"能力 (mock 提取 + 真 LLM 留接口),
//!   调用方不应再把它当作"用户偏好唯一来源"。
//!
//! ## 设计
//! - `UserProfile`: 用户画像数据结构 (preferences / recurring_topics /
//!   communication_style / expertise_areas / interaction_count / last_active)
//! - `ProfileExtractor`: 从 `SqliteMemoryStore` + `SemanticIndex` 提取画像
//!   - **mock 路径**: 用 hash embedder 聚类 + role 分布 + 关键词提取
//!   - **真 LLM 路径**: 留接口 — 调用方实现自己的 `EmbedFn`, 内部走
//!     `apeireth_api::llm::LlmProvider` 拿更深层画像 (R21+ 续接)

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_core::Episode;
use serde::{Deserialize, Serialize};

use crate::episode::EpisodeStore;
use crate::semantic::{EmbedFn, SemanticIndex};
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 用户画像数据结构 (memory 层统计画像).
///
/// R19 P2 mock 提取; 真实 LLM 集成留接口.
/// 注意: `preferences` 是关键词/长度启发式推断, **不是**对话偏好注入的真实来源 —
/// 生产偏好注入走 `apeireth-companion::memory_extractor::preference_injection` (pref-*).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    /// 用户偏好 (启发式推断, e.g. "简短回答", "代码示例", "中文回复").
    /// ⚠️ 仅统计启发式: 真实偏好注入走 companion pref-* 偏好库 (见模块头).
    pub preferences: Vec<String>,
    /// 反复出现的主题 (从 episodes 内容聚类得到).
    pub recurring_topics: Vec<String>,
    /// 沟通风格 ("用户主导" / "助手主导" / "混合" / 其他).
    pub communication_style: String,
    /// 擅长领域 (从内容关键词识别).
    pub expertise_areas: Vec<String>,
    /// 总交互次数 (= episodes 数量).
    pub interaction_count: usize,
    /// 最后活跃时间 (epoch seconds, None 表示从未活跃).
    pub last_active: Option<i64>,
}

impl UserProfile {
    /// 构造一个"空"画像 (0 episodes 时使用).
    pub fn empty() -> Self {
        Self {
            preferences: Vec::new(),
            recurring_topics: Vec::new(),
            communication_style: "未知".to_string(),
            expertise_areas: Vec::new(),
            interaction_count: 0,
            last_active: None,
        }
    }
}

/// 用户画像提取器 (memory 层统计画像, 非偏好注入主路径 — 见模块头).
///
/// ## 构造
/// - `ProfileExtractor::new(embedder)`: 给定 embedder, 后续可多次 `extract`
/// - 内部维护 top_k (默认 5) 决定"代表性 episode"的数量
pub struct ProfileExtractor {
    embedder: Arc<dyn EmbedFn>,
    top_k: usize,
}

impl ProfileEmbedder for ProfileExtractor {
    fn embedder(&self) -> &Arc<dyn EmbedFn> {
        &self.embedder
    }
}

impl ProfileExtractor {
    /// 构造一个提取器.
    pub fn new(embedder: Arc<dyn EmbedFn>) -> Self {
        Self { embedder, top_k: 5 }
    }

    /// 链式: 设置 top_k (代表性 episode 数量, 默认 5).
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k.max(1);
        self
    }

    /// 从 memory + (可选) index 提取 UserProfile.
    ///
    /// `index` 可以是 `None`, 此时只用 episodes 做 role 分布 + 关键词 (弱版).
    pub fn extract(
        &self,
        memory: &SqliteMemoryStore,
        index: Option<&SemanticIndex<'_>>,
    ) -> MemoryResult<UserProfile> {
        // 1. 拉所有 episodes (limit 100_000 上限, 防 OOM)
        let eps = <SqliteMemoryStore as EpisodeStore>::query(
            memory,
            &crate::episode::EpisodeQuery::new().limit(100_000),
        )?;
        if eps.is_empty() {
            return Ok(UserProfile::empty());
        }

        // 2. 基础统计
        let interaction_count = eps.len();
        let last_active = eps.iter().map(|e| e.timestamp).max();

        // 3. role 分布 → communication_style
        let mut role_counts: HashMap<&str, usize> = HashMap::new();
        for ep in &eps {
            *role_counts.entry(ep.role.as_str()).or_insert(0) += 1;
        }
        let user_count = *role_counts.get("user").unwrap_or(&0);
        let assistant_count = *role_counts.get("assistant").unwrap_or(&0);
        let other_count: usize = role_counts
            .iter()
            .filter(|(k, _)| **k != "user" && **k != "assistant")
            .map(|(_, v)| *v)
            .sum();
        let communication_style = match (user_count, assistant_count, other_count) {
            (u, a, _o) if u + a == 0 => "未知".to_string(),
            (u, a, _) if u > a * 2 => "用户主导".to_string(),
            (u, a, _) if a > u * 2 => "助手主导".to_string(),
            (u, a, o) if o > (u + a) / 2 => format!("混合 (含 {} 条 system/tool)", o),
            _ => "混合".to_string(),
        };

        // 4. 关键词提取 (mock) → expertise_areas + preferences
        // 简单规则: content 含特定关键词 → 算 expertise
        const EXPERTISE_KEYWORDS: &[&str] = &[
            "rust",
            "python",
            "javascript",
            "typescript",
            "go",
            "java",
            "c++",
            "sql",
            "database",
            "vector",
            "embedding",
            "machine learning",
            "算法",
            "数据库",
            "向量",
            "检索",
            "嵌入",
        ];
        let mut keyword_hits: HashMap<String, usize> = HashMap::new();
        for ep in &eps {
            let lower = ep.content.to_lowercase();
            for &kw in EXPERTISE_KEYWORDS {
                if lower.contains(kw) {
                    *keyword_hits.entry(kw.to_string()).or_insert(0) += 1;
                }
            }
        }
        let mut expertise_vec: Vec<(String, usize)> = keyword_hits.into_iter().collect();
        expertise_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let expertise_areas: Vec<String> =
            expertise_vec.into_iter().take(5).map(|(k, _)| k).collect();

        // 5. recurring_topics 用 index 检索代表性 (top_k) episode 的 content 前 30 字
        //    拿"中心向量"附近的 episode, 用其 content 摘要当 topic
        //    没有 index 时退化为 "按 timestamp 取最近 top_k 条"
        let topics = if let Some(idx) = index {
            // 用自己 embedder 算一个"中心" — 用 user message 的平均向量
            let user_eps: Vec<&Episode> = eps.iter().filter(|e| e.role == "user").collect();
            if user_eps.is_empty() {
                self.recent_topics(&eps)
            } else {
                // 简化为: 直接用 idx 检索 "" 这个 query, vec0 会返 top_k 最相似 (跟所有都最近)
                // 但更稳的是: 拿"全部 user content" 拼成一个 mega string 当 query
                let combined: String = user_eps
                    .iter()
                    .map(|e| e.content.as_str())
                    .collect::<Vec<&str>>()
                    .join(" ");
                let hits = idx
                    .search(&combined, self.top_k)
                    .map_err(|e| MemoryError::Other(format!("index search for profile: {e}")))?;
                if hits.is_empty() {
                    self.recent_topics(&eps)
                } else {
                    hits.iter()
                        .map(|e| topic_snippet(&e.content))
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            }
        } else {
            self.recent_topics(&eps)
        };

        // 6. preferences: 从 user role 的 content 长度判断 (mock)
        //    平均 user content < 30 字 → "简短偏好", > 100 字 → "详细偏好"
        let avg_user_len = if user_count > 0 {
            eps.iter()
                .filter(|e| e.role == "user")
                .map(|e| e.content.chars().count())
                .sum::<usize>() as f64
                / user_count as f64
        } else {
            0.0
        };
        let preferences = if user_count == 0 {
            Vec::new()
        } else if avg_user_len < 30.0 {
            vec!["简短回答".to_string()]
        } else if avg_user_len > 100.0 {
            vec!["详细回答".to_string()]
        } else {
            vec!["平衡回答".to_string()]
        };

        Ok(UserProfile {
            preferences,
            recurring_topics: topics,
            communication_style,
            expertise_areas,
            interaction_count,
            last_active,
        })
    }

    /// 没有 index 时, 用最近 top_k 条 episode 摘要作 topic.
    fn recent_topics(&self, eps: &[Episode]) -> Vec<String> {
        let mut sorted: Vec<&Episode> = eps.iter().collect();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        sorted
            .into_iter()
            .take(self.top_k)
            .map(|e| topic_snippet(&e.content))
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// 提取 episode.content 的前 30 字 (中文友好, 不用 split whitespace).
fn topic_snippet(content: &str) -> String {
    let trimmed: String = content.chars().take(30).collect();
    trimmed.trim().to_string()
}

/// 让 embedder 可以外部注入 (trait 形式).
pub trait ProfileEmbedder {
    fn embedder(&self) -> &Arc<dyn EmbedFn>;
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::EpisodeStore;
    use crate::semantic::HashEmbedder;
    use apeireth_vector::SqliteVecBackend;
    use std::sync::Arc;

    fn fresh() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("open")
    }

    fn make_episode(id: &str, ts: i64, role: &str, content: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: ts,
            role: role.into(),
            content: content.into(),
            session_id: "s1".into(),
        }
    }

    #[test]
    fn empty_profile_for_empty_memory() {
        let mem = fresh();
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert_eq!(p.interaction_count, 0);
        assert!(p.last_active.is_none());
        assert_eq!(p.communication_style, "未知");
        assert!(p.preferences.is_empty());
    }

    #[test]
    fn user_dominant_style_when_user_msgs_much_more() {
        let mem = fresh();
        for i in 0..10 {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &mem,
                &make_episode(&format!("u{i}"), 100 + i, "user", "hi"),
            )
            .unwrap();
        }
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("a1", 200, "assistant", "hello!"),
        )
        .unwrap();
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert_eq!(p.interaction_count, 11);
        assert_eq!(p.communication_style, "用户主导");
    }

    #[test]
    fn mixed_style_when_roles_balanced() {
        let mem = fresh();
        for i in 0..3 {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &mem,
                &make_episode(&format!("u{i}"), 100 + i, "user", "hi"),
            )
            .unwrap();
        }
        for i in 0..3 {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &mem,
                &make_episode(&format!("a{i}"), 200 + i, "assistant", "hello"),
            )
            .unwrap();
        }
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert_eq!(p.interaction_count, 6);
        // 3 vs 3, 平衡, 应返 "混合"
        assert!(p.communication_style == "混合" || p.communication_style.starts_with("混合"));
    }

    #[test]
    fn expertise_extracted_from_keywords() {
        let mem = fresh();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e1", 100, "user", "I want to learn rust programming"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e2", 200, "user", "rust borrow checker is hard"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e3", 300, "user", "what about sql joins?"),
        )
        .unwrap();
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert!(p.expertise_areas.contains(&"rust".to_string()));
        assert!(p.expertise_areas.contains(&"sql".to_string()));
    }

    #[test]
    fn preferences_short_answers_when_user_msg_short() {
        let mem = fresh();
        for i in 0..5 {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &mem,
                &make_episode(&format!("u{i}"), 100 + i, "user", "ok"),
            )
            .unwrap();
        }
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert!(p.preferences.contains(&"简短回答".to_string()));
    }

    #[test]
    fn preferences_detailed_when_user_msg_long() {
        let mem = fresh();
        // 写一段 > 100 字符的长 user msg, 模拟"用户偏好详细回答"
        let long_msg = "我想深入了解一下 Rust 编程语言中的 borrow checker 是怎么工作的, 它的底层原理是什么, 以及如何在实际项目中避免常见的 use after free 错误, 以及 unsafe 代码块的最佳实践".to_string();
        assert!(
            long_msg.chars().count() > 100,
            "msg should be > 100 chars for test"
        );
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("u1", 100, "user", &long_msg),
        )
        .unwrap();
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert!(p.preferences.contains(&"详细回答".to_string()));
    }

    #[test]
    fn last_active_is_max_timestamp() {
        let mem = fresh();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e1", 100, "user", "first"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e2", 500, "user", "middle"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &mem,
            &make_episode("e3", 300, "user", "last"),
        )
        .unwrap();
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32)));
        let p = ext.extract(&mem, None).unwrap();
        assert_eq!(p.last_active, Some(500));
    }

    #[test]
    fn extract_with_index_uses_search() {
        let mem = fresh();
        let backend = SqliteVecBackend::open_in_memory().unwrap();
        let embedder: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(32));
        let idx = SemanticIndex::new(&mem, Box::new(backend), Arc::clone(&embedder));

        for i in 0..3 {
            let ep = make_episode(
                &format!("e{i}"),
                100 + i,
                "user",
                &format!("episode about topic {i}"),
            );
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            idx.index_episode(&ep).unwrap();
        }

        let ext = ProfileExtractor::new(Arc::clone(&embedder));
        let p = ext.extract(&mem, Some(&idx)).unwrap();
        assert_eq!(p.interaction_count, 3);
        // topics 应该非空 (有 index 时走搜索路径)
        assert!(!p.recurring_topics.is_empty());
    }

    #[test]
    fn with_top_k_chains_correctly() {
        let ext = ProfileExtractor::new(Arc::new(HashEmbedder::new(32))).with_top_k(10);
        assert_eq!(ext.top_k, 10);
    }
}
