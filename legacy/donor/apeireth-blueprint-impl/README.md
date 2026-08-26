# apeireth-blueprint-impl

> Apeireth R20 阶段 4 估补 (RIVAL §2.4): 蓝图实装 = 4 风险类 (K-1/K-2/K-3/K-4) + 4 决策表 (D-01..D-04) + 6 实战模板 (A-F) + 5 R-Measure (R-1..R-5) + 3 评估指标 (Q1/Q2/Q3). 1 crate 打包, 跟 V0.5 命名 (apeireth-naming-v05) 互不冲突.

apeireth-blueprint-impl 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (8 src 文件 / 134 测试 + 2 Kani proof)

- `src/lib.rs` — 入口 re-export + 21 测试
- `src/risk.rs` — K-1/K-2/K-3/K-4 风险类 (4 trait) + 19 测试
- `src/decision.rs` — 4 决策表 (D-01..D-04) GuardDecision + 24 测试
- `src/template.rs` — 6 实战模板 (A-F) + 19 测试
- `src/r_measure.rs` — 5 R-Measure (R-1..R-5) 聚合 + 22 测试
- `src/q_metric.rs` — 3 评估指标 (Q1/Q2/Q3) + 18 测试
- `src/error.rs` — BlueprintError + BlueprintResult + 6 测试
- `src/organ_kani_proofs.rs` — R177 blueprint organ Kani proofs (5 测试 + 2 `#[kani::proof]`)
