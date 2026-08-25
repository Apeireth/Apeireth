//! 多人投票 — ≥2 真实人类 trait + Rust mock

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanId {
    pub id: String,
    pub name: String,
    pub role: String,
}

impl HumanId {
    pub fn new(id: impl Into<String>, name: impl Into<String>, role: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into(), role: role.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanVote {
    pub voter: HumanId,
    pub vote: Vote,
    pub rationale: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HumanVoteOutcome {
    Approved { approve_count: usize, reject_count: usize, abstain_count: usize },
    Rejected { approve_count: usize, reject_count: usize, reason: String },
    InsufficientVotes { approve_count: usize, reject_count: usize },
}

#[derive(Debug, Error)]
pub enum HumanVoteError {
    #[error("voter `{0}` not registered")]
    UnknownVoter(String),
    #[error("voter `{0}` already voted")]
    DuplicateVote(String),
}

pub trait HumanVoter: Send + Sync {
    fn register(&mut self, human: HumanId);
    fn cast_vote(&mut self, voter_id: &str, vote: Vote, rationale: String) -> Result<HumanVote, HumanVoteError>;
    fn tally(&self) -> HumanVoteOutcome;
    fn registered_count(&self) -> usize;
    fn vote_count(&self) -> usize;
    fn has_quorum(&self) -> bool {
        let outcome = self.tally();
        matches!(outcome, HumanVoteOutcome::Approved { approve_count, .. } if approve_count >= 2)
    }
}

#[derive(Debug, Default)]
pub struct InMemoryHumanVoter {
    registered: Vec<HumanId>,
    votes: Vec<HumanVote>,
}

impl InMemoryHumanVoter {
    pub fn new() -> Self { Self::default() }
    pub fn with_population(humans: Vec<HumanId>) -> Self {
        let mut v = Self::new();
        for h in humans { v.register(h); }
        v
    }
}

impl HumanVoter for InMemoryHumanVoter {
    fn register(&mut self, human: HumanId) {
        if !self.registered.iter().any(|h| h.id == human.id) { self.registered.push(human); }
    }
    fn cast_vote(&mut self, voter_id: &str, vote: Vote, rationale: String) -> Result<HumanVote, HumanVoteError> {
        let human = self.registered.iter().find(|h| h.id == voter_id).cloned()
            .ok_or_else(|| HumanVoteError::UnknownVoter(voter_id.into()))?;
        if self.votes.iter().any(|v| v.voter.id == voter_id) {
            return Err(HumanVoteError::DuplicateVote(voter_id.into()));
        }
        let vote_record = HumanVote { voter: human, vote, rationale, timestamp: chrono::Utc::now().timestamp() };
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
            return HumanVoteOutcome::Rejected { approve_count: approve, reject_count: reject, reason: format!("{} 个真实人类反对", reject) };
        }
        if approve < 2 {
            return HumanVoteOutcome::InsufficientVotes { approve_count: approve, reject_count: reject };
        }
        HumanVoteOutcome::Approved { approve_count: approve, reject_count: reject, abstain_count: abstain }
    }
    fn registered_count(&self) -> usize { self.registered.len() }
    fn vote_count(&self) -> usize { self.votes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn alice() -> HumanId { HumanId::new("alice", "Alice", "owner") }
    fn bob() -> HumanId { HumanId::new("bob", "Bob", "co-owner") }
    fn carol() -> HumanId { HumanId::new("carol", "Carol", "witness") }

    #[test]
    fn requires_two_approves() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice());
        v.register(bob());
        v.cast_vote("alice", Vote::Approve, "yes".to_string()).unwrap();
        match v.tally() {
            HumanVoteOutcome::InsufficientVotes { approve_count, .. } => assert_eq!(approve_count, 1),
            _ => panic!("should be InsufficientVotes"),
        }
        v.cast_vote("bob", Vote::Approve, "yes".to_string()).unwrap();
        match v.tally() {
            HumanVoteOutcome::Approved { approve_count, .. } => assert_eq!(approve_count, 2),
            _ => panic!("should be Approved"),
        }
    }
    #[test]
    fn reject_overrides_approves() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice()); v.register(bob()); v.register(carol());
        v.cast_vote("alice", Vote::Approve, "y".to_string()).unwrap();
        v.cast_vote("bob", Vote::Approve, "y".to_string()).unwrap();
        v.cast_vote("carol", Vote::Reject, "n".to_string()).unwrap();
        assert!(matches!(v.tally(), HumanVoteOutcome::Rejected { .. }));
    }
    #[test]
    fn rejects_duplicate_vote() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice()); v.register(bob());
        v.cast_vote("alice", Vote::Approve, "yes".to_string()).unwrap();
        assert!(matches!(v.cast_vote("alice", Vote::Approve, "again".to_string()),
            Err(HumanVoteError::DuplicateVote(_))));
    }
    #[test]
    fn rejects_unknown_voter() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice());
        assert!(matches!(v.cast_vote("eve", Vote::Approve, "x".to_string()),
            Err(HumanVoteError::UnknownVoter(_))));
    }
    #[test]
    fn abstain_increments_abstain_count() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice()); v.register(bob()); v.register(carol());
        v.cast_vote("alice", Vote::Approve, "y".to_string()).unwrap();
        v.cast_vote("bob", Vote::Approve, "y".to_string()).unwrap();
        v.cast_vote("carol", Vote::Abstain, "skip".to_string()).unwrap();
        match v.tally() {
            HumanVoteOutcome::Approved { approve_count, abstain_count, .. } => {
                assert_eq!(approve_count, 2);
                assert_eq!(abstain_count, 1);
            }
            _ => panic!(),
        }
    }
    #[test]
    fn with_population_constructor() {
        let v = InMemoryHumanVoter::with_population(vec![alice(), bob()]);
        assert_eq!(v.registered_count(), 2);
        assert_eq!(v.vote_count(), 0);
    }
    #[test]
    fn has_quorum_when_2_approved() {
        let mut v = InMemoryHumanVoter::new();
        v.register(alice()); v.register(bob());
        v.cast_vote("alice", Vote::Approve, "y".to_string()).unwrap();
        assert!(!v.has_quorum());
        v.cast_vote("bob", Vote::Approve, "y".to_string()).unwrap();
        assert!(v.has_quorum());
    }
    #[test]
    fn registered_count_and_vote_count() {
        let mut v = InMemoryHumanVoter::new();
        assert_eq!(v.registered_count(), 0);
        v.register(alice());
        assert_eq!(v.registered_count(), 1);
        v.cast_vote("alice", Vote::Approve, "y".to_string()).unwrap();
        assert_eq!(v.vote_count(), 1);
    }
}
