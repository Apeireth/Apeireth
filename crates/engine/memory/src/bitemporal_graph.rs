//! Zep 风格双时态事实图谱 (Bitemporal Knowledge Graph) 与 Intrinsic Residual 特异性打分.
//!
//! 1. 事实三元组包含 `valid_at` / `invalid_at` / 单调 `rev` 版本链；
//! 2. 事实演化时旧边不物理删除，而是将其 `invalid_at` 设为当前时间并递增 `rev` 产生新版本；
//! 3. 混合检索结合重要性与实体逆频稀有度 (Intrinsic Residual Specificity).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 双时态图谱事实三元组.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitemporalFact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub rev: u32,
    pub valid_at_ms: u64,
    pub invalid_at_ms: Option<u64>,
    pub importance: f32,
}

/// 双时态知识图谱.
#[derive(Debug, Clone, Default)]
pub struct BitemporalGraph {
    /// 全部历史事实列表 (包含已失效的版本)
    facts: Vec<BitemporalFact>,
    /// 实体出现频次统计 (用于计算逆频稀有度/残差特异性)
    entity_frequency: HashMap<String, usize>,
}

impl BitemporalGraph {
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
            entity_frequency: HashMap::new(),
        }
    }

    /// 插入或演化事实.
    ///
    /// 若存在相同的 `(subject, predicate)` 且处于有效状态，则将旧事实标记为已失效，并以 `rev + 1` 写入新事实.
    pub fn upsert_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        importance: f32,
        now_ms: u64,
    ) -> BitemporalFact {
        let mut next_rev = 1;

        // 1. 查找并废弃先前的有效事实
        for fact in &mut self.facts {
            if fact.invalid_at_ms.is_none()
                && fact.subject == subject
                && fact.predicate == predicate
            {
                fact.invalid_at_ms = Some(now_ms);
                next_rev = fact.rev + 1;
            }
        }

        // 2. 构造新事实
        let new_fact = BitemporalFact {
            id: format!("fact_{}_{}", self.facts.len() + 1, next_rev),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            rev: next_rev,
            valid_at_ms: now_ms,
            invalid_at_ms: None,
            importance,
        };

        // 3. 统计实体频次
        *self
            .entity_frequency
            .entry(subject.to_string())
            .or_insert(0) += 1;
        *self.entity_frequency.entry(object.to_string()).or_insert(0) += 1;

        self.facts.push(new_fact.clone());
        new_fact
    }

    /// 获取指定时间戳下有效的全部事实 (支持历史时空回溯).
    pub fn get_valid_facts_at(&self, timestamp_ms: u64) -> Vec<&BitemporalFact> {
        self.facts
            .iter()
            .filter(|f| {
                f.valid_at_ms <= timestamp_ms
                    && match f.invalid_at_ms {
                        Some(invalid_ms) => timestamp_ms < invalid_ms,
                        None => true,
                    }
            })
            .collect()
    }

    /// 获取当前仍然有效的全部事实.
    pub fn get_current_valid_facts(&self) -> Vec<&BitemporalFact> {
        self.facts
            .iter()
            .filter(|f| f.invalid_at_ms.is_none())
            .collect()
    }

    /// 计算实体的 Intrinsic Residual 特异性 (逆频稀有度, 0.0 ~ 1.0).
    ///
    /// 全图仅出现 1 次的极其罕见实体获得最高特异性 (1.0)，频繁出现的大众实体得分衰减.
    pub fn compute_entity_specificity(&self, entity: &str) -> f32 {
        let count = *self.entity_frequency.get(entity).unwrap_or(&1);
        1.0 / (1.0 + (count as f32 - 1.0).ln_1p())
    }

    /// 检索与 query 相关的有效事实，并按 (重要性 * 残差特异性) 综合打分排序.
    pub fn search_facts(&self, query: &str, top_k: usize) -> Vec<(&BitemporalFact, f32)> {
        let valid_facts = self.get_current_valid_facts();
        let query_lower = query.to_lowercase();

        let mut scored_facts: Vec<(&BitemporalFact, f32)> = valid_facts
            .into_iter()
            .filter_map(|fact| {
                let match_subj = fact.subject.to_lowercase().contains(&query_lower);
                let match_pred = fact.predicate.to_lowercase().contains(&query_lower);
                let match_obj = fact.object.to_lowercase().contains(&query_lower);

                if match_subj || match_pred || match_obj {
                    let s_spec = self.compute_entity_specificity(&fact.subject);
                    let o_spec = self.compute_entity_specificity(&fact.object);
                    let avg_spec = (s_spec + o_spec) / 2.0;

                    // 综合评分 = 基础重要性 (0.6) + 特异性残差 (0.4)
                    let score = (fact.importance * 0.6) + (avg_spec * 0.4);
                    Some((fact, score))
                } else {
                    None
                }
            })
            .collect();

        scored_facts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_facts.truncate(top_k);
        scored_facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitemporal_evolution_and_history_replay() {
        let mut graph = BitemporalGraph::new();

        // 1. 在 t=1000 时记录主人的居住城市为 "北京"
        let f1 = graph.upsert_fact("user", "lives_in", "北京", 0.8, 1000);
        assert_eq!(f1.rev, 1);
        assert_eq!(f1.valid_at_ms, 1000);
        assert_eq!(f1.invalid_at_ms, None);

        // 2. 在 t=2000 时主人搬家到 "上海"
        let f2 = graph.upsert_fact("user", "lives_in", "上海", 0.9, 2000);
        assert_eq!(f2.rev, 2);
        assert_eq!(f2.valid_at_ms, 2000);
        assert_eq!(f2.invalid_at_ms, None);

        // 3. 验证历史时空回溯: t=1500 时主人应该在 "北京"
        let facts_at_1500 = graph.get_valid_facts_at(1500);
        assert_eq!(facts_at_1500.len(), 1);
        assert_eq!(facts_at_1500[0].object, "北京");

        // 4. 验证当前事实: 主人应该在 "上海"
        let current_facts = graph.get_current_valid_facts();
        assert_eq!(current_facts.len(), 1);
        assert_eq!(current_facts[0].object, "上海");
    }

    #[test]
    fn test_search_with_intrinsic_residual() {
        let mut graph = BitemporalGraph::new();
        graph.upsert_fact("user", "studies", "量子力学", 0.85, 1000);
        graph.upsert_fact("user", "loves", "咖啡", 0.70, 1000);

        let results = graph.search_facts("user", 10);
        assert_eq!(results.len(), 2);
        // 量子力学重要性更高且实体更罕见，综合得分应排名第一
        assert_eq!(results[0].0.object, "量子力学");
    }
}
