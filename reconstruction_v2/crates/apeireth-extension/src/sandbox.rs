//! sandbox — 权限 + 输入大小沙盒
//!
//! 每次调用插件前, `Sandbox::check` 必须通过:
//! 1. 权限匹配: caller_perms ⊇ plugin.permissions (超集)
//! 2. 输入大小 ≤ plugin.max_input_bytes
//!
//! `Sandbox::check_output` 在执行后可选检查输出大小.

use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use std::collections::BTreeSet;

/// 权限名 (语义化字符串, e.g. "invoke", "read", "write", "system")
pub type Permission = String;

/// 沙盒配置 (全局)
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 调用方持有的权限集合
    pub caller_permissions: BTreeSet<Permission>,
    /// 是否启用输出大小检查
    pub enforce_output_size: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        let mut perms = BTreeSet::new();
        // 默认调用方持有基础权限
        perms.insert("invoke".to_string());
        perms.insert("read".to_string());
        Self {
            caller_permissions: perms,
            enforce_output_size: true,
        }
    }
}

impl SandboxConfig {
    /// 构造空沙盒 (无权限, 所有调用拒绝)
    pub fn empty() -> Self {
        Self {
            caller_permissions: BTreeSet::new(),
            enforce_output_size: true,
        }
    }

    /// 构造特权沙盒 (持有所有权限)
    pub fn privileged() -> Self {
        let mut perms = BTreeSet::new();
        perms.insert("invoke".to_string());
        perms.insert("read".to_string());
        perms.insert("write".to_string());
        perms.insert("system".to_string());
        perms.insert("llm_call".to_string());
        perms.insert("ask_user".to_string());
        perms.insert("render".to_string());
        Self {
            caller_permissions: perms,
            enforce_output_size: true,
        }
    }

    /// 添加权限
    pub fn with_permission(mut self, p: impl Into<String>) -> Self {
        self.caller_permissions.insert(p.into());
        self
    }

    /// 移除权限
    pub fn without_permission(mut self, p: &str) -> Self {
        self.caller_permissions.remove(p);
        self
    }
}

/// 沙盒
#[derive(Debug, Clone)]
pub struct Sandbox {
    config: SandboxConfig,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new(SandboxConfig::default())
    }
}

impl Sandbox {
    /// 构造沙盒
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// 沙盒检查 (input 权限 + 大小)
    pub fn check(&self, manifest: &Manifest, input_bytes: usize) -> Result<()> {
        // 1. 权限
        for required in &manifest.permissions {
            if !self.config.caller_permissions.contains(required) {
                let caller = self
                    .config
                    .caller_permissions
                    .iter()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "<none>".to_string());
                return Err(ExtensionError::PermissionDenied {
                    plugin: manifest.name.clone(),
                    required: required.clone(),
                    caller,
                });
            }
        }
        // 2. 输入大小
        if input_bytes > manifest.max_input_bytes {
            return Err(ExtensionError::InputTooLarge {
                actual: input_bytes,
                max: manifest.max_input_bytes,
                plugin: manifest.name.clone(),
            });
        }
        Ok(())
    }

    /// 输出大小检查 (可选, 通过 `enforce_output_size` 控制)
    pub fn check_output(&self, manifest: &Manifest, output_bytes: usize) -> Result<()> {
        if self.config.enforce_output_size && output_bytes > manifest.max_output_bytes {
            return Err(ExtensionError::Execution(format!(
                "output too large: {output_bytes} > {} (plugin={})",
                manifest.max_output_bytes, manifest.name
            )));
        }
        Ok(())
    }

    /// 读取配置
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PluginKind;

    fn make_manifest(perms: Vec<&str>, max_in: usize, max_out: usize) -> Manifest {
        Manifest {
            name: "test".into(),
            version: "0.1.0".into(),
            kind: PluginKind::Sync,
            description: "test".into(),
            entry: "lib.rs".into(),
            permissions: perms.into_iter().map(String::from).collect(),
            max_input_bytes: max_in,
            max_output_bytes: max_out,
            timeout_ms: 1000,
        }
    }

    #[test]
    fn default_caller_has_invoke_read() {
        let s = Sandbox::default();
        assert!(s.config().caller_permissions.contains("invoke"));
        assert!(s.config().caller_permissions.contains("read"));
    }

    #[test]
    fn empty_caller_rejects_invoke() {
        let s = Sandbox::new(SandboxConfig::empty());
        let m = make_manifest(vec!["invoke"], 1024, 1024);
        let res = s.check(&m, 10);
        assert!(matches!(res, Err(ExtensionError::PermissionDenied { .. })));
    }

    #[test]
    fn privileged_caller_accepts_all() {
        let s = Sandbox::new(SandboxConfig::privileged());
        let m = make_manifest(vec!["invoke", "read", "write", "system"], 1024, 1024);
        assert!(s.check(&m, 100).is_ok());
    }

    #[test]
    fn input_too_large_rejected() {
        let s = Sandbox::new(SandboxConfig::privileged());
        let m = make_manifest(vec!["invoke"], 100, 100);
        let res = s.check(&m, 101);
        assert!(matches!(res, Err(ExtensionError::InputTooLarge { .. })));
    }

    #[test]
    fn input_at_exact_limit_ok() {
        let s = Sandbox::new(SandboxConfig::privileged());
        let m = make_manifest(vec!["invoke"], 100, 100);
        assert!(s.check(&m, 100).is_ok());
    }

    #[test]
    fn output_size_check_disabled() {
        let mut cfg = SandboxConfig::privileged();
        cfg.enforce_output_size = false;
        let s = Sandbox::new(cfg);
        let m = make_manifest(vec!["invoke"], 1024, 50);
        // input ok
        assert!(s.check(&m, 10).is_ok());
        // output huge, but check disabled
        assert!(s.check_output(&m, 100_000).is_ok());
    }

    #[test]
    fn output_size_check_enabled() {
        let s = Sandbox::new(SandboxConfig::privileged());
        let m = make_manifest(vec!["invoke"], 1024, 50);
        assert!(s.check(&m, 10).is_ok());
        let res = s.check_output(&m, 51);
        assert!(matches!(res, Err(ExtensionError::Execution(_))));
    }

    #[test]
    fn with_permission_extends_set() {
        let cfg = SandboxConfig::default().with_permission("system");
        assert!(cfg.caller_permissions.contains("system"));
    }

    #[test]
    fn without_permission_removes() {
        let cfg = SandboxConfig::default().without_permission("invoke");
        assert!(!cfg.caller_permissions.contains("invoke"));
    }
}
