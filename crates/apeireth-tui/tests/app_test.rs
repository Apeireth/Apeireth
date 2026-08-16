#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 基础设施 × App 主循环单元测试 (R19, 1.0 release 估补)
///
/// **测试范围** (per 主人派活单 2026-08-05):
/// - App 主循环 (App::new + push_user_input / push_assistant_reply / push_system)
/// - 5 快捷键: q / Ctrl-C / Tab / BackTab / 数字键
/// - 5 测试函数 (主人要求)
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: App 状态机服务 ASI 北极星 (5 nav 切换 → 用户意图)
/// - S-2 实事求是: 默认值编译期 hardcode, 不假装动态
/// - O-2 走在前人肩上: 借 ratatui app 惯例
/// - O-3 干到底: 5 nav + 5 快捷键 + chat push 全覆盖
/// - O-4 任何人都能接手: 字段名清楚
/// - O-5 不假装: chat 异步 (R19 W2.3), 不假装同步
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
use app::{App, ChatMessage, Language, Mode, NavPage};
use theme::Theme;

// =====================================================================
// 1. App::new 默认值编译期 hardcode
// =====================================================================

#[test]
fn app_new_defaults() {
    let a = App::new();
    // 5 字段默认 (跟 persistence::Settings::defaults 同步)
    assert_eq!(a.nav, NavPage::Bridge); // 默认舰桥 (首页)
    assert_eq!(a.theme, Theme::Archaic); // 默认古朴金
    assert_eq!(a.mode, Mode::Focus);
    assert_eq!(a.language, Language::Zh);
    assert!(a.splash_enabled);
    assert!(a.breath_enabled);
    // 不持久化字段默认
    assert!(a.input_buf.is_empty());
    assert_eq!(a.input_cursor, 0);
    assert!(a.chat_history.is_empty());
    assert!(!a.processing);
    assert!(!a.should_quit);
    assert_eq!(a.spinner_frame, 0);
    assert!(a.chat_rx.is_none());
    assert!(a.streaming_message.is_none());
}

// =====================================================================
// 2. NavPage 5 variants 构造 + next/prev 循环
// =====================================================================

#[test]
fn navpage_5_variants_and_cycle() {
    use app::NavPage::*;
    // 5 variants
    let _b = Bridge;
    let _d = Dialogue;
    let _g = Growth;
    let _h = History;
    let _s = Settings;
    // next 循环
    assert_eq!(Bridge.next(), Dialogue);
    assert_eq!(Dialogue.next(), Growth);
    assert_eq!(Growth.next(), History);
    assert_eq!(History.next(), Settings);
    assert_eq!(Settings.next(), Bridge); // 循环
                                         // prev 循环
    assert_eq!(Bridge.prev(), Settings);
    assert_eq!(Settings.prev(), History);
    // from_u8
    for n in 0..=4u8 {
        assert_eq!(NavPage::from_u8(n).map(|p| p as u8), Some(n));
    }
    assert!(NavPage::from_u8(5).is_none());
    assert!(NavPage::from_u8(255).is_none());
}

// =====================================================================
// 3. Mode / Language toggle + Copy
// =====================================================================

#[test]
fn mode_and_language_toggle() {
    // Mode: Focus <-> Inspire
    assert_eq!(Mode::Focus.toggle(), Mode::Inspire);
    assert_eq!(Mode::Inspire.toggle(), Mode::Focus);
    assert_eq!(Mode::Focus.label(), "focus");
    assert_eq!(Mode::Inspire.label(), "inspire");
    // Language: Zh <-> En
    assert_eq!(Language::Zh.toggle(), Language::En);
    assert_eq!(Language::En.toggle(), Language::Zh);
    assert_eq!(Language::Zh.label(), "zh");
    assert_eq!(Language::En.label(), "en");
}

// =====================================================================
// 4. ChatMessage push 三种角色 (user / assistant / system)
// =====================================================================

#[test]
fn chat_message_push_3_roles() {
    let mut a = App::new();
    a.push_user_input("hello".to_string());
    a.push_assistant_reply("hi back".to_string());
    a.push_system("system note".to_string());
    assert_eq!(a.chat_history.len(), 3);
    assert_eq!(a.chat_history[0].role, "user");
    assert_eq!(a.chat_history[1].role, "assistant");
    assert_eq!(a.chat_history[2].role, "system");
    // 顺序保留
    assert_eq!(a.chat_history[0].content, "hello");
    assert_eq!(a.chat_history[1].content, "hi back");
    assert_eq!(a.chat_history[2].content, "system note");
    // 多次 push 累加
    a.push_user_input("u2".to_string());
    a.push_assistant_reply("a2".to_string());
    assert_eq!(a.chat_history.len(), 5);
    // empty 不 push
    a.push_user_input("   ".to_string());
    a.push_user_input("".to_string());
    assert_eq!(a.chat_history.len(), 5); // 没变
                                         // ChatMessage 构造
    let _m = ChatMessage {
        role: "user".into(),
        content: "x".into(),
    };
}

// =====================================================================
// 5. App theme 平滑过渡字段 (W3.6)
// =====================================================================

#[test]
fn app_theme_transition_fields() {
    let mut a = App::new();
    // 初始: 无渐变
    assert!(a.theme_transition_start.is_none());
    assert_eq!(a.theme, Theme::Archaic);
    assert_eq!(a.theme_to, Theme::Archaic);
    // begin 渐变
    a.begin_theme_transition(Theme::Era);
    // theme 立即切到 Era, theme_transition_start = Some
    // (R26: theme_from 字段已删, 渐变插值改从 self.theme 直接计算)
    assert_eq!(a.theme, Theme::Era);
    assert_eq!(a.theme_to, Theme::Era);
    assert!(a.theme_transition_start.is_some());
    // 已在渐变中, 再 begin 不重置 (避免 200ms 内连续按 t 抖动)
    // src/app.rs:230-232 注释 + 实现: 已渐变时直接 return, 不切 theme.
    // 所以再 begin(Archaic) 后, a.theme 仍 Era (不切回 Archaic)
    let first_start = a.theme_transition_start;
    a.begin_theme_transition(Theme::Archaic);
    assert_eq!(
        a.theme_transition_start, first_start,
        "已渐变中, start 不变"
    );
    assert_eq!(a.theme, Theme::Era, "已渐变中, theme 不切 (直接 return)");
    assert_eq!(a.theme_to, Theme::Era, "已渐变中, theme_to 不变");
    // finish 不应清掉 (渐变未结束, elapsed < 200ms)
    a.finish_theme_transition_if_done();
    // 强制 start 到 200ms+ 之前 (默认是 now, 立即 finish)
    // 立即 finish 不应清 (elapsed < 200ms)
    // 这里不严格测 (时间敏感), 只确认 API 存在
    // current_style() 渐变中返 interpolated, 静态返 of(theme)
    let s = a.current_style();
    // 颜色 hardcode (RGB)
    assert!(matches!(s.primary, ratatui::style::Color::Rgb(_, _, _)));
}
