# CognitiveQuotaScheduler 归档（R7-4b-2，ADR-0007 §2.3）

- 来源: `crates/foundation/orchestration/src/cognitive_quota_scheduler.rs`（同文件 `CognitiveQuota` / `CognitiveTaskControlBlock` / `CognitiveInterrupt` / `CognitivePriority` 等保持原位，它们被 runtime 生产使用）
- 归档内容: `CognitiveQuotaScheduler`（struct/Default/impl/SchedulerInner）+ `MAX_TRACKED_TASKS` + 7 个专项测试与 `tcb` 测试助手；快照 = 删除前完整源文件 `cognitive_quota_scheduler_original.rs`（HEAD 版本）。
- 原因（owner 2026-10-20「按建议来」）: 除 lib re-export 外 0 生产消费（R7-1 清单），无触发接线。
- 恢复步骤:
  1. 从快照取回 scheduler 相关段（或 `git show <删除前commit>:crates/foundation/orchestration/src/cognitive_quota_scheduler.rs`）
  2. 在 `lib.rs` 的 `pub use cognitive_quota_scheduler::{...}` 恢复 `CognitiveQuotaScheduler`
  3. 重新接线须先写 ADR（R7 范围外）+ 触发点/遥测证据
- 关联: ADR-0007（Accepted）；R7-1 清单见 docs/architecture/r7-inventory-2026-10-20.md
