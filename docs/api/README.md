# Apeireth API Reference (v1.0.0)

> **性质**: Apeireth HTTP + WebSocket API v1 总览（1.0 release 配套文档）
> **依据**: `crates/apeireth-api/src/` 实际实现 + `docs/stage4/r20-stage-2-3-prep-2026-08-05.md` 蓝图
> **最后更新**: 2026-08-05
> **不假装**: 仅描述 **已 commit 代码**；TODO 标 R21 估补

---

## 1. API 形态

Apeireth 暴露三类端点：

| 形态 | 协议 | 用途 | 端口（默认） |
|---|---|---|---|
| **REST v1** | HTTP/1.1 + JSON | 6 工具调用 + 3 observability | 8080 |
| **REST v2** | HTTP/1.1 + JSON | 兼容旧版 organs 路由 | 8080（`/v2/...` 子路径） |
| **WebSocket** | RFC 6455 | 流式 LLM 响应 + 事件订阅 | 8080（`/v1/ws`） |

**协议版本**: `apeireth-api` v1.0.0（与 workspace.version 同步）
**鉴权**: 5 组件 token 体系（详见 [`auth.md`](auth.md)）
**限流**: token bucket（详见 [`rate-limit.md`](rate-limit.md)）
**错误**: 统一 error code（详见 [`error-codes.md`](error-codes.md)）

---

## 2. 端点总览

### 2.1 6 工具 endpoint（v1/tools/{name}/invoke）

| 工具 | 路径 | 方法 | 真实接 vs Stub | 文档 |
|---|---|---|---|---|
| **calendar** | `/v1/tools/calendar/invoke` | POST | ✅ 真实（per D-01 主人拍板） | [v1-tools-calendar.md](v1-tools-calendar.md) |
| **message** | `/v1/tools/message/invoke` | POST | ✅ 真实 | [v1-tools-message.md](v1-tools-message.md) |
| **contact** | `/v1/tools/contact/invoke` | POST | 🟡 部分（lookup 真实, write stub 501） | [v1-tools-contact.md](v1-tools-contact.md) |
| **task** | `/v1/tools/task/invoke` | POST | 🟡 部分 | [v1-tools-task.md](v1-tools-task.md) |
| **search** | `/v1/tools/search/invoke` | POST | ✅ 真实 | [v1-tools-search.md](v1-tools-search.md) |
| **drive** | `/v1/tools/drive/invoke` | POST | ✅ 真实（聚合 file_ops + storage） | [v1-tools-drive.md](v1-tools-drive.md) |

> 6 工具详细 schema + 示例 + 错误码见 [`v1-tools.md`](v1-tools.md)。

### 2.2 3 Observability 端点

| 端点 | 路径 | 方法 | 用途 | 文档 |
|---|---|---|---|---|
| **metrics** | `/metrics` | GET | Prometheus 8 指标暴露 | [v1-observability.md](v1-observability.md) |
| **health** | `/health` | GET | Liveness probe | [v1-observability.md](v1-observability.md) |
| **status** | `/v1/status` | GET | 详细运行时状态 | [v1-observability.md](v1-observability.md) |

### 2.3 WebSocket 8 帧

| 帧 | 方向 | 用途 | 文档 |
|---|---|---|---|
| `auth` | C → S | 链接 token 5min TTL（per D-03） | [v1-websocket.md](v1-websocket.md) |
| `chat` | C → S | 发送消息 | [v1-websocket.md](v1-websocket.md) |
| `delta` | S → C | 流式 token 增量 | [v1-websocket.md](v1-websocket.md) |
| `tool_call` | S → C | 工具调用请求 | [v1-websocket.md](v1-websocket.md) |
| `tool_result` | C → S | 工具调用结果 | [v1-websocket.md](v1-websocket.md) |
| `error` | S → C | 错误响应 | [v1-websocket.md](v1-websocket.md) |
| `ping`/`pong` | 双向 | 心跳保活 | [v1-websocket.md](v1-websocket.md) |
| `close` | 双向 | 优雅关闭 | [v1-websocket.md](v1-websocket.md) |

### 2.4 Provider 客户端（5 个）

| Provider | crate | 真接 vs Stub | 文档 |
|---|---|---|---|
| **claude-code** | `apeireth-provider-claude-code` | ✅ 真接 SDK | [provider-claude-code.md](provider-claude-code.md) |
| **gemini-cli** | `apeireth-provider-gemini-cli` | 🟡 stub（client 实现, SDK 留 R21） | 同上格式（暂未单独文件） |
| **codex** | `apeireth-provider-codex` | 🟡 stub | 同上 |
| **copilot** | `apeireth-provider-copilot` | 🟡 stub | 同上 |
| **opencode** | `apeireth-provider-opencode` | 🟡 stub | 同上 |

---

## 3. 不假装边界（per APEIRETH-CONVENTIONS.md §10）

| 端点 | 状态 | 备注 |
|---|---|---|
| `/v1/tools/calendar/invoke` (write) | 🟡 PARTIAL | 读真接, 写 501（CRUD 需 R21 商业化版） |
| `/v1/tools/message/invoke` (send) | ✅ 真接 | 直发 SMTP/Lark webhook |
| `/v1/tools/contact/invoke` (write) | 🟡 STUB | write 返 501（per D-05） |
| `/v1/tools/task/invoke` (CRUD) | 🟡 PARTIAL | list/get 真接, create/update/delete stub |
| `/v1/tools/search/invoke` | ✅ 真接 | tantivy 索引（per `apeireth-vector`） |
| `/v1/tools/drive/invoke` (upload) | ✅ 真接 | S3-compatible + 本地 storage 后端 |
| `/v1/quota/*` | ⚪ TODO R21 | quota 体系 R21 商业化才实装 |
| `/v1/admin/*` | ⚪ TODO R21 | admin 端点 R21 |

---

## 4. 不修改承诺

- ✅ 不假装已实现：本文档仅描述 **已 commit 代码**
- ✅ 编译期 hardcode：tool 白名单 + 协议 schema 编译期固定
- ✅ 不改 LOCKED：APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md 不动
- ✅ 不改 workspace version：1.0.0 严守

---

## 5. 相关文档

- **SDK 视角**: [`docs/sdk/README.md`](../sdk/README.md)
- **架构决策**: [`docs/adr/README.md`](../adr/README.md)
- **1.0 release 索引**: [`docs/1.0-release/README.md`](../1.0-release/README.md)
- **API 实施蓝图**: `docs/stage4/r20-stage-2-3-prep-2026-08-05.md` §1.1
