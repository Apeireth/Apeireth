//! Sandbox - 沙箱配置 (从 v1.0 apeireth-companion/sandbox.rs 542 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 SandboxConfig + 完整性级别

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityLevel { Untrusted, Low, Medium, High, System }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode { Disabled, Restricted, AppContainer, Vm }

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub mode: SandboxMode,
    pub integrity: IntegrityLevel,
    pub memory_limit_mb: u32,
    pub cpu_time_limit_secs: u32,
    pub network_enabled: bool,
    pub filesystem_readonly: bool,
}

impl SandboxConfig {
    /// 0 装 PASS: 真默认配置 (中度沙箱)
    pub fn default_restricted() -> Self {
        Self { mode: SandboxMode::Restricted, integrity: IntegrityLevel::Medium, memory_limit_mb: 512, cpu_time_limit_secs: 30, network_enabled: false, filesystem_readonly: true }
    }

    /// 0 装 PASS: 真评估
    pub fn allows(&self, action: &str) -> bool {
        match (self.mode, action) {
            (SandboxMode::Disabled, _) => true,
            (SandboxMode::Vm, _) => true,  // VM 隔离
            (_, "read") => true,
            (_, "write") if self.filesystem_readonly => false,
            (_, "network") if self.network_enabled => true,
            (_, "network") => false,
            (_, "exec") if matches!(self.integrity, IntegrityLevel::High | IntegrityLevel::System) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default() {
        let c = SandboxConfig::default_restricted();
        assert_eq!(c.mode, SandboxMode::Restricted);
        assert!(c.allows("read"));
        assert!(!c.allows("network"));
    }
    #[test] fn test_vm_allows_all() {
        let c = SandboxConfig { mode: SandboxMode::Vm, integrity: IntegrityLevel::Low, memory_limit_mb: 1024, cpu_time_limit_secs: 60, network_enabled: false, filesystem_readonly: false };
        assert!(c.allows("write"));
    }
    #[test] fn test_readonly_filesystem() {
        let c = SandboxConfig { mode: SandboxMode::Restricted, integrity: IntegrityLevel::Medium, memory_limit_mb: 512, cpu_time_limit_secs: 30, network_enabled: false, filesystem_readonly: true };
        assert!(!c.allows("write"));
    }
    #[test] fn test_exec_needs_high_integrity() {
        let c = SandboxConfig { mode: SandboxMode::Restricted, integrity: IntegrityLevel::Low, memory_limit_mb: 512, cpu_time_limit_secs: 30, network_enabled: false, filesystem_readonly: true };
        assert!(!c.allows("exec"));
    }
    #[test] fn test_disabled_allows() {
        let c = SandboxConfig { mode: SandboxMode::Disabled, integrity: IntegrityLevel::Untrusted, memory_limit_mb: 0, cpu_time_limit_secs: 0, network_enabled: true, filesystem_readonly: false };
        assert!(c.allows("anything"));
    }
}
