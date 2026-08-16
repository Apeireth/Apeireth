//! education_suite_demo — 教育升级套件端到端演示 (插件 → 套件 → 真工具调用).
//!
//! 链路: 装插件 education-dx-check (注册 dx_check 工具 + 授权)
//!      → 装配 education-suite (校验插件已装 + 工具已注册 + 登记权限包)
//!      → 桥执行 dx_check: 忘换 dx 场景 → fix; 正确场景 → ok.
//!
//! 0 假装: 规则层检查 (非 CAS); 演示即验收 — 真跑真输出.

use std::sync::Arc;

use apeireth_companion::education::EducationDxPlugin;
use apeireth_companion::plugin::PluginRegistry;
use apeireth_companion::suites::SuiteCatalog;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::SqliteMemoryStore;
use apeireth_tool_runtime::parser::ParsedToolCall;
use serde_json::json;

#[tokio::main]
async fn main() {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let bridge = Arc::new(ToolBridge::new(store));
    let plugins = PluginRegistry::new();

    println!("═══════════ education_suite_demo — 教育升级套件 ═══════════\n");

    // 1) 先装插件 (生态最小单元)
    plugins.install(&bridge, Arc::new(EducationDxPlugin)).unwrap();
    println!("[1] 插件已装: education-dx-check (dx_check 工具已注册)");

    // 2) 套件装配 (校验: 插件已装 + 工具已注册 + 授权)
    let cat = SuiteCatalog::builtin();
    let r = cat.install_with_plugins(&bridge, Some(&plugins), "education-suite").unwrap();
    println!("[2] {r}");

    // 3) 真工具调用 — 忘换 dx 场景 (主人两个老毛病之一)
    let bad = ParsedToolCall {
        tool_name: "dx_check".into(),
        args: json!({
            "problem": "∫ x·√(1-x²) dx",
            "substitution": "令 t = 1-x²",
            "after": "∫ x·√(t) dx"
        }),
        raw_marker: String::new(),
        archery: false,
        archery_no_reply: false,
    };
    let r = bridge.execute_if_allowed(&bad).await;
    println!("\n[3] 忘换 dx 场景 → {:?}", r.output);

    // 4) 正确写法 + 根号模式提示
    let good = ParsedToolCall {
        tool_name: "dx_check".into(),
        args: json!({
            "problem": "∫ x/√(1-x²) dx",
            "substitution": "x = sinθ",
            "after": "∫ (sinθ/cosθ) · cosθ dθ"
        }),
        raw_marker: String::new(),
        archery: false,
        archery_no_reply: false,
    };
    let r = bridge.execute_if_allowed(&good).await;
    println!("[4] 三角换元正确场景 → {:?}", r.output);

    // 5) 卸载插件 → 套件应拒绝再装配 (插件是前置条件)
    plugins.uninstall(&bridge, "education-dx-check").unwrap();
    let err = cat.install_with_plugins(&bridge, Some(&plugins), "education-suite").unwrap_err();
    println!("\n[5] 卸载后装配拒绝 (预期): {err}");
    println!("\n═══════════ 演示完成: 教育套件 = 插件组官方打包, 全链路真跑 ═══════════");
}
