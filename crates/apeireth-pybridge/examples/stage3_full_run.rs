//! Stage 3 综合 runnable example — 端到端 + 性能 + 跨模块 (per decision-58 §2.1 P10-3)
//!
//! 一次性跑 3 部分:
//! 1. 端到端 smoke (Stage 3 e2e)
//! 2. 性能基准 (Stage 3 bench 5 target × 100 iter)
//! 3. 跨模块集成 (Stage 3 cross_module 5 探针 + 8 硬墙 verify)
//!
//! ```bash
//! cargo run -p apeireth-pybridge --example stage3_full_run
//! ```

use apeireth_pybridge::{
    stage3_bench_run_default, stage3_cross_module_probes, stage3_e2e_smoke, HardWallsVerify,
};

fn main() {
    println!("================================================================");
    println!("Stage 3 综合集成验证 (per decision-58 §2.1 P10-3)");
    println!("================================================================\n");

    // ============================================================
    // Part 1: 端到端 smoke
    // ============================================================
    println!("[Part 1/3] 端到端 smoke (借鉴 hyper 80 + servers 175):");
    let e2e = stage3_e2e_smoke();
    println!(
        "  asi_count: {}, r11_count: {}",
        e2e.asi_module_count, e2e.r11_module_count
    );
    println!(
        "  pool: max_idle={} idle_timeout={}s",
        e2e.pool_max_idle, e2e.pool_idle_timeout_secs
    );
    println!(
        "  categories_in_use: {}, ceiling_critical: {}",
        e2e.categories_in_use,
        e2e.ceiling_critical_modules.len()
    );
    println!("  modules_in_scope: {:?}", e2e.modules_in_scope);
    println!(
        "  e2e_ok: {} (python_ext: {}, python_available: {})\n",
        e2e.e2e_ok, e2e.python_ext_active, e2e.python_available
    );

    // ============================================================
    // Part 2: 性能基准
    // ============================================================
    println!("[Part 2/3] 性能基准 (借鉴 superpowers 234 skill execution):");
    let bench = stage3_bench_run_default();
    println!(
        "  total_wallclock: {:.2}ms",
        bench.total_wallclock.as_micros() as f64 / 1000.0
    );
    for tr in &bench.target_reports {
        println!(
            "  - {:<22} mean={:>6.2}μs p95={:>6.2}μs (n={})",
            tr.target_id,
            tr.stats.mean.as_nanos() as f64 / 1000.0,
            tr.stats.p95.as_nanos() as f64 / 1000.0,
            tr.stats.n,
        );
    }
    let total_iters: usize = bench.target_reports.iter().map(|t| t.stats.n).sum();
    println!(
        "  吞吐: {:.0} iter/sec\n",
        total_iters as f64 / bench.total_wallclock.as_secs_f64()
    );

    // ============================================================
    // Part 3: 跨模块集成 + 8 硬墙 verify
    // ============================================================
    println!("[Part 3/3] 跨模块集成 (借鉴 PyO3 928 pybridge):");
    let xmod = stage3_cross_module_probes();
    println!("  modules in scope: {:?}", xmod.modules_in_scope);
    println!(
        "  probes: {}/{} OK",
        xmod.probe_results.iter().filter(|p| p.ok).count(),
        xmod.probe_results.len()
    );
    for p in &xmod.probe_results {
        println!(
            "  - {:?}: {} ({})",
            p.kind,
            if p.ok { "OK" } else { "FAIL" },
            p.description
        );
    }
    println!();

    // 8 硬墙 verify
    let walls = HardWallsVerify::auto_verify();
    println!("  8 硬墙 verify (B1-B7 + A1-A3 + C1-C3):");
    println!(
        "    B2 (workspace.version 1.2.0 0 改):     {}",
        if walls.b2_workspace_version_unchanged {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "    A1 (R11 baseline 0.8682/0.8532/0.9063): {}",
        if walls.a1_baseline_locked {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "    B1 (24 LOCKED 入口签名 0 改):           {}",
        if walls.b1_24_locked_unchanged {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "    B5 (8 哲学锚):                          {}",
        if walls.b5_8_philosophical_anchors {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "    B3 (V0.5 30 维):                        {}",
        if walls.b3_30_dimensions {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "    B4 (6 重守门 v7):                       {}",
        if walls.b4_6_gates_v7 { "PASS" } else { "FAIL" }
    );
    println!(
        "    A3 (12 键 + PHL-07 = 13 键):            {}",
        if walls.a3_13_keys { "PASS" } else { "FAIL" }
    );
    println!(
        "    C1 (0 主动 commit):                     {}",
        if walls.c1_no_commit { "PASS" } else { "FAIL" }
    );
    println!(
        "    C2 (0 装 PASS 严守):                    {}",
        if walls.c2_no_fake_pass {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "    C3 (升 6 重 v7):                        {}",
        if walls.c3_6_gates_v7 { "PASS" } else { "FAIL" }
    );
    println!();

    // ============================================================
    // 综合判定
    // ============================================================
    println!("================================================================");
    let all_pass = walls.all_pass() && xmod.all_ok;
    if all_pass {
        println!("✅ Stage 3 综合集成验证 ALL PASS (0 装 PASS 严守 100%)");
    } else {
        println!("❌ Stage 3 综合集成验证部分 FAIL");
    }
    println!("================================================================");
}
