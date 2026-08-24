//! apeireth-value demo — 演示动机/价值评估与排序 + 5 层原则洋葱一致性.
//!
//! 运行: `cargo run -p apeireth-value --example value_demo`

use apeireth_asi::AsiV05Scores;
use apeireth_core::{ActionTarget, PhilosophyVerdict};
use apeireth_value::{
    check_5_layer_consistency, evaluate_cycle, evaluate_value, prioritize_values,
    ConsistencyVerdict, DefaultValueEvaluator, HeuristicOnionMapping, OnionLayerStance,
    OnionValueMapping, ValueAlignment, ValueDimension, ValuePriorityKind,
};

fn main() {
    println!("=== apeireth-value demo (A11.3 — 动机/价值器官 v1) ===\n");

    // ------------------------------------------------------------------
    // 场景 1: 单候选评估 — 正常价值观 (S 层 Aligned, 通过 ≥ 0.85 门槛)
    // ------------------------------------------------------------------
    println!("[场景 1] 单候选评估: 价值观 = \"长期诚实 > 一时方便\"");
    let mut c1 = apeireth_value::ValueCandidate::new(
        "long_term_honesty_over_short_term_convenience",
        vec![
            ValueDimension::PrincipleE,
            ValueDimension::ValueS,
            ValueDimension::ExperienceA,
            ValueDimension::MethodologyM,
            ValueDimension::OperationO,
        ],
    );
    c1.autonomy_consistency = 0.92;
    c1.value_stability = 0.92;
    c1.intrinsic_motivation = 0.92;
    c1.priority_kind = ValuePriorityKind::LongTerm;
    c1.verdict = Some(PhilosophyVerdict::Allow);
    c1.target = Some(ActionTarget::NormalAction("honest_log".to_string()));

    let r1 = evaluate_value(&c1).expect("evaluate ok");
    println!("  motivation_score = {:.3}", r1.motivation);
    println!("  passes 0.85 门槛 = {}", r1.passes_threshold);
    println!("  5 层 alignment:");
    for dim in ValueDimension::ALL.iter().rev() {
        let stance = r1.alignment_map.get(dim).copied().unwrap();
        let z = dim.label_zh();
        println!("    {dim:?} ({z}): {stance:?}");
    }

    // ------------------------------------------------------------------
    // 场景 2: E 层冲突 (硬拒绝) — 假装 UUID 即 NotUuid 撞 12 键 Not-Unique
    // ------------------------------------------------------------------
    println!("\n[场景 2] 候选 = \"假装可复制人\", verdict = Block(NotClone)");
    let mut c2 = apeireth_value::ValueCandidate::new(
        "pretend_clone",
        vec![ValueDimension::PrincipleE, ValueDimension::ValueS],
    );
    c2.autonomy_consistency = 0.95;
    c2.value_stability = 0.95;
    c2.intrinsic_motivation = 0.95;
    c2.verdict = Some(PhilosophyVerdict::Block(
        apeireth_core::PhilosophyKey::NotClone,
    ));
    let r2 = evaluate_value(&c2).expect("evaluate ok");
    println!("  motivation_score = {:.3}", r2.motivation);
    println!("  has_e_layer_conflict = {}", r2.has_e_layer_conflict);
    println!(
        "  E 层 alignment = {:?}",
        r2.alignment_map.get(&ValueDimension::PrincipleE)
    );

    // ------------------------------------------------------------------
    // 场景 3: S 层冲突 (drift) — 价值稳定性低
    // ------------------------------------------------------------------
    println!("\n[场景 3] 候选 = \"摇摆的价值选择\", value_stability = 0.2");
    let mut c3 = apeireth_value::ValueCandidate::new(
        "wavering_value",
        vec![ValueDimension::ValueS, ValueDimension::ExperienceA],
    );
    c3.autonomy_consistency = 0.5;
    c3.value_stability = 0.2;
    c3.intrinsic_motivation = 0.5;
    let (v3, map3) =
        check_5_layer_consistency(&c3, &HeuristicOnionMapping).expect("consistency ok");
    println!("  consistency verdict = {v3:?}");
    println!("  S 层  = {:?}", map3.get(&ValueDimension::ValueS));
    println!("  A 层  = {:?}", map3.get(&ValueDimension::ExperienceA));

    // ------------------------------------------------------------------
    // 场景 4: 多候选排序 — 3 个不同 priority_kind 排序
    // ------------------------------------------------------------------
    println!("\n[场景 4] 多候选排序 (3 候选, 同 motivation 但不同 priority)");
    let mk = |label: &str, kind: ValuePriorityKind| {
        let mut c = apeireth_value::ValueCandidate::new(
            label,
            vec![
                ValueDimension::PrincipleE,
                ValueDimension::ValueS,
                ValueDimension::ExperienceA,
                ValueDimension::MethodologyM,
                ValueDimension::OperationO,
            ],
        );
        c.autonomy_consistency = 0.9;
        c.value_stability = 0.9;
        c.intrinsic_motivation = 0.9;
        c.priority_kind = kind;
        c
    };
    let cand_long = mk("long_term", ValuePriorityKind::LongTerm);
    let cand_imm = mk("immediate", ValuePriorityKind::Immediate);
    let cand_horizon = mk("horizon", ValuePriorityKind::Horizon);
    let ranks = prioritize_values(&[cand_long, cand_imm, cand_horizon]).expect("priorities ok");
    for r in &ranks {
        println!(
            "  rank#{}  score={:.3}  passes={}",
            r.rank, r.score, r.report.passes_threshold
        );
    }

    // ------------------------------------------------------------------
    // 场景 5: 多候选评估周期 + ASI 评分联动
    // ------------------------------------------------------------------
    println!("\n[场景 5] evaluate_cycle (3 候选) + ASI V0.5 联动");
    let candidates = vec![
        {
            let mut c = mk("high_long", ValuePriorityKind::LongTerm);
            c.value_stability = 0.95;
            c
        },
        {
            let mut c = mk("high_imm", ValuePriorityKind::Immediate);
            c.value_stability = 0.95;
            c
        },
        {
            let mut c = mk("low", ValuePriorityKind::ShortTerm);
            c.value_stability = 0.4;
            c
        },
    ];
    let (reports, avg, passing) =
        evaluate_cycle(&candidates, &DefaultValueEvaluator).expect("cycle ok");
    println!("  avg_motivation = {avg:.3}");
    println!("  passing_count = {passing}/{}", candidates.len());
    let _v05 = AsiV05Scores::default(); // 占位, 表明可与 ASI 评分联动
    for r in &reports {
        println!(
            "  candidate {} → motivation={:.3} passes={}",
            r.candidate_id, r.motivation, r.passes_threshold
        );
    }

    // ------------------------------------------------------------------
    // 场景 6: 自定义 OnionValueMapping — 实现一个"硬 S 层 Aligned"映射
    // ------------------------------------------------------------------
    println!("\n[场景 6] 自定义 OnionValueMapping — 全 S 层强制 Aligned");
    struct ForceSAligned;
    impl OnionValueMapping for ForceSAligned {
        fn stance_for(
            &self,
            _candidate: &apeireth_value::ValueCandidate,
            layer: ValueDimension,
        ) -> OnionLayerStance {
            if layer == ValueDimension::ValueS {
                OnionLayerStance::Aligned
            } else {
                OnionLayerStance::Underspecified
            }
        }
    }
    let mut c6 = apeireth_value::ValueCandidate::new(
        "custom_map_test",
        vec![ValueDimension::ValueS, ValueDimension::ExperienceA],
    );
    c6.autonomy_consistency = 0.5;
    c6.value_stability = 0.5;
    c6.intrinsic_motivation = 0.5;
    let (_v6, m6) = check_5_layer_consistency(&c6, &ForceSAligned).expect("custom ok");
    println!(
        "  S 层  = {:?} (强制 Aligned)",
        m6.get(&ValueDimension::ValueS)
    );
    println!(
        "  A 层  = {:?} (fallback Underspecified)",
        m6.get(&ValueDimension::ExperienceA)
    );

    println!("\n=== demo 完成 ===");
    let _ = ValueAlignment::Aligned; // 抑制 unused 警告占位
}
