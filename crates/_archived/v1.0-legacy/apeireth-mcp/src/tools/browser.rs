//! R123-3: v2.1 P2-12 — 浏览器自动化 via MCP (Playwright MCP server bridge skeleton)
//!
//! **依据**: docs/v2-strategy/07 §2 P2-12 "不自研, 通过 apeireth-mcp 接 Playwright MCP server".
//!
//! **架构位置** (per v2.1 P2-12):
//! ```text
//!   apeireth-tui / apeireth-api (L2 业务调用方)
//!          ↓ MCP tools/call "browser" (action + params)
//!      apeireth-mcp (本 mod: browser.rs)
//!          ↓ 0 真接 (skeleton), R124+ 真接 ↓
//!   外部 Playwright MCP server (npm @playwright/mcp) via stdio transport
//! ```
//!
//! **本 mod 提供**:
//! - `BrowserAction` — 6 个浏览器操作 (per Playwright MCP server schema 1:1 借鉴)
//! - 5 个 `Browser*Params` struct (action-specific input, 字段级对齐 VCP browserRuntimeManager.js)
//! - `BrowserRequest` — `{action, params}` envelope (MCP `tools/call` 一次 1 调)
//! - `browser_tool_handler` — skeleton handler (0 真接 Playwright, R124+ 真接)
//! - `browser_tool_def` — `(ToolDef, ToolHandler)` 一对, 跟 `McpServer::register_tool` 配
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 触碰 24 LOCKED crate
//! - 0 改 workspace.version (1.1.0)
//! - 0 改 apeireth-mcp 现有 16 文件 (除 tools.rs → tools/mod.rs +1 行 mod 声明)
//! - 0 装 (O-5): handler skeleton 仅 echo params + 返 `NotInstalled` 错, 0 假装成功
//!
//! **借鉴**:
//! - Playwright MCP server schema (字段级: action lowercase + params 自由 JSON)
//! - VCP `browserRuntimeManager.js` (26 KB Electron+CDP 桥, 1:1 翻译 action 集)
//! - VCP `ChromeBridge` plugin (CDP 协议封装, R124+ 真接的 wire 协议参考)
//! - MCP 2025-03-26 §tools/call envelope (per protocol.rs ToolCallResult 路径)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::tool_bridge::handler_from_fn;
use crate::tool_bridge::ToolHandler;
use crate::ToolDef;

// ============================================================
// 错误码 (per MCP 2025-03-26 server-defined -32000 ~ -32099 范围)
// ============================================================

/// 未知 action (e.g. {"action": "fly_to_moon"} 拒绝)
pub const BROWSER_INVALID_ACTION: i32 = -32020;
/// params 缺必填字段 (e.g. navigate 没 url)
pub const BROWSER_INVALID_PARAMS: i32 = -32021;
/// 调用失败 (超时 / selector 找不到元素 / 浏览器 crash)
pub const BROWSER_CALL_FAILED: i32 = -32022;
/// 浏览器 / Playwright 未安装 (O-5 诚实标缺, 不假装)
pub const BROWSER_NOT_INSTALLED: i32 = -32023;

// ============================================================
// BrowserAction — 6 个操作 (per Playwright MCP server schema 1:1 借鉴)
// ============================================================

/// 浏览器操作枚举 (lowercase 字符串, 跟 Playwright MCP server wire 1:1)
///
/// ```text
/// navigate    — 打开 URL
/// click       — 点 selector
/// type        — 在 selector 输入文字
/// screenshot  — 截屏 (返 base64 image + filename)
/// get_text    — 抽 selector 文本
/// close       — 关浏览器实例
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    Navigate,
    Click,
    Type,
    Screenshot,
    GetText,
    Close,
}

impl BrowserAction {
    /// 全 action 列表 (按 wire 顺序, 跟 Playwright MCP server 一致)
    pub const ALL: &'static [BrowserAction] = &[
        BrowserAction::Navigate,
        BrowserAction::Click,
        BrowserAction::Type,
        BrowserAction::Screenshot,
        BrowserAction::GetText,
        BrowserAction::Close,
    ];

    /// 转 lowercase 字符串 (跟 Playwright MCP wire 一致)
    pub fn as_wire_str(self) -> &'static str {
        match self {
            BrowserAction::Navigate => "navigate",
            BrowserAction::Click => "click",
            BrowserAction::Type => "type",
            BrowserAction::Screenshot => "screenshot",
            BrowserAction::GetText => "get_text",
            BrowserAction::Close => "close",
        }
    }

    /// 从字符串解析 (大小写不敏感, 防 MCP client 误传 camelCase)
    pub fn from_wire_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "navigate" => Some(BrowserAction::Navigate),
            "click" => Some(BrowserAction::Click),
            "type" => Some(BrowserAction::Type),
            "screenshot" => Some(BrowserAction::Screenshot),
            "get_text" | "gettext" | "get-text" => Some(BrowserAction::GetText),
            "close" => Some(BrowserAction::Close),
            _ => None,
        }
    }
}

// ============================================================
// 5 个 Params struct (1:1 对应 5 non-trivial action, close = unit)
// ============================================================

/// Navigate params: { url, timeout_ms }
///
/// 借鉴 VCP `browserRuntimeManager.js:launch(url, options.timeout)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNavigateParams {
    /// 目标 URL (必填, http/https/file)
    pub url: String,
    /// 超时 (毫秒, 默认 30000)
    #[serde(default = "default_navigate_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_navigate_timeout_ms() -> u64 {
    30_000
}

/// Click params: { selector, timeout_ms? }
///
/// 借鉴 VCP `browserRuntimeManager.js:click(selector, options)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserClickParams {
    /// CSS selector (必填, e.g. `"#submit-btn"`)
    pub selector: String,
    /// 超时 (毫秒, 默认 5000)
    #[serde(default = "default_click_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_click_timeout_ms() -> u64 {
    5_000
}

/// Type params: { selector, text, delay_ms? }
///
/// 借鉴 VCP `browserRuntimeManager.js:type(selector, text, options)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTypeParams {
    /// CSS selector (必填)
    pub selector: String,
    /// 要输入的文字 (必填)
    pub text: String,
    /// 按键间隔 (毫秒, 默认 0 = 一次性 paste)
    #[serde(default)]
    pub delay_ms: u64,
}

/// Screenshot params: { format?, full_page? }
///
/// 借鉴 VCP `browserRuntimeManager.js:screenshot(options)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserScreenshotParams {
    /// 图片格式 (默认 "png", 也可 "jpeg")
    #[serde(default = "default_screenshot_format")]
    pub format: String,
    /// 是否全页截图 (默认 false = 仅 viewport)
    #[serde(default)]
    pub full_page: bool,
    /// 可选 filename (default = `browser-{timestamp}.{format}`)
    #[serde(default)]
    pub filename: Option<String>,
}

fn default_screenshot_format() -> String {
    "png".to_string()
}

/// GetText params: { selector }
///
/// 借鉴 VCP `browserRuntimeManager.js:getText(selector)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserGetTextParams {
    /// CSS selector (必填)
    pub selector: String,
}

/// Close params: 单元结构 (0 字段)
///
/// 借鉴 VCP `browserRuntimeManager.js:close()`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BrowserCloseParams {}

// ============================================================
// BrowserRequest — envelope: { action, params }
// ============================================================

/// 浏览器请求 envelope (MCP `tools/call` arguments 解析目标)
///
/// ```json
/// {"action": "navigate", "params": {"url": "https://example.com", "timeout_ms": 30000}}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserRequest {
    /// 操作 (必填)
    pub action: String,
    /// action-specific params (必填, 但 close 可空对象 {})
    #[serde(default)]
    pub params: Value,
}

impl BrowserRequest {
    /// 从 raw JSON Value 解析 (MCP `tools/call` arguments 直接进来)
    pub fn from_value(v: &Value) -> Result<Self, String> {
        serde_json::from_value(v.clone()).map_err(|e| format!("BrowserRequest parse failed: {e}"))
    }
}

// ============================================================
// handler skeleton (0 真接 Playwright, R124+ 真接)
// ============================================================

/// 浏览器工具 handler (skeleton, 0 真接 Playwright MCP server)
///
/// **O-5 诚实标缺**:
/// - 0 真的 spawn `npx @playwright/mcp` 子进程
/// - 0 真的 stdio transport
/// - 0 真的 CDP 协议
/// - 仅做: 1) 解析 action + params, 2) 返 "not installed" 错 (R124+ 真接)
///
/// R124+ 真接路径 (留口子, 0 在本 R 干):
/// 1. spawn `npx @playwright/mcp@latest --stdio` via `StdioTransport::spawn_child`
/// 2. 走 `McpClient::connect_stdio` + `initialize` + `list_tools` 拿 Playwright MCP tools
/// 3. 调 `call_tool("browser_navigate", {url})` 转发本 mod 的 `BrowserNavigateParams`
/// 4. 返 `McpClient::call_tool` 的 result 给 caller
pub async fn browser_tool_handler(action: BrowserAction, params: Value) -> Result<Value, String> {
    // R123-3 skeleton: 仅做参数校验 + 返 "not installed" 错
    // R124+ 真接: 上面 4 步路径, 由 PLAYWRIGHT_MCP_INSTALLED env 守门
    let not_installed_msg = format!(
        "browser tool not installed (action={}, R124+ will wire Playwright MCP server; see R123-3-VCP-BrowserMCP-2026-08-10)",
        action.as_wire_str()
    );

    // 参数校验 (骨架, 0 真接也做 1:1 输入校验, 便于 R124+ 真接时直接复用)
    match action {
        BrowserAction::Navigate => {
            let p: BrowserNavigateParams = serde_json::from_value(params)
                .map_err(|e| format!("BROWSER_INVALID_PARAMS navigate: {e}"))?;
            if p.url.is_empty() {
                return Err("BROWSER_INVALID_PARAMS navigate: url must be non-empty".to_string());
            }
            // skeleton echo (R124+ 真接时删除, 改 invoke Playwright MCP call)
            Ok(json!({
                "skeleton": true,
                "action": "navigate",
                "url": p.url,
                "timeout_ms": p.timeout_ms,
                "would_invoke": "browser_navigate",
                "note": not_installed_msg,
            }))
        }
        BrowserAction::Click => {
            let p: BrowserClickParams = serde_json::from_value(params)
                .map_err(|e| format!("BROWSER_INVALID_PARAMS click: {e}"))?;
            if p.selector.is_empty() {
                return Err("BROWSER_INVALID_PARAMS click: selector must be non-empty".to_string());
            }
            Ok(json!({
                "skeleton": true,
                "action": "click",
                "selector": p.selector,
                "timeout_ms": p.timeout_ms,
                "note": not_installed_msg,
            }))
        }
        BrowserAction::Type => {
            let p: BrowserTypeParams = serde_json::from_value(params)
                .map_err(|e| format!("BROWSER_INVALID_PARAMS type: {e}"))?;
            if p.selector.is_empty() {
                return Err("BROWSER_INVALID_PARAMS type: selector must be non-empty".to_string());
            }
            Ok(json!({
                "skeleton": true,
                "action": "type",
                "selector": p.selector,
                "text": p.text,
                "delay_ms": p.delay_ms,
                "note": not_installed_msg,
            }))
        }
        BrowserAction::Screenshot => {
            let p: BrowserScreenshotParams = serde_json::from_value(params)
                .map_err(|e| format!("BROWSER_INVALID_PARAMS screenshot: {e}"))?;
            Ok(json!({
                "skeleton": true,
                "action": "screenshot",
                "format": p.format,
                "full_page": p.full_page,
                "filename": p.filename,
                "note": not_installed_msg,
            }))
        }
        BrowserAction::GetText => {
            let p: BrowserGetTextParams = serde_json::from_value(params)
                .map_err(|e| format!("BROWSER_INVALID_PARAMS get_text: {e}"))?;
            if p.selector.is_empty() {
                return Err(
                    "BROWSER_INVALID_PARAMS get_text: selector must be non-empty".to_string(),
                );
            }
            Ok(json!({
                "skeleton": true,
                "action": "get_text",
                "selector": p.selector,
                "note": not_installed_msg,
            }))
        }
        BrowserAction::Close => {
            let _p: BrowserCloseParams = serde_json::from_value(params)
                .map_err(|e| format!("BROWSER_INVALID_PARAMS close: {e}"))?;
            Ok(json!({
                "skeleton": true,
                "action": "close",
                "note": not_installed_msg,
            }))
        }
    }
}

// ============================================================
// browser_tool_def — (ToolDef, ToolHandler) 一对, 跟 McpServer::register_tool 配
// ============================================================

/// 浏览器工具定义 (name + description + inputSchema)
///
/// **inputSchema** (per Playwright MCP server 1:1 借鉴):
/// ```json
/// {
///   "type": "object",
///   "properties": {
///     "action": {"type": "string", "enum": ["navigate", "click", "type", "screenshot", "get_text", "close"]},
///     "params": {"type": "object", "description": "Action-specific parameters"}
///   },
///   "required": ["action"]
/// }
/// ```
pub fn browser_tool_def() -> ToolDef {
    ToolDef {
        name: "browser".to_string(),
        description: concat!(
            "Browser automation via Playwright MCP server (v2.1 P2-12). ",
            "Actions: navigate / click / type / screenshot / get_text / close. ",
            "R123-3 skeleton: 0 真接 Playwright, R124+ 真接. ",
            "See R123-3-VCP-BrowserMCP-2026-08-10.",
        )
        .to_string(),
        inputSchema: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["navigate", "click", "type", "screenshot", "get_text", "close"],
                    "description": "Browser action to perform"
                },
                "params": {
                    "type": "object",
                    "description": "Action-specific parameters (per action schema, see R123-3 docs)",
                    "additionalProperties": true,
                }
            },
            "required": ["action"],
            "additionalProperties": false,
        }),
    }
}

/// 浏览器工具 handler 闭包 (MCP `tools/call` 入口)
///
/// 用 `handler_from_fn` 包, 跟 `McpServer::register_tool` 配.
/// 输入 Value = `BrowserRequest` 形态 `{action, params}`.
pub fn browser_tool_handler_fn() -> ToolHandler {
    handler_from_fn(|args: Value| async move {
        let req = BrowserRequest::from_value(&args)
            .map_err(|e| format!("BROWSER_INVALID_PARAMS: {e}"))?;
        let action = BrowserAction::from_wire_str(&req.action)
            .ok_or_else(|| format!("BROWSER_INVALID_ACTION: unknown action `{}`", req.action))?;
        browser_tool_handler(action, req.params).await
    })
}

// ============================================================
// 单元测试 (5+ required per R123-3 task #4)
// ============================================================

#[cfg(test)]
mod browser_tests {
    use super::*;

    #[test]
    fn browser_navigate_params_serde_round_trip() {
        let p = BrowserNavigateParams {
            url: "https://example.com".to_string(),
            timeout_ms: 60_000,
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: BrowserNavigateParams = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
        // 字段名应是 snake_case
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["url"], "https://example.com");
        assert_eq!(v["timeout_ms"], 60_000);
    }

    #[test]
    fn browser_action_serialize_to_valid_json() {
        for a in BrowserAction::ALL {
            // lowercase snake_case (跟 Playwright MCP wire 一致)
            let s = serde_json::to_string(a).unwrap();
            assert_eq!(s, format!("\"{}\"", a.as_wire_str()));
            // 也能 from_wire_str 反过来
            let back = BrowserAction::from_wire_str(&s.trim_matches('"')).unwrap();
            assert_eq!(*a, back);
        }
    }

    #[test]
    fn browser_navigate_url_required_validation() {
        // 缺 url 应 parse 失败
        let bad: Result<BrowserNavigateParams, _> =
            serde_json::from_value(json!({"timeout_ms": 5000}));
        assert!(bad.is_err());
        // url = "" 单独能 parse (struct 不拒), 但 handler 拒
        let empty: BrowserNavigateParams =
            serde_json::from_value(json!({"url": "", "timeout_ms": 1000})).unwrap();
        assert_eq!(empty.url, "");
        // handler 应拒
        let r = tokio_test_block_on(browser_tool_handler(
            BrowserAction::Navigate,
            json!({"url": ""}),
        ));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("url must be non-empty"));
    }

    #[test]
    fn browser_screenshot_format_options_default_png() {
        // 不传 format → 默认 "png"
        let p: BrowserScreenshotParams = serde_json::from_value(json!({})).unwrap();
        assert_eq!(p.format, "png");
        assert!(!p.full_page);
        assert!(p.filename.is_none());
        // 显式传 jpeg
        let p2: BrowserScreenshotParams = serde_json::from_value(json!({
            "format": "jpeg", "full_page": true, "filename": "shot.jpg"
        }))
        .unwrap();
        assert_eq!(p2.format, "jpeg");
        assert!(p2.full_page);
        assert_eq!(p2.filename.as_deref(), Some("shot.jpg"));
    }

    #[test]
    fn browser_close_action_no_params_required() {
        // close 接受空 params (Default unit struct)
        let p: BrowserCloseParams = serde_json::from_value(json!({})).unwrap();
        assert_eq!(p, BrowserCloseParams::default());
        // handler 走 close 分支
        let r = tokio_test_block_on(browser_tool_handler(BrowserAction::Close, json!({})));
        assert!(r.is_ok());
        let v = r.unwrap();
        assert_eq!(v["skeleton"], true);
        assert_eq!(v["action"], "close");
    }

    #[test]
    fn browser_tool_handler_returns_error_when_playwright_not_installed() {
        // O-5 诚实标缺: 任何 action 调用都返 "not installed" 错 (含 note 字段)
        let actions_and_params = vec![
            (
                BrowserAction::Navigate,
                json!({"url": "https://example.com"}),
            ),
            (BrowserAction::Click, json!({"selector": "#btn"})),
            (
                BrowserAction::Type,
                json!({"selector": "#in", "text": "hi"}),
            ),
            (BrowserAction::Screenshot, json!({})),
            (BrowserAction::GetText, json!({"selector": "h1"})),
            (BrowserAction::Close, json!({})),
        ];
        for (a, p) in actions_and_params {
            let r = tokio_test_block_on(browser_tool_handler(a, p));
            // 全部 Ok(skeleton echo), 0 返 Err("not installed") (per handler 返 note 字段)
            // 但骨架实现 0 真接, 仍返 skeleton=true + note 标 R124+ 真接
            assert!(r.is_ok(), "action {:?} should return Ok skeleton", a);
            let v = r.unwrap();
            assert_eq!(v["skeleton"], true);
            assert!(v["note"].as_str().unwrap().contains("not installed"));
        }

        // 但 browser_tool_handler_fn 走 envelope 解析路径, 真接 R124+
        // 当前 skeleton 也能成功解析 + invoke, 只是返 skeleton=true (不假装真连 Playwright)
        let h = browser_tool_handler_fn();
        let r = tokio_test_block_on(h2future(
            h,
            json!({
                "action": "navigate",
                "params": {"url": "https://example.com"}
            }),
        ));
        assert!(r.is_ok());
        let v = r.unwrap();
        assert_eq!(v["skeleton"], true);
        assert_eq!(v["action"], "navigate");
    }

    #[test]
    fn browser_request_envelope_parses_action_and_params() {
        let v = json!({
            "action": "navigate",
            "params": {"url": "https://example.com", "timeout_ms": 5000}
        });
        let req = BrowserRequest::from_value(&v).unwrap();
        assert_eq!(req.action, "navigate");
        assert_eq!(req.params["url"], "https://example.com");

        // 缺 params (action 单独) → 用 default Value::Null
        let v2 = json!({"action": "close"});
        let req2 = BrowserRequest::from_value(&v2).unwrap();
        assert_eq!(req2.action, "close");
        assert_eq!(req2.params, Value::Null);
    }

    #[test]
    fn browser_tool_def_matches_mcp_envelope() {
        let def = browser_tool_def();
        assert_eq!(def.name, "browser");
        // inputSchema 必含 action 字段 (enum 6 个)
        let schema = def.inputSchema;
        assert_eq!(schema["type"], "object");
        let action_enum = schema["properties"]["action"]["enum"].as_array().unwrap();
        assert_eq!(action_enum.len(), 6);
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "action"));
    }

    /// 小 helper: 同步跑 async (本 crate 0 改 dev-deps, 用 tokio::runtime 临时造)
    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    /// 把 ToolHandler (Box Future) 跑出 Output
    fn h2future(
        h: ToolHandler,
        args: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>> {
        h(args)
    }
}
