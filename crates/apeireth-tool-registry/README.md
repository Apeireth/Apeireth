# apeireth-tool-registry

> Apeireth R17 战役 2-1: 工具注册中心 (6 类 enum + 5 轴正交 + token 预算三层 + notify 热加载 + 异步任务推送, 借鉴 §6.2.1 #12/#13 + §6.2.2 #15 + agentManager.js chokidar (origin: open-source))

apeireth-tool-registry 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (13 src 文件 / 136 测试 + 2 Kani proof + 24 集成)

- `src/lib.rs` — 入口 + ToolKind 6 类 enum re-export + 5 测试
- `src/registry.rs` — 核心注册表 (RW map + 6 ToolKind 分桶 + notify 5 hot-reload watcher) + 13 测试
- `src/trait_def.rs` — ToolBridge / ToolSpec / ToolHandler trait 定义 + 3 测试
- `src/types.rs` — 6 类 ToolKind + 5 正交轴 (TriggerAxis / AwaitingAxis / ResidentAxis / TransportAxis / OutputAxis) + 16 测试
- `src/catalog.rs` — ToolBridge catalog (含 11 个工具子 crate 注册) + 4 测试
- `src/classifier.rs` — 风险分类器 (VCP 7 类别分级) + 29 测试
- `src/vcp_category.rs` — VcpCategory enum (per VCP §2.1, 7 类) + 10 测试
- `src/chain.rs` — classify chain (多阶段: syntactic → semantic → risk) + 10 测试
- `src/handoff.rs` — 工具间 handoff (per agentManager.js chokidar 借鉴) + 11 测试
- `src/injection.rs` — runtime injection (per dev_inject_plugin 借鉴) + 9 测试
- `src/token_budget.rs` — 三层 token 预算 (per §6.2.2 #15) + 15 测试
- `src/async_task.rs` — 异步任务推送 (TaskRecord + TaskStatus enum + NotifyChannel) + 1 测试
- `src/organ_kani_proofs.rs` — tool-registry organ Kani proofs (R177, 10 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/registry.rs` (16) + `tests/classifier_integration.rs` (8)
- 例: `examples/registry_demo.rs`
