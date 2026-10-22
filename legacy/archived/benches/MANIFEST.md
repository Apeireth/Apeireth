# quota-scheduler 基准归档（R7-4 收尾，ADR-0007 §2.3）

- 来源: `benches/src/quota_scheduler.rs`（被测对象 `CognitiveQuotaScheduler` 已于 R7-4b-2
  commit `d5a4c537` 归档，见 `legacy/archived/schedulers/MANIFEST.md`）。
- 归档内容: 整个基准模块快照（原文件 HEAD 版本）；`benches/src/main.rs` 同步移除
  `mod quota_scheduler;`、`BENCH_NAMES` 中的 `quota-scheduler` 项与 `run_one` 分派臂。
- 原因: 被测类型归档后该基准无法编译（workspace `cargo check --all-targets` E0432 实测），
  且无 README 性能目标行引用（`git grep 'quota-scheduler' -- README.md README.zh-CN.md docs/`
  零命中）；保留死基准会持续制造编译回归。
- 恢复步骤:
  1. 先按 `legacy/archived/schedulers/MANIFEST.md` 恢复 `CognitiveQuotaScheduler`（须先写 ADR + 触发点证据）；
  2. 从本目录取回基准文件放回 `benches/src/quota_scheduler.rs`，并在 `main.rs` 恢复
     `mod` / `BENCH_NAMES` / `run_one` 三处；
  3. `cargo check -p apeireth-bench-harness --all-targets` 验证。
- 关联: ADR-0007（Accepted）；`docs/architecture/r7-inventory-2026-10-20.md` §7。
