//! 性质族 6 · SAGA/CoW 回滚精确性 harness (runtime-assembly canonical
//! causal_world_model: fork/commit/rollback 三语义)。
//!
//! 对应宣称: "rollback(commit(x)) 恢复 x"。本 API 的真实形态是分支级
//! CoW + SAGA 补偿栈:
//!   - rollback 仅作用于**未提交**分支 (等价于 abort: 世界恢复 fork 基线,
//!     补偿栈按 LIFO 精确逆序交还调用方);
//!   - commit 落地为新主快照后, rollback_branch 显式拒绝 (不存在隐式撤销)。
//! 因此"恢复 x"的实现形态命题为: (a) rollback(fork(S0) ⊕ writes) ≡ S0;
//! (b) commit 持久, rollback(commit(x)) 被拒且世界不变。同构 TLA/纯模型
//! 口径见 README。
//!
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::causal_world_model::{CausalWorldModel, SagaCompensatingAction};

fn saga(action_id: &str) -> SagaCompensatingAction {
    SagaCompensatingAction {
        action_id: action_id.to_string(),
        forward_action_name: "do".to_string(),
        compensation_action_name: "undo".to_string(),
        payload: std::collections::HashMap::new(),
    }
}

fn bounded_string<const N: usize>() -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(N);
    for _ in 0..N {
        bytes.push(kani::any::<u8>());
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 证明: 分支回滚精确恢复基线 —— 任意 ≤2 笔投机写 + ≤2 个 SAGA 补偿动作后
/// rollback_branch, (a) 世界快照与 fork 前逐字段相等; (b) 返回的补偿序列
/// 恰为入栈顺序的 LIFO 逆序; (c) 回滚后分支关闭 (拒绝再写)。
/// 边界: 路径/动作 id 为具体短串 (符号键会触发 SipHash 符号展开),
/// 写/补偿各 ≤2 (有界), unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_saga_rollback_restores_base_lifo() {
    let mut world = CausalWorldModel::new("s0");
    let base = world.current_snapshot().expect("根快照存在").clone();
    world.fork_branch("b1").expect("fork 成功");

    let writes = 1 + (kani::any::<usize>() % 2);
    for i in 0..writes {
        let path = if i == 0 { "p" } else { "q" };
        world
            .record_speculative_write("b1", path, "h")
            .expect("活跃分支可写");
    }

    let comps_pushed = 1 + (kani::any::<usize>() % 2);
    world
        .push_saga_compensation("b1", saga("a1"))
        .expect("入栈");
    if comps_pushed == 2 {
        world
            .push_saga_compensation("b1", saga("a2"))
            .expect("入栈");
    }

    let comps = world.rollback_branch("b1").expect("活跃分支可回滚");

    assert_eq!(comps.len(), comps_pushed, "补偿动作全量交还");
    assert_eq!(
        comps[0].action_id,
        if comps_pushed == 2 { "a2" } else { "a1" }
    );
    if comps_pushed == 2 {
        assert_eq!(comps[1].action_id, "a1", "SAGA 补偿按 LIFO 逆序交还");
    }
    assert_eq!(
        world.current_snapshot().expect("世界存在"),
        &base,
        "rollback 后世界逐字段恢复 fork 基线"
    );
    assert!(
        world.record_speculative_write("b1", "z", "hz").is_err(),
        "回滚后分支关闭, 拒绝再写"
    );
}

/// 证明: commit 持久、rollback(commit(x)) 显式拒绝 —— (a) commit 产出的新
/// 主快照包含投机写 x 且保留未修改基线条目 (CoW 合并); (b) 对已提交分支的
/// rollback_branch 返回 Err 且世界快照不变 —— 不存在绕过审批/审计的隐式撤销
/// (commit 后无"恢复旧值"路径)。
/// 边界: 路径为具体短串, checksum 值任意 ≤2 字节串 (符号值, 键具体),
/// 两段式 (建基线 → 提交目标分支), unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_saga_commit_durable_rollback_rejected() {
    let mut world = CausalWorldModel::new("s0");

    // 建立含内容的基线 s1 = {"b": "cb"}。
    world.fork_branch("b0").expect("fork 成功");
    world
        .record_speculative_write("b0", "b", "cb")
        .expect("活跃分支可写");
    let committed = world.commit_branch("b0", "s1", 1).expect("提交成功");
    assert_eq!(
        committed.file_checksums.get("b").map(String::as_str),
        Some("cb")
    );
    assert_eq!(
        world.current_snapshot().expect("世界存在").snapshot_id,
        "s1"
    );

    // 目标分支 b1 基于 s1, 写入符号 checksum 值 x。
    world.fork_branch("b1").expect("fork 成功");
    let x = bounded_string::<2>();
    world
        .record_speculative_write("b1", "p", &x)
        .expect("活跃分支可写");
    let s2 = world.commit_branch("b1", "s2", 2).expect("提交成功");

    assert_eq!(
        s2.file_checksums.get("p").map(String::as_str),
        Some(x.as_str()),
        "commit(x) 的效果 x 落地"
    );
    assert_eq!(
        s2.file_checksums.get("b").map(String::as_str),
        Some("cb"),
        "未修改基线条目随 CoW 保留"
    );
    assert_eq!(
        s2.parent_snapshot_id.as_deref(),
        Some("s1"),
        "快照链指回基线"
    );

    // rollback(commit(x)): 显式拒绝, 世界不变 (commit 持久)。
    let before = world.current_snapshot().expect("世界存在").clone();
    assert!(
        world.rollback_branch("b1").is_err(),
        "已提交分支不可回滚 (无隐式撤销路径)"
    );
    assert_eq!(
        world.current_snapshot().expect("世界存在"),
        &before,
        "拒绝回滚后世界不变, commit 持久"
    );
}
