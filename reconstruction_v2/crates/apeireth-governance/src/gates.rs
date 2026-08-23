use apeireth_core::philosophy::{PhilosophyKey, VerdictCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTarget {
    pub name: String,
    pub risk_level: RiskLevel,
    pub requires_council: bool,
    pub external_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: &'static str,
    pub passed: bool,
    pub reason: String,
}

pub struct CompileTimeGate;
impl CompileTimeGate {
    pub fn verify_signature(schema_version: &str) -> GateResult {
        if schema_version.starts_with("2.") {
            GateResult { gate_name: "CompileTimeGate", passed: true, reason: "Schema contract v2.x matched".into() }
        } else {
            GateResult { gate_name: "CompileTimeGate", passed: false, reason: "Outdated schema contract".into() }
        }
    }
}

pub struct RuntimeGate;
impl RuntimeGate {
    pub fn check_rate_and_budget(remaining_budget: f64, tokens_requested: usize) -> GateResult {
        if remaining_budget <= 0.0 {
            GateResult { gate_name: "RuntimeGate", passed: false, reason: "Budget exhausted".into() }
        } else if tokens_requested > 32768 {
            GateResult { gate_name: "RuntimeGate", passed: false, reason: "Single request token limit exceeded".into() }
        } else {
            GateResult { gate_name: "RuntimeGate", passed: true, reason: "Budget and token limits OK".into() }
        }
    }
}

pub struct CouncilGate;
impl CouncilGate {
    pub fn evaluate(target: &ActionTarget, council_votes_yes: usize, total_council: usize) -> GateResult {
        if !target.requires_council && target.risk_level < RiskLevel::Critical {
            return GateResult { gate_name: "CouncilGate", passed: true, reason: "Council approval not required for low/med action".into() };
        }
        if total_council == 0 {
            return GateResult { gate_name: "CouncilGate", passed: false, reason: "No council members available".into() };
        }
        let threshold = (total_council as f64 * 0.66).ceil() as usize;
        if council_votes_yes >= threshold {
            GateResult { gate_name: "CouncilGate", passed: true, reason: format!("Council consensus reached ({}/{})", council_votes_yes, total_council) }
        } else {
            GateResult { gate_name: "CouncilGate", passed: false, reason: format!("Council veto: insufficient votes ({}/{})", council_votes_yes, total_council) }
        }
    }
}

pub struct PhysicalIsolationGate;
impl PhysicalIsolationGate {
    pub fn check_network_boundary(target: &ActionTarget, egress_allowed: bool) -> GateResult {
        if target.external_network && !egress_allowed {
            GateResult { gate_name: "PhysicalIsolationGate", passed: false, reason: "Default-Deny blocked external egress".into() }
        } else {
            GateResult { gate_name: "PhysicalIsolationGate", passed: true, reason: "Physical boundary intact".into() }
        }
    }
}

pub struct ReflectionAuditGate;
impl ReflectionAuditGate {
    pub fn evaluate(action_name: &str, verdict_cache: &mut VerdictCache) -> GateResult {
        let v = verdict_cache.evaluate_action(PhilosophyKey::K2_ZeroPretending, action_name);
        if v.allowed {
            GateResult { gate_name: "ReflectionAuditGate", passed: true, reason: v.rationale }
        } else {
            GateResult { gate_name: "ReflectionAuditGate", passed: false, reason: v.rationale }
        }
    }
}

pub struct GovernancePipeline {
    verdict_cache: VerdictCache,
}

impl Default for GovernancePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl GovernancePipeline {
    pub fn new() -> Self {
        Self {
            verdict_cache: VerdictCache::new(),
        }
    }

    pub fn evaluate_action(
        &mut self,
        target: &ActionTarget,
        schema_version: &str,
        remaining_budget: f64,
        tokens: usize,
        council_yes: usize,
        council_total: usize,
        egress_allowed: bool,
    ) -> Result<Vec<GateResult>, String> {
        let results = vec![
            CompileTimeGate::verify_signature(schema_version),
            RuntimeGate::check_rate_and_budget(remaining_budget, tokens),
            CouncilGate::evaluate(target, council_yes, council_total),
            PhysicalIsolationGate::check_network_boundary(target, egress_allowed),
            ReflectionAuditGate::evaluate(&target.name, &mut self.verdict_cache),
        ];

        for res in &results {
            if !res.passed {
                return Err(format!("Gate [{}] REJECTED: {}", res.gate_name, res.reason));
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governance_pipeline_pass() {
        let mut pipeline = GovernancePipeline::new();
        let target = ActionTarget {
            name: "generate_response".into(),
            risk_level: RiskLevel::Low,
            requires_council: false,
            external_network: true,
        };

        let res = pipeline.evaluate_action(&target, "2.0.0", 1.0, 500, 3, 3, true);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 5);
    }

    #[test]
    fn test_governance_pipeline_reject_forbidden_action() {
        let mut pipeline = GovernancePipeline::new();
        let target = ActionTarget {
            name: "pretend_to_feel_love".into(),
            risk_level: RiskLevel::Low,
            requires_council: false,
            external_network: false,
        };

        let res = pipeline.evaluate_action(&target, "2.0.0", 1.0, 500, 3, 3, true);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("ReflectionAuditGate"));
    }

    #[test]
    fn test_council_gate_rejection() {
        let target = ActionTarget {
            name: "modify_system_core".into(),
            risk_level: RiskLevel::Critical,
            requires_council: true,
            external_network: false,
        };

        let result = CouncilGate::evaluate(&target, 1, 3); // 1/3 is less than 2/3
        assert!(!result.passed);
        assert!(result.reason.contains("Council veto"));
    }
}

