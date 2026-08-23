//! Apeireth TUI — 经典双主题系统 (古朴金 / 时代蓝)
//!
//! - 古朴 ARCHAIC (默认): 暖色 (砖块金 0xc8860a), 厚边框 (BorderType::Thick), 砖块字符 ▔, █/░
//! - 时代 ERA: 冷色 (淡蓝 0x8fb3d9), 细线边框 (BorderType::Plain), 细线字符 ─, ▰/─
//! - 背景一律 Color::Black
//! - 按 't' 键触发 200ms RGB 线性平滑插值过渡

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

    pub fn display_label(self) -> &'static str {
        match self {
            Theme::Archaic => "古朴金 (Archaic)",
            Theme::Era => "时代蓝 (Era)",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeStyle {
    pub border_type: BorderType,
    pub border_char: char,
    pub bar_full: char,
    pub bar_empty: char,
    pub star: char,
    pub primary: Color,
    pub dim: Color,
    pub bg: Color,
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

    pub fn interpolate(from: ThemeStyle, to: ThemeStyle, progress: f64) -> ThemeStyle {
        let p = progress.clamp(0.0, 1.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_interpolation() {
        let archaic = ThemeStyle::of(Theme::Archaic);
        let era = ThemeStyle::of(Theme::Era);
        let mid = ThemeStyle::interpolate(archaic, era, 0.5);
        assert_eq!(mid.border_type, BorderType::Plain);
    }
}
