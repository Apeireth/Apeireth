//! 性质族 2 · 记忆守恒 harness (对应"记忆不会丢")。
//!
//! 真实 protect/forget/retention API 的转移规则落在 SQLite 路径
//! (memory_governance::forget_episode_impl 的 protected/AlreadyForgotten 前置检查、
//! retention::sweep_session 的 protected 跳过), Kani 无法驱动 DB; 本族在
//! 同 crate 的纯 API `bitemporal_graph::BitemporalGraph` (撤回 = 墓碑版本,
//! 永不物理删除) 上证明守恒命题的纯模型投影, SQL 侧契约由
//! research/verification/tla/ProtectForget.tla 全状态空间覆盖 (见 README)。
//!
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::bitemporal_graph::{BitemporalFact, BitemporalGraph};

/// 双时态切片中属于键 (s2, p2) 的可见版本视图。
fn key_view<'a>(graph: &'a BitemporalGraph, t_ask: u64, t_belief: u64) -> Vec<&'a BitemporalFact> {
    graph
        .facts_as_of(t_ask, t_belief)
        .into_iter()
        .filter(|f| f.subject == "s2" && f.predicate == "p2")
        .collect()
}

/// 严格递增的信念时刻序列 (单调时钟假设, 见各命题边界)。
fn times() -> (u64, u64, u64) {
    let t0: u64 = kani::any();
    if t0 > u64::MAX - 32 {
        return (0, 1, 2); // 早退守卫: 防有界时刻构造溢出 (等价 kani::assume)
    }
    let t1 = t0 + 1 + (kani::any::<u64>() % 8);
    let t2 = t1 + 1 + (kani::any::<u64>() % 8);
    (t0, t1, t2)
}

/// 证明: 撤回 (forget) 永不物理删除旧版本 —— 任意 ≤2 次写入后撤回,
/// 先前每个版本仍在其自身信念时刻的切片中可见 (append-only 守恒)。
/// 边界: 键具体短串 ("s1"/"p1", 避免符号键入 SipHash), 时刻严格递增
/// (单调时钟), unwind 64。
#[kani::proof]
#[kani::unwind(64)]
fn kani_memory_retract_never_loses_versions() {
    let (t0, t1, t2) = times();
    let mut g = BitemporalGraph::new();
    let v1 = g.upsert_fact("s1", "p1", "o1", 1.0, t0);
    let v2 = g.upsert_fact("s1", "p1", "o2", 1.0, t1);
    g.retract_fact("s1", "p1", t2, "r");

    assert!(
        g.beliefs_as_of(t0).iter().any(|f| f.id == v1.id),
        "撤回后首版本仍在其信念切片 (记忆不会丢)"
    );
    assert!(
        g.beliefs_as_of(t1).iter().any(|f| f.id == v2.id),
        "撤回后次版本仍在其信念切片 (记忆不会丢)"
    );
}

/// 证明: 二次撤回 (forget) 幂等 —— 对任意查询时刻 (t_ask, t_belief),
/// 撤回两次与撤回一次的任意双时态切片完全一致 (当前态无变化)。
/// 边界: 撤回时刻严格递增; 幂等性在 facts_as_of/retrospective 维度成立,
/// beliefs_as_of 属 append-only 审计日志维度, 按设计逐次记录每次操作;
/// unwind 64。
#[kani::proof]
#[kani::unwind(64)]
fn kani_memory_forget_idempotent_observational() {
    let (t0, t1, t2) = times();
    let t3 = t2 + 1 + (kani::any::<u64>() % 8);
    let mut g = BitemporalGraph::new();
    g.upsert_fact("s1", "p1", "o1", 1.0, t0);
    g.upsert_fact("s2", "p2", "o2", 1.0, t1);
    g.retract_fact("s1", "p1", t2, "r");

    let once = g.clone();
    g.retract_fact("s1", "p1", t3, "r");

    let t_ask: u64 = kani::any();
    let t_belief: u64 = kani::any();
    assert_eq!(
        once.facts_as_of(t_ask, t_belief),
        g.facts_as_of(t_ask, t_belief),
        "二次 forget 后任意事实切片无变化"
    );
    assert_eq!(
        once.retrospective(t_ask, t_belief),
        g.retrospective(t_ask, t_belief),
        "二次 forget 后任意双时态切片无变化"
    );
}

/// 证明: forget/retention 路径只作用于目标键 —— 目标键上的任意
/// 撤回/演化操作不改变任意其它键 (protect 语义的纯模型投影: 被保护条目
/// 不在遗忘路径的作用域内) 在任意双时态切片上的可见集。
/// 边界: 两对具体短键, 每键 ≤2 次写入 + 1 次目标键操作, 时刻严格递增;
/// SQL 侧 protected 标志的守恒由 TLA+ ProtectForget 模型覆盖; unwind 64。
#[kani::proof]
#[kani::unwind(64)]
fn kani_memory_forget_confined_to_target_key() {
    let (t0, t1, t2) = times();
    let mut g = BitemporalGraph::new();
    g.upsert_fact("s1", "p1", "o1", 1.0, t0);
    g.upsert_fact("s2", "p2", "o2", 1.0, t1);
    let baseline = g.clone();

    // 目标键 (s1,p1) 上的任意遗忘/演化操作。
    if kani::any::<bool>() {
        g.upsert_fact("s1", "p1", "o3", 1.0, t2);
    } else {
        g.retract_fact("s1", "p1", t2, "r");
    }

    let t_ask: u64 = kani::any();
    let t_belief: u64 = kani::any();
    assert_eq!(
        key_view(&baseline, t_ask, t_belief),
        key_view(&g, t_ask, t_belief),
        "无关键 (protect 投影) 不受目标键 forget 影响"
    );
}
