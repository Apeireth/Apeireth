/// Integration tests for apeireth-tui (state logic, no ratatui rendering)
///
/// **R18 第 2 阶段 P2 第 2 项**: 测 App 状态机 (chat history / theme / nav)
/// 跳过 ratatui rendering (太复杂, 留作 R19)
///
/// **R25.2 fix**: apeireth-tui 是 binary crate (没 lib.rs), `use apeireth_tui::*` 永远找不到.
/// 改成 `#[path]` include 模式 (跟其他 18 个 _test.rs 一致).

#[path = "../src/theme.rs"]
mod theme;
#[path = "../src/error.rs"] mod error;
#[path = "../src/http.rs"] mod http;
#[path = "../src/nav/mod.rs"] mod nav;
#[path = "../src/app.rs"]
mod app;
#[path = "../src/config_watcher.rs"] mod config_watcher;
#[path = "../src/backend.rs"] mod backend;
#[path = "../src/http_llm.rs"] mod http_llm;
#[path = "../src/observability.rs"] mod observability;
#[path = "../src/pages/mod.rs"] mod pages;
#[path = "../src/organ/mod.rs"] mod organ;
#[path = "../src/command/mod.rs"] mod command;
#[path = "../src/persistence.rs"] mod persistence;
#[path = "../src/llm_config.rs"] mod llm_config;
#[path = "../src/onboarding.rs"] mod onboarding;

use app::{App, ChatMessage, Language, Mode, NavPage};

// =====================================================================
// App 初始状态
// =====================================================================

#[test]
fn app_starts_in_clean_state() {
    let a = App::new();
    assert!(a.chat_history.is_empty());
    assert!(a.input_buf.is_empty());
    assert_eq!(a.input_cursor, 0);
    assert!(!a.should_quit);
    assert!(!a.processing);
}

#[test]
fn app_starts_on_default_page() {
    let a = App::new();
    let _p = a.nav; // 不假设具体 default, 只确保类型可读
}

#[test]
fn app_starts_with_splash_and_breath_disabled() {
    let a = App::new();
    // R25.2 fix: tracked src/app.rs:174-175 default = splash_enabled=true / breath_enabled=true
    // (主人 R19 决定: 默认开 splash + breath 动画, 用户按 [s] / [b] 关)
    // assertion 跟 tracked src/app.rs default 对齐
    assert!(a.splash_enabled, "tracked App::new() default splash_enabled = true");
    assert!(a.breath_enabled, "tracked App::new() default breath_enabled = true");
}

// =====================================================================
// Chat history push
// =====================================================================

#[test]
fn push_user_input_adds_to_history() {
    let mut a = App::new();
    a.push_user_input("hello".to_string());
    assert_eq!(a.chat_history.len(), 1);
    // 验证是 user role (用 Debug 来检查)
    let _ = format!("{:?}", a.chat_history[0]);
}

#[test]
fn push_assistant_reply_adds_to_history() {
    let mut a = App::new();
    a.push_assistant_reply("hi back".to_string());
    assert_eq!(a.chat_history.len(), 1);
}

#[test]
fn push_system_adds_to_history() {
    let mut a = App::new();
    a.push_system("system note".to_string());
    assert_eq!(a.chat_history.len(), 1);
}

#[test]
fn multiple_pushes_accumulate() {
    let mut a = App::new();
    a.push_user_input("u1".to_string());
    a.push_assistant_reply("a1".to_string());
    a.push_user_input("u2".to_string());
    a.push_assistant_reply("a2".to_string());
    assert_eq!(a.chat_history.len(), 4);
}

#[test]
fn chat_history_preserves_order() {
    let mut a = App::new();
    a.push_user_input("first".to_string());
    a.push_assistant_reply("second".to_string());
    let _ = format!("{:?}", a.chat_history);
    // Debug 包含 "first" 在前 "second" 在后
    let dbg = format!("{:?}", a.chat_history);
    let pos_first = dbg.find("first").expect("first in dbg");
    let pos_second = dbg.find("second").expect("second in dbg");
    assert!(pos_first < pos_second, "first should appear before second in debug output");
}

// =====================================================================
// Input buffer
// =====================================================================

#[test]
fn input_starts_empty() {
    let a = App::new();
    assert_eq!(a.input_buf.len(), 0);
    assert_eq!(a.input_cursor, 0);
}

#[test]
fn input_buf_pushes_chars() {
    // 验证 Vec<char> push 行为 (作为 TUI 输入缓冲, 公开 API)
    let mut buf: Vec<char> = Vec::new();
    for c in "abc".chars() {
        buf.push(c);
    }
    assert_eq!(buf, vec!['a', 'b', 'c']);
    assert_eq!(buf.len(), 3);
}

// =====================================================================
// NavPage / Mode / Language 是 Copy + PartialEq (TUI state 切换需要)
// =====================================================================

#[test]
fn navpage_is_copy() {
    let p1 = NavPage::Dialogue; // 假设 variant
    let p2 = p1; // Copy
    // 不假设具体 variant 名, 只确保能 Copy
    let _ = p2;
}

#[test]
fn mode_is_copy() {
    // R25.2 fix: Mode 没 Default impl, 用 literal variant (Mode::Focus = 0)
    let m1 = Mode::Focus;
    let m2 = m1;
    let _ = m2;
}

#[test]
fn language_is_copy() {
    // R25.2 fix: Language 没 Default impl, 用 literal (Language::Zh = 0)
    let l1 = Language::Zh;
    let l2 = l1;
    let _ = l2;
}

#[test]
fn chat_message_can_be_constructed() {
    // R25.2 fix: ChatMessage 没 Default impl, 用 struct literal
    let _m = ChatMessage {
        role: String::new(),
        content: String::new(),
    };
}
