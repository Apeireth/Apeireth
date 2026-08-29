//! DeepSeek Harness / Harness-R1 风格自进化修补引擎 (HarnessPatchEngine).
//!
//! “不微调大模型权重，而是基于执行失败轨迹自动微调 Agent 的运行策略与上下文构造 (Harness)”.
//! 收集工具失败、审批拒绝、无限递归等异常轨迹，自动生成可执行的策略修补方案并在沙箱中评估.

use serde::{Deserialize, Serialize};

/// Agent 执行失败原因分类.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCategory {
    /// 工具入参格式错误或缺少必填字段
    ToolArgumentInvalid,
    /// 工具执行环境或路径未找到
    ToolExecutionNotFound,
    /// 治理策略审批硬性拒绝
    GovernanceDenial,
    /// 循环思维推演达到最大深度熔断
    RecursiveThinkingExhausted,
    /// 上下文超长截断丢失关键信息
    ContextTruncationLoss,
}

/// 失败轨迹记录.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureTrajectory {
    pub session_id: String,
    pub step_index: u32,
    pub category: FailureCategory,
    pub goal_description: String,
    pub failed_action: String,
    pub error_message: String,
    pub timestamp_ms: u64,
}

/// 自动生成的策略补丁类型.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarnessPatchAction {
    /// 在工具调用前注入前置验证/格式提示
    InjectPreCallGuidance { tool_name: String, guidance: String },
    /// 调整单步思考深度上限
    AdjustThinkingBudget { new_max_depth: u32 },
    /// 增加路径防报错前置兜底
    AddFallbackPath { target_pattern: String, fallback_value: String },
}

/// 策略补丁包.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessPatch {
    pub patch_id: String,
    pub target_category: FailureCategory,
    pub description: String,
    pub action: HarnessPatchAction,
    pub is_active: bool,
}

/// Harness 自修复引擎.
#[derive(Debug, Clone, Default)]
pub struct HarnessPatchEngine {
    trajectories: Vec<FailureTrajectory>,
    active_patches: Vec<HarnessPatch>,
}

impl HarnessPatchEngine {
    pub fn new() -> Self {
        Self {
            trajectories: Vec::new(),
            active_patches: Vec::new(),
        }
    }

    /// 记录一条失败轨迹.
    pub fn record_failure(&mut self, trajectory: FailureTrajectory) {
        self.trajectories.push(trajectory);
    }

    /// 根据累积的失败轨迹自动推导并生成修补补丁 (Rule Deduction).
    pub fn synthesize_patches(&mut self) -> Vec<HarnessPatch> {
        let mut new_patches = Vec::new();

        for traj in &self.trajectories {
            match traj.category {
                FailureCategory::ToolArgumentInvalid => {
                    new_patches.push(HarnessPatch {
                        patch_id: format!("patch_arg_{}", self.active_patches.len() + new_patches.len() + 1),
                        target_category: FailureCategory::ToolArgumentInvalid,
                        description: format!("针对 {} 的入参格式注入提示", traj.failed_action),
                        action: HarnessPatchAction::InjectPreCallGuidance {
                            tool_name: traj.failed_action.clone(),
                            guidance: "请严格遵循 JSON Schema 必填字段定义".to_string(),
                        },
                        is_active: true,
                    });
                }
                FailureCategory::RecursiveThinkingExhausted => {
                    new_patches.push(HarnessPatch {
                        patch_id: format!("patch_think_{}", self.active_patches.len() + new_patches.len() + 1),
                        target_category: FailureCategory::RecursiveThinkingExhausted,
                        description: "增加单步反思深度保护上限".to_string(),
                        action: HarnessPatchAction::AdjustThinkingBudget { new_max_depth: 8 },
                        is_active: true,
                    });
                }
                _ => {}
            }
        }

        for p in &new_patches {
            self.active_patches.push(p.clone());
        }

        new_patches
    }

    /// 获取当前所有生效中的 Harness 策略补丁.
    pub fn get_active_patches(&self) -> &[HarnessPatch] {
        &self.active_patches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_patch_synthesis() {
        let mut engine = HarnessPatchEngine::new();

        engine.record_failure(FailureTrajectory {
            session_id: "s1".to_string(),
            step_index: 3,
            category: FailureCategory::ToolArgumentInvalid,
            goal_description: "查询天气".to_string(),
            failed_action: "weather_tool".to_string(),
            error_message: "missing required field 'city'".to_string(),
            timestamp_ms: 1000,
        });

        let patches = engine.synthesize_patches();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].target_category, FailureCategory::ToolArgumentInvalid);
        assert!(matches!(patches[0].action, HarnessPatchAction::InjectPreCallGuidance { .. }));

        assert_eq!(engine.get_active_patches().len(), 1);
    }
}
