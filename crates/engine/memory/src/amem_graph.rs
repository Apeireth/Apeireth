//! A-MEM / Zep graph algorithms recovered from companion `memory_graph.rs`.
//!
//! v2 already owns:
//! - [`crate::bitemporal_graph::BitemporalGraph`] — subject+predicate version chain +
//!   `1/(1+ln1p(count-1))` specificity
//! - [`crate::canonical::graph::MemoryGraph`] — node/edge BFS, unweighted shortest path
//!
//! This module recovers the **missing** donor algorithms without a second store:
//! - full-triple chain (`s|p|o`) + inverse-frequency mean specificity (`1/n`)
//! - A-MEM `link_on_write` (Jaccard ≥ 0.3)
//! - residual-boosted CRAWL: `weight * (1 + residual_weight * content_residual)`
//! - character-set content residual (VCP residual-norm text analogue)
//!
//! Persistence is caller-owned. Default-off; not production-wired.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Jaccard overlap threshold for A-MEM auto-link (donor `>= 0.3`).
pub const LINK_OVERLAP_THRESHOLD: f64 = 0.3;

/// Default injection-block cap (donor `take(10)`).
pub const GRAPH_INJECTION_LIMIT: usize = 10;

/// Temporal graph fact (full-triple chain).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphFact {
    pub id: String,
    /// Logical chain key (`subject|predicate|object`).
    pub chain: String,
    pub rev: u64,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_at: i64,
    pub invalid_at: Option<i64>,
    pub importance: u8,
}

impl GraphFact {
    pub fn chain_key(subject: &str, predicate: &str, object: &str) -> String {
        format!("{subject}|{predicate}|{object}")
    }
}

/// Weighted A-MEM link between two content nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryLink {
    pub id: String,
    pub from: String,
    pub to: String,
    /// Weight in `[0, 1]` (v1 = character-set Jaccard).
    pub weight: f64,
}

/// Retrieval mix of importance vs residual specificity.
///
/// `combined = importance_weight × (importance/10) + residual_weight × specificity`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphRankConfig {
    pub importance_weight: f64,
    pub residual_weight: f64,
}

impl Default for GraphRankConfig {
    fn default() -> Self {
        Self {
            importance_weight: 1.0,
            residual_weight: 1.0,
        }
    }
}

/// In-memory A-MEM graph. Not a [`crate::SqliteMemoryStore`] and does not write
/// `factg-*` / `link-*` episodes (donor persistence was episode pollution).
#[derive(Debug, Clone)]
pub struct AmemGraph {
    facts: Vec<GraphFact>,
    links: Vec<MemoryLink>,
    /// Node id → content used by CRAWL / residual.
    contents: HashMap<String, String>,
    entity_counts: HashMap<String, u32>,
    rank_config: GraphRankConfig,
    next_rev_hint: HashMap<String, u64>,
}

impl Default for AmemGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AmemGraph {
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
            links: Vec::new(),
            contents: HashMap::new(),
            entity_counts: HashMap::new(),
            rank_config: GraphRankConfig::default(),
            next_rev_hint: HashMap::new(),
        }
    }

    pub fn with_rank_config(mut self, cfg: GraphRankConfig) -> Self {
        self.rank_config = cfg;
        self
    }

    pub fn rank_config(&self) -> GraphRankConfig {
        self.rank_config
    }

    /// Register node content for CRAWL / residual (does not create a fact).
    pub fn put_content(&mut self, id: impl Into<String>, content: impl Into<String>) {
        self.contents.insert(id.into(), content.into());
    }

    pub fn content_of(&self, id: &str) -> Option<&str> {
        self.contents.get(id).map(String::as_str)
    }

    pub fn links(&self) -> &[MemoryLink] {
        &self.links
    }

    /// Add a fact. Same `(s,p,o)` invalidates the previous active edge
    /// (append-only: old row kept with `invalid_at`). New-triple entities
    /// increment inverse-frequency counts; same-chain replacement does not.
    pub fn add_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        importance: u8,
        now: i64,
    ) -> String {
        let chain = GraphFact::chain_key(subject, predicate, object);
        let mut next_rev = self
            .facts
            .iter()
            .filter(|f| f.chain == chain)
            .map(|f| f.rev)
            .max()
            .or_else(|| self.next_rev_hint.get(&chain).copied())
            .unwrap_or(0)
            + 1;

        let existing = self.valid_for(&chain).cloned();
        if let Some(old) = existing.as_ref() {
            let mut inv = old.clone();
            inv.id = format!("factg-inv-{next_rev}");
            inv.rev = next_rev;
            inv.invalid_at = Some(now);
            self.facts.push(inv);
            next_rev += 1;
        }

        let id = format!("factg-{next_rev}-{}", self.facts.len() + 1);
        let fact = GraphFact {
            id: id.clone(),
            chain: chain.clone(),
            rev: next_rev,
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            valid_at: now,
            invalid_at: None,
            importance,
        };
        if existing.is_none() {
            self.observe_fact(&fact);
        }
        self.next_rev_hint.insert(chain, next_rev);
        self.facts.push(fact);
        id
    }

    fn observe_fact(&mut self, f: &GraphFact) {
        for e in [&f.subject, &f.predicate, &f.object] {
            *self.entity_counts.entry(e.clone()).or_insert(0) += 1;
        }
    }

    fn valid_for(&self, chain: &str) -> Option<&GraphFact> {
        self.active_facts().into_iter().find(|f| f.chain == chain)
    }

    /// Current valid facts: highest-rev per chain with `invalid_at == None`.
    pub fn active_facts(&self) -> Vec<&GraphFact> {
        let mut by_chain: HashMap<&str, &GraphFact> = HashMap::new();
        for f in &self.facts {
            match by_chain.get(f.chain.as_str()) {
                Some(existing) if existing.rev >= f.rev => {}
                _ => {
                    by_chain.insert(f.chain.as_str(), f);
                }
            }
        }
        by_chain
            .into_values()
            .filter(|f| f.invalid_at.is_none())
            .collect()
    }

    /// Inverse-frequency mean of s/p/o. Unique-across-graph entity → 1.0.
    pub fn specificity(&self, f: &GraphFact) -> f64 {
        fact_specificity(&self.entity_counts, f)
    }

    pub fn combined_score(&self, f: &GraphFact) -> f64 {
        combined_score(f.importance, self.specificity(f), self.rank_config)
    }

    /// Filter + rank active facts. Empty filters match everything.
    pub fn query(
        &self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
    ) -> Vec<&GraphFact> {
        let filtered: Vec<&GraphFact> = self
            .active_facts()
            .into_iter()
            .filter(|f| subject.is_none_or(|s| f.subject == s))
            .filter(|f| predicate.is_none_or(|p| f.predicate == p))
            .filter(|f| object.is_none_or(|o| f.object == o))
            .collect();
        self.rank(filtered)
    }

    fn rank<'a>(&self, mut facts: Vec<&'a GraphFact>) -> Vec<&'a GraphFact> {
        facts.sort_by(|a, b| {
            self.combined_score(b)
                .partial_cmp(&self.combined_score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chain.cmp(&b.chain))
                .then_with(|| a.id.cmp(&b.id))
        });
        facts
    }

    /// Ranked injection block of active facts (empty graph → empty string).
    pub fn graph_injection(&self) -> String {
        let facts = self.query(None, None, None);
        if facts.is_empty() {
            return String::new();
        }
        let mut s = String::from("【事实图】(时序知识图谱, 双时态有效事实):\n");
        for f in facts.iter().take(GRAPH_INJECTION_LIMIT) {
            s.push_str(&format!(
                "  • {} {} {} (有效自 {})\n",
                f.subject, f.predicate, f.object, f.valid_at
            ));
        }
        s
    }

    /// Auto-link `new_id` against registered contents with Jaccard ≥ threshold.
    /// Skips self, `link-*`, and `tomb-*` ids (donor skip list).
    pub fn link_on_write(&mut self, new_id: &str, new_content: &str) {
        self.put_content(new_id, new_content);
        let existing: Vec<(String, String)> = self
            .contents
            .iter()
            .filter(|(id, _)| {
                id.as_str() != new_id && !id.starts_with("link-") && !id.starts_with("tomb-")
            })
            .map(|(id, c)| (id.clone(), c.clone()))
            .collect();
        for (to, content) in existing {
            let w = text_overlap(new_content, &content);
            if w >= LINK_OVERLAP_THRESHOLD {
                self.links.push(MemoryLink {
                    id: format!("link-{}-{}", new_id, to),
                    from: new_id.to_string(),
                    to,
                    weight: w,
                });
            }
        }
    }

    /// Seeded CRAWL with residual anchor boost. Returns **contents** in visit order.
    pub fn crawl(&self, seeds: &[String], budget: usize) -> Vec<String> {
        crawl(
            seeds,
            budget,
            &self.contents,
            &self.links,
            self.rank_config.residual_weight,
        )
    }
}

/// Inverse-frequency mean of the three entities.
pub fn fact_specificity(counts: &HashMap<String, u32>, f: &GraphFact) -> f64 {
    let rarity = |e: &str| -> f64 {
        match counts.get(e) {
            Some(&n) if n > 0 => 1.0 / f64::from(n),
            _ => 1.0,
        }
    };
    (rarity(&f.subject) + rarity(&f.predicate) + rarity(&f.object)) / 3.0
}

pub fn combined_score(importance: u8, specificity: f64, cfg: GraphRankConfig) -> f64 {
    let imp = f64::from(importance).min(10.0) / 10.0;
    cfg.importance_weight * imp + cfg.residual_weight * specificity
}

/// Character-set Jaccard after stripping whitespace and ASCII punctuation.
pub fn text_overlap(a: &str, b: &str) -> f64 {
    let sa = norm_chars(a);
    let sb = norm_chars(b);
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Fraction of this node's charset not explained by any neighbour. Empty
/// content → 0.0; no neighbours → 1.0.
pub fn content_residual(content: &str, neighbors: &[&str]) -> f64 {
    let mine = norm_chars(content);
    if mine.is_empty() {
        return 0.0;
    }
    if neighbors.is_empty() {
        return 1.0;
    }
    let mut theirs = HashSet::new();
    for n in neighbors {
        theirs.extend(norm_chars(n));
    }
    let unexplained = mine.difference(&theirs).count();
    unexplained as f64 / mine.len() as f64
}

fn norm_chars(s: &str) -> HashSet<char> {
    s.chars()
        .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
        .collect()
}

/// Residual-boosted CRAWL over an explicit content/link snapshot.
pub fn crawl(
    seeds: &[String],
    budget: usize,
    contents: &HashMap<String, String>,
    links: &[MemoryLink],
    residual_weight: f64,
) -> Vec<String> {
    let content_of = |id: &str| contents.get(id).cloned();
    let residual_of = |id: &str| -> f64 {
        let Some(content) = content_of(id) else {
            return 0.0;
        };
        let neighbor_texts: Vec<String> = links
            .iter()
            .filter(|l| l.from == id || l.to == id)
            .filter_map(|l| content_of(if l.from == id { &l.to } else { &l.from }))
            .collect();
        let refs: Vec<&str> = neighbor_texts.iter().map(String::as_str).collect();
        content_residual(&content, &refs)
    };

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: Vec<(String, f64)> = seeds.iter().map(|s| (s.clone(), 1.0)).collect();
    while !queue.is_empty() && out.len() < budget {
        queue.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (id, _) = queue.remove(0);
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(c) = content_of(&id) {
            out.push(c);
        }
        for l in links.iter().filter(|l| l.from == id) {
            if !seen.contains(&l.to) {
                let boosted = l.weight * (1.0 + residual_weight * residual_of(&l.to));
                queue.push((l.to.clone(), boosted));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_facts_invalidate_old() {
        let mut g = AmemGraph::new();
        g.add_fact("主人", "备考", "高数期中", 8, 1);
        g.add_fact("主人", "备考", "高数期中", 9, 2);
        let active = g.active_facts();
        assert_eq!(active.len(), 1, "old edge invalidated");
        assert_eq!(active[0].importance, 9);
        g.add_fact("主人", "喜欢", "烟火", 7, 3);
        assert_eq!(g.active_facts().len(), 2);
        let inj = g.graph_injection();
        assert!(inj.contains("备考"));
        assert!(inj.contains("喜欢"));
    }

    #[test]
    fn links_and_crawl() {
        let mut g = AmemGraph::new();
        g.put_content("mem-ex-a", "主人喜欢水墨画风格");
        g.link_on_write("mem-ex-b", "主人偏好水墨画风格和深蓝夜空");
        let crawled = g.crawl(&["mem-ex-b".into()], 3);
        assert!(
            crawled.iter().any(|c| c.contains("喜欢水墨画")),
            "CRAWL should expand to id1: {crawled:?}"
        );
    }

    #[test]
    fn text_overlap_basic() {
        assert!(text_overlap("abcde", "abcxy") > 0.3);
        assert!(
            text_overlap("今天天气很好", "主人喜欢深蓝夜空") < 0.3,
            "no shared charset"
        );
    }

    #[test]
    fn structured_query_filters_active_facts() {
        let mut g = AmemGraph::new();
        g.add_fact("主人", "备考", "高数期中", 8, 1);
        g.add_fact("主人", "喜欢", "烟火", 7, 1);
        g.add_fact("本座", "负责", "基地", 6, 1);
        assert_eq!(g.query(Some("主人"), None, None).len(), 2);
        let by_pred = g.query(None, Some("喜欢"), None);
        assert_eq!(by_pred.len(), 1);
        assert_eq!(by_pred[0].object, "烟火");
        assert_eq!(g.query(Some("主人"), Some("备考"), None).len(), 1);
        assert!(g.query(Some("不存在"), None, None).is_empty());
        assert_eq!(g.query(None, None, None).len(), 3);
    }

    #[test]
    fn n6_boundary_empty_graph_and_single_node() {
        let mut g = AmemGraph::new();
        assert!(g.query(None, None, None).is_empty());
        assert_eq!(g.graph_injection(), "");
        assert!(g.crawl(&[], 5).is_empty());
        g.add_fact("主人", "备考", "高数期中", 8, 1);
        let facts = g.query(None, None, None);
        let s = g.specificity(facts[0]);
        assert!(
            (s - 1.0).abs() < 1e-9,
            "single node max specificity, got {s}"
        );
    }

    #[test]
    fn n6_specificity_discriminates_shared_vs_unique() {
        let mut g = AmemGraph::new();
        g.add_fact("主人", "喜欢", "烟火", 7, 1);
        g.add_fact("主人", "喜欢", "深蓝夜空", 7, 1);
        g.add_fact("本座", "负责", "基地", 6, 1);
        let active = g.active_facts();
        let common = active.iter().find(|f| f.object == "烟火").unwrap();
        let unique = active.iter().find(|f| f.subject == "本座").unwrap();
        let s_common = g.specificity(common);
        let expected = (0.5 + 0.5 + 1.0) / 3.0;
        assert!(
            (s_common - expected).abs() < 1e-9,
            "got {s_common}, want {expected}"
        );
        let s_unique = g.specificity(unique);
        assert!((s_unique - 1.0).abs() < 1e-9);
        assert!(s_unique > s_common);
    }

    #[test]
    fn n6_content_residual_boundary() {
        assert_eq!(content_residual("", &[]), 0.0);
        assert_eq!(content_residual("abc", &[]), 1.0);
        assert!((content_residual("abc", &["abc"]) - 0.0).abs() < 1e-9);
        assert!((content_residual("abc", &["xyz"]) - 1.0).abs() < 1e-9);
        assert!((content_residual("abcd", &["ab"]) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn n6_combined_rank_weights_configurable() {
        let build = |cfg: GraphRankConfig| {
            let mut g = AmemGraph::new().with_rank_config(cfg);
            g.add_fact("主人", "喜欢", "烟火", 9, 1);
            g.add_fact("主人", "喜欢", "深蓝", 9, 1);
            g.add_fact("主人", "喜欢", "水墨", 9, 1);
            g.add_fact("主人", "喜欢", "星空", 9, 1);
            g.add_fact("孤本", "罕有", "残卷", 3, 1);
            g
        };
        let g_imp = build(GraphRankConfig {
            importance_weight: 1.0,
            residual_weight: 0.0,
        });
        assert_eq!(g_imp.query(None, None, None)[0].importance, 9);
        let g_res = build(GraphRankConfig {
            importance_weight: 0.0,
            residual_weight: 1.0,
        });
        let top = g_res.query(None, None, None);
        assert_eq!(top[0].subject, "孤本");
    }

    #[test]
    fn n6_deterministic_scores_and_order() {
        let build = || {
            let mut g = AmemGraph::new();
            g.add_fact("主人", "喜欢", "烟火", 7, 1);
            g.add_fact("主人", "备考", "高数期中", 8, 1);
            g.add_fact("本座", "负责", "基地", 6, 1);
            g.add_fact("烟火", "照亮", "夜空", 5, 1);
            g
        };
        let a = build();
        let b = build();
        let ra = a.query(None, None, None);
        let rb = b.query(None, None, None);
        assert_eq!(ra.len(), rb.len());
        for (fa, fb) in ra.iter().zip(rb.iter()) {
            assert_eq!(fa.chain, fb.chain);
            assert!((a.combined_score(fa) - b.combined_score(fb)).abs() < 1e-12);
        }
    }

    #[test]
    fn n6_incremental_counts_match_cold_start() {
        let mut g = AmemGraph::new();
        g.add_fact("主人", "喜欢", "烟火", 7, 1);
        let _ = g.query(None, None, None);
        g.add_fact("主人", "备考", "高数期中", 8, 2);
        g.add_fact("主人", "备考", "高数期中", 9, 3);
        let mut cold = AmemGraph::new();
        cold.add_fact("主人", "喜欢", "烟火", 7, 1);
        cold.add_fact("主人", "备考", "高数期中", 9, 3);
        let active = g.active_facts();
        assert_eq!(active.len(), 2);
        for f in &active {
            let inc = g.specificity(f);
            let cold_s = cold.specificity(f);
            assert!(
                (inc - cold_s).abs() < 1e-12,
                "incremental vs cold: {inc} vs {cold_s}"
            );
        }
    }

    #[test]
    fn n6_crawl_anchor_boost_prefers_residual() {
        let mut contents = HashMap::new();
        contents.insert("mem-seed".into(), "aaaa bbbb".into());
        contents.insert("mem-dup".into(), "aaaa bbbb".into());
        contents.insert("mem-uniq".into(), "zzzz yyyy".into());
        let links = vec![
            MemoryLink {
                id: "link-1".into(),
                from: "mem-seed".into(),
                to: "mem-dup".into(),
                weight: 0.5,
            },
            MemoryLink {
                id: "link-2".into(),
                from: "mem-seed".into(),
                to: "mem-uniq".into(),
                weight: 0.31,
            },
        ];
        let out = crawl(&["mem-seed".into()], 2, &contents, &links, 1.0);
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("aaaa"), "seed first");
        assert!(
            out[1].contains("zzzz"),
            "residual boost should outrank high-weight duplicate: {:?}",
            out[1]
        );
    }

    #[test]
    fn same_chain_replacement_does_not_inflate_entity_counts() {
        let mut g = AmemGraph::new();
        g.add_fact("主人", "备考", "高数期中", 8, 1);
        g.add_fact("主人", "备考", "高数期中", 9, 2);
        let facts = g.query(None, None, None);
        assert_eq!(facts.len(), 1);
        let s = g.specificity(facts[0]);
        assert!(
            (s - 1.0).abs() < 1e-9,
            "replacement must not bump entity freq: {s}"
        );
    }
}
