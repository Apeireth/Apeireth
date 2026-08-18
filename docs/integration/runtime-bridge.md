# Phase 4 — Agent Runtime Bridge（2026-08-19）

> 目标（§15）：建立 Apeireth 自己的 runtime contract，UI 不直接暴露 OpenAI SDK / 裸 fetch。

## Contract 形态

```text
Companion UI
     ↓  (仅面对 AgentRuntime 接口)
Agent Runtime Contract      ← frontend/companion-desktop/src/lib/runtime.ts
     ↓  (HTTP/SSE adapter)
Apeireth 端点 (companion_serve :8090 / apeireth-api :8080)
     ↓
Runtime / Provider / Tools
```

## 类型清单（runtime.ts）

| 类型 | 用途 |
|---|---|
| `ModelReference` | 模型引用 (id + provider + label)，不暴露 SDK provider 细节 |
| `AgentMessage` | 标准化消息 (user/assistant/system) |
| `AgentRunRequest` | 运行请求 (messages + model + sessionId + context) |
| `RuntimeEvent` | 可辨识联合事件流 |
| `RuntimeError` | 标准化错误 (http/network/auth/timeout/aborted/unknown) |
| `AgentRuntime` | UI 唯一入口接口 (run/abort/running/health) |

## RuntimeEvent（§15 要求的 9 事件全预留）

```text
run-start · message-start · text-delta · reasoning-delta
tool-call · tool-result · message-end · run-error · run-end
```

当前实现：`run-start` / `message-start` / `text-delta` / `message-end` / `run-error` / `run-end` 已发射；
`reasoning-delta` / `tool-call` / `tool-result` 类型已预留（后端 future 暴露时接入）。

## §16 Commander / Worker 预留

- `AgentRunRequest.sessionId` — 会话上下文 ID，未来多节点/多设备共享会话的锚点
- `AgentRunRequest.context.persona/user` — persona / long-term memory / user context 注入点
- run() 事件流形态 — 天然可经 socket/bus 透传，不绑定单机 HTTP

## UI 集成（App.svelte）

- `createAgentRuntime(config)` 工厂，UI 通过 `agentRuntime.run(request, onEvent)` 对话
- 事件回调按 `text-delta` 增量更新消息；错误经 `RuntimeError` 标准化显示
- 设置保存时重建 runtime 绑定新配置

## 未做（诚实标注）

- `reasoning-delta` / `tool-call` / `tool-result` 的 UI 渲染：后端当前无 reasoning/tool 流事件，
  等 companion_serve 暴露后接入（类型已就绪）
- 完整 RAG / vector DB：§17 明确不做，context 注入点已留位
