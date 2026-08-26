//! Integration: 沙盒 + 审核 + 审计 端到端 pipeline

use apeireth_extension::audit::AuditLog;
use apeireth_extension::error::ExtensionError;
use apeireth_extension::manifest::Manifest;
use apeireth_extension::plugins::{
    AsyncPlugin, HybridPlugin, MessagePreprocessorPlugin, ServicePlugin, StaticPlugin, SyncPlugin,
};
use apeireth_extension::registry::AuditRegistry;
use apeireth_extension::sandbox::SandboxConfig;
use apeireth_extension::traits::ExtensionInput;
use serde_json::json;

#[tokio::test]
async fn pipeline_01_register_caller_call_audit() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(SyncPlugin::example_add("p1")).unwrap();
    let _ = r
        .call("p1", ExtensionInput::new(json!({"a": 1.0, "b": 2.0})))
        .await
        .unwrap();
    let _ = r
        .call("p1", ExtensionInput::new(json!({"a": 10.0, "b": 20.0})))
        .await
        .unwrap();

    let log = r.audit_log();
    assert_eq!(log.len(), 2);
    assert!(log.by_plugin("p1").len() == 2);
    let stats = r.stats();
    assert_eq!(stats.total_calls, 2);
    assert_eq!(stats.total_failures, 0);
    assert_eq!(stats.registered, 1);
}

#[tokio::test]
async fn pipeline_02_audit_rejects_before_register() {
    use apeireth_extension::audit::audit_manifest;
    let m = Manifest {
        name: "bad".into(),
        version: "0.1.0".into(),
        kind: apeireth_extension::types::PluginKind::Sync,
        description: "x".into(),
        entry: "x.rs".into(),
        permissions: vec![], // empty → audit fails
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
    };
    let res = audit_manifest(&m);
    assert!(matches!(res, Err(ExtensionError::AuditRejected(_))));
}

#[tokio::test]
async fn pipeline_03_sandbox_blocks_oversize_input() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(SyncPlugin::example_add("p2")).unwrap();
    let huge = json!({"data": "x".repeat(2048)});
    let res = r.call("p2", ExtensionInput::new(huge)).await;
    assert!(matches!(res, Err(ExtensionError::InputTooLarge { .. })));
    // failure recorded in audit log
    let log = r.audit_log();
    assert_eq!(log.failures().len(), 1);
}

#[tokio::test]
async fn pipeline_04_sandbox_missing_permission() {
    use apeireth_extension::types::PluginKind;
    let m = Manifest {
        name: "needs-system".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Sync,
        description: "needs system perm".into(),
        entry: "x.rs".into(),
        permissions: vec!["system".into()],
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
    };
    let plugin = SyncPlugin::new(m, |_| Ok(json!({})));
    let mut r = AuditRegistry::new(SandboxConfig::default()); // no system perm
    r.register(plugin).unwrap();
    let res = r.call("needs-system", ExtensionInput::new(json!({}))).await;
    assert!(matches!(res, Err(ExtensionError::PermissionDenied { .. })));
}

#[tokio::test]
async fn pipeline_05_audit_log_separate_per_plugin() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(SyncPlugin::example_add("a")).unwrap();
    r.register(AsyncPlugin::example_io("b")).unwrap();
    r.register(StaticPlugin::example_lookup("c")).unwrap();
    r.register(ServicePlugin::example_counter("d")).unwrap();
    r.register(MessagePreprocessorPlugin::example_uppercase("e"))
        .unwrap();
    r.register(HybridPlugin::example_echo("f")).unwrap();
    let _ = r
        .call("a", ExtensionInput::new(json!({"a": 1.0, "b": 1.0})))
        .await;
    let _ = r
        .call("b", ExtensionInput::new(json!({"query": "x"})))
        .await;
    let _ = r
        .call("c", ExtensionInput::new(json!({"key": "alpha"})))
        .await;
    let _ = r.call("e", ExtensionInput::new(json!({"text": "x"}))).await;
    let _ = r.call("f", ExtensionInput::new(json!({}))).await;
    let log = r.audit_log();
    let mut plugins_called = std::collections::BTreeSet::new();
    for e in log.entries() {
        plugins_called.insert(e.plugin.clone());
    }
    // 5 plugins called (d not called because service not started)
    assert_eq!(plugins_called.len(), 5);
}

#[tokio::test]
async fn pipeline_06_stats_reflect_calls() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(SyncPlugin::example_add("p")).unwrap();
    let _ = r
        .call("p", ExtensionInput::new(json!({"a": 1.0, "b": 1.0})))
        .await;
    let _ = r
        .call("p", ExtensionInput::new(json!({"a": 2.0, "b": 2.0})))
        .await;
    let _ = r.call("ghost", ExtensionInput::new(json!({}))).await; // not found
    let s = r.stats();
    assert_eq!(s.total_calls, 2); // not_found does not count
    assert_eq!(s.registered, 1);
}

#[tokio::test]
async fn pipeline_07_audit_log_thread_safe() {
    use apeireth_extension::types::{AuditEntry, PluginKind};
    use std::sync::Arc;
    let log = Arc::new(AuditLog::new());
    let mut handles = vec![];
    for i in 0..5 {
        let l = log.clone();
        handles.push(tokio::spawn(async move {
            l.push_success(AuditEntry::success(
                &format!("p{i}"),
                PluginKind::Sync,
                10,
                20,
                100,
            ));
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(log.len(), 5);
}
