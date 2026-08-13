# R161 - apeireth-memory x apeireth-pipeline-g5 一体化集成 (memory bridge)

## Context

apeireth-memory (R14 Phase 1, 14 src files: append_only/continuity_link/episode/history_streams/identity/lib/llm_analysis/migrations/semantic_persist/semantic/session_note/streams/three_layer/user_profile + dailynote/lightmemo subdirs) 接 apeireth-pipeline-g5 作为第 5 个生产调用方.

历史:
- R132.4: tool-runtime 第 1 个生产调用方
- R157: chat pipeline 第 2 个生产调用方
- R159: council 第 3 个生产调用方
- R160: runtime 第 4 个生产调用方
- R161 (本 cycle): memory 第 5 个生产调用方

## Strategy: ADDITIVE 集成 (非 refactor)

不重构现有 memory 14 src files (Episode/Note/Session SQLite 存储 + BM25 检索), 而是在 memory crate 内加新模块 g5_memory_bridge.rs:

- 5 个独立 Stage<I, O> impl, 各包一个 memory insert/retrieve concern
- MemoryPipelineBuilder::new().build() 拿 Pipeline<MemoryPipeline, PipelineMessage, PipelineMessage>
- 现有 197 lib tests 0 改动, 0 breaking change

## 5 步 -> 5 阶段映射

| g5 阶段 | memory 概念 | bridge struct |
|---------|---------|-------------|
| Dispatch | memory kind 默认 episode-insert | MemoryDispatchStage |
| Normalize | key 256 char + payload 1MB cap | MemoryNormalizeStage |
| Policy | TTL 30 天默认 (写入 trace_id) | MemoryPolicyStage |
| Reliability | 60s fingerprint dedup | MemoryReliabilityStage |
| Throttle | per-key rate limit (100/key 默认) | MemoryThrottleStage |

## Why ADDITIVE 而不是 refactor

现有 memory 14 src files 是 197 lib tests 实战打磨过的代码, 不动.

R161 给想要 g5 substrate 严格 5 阶段语义的 memory 消费者一个 opt-in 入口.

## Implementation

- crates/apeireth-memory/Cargo.toml: 加 apeireth-pipeline-g5 = { path = ../apeireth-pipeline-g5 }
- crates/apeireth-memory/src/lib.rs: pub mod g5_memory_bridge;
- crates/apeireth-memory/src/g5_memory_bridge.rs: 167 行 (新文件, 包含 5 stage impl + builder + 10 tests)

## Tests

- cargo check -p apeireth-memory: 0 errors
- cargo test -p apeireth-memory --lib g5_memory_bridge: 10 passed

## Borrowed upstream reference (per O-5)

- apeireth-pipeline-g5 (generic 5-stage substrate) - 借鉴 Golutra v0.1.0
- R132.4/R157/R159/R160: 4 prior g5 集成参考模板
- R14 Phase 1 memory subsystem (V1130 wallclock 2.5s baseline)

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 touches 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache)
- 0 changes to existing memory 14 src files
- 0 changes to apeireth-pipeline-g5 existing Stage trait / Pipeline struct
- 1 ADDITIVE dep: apeireth-pipeline-g5 = path
- 1 ADDITIVE pub mod: g5_memory_bridge
- cargo check --workspace: 0 errors

## Next: R162+

- R162: 继续 GitHub 调研 + per-module improvements (memory/onion/formal 补强)
- g5 substrate 现在 5 个生产调用方完成: tool-runtime / chat / council / runtime / memory
