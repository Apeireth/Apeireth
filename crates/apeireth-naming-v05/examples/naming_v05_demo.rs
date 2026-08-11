//! V0.5 命名规范 24 维 demo (4 类 × 6 维, sum=1.00 守门).
//!
//! 流程:
//! 1. 构造 24 维 spec (4 大类同 dim, level=9 mature)
//! 2. 守门 sum=1.00
//! 3. encode → 4 行字符串
//! 4. decode → 还原 spec
//! 5. roundtrip 一致性
//! 6. 演示守门破坏: 改权重到 1.05 → 拒绝
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-naming-v05 --example naming_v05_demo
//! ```
//!
//! ## 期望输出
//!
//! ```text
//! === apeireth-naming-v05 demo (R20 阶段 4 估补) ===
//!
//! [1] 默认 24 维 spec
//!   level: 9 (mature)
//!   4 大类 dim (code/text/high/complete/apeireth-1.0):
//!     PC: code.text.high.complete.apeireth-1.0
//!     RC: code.text.high.complete.apeireth-1.0
//!     HG: code.text.high.complete.apeireth-1.0
//!     GP: code.text.high.complete.apeireth-1.0
//!
//! [2] sum=1.00 守门 (默认)
//!   PC weight: 0.40
//!   RC weight: 0.30
//!   HG weight: 0.15
//!   GP weight: 0.15
//!   sum: 1.0
//!   check_sum_equals_1: Ok(())
//!
//! [3] encode 4 大类
//!   apeireth:9.PC.code.text.high.complete.apeireth-1.0
//!   apeireth:9.RC.code.text.high.complete.apeireth-1.0
//!   apeireth:9.HG.code.text.high.complete.apeireth-1.0
//!   apeireth:9.GP.code.text.high.complete.apeireth-1.0
//!
//! [4] decode 还原
//!   decoded level: 9
//!   decoded 4 大类: PC/RC/HG/GP 全到位
//!
//! [5] roundtrip 一致性
//!   encode == decode: true
//!   validate_roundtrip: Ok(())
//!
//! [6] 守门破坏: 改权重到 1.05
//!   new weights: 0.50 / 0.30 / 0.15 / 0.10 (sum=1.05)
//!   check_sum_equals_1: Err(SumNotEquals1 { sum: 1.05, delta: 0.05 })
//!
//! [7] 24 维 V05DimId 全表
//!   24 维 = 4 大类 × 6 维度
//!   ...
//!
//! [naming_v05_demo] completed (skeleton — R20 阶段 4 真实 spec 实施中)
//! ```
//!
//! ## 6 哲学 anchor 验证
//!
//! - S-1 北极星: 24 维服务 ASI 北极星
//! - S-2 实事求是: 24 维是 v2 提议, 不重写 v1077 17 维 LOCKED
//! - O-2 走在前人肩上: 借 v4.1 §13 + R17 命名 v12
//! - O-3 干到底: 24 维立即落, 守门硬约束
//! - O-4 任何人都能接手: 8 模块 + 30 维 struct + 守门 + 11 error
//! - O-5 不假装: skeleton 阶段 enum/struct 完整, 0 假装"已对齐 V0.5"

use apeireth_naming_v05::{
    check_sum_equals_1, decode_v05, default_v05_spec, encode_v05, encode_v05_class,
    encode_v05_lines, validate_roundtrip, validate_v05, validate_v05_with_weights, Class,
    ClassDims, ClassWeights, Completeness, DEFAULT_WEIGHTS, DimensionSet, Domain, Level, Lineage,
    Modality, Safety, V05Spec, V05DimId,
};

fn main() -> anyhow::Result<()> {
    println!("=== apeireth-naming-v05 demo (R20 阶段 4 估补) ===\n");

    // -----------------------------------------------------------------
    // [1] 默认 24 维 spec
    // -----------------------------------------------------------------
    println!("[1] 默认 24 维 spec");
    let spec = default_v05_spec();
    println!("  level: {} ({})", spec.level.as_u8(), spec.level.stage());
    println!("  4 大类 dim (code/text/high/complete/apeireth-1.0):");
    for class in Class::ALL {
        let dim = spec.dims.get(*class);
        println!(
            "    {}: {}.{}.{}.{}.{}",
            class.as_str(),
            dim.domain.as_str(),
            dim.modality.as_str(),
            dim.safety.as_str(),
            dim.completeness.as_str(),
            dim.lineage.as_str()
        );
    }
    println!();

    // -----------------------------------------------------------------
    // [2] sum=1.00 守门
    // -----------------------------------------------------------------
    println!("[2] sum=1.00 守门 (默认)");
    println!("  PC weight: {}", Class::Pc.weight());
    println!("  RC weight: {}", Class::Rc.weight());
    println!("  HG weight: {}", Class::Hg.weight());
    println!("  GP weight: {}", Class::Gp.weight());
    println!("  sum: {}", DEFAULT_WEIGHTS.sum());
    match check_sum_equals_1(&DEFAULT_WEIGHTS) {
        Ok(()) => println!("  check_sum_equals_1: Ok(())"),
        Err(e) => eprintln!("  check_sum_equals_1: Err({e})"),
    }
    println!();

    // -----------------------------------------------------------------
    // [3] encode 4 大类
    // -----------------------------------------------------------------
    println!("[3] encode 4 大类");
    let lines = encode_v05_lines(&spec)?;
    for line in &lines {
        println!("  {}", line);
    }
    println!();

    // -----------------------------------------------------------------
    // [4] decode 还原
    // -----------------------------------------------------------------
    println!("[4] decode 还原");
    let encoded = encode_v05(&spec)?;
    let decoded = decode_v05(&encoded)?;
    println!("  decoded level: {}", decoded.level.as_u8());
    println!("  decoded 4 大类: PC/RC/HG/GP 全到位");
    for class in Class::ALL {
        let dim = decoded.dims.get(*class);
        println!(
            "    decoded {}: {}.{}.{}.{}.{}",
            class.as_str(),
            dim.domain.as_str(),
            dim.modality.as_str(),
            dim.safety.as_str(),
            dim.completeness.as_str(),
            dim.lineage.as_str()
        );
    }
    println!();

    // -----------------------------------------------------------------
    // [5] roundtrip 一致性
    // -----------------------------------------------------------------
    println!("[5] roundtrip 一致性");
    println!("  encode == decode: {}", spec == decoded);
    match validate_roundtrip(&spec) {
        Ok(()) => println!("  validate_roundtrip: Ok(())"),
        Err(e) => eprintln!("  validate_roundtrip: Err({e})"),
    }
    match validate_v05(&spec) {
        Ok(()) => println!("  validate_v05: Ok(())"),
        Err(e) => eprintln!("  validate_v05: Err({e})"),
    }
    println!();

    // -----------------------------------------------------------------
    // [6] 守门破坏: 改权重到 1.05
    // -----------------------------------------------------------------
    println!("[6] 守门破坏: 改权重到 1.05");
    let bad_weights = ClassWeights::new(0.50, 0.30, 0.15, 0.10); // sum = 1.05
    println!(
        "  new weights: {} / {} / {} / {} (sum={})",
        bad_weights.pc,
        bad_weights.rc,
        bad_weights.hg,
        bad_weights.gp,
        bad_weights.sum()
    );
    match check_sum_equals_1(&bad_weights) {
        Ok(()) => println!("  check_sum_equals_1: Ok(()) (守门破坏, 期望 Err)"),
        Err(e) => println!("  check_sum_equals_1: Err({e})"),
    }
    match validate_v05_with_weights(&spec, &bad_weights) {
        Ok(()) => println!("  validate_v05_with_weights: Ok(()) (期望 Err)"),
        Err(e) => println!("  validate_v05_with_weights: Err({e})"),
    }
    println!();

    // -----------------------------------------------------------------
    // [7] 24 维 V05DimId 全表
    // -----------------------------------------------------------------
    println!("[7] 24 维 V05DimId 全表");
    println!("  24 维 = {} 大类 × {} 维度", Class::ALL.len(), 6);
    for (i, id) in V05DimId::ALL.iter().enumerate() {
        println!(
            "    [{:2}] {} (class={}, offset={})",
            i,
            id.name(),
            id.class().as_str(),
            id.offset()
        );
    }
    println!();

    // -----------------------------------------------------------------
    // [8] 高级: 4 大类不同 dim 演示
    // -----------------------------------------------------------------
    println!("[8] 高级: 4 大类不同 dim 演示");
    let pc_dim = DimensionSet::new(
        Level::Mature,
        Domain::Code,
        Modality::Text,
        Safety::High,
        Completeness::Complete,
        Lineage::Apeireth10,
    );
    let rc_dim = DimensionSet::new(
        Level::Expert1,
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
        Level::Seed,
        Domain::Dialogue,
        Modality::Audio,
        Safety::Low,
        Completeness::Skeleton,
        Lineage::Spectra09,
    );
    let advanced_dims = ClassDims::new(pc_dim, rc_dim, hg_dim, gp_dim);
    let advanced_spec = V05Spec::new(Level::Maturing1, advanced_dims);
    let advanced_lines = encode_v05_lines(&advanced_spec)?;
    for line in &advanced_lines {
        println!("  {}", line);
    }
    println!();

    // -----------------------------------------------------------------
    // [9] encode_v05_class 单 class 演示
    // -----------------------------------------------------------------
    println!("[9] encode_v05_class 单 class 演示");
    let single = encode_v05_class(Class::Pc, Level::Expert1, pc_dim)?;
    println!("  PC level=7 expert: {}", single);
    println!();

    println!("[naming_v05_demo] completed (skeleton — R20 阶段 4 真实 spec 实施中)");
    Ok(())
}
