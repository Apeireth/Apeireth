//! 驾驶舱氛围 tokens: 深空 HUD 调色 + 动效档位 + 等宽跳动数字。
//!
//! 美学定位为「深空 HUD」, 正文可读性优先: 高亮发光色只用于描边 / 徽章 /
//! 数字, 正文始终用高可读前景色。动效全部集中在 [`MotionTokens`], 每一项
//! 都可被 [`MotionMode::Reduced`] 一档关掉。

use ratatui::style::Color;

/// 动效档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionMode {
    /// 全开: 扫描线 / 呼吸光 / 跳动数字 / 活动指示逐帧动。
    #[default]
    Full,
    /// 减少动态: 一切动画退化为静态呈现 (无障碍档)。
    Reduced,
}

impl MotionMode {
    /// 解析档位名 (`full` / `reduced`, 大小写不敏感)。
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "full" => Some(Self::Full),
            "reduced" => Some(Self::Reduced),
            _ => None,
        }
    }

    /// 是否为全开档。
    pub fn is_full(self) -> bool {
        matches!(self, Self::Full)
    }

    /// 档位的上屏名。
    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "FULL",
            Self::Reduced => "REDUCED",
        }
    }
}

/// 驾驶舱调色 tokens (深空 HUD)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteTokens {
    /// 深空底色。
    pub deep_space: Color,
    /// 舱体面板底色。
    pub hull: Color,
    /// 面板描边基色。
    pub panel_edge: Color,
    /// 主发光描边。
    pub glow_primary: Color,
    /// 次发光描边。
    pub glow_secondary: Color,
    /// 正常 / 成功发光。
    pub glow_ok: Color,
    /// 告警发光。
    pub glow_warn: Color,
    /// 危险发光。
    pub glow_danger: Color,
    /// 正文主色。
    pub text_primary: Color,
    /// 正文弱色。
    pub text_dim: Color,
    /// 扫描线氛围色。
    pub scanline: Color,
    /// 跳动数字色。
    pub digit: Color,
    /// 代码高亮: 关键字。
    pub code_keyword: Color,
    /// 代码高亮: 字符串。
    pub code_string: Color,
    /// 代码高亮: 注释。
    pub code_comment: Color,
    /// 代码高亮: 数字。
    pub code_number: Color,
    /// 代码高亮: 朴素文本。
    pub code_plain: Color,
    /// 工具卡片描边色。
    pub tool_card: Color,
}

impl PaletteTokens {
    /// token 角色全集 (氛围齐备性检查用)。
    pub const ROLE_NAMES: &'static [&'static str] = &[
        "deep_space",
        "hull",
        "panel_edge",
        "glow_primary",
        "glow_secondary",
        "glow_ok",
        "glow_warn",
        "glow_danger",
        "text_primary",
        "text_dim",
        "scanline",
        "digit",
        "code_keyword",
        "code_string",
        "code_comment",
        "code_number",
        "code_plain",
        "tool_card",
    ];

    /// 深空 HUD 调色。
    pub const fn deep_space_hud() -> Self {
        Self {
            deep_space: Color::Rgb(6, 10, 24),
            hull: Color::Rgb(13, 20, 38),
            panel_edge: Color::Rgb(38, 58, 92),
            glow_primary: Color::Rgb(64, 224, 255),
            glow_secondary: Color::Rgb(168, 120, 255),
            glow_ok: Color::Rgb(80, 250, 160),
            glow_warn: Color::Rgb(255, 196, 84),
            glow_danger: Color::Rgb(255, 96, 110),
            text_primary: Color::Rgb(214, 228, 246),
            text_dim: Color::Rgb(122, 142, 172),
            scanline: Color::Rgb(30, 52, 84),
            digit: Color::Rgb(120, 240, 220),
            code_keyword: Color::Rgb(255, 158, 100),
            code_string: Color::Rgb(178, 240, 148),
            code_comment: Color::Rgb(106, 126, 156),
            code_number: Color::Rgb(255, 214, 120),
            code_plain: Color::Rgb(196, 208, 226),
            tool_card: Color::Rgb(255, 138, 196),
        }
    }

    /// (角色名, 颜色) 全集 —— 让齐备性检查可遍历。
    pub fn roles(&self) -> Vec<(&'static str, Color)> {
        vec![
            ("deep_space", self.deep_space),
            ("hull", self.hull),
            ("panel_edge", self.panel_edge),
            ("glow_primary", self.glow_primary),
            ("glow_secondary", self.glow_secondary),
            ("glow_ok", self.glow_ok),
            ("glow_warn", self.glow_warn),
            ("glow_danger", self.glow_danger),
            ("text_primary", self.text_primary),
            ("text_dim", self.text_dim),
            ("scanline", self.scanline),
            ("digit", self.digit),
            ("code_keyword", self.code_keyword),
            ("code_string", self.code_string),
            ("code_comment", self.code_comment),
            ("code_number", self.code_number),
            ("code_plain", self.code_plain),
            ("tool_card", self.tool_card),
        ]
    }
}

/// 动效 tokens: 每一项都可被 reduced 档关掉。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionTokens {
    /// 扫描线氛围纹理 (装饰暗纹, 不覆盖正文)。
    pub scanline_enabled: bool,
    /// 呼吸光 (描边亮度脉动)。
    pub glow_pulse_enabled: bool,
    /// 等宽跳动数字 (装饰性计数)。
    pub digit_tick_enabled: bool,
    /// 活动指示逐帧转动。
    pub spinner_enabled: bool,
    /// 活动指示帧序列。
    pub spinner_frames: &'static [&'static str],
    /// 扫描线行距 (行); 0 = 关。
    pub scanline_stride: u16,
}

/// 驾驶舱主题 = 调色 + 动效档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CockpitTheme {
    /// 当前档位。
    pub mode: MotionMode,
    /// 调色 tokens。
    pub palette: PaletteTokens,
    /// 动效 tokens。
    pub motion: MotionTokens,
}

impl CockpitTheme {
    /// 按档位构建主题。
    pub fn new(mode: MotionMode) -> Self {
        let motion = match mode {
            MotionMode::Full => MotionTokens {
                scanline_enabled: true,
                glow_pulse_enabled: true,
                digit_tick_enabled: true,
                spinner_enabled: true,
                spinner_frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
                scanline_stride: 3,
            },
            MotionMode::Reduced => MotionTokens {
                scanline_enabled: false,
                glow_pulse_enabled: false,
                digit_tick_enabled: false,
                spinner_enabled: false,
                spinner_frames: &["◆"],
                scanline_stride: 0,
            },
        };
        Self {
            mode,
            palette: PaletteTokens::deep_space_hud(),
            motion,
        }
    }

    /// 活动指示的当前帧。
    pub fn spinner_glyph(&self, frame: u64) -> &'static str {
        let frames = self.motion.spinner_frames;
        let index = if self.motion.spinner_enabled {
            (frame % frames.len() as u64) as usize
        } else {
            0
        };
        frames[index]
    }
}

/// 等宽跳动数字: 只用于氛围条上的装饰性计数, 不承载遥测 (真实指标另有
/// 诚实来源, 绝不混用跳动值)。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DigitTicker {
    frame: u64,
}

impl DigitTicker {
    /// 前进一帧。
    pub fn advance(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// 当前帧号。
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// 渲染氛围数字: full 档逐帧跳动, reduced 档恒定静止。
    pub fn render(&self, motion: &MotionTokens) -> String {
        let phase = if motion.digit_tick_enabled {
            self.frame
        } else {
            0
        };
        let chaos = phase
            .wrapping_mul(2_654_435_761)
            .wrapping_add(0x9E37)
            .wrapping_shr(12);
        format!("FLUX {:04X}", chaos & 0xFFFF)
    }

    /// 呼吸光强度 (0-15): full 档三角波脉动, reduced 档恒定。
    pub fn pulse_level(&self, motion: &MotionTokens) -> u8 {
        if !motion.glow_pulse_enabled {
            return 12;
        }
        let phase = self.frame % 16;
        if phase < 8 {
            8 + phase as u8
        } else {
            23 - phase as u8
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 驾驶舱氛围 tokens 齐备: 角色全集完整, 每个角色都有真实颜色。
    #[test]
    fn atmosphere_tokens_cover_all_roles() {
        let palette = PaletteTokens::deep_space_hud();
        let roles = palette.roles();
        assert_eq!(roles.len(), PaletteTokens::ROLE_NAMES.len());
        for (name, color) in &roles {
            assert!(
                PaletteTokens::ROLE_NAMES.contains(name),
                "未知氛围 token 角色: {name}"
            );
            assert_ne!(*color, Color::Reset, "token {name} 未赋色");
        }
        // 角色名与列表一一对应且无重复。
        for role in PaletteTokens::ROLE_NAMES {
            let count = roles.iter().filter(|(name, _)| name == role).count();
            assert_eq!(count, 1, "token 角色 {role} 缺失或重复");
        }
        // 发光三色互相区分 (正常/告警/危险不可同色)。
        assert_ne!(palette.glow_ok, palette.glow_warn);
        assert_ne!(palette.glow_warn, palette.glow_danger);
        assert_ne!(palette.glow_ok, palette.glow_danger);
    }

    /// reduced-motion 档: 全部动效 token 关闭, full 档全部开启。
    #[test]
    fn reduced_motion_disables_every_motion_token() {
        let reduced = CockpitTheme::new(MotionMode::Reduced);
        assert!(!reduced.motion.scanline_enabled);
        assert!(!reduced.motion.glow_pulse_enabled);
        assert!(!reduced.motion.digit_tick_enabled);
        assert!(!reduced.motion.spinner_enabled);
        assert_eq!(reduced.motion.scanline_stride, 0);

        let full = CockpitTheme::new(MotionMode::Full);
        assert!(full.motion.scanline_enabled);
        assert!(full.motion.glow_pulse_enabled);
        assert!(full.motion.digit_tick_enabled);
        assert!(full.motion.spinner_enabled);
        assert!(full.motion.scanline_stride > 0);
    }

    /// 档位解析: full / reduced 可解析, 其余拒绝。
    #[test]
    fn motion_mode_parses_only_known_tiers() {
        assert_eq!(MotionMode::parse("FULL"), Some(MotionMode::Full));
        assert_eq!(MotionMode::parse(" reduced "), Some(MotionMode::Reduced));
        assert_eq!(MotionMode::parse("calm"), None);
    }

    /// 等宽跳动数字: full 档逐帧跳动, reduced 档静止不动。
    #[test]
    fn digit_ticker_moves_in_full_and_freezes_in_reduced() {
        let mut ticker = DigitTicker::default();
        let full = CockpitTheme::new(MotionMode::Full).motion;
        let reduced = CockpitTheme::new(MotionMode::Reduced).motion;

        let reduced_first = ticker.render(&reduced);
        let mut full_seen = Vec::new();
        for _ in 0..6 {
            ticker.advance();
            full_seen.push(ticker.render(&full));
            assert_eq!(ticker.render(&reduced), reduced_first);
        }
        let unique: std::collections::HashSet<String> = full_seen.into_iter().collect();
        assert!(unique.len() > 1, "full 档跳动数字应逐帧变化");

        // 呼吸光: reduced 恒定, full 有起伏。
        let mut levels = std::collections::HashSet::new();
        for _ in 0..16 {
            levels.insert(ticker.pulse_level(&full));
            ticker.advance();
        }
        assert!(levels.len() > 1);
        assert_eq!(ticker.pulse_level(&reduced), 12);
    }
}
