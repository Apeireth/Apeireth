# apeireth-pipeline

> **Apeireth R17 战役 1-3 主 chat 管线** — VCP 借鉴 §6.2.2 #15/#17/#19/#20 (token 预算三层 / placeholder 递归 / Force-Translate / 15s 抑制窗口).
> **当前状态**: 12 src files / ~184KB, 真接 minimax via `apeireth-api`.

---

## 公共 API

- `Pipeline` — 5 步管线主类型 (R17 战役 1-3)
- `provider_registry::ProviderRegistry` — provider 路由表
- `model_router::ModelRouter` — 模型路由 (借鉴 VCP semanticModelRouter.js)
- `placeholder::Placeholder` — 递归 placeholder 处理
- `force_translate::ForceTranslate` — VCP §6.2.2 #17 借鉴
- `token_budget::TokenBudget` — 3 层 token 预算 (VCP §6.2.2 #15)
- `retry_suppression::RetrySuppression` — 15s 抑制窗口 (VCP §6.2.2 #19)
- `tiktoken_counter::TiktokenCounter` — 精确 token 计数 (借鉴 VCP finalContextStore.js)
- `role_divider::RoleDivider` — 多角色消息划分
- `tool_loop::ToolLoop` — 工具调用循环
- `streaming::Streaming` — 流式响应

## 跟 `apeireth-pipeline-g5` 关系

- `apeireth-pipeline` — chat 专用管线, VCP §6.2.2 借鉴
- `apeireth-pipeline-g5` — 通用 5 阶段框架 (Dispatch → Normalize → Policy → Reliability → Throttle), 借鉴 Golutra v0.1.0
- 两个不重复, 互补: pipeline 走 chat, pipeline-g5 走通用

## 依赖

- `apeireth-protocol` (4 协议归一化)
- `apeireth-http-client` (Keep-Alive LIFO)
- `tokio` + `futures` + `serde` + `serde_json` + `serde_yaml` + `tiktoken-rs` + `thiserror`

## 验证

```bash
cargo check -p apeireth-pipeline    # 0 errors
cargo test -p apeireth-pipeline     # 5 步管线 + 路由 + 抑制窗口测试
cargo run -p apeireth-pipeline --example pipeline_demo    # chat pipeline demo
cargo run -p apeireth-pipeline --example provider_registry_demo  # 多 provider 路由 demo
```

## See also

- [VCP 借鉴映射 (decision-130 §3.5)](../../reports/decision-130-12-15-tick-owner-3-q-a-6-b-phl-07-b-integrate-5-1-commit-execute-2026-08-11.md)
- [通用 5 阶段 framework](../apeireth-pipeline-g5/)