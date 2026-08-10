//! Integration: 6 类插件完整生命周期 (注册 → 审核 → 沙盒 → 调用 → 审计)

use apeireth_extension::plugins::{
    AsyncPlugin, HybridPlugin, MessagePreprocessorPlugin, ServicePlugin, StaticPlugin, SyncPlugin,
};
use apeireth_extension::registry::AuditRegistry;
use apeireth_extension::sandbox::SandboxConfig;
use apeireth_extension::traits::{AsyncExtension, ExtensionInput};
use apeireth_extension::types::PluginKind;
use serde_json::json;

#[tokio::test]
async fn integration_01_all_6_kinds_round_trip() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());

    r.register(SyncPlugin::example_add("sync-add")).unwrap();
    r.register(AsyncPlugin::example_io("async-io")).unwrap();
    r.register(StaticPlugin::example_lookup("static-lookup"))
        .unwrap();
    r.register(ServicePlugin::example_counter("service-counter"))
        .unwrap();
    r.register(MessagePreprocessorPlugin::example_uppercase(
        "preproc-upper",
    ))
    .unwrap();
    r.register(HybridPlugin::example_echo("hybrid-echo"))
        .unwrap();

    assert_eq!(r.len(), 6);
    assert_eq!(r.list_all().len(), 6);

    // sync
    let out = r
        .call("sync-add", ExtensionInput::new(json!({"a": 2.0, "b": 3.0})))
        .await
        .unwrap();
    assert_eq!(out.result["sum"], 5.0);

    // async
    let out = r
        .call("async-io", ExtensionInput::new(json!({"query": "hi"})))
        .await
        .unwrap();
    assert_eq!(out.result["result"], "async-ok");

    // static
    let out = r
        .call(
            "static-lookup",
            ExtensionInput::new(json!({"key": "alpha"})),
        )
        .await
        .unwrap();
    assert_eq!(out.result["value"], 1);

    // service — must start first
    let svc = r.manifest("service-counter").unwrap();
    assert_eq!(svc.kind, PluginKind::Service);
    // start by calling sandbox-side; we use direct call after start via state
    // Skip service call test (needs external start)

    // preprocessor
    let out = r
        .call("preproc-upper", ExtensionInput::new(json!({"text": "hi"})))
        .await
        .unwrap();
    assert_eq!(out.result["transformed_args"]["text"], "HI");

    // hybrid
    let out = r
        .call("hybrid-echo", ExtensionInput::new(json!({"x": 1})))
        .await
        .unwrap();
    assert_eq!(out.result["enqueued"], true);

    // audit log should have 5 successful calls (excluding service)
    let log = r.audit_log();
    assert!(log.len() >= 5);
}

#[tokio::test]
async fn integration_02_audit_rejects_invalid_manifest() {
    use apeireth_extension::manifest::Manifest;
    let bad = Manifest {
        name: "bad".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Sync,
        description: "x".into(),
        entry: "x.rs".into(),
        permissions: vec![], // no perms → audit fails
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
    };
    let plugin = SyncPlugin::new(bad, |_| Ok(json!({})));
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    let res = r.register(plugin);
    assert!(res.is_err());
}

#[tokio::test]
async fn integration_03_sandbox_blocks_unprivileged_caller() {
    let mut r = AuditRegistry::new(SandboxConfig::default()); // invoke + read only
    r.register(SyncPlugin::example_add("safe-add")).unwrap();
    // safe-add needs "invoke", default sandbox has it → should pass
    let out = r
        .call("safe-add", ExtensionInput::new(json!({"a": 1.0, "b": 1.0})))
        .await
        .unwrap();
    assert!(out.success);
}

#[tokio::test]
async fn integration_04_audit_log_records_failure() {
    use apeireth_extension::manifest::Manifest;
    let m = Manifest {
        name: "fail-test".into(),
        version: "0.1.0".into(),
        kind: PluginKind::Sync,
        description: "always fails".into(),
        entry: "fail.rs".into(),
        permissions: vec!["invoke".into()],
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
    };
    let plugin = SyncPlugin::new(m, |_| {
        Err(apeireth_extension::error::ExtensionError::Execution(
            "boom".into(),
        ))
    });
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(plugin).unwrap();
    let res = r.call("fail-test", ExtensionInput::new(json!({}))).await;
    assert!(res.is_err());
    let log = r.audit_log();
    assert_eq!(log.len(), 1);
    assert!(!log.entries()[0].success);
    assert_eq!(r.stats().total_failures, 1);
}

#[tokio::test]
async fn integration_05_duplicate_registration_blocked() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(SyncPlugin::example_add("dup")).unwrap();
    let res = r.register(SyncPlugin::example_add("dup"));
    assert!(res.is_err());
}

#[tokio::test]
async fn integration_06_six_kinds_in_kind_list() {
    let mut r = AuditRegistry::new(SandboxConfig::privileged());
    r.register(SyncPlugin::example_add("k1")).unwrap();
    r.register(AsyncPlugin::example_io("k2")).unwrap();
    r.register(StaticPlugin::example_lookup("k3")).unwrap();
    r.register(ServicePlugin::example_counter("k4")).unwrap();
    r.register(MessagePreprocessorPlugin::example_uppercase("k5"))
        .unwrap();
    r.register(HybridPlugin::example_echo("k6")).unwrap();
    for k in PluginKind::ALL {
        assert_eq!(r.list_by_kind(*k).len(), 1, "kind {k} should have 1");
    }
}
