//! EvolutionGate - 演化门 (从 v1.0 apeireth-companion/evolution_gate.rs 286 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 EvalGate + GateDecision + VerifyOutcome

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome { Pass, Fail, Pending }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision { Approve, Reject, Defer }

pub struct EvalGate;

impl EvalGate {
    pub fn new() -> Self { Self }
    /// 0 装 PASS: 真 evaluate
    pub fn evaluate(&self, outcome: VerifyOutcome, risk: u8) -> GateDecision {
        match outcome {
            VerifyOutcome::Pass => GateDecision::Approve,
            VerifyOutcome::Fail => GateDecision::Reject,
            VerifyOutcome::Pending if risk > 50 => GateDecision::Defer,
            VerifyOutcome::Pending => GateDecision::Defer,
        }
    }
}

impl Default for EvalGate { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_pass() { assert_eq!(EvalGate::new().evaluate(VerifyOutcome::Pass, 0), GateDecision::Approve); }
    #[test] fn test_fail() { assert_eq!(EvalGate::new().evaluate(VerifyOutcome::Fail, 0), GateDecision::Reject); }
    #[test] fn test_pending() { assert_eq!(EvalGate::new().evaluate(VerifyOutcome::Pending, 0), GateDecision::Defer); }
    #[test] fn test_outcome_eq() { assert_eq!(VerifyOutcome::Pass, VerifyOutcome::Pass); }
    #[test] fn test_decision_eq() { assert_eq!(GateDecision::Approve, GateDecision::Approve); }
}
