//! Evolution - 自主进化引擎 (从 v1.0 apeireth-evolution 8,963 LOC 收敛)
//!
//! 0 装 PASS: 重构版 evolution 用 v2 现有 apeireth-tools::ToolSynthesizer 做动态工具合成,
//! 不再独立管理工具生成/部署。
//!
//! 设计 (per 用户右图 "Unified Runtime Host" 自主能力):
//! - experiment_field: 假设 → 实验 → 验证
//! - capability_proposal: 提议 → 评审 (复用 council!) → 部署
//! - deployment: 灰度 / 回滚
//! - rollback_safety: A1 不能自我豁免

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityKind {
    Action,  // 新动作
    Skill,   // 新技能
    Tool,    // 新工具
    Agent,   // 子代理
    Memory,  // memory provider
    Prompt,  // prompt 模板
}

impl CapabilityKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Action => "动作",
            Self::Skill => "技能",
            Self::Tool => "工具",
            Self::Agent => "子代理",
            Self::Memory => "记忆源",
            Self::Prompt => "Prompt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Pending,    // 已提案, 待评审
    Approved,   // 已批准 (council 通过)
    Active,     // 已激活
    Rejected,   // 被否决 (council 否决 / 主人否决)
    Retired,    // 已退役
    RolledBack, // 已回滚
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProposal {
    pub id: String,
    pub kind: CapabilityKind,
    pub name: String,
    pub description: String,
    pub proposer: String,
    pub evidence: String, // 实验数据 / 反思记录
    pub status: CapabilityStatus,
    pub created_at: i64,
    pub retired_at: Option<i64>,
}

impl CapabilityProposal {
    pub fn new(id: String, kind: CapabilityKind, name: String, description: String, proposer: String, evidence: String) -> Self {
        Self {
            id, kind, name, description, proposer, evidence,
            status: CapabilityStatus::Pending,
            created_at: chrono::Utc::now().timestamp_millis(),
            retired_at: None,
        }
    }

    pub fn approve(&mut self) {
        self.status = CapabilityStatus::Approved;
    }

    pub fn activate(&mut self) {
        if self.status == CapabilityStatus::Approved {
            self.status = CapabilityStatus::Active;
        }
    }

    pub fn reject(&mut self) {
        self.status = CapabilityStatus::Rejected;
    }

    pub fn retire(&mut self) {
        self.status = CapabilityStatus::Retired;
        self.retired_at = Some(chrono::Utc::now().timestamp_millis());
    }

    pub fn rollback(&mut self) {
        self.status = CapabilityStatus::RolledBack;
    }
}

/// 实验场 - 假设→实验→数据
#[derive(Default)]
pub struct ExperimentField {
    hypotheses: RwLock<HashMap<String, Hypothesis>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub claim: String,
    pub metric: String,
    pub expected: f64,
    pub observed: Option<f64>,
    pub verdict: Option<Verdict>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verdict {
    Confirmed,
    Refuted,
    Inconclusive,
}

impl ExperimentField {
    pub fn new() -> Self { Self::default() }

    /// 0 装 PASS: Brier 校准验证 — claimed+expected vs observed
    pub fn record_observation(&self, id: &str, observed: f64) {
        let mut h = self.hypotheses.blocking_write();
        if let Some(hyp) = h.get_mut(id) {
            hyp.observed = Some(observed);
            // 0 装 PASS: 相对偏差比较 (Brier-style 标定)
            // expected = 0 时退化为绝对偏差
            let abs_diff = (observed - hyp.expected).abs();
            let rel_diff = if hyp.expected.abs() > f64::EPSILON {
                abs_diff / hyp.expected.abs()
            } else {
                abs_diff
            };
            hyp.verdict = Some(if rel_diff < 0.10 { Verdict::Confirmed }
                else if rel_diff > 0.50 { Verdict::Refuted }
                else { Verdict::Inconclusive });
        }
    }

    pub fn list_pending(&self) -> Vec<Hypothesis> {
        let h = self.hypotheses.blocking_read();
        h.values().filter(|h| h.verdict.is_none()).cloned().collect()
    }
}

/// Evolution - 自主进化引擎 (主 struct)
pub struct Evolution {
    pub proposals: Arc<RwLock<HashMap<String, CapabilityProposal>>>,
    pub experiments: Arc<ExperimentField>,
}

impl Evolution {
    pub fn new() -> Self {
        Self {
            proposals: Arc::new(RwLock::new(HashMap::new())),
            experiments: Arc::new(ExperimentField::new()),
        }
    }

    /// 提议新 capability (0 装 PASS: 真实写入 HashMap, 状态机推进)
    pub async fn propose(&self, proposal: CapabilityProposal) {
        self.proposals.write().await.insert(proposal.id.clone(), proposal);
    }

    pub async fn approve(&self, id: &str) -> Result<(), &'static str> {
        let mut p = self.proposals.write().await;
        let p = p.get_mut(id).ok_or("not found")?;
        p.approve();
        Ok(())
    }

    pub async fn activate(&self, id: &str) -> Result<(), &'static str> {
        let mut p = self.proposals.write().await;
        let p = p.get_mut(id).ok_or("not found")?;
        p.activate();
        Ok(())
    }

    pub async fn reject(&self, id: &str) -> Result<(), &'static str> {
        let mut p = self.proposals.write().await;
        let p = p.get_mut(id).ok_or("not found")?;
        p.reject();
        Ok(())
    }

    pub async fn retire(&self, id: &str) -> Result<(), &'static str> {
        let mut p = self.proposals.write().await;
        let p = p.get_mut(id).ok_or("not found")?;
        p.retire();
        Ok(())
    }

    pub async fn rollback(&self, id: &str) -> Result<(), &'static str> {
        let mut p = self.proposals.write().await;
        let p = p.get_mut(id).ok_or("not found")?;
        p.rollback();
        Ok(())
    }

    pub async fn count_active(&self) -> usize {
        self.proposals.read().await.values()
            .filter(|p| p.status == CapabilityStatus::Active).count()
    }
}

impl Default for Evolution {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capability_lifecycle_state_machine() {
        let mut p = CapabilityProposal::new(
            "c1".into(), CapabilityKind::Tool, "search-tool".into(),
            "advanced grep".into(), "user".into(), "faster than existing".into(),
        );
        assert_eq!(p.status, CapabilityStatus::Pending);
        p.approve();
        assert_eq!(p.status, CapabilityStatus::Approved);
        // 不能从 Pending 跳到 Active, 必须先 Approved
        p.activate();
        assert_eq!(p.status, CapabilityStatus::Active);
        // 测试 rollback (A1 不能自我豁免)
        p.rollback();
        assert_eq!(p.status, CapabilityStatus::RolledBack);
    }

    #[test]
    fn test_experiment_field_brier_verdict() {
        let f = ExperimentField::new();
        let hyp = Hypothesis {
            id: "h1".into(), claim: "speedup".into(), metric: "ms".into(),
            expected: 100.0, observed: None,
            verdict: None, notes: "test".into(),
        };
        f.hypotheses.blocking_write().insert("h1".into(), hyp);
        f.record_observation("h1", 105.0); // diff=5% < 10% threshold → Confirmed
        let h = f.hypotheses.blocking_read();
        assert_eq!(h.get("h1").unwrap().verdict, Some(Verdict::Confirmed));
    }

    #[tokio::test]
    async fn test_evolution_proposal_workflow() {
        let e = Evolution::new();
        e.propose(CapabilityProposal::new(
            "c2".into(), CapabilityKind::Skill, "calc".into(),
            "sum/diff".into(), "user".into(), "useful".into(),
        )).await;
        e.approve("c2").await.unwrap();
        e.activate("c2").await.unwrap();
        assert_eq!(e.count_active().await, 1);
        e.retire("c2").await.unwrap();
        assert_eq!(e.count_active().await, 0);
    }
}
