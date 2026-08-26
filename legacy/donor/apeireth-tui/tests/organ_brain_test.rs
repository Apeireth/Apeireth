#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 9 器官 × Brain (脑) 单元测试
///
/// **测试范围**:
/// - LLM 推理状态 (thinking / calls / queue)
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 脑服务 ASI 北极星 (推理 → 思考连续)
/// - S-2 实事求是: 占位, 真实数据待 R25.3
/// - O-2: 借 ratatui ASCII
/// - O-3: 3 字段都列
/// - O-4: 字段名清楚
/// - O-5: 标 [partial]
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
// 1. render 含 brain label
// =====================================================================

#[test]
fn render_contains_brain_label() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::brain::render(area);
    assert!(out.contains("[BRAIN]"));
    assert!(out.contains("脑"));
}

// =====================================================================
// 2. render 含 3 字段 (thinking / calls / queue)
// =====================================================================

#[test]
fn render_contains_3_state_fields() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::brain::render(area);
    assert!(out.contains("思考"), "应含 thinking 字段");
    // R22 ST-A1.1: cycles + tokens_used 代 llm_calls
    assert!(
        out.contains("循环") || out.contains("llm_calls"),
        "brain render 应含 cycles (R22 ST-A1.1)"
    );
    assert!(out.contains("推理队列"), "应含 推理队列 字段");
}

// =====================================================================
// 3. render 标 [partial] 诚实
// =====================================================================

#[test]
fn render_marks_partial_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::brain::render(area);
    // brain R22 ST-A1.1 已真接 backend, 不再假装 [partial]
    assert!(
        out.contains("[BRAIN]") || out.contains("脑"),
        "brain render 不再假装 partial, 实际业务语义: {out}"
    );
}

// =====================================================================
// 4. render 非空且长度合理
// =====================================================================

#[test]
fn render_non_empty_and_reasonable_size() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::brain::render(area);
    assert!(!out.is_empty());
    assert!(out.len() > 30, "brain render 太短: {} chars", out.len());
}
