#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 5 nav × Tools 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围**:
/// - 6 工具 entry (calendar / message / contact / task / search / drive)
/// - 6 工具中文 label 互不相同
/// - endpoint path 格式
/// - 5 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: Tools 屏服务 ASI 北极星 (工具能力 → 行动)
/// - S-2 实事求是: 6 工具对齐 apeireth-api 真端点, 不自己造
/// - O-2 走在前人肩上: 借 apeireth-api `/v1/tools/{name}/invoke`
/// - O-3 干到底: 6 工具全列, 不只画 UI
/// - O-4 任何人都能接手: 工具名 + 中文 label 清楚
/// - O-5 不假装: 真实 invoke 待 R25.3
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"]
mod config_watcher;
#[path = "../src/http_llm.rs"]
mod http_llm;
#[path = "../src/llm_config.rs"]
mod llm_config;
#[path = "../src/observability.rs"]
mod observability;
#[path = "../src/onboarding.rs"]
mod onboarding;
#[path = "../src/organ/mod.rs"]
mod organ;
#[path = "../src/pages/mod.rs"]
mod pages;
#[path = "../src/persistence.rs"]
mod persistence;
#[path = "../src/theme.rs"]
mod theme;

#[path = "../src/error.rs"]
mod error;
#[path = "../src/http.rs"]
mod http;
#[path = "../src/nav/mod.rs"]
mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
/// **8 项承诺**: 全部遵守
mod test_common;

use ratatui::layout::Rect;
use test_common::TOOL_WHITELIST;

// =====================================================================
// 1. 6 工具硬白名单 (跟 error::TOOL_WHITELIST 一致)
// =====================================================================

#[test]
fn six_tools_match_error_whitelist() {
    assert_eq!(nav::tools::SIX_TOOLS.len(), 6);
    assert_eq!(
        nav::tools::SIX_TOOLS,
        error::TOOL_WHITELIST,
        "nav/tools SIX_TOOLS 必须跟 error TOOL_WHITELIST 完全一致, 防漂移"
    );
    // 也跟 test_common::TOOL_WHITELIST 一致
    assert_eq!(nav::tools::SIX_TOOLS, TOOL_WHITELIST);
}

// =====================================================================
// 2. 6 工具中文 label 互不相同
// =====================================================================

#[test]
fn tool_label_zh_6_distinct() {
    let labels: Vec<&str> = nav::tools::SIX_TOOLS
        .iter()
        .map(|t| nav::tools::tool_label_zh(t))
        .collect();
    let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
    assert_eq!(unique.len(), 6, "6 工具中文 label 互不相同");
    // 兜底: 未知工具返 "?"
    assert_eq!(nav::tools::tool_label_zh("unknown"), "?");
}

// =====================================================================
// 3. endpoint path 格式
// =====================================================================

#[test]
fn endpoint_path_format() {
    assert_eq!(
        nav::tools::endpoint_path("calendar"),
        "/v1/tools/calendar/invoke"
    );
    assert_eq!(
        nav::tools::endpoint_path("message"),
        "/v1/tools/message/invoke"
    );
    assert_eq!(
        nav::tools::endpoint_path("contact"),
        "/v1/tools/contact/invoke"
    );
    assert_eq!(nav::tools::endpoint_path("task"), "/v1/tools/task/invoke");
    assert_eq!(
        nav::tools::endpoint_path("search"),
        "/v1/tools/search/invoke"
    );
    assert_eq!(nav::tools::endpoint_path("drive"), "/v1/tools/drive/invoke");
}

// =====================================================================
// 4. render 列出 6 工具 + 标 [partial]
// =====================================================================

#[test]
fn render_lists_6_tools_and_marks_partial() {
    let area = Rect::new(0, 0, 80, 24);
    let out = nav::tools::render(area);
    for tool in nav::tools::SIX_TOOLS {
        assert!(out.contains(tool), "render 应含工具 {tool}");
    }
    assert!(
        out.contains("[partial]") || out.contains("partial"),
        "Tools render 应标 partial, 不假装接 HTTP: {out}"
    );
    assert!(out.contains("6 工具"), "render 应说明 6 工具");
}

// =====================================================================
// 5. invoke (R25.2 partial, 占位 OK)
// =====================================================================

#[tokio::test]
async fn invoke_partial_returns_placeholder() {
    let c = http::ApeirethClient::default_no_auth().expect("client");
    let r = nav::tools::invoke(&c, "calendar").await;
    // R25.2 partial: 返占位 OK, 不真接
    assert!(r.is_ok(), "R25.2 invoke 标 partial 应返 Ok, 实 {r:?}");
    let s = r.unwrap();
    assert!(
        s.contains("R25.2 partial") || s.contains("partial"),
        "占位响应应说明 partial: {s}"
    );
}
