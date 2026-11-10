//! **审批状态机基准**（四层验收的 **bench 层** · 2026-10-30）。
//!
//! # 为什么需要它
//!
//! `docs/status/verification-ledger.yaml` 现有 5 条分层条目里：
//! **formal**（Kani）2 条 · **scenario** 3 条 · **bench** **0 条** · **signoff** 0 条
//! ⇒ ⇒ 「四层齐全的 claim = 0」里，**bench 是唯一我可以自己补的一层**
//! （formal 要 Kani · scenario 要人跑 · signoff 只能真人签）。
//!
//! # 本 bench 量什么（**有发现**）
//!
//! `ResearchApprovalMachine` 的记录容器是 **`KaniLinearMap`** —— 为 Kani 的**有界模型检查**
//! 友好而**故意用线性表** ⇒ ⇒ 生产里 `next()` 的查找是 **O(n)**（n = 活跃审批数）。
//!
//! ⇒ 本 bench **量化**这条代价：随活跃审批数增长，单次决策延迟如何变化
//! ⇒ ⇒ 结论若为「显著线性」⇒ 那是一条**真实的生产侧代价**（Kani 友好性 ↔ 生产性能的取舍）
//! ⇒ ⇒ 应被**记录**，而不是藏起来。
//!
//! # 口径（**可重跑**）
//!
//! * 固定工作量（同一组操作序列，**无随机**）⇒ 两次跑出的**相对**结论一致；
//! * 每档重复多轮 ⇒ 取 **中位数**（p50）与 **p95**，单次抖动不主导结论；
//! * 只报**相对**趋势与比值（×N），**不**报绝对 ns（CI 机器差异大）；
//! * **零新增依赖**（只用 `std::time`）。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-runtime --example research_approval_bench --release --locked
//! ```

use apeireth_runtime::canonical::research_approval_sm::{
    ResearchApprovalEvent, ResearchApprovalMachine,
};
use std::time::Instant;

/// 每档的**重复轮数**（取中位数 ⇒ 抗抖动）。
const ROUNDS: usize = 9;
/// 每轮的操作次数。
const OPS_PER_ROUND: usize = 2_000;
/// 活跃审批数的档位。
const SIZES: [usize; 5] = [1, 10, 100, 1_000, 5_000];

/// 跑一轮固定操作序列，返回该轮耗时（秒）。
fn one_round(size: usize) -> f64 {
    let mut machine = ResearchApprovalMachine::new();
    for index in 0..size {
        machine.create(format!("a-{index}"), None);
    }
    let started = Instant::now();
    // 固定序列：对每条审批做「创建 → 批准」；只取**合法转移**路径。
    // ⚠️ 事件用 `Approve`（Pending → Approved 是合法转移）⇒ 不测非法转移（那是 scenario 层的活）。
    for index in 0..size {
        let id = format!("a-{index}");
        let _ = machine.next(&id, ResearchApprovalEvent::Approve);
    }
    // 追加固定量的重复查询（模拟同一轮里的多次决策）。
    let extra = OPS_PER_ROUND.saturating_sub(size);
    for step in 0..extra {
        let id = format!("a-{}", step % size.max(1));
        let _ = machine.next(&id, ResearchApprovalEvent::Complete);
    }
    started.elapsed().as_secs_f64()
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let index = ((sorted.len() as f64 - 1.0) * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn main() {
    println!("════════ 审批状态机基准（bench 层）════════");
    println!("工作量：每档 {OPS_PER_ROUND} 次决策 · 每档重复 {ROUNDS} 轮取中位数");
    println!("容器：`KaniLinearMap`（为 Kani 有界检查而用的**线性表**）⇒ 预期 O(n)");
    println!();

    println!(
        "{:>8} {:>12} {:>12} {:>10}",
        "活跃审批数", "p50 (ms)", "p95 (ms)", "相对基线"
    );
    println!("{}", "-".repeat(46));

    let mut baseline: Option<f64> = None;
    for size in SIZES {
        // 预热（避免首次分配计入）。
        let _ = one_round(size.min(10));

        let mut samples: Vec<f64> = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            samples.push(one_round(size));
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
            "{:>8} {:>12.3} {:>12.3} {:>10}",
            size,
            p50 * 1_000.0,
            p95 * 1_000.0,
            relative
        );
    }

    println!();
    println!("⚠️ 口径与边界（如实）：");
    println!("  ① 只测**合法转移**的查询路径 ⇒ 非法转移（Safety 拒绝）属 scenario 层，不在此测；");
    println!("  ② 只报**相对**趋势（机器差异大 ⇒ 绝对 ns 不可跨机比）；");
    println!("  ③ 这是**研究用**状态机（`research_approval_sm`）⇒ 其容器为 Kani 友好而线性；");
    println!("     ⇒ **生产审批路径若复用此容器**，成本须另行实测（本 bench 不代表生产）；");
    println!("  ④ 零新增依赖（只用 `std::time`）⇒ 可在任何机器重跑。");
}
