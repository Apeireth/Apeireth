# apeireth-value

> Apeireth 价值器官 (A11.3 落点 — R14 Phase 4 动机/价值评估: ValueEvaluation + ValuePrioritization + 5 层原则洋葱一致性 + motivation_score 0.85 门槛)

apeireth-value 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (5 src 文件 / 56 测试 + 2 Kani proof + 15 集成)

- `src/lib.rs` — 价值器官 facade 入口 (ValueEvaluation + ValuePrioritization re-export) + 14 测试
- `src/evaluation.rs` — ValueEvaluation 主逻辑 (per A11.3) + 12 测试
- `src/prioritization.rs` — ValuePrioritization + 5 层原则洋葱一致性 + 8 测试
- `src/onion_consistency.rs` — 原则洋葱 5 层校验 (per core/onion 衔接) + 12 测试
- `src/organ_kani_proofs.rs` — value organ Kani proofs (R177, 10 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/value_tests.rs` (15)
- 例: `examples/value_demo.rs`
