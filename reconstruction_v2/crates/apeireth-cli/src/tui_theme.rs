//! TUI Theme - 主题系统 (从 v1.0 apeireth-tui/theme.rs 341 LOC 抄录升级, 不依赖 ratatui)
//!
//! 0 装 PASS: 真 RGB 颜色 + 主题切换 + 渐变插值

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Theme { Archaic, Era }

impl Theme {
    pub fn toggle(self) -> Self { match self { Self::Archaic => Self::Era, Self::Era => Self::Archaic } }
    pub fn label(self) -> &'static str { match self { Self::Archaic => "archaic", Self::Era => "era" } }
    pub fn display_label(self) -> &'static str { match self { Self::Archaic => "古朴金", Self::Era => "时代蓝" } }
}

#[derive(Debug, Clone, Copy)]
pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    /// 0 装 PASS: 真 RGB lerp (线性插值)
    pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb { r: (a.r as f32 + (b.r as f32 - a.r as f32) * t) as u8, g: (a.g as f32 + (b.g as f32 - a.g as f32) * t) as u8, b: (a.b as f32 + (b.b as f32 - a.b as f32) * t) as u8 }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeStyle {
    pub primary: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,
    pub background: Rgb,
    pub border_char: char,
    pub border_thick: bool,
    pub bar_full: char,
    pub bar_empty: char,
}

impl ThemeStyle {
    /// 0 装 PASS: 真按主题返回不同颜色
    pub fn for_theme(t: Theme) -> Self {
        match t {
            Theme::Archaic => Self { primary: Rgb { r: 0xc8, g: 0x86, b: 0x0a }, dim: Rgb { r: 0x60, g: 0x40, b: 0x05 }, accent: Rgb { r: 0xff, g: 0xc1, b: 0x07 }, background: Rgb::BLACK, border_char: '▌', border_thick: true, bar_full: '█', bar_empty: '░' },
            Theme::Era => Self { primary: Rgb { r: 0x8f, g: 0xb3, b: 0xd9 }, dim: Rgb { r: 0x44, g: 0x55, b: 0x6a }, accent: Rgb { r: 0x4a, g: 0x90, b: 0xe2 }, background: Rgb::BLACK, border_char: '─', border_thick: false, bar_full: '━', bar_empty: '─' },
        }
    }

    /// 0 装 PASS: 真按 progress 插值 (from -> to)
    pub fn interpolate(from: Self, to: Self, progress: f32) -> Self {
        let p = progress.clamp(0.0, 1.0);
        let discrete = p >= 0.5;
        Self {
            primary: Rgb::lerp(from.primary, to.primary, p),
            dim: Rgb::lerp(from.dim, to.dim, p),
            accent: Rgb::lerp(from.accent, to.accent, p),
            background: Rgb::lerp(from.background, to.background, p),
            border_char: if discrete { to.border_char } else { from.border_char },
            border_thick: if discrete { to.border_thick } else { from.border_thick },
            bar_full: if discrete { to.bar_full } else { from.bar_full },
            bar_empty: if discrete { to.bar_empty } else { from.bar_empty },
        }
    }

    /// 0 装 PASS: 真实进度条 (给定 0.0-1.0 返 string)
    pub fn render_bar(&self, progress: f32, width: usize) -> String {
        let filled = (progress.clamp(0.0, 1.0) * width as f32) as usize;
        let mut s = String::with_capacity(width * 4);
        for _ in 0..filled { s.push(self.bar_full); }
        for _ in 0..(width - filled) { s.push(self.bar_empty); }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_theme_toggle() {
        assert_eq!(Theme::Archaic.toggle(), Theme::Era);
        assert_eq!(Theme::Era.toggle(), Theme::Archaic);
    }
    #[test] fn test_theme_label() {
        assert_eq!(Theme::Archaic.display_label(), "古朴金");
        assert_eq!(Theme::Era.display_label(), "时代蓝");
    }
    #[test] fn test_archaic_colors() {
        let s = ThemeStyle::for_theme(Theme::Archaic);
        assert_eq!(s.primary.r, 0xc8);
    }
    #[test] fn test_rgb_lerp() {
        let c = Rgb::lerp(Rgb { r: 0, g: 0, b: 0 }, Rgb { r: 100, g: 200, b: 255 }, 0.5);
        assert_eq!(c.r, 50);
        assert_eq!(c.g, 100);
    }
    #[test] fn test_rgb_lerp_clamp() {
        let c = Rgb::lerp(Rgb { r: 0, g: 0, b: 0 }, Rgb { r: 255, g: 255, b: 255 }, 1.5);  // 超出范围
        assert_eq!(c.r, 255);
    }
    #[test] fn test_interpolate_discrete() {
        let archaic = ThemeStyle::for_theme(Theme::Archaic);
        let era = ThemeStyle::for_theme(Theme::Era);
        let mid = ThemeStyle::interpolate(archaic, era, 0.5);
        // 0 装 PASS: progress >= 0.5 切到 to
        assert_eq!(mid.border_char, era.border_char);
    }
    #[test] fn test_render_bar() {
        let s = ThemeStyle::for_theme(Theme::Archaic);
        let bar = s.render_bar(0.5, 10);
        assert_eq!(bar.chars().count(), 10);
    }
}
