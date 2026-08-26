//! registry — 审核后注册 + 调用审计
//!
//! 流程:
//! 1. register(plugin) → 验证 plugin.manifest 严格 schema (已通过) → 审核 manifest → 注册
//! 2. call(name, input) → sandbox.check → 异步 plugin.call → 审计日志 → 返回

use crate::audit::{audit_manifest, AuditLog};
use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use crate::sandbox::{Sandbox, SandboxConfig};
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::{AuditEntry, PluginKind};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 注册统计
#[derive(Debug, Clone, Copy)]
pub struct RegistryStats {
    /// 已注册插件数
    pub registered: usize,
    /// 累计调用次数
    pub total_calls: usize,
    /// 累计失败次数
    pub total_failures: usize,
    /// 累计拒绝次数 (沙盒 / 审核)
    pub total_rejections: usize,
    /// 累计审核不通过次数
    pub total_audit_rejects: usize,
}

/// 审核后注册中心
pub struct AuditRegistry {
    plugins: HashMap<String, Arc<dyn AsyncExtension>>,
    sandbox: Sandbox,
    audit_log: AuditLog,
    total_calls: AtomicUsize,
    total_failures: AtomicUsize,
    total_rejections: AtomicUsize,
    total_audit_rejects: AtomicUsize,
}

impl std::fmt::Debug for AuditRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditRegistry")
            .field("registered", &self.plugins.len())
            .field("total_calls", &self.total_calls.load(Ordering::Relaxed))
            .finish()
    }
}

impl Default for AuditRegistry {
    fn default() -> Self {
        Self::new(SandboxConfig::default())
    }
}

impl AuditRegistry {
    /// 构造注册中心 (默认沙盒)
    pub fn new(sandbox_config: SandboxConfig) -> Self {
        Self {
            plugins: HashMap::new(),
            sandbox: Sandbox::new(sandbox_config),
            audit_log: AuditLog::new(),
            total_calls: AtomicUsize::new(0),
            total_failures: AtomicUsize::new(0),
            total_rejections: AtomicUsize::new(0),
            total_audit_rejects: AtomicUsize::new(0),
        }
    }

    /// 注册插件 (审核后)
    pub fn register<P: AsyncExtension + 'static>(&mut self, plugin: P) -> Result<()> {
        let manifest = plugin.manifest().clone();
        let name = manifest.name.clone();

        // 1. 审核
        audit_manifest(&manifest).map_err(|e| {
            self.total_audit_rejects.fetch_add(1, Ordering::Relaxed);
            e
        })?;

        // 2. 唯一性
        if self.plugins.contains_key(&name) {
            return Err(ExtensionError::AlreadyRegistered(name));
        }

        // 3. 注册
        self.plugins.insert(name, Arc::new(plugin));
        Ok(())
    }

    /// 按 kind 列表
    pub fn list_by_kind(&self, kind: PluginKind) -> Vec<String> {
        self.plugins
            .values()
            .filter(|p| p.kind() == kind)
            .map(|p| p.name().to_string())
            .collect()
    }

    /// 全部
    pub fn list_all(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// 数量
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// 空
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// 取得插件 manifest (按 name)
    pub fn manifest(&self, name: &str) -> Result<Manifest> {
        self.plugins
            .get(name)
            .map(|p| p.manifest().clone())
            .ok_or_else(|| ExtensionError::NotFound(name.to_string()))
    }

    /// 调用插件 (沙盒检查 + 执行 + 审计)
    pub async fn call(&self, name: &str, input: ExtensionInput) -> Result<ExtensionOutput> {
        let plugin = self
            .plugins
            .get(name)
            .ok_or_else(|| ExtensionError::NotFound(name.to_string()))?
            .clone();
        let manifest = plugin.manifest().clone();
        let kind = plugin.kind();
        let input_bytes = input.byte_size();

        // 1. 沙盒
        if let Err(e) = self.sandbox.check(&manifest, input_bytes) {
            self.total_rejections.fetch_add(1, Ordering::Relaxed);
            self.audit_log.push_failure(AuditEntry::failure(
                name,
                kind,
                input_bytes,
                0,
                e.to_string(),
            ));
            return Err(e);
        }

        // 2. 执行
        let t0 = Instant::now();
        let result = plugin.call(input).await;
        let elapsed = t0.elapsed().as_micros();

        // 3. 审计
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        match &result {
            Ok(out) => {
                if out.success {
                    self.audit_log.push_success(AuditEntry::success(
                        name,
                        kind,
                        input_bytes,
                        out.byte_size(),
                        elapsed,
                    ));
                } else {
                    self.total_failures.fetch_add(1, Ordering::Relaxed);
                    let err = out.error.clone().unwrap_or_else(|| "unknown".into());
                    self.audit_log.push_failure(AuditEntry::failure(
                        name,
                        kind,
                        input_bytes,
                        elapsed,
                        err,
                    ));
                }
            }
            Err(e) => {
                self.total_failures.fetch_add(1, Ordering::Relaxed);
                self.audit_log.push_failure(AuditEntry::failure(
                    name,
                    kind,
                    input_bytes,
                    elapsed,
                    e.to_string(),
                ));
            }
        }

        // 4. 输出大小检查
        if let Ok(out) = &result {
            if let Err(e) = self.sandbox.check_output(&manifest, out.byte_size()) {
                self.total_rejections.fetch_add(1, Ordering::Relaxed);
                return Err(e);
            }
        }

        result
    }

    /// 沙盒引用
    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    /// 审计日志引用
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit_log
    }

    /// 统计
    pub fn stats(&self) -> RegistryStats {
        RegistryStats {
            registered: self.plugins.len(),
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_rejections: self.total_rejections.load(Ordering::Relaxed),
            total_audit_rejects: self.total_audit_rejects.load(Ordering::Relaxed),
        }
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{
        AsyncPlugin, HybridPlugin, MessagePreprocessorPlugin, ServicePlugin, StaticPlugin,
        SyncPlugin,
    };
    use crate::sandbox::SandboxConfig;
    use serde_json::json;

    fn privileged() -> SandboxConfig {
        SandboxConfig::privileged()
    }

    #[test]
    fn register_audit_rejects_no_perms() {
        let m = Manifest {
            name: "x".into(),
            version: "0.1.0".into(),
            kind: PluginKind::Sync,
            description: "x".into(),
            entry: "x.rs".into(),
            permissions: vec![], // ← no perms
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
        };
        // Build plugin bypassing constructor
        let plugin = SyncPlugin::new(m, |_| Ok(json!({})));
        let mut r = AuditRegistry::new(privileged());
        let res = r.register(plugin);
        assert!(matches!(res, Err(ExtensionError::AuditRejected(_))));
        assert_eq!(r.stats().total_audit_rejects, 1);
    }

    #[test]
    fn register_duplicate_rejected() {
        let mut r = AuditRegistry::new(privileged());
        r.register(SyncPlugin::example_add("dup")).unwrap();
        let res = r.register(SyncPlugin::example_add("dup"));
        assert!(matches!(res, Err(ExtensionError::AlreadyRegistered(_))));
    }

    #[tokio::test]
    async fn register_then_call_round_trip() {
        let mut r = AuditRegistry::new(privileged());
        r.register(SyncPlugin::example_add("add")).unwrap();
        let out = r
            .call("add", ExtensionInput::new(json!({"a": 1.0, "b": 2.0})))
            .await
            .unwrap();
        assert_eq!(out.result["sum"], 3.0);
        assert_eq!(r.stats().total_calls, 1);
    }

    #[tokio::test]
    async fn call_not_found() {
        let r = AuditRegistry::new(privileged());
        let res = r.call("ghost", ExtensionInput::new(json!({}))).await;
        assert!(matches!(res, Err(ExtensionError::NotFound(_))));
    }

    #[tokio::test]
    async fn call_sandbox_rejects_input_too_large() {
        let mut r = AuditRegistry::new(privileged());
        r.register(SyncPlugin::example_add("add")).unwrap();
        let big = json!({"data": "x".repeat(2048)});
        let res = r.call("add", ExtensionInput::new(big)).await;
        assert!(matches!(res, Err(ExtensionError::InputTooLarge { .. })));
        assert_eq!(r.stats().total_rejections, 1);
    }

    #[tokio::test]
    async fn call_sandbox_rejects_no_permission() {
        let m = Manifest {
            name: "needs-write".into(),
            version: "0.1.0".into(),
            kind: PluginKind::Sync,
            description: "x".into(),
            entry: "x.rs".into(),
            permissions: vec!["write".into()],
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
        };
        let plugin = SyncPlugin::new(m, |_| Ok(json!({})));
        let mut r = AuditRegistry::new(SandboxConfig::default()); // only invoke, read
        r.register(plugin).unwrap();
        let res = r.call("needs-write", ExtensionInput::new(json!({}))).await;
        assert!(matches!(res, Err(ExtensionError::PermissionDenied { .. })));
    }

    #[tokio::test]
    async fn audit_log_records_success() {
        let mut r = AuditRegistry::new(privileged());
        r.register(SyncPlugin::example_add("add")).unwrap();
        let _ = r
            .call("add", ExtensionInput::new(json!({"a": 1.0, "b": 1.0})))
            .await;
        let log = r.audit_log();
        assert_eq!(log.len(), 1);
        assert!(log.successes()[0].success);
    }

    #[tokio::test]
    async fn list_by_kind_works() {
        let mut r = AuditRegistry::new(privileged());
        r.register(SyncPlugin::example_add("s1")).unwrap();
        r.register(AsyncPlugin::example_io("a1")).unwrap();
        r.register(StaticPlugin::example_lookup("l1")).unwrap();
        r.register(ServicePlugin::example_counter("c1")).unwrap();
        r.register(MessagePreprocessorPlugin::example_uppercase("u1"))
            .unwrap();
        r.register(HybridPlugin::example_echo("h1")).unwrap();
        assert_eq!(r.list_by_kind(PluginKind::Sync).len(), 1);
        assert_eq!(r.list_by_kind(PluginKind::Async).len(), 1);
        assert_eq!(r.list_by_kind(PluginKind::Static).len(), 1);
        assert_eq!(r.list_by_kind(PluginKind::Service).len(), 1);
        assert_eq!(r.list_by_kind(PluginKind::MessagePreprocessor).len(), 1);
        assert_eq!(r.list_by_kind(PluginKind::Hybrid).len(), 1);
    }

    #[tokio::test]
    async fn all_6_kinds_registered() {
        let mut r = AuditRegistry::new(privileged());
        r.register(SyncPlugin::example_add("k1")).unwrap();
        r.register(AsyncPlugin::example_io("k2")).unwrap();
        r.register(StaticPlugin::example_lookup("k3")).unwrap();
        r.register(ServicePlugin::example_counter("k4")).unwrap();
        r.register(MessagePreprocessorPlugin::example_uppercase("k5"))
            .unwrap();
        r.register(HybridPlugin::example_echo("k6")).unwrap();
        assert_eq!(r.len(), 6);
        assert_eq!(r.list_all().len(), 6);
    }
}
