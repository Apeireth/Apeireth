//! M2: 图社区分层聚合 + 双级检索 (LightRAG/GraphRAG 精神, 记忆调研批 ⭐).
//!
//! 背景: 台账 M2 — memory_graph 只有实体链 CRAWL 局部检索, 无社区层.
//! 轻量确定性实现纪律: 不上 Leiden/外部图库; 全部排序规则显式, 复测必同.
//! 不改 CRAWL 本体评分 (memory_graph::crawl 0 改动).
//!
//! 三件:
//! 1. 社区检测 [`detect_communities`]: s/p/o 值在同一事实共现 → 无向图;
//!    连通分量 = 社区 (字典序遍历, 社区 id `comm-{i}` 稳定).
//! 2. 社区滚动摘要: 确定性版 [`deterministic_summary`] = 社区内高频实体 top-N
//!    (频次降序 → 字典序升序); 提炼调度口留 [`Summarizer`] trait 0 装
//!    (升级路径: LLM 提炼实现该 trait 即可替换, 0 装 PASS).
//! 3. 双级检索分诊 [`triage`]: 查询含实体 (s/o 值子串命中, 长度≥2) → Entity
//!    (调用方持 matched_entities 续走实体链 CRAWL); 无实体命中 → Broad
//!    (社区摘要 brief, 按社区事实数降序). 确定性路由规则, 无随机.

use crate::memory_graph::GraphFact;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// 一个社区: members = 组成 s/p/o 值 (字典序); facts = 社区内事实 (原输入序).
#[derive(Debug, Clone)]
pub struct Community {
    pub id: String,
    pub members: Vec<String>,
    pub facts: Vec<GraphFact>,
}

/// 分诊路由.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// 具体问题 (含实体) → 实体链 CRAWL.
    Entity,
    /// 宽泛问题 (无实体命中) → 社区摘要.
    Broad,
}

/// 分诊结果.
#[derive(Debug, Clone)]
pub struct TriageResult {
    pub route: Route,
    /// Entity 路由: 命中实体值 (字典序); 调用方作为实体链 CRAWL 的种子线索.
    pub matched_entities: Vec<String>,
    /// Broad 路由: 社区摘要 brief (事实数降序 → id 升序, 截 max_communities).
    pub community_briefs: Vec<String>,
}

/// 提炼调度口 (0 装): 社区摘要的 LLM 提炼版未来实现此 trait 替换确定性版.
pub trait Summarizer {
    fn summarize(&self, c: &Community, top_n: usize) -> String;
}

/// 确定性摘要器: 高频实体 top-N (频次降序 → 字典序升序).
#[derive(Debug, Default)]
pub struct DeterministicSummarizer;

impl Summarizer for DeterministicSummarizer {
    fn summarize(&self, c: &Community, top_n: usize) -> String {
        deterministic_summary(c, top_n)
    }
}

/// 轻量确定性社区检测: s/p/o 共现聚簇 (同事实共现连边 → 连通分量).
/// 确定性: BTree 字典序遍历 → 社区 id `comm-{i}` 稳定 (comm-0 含最小节点).
pub fn detect_communities(facts: &[GraphFact]) -> Vec<Community> {
    if facts.is_empty() {
        return Vec::new();
    }
    let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut fact_of: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, f) in facts.iter().enumerate() {
        let mut vals: Vec<&String> = [&f.subject, &f.predicate, &f.object]
            .iter()
            .filter(|v| !v.trim().is_empty())
            .copied()
            .collect();
        vals.sort();
        for v in &vals {
            adj.entry((*v).clone()).or_default();
            fact_of.entry((*v).clone()).or_default().push(i);
        }
        // 共现边: 排序后相邻连接即保证同事实值全连通.
        for pair in vals.windows(2) {
            adj.entry(pair[0].clone())
                .or_default()
                .insert(pair[1].clone());
            adj.entry(pair[1].clone())
                .or_default()
                .insert(pair[0].clone());
        }
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut comms: Vec<Community> = Vec::new();
    for start in adj.keys() {
        if seen.contains(start) {
            continue;
        }
        // 连通分量 (栈式遍历, BTreeSet 收集 → 成员字典序).
        let mut comp: BTreeSet<String> = BTreeSet::new();
        let mut stack: Vec<String> = vec![start.clone()];
        while let Some(n) = stack.pop() {
            if !comp.insert(n.clone()) {
                continue;
            }
            if let Some(nbrs) = adj.get(&n) {
                for nb in nbrs {
                    if !comp.contains(nb) {
                        stack.push(nb.clone());
                    }
                }
            }
        }
        seen.extend(comp.iter().cloned());
        let mut fidx: BTreeSet<usize> = BTreeSet::new();
        for m in &comp {
            if let Some(idxs) = fact_of.get(m) {
                fidx.extend(idxs);
            }
        }
        let cfacts: Vec<GraphFact> = fidx.into_iter().map(|i| facts[i].clone()).collect();
        comms.push(Community {
            id: format!("comm-{}", comms.len()),
            members: comp.into_iter().collect(),
            facts: cfacts,
        });
    }
    comms
}

/// 确定性滚动摘要: 社区内高频实体 (subject/object) top-N + 规模信息.
pub fn deterministic_summary(c: &Community, top_n: usize) -> String {
    let mut cnt: HashMap<&str, usize> = HashMap::new();
    for f in &c.facts {
        for v in [&f.subject, &f.object] {
            if !v.trim().is_empty() {
                *cnt.entry(v.as_str()).or_insert(0) += 1;
            }
        }
    }
    let mut ranked: Vec<(&str, usize)> = cnt.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let top: Vec<&str> = ranked.iter().take(top_n).map(|(s, _)| *s).collect();
    format!(
        "社区 {} ({} 事实 | {} 成员): {}",
        c.id,
        c.facts.len(),
        c.members.len(),
        top.join(", ")
    )
}

/// 双级检索分诊 (确定性路由规则):
/// - 查询含实体 (任一 s/o 值, 长度≥2, 为查询子串) → Entity (CRAWL 方向);
/// - 否则 → Broad (社区摘要 brief, 事实数降序 → id 升序).
/// 空图/空查询安全: 空图 Broad 且 briefs 空, 不 panic.
pub fn triage(
    query: &str,
    facts: &[GraphFact],
    top_n: usize,
    max_communities: usize,
) -> TriageResult {
    let q = query.trim();
    let mut matched: BTreeSet<String> = BTreeSet::new();
    for f in facts {
        for v in [&f.subject, &f.object] {
            let vt = v.trim();
            if vt.chars().count() >= 2 && q.contains(vt) {
                matched.insert(vt.to_string());
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
    let comms = detect_communities(facts);
    let mut by_size: Vec<&Community> = comms.iter().collect();
    by_size.sort_by(|a, b| {
        b.facts
            .len()
            .cmp(&a.facts.len())
            .then_with(|| a.id.cmp(&b.id))
    });
    let briefs: Vec<String> = by_size
        .iter()
        .take(max_communities)
        .map(|c| deterministic_summary(c, top_n))
        .collect();
    TriageResult {
        route: Route::Broad,
        matched_entities: Vec::new(),
        community_briefs: briefs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(s: &str, p: &str, o: &str) -> GraphFact {
        GraphFact {
            id: format!("{s}-{p}-{o}"),
            chain: format!("{s}|{p}|{o}"),
            rev: 0,
            subject: s.into(),
            predicate: p.into(),
            object: o.into(),
            valid_at: 0,
            invalid_at: None,
            importance: 5,
        }
    }

    /// 空图: 无社区; 分诊 Broad 且 briefs 空不 panic.
    #[test]
    fn empty_graph_paths() {
        assert!(detect_communities(&[]).is_empty());
        let r = triage("随便问问", &[], 5, 8);
        assert_eq!(r.route, Route::Broad);
        assert!(r.community_briefs.is_empty());
        let r2 = triage("", &[], 5, 8);
        assert_eq!(r2.route, Route::Broad);
    }

    /// 共现聚簇: 两个不相交簇 → 2 社区; 共享值桥接 → 1 社区.
    #[test]
    fn clustering_disjoint_and_bridged() {
        let disjoint = vec![
            fact("小明", "喜欢", "篮球"),
            fact("服务器A", "位于", "机房1"),
        ];
        let comms = detect_communities(&disjoint);
        assert_eq!(comms.len(), 2, "不相交簇应为 2 社区");
        assert_eq!(comms[0].id, "comm-0");
        // 确定性: comm-0 = 含全局字典序最小节点 ("位于") 的分量 (服务器簇)
        assert!(comms[0].members.iter().any(|m| m == "位于"));
        // 社区成员集合与输入序无关的确定性检查 (按成员排序后比对)
        let mut sets: Vec<Vec<String>> = comms.iter().map(|c| c.members.clone()).collect();
        sets.sort();
        let mut expect_a = vec!["喜欢".to_string(), "小明".to_string(), "篮球".to_string()];
        let mut expect_b = vec![
            "位于".to_string(),
            "机房1".to_string(),
            "服务器A".to_string(),
        ];
        expect_a.sort();
        expect_b.sort();
        let mut expect = vec![expect_a, expect_b];
        expect.sort();
        assert_eq!(sets, expect);
        // 桥接: 新增事实连接两簇 (篮球→服务器A 同事实共现)
        let mut bridged = disjoint.clone();
        bridged.push(fact("篮球", "存放于", "服务器A"));
        let comms2 = detect_communities(&bridged);
        assert_eq!(comms2.len(), 1, "桥接后应并为 1 社区");
        assert_eq!(comms2[0].facts.len(), 3);
    }

    /// 确定性复测: 输入乱序 → 社区集合与 id 分配完全一致.
    #[test]
    fn deterministic_across_input_order() {
        let a = vec![
            fact("小明", "喜欢", "篮球"),
            fact("服务器A", "位于", "机房1"),
            fact("小明", "住在", "北京"),
        ];
        let mut b = a.clone();
        b.reverse();
        let ca = detect_communities(&a);
        let cb = detect_communities(&b);
        assert_eq!(ca.len(), cb.len());
        // 社区内容集合一致 (id 按最小成员字典序分配, 与输入序无关)
        let norm = |cs: &[Community]| -> Vec<(Vec<String>, usize)> {
            let mut v: Vec<(Vec<String>, usize)> = cs
                .iter()
                .map(|c| (c.members.clone(), c.facts.len()))
                .collect();
            v.sort();
            v
        };
        assert_eq!(norm(&ca), norm(&cb), "乱序输入必须产出相同社区划分");
        // 同一输入二次运行逐字节一致
        let ca2 = detect_communities(&a);
        let fmt = |cs: &[Community]| -> String {
            cs.iter()
                .map(|c| format!("{}:{:?}", c.id, c.members))
                .collect::<Vec<_>>()
                .join("|")
        };
        assert_eq!(fmt(&ca), fmt(&ca2));
    }

    /// 滚动摘要: 频次降序 → 字典序升序截 top-N.
    #[test]
    fn deterministic_summary_topn_order() {
        let facts = vec![
            fact("小明", "喜欢", "篮球"),
            fact("小明", "住在", "北京"),
            fact("小明", "养", "猫"),
            fact("篮球", "品牌", "耐克"),
        ];
        let comms = detect_communities(&facts);
        assert_eq!(comms.len(), 1);
        let s = deterministic_summary(&comms[0], 3);
        // 小明 出现 3 次 → 首位; 篮球 2 次 → 次位
        assert!(s.starts_with("社区 comm-0 (4 事实"), "摘要头应含规模: {s}");
        let body = s.split(": ").nth(1).unwrap();
        let top3: Vec<&str> = body.split(", ").collect();
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0], "小明");
        assert_eq!(top3[1], "篮球");
        // 第三位频次并列 → 字典序升序
        assert!(top3[2] == "北京" || top3[2] == "耐克" || top3[2] == "猫");
        // Summarizer trait 口与直接函数一致
        let ds = DeterministicSummarizer;
        assert_eq!(ds.summarize(&comms[0], 3), s);
    }

    /// 分诊路由: 含实体 → Entity + matched 字典序; 无实体 → Broad + briefs.
    #[test]
    fn triage_entity_vs_broad() {
        let facts = vec![
            fact("小明", "喜欢", "篮球"),
            fact("小明", "住在", "北京"),
            fact("服务器A", "位于", "机房1"),
        ];
        // 具体问题 (含实体): 走 Entity, 命中实体字典序
        let r = triage("小明还喜欢什么", &facts, 3, 8);
        assert_eq!(r.route, Route::Entity);
        assert_eq!(r.matched_entities, vec!["小明".to_string()]);
        assert!(r.community_briefs.is_empty());
        // 宽泛问题 (无实体): 走 Broad, briefs 按社区事实数降序
        let r2 = triage("最近有什么值得关注的", &facts, 3, 8);
        assert_eq!(r2.route, Route::Broad);
        assert!(r2.matched_entities.is_empty());
        assert_eq!(r2.community_briefs.len(), 2, "两簇 → 两条 brief");
        assert!(
            r2.community_briefs[0].contains("小明"),
            "事实多的社区 (小明簇 2 事实) 应排前"
        );
        // 分诊确定性: 同输入复跑一致
        let r3 = triage("最近有什么值得关注的", &facts, 3, 8);
        assert_eq!(r2.community_briefs, r3.community_briefs);
    }

    /// 实体匹配规则: 长度≥2 才匹配 (防单字误命中); object 也可命中.
    #[test]
    fn entity_match_rules() {
        let facts = vec![fact("机", "属于", "机房1"), fact("北京", "是", "城市")];
        // 单字实体不命中 → Broad
        let r = triage("机器怎么样", &facts, 3, 8);
        assert_eq!(r.route, Route::Broad, "单字实体不应命中");
        // object 值命中 → Entity
        let r2 = triage("机房1 的温度如何", &facts, 3, 8);
        assert_eq!(r2.route, Route::Entity);
        assert_eq!(r2.matched_entities, vec!["机房1".to_string()]);
    }
}
