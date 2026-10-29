//! `apeireth-memory::cross_diary_wiring` — 跨日记检索接线面 (Cross-Diary Retrieval Wiring).
//!
//! # 这层补的是哪个缺口
//!
//! [`crate::cross_diary`] 提供了**确定性关联算法**（日记条目 ↔ 图事实共享 token 建链），
//! 但此前**没有生产消费方** —— 日记只写不读：`apeireth dream` 把日记按日 JSON 落盘，
//! 检索主路径（[`crate::coordinator::MemoryCoordinator`]）拿不到日记面，
//! 于是「用户写下的东西永远检索不到」。
//!
//! 本模块就是那条**读取方**：
//! - **只读**：只调 [`DiaryStore::list_days`] / [`DiaryStore::read_day`]，
//!   绝不写、绝不删日记；
//! - **限量**：扫描天数、每事实片段条数、片段长度都有硬上限（见 [`CrossDiaryLimits`]）；
//! - **确定性**：排序口径固定（日期升序 → 条内序号升序 → 共享词数降序 → 共享词字典序），
//!   同一份日记 + 同一批命中事实 ⇒ 同一份上下文；
//! - **优雅降级**：日记目录不存在 / 读失败 ⇒ 返回空上下文（不报错、不编造关联）。
//!
//! # 缺省不改变既有行为
//!
//! 本模块**不自带触发点**：检索路径只有被**注入**了日记面才会产出片段。
//! 未注入时召回结果逐字节不变（`diary_snippets` 恒为空数组）。

use serde::{Deserialize, Serialize};

use crate::cross_diary::CrossDiaryIndex;
use crate::diary::DiaryStore;
use crate::layers::RecalledMemoryItem;

/// 默认最小共享 token 数 (至少 2 个共享词才算关联, 避免单字巧合)。
pub const DIARY_MIN_SHARED_TOKENS: usize = 2;

/// 单个命中事实最多附多少条日记片段.
pub const DIARY_SNIPPETS_PER_FACT_MAX: usize = 3;

/// 单条日记片段的字符上限.
pub const DIARY_SNIPPET_MAX_CHARS: usize = 120;

/// 单次召回最多扫描多少天日记.
///
/// **限量理由**: 召回发生在**每轮**对话里；全量扫描日记会让召回延迟随日记体积
/// 线性增长。只看最近若干天既够用（近况叙事），又给出确定的延迟上界。
pub const DIARY_SCAN_MAX_DAYS: usize = 14;

/// 一条关联日记片段 (挂在命中事实上).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiarySnippet {
    /// 命中事实 ID (召回条目的 `id`).
    pub fact_id: String,
    /// 日记页日期 (YYYY-MM-DD).
    pub diary_date: String,
    /// 日内条目索引.
    pub diary_entry_idx: usize,
    /// 共享 token 列表 (可解释审计证据, 升序去重).
    pub shared_tokens: Vec<String>,
    /// 日记条目正文摘要 (有界截断).
    pub snippet: String,
}

/// 跨日记检索的限量口径.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossDiaryLimits {
    /// 判定关联所需的最小共享 token 数.
    pub min_shared_tokens: usize,
    /// 单个事实最多附多少条片段.
    pub max_snippets_per_fact: usize,
    /// 单条片段的字符上限.
    pub max_snippet_chars: usize,
    /// 单次最多扫描多少天日记.
    pub max_scan_days: usize,
}

impl Default for CrossDiaryLimits {
    fn default() -> Self {
        Self {
            min_shared_tokens: DIARY_MIN_SHARED_TOKENS,
            max_snippets_per_fact: DIARY_SNIPPETS_PER_FACT_MAX,
            max_snippet_chars: DIARY_SNIPPET_MAX_CHARS,
            max_scan_days: DIARY_SCAN_MAX_DAYS,
        }
    }
}

/// 一批命中事实的跨日记关联上下文.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossDiaryContext {
    /// 全部关联片段 (确定性顺序: 事实输入顺序 → 日期升序 → 条内序号升序).
    pub snippets: Vec<DiarySnippet>,
}

impl CrossDiaryContext {
    /// 上下文是否为空 (无关联 = 不产出任何片段).
    pub fn is_empty(&self) -> bool {
        self.snippets.is_empty()
    }

    /// 取某个命中事实的全部关联片段 (保持上下文内的确定性顺序).
    pub fn snippets_for(&self, fact_id: &str) -> Vec<&DiarySnippet> {
        self.snippets
            .iter()
            .filter(|snippet| snippet.fact_id == fact_id)
            .collect()
    }
}

/// 为一批命中事实构建跨日记关联上下文 (只读, 限量, 确定性).
///
/// `facts` 是 `(事实 ID, 事实文本)` 对；事实文本参与共享 token 匹配。
///
/// **降级语义**: 日记面读失败 (IO / JSON / 锁中毒) 或没有日记时返回**空上下文** ——
/// 检索路径不因日记不可用而失败，也**不编造**任何关联。
pub fn build_cross_diary_context(
    diary: &dyn DiaryStore,
    facts: &[(String, String)],
    limits: CrossDiaryLimits,
) -> CrossDiaryContext {
    if facts.is_empty() || limits.max_snippets_per_fact == 0 {
        return CrossDiaryContext::default();
    }

    let Ok(index) =
        CrossDiaryIndex::build_recent(diary, facts, limits.min_shared_tokens, limits.max_scan_days)
    else {
        return CrossDiaryContext::default();
    };

    let mut snippets = Vec::new();
    for (fact_id, _) in facts {
        let mut links = index.links_for_fact(fact_id);
        // 确定性排序: 日期升序 → 条内序号升序 → 共享词数降序 → 共享词字典序。
        links.sort_by(|left, right| {
            left.diary_date
                .cmp(&right.diary_date)
                .then(left.diary_entry_idx.cmp(&right.diary_entry_idx))
                .then(right.shared_tokens.len().cmp(&left.shared_tokens.len()))
                .then(left.shared_tokens.cmp(&right.shared_tokens))
        });
        for link in links.into_iter().take(limits.max_snippets_per_fact) {
            snippets.push(DiarySnippet {
                fact_id: fact_id.clone(),
                diary_date: link.diary_date.clone(),
                diary_entry_idx: link.diary_entry_idx,
                shared_tokens: link.shared_tokens.clone(),
                snippet: link
                    .snippet
                    .chars()
                    .take(limits.max_snippet_chars)
                    .collect(),
            });
        }
    }

    CrossDiaryContext { snippets }
}

/// 把跨日记片段挂到命中事实上 (就地写回召回条目).
///
/// 未命中的条目保持空片段列表 —— 缺省 (未注入日记面) 时本函数不被调用，
/// 召回结果逐字节不变。
pub fn attach_diary_snippets(
    items: &mut [RecalledMemoryItem],
    diary: &dyn DiaryStore,
    limits: CrossDiaryLimits,
) -> CrossDiaryContext {
    if items.is_empty() {
        return CrossDiaryContext::default();
    }

    let facts: Vec<(String, String)> = items
        .iter()
        .map(|item| (item.id.clone(), item.content.clone()))
        .collect();
    let context = build_cross_diary_context(diary, &facts, limits);
    if context.is_empty() {
        return context;
    }

    for item in items.iter_mut() {
        item.diary_snippets = context
            .snippets_for(&item.id)
            .into_iter()
            .cloned()
            .collect();
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diary::{DiaryEntry, InMemoryDiaryStore};

    fn store_with_two_days() -> InMemoryDiaryStore {
        let store = InMemoryDiaryStore::new();
        store
            .append(
                "2026-08-28",
                DiaryEntry::new("user", "今天主人教我写 Rust 所有权", 1_000),
            )
            .unwrap();
        store
            .append(
                "2026-08-29",
                DiaryEntry::new("reflection", "今天主人提到 Rust 生命周期与所有权", 2_000),
            )
            .unwrap();
        store
    }

    #[test]
    fn matched_fact_carries_bounded_diary_snippets() {
        let store = store_with_two_days();
        let facts = vec![("ep-1".to_string(), "Rust 所有权 编译 安全".to_string())];
        let context = build_cross_diary_context(&store, &facts, CrossDiaryLimits::default());
        assert!(!context.is_empty());
        let snippets = context.snippets_for("ep-1");
        assert!(!snippets.is_empty());
        assert!(snippets.iter().all(|s| s.fact_id == "ep-1"));
        assert!(snippets
            .iter()
            .all(|s| s.snippet.chars().count() <= DIARY_SNIPPET_MAX_CHARS));
        // 确定性: 同一输入两次构建结果一致。
        let again = build_cross_diary_context(&store, &facts, CrossDiaryLimits::default());
        assert_eq!(context, again);
    }

    #[test]
    fn unrelated_fact_gets_no_snippet() {
        let store = store_with_two_days();
        let facts = vec![("ep-x".to_string(), "完全无关的量子色动力学".to_string())];
        let context = build_cross_diary_context(&store, &facts, CrossDiaryLimits::default());
        assert!(context.is_empty());
    }

    #[test]
    fn missing_diary_degrades_to_empty_context() {
        let store = InMemoryDiaryStore::new();
        let facts = vec![("ep-1".to_string(), "Rust 所有权".to_string())];
        let context = build_cross_diary_context(&store, &facts, CrossDiaryLimits::default());
        assert!(context.is_empty());
    }

    #[test]
    fn scan_day_limit_bounds_the_read_window() {
        let store = InMemoryDiaryStore::new();
        store
            .append(
                "2026-08-01",
                DiaryEntry::new("user", "Rust 所有权 早期记录", 1),
            )
            .unwrap();
        store
            .append(
                "2026-08-29",
                DiaryEntry::new("user", "Rust 所有权 近期记录", 2),
            )
            .unwrap();
        let facts = vec![("ep-1".to_string(), "Rust 所有权".to_string())];

        let limited = CrossDiaryLimits {
            max_scan_days: 1,
            ..CrossDiaryLimits::default()
        };
        let context = build_cross_diary_context(&store, &facts, limited);
        let dates: Vec<&str> = context
            .snippets_for("ep-1")
            .iter()
            .map(|s| s.diary_date.as_str())
            .collect();
        assert_eq!(dates, vec!["2026-08-29"]);
    }

    #[test]
    fn snippet_count_per_fact_is_capped() {
        let store = InMemoryDiaryStore::new();
        for day in ["2026-08-20", "2026-08-21", "2026-08-22", "2026-08-23"] {
            store
                .append(day, DiaryEntry::new("user", "Rust 所有权 记录", 1))
                .unwrap();
        }
        let facts = vec![("ep-1".to_string(), "Rust 所有权".to_string())];
        let limited = CrossDiaryLimits {
            max_snippets_per_fact: 2,
            ..CrossDiaryLimits::default()
        };
        let context = build_cross_diary_context(&store, &facts, limited);
        assert_eq!(context.snippets_for("ep-1").len(), 2);
    }
}
