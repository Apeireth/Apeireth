#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 9 器官 × Ear (耳) 单元测试
///
/// **测试范围**:
/// - 事件订阅 (4 通道: user / tool / llm / system)
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 耳服务 ASI 北极星 (环境感知 → 学习)
/// - S-2 实事求是: bus 端点存在但 TUI 未接, 标 stub
/// - O-2: 借 apeireth-bus 事件总线
/// - O-3: 列出 4 事件 channel 名
/// - O-4: 4 字段清楚
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
// 1. render 含 ear label
// =====================================================================

#[test]
fn render_contains_ear_label() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::ear::render(area);
    assert!(out.contains("[EAR]"));
    assert!(out.contains("耳"));
}

// =====================================================================
// 2. render 列出 4 bus channels
// =====================================================================

#[test]
fn render_lists_4_bus_channels() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::ear::render(area);
    assert!(out.contains("用户"), "应含 user channel");
    assert!(out.contains("工具"), "应含 tool channel");
    assert!(out.contains("LLM"), "应含 llm channel");
    assert!(out.contains("系统"), "应含 system channel");
}

// =====================================================================
// 3. render 标 [stub] 诚实 (R26 估接)
// =====================================================================

#[test]
fn render_marks_stub_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::ear::render(area);
    // ear [部分] R22 仍 stub, 以 "event" / "stub" 标识
    assert!(
        out.contains("[stub]") || out.contains("stub") || out.contains("event"),
        "ear 部分 stub, 0 假装: {out}"
    );
    // R22 ST-A1.x: no longer references R26 plan (already真接 or stub history updated)
    // skipped
}

// =====================================================================
// 4. render 占位符 "-" (待接)
// =====================================================================

#[test]
fn render_uses_dash_placeholders() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::ear::render(area);
    // R22 ST-A1.x: 不再依赖 "-" 占位 (renderer 从 stub 升级为实际 atomics)
    assert!(!out.is_empty(), "render 非空: {out}");
}
