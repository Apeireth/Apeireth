//! **§5.1 记忆域深化包 机制④ / 跨日记关联 — diary ↔ memory_graph 联动**
//!
//! **目标**: 日记条目（diary 按日归档）与记忆图事实（memory_graph 双时态边）
//! 建立确定性关联索引, 支持双向查询: 给定记忆节点 → 相关日记片段;
//! 给定日记条目 → 相关记忆节点。§5.1 五个机制的最后一件。
//!
//! **纪律** (任务 3cd1f8c0):
//! - 自包含模块: 只通过 diary/memory_graph **已有公开接口**采集数据
//!   (`DiaryStore::list_days/read_day`, `MemoryGraph::active_facts`), 不改两模块本体
//! - **确定性关联**: 共享 token 匹配 (复用 `topic_groups::topic_tokens`,
//!   CJK bigram + 拉丁词, 停用词切分) — 0 向量 0 嵌入 0 远程
//! - 注入侧留 trait 口 ([`CrossDiaryInjector`]), 关联上下文注入延后统一接线 (0 装 PASS)
//!
//! **VCP 对照**: VCP diary 关联走嵌入相似度; 我们走确定性 token 交集,
//! 可审计 (每条关联带 shared_tokens 证据), 同输入必同输出 (有测试守)。

use crate::diary::DiaryStore;
use crate::memory_graph::MemoryGraph;
use crate::topic_groups::topic_tokens;

/// 关联片段展示上限 (与记忆证据块同口径)
const SNIPPET_MAX_CHARS: usize = 120;

/// **一条跨域关联记录** (确定性, 可审计)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossLink {
    /// 记忆图事实 id (`GraphFact::id`)
    pub fact_id: String,
    /// 日记页日期 (YYYY-MM-DD)
    pub diary_date: String,
    /// 日内条目存储序 (确定性标识, 与 DiaryStore 读取序一致)
    pub diary_entry_idx: usize,
    /// 共享 token (关联建立的审计证据, 排序去重)
    pub shared_tokens: Vec<String>,
    /// 日记条目体 ≤120 字 (查询自足, 不必回读 DiaryStore)
    pub snippet: String,
}

/// **跨日记关联索引** (构建后只读; 同输入必同输出)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossDiaryIndex {
    /// 关联记录 (顺序确定性: 日记按日期升序+日内序, 事实按图内序)
    pub links: Vec<CrossLink>,
}

/// **关联核心** (纯函数, 确定性): 日记条目 × 记忆事实文本 → 共享 token ≥ min_shared 建链
///
/// - `diary_items`: (date, entry_idx, body)
/// - `fact_items`: (fact_id, fact_text) — fact_text 建议 subject+predicate+object 拼接
pub fn link_core(
    diary_items: &[(String, usize, String)],
    fact_items: &[(String, String)],
    min_shared: usize,
) -> Vec<CrossLink> {
    let mut links = Vec::new();
    for (date, idx, body) in diary_items {
        let body_tokens = topic_tokens(body);
        if body_tokens.is_empty() {
            continue;
        }
        for (fact_id, fact_text) in fact_items {
            let fact_tokens = topic_tokens(fact_text);
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
    /// **构建关联索引** — 只经 diary/memory_graph 已有公开接口采集 (不改两模块本体)
    ///
    /// `min_shared`: 建链阈值 (共享 token 数下限; 建议 2, 防单 bigram 弱关联)
    pub fn build(diary: &DiaryStore, graph: &MemoryGraph, min_shared: usize) -> Self {
        // 日记侧: list_days (日期升序) → read_day → 条目存储序
        let mut diary_items: Vec<(String, usize, String)> = Vec::new();
        for date in diary.list_days() {
            if let Some(page) = diary.read_day(&date) {
                for (idx, entry) in page.entries.iter().enumerate() {
                    diary_items.push((page.date.clone(), idx, entry.body.clone()));
                }
            }
        }
        // 图侧: active_facts (当前有效边) → s+p+o 拼文本
        let fact_items: Vec<(String, String)> = graph
            .active_facts()
            .into_iter()
            .map(|f| (f.id, format!("{} {} {}", f.subject, f.predicate, f.object)))
            .collect();
        Self {
            links: link_core(&diary_items, &fact_items, min_shared),
        }
    }

    /// **正向查询**: 给定记忆节点 id → 相关日记片段 (关联记录序)
    pub fn diary_for_fact(&self, fact_id: &str) -> Vec<&CrossLink> {
        self.links.iter().filter(|l| l.fact_id == fact_id).collect()
    }

    /// **反向查询**: 给定日记条目 (date, entry_idx) → 相关记忆节点 id (去重保序)
    pub fn facts_for_diary(&self, date: &str, entry_idx: usize) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for l in self
            .links
            .iter()
            .filter(|l| l.diary_date == date && l.diary_entry_idx == entry_idx)
        {
            if !out.contains(&l.fact_id.as_str()) {
                out.push(&l.fact_id);
            }
        }
        out
    }
}

/// **关联上下文注入 机制口** (0 装 PASS: 统一接线延后, 届时把索引查询结果渲染进注入链)
///
/// 接线时实现本 trait: 由 seed (记忆节点内容或日记片段) 查索引 → 渲染关联块,
/// 挂 assemble.rs 注入管线 (届时一处挂接, 与 memory_block 同层)。
pub trait CrossDiaryInjector: Send + Sync {
    /// 给定种子内容 (记忆节点或日记片段), 返回关联上下文注入块
    fn inject_related(&self, seed_content: &str) -> String;
}

// ============================================================
// 测试 (关联建立 / 双向查询 / 空关联 / 确定性复测 / 真实接口集成)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> (Vec<(String, usize, String)>, Vec<(String, String)>) {
        (
            vec![
                (
                    "2026-08-15".into(),
                    0,
                    "今天复习线代, 换元法还是要多练".into(),
                ),
                ("2026-08-15".into(), 1, "晚上泡了深烘咖啡, 香气很足".into()),
                (
                    "2026-08-16".into(),
                    0,
                    "线代作业交了, 换元积分写得顺".into(),
                ),
            ],
            vec![
                ("f1".into(), "主人 擅长薄弱点 线代换元法".into()),
                ("f2".into(), "主人 偏好 深烘咖啡豆".into()),
                ("f3".into(), "量子退相干 属于 物理".into()),
            ],
        )
    }

    #[test]
    fn links_built_on_shared_tokens_with_audit() {
        let (d, f) = corpus();
        let links = link_core(&d, &f, 2);
        // 线代两条日记 ↔ f1 (线代/换元 共享); 咖啡日记 ↔ f2 (深烘/咖啡)
        assert!(links
            .iter()
            .any(|l| l.fact_id == "f1" && l.diary_date == "2026-08-15" && l.diary_entry_idx == 0));
        assert!(links
            .iter()
            .any(|l| l.fact_id == "f1" && l.diary_date == "2026-08-16"));
        assert!(links
            .iter()
            .any(|l| l.fact_id == "f2" && l.diary_entry_idx == 1));
        assert!(!links.iter().any(|l| l.fact_id == "f3"), "无共享不应建链");
        // 审计证据非空且排序去重
        let any_link = links.iter().find(|l| l.fact_id == "f1").unwrap();
        assert!(!any_link.shared_tokens.is_empty());
        let mut sorted = any_link.shared_tokens.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, any_link.shared_tokens);
    }

    #[test]
    fn bidirectional_queries() {
        let (d, f) = corpus();
        let idx = CrossDiaryIndex {
            links: link_core(&d, &f, 2),
        };
        // 正向: f1 → 两条线代日记
        let forward = idx.diary_for_fact("f1");
        assert_eq!(forward.len(), 2);
        assert!(forward[0].snippet.contains("线代"));
        // 反向: 8-15 第 0 条 → f1
        let backward = idx.facts_for_diary("2026-08-15", 0);
        assert_eq!(backward, vec!["f1"]);
        // 反向: 咖啡条目 → f2
        assert_eq!(idx.facts_for_diary("2026-08-15", 1), vec!["f2"]);
    }

    #[test]
    fn empty_sides_no_links() {
        let (_, f) = corpus();
        assert!(link_core(&[], &f, 1).is_empty(), "空日记 → 无关联");
        let (d, _) = corpus();
        assert!(link_core(&d, &[], 1).is_empty(), "空事实 → 无关联");
        let idx = CrossDiaryIndex::default();
        assert!(idx.diary_for_fact("f1").is_empty());
        assert!(idx.facts_for_diary("2026-08-15", 0).is_empty());
    }

    #[test]
    fn no_shared_tokens_no_links() {
        let d = vec![("2026-08-16".into(), 0, "攀登雪山要检查冰爪".into())];
        let f = vec![("f9".into(), "陶土窑变 讲究 釉色".into())];
        assert!(link_core(&d, &f, 1).is_empty(), "零共享 token 不建链");
    }

    #[test]
    fn min_shared_threshold_filters_weak_links() {
        let d = vec![("2026-08-16".into(), 0, "线代复习完了".into())];
        let f = vec![("f1".into(), "线代 难度 中等".into())];
        // 共享仅"线代"一个 bigram → min_shared=2 过滤
        assert!(link_core(&d, &f, 2).is_empty());
        assert_eq!(link_core(&d, &f, 1).len(), 1);
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let (d, f) = corpus();
        let a = link_core(&d, &f, 2);
        let b = link_core(&d, &f, 2);
        assert_eq!(a, b, "确定性: 同输入必同输出");
        let ia = CrossDiaryIndex { links: a };
        let ib = CrossDiaryIndex { links: b };
        assert_eq!(ia, ib);
    }

    #[test]
    fn snippet_truncated_to_120() {
        let long = "线代".to_string() + &"长".repeat(300);
        let d = vec![("2026-08-16".into(), 0, long)];
        let f = vec![("f1".into(), "线代 作业 记录".into())];
        let links = link_core(&d, &f, 1);
        assert_eq!(links.len(), 1);
        assert!(links[0].snippet.chars().count() <= 120);
    }

    #[test]
    fn build_via_public_interfaces_diary_and_graph() {
        // 集成: 真实 DiaryStore (tempdir + VirtualClock) + MemoryGraph (in-memory sqlite)
        use crate::memory_graph::MemoryGraph;
        use apeireth_core::clock::VirtualClock;
        use apeireth_memory::SqliteMemoryStore;
        use chrono::TimeZone;
        use std::sync::Arc;

        let root = std::env::temp_dir().join(format!(
            "apeireth-crossdiary-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let clock = Arc::new(VirtualClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 8, 16, 6, 0, 0)
                .single()
                .unwrap(),
        ));
        let diary = DiaryStore::new(&root, clock);
        diary
            .append("test", "今天复习线代换元法, 感觉顺手")
            .unwrap();

        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let graph = MemoryGraph::new(store);
        let fact_id = graph.add_fact("主人", "掌握进度", "线代换元法", 5);

        let idx = CrossDiaryIndex::build(&diary, &graph, 2);
        assert!(
            idx.links.iter().any(|l| l.fact_id == fact_id),
            "真实双模块公开接口应建立关联: {:?}",
            idx.links
        );
        assert_eq!(idx.facts_for_diary("2026-08-16", 0), vec![fact_id.as_str()]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
