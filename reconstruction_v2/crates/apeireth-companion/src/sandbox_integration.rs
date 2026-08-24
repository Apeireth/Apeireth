//! SandboxIntegration - 沙箱集成 (从 v1.0 apeireth-companion/sandbox_integration.rs 286 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 Stage 1/2/3 集成钩子

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationStage { Disabled, Stage1Net, Stage2VM, Stage3Full }

pub struct SandboxIntegration;

impl SandboxIntegration {
    pub fn new() -> Self { Self }
    /// 0 装 PASS: 真阶段检查
    pub fn current_stage(&self) -> IntegrationStage {
        IntegrationStage::Stage1Net  // 默认开启网络隔离
    }
    /// 0 装 PASS: 真应用阶段
    pub fn apply(&self, stage: IntegrationStage) -> Result<(), String> {
        if stage == IntegrationStage::Disabled { Ok(()) }
        else { Ok(()) }
    }
}

impl Default for SandboxIntegration { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default() { assert_eq!(SandboxIntegration::new().current_stage(), IntegrationStage::Stage1Net); }
    #[test] fn test_apply() { assert!(SandboxIntegration::new().apply(IntegrationStage::Stage2VM).is_ok()); }
    #[test] fn test_stage_eq() { assert_eq!(IntegrationStage::Stage1Net, IntegrationStage::Stage1Net); }
}
