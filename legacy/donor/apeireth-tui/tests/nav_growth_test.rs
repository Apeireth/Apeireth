#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// Nav × Growth (生长) 渲染快照测试 (R26 升级)
///
/// **测试范围**:
/// - 渲染 Growth 页 4 卡, 验证文本里含 4 阶段工程用语
/// - 砍掉的阶段 (Birth/Reproduction/Migration/Rebirth) 不应出现
/// - 反思环 R26 真接 backend
///
/// **R11 LOCKED 边界**: 0 触, 仅 TUI 层 nav 渲染测试.

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

use app::App;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use theme::{Theme, ThemeStyle};

fn render_growth_to_string() -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = App::new();
    let style = ThemeStyle::of(Theme::Archaic);
    terminal
        .draw(|f| pages::growth::render(f, Rect::new(0, 0, 120, 40), &app, &style))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        out.push('\n');
    }
    out
}

#[test]
fn growth_page_contains_4_stage_engineering_terms() {
    let out = render_growth_to_string();
    assert!(out.contains("Init"), "Growth 页应含 Init 阶段");
    assert!(out.contains("Bootstrap"), "Growth 页应含 Bootstrap 阶段");
    assert!(out.contains("Serving"), "Growth 页应含 Serving 阶段");
    assert!(out.contains("Saturated"), "Growth 页应含 Saturated 阶段");
}

#[test]
fn growth_page_does_not_show_culled_stages() {
    let out = render_growth_to_string();
    // R26 砍掉的阶段不应出现在 UI
    assert!(!out.contains("Birth"), "Growth 页不应含 Birth");
    assert!(
        !out.contains("Reproduction"),
        "Growth 页不应含 Reproduction"
    );
    assert!(!out.contains("Migration"), "Growth 页不应含 Migration");
    assert!(!out.contains("Rebirth"), "Growth 页不应含 Rebirth");
    assert!(!out.contains("Decline"), "Growth 页不应含 Decline");
    assert!(!out.contains("Death"), "Growth 页不应含 Death");
}

#[test]
fn growth_page_shows_reflection_ring_status() {
    let out = render_growth_to_string();
    // R26 真接 backend compute_reflection_progress
    // ring rendering 含 ◔/◐/◑/◓/● 字符 (compute_reflection_progress 取字符)
    let has_ring = out.contains('◔')
        || out.contains('◐')
        || out.contains('◑')
        || out.contains('◓')
        || out.contains('●')
        || out.contains('○');
    let has_hint = out.contains("无反思") || out.contains("反思充分") || out.contains("进行中");
    assert!(
        has_ring || has_hint,
        "Growth 页应含反思环 (R26 真接): {out}"
    );
}

#[test]
fn growth_page_renders_4_cards_not_8() {
    let out = render_growth_to_string();
    let stage_count = ["Init", "Bootstrap", "Serving", "Saturated"]
        .iter()
        .filter(|n| out.contains(*n))
        .count();
    assert_eq!(
        stage_count, 4,
        "Growth 页应渲染 4 卡 (R26 砍 8): {stage_count}"
    );
}
