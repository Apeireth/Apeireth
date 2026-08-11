# API Reference — 1.0.0

> **Base URL**: `http://localhost:8080` (默认, `APEIRETH_API_PORT` 环境变量可改)
> **Version**: 1.0.0 · **整合 #4 commit**: `abf12243`

Apeireth 1.0 API = 13 键 verdict cache + 30 维 V0.5 + 6 重守门 v7 + 24 LOCKED crate 入口签名.

---

## 1. 13 键 verdict cache (A3, per 决策 #33 §2.3 + 整合 #4 commit)

12 键 + PHL-07 = **13 键** verdict cache (per 决策 #33 + 整合 #4 commit done):

| # | 键 | 类型 | 维度来源 | 用途 |
|---:|-----|------|---------|------|
| 1 | `COR-01` | verdict | core | 核心正确性 |
| 2 | `COR-02` | verdict | core | 边界正确性 |
| 3 | `SEC-01` | verdict | security | 守门 1 |
| 4 | `SEC-02` | verdict | security | 守门 2 |
| 5 | `SEC-03` | verdict | security | 守门 3 |
| 6 | `SEC-04` | verdict | security | 守门 4 |
| 7 | `SEC-05` | verdict | security | 守门 5 |
| 8 | `SEC-06` | verdict | security | 守门 6 |
| 9 | `PHL-01` | verdict | philosophy | 哲学锚 1 (S-1 复杂可推导) |
| 10 | `PHL-02` | verdict | philosophy | 哲学锚 2 (S-2 实现可靠) |
| 11 | `PHL-03` | verdict | philosophy | 哲学锚 3 (S-3 流程自化) |
| 12 | `PHL-04` | verdict | philosophy | 哲学锚 4 (O-1 安全优先) |
| 13 | `PHL-07` | verdict | philosophy | 哲学锚 7 (O-4 任何人都能接手) |

**整合 #4 commit done**, 13 键 hardcode 编译期内保证 (决策 #33 §2.3 A3 严守).

## 2. 30 维 V0.5 (B3, per P1-4 R126 25→30 维 verify retry done)

**V0.5 30 维** (per 决策 #33 §2.3 B3 + P1-4 R126 verify retry done):

| 组 | 维度数 | 来源 | 备注 |
|----|------:|------|------|
| Core | 8 | 核心正确性 (R125-13) | COR-01~08 |
| Security | 6 | 6 重守门 v7 | SEC-01~06 |
| Robustness | ³ | R11 baseline 3 值 (0.8682/0.8532/0.9063, A1 严守) | RB-01~03 |
| Philosophy | 8 | 8 哲学锚 (B5) | PHL-01~08 |
| Extended | 5 | 5 扩展 (V0.5 25→30 维新增) | EXT-01~05 |
| **总** | **30** | **V0.5 完整维度** | **sum=1.0 严守** |

**P1-4 R126 25→30 维 verify retry done** (60 tests 30 维 sum=1.0 严守).

## 3. 6 重守门 v7 (B4, per P1-3 R126 6 重守门 v7 retry done)

**6 重守门 v7** = 5 嵌套 + Colang DSL (per 决策 #33 §2.3 B4 + P1-3 R126 retry done):

| # | 守门 | 类型 | v7 升级 |
|---:|------|------|---------|
| 1 | 守门 1 (基础类型 / 范围) | nested | v6 → v7 + 边界场景 |
| 2 | 守门 2 (关联一致性) | nested | v6 → v7 + 跨字段 |
| 3 | 守门 3 (业务规则) | nested | v6 → v7 + DSL 表达式 |
| 4 | 守门 4 (权限 / RBAC) | nested | v6 → v7 + 角色继承 |
| 5 | 守门 5 (审计 / 可追溯) | nested | v6 → v7 + 决策链 |
| 6 | 守门 6 (Colang DSL) | dsl | v7 新增 (Colang 模板) |

**v6 → v7 升级**: 5 重嵌套 + Colang DSL 第 6 重, 升级由 P1-3 R126 retry done (per 决策 #55 §3 升 v7).

## 4. 24 LOCKED crate 入口签名 (B1, per P2-3 + P4-1 + P14-1 retry 三方 verify)

**24 LOCKED crate** (B1, 12 已知 + 12 Mavis 自主, 整合 #4 commit 严守):

```
apeireth-agent        apeireth-central        apeireth-cli
apeireth-evolution    apeireth-formal         apeireth-graph
apeireth-http-client  apeireth-mcp            apeireth-naming-v05
apeireth-pipeline     apeireth-pybridge       apeireth-skills
apeireth-sovereignty  apeireth-tool-runtime
+ 12 Mavis 自主 LOCKED (整合 #4 commit 严守)
```

**入口签名 0 改** (per P2-3 + P4-1 + P14-1 retry 三方 verify done, 24/24 PASS).
**内部 fn 实施可改** (per 决策 #33 §2.3 B1 + 主人 0:03 授权 "技术性 locked 全部解锁").

## 5. v1 API Endpoints (per 决策 #11 阶段 4 frontend-proposal)

### 5.1 Observability (per `docs/api/v1-observability.md`)

```http
GET /v1/observability/health
GET /v1/observability/metrics
GET /v1/observability/traces
```

### 5.2 Tools (per `docs/api/v1-tools*.md`)

| Endpoint | Method | 用途 |
|----------|--------|------|
| `/v1/tools/calendar` | GET/POST | 日历工具 |
| `/v1/tools/contact` | GET/POST | 联系人工具 |
| `/v1/tools/drive` | GET/POST | 云盘工具 |
| `/v1/tools/message` | GET/POST | 消息工具 |
| `/v1/tools/search` | GET/POST | 搜索工具 |
| `/v1/tools/task` | GET/POST | 任务工具 |
| `/v1/tools` | GET | 工具列表 |

### 5.3 WebSocket (per `docs/api/v1-websocket.md`)

```http
WS /v1/ws
```

**WS 8 帧** (per R20 阶段 6): auth, ping, pong, data, error, close, sub, unsub.

### 5.4 Rate Limit (per `docs/api/rate-limit.md`)

```http
GET /v1/rate-limit/status
```

**Token Bucket 限流** (per R20 阶段 6 决策 D-04).

## 6. 9 organ 拟人化 API (per 决策 #11 阶段 4 frontend-proposal + 用户记忆 #3-#5)

**9 organ 拟人化** (主人用户记忆 #3-#5, "信息密度高 = 拟人化 + 拟物化"):

| Organ | API 文档 | 拟人化 |
|-------|---------|--------|
| brain | [`docs/api/organ-brain.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-brain.md) | 决策 |
| heart | [`docs/api/organ-heart.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-heart.md) | 状态 |
| memory | [`docs/api/organ-memory.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-memory.md) | 长期记忆 |
| mind | [`docs/api/organ-mind.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-mind.md) | 推理 |
| voice | [`docs/api/organ-voice.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-voice.md) | 表达 |
| eye | [`docs/api/organ-eye.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-eye.md) | 感知 |
| ear | [`docs/api/organ-ear.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-ear.md) | 监听 |
| hand | [`docs/api/organ-hand.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-hand.md) | 执行 |
| body | [`docs/api/organ-body.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/organ-body.md) | 总线 |

**9 organ = TUI/Tauri 终极前端核心** (per 决策 #11 + 用户记忆 #5).

## 7. Auth (per `docs/api/auth.md`)

```http
POST /v1/auth/login
POST /v1/auth/refresh
POST /v1/auth/logout
```

**WS Auth Link Token** (per R20 阶段 6 决策 D-03).

## 8. Error Codes (per `docs/api/error-codes.md`)

| Code | HTTP | 含义 |
|------|-----:|------|
| `APEIRETH_OK` | 200 | 成功 |
| `APEIRETH_INVALID_INPUT` | 400 | 输入校验失败 |
| `APEIRETH_UNAUTHORIZED` | 401 | 未授权 |
| `APEIRETH_FORBIDDEN` | 403 | 权限不足 |
| `APEIRETH_NOT_FOUND` | 404 | 资源不存在 |
| `APEIRETH_RATE_LIMIT` | 429 | 限流 |
| `APEIRETH_INTERNAL` | 500 | 内部错误 |

完整错误码见 [`docs/api/error-codes.md`](https://github.com/apeireth/apeireth-rust/blob/main/docs/api/error-codes.md).

## 9. 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

API 文档 0 借具体源码, 所有 endpoint 描述基于公开设计 + 决策链 #22-#62 + 整合 #4 commit 严守.

**借鉴 8/11 ✅ 真实施 (per 决策 #55 §3 + 决策 #57 §3)**:
- [clap-rs/clap 4.6.6](https://github.com/clap-rs/clap) — derive 模式
- [hyperium/hyper 0.1.20](https://github.com/hyperium/hyper) — 池复用
- [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) — MCP 协议
- [PyO3/PyO3 0.29.2](https://github.com/PyO3/PyO3) — pybridge
- [model-checking/kani 0.67.0](https://github.com/model-checking/kani) — 形式化
- [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) — StateGraph
- [obra/superpowers 6.2.0](https://github.com/obra/superpowers) — 9 skill files
- [BerriAI/litellm](https://github.com/BerriAI/litellm) — Provider Registry
