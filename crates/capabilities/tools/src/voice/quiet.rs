//! 安静纪律 · 勿扰窗口 (最小自建面) —— 发声面的「不该出声时不出声」。
//!
//! # 与关怀纪律的同源接缝
//!
//! 勿扰窗口与 V3-a 关怀纪律**同源**: `care_rules` 的打扰纪律
//! ([`CareDiscipline`] 字段) 配了勿扰窗口时, 组装根**引用它的窗口**
//! (runtime-assembly 的 `voice_quiet_hours_from_care_rules` 接缝);
//! 未落 / 未配窗口时回落本面的**最小自建 env** ([`VOICE_QUIET_HOURS_ENV`],
//! 默认 `23:00-08:00` 跨午夜, 与主动关怀打扰纪律的默认窗口同口径)。
//!
//! # 口径
//!
//! - **时间口径同源**: 「当日第几分钟」按 **UTC 分钟**取, 与关怀纪律的
//!   [`CareDiscipline::in_quiet_window`] / 主动关怀安静窗口同一时间基准
//!   (同一窗口语义才叫同源); 读数由注入时钟提供 ([`system_quiet_clock`]),
//!   工具面零时区库依赖, 测试钉死;
//! - **fail-closed 朝安静**: env 缺省 = 默认勿扰窗口生效; env 坏值 = 拒绝
//!   解析 (`from_env` 报错, 不静默回退), 只有显式 `off` 才关窗;
//! - **抑制 = 错误即帧**: 窗口内发声请求给正式拒绝帧 (见 [`QuietHours::suppression`]),
//!   不静默吞请求、不假报已说。
//!
//! [`CareDiscipline`]: 语义参照关怀纪律的打扰纪律配置 (窗口起止为当日分钟,
//! 跨午夜 = 起 > 止); 本面不重复实现关怀判定, 只承接它的窗口值。

use std::sync::Arc;

/// 勿扰窗口旋钮 (最小自建 env; `care_rules` 配了窗口时以其为准)。
pub const VOICE_QUIET_HOURS_ENV: &str = "APEIRETH_VOICE_QUIET_HOURS";

/// 默认勿扰窗口 (`23:00-08:00`, 跨午夜; 与主动关怀打扰纪律的默认窗口同口径,
/// 设计裁定 R1⑭)。
pub const VOICE_QUIET_HOURS_DEFAULT: &str = "23:00-08:00";

/// 显式关窗取值 (只有这个词组关闭勿扰; 其余坏值一律拒绝解析)。
pub const VOICE_QUIET_HOURS_OFF: &str = "off";

/// 一天的分钟数 (当日第几分钟的取值域 `[0, 1440)`)。
pub const MINUTES_PER_DAY: u32 = 24 * 60;

/// 「当日第几分钟」读数 (UTC 口径, 0..1440; 与关怀纪律/主动关怀同基准)。
///
/// 由组装根或 [`system_quiet_clock`] 提供, 工具面不自带时区库; 测试钉固定读数。
pub type QuietClock = Arc<dyn Fn() -> u32 + Send + Sync>;

/// 固定读数时钟 (测试钉死 / 宿主显式给定时刻用)。
pub fn fixed_clock(minutes_of_day: u32) -> QuietClock {
    Arc::new(move || minutes_of_day % MINUTES_PER_DAY)
}

/// epoch 毫秒 → 当天分钟 (UTC, 0..1439; 与关怀纪律同一时间基准)。
pub fn utc_minutes_of_day(at_ms: i64) -> u32 {
    let minutes = (at_ms.rem_euclid(86_400_000) / 60_000).max(0) as u32;
    minutes.min(MINUTES_PER_DAY - 1)
}

/// 系统时钟 (UTC 当日分钟; 生产缺省注入)。
pub fn system_quiet_clock() -> QuietClock {
    Arc::new(|| {
        let at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        utc_minutes_of_day(at_ms)
    })
}

/// 一个跨午夜安全的勿扰窗口 (`HH:MM-HH:MM`; `start > end` = 跨午夜)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuietWindow {
    start_min: u32,
    end_min: u32,
}

impl QuietWindow {
    /// 构造窗口 (已校验的当日分钟数; `start == end` 拒绝 —— 零长窗口没有语义)。
    pub fn new(start_min: u32, end_min: u32) -> Result<Self, String> {
        if start_min >= MINUTES_PER_DAY || end_min >= MINUTES_PER_DAY {
            return Err(format!(
                "拒绝配置: 勿扰窗口边界必须落在一天内 (0..{MINUTES_PER_DAY}), got {start_min}-{end_min}"
            ));
        }
        if start_min == end_min {
            return Err(format!(
                "拒绝配置: 勿扰窗口起止相同 ({start_min}-{end_min}) 没有语义 (要么给区间, 要么用 {VOICE_QUIET_HOURS_OFF} 关窗)"
            ));
        }
        Ok(Self { start_min, end_min })
    }

    /// 起点 (当日分钟数)。
    pub const fn start_min(&self) -> u32 {
        self.start_min
    }

    /// 终点 (当日分钟数; 不含)。
    pub const fn end_min(&self) -> u32 {
        self.end_min
    }

    /// `minutes_of_day` 是否落在窗口内 (跨午夜语义: `start > end` 时两端各自成段)。
    pub fn contains(&self, minutes_of_day: u32) -> bool {
        let now = minutes_of_day % MINUTES_PER_DAY;
        if self.start_min < self.end_min {
            self.start_min <= now && now < self.end_min
        } else {
            now >= self.start_min || now < self.end_min
        }
    }

    /// 窗口的人读描述 (`HH:MM-HH:MM`, 拒绝即帧文案与自述共用)。
    pub fn describe(&self) -> String {
        format!(
            "{:02}:{:02}-{:02}:{:02}",
            self.start_min / 60,
            self.start_min % 60,
            self.end_min / 60,
            self.end_min % 60
        )
    }
}

/// 勿扰纪律的生效面: 窗口 + 抑制判定 (关窗时恒不抑制)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuietHours {
    /// `None` = 显式关窗 ([`VOICE_QUIET_HOURS_OFF`])。
    window: Option<QuietWindow>,
}

impl Default for QuietHours {
    /// 缺省 = 默认勿扰窗口生效 (fail-closed 朝安静, 不是关窗)。
    fn default() -> Self {
        Self::parse(VOICE_QUIET_HOURS_DEFAULT).expect("default quiet window parses")
    }
}

impl QuietHours {
    /// 关窗 (显式关闭勿扰, 测试与宿主显式选择用)。
    pub const fn off() -> Self {
        Self { window: None }
    }

    /// 开窗 (给定窗口)。
    pub const fn windowed(window: QuietWindow) -> Self {
        Self {
            window: Some(window),
        }
    }

    /// 解析旋钮值: `HH:MM-HH:MM` 开窗 / `off` 关窗 / 空串取默认窗口。
    ///
    /// 坏值**拒绝解析** (fail-closed: 不静默回退默认, 不猜意图)。
    pub fn parse(spec: &str) -> Result<Self, String> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return Ok(Self::parse(VOICE_QUIET_HOURS_DEFAULT).expect("default window parses"));
        }
        if trimmed.eq_ignore_ascii_case(VOICE_QUIET_HOURS_OFF) {
            return Ok(Self::off());
        }
        let Some((start, end)) = trimmed.split_once('-') else {
            return Err(format!(
                "拒绝配置: 勿扰窗口必须是 HH:MM-HH:MM 或 {VOICE_QUIET_HOURS_OFF}, got {trimmed:?}"
            ));
        };
        Ok(Self::windowed(QuietWindow::new(
            parse_hhmm(start)?,
            parse_hhmm(end)?,
        )?))
    }

    /// 从 env 读旋钮 (缺省 = 默认窗口; 坏值报错, 不静默回退)。
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var(VOICE_QUIET_HOURS_ENV).unwrap_or_default();
        Self::parse(&raw).map_err(|error| format!("{VOICE_QUIET_HOURS_ENV}: {error}"))
    }

    /// 当前生效窗口 (`None` = 关窗)。
    pub const fn window(&self) -> Option<QuietWindow> {
        self.window
    }

    /// `minutes_of_day` 是否被勿扰窗口抑制。
    pub fn suppressed_at(&self, minutes_of_day: u32) -> bool {
        self.window
            .is_some_and(|window| window.contains(minutes_of_day))
    }

    /// 抑制时给拒绝即帧文案; 不抑制给 `None` (窗口内**不静默**: 走正式拒绝帧)。
    pub fn suppression(&self, minutes_of_day: u32) -> Option<String> {
        let window = self.window?;
        window.contains(minutes_of_day).then(|| {
            format!(
                "拒绝发声: 勿扰窗口内 ({}) 抑制出声 (安静纪律, 如实拒不静默)",
                window.describe()
            )
        })
    }
}

/// `HH:MM` → 当日分钟数 (严格: 两位/两位, 越界拒)。
fn parse_hhmm(raw: &str) -> Result<u32, String> {
    let trimmed = raw.trim();
    let Some((hh, mm)) = trimmed.split_once(':') else {
        return Err(format!("拒绝配置: 时刻必须是 HH:MM, got {trimmed:?}"));
    };
    let hour: u32 = hh
        .trim()
        .parse()
        .map_err(|_| format!("拒绝配置: 小时不是数字, got {hh:?}"))?;
    let minute: u32 = mm
        .trim()
        .parse()
        .map_err(|_| format!("拒绝配置: 分钟不是数字, got {mm:?}"))?;
    if hour >= 24 {
        return Err(format!("拒绝配置: 小时越界 (0..24), got {hour}"));
    }
    if minute >= 60 {
        return Err(format!("拒绝配置: 分钟越界 (0..60), got {minute}"));
    }
    Ok(hour * 60 + minute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_parsing_accepts_the_named_shapes_and_refuses_garbage() {
        let windowed = QuietHours::parse("22:00-06:00").expect("default shape parses");
        assert!(windowed.suppressed_at(23 * 60), "23:00 在窗内");
        assert!(windowed.suppressed_at(5 * 60), "05:00 跨午夜在窗内");
        assert!(!windowed.suppressed_at(12 * 60), "12:00 不在窗内");
        assert!(!windowed.suppressed_at(22 * 60 - 1), "差一分不到窗");
        assert!(windowed.suppressed_at(6 * 60 - 1), "终点不含");
        assert!(!windowed.suppressed_at(6 * 60), "终点整点已出窗");

        let day = QuietHours::parse("09:00-18:00").expect("non-crossing window");
        assert!(day.suppressed_at(9 * 60));
        assert!(!day.suppressed_at(18 * 60));

        let off = QuietHours::parse("off").expect("off parses");
        assert!(!off.suppressed_at(23 * 60), "显式关窗恒不抑制");

        assert!(
            QuietHours::parse("25:00-06:00").is_err(),
            "小时越界拒绝解析 (fail-closed 朝安静, 不静默回退)"
        );
        assert!(QuietHours::parse("22:00").is_err(), "缺区间拒绝");
        assert!(QuietHours::parse("22:00-22:00").is_err(), "零长窗口拒绝");
        assert!(QuietHours::parse("ab:cd-06:00").is_err(), "非数字拒绝");
    }

    #[test]
    fn empty_spec_takes_the_default_window() {
        let quiet = QuietHours::parse("").expect("empty = default");
        assert_eq!(
            quiet.window().map(|window| window.describe()),
            Some(VOICE_QUIET_HOURS_DEFAULT.to_string()),
            "缺省取默认勿扰窗口 (fail-closed 朝安静)"
        );
    }

    /// 设计裁定 R1⑭: 勿扰窗口默认 23:00-8:00 (与 care_rules 打扰纪律同源)。
    #[test]
    fn default_window_is_2300_to_0800_per_the_ruling() {
        assert_eq!(
            VOICE_QUIET_HOURS_DEFAULT, "23:00-08:00",
            "勿扰窗口默认口径 = 23:00-8:00"
        );
        let quiet = QuietHours::default();
        assert!(quiet.suppressed_at(23 * 60), "23:00 起进窗");
        assert!(quiet.suppressed_at(3 * 60), "深夜在窗内");
        assert!(quiet.suppressed_at(8 * 60 - 1), "终点不含");
        assert!(!quiet.suppressed_at(8 * 60), "08:00 整点已出窗");
        assert!(!quiet.suppressed_at(12 * 60), "白天不抑制");
    }

    #[test]
    fn suppression_is_an_explicit_refusal_frame_not_silence() {
        let quiet = QuietHours::parse("22:00-06:00").expect("window");
        let refusal = quiet
            .suppression(23 * 60)
            .expect("窗口内必须给拒绝即帧文案");
        assert!(refusal.contains("勿扰窗口"), "{refusal}");
        assert!(refusal.contains("22:00-06:00"), "{refusal}");
        assert!(quiet.suppression(12 * 60).is_none(), "窗口外无抑制");
    }

    #[test]
    fn window_bounds_are_validated() {
        assert!(QuietWindow::new(MINUTES_PER_DAY, 0).is_err(), "起点越界拒");
        assert!(QuietWindow::new(0, MINUTES_PER_DAY).is_err(), "终点越界拒");
        assert!(QuietWindow::new(10, 10).is_err(), "零长拒");
    }

    #[test]
    fn time_basis_is_the_care_discipline_utc_minutes() {
        // 同源接缝的时间基准: 当天分钟按 UTC 取 (与关怀纪律的窗口判定同基准)。
        assert_eq!(utc_minutes_of_day(0), 0);
        assert_eq!(utc_minutes_of_day(3_600_000), 60);
        assert_eq!(utc_minutes_of_day(22 * 3_600_000), 22 * 60);
        assert_eq!(
            utc_minutes_of_day(-1),
            MINUTES_PER_DAY - 1,
            "负时刻按当日回卷"
        );
    }
}
