# R160 - apeireth-runtime x apeireth-pipeline-g5 一体化集成 (runtime bridge)

## Context

apeireth-runtime (R147 end-to-end orchestration, 24KB lib.rs, 7 modules wired: HeartbeatScheduler + AsyncTaskStore + ChanneledBus + ArbitrationLog + SearchEngine + GroupChat + EmotionEngine, 23 tests) 接 apeireth-pipeline-g5 作为第 4 个生产调用方.

历史:
- R132.4: tool-runtime 第 1 个生产调用方
- R157: chat pipeline 第 2 个生产调用方
- R159: council 第 3 个生产调用方
- R160 (本 cycle): runtime 第 4 个生产调用方

## Strategy: ADDITIVE 集成 (非 refactor)

不重构现有 runtime 7 模块编织 (lib.rs HeartbeatScheduler + AsyncTaskStore + ChanneledBus + ArbitrationLog + SearchEngine + GroupChat + EmotionEngine), 而是在 runtime crate 内加新模块 g5_runtime_bridge.rs:

- 5 个独立 Stage<I, O> impl, 各包一个 runtime task lifecycle concern
- RuntimePipelineBuilder::new().with_policy().build() 拿 Pipeline<RuntimePipeline, PipelineMessage, PipelineMessage>
- 现有 23 lib tests 0 改动, 0 breaking change

## 5 步 -> 5 阶段映射

| g5 阶段 | runtime 概念 | bridge struct |
|---------|---------|-------------|
| Dispatch | task register (Pending) | RuntimeDispatchStage |
| Normalize | payload size cap (16KB) | RuntimeNormalizeStage |
| Policy | concurrency cap (16 默认) | RuntimePolicyStage |
| Reliability | 30s retry 抑制窗口 | RuntimeReliabilityStage |
| Throttle | per-tick rate limit (100/tick 默认) | RuntimeThrottleStage |

## Why ADDITIVE 而不是 refactor

现有 runtime 7 模块编织是 R147 端到端 24KB lib.rs + 23 tests 实战打磨过的代码, 不动.

R160 给想要 g5 substrate 严格 5 阶段语义的 runtime task 消费者一个 opt-in 入口 (例如: 第三方 scheduler / external driver).

## Implementation

- crates/apeireth-runtime/Cargo.toml: 加 apeireth-pipeline-g5 = { path = ../apeireth-pipeline-g5 }
- crates/apeireth-runtime/src/lib.rs: pub mod g5_runtime_bridge;
- crates/apeireth-runtime/src/g5_runtime_bridge.rs: 122 行 (新文件, 包含 5 stage impl + builder + 9 tests)

## Tests

- cargo check -p apeireth-runtime: 0 errors
- cargo test -p apeireth-runtime --lib g5_runtime_bridge: 9 passed
- cargo test -p apeireth-runtime --lib: 23 passed (14 pre-existing + 9 new bridge)

## Borrowed upstream reference (per O-5)

- apeireth-pipeline-g5 (generic 5-stage substrate) - 借鉴 Golutra v0.1.0
- apeireth-tool-runtime::tool_pipeline (R132.4) - 第 1 个 g5 集成
- apeireth-pipeline::g5_chat_bridge (R157) - 第 2 个 g5 集成
- apeireth-council::g5_council_bridge (R159) - 第 3 个 g5 集成
- self-driven living day concept (R147 runtime origin) - borrowed, lifted to Rust compile-time unified runtime

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 touches 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache)
- 0 changes to existing 7 模块编织 (HeartbeatScheduler + AsyncTaskStore + ChanneledBus + ArbitrationLog + SearchEngine + GroupChat + EmotionEngine)
- 0 changes to apeireth-pipeline-g5 existing Stage trait / Pipeline struct
- 1 ADDITIVE dep: apeireth-pipeline-g5 = path
- 1 ADDITIVE pub mod: g5_runtime_bridge
- cargo check --workspace: 0 errors

## Next: R161+

- R161: apeireth-memory 接 g5 (第 5 个生产调用方)
- R162+: GitHub research + per-module improvements (memory/onion/formal 补强)
