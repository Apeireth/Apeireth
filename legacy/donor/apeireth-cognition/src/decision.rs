//! 决策模块 — 应用 12 键 verdict 守门 + 决策合成.

use apeireth_core::{verdict_for_target, ActionTarget, PhilosophyKey, PhilosophyVerdict};

use crate::{CognitionError, CognitionResult};

/// 决策管道 — 串联 verdict 守门与决策合成.
pub struct CognitivePipeline;

/// 认知输出 — 决策/拒绝/需精化.
#[derive(Debug, Clone)]
pub enum CognitiveOutput {
    /// 通过决策 (含描述).
    Decision(String),
    /// 被 12 键 verdict 拒绝 (含首个 Block 键名).
    Reject(PhilosophyKey),
}

/// 对所有候选行动应用 12 键 verdict 守门 (调用 apeireth-core `verdict_for_target`).
pub fn evaluate_actions(targets: &[ActionTarget]) -> Vec<PhilosophyVerdict> {
    targets.iter().map(verdict_for_target).collect()
}

/// 合成最终决策 — 任一 Block 即 Reject, 全部 Allow 即 Decision.
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
        // 占位: CognitivePipeline 为模块化命名空间, 后续扩展用.
    }
}
