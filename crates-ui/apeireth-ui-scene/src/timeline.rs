//! 舰内时间线照明（纯函数；四个时间线取值）。
//!
//! 四个取值与四个着色器取值一一对应：
//!
//! | 取值 | 含义 |
//! |---|---|
//! | `amb` | 行照环境蓝光强度 `0..1` |
//! | `eclipse` | 凌日日食 `0..1` |
//! | `bright` | 时间线亮度增益（乘性） |
//! | `gold` | 时间线金色占比 `0..1` |
//!
//! 曲线：
//!
//! - 黎明上升沿 04:30–08:00（3.5h，smoothstep）；
//! - 黄昏下降沿 14:00–18:00（4h，smoothstep，与黎明对称）；
//! - 凌日窗 12:00±20min（`ec = 1 - sstep(0.20, 0.3334, |h - 12|)`）；
//! - 调制量 `bright = 1 + 0.22·amb + 0.15·ec`；`gold = 1 - 0.55·amb`。

use crate::jsnum;

/// 黎明上升沿（小时）。
pub const DAWN_START_H: f64 = 4.5;
/// 黎明上升沿终点（小时）。
pub const DAWN_END_H: f64 = 8.0;
/// 黄昏下降沿起点（小时）。
pub const DUSK_START_H: f64 = 14.0;
/// 黄昏下降沿终点（小时）。
pub const DUSK_END_H: f64 = 18.0;
/// 凌日窗中心（小时）。
pub const ECLIPSE_CENTER_H: f64 = 12.0;
/// 凌日窗软化起点（距中心的绝对小时差）。
pub const ECLIPSE_SOFT_START_H: f64 = 0.2;
/// 凌日窗软化终点（距中心的绝对小时差）。
pub const ECLIPSE_SOFT_END_H: f64 = 0.3334;
/// 亮度增益里的行照系数。
pub const BRIGHT_AMB_GAIN: f64 = 0.22;
/// 亮度增益里的凌日系数。
pub const BRIGHT_ECLIPSE_GAIN: f64 = 0.15;
/// 金色占比里的行照损失系数。
pub const GOLD_AMB_LOSS: f64 = 0.55;

/// 时间线照明参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineParams {
    /// 行照环境蓝光强度 `0..1`。
    pub amb: f64,
    /// 凌日日食 `0..1`。
    pub eclipse: f64,
    /// 时间线亮度增益（乘性）。
    pub bright: f64,
    /// 时间线金色占比 `0..1`。
    pub gold: f64,
}

/// 把任意时刻折回 `[0, 24)`。
pub fn normalize_hour(hour: f64) -> f64 {
    ((hour % 24.0) + 24.0) % 24.0
}

/// smoothstep（两端夹紧）。
pub fn sstep(a: f64, b: f64, x: f64) -> f64 {
    let t = jsnum::min(1.0, jsnum::max(0.0, (x - a) / (b - a)));
    t * t * (3.0 - 2.0 * t)
}

/// 由舰内时刻算出四个时间线取值。
pub fn timeline_params(hour: f64) -> TimelineParams {
    let h = normalize_hour(hour);
    let amb = sstep(DAWN_START_H, DAWN_END_H, h) * (1.0 - sstep(DUSK_START_H, DUSK_END_H, h));
    let eclipse = 1.0
        - sstep(
            ECLIPSE_SOFT_START_H,
            ECLIPSE_SOFT_END_H,
            (h - ECLIPSE_CENTER_H).abs(),
        );
    TimelineParams {
        amb,
        eclipse,
        bright: 1.0 + BRIGHT_AMB_GAIN * amb + BRIGHT_ECLIPSE_GAIN * eclipse,
        gold: 1.0 - GOLD_AMB_LOSS * amb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hour_folds_into_the_day() {
        assert_eq!(normalize_hour(0.0), 0.0);
        assert_eq!(normalize_hour(12.5), 12.5);
        assert_eq!(normalize_hour(24.0), 0.0);
        assert_eq!(normalize_hour(-1.0), 23.0);
        assert_eq!(normalize_hour(25.0), 1.0);
        assert_eq!(normalize_hour(-25.0), 23.0);
    }

    #[test]
    fn smoothstep_clamps_outside_the_ends() {
        assert_eq!(sstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(sstep(0.0, 1.0, 2.0), 1.0);
        assert_eq!(sstep(0.0, 1.0, 0.5), 0.5);
        assert_eq!(sstep(4.5, 8.0, 4.5), 0.0);
        assert_eq!(sstep(4.5, 8.0, 8.0), 1.0);
    }

    #[test]
    fn night_hours_have_no_ambient_and_no_eclipse() {
        for hour in [0.0, 2.0, 4.0, 20.0, 23.0] {
            let params = timeline_params(hour);
            assert_eq!(params.amb, 0.0, "{hour} 时不该有行照");
            assert_eq!(params.eclipse, 0.0, "{hour} 时不该有凌日");
            assert_eq!(params.bright, 1.0);
            assert_eq!(params.gold, 1.0);
        }
    }

    #[test]
    fn full_day_ambient_peaks_at_noon() {
        let noon = timeline_params(12.0);
        assert_eq!(noon.amb, 1.0, "正午行照强度为满");
        assert_eq!(noon.eclipse, 1.0, "正午在凌日窗中心");
        assert!((noon.bright - (1.0 + 0.22 + 0.15)).abs() < 1e-12);
        assert!((noon.gold - 0.45).abs() < 1e-12);
    }

    #[test]
    fn eclipse_window_falls_off_outside_the_soft_band() {
        let center = timeline_params(12.0);
        let edge = timeline_params(12.0 + 0.2);
        let outside = timeline_params(12.0 + 0.3334);
        assert_eq!(center.eclipse, 1.0);
        assert_eq!(edge.eclipse, 1.0, "软化起点处仍是满值");
        assert!(outside.eclipse < 1e-12, "软化终点外必须退出");
    }

    #[test]
    fn dusk_edge_falls_between_fourteen_and_eighteen() {
        assert_eq!(timeline_params(13.9).amb, 1.0);
        assert_eq!(timeline_params(18.0).amb, 0.0);
        let mid = timeline_params(16.0);
        assert!(
            mid.amb > 0.0 && mid.amb < 1.0,
            "黄昏是下降沿 ⇒ 中点是中间值"
        );
    }

    #[test]
    fn dawn_edge_rises_between_four_thirty_and_eight() {
        assert_eq!(timeline_params(4.4).amb, 0.0);
        assert_eq!(timeline_params(8.0).amb, 1.0);
        let mid = timeline_params(6.25);
        assert!((mid.amb - 0.5).abs() < 1e-12, "上升沿中点为 0.5");
    }

    #[test]
    fn gold_falls_with_ambient_bright_rises() {
        let night = timeline_params(2.0);
        let day = timeline_params(9.0);
        assert!(day.gold < night.gold, "行照时金色占比降低");
        assert!(day.bright > night.bright, "行照时亮度增益抬高");
    }

    #[test]
    fn parameters_are_invariant_under_day_folding() {
        for hour in [-3.0, 21.0, 45.0] {
            let folded = timeline_params(hour);
            let direct = timeline_params(normalize_hour(hour));
            assert_eq!(folded, direct);
        }
    }
}
