//! 因果世界模型 CoW 基准: 假设分支 fork + 快照差分。
//!
//! 口径 (每个操作):
//! 1. `fork_branch` 从当前主快照 fork 一个假设分支 (CoW: 分支只记改动);
//! 2. `record_speculative_write` × 10 记录推测写;
//! 3. `commit_branch` 提交为新主快照 (在快照上合并改动 = CoW 快照差分);
//! 4. 手工差分: 对比新快照与基快照的校验和表, 收集改动项。
//!
//! 基快照预置 200 个文件条目。被测对象为进程内纯内存实现 (无磁盘 I/O)。

use std::collections::HashMap;
use std::hint::black_box;

use apeireth_runtime_assembly::canonical::causal_world_model::CausalWorldModel;

use crate::support::{measure_per_op, BenchConfig, Outcome, Target};

/// 基快照文件数。
const BASE_FILES: usize = 200;
/// 每操作推测写次数。
const WRITES: usize = 10;
/// 每样本操作数。
const BATCH: usize = 10;

/// 运行因果世界模型 CoW 基准。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let mut model = CausalWorldModel::new("snap-root");

    // 预置基快照: 200 个文件条目。
    model.fork_branch("seed").expect("fork 种子分支");
    for i in 0..BASE_FILES {
        model
            .record_speculative_write("seed", &format!("file-{i:04}"), &format!("sum-{i:04}"))
            .expect("种子写");
    }
    model
        .commit_branch("seed", "snap-base", 0)
        .expect("提交基快照");

    let counter = std::cell::Cell::new(0u64);
    let samples_ns = measure_per_op(cfg.samples(3), cfg.samples(40), BATCH, || {
        let n = counter.get();
        counter.set(n + 1);
        let branch = format!("branch-{n}");
        let snapshot = format!("snap-{n}");
        // 差分前像: fork 前的主快照 (快照差分的被比较对象)。
        let before = model.current_snapshot().expect("fork 前主快照").clone();
        model.fork_branch(&branch).expect("fork 分支");
        for i in 0..WRITES {
            model
                .record_speculative_write(
                    &branch,
                    &format!("file-{i:04}"),
                    &format!("hash-{n}-{i}"),
                )
                .expect("推测写");
        }
        let committed = model
            .commit_branch(&branch, &snapshot, n)
            .expect("提交分支");
        // 快照差分: 新快照 vs 前像 (改动项收集)。
        let diff: HashMap<&String, &String> = committed
            .file_checksums
            .iter()
            .filter(|(path, sum)| {
                before
                    .file_checksums
                    .get(*path)
                    .map(|b| b != *sum)
                    .unwrap_or(true)
            })
            .collect();
        black_box(diff.len());
    });

    vec![Outcome::from_ns(
        "causal-cow",
        "因果世界模型 CoW (假设分支 fork + 快照差分)",
        "fork_branch + 10 次推测写 + commit_branch(快照合并) + 新旧快照差分收集 单次",
        "us",
        BATCH,
        cfg.samples(3),
        samples_ns,
        Some(Target {
            label: "< 1.0 ms".to_string(),
            value: 1000.0,
        }),
        "基快照 200 文件条目; 纯内存无磁盘 I/O; 每样本 = 10 个操作的平均; 分支/快照对象保留被测语义 (已提交分支留在模型内)",
    )]
}
