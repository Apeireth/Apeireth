//! M0 探针：真实发现 REA MCP server 的工具清单（默认忽略，手动运行）。
//!
//! ```text
//! cargo test -p apeireth-tools-canonical --test rea_bridge_probe -- --ignored --nocapture
//! ```
//!
//! 依赖：本机已安装固定版本 REA（`/usr/local/bin/rea-mcp`，见 `/opt/rea`）。
//! 可用 `REA_MCP_CMD` 覆盖（例如 `npx`）。
//! 该测试不属于常规 CI：它验证的是本机 provider 的真实可用性。

use apeireth_tools_canonical::mcp_bridge::{
    McpBridgeOptions, McpServerConfig, McpServerSpec, McpToolBridge,
};

#[tokio::test]
#[ignore = "requires a local REA install; run manually for M0 capability probing"]
async fn discovers_rea_tools() {
    let command = std::env::var("REA_MCP_CMD").unwrap_or_else(|_| "rea-mcp".to_string());
    let spec = McpServerSpec::stdio("rea", command)
        .with_auto_authorize("inspect_artifact")
        .with_auto_authorize("list_names");
    let config = McpServerConfig::new(vec![spec]);
    let bridge =
        McpToolBridge::new(config, McpBridgeOptions::default()).expect("rea spec must validate");

    let report = bridge.refresh().await.expect("rea tools/list must succeed");
    let names = report.registered_names();

    eprintln!("REA discovered {} tools", names.len());
    for name in names.iter().take(10) {
        eprintln!("  {name}");
    }
    let live: Vec<_> = names.iter().filter(|n| n.starts_with("mcp:rea:")).collect();
    eprintln!("namespaced under mcp:rea: {}", live.len());

    assert!(
        names.len() >= 200,
        "expected at least 200 REA tools, discovered {}",
        names.len()
    );

    // 操作员白名单必须转成显式 grant：列出的工具自动放行，未列出的仍需审批。
    let policy = bridge.policy().lock().expect("policy lock");
    assert_eq!(
        policy.decision_for_capability("tool.mcp.rea.inspect_artifact"),
        apeireth_governance::Decision::Allow
    );
    assert_ne!(
        policy.decision_for_capability("tool.mcp.rea.comment"),
        apeireth_governance::Decision::Allow
    );
}
