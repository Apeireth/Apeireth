# apeireth-test

> apeireth-test — TestCase + RetryPolicy + SuiteSummary + Budget + flaky_cases (R23 P1 #5 实质化, lib.rs:38). 实质 1 模块 (lib.rs 包含全部真 retry counting / 真 exponential backoff / 真 budget accounting). 2 private 测试 mod (property_tests + organ_kani_proofs, 不算 src 模块). **0 改 workspace.version**. 旧 README "R23 6 module test 子模块" 系 R23 设计稿残影, 该 crate 从未按 6 模块拆分 (代码 src/ 仅 3 .rs 文件: lib.rs / organ_kani_proofs.rs / property_tests.rs).

apeireth-test 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
