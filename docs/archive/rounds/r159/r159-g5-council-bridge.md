# R159 - apeireth-council x apeireth-pipeline-g5 一体化集成 (council bridge)

## Context

apeireth-council (286 tests, 24 src files, 7 mandatory advisors: Safety/Performance/Philosophy/History/Strategy/Ethics/Legal) 接 apeireth-pipeline-g5 (generic 5-stage substrate) 作为第 3 个生产调用方.

历史:
- R131.7 audit: g5 0 调用方 (Option A 失败)
- R132.4: tool-runtime 作为第 1 个生产调用方
- R157: chat pipeline 作为第 2 个生产调用方
- R159 (本 cycle): council 作为第 3 个生产调用方

## Strategy: ADDITIVE 集成 (非 refactor)

不重构现有 council deliberation (deliberation.rs Council::deliberate), 而是在 council crate 内加新模块 g5_council_bridge.rs:

- 5 个独立 Stage<I, O> impl, 各包一个 council concern
- CouncilPipelineBuilder::new().build() 拿 Pipeline<CouncilPipeline, PipelineMessage, PipelineMessage>
- 现有 7 advisor + deliberation + synthesis + hold 流程 0 改动, 0 breaking change

## 5 步 -> 5 阶段映射

| g5 阶段 | council 概念 | bridge struct |
|---------|---------|-------------|
| Dispatch | 按 area 路由 advisor subset | CouncilDispatchStage |
| Normalize | clamp description 长度 + dedup refs | CouncilNormalizeStage |
| Policy | L0 HA + nuclear risk 标记 | CouncilPolicyStage |
| Reliability | 60s 抑制窗口 (synthesis 幂等) | CouncilReliabilityStage |
| Throttle | 每分钟 max deliberation rate | CouncilThrottleStage |

I/O 共享: apeireth_pipeline_g5::PipelineMessage (kind 跟踪 council-pending -> dispatched -> normalized -> policy-checked -> reliability -> throttled).

## Why ADDITIVE 而不是 refactor

现有 council deliberation 流程 (deliberation.rs) 是 286 tests 实战打磨过的代码, 不动.

R159 给想要 g5 substrate 严格 5 阶段语义的 council 消费者 (apeireth-runtime 编排, apeireth-api /v1/guard, 等) 一个 opt-in 入口.

## Implementation

- crates/apeireth-council/Cargo.toml: 加 apeireth-pipeline-g5 = { path = ../apeireth-pipeline-g5 }
- crates/apeireth-council/src/lib.rs: pub mod g5_council_bridge;
- crates/apeireth-council/src/g5_council_bridge.rs: 248 行 (新文件, 包含 5 stage impl + builder + 13 tests)

## Tests

- cargo check -p apeireth-council: 0 errors
- cargo test -p apeireth-council --lib g5_council_bridge: 13 passed
- cargo test -p apeireth-council --lib advisors: 35 passed (7 advisor 模块)
- cargo test -p apeireth-council --lib session_capture: 17 passed
- (full suite 286 tests 慢, 跳过 - 已知 timeout 问题)

## Borrowed upstream reference (per O-5)

- apeireth-pipeline-g5 (generic 5-stage substrate) - 借鉴 Golutra v0.1.0 chat_db/pipeline
- apeireth-tool-runtime::tool_pipeline (R132.4) - 第 1 个 g5 集成参考模板
- apeireth-pipeline::g5_chat_bridge (R157) - 第 2 个 g5 集成参考模板
- AutoGen GroupChat + VCP vcpLoop (R33-4-1) - council deliberation 真实设计模式

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 touches 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache)
- 0 touches 8 不修改承诺
- 0 changes to existing 7 advisor + deliberation.rs Council::deliberate 流程 (additive integration only)
- 0 changes to apeireth-pipeline-g5 现有 Stage trait / Pipeline struct
- 1 ADDITIVE dep: apeireth-pipeline-g5 = path
- 1 ADDITIVE pub mod: g5_council_bridge
- cargo check --workspace: 0 errors

## Next: R160+

- apeireth-runtime 接 g5 (编排调度 5 阶段, 第 4 个生产调用方)
- apeireth-memory 接 g5 (lightmemo + semantic_persist 5 阶段, 第 5 个生产调用方)
- 至此 g5 将是 5 个核心子系统的统一 substrate, 一体化优美完全实现
