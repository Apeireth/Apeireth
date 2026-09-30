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
    /// 动画帧预算间隔 (毫秒): full 档 250ms (4fps, ≤5fps 红线), 0 = 无动画帧
    /// (reduced 档: 一切动画退化静态, 只在数据变化时重绘)。
    pub anim_tick_ms: u64,
    /// 面板组呼吸辉光周期 (毫秒, 3.8s 正弦循环); 0 = 描边恒定 (reduced 档)。
    pub glow_period_ms: u64,
    /// 大数字变化闪烁: 新值到达亮度脉冲一帧 (reduced 档恒定亮度)。
    pub digit_flash_enabled: bool,
    /// 治理灯渐入帧数 (新事件灯从暗到亮); 0 = 瞬时点亮 (reduced 档)。
    pub lamp_fade_frames: u64,
}

impl MotionTokens {
    /// 是否有逐帧动画 (reduced 档无动画帧: 只在数据变化时重绘)。
    pub fn animations_enabled(self) -> bool {
        self.anim_tick_ms > 0
    }

    /// 面板组呼吸辉光亮度 (0-15): full 档按 3.8s 正弦循环, reduced 档恒定。
    pub fn breath_level(self, frame: u64) -> u8 {
        if !self.animations_enabled() || self.glow_period_ms == 0 {
            return 12;
        }
        let period_ticks = (self.glow_period_ms / self.anim_tick_ms).max(2);
        let phase = frame % period_ticks;
        let angle = (phase as f64) * std::f64::consts::TAU / (period_ticks as f64);
        let level = 8.0 + 7.0 * angle.sin();
        level.round().clamp(1.0, 15.0) as u8
    }

    /// 面板描边色: 基色 → 发光色按呼吸亮度混合 (reduced 档恒定基色)。
    pub fn breath_border(self, edge: Color, accent: Color, frame: u64) -> Color {
        if !self.animations_enabled() || self.glow_period_ms == 0 {
            return edge;
        }
        blend(edge, accent, self.breath_level(frame))
    }

    /// 治理灯渐入亮度 (0-15): 新灯从暗到亮逐帧渐入; 0 帧渐入 = 瞬时全亮。
    pub fn lamp_level(self, age_frames: u64) -> u8 {
        if self.lamp_fade_frames == 0 {
            return 15;
        }
        let lit = (age_frames.min(self.lamp_fade_frames) as u32 + 1) * 15
            / (self.lamp_fade_frames as u32 + 1);
        lit.min(15) as u8
    }
}

/// 颜色按亮度档混合 (0=全 `from`, 15=全 `to`); 非 RGB 色按中点二选一。
pub fn blend(from: Color, to: Color, level: u8) -> Color {
    let level = level.min(15);
    match (from, to) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => {
            let mix = |a: u8, b: u8| -> u8 {
                let a = u32::from(a);
                let b = u32::from(b);
                let level = u32::from(level);
                ((a * (15 - level) + b * level) / 15) as u8
            };
            Color::Rgb(mix(fr, tr), mix(fg, tg), mix(fb, tb))
        }
        _ => {
            if level >= 8 {
                to
            } else {
                from
            }
        }
    }
}

/// 动画帧预算调度: 只在到点时产出动画帧 (full 档 ≥200ms/帧, ≤5fps 红线;
/// reduced 档 0 帧 —— 静态呈现, 重绘只由数据变化触发)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimClock {
    /// 动画帧间隔 (毫秒); 0 = 无动画帧。
    interval_ms: u64,
    /// 下一个动画帧到点时刻 (毫秒)。
    next_due_ms: u64,
}

impl AnimClock {
    /// 按动效档构建调度 (起点 0: 首帧立即到点)。
    pub fn new(motion: &MotionTokens) -> Self {
        Self {
            interval_ms: motion.anim_tick_ms,
            next_due_ms: 0,
        }
    }

    /// 动画帧间隔 (毫秒); 0 = 无动画帧。
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// 当前时刻是否到动画帧; 到点则顺延下一帧 (跳时不补帧, 防帧风暴)。
    pub fn due(&mut self, now_ms: u64) -> bool {
        if self.interval_ms == 0 {
            return false;
        }
        if now_ms >= self.next_due_ms {
            self.next_due_ms = now_ms + self.interval_ms;
            true
        } else {
            false
        }
    }

    /// 距下一个动画帧的毫秒数; None = 永不到点 (reduced 档)。
    pub fn next_due_in(&self, now_ms: u64) -> Option<u64> {
        if self.interval_ms == 0 {
            None
        } else {
            Some(self.next_due_ms.saturating_sub(now_ms))
        }
    }
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
                anim_tick_ms: 250,
                glow_period_ms: 3_800,
                digit_flash_enabled: true,
                lamp_fade_frames: 4,
            },
            MotionMode::Reduced => MotionTokens {
                scanline_enabled: false,
                glow_pulse_enabled: false,
                digit_tick_enabled: false,
                spinner_enabled: false,
                spinner_frames: &["◆"],
                scanline_stride: 0,
                anim_tick_ms: 0,
                glow_period_ms: 0,
                digit_flash_enabled: false,
                lamp_fade_frames: 0,
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

    /// 动画帧预算: full 档帧间隔落在 200-500ms 且 ≤5fps; reduced 档 0 帧。
    #[test]
    fn animation_tick_budget_caps_frames_at_five_fps() {
        let full = CockpitTheme::new(MotionMode::Full).motion;
        let mut clock = AnimClock::new(&full);
        let interval = clock.interval_ms();
        assert!(
            (200..=500).contains(&interval),
            "动画帧间隔须落在 200-500ms (波形推进/呼吸节奏), 实际 {interval}ms"
        );

        // 一秒窗口内 ≤5 帧 (≤5fps 红线)。
        let mut frames = 0u32;
        for now in (0..=1_000).step_by(10) {
            if clock.due(now as u64) {
                frames += 1;
            }
        }
        assert!(frames <= 5, "一秒内动画帧 {frames} 超出 5fps 预算");

        // 十秒窗口同样 ≤5fps (跳时不补帧)。
        let mut clock = AnimClock::new(&full);
        let mut frames = 0u32;
        for now in (0..=10_000).step_by(100) {
            if clock.due(now as u64) {
                frames += 1;
            }
        }
        assert!(frames <= 50, "十秒内动画帧 {frames} 超出 5fps 预算");

        // reduced 档: 无动画帧, 调度永不到点。
        let reduced = CockpitTheme::new(MotionMode::Reduced).motion;
        let mut frozen = AnimClock::new(&reduced);
        assert_eq!(frozen.interval_ms(), 0);
        for now in (0..=10_000).step_by(100) {
            assert!(!frozen.due(now as u64), "reduced 档不得产出动画帧");
        }
        assert_eq!(frozen.next_due_in(0), None);
    }

    /// 呼吸辉光帧循环: full 档按 3.8s 正弦起伏并回到起点, reduced 档恒定静止。
    #[test]
    fn breathing_glow_cycles_over_period_and_freezes_in_reduced() {
        let full = CockpitTheme::new(MotionMode::Full).motion;
        assert_eq!(full.glow_period_ms, 3_800);
        let period_ticks = full.glow_period_ms / full.anim_tick_ms;
        let mut levels = Vec::new();
        for frame in 0..period_ticks {
            levels.push(full.breath_level(frame));
        }
        let distinct: std::collections::HashSet<u8> = levels.iter().copied().collect();
        assert!(distinct.len() >= 3, "呼吸辉光应有起伏: {levels:?}");
        // 帧循环闭合: 一个周期后回到起点。
        assert_eq!(full.breath_level(period_ticks), full.breath_level(0));
        for level in &levels {
            assert!(*level <= 15, "亮度档越界: {levels:?}");
        }

        let reduced = CockpitTheme::new(MotionMode::Reduced).motion;
        let steady = reduced.breath_level(0);
        for frame in 0..period_ticks {
            assert_eq!(
                reduced.breath_level(frame),
                steady,
                "reduced 档呼吸光须静止"
            );
        }

        // 描边混合: full 档随帧变色, reduced 档恒定基色。
        let edge = Color::Rgb(38, 58, 92);
        let accent = Color::Rgb(64, 224, 255);
        let colors: std::collections::HashSet<Color> = (0..period_ticks)
            .map(|frame| full.breath_border(edge, accent, frame))
            .collect();
        assert!(colors.len() >= 3, "呼吸描边应随帧变色");
        for frame in 0..period_ticks {
            assert_eq!(reduced.breath_border(edge, accent, frame), edge);
        }
    }

    /// 治理灯渐入: full 档新灯从暗到亮逐帧渐入, reduced 档瞬时全亮。
    #[test]
    fn lamp_fade_ramps_in_full_and_is_instant_in_reduced() {
        let full = CockpitTheme::new(MotionMode::Full).motion;
        assert!(full.lamp_fade_frames > 0);
        let mut ramp = Vec::new();
        for age in 0..full.lamp_fade_frames {
            ramp.push(full.lamp_level(age));
        }
        ramp.push(full.lamp_level(full.lamp_fade_frames));
        assert!(
            ramp[0] < *ramp.last().expect("渐入尾帧"),
            "新灯须从暗渐入: {ramp:?}"
        );
        for pair in ramp.windows(2) {
            assert!(pair[0] <= pair[1], "渐入须单调不减: {ramp:?}");
        }
        assert_eq!(*ramp.last().expect("渐入尾帧"), 15);

        let reduced = CockpitTheme::new(MotionMode::Reduced).motion;
        assert_eq!(reduced.lamp_level(0), 15);
        assert_eq!(reduced.lamp_level(3), 15);
    }
}
