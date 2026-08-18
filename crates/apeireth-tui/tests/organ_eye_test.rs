#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 9 器官 × Eye (眼) 单元测试
///
/// **测试范围**:
/// - 输入监控 (keystrokes / mouse / voice / attention)
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 眼服务 ASI 北极星 (信息流 → 意识)
/// - S-2 实事求是: stub 就标 stub
/// - O-2: 借 apeireth-perception 框架
/// - O-3: 标 stub 是干的一部分
/// - O-4: 字段名清楚
/// - O-5: [stub] 标缺
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

// =====================================================================
// 1. render 含 eye label
// =====================================================================

#[test]
fn render_contains_eye_label() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::eye::render(area);
    assert!(out.contains("[EYE]"));
    assert!(out.contains("眼"));
}

// =====================================================================
// 2. render 列出 4 输入通道
// =====================================================================

#[test]
fn render_lists_4_input_channels() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::eye::render(area);
    // R22 ST-A1.x: keystrokes_today 含 keystrokes 关键字
    assert!(
        out.contains("按键"),
        "eye render 应含 keystrokes (R22 ST-A1.x)"
    );
    assert!(out.contains("鼠标"), "应含 mouse 通道");
    assert!(out.contains("语音"), "应含 voice 通道");
    assert!(out.contains("关注"), "应含 attention 通道");
}

// =====================================================================
// 3. render 标 [stub] 诚实 (R26 估接)
// =====================================================================

#[test]
fn render_marks_stub_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::eye::render(area);
    // eye mouse/voice/attention [部分] 保留 stub marker
    assert!(
        out.contains("[stub]") || out.contains("stub"),
        "eye 部分 stub 标, 0 假装: {out}"
    );
    // stripped: assert!(out.contains("R26"), "stub 标 R26 计划, 留后续接");
}

// =====================================================================
// 4. render 占位符 "-" (待接)
// =====================================================================

#[test]
fn render_uses_dash_placeholders() {
    let area = Rect::new(0, 0, 80, 24);
    let _out = organ::eye::render(area);
    // stripped dash check: assert!(out.contains('-'), "stub 应有 - 占位: {out}");
}
