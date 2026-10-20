# RecalibrationScheduler 归档（R7-4b，ADR-0007 §2.3）

- 来源: `crates/engine/memory/src/online_calibration.rs`（同文件其余项 AdaptiveBaseline / LinearCalibration / DriftDetector / UserFeedback 保持原位）
- 归档内容: `ScheduleReport` + `RecalibrationScheduler`（struct/Default/impl）+ 5 个针对它的测试。快照 = 删除前完整源文件 `online_calibration_original.rs`（HEAD 版本，含上述代码；恢复时可从中取）。
- 原因（owner 2026-10-20「按建议来」）: 除 memory `lib.rs` re-export 外 0 生产消费（R7-1 清单），无触发接线。
- 恢复步骤:
  1. 从快照取回 `ScheduleReport`/`RecalibrationScheduler` 相关段（或 `git show <删除前commit>:crates/engine/memory/src/online_calibration.rs`）
  2. 在 memory `lib.rs` 的 `pub use online_calibration::{...}` 恢复 `RecalibrationScheduler, ScheduleReport`
  3. 重新接线须先写 ADR（R7 范围外）+ 触发点/遥测证据
- 关联: ADR-0007（Accepted）；R7-1 清单见 docs/architecture/r7-inventory-2026-10-20.md
