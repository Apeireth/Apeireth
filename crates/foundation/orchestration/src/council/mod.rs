//! Council 真实现 (RC-6) — 7 LlmAdvisor 并行 + 60s timeout + DeferToHuman.

use std::sync::Arc;
use std::time::Duration;

use crate::llm::LlmFactory;
use crate::{Advisor, AdvisorKind, AdvisorVerdict, CouncilVerdict, Proposal};

pub mod advisors_llm;

pub use advisors_llm::{default_seven_advisors, seven_system_prompts, LlmAdvisor};

/// Council 默认 timeout (per scene-d §5 决策 1, 60s 共识机制)
pub const DEFAULT_COUNCIL_TIMEOUT: Duration = Duration::from_secs(60);

pub struct Council {
    advisors: Vec<Arc<dyn Advisor>>,
}

impl Council {
    pub fn default_allow() -> Self {
        Self {
            advisors: vec![
                Arc::new(NoopAdvisor::new(AdvisorKind::Safety)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Performance)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Philosophy)),
                Arc::new(NoopAdvisor::new(AdvisorKind::History)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Strategy)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Ethics)),
                Arc::new(NoopAdvisor::new(AdvisorKind::Legal)),
            ],
        }
    }

    pub fn new(advisors: Vec<Arc<dyn Advisor>>) -> Self {
        Self { advisors }
    }

    pub fn with_factory(factory: Arc<dyn LlmFactory>, primary_model: &str) -> Self {
        Self {
            advisors: default_seven_advisors(factory, primary_model),
        }
    }

    pub fn advisors(&self) -> &[Arc<dyn Advisor>] {
        &self.advisors
    }

    pub async fn decide(&self, proposal: &Proposal) -> CouncilVerdict {
        let futures = self.advisors.iter().map(|a| {
            let advisor = Arc::clone(a);
            async move { (advisor.kind(), advisor.evaluate(proposal).await) }
        });
        let join_all = futures::future::join_all(futures);

        match tokio::time::timeout(DEFAULT_COUNCIL_TIMEOUT, join_all).await {
            Ok(results) => {
                for (kind, verdict) in results {
                    if let AdvisorVerdict::Deny { reason } = verdict {
                        return CouncilVerdict::Vetoed { by: kind, reason };
                    }
                }
                CouncilVerdict::Approved
            }
            Err(_elapsed) => CouncilVerdict::DeferToHuman {
                reason: format!(
                    "{}s no consensus: 7 advisor did not complete in time",
                    DEFAULT_COUNCIL_TIMEOUT.as_secs()
                ),
            },
        }
    }
}

pub struct NoopAdvisor {
    kind: AdvisorKind,
}

impl NoopAdvisor {
    pub fn new(kind: AdvisorKind) -> Self {
        Self { kind }
    }
}

#[async_trait::async_trait]
impl Advisor for NoopAdvisor {
    fn name(&self) -> &'static str {
        match self.kind {
            AdvisorKind::Safety => "noop_safety",
            AdvisorKind::Performance => "noop_performance",
            AdvisorKind::Philosophy => "noop_philosophy",
            AdvisorKind::History => "noop_history",
            AdvisorKind::Strategy => "noop_strategy",
            AdvisorKind::Ethics => "noop_ethics",
            AdvisorKind::Legal => "noop_legal",
        }
    }

    fn kind(&self) -> AdvisorKind {
        self.kind
    }

    async fn evaluate(&self, _proposal: &Proposal) -> AdvisorVerdict {
        AdvisorVerdict::Allow
    }
}
