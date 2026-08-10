//! extension_lifecycle — 6 类插件端到端演示
//!
//! 运行: `cargo run --example extension_lifecycle -p apeireth-extension`

use apeireth_extension::audit::AuditLog;
use apeireth_extension::manifest::Manifest;
use apeireth_extension::plugins::{
    AsyncPlugin, HybridPlugin, MessagePreprocessorPlugin, ServicePlugin, StaticPlugin, SyncPlugin,
};
use apeireth_extension::registry::AuditRegistry;
use apeireth_extension::sandbox::SandboxConfig;
use apeireth_extension::traits::ExtensionInput;
use apeireth_extension::types::PluginKind;
use serde_json::json;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-extension lifecycle demo ===");

    // 1. 从 TOML 解析 manifest (严格 schema)
    let toml_text = r#"
[extension]
name = "demo-add"
version = "0.1.0"
kind = "sync"
description = "Demo add plugin"
entry = "add.rs"
permissions = ["invoke"]
max_input_bytes = 4096
max_output_bytes = 4096
timeout_ms = 1000
"#;
    let manifest = Manifest::from_toml(toml_text).expect("TOML parse");
    println!("[1] Manifest parsed: {} ({})", manifest.name, manifest.kind);

    // 2. 构造 6 类插件
    let sync = SyncPlugin::example_add("sync-1");
    let async_p = AsyncPlugin::example_io("async-1");
    let static_p = StaticPlugin::example_lookup("static-1");
    let service = ServicePlugin::example_counter("service-1");
    let preproc = MessagePreprocessorPlugin::example_uppercase("preproc-1");
    let hybrid = HybridPlugin::example_echo("hybrid-1");

    // 3. 注册到 audit-then-register 中心
    let mut registry = AuditRegistry::new(SandboxConfig::privileged());
    registry.register(sync).unwrap();
    registry.register(async_p).unwrap();
    registry.register(static_p).unwrap();
    registry.register(service).unwrap();
    registry.register(preproc).unwrap();
    registry.register(hybrid).unwrap();
    println!("[2] Registered {} plugins", registry.len());
    for k in PluginKind::ALL {
        let names = registry.list_by_kind(*k);
        println!("    {} -> {} plugin(s)", k, names.len());
    }

    // 4. 调用 sync
    let out = registry
        .call("sync-1", ExtensionInput::new(json!({"a": 1.0, "b": 2.0})))
        .await
        .unwrap();
    println!("[3] sync-1 -> {}", out.result);

    // 5. 调用 async
    let out = registry
        .call("async-1", ExtensionInput::new(json!({"query": "hello"})))
        .await
        .unwrap();
    println!("[4] async-1 -> {}", out.result);

    // 6. 调用 static
    let out = registry
        .call("static-1", ExtensionInput::new(json!({"key": "alpha"})))
        .await
        .unwrap();
    println!("[5] static-1 -> {}", out.result);

    // 7. service 需要先 start
    let svc_handle = registry.manifest("service-1").unwrap();
    assert_eq!(svc_handle.kind, PluginKind::Service);
    println!(
        "[6] service-1 manifest: {} ({})",
        svc_handle.name, svc_handle.kind
    );

    // 8. 调用 preprocessor
    let out = registry
        .call(
            "preproc-1",
            ExtensionInput::new(json!({"text": "hello world"})),
        )
        .await
        .unwrap();
    println!("[7] preproc-1 -> {}", out.result);

    // 9. 调用 hybrid
    let out = registry
        .call("hybrid-1", ExtensionInput::new(json!({"x": 42})))
        .await
        .unwrap();
    println!("[8] hybrid-1 -> {}", out.result);

    // 10. 等待 hybrid 队列清空
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 11. 打印审计日志
    let log = registry.audit_log();
    println!("[9] Audit log: {} entries", log.len());
    for (i, e) in log.entries().iter().enumerate() {
        println!(
            "    #{}: {} ({}): in={}B out={}B elapsed={}us success={}",
            i + 1,
            e.plugin,
            e.kind,
            e.input_bytes,
            e.output_bytes,
            e.elapsed_us,
            e.success
        );
    }

    // 12. 统计
    let stats = registry.stats();
    println!(
        "[10] Stats: registered={} calls={} failures={} rejections={} audit_rejects={}",
        stats.registered,
        stats.total_calls,
        stats.total_failures,
        stats.total_rejections,
        stats.total_audit_rejects
    );

    // 13. AuditLog 独立使用示例
    let standalone = AuditLog::new();
    use apeireth_extension::types::AuditEntry;
    standalone.push_success(AuditEntry::success(
        "standalone",
        PluginKind::Sync,
        10,
        20,
        100,
    ));
    assert_eq!(standalone.len(), 1);
    println!("[11] Standalone AuditLog ok: {} entries", standalone.len());

    println!("=== demo complete ===");
}
