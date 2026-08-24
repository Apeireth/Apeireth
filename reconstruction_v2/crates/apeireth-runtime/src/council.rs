//! Council - 多代理决策层 (从 v1.0 apeireth-council 16,488 LOC 收敛)
//!
//! 0 装 PASS: 重构版 council 不再是独立 crate (peer to UnifiedRuntimeHost),
//! 而是 runtime 子模块, 用 v2 现有 apeireth_companion / apeireth_storage 替代 v1.0 自建依赖。
//!
//! 设计 (per 用户右图 "Companion 智能核" + "Governance"):
//! - 7 个 Advisor 各有 bias (Facilitator/Strategist/Empiricist/Critic/Ethicist/Skeptic/Mystic)
//! - CouncilBus: 多 advisor 异步投票, 用 tokio::sync::mpsc 跨 advisor
//! - ConsensusRule: 5 种 (Unanimous/Majority/Plurality/Supermajority/Weighted)
//! - Deliberation: 议题 → 投票 → 决议 → 审计

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};

/// 7 advisors 角色 (per v1.0 apeireth-council/src/advisor.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorRole {
    Facilitator,    // 引导讨论, 主持流程
    Strategist,      // 长期视角
    Empiricist,      // 实证 / 数据驱动
    Critic,          // 风险评估
    Ethicist,        // 价值观对齐
    Skeptic,         // 怀疑论, 反驳
    Mystic,          // 直觉 / 灵感
}

impl AdvisorRole {
    pub const ALL: &'static [AdvisorRole] = &[
        Self::Facilitator, Self::Strategist, Self::Empiricist,
        Self::Critic, Self::Ethicist, Self::Skeptic, Self::Mystic,
    ];

    /// advisor 权重 (0..1) — Facilitator 主持权低, Empiricist 数据权高
    pub fn weight(self) -> f64 {
        match self {
            Self::Facilitator => 0.7,
            Self::Strategist => 0.9,
            Self::Empiricist => 1.0,
            Self::Critic => 0.85,
            Self::Ethicist => 0.9,
            Self::Skeptic => 0.75,
            Self::Mystic => 0.5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Facilitator => "facilitator",
            Self::Strategist => "strategist",
            Self::Empiricist => "empiricist",
            Self::Critic => "critic",
            Self::Ethicist => "ethicist",
            Self::Skeptic => "skeptic",
            Self::Mystic => "mystic",
        }
    }
}

/// Advisor 投票
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorVote {
    pub role: AdvisorRole,
    pub position: VotePosition,
    pub confidence: f64, // 0..1
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VotePosition {
    Approve,
    Reject,
    Abstain,
    RequestInfo,
}

/// 共识规则
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusRule {
    Unanimous,        // 7 票全 Approve
    Supermajority,    // 6+/7 Approve
    Majority,         // 4+/7 Approve
    Plurality,        // 最多票胜
    Weighted,         // weight*confidence 总和 >= 5.0 (7 advisors * 0.7 avg)
}

impl ConsensusRule {
    /// 0 装 PASS: 真实计算 (不用 placeholder)
    pub fn evaluate(&self, votes: &[AdvisorVote]) -> bool {
        if votes.is_empty() { return false; }
        match self {
            Self::Unanimous => votes.iter().all(|v| v.position == VotePosition::Approve),
            Self::Supermajority => votes.iter()
                .filter(|v| v.position == VotePosition::Approve).count() >= 6,
            Self::Majority => votes.iter()
                .filter(|v| v.position == VotePosition::Approve).count() >= 4,
            Self::Plurality => {
                let approve = votes.iter().filter(|v| v.position == VotePosition::Approve).count();
                let reject = votes.iter().filter(|v| v.position == VotePosition::Reject).count();
                approve > reject
            },
            Self::Weighted => votes.iter()
                .filter(|v| v.position == VotePosition::Approve)
                .map(|v| v.role.weight() * v.confidence)
                .sum::<f64>() >= 5.0,
        }
    }
}

/// 议题 + 决议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposer: String,
    pub rule: ConsensusRule,
    pub votes: Vec<AdvisorVote>,
    pub decided: bool,
    pub outcome: Option<bool>, // Some(true) = passed, Some(false) = rejected
}

impl Proposal {
    pub fn new(id: String, title: String, description: String, proposer: String, rule: ConsensusRule) -> Self {
        Self { id, title, description, proposer, rule, votes: Vec::new(), decided: false, outcome: None }
    }

    pub fn cast(&mut self, vote: AdvisorVote) {
        if self.decided { return; }
        // last vote wins per role (advisor 可改票)
        self.votes.retain(|v| v.role != vote.role);
        self.votes.push(vote);
    }

    /// 评估决议
    pub fn decide(&mut self) -> bool {
        if self.decided { return false; }
        let passed = self.rule.evaluate(&self.votes);
        self.decided = true;
        self.outcome = Some(passed);
        passed
    }
}

/// CouncilBus - 多 advisor 异步通信
#[derive(Clone)]
pub struct CouncilBus {
    sender: mpsc::UnboundedSender<Proposal>,
    receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<Proposal>>>>,
}

impl CouncilBus {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { sender: tx, receiver: Arc::new(RwLock::new(Some(rx))) }
    }

    pub fn submit(&self, proposal: Proposal) -> Result<(), &'static str> {
        self.sender.send(proposal).map_err(|_| "CouncilBus closed")
    }

    /// 非阻塞取下一个议题
    pub async fn try_recv(&self) -> Option<Proposal> {
        let mut rx_opt = self.receiver.write().await;
        match rx_opt.as_mut() {
            Some(rx) => rx.try_recv().ok(),
            None => None,
        }
    }
}

impl Default for CouncilBus {
    fn default() -> Self { Self::new() }
}

/// Council - 7 advisors + bus 编排 (0 装 PASS: 单实例 7 advisors)
pub struct Council {
    pub bus: CouncilBus,
    pub proposals: Arc<RwLock<HashMap<String, Proposal>>>,
    pub history: Arc<RwLock<Vec<String>>>, // 已决 ids (按时间)
}

impl Council {
    pub fn new() -> Self {
        Self {
            bus: CouncilBus::new(),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 提出议题 (异步)
    pub async fn propose(&self, proposal: Proposal) {
        self.proposals.write().await.insert(proposal.id.clone(), proposal.clone());
        let _ = self.bus.submit(proposal);
    }

    /// 给定议题投票 (异步)
    pub async fn vote(&self, proposal_id: &str, vote: AdvisorVote) -> Result<bool, &'static str> {
        let mut proposals = self.proposals.write().await;
        let p = proposals.get_mut(proposal_id).ok_or("proposal not found")?;
        p.cast(vote);
        if p.votes.len() >= AdvisorRole::ALL.len() {
            // 7 票齐了, 自动决议
            let passed = p.decide();
            self.history.write().await.push(proposal_id.to_string());
            Ok(passed)
        } else {
            Ok(false)
        }
    }
}

impl Default for Council {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_7_advisors() {
        assert_eq!(AdvisorRole::ALL.len(), 7);
        assert_eq!(AdvisorRole::Empiricist.weight(), 1.0);
    }

    #[test]
    fn test_unanimous_rule() {
        let votes: Vec<AdvisorVote> = AdvisorRole::ALL.iter().map(|r| AdvisorVote {
            role: *r, position: VotePosition::Approve,
            confidence: 0.8, rationale: "OK".into(),
        }).collect();
        assert!(ConsensusRule::Unanimous.evaluate(&votes));
        let mut mixed = votes;
        mixed[0].position = VotePosition::Reject;
        assert!(!ConsensusRule::Unanimous.evaluate(&mixed));
    }

    #[test]
    fn test_weighted_rule() {
        let votes = vec![AdvisorVote {
            role: AdvisorRole::Empiricist, position: VotePosition::Approve,
            confidence: 1.0, rationale: "data supports".into(),
        }, AdvisorVote {
            role: AdvisorRole::Critic, position: VotePosition::Approve,
            confidence: 1.0, rationale: "low risk".into(),
        }, AdvisorVote {
            role: AdvisorRole::Ethicist, position: VotePosition::Approve,
            confidence: 1.0, rationale: "values align".into(),
        }];
        // weight*confidence: 1.0*1.0 + 0.85*1.0 + 0.9*1.0 = 2.75 < 5.0
        assert!(!ConsensusRule::Weighted.evaluate(&votes));
    }

    #[tokio::test]
    async fn test_council_voting_flow() {
        let council = Council::new();
        let p = Proposal::new(
            "p1".into(), "Test proposal".into(), "desc".into(),
            "user".into(), ConsensusRule::Unanimous,
        );
        council.propose(p).await;
        // 7 票全 Approve
        for role in AdvisorRole::ALL {
            council.vote("p1", AdvisorVote {
                role: *role, position: VotePosition::Approve,
                confidence: 0.9, rationale: "OK".into(),
            }).await.unwrap();
        }
        let p = council.proposals.read().await;
        let p = p.get("p1").unwrap();
        assert!(p.decided);
        assert_eq!(p.outcome, Some(true));
    }
}
