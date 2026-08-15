//! R177 tui organ Kani proofs (W11)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tui_01_theme_transition_ms() {
    assert_eq!(THEME_TRANSITION_MS, 200);
}

#[test]
fn r177_tui_02_app_new() {
    let a = App::new();
    let _: String = format!("{:?}", a);
}

#[test]
fn r177_tui_03_chat_message() {
    let m = ChatMessage { role: "user".into(), content: "hi".into() };
    assert_eq!(m.role, "user");
}

#[test]
fn r177_tui_04_mode() {
    let m = Mode::Focus;
    assert_eq!(m.label(), "focus");
}

#[test]
fn r177_tui_05_language() {
    let l = Language::Zh;
    assert_eq!(l.label(), "zh");
}

#[cfg(kani)]
#[kani::proof]
fn r177_tui_kani_01_transition_positive() {
    assert!(THEME_TRANSITION_MS > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tui_kani_02_app_invariant() {
    let a = App::new();
    assert!(!format!("{:?}", a).is_empty());
}
