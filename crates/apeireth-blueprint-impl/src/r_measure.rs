//! # R-1 ~ R-5 — 5 R-Measure 实装
//!
//! 5 R-Measure 源自 RIVAL 蓝图 §2.4 + R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063):
//!
//! | 指标 | 含义 | 函数 | 范围 |
//! |------|------|------|------|
//! | **R-1** | 直行率 (no detour) | [`r1_directness`] | [0.0, 1.0] |
//! | **R-2** | 直说率 (no equivocation) | [`r2_candor`] | [0.0, 1.0] |
//! | **R-3** | 闭环率 (closed loop) | [`r3_closure`] | [0.0, 1.0] |
//! | **R-4** | 守门率 (8 项承诺) | [`r4_promise`] | [0.0, 1.0] |
//! | **R-5** | 失败诚实率 (O-5 不假装) | [`r5_failure_honesty`] | [0.0, 1.0] |
//!
//! 所有 5 函数都是 `pub fn` (无 trait), 输入是 `&[ActionSample]`, 输出 `f64` 比例.
//!
//! ## 6 哲学锚穿透
//!
//! - S-1 主 22:33 — R-Measure 服务 ASI 北极星量化, 不装饰.
//! - S-2 主 17:43 — 5 函数全部实装, 无 TODO 占位.
//! - O-5 主 17:58 — R-5 失败诚实率 = 核心 (R11 baseline 不假装).
//! - O-2 主 19:33 — 借鉴 v0.9.21 商业版 R-Measure 5 维 (1:1 翻译).
//! - O-3 主 23:44 — 5 函数一次写齐 + 8-10 测试.
//! - O-4 主 00:56 — 5 函数签名一致, 接手者能照葫芦画瓢加 R-6/R-7.
//!
//! ## 8 项不修改承诺
//!
//! 1. 5 函数签名不变 (业务方依赖).
//! 2. 输入都是 `&[ActionSample]`, 不假设 caller 持有锁.
//! 3. 输出范围 [0.0, 1.0] (空样本 → 0.0, 不 NaN).
//! 4. R-4 8 项承诺清单不修改 (跟 RIVAL §2.4 对齐).
//! 5. 不假设 ActionSample 来自特定 crate — 通用数据结构.
//! 6. 不依赖任何外部 crate (除 serde).
//! 7. 5 函数都做空样本保护 (空切片 → 0.0).
//! 8. 不假装 baseline — 实测时算 (R11 baseline 三值 LOCKED 单独维护).

use crate::error::BlueprintResult;
use serde::{Deserialize, Serialize};

/// 5 R-Measure 共用输入 — 一个 action 样本.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionSample {
    /// 动作是否直行 (无 detour / 无重复 / 无绕弯)
    pub direct: bool,
    /// 表达是否直说 (无 equivocation / 无 hedge / 无 "可能 / 也许 / 大概")
    pub candid: bool,
    /// 闭环 (有始有终: 起始 → 中间 → 完成, 跟下一个 action 链接)
    pub closed_loop: bool,
    /// 8 项承诺守门通过 (8 个 bool 字段全 true)
    pub promises: [bool; 8],
    /// 失败时是否诚实报告 (true = 诚实; false = 假装成功)
    pub failure_honest: bool,
}

impl ActionSample {
    /// 构造一个完美样本 (5 维全 true)
    pub fn perfect() -> Self {
        Self {
            direct: true,
            candid: true,
            closed_loop: true,
            promises: [true; 8],
            failure_honest: true,
        }
    }

    /// 构造一个最差样本 (5 维全 false)
    pub fn worst() -> Self {
        Self {
            direct: false,
            candid: false,
            closed_loop: false,
            promises: [false; 8],
            failure_honest: false,
        }
    }
}

// ============================================
// R-1 直行率
// ============================================

/// R-1 直行率 — `direct=true` 样本占比.
///
/// 期望: 高 (>0.9). R11 baseline V1136 = 0.9063.
pub fn r1_directness(samples: &[ActionSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n_direct = samples.iter().filter(|s| s.direct).count();
    n_direct as f64 / samples.len() as f64
}

// ============================================
// R-2 直说率
// ============================================

/// R-2 直说率 — `candid=true` 样本占比.
///
/// 期望: 高 (>0.85). R11 baseline V1131 = 0.8532.
pub fn r2_candor(samples: &[ActionSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n_candid = samples.iter().filter(|s| s.candid).count();
    n_candid as f64 / samples.len() as f64
}

// ============================================
// R-3 闭环率
// ============================================

/// R-3 闭环率 — `closed_loop=true` 样本占比.
///
/// 期望: 高 (>0.85).
pub fn r3_closure(samples: &[ActionSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n_closed = samples.iter().filter(|s| s.closed_loop).count();
    n_closed as f64 / samples.len() as f64
}

// ============================================
// R-4 守门率 (8 项承诺)
// ============================================

/// R-4 守门率 — 8 项承诺全通过的样本占比.
///
/// 8 项承诺 (跟 RIVAL §2.4 对齐):
/// 1. 不假装已实现
/// 2. 编译期 hardcode
/// 3. 不改 LOCKED
/// 4. 8 项不修改承诺
/// 5. 失败时 Err
/// 6. 不静默 fallback
/// 7. 留 R-Measure 入口
/// 8. 任何人都能接手
///
/// 期望: 高 (>0.86). R11 baseline V1141 = 0.8682.
pub fn r4_promise(samples: &[ActionSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n_promises_kept = samples
        .iter()
        .filter(|s| s.promises.iter().all(|&p| p))
        .count();
    n_promises_kept as f64 / samples.len() as f64
}

// ============================================
// R-5 失败诚实率 (O-5 不假装)
// ============================================

/// R-5 失败诚实率 — `failure_honest=true` 样本占比.
///
/// **核心指标** (O-5 主 17:58 不假装).
/// 期望: 100% (1.0). 任何 < 1.0 都说明系统在假装.
pub fn r5_failure_honesty(samples: &[ActionSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n_honest = samples.iter().filter(|s| s.failure_honest).count();
    n_honest as f64 / samples.len() as f64
}

// ============================================
// R-Measure 5 维打包 + 校验
// ============================================

/// 5 维 R-Measure 打包.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RMeasureAll {
    pub r1_directness: f64,
    pub r2_candor: f64,
    pub r3_closure: f64,
    pub r4_promise: f64,
    pub r5_failure_honesty: f64,
}

impl RMeasureAll {
    /// 5 维算全
    pub fn from_samples(samples: &[ActionSample]) -> Self {
        Self {
            r1_directness: r1_directness(samples),
            r2_candor: r2_candor(samples),
            r3_closure: r3_closure(samples),
            r4_promise: r4_promise(samples),
            r5_failure_honesty: r5_failure_honesty(samples),
        }
    }

    /// 平均分 (5 维等权)
    pub fn average(&self) -> f64 {
        (self.r1_directness
            + self.r2_candor
            + self.r3_closure
            + self.r4_promise
            + self.r5_failure_honesty)
            / 5.0
    }

    /// 校验范围 [0.0, 1.0]
    pub fn validate(&self) -> BlueprintResult<()> {
        use crate::error::BlueprintError;
        for (name, val) in [
            ("r1_directness", self.r1_directness),
            ("r2_candor", self.r2_candor),
            ("r3_closure", self.r3_closure),
            ("r4_promise", self.r4_promise),
            ("r5_failure_honesty", self.r5_failure_honesty),
        ] {
            if !(0.0..=1.0).contains(&val) || val.is_nan() {
                return Err(BlueprintError::QMetricOutOfRange {
                    metric: name.into(),
                    value: val,
                });
            }
        }
        Ok(())
    }

    /// 跟 baseline 对比 (返回 (维度, 偏差)). baseline 三值:
    /// V1141-R11 = 0.8682 (R-4 对应)
    /// V1131-R11 = 0.8532 (R-2 对应)
    /// V1136-R11 = 0.9063 (R-1 对应)
    pub fn drift(&self) -> RMeasureDrift {
        RMeasureDrift {
            r1: self.r1_directness - 0.9063,
            r2: self.r2_candor - 0.8532,
            r3: self.r3_closure,  // R-3 无 baseline LOCKED (留 0)
            r4: self.r4_promise - 0.8682,
            r5: self.r5_failure_honesty,  // R-5 期望 1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RMeasureDrift {
    pub r1: f64,
    pub r2: f64,
    pub r3: f64,
    pub r4: f64,
    pub r5: f64,
}

impl RMeasureDrift {
    /// 是否全部 ≥ baseline (drift >= 0)
    pub fn all_meet_baseline(&self) -> bool {
        self.r1 >= 0.0 && self.r2 >= 0.0 && self.r4 >= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perfect_n(n: usize) -> Vec<ActionSample> {
        (0..n).map(|_| ActionSample::perfect()).collect()
    }

    fn mixed_n(n: usize, ratio_perfect: f64) -> Vec<ActionSample> {
        let n_perfect = (n as f64 * ratio_perfect).round() as usize;
        (0..n)
            .map(|i| {
                if i < n_perfect {
                    ActionSample::perfect()
                } else {
                    ActionSample::worst()
                }
            })
            .collect()
    }

    // --- R-1 ---
    #[test]
    fn r1_empty_returns_zero() {
        assert_eq!(r1_directness(&[]), 0.0);
    }

    #[test]
    fn r1_all_direct_is_one() {
        assert_eq!(r1_directness(&perfect_n(10)), 1.0);
    }

    #[test]
    fn r1_no_direct_is_zero() {
        let samples: Vec<_> = (0..5).map(|_| ActionSample::worst()).collect();
        assert_eq!(r1_directness(&samples), 0.0);
    }

    #[test]
    fn r1_half_is_half() {
        assert_eq!(r1_directness(&mixed_n(10, 0.5)), 0.5);
    }

    #[test]
    fn r1_above_r11_baseline() {
        // V1136 = 0.9063 (R-1 baseline)
        // 构造 91% direct 样本 (超 baseline)
        let samples = mixed_n(100, 0.91);
        let r = r1_directness(&samples);
        assert!(r > 0.9063, "r1={r} should exceed baseline 0.9063");
    }

    // --- R-2 ---
    #[test]
    fn r2_empty_returns_zero() {
        assert_eq!(r2_candor(&[]), 0.0);
    }

    #[test]
    fn r2_perfect_is_one() {
        assert_eq!(r2_candor(&perfect_n(20)), 1.0);
    }

    #[test]
    fn r2_above_r11_baseline() {
        // V1131 = 0.8532 (R-2 baseline)
        let samples = mixed_n(100, 0.86);
        let r = r2_candor(&samples);
        assert!(r > 0.8532);
    }

    // --- R-3 ---
    #[test]
    fn r3_empty_returns_zero() {
        assert_eq!(r3_closure(&[]), 0.0);
    }

    #[test]
    fn r3_perfect_is_one() {
        assert_eq!(r3_closure(&perfect_n(7)), 1.0);
    }

    // --- R-4 ---
    #[test]
    fn r4_empty_returns_zero() {
        assert_eq!(r4_promise(&[]), 0.0);
    }

    #[test]
    fn r4_perfect_is_one() {
        assert_eq!(r4_promise(&perfect_n(3)), 1.0);
    }

    #[test]
    fn r4_above_r11_baseline() {
        // V1141 = 0.8682 (R-4 baseline)
        let samples = mixed_n(100, 0.87);
        let r = r4_promise(&samples);
        assert!(r > 0.8682);
    }

    #[test]
    fn r4_one_broken_promise_drops_to_zero() {
        // 一个 promise = false → 整个样本 fail
        let mut s = ActionSample::perfect();
        s.promises[3] = false;
        let samples = vec![ActionSample::perfect(), s, ActionSample::perfect()];
        assert_eq!(r4_promise(&samples), 2.0 / 3.0);
    }

    // --- R-5 ---
    #[test]
    fn r5_empty_returns_zero() {
        assert_eq!(r5_failure_honesty(&[]), 0.0);
    }

    #[test]
    fn r5_perfect_is_one() {
        assert_eq!(r5_failure_honesty(&perfect_n(5)), 1.0);
    }

    #[test]
    fn r5_one_dishonest_drops_to_partial() {
        let samples = vec![
            ActionSample::perfect(),
            ActionSample { failure_honest: false, ..ActionSample::perfect() },
            ActionSample::perfect(),
        ];
        assert_eq!(r5_failure_honesty(&samples), 2.0 / 3.0);
    }

    // --- RMeasureAll ---
    #[test]
    fn rmeasure_all_from_samples_aggregates() {
        let all = RMeasureAll::from_samples(&perfect_n(10));
        assert_eq!(all.r1_directness, 1.0);
        assert_eq!(all.r2_candor, 1.0);
        assert_eq!(all.r3_closure, 1.0);
        assert_eq!(all.r4_promise, 1.0);
        assert_eq!(all.r5_failure_honesty, 1.0);
    }

    #[test]
    fn rmeasure_all_average() {
        let all = RMeasureAll {
            r1_directness: 1.0,
            r2_candor: 0.8,
            r3_closure: 0.6,
            r4_promise: 0.4,
            r5_failure_honesty: 0.2,
        };
        assert!((all.average() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn rmeasure_all_validate_rejects_out_of_range() {
        let bad = RMeasureAll {
            r1_directness: 1.5,
            r2_candor: 0.5,
            r3_closure: 0.5,
            r4_promise: 0.5,
            r5_failure_honesty: 0.5,
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn rmeasure_all_drift_meets_baseline() {
        let all = RMeasureAll {
            r1_directness: 0.95,    // > 0.9063
            r2_candor: 0.90,        // > 0.8532
            r3_closure: 0.85,       // R-3 无 baseline
            r4_promise: 0.90,       // > 0.8682
            r5_failure_honesty: 1.0,
        };
        let drift = all.drift();
        assert!(drift.all_meet_baseline());
        assert!(drift.r1 > 0.0);
        assert!(drift.r2 > 0.0);
        assert!(drift.r4 > 0.0);
    }

    #[test]
    fn rmeasure_all_drift_below_baseline() {
        let all = RMeasureAll {
            r1_directness: 0.5,    // < 0.9063
            r2_candor: 0.5,
            r3_closure: 0.5,
            r4_promise: 0.5,
            r5_failure_honesty: 0.5,
        };
        let drift = all.drift();
        assert!(!drift.all_meet_baseline());
    }
}
