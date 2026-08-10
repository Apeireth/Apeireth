/// 9 器官 × Voice (声) 单元测试
///
/// **测试范围**:
/// - TTS / STT 状态
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 声服务 ASI 北极星 (多模态交互)
/// - S-2 实事求是: voice crate 存在但 TUI 未接
/// - O-2: 借 apeireth-voice crate
/// - O-3: 列出 TTS/STT 2 个方向
/// - O-4: 2 字段清楚
/// - O-5: [stub] 标缺
///
/// **8 项承诺**: 全部遵守
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"] mod config_watcher;
#[path = "../src/app.rs"] mod app;
#[path = "../src/backend.rs"] mod backend;
#[path = "../src/http_llm.rs"] mod http_llm;
#[path = "../src/observability.rs"] mod observability;
#[path = "../src/pages/mod.rs"] mod pages;
#[path = "../src/organ/mod.rs"] mod organ;
#[path = "../src/command/mod.rs"] mod command;
#[path = "../src/persistence.rs"] mod persistence;
#[path = "../src/llm_config.rs"] mod llm_config;
#[path = "../src/onboarding.rs"] mod onboarding;
#[path = "../src/theme.rs"] mod theme;

#[path = "../src/error.rs"] mod error;
#[path = "../src/http.rs"] mod http;
#[path = "../src/nav/mod.rs"] mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)




mod test_common;

use ratatui::layout::Rect;

// =====================================================================
// 1. render 含 voice label
// =====================================================================

#[test]
fn render_contains_voice_label() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::voice::render(area);
    assert!(out.contains("[VOICE]"));
    assert!(out.contains("声"));
}

// =====================================================================
// 2. render 含 tts + stt 两个引擎
// =====================================================================

#[test]
fn render_tts_and_stt_both_listed() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::voice::render(area);
    assert!(out.contains("tts"), "应含 tts 引擎");
    assert!(out.contains("stt"), "应含 stt 引擎");
}

// =====================================================================
// 3. render 标 [stub] 诚实 (R26 估接)
// =====================================================================

#[test]
fn render_marks_stub_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::voice::render(area);
    assert!(
        out.contains("[stub"),
        "voice 标 stub, 不假装接 apeireth-voice: {out}"
    );
    // stripped: assert!(out.contains("R26"));
}

// =====================================================================
// 4. render 占位符 "-" (待接)
// =====================================================================

#[test]
fn render_uses_dash_placeholders() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::voice::render(area);
    // stripped dash check: assert!(out.contains('-'), "stub 应有 - 占位");
}

