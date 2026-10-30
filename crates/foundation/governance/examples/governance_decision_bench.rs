//! **治理决策路径基准**（四层验收 **bench 层** · 2026-10-30）。
//!
//! # 为什么是它
//!
//! 本仓要凑齐**第一条「四层齐全」的 claim**（ADR-0011 R6-C 目标：「至少 1 条 claim 走通四层」），
//! 候选是**生产审批/治理决策能力**，四层里：
//!
//! * **formal** ✅ 已有 —— Kani 治理单调性族（4 件，含 `kani_governance_fail_closed_no_bypass`
//!   「RequireApproval 不得跳过审批」）；
//! * **scenario** ✅ 已有 —— `crates/adapters/gateway/tests/canonical_entry_e2e.rs`
//!   的 `http_pending_approval_can_be_approved_without_double_execution`
//!   （202 待审 → resolve 批准 → **恰好执行 1 次** → 重复 resolve 400「already resolved」）；
//! * **bench** ⬜ **本文件补上**（可重跑 + 出数字）；
//! * **signoff** ⬜ 由 owner 的真机记录如实回填。
//!
//! # 量什么
//!
//! **生产**决策入口 [`PermissionPolicy::decision_for_capability`] 的**吞吐与延迟**，
//! 覆盖三条判定路径：
//!
//! | 路径 | 语义（与 Kani 族同口径）|
//! |---|---|
//! | `allow` | 已授权且不需审批 ⇒ `Allow` |
//! | `require_approval` | 已授权但标记需人工审批 ⇒ `RequireApproval` |
//! | `deny` | 未授权 ⇒ `Deny`（**默认拒绝**）|
//!
//! # 口径（**可重跑**）
//!
//! * 固定工作量、**无随机**；每档重复多轮取 **p50 / p95**；
//! * **零新增依赖**（只用 `std::time`）；
//! * 只报**相对**趋势（跨机绝对 ns 不可比）与**判定分布**（三条路径各占多少）
//!   ⇒ 分布是**正确性侧**的证据（确认三条路径都被走到，不是只跑了一条）。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-governance --example governance_decision_bench --release --locked
//! ```

use apeireth_governance::permission::PermissionPolicy;
use apeireth_governance::Decision;
use std::time::Instant;

/// 每轮决策次数。
const DECISIONS_PER_ROUND: usize = 20_000;
/// 重复轮数（取中位数 ⇒ 抗抖动）。
const ROUNDS: usize = 9;

fn build_policy(grants: usize, approvals: usize) -> PermissionPolicy {
    let mut policy = PermissionPolicy::new();
    for index in 0..grants {
        policy.grant(apeireth_governance::Permission::ExecuteTool(format!(
            "tool.cap-{index}"
        )));
    }
    for index in 0..approvals {
        policy.require_approval_for(format!("tool.cap-{index}"));
    }
    policy
}

/// 跑一轮固定序列，返回耗时（秒）+ 三条判定路径的命中数。
fn one_round(policy: &PermissionPolicy, grants: usize) -> (f64, (usize, usize, usize)) {
    let started = Instant::now();
    let (mut allow, mut require, mut deny) = (0usize, 0usize, 0usize);
    for index in 0..DECISIONS_PER_ROUND {
        // 三条路径轮转：授权且需审批 / 授权不需审批 / 未授权。
        let capability = if index % 3 == 0 {
            format!("tool.cap-{}", index % grants.max(1))
        } else if index % 3 == 1 {
            format!("tool.cap-{}", grants + index)
        } else {
            format!("tool.denied-{index}")
        };
        match policy.decision_for_capability(&capability) {
            Decision::Allow => allow += 1,
            Decision::RequireApproval { .. } => require += 1,
            Decision::Deny { .. } => deny += 1,
        }
    }
    (started.elapsed().as_secs_f64(), (allow, require, deny))
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let index = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn main() {
    println!("════════ 治理决策路径基准（bench 层）════════");
    println!("每轮 {DECISIONS_PER_ROUND} 次决策 · 重复 {ROUNDS} 轮取中位数 · 三条判定路径轮转");
    println!();

    // 档位：授权能力数（含需审批的能力数）⇒ 看**策略规模**对决策延迟的影响。
    let sizes = [(16usize, 4usize), (64, 16), (256, 64), (1_024, 256)];

    println!(
        "{:>12} {:>12} {:>12} {:>10}   {}",
        "授权/需审批", "p50 (µs/千次)", "p95 (µs/千次)", "相对基线", "判定分布 allow/require/deny"
    );
    println!("{}", "-".repeat(78));

    let mut baseline: Option<f64> = None;
    for (grants, approvals) in sizes {
        let policy = build_policy(grants, approvals);
        // 预热。
        let _ = one_round(&policy, grants);

        let mut samples: Vec<f64> = Vec::with_capacity(ROUNDS);
        let mut distribution = (0usize, 0usize, 0usize);
        for _ in 0..ROUNDS {
            let (seconds, counts) = one_round(&policy, grants);
            samples.push(seconds);
            distribution = counts;
        }
        samples.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        let p50 = percentile(&samples, 0.5);
        let p95 = percentile(&samples, 0.95);
        let relative = match baseline {
            None => {
                baseline = Some(p50);
                "—".to_string()
            }
            Some(base) if base > 0.0 => format!("{:.2}×", p50 / base),
            Some(_) => "—".to_string(),
        };
        println!(
            "{:>12} {:>12.2} {:>12.2} {:>10}   {}/{}/{}",
            format!("{grants}/{approvals}"),
            p50 * 1_000_000.0 / (DECISIONS_PER_ROUND as f64 / 1_000.0),
            p95 * 1_000_000.0 / (DECISIONS_PER_ROUND as f64 / 1_000.0),
            relative,
            distribution.0,
            distribution.1,
            distribution.2
        );
    }

    println!();
    println!("⚠️ 口径与边界（如实）：");
    println!("  ① 只测**纯判定**（`decision_for_capability`）⇒ 不含审批**等待**与持久化 ⇒ 真实端到端更慢；");
    println!("  ② 「判定分布」是**正确性侧**证据：确认 allow / require_approval / deny **三条路径都被走到**；");
    println!("  ③ 只报**相对**趋势（跨机绝对值不可比）⇒ 结论只讲**随策略规模的增长**；");
    println!("  ④ 零新增依赖（只用 `std::time`）⇒ 任何机器可重跑。");
}
