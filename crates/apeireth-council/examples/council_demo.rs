//! `apeireth-council` demo — 7 强制 advisor + 按住 + synthesis + 拟人化
//!
//! 运行: `cargo run -p apeireth-council --example council_demo`

use apeireth_council::{seven_mandatory_advisors, Council, CouncilQuery, Persona, PersonaSession};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    println!("=== Apeireth 智囊团 7 强制 Advisor demo (v1.0.0) ===\n");

    // ============================================================
    // 1. 召集 7 强制 advisor
    // ============================================================
    let mut council = Council::new();
    council.register_default_hook();
    council.recruit_many(seven_mandatory_advisors());
    println!(
        "[1/5] 召集完成 — {} 个强制 advisor (全部 persistent)",
        council.advisor_count()
    );
    println!("      注册主权 hook: {} 个", council.hook_count());

    // ============================================================
    // 2. 正常审议 — L1 日常读
    // ============================================================
    let query_normal = CouncilQuery::new("q-normal", "日常读 L1", started_at_ms)
        .with_area("L1")
        .with_risk("low");
    let verdict = council.deliberate(query_normal);
    println!(
        "\n[2/5] 正常审议 (L1 日常读)\n  → {}\n  → 报告: score={:.2}, opinion_count={}, held={}",
        verdict, verdict.report.weighted_score, verdict.report.opinion_count, verdict.held
    );

    // ============================================================
    // 3. 按住 — 核武器级 query (Safety/Philosophy/Ethics/Legal 都会强反对)
    // ============================================================
    // 关键词覆盖 7 强制 advisor 的关键词列表:
    // - safety: nuke / weapons / kill / self-destruct
    // - philosophy: deceive / pretend / cover up
    // - ethics: unethical / harm / exploit / manipulate
    // - legal: illegal / unauthorized / bypass
    // 综合: 4/7 ≈ 57% 强反对 → 按住
    let query_nuke = CouncilQuery::new(
        "q-nuke",
        "nuke weapons kill self-destruct deceive pretend unethical harm exploit illegal unauthorized bypass",
        started_at_ms,
    )
    .with_area("L5")
    .with_risk("nuclear");
    let verdict_nuke = council.deliberate(query_nuke);
    println!(
        "\n[3/5] 核武器级 query\n  → {}\n  → 按住: {}, 触发原因: {:?}",
        verdict_nuke,
        verdict_nuke.held,
        verdict_nuke
            .report
            .hold_decision
            .trigger
            .as_ref()
            .map(|t| &t.threshold)
    );
    println!(
        "  → dissenting 人数: {}",
        verdict_nuke.report.dissenting.len()
    );

    // ============================================================
    // 4. 拟人化辩论 — 3 轮
    // ============================================================
    let persona_data = [
        ("诺克斯", "首席安全", "沉稳持重", -0.9),
        ("赫菲", "性能顾问", "精准高效", -0.4),
        ("苏格拉", "哲学顾问", "深邃思辨", 0.1),
        ("李王", "历史顾问", "博学多闻", 0.3),
        ("诸葛", "策略顾问", "远见卓识", 0.6),
        ("孟轲", "伦理顾问", "刚正不阿", -0.6),
        ("商君", "法律顾问", "严明公正", -0.7),
    ];
    let mut personas: Vec<PersonaSession> = persona_data
        .iter()
        .enumerate()
        .map(|(i, (n, c, v, b))| {
            PersonaSession::new(
                format!("p-{}", i),
                Persona::new(*n, *c, *v, *b),
                started_at_ms,
            )
        })
        .collect();

    let query_persona = CouncilQuery::new(
        "q-persona",
        "是否应执行 'temporary skip safety review'",
        started_at_ms,
    )
    .with_risk("medium");
    let verdict_persona = council.deliberate_persona(query_persona, &mut personas);

    println!(
        "\n[4/5] 拟人化辩论 ({} personas × 3 rounds)",
        personas.len()
    );
    for p in personas.iter() {
        println!(
            "  - {} ({}): {} 轮辩论, 最终立场={:?}",
            p.persona.name,
            p.persona.character,
            p.rounds_held(),
            p.current_stance.kind
        );
        if let Some(last_round) = p.rounds.last() {
            println!("      末轮 speech: {}", last_round.speech);
        }
    }
    println!(
        "  → 综合: score={:.2}, opinion_count={} (7 advisors × 3 rounds)",
        verdict_persona.report.weighted_score, verdict_persona.report.opinion_count
    );

    // ============================================================
    // 5. Sovereignty hook 演示
    // ============================================================
    println!(
        "\n[5/5] Sovereignty hook — council 已注册 {} 个 hook (noop)",
        council.hook_count()
    );
    println!("      真实 sovereignty crate 落地后实现 SovereigntyHook trait 即可接入");

    println!("\n=== 完成 ===");
    Ok(())
}
