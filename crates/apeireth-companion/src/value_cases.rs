//! `apeireth-companion::value_cases` — F6 价值内化 (案例库 + 裁决记录 + 主人反馈回流).
//!
//! ## 哲学 (road-to-asi 原型五, 主人确认"设计过未实施")
//!
//! 宪法是规则表, 规则总有未覆盖的情况 — 价值内化 = 在规则沉默处, 凭对
//! "主人意图与长期福祉"的理解做决定. 渐进内化: **规则 → 案例 → 判断**.
//!
//! 本模块是案例层: 价值冲突场景 → 裁决记录 → 主人反馈回流 →
//! 同一模式多次一致 → 提升为原则候选 (回喂动态原则层, 0 装: 提升由调用方决定).
//!
//! 与情感记忆 (F1) 同一块地: F1 记"主人此刻的状态", F6 学"对你重要的事".
//!
//! ## 机制 (确定性, 无 LLM)
//!
//! - **ValueCaseStore**: 案例库 (场景 + 冲突价值 + 裁决 + 依据 + 反馈).
//! - **feedback(id, Agree/Disagree)**: 主人反馈回流 — 同意加权, 不同意降权.
//! - **promote_candidates(n)**: 同一冲突模式多次一致 (含主人同意) → 原则候选.
//! - **decision_for(scenario)**: 相似案例 (冲突价值集合匹配) 优先决策参照.

/// 裁决依据 (诚实标注来源).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionBasis {
    /// 宪法规则直接覆盖.
    ConstitutionRule,
    /// 智囊团审议 (council 7 advisor).
    CouncilDeliberation,
    /// 主人亲自裁决 (最高依据).
    MasterDecision,
}

/// 主人反馈 (回流信号).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    Agree,
    Disagree,
}

/// 一个价值案例.
#[derive(Debug, Clone)]
pub struct ValueCase {
    pub id: u64,
    /// 冲突场景描述 ("是否替主人拒绝高风险工具调用").
    pub scenario: String,
    /// 冲突的价值集合 (确定性排序后比较).
    pub values: Vec<String>,
    pub decision: String,
    pub basis: DecisionBasis,
    /// 主人反馈: None = 未回流, Some = 已回流.
    pub feedback: Option<Feedback>,
    /// 同意计数 (含主人同意 + 多次一致).
    pub agree_count: usize,
    pub at_ms: i64,
}

/// 价值案例库 (确定性).
#[derive(Debug, Default)]
pub struct ValueCaseStore {
    cases: Vec<ValueCase>,
    next_id: u64,
}

impl ValueCaseStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次裁决 (案例入库).
    pub fn record(
        &mut self,
        scenario: impl Into<String>,
        values: Vec<String>,
        decision: impl Into<String>,
        basis: DecisionBasis,
    ) -> ValueCase {
        let mut values = values;
        values.sort(); // 确定性: 冲突集合排序后比较
        values.dedup();
        let case = ValueCase {
            id: self.next_id,
            scenario: scenario.into(),
            values,
            decision: decision.into(),
            basis,
            feedback: None,
            agree_count: 0,
            at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.next_id += 1;
        self.cases.push(case.clone());
        case
    }

    /// 主人反馈回流: Agree → agree_count+1; Disagree → 标记 + 计 0 (不被提升).
    pub fn feedback(&mut self, id: u64, fb: Feedback) -> Result<(), String> {
        let c = self.cases.iter_mut().find(|c| c.id == id).ok_or("案例不存在")?;
        c.feedback = Some(fb);
        if fb == Feedback::Agree {
            c.agree_count += 1;
        }
        Ok(())
    }

    /// 提升候选: 同一冲突价值集合的模式, 多次一致 (agree_count ≥ threshold) → 原则候选.
    /// 返回 (冲突集合, 一致裁决, 同意次数) — 提升动作由调用方决定 (0 装).
    pub fn promote_candidates(&self, threshold: usize) -> Vec<(Vec<String>, String, usize)> {
        let mut groups: std::collections::HashMap<Vec<String>, Vec<&ValueCase>> = Default::default();
        for c in &self.cases {
            if c.feedback != Some(Feedback::Disagree) {
                groups.entry(c.values.clone()).or_default().push(c);
            }
        }
        let mut out = Vec::new();
        for (values, cases) in groups {
            let agree: usize = cases.iter().map(|c| c.agree_count).sum();
            if agree >= threshold {
                let decision = cases[0].decision.clone();
                out.push((values, decision, agree));
            }
        }
        out.sort();
        out
    }

    /// 相似案例检索: 冲突价值集合完全匹配的案例 (决策参照).
    pub fn decision_for(&self, values: &[String]) -> Option<&ValueCase> {
        let mut key = values.to_vec();
        key.sort();
        key.dedup();
        self.cases.iter().rev().find(|c| c.values == key)
    }

    /// 场景检索 (关键词包含).
    pub fn recall(&self, keyword: &str) -> Vec<&ValueCase> {
        self.cases
            .iter()
            .filter(|c| c.scenario.contains(keyword))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.cases.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_recall_by_keyword() {
        let mut store = ValueCaseStore::new();
        store.record(
            "是否替主人拒绝高风险工具调用",
            vec!["安全".into(), "自主".into()],
            "拒绝, 等主人批准",
            DecisionBasis::ConstitutionRule,
        );
        assert_eq!(store.len(), 1);
        let hits = store.recall("高风险");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].decision.contains("拒绝"));
        assert_eq!(store.recall("不存在").len(), 0);
    }

    #[test]
    fn feedback_flow_back_agree_counts() {
        let mut store = ValueCaseStore::new();
        let c = store.record(
            "是否继续熬夜工作",
            vec!["健康".into(), "进度".into()],
            "劝主人休息",
            DecisionBasis::CouncilDeliberation,
        );
        store.feedback(c.id, Feedback::Agree).unwrap();
        store.feedback(c.id, Feedback::Agree).unwrap();
        let cands = store.promote_candidates(2);
        assert_eq!(cands.len(), 1, "2 次同意 → 提升候选");
        assert_eq!(cands[0].1, "劝主人休息");
    }

    #[test]
    fn disagree_blocks_promotion() {
        let mut store = ValueCaseStore::new();
        let c = store.record(
            "场景X",
            vec!["a".into(), "b".into()],
            "决定A",
            DecisionBasis::MasterDecision,
        );
        store.feedback(c.id, Feedback::Disagree).unwrap();
        assert!(store.promote_candidates(1).is_empty(), "主人不同意 → 不提升");
    }

    #[test]
    fn decision_for_matches_value_set() {
        let mut store = ValueCaseStore::new();
        store.record(
            "场景1",
            vec!["安全".into(), "速度".into()],
            "安全优先",
            DecisionBasis::ConstitutionRule,
        );
        // 值集合乱序传入 → 排序后匹配
        let d = store.decision_for(&["速度".into(), "安全".into()]).unwrap();
        assert_eq!(d.decision, "安全优先");
        // 不同值集合不匹配
        assert!(store.decision_for(&["速度".into()]).is_none());
    }

    #[test]
    fn values_sorted_deduped_deterministic() {
        let mut store = ValueCaseStore::new();
        let c = store.record(
            "s",
            vec!["b".into(), "a".into(), "b".into()],
            "d",
            DecisionBasis::ConstitutionRule,
        );
        assert_eq!(c.values, vec!["a".to_string(), "b".to_string()], "排序 + 去重");
    }
}
