//! 与宿主脚本引擎一致的数值原语。
//!
//! 生产侧的场景类跑在 JS 运行时里 ⇒ 它的 `Math.min` / `Math.max` /
//! `Math.round` 与 Rust 标准库的同名函数在**边界情形**上并不完全一致
//! （NaN 传播、`±0` 的符号、「半数」向哪边取整）。
//! 本模块把那三个函数的语义逐条钉住 ⇒ 本 crate 算出的取值与生产侧**逐位**相同。
//!
//! 这里的每个函数都对应生产侧源码里的一处调用；本模块**不含**任何场景语义。

/// 与 `Math.min` 一致的取小。
///
/// - 任一实参是 NaN ⇒ NaN（Rust 的 `f64::min` 会跳过 NaN ⇒ 语义不同）；
/// - `±0` 相等时保留 `-0`（与 `Math.min` 的规范行为一致）。
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a < b {
        return a;
    }
    if b < a {
        return b;
    }
    if a == 0.0 && b == 0.0 {
        // 相等（含 ±0）：取小保留负零。
        return if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        };
    }
    a
}

/// 与 `Math.max` 一致的取大。
///
/// - 任一实参是 NaN ⇒ NaN；
/// - `±0` 相等时保留 `+0`。
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a > b {
        return a;
    }
    if b > a {
        return b;
    }
    if a == 0.0 && b == 0.0 {
        // 相等（含 ±0）：取大保留正零。
        return if a.is_sign_positive() || b.is_sign_positive() {
            0.0
        } else {
            -0.0
        };
    }
    a
}

/// 与 `Math.round` 一致的取整（**半数向 +∞**，负半数的零保留负号）。
///
/// - `Math.round(-0.5) = -0`、`Math.round(-0.3) = -0`（Rust 的 `f64::round`
///   是「半数远离零」⇒ `-0.5` 会得 `-1` ⇒ 语义不同）；
/// - `NaN` / `±∞` 原样返回。
pub fn round(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let result = (value + 0.5).floor();
    if result == 0.0 && value < 0.0 {
        // 负数落在 (-1, 0) ⇒ 结果是负零。
        return -0.0;
    }
    result
}

/// 把任意值夹进 `[0, 1]`（与生产侧 `Math.min(1, Math.max(0, v))` 同序）。
pub fn clamp01(value: f64) -> f64 {
    min(1.0, max(0.0, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_propagates_nan_like_math_min() {
        assert!(min(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN).is_nan());
        assert!(min(f64::NAN, f64::NAN).is_nan());
        assert_eq!(min(1.0, 2.0), 1.0);
        assert_eq!(min(2.0, 1.0), 1.0);
    }

    #[test]
    fn max_propagates_nan_like_math_max() {
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(max(1.0, f64::NAN).is_nan());
        assert_eq!(max(1.0, 2.0), 2.0);
        assert_eq!(max(2.0, 1.0), 2.0);
    }

    #[test]
    fn signed_zero_follows_math_semantics() {
        assert!(min(-0.0, 0.0).is_sign_negative());
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert!(max(0.0, -0.0).is_sign_positive());
    }

    #[test]
    fn round_matches_math_round_on_half_values() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(2.4), 2.0);
        assert_eq!(round(-2.6), -3.0);
        assert_eq!(round(0.5), 1.0);
        assert_eq!(round(-0.5), 0.0);
        assert!(round(-0.5).is_sign_negative(), "Math.round(-0.5) 是 -0");
        assert!(round(-0.3).is_sign_negative(), "Math.round(-0.3) 是 -0");
        assert_eq!(round(0.0), 0.0);
    }

    #[test]
    fn round_leaves_non_finite_values_alone() {
        assert!(round(f64::NAN).is_nan());
        assert_eq!(round(f64::INFINITY), f64::INFINITY);
        assert_eq!(round(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn clamp01_bounds_values() {
        assert_eq!(clamp01(-1.0), 0.0);
        assert_eq!(clamp01(2.0), 1.0);
        assert_eq!(clamp01(0.25), 0.25);
        assert!(clamp01(f64::NAN).is_nan());
    }
}
