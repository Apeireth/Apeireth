//! 基准复现 harness 入口: 每个子命令跑一个基准, `all` 一键全跑。
//!
//! 用法 (工作区根目录):
//! ```text
//! cargo run --release -p apeireth-bench-harness -- all            # 全跑
//! cargo run --release -p apeireth-bench-harness -- hybrid-search  # 单项
//! cargo run --release -p apeireth-bench-harness -- list           # 列表
//! ```
//! 环境变量: `BENCH_SCALE` 缩放样本量; `BENCH_QUICK=1` 冒烟 (0.1 倍);
//! `BENCH_RAW=1` 输出逐样本原始值。一键封装见 `scripts/run-benchmarks.ps1`。
//!
//! 诊断子命令 (不计入 `all`, 无 README 目标行, 仅供剖析归因):
//! `profile-cold-start` (冷启动分段计时) / `profile-sandbox` (沙箱 spawn 分解),
//! 见 [`profile_startup`] 模块文档。

mod assembly;
mod atomic_write;
mod barge_in;
mod causal_cow;
mod cold_start;
mod ember_hud;
mod fold_surface;
mod hybrid_search;
mod idle_footprint;
mod os_sandbox;
mod profile_startup;
mod quota_scheduler;
mod saga_rollback;
mod spill_truncate;
mod support;

use support::{print_outcome, BenchConfig, Outcome};

/// 全部基准的稳定 key 列表 (与 README 性能目标表/报告行一一对应)。
const BENCH_NAMES: &[&str] = &[
    "hybrid-search",
    "quota-scheduler",
    "causal-cow",
    "saga-rollback",
    "barge-in",
    "ember-hud",
    "os-sandbox",
    "cold-start",
    "idle-footprint",
    "fold-surface",
    "spill-truncate",
    "atomic-write",
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("all");
    let cfg = BenchConfig::from_env();

    match command {
        "list" => {
            for name in BENCH_NAMES {
                println!("{name}");
            }
        }
        "all" => {
            run_all(&cfg);
        }
        name => match run_one(name, &cfg) {
            Some(outcomes) => {
                for outcome in &outcomes {
                    print_outcome(outcome, cfg.raw);
                }
            }
            None => {
                eprintln!(
                    "未知基准: {name} (可用: all / list / {})",
                    BENCH_NAMES.join(" / ")
                );
                std::process::exit(2);
            }
        },
    }
}

/// 跑单个基准 (按稳定 key)。
fn run_one(name: &str, cfg: &BenchConfig) -> Option<Vec<Outcome>> {
    match name {
        "hybrid-search" => Some(hybrid_search::run(cfg)),
        "quota-scheduler" => Some(quota_scheduler::run(cfg)),
        "fold-surface" => Some(fold_surface::run(cfg)),
        "spill-truncate" => Some(spill_truncate::run(cfg)),
        "atomic-write" => Some(atomic_write::run(cfg)),
        "causal-cow" => Some(causal_cow::run(cfg)),
        "saga-rollback" => Some(saga_rollback::run(cfg)),
        "barge-in" => Some(barge_in::run(cfg)),
        "ember-hud" => Some(ember_hud::run(cfg)),
        "os-sandbox" => Some(os_sandbox::run(cfg)),
        "cold-start" => Some(cold_start::run(cfg)),
        "idle-footprint" => Some(idle_footprint::run(cfg)),
        // 诊断子命令: 不在 BENCH_NAMES (不入 all), 按需显式跑。
        "profile-cold-start" => Some(profile_startup::run_cold_start(cfg)),
        "profile-sandbox" => Some(profile_startup::run_sandbox(cfg)),
        _ => None,
    }
}

/// 一键全跑: 按 README 表顺序执行并逐项输出结果块 (含 ROW 行)。
fn run_all(cfg: &BenchConfig) {
    println!("# apeireth-bench-harness 全量基准输出");
    println!();
    for name in BENCH_NAMES {
        let outcomes = run_one(name, cfg).expect("基准名来自常量表");
        for outcome in &outcomes {
            print_outcome(outcome, cfg.raw);
        }
    }
}
