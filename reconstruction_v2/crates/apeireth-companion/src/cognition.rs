//! Cognition - 认知引擎 (从 v1.0 apeireth-cognition 3,890 LOC 收敛)
//!
//! 0 装 PASS: 重构版 cognition 集成 Apeireth 已有的 curiosity / world_model / streaming 模块,
//! 提供高层认知状态机, 不再独立管理 thinking loop.
//!
//! 设计 (per user 右图 "Companion 智能核"):
//! - CognitiveState: 6 态机 (Perceive → Reflect → Reason → Plan → Act → Verify)
//! - ThoughtFrame: 单次认知产出 (含引用 + 不确定性)
//! - ChainReasoning: 多步推理链

use serde::{Deserialize, Serialize};

/// 认知状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CognitiveState {
    Perceive,   // 感知输入
    Reflect,    // 反思历史
    Reason,     // 推理 (chain)
    Plan,       // 规划步骤
    Act,        // 执行 (调用工具)
    Verify,     // 验证结果
}

impl CognitiveState {
    /// 状态机转移合法性
    pub fn can_transition_to(self, next: Self) -> bool {
        use CognitiveState::*;
        matches!((self, next),
            (Perceive, Reflect) | (Perceive, Act) |  // 感知 → 反思 或 感知 → 直达行动
            (Reflect, Reason) | (Reflect, Perceive) | (Reflect, Act) |  // 反思后可直接行动
            (Reason, Plan) | (Reason, Reason) |       // 可以多步推理
            (Plan, Act) | (Plan, Reason) |           // 规划可以回退到推理
            (Act, Verify) | (Act, Plan) |
            (Verify, Act) | (Verify, Reflect) | (Verify, Perceive)
        )
    }
}

/// 单次认知产出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtFrame {
    pub state: CognitiveState,
    pub content: String,
    pub citations: Vec<String>,  // 引用的记忆 / 来源
    pub uncertainty: f64,        // 0..1, 1 = 完全不确定
    pub next_action: Option<String>,
}

impl ThoughtFrame {
    pub fn new(state: CognitiveState, content: String) -> Self {
        Self {
            state,
            content,
            citations: Vec::new(),
            uncertainty: 0.5, // 默认中等不确定
            next_action: None,
        }
    }
}

/// ChainReasoning - 多步推理链
#[derive(Default)]
pub struct ReasoningChain {
    pub frames: Vec<ThoughtFrame>,
    pub conclusions: Vec<String>,
}

impl ReasoningChain {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, frame: ThoughtFrame) {
        if let Some(last) = self.frames.last() {
            // 0 装 PASS: 状态机合法性检查
            assert!(last.state.can_transition_to(frame.state),
                "invalid state transition: {:?} -> {:?}", last.state, frame.state);
        }
        if frame.state == CognitiveState::Verify {
            self.conclusions.push(frame.content.clone());
        }
        self.frames.push(frame);
    }

    pub fn final_uncertainty(&self) -> f64 {
        // 0 装 PASS: 取最后一帧的不确定性 (最新的反思结果)
        self.frames.last().map(|f| f.uncertainty).unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_state_transitions() {
        use CognitiveState::*;
        assert!(Perceive.can_transition_to(Reflect));
        assert!(Reflect.can_transition_to(Reason));
        assert!(Reason.can_transition_to(Plan));
        assert!(Plan.can_transition_to(Act));
        assert!(Act.can_transition_to(Verify));
        // 非法
        assert!(!Perceive.can_transition_to(Plan));
        assert!(!Verify.can_transition_to(Reason));
    }

    #[test]
    fn test_reasoning_chain_full() {
        use CognitiveState::*;
        let mut chain = ReasoningChain::new();
        chain.add(ThoughtFrame::new(Perceive, "user asked X".into()));
        chain.add(ThoughtFrame::new(Reflect, "remembered similar case".into()));
        chain.add(ThoughtFrame::new(Reason, "inferring...".into()));
        chain.add(ThoughtFrame::new(Plan, "step 1, 2, 3".into()));
        chain.add(ThoughtFrame::new(Act, "called tool Y".into()));
        chain.add(ThoughtFrame::new(Verify, "conclusion Z".into()));
        assert_eq!(chain.frames.len(), 6);
        assert_eq!(chain.conclusions, vec!["conclusion Z"]);
        assert_eq!(chain.final_uncertainty(), 0.5); // 默认值
    }

    #[test]
    #[should_panic(expected = "invalid state transition")]
    fn test_invalid_transition_panics() {
        use CognitiveState::*;
        let mut chain = ReasoningChain::new();
        chain.add(ThoughtFrame::new(Perceive, "x".into()));
        chain.add(ThoughtFrame::new(Plan, "skip reason".into())); // 非法: Perceive → Plan
    }
}
