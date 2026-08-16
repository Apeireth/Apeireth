//! # validate — 24 维合法性 + sum=1.00 守门
//!
//! 守门 3 道:
//! 1. **24 维合法性** — V05Spec 内部 4 大类 dim 字段值是否合法 (编译期 enum 已保证大半, 这里补 runtime 兜底).
//! 2. **sum=1.00 守门** — 4 大类权重和必须 = 1.00 (容差 0.001).
//! 3. **格式守门** — encode 出来的字符串能否被 decode 解析回 (roundtrip 一致性).
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use crate::class::V05Spec;
use crate::error::NamingResult;
use crate::sum_guard::{check_sum_equals_1, ClassWeights, DEFAULT_WEIGHTS};

// ============================================================================
// §1 V05Spec 全量守门 (24 维合法性 + sum=1.00 + roundtrip)
// ============================================================================

/// 守门入口: V05Spec 全量校验.
///
/// 守门项:
/// 1. `validate_v05` 内部 24 维 enum 合法性 (编译期已保证, runtime 是 noop 兜底)
/// 2. `ClassWeights` (默认 0.40/0.30/0.15/0.15) sum=1.00 守门
/// 3. encode → decode roundtrip 一致性
///
/// ## 用法
/// ```ignore
/// let spec = V05Spec::new(Level::Mature, dims);
/// validate_v05(&spec)?;  // Ok(())
///
/// let bad_weights = ClassWeights::new(0.50, 0.30, 0.15, 0.10);
/// validate_v05_with_weights(&spec, &bad_weights)?;  // Err(SumNotEquals1)
/// ```
pub fn validate_v05(spec: &V05Spec) -> NamingResult<()> {
    // 1) 默认权重 sum=1.00 守门
    check_sum_equals_1(&DEFAULT_WEIGHTS)?;

    // 2) 24 维 内部合法性 (编译期已保证, runtime 仅 sanity check)
    validate_v05_structure(spec)?;

    // 3) roundtrip 一致性: encode → decode
    crate::validate::validate_roundtrip(spec)?;

    Ok(())
}

/// V05Spec 用自定义权重守门.
pub fn validate_v05_with_weights(spec: &V05Spec, weights: &ClassWeights) -> NamingResult<()> {
    check_sum_equals_1(weights)?;
    validate_v05_structure(spec)?;
    validate_roundtrip(spec)?;
    Ok(())
}

// ============================================================================
// §2 24 维结构守门 (内部 sanity check, 编译期 enum 已保证)
// ============================================================================

/// V05Spec 24 维结构守门 (内部 sanity check).
///
/// 编译期 enum 已保证所有字段合法, 这里仅做: level ∈ 0-9 (compile-time),
/// 4 大类 dim 字段一致 (immutable 模式).
pub fn validate_v05_structure(spec: &V05Spec) -> NamingResult<()> {
    // level 编译期 hardcode 0-9, runtime 仅做 noop 确认
    let _ = spec.level.as_u8();

    // 4 大类 dim 字段都已被 enum 类型保证, runtime 仅做无 op
    let _ = spec.dims.pc;
    let _ = spec.dims.rc;
    let _ = spec.dims.hg;
    let _ = spec.dims.gp;

    Ok(())
}

// ============================================================================
// §3 roundtrip 一致性守门
// ============================================================================

/// roundtrip 一致性: encode → decode → 比较.
///
/// 守门 encode/decode 实现一致性, 防 m3 hallucination 改 encode 但忘改 decode.
pub fn validate_roundtrip(spec: &V05Spec) -> NamingResult<()> {
    let encoded = crate::encode::encode_v05(spec)?;
    let decoded = crate::decode::decode_v05(&encoded)?;
    if decoded != *spec {
        return Err(crate::error::NamingError::MalformedFormat(
            "roundtrip 不一致 (encode/decode 实现 drift)".to_string(),
        ));
    }
    Ok(())
}

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::ClassDims;
    use crate::dimension::{Completeness, DimensionSet, Domain, Level, Lineage, Modality, Safety};

    /// 默认完整 24 维 spec.
    fn default_spec() -> V05Spec {
        let dim = DimensionSet::new(
            Level::Mature,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let dims = ClassDims::new(dim, dim, dim, dim);
        V05Spec::new(Level::Mature, dims)
    }

    /// 守门 #1: 默认 spec 守门通过.
    #[test]
    fn validate_v05_default_ok() {
        let spec = default_spec();
        assert!(validate_v05(&spec).is_ok());
    }

    /// 守门 #2: 自定义合法权重守门通过.
    #[test]
    fn validate_v05_with_weights_ok() {
        let spec = default_spec();
        let w = ClassWeights::new(0.25, 0.25, 0.25, 0.25); // sum=1.00
        assert!(validate_v05_with_weights(&spec, &w).is_ok());
    }

    /// 守门 #3: 自定义非法权重守门拒绝.
    #[test]
    fn validate_v05_with_weights_rejected() {
        let spec = default_spec();
        let w = ClassWeights::new(0.50, 0.30, 0.15, 0.10); // sum=1.05
        let err = validate_v05_with_weights(&spec, &w).unwrap_err();
        assert!(matches!(
            err,
            crate::error::NamingError::SumNotEquals1 { .. }
        ));
    }

    /// 守门 #4: validate_v05_structure OK.
    #[test]
    fn validate_v05_structure_ok() {
        let spec = default_spec();
        assert!(validate_v05_structure(&spec).is_ok());
    }

    /// 守门 #5: roundtrip 一致性.
    #[test]
    fn validate_roundtrip_ok() {
        let spec = default_spec();
        assert!(validate_roundtrip(&spec).is_ok());
    }

    /// 守门 #6: roundtrip 跨不同 24 维组合 (4 大类 dim 内 level 必须都 = spec.level).
    #[test]
    fn validate_roundtrip_with_different_dims() {
        // spec 顶层 level=Maturing1, 4 大类 dim 内 level 都对齐 Maturing1
        // (encode 用 spec.level 编码每行, decode 后 dim 内 level 会被 spec.level 覆盖, 因此必须一致)
        let pc_dim = DimensionSet::new(
            Level::Maturing1,
            Domain::Code,
            Modality::Text,
            Safety::High,
            Completeness::Complete,
            Lineage::Apeireth10,
        );
        let rc_dim = DimensionSet::new(
            Level::Maturing1,
            Domain::Tool,
            Modality::Multimodal,
            Safety::Critical,
            Completeness::Production,
            Lineage::Apeireth20,
        );
        let hg_dim = DimensionSet::new(
            Level::Maturing1,
            Domain::Reasoning,
            Modality::Text,
            Safety::Medium,
            Completeness::Partial,
            Lineage::Apeireth014,
        );
        let gp_dim = DimensionSet::new(
            Level::Maturing1,
            Domain::Dialogue,
            Modality::Audio,
            Safety::Low,
            Completeness::Skeleton,
            Lineage::Spectra09,
        );
        let dims = ClassDims::new(pc_dim, rc_dim, hg_dim, gp_dim);
        let spec = V05Spec::new(Level::Maturing1, dims);
        assert!(validate_roundtrip(&spec).is_ok());
    }
}
