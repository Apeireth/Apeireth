//! 多人投票 — ≥2 真实人类 trait + Rust mock
//!
//! **设计** (阶段 1 §18.6 + 阶段 2 D2 §9):
//! - 关键操作需 ≥2 真实人类批准 (多人多签)
//! - trait 抽象 — 不依赖外部 SDK (LDAP / OAuth / Web3)
//! - Rust mock 实现: 内存中存一组真实人类 + 投票记录
//!
//! **硬约束**: 不模拟外部身份服务; 测试用 `InMemoryHumanRegistry` 即可

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 人类投票
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    /// 同意
    Approve,
    /// 反对
    Reject,
    /// 弃权
    Abstain,
}

/// 真实人类身份
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanId {
    /// 唯一 ID
    pub id: String,
    /// 显示名
    pub name: String,
    /// 角色 (owner / co-owner / witness / ...)
    pub role: String,
}

impl HumanId {
    /// 新建身份
    pub fn new(id: impl Into<String>, name: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role: role.into(),
        }
    }
}

/// 单个人类投票记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanVote {
    /// 投票人
    pub voter: HumanId,
    /// 投票 (approve/reject/abstain)
    pub vote: Vote,
    /// 投票理由
    pub rationale: String,
    /// 投票时间 (epoch seconds)
    pub timestamp: i64,
}

/// 多人投票裁决
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HumanVoteOutcome {
    /// 通过 (≥2 approve, 无 reject)
    Approved {
        approve_count: usize,
        reject_count: usize,
        abstain_count: usize,
    },
    /// 拒绝 (≥1 reject)
    Rejected {
        approve_count: usize,
        reject_count: usize,
        reason: String,
    },
    /// 票数不足 (<2 approve)
    InsufficientVotes {
        approve_count: usize,
        reject_count: usize,
    },
}

/// 多人投票错误
#[derive(Debug, Error)]
pub enum HumanVoteError {
    #[error("voter `{0}` not registered")]
    UnknownVoter(String),
    #[error("voter `{0}` already voted")]
    DuplicateVote(String),
}

/// HumanVoter trait — 抽象多人投票接口
///
/// **不变量**:
/// - 至少需要 ≥2 真实人类投票 (≥1 approve + ≥1 approve 才算通过)
/// - 任何 reject 立即导致 Rejected (无须凑够多数)
/// - 同一 voter 不可重复投票
///
/// **dyn 兼容性**: 不使用 generic 方法 — `rationale` 参数为 `String` 而非 `impl Into<String>`
pub trait HumanVoter: Send + Sync {
    /// 注册一个真实人类
    fn register(&mut self, human: HumanId);

    /// 投票 (每人仅一次)
    fn cast_vote(
        &mut self,
        voter_id: &str,
        vote: Vote,
        rationale: String,
    ) -> Result<HumanVote, HumanVoteError>;

    /// 聚合投票结果
    fn tally(&self) -> HumanVoteOutcome;

    /// 已注册的人类数
    fn registered_count(&self) -> usize;

    /// 已投票数
    fn vote_count(&self) -> usize;

    /// 是否达到最低票数 (≥2 approve + ≥1 reject 都没有)
    fn has_quorum(&self) -> bool {
        let outcome = self.tally();
        matches!(outcome, HumanVoteOutcome::Approved { approve_count, .. } if approve_count >= 2)
    }
}

/// 内存 mock 实现 — 测试 / 单进程运行用
#[derive(Debug, Default)]
pub struct InMemoryHumanVoter {
    registered: Vec<HumanId>,
    votes: Vec<HumanVote>,
}

impl InMemoryHumanVoter {
    /// 新建空 voter
    pub fn new() -> Self {
        Self::default()
    }

    /// 一次性注册 N 个人类
    pub fn with_population(humans: Vec<HumanId>) -> Self {
        let mut v = Self::new();
        for h in humans {
            v.register(h);
        }
        v
    }
}

impl HumanVoter for InMemoryHumanVoter {
    fn register(&mut self, human: HumanId) {
        if !self.registered.iter().any(|h| h.id == human.id) {
            self.registered.push(human);
        }
    }

    fn cast_vote(
        &mut self,
        voter_id: &str,
        vote: Vote,
        rationale: String,
    ) -> Result<HumanVote, HumanVoteError> {
        let human = self
            .registered
            .iter()
            .find(|h| h.id == voter_id)
            .cloned()
            .ok_or_else(|| HumanVoteError::UnknownVoter(voter_id.into()))?;
        if self.votes.iter().any(|v| v.voter.id == voter_id) {
            return Err(HumanVoteError::DuplicateVote(voter_id.into()));
        }
        let vote_record = HumanVote {
            voter: human,
            vote,
            rationale,
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.votes.push(vote_record.clone());
        Ok(vote_record)
    }

    fn tally(&self) -> HumanVoteOutcome {
        let mut approve = 0;
        let mut reject = 0;
        let mut abstain = 0;
        for v in &self.votes {
            match v.vote {
                Vote::Approve => approve += 1,
                Vote::Reject => reject += 1,
                Vote::Abstain => abstain += 1,
            }
        }
        if reject > 0 {
            return HumanVoteOutcome::Rejected {
                approve_count: approve,
                reject_count: reject,
                reason: format!("{reject} 个真实人类反对"),
            };
        }
        if approve < 2 {
            return HumanVoteOutcome::InsufficientVotes {
                approve_count: approve,
                reject_count: reject,
            };
        }
        HumanVoteOutcome::Approved {
            approve_count: approve,
            reject_count: reject,
            abstain_count: abstain,
        }
    }

    fn registered_count(&self) -> usize {
        self.registered.len()
    }

    fn vote_count(&self) -> usize {
        self.votes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> HumanId {
        HumanId::new("alice", "Alice", "owner")
    }
    fn bob() -> HumanId {
        HumanId::new("bob", "Bob", "co-owner")
    }
    fn carol() -> HumanId {
        HumanId::new("carol", "Carol", "witness")
    }

    #[test]
    fn multi_human_requires_two_approves() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice());
        v.register(bob());
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        // 只有 1 approve → InsufficientVotes
        match v.tally() {
            HumanVoteOutcome::InsufficientVotes { approve_count, .. } => {
                assert_eq!(approve_count, 1);
            }
            _ => panic!("should be InsufficientVotes"),
        }
        v.cast_vote("bob", Vote::Approve, "yes".to_string())
            .unwrap();
        match v.tally() {
            HumanVoteOutcome::Approved { approve_count, .. } => {
                assert_eq!(approve_count, 2);
            }
            _ => panic!("should be Approved"),
        }
    }

    #[test]
    fn multi_human_reject_overrides_approves() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice());
        v.register(bob());
        v.register(carol());
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("bob", Vote::Approve, "yes".to_string())
            .unwrap();
        v.cast_vote("carol", Vote::Reject, "no".to_string())
            .unwrap();
        assert!(matches!(v.tally(), HumanVoteOutcome::Rejected { .. }));
    }

    #[test]
    fn multi_human_rejects_duplicate_vote() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice());
        v.register(bob());
        v.cast_vote("alice", Vote::Approve, "yes".to_string())
            .unwrap();
        assert!(matches!(
            v.cast_vote("alice", Vote::Approve, "again".to_string()),
            Err(HumanVoteError::DuplicateVote(_))
        ));
    }

    #[test]
    fn multi_human_rejects_unknown_voter() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice());
        assert!(matches!(
            v.cast_vote("eve", Vote::Approve, "x".to_string()),
            Err(HumanVoteError::UnknownVoter(_))
        ));
    }
}
