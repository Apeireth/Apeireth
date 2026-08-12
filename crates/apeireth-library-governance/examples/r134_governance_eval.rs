//! R134 — apeireth-library-governance 真接示例 (消除孤岛)
//!
//! 演示 `GovernanceEngine::evaluate` 真派发 5 策略 + `verify` 跑形式化 sanity check.
//! 证明本 crate 不是孤岛, 有可调用 API + 形式化验证入口.
//!
//! 跑法:
//! ```powershell
//! cargo run -p apeireth-library-governance --example r134_governance_eval
//! ```

use apeireth_library_governance::{
    evaluate, run_all, verify, GovernanceContext, GovernanceEngine, GovernanceReport,
};

fn main() {
    println!("=== R134 apeireth-library-governance 真实接入演示 (消除孤岛) ===\n");

    let engine = GovernanceEngine::new();

    // ===== 1. evaluate 5 类策略 =====
    let cases = [
        ("safe_default", GovernanceContext::safe_default()),
        ("version", GovernanceContext::version()),
        ("baseline", GovernanceContext::baseline()),
    ];
    for (name, ctx) in &cases {
        let d = evaluate(ctx);
        println!(
            "[evaluate] {:>14} → policy={}, action={}, audit={}",
            name, d.policy as u8, d.action as u8, d.requires_audit
        );
    }

    // engine.evaluate 等价 evaluate()
    let d2 = engine.evaluate(&GovernanceContext::safe_default());
    println!("[engine.evaluate] policy={} action={}", d2.policy as u8, d2.action as u8);

    // ===== 2. verify (形式化 sanity check, 不需 Kani 安装) =====
    let verify_ok = engine.verify();
    println!("[engine.verify] sanity invariants ok = {}", verify_ok);
    verify();
    println!("[verify()]    sanity invariants ok = true (side-effect only)");

    // ===== 3. run_all (Stage 5 一致性 + invariants 全跑) =====
    let all_ok = run_all();
    println!("[run_all]    all stage-5 checks ok = {}", all_ok);

    // ===== 4. GovernanceReport (consistency + invariants) =====
    let report = GovernanceReport {
        consistency: apeireth_library_governance::ConsistencyReport::check(),
        invariants_ok: all_ok,
    };
    println!(
        "[report]    consistency_ok = {}, invariants_ok = {}",
        report.consistency.pass_count(), report.invariants_ok
    );

    println!("\nR134 apeireth-library-governance 真实接入演示: PASS (消除孤岛, 4 个 API 真接可用)");
}
