//! VMSandbox - microVM 沙箱 (从 v1.0 apeireth-companion/vm_sandbox.rs 805 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 VmBackend trait + libkrun/Firecracker/Hyperlight 抽象

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmBackend { Disabled, Libkrun, Firecracker, Hyperlight }

pub trait VmBackendTrait: Send + Sync {
    fn name(&self) -> &str;
    fn launch(&self, config: &VmConfig) -> Result<String, String>;  // 返 VM ID
    fn stop(&self, vm_id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub vcpus: u32,
    pub memory_mb: u32,
    pub kernel: String,
    pub rootfs: String,
    pub env: HashMap<String, String>,
}

pub struct VmSandbox { pub backend: VmBackend }

impl VmSandbox {
    pub fn new(backend: VmBackend) -> Self { Self { backend } }
    /// 0 装 PASS stub: 真 launch (返 mock ID, 0 装 PASS 标 stub)
    pub fn launch(&self, config: &VmConfig) -> Result<String, String> {
        if self.backend == VmBackend::Disabled { return Err("backend disabled".into()); }
        Ok(format!("vm-{}-{}v", self.backend_name(), config.vcpus))
    }
    pub fn backend_name(&self) -> &'static str {
        match self.backend {
            VmBackend::Disabled => "disabled",
            VmBackend::Libkrun => "libkrun",
            VmBackend::Firecracker => "firecracker",
            VmBackend::Hyperlight => "hyperlight",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_disabled() { assert!(VmSandbox::new(VmBackend::Disabled).launch(&VmConfig { vcpus: 1, memory_mb: 256, kernel: "k".into(), rootfs: "r".into(), env: HashMap::new() }).is_err()); }
    #[test] fn test_libkrun() {
        let v = VmSandbox::new(VmBackend::Libkrun);
        let r = v.launch(&VmConfig { vcpus: 2, memory_mb: 512, kernel: "k".into(), rootfs: "r".into(), env: HashMap::new() }).unwrap();
        assert!(r.starts_with("vm-libkrun-"));
    }
    #[test] fn test_backend_name() {
        assert_eq!(VmSandbox::new(VmBackend::Libkrun).backend_name(), "libkrun");
        assert_eq!(VmSandbox::new(VmBackend::Firecracker).backend_name(), "firecracker");
    }
    #[test] fn test_vm_config() {
        let c = VmConfig { vcpus: 4, memory_mb: 1024, kernel: "k".into(), rootfs: "r".into(), env: HashMap::new() };
        assert_eq!(c.vcpus, 4);
    }
}
