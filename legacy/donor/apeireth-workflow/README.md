# apeireth-workflow

> Apeireth R152: Temporal-style workflow engine (Activity trait + WorkflowRunner + EventHistory). Borrows temporalio/temporal (13K stars) design, self-impl 0 引外部 dep.

apeireth-workflow 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (2 src 文件 / 23 测试 + 2 Kani proof + 3 集成)

- `src/lib.rs` — Temporal-style workflow engine (Activity trait + WorkflowRunner + EventHistory + 18 测试)
- `src/organ_kani_proofs.rs` — workflow organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/test_mcp_in_process.rs` (3)
- 例: `examples/workflow_demo.rs`
