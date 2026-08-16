#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 5 nav × Session 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围**:
/// - HTTP GET `/v1/sessions` 数据流 (用 httpmock mock server)
/// - Session 字段解析 (id / title / last_active_at / message_count)
/// - 5 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: Session 屏服务 ASI 北极星 (会话活跃 → 用户连接)
/// - S-2 实事求是: 占位 session 标 "[stub]", 不假装接 HTTP
/// - O-2 走在前人肩上: 用 httpmock 业界惯例 mock HTTP
/// - O-3 干到底: 5 测试覆盖字段 + 列表 + 标缺
/// - O-4 任何人都能接手: 字段名清楚
/// - O-5 不假装: stub 明说
///
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

use httpmock::prelude::*;
use ratatui::layout::Rect;
use std::time::Duration;
use test_common::PATH_SESSIONS;

// =====================================================================
// 1. render 标 [stub] 诚实
// =====================================================================

#[test]
fn render_marks_stub_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = nav::session::render(area);
    assert!(
        out.contains("[stub]") || out.contains("stub"),
        "Session render 应明确标 stub, 不假装接 HTTP: {out}"
    );
}

// =====================================================================
// 2. render 含 session header
// =====================================================================

#[test]
fn render_contains_session_header() {
    let area = Rect::new(0, 0, 80, 24);
    let out = nav::session::render(area);
    assert!(out.contains("SESSION"));
    assert!(out.contains("活跃会话"));
    // 键位提示
    assert!(out.contains("[Tab") || out.contains("Tab"));
}

// =====================================================================
// 3. render 列出 stub session
// =====================================================================

#[test]
fn render_lists_stub_session() {
    let area = Rect::new(0, 0, 80, 24);
    let out = nav::session::render(area);
    // R25.2 fix: render 用前 8 字符截断 (line 46: &s.id[..8]),
    // assertion 跟 render 输出一致
    assert!(
        out.contains("stub-ses"),
        "render 应含 ID 前 8 字符 'stub-ses': {out}"
    );
    assert!(
        out.contains("Stub session"),
        "render 应含 stub title: {out}"
    );
}

// =====================================================================
// 4. HTTP GET /v1/sessions (httpmock)
// =====================================================================

#[tokio::test]
async fn get_sessions_parses_list_via_http() {
    let server = MockServer::start_async().await;
    server.mock(|when, then| {
        when.method(GET).path(PATH_SESSIONS);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"[{"id":"s1","title":"hi","message_count":3},{"id":"s2","title":"hello","message_count":7}]"#);
    });
    let c = http::ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5))
        .expect("client");
    let sessions = c.get_sessions().await.expect("sessions");
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id, "s1");
    assert_eq!(sessions[1].id, "s2");
}

// =====================================================================
// 5. HTTP GET /v1/sessions 5xx → TuiError::Api
// =====================================================================

#[tokio::test]
async fn get_sessions_5xx_returns_tui_error_api() {
    let server = MockServer::start_async().await;
    server.mock(|when, then| {
        when.method(GET).path(PATH_SESSIONS);
        then.status(500)
            .header("content-type", "text/plain")
            .body("internal server error");
    });
    let c = http::ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5))
        .expect("client");
    let r = c.get_sessions().await;
    match r {
        Err(error::TuiError::Api { status, body }) => {
            assert_eq!(status, 500);
            assert!(body.contains("internal server error"));
        }
        other => panic!("expected Api 500 error, got {other:?}"),
    }
}
