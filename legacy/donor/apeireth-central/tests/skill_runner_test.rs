// ⚠️ MARKER: R125-16 sub-agent (P0-3) 写错方向, 本文件待 Mavis 整合 #5 commit 时删除.
//
// 历史: R125-16 sub-agent 8/10 20:39 写了 tests/skill_runner_test.rs (8 集成 test, 测
// SkillRunner), 但发现
// 1) 覆盖了 R125-18 (P3-1) 已有的 skill_execution.rs (SkillExecutor) - 严重违反 0
//    重复造轮子严守
// 2) test 测的 SkillRunner 跟 R125-18 SkillExecutor + R125-19 5 phase state machine
//    重叠
// 3) R125-18 还在跑 (P3-1 bg_bfeb840c) + R125-19 已 done (P3-2 bg_68dcfdb9,
//    tests/skill_executor_test.rs 8 集成 test 已写)
//
// 处理: 立即撤销 + 覆盖本文件为 marker + 改 R125-16 升级方向为 skill_recommender.
// R125-18 跑完会重写 skill_execution.rs + tests/skill_execution_test.rs (12 集成 test,
// per readmap §4.1). R125-19 已有 tests/skill_executor_test.rs (8 集成 test, 写于
// apeireth-skills crate).

// 实际 0 测试 (marker only). 整合 #5 commit 时 Mavis 删除.
