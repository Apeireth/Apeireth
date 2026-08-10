//! Apeireth R19 TUI — 主题 (古朴金 / 时代蓝)
//!
//! **设计原则** (主人 R19 决定):
//! - 古朴 ARCHAIC: 暖色 (砖块金 0xc8860a), 厚边框 (BorderType::THICK)
//! - 时代 ERA: 冷色 (淡蓝 0x8fb3d9), 细线边框 (BorderType::PLAIN)
//! - 背景一律 `Color::Black`
//! - 主题切换时全屏重新渲染, 不需要重启 (主循环每帧重画)
//!
//! **W1 实现说明**: ratatui 0.29 的 `symbols::border::Set` 只支持 `&str` 字段,
//! 而设计文档要求单字符边框 (`▌▐▍▎▔▕` 等), 我们用 ratatui 内置的
//! `BorderType::THICK` (近似砖块感) / `BorderType::PLAIN` (细线感) 来近似,
//! 主题颜色 + bar 字符完全保留.
//!
//! **W3.6 平滑过渡**: `ThemeStyle::interpolate(from, to, progress)` 用于按 `t` 键切主题
//! 时的 200ms RGB 线性插值 — 颜色 (primary/dim/bg/accent) 走 RGB lerp,
//! 离散字段 (border_type / border_char / bar_full / bar_empty / star) 在 progress ≥ 0.5
//! 切到 to, 否则保留 from. progress ∈ [0.0, 1.0], 范围外自动 clamp.

use ratatui::style::Color;
use ratatui::widgets::BorderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// 砖块金 (古朴, 默认)
    Archaic,
    /// 细线蓝 (时代)
    Era,
}

impl Theme {
    pub fn toggle(self) -> Self {
        match self {
            Theme::Archaic => Theme::Era,
            Theme::Era => Theme::Archaic,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Archaic => "archaic",
            Theme::Era => "era",
        }
    }

    /// R26: 中文显示 (UI 用, 跟 canonical label() 区分, 不改持久化 keys)
    pub fn display_label(self) -> &'static str {
        match self {
            Theme::Archaic => "古朴金",
            Theme::Era => "时代蓝",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeStyle {
    /// ratatui 0.29 内置边框类型 (THICK ≈ 砖块, PLAIN = 细线)
    pub border_type: BorderType,
    /// 顶/底横线单字符 (用于画进度条 / 星图字符装饰)
    pub border_char: char,
    /// 进度条满
    pub bar_full: char,
    /// 进度条空
    pub bar_empty: char,
    /// 星图点
    pub star: char,
    /// 主色
    pub primary: Color,
    /// 暗色
    pub dim: Color,
    /// 背景 (恒为 Black, W1 设计)
    #[allow(dead_code)]
    pub bg: Color,
    /// 高亮 (反色)
    pub accent: Color,
}

impl ThemeStyle {
    pub fn of(theme: Theme) -> Self {
        match theme {
            Theme::Archaic => Self {
                border_type: BorderType::Thick,
                border_char: '▔',
                bar_full: '█',
                bar_empty: '░',
                star: '·',
                primary: Color::Rgb(0xc8, 0x86, 0x0a), // 砖块金
                dim: Color::Rgb(0x80, 0x60, 0x40),     // 暗金
                bg: Color::Black,
                accent: Color::Rgb(0xff, 0xd8, 0x8a), // 高亮金
            },
            Theme::Era => Self {
                border_type: BorderType::Plain,
                border_char: '─',
                bar_full: '▰',
                bar_empty: '─',
                star: '·',
                primary: Color::Rgb(0x8f, 0xb3, 0xd9), // 淡蓝
                dim: Color::Rgb(0x50, 0x68, 0x80),     // 暗蓝
                bg: Color::Black,
                accent: Color::Rgb(0xc8, 0xe0, 0xff), // 高亮蓝
            },
        }
    }

    /// 200ms 平滑过渡用的颜色插值 (W3.6)
    ///
    /// progress 0.0 = `from`, 1.0 = `to` (整数端点). 范围外自动 clamp.
    /// 颜色 (primary / dim / bg / accent) 走 RGB 线性插值 — 真改 RGB, 不是 sleep.
    /// 离散字段 (border_type / border_char / bar_full / bar_empty / star) 在
    /// progress < 0.5 用 from, ≥ 0.5 用 to (跨中线切换, 避免每帧抖).
    ///
    /// 兜底: 任一端不是 RGB (例如 `Color::Reset` / `Color::Indexed`) 时
    /// progress < 0.5 选 from, 否则选 to (保持原值, 不崩).
    pub fn interpolate(from: ThemeStyle, to: ThemeStyle, progress: f64) -> ThemeStyle {
        let p = progress.clamp(0.0, 1.0);
        // 离散字段: 中线切换
        let (border_type, border_char, bar_full, bar_empty, star) = if p < 0.5 {
            (
                from.border_type,
                from.border_char,
                from.bar_full,
                from.bar_empty,
                from.star,
            )
        } else {
            (
                to.border_type,
                to.border_char,
                to.bar_full,
                to.bar_empty,
                to.star,
            )
        };
        Self {
            border_type,
            border_char,
            bar_full,
            bar_empty,
            star,
            primary: lerp_color(from.primary, to.primary, p),
            dim: lerp_color(from.dim, to.dim, p),
            bg: lerp_color(from.bg, to.bg, p),
            accent: lerp_color(from.accent, to.accent, p),
        }
    }
}

/// RGB 线性插值 (W3.6). progress 0.0 = a, 1.0 = b. 整数端点精确 (无浮点漂移).
/// 兜底: 任一端非 RGB → 中线选 a/b (不崩).
fn lerp_color(a: Color, b: Color, p: f64) -> Color {
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) {
        let mix = |x: u8, y: u8| -> u8 {
            let fx = f64::from(x);
            let fy = f64::from(y);
            let v = fx + (fy - fx) * p;
            v.round().clamp(0.0, 255.0) as u8
        };
        Color::Rgb(mix(r1, r2), mix(g1, g2), mix(b1, b2))
    } else if p < 0.5 {
        a
    } else {
        b
    }
}

// ============================================================
// 单元测试 (W3.6, 5 个: 0.0 / 1.0 / 0.5 / clamp / 离散字段中线切换)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(c: Color) -> Option<(u8, u8, u8)> {
        if let Color::Rgb(r, g, b) = c {
            Some((r, g, b))
        } else {
            None
        }
    }

    /// ratatui 0.29 里 `Color::Black` 实际是 `Color::Indexed(0)`, 不是 `Color::Rgb(0,0,0)`.
    /// 测试断言时把它当作 RGB(0,0,0) 等价 (因为 R19 W1 设计 "背景一律 Color::Black"
    /// 实际就是黑色, 跟 RGB(0,0,0) 视觉一致).
    fn to_rgb(c: Color) -> Option<(u8, u8, u8)> {
        if let Some(rgb) = rgb(c) {
            Some(rgb)
        } else if matches!(c, Color::Black) {
            Some((0, 0, 0))
        } else {
            None
        }
    }

    fn color_close(a: Color, b: Color, tol: i32) -> bool {
        match (to_rgb(a), to_rgb(b)) {
            (Some(x), Some(y)) => {
                let dr = (i32::from(x.0) - i32::from(y.0)).abs();
                let dg = (i32::from(x.1) - i32::from(y.1)).abs();
                let db = (i32::from(x.2) - i32::from(y.2)).abs();
                dr <= tol && dg <= tol && db <= tol
            }
            _ => false,
        }
    }

    #[test]
    fn interpolate_at_zero_returns_from() {
        // progress = 0.0: 颜色 / 离散字段全部等于 from (archaic)
        let from = ThemeStyle::of(Theme::Archaic);
        let to = ThemeStyle::of(Theme::Era);
        let s = ThemeStyle::interpolate(from, to, 0.0);
        // 颜色必须几乎相等 (浮点四舍五入 ±1 容差)
        assert!(
            color_close(s.primary, from.primary, 1),
            "primary should equal from"
        );
        assert!(color_close(s.dim, from.dim, 1), "dim should equal from");
        assert!(color_close(s.bg, from.bg, 1), "bg should equal from");
        assert!(
            color_close(s.accent, from.accent, 1),
            "accent should equal from"
        );
        // 离散字段 (progress 0.0 < 0.5) 保留 from
        assert_eq!(s.border_type, from.border_type);
        assert_eq!(s.border_char, from.border_char);
        assert_eq!(s.bar_full, from.bar_full);
        assert_eq!(s.bar_empty, from.bar_empty);
        assert_eq!(s.star, from.star);
    }

    #[test]
    fn interpolate_at_one_returns_to() {
        // progress = 1.0: 颜色 / 离散字段全部等于 to (era)
        let from = ThemeStyle::of(Theme::Archaic);
        let to = ThemeStyle::of(Theme::Era);
        let s = ThemeStyle::interpolate(from, to, 1.0);
        assert!(
            color_close(s.primary, to.primary, 1),
            "primary should equal to"
        );
        assert!(color_close(s.dim, to.dim, 1), "dim should equal to");
        assert!(color_close(s.bg, to.bg, 1), "bg should equal to");
        assert!(
            color_close(s.accent, to.accent, 1),
            "accent should equal to"
        );
        // 离散字段 (progress 1.0 ≥ 0.5) 切到 to
        assert_eq!(s.border_type, to.border_type);
        assert_eq!(s.border_char, to.border_char);
        assert_eq!(s.bar_full, to.bar_full);
        assert_eq!(s.bar_empty, to.bar_empty);
        assert_eq!(s.star, to.star);
    }

    #[test]
    fn interpolate_at_half_is_midway() {
        // progress = 0.5: 颜色在 from/to 中点 (RGB 线性插值)
        let from = ThemeStyle::of(Theme::Archaic);
        let to = ThemeStyle::of(Theme::Era);
        let s = ThemeStyle::interpolate(from, to, 0.5);
        // primary: (200,134,10) → (143,179,217) 中点 ≈ (172, 156, 113)
        let (r, g, b) = rgb(s.primary).expect("primary is RGB");
        // ±2 容差 (中间值会因 round() 微差, 但不应偏向任何一端)
        assert!((i32::from(r) - 171).abs() <= 2, "primary.r ≈ 171, got {}", r);
        assert!((i32::from(g) - 156).abs() <= 2, "primary.g ≈ 156, got {}", g);
        assert!((i32::from(b) - 113).abs() <= 2, "primary.b ≈ 113, got {}", b);
        // 严格在 from/to 之间 (不偏向任何一端)
        let (rf, gf, bf) = rgb(from.primary).unwrap();
        let (rt, gt, bt) = rgb(to.primary).unwrap();
        assert!(
            r > rf.min(rt) && r < rf.max(rt),
            "primary.r must be strictly between"
        );
        assert!(
            g > gf.min(gt) && g < gf.max(gt),
            "primary.g must be strictly between"
        );
        assert!(
            b > bf.min(bt) && b < bf.max(bt),
            "primary.b must be strictly between"
        );
        // 离散字段: progress = 0.5 (边界, 我们用 ≥ 0.5 切到 to)
        assert_eq!(s.border_type, to.border_type);
    }

    #[test]
    fn interpolate_clamps_out_of_range_progress() {
        // progress < 0.0 → clamp 到 0.0 (全 from)
        // progress > 1.0 → clamp 到 1.0 (全 to)
        let from = ThemeStyle::of(Theme::Archaic);
        let to = ThemeStyle::of(Theme::Era);
        let s_low = ThemeStyle::interpolate(from, to, -1.0);
        let s_high = ThemeStyle::interpolate(from, to, 2.0);
        // s_low ≈ from
        assert!(color_close(s_low.primary, from.primary, 1));
        assert_eq!(s_low.border_type, from.border_type);
        // s_high ≈ to
        assert!(color_close(s_high.primary, to.primary, 1));
        assert_eq!(s_high.border_type, to.border_type);
    }

    #[test]
    fn interpolate_discrete_fields_switch_at_half() {
        // progress < 0.5: 离散字段保留 from
        // progress ≥ 0.5: 离散字段切到 to
        let from = ThemeStyle::of(Theme::Archaic);
        let to = ThemeStyle::of(Theme::Era);
        let s_30 = ThemeStyle::interpolate(from, to, 0.3);
        let s_70 = ThemeStyle::interpolate(from, to, 0.7);
        // 0.3 < 0.5: 离散字段 = from
        assert_eq!(s_30.border_type, from.border_type);
        assert_eq!(s_30.border_char, from.border_char);
        assert_eq!(s_30.bar_full, from.bar_full);
        // 0.7 ≥ 0.5: 离散字段 = to
        assert_eq!(s_70.border_type, to.border_type);
        assert_eq!(s_70.border_char, to.border_char);
        assert_eq!(s_70.bar_full, to.bar_full);
        // 但 0.3 时颜色已经是中间色 (RGB 连续插值, 不受中线切换影响)
        let (r30, _, _) = rgb(s_30.primary).unwrap();
        let (rf, _, _) = rgb(from.primary).unwrap();
        let (rt, _, _) = rgb(to.primary).unwrap();
        assert!(
            r30 > rf.min(rt) && r30 < rf.max(rt),
            "0.3 时 primary.r 应该在 from/to 之间 (continuous lerp), got {}",
            r30
        );
    }
}
