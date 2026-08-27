# Apeireth API Reference

> 对齐 canonical gateway 实际路由（2026-08-27，v2.0.0-alpha.1）。
> Base URL: `http://127.0.0.1:8080`（`apeireth gateway serve --port <port>` 可改）。

> **历史（v1 时代）**：旧 companion_serve（:8090）的 OpenAI 兼容端点、`/v1/apeireth/*` 审批队列、
> `/panel` 面板与 `<<<[TOOL_REQUEST]>>>` marker 协议均为 86-crate 时代产物，代码现位于
> `legacy/donor/apeireth-companion/examples/companion_serve.rs` 及相关 crate；其能力将按根 `ROADMAP.md` §4 恢复。

## Gateway 端点

### `GET /health`

健康检查。

### `POST /v1/chat`

伙伴主链路对话端点。认证：生产前为本地场景，见"认证与安全"。

```json
{
  "session": "optional-session-id",
  "input": "你好",
  "model": "MiniMax-M3"
}
```

### `POST /v1/chat/completions`

OpenAI Chat Completions 兼容格式。

```json
{
  "model": "MiniMax-M3",
  "messages": [{"role": "user", "content": "你好"}]
}
```

两个端点都委托 `apeireth-runtime::canonical::Runtime::execute`，不自行编排。
单轮行为：governance（completion）→ provider 路由/调用 → 工具调用则走
capability 查找 + 插件分发 → 工具结果回灌 transcript 继续，直到最终回复；
approval 以 outcome 形式返回（pending approval 可恢复），trace 随响应返回且不含原始 CoT。

## CLI

```text
apeireth session
apeireth chat "<prompt>" [--model <model>]
apeireth gateway serve --port 8080
```

## Provider 配置

| Provider | Key 环境变量 | 可选变量 |
|---|---|---|
| MiniMax | `APEIRETH_API_KEY` | `APEIRETH_API_URL` / `APEIRETH_API_MODELS` |
| Anthropic | `APEIRETH_ANTHROPIC_KEY` | `APEIRETH_ANTHROPIC_URL` / `APEIRETH_ANTHROPIC_MODELS` |
| OpenAI-compatible | `OPENAI_API_KEY` | `APEIRETH_OPENAI_URL` / `APEIRETH_OPENAI_MODELS` |

凭据每轮经 `CredentialResolver` 解析，runtime/provider 不长期持有 key。

## 工具（默认状态）

| 工具 | 默认 | 说明 |
|---|---|---|
| `tool.filesystem` / `tool.search` / `tool.repo` | ✅ 启用 | 只读 |
| `tool.shell` | ❌ 关闭 | 显式配置才启用（opt-in） |
| `tool.fetch` | ❌ 关闭 | GET-only + DNS 钉扎 + 逐跳重校验，显式配置才启用 |

## 认证与安全

- Gateway 传输层认证：本地伙伴场景；生产化令牌方案待排期（见 `SECURITY.md`）。
- 工具执行走唯一 `ProcessExecutor` 边界（Windows Job Object 等，见 `docs/01-architecture/architecture.md`）。
- 出站 HTTP 走 `crates/capabilities/tools/src/egress.rs` 策略（M2D）；fetch 走 M3A 受控实现。
- 生产 governance pipeline（PII/注入/权限策略）接线状态见根 `ROADMAP.md` §4 P0。
