//! **apeireth-mcp / browser_mcp_demo — R123-3 P2-12 浏览器自动化 skeleton 演示**
//!
//! **依据**: docs/v2-strategy/07 §2 P2-12 "不自研, 通过 apeireth-mcp 接 Playwright MCP server".
//! **借鉴 ID**: R123-3-VCP-BrowserMCP-2026-08-10
//!
//! **演示流程** (R123-3 skeleton, 0 真接 Playwright):
//! 1. 序列 5 个 action 各自的 params (展示 JSON Schema 1:1 借鉴 Playwright MCP wire)
//! 2. 组装 5 个 `BrowserRequest` envelope (`{action, params}`)
//! 3. 走 `browser_tool_handler_fn()` 闭包跑 1 次, 返 skeleton + "not installed" note
//!
//! **O-5 诚实标缺**: 全部 5 action 都返 `skeleton: true` + `note: "not installed"`,
//! R124+ 真接 Playwright MCP server.
//!
//! **运行**: `cargo run -p apeireth-mcp --example browser_mcp_demo`

use apeireth_mcp::tools::browser::{
    browser_tool_def, browser_tool_handler_fn, BrowserAction, BrowserClickParams,
    BrowserGetTextParams, BrowserNavigateParams, BrowserRequest, BrowserScreenshotParams,
    BrowserTypeParams,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-mcp browser_mcp_demo (R123-3 skeleton) ===\n");

    // ----- 1. 序列 5 个 action 各自的 params -----
    let nav = BrowserNavigateParams {
        url: "https://example.com".into(),
        timeout_ms: 30_000,
    };
    let click = BrowserClickParams {
        selector: "#submit-btn".into(),
        timeout_ms: 5_000,
    };
    let typ = BrowserTypeParams {
        selector: "#search-input".into(),
        text: "Apeireth".into(),
        delay_ms: 50,
    };
    let shot = BrowserScreenshotParams {
        format: "png".into(),
        full_page: true,
        filename: Some("example.png".into()),
    };
    let gt = BrowserGetTextParams {
        selector: "h1.title".into(),
    };

    for (name, val) in [
        ("navigate", serde_json::to_value(&nav)?),
        ("click", serde_json::to_value(&click)?),
        ("type", serde_json::to_value(&typ)?),
        ("screenshot", serde_json::to_value(&shot)?),
        ("get_text", serde_json::to_value(&gt)?),
    ] {
        println!("→ params.{name} = {val:#}");
    }

    // ----- 2. 组装 5 个 BrowserRequest envelope -----
    let reqs = vec![
        BrowserRequest {
            action: BrowserAction::Navigate.as_wire_str().into(),
            params: serde_json::to_value(&nav)?,
        },
        BrowserRequest {
            action: BrowserAction::Click.as_wire_str().into(),
            params: serde_json::to_value(&click)?,
        },
        BrowserRequest {
            action: BrowserAction::Type.as_wire_str().into(),
            params: serde_json::to_value(&typ)?,
        },
        BrowserRequest {
            action: BrowserAction::Screenshot.as_wire_str().into(),
            params: serde_json::to_value(&shot)?,
        },
        BrowserRequest {
            action: BrowserAction::GetText.as_wire_str().into(),
            params: serde_json::to_value(&gt)?,
        },
    ];
    for r in &reqs {
        let v = serde_json::to_value(r)?;
        println!("\n→ request = {v:#}");
    }

    // ----- 3. 走 browser_tool_handler_fn 跑 1 次 (navigate) -----
    let h = browser_tool_handler_fn();
    let nav_request = serde_json::to_value(&reqs[0])?;
    println!("\n→ invoke browser_tool_handler_fn(navigate)");
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(h(nav_request))?;
    println!("  ✓ result = {result:#}");

    // ----- 4. 展示 tool def (MCP tools/list 1 项) -----
    let def = browser_tool_def();
    println!("\n→ tool def:");
    println!("  name        = {}", def.name);
    println!("  description = {}", def.description);
    println!("  inputSchema = {}", def.inputSchema);

    println!("\n=== done (skeleton, 0 真接 Playwright, R124+ 真接) ===");
    Ok(())
}
