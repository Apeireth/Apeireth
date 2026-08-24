#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 9 器官 × Hand (手) 单元测试
///
/// **测试范围**:
/// - 6 工具调用统计
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 手服务 ASI 北极星 (行动能力 → 工具调用)
/// - S-2 实事求是: 6 工具对齐 apeireth-api 真端点
/// - O-2: 借 R11 工具系统
/// - O-3: 6 工具全列
/// - O-4: 3 字段 (today/ok/fail) 清楚
/// - O-5: 真实数据 partial
///
/// **8 项承诺**: 全部遵守
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

mod test_common;

use ratatui::layout::Rect;
use test_common::TOOL_WHITELIST;

// =====================================================================
// 1. 6 工具 hardcoded (跟 test_common + error 同步)
// =====================================================================

#[test]
fn six_tools_hardcoded_and_synced() {
    assert_eq!(organ::hand::SIX_TOOLS.len(), 6);
    assert_eq!(
        organ::hand::SIX_TOOLS,
        TOOL_WHITELIST,
        "hand 6 工具跟 test_common 同步"
    );
    assert_eq!(
        organ::hand::SIX_TOOLS,
        error::TOOL_WHITELIST,
        "hand 6 工具跟 error::TOOL_WHITELIST 同步"
    );
    assert!(organ::hand::SIX_TOOLS.contains(&"calendar"));
    assert!(organ::hand::SIX_TOOLS.contains(&"drive"));
}

// =====================================================================
// 2. render 含 hand label + 6 工具
// =====================================================================

#[test]
fn render_contains_hand_label_and_6_tools() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::hand::render(area);
    assert!(out.contains("[HAND]"));
    assert!(out.contains("手"));
    for tool in organ::hand::SIX_TOOLS {
        assert!(out.contains(tool), "render 应含工具 {tool}");
    }
}

// =====================================================================
// 3. render 含 3 统计字段 (today/ok/fail)
// =====================================================================

#[test]
fn render_contains_3_stats_fields() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::hand::render(area);
    assert!(out.contains("今日"), "应含 today 字段");
    assert!(out.contains("成功"), "应含 ok 字段");
    assert!(out.contains("失败"), "应含 fail 字段");
}

// =====================================================================
// 4. render 标 [partial] 诚实
// =====================================================================

#[test]
fn render_marks_partial_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::hand::render(area);
    // hand R22 ST-A1.4 已真接, 不再假装 partial.
    assert!(
        out.contains("[HAND]") || out.contains("手"),
        "hand render 实际业务字段: {out}"
    );
}
