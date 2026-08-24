//! Stage 3 端到端 smoke runnable example
//!
//! 跨 6 子模块 (bridge / bridge_pool / asi_modules / r11_compat / type_convert / python_bindings) 协同校验
//!
//! ```bash
//! cargo run -p apeireth-pybridge --example stage3_e2e_run
//! ```

use apeireth_pybridge::{
    cross_language_smoke_check, end_to_end_smoke_check, stage3_cross_module_count,
    stage3_e2e_smoke, stage3_e2e_summary,
};

fn main() {
    println!("================================================================");
    println!("Stage 3 端到端 smoke (per decision-58 §2.1 P10-3)");
    println!("================================================================");
    println!();

    // Stage 3 端到端 smoke
    let s3 = stage3_e2e_smoke();
    println!("--- stage3_e2e_smoke() ---");
    println!("{s3}");

    // Stage 3 跨模块协同
    println!();
    println!("--- stage3_cross_module_count() ---");
    let (ok, total) = stage3_cross_module_count();
    println!("跨模块协同: {ok}/{total} OK");

    // Stage 3 summary
    println!();
    println!("--- stage3_e2e_summary() ---");
    println!("{}", stage3_e2e_summary());

    // 跟 Stage 2 已有 API 比较
    println!();
    println!("--- Stage 2 已存 API (per decision-57 §2.1 P10-2) ---");
    println!("end_to_end_smoke_check:");
    let s2_e2e = end_to_end_smoke_check();
    println!(
        "  r11: {} ({} modules)",
        s2_e2e.r11_compat_version, s2_e2e.r11_module_count
    );
    println!(
        "  pool: max_idle={} idle_timeout={}s",
        s2_e2e.pool_max_idle, s2_e2e.pool_idle_timeout_secs
    );

    println!();
    println!("cross_language_smoke_check:");
    let s2_xlang = cross_language_smoke_check();
    println!(
        "  python_available: {}, bidirectional_ok: {}",
        s2_xlang.python_available, s2_xlang.bidirectional_ok
    );

    println!();
    println!("================================================================");
    println!("Stage 3 端到端 smoke done");
    println!("================================================================");
}
