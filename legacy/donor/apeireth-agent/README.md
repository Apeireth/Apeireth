# apeireth-agent

> Apeireth R17 战役 2-4: Agent 管理系统 (alias 解析 + LRU cache + notify 热加载, 字段级复刻 agentManager.js 339 行)

apeireth-agent 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (5 文件 / 64 测试 + 2 Kani proof)

- `src/lib.rs` — 入口 re-export + 7 测试
- `src/agent.rs` — Agent 数据结构 + 生命周期 + 10 测试
- `src/manager.rs` — AgentManager (LRU cache + alias 解析 + notify 热加载) + 35 测试
- `src/subagent.rs` — SubAgent 派生关系 + 7 测试
- `src/organ_kani_proofs.rs` — R177 organ Kani proofs (5 测试 + 2 `#[kani::proof]`)
