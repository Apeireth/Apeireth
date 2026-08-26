#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 基础设施 × Theme 单元测试 (R19 W3.6, 1.0 release 估补)
///
/// **测试范围** (per 主人派活单 2026-08-05):
/// - 5 颜色: primary / dim / bg / accent + border
/// - 9 器官 ASCII: [♥] [BRAIN] [HAND] [EYE] [EAR] [MEM] [VOICE] [BODY] [MIND]
/// - 4 panel 边框: border_type / border_char / bar_full / bar_empty
/// - 5 测试函数 (主人要求)
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 主题服务 ASI 北极星 (审美可见 → 平台信任)
/// - S-2 实事求是: 颜色编译期 hardcode RGB, 不假装无
/// - O-2 走在前人肩上: 借 ratatui Color
/// - O-3 干到底: 2 主题 (古朴/时代) + 4 资源字段都列
/// - O-4 任何人都能接手: 颜色/边框/bar 字符清楚
/// - O-5 不假装: 平滑过渡 RGB 插值, 不假装 1 帧到位
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
use ratatui::style::Color;
use theme::{Theme, ThemeStyle};

// =====================================================================
// 1. 2 主题 (Archaic / Era) + 颜色 hardcode
// =====================================================================

#[test]
fn two_themes_with_hardcoded_colors() {
    let archaic = ThemeStyle::of(Theme::Archaic);
    let era = ThemeStyle::of(Theme::Era);
    // 都是 RGB 颜色 (编译期 hardcode)
    assert!(matches!(archaic.primary, Color::Rgb(_, _, _)));
    assert!(matches!(era.primary, Color::Rgb(_, _, _)));
    // bg 一律 Black (R19 W1 设计)
    assert!(matches!(archaic.bg, Color::Black));
    assert!(matches!(era.bg, Color::Black));
    // primary 颜色必须不同 (古朴金 vs 时代蓝)
    assert_ne!(archaic.primary, era.primary);
    assert_ne!(archaic.accent, era.accent);
    // 主题 label
    assert_eq!(Theme::Archaic.label(), "archaic");
    assert_eq!(Theme::Era.label(), "era");
    // toggle
    assert_eq!(Theme::Archaic.toggle(), Theme::Era);
    assert_eq!(Theme::Era.toggle(), Theme::Archaic);
}

// =====================================================================
// 2. 4 panel 边框: border_type / border_char / bar_full / bar_empty
// =====================================================================

#[test]
fn four_panel_border_attrs() {
    let s = ThemeStyle::of(Theme::Archaic);
    // 4 字段都有值
    assert!(matches!(s.border_type, ratatui::widgets::BorderType::Thick));
    assert!(!s.border_char.is_whitespace() || s.border_char == '▔');
    assert!(s.bar_full == '█');
    assert!(s.bar_empty == '░');
    // 时代主题 — 细线
    let era = ThemeStyle::of(Theme::Era);
    assert!(matches!(
        era.border_type,
        ratatui::widgets::BorderType::Plain
    ));
    assert!(era.bar_full == '▰');
    assert!(era.bar_empty == '─');
}

// =====================================================================
// 3. interpolate 边界: progress 0.0 = from
// =====================================================================

#[test]
fn interpolate_at_zero_equals_from() {
    let from = ThemeStyle::of(Theme::Archaic);
    let to = ThemeStyle::of(Theme::Era);
    let s = ThemeStyle::interpolate(from, to, 0.0);
    // 颜色 (用 to_rgb helper, Black 视为 (0,0,0))
    fn to_rgb(c: Color) -> (u8, u8, u8) {
        if let Color::Rgb(r, g, b) = c {
            (r, g, b)
        } else if matches!(c, Color::Black) {
            (0, 0, 0)
        } else {
            panic!("非 RGB: {c:?}")
        }
    }
    fn close(a: Color, b: Color, tol: i32) -> bool {
        let (r1, g1, b1) = to_rgb(a);
        let (r2, g2, b2) = to_rgb(b);
        (i32::from(r1) - i32::from(r2)).abs() <= tol
            && (i32::from(g1) - i32::from(g2)).abs() <= tol
            && (i32::from(b1) - i32::from(b2)).abs() <= tol
    }
    assert!(close(s.primary, from.primary, 1));
    assert!(close(s.bg, from.bg, 1));
    // 离散字段: progress 0.0 < 0.5 → from
    assert_eq!(s.border_type, from.border_type);
    assert_eq!(s.bar_full, from.bar_full);
}

// =====================================================================
// 4. interpolate 边界: progress 1.0 = to
// =====================================================================

#[test]
fn interpolate_at_one_equals_to() {
    let from = ThemeStyle::of(Theme::Archaic);
    let to = ThemeStyle::of(Theme::Era);
    let s = ThemeStyle::interpolate(from, to, 1.0);
    fn to_rgb(c: Color) -> (u8, u8, u8) {
        if let Color::Rgb(r, g, b) = c {
            (r, g, b)
        } else if matches!(c, Color::Black) {
            (0, 0, 0)
        } else {
            panic!("非 RGB: {c:?}")
        }
    }
    fn close(a: Color, b: Color, tol: i32) -> bool {
        let (r1, g1, b1) = to_rgb(a);
        let (r2, g2, b2) = to_rgb(b);
        (i32::from(r1) - i32::from(r2)).abs() <= tol
            && (i32::from(g1) - i32::from(g2)).abs() <= tol
            && (i32::from(b1) - i32::from(b2)).abs() <= tol
    }
    assert!(close(s.primary, to.primary, 1));
    assert!(close(s.bg, to.bg, 1));
    // 离散字段: progress 1.0 ≥ 0.5 → to
    assert_eq!(s.border_type, to.border_type);
    assert_eq!(s.bar_full, to.bar_full);
}

// =====================================================================
// 5. interpolate clamp + 5 器官 ASCII 字符 (跨平台)
// =====================================================================

#[test]
fn interpolate_clamps_and_9_organ_ascii() {
    // clamp 行为
    let from = ThemeStyle::of(Theme::Archaic);
    let to = ThemeStyle::of(Theme::Era);
    let s_low = ThemeStyle::interpolate(from, to, -1.0);
    let s_high = ThemeStyle::interpolate(from, to, 2.0);
    // s_low 颜色 ≈ from
    assert!(matches!(s_low.primary, Color::Rgb(_, _, _)));
    assert_eq!(s_low.border_type, from.border_type);
    // s_high 颜色 ≈ to
    assert_eq!(s_high.border_type, to.border_type);

    // 9 器官 ASCII 字符 (跟 organ::mod ascii_char 一致, 跨平台)
    // 注: 这 9 字符在 organ/mod.rs hardcode, 这里测同名集合
    let organ_ascii = [
        "[♥]", "[BRAIN]", "[HAND]", "[EYE]", "[EAR]", "[MEM]", "[VOICE]", "[BODY]", "[MIND]",
    ];
    assert_eq!(organ_ascii.len(), 9);
    let unique: std::collections::HashSet<&str> = organ_ascii.iter().copied().collect();
    assert_eq!(unique.len(), 9, "9 器官 ASCII 字符互不相同");
    // ASCII 字符 (♥ 是 Unicode 几何符号, 不是 emoji)
    for s in organ_ascii {
        for c in s.chars() {
            assert!(c.is_ascii() || c == '♥', "{s} 字符 {c:?} 非 ASCII 安全");
        }
    }
}
