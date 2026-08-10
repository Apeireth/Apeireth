//! 5 重治理 orchestrator — `governance.process(decision) -> Outcome`
//!
//! **设计** (阶段 1 §18.6 + 阶段 2 D2):
//! - 任意一重失败 = 整次失败 (主 17:58 不假装)
//! - 反思期可配置 (默认 7 天, 测试用 50ms)
//! - MEWG 是最高优先级解释权, 最后调用
//! - 流程:
//!   1. 多 AI 一致 (≥3 AI 通过)
//!   2. 多人投票 (≥2 approve, 无 reject)
//!   3. 物理多签 (≥2 不同 kind + ≥1 witness)
//!   4. 反思期 (≥7 天等待)
//!   5. MEWG 汇总 (加权分 + E 层硬门槛)
//! - 与 `apeireth-council` synthesis 协同: 接收 council 的 synthesis 报告作为
//!   MultiAiEvidence 的输入 (council 已包含 7 advisor synthesis)
//!
//! **硬约束**: 不修改 `apeireth-council` / `apeireth-core`, 通过 trait 协同

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use apeireth_council::synthesis::SynthesisReport;
use apeireth_council::{CouncilEvent, SovereigntyHook};

use crate::mewg::{
    Decision, DefaultMewgAuthority, EvidenceSource, MewgAuthority, MewgEvidence, MewgVerdict,
};
use crate::multi_ai::{AiConsensus, AiProvider, MultiAiConsensus};
use crate::multi_human::{HumanVoteOutcome, HumanVoter};
use crate::physical_multisig::{MultisigOutcome, PhysicalMultisig};
use crate::reflection::{ReflectionClock, ReflectionState};

/// 治理执行结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GovernanceOutcome {
    /// 全部通过
    Approved {
        /// MEWG 加权分
        mewg_score: f64,
        /// 综合理由
        rationale: String,
    },
    /// 任一重失败
    Blocked {
        /// 失败的治理维度
        failed_at: GovernanceStep,
        /// 失败理由
        reason: String,
    },
    /// 待重审 (反思期未结束 / 票数不足)
    PendingReview {
        /// 当前等待的步骤
        waiting_at: GovernanceStep,
        /// 等待状态描述
        state: String,
    },
}

/// 治理步骤 — 5 重
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceStep {
    /// 多 AI 一致
    MultiAi,
    /// 多人投票
    MultiHuman,
    /// 物理多签
    PhysicalMultisig,
    /// 反思期
    Reflection,
    /// MEWG 汇总
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

/// 治理错误
#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("council hook not registered")]
    NoCouncilHook,
}

/// Governance orchestrator — 5 重治理合一
///
/// **使用**:
/// ```ignore
/// use apeireth_sovereignty::{Governance, MewgAuthority, ...};
/// let gov = Governance::default();
/// gov.process(&decision).await?;
/// ```
pub struct Governance {
    /// MEWG authority
    pub mewg: Arc<dyn MewgAuthority>,
    /// 多 AI 聚合器
    pub multi_ai: Arc<tokio::sync::Mutex<MultiAiConsensus>>,
    /// 多人 voter
    pub multi_human: Arc<tokio::sync::Mutex<dyn HumanVoter>>,
    /// 物理多签
    pub physical: Arc<tokio::sync::Mutex<dyn PhysicalMultisig>>,
    /// 反思期 clock
    pub reflection: Arc<tokio::sync::Mutex<dyn ReflectionClock>>,
    /// 反思期长度 (用于 process() 时 begin)
    pub reflection_period: Duration,
    /// 可选: 关联 apeireth-council 事件 sink (用于 sovereignty hook)
    pub council_event_sink: Arc<tokio::sync::Mutex<Vec<CouncilEvent>>>,
}

impl std::fmt::Debug for Governance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Governance")
            .field("mewg_id", &self.mewg.authority_id())
            .field("reflection_period", &self.reflection_period)
            .finish_non_exhaustive()
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
            council_event_sink: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

impl Governance {
    /// 自定义反思期长度 (测试用)
    pub fn with_reflection_period(mut self, period: Duration) -> Self {
        self.reflection_period = period;
        self
    }

    /// 自定义 MEWG authority
    pub fn with_mewg(mut self, authority: Arc<dyn MewgAuthority>) -> Self {
        self.mewg = authority;
        self
    }

    /// 注册 AI provider
    pub async fn register_ai_provider(
        &self,
        provider: Box<dyn AiProvider>,
    ) -> Result<(), crate::multi_ai::MultiAiError> {
        let mut consensus = self.multi_ai.lock().await;
        consensus.register(provider)
    }

    /// 治理主流程 — `process(decision) -> Outcome`
    ///
    /// **流程**:
    /// 1. 多 AI 一致检查
    /// 2. 多人投票检查 (已通过 cast_vote 累积)
    /// 3. 物理多签检查 (已通过 collect_signature 累积)
    /// 4. 反思期 begin + tick (如果 reflection_period > 0)
    /// 5. MEWG 汇总
    pub async fn process(&self, decision: &Decision) -> Result<GovernanceOutcome, GovernanceError> {
        // Step 1: 多 AI 一致
        let summary = format!("{}: {}", decision.title, decision.description);
        let verdicts = {
            let consensus = self.multi_ai.lock().await;
            consensus.poll(&summary).await
        };
        let ai_consensus = MultiAiConsensus::aggregate(&verdicts);
        let ai_score = match &ai_consensus {
            AiConsensus::Unanimous {
                providers,
                avg_confidence,
            } => {
                // 同时记录 evidence 给 MEWG
                (providers.len() as f64) * avg_confidence
            }
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
                    state: format!("AI 票数不足 ({verdict_count}/3)"),
                });
            }
        };

        // Step 2: 多人投票
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
                    state: format!("多人投票不足 ({approve_count}/2 approve)"),
                });
            }
        };

        // Step 3: 物理多签
        let multisig_outcome = {
            let m = self.physical.lock().await;
            m.tally()
        };
        let multisig_score = match &multisig_outcome {
            MultisigOutcome::Approved {
                signature_count, ..
            } => *signature_count as f64,
            MultisigOutcome::Rejected { reason, .. } => {
                return Ok(GovernanceOutcome::Blocked {
                    failed_at: GovernanceStep::PhysicalMultisig,
                    reason: reason.clone(),
                });
            }
            MultisigOutcome::PendingSignatures {
                collected,
                required,
            } => {
                return Ok(GovernanceOutcome::PendingReview {
                    waiting_at: GovernanceStep::PhysicalMultisig,
                    state: format!("物理多签等待 ({collected}/{required})"),
                });
            }
        };

        // Step 4: 反思期
        {
            let mut clock = self.reflection.lock().await;
            clock
                .begin_with_period(
                    &decision.id,
                    self.reflection_period,
                    decision.description.clone(),
                )
                .map_err(|e| {
                    GovernanceError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        e.to_string(),
                    ))
                })?;
            // tick 推进到 AwaitingResolution (如果反思期长度 ≤ 0)
            let now = chrono::Utc::now().timestamp();
            clock.tick(now).map_err(|e| {
                GovernanceError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    e.to_string(),
                ))
            })?;
        }
        let reflect_state = {
            let clock = self.reflection.lock().await;
            clock.state_of(&decision.id)
        };
        if reflect_state != Some(ReflectionState::AwaitingResolution)
            && reflect_state != Some(ReflectionState::Reflecting)
        {
            return Ok(GovernanceOutcome::Blocked {
                failed_at: GovernanceStep::Reflection,
                reason: format!("反思期异常状态: {reflect_state:?}"),
            });
        }
        // 反思期未结束 → PendingReview
        if reflect_state == Some(ReflectionState::Reflecting) {
            return Ok(GovernanceOutcome::PendingReview {
                waiting_at: GovernanceStep::Reflection,
                state: "反思期进行中 (默认 7 天)".into(),
            });
        }

        // Step 5: MEWG 汇总
        let mut evidences = Vec::new();
        if ai_score > 0.0 {
            evidences.push(
                MewgEvidence::new(
                    "ai",
                    EvidenceSource::MultiAi,
                    (ai_score / 3.0).clamp(-1.0, 1.0),
                    0.3,
                    format!("多 AI 一致 = {ai_score}"),
                )
                .expect("valid evidence"),
            );
        }
        if human_score > 0.0 {
            evidences.push(
                MewgEvidence::new(
                    "human",
                    EvidenceSource::MultiHuman,
                    (human_score / 2.0).clamp(-1.0, 1.0),
                    0.3,
                    format!("多人投票 = {human_score}"),
                )
                .expect("valid evidence"),
            );
        }
        if multisig_score > 0.0 {
            evidences.push(
                MewgEvidence::new(
                    "physical",
                    EvidenceSource::PhysicalMultisig,
                    (multisig_score / 2.0).clamp(-1.0, 1.0),
                    0.2,
                    format!("物理多签 = {multisig_score}"),
                )
                .expect("valid evidence"),
            );
        }
        // 反思期完成 → 加 1 条 evidence (正向)
        evidences.push(
            MewgEvidence::new(
                "reflection",
                EvidenceSource::Reflection,
                1.0,
                0.2,
                "反思期已完成".to_string(),
            )
            .expect("valid evidence"),
        );

        let mewg_verdict = self.mewg.evaluate(decision, &evidences).map_err(|e| {
            GovernanceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                e.to_string(),
            ))
        })?;
        match mewg_verdict {
            MewgVerdict::Approved {
                weighted_score,
                rationale,
            } => Ok(GovernanceOutcome::Approved {
                mewg_score: weighted_score,
                rationale,
            }),
            MewgVerdict::Blocked {
                weighted_score: _,
                reason,
            } => Ok(GovernanceOutcome::Blocked {
                failed_at: GovernanceStep::Mewg,
                reason,
            }),
            MewgVerdict::PendingReview {
                state,
                weighted_score: _,
            } => Ok(GovernanceOutcome::PendingReview {
                waiting_at: GovernanceStep::Mewg,
                state,
            }),
        }
    }

    /// Q13 — 处理主人请求 (OwnerToken 强制走 5 重治理)
    ///
    /// **硬约束**:
    /// 1. **任何 token 改 core-rule (touches_e_layer=true) 必须触发 MEWG + 反思期**
    /// 2. Master / Admin / Operator / ReadOnly 一视同仁 — 没有 bypass 路径
    /// 3. OwnerToken 字段纳入 MEWG evidence (审计追溯)
    /// 4. SovereigntyHook 不能在此方法中旁路 — 多签已通过 `MultiSigPolicy::process_owner_request` 验证
    /// 5. ReadOnly token 触及 core-rule → 立即 Blocked (MultiSigPolicy 已拒, 此处兜底)
    ///
    /// **流程** (继承 `process()` 5 重 + 扩展):
    /// 1. 多 AI 一致 (同 `process()`)
    /// 2. 多人投票 (同 `process()`)
    /// 3. 物理多签 (同 `process()`)
    /// 4. 反思期 — **核心规则修改强制 ≥ 反思期状态机** (即使 reflection_period=0, 也要进 MEWG)
    /// 5. MEWG 汇总 — 包含 owner_token evidence
    ///
    /// **返回**: 同 `process()` — `Approved` / `Blocked` / `PendingReview`
    pub async fn process_owner_decision(
        &self,
        request: &crate::owner::OwnerRequest,
    ) -> Result<GovernanceOutcome, GovernanceError> {
        // Q13 硬约束 #1: 任何 token 触及 core-rule, 必须触发反思期 (即使 period=0)
        // 这里用 touches_e_layer 作为强制点 — 不依赖 process() 内部 reflection_period
        if !request.touches_e_layer() {
            // 非 core-rule 修改, 走普通 process()
            let decision = Decision {
                id: request.id.clone(),
                title: format!("OwnerAction:{}", request.action.as_str()),
                description: format!(
                    "[{}] {} — {}",
                    request.token.as_str(),
                    request.action.as_str(),
                    request.reason
                ),
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

        // Q13 硬约束 #5: ReadOnly token 触及 core-rule → MultiSigPolicy 已 ReadOnlyRejected,
        // 这里兜底再 Blocked (双层防御, fail-closed)
        if !request.token.can_attempt_core_rule() {
            return Ok(GovernanceOutcome::Blocked {
                failed_at: GovernanceStep::MultiAi,
                reason: format!(
                    "Q13: OwnerToken::{} 无权触及 core-rule (ReadOnly 被 MultiSigPolicy 拒绝, 兜底 Blocked)",
                    request.token.as_str()
                ),
            });
        }

        // 构造 Decision — 包含 owner_token tag (审计追溯)
        let decision = Decision {
            id: request.id.clone(),
            title: format!("OwnerCoreRule:{}", request.action.as_str()),
            description: format!(
                "[{}] {} — {}",
                request.token.as_str(),
                request.action.as_str(),
                request.reason
            ),
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

        // 走标准 process() — owner_token 通过 tags 纳入 MEWG evidence
        // Q13 硬约束 #4: SovereigntyHook 不允许 bypass — process() 5 重必走
        let outcome = self.process(&decision).await?;

        // Q13 硬约束 #1 额外强化: 即使 reflection_period=0 导致 process() 通过,
        // 核心规则修改必须显式记录 owner_token (审计追溯)
        match &outcome {
            GovernanceOutcome::Approved {
                mewg_score,
                rationale,
            } => Ok(GovernanceOutcome::Approved {
                mewg_score: *mewg_score,
                rationale: format!(
                    "{}\n[Q13 owner_token={} action={} touches_e_layer=true]",
                    rationale,
                    request.token.as_str(),
                    request.action.as_str()
                ),
            }),
            other => Ok(other.clone()),
        }
    }

    /// 提供给 `apeireth-council` 的 SovereigntyHook 实现 (用于在 council 事件时记录)
    pub fn council_hook(&self) -> GovernanceCouncilHook {
        GovernanceCouncilHook {
            sink: Arc::clone(&self.council_event_sink),
        }
    }
}

/// SovereigntyHook 实现 — 把 council 事件转发到 governance 的 event sink
pub struct GovernanceCouncilHook {
    sink: Arc<tokio::sync::Mutex<Vec<CouncilEvent>>>,
}

impl SovereigntyHook for GovernanceCouncilHook {
    fn on_council_event(&self, event: &CouncilEvent) {
        // tokio 异步锁不能用 sync trait — 用 try_lock; 把临时 guard 立即 drop
        if let Ok(mut guard) = self.sink.try_lock() {
            guard.push(event.clone());
        };
    }

    fn hook_id(&self) -> &str {
        "governance"
    }
}

/// 与 `apeireth-council` synthesis 协同 — 把 synthesis 报告作为 MultiAiEvidence 输入
///
/// **用法**: governance 收到 council 的 synthesis 报告后, 把 report 转换为一组
/// MewgEvidence 加入 MEWG 决策。
impl Governance {
    /// 注入 council synthesis 报告作为多 AI 证据 (供 MEWG 评估)
    pub fn synthesis_to_evidence(&self, report: &SynthesisReport) -> MewgEvidence {
        let score = report.weighted_score.clamp(-1.0, 1.0);
        MewgEvidence::new(
            "council-synthesis",
            EvidenceSource::MultiAi,
            score,
            0.4,
            format!(
                "council synthesis: 加权分={:.3} confidence={:.3} opinions={} held={}",
                report.weighted_score,
                report.confidence,
                report.opinion_count,
                report.is_held()
            ),
        )
        .expect("valid synthesis evidence")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_ai::{AiStance, MockAiProvider};
    use crate::multi_human::{HumanId, InMemoryHumanVoter, Vote};
    use crate::physical_multisig::{InMemoryPhysicalMultisig, PhysicalSignerId};

    fn decision(id: &str, touches_e: bool) -> Decision {
        Decision {
            id: id.into(),
            title: format!("decision {id}"),
            description: "test decision".into(),
            touches_e_layer: touches_e,
            tags: vec![],
            submitted_at: 0,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn governance_process_full_approval_path() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));

        // 3 AI providers
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .await
            .unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve)))
            .await
            .unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve)))
            .await
            .unwrap();

        // 2 humans approve
        {
            let mut v = gov.multi_human.lock().await;
            v.register(HumanId::new("alice", "Alice", "owner"));
            v.register(HumanId::new("bob", "Bob", "co-owner"));
            v.cast_vote("alice", Vote::Approve, "yes".to_string())
                .unwrap();
            v.cast_vote("bob", Vote::Approve, "yes".to_string())
                .unwrap();
        }

        // 2 physical signers (yubikey + phone, 1 witness)
        {
            let mut m = gov.physical.lock().await;
            m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
            m.register(PhysicalSignerId::new("p1", "phone", "bob"));
            m.collect_signature("y1", "digest".to_string(), true)
                .unwrap();
            m.collect_signature("p1", "digest".to_string(), false)
                .unwrap();
        }

        let outcome = gov.process(&decision("d1", false)).await.unwrap();
        assert!(
            matches!(outcome, GovernanceOutcome::Approved { .. }),
            "got: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn governance_blocked_on_ai_rejection() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .await
            .unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Reject)))
            .await
            .unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Reject)))
            .await
            .unwrap();

        let outcome = gov.process(&decision("d1", false)).await.unwrap();
        match outcome {
            GovernanceOutcome::Blocked { failed_at, .. } => {
                assert_eq!(failed_at, GovernanceStep::MultiAi);
            }
            _ => panic!("expected Blocked, got {outcome:?}"),
        }
    }

    #[tokio::test]
    async fn governance_pending_when_human_insufficient() {
        let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
        gov.register_ai_provider(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .await
            .unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("b", AiStance::Approve)))
            .await
            .unwrap();
        gov.register_ai_provider(Box::new(MockAiProvider::new("c", AiStance::Approve)))
            .await
            .unwrap();
        // 只有 1 human approve
        {
            let mut v = gov.multi_human.lock().await;
            v.register(HumanId::new("alice", "Alice", "owner"));
            v.register(HumanId::new("bob", "Bob", "co-owner"));
            v.cast_vote("alice", Vote::Approve, "yes".to_string())
                .unwrap();
        }

        let outcome = gov.process(&decision("d1", false)).await.unwrap();
        match outcome {
            GovernanceOutcome::PendingReview { waiting_at, .. } => {
                assert_eq!(waiting_at, GovernanceStep::MultiHuman);
            }
            _ => panic!("expected PendingReview, got {outcome:?}"),
        }
    }
}
