# apeireth-team-lead

> Apeireth R20 阶段 1: Team Lead (1:1 翻译 v0.9.21 商业版 out/main/agent/AgentMCPServer.js Orchestrator 缺 P0, A 改 13:34 的版本同步)

apeireth-team-lead 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (4 src 文件 / 16 测试 + 2 Kani proof + 3 集成)

- `src/lib.rs` — TeamLead orchestrator 入口 + Handoff 委托协议 (parking_lot::RwLock) + 1 测试
- `src/approval_bridge.rs` — 工具审批门桥接 (tool-runtime + tool-registry → 7-器官高危拦截) + 10 测试
- `src/md/supervisor_prompt.md` — supervisor prompt 模板 (含 R20 13:34 同步版本)
- `src/organ_kani_proofs.rs` — team-lead organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/test_mcp_in_process.rs` (3)
- 例: `examples/team_lead_demo.rs`, bench: `benches/bench.rs`
