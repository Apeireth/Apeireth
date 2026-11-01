# apeireth-memory

> Apeireth 记忆子系统 (Episode/Note/Session SQLite 存储 + BM25 检索) — R14 Phase 1 主目标 (V1130 wallclock 2.5s)

Apeireth v2 工作区 crate。**src 模块 21 个** (实际 ls crates/apeireth-memory/src/: append_only / continuity_link / dedup / episode / g5_memory_bridge / gen_cache / hallways / history_streams / identity / llm_analysis / migrations / onnx / provenance / semantic / semantic_persist / session_note / streams / three_layer / user_profile + lib + tests 等; README 旧版只列 8 个, 缩水)。测试数字单一真源见 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md)（本文不抄写）。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
