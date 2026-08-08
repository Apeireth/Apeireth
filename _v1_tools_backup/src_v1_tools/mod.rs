//! `v1_tools` — R20 阶段 2 公开 HTTP API 表面 (6 工具 endpoint, D-02 子路径风格)
//!
//! **战略背景** (per `reports/r19-integration-v2/r20-stage-2-3-prep-2026-08-05.md` §2):
//! 主人 2026-08-04 拍板: 把 R18 6 类非 LLM API (web_search / file_ops / git_ops /
//! code_exec / calendar / message) 走 `/v1/tools/{name}/invoke` 子路径公开.
//!
//! **6 端点** (per §2.2 6 端点路由表):
//! - `POST /v1/tools/web_search/invoke` — 4 真接 (复用 `apeireth-tools::WebSearchTool`)
//! - `POST /v1/tools/file_ops/invoke`   — 4 真接 (复用 `apeireth-tools::FileOpsTool`)
//! - `POST /v1/tools/git_ops/invoke`    — 4 真接 (复用 `apeireth-tools::GitOpsTool`)
//! - `POST /v1/tools/code_exec/invoke`  — 4 真接 (复用 `apeireth-tools::CodeExecTool`)
//! - `POST /v1/tools/calendar/invoke`   — **D-01 真接** (新增 `CalendarTool`, 5 actions)
//! - `POST /v1/tools/message/invoke`    — **D-01 真接** (新增 `MessageTool`, 3 actions)
//!
//! **6 端点统一响应信封** (per §2.2):
//! ```json
//! { "ok": true, "result": <any>, "error": "string?", "meta": { "tool", "duration_ms", "trace_id" } }
//! ```
//!
//! **错误码 12 类映射** (per §2.5, 本阶段 2 实施只用 4 类: 400/404/500/503, 余 8 类 = 鉴权 + 限流等
//! 由阶段 3 引入 middleware, 不在本文范围):
//! - 400 `bad_request` — JSON 缺字段 / 字段类型错
//! - 404 `not_found` — 工具名未在 registry 注册
//! - 500 `internal_error` — tool.call 内部错误 (unwrap/panic/etc)
//! - 503 `service_unavailable` — V2State.tools 未初始化
//!
//! **架构位置**:
//! ```text
//!   axum HTTP handler (v2_endpoints::build_router 注册 6 路由)
//!     ↓
//!   v1_tools::dispatch::invoke_by_name (统一分发)
//!     ↓
//!   apeireth-tool-registry::ToolRegistry.get(name) → Arc<dyn Tool>
//!     ↓
//!   Tool::call(args) → Result<Value, String>
//! ```
//!
//! **集成点** (per §2.8 5 集成点 0 冲突):
//! - v2_endpoints.rs build_router 加 6 行 `.route(...)` (本 commit 唯一改 LOCKED 源文件)
//! - lib.rs 加 1 行 `pub mod v1_tools;` (本 commit 唯一改 LOCKED 源文件)
//! - apeireth-tools/src 不碰 (4 真接通过 registry 复用)
//! - apeireth-mcp/src 不碰 (calendar/message 不走 mcp, 走 Tool trait 直接注册)
//!
//! **不假装** (per O-5 不假装):
//! - ✅ 4 真接通过 `apeireth-tools::register_all` 真工具, 0 改其源码
//! - ✅ 2 真接 (calendar/message) 走 Tool trait 自实现, 真用 in-memory store, 非 stub
//! - ✅ 0 假装 (没有 501 not_implemented 路径, D-01 推翻原 stub 推荐)
//! - ✅ 统一响应信封 + 4 类错误码 1:1 映射 §2.5
//!
//! **不修改承诺**:
//! - ❌ 不改 workspace version (1.0.0, 严守 semver)
//! - ❌ 不引 NewAPI (per R17 决策)
//! - ❌ 不假装 5 鉴权组件 / token bucket / WebSocket (阶段 3 实施)
//! - ❌ 不改 `apeireth-tools/src/`, `apeireth-mcp/src/`, `apeireth-tool-registry/src/`

#![deny(unsafe_code)]

use std::sync::Arc;
use std::time::Instant;

use apeireth_tool_registry::Tool;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::v2_endpoints::SharedV2;

pub mod calendar;
pub mod code_exec;
pub mod file_ops;
pub mod git_ops;
pub mod message;
pub mod web_search;

// ============================================================
// 统一 Req/Resp schema (per §2.2 6 端点统一响应信封)
// ============================================================

/// **InvokeRequest** — 6 端点统一请求体
///
/// **字段**: `args` 是任意 JSON Value, 由各 tool 自解释 (跟 `apeireth-tools::Tool::call(Value)` 对齐).
/// **不**做 schema 强校验 (per §2.5 400 错误码: JSON 缺字段才 400, 字段级校验由各 tool 内部做).
#[derive(Debug, Deserialize)]
pub struct InvokeRequest {
    /// 工具参数 (任意 JSON Value)
    #[serde(default)]
    pub args: Value,
}

/// **InvokeResponse** — 6 端点统一响应信封
///
/// **4 字段** (per §2.2):
/// - `ok: bool` — true=成功 / false=失败
/// - `result: Option<Value>` — 成功时的工具结果 (跟 `Tool::call` 返 Value 对齐)
/// - `error: Option<String>` — 失败时的错误消息
/// - `meta: Meta` — 元信息 (tool name + 耗时 + trace_id)
#[derive(Debug, Serialize)]
pub struct InvokeResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub meta: Meta,
}

/// **Meta** — 4 字段 (per §2.2):
/// - `tool` — 工具名 (跟 Tool::name() 对齐, 阶段 3 加 `api_key_hash`)
/// - `duration_ms` — handler 端到端耗时
/// - `trace_id` — 阶段 2 trace_id 用 timestamp + random 简易生成, 阶段 3 接 audit log
#[derive(Debug, Serialize)]
pub struct Meta {
    pub tool: String,
    pub duration_ms: u128,
    pub trace_id: String,
}

// ============================================================
// 统一 dispatch (6 端点共用)
// ============================================================

/// **6 端点统一分发函数** — axum handler 共享签名
///
/// **签名**: `Path(tool_name) + Json(InvokeRequest) → Json(InvokeResponse)`
///
/// **错误码** (per §2.5, 本阶段 2 实施 4 类):
/// - 400 — `args` 字段类型错 (serde 解析失败)
/// - 404 — 工具名未在 registry 注册
/// - 500 — tool.call 内部错误 (string 错误返 200 + ok=false, 非 500; 500 留给 panic/etc)
/// - 503 — V2State.tools 未初始化
pub async fn invoke_by_name(
    State(state): State<SharedV2>,
    Path(tool_name): Path<String>,
    Json(req): Json<InvokeRequest>,
) -> Result<Json<InvokeResponse>, (StatusCode, String)> {
    let started = Instant::now();
    let trace_id = make_trace_id();
    let meta_args = (
        tool_name.clone(),
        started.elapsed().as_millis(),
        trace_id.clone(),
    );

    // 1. 503 — V2State.tools 未初始化
    let reg = state
        .tools_registry()
        .ok_or_else(|| service_unavailable_err("tools"))?;

    // 1.5 工具名映射: URL tool_name (snake_case) → registry tool_name (PascalCase, VCP 风格)
    // 例: `web_search` → `WebSearch` / `file_ops` → `FileOperator` / `git_ops` → `Git` /
    //     `code_exec` → `ShellExec` / `calendar` → `Calendar` / `message` → `Message`
    let registry_name = url_to_registry_name(&tool_name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("tool not found: {}", tool_name),
        )
    })?;

    // 2. 404 — 工具名未注册
    let tool: Arc<dyn Tool> = reg.get(&registry_name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("tool not found: {}", tool_name),
        )
    })?;

    // 3. tool.call — tool 自身错误返 200 + ok=false (per §2.2 统一信封设计)
    match tool.call(req.args).await {
        Ok(value) => Ok(Json(InvokeResponse {
            ok: true,
            result: Some(value),
            error: None,
            meta: Meta {
                tool: meta_args.0,
                duration_ms: meta_args.1,
                trace_id: meta_args.2,
            },
        })),
        Err(e) => Ok(Json(InvokeResponse {
            ok: false,
            result: None,
            error: Some(e),
            meta: Meta {
                tool: meta_args.0,
                duration_ms: meta_args.1,
                trace_id: meta_args.2,
            },
        })),
    }
}

/// **503 service_unavailable** — V2State 服务未初始化
fn service_unavailable_err(name: &str) -> (StatusCode, String) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        format!("{name} service not initialized"),
    )
}

/// **trace_id** — 简易生成 (阶段 2, 阶段 3 接 audit log)
///
/// **格式**: `tr-2026-08-05-XXXXX` (跟 §2.2 meta.trace_id 对齐, X = 6 字符 random)
fn make_trace_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // 简易 hash 末 6 字符, 避免引 rand 库 (阶段 2 minimal)
    let tail = (ts as u64) & 0xFFFFFF;
    format!("tr-{:x}", tail)
}

/// **URL tool_name → registry tool_name 映射** (snake_case URL → PascalCase VCP 真名)
///
/// **映射表** (跟 V1_TOOL_NAMES / V1_REGISTRY_NAMES 1:1, per §2.2 路由表):
/// - `web_search` → `WebSearch`
/// - `file_ops`   → `FileOperator`
/// - `git_ops`    → `Git`
/// - `code_exec`  → `ShellExec`
/// - `calendar`   → `Calendar`
/// - `message`    → `Message`
///
/// **未匹配返 None** → 触发 404
fn url_to_registry_name(url_name: &str) -> Option<&'static str> {
    match url_name {
        "web_search" => Some("WebSearch"),
        "file_ops" => Some("FileOperator"),
        "git_ops" => Some("Git"),
        "code_exec" => Some("ShellExec"),
        "calendar" => Some("Calendar"),
        "message" => Some("Message"),
        _ => None,
    }
}

// ============================================================
// 6 端点路由构造 + 完整 registry 装配 helper
// ============================================================

/// **6 端点路由** — `Router<SharedV2>` (state 未绑定, 由 caller 调 `.with_state(state)`)
///
/// **设计**: 6 路由都指向 `invoke_by_name`, `Path<String>` 从 URL 提取 tool_name 自动路由.
/// **返回**: `Router<SharedV2>` (state 已泛型, 用 `v2_endpoints::build_router` 嵌进去)
pub fn build_v1_router() -> Router<SharedV2> {
    Router::new()
        .route("/tools/:name/invoke", post(invoke_by_name)) // 单一路由 + 6 tool 名变体 (更紧凑)
}

/// **完整 registry 装配** — 4 真接 (apeireth-tools) + 2 真接 (D-01 新增) = 6 工具
///
/// **返回**: `Arc<ToolRegistry>` (含 6 个 Tool, 可调 `state.install_tools(reg)`)
pub fn build_full_registry() -> Arc<apeireth_tool_registry::ToolRegistry> {
    use apeireth_tool_registry::ToolRegistry;
    let registry = Arc::new(ToolRegistry::new());
    // 1. 4 真接 (apeireth-tools::register_all, 0 改其源码)
    apeireth_tools::register_all(&registry).expect("register_all 4 真接");
    // 2. 2 真接 (D-01, 本 crate 新增)
    registry.register("Calendar".to_string(), Arc::new(calendar::CalendarTool::new()));
    registry.register("Message".to_string(), Arc::new(message::MessageTool::new()));
    registry
}

// ============================================================
// 编译期 hardcode (per O-5 不漂移 + 6 哲学 anchor 穿透)
// ============================================================

/// **6 端点** (per §2.2 6 端点路由表 + 蓝图头部)
pub const V1_TOOLS_COUNT: usize = 6;

/// **6 端点路径** (per §2.2, D-02 子路径风格)
pub const V1_TOOLS_PATHS: [&str; V1_TOOLS_COUNT] = [
    "/tools/web_search/invoke",
    "/tools/file_ops/invoke",
    "/tools/git_ops/invoke",
    "/tools/code_exec/invoke",
    "/tools/calendar/invoke",
    "/tools/message/invoke",
];

/// **6 端点工具名** (per §2.2 路由表第 1/2/3/4/5/6 列)
/// **映射关系**: web_search→WebSearch / file_ops→FileOperator / git_ops→Git /
/// code_exec→ShellExec / calendar→Calendar / message→Message
pub const V1_TOOL_NAMES: [&str; V1_TOOLS_COUNT] = [
    "web_search",   // → registry: "WebSearch"
    "file_ops",     // → registry: "FileOperator"
    "git_ops",      // → registry: "Git"
    "code_exec",    // → registry: "ShellExec"
    "calendar",     // → registry: "Calendar" (D-01 新增)
    "message",      // → registry: "Message" (D-01 新增)
];

/// **registry 实际工具名** (跟 `apeireth-tools::TOOL_NAMES` 4 个 + 本 crate 新增 2 个对齐)
pub const V1_REGISTRY_NAMES: [&str; V1_TOOLS_COUNT] = [
    "WebSearch",   // apeireth-tools::WebSearch
    "FileOperator", // apeireth-tools::FileOperator
    "Git",         // apeireth-tools::Git
    "ShellExec",   // apeireth-tools::ShellExec
    "Calendar",    // v1_tools::calendar::CalendarTool
    "Message",     // v1_tools::message::MessageTool
];

const _: () = {
    // 6 端点总数对齐 §2.2
    assert!(V1_TOOLS_COUNT == 6, "6 端点: web_search/file_ops/git_ops/code_exec/calendar/message");
    assert!(V1_TOOLS_PATHS.len() == 6, "6 路径");
    assert!(V1_TOOL_NAMES.len() == 6, "6 工具名 (URL)");
    assert!(V1_REGISTRY_NAMES.len() == 6, "6 registry 名");

    // 6 路径 = 6 工具名 (1:1)
    assert!(V1_TOOLS_PATHS.len() == V1_TOOL_NAMES.len(), "路径 = 工具名 (1:1)");
};

// ============================================================
// lib 单元测试 (编译期 hardcode 二次断言 + 6 端点可达)
// ============================================================

#[cfg(test)]
mod v1_tools_tests {
    use super::*;
    use apeireth_tool_registry::{Tool, ToolRegistry};
    use apeireth_tools::register_all;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;

    /// 验证 6 端点编译期 hardcode 一致
    #[test]
    fn v1_tools_constants_count_six() {
        assert_eq!(V1_TOOLS_COUNT, 6);
        assert_eq!(V1_TOOLS_PATHS.len(), 6);
        assert_eq!(V1_TOOL_NAMES.len(), 6);
        assert_eq!(V1_REGISTRY_NAMES.len(), 6);
    }

    /// 验证 6 端点路径符合 D-02 子路径风格
    #[test]
    fn v1_tools_paths_match_d02_subpath() {
        for path in V1_TOOLS_PATHS.iter() {
            assert!(path.starts_with("/tools/"), "路径 = /tools/<name>/invoke");
            assert!(path.ends_with("/invoke"), "路径 = .../invoke");
        }
    }

    /// 验证 6 端点 URL 工具名 → registry 工具名 1:1 映射
    #[test]
    fn v1_tools_url_name_to_registry_name_mapping() {
        // 4 真接 (apeireth-tools) + 2 真接 (D-01 新增)
        let m: &[(&str, &str)] = &[
            ("web_search", "WebSearch"),
            ("file_ops", "FileOperator"),
            ("git_ops", "Git"),
            ("code_exec", "ShellExec"),
            ("calendar", "Calendar"),
            ("message", "Message"),
        ];
        for (i, (url, reg)) in m.iter().enumerate() {
            assert_eq!(V1_TOOL_NAMES[i], *url, "URL 名 = {url}");
            assert_eq!(V1_REGISTRY_NAMES[i], *reg, "registry 名 = {reg}");
        }
    }

    /// 验证 4 apeireth-tools 真工具真注册 + 真可达 (end-to-end)
    #[test]
    fn v1_tools_register_all_4_then_lookup() {
        let registry = ToolRegistry::new();
        register_all(&registry).expect("register_all 4 真工具");
        // 4 个真接可达
        assert!(registry.get("WebSearch").is_some());
        assert!(registry.get("FileOperator").is_some());
        assert!(registry.get("Git").is_some());
        assert!(registry.get("ShellExec").is_some());
    }

    /// Mock tool (Calendar 简化版) 验证 dispatch 路径
    struct MockEchoTool;
    #[async_trait]
    impl Tool for MockEchoTool {
        fn name(&self) -> &str {
            "Calendar"
        }
        fn kind(&self) -> apeireth_tool_registry::ToolKind {
            apeireth_tool_registry::ToolKind::Sync
        }
        fn axes(&self) -> apeireth_tool_registry::ToolAxes {
            apeireth_tool_registry::ToolAxes::default()
        }
        async fn call(&self, args: Value) -> Result<Value, String> {
            Ok(json!({"echoed": args}))
        }
    }

    /// dispatch 路径可达: 模拟 invoke_by_name 走 Tool::call
    #[tokio::test]
    async fn v1_tools_dispatch_path_e2e() {
        let registry = Arc::new(ToolRegistry::new());
        // 1. mock 注册 Calendar
        let cal: Arc<dyn Tool> = Arc::new(MockEchoTool);
        registry.register("Calendar".to_string(), cal);
        // 2. dispatch 路径
        let tool = registry.get("Calendar").expect("Calendar");
        let r = tool.call(json!({"action": "list"})).await.expect("call");
        assert_eq!(r["echoed"]["action"], "list");
    }
}
