//! apeireth-constraint 演示 — 12 键 verdict cache + 5 重守门
//!
//! 运行: `cargo run -p apeireth-constraint --example constraint_demo`

// v15 命名修正: verify_all_five_gates 保留为向后兼容别名, 示例演示 v14 旧 API
#![allow(deprecated)]

use apeireth_constraint::{
    multi_ai_consensus, physical_isolation_check, reflection_period_audit, runtime_intercept,
    verify_all_five_gates, verify_at_compile_time, ConstraintEngine, FiveGates, GateVerdict,
    HardCodeConstraint, PhilosophyKeyAccess, TwelveKeysHardcode, VerdictCache,
};
use apeireth_core::{Action, ActionTarget, PhilosophyVerdict, RiskLevel};

fn main() {
    println!("=== apeireth-constraint 演示 (P12 — v4.1 新增) ===\n");

    // 1. 编译时 hardcode 验证 — `const` 调用不 panic 即通过
    let twelve = verify_at_compile_time();
    println!("[守门 1] 编译时 hardcode: ALL_TWELVE_KEYS.len() = {twelve}");

    // TwelveKeysHardcode 的 const_assert 二次断言
    let _ = <TwelveKeysHardcode as HardCodeConstraint>::const_assert(12);

    // 2. 创建引擎 + 列出 12 键 (复用 apeireth-core ALL_TWELVE_KEYS)
    let engine = ConstraintEngine::new();
    let keys = <ConstraintEngine as PhilosophyKeyAccess>::all_twelve_keys();
    println!("\n[12 键清单] 共 {} 键 (复用 apeireth-core):", keys.len());
    for (i, key) in keys.iter().enumerate() {
        println!(
            "  #{} {:?} (PHL-{:02}, {})",
            i + 1,
            key,
            key.group_id(),
            key.description()
        );
    }

    // 3. 演示 VerdictCache — 写入 + 查询 + 清空
    let mut cache = VerdictCache::new();
    cache.put("demo-001", PhilosophyVerdict::Allow);
    cache.put(
        "demo-002",
        PhilosophyVerdict::Block(apeireth_core::PhilosophyKey::NotSafe),
    );
    println!("\n[VerdictCache] 写入 2 条, cache.len() = {}", cache.len());
    match cache.get("demo-001") {
        Some(PhilosophyVerdict::Allow) => println!("  demo-001 -> Allow"),
        Some(PhilosophyVerdict::Block(k)) => println!("  demo-001 -> Block({:?})", k),
        None => println!("  demo-001 -> None"),
    }

    // 4. 演示 5 重守门 — 一次跑完
    let action = Action {
        id: "demo-action-1".into(),
        description: "演示用普通 action".into(),
        risk_level: RiskLevel::Low,
        target: ActionTarget::NormalAction("demo".into()),
    };

    println!("\n[5 重守门] 演示 action = {}", action.id);
    let v1 = engine.gate1_compile_time();
    let v2 = runtime_intercept(&engine, &action);
    let v3 = multi_ai_consensus(&engine, &action);
    let v4 = physical_isolation_check(&engine, &action);
    let v5 = reflection_period_audit(&engine, &action);
    println!("  守门 1 (编译时 hardcode): {:?}", v1);
    println!("  守门 2 (运行时拦截):      {:?}", v2);
    println!("  守门 3 (多 AI 一致):      {:?}", v3);
    println!("  守门 4 (物理隔离 HA):     {:?}", v4);
    println!("  守门 5 (反思期审计):      {:?}", v5);

    // 5. 一次性跑完 5 重守门 — 默认拒绝 (主 17:58 不假装安全)
    println!("\n[verify_all_five_gates] 默认拒绝模式:");
    match verify_all_five_gates(&engine, &action) {
        Ok(()) => println!("  全部通过 ✓"),
        Err(e) => println!("  拒绝 ✗ (错误: {})", e),
    }

    // 6. 缓存 Allow 后再跑 — 守门 1-4 通过, 守门 5 默认 Block (P19 待接入)
    let mut engine_with_cache = ConstraintEngine::new();
    engine_with_cache
        .cache_mut()
        .put("demo-action-1", PhilosophyVerdict::Allow);
    println!("\n[verify_all_five_gates] 缓存 Allow 后:");
    match verify_all_five_gates(&engine_with_cache, &action) {
        Ok(()) => println!("  全部通过 ✓"),
        Err(e) => println!("  拒绝 ✗ (错误: {})", e),
    }

    // 7. GateVerdict 相等性测试 (round12-09 修复 clippy::eq_op)
    let g_pass_a = GateVerdict::Pass;
    let g_pass_b = GateVerdict::Pass;
    let g_fail = GateVerdict::Block("test".into());
    println!("\n[GateVerdict] Pass_a == Pass_b: {}", g_pass_a == g_pass_b);
    println!("[GateVerdict] Pass != Block: {}", g_pass_a != g_fail);

    println!("\n=== 演示结束 ===");
}
