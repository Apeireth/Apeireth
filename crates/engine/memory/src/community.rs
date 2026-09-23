//! `apeireth-memory::community` — 图社区分层聚合 + 双级检索分诊 (W3 移植批)。
//!
//! **移植来源**: v1 `legacy/donor/apeireth-companion/src/community.rs` (台账 M2,
//! LightRAG/GraphRAG 精神, 记忆调研批)。**0 装纪律原样移植**: 轻量确定性实现 ——
//! 不上 Leiden/外部图库; 全部排序规则显式, 复测必同; 不改 CRAWL 本体评分。
//!
//! # 三件
//!
//! 1. **社区检测** [`detect_communities`]: s/p/o 值在同一事实共现 → 无向图;
//!    连通分量 = 社区 (字典序遍历, 社区 id `comm-{i}` 稳定);
//! 2. **社区滚动摘要**: 确定性版 [`deterministic_summary`] = 社区内高频实体
//!    top-N (频次降序 → 字典序升序); 提炼调度口留 [`Summarizer`] trait 0 装
//!    (升级路径: LLM 提炼实现该 trait 即可替换, 0 装 PASS);
//! 3. **双级检索分诊** [`triage`]: 查询含实体 (s/o 值子串命中, 长度≥2) →
//!    [`Route::Entity`] (调用方持 `matched_entities` 续走实体链 CRAWL); 无实体
//!    命中 → [`Route::Broad`] (社区摘要 brief, 按社区事实数降序 → id 升序)。
//!
//! # 防重造轮子 diff 记录
//!
//! `crate::graph_algo::connected_components` 是 **Memory(Id) 域**的通用图原语;
//! 本模块面向 **GraphFact 字符串值域**的共现图 (节点 = s/p/o 值, 不同输入域),
//! 自包含确定性实现 (v1 原样), 不重复该原语。
//!
//! # 消费边界 (0 假装, 2026-10-10 实记)
//!
//! 生产图谱契约 (`apeireth_plugin::experience::KnowledgeGraphStore`) 只有
//! `facts_from(subject, limit)` **单跳读**, 无全量列举 —— 社区检测需要全集,
//! 故本批 = **移植 + 测试 (IMPLEMENTED)**; 生产消费 (接检索前置) = 契约扩展
//! (`all_facts`/subject 枚举) 之后的接线项, 已记台账 #57。

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::amem_graph::GraphFact;

/// 一个社区: members = 组成 s/p/o 值 (字典序); facts = 社区内事实 (原输入序)。
#[derive(Debug, Clone, PartialEq)]
pub struct Community {
    /// 稳定社区 id (`comm-{i}`, 按首个成员字典序分配)
    pub id: String,
    /// 社区成员 (s/p/o 值, 去重字典序)
    pub members: Vec<String>,
    /// 社区内事实 (保持原输入序)
    pub facts: Vec<GraphFact>,
}

/// 分诊路由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// 查询命中实体 → 调用方走实体链 CRAWL 方向。
    Entity,
    /// 查询无实体命中 → 走社区摘要 brief 方向。
    Broad,
}

/// 分诊结果 (双级检索的确定性路由)。
#[derive(Debug, Clone, PartialEq)]
pub struct TriageResult {
    /// 路由决策
    pub route: Route,
    /// Entity 路由的命中实体 (字典序; Broad 时空)
    pub matched_entities: Vec<String>,
    /// Broad 路由的社区摘要 brief (事实数降序 → id 升序; Entity 时空)
    pub community_briefs: Vec<String>,
}

/// 社区摘要提炼口 (0 装: 默认确定性实现; LLM 提炼为实现该 trait 即可替换)。
pub trait Summarizer: Send + Sync {
    /// 产出一个社区的摘要文本。
    fn summarize(&self, community: &Community, top_n: usize) -> String;
}

/// 确定性摘要器 (v1 原样): 社区内高频实体 top-N (频次降序 → 字典序升序)。
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicSummarizer;

impl Summarizer for DeterministicSummarizer {
    fn summarize(&self, community: &Community, top_n: usize) -> String {
        deterministic_summary(community, top_n)
    }
}

/// 社区检测 (v1 原样): s/p/o 值共现 → 无向图连通分量 = 社区。
///
/// 确定性: 成员与社区 id 均字典序稳定 (复测必同); 事实保持原输入序;
/// 空输入 → 空社区表。
pub fn detect_communities(facts: &[GraphFact]) -> Vec<Community> {
    // 值 → 所属事实下标 (共现边 = 共享任一 s/p/o 值)。
    let mut adjacency: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    for (index, fact) in facts.iter().enumerate() {
        for value in [&fact.subject, &fact.predicate, &fact.object] {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                adjacency.entry(trimmed).or_default().insert(index);
            }
        }
    }

    // 并查集聚簇 (事实下标域)。
    let mut parent: Vec<usize> = (0..facts.len()).collect();
    fn find(parent: &mut Vec<usize>, mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for indexes in adjacency.values() {
        let mut iter = indexes.iter().copied();
        if let Some(first) = iter.next() {
            for other in iter {
                let (a, b) = (find(&mut parent, first), find(&mut parent, other));
                if a != b {
                    parent[a] = b;
                }
            }
        }
    }

    // 分量 → 事实下标集合 (字典序遍历簇根)。
    let mut clusters: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..facts.len() {
        let root = find(&mut parent, index);
        clusters.entry(root).or_default().push(index);
    }

    // 社区装配: 成员 = 簇内全部 s/p/o 值 (字典序); facts 保序;
    // id 按"首个成员字典序"稳定编号 (与 v1 相同语义)。
    let mut communities: Vec<Community> = clusters
        .into_values()
        .map(|indexes| {
            let mut members: BTreeSet<&str> = BTreeSet::new();
            let mut member_facts = Vec::with_capacity(indexes.len());
            for index in indexes {
                let fact = &facts[index];
                for value in [&fact.subject, &fact.predicate, &fact.object] {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        members.insert(trimmed);
                    }
                }
                member_facts.push(fact.clone());
            }
            let members: Vec<String> = members.into_iter().map(str::to_string).collect();
            Community {
                id: String::new(),
                members,
                facts: member_facts,
            }
        })
        .collect();

    communities.sort_by(|a, b| a.members.cmp(&b.members));
    for (index, community) in communities.iter_mut().enumerate() {
        community.id = format!("comm-{index}");
    }
    communities
}

/// 确定性社区摘要 (v1 原样): 社区内**实体**高频 top-N (仅 s/o, 不含 p)。
pub fn deterministic_summary(community: &Community, top_n: usize) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for fact in &community.facts {
        for value in [&fact.subject, &fact.object] {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                *counts.entry(trimmed).or_insert(0) += 1;
            }
        }
    }
    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let top: Vec<&str> = ranked.iter().take(top_n).map(|(value, _)| *value).collect();
    format!(
        "社区 {} ({} 事实 | {} 成员): {}",
        community.id,
        community.facts.len(),
        community.members.len(),
        top.join(", ")
    )
}

/// 双级检索分诊 (v1 原样, 确定性路由规则):
/// - 查询含实体 (任一 s/o 值, 长度≥2, 为查询子串) → [`Route::Entity`];
/// - 否则 → [`Route::Broad`] (社区摘要 brief, 事实数降序 → id 升序)。
///
/// 空图/空查询安全: 空图 Broad 且 briefs 空, 不 panic。
pub fn triage(
    query: &str,
    facts: &[GraphFact],
    top_n: usize,
    max_communities: usize,
) -> TriageResult {
    let trimmed_query = query.trim();
    let mut matched: BTreeSet<String> = BTreeSet::new();
    for fact in facts {
        for value in [&fact.subject, &fact.object] {
            let trimmed = value.trim();
            if trimmed.chars().count() >= 2 && trimmed_query.contains(trimmed) {
                matched.insert(trimmed.to_string());
            }
        }
    }
    if !matched.is_empty() {
        return TriageResult {
            route: Route::Entity,
            matched_entities: matched.into_iter().collect(),
            community_briefs: Vec::new(),
        };
    }

    let communities = detect_communities(facts);
    let mut by_size: Vec<&Community> = communities.iter().collect();
    by_size.sort_by(|a, b| {
        b.facts
            .len()
            .cmp(&a.facts.len())
            .then_with(|| a.id.cmp(&b.id))
    });
    let briefs: Vec<String> = by_size
        .iter()
        .take(max_communities)
        .map(|community| deterministic_summary(community, top_n))
        .collect();
    TriageResult {
        route: Route::Broad,
        matched_entities: Vec::new(),
        community_briefs: briefs,
    }
}

#[cfg(test)]
mod community_tests {
    use super::*;

    fn fact(subject: &str, predicate: &str, object: &str) -> GraphFact {
        GraphFact {
            id: format!("{subject}-{predicate}-{object}"),
            chain: GraphFact::chain_key(subject, predicate, object),
            rev: 0,
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            valid_at: 0,
            invalid_at: None,
            importance: 5,
        }
    }

    /// 空图/空查询安全 (不 panic, Broad 空 brief)。
    #[test]
    fn empty_paths_are_safe() {
        assert!(detect_communities(&[]).is_empty());
        let broad = triage("随便问问", &[], 5, 8);
        assert_eq!(broad.route, Route::Broad);
        assert!(broad.community_briefs.is_empty());
        assert_eq!(triage("", &[], 5, 8).route, Route::Broad);
    }

    /// 共现聚簇: 不相交 → 2 社区; 共享值桥接 → 1 社区。
    #[test]
    fn clustering_disjoint_and_bridged() {
        let disjoint = vec![
            fact("小明", "喜欢", "篮球"),
            fact("服务器A", "位于", "机房1"),
        ];
        let communities = detect_communities(&disjoint);
        assert_eq!(communities.len(), 2, "不相交簇应为 2 社区");
        assert_eq!(communities[0].id, "comm-0");

        let bridged = vec![
            fact("小明", "喜欢", "篮球"),
            fact("篮球", "属于", "运动"),
            fact("服务器A", "位于", "机房1"),
            fact("机房1", "属于", "基础设施"),
        ];
        let bridged_communities = detect_communities(&bridged);
        // s/p/o **全参**共现: 篮球桥接 0↔1, 机房1 桥接 2↔3, 而共享谓词"属于"
        // 出现在 1 和 3 → 两个簇最终合并为 1 社区 (v1 原语义, 谓词即共现键)。
        assert_eq!(bridged_communities.len(), 1);
    }

    /// 确定性: 同输入复测必同 (id/members 稳定)。
    #[test]
    fn detection_is_reproducible() {
        let facts = vec![
            fact("a", "r1", "b"),
            fact("b", "r2", "c"),
            fact("x", "r3", "y"),
        ];
        assert_eq!(detect_communities(&facts), detect_communities(&facts));
    }

    /// 摘要: 高频实体 top-N, 频次降序 → 字典序升序; 只算 s/o 不算 p。
    #[test]
    fn deterministic_summary_ranks_entities() {
        let facts = vec![
            fact("小明", "喜欢", "篮球"),
            fact("小明", "打", "篮球"),
            fact("小红", "看", "篮球"),
        ];
        let communities = detect_communities(&facts);
        assert_eq!(communities.len(), 1);
        let summary = deterministic_summary(&communities[0], 2);
        // 篮球 3 次 > 小明 2 次 > 小红 1 次; top-2 = 篮球, 小明。
        assert!(summary.contains("篮球"), "{summary}");
        assert!(summary.contains("小明"), "{summary}");
        assert!(!summary.contains("小红"), "{summary}");
        assert!(summary.starts_with("社区 comm-0"), "{summary}");
    }

    /// 分诊 Entity 路由: s/o 子串命中 (长度≥2) → matched_entities 字典序。
    #[test]
    fn triage_entity_route_on_substring_hit() {
        let facts = vec![
            fact("小明", "喜欢", "篮球"),
            fact("小刚", "打", "排球"),
        ];
        let result = triage("小明在干什么", &facts, 5, 8);
        assert_eq!(result.route, Route::Entity);
        assert_eq!(result.matched_entities, vec!["小明".to_string()]);
        assert!(result.community_briefs.is_empty());
    }

    /// 单字符实体不命中 (长度≥2 护栏)。
    #[test]
    fn triage_ignores_single_char_values() {
        let facts = vec![fact("x", "r", "y")];
        let result = triage("x 和 y 的关系", &facts, 5, 8);
        assert_eq!(result.route, Route::Broad, "单字符值不应触发 Entity 路由");
    }

    /// Broad 路由: 社区按事实数降序 → id 升序; max_communities 封顶。
    #[test]
    fn triage_broad_route_orders_and_caps() {
        let facts = vec![
            fact("a", "r", "b"),
            fact("a", "r", "c"),
            fact("a", "r", "d"),
            fact("x", "r", "y"),
        ];
        let result = triage("讲讲图里都有什么", &facts, 3, 1);
        assert_eq!(result.route, Route::Broad);
        assert_eq!(result.community_briefs.len(), 1, "max_communities 封顶");
        assert!(result.community_briefs[0].contains("a"), "最大社区在前");
    }

    /// Summarizer trait 0 装升级路径: 确定性实现可替换。
    #[test]
    fn summarizer_trait_is_the_upgrade_seam() {
        let facts = vec![fact("小明", "喜欢", "篮球")];
        let communities = detect_communities(&facts);
        let via_trait = DeterministicSummarizer.summarize(&communities[0], 3);
        assert_eq!(via_trait, deterministic_summary(&communities[0], 3));
    }
}
