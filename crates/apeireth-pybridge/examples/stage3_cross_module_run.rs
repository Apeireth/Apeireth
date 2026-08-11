//! Stage 3 跨模块集成验证 runnable example
//!
//! 5 大子模块协同探针 + 8 硬墙 verify
//!
//! ```bash
//! cargo run -p apeireth-pybridge --example stage3_cross_module_run
//! ```

use apeireth_pybridge::{stage3_cross_module_probes, HardWallsVerify};

fn main() {
    println!("================================================================");
    println!("Stage 3 跨模块集成验证 (per decision-58 §2.1 P10-3)");
    println!("================================================================");
    println!();

    // 5 探针
    let report = stage3_cross_module_probes();
    println!("--- stage3_cross_module_probes() ---");
    println!("{report}");

    // 8 硬墙 verify
    println!();
    println!("--- HardWallsVerify::auto_verify() ---");
    let walls = HardWallsVerify::auto_verify();
    println!("{walls}");

    // 综合判定
    println!();
    println!("--- 综合判定 ---");
    let mut all_pass = walls.all_pass();
    let mut probe_all_ok = report.all_ok;
    println!("  8 硬墙 (10 项) verify: {}", if all_pass { "✅ ALL PASS" } else { "❌ FAILED" });
    println!("  5 探针 (跨模块):     {}", if probe_all_ok { "✅ ALL OK" } else { "❌ FAILED" });

    // Honest disclosure
    if !all_pass || !probe_all_ok {
        println!();
        println!("⚠️  Stage 3 verify 部分失败, 需检查");
    } else {
        println!();
        println!("✅ Stage 3 集成验证全 PASS — 0 装 PASS 严守 100%");
    }

    println!();
    println!("================================================================");
    println!("Stage 3 跨模块集成验证 done");
    println!("================================================================");
}
