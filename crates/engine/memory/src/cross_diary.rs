//! `apeireth-memory::cross_diary` — 跨日记与记忆图关联索引 (Cross-Diary Index).
//!
//! **设计哲学 (日记叙事与图谱事实的双向连通)**:
//! - **① 双向确定性索引**: 在日记条目 (`DiaryEntry`) 与图事实 (`fact_id`) 之间建立共享 token 关联；
//! - **② 纯确定性 Safe Rust 分词**: 复用 `crate::hybrid_search::tokenize` 进行 ASCII 单词与 CJK 滑窗切分，0 外部 NLP 依赖；
//! - **③ 可审计证据 (`shared_tokens`)**: 每一条跨域关联均携带匹配到的 shared token 列表作为溯源证据；
//! - **④ 上下文抽取 (`render_cross_context`)**: 提供字符预算受控的跨域关联片段渲染。

use serde::{Deserialize, Serialize};

use crate::diary::{DiaryError, DiaryStore};
use crate::hybrid_search::tokenize;

/// 关联片段展示字符上限.
pub const SNIPPET_MAX_CHARS: usize = 120;

/// 一条跨域关联记录.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossLink {
    /// 记忆图事实 ID
    pub fact_id: String,
    /// 日记页日期 (YYYY-MM-DD)
    pub diary_date: String,
    /// 日内条目索引
    pub diary_entry_idx: usize,
    /// 共享 token 列表 (作为可解释审计证据，升序去重)
    pub shared_tokens: Vec<String>,
    /// 日记条目正文摘要 (有界截断)
    pub snippet: String,
}

/// 跨日记关联索引.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossDiaryIndex {
    /// 全量关联记录列表 (按日期与条目顺序排列)
    pub links: Vec<CrossLink>,
}

/// 跨日记与图事实关联核心算法 (纯函数，0 外部副作用).
pub fn link_core(
    diary_items: &[(String, usize, String)],
    fact_items: &[(String, String)],
    min_shared: usize,
) -> Vec<CrossLink> {
    let mut links = Vec::new();
    for (date, idx, body) in diary_items {
        let body_tokens = tokenize(body);
        if body_tokens.is_empty() {
            continue;
        }
        for (fact_id, fact_text) in fact_items {
            let fact_tokens = tokenize(fact_text);
            let mut shared: Vec<String> = body_tokens
                .iter()
                .filter(|t| fact_tokens.contains(t))
                .cloned()
                .collect();
            shared.sort();
            shared.dedup();
            if shared.len() >= min_shared {
                links.push(CrossLink {
                    fact_id: fact_id.clone(),
                    diary_date: date.clone(),
                    diary_entry_idx: *idx,
                    shared_tokens: shared,
                    snippet: body.chars().take(SNIPPET_MAX_CHARS).collect(),
                });
            }
        }
    }
    links
}

impl CrossDiaryIndex {
    /// 从 `DiaryStore` 与图事实列表构建索引.
    pub fn build(
        diary: &dyn DiaryStore,
        fact_items: &[(String, String)],
        min_shared: usize,
    ) -> Result<Self, DiaryError> {
        let days = diary.list_days()?;
        let mut diary_items = Vec::new();
        for date in days {
            let page = diary.read_day(&date)?;
            for (idx, entry) in page.entries.iter().enumerate() {
                diary_items.push((page.date.clone(), idx, entry.body.clone()));
            }
        }
        let links = link_core(&diary_items, fact_items, min_shared);
        Ok(Self { links })
    }

    /// 查询特定事实关联的全部日记片段.
    pub fn links_for_fact(&self, fact_id: &str) -> Vec<&CrossLink> {
        self.links
            .iter()
            .filter(|link| link.fact_id == fact_id)
            .collect()
    }

    /// 查询特定日记条目关联的全部事实.
    pub fn links_for_diary(&self, date: &str, entry_idx: usize) -> Vec<&CrossLink> {
        self.links
            .iter()
            .filter(|link| link.diary_date == date && link.diary_entry_idx == entry_idx)
            .collect()
    }

    /// 为指定事实渲染关联日记上下文.
    pub fn render_cross_context(&self, fact_id: &str, budget_chars: usize) -> Option<String> {
        let matches = self.links_for_fact(fact_id);
        if matches.is_empty() || budget_chars == 0 {
            return None;
        }

        let mut lines = Vec::new();
        lines.push(format!("【事实关联日记 · {}】", fact_id));
        let mut current_chars = lines[0].chars().count();

        for link in matches {
            let item = format!(
                "- [{}] (匹配词: {}): {}",
                link.diary_date,
                link.shared_tokens.join(", "),
                link.snippet
            );
            let item_chars = item.chars().count();
            if current_chars + item_chars + 1 > budget_chars {
                lines.push("…(已截断)".to_string());
                break;
            }
            lines.push(item);
            current_chars += item_chars + 1;
        }

        if lines.len() <= 1 {
            None
        } else {
            Some(lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diary::{DiaryEntry, InMemoryDiaryStore};

    #[test]
    fn cross_diary_linking_and_query() {
        let store = InMemoryDiaryStore::new();
        store
            .append(
                "2026-08-28",
                DiaryEntry::new("user", "今天研究了 Rust 语言所有权与类型系统", 1000),
            )
            .unwrap();
        store
            .append(
                "2026-08-29",
                DiaryEntry::new("user", "今天学习了 Python 脚本", 2000),
            )
            .unwrap();

        let facts = vec![
            (
                "fact-1".to_string(),
                "Rust 语言 编译 安全 所有权".to_string(),
            ),
            (
                "fact-2".to_string(),
                "Python 脚本 解释器 动态类型".to_string(),
            ),
        ];

        let index = CrossDiaryIndex::build(&store, &facts, 2).unwrap();
        assert_eq!(index.links.len(), 2);

        let rust_links = index.links_for_fact("fact-1");
        assert_eq!(rust_links.len(), 1);
        assert_eq!(rust_links[0].diary_date, "2026-08-28");
        assert!(rust_links[0].shared_tokens.contains(&"rust".to_string()));

        let context = index.render_cross_context("fact-1", 200).unwrap();
        assert!(context.contains("【事实关联日记 · fact-1】"));
        assert!(context.contains("2026-08-28"));
    }
}
