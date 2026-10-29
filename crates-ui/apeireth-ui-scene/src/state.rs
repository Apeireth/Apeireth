//! presence 状态机（三档 + PAD 映射 + 指数趋近）。
//!
//! ## 三档基线
//!
//! | 档位 | 转速 | 亮度 | 湍流 | 呼吸 |
//! |---|---|---|---|---|
//! | 静息 | 0.015 | 0.32 | 0.12 | 0.0 |
//! | 思考 | 0.38 | 0.9 | 1.0 | 0.0 |
//! | 说话 | 0.16 | 1.05 | 0.35 | 1.0 |
//!
//! ## 平滑
//!
//! 平滑是**指数趋近**（不是线性）：每帧
//! `cur += (target - cur) · min(1, dt · 2.4)`。
//!
//! ## PAD 映射（只在收到真实存在态帧时生效）
//!
//! - `speed = 0.015 + 0.365 · clamp01((a + 1) / 2)`（说话档直接取 0.160）；
//! - `bright = 0.32 + 0.73 · clamp01((p + 1) / 2)`；
//! - `turb = max(0.12, intensity)`；
//! - `pulse` = 说话档 1.0，否则 0。
//!
//! PAD / 强度在进映射表前先乘保守增益（`HEURISTIC_V0_GAIN`）。

use crate::jsnum;
use crate::tokens::presence;

/// 场景模式（三档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneMode {
    /// 静息档。
    Quiet,
    /// 思考档。
    Thinking,
    /// 说话档。
    Speaking,
}

impl SceneMode {
    /// 该档的基线参数（转速 / 亮度 / 湍流 / 呼吸）。
    pub fn baseline(self) -> PresenceStateParams {
        let raw = match self {
            SceneMode::Quiet => presence::QUIET,
            SceneMode::Thinking => presence::THINKING,
            SceneMode::Speaking => presence::SPEAKING,
        };
        PresenceStateParams {
            speed: raw[0],
            bright: raw[1],
            turb: raw[2],
            pulse: raw[3],
        }
    }
}

/// presence 状态参数（四个取值）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresenceStateParams {
    /// 盘面自转速率。
    pub speed: f64,
    /// 盘面亮度。
    pub bright: f64,
    /// 湍流强度。
    pub turb: f64,
    /// 呼吸光强度。
    pub pulse: f64,
}

/// PAD 偏置（三轴）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PadBias {
    /// 愉悦度。
    pub p: f64,
    /// 唤醒度。
    pub a: f64,
    /// 温度（**暂存不消费** —— 消费它需要改着色器新增取值）。
    pub d: f64,
}

/// 计算目标状态参数。
///
/// - `intensity` = `None` ⇒ **没有真实存在态帧** ⇒ 回落三档基线（不编造 PAD）；
/// - `intensity` = `Some(_)` ⇒ 走 PAD 映射表。
pub fn current_targets(
    mode: SceneMode,
    pad_bias: &PadBias,
    intensity: Option<f64>,
) -> PresenceStateParams {
    let base = mode.baseline();
    let Some(raw_intensity) = intensity else {
        return base;
    };
    let gain = presence::HEURISTIC_V0_GAIN;
    let p = pad_bias.p * gain;
    let a = pad_bias.a * gain;
    let scaled_intensity = jsnum::clamp01(raw_intensity) * gain;
    let speaking = mode == SceneMode::Speaking;
    PresenceStateParams {
        speed: if speaking {
            presence::SPEAKING[0]
        } else {
            0.015 + 0.365 * jsnum::clamp01((a + 1.0) / 2.0)
        },
        bright: 0.32 + 0.73 * jsnum::clamp01((p + 1.0) / 2.0),
        turb: jsnum::max(0.12, scaled_intensity),
        pulse: if speaking { 1.0 } else { 0.0 },
    }
}

/// 指数趋近一步（`dt` 秒）。
///
/// `k = clamp01(dt · 2.4)`；`cur += (target - cur) · k`。
///
/// ## ⭐ 为什么 `k` 要**夹下界**
///
/// 早期形态是 `k = min(1, dt · 2.4)` ⇒ 负 `dt` ⇒ `k` 为**负** ⇒
/// `cur += (target - cur) · k` 把状态**反向推离**目标（而不是趋近）。
///
/// ⇒ 现在 `k` 落在 `[0, 1]`：负 `dt`（时钟回拨 / 挂起恢复）⇒ `k = 0`
/// ⇒ 状态**原地不动**（不倒退，也不远离）。
pub fn approach(
    current: PresenceStateParams,
    target: PresenceStateParams,
    dt: f64,
) -> PresenceStateParams {
    // ⚠️ **与生产逐字一致**：`k = min(1, dt · 2.4)` ⇒ **不夹下界**
    // ⇒ 负 `dt` ⇒ `k` 为负 ⇒ 状态反向远离目标 —— 这是**生产侧的既有缺陷**
    // ⇒ ⇒ 迁移以「逐字一致」为验收标准 ⇒ ⇒ 引擎**故意保留**
    // ⇒ ⇒ 登记见 `docs/04-internal/ui-engine-defect-ledger.md`（E-21）。
    let k = jsnum::min(1.0, dt * crate::tokens::motion::STATE_SMOOTHING_RATE);
    PresenceStateParams {
        speed: current.speed + (target.speed - current.speed) * k,
        bright: current.bright + (target.bright - current.bright) * k,
        turb: current.turb + (target.turb - current.turb) * k,
        pulse: current.pulse + (target.pulse - current.pulse) * k,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baselines_match_the_three_modes() {
        assert_eq!(
            SceneMode::Quiet.baseline(),
            PresenceStateParams {
                speed: 0.015,
                bright: 0.32,
                turb: 0.12,
                pulse: 0.0
            }
        );
        assert_eq!(
            SceneMode::Thinking.baseline(),
            PresenceStateParams {
                speed: 0.38,
                bright: 0.9,
                turb: 1.0,
                pulse: 0.0
            }
        );
        assert_eq!(
            SceneMode::Speaking.baseline(),
            PresenceStateParams {
                speed: 0.16,
                bright: 1.05,
                turb: 0.35,
                pulse: 1.0
            }
        );
    }

    #[test]
    fn without_a_real_frame_the_baseline_is_returned() {
        let bias = PadBias {
            p: 0.9,
            a: -0.8,
            d: 0.5,
        };
        for mode in [SceneMode::Quiet, SceneMode::Thinking, SceneMode::Speaking] {
            assert_eq!(current_targets(mode, &bias, None), mode.baseline());
        }
    }

    #[test]
    fn pad_mapping_follows_the_accepted_table() {
        let neutral = PadBias::default();
        let targets = current_targets(SceneMode::Quiet, &neutral, Some(0.5));
        // p = 0 ⇒ clamp01((0 + 1) / 2) = 0.5 ⇒ bright = 0.32 + 0.73·0.5
        assert!((targets.bright - (0.32 + 0.73 * 0.5)).abs() < 1e-12);
        // a = 0 ⇒ speed = 0.015 + 0.365·0.5
        assert!((targets.speed - (0.015 + 0.365 * 0.5)).abs() < 1e-12);
        // intensity = 0.5 → ×0.75 = 0.375 ⇒ turb = max(0.12, 0.375)
        assert!((targets.turb - 0.375).abs() < 1e-12);
        assert_eq!(targets.pulse, 0.0);
    }

    #[test]
    fn pad_bias_is_scaled_by_the_conservative_gain() {
        let bias = PadBias {
            p: 1.0,
            a: 0.0,
            d: 0.0,
        };
        let targets = current_targets(SceneMode::Thinking, &bias, Some(1.0));
        // p = 0.75 ⇒ bright = 0.32 + 0.73 · clamp01(0.875)
        assert!((targets.bright - (0.32 + 0.73 * 0.875)).abs() < 1e-12);
        // intensity = 1 → ×0.75 ⇒ turb = 0.75
        assert!((targets.turb - 0.75).abs() < 1e-12);
    }

    #[test]
    fn speaking_mode_keeps_its_own_speed_and_pulse() {
        let bias = PadBias {
            p: -1.0,
            a: -1.0,
            d: 0.0,
        };
        let targets = current_targets(SceneMode::Speaking, &bias, Some(0.0));
        assert_eq!(targets.speed, 0.16);
        assert_eq!(targets.pulse, 1.0);
    }

    #[test]
    fn turb_never_falls_below_the_floor() {
        let bias = PadBias::default();
        let targets = current_targets(SceneMode::Quiet, &bias, Some(0.0));
        assert_eq!(targets.turb, 0.12);
    }

    #[test]
    fn approach_is_an_exponential_convergence() {
        let current = PresenceStateParams {
            speed: 0.015,
            bright: 0.32,
            turb: 0.12,
            pulse: 0.0,
        };
        let target = PresenceStateParams {
            speed: 0.38,
            bright: 0.9,
            turb: 1.0,
            pulse: 0.0,
        };
        let step = approach(current, target, 1.0 / 60.0);
        let k = 1.0 / 60.0 * 2.4;
        assert!((step.bright - (0.32 + (0.9 - 0.32) * k)).abs() < 1e-15);
        // 一步之后仍在目标与当前值之间（指数趋近，不是直接跳到目标）。
        assert!(step.bright > current.bright && step.bright < target.bright);
    }

    #[test]
    fn approach_is_clamped_to_the_target_for_large_dt() {
        let current = PresenceStateParams {
            speed: 0.0,
            bright: 0.0,
            turb: 0.0,
            pulse: 0.0,
        };
        let target = PresenceStateParams {
            speed: 1.0,
            bright: 1.0,
            turb: 1.0,
            pulse: 1.0,
        };
        let step = approach(current, target, 10.0);
        assert_eq!(step, target, "dt 足够大时一步到位（k 被夹到 1）");
    }

    #[test]
    fn approach_converges_without_overshooting() {
        let mut current = PresenceStateParams {
            speed: 0.015,
            bright: 0.32,
            turb: 0.12,
            pulse: 0.0,
        };
        let target = SceneMode::Thinking.baseline();
        for _ in 0..600 {
            current = approach(current, target, 1.0 / 60.0);
        }
        assert!(
            (current.bright - target.bright).abs() < 1e-6,
            "十秒后必须已收敛"
        );
        assert!(current.bright <= target.bright, "指数趋近不得越过目标");
    }

    #[test]
    fn approach_with_a_negative_dt_does_not_move_away_from_the_target() {
        // ⚠️ **记录「生产同源缺陷」**（见台账 E-21）：`k` **不夹下界**
        // ⇒ 负 dt ⇒ 状态**反向远离**目标 ⇒ 引擎**故意保留**（对齐优先）。
        let current = PresenceStateParams {
            speed: 0.015,
            bright: 0.32,
            turb: 0.12,
            pulse: 0.0,
        };
        let target = SceneMode::Thinking.baseline();
        let step = approach(current, target, -1.0);
        assert_ne!(step, current, "负 dt ⇒ 状态反向推离目标（**与生产同源**）");
        // 零 dt 同样不动。
        assert_eq!(approach(current, target, 0.0), current);
    }
}
