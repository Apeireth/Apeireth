// ⚠️ MARKER: R125-16 sub-agent (P0-3) 写错方向, 本文件待 Mavis 整合 #5 commit 时删除.
//
// 历史: R125-16 sub-agent 8/10 20:39 写了 skill_outcome.rs (StepKind / StepOutcome /
// ExecutionStatus / SkillOutcome / StepResult / ExecutionError 6 类型), 但发现
// 1) 覆盖了 R125-18 (P3-1) 已有的 skill_execution.rs (SkillExecutor + StepExecution)
//    严重违反 0 重复造轮子严守 (per 主人 10 项偏好 #6)
// 2) skill_outcome.rs 功能跟 R125-18 StepExecution 重叠
// 3) R125-18 还在跑 (P3-1 bg_bfeb840c), 它的 readmap 明确说写 SkillExecutor + StepExecution
//    4 大块 (per reports/agent-r125-18-readmap-2026-08-10.md)
//
// 处理: 立即撤销 lib.rs / Cargo.toml 改动 (pub mod skill_outcome; + pub mod skill_runner; +
// 1 段 R125-16 doc) + 覆盖 4 文件为 marker + 改 R125-16 升级方向为 skill_recommender
// (0 跟 R125-15e / R125-18 / R125-19 冲突).
//
// R125-18 跑完会重写 skill_execution.rs 为完整 SkillExecutor + 9 unit test. R125-16
// sub-agent 临时维护 1 个简化版 (5 unit test), 标明 "R125-18 readmap 1:1 简化".

// 实际 0 代码 (marker only). 整合 #5 commit 时 Mavis 删除.
