//! 与 council 的集成桥 (v2 stub — local types only)
//!
//! v1 era 通过 apeireth_council 提供 CouncilEvent 等类型, v2 因 apeireth-council
//! v2 port 不完整, 用本地等效类型替代. 公开 API 表面 1:1 保留.

use crate::engine::{EvolutionEngine, EvolutionStep};
use crate::fail::{FailKind, FailOutcome};
use crate::{EvolutionError, EvolutionResult};
use serde::{Deserialize, Serialize};

/// 默认最大 retry 轮次 (与 council MAX_PERSONA_DEBATE_ROUNDS=3 留余量)。
pub const DEFAULT_MAX_RETRY_ROUNDS: u32 = 3;

/// 默认反思窗口 (与 council HOLD_DELIBERATION_TIMEOUT_MS=60_000 一致)。
pub const DEFAULT_REFLECTION_WINDOW_MS: u64 = 60_000;

/// 演化提案描述 (提交给 council 前的载体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionProposal {
    /// 提案 ID
    pub proposal_id: String,
    /// 描述
    pub description: String,
    /// 目标层
    pub target_layer: String,
    /// 风险等级
    pub risk: String,
}

impl EvolutionProposal {
    pub fn new(
        proposal_id: impl Into<String>,
        description: impl Into<String>,
        target_layer: impl Into<String>,
        risk: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            description: description.into(),
            target_layer: target_layer.into(),
            risk: risk.into(),
        }
    }

    /// 是否触及 L0 (硬件锚定层)。
    pub fn targets_l0(&self) -> bool {
        self.target_layer.eq_ignore_ascii_case("L0")
    }
}

/// CouncilAdapter 集成配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouncilIntegrationConfig {
    pub max_retry: u32,
    pub reflection_window_ms: u64,
}

impl Default for CouncilIntegrationConfig {
    fn default() -> Self {
        Self {
            max_retry: DEFAULT_MAX_RETRY_ROUNDS,
            reflection_window_ms: DEFAULT_REFLECTION_WINDOW_MS,
        }
    }
}

/// 演化裁决产出 (CouncilAdapter 翻译完 verdict 后返回)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionOutcome {
    Ratified,
    Retried { attempt: u32 },
    Rejected { reason: String },
    SovereigntyAdjudicated { released: bool },
    L0Guard,
    Ignored,
}

/// 本地仿造 CouncilDomain (替代 apeireth_council::AdvisorDomain)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorDomain {
    Safety,
    Legal,
    Performance,
    Strategy,
    Philosophy,
    Ethics,
}

/// 本地仿造 CouncilId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdvisorId(pub u32);

/// 本地仿造 Stance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StanceKind {
    For,
    Against,
    Conditional,
    Abstain,
}

/// 本地仿造 AdvisorOpinion
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisorOpinion {
    pub opinion_id: String,
    pub author_id: AdvisorId,
    pub domain: AdvisorDomain,
    pub stance: Stance,
    pub confidence: f32,
    pub rationale: String,
}

/// 本地仿造 Stance
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stance {
    pub kind: StanceKind,
    pub confidence: f32,
}

/// 本地仿造 CouncilVerdict
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CouncilVerdict {
    Approved,
    Held { trigger: String },
    Rejected { reason: String },
}

/// 本地仿造 HoldTrigger
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoldTrigger {
    pub trigger_id: String,
    pub reason: String,
}

/// 本地仿造 HoldThreshold
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HoldThreshold {
    pub min_against: u32,
    pub max_risk: RiskClass,
}

/// 本地仿造 RiskClass
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

/// 本地仿造 HoldOutcome
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HoldOutcome {
    Released,
    Sustained { reason: String },
}

/// 本地仿造 CouncilEvent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CouncilEvent {
    DeliberationStarted { session_id: String, query_id: String },
    OpinionIssued { opinion: AdvisorOpinion },
    HoldTriggered { trigger: HoldTrigger },
    SovereigntyAdjudicated { released: bool },
    DeliberationCompleted { verdict: CouncilVerdict },
}

/// 本地仿造 SynthesisReport
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisReport {
    pub report_id: String,
    pub opinions: Vec<AdvisorOpinion>,
    pub verdict: CouncilVerdict,
}

/// CouncilAdapter — v2 stub (本地实现, 0 真接 apeireth-council)
pub struct CouncilAdapter {
    pub config: CouncilIntegrationConfig,
    pub engine: EvolutionEngine,
    pub retry_count: u32,
}

impl CouncilAdapter {
    pub fn new(engine: EvolutionEngine) -> Self {
        Self {
            config: CouncilIntegrationConfig::default(),
            engine,
            retry_count: 0,
        }
    }

    pub fn with_config(engine: EvolutionEngine, config: CouncilIntegrationConfig) -> Self {
        Self { config, engine, retry_count: 0 }
    }

    /// 处理 council event, 翻译成 evolution action
    pub fn on_event(&mut self, event: &CouncilEvent) -> EvolutionResult<EvolutionOutcome> {
        match event {
            CouncilEvent::DeliberationCompleted { verdict } => {
                match verdict {
                    CouncilVerdict::Approved => Ok(EvolutionOutcome::Ratified),
                    CouncilVerdict::Held { .. } => {
                        self.retry_count += 1;
                        if self.retry_count > self.config.max_retry {
                            Ok(EvolutionOutcome::Rejected {
                                reason: "max retry reached".into(),
                            })
                        } else {
                            Ok(EvolutionOutcome::Retried {
                                attempt: self.retry_count,
                            })
                        }
                    }
                    CouncilVerdict::Rejected { reason } => Ok(EvolutionOutcome::Rejected {
                        reason: reason.clone(),
                    }),
                }
            }
            CouncilEvent::SovereigntyAdjudicated { released } => {
                Ok(EvolutionOutcome::SovereigntyAdjudicated { released: *released })
            }
            CouncilEvent::HoldTriggered { .. } => Ok(EvolutionOutcome::L0Guard),
            _ => Ok(EvolutionOutcome::Ignored),
        }
    }

    /// L0 锚定护栏
    pub fn check_l0(&self, proposal: &EvolutionProposal) -> bool {
        proposal.targets_l0()
    }
}

/// synthesize stub — 返回基于 opinions 的简单 report
pub fn synthesize(opinions: &[AdvisorOpinion]) -> SynthesisReport {
    let verdict = if opinions.iter().any(|o| matches!(o.stance.kind, StanceKind::Against)) {
        CouncilVerdict::Rejected {
            reason: "at least one against".into(),
        }
    } else {
        CouncilVerdict::Approved
    };
    SynthesisReport {
        report_id: format!("syn-{}", opinions.len()),
        opinions: opinions.to_vec(),
        verdict,
    }
}

/// SynthesisWeights stub
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SynthesisWeights {
    pub stance_weight: f32,
    pub confidence_weight: f32,
    pub domain_weight: f32,
}

impl Default for SynthesisWeights {
    fn default() -> Self {
        Self {
            stance_weight: 1.0,
            confidence_weight: 1.0,
            domain_weight: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolution_proposal_l0_detection() {
        let p = EvolutionProposal::new("p1", "d", "L0", "high");
        assert!(p.targets_l0());
        let p2 = EvolutionProposal::new("p1", "d", "L1", "low");
        assert!(!p2.targets_l0());
    }

    #[test]
    fn council_integration_config_defaults() {
        let c = CouncilIntegrationConfig::default();
        assert_eq!(c.max_retry, DEFAULT_MAX_RETRY_ROUNDS);
        assert_eq!(c.reflection_window_ms, DEFAULT_REFLECTION_WINDOW_MS);
    }

    #[test]
    fn adapter_releases_after_adjudication() {
        let engine = EvolutionEngine::new("test-prop", crate::fail::StrictFailPolicy);
        let mut adapter = CouncilAdapter::new(engine);
        let event = CouncilEvent::SovereigntyAdjudicated { released: true };
        let out = adapter.on_event(&event).unwrap();
        assert!(matches!(out, EvolutionOutcome::SovereigntyAdjudicated { released: true }));
    }
}