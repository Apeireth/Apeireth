//! 双时态事实图谱 (Bitemporal Knowledge Graph) 与 Intrinsic Residual 特异性打分.
//!
//! Phase 2 升级 (per `_research_mem/ra/ra2-bitemporal-algebra-proposal.md`):
//! 从「单时态 valid-time 版本链」升级为 BTFM 五元组
//! `(fid, φ, V, B, π, θ)` —— valid interval + belief interval (transaction time)
//! + provenance + query-time trust 派生。
//!
//! ## 旧 API 语义不变 (零破坏)
//! - `upsert_fact(subject, predicate, object, importance, now_ms)` 签名与行为不变:
//!   `valid_at_ms = now_ms`、`belief_at_ms = now_ms` (退化等价: belief == valid)。
//! - `get_valid_facts_at` / `get_current_valid_facts` 语义不变
//!   (在退化情形与 `facts_as_of` / `NOW` 等价)。
//!
//! ## 新增 (additive)
//! - 字段: `belief_at_ms` / `belief_until_ms` / `provenance` / `conflict_count`
//!   (serde 缺省: 旧序列化数据反序列化后 belief_at=0, 语义视为退化等价, 见
//!   [`BitemporalFact::normalize_legacy_belief`])。
//! - 方法: `insert_fact_full` (迟到事实入口) / `retract_fact` (撤回追加) /
//!   `facts_as_of` / `beliefs_as_of` / `retrospective` / `belief_trust` /
//!   `active_arbitrated_facts` (信任半环 ⊕=max 仲裁)。
//!
//! ## 公理 (RA-2 §7)
//! - A1 append-only belief: b_s 不可变, b_e 只 ∞→t; 旧行永不改删。
//! - A2 valid-interval 良构: v_s < v_e 或 v_e=∞; 撤回用空区间 [t, t) 表达。
//! - A3 单活跃信念: 每键每信念时刻至多一个活跃版本 (线性 rev 链)。
//! - A4 更正即追加: 更正 = 新版本 + 闭包旧信念 (belief_until = 新 belief_at)。
//! - A6 trust 单调: ⊕=max 幂等 ⇒ 增证据不降 trust。
//!
//! ## 边界 (0 装)
//! - IC3 按线性信念链处理 (多智能体分支信念留后续)。
//! - trust 的 w/λ/κ 是工程策略参数, 非数据学习结论 (RA-2 §8.2)。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 事实来源类型 (RA-2 §5.2, 基础权重为可配置策略)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FactProvenance {
    /// 用户显式注入 (默认最高权重)。
    #[default]
    Manual,
    /// 工具执行产物。
    Tool,
    /// LLM 对话提取。
    Dialog,
    /// 观察钩子。
    Observation,
    /// 反思期二次提炼 (推导, 需打折)。
    Reflection,
}

impl FactProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Tool => "tool",
            Self::Dialog => "dialog",
            Self::Observation => "observation",
            Self::Reflection => "reflection",
        }
    }
}

/// 信任半环参数 (RA-2 §5.2: w / δ / κ, 全部可配置策略)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustWeights {
    pub w_manual: f32,
    pub w_tool: f32,
    pub w_dialog: f32,
    pub w_observation: f32,
    pub w_reflection: f32,
    /// δ(Δt) = exp(-λ · Δt_ms)。
    pub decay_lambda_per_ms: f32,
}

impl Default for TrustWeights {
    fn default() -> Self {
        Self {
            w_manual: 1.0,
            w_tool: 0.9,
            w_dialog: 0.7,
            w_observation: 0.6,
            w_reflection: 0.5,
            decay_lambda_per_ms: 1e-6,
        }
    }
}

impl TrustWeights {
    fn w(&self, p: FactProvenance) -> f32 {
        match p {
            FactProvenance::Manual => self.w_manual,
            FactProvenance::Tool => self.w_tool,
            FactProvenance::Dialog => self.w_dialog,
            FactProvenance::Observation => self.w_observation,
            FactProvenance::Reflection => self.w_reflection,
        }
    }
}

/// 双时态图谱事实版本 (BTFM 五元组)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitemporalFact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub rev: u32,
    /// V.v_s — 事实时间起点 (迟到事实可早于信念时间)。
    pub valid_at_ms: u64,
    /// V.v_e — 事实时间终点 (None = ∞)。
    pub invalid_at_ms: Option<u64>,
    pub importance: f32,
    /// B.b_s — 信念(事务)时间起点; 系统赋值, append-only。
    #[serde(default)]
    pub belief_at_ms: u64,
    /// B.b_e — 信念时间终点 (None = ∞; 闭包时置为后继版本的 b_s)。
    #[serde(default)]
    pub belief_until_ms: Option<u64>,
    /// π — 来源注解。
    #[serde(default)]
    pub provenance: FactProvenance,
    /// 键被推翻/冲突次数 (κ 冲突惩罚输入)。
    #[serde(default)]
    pub conflict_count: u32,
}

impl BitemporalFact {
    /// 旧序列化数据 (无信念字段) 反序列化后 belief_at=0; 语义上按退化等价
    /// (belief == valid) 归一。仅在加载旧数据路径显式调用。
    pub fn normalize_legacy_belief(&mut self) {
        if self.belief_at_ms == 0 {
            self.belief_at_ms = self.valid_at_ms;
        }
    }
}

/// 双时态知识图谱。
#[derive(Debug, Clone, Default)]
pub struct BitemporalGraph {
    /// 全部历史事实版本 (包含已失效版本; 永不物理删除)。
    facts: Vec<BitemporalFact>,
    /// 实体出现频次统计 (Intrinsic Residual 特异性)。
    entity_frequency: HashMap<String, usize>,
}

impl BitemporalGraph {
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
            entity_frequency: HashMap::new(),
        }
    }

    fn bump_entity_frequency(&mut self, subject: &str, object: &str) {
        *self.entity_frequency.entry(subject.to_string()).or_insert(0) += 1;
        *self.entity_frequency.entry(object.to_string()).or_insert(0) += 1;
    }

    /// 闭包键 k 的全部活跃信念版本 (A4: belief_until ← 新版本 b_s)。
    /// 返回 (旧活跃版本数, 冲突计数 = 被闭包版本数)。
    fn close_active_beliefs(&mut self, key_subject: &str, key_predicate: &str, at_ms: u64) -> u32 {
        let mut closed = 0u32;
        for fact in &mut self.facts {
            if fact.belief_until_ms.is_none()
                && fact.subject == key_subject
                && fact.predicate == key_predicate
            {
                fact.belief_until_ms = Some(at_ms);
                closed += 1;
            }
        }
        closed
    }

    fn next_rev_for(&self, key_subject: &str, key_predicate: &str) -> u32 {
        self.facts
            .iter()
            .filter(|f| f.subject == key_subject && f.predicate == key_predicate)
            .map(|f| f.rev)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// 插入或演化事实 (旧 API, 签名与语义不变)。
    ///
    /// 若存在相同的 `(subject, predicate)` 且处于有效状态，则将旧事实标记为已失效，
    /// 并以 `rev + 1` 写入新事实。belief_at = now_ms (退化等价: belief == valid)。
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

        // 2. 构造新事实 (信念时间 = 写入时刻: 旧行为)
        let new_fact = BitemporalFact {
            id: format!("fact_{}_{}", self.facts.len() + 1, next_rev),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            rev: next_rev,
            valid_at_ms: now_ms,
            invalid_at_ms: None,
            importance,
            belief_at_ms: now_ms,
            belief_until_ms: None,
            provenance: FactProvenance::Manual,
            conflict_count: 0,
        };

        // 3. 统计实体频次
        self.bump_entity_frequency(subject, object);

        self.facts.push(new_fact.clone());
        new_fact
    }

    /// 迟到事实入口 (RA-2 §6.2 新增): 显式指定 valid 区间与 belief_at。
    ///
    /// 允许 `valid_from_ms < belief_at_ms` (真相早于得知)。闭包同键旧信念 (A4)，
    /// conflict_count = 被闭包的活跃版本数。返回新版本。
    #[allow(clippy::too_many_arguments)]
    pub fn insert_fact_full(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        importance: f32,
        valid_from_ms: u64,
        valid_until_ms: Option<u64>,
        belief_at_ms: u64,
        provenance: FactProvenance,
    ) -> BitemporalFact {
        let rev = self.next_rev_for(subject, predicate);
        let closed = self.close_active_beliefs(subject, predicate, belief_at_ms);
        let new_fact = BitemporalFact {
            id: format!("fact_{}_{}", self.facts.len() + 1, rev),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            rev,
            valid_at_ms: valid_from_ms,
            invalid_at_ms: valid_until_ms,
            importance,
            belief_at_ms,
            belief_until_ms: None,
            provenance,
            conflict_count: closed,
        };
        self.bump_entity_frequency(subject, object);
        self.facts.push(new_fact.clone());
        new_fact
    }

    /// 撤回 (RA-2 §6.2 新增): 追加空 valid 区间 [t, t) 的 tombstone 版本。
    ///
    /// 旧行保留 (A4); 三类查询均排除该键 (v_s = v_e 使 valid 谓词恒假)。
    /// 返回 tombstone 版本。
    pub fn retract_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        belief_at_ms: u64,
        reason: &str,
    ) -> BitemporalFact {
        let rev = self.next_rev_for(subject, predicate);
        let closed = self.close_active_beliefs(subject, predicate, belief_at_ms);
        let tombstone = BitemporalFact {
            id: format!("fact_{}_{}", self.facts.len() + 1, rev),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: format!("(retracted: {reason})"),
            rev,
            valid_at_ms: belief_at_ms,
            invalid_at_ms: Some(belief_at_ms), // 空区间: [t, t)
            importance: 0.0,
            belief_at_ms,
            belief_until_ms: None,
            provenance: FactProvenance::Manual,
            conflict_count: closed,
        };
        self.facts.push(tombstone.clone());
        tombstone
    }

    /// 当前信念下的事实时间切片 `facts_as_of(t)` (RA-2 §3 查询 2)。
    ///
    /// 语义 (以 §3.2 Datalog/代数为准, 修正 §4 表格与代数不符处):
    /// 每键取 **max-rev** 且 `t ∈ V`、`b_s ≤ now` 的版本 —— 即"版本链对
    /// valid 时间点的最新声明"(迟到更正覆盖重叠区间的语义)。
    /// 键的最新版本为撤回 tombstone (空 valid 区间) 时整键排除。
    pub fn facts_as_of(&self, t_ms: u64, now_ms: u64) -> Vec<&BitemporalFact> {
        self.retrospective_impl(t_ms, now_ms)
    }

    /// 信念时间切片 `beliefs_as_of(t)`: "当时数据库里是什么" (RA-2 §3 查询 3)。
    pub fn beliefs_as_of(&self, t_ms: u64) -> Vec<&BitemporalFact> {
        self.facts
            .iter()
            .filter(|f| {
                f.belief_at_ms <= t_ms
                    && match f.belief_until_ms {
                        Some(be) => t_ms < be,
                        None => true,
                    }
            })
            .collect()
    }

    /// 双时态切片 `retrospective(t_ask, t_belief)`: 截至信念时刻 t_belief，
    /// 系统认为事实时刻 t_ask 是什么 (RA-2 §3 查询 4)。
    ///
    /// 语义: 每键取 max-rev 且 `t_ask ∈ V`、`b_s ≤ t_belief` 的版本
    /// (信念时间只约束"版本已到达", 不要求其信念区间仍开放 ——
    /// 版本链对 valid 时间的声明由后继版本覆盖, 见 §4 迟到更正例)。
    pub fn retrospective(&self, t_ask_ms: u64, t_belief_ms: u64) -> Vec<&BitemporalFact> {
        self.retrospective_impl(t_ask_ms, t_belief_ms)
    }

    /// 共享实现: 版本链选择。
    ///
    /// 规则 (以 §3.2 Datalog/代数为准):
    /// 1. 每键取 `b_s ≤ t_belief` 的**最高 rev** 版本 (无论其 valid 区间);
    /// 2. 该版本为撤回 tombstone ⇒ 整键排除 (撤回语义);
    /// 3. 否则在 `t_ask ∈ V` 且 `b_s ≤ t_belief` 的版本中取最高 rev 输出。
    fn retrospective_impl(
        &self,
        t_ask_ms: u64,
        t_belief_ms: u64,
    ) -> Vec<&BitemporalFact> {
        // 按键收集 (最新已到达版本, 可选有效版本)。
        let mut latest: HashMap<(String, String), &BitemporalFact> = HashMap::new();
        for f in &self.facts {
            if f.belief_at_ms > t_belief_ms {
                continue;
            }
            let key = (f.subject.clone(), f.predicate.clone());
            match latest.get(&key) {
                None => {
                    latest.insert(key, f);
                }
                Some(cur) if f.rev > cur.rev => {
                    latest.insert(key, f);
                }
                Some(_) => {}
            }
        }
        let mut out: Vec<&BitemporalFact> = Vec::new();
        for (key, newest) in latest {
            if is_tombstone(newest) {
                continue; // 整键已撤回
            }
            // 在 valid 覆盖 t_ask 的版本中取最高 rev。
            let mut best: Option<&BitemporalFact> = None;
            for f in &self.facts {
                if f.subject != key.0 || f.predicate != key.1 {
                    continue;
                }
                let valid = f.valid_at_ms <= t_ask_ms
                    && match f.invalid_at_ms {
                        Some(ve) => t_ask_ms < ve,
                        None => true,
                    };
                if !valid || f.belief_at_ms > t_belief_ms {
                    continue;
                }
                match best {
                    None => best = Some(f),
                    Some(cur) if f.rev > cur.rev => best = Some(f),
                    Some(_) => {}
                }
            }
            if let Some(b) = best {
                out.push(b);
            }
        }
        out.sort_by(|a, b| a.subject.cmp(&b.subject).then(a.predicate.cmp(&b.predicate)));
        out
    }

    /// 查询时 trust 派生 (RA-2 §5.2): θ = w(π) ⊗ δ(Δt) ⊗ κ(n)。
    ///
    /// Viterbi 半环: ⊗=×; 时间衰减 δ=exp(-λ·Δt); 冲突惩罚 κ=1/(1+n)。
    pub fn belief_trust(&self, fact: &BitemporalFact, now_ms: u64, w: &TrustWeights) -> f32 {
        let dt = now_ms.saturating_sub(fact.belief_at_ms);
        let delta = (-w.decay_lambda_per_ms * dt as f32).exp();
        let kappa = 1.0 / (1.0 + fact.conflict_count as f32);
        w.w(fact.provenance) * delta * kappa
    }

    /// ⊕=max 仲裁 (RA-2 §5.3): 每键当前活跃版本取 trust 最大者;
    /// 平局: b_s 晚者胜; 再平局: 载荷字典序 (确定性)。
    pub fn active_arbitrated_facts<'a>(
        &'a self,
        now_ms: u64,
        w: &TrustWeights,
    ) -> HashMap<(String, String), &'a BitemporalFact> {
        let mut out: HashMap<(String, String), &BitemporalFact> = HashMap::new();
        for f in &self.facts {
            let active = match f.belief_until_ms {
                Some(be) => now_ms < be,
                None => true,
            } && f.belief_at_ms <= now_ms;
            if !active {
                continue;
            }
            let key = (f.subject.clone(), f.predicate.clone());
            let trust = self.belief_trust(f, now_ms, w);
            let replace = match out.get(&key) {
                None => true,
                Some(cur) => {
                    let cur_trust = self.belief_trust(cur, now_ms, w);
                    trust > cur_trust
                        || (trust == cur_trust && f.belief_at_ms > cur.belief_at_ms)
                        || (trust == cur_trust
                            && f.belief_at_ms == cur.belief_at_ms
                            && format!("{}:{}", f.subject, f.object)
                                > format!("{}:{}", cur.subject, cur.object))
                }
            };
            if replace {
                out.insert(key, f);
            }
        }
        out
    }

    /// 获取指定时间戳下有效的全部事实 (旧 API, 语义不变).
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

    /// 获取当前仍然有效的全部事实 (旧 API, 语义不变).
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

/// 撤回 tombstone 判定: 空 valid 区间 [t, t) (A2 允许的"撤回"表达)。
fn is_tombstone(f: &BitemporalFact) -> bool {
    f.invalid_at_ms == Some(f.valid_at_ms)
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

    /// 旧 API 退化等价: upsert 路径下 belief == valid, 三类查询与旧查询一致。
    #[test]
    fn legacy_upsert_degnerate_equivalence() {
        let mut graph = BitemporalGraph::new();
        graph.upsert_fact("user", "lives_in", "北京", 0.8, 1000);
        graph.upsert_fact("user", "lives_in", "上海", 0.9, 2000);
        let now = 2500;
        let legacy = graph.get_valid_facts_at(1500);
        let facts = graph.facts_as_of(1500, now);
        assert_eq!(legacy.len(), facts.len());
        assert_eq!(legacy[0].object, facts[0].object);
        // 旧路径 belief 与 valid 同源
        for f in graph.get_current_valid_facts() {
            assert_eq!(f.belief_at_ms, f.valid_at_ms);
        }
    }

    /// RA-2 §4 工作示例: 迟到事实 + 撤回/更正, 9 行查询表逐条验证。
    #[test]
    fn ra2_worked_example_late_arrival_and_correction() {
        let mut graph = BitemporalGraph::new();
        // d1: 北京 V=[0,200) B=[100,300) rev=1
        graph.insert_fact_full("user", "lives_in", "北京", 0.8, 0, Some(200), 100, FactProvenance::Dialog);
        // d2: 上海 V=[200,∞) B=[300,400) rev=2
        graph.insert_fact_full("user", "lives_in", "上海", 0.9, 200, None, 300, FactProvenance::Dialog);
        // d3: 上海 V=[180,∞) B=[400,∞) rev=3 (迟到更正)
        graph.insert_fact_full("user", "lives_in", "上海", 0.9, 180, None, 400, FactProvenance::Manual);
        let tau = 500;

        let f = |t: u64| {
            graph
                .facts_as_of(t, tau)
                .into_iter()
                .map(|x| x.object.as_str())
                .collect::<Vec<_>>()
        };
        let b = |t: u64| {
            graph
                .beliefs_as_of(t)
                .into_iter()
                .map(|x| x.object.as_str())
                .collect::<Vec<_>>()
        };
        let r = |ta: u64, tb: u64| {
            graph
                .retrospective(ta, tb)
                .into_iter()
                .map(|x| x.object.as_str())
                .collect::<Vec<_>>()
        };

        assert_eq!(f(150), vec!["北京"]);
        assert_eq!(f(190), vec!["上海"], "迟到更正后 190 也是上海");
        assert_eq!(f(250), vec!["上海"]);
        assert_eq!(b(150), vec!["北京"], "当时只知道北京");
        assert_eq!(b(250), vec!["北京"], "当时 (t<300) 仍以为北京");
        assert_eq!(b(350), vec!["上海"], "当时认为上海自 200 起");
        // 注 (0 装): 提案 §4 表格此行写 r(250,250)=北京, 但 250∉V(d1)=[0,200),
        // 与 §3.2 Datalog 规范矛盾; 以代数为准 → 空集.
        assert!(r(250, 250).is_empty(), "250 不在任何截至 250 已到达版本的 valid 区间内");
        assert_eq!(r(250, 500), vec!["上海"], "截至 500 的信念: 已更正");
        assert_eq!(r(190, 350), vec!["北京"], "截至 350 尚未收到迟到更正");
    }

    /// A1/A4: beliefs_as_of(250) 在全部更正之后仍返回旧信念 (审计链完整)。
    #[test]
    fn audit_history_immutable_belief_slice() {
        let mut graph = BitemporalGraph::new();
        graph.insert_fact_full("user", "lives_in", "北京", 0.8, 0, Some(200), 100, FactProvenance::Dialog);
        graph.insert_fact_full("user", "lives_in", "上海", 0.9, 200, None, 300, FactProvenance::Dialog);
        graph.insert_fact_full("user", "lives_in", "上海", 0.9, 180, None, 400, FactProvenance::Manual);
        let old = graph
            .beliefs_as_of(250)
            .into_iter()
            .map(|x| x.object.as_str())
            .collect::<Vec<_>>();
        assert_eq!(old, vec!["北京"]);
    }

    /// 撤回: 空 valid 区间 tombstone, facts_as_of 整键排除; 旧行保留 (A4);
    /// beliefs_as_of 如实显示"当时存在撤回标记"。
    #[test]
    fn retract_appends_tombstone_and_excludes_key() {
        let mut graph = BitemporalGraph::new();
        graph.insert_fact_full("user", "lives_in", "北京", 0.8, 0, Some(200), 100, FactProvenance::Manual);
        let n_before = graph.facts.len();
        let tomb = graph.retract_fact("user", "lives_in", 500, "用户要求撤回");
        assert_eq!(graph.facts.len(), n_before + 1, "撤回是追加, 不删旧行");
        assert_eq!(tomb.valid_at_ms, tomb.invalid_at_ms.unwrap());
        assert!(graph.facts_as_of(100, 600).is_empty(), "tombstone 整键排除");
        // 信念切片如实包含 tombstone (as-it-was-then)
        let believed = graph.beliefs_as_of(600);
        assert_eq!(believed.len(), 1);
        assert!(is_tombstone(believed[0]));
        // 旧行仍在 (A4 更正即追加)
        assert!(graph
            .facts
            .iter()
            .any(|x| x.object == "北京" && x.belief_until_ms == Some(500)));
    }

    /// 信任半环: κ 冲突惩罚随 conflict_count 递减; δ 时间衰减; ⊕=max 仲裁确定性。
    #[test]
    fn trust_semiring_decay_conflict_and_arbitration() {
        let w = TrustWeights::default();
        let mut graph = BitemporalGraph::new();
        let d1 = graph.insert_fact_full("k", "p", "v1", 1.0, 0, None, 100, FactProvenance::Manual);
        let d2 = graph.insert_fact_full("k", "p", "v2", 1.0, 0, None, 200, FactProvenance::Manual);
        let d3 = graph.insert_fact_full("k", "p", "v3", 1.0, 0, None, 300, FactProvenance::Manual);
        // conflict_count: d1=0, d2=1, d3=1 (每版本 = 被闭包的活跃数)
        assert_eq!(d1.conflict_count, 0);
        assert_eq!(d2.conflict_count, 1);
        assert_eq!(d3.conflict_count, 1);
        let t1 = graph.belief_trust(&d1, 300, &w);
        let t2 = graph.belief_trust(&d2, 300, &w);
        let t3 = graph.belief_trust(&d3, 300, &w);
        // d3 无冲突且无衰减 → 仲裁必选 d3
        let active = graph.active_arbitrated_facts(300, &w);
        assert_eq!(active[&("k".to_string(), "p".to_string())].object, "v3");
        // κ 冲突惩罚: d1 (κ=1, 冲突 0) > d3 (κ=0.5, 冲突 1), 尽管 d3 更新
        assert!(t1 > t3, "冲突惩罚使旧无冲突版本信任更高");
        // δ 衰减: 同为冲突 1, 更新的 d3 比 d2 少衰减
        assert!(t3 > t2);
        // δ 衰减: 同一事实在更晚时刻 trust 下降
        let t_early = graph.belief_trust(&d3, 300, &w);
        let t_late = graph.belief_trust(&d3, 300_000, &w);
        assert!(t_late < t_early);
        // 来源权重: Reflection < Manual
        let mut g2 = BitemporalGraph::new();
        let r = g2.insert_fact_full("k2", "p2", "v", 1.0, 0, None, 100, FactProvenance::Reflection);
        let m = g2.insert_fact_full("k3", "p3", "v", 1.0, 0, None, 100, FactProvenance::Manual);
        assert!(g2.belief_trust(&r, 200, &w) < g2.belief_trust(&m, 200, &w));
    }

    /// serde 旧数据兼容: 无信念字段的 JSON 可反序列化 + normalize 退化等价。
    #[test]
    fn legacy_json_deserializes_and_normalizes() {
        let old_json = r#"{"id":"fact_1_1","subject":"s","predicate":"p","object":"o","rev":1,"valid_at_ms":1000,"invalid_at_ms":null,"importance":0.5}"#;
        let mut f: BitemporalFact = serde_json::from_str(old_json).unwrap();
        assert_eq!(f.belief_at_ms, 0);
        f.normalize_legacy_belief();
        assert_eq!(f.belief_at_ms, 1000);
    }
}
