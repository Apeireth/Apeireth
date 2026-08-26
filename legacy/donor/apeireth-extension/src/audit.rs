//! audit — 审核 + 调用审计日志
//!
//! 两个职责:
//! 1. `audit_manifest` 审核 manifest (注册前)
//! 2. `AuditLog` 累积每次调用的审计记录
//!
//! 审核 = 二次校验 (注册前的最后一道关卡):
//! - 名称格式 (a-z0-9-_ 1..=64)
//! - 至少 1 项权限
//! - 输入/输出大小 ≥ 1024 (避免 0 大小配置)
//! - 超时 ≤ 10 min

use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use crate::types::AuditEntry;
use std::sync::{Arc, Mutex};

/// 审核 manifest (注册前最后一道关)
pub fn audit_manifest(manifest: &Manifest) -> Result<()> {
    // 名称格式
    if manifest.name.is_empty() {
        return Err(ExtensionError::AuditRejected("name empty".into()));
    }
    // 至少 1 项权限
    if manifest.permissions.is_empty() {
        return Err(ExtensionError::AuditRejected(format!(
            "plugin '{}' has no permissions declared",
            manifest.name
        )));
    }
    // 大小下限
    if manifest.max_input_bytes < 1024 {
        return Err(ExtensionError::AuditRejected(format!(
            "max_input_bytes too small: {} (min 1024)",
            manifest.max_input_bytes
        )));
    }
    if manifest.max_output_bytes < 1024 {
        return Err(ExtensionError::AuditRejected(format!(
            "max_output_bytes too small: {} (min 1024)",
            manifest.max_output_bytes
        )));
    }
    // 超时
    if manifest.timeout_ms > 600_000 {
        return Err(ExtensionError::AuditRejected(format!(
            "timeout_ms too large: {} (max 600000)",
            manifest.timeout_ms
        )));
    }
    Ok(())
}

/// 审计日志 (线程安全)
#[derive(Debug, Default, Clone)]
pub struct AuditLog {
    inner: Arc<Mutex<Vec<AuditEntry>>>,
}

impl AuditLog {
    /// 构造空日志
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加成功记录
    pub fn push_success(&self, entry: AuditEntry) {
        let mut g = self.inner.lock().unwrap();
        g.push(entry);
    }

    /// 追加失败记录
    pub fn push_failure(&self, entry: AuditEntry) {
        let mut g = self.inner.lock().unwrap();
        g.push(entry);
    }

    /// 当前记录数
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 全部记录快照
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.inner.lock().unwrap().clone()
    }

    /// 仅失败记录
    pub fn failures(&self) -> Vec<AuditEntry> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .filter(|e| !e.success)
            .cloned()
            .collect()
    }

    /// 仅成功记录
    pub fn successes(&self) -> Vec<AuditEntry> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.success)
            .cloned()
            .collect()
    }

    /// 清空日志
    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    /// 按插件名过滤
    pub fn by_plugin(&self, plugin: &str) -> Vec<AuditEntry> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.plugin == plugin)
            .cloned()
            .collect()
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PluginKind;

    fn m_with_perms(n: usize) -> Manifest {
        Manifest {
            name: "test".into(),
            version: "0.1.0".into(),
            kind: PluginKind::Sync,
            description: "test".into(),
            entry: "lib.rs".into(),
            permissions: (0..n).map(|i| format!("p{i}")).collect(),
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
        }
    }

    #[test]
    fn audit_ok_with_minimum() {
        let m = m_with_perms(1);
        assert!(audit_manifest(&m).is_ok());
    }

    #[test]
    fn audit_rejects_no_permissions() {
        let m = m_with_perms(0);
        let res = audit_manifest(&m);
        assert!(matches!(res, Err(ExtensionError::AuditRejected(_))));
    }

    #[test]
    fn audit_rejects_tiny_input() {
        let mut m = m_with_perms(1);
        m.max_input_bytes = 100;
        let res = audit_manifest(&m);
        assert!(matches!(res, Err(ExtensionError::AuditRejected(_))));
    }

    #[test]
    fn audit_rejects_tiny_output() {
        let mut m = m_with_perms(1);
        m.max_output_bytes = 100;
        let res = audit_manifest(&m);
        assert!(matches!(res, Err(ExtensionError::AuditRejected(_))));
    }

    #[test]
    fn audit_rejects_huge_timeout() {
        let mut m = m_with_perms(1);
        m.timeout_ms = 700_000;
        let res = audit_manifest(&m);
        assert!(matches!(res, Err(ExtensionError::AuditRejected(_))));
    }

    #[test]
    fn log_push_and_len() {
        let log = AuditLog::new();
        assert_eq!(log.len(), 0);
        log.push_success(AuditEntry::success("a", PluginKind::Sync, 10, 20, 100));
        log.push_failure(AuditEntry::failure("b", PluginKind::Async, 30, 50, "oops"));
        assert_eq!(log.len(), 2);
        assert_eq!(log.successes().len(), 1);
        assert_eq!(log.failures().len(), 1);
    }

    #[test]
    fn log_by_plugin() {
        let log = AuditLog::new();
        log.push_success(AuditEntry::success("a", PluginKind::Sync, 10, 20, 100));
        log.push_success(AuditEntry::success("b", PluginKind::Async, 10, 20, 100));
        log.push_success(AuditEntry::success("a", PluginKind::Sync, 10, 20, 100));
        let a_entries = log.by_plugin("a");
        assert_eq!(a_entries.len(), 2);
    }

    #[test]
    fn log_clear() {
        let log = AuditLog::new();
        log.push_success(AuditEntry::success("a", PluginKind::Sync, 10, 20, 100));
        assert_eq!(log.len(), 1);
        log.clear();
        assert!(log.is_empty());
    }
}
