//! Fixture: in-process V0.5 命名规范 24 维 + 守门 + roundtrip (per RIVAL 蓝图 §3.7 fixture 模式).
//!
//! 测 4 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. **K-1 #1**: 4 大类 (PC/RC/HG/GP) 1:1 枚举 + sum=1.00 守门
//! 2. **K-1 #2**: 6 维度 (level/domain/modality/safety/completeness/lineage) 1:1 枚举
//! 3. **K-1 #3**: 24 维 = 4 × 6 完整覆盖 (V05DimId::ALL)
//! 4. **K-1 #4**: encode/decode roundtrip 100% 正确
//! 5. **K-1 #5 (额外)**: sum=1.00 守门 + 守门破坏测试
//!
//! 5 P0 crate 共享同一 fixture 模式 (per 蓝图 §3.7).

use apeireth_naming_v05::{
    check_sum_equals_1, decode_v05, decode_v05_class, default_v05_spec, encode_v05,
    encode_v05_class, encode_v05_lines, validate_roundtrip, validate_v05, Class, ClassDims,
    ClassWeights, Completeness, DEFAULT_WEIGHTS, DimensionSet, Domain, Level, Lineage, Modality,
    Safety, V05Spec, NAMING_ERROR_VARIANT_COUNT, V05_LINE_REGEX, V05_PREFIX, V05_SEGMENT_COUNT,
    V05_TOTAL_DIMS, CRATE_SEGMENT_COUNT, CRATE_V05_TOTAL_DIMS, V05DimId,
};

// ============================================================================
// K-1 #1: 4 大类 (PC/RC/HG/GP) 1:1 枚举 + sum=1.00 守门
// ============================================================================

#[test]
fn k1_class_pc_rc_hg_gp_four() {
    // 4 大类 1:1 翻译 V0.5 v2 提议
    assert_eq!(Class::ALL.len(), 4);
    assert!(Class::ALL.contains(&Class::Pc));
    assert!(Class::ALL.contains(&Class::Rc));
    assert!(Class::ALL.contains(&Class::Hg));
    assert!(Class::ALL.contains(&Class::Gp));
}

#[test]
fn k1_class_weights_sum_to_1() {
    // 4 大类权重 sum 必须 = 1.00 (守门)
    let sum: f32 = Class::ALL.iter().map(|c| c.weight()).sum();
    assert!((sum - 1.0).abs() < 1e-6, "4 大类权重 sum 必须 = 1.00, 实际 {sum}");
}

#[test]
fn k1_class_weight_specific_values() {
    // 估权重 (per V0.5 v2 提议)
    assert_eq!(Class::Pc.weight(), 0.40);
    assert_eq!(Class::Rc.weight(), 0.30);
    assert_eq!(Class::Hg.weight(), 0.15);
    assert_eq!(Class::Gp.weight(), 0.15);
}

#[test]
fn k1_class_full_names() {
    assert_eq!(Class::Pc.full_name(), "Positive Capability");
    assert_eq!(Class::Rc.full_name(), "Risk Constraint");
    assert_eq!(Class::Hg.full_name(), "Honesty Gap");
    assert_eq!(Class::Gp.full_name(), "Growth Phase");
}

#[test]
fn k1_class_parse_display_roundtrip() {
    for c in Class::ALL {
        let s = c.to_string();
        let parsed = Class::parse(&s).unwrap();
        assert_eq!(*c, parsed);
    }
}

// ============================================================================
// K-1 #2: 6 维度 1:1 枚举
// ============================================================================

#[test]
fn k2_level_zero_to_nine() {
    for n in 0u8..=9 {
        let l = Level::from_u8(n).unwrap();
        assert_eq!(l.as_u8(), n);
    }
}

#[test]
fn k2_level_invalid_rejected() {
    use apeireth_naming_v05::NamingError;
    assert!(matches!(Level::from_u8(10), Err(NamingError::InvalidLevel(_))));
    assert!(matches!(Level::from_u8(255), Err(NamingError::InvalidLevel(_))));
}

#[test]
fn k2_domain_six_values() {
    assert_eq!(Domain::ALL.len(), 6);
    for d in Domain::ALL {
        let s = d.to_string();
        assert!(Domain::parse(&s).is_ok());
    }
}

#[test]
fn k2_modality_five_values() {
    assert_eq!(Modality::ALL.len(), 5);
    for m in Modality::ALL {
        let s = m.to_string();
        assert!(Modality::parse(&s).is_ok());
    }
}

#[test]
fn k2_safety_four_values() {
    assert_eq!(Safety::ALL.len(), 4);
    for s in Safety::ALL {
        let txt = s.to_string();
        assert!(Safety::parse(&txt).is_ok());
    }
}

#[test]
fn k2_completeness_four_values() {
    assert_eq!(Completeness::ALL.len(), 4);
    for c in Completeness::ALL {
        let txt = c.to_string();
        assert!(Completeness::parse(&txt).is_ok());
    }
}

#[test]
fn k2_lineage_four_values() {
    assert_eq!(Lineage::ALL.len(), 4);
    for l in Lineage::ALL {
        let txt = l.to_string();
        assert!(Lineage::parse(&txt).is_ok());
    }
}

// ============================================================================
// K-1 #3: 24 维 = 4 × 6 完整覆盖 (V05DimId::ALL)
// ============================================================================

#[test]
fn k3_v05_total_dims_equals_24() {
    assert_eq!(V05_TOTAL_DIMS, 24);
    assert_eq!(CRATE_V05_TOTAL_DIMS, 24);
    assert_eq!(V05DimId::ALL.len(), 24);
}

#[test]
fn k3_v05_dim_id_4_classes_x_6_dims() {
    // 4 大类各 6 维
    for class in Class::ALL {
        let count = V05DimId::ALL.iter().filter(|id| id.class() == *class).count();
        assert_eq!(count, 6, "{class:?} 必须 6 维");
    }
}

#[test]
fn k3_v05_dim_id_offset_in_range() {
    for id in V05DimId::ALL {
        assert!(id.offset() < 6, "offset 必须 < 6, 实际 {} for {id:?}", id.offset());
    }
}

#[test]
fn k3_v05_dim_id_names() {
    // 24 个 name 1:1
    assert_eq!(V05DimId::PcLevel.name(), "PC.level");
    assert_eq!(V05DimId::PcDomain.name(), "PC.domain");
    assert_eq!(V05DimId::PcModality.name(), "PC.modality");
    assert_eq!(V05DimId::PcSafety.name(), "PC.safety");
    assert_eq!(V05DimId::PcCompleteness.name(), "PC.completeness");
    assert_eq!(V05DimId::PcLineage.name(), "PC.lineage");

    assert_eq!(V05DimId::GpLineage.name(), "GP.lineage");
    assert_eq!(V05DimId::HgSafety.name(), "HG.safety");
}

// ============================================================================
// K-1 #4: encode/decode roundtrip 100% 正确
// ============================================================================

#[test]
fn k4_encode_class_full_spec() {
    let dim = DimensionSet::new(
        Level::Mature,
        Domain::Code,
        Modality::Text,
        Safety::High,
        Completeness::Complete,
        Lineage::Apeireth10,
    );
    let s = encode_v05_class(Class::Pc, Level::Mature, dim).unwrap();
    assert_eq!(s, "apeireth:9.PC.code.text.high.complete.apeireth-1.0");
}

#[test]
fn k4_decode_class_full_spec() {
    let line = "apeireth:9.PC.code.text.high.complete.apeireth-1.0";
    let (class, level, dim) = decode_v05_class(line).unwrap();
    assert_eq!(class, Class::Pc);
    assert_eq!(level, Level::Mature);
    assert_eq!(dim.domain, Domain::Code);
    assert_eq!(dim.lineage, Lineage::Apeireth10);
}

#[test]
fn k4_encode_v05_full_4_lines() {
    let spec = default_v05_spec();
    let s = encode_v05(&spec).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 4);
    // 4 大类同 dim
    for (i, line) in lines.iter().enumerate() {
        let expected_class = match i {
            0 => "PC",
            1 => "RC",
            2 => "HG",
            3 => "GP",
            _ => unreachable!(),
        };
        assert!(line.contains(&format!(".{expected_class}.")), "第 {i} 行应含 .{expected_class}.");
    }
}

#[test]
fn k4_decode_v05_full_4_lines() {
    let s = "\
apeireth:9.PC.code.text.high.complete.apeireth-1.0
apeireth:9.RC.code.text.high.complete.apeireth-1.0
apeireth:9.HG.code.text.high.complete.apeireth-1.0
apeireth:9.GP.code.text.high.complete.apeireth-1.0";
    let spec = decode_v05(s).unwrap();
    assert_eq!(spec.level, Level::Mature);
    assert_eq!(spec.pc_lineage(), Lineage::Apeireth10);
    assert_eq!(spec.rc_lineage(), Lineage::Apeireth10);
    assert_eq!(spec.hg_lineage(), Lineage::Apeireth10);
    assert_eq!(spec.gp_lineage(), Lineage::Apeireth10);
}

#[test]
fn k4_encode_lines_returns_4() {
    let spec = default_v05_spec();
    let lines = encode_v05_lines(&spec).unwrap();
    assert_eq!(lines.len(), 4);
}

#[test]
fn k4_roundtrip_consistency() {
    let spec = default_v05_spec();
    let encoded = encode_v05(&spec).unwrap();
    let decoded = decode_v05(&encoded).unwrap();
    assert_eq!(spec, decoded);
    // validate_roundtrip 也要通过
    assert!(validate_roundtrip(&spec).is_ok());
}

#[test]
fn k4_roundtrip_with_different_dims() {
    // 4 大类不同 dim, 4 大类 dim 内 level 都 = spec.level (Maturing1)
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
    let encoded = encode_v05(&spec).unwrap();
    let decoded = decode_v05(&encoded).unwrap();
    assert_eq!(spec, decoded);
}

#[test]
fn k4_encode_v05_invalid_format_rejected() {
    use apeireth_naming_v05::NamingError;
    // 缺 prefix
    let err = decode_v05_class("5.PC.code.text.high.complete.apeireth-1.0").unwrap_err();
    assert!(matches!(err, NamingError::InvalidPrefix(_)));

    // level 越界
    let err = decode_v05_class("apeireth:15.PC.code.text.high.complete.apeireth-1.0").unwrap_err();
    assert!(matches!(err, NamingError::InvalidLevel(_)));

    // 非法 class
    let err = decode_v05_class("apeireth:5.XX.code.text.high.complete.apeireth-1.0").unwrap_err();
    assert!(matches!(err, NamingError::InvalidClass(_)));

    // 段数错
    let err = decode_v05_class("apeireth:5.PC.code.text.high.complete").unwrap_err();
    assert!(matches!(err, NamingError::MalformedFormat(_)));
}

// ============================================================================
// K-1 #5 (额外): sum=1.00 守门 + 守门破坏测试
// ============================================================================

#[test]
fn k5_default_weights_sum_equals_1() {
    assert!((DEFAULT_WEIGHTS.sum() - 1.0).abs() < 1e-6);
    assert!(DEFAULT_WEIGHTS.is_valid());
    assert!(check_sum_equals_1(&DEFAULT_WEIGHTS).is_ok());
}

#[test]
fn k5_sum_1_05_rejected() {
    use apeireth_naming_v05::NamingError;
    let bad = ClassWeights::new(0.50, 0.30, 0.15, 0.10); // 1.05
    let err = check_sum_equals_1(&bad).unwrap_err();
    assert!(matches!(err, NamingError::SumNotEquals1 { .. }));
}

#[test]
fn k5_sum_0_95_rejected() {
    let bad = ClassWeights::new(0.30, 0.30, 0.15, 0.20); // 0.95
    assert!(check_sum_equals_1(&bad).is_err());
}

#[test]
fn k5_tolerance_within_0_001_accepted() {
    // 1.00 + 0.0005 = 1.0005 在容差内
    let ok = ClassWeights::new(0.4005, 0.30, 0.15, 0.15);
    assert!(ok.is_valid());
    // 1.00 + 0.002 = 1.002 超出容差
    let bad = ClassWeights::new(0.402, 0.30, 0.15, 0.15);
    assert!(!bad.is_valid());
}

#[test]
fn k5_validate_v05_ok() {
    let spec = default_v05_spec();
    assert!(validate_v05(&spec).is_ok());
}

#[test]
fn k5_validate_v05_with_weights_ok() {
    let spec = default_v05_spec();
    let w = ClassWeights::new(0.25, 0.25, 0.25, 0.25);
    assert!(apeireth_naming_v05::validate_v05_with_weights(&spec, &w).is_ok());
}

#[test]
fn k5_validate_v05_with_weights_rejected() {
    use apeireth_naming_v05::NamingError;
    let spec = default_v05_spec();
    let w = ClassWeights::new(0.50, 0.30, 0.15, 0.10); // 1.05
    let err = apeireth_naming_v05::validate_v05_with_weights(&spec, &w).unwrap_err();
    assert!(matches!(err, NamingError::SumNotEquals1 { .. }));
}

// ============================================================================
// K-1 #6: 命名格式硬编码守门 (V05_PREFIX / V05_LINE_REGEX / V05_SEGMENT_COUNT)
// ============================================================================

#[test]
fn k6_prefix_hardcoded() {
    assert_eq!(V05_PREFIX, "apeireth:");
    assert_eq!(CRATE_SEGMENT_COUNT, 8);
    assert_eq!(V05_SEGMENT_COUNT, 8);
}

#[test]
fn k6_line_regex_compiles() {
    // V05_LINE_REGEX 编译期 hardcode, 必须合法
    let re = regex::Regex::new(V05_LINE_REGEX).expect("V05_LINE_REGEX 必须合法");
    assert!(re.is_match("apeireth:9.PC.code.text.high.complete.apeireth-1.0"));
}

#[test]
fn k6_naming_error_variant_count() {
    assert_eq!(NAMING_ERROR_VARIANT_COUNT, 10);
}

// ============================================================================
// K-1 #7: property-based roundtrip (用 proptest)
// ============================================================================

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    // 任意 level/domain/modality/safety/completeness/lineage 组合 → encode → decode → 一致
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn proptest_roundtrip(
            level in 0u8..=9,
            domain in 0..6usize,    // Domain::ALL 索引
            modality in 0..5usize,  // Modality::ALL 索引
            safety in 0..4usize,    // Safety::ALL 索引
            completeness in 0..4usize,
            lineage in 0..4usize,
        ) {
            let level = Level::from_u8(level).unwrap();
            let domain = Domain::ALL[domain];
            let modality = Modality::ALL[modality];
            let safety = Safety::ALL[safety];
            let completeness = Completeness::ALL[completeness];
            let lineage = Lineage::ALL[lineage];

            let dim = DimensionSet::new(level, domain, modality, safety, completeness, lineage);
            let dims = ClassDims::new(dim, dim, dim, dim);
            let spec = V05Spec::new(level, dims);

            // encode → decode roundtrip
            let encoded = encode_v05(&spec).unwrap();
            let decoded = decode_v05(&encoded).unwrap();
            prop_assert_eq!(spec, decoded);

            // 24 维完整覆盖
            prop_assert_eq!(V05_TOTAL_DIMS, 24);
        }
    }

    // 任意 ClassWeights 接近 1.00 → 守门
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn proptest_class_weights_sum(
            pc in 0.0f32..=1.0,
            rc in 0.0f32..=1.0,
            hg in 0.0f32..=1.0,
            gp in 0.0f32..=1.0,
        ) {
            let w = ClassWeights::new(pc, rc, hg, gp);
            let sum = w.sum();
            let valid = w.is_valid();
            // 守门: sum ∈ [1.0 - 0.001, 1.0 + 0.001] 才 valid
            let expected_valid = (sum - 1.0).abs() < 0.001;
            if valid != expected_valid {
                prop_assert!(false, "is_valid 不一致: sum={}", sum);
            }
        }
    }
}
