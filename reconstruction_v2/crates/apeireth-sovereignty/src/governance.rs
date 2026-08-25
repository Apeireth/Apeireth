//! 5 重治理 orchestrator

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mewg::{
    Decision, DefaultMewgAuthority, EvidenceSource, MewgAuthority, MewgEvidence, MewgVerdict,
};
use crate::multi_ai::{AiConsensus, AiProvider, MultiAiConsensus};
use crate::multi_human::{HumanVoteOutcome, HumanVoter};
use crate::physical_multisig::{MultisigOutcome, PhysicalMultisig};
use crate::reflection::{ReflectionClock, ReflectionState};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GovernanceOutcome {
    Approved { mewg_score: f64, rationale: String },
    Blocked { failed_at: GovernanceStep, reason: String },
    PendingReview { waiting_at: GovernanceStep, state: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceStep {
    MultiAi,
    MultiHuman,
    PhysicalMultisig,
    Reflection,
    Mewg,
}

impl GovernanceStep {
    pub fn name(&self) -> &'static str {
        match self {
            GovernanceStep::MultiAi => "MultiAi",
            GovernanceStep::MultiHuman => "MultiHuman",
            GovernanceStep::PhysicalMultisig => "PhysicalMultisig",
            GovernanceStep::Reflection => "Reflection",
            GovernanceStep::Mewg => "Mewg",
        }
    }
}

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Governance {
    pub mewg: Arc<dyn MewgAuthority>,
    pub multi_ai: Arc<tokio::sync::Mutex<MultiAiConsensus>>,
    pub multi_human: Arc<tokio::sync::Mutex<dyn HumanVoter>>,
    pub physical: Arc<tokio::sync::Mutex<dyn PhysicalMultisig>>,
    pub reflection: Arc<tokio::sync::Mutex<dyn ReflectionClock>>,
    pub reflection_period: Duration,
}

impl std::fmt::Debug for Governance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Governance")
            .field("mewg_id", &self.mewg.authority_id())
            .field("reflection_period", &self.reflection_period)
            .finish()
    }
}

impl Default for Governance {
    fn default() -> Self {
        use crate::multi_human::InMemoryHumanVoter;
        use crate::physical_multisig::InMemoryPhysicalMultisig;
        use crate::reflection::InMemoryReflectionClock;
        Self {
            mewg: Arc::new(DefaultMewgAuthority::new()),
            multi_ai: Arc::new(tokio::sync::Mutex::new(MultiAiConsensus::new())),
            multi_human: Arc::new(tokio::sync::Mutex::new(InMemoryHumanVoter::new())),
            physical: Arc::new(tokio::sync::Mutex::new(InMemoryPhysicalMultisig::new())),
            reflection: Arc::new(tokio::sync::Mutex::new(InMemoryReflectionClock::new())),
            reflection_period: crate::reflection::DEFAULT_REFLECTION_PERIOD,
        }
    }
}

impl Governance {
    pub fn with_reflection_period(mut self, period: Duration) -> Self {
        self.reflection_period = period; self
    }
    pub fn with_mewg(mut self, authority: Arc<dyn MewgAuthority>) -> Self {
        self.mewg = authority; self
    }
    pub async fn register_ai_provider(&self, provider: Box<dyn AiProvider>) -> Result<(), crate::multi_ai::MultiAiError> {
        let mut consensus = self.multi_ai.lock().await;
        consensus.register(provider)
    }

    pub async fn process(&self, decision: &Decision) -> Result<GovernanceOutcome, GovernanceError> {
        let summary = format!("{}: {}", decision.title, decision.description);
        let verdicts = {
            let consensus = self.multi_ai.lock().await;
            consensus.poll(&summary).await
        };
        let ai_consensus = MultiAiConsensus::aggregate(&verdicts);
        let ai_score = match &ai_consensus {
            AiConsensus::Unanimous { providers, avg_confidence } => (providers.len() as f64) * avg_confidence,
            AiConsensus::Partial { approve, .. } => approve.len() as f64,
            AiConsensus::Rejected { reject, .. } => {
                return Ok(GovernanceOutcome::Blocked {
                    failed_at: GovernanceStep::MultiAi,
                    reason: format!("多 AI 一致失败: {} 个 AI 反对", reject.len()),
                });
            }
            AiConsensus::Insufficient { verdict_count } => {
                return Ok(GovernanceOutcome::PendingReview {
                    waiting_at: GovernanceStep::MultiAi,
                    state: format!("AI 票数不足 ({}/3)", verdict_count),
                });
            }
        };

        let human_outcome = {
            let voter = self.multi_human.lock().await;
            voter.tally()
        };
        let human_score = match &human_outcome {
            HumanVoteOutcome::Approved { approve_count, .. } => *approve_count as f64,
            HumanVoteOutcome::Rejected { reason, .. } => {
                return Ok(GovernanceOutcome::Blocked {
                    failed_at: GovernanceStep::MultiHuman,
                    reason: reason.clone(),
                });
            }
            HumanVoteOutcome::InsufficientVotes { approve_count, .. } => {
                return Ok(GovernanceOutcome::PendingReview {
                    waiting_at: GovernanceStep::MultiHuman,
                    state: format!("多人投票不足 ({}/2 approve)", approve_count),
                });
            }
        };

        let multisig_outcome = {
            let m = self.physical.lock().await;
            m.tally()
        };
        let multisig_score = match &multisig_outcome {
            MultisigOutcome::Approved { signature_count, .. } => *signature_count as f64,
            MultisigOutcome::Rejected { reason, .. } => {
                return Ok(GovernanceOutcome::Blocked {
                    failed_at: GovernanceStep::PhysicalMultisig,
                    reason: reason.clone(),
                });
            }
            MultisigOutcome::PendingSignatures { collected, required } => {
                return Ok(GovernanceOutcome::PendingReview {
                    waiting_at: GovernanceStep::PhysicalMultisig,
                    state: format!("物理多签等待 ({}/{})", collected, required),
                });
            }
        };

        {
            let mut clock = self.reflection.lock().await;
            clock.begin_with_period(&decision.id, self.reflection_period, decision.description.clone())
                .map_err(|e| GovernanceError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())))?;
            let now = chrono::Utc::now().timestamp();
            clock.tick(now).map_err(|e| GovernanceError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())))?;
        }
        let reflect_state = {
            let clock = self.reflection.lock().await;
            clock.state_of(&decision.id)
        };
        if reflect_state != Some(ReflectionState::AwaitingResolution) && reflect_state != Some(ReflectionState::Reflecting) {
            return Ok(GovernanceOutcome::Blocked {
                failed_at: GovernanceStep::Reflection,
                reason: format!("反思期异常状态: {:?}", reflect_state),
            });
        }
        if reflect_state == Some(ReflectionState::Reflecting) {
            return Ok(GovernanceOutcome::PendingReview {
                waiting_at: GovernanceStep::Reflection,
                state: "反思期进行中 (默认 7 天)".into(),
            });
        }

        let mut evidences = Vec::new();
        if ai_score > 0.0 {
            evidences.push(MewgEvidence::new("ai", EvidenceSource::MultiAi, (ai_score / 3.0).clamp(-1.0, 1.0), 0.3, format!("多 AI 一致 = {}", ai_score)).unwrap());
        }
        if human_score > 0.0 {
            evidences.push(MewgEvidence::new("human", EvidenceSource::MultiHuman, (human_score / 2.0).clamp(-1.0, 1.0), 0.3, format!("多人投票 = {}", human_score)).unwrap());
        }
        if multisig_score > 0.0 {
            evidences.push(MewgEvidence::new("physical", EvidenceSource::PhysicalMultisig, (multisig_score / 2.0).clamp(-1.0, 1.0), 0.2, format!("物理多签 = {}", multisig_score)).unwrap());
        }
        evidences.push(MewgEvidence::new("reflection", EvidenceSource::Reflection, 1.0, 0.2, "反思期已完成".to_string()).unwrap());

        let mewg_verdict = self.mewg.evaluate(decision, &evidences)
            .map_err(|e| GovernanceError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())))?;
        match mewg_verdict {
            MewgVerdict::Approved { weighted_score, rationale } => Ok(GovernanceOutcome::Approved { mewg_score: weighted_score, rationale }),
            MewgVerdict::Blocked { reason, .. } => Ok(GovernanceOutcome::Blocked { failed_at: GovernanceStep::Mewg, reason }),
            MewgVerdict::PendingReview { state, .. } => Ok(GovernanceOutcome::PendingReview { waiting_at: GovernanceStep::Mewg, state }),
        }
    }

    pub async fn process_owner_decision(&self, request: &crate::owner::OwnerRequest) -> Result<GovernanceOutcome, GovernanceError> {
        if !request.touches_e_layer() {
            let decision = Decision {
                id: request.id.clone(),
                title: format!("OwnerAction:{}", request.action.as_str()),
                description: format!("[{}] {} — {}", request.token.as_str(), request.action.as_str(), request.reason),
                touches_e_layer: false,
                tags: vec![format!("owner:{}", request.token.as_str())],
                submitted_at: request.submitted_at / 1000,
                metadata: Some(serde_json::json!({
                    "owner_token": request.token.as_str(),
                    "owner_action": request.action.as_str(),
                    "touches_e_layer": false,
                })),
            };
            return self.process(&decision).await;
        }
        if !request.token.can_attempt_core_rule() {
            return Ok(GovernanceOutcome::Blocked {
                failed_at: GovernanceStep::MultiAi,
                reason: format!("Q13: OwnerToken::{} 无权触及 core-rule", request.token.as_str()),
            });
        }
        let decision = Decision {
            id: request.id.clone(),
            title: format!("OwnerCoreRule:{}", request.action.as_str()),
            description: format!("[{}] {} — {}", request.token.as_str(), request.action.as_str(), request.reason),
            touches_e_layer: true,
            tags: vec![
                format!("owner:{}", request.token.as_str()),
                format!("core_rule:{}", request.action.as_str()),
            ],
            submitted_at: request.submitted_at / 1000,
            metadata: Some(serde_json::json!({
                "owner_token": request.token.as_str(),
                "owner_action": request.action.as_str(),
                "touches_e_layer": true,
            })),
        };
        let outcome = self.process(&decision).await?;
        match &outcome {
            GovernanceOutcome::Approved { mewg_score, rationale } => Ok(GovernanceOutcome::Approved {
                mewg_score: *mewg_score,
                rationale: format!("{}
[Q13 owner_token={} action={} touches_e_layer=true]", rationale, request.token.as_str(), request.action.as_str()),
            }),
            other => Ok(other.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_ai::{AiStance, MockAiProvider};
    use crate::multi_human::{HumanId, InMemoryHumanVoter, Vote};
    use crate::physical_multisig::{InMemoryPhysicalMultisig, PhysicalSignerId};

    fn dec(id: &str, touches_e: bool) -> Decision {
        Decision { id: id.into(), title: format!("d{}", id), description: "x".into(), touches_e_layer: touches_e, tags: vec![], submitted_at: 0, metadata: None }
    }

    #[tokio::test]
    async fn full_approval_path() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve))).await.unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve))).await.unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve))).await.unwrap();
        {
            let mut v = gov.multi_human.lock().await;
            v.register(HumanId::new("alice", "A", "owner"));
            v.register(HumanId::new("bob", "B", "co-owner"));
            v.cast_vote("alice", Vote::Approve, "y".to_string()).unwrap();
            v.cast_vote("bob", Vote::Approve, "y".to_string()).unwrap();
        }
        {
            let mut m = gov.physical.lock().await;
            m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
            m.register(PhysicalSignerId::new("p1", "phone", "bob"));
            m.collect_signature("y1", "d".to_string(), true).unwrap();
            m.collect_signature("p1", "d".to_string(), false).unwrap();
        }
        let out = gov.process(&dec("d1", false)).await.unwrap();
        assert!(matches!(out, GovernanceOutcome::Approved { .. }), "got {:?}", out);
    }

    #[tokio::test]
    async fn blocked_on_ai_rejection() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve))).await.unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Reject))).await.unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Reject))).await.unwrap();
        let out = gov.process(&dec("d1", false)).await.unwrap();
        match out {
            GovernanceOutcome::Blocked { failed_at, .. } => assert_eq!(failed_at, GovernanceStep::MultiAi),
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn pending_when_human_insufficient() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve))).await.unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve))).await.unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve))).await.unwrap();
        {
            let mut v = gov.multi_human.lock().await;
            v.register(HumanId::new("alice", "A", "owner"));
            v.register(HumanId::new("bob", "B", "co-owner"));
            v.cast_vote("alice", Vote::Approve, "y".to_string()).unwrap();
        }
        let out = gov.process(&dec("d1", false)).await.unwrap();
        match out {
            GovernanceOutcome::PendingReview { waiting_at, .. } => assert_eq!(waiting_at, GovernanceStep::MultiHuman),
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn pending_when_ai_insufficient() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve))).await.unwrap();
        let out = gov.process(&dec("d1", false)).await.unwrap();
        match out {
            GovernanceOutcome::PendingReview { waiting_at, .. } => assert_eq!(waiting_at, GovernanceStep::MultiAi),
            _ => panic!(),
        }
    }

    #[test]
    fn step_names() {
        assert_eq!(GovernanceStep::MultiAi.name(), "MultiAi");
        assert_eq!(GovernanceStep::MultiHuman.name(), "MultiHuman");
        assert_eq!(GovernanceStep::PhysicalMultisig.name(), "PhysicalMultisig");
        assert_eq!(GovernanceStep::Reflection.name(), "Reflection");
        assert_eq!(GovernanceStep::Mewg.name(), "Mewg");
    }

}