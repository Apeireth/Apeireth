//! Decision module - applies 13-key verdict gate + decision synthesis.

use apeireth_core::{verdict_for_target, ActionTarget, PhilosophyKey, PhilosophyVerdict};

use crate::CognitionResult;

/// Decision pipeline - composes verdict gate and decision synthesis.
pub struct CognitivePipeline;

/// Cognition output - decision / reject.
#[derive(Debug, Clone)]
pub enum CognitiveOutput {
    /// Decision passed (with description).
    Decision(String),
    /// Rejected by 13-key verdict (with first Block key name).
    Reject(PhilosophyKey),
}

/// Apply 13-key verdict gate to all candidate actions (calls apeireth-core `verdict_for_target`).
pub fn evaluate_actions(targets: &[ActionTarget]) -> Vec<PhilosophyVerdict> {
    targets.iter().map(verdict_for_target).collect()
}

/// Synthesize final decision - any Block rejects, all Allow approves.
pub fn decide(verdicts: &[PhilosophyVerdict]) -> CognitionResult<CognitiveOutput> {
    for v in verdicts {
        if let PhilosophyVerdict::Block(key) = v {
            return Ok(CognitiveOutput::Reject(*key));
        }
    }
    Ok(CognitiveOutput::Decision(format!(
        "approved_{}_actions",
        verdicts.len()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_actions_returns_one_verdict_per_target() {
        let targets = vec![
            ActionTarget::NormalAction("a".to_string()),
            ActionTarget::NormalAction("b".to_string()),
        ];
        let verdicts = evaluate_actions(&targets);
        assert_eq!(verdicts.len(), 2);
    }

    #[test]
    fn evaluate_actions_blocks_modify_l0_ha() {
        let targets = vec![ActionTarget::ModifyL0HA];
        let verdicts = evaluate_actions(&targets);
        assert!(matches!(verdicts[0], PhilosophyVerdict::Block(_)));
    }

    #[test]
    fn evaluate_actions_allows_normal_action() {
        let targets = vec![ActionTarget::NormalAction("x".to_string())];
        let verdicts = evaluate_actions(&targets);
        assert_eq!(verdicts[0], PhilosophyVerdict::Allow);
    }

    #[test]
    fn decide_allows_when_all_allow() {
        let verdicts = vec![PhilosophyVerdict::Allow, PhilosophyVerdict::Allow];
        let output = decide(&verdicts).expect("decide ok");
        assert!(matches!(output, CognitiveOutput::Decision(_)));
    }

    #[test]
    fn decide_rejects_when_any_block() {
        let verdicts = vec![
            PhilosophyVerdict::Allow,
            PhilosophyVerdict::Block(PhilosophyKey::NotClone),
        ];
        let output = decide(&verdicts).expect("decide ok");
        assert!(matches!(
            output,
            CognitiveOutput::Reject(PhilosophyKey::NotClone)
        ));
    }

    #[test]
    fn decide_handles_empty_verdicts_as_decision() {
        let verdicts: Vec<PhilosophyVerdict> = vec![];
        let output = decide(&verdicts).expect("decide ok");
        assert!(matches!(output, CognitiveOutput::Decision(_)));
    }

    #[test]
    fn decision_pipeline_construction_is_zero_cost() {
        let _ = CognitivePipeline;
        // placeholder: CognitivePipeline is a modular namespace, future extension point.
    }

    #[test]
    fn cognition_error_clone_debug() {
        let e = crate::CognitionError::InvalidInput("bad".to_string());
        let s = format!("{:?}", e);
        assert!(s.contains("InvalidInput"));
    }
}
