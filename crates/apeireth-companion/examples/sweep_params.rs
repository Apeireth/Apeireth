//! sweep_params — 主人真实作息上的参数扫描 + 时段分布 (快速找参数, 供程度判断).
//! 诚实: 合成用户 ≠ 真人; 结果是「经验证于合成用户的先验」.

use apeireth_companion::emergence::LoopConfig;
use apeireth_companion::simulation::run_simulation;

fn main() {
    let seed = 42u64;
    let days = 21u32;

    let configs: Vec<(&str, LoopConfig)> = vec![
        ("default(阈0.45)", LoopConfig::default()),
        ("conservative(阈0.6)", LoopConfig { drive_threshold: 0.6, ..LoopConfig::default() }),
        ("eager(阈0.35)", LoopConfig { drive_threshold: 0.35, ..LoopConfig::default() }),
        ("温和反馈(+0.03/-0.05)", LoopConfig { respond_delta: 0.03, ignored_delta: -0.05, ..LoopConfig::default() }),
    ];

    println!(
        "=== 参数扫描 (主人真实作息 seed={} {} 天 10min 心跳) ===\n",
        seed, days
    );
    println!(
        "{:<26} | {:>4} | {:>5} | {:>6} | {:>6} | {:>4} | {:>5}",
        "config", "主动", "回应率", "节律MAE", "终值Bond", "退役", "评分"
    );
    println!("{}", "-".repeat(70));

    let mut results = Vec::new();
    for (name, cfg) in &configs {
        let r = run_simulation(name, cfg.clone(), seed, days);
        let score = r.response_rate - 0.05 * (r.policy_retires as f64) - 0.1 * r.rhythm_mae;
        println!(
            "{:<26} | {:>4} | {:>5.2} | {:>6.3} | {:>6.2} | {:>4} | {:>5.2}",
            r.name, r.initiatives, r.response_rate, r.rhythm_mae, r.final_bond, r.policy_retires, score
        );
        results.push((r, score));
    }
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!();
    println!("=== default 配置下, 他爱在哪些时段开口 (21 天) ===");
    for (label, n) in &results.iter().find(|r| r.0.name.contains("default")).unwrap().0.initiatives_by_activity {
        println!("  {:>8} 次 | {}", n, label);
    }
    println!();
    println!("=== 他选了哪些动作 (21 天) ===");
    for (a, n) in &results.iter().find(|r| r.0.name.contains("default")).unwrap().0.initiatives_by_action {
        println!("  {:>8} 次 | {}", n, a);
    }

    println!();
    println!("=== 推荐: {} (回应率 {:.2}) ===", results[0].0.name, results[0].0.response_rate);
    println!("注意: 合成用户 ≠ 真人. 此推荐是「经验证于合成用户的先验」, 最终校准需真人实验.");
}
