//! **战役 2-1 / Example — registry_demo**
//!
//! **目标**: 注册 3 个 mock 工具 (Sync / Async / Static) + 查 + 调用,
//! 演示 VCP 6 类 enum + 5 轴正交 + token 预算 + notify 热加载
//!
//! **运行**:
//! ```bash
//! cargo run -p apeireth-tool-registry --example registry_demo
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_tool_registry::{
    estimate_tool_tokens, MockAsyncTool, MockStaticTool, MockSyncTool, ToolKind, ToolRegistry,
    LIGHT_LIST_TOKEN_BUDGET, MAX_INJECTION_CHARS,
};
use serde_json::json;
use tempfile::TempDir;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    banner("战役 2-1 registry_demo 启动");

    // === 1. 新建 registry ===
    let registry = ToolRegistry::new();
    assert!(registry.is_empty());
    println!("[1] 新建空 registry, tools = 0");

    // === 2. 注册 3 个 mock 工具 (Sync / Async / Static) ===
    let sync_tool = Arc::new(MockSyncTool {
        name: "EchoSync".to_string(),
    });
    let async_tool = Arc::new(MockAsyncTool {
        name: "SlowAsync".to_string(),
        delay_ms: 50,
    });
    let static_tool = Arc::new(MockStaticTool {
        name: "ConfigVersion".to_string(),
        static_value: "0.14.0".to_string(),
    });

    registry.register("EchoSync".to_string(), sync_tool.clone());
    registry.register("SlowAsync".to_string(), async_tool.clone());
    registry.register("ConfigVersion".to_string(), static_tool.clone());

    println!(
        "[2] 注册 3 个 mock 工具, registry.list() = {:?}",
        registry.list()
    );
    assert_eq!(registry.len(), 3);

    // === 3. 列出 6 类分组 ===
    let by_kind = registry.list_by_kind();
    println!("[3] 按 6 类分组:");
    for kind in ToolKind::all().iter() {
        let names = by_kind.get(kind).cloned().unwrap_or_default();
        println!(
            "    {} ({:?}) → {} 个工具: {:?}",
            kind.as_legacy_str(),
            kind,
            names.len(),
            names
        );
    }

    // === 4. 调 3 个 mock 工具 (真跑 call) ===
    println!("[4] 真调 3 个 mock 工具:");

    // Sync
    let r1 = registry
        .get("EchoSync")
        .unwrap()
        .call(json!({"input": "hello"}))
        .await
        .unwrap();
    println!("    EchoSync.call({{input:\"hello\"}}) = {r1}");
    assert_eq!(r1["kind"], "sync");
    assert_eq!(r1["echo"], "hello");

    // Async (带 50ms 延迟)
    let start = Instant::now();
    let r2 = registry
        .get("SlowAsync")
        .unwrap()
        .call(json!({"input": "world"}))
        .await
        .unwrap();
    let elapsed = start.elapsed();
    println!("    SlowAsync.call({{input:\"world\"}}) = {r2} (elapsed = {elapsed:?})");
    assert_eq!(r2["kind"], "async");
    assert!(elapsed >= Duration::from_millis(50));

    // Static
    let r3 = registry
        .get("ConfigVersion")
        .unwrap()
        .call(json!({}))
        .await
        .unwrap();
    println!("    ConfigVersion.call({{}}) = {r3}");
    assert_eq!(r3["kind"], "static");
    assert_eq!(r3["value"], "0.14.0");

    // === 5. token 预算演示 ===
    println!("[5] VCP §6.2.2 #15 token 预算演示:");
    println!("    LIGHT_LIST_TOKEN_BUDGET = {LIGHT_LIST_TOKEN_BUDGET}");
    println!("    MAX_INJECTION_CHARS      = {MAX_INJECTION_CHARS}");

    let tokens = estimate_tool_tokens("EchoSync", "同步 echo 输入");
    println!("    estimate_tool_tokens(\"EchoSync\", \"同步 echo 输入\") = {tokens}");
    assert!(tokens < LIGHT_LIST_TOKEN_BUDGET);

    // === 6. notify 热加载演示 ===
    println!("[6] notify 热加载演示 (VCP chokidar → Rust notify 5.x):");
    let tmp = TempDir::new().expect("create tempdir");
    let watch_path = tmp.path().to_path_buf();
    println!("    监听目录: {}", watch_path.display());

    registry
        .watch_plugin_dir(&watch_path)
        .expect("watch_plugin_dir");
    println!("    watcher 启动, 等 100ms 稳定");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // 写文件触发事件
    let plugin_file = watch_path.join("MyPlugin.toml");
    std::fs::write(&plugin_file, "name = \"MyPlugin\"").expect("write file");
    println!("    写文件: {}", plugin_file.display());

    // 等通知 (Linux/macOS 快, Windows 较慢)
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let events = registry.take_notify_events();
    let hit = events
        .iter()
        .any(|p| p.file_name() == Some(std::ffi::OsStr::new("MyPlugin.toml")));
    if hit {
        println!(
            "    ✓ notify 事件触发: 共 {} 个, 首个 = {:?}",
            events.len(),
            events.first()
        );
    } else {
        println!("    ⚠ notify 事件未触发 (可能 Windows 上较慢, 实际事件: {events:?})");
        println!("    (本 example 是 best-effort 演示, CI 跑 6 类 mock 验证在 unit test)");
    }

    registry.stop_watching();
    println!("    watcher 停止");

    // === 7. 注销 ===
    println!("[7] 注销 EchoSync:");
    let removed = registry.unregister("EchoSync");
    println!("    removed.is_some() = {}", removed.is_some());
    assert_eq!(registry.len(), 2);

    banner("战役 2-1 registry_demo 完结 ✓");
    println!("总工具数: {}", registry.len());
}

fn banner(s: &str) {
    println!("========================================");
    println!("{s}");
    println!("========================================");
}
