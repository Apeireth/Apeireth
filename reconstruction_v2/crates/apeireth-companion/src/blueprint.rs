//! Blueprint - 蓝图执行器 (从 v1.0 apeireth-blueprint-impl 4,695 LOC 收敛)
//!
//! 0 装 PASS: Blueprint 是"模板化的多步认知流程", 嵌入 companion + workflow,
//! 不再独立管理 pipeline.
//!
//! 设计 (per user 右图 "Companion 智能核"):
//! - Blueprint: 模板 (多步 ThoughtFrame + 条件分支)
//! - BlueprintStep: 单步 (state 目标 + 失败回退)
//! - BlueprintReport: 执行结果 (成功率 + 失败原因)

use serde::{Deserialize, Serialize};

use super::cognition::{CognitiveState, ThoughtFrame};

/// 蓝图步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintStep {
    pub name: String,
    pub target_state: CognitiveState,
    pub expected_outcome: String,   // 期望 outcome (用于 verify)
    pub on_failure: FailureMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureMode {
    Abort,
    Retry { max: u8 },
    Fallback,  // 跳到 fallback step
    Skip,
}

/// Blueprint 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub description: String,
    pub steps: Vec<BlueprintStep>,
    pub fallback_step: Option<usize>, // 失败时 fallback 到哪一步
}

impl Blueprint {
    pub fn new(name: String, description: String) -> Self {
        Self { name, description, steps: Vec::new(), fallback_step: None }
    }

    pub fn add_step(&mut self, step: BlueprintStep) {
        self.steps.push(step);
    }

    /// 0 装 PASS: 验证 step 顺序的 state 转移合法 (Perceive → Reflect → ... → Verify)
    pub fn validate(&self) -> Result<(), String> {
        for i in 1..self.steps.len() {
            if !self.steps[i-1].target_state.can_transition_to(self.steps[i].target_state) {
                return Err(format!("invalid transition at step {}: {:?} -> {:?}",
                    i, self.steps[i-1].target_state, self.steps[i].target_state));
            }
        }
        Ok(())
    }
}

/// BlueprintReport - 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintReport {
    pub blueprint_name: String,
    pub total_steps: usize,
    pub succeeded: usize,
    pub failed_step: Option<usize>,
    pub failure_reason: String,
    pub frames: Vec<ThoughtFrame>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::cognition::CognitiveState::*;

    #[test]
    fn test_blueprint_full_flow() {
        let mut bp = Blueprint::new("test".into(), "desc".into());
        bp.add_step(BlueprintStep { name: "perceive".into(), target_state: Perceive, expected_outcome: "input captured".into(), on_failure: FailureMode::Retry { max: 3 } });
        bp.add_step(BlueprintStep { name: "reflect".into(), target_state: Reflect, expected_outcome: "memory retrieved".into(), on_failure: FailureMode::Fallback });
        bp.add_step(BlueprintStep { name: "act".into(), target_state: Act, expected_outcome: "tool executed".into(), on_failure: FailureMode::Abort });
        bp.add_step(BlueprintStep { name: "verify".into(), target_state: Verify, expected_outcome: "outcome matches".into(), on_failure: FailureMode::Skip });
        assert!(bp.validate().is_ok());
        assert_eq!(bp.steps.len(), 4);
    }

    #[test]
    fn test_blueprint_invalid_transition() {
        let mut bp = Blueprint::new("bad".into(), "desc".into());
        bp.add_step(BlueprintStep { name: "perceive".into(), target_state: Perceive, expected_outcome: "x".into(), on_failure: FailureMode::Skip });
        bp.add_step(BlueprintStep { name: "skip_reason".into(), target_state: Plan, expected_outcome: "y".into(), on_failure: FailureMode::Skip });
        // Perceive → Plan 非法
        assert!(bp.validate().is_err());
    }
}
