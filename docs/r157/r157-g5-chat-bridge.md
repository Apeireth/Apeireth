# R157 - apeireth-pipeline x apeireth-pipeline-g5 一体化集成

## Context

apeireth-pipeline (chat-specific, 184KB, 12 files, R17 battles 1-3) 和 apeireth-pipeline-g5 (generic 5-stage substrate, 76KB, 10 files) 是两个独立 crate, 概念互补但不共享代码:

- apeireth-pipeline: 借鉴 VCP §6.2.2 #15/#17/#19/#20, chat 专用 5 步管线
- apeireth-pipeline-g5: 借鉴 Golutra v0.1.0 chat_db/pipeline, 通用 Dispatch/Normalize/Policy/Reliability/Throttle 5 阶段框架

R131.7 audit 选 A 案 (不合并 + pipeline-g5 0 调用方), R132.4 主人拍 B 案 (接入生产调用方), tool-runtime 作为第 1 个生产调用方 (R132.4 已落).

R157 接入第 2 个生产调用方: chat pipeline 5 步映射到 g5 5 阶段.

## Strategy: ADDITIVE 集成 (非 refactor)

不重构现有 chat 5 步编排 (lib.rs Pipeline::with_config), 而是在 chat pipeline crate 内加新模块 g5_chat_bridge.rs:

- 5 个独立 Stage<I, O> impl, 各包一个 chat concern
- ChatPipelineBuilder::new().with_placeholder(ctx).build() 拿 Pipeline<ChatPipeline, PipelineMessage, PipelineMessage>
- 现有 R17 chat pipeline 0 改动, 0 breaking change

## 5 步 -> 5 阶段映射

| g5 阶段 | chat 概念 | bridge struct |
|---------|---------|-------------|
| Dispatch | 写 kind (默认 chat) | ChatDispatchStage |
| Normalize | placeholder 递归解析 (R17 #17) | ChatNormalizeStage |
| Policy | token 预算截断 (R17 #15) | ChatPolicyStage |
| Reliability | 15s 抑制窗口 (R17 #19) | ChatReliabilityStage |
| Throttle | 当前 0 限流 (留 R133+ 接) | ChatThrottleStage |

I/O 共享: apeireth_pipeline_g5::PipelineMessage, 各阶段 mutate 自己的字段.

## Why ADDITIVE 而不是 refactor

现有 chat 5 步 (lib.rs Pipeline::with_config) 是 VCP 真借鉴 (§6.2.2 #15/#17/#19/#20), R17 战役 1-3 实战打磨过的代码, 不动.

R157 给想要 g5 substrate 严格 5 阶段语义的消费者一个 opt-in 入口 (例如: apeireth-council 决策后投递, apeireth-runtime 编排调度, 等).

## Implementation

- crates/apeireth-pipeline/Cargo.toml: 加 apeireth-pipeline-g5 = { path = "../apeireth-pipeline-g5" }
- crates/apeireth-pipeline/src/lib.rs: pub mod g5_chat_bridge;
- crates/apeireth-pipeline/src/g5_chat_bridge.rs: 199 行 (新文件, 包含 5 stage impl + builder + 13 tests)
- crates/apeireth-pipeline/README.md: 加 R157 section 说明 bridge 用法

## Tests

- cargo check -p apeireth-pipeline: 0 warnings, 0 errors
- cargo test -p apeireth-pipeline --lib: 145 passed (132 pre-existing + 13 new bridge tests)
  - dispatch_defaults_to_chat, dispatch_preserves_kind
  - normalize_resolves_vars, normalize_keeps_unknown, normalize_prevents_loops
  - policy_truncates_long, policy_passes_short
  - reliability_first_run, reliability_second_suppressed
  - throttle_passes
  - full_pipeline_runs, pipeline_suppresses_repeat, stage_order_is_dispatch_normalize_policy_reliability_throttle

## Borrowed upstream reference (per O-5)

- apeireth-pipeline-g5 (generic 5-stage substrate) - 借鉴 Golutra v0.1.0 chat_db/pipeline
- apeireth-tool-runtime::tool_pipeline (R132.4) - 第 1 个 g5 集成参考模板
- VCP chatCompletionHandler.js:1-220 (R17 #17/#19) - placeholder 递归 + 15s 抑制窗口
- apeireth-pipeline 现有 R17 lib.rs Pipeline - 实战 132 tests 打磨过的 5 步编排

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 touches 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache)
- 0 touches 8 不修改承诺
- 0 changes to existing chat pipeline 5 步编排 (R17 lib.rs Pipeline::with_config 0 改动)
- 0 changes to apeireth-pipeline-g5 现有 Stage trait / Pipeline struct
- 1 ADDITIVE dep: apeireth-pipeline-g5 = path (原本 cargo dep 没声明, 现在加)
- 1 ADDITIVE pub mod: g5_chat_bridge (原本 lib.rs 没声明, 现在加)
- cargo check --workspace: 0 errors

## Next: R158+

- apeireth-council 可选接 g5 (7 advisor 决策后投递)
- apeireth-runtime 可选接 g5 (编排调度 5 阶段)
- 继续 GitHub 调研 (swarms-rs / chidori / tirea-ai 等) 对比 g5 substrate 看是否需要进一步升级
