# 缓存友好提示组装 (Cache-Friendly Prompt Assembly)

```text
[Document-Meta]
Document: docs/02-guides/cache-friendly-prompt-assembly.md
Version: S3 V1 pairing
Last-Modified: 2026-10-29
Status: 活跃；组装行为有离线 Runtime 回归，实际服务商缓存命中率未实测
Author: Codex
```

## 1. 请求顺序与信任边界

静态 Host system / 身份 / 安全约定保留在会话前缀。工具声明使用既有目录确定的顺序。
行为模块的动态文本经 RegisteredBehavior、固定 Hook、Dispatcher 和 ToolEventV1，
以配对的 assistant call + tool result 追加；调用批未闭合时先等待，不插入开放批次。
`compose_provider_messages` 只组合已提交视图与最后的 `retry_scaffolding`。

```text
[ 静态 Host system | 已提交历史（含合法模块工具对）| 重试反馈（若有）]
```

模块按注册顺序执行。同一 Hook 的结果只提交一次；空观察仅审计，不制造空工具对。
下一轮保留已提交事实，不重新复制同一结果。业务引擎使用剔除已认证模块工具对的历史，
因此模块内容不会变成用户输入或被再次学习。静态 Host policy 与动态数据角色分开。

## 2. Overlay 兼容表示与预算

S3 之前，未注册的 `with_module` 可以直接执行回调，把暂态 Overlay 当 system 消息尾插。
该执行与注入分支已移除。`with_module` / `with_behavior` 与 `with_registered_behavior`
使用同一注册工具路径，Host 仍须显式授权。

`PromptOverlay` / `ModuleOutcome.prompt_overlays` 保留为业务产生者与兼容 reader 的数据表示。
`system` 构造器的名称不赋予动态内容 system 权限；适配器丢弃消息角色，将文本与来源标签放入
`untrusted_data` 工具载荷。V1 的字段、版本与配对规则不变。

`behavior_projection::budget_module_context` 只处理与持久 ToolEventV1 精确匹配的工具对，
不会根据用户文本里的 JSON 或工具名推断身份。它对派生 Provider 视图统一预算，不改写审计原文、
call id、来源或 Session 消息。静态 system 核心块计入预算且不截断。

跨源内容继续经过 `UntrustedEnvelope`：固定告警、来源、边界与防伪转义保留在工具数据中。
超预算沿用最长非核心块优先裁剪与 spill；文件含完整原文，载荷含预览和真实取回指引。
无 spill sink 或写入失败时保留完整内容，不制造不可取回的截断。

## 3. 前缀稳定性与明确断点

在相同预算和视图下，追加历史可复用之前的消息前缀。不能据此承诺真实服务商的缓存命中率；
协议序列化、模型缓存策略、历史压缩及预算改变均可能影响缓存。

| 断点 | 触发与行为 |
| --- | --- |
| 压缩检查点 | 派生表层以摘要替换可压缩历史，原文与事件仍保留 |
| 重试反馈 | `Retry { feedback }` 的暂态反馈置于末尾，不写成用户历史 |
| 溢出预算收缩 | 上下文超限后，从原始认证事实重新预算工具载荷；只有预算变小且组装变化才重试，保留原重试上限 |
| 跨源披露或总预算变化 | 封套与预览可能改变所在位置之后的派生前缀；不重新运行回调或重复执行工具 |
| 新会话 | 静态 system 与首条消息建立新前缀 |

## 4. 缓存诊断

`APEIRETH_CACHE_TRACE=1` 打开既有脱敏 stderr 简报，含稳定前缀字节数与变化范围，
不打印消息文本、工具名或模型输出。未设置时关闭。
源码为 `crates/engine/runtime/src/canonical/cache_trace.rs`。

## 5. 验证与表示迁移

`cache_prefix`、`context_budget`、`context_overflow` 和 execute 单元测试保留前缀、顺序、
跨源封套、spill 可取回性、核心块、预算收缩与重试断点断言。
旧“动态 system / 暂态不持久”断言在 S3 明确改为 V1 配对、数据角色及持久一次，
同时保留用户历史不污染与业务历史过滤。实际命令和结果见
[S3 交付账](../04-internal/kernel-convergence-s3-2026-10-29.md)。
