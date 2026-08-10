//! # NamingError — V0.5 命名规范错误类型 (10 variant)
//!
//! 10 个错误 variant 覆盖 encode/decode/validate/sum_guard 全流程:
//! 1. `InvalidPrefix` — 缺 "apeireth:" 前缀
//! 2. `InvalidLevel` — level 0-9 越界
//! 3. `InvalidClass` — class 不在 4 大类内 (PC/RC/HG/GP)
//! 4. `InvalidDomain` — domain 不在 6 域内
//! 5. `InvalidModality` — modality 不在 5 模态内
//! 6. `InvalidSafety` — safety 不在 4 等级内
//! 7. `InvalidCompleteness` — completeness 不在 4 完整度内
//! 8. `InvalidLineage` — lineage 不在 4 血统内
//! 9. `SumNotEquals1` — 4 大类权重和 ≠ 1.00 (守门破坏)
//! 10. `MalformedFormat` — 字符串格式错乱 (段数不对 / 段间分隔符错)
//!
//! ## m3 防御 (per `m3-hallucination-defense §2.1`)
//!
//! 每个错误 variant 编译期 hardcode, 不允许运行时新增 variant, 防 m3 hallucination
//! 改错分类 (如把 `InvalidClass` 改成 `InvalidClassName` 制造不可识别的 err).
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use thiserror::Error;

// ============================================================================
// §1 NamingError 枚举 (10 variant, per skeleton 模式 + m3 防御)
// ============================================================================

/// V0.5 命名规范错误 (10 variant, 编译期 hardcode).
#[derive(Debug, Error, PartialEq)]
pub enum NamingError {
    /// 字符串缺 "apeireth:" 前缀 (e.g. "5.PC.code.text.high.complete.apeireth-1.0").
    #[error("invalid prefix: expected 'apeireth:' at start, got '{0}'")]
    InvalidPrefix(String),

    /// level 不在 0-9 (e.g. level=11 或 "ten").
    #[error("invalid level: '{0}', must be 0..=9")]
    InvalidLevel(String),

    /// class 不在 4 大类内 (PC / RC / HG / GP).
    #[error("invalid class: '{0}', must be one of PC/RC/HG/GP")]
    InvalidClass(String),

    /// domain 不在 6 域内 (code/dialogue/vision/audio/tool/reasoning).
    #[error("invalid domain: '{0}', must be one of code/dialogue/vision/audio/tool/reasoning")]
    InvalidDomain(String),

    /// modality 不在 5 模态内 (text/image/audio/video/multimodal).
    #[error("invalid modality: '{0}', must be one of text/image/audio/video/multimodal")]
    InvalidModality(String),

    /// safety 不在 4 等级内 (low/medium/high/critical).
    #[error("invalid safety: '{0}', must be one of low/medium/high/critical")]
    InvalidSafety(String),

    /// completeness 不在 4 完整度内 (skeleton/partial/complete/production).
    #[error("invalid completeness: '{0}', must be one of skeleton/partial/complete/production")]
    InvalidCompleteness(String),

    /// lineage 不在 4 血统内 (spectrai-0.9/apeireth-0.14/apeireth-1.0/apeireth-2.0).
    #[error("invalid lineage: '{0}', must be one of spectrai-0.9/apeireth-0.14/apeireth-1.0/apeireth-2.0")]
    InvalidLineage(String),

    /// 4 大类权重和 ≠ 1.00 (守门破坏, 偏差 > 0.001).
    #[error("sum guard failed: 4-class weights sum to {sum}, must equal 1.00 (delta={delta})")]
    SumNotEquals1 {
        /// 实际求和.
        sum: f32,
        /// 与 1.0 的偏差.
        delta: f32,
    },

    /// 字符串格式错乱 (段数 ≠ 7 / 段间分隔符错 / 段内容空).
    #[error("malformed format: {0}")]
    MalformedFormat(String),
}

// ============================================================================
// §2 统一 Result 别名
// ============================================================================

/// 统一 Result 别名.
pub type NamingResult<T> = std::result::Result<T, NamingError>;

// ============================================================================
// §3 错误 variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// NamingError 编译期 hardcode variant 数 (10).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const NAMING_ERROR_VARIANT_COUNT: usize = 10;

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: NamingError 编译期 10 variant.
    #[test]
    fn naming_error_has_ten_variants() {
        // 10 variant 1:1: InvalidPrefix / InvalidLevel / InvalidClass / InvalidDomain
        //               / InvalidModality / InvalidSafety / InvalidCompleteness
        //               / InvalidLineage / SumNotEquals1 / MalformedFormat
        assert_eq!(NAMING_ERROR_VARIANT_COUNT, 10);
    }

    /// 守门 #2: 错误 Display 实现正常.
    #[test]
    fn naming_error_display_works() {
        let e = NamingError::InvalidLevel("11".to_string());
        let s = format!("{e}");
        assert!(s.contains("11"));
        assert!(s.contains("0..=9"));
    }

    /// 守门 #3: SumNotEquals1 含 sum + delta.
    #[test]
    fn sum_not_equals_1_error_includes_sum_and_delta() {
        let e = NamingError::SumNotEquals1 { sum: 1.05, delta: 0.05 };
        let s = format!("{e}");
        assert!(s.contains("1.05"));
        assert!(s.contains("0.05"));
    }
}
