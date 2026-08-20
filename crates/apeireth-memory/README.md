# apeireth-memory

> Apeireth 记忆子系统 (Episode/Note/Session SQLite 存储 + BM25 检索) — R14 Phase 1 主目标 (V1130 wallclock 2.5s)

Apeireth 1.0 工作区 crate。**src 模块 21 个** (实际 ls crates/apeireth-memory/src/: append_only / continuity_link / dedup / episode / g5_memory_bridge / gen_cache / hallways / history_streams / identity / llm_analysis / migrations / onnx / provenance / semantic / semantic_persist / session_note / streams / three_layer / user_profile + lib + tests 等; README 旧版只列 8 个, 缩水)。**测试数 (tests/ 集成测试): 24 个 #[test]** (integration_six_streams.rs 9 + semantic_pipeline_e2e.rs 2 + sqlite.rs 6 + vector_persistence.rs 7, README 旧版 "317" 严重膨胀)。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../docs/03-reference/crates.md)
