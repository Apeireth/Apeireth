//! # `tests/test_tui_e2e_in_process.rs` — 20+ e2e 测试 (派活单 §7)
//!
//! **职责**: 集成测试, 跑 `apeireth_tui_e2e` 的公开 API, 端到端验证
//! backend / harness / render / 5 nav / 9 organ / 颜色 / 尺寸 / 哲学锚.
//!
//! **20+ 测试** (派活单明确列了 20 个, 本文件加到 25+ 含边界用例):
//! 1. test_backend_24x80_default
//! 2. test_backend_120x40_wide
//! 3. test_harness_start_quit
//! 4. test_harness_tick_1s
//! 5. test_harness_send_key_q_quit
//! 6. test_harness_send_key_tab_navigation
//! 7. test_harness_send_key_1_to_5_jump
//! 8. test_render_4_panel_layout
//! 9. test_render_5_nav_top_bar
//! 10. test_render_9_organ_middle_bar
//! 11. test_render_status_bar
//! 12. test_5_nav_each_renders
//! 13. test_9_organ_each_renders
//! 14. test_color_red_for_error
//! 15. test_color_green_for_ok
//! 16. test_color_yellow_for_warning
//! 17. test_organ_heart_pulse_animation
//! 18. test_organ_mind_anchors_visible
//! 19. test_help_anchors_count_6
//! 20. test_k1_zero_size_backend
//!
//! 附加 (5 个边界):
//! 21. test_send_key_esc_quit
//! 22. test_chat_history_user_assistant
//! 23. test_theme_cycle_archaic_modern_cosmic
//! 24. test_5_r_measures_in_status
//! 25. test_8_promises_in_settings
//!
//! **8 不修改承诺**: 跟 src/ 一致

use apeireth_tui_e2e::prelude::*;
use apeireth_tui_e2e::{
    EIGHT_PROMISES, FIVE_R_MEASURES, Nav, NavPage, Organ, SIX_PHI_ANCHORS, TuiApp, TuiHarness,
    TuiTestBackend,
};
use crossterm::event::KeyCode;
use ratatui::style::Color;

// =====================================================================
// 1-2. backend 尺寸
// =====================================================================

#[test]
fn test_backend_24x80_default() {
    let b = TuiTestBackend::default_24x80().expect("default 24x80");
    assert_eq!(b.width, 80);
    assert_eq!(b.height, 24);
    let snap = b.snapshot();
    assert_eq!(snap.width, 80);
    assert_eq!(snap.height, 24);
}

#[test]
fn test_backend_120x40_wide() {
    let b = TuiTestBackend::wide_120x40().expect("wide 120x40");
    assert_eq!(b.width, 120);
    assert_eq!(b.height, 40);
}

// =====================================================================
// 3-7. harness 启动 + tick + key
// =====================================================================

#[test]
fn test_harness_start_quit() {
    let mut h = TuiHarness::start().expect("harness start");
    assert!(!h.app.should_quit);
    h.quit().expect("quit");
    assert!(h.app.should_quit);
}

#[test]
fn test_harness_tick_1s() {
    let mut h = TuiHarness::start().expect("harness start");
    let before = h.app.spinner_frame;
    h.tick().expect("tick");
    assert_ne!(h.app.spinner_frame, before, "spinner 推进");
    assert!(h.app.render_tick > 0, "render_tick 推进");
}

#[test]
fn test_harness_send_key_q_quit() {
    let mut h = TuiHarness::start().expect("harness start");
    h.send_key(KeyCode::Char('q')).expect("send q");
    assert!(h.app.should_quit);
}

#[test]
fn test_harness_send_key_tab_navigation() {
    let mut h = TuiHarness::start().expect("harness start");
    assert_eq!(h.app.nav, NavPage::Bridge);
    h.send_key(KeyCode::Tab).expect("tab");
    assert_eq!(h.app.nav, NavPage::Dialogue);
    h.send_key(KeyCode::BackTab).expect("backtab");
    assert_eq!(h.app.nav, NavPage::Bridge);
}

#[test]
fn test_harness_send_key_1_to_5_jump() {
    let mut h = TuiHarness::start().expect("harness start");
    h.send_key(KeyCode::Char('3')).expect("3");
    assert_eq!(h.app.nav, NavPage::Growth);
    h.send_key(KeyCode::Char('5')).expect("5");
    assert_eq!(h.app.nav, NavPage::Settings);
    h.send_key(KeyCode::Char('1')).expect("1");
    assert_eq!(h.app.nav, NavPage::Bridge);
}

#[test]
fn test_send_key_esc_quit() {
    let mut h = TuiHarness::start().expect("harness start");
    h.send_key(KeyCode::Esc).expect("esc");
    assert!(h.app.should_quit);
}

// =====================================================================
// 8-11. 渲染 — 1 屏 4 panel + 5 nav + 9 organ + status
// =====================================================================

#[test]
fn test_render_4_panel_layout() {
    let mut h = TuiHarness::start().expect("harness start");
    h.render_4_panel().expect("render 4 panel");
    let snap = h.snapshot();
    // 4 panel 都得有内容
    assert!(snap.text.len() > 100, "4 panel 渲染应产生非空 buffer");
    // top nav 应有 ▶ (当前 nav 标记)
    assert!(snap.contains("▶"), "top nav 应有当前标记");
    // status bar 应有 5 R-Measure 标记 (紧凑: R5: 前缀)
    assert!(snap.contains("R5:"), "status bar 应有 R5: 前缀");
}

#[test]
fn test_render_5_nav_top_bar() {
    let mut h = TuiHarness::start().expect("harness start");
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    // 5 nav 中文标签都应在 top bar
    for n in 0..NavPage::COUNT {
        let page = NavPage::from_u8(n).unwrap();
        assert!(
            snap.contains(page.label_zh()),
            "top bar 应有 nav {n}: {}",
            page.label_zh()
        );
    }
    // 5 个数字 1-5
    for n in 1..=5 {
        assert!(snap.contains(&n.to_string()), "top bar 应有数字 {n}");
    }
}

#[test]
fn test_render_9_organ_middle_bar() {
    let mut h = TuiHarness::start().expect("harness start");
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    // 9 organ ASCII 都应在 middle bar 或 content area
    for i in 0..Organ::COUNT {
        let organ = Organ::from_u8(i).unwrap();
        assert!(
            snap.contains(organ.ascii()),
            "应渲染 organ {}: {}",
            i,
            organ.ascii()
        );
    }
}

#[test]
fn test_render_status_bar() {
    let mut h = TuiHarness::start().expect("harness start");
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    // status bar 5 元素 (紧凑形式: c= / t= / 5s=)
    assert!(snap.contains("c="), "status 应有 cycle (c=)");
    assert!(snap.contains("t="), "status 应有 token (t=)");
    assert!(snap.contains("5s="), "status 应有 5self (5s=)");
    // 5 R-Measure 紧凑码
    assert!(snap.contains("R5:"));
    assert!(snap.contains("C|D|K|A|P"), "5 R-Measure 紧凑码");
}

// =====================================================================
// 12-13. 5 nav 各自渲染 + 9 organ 各自渲染
// =====================================================================

#[test]
fn test_5_nav_each_renders() {
    let mut h = TuiHarness::start().expect("harness start");
    for n in 0..NavPage::COUNT {
        h.send_key(KeyCode::Char((b'1' + n) as char)).expect("key");
        h.render_4_panel().expect("render");
        let snap = h.snapshot();
        // 每个 nav 都应渲染
        assert!(snap.text.len() > 50, "nav {n} 渲染 buffer 非空");
    }
}

#[test]
fn test_9_organ_each_renders() {
    let mut h = TuiHarness::start().expect("harness start");
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    // 9 organ ASCII 全部命中
    let mut count = 0;
    for i in 0..Organ::COUNT {
        let organ = Organ::from_u8(i).unwrap();
        if snap.contains(organ.ascii()) {
            count += 1;
        }
    }
    assert_eq!(count, 9, "9 器官 ASCII 全应命中, 实际 {count}");
}

// =====================================================================
// 14-16. 颜色 — 红 / 绿 / 黄
// =====================================================================

#[test]
fn test_color_red_for_error() {
    // 0% health → 红色
    let mut h = TuiHarness::start().expect("harness start");
    h.app.organ_health[Organ::Heart as usize] = 0.1; // < 50% → 红
    h.render_4_panel().expect("render");
    let buf = h.buffer();
    // middle bar 第 0 段是 HEART, 找到第一个 'R' 字符位置, 查 fg
    // 简化: 找一个明显红色 (Color::Red) 像素
    let mut found_red = false;
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].fg == Color::Red {
                found_red = true;
                break;
            }
        }
        if found_red {
            break;
        }
    }
    assert!(found_red, "应有红色像素 (HEART < 50% health)");
}

#[test]
fn test_color_green_for_ok() {
    // 100% health → 绿色
    let mut h = TuiHarness::start().expect("harness start");
    h.app.organ_health[Organ::Heart as usize] = 1.0;
    h.render_4_panel().expect("render");
    let buf = h.buffer();
    let mut found_green = false;
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].fg == Color::Green {
                found_green = true;
                break;
            }
        }
        if found_green {
            break;
        }
    }
    assert!(found_green, "应有绿色像素 (HEART 100% health)");
}

#[test]
fn test_color_yellow_for_warning() {
    // 60% health → 黄色
    let mut h = TuiHarness::start().expect("harness start");
    h.app.organ_health[Organ::Brain as usize] = 0.6;
    h.render_4_panel().expect("render");
    let buf = h.buffer();
    let mut found_yellow = false;
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].fg == Color::Yellow {
                found_yellow = true;
                break;
            }
        }
        if found_yellow {
            break;
        }
    }
    assert!(found_yellow, "应有黄色像素 (BRAIN 60% health)");
}

// =====================================================================
// 17-19. 器官动画 + 哲学锚 + 6 锚计数
// =====================================================================

#[test]
fn test_organ_heart_pulse_animation() {
    let mut h = TuiHarness::start().expect("harness start");
    h.app.organ_health[Organ::Heart as usize] = 1.0;
    h.render_4_panel().expect("render");
    // 4 次 tick 模拟 1s, spinner 回到 0
    for _ in 0..4 {
        h.tick().expect("tick");
    }
    assert_eq!(h.app.spinner_frame, 0, "4 tick 后 spinner 回到 0");
    // render 不 panic
    h.render_4_panel().expect("render after tick");
}

#[test]
fn test_organ_mind_anchors_visible() {
    let mut h = TuiHarness::start().expect("harness start");
    h.app.nav = NavPage::Bridge; // Bridge 渲染 Mind + 6 锚
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    assert!(snap.contains("[MIND]"), "Mind 渲染");
    // 6 哲学锚全显
    for (id, _, _) in SIX_PHI_ANCHORS.iter() {
        assert!(snap.contains(id), "锚 {id} 渲染");
    }
}

#[test]
fn test_help_anchors_count_6() {
    // 跟派活单 "Help 6 哲学锚" 同步, 验证 6 锚数 hardcode
    assert_eq!(SIX_PHI_ANCHORS.len(), 6, "6 哲学锚 hardcode");
    let mut h = TuiHarness::start().expect("harness start");
    h.app.sub_nav = Nav::Help;
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    // 6 个 ID 全部命中
    let mut hit = 0;
    for (id, _, _) in SIX_PHI_ANCHORS.iter() {
        if snap.contains(id) {
            hit += 1;
        }
    }
    assert_eq!(hit, 6, "Help 渲染 6 哲学锚, 实际 {hit}");
}

// =====================================================================
// 20. K-1 强校验 — 0 尺寸 backend 拒绝
// =====================================================================

#[test]
fn test_k1_zero_size_backend() {
    let r = TuiTestBackend::new(0, 0);
    assert!(r.is_err(), "0x0 backend 应被 K-1 强校验拒绝");
    let r = TuiTestBackend::new(0, 24);
    assert!(r.is_err(), "0 宽应被 K-1 强校验拒绝");
    let r = TuiTestBackend::new(24, 0);
    assert!(r.is_err(), "0 高应被 K-1 强校验拒绝");
}

// =====================================================================
// 21-25. 边界用例 (5 个附加)
// =====================================================================

#[test]
fn test_chat_history_user_assistant() {
    let mut h = TuiHarness::start_with_chat(vec![
        ("user", "question 1"),
        ("assistant", "answer 1"),
    ])
    .expect("harness with chat");
    assert_eq!(h.app.chat_history.len(), 2);
    h.app.nav = NavPage::History;
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    assert!(snap.contains("question 1"));
    assert!(snap.contains("answer 1"));
}

#[test]
fn test_theme_cycle_archaic_modern_cosmic() {
    let mut h = TuiHarness::start().expect("harness start");
    assert_eq!(h.app.theme, Theme::Archaic);
    h.send_key(KeyCode::Char('t')).expect("t");
    assert_eq!(h.app.theme, Theme::Modern);
    h.send_key(KeyCode::Char('t')).expect("t");
    assert_eq!(h.app.theme, Theme::Cosmic);
    h.send_key(KeyCode::Char('t')).expect("t");
    assert_eq!(h.app.theme, Theme::Archaic);
}

#[test]
fn test_5_r_measures_in_status() {
    assert_eq!(FIVE_R_MEASURES.len(), 5, "5 R-Measure hardcode");
    let mut h = TuiHarness::start().expect("harness start");
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    // 紧凑 R5:C|D|K|A|P 在 status bar
    assert!(snap.contains("R5:"), "status bar 应有 R5: 前缀");
    assert!(snap.contains("C|D|K|A|P"), "status bar 应有 5 紧凑码");
    // 切到 Help sub-nav 看 5 R-Measure 全名
    h.app.sub_nav = Nav::Help;
    h.render_4_panel().expect("render help");
    let snap2 = h.snapshot();
    for m in FIVE_R_MEASURES.iter() {
        assert!(
            snap2.contains(m),
            "Help sub-nav 应有完整 R-Measure {m}"
        );
    }
}

#[test]
fn test_8_promises_in_settings() {
    assert_eq!(EIGHT_PROMISES.len(), 8, "8 不修改承诺 hardcode");
    let mut h = TuiHarness::start().expect("harness start");
    h.app.nav = NavPage::Settings;
    h.render_4_panel().expect("render");
    let snap = h.snapshot();
    for (i, _) in EIGHT_PROMISES.iter().enumerate() {
        // Settings 渲染 "1. ..." "2. ..." 等
        assert!(
            snap.contains(&format!("{}.", i + 1)),
            "Settings 应有第 {} 承诺",
            i + 1
        );
    }
}

#[test]
fn test_all_render_no_panic_at_extreme_sizes() {
    // 边界尺寸 — 测 ratatui Layout 的鲁棒性
    for (w, h) in [(20u16, 5u16), (200, 50), (1, 1), (80, 24)] {
        let mut harness = TuiHarness::start_with_size(w, h).expect("harness start");
        harness.render_4_panel().expect("render 4 panel");
    }
}
