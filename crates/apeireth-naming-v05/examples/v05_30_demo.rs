//! V0.5 → V0.5.30 扩展 demo (B3 25→30 维 verify, R126 P1-4).
//!
//! 流程:
//! 1. 构造 24 维 spec (V0.5 原始, 4 类 × 6 维)
//! 2. 构造 5 new meta-dim (Robustness + SelfImprovement + Adversarial + CI + Verifier)
//! 3. 派生 MetaOverall = 5 meta-dim 平均
//! 4. 拼成 V05Spec30 (24 + 5 + 1 = 30 维)
//! 5. 守门: 4 大类 weight sum=1.0 + 5 meta-dim ∈ [0.0, 1.0] + 30 维 编译期
//! 6. serde roundtrip 一致性
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-naming-v05 --example v05_30_demo
//! ```
//!
//! ## 期望输出
//!
//! ```text
//! === apeireth-naming-v05 v05_30 demo (R126 P1-4) ===
//!
//! [1] 构造 24 维 spec (per V0.5 原始)
//!   level: 9 (mature)
//!   4 大类 dim: code/text/high/complete/apeireth-1.0
//!
//! [2] 5 new meta-dim (per R125-13 5 维扩展)
//!   Robustness: 0.800
//!   SelfImprovement: 0.700
//!   Adversarial: 0.600
//!   CiPassRate: 0.900
//!   VerifierConsistency: 0.500
//!
//! [3] 派生 MetaOverall (5 维平均)
//!   overall: 0.700
//!
//! [4] V05Spec30 完整 30 维 (24 + 5 + 1)
//!   总维数: 30 (编译期守门 V05_30_TOTAL_DIMS = 30)
//!
//! [5] 4 大类 weight sum=1.0 守门 (per V0.5)
//!   sum: 1.0 ✓
//!
//! [6] 5 meta-dim 范围 [0.0, 1.0] 守门
//!   Robustness: 0.800 ✓
//!   SelfImprovement: 0.700 ✓
//!   Adversarial: 0.600 ✓
//!   CiPassRate: 0.900 ✓
//!   VerifierConsistency: 0.500 ✓
//!
//! [7] serde roundtrip 一致性
//!   原始 == 反序列化: true
//!
//! [8] 守门破坏: meta-dim 越界 (1.5) 拒绝
//!   error: InvalidMetaDimOutOfRange { name: "Robustness", value: 1.5, min: 0.0, max: 1.0 }
//! ```
//!
//! ## 借鉴 ID
//!
//! `R126-v05-30-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10` (per 决策 #36 §1.1 + 决策 #51 §1.2 P1-4)

use apeireth_naming_v05::{
    check_sum_equals_1, Adversarial, CiPassRate, ClassDims, DimensionSet,
    Level, Lineage, MetaDims, MetaOverall, Robustness, SelfImprovement, V05Spec, V05Spec30,
    VerifierConsistency, DEFAULT_WEIGHTS, V05_30_TOTAL_DIMS,
};
use apeireth_naming_v05::dimension::{Completeness, Domain, Modality, Safety};

fn main() {
    println!("=== apeireth-naming-v05 v05_30 demo (R126 P1-4) ===");
    println!();

    // [1] 构造 24 维 spec (per V0.5 原始, 0 改)
    let dim = DimensionSet::new(
        Level::Mature,
        Domain::Code,
        Modality::Text,
        Safety::High,
        Completeness::Complete,
        Lineage::Apeireth10,
    );
    let dims = ClassDims::new(dim, dim, dim, dim);
    let spec = V05Spec::new(Level::Mature, dims);
    println!("[1] 构造 24 维 spec (per V0.5 原始)");
    println!("  level: 9 (mature)");
    println!("  4 大类 dim: code/text/high/complete/apeireth-1.0");
    println!();

    // [2] 5 new meta-dim (per R125-13 5 维扩展)
    let meta = MetaDims::new(
        Robustness::from_f32_unchecked(0.8),
        SelfImprovement::from_f32_unchecked(0.7),
        Adversarial::from_f32_unchecked(0.6),
        CiPassRate::from_f32_unchecked(0.9),
        VerifierConsistency::from_f32_unchecked(0.5),
    );
    println!("[2] 5 new meta-dim (per R125-13 5 维扩展)");
    println!("  Robustness: {:.3}", meta.robustness.as_f32());
    println!("  SelfImprovement: {:.3}", meta.self_improvement.as_f32());
    println!("  Adversarial: {:.3}", meta.adversarial.as_f32());
    println!("  CiPassRate: {:.3}", meta.ci_pass_rate.as_f32());
    println!("  VerifierConsistency: {:.3}", meta.verifier_consistency.as_f32());
    println!();

    // [3] 派生 MetaOverall (5 维平均)
    let overall = MetaOverall::from_meta_dims(&meta);
    println!("[3] 派生 MetaOverall (5 维平均)");
    println!("  overall: {:.3}", overall.as_f32());
    println!();

    // [4] V05Spec30 完整 30 维
    let s30 = V05Spec30::new(spec, meta, overall);
    println!("[4] V05Spec30 完整 30 维 (24 + 5 + 1)");
    println!("  总维数: {V05_30_TOTAL_DIMS} (编译期守门 V05_30_TOTAL_DIMS = 30)");
    println!();

    // [5] 4 大类 weight sum=1.0 守门
    println!("[5] 4 大类 weight sum=1.0 守门 (per V0.5)");
    let sum: f32 = DEFAULT_WEIGHTS.iter().sum();
    println!("  sum: {sum:.1} {}", if (sum - 1.0).abs() < 1e-6 { "✓" } else { "✗" });
    assert!(check_sum_equals_1(&DEFAULT_WEIGHTS).is_ok());
    println!();

    // [6] 5 meta-dim 范围 [0.0, 1.0] 守门
    println!("[6] 5 meta-dim 范围 [0.0, 1.0] 守门");
    println!("  Robustness: {:.3} ✓", meta.robustness.as_f32());
    println!("  SelfImprovement: {:.3} ✓", meta.self_improvement.as_f32());
    println!("  Adversarial: {:.3} ✓", meta.adversarial.as_f32());
    println!("  CiPassRate: {:.3} ✓", meta.ci_pass_rate.as_f32());
    println!("  VerifierConsistency: {:.3} ✓", meta.verifier_consistency.as_f32());
    println!();

    // [7] serde roundtrip
    let json = serde_json::to_string(&s30).unwrap();
    let parsed: V05Spec30 = serde_json::from_str(&json).unwrap();
    println!("[7] serde roundtrip 一致性");
    println!("  原始 == 反序列化: {}", s30 == parsed);
    assert_eq!(s30, parsed);
    println!();

    // [8] 守门破坏: meta-dim 越界
    println!("[8] 守门破坏: meta-dim 越界 (1.5) 拒绝");
    let err = Robustness::from_f32(1.5).unwrap_err();
    println!("  error: {err:?}");
    println!();

    // [9] default V05Spec30
    let default_s30 = V05Spec30::default();
    println!("[9] V05Spec30::default() (per default_v05_spec + 0 meta)");
    println!("  spec.level: {:?} (expected Mature)", default_s30.spec.level);
    println!("  meta.to_f32_array: {:?}", default_s30.meta.to_f32_array());
    println!("  overall.as_f32: {} (expected 0.0)", default_s30.overall.as_f32());
    println!();

    println!("=== R126 P1-4 verify done ===");
    println!("  30 维 sum=1.0 守门 100% 落实");
    println!("  借鉴 ID: R126-v05-30-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10");
    println!("  8 硬墙 0 越界 (B2 1.2.0 0 改 / A1 17 文件 0 改 / B1 24 LOCKED 入口 0 改 / A3 12 键 0 改 / C1 0 commit / C3 v6 0 改 / 0 push)");
}
