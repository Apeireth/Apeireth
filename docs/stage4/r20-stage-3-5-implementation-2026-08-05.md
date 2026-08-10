[Document-Meta]
Document: docs/stage4/r20-stage-3-5-implementation-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R20 阶段 3-5 (API 公开 + SDK 完善 + 文档营销)
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)

---

# R20 阶段 3-5 实施指南 (API 公开 + SDK 完善 + 文档营销)

> **性质**: R20 收产品 5 阶段里的"对外"3 阶段 (阶段 3-5), 紧接 `r20-stage-1-2-implementation-2026-08-05.md`. 给后续 sub-agent / team lead 照着干, **不写实际代码**.
>
> **依据**:
> - `docs/roadmap/r20-product-finalize-2026-08-05.md` (R20 总路线图, 5 阶段, 7-10 周) — M 标记, **不**读
> - `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` (R20 阶段 1-2 已写, 任务粒度模板)
> - `docs/stage4/tauri-team-collab-sop-2026-08-05.md` (Tauri 团队双边界 SOP, 5 步可执行)
> - `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` (SDK 现状, ~14000 LOC 低层 FFI, 缺用户面向 SDK 入口)
> - `docs/stage4/r-measure-verification-design-2026-08-05.md` (R-Measure 守门, 3 baseline 编译期 hardcode)
> - `reports/round10-12-asi-24-dim-9-sub-real-measurement-qa-engineer.md` (24 维 V0.5 LOCKED + V1136 9 子测度真实测量, **注**: 用户描述里叫 `apeireth-asi-24dim-api-2026-08-05.md`, 实际文件名以 round10-12 为准)
> - 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3 (本指南"阶段 X.Y" = 套 B R20 收产品 5 阶段子阶段)
>
> **承接**: R20 阶段 1-2 (产品 + 部署, 4 周) → R20 阶段 3-5 (本指南, 4.6 周) → R21 商业化.
>
> **不修改承诺**: 阶段 1+2+3 LOCKED 文档 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 全保留 (见 §7).

---

## §1 战略背景 (为什么)

### 1.1 R20 阶段 3-5 在 R20 5 阶段哪一行

| 阶段 | 焦点 | 时长 | 状态 |
|------|------|-----:|------|
| 1. 产品基础 | TUI + team-lead + mid-task bug | 1-2 周 | 🟢 已写 (`r20-stage-1-2-implementation`) |
| 2. 部署基础 | Docker + 离线包 + 系统包 + install | 2 周 | 🟢 已写 |
| **3. API 公开** | REST + WebSocket + OpenAPI + 鉴权 | **1.8 周** | 🟡 **本指南** |
| **4. SDK 完善** | Python / TS / Rust 3 SDK | **1.4-1.6 周** | 🟡 **本指南** |
| **5. 文档 + 营销** | 4 docs 站 + landing + 社区 | **1.4-1.6 周** | 🟡 **本指南** |

**总时长 4.6 周**, 紧接阶段 1-2 (4 周), 累计 R20 = 8.6 周. 目标完工: 2026-09-30.

### 1.2 阶段 3-5 是"对外"阶段

> 阶段 1-2 装电梯 + 大门 + 公路 (内部, 单机 / 容器).
> 阶段 3-5 **开大门, 铺公路到用户**: API 公开 (HTTP+WS) → SDK 包装 (3 语言) → 文档+社区 (用户上手 + 开发者接入 + 营销传播).
> R21 = 卖门票 (计费/订阅/配额, 不在本指南).

**3 阶段关系** (sequential, 不能并行):
- 阶段 3 (API) 必先完工 → OpenAPI 规范是 SDK 和文档的 single source
- 阶段 4 (SDK) 必在阶段 3 之后 → SDK 调 10 REST + 1 WS 端点
- 阶段 5 (文档) 必在阶段 3+4 之后 → 用户文档引 API+SDK, 开发者文档引架构

**关键不修改原则**:
- ❌ **不重写 41 crate** (R19 工程基线保留, 阶段 1-2 已增 team-lead + mcp::team)
- ❌ **不动 R11 baseline 3 值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, 编译期 hardcode)
- ❌ **不动 24 维 V0.5 + V1136 9 子测度** (R14 Rust rewrite round10-12 LOCKED)
- ✅ **只加** HTTP wrapper / WS 端点 / OpenAPI / 鉴权限流 / 3 SDK 入口 / 文档 / 社区

### 1.3 跟 R19+ 集成蓝图 + Tauri SOP 衔接

| R19+ / Tauri 资产 | R20 阶段 3-5 落地 |
|------------------|------------------|
| `apeireth-api` (4 协议 + 6 V2 = 10 端点) | 阶段 3.1: HTTP wrapper 包装, OpenAPI 规范定 |
| `apeireth-protocol` (4 adapter, ProviderEvent) | 阶段 3.2: WebSocket 双向流协议 (基于 ProviderEvent) |
| `apeireth-sdk` (低层 FFI 4 抽象: SdkVersion/Envelope/SdkErrorCode/C-ABI) | 阶段 4.1: 加 ApeirethClient 高层入口, 复用 4 抽象 |
| `apeireth-pybridge` + `src-py/` (Python wrapper 已生成) | 阶段 4.2: 加 client.py / api.py / models.py / streaming.py |
| napi-rs 桥 (新 crate `apeireth-jsbridge`) | 阶段 4.3: 全建 + npm 包 `@apeireth/sdk` |
| `crates/apeireth-team-lead/` (R20 阶段 1) | 阶段 3.1: `/v1/team/*` 端点包装 |
| `apeireth-mcp::team` 14 工具 (R20 阶段 1) | 阶段 3.1: 14 工具走 `/v1/team/*` 端点 |
| Tauri 团队 SOP 5 步 | 阶段 3.3: OpenAPI 规范 = Tauri 团队消费契约 |
| R-Measure verify (设计文档) | 阶段 3.5 / 4.4 / 5.4: 端到端守门 |

### 1.4 R-Measure 守门 (每子阶段必跑)

per `r-measure-verification-design-2026-08-05.md` §2.3 + APEIRETH-CONVENTIONS §11:

| 指标 | 值 | 含义 |
|---|---:|---|
| **V1141-R11** | 0.8682 | IC-001 fresh 测量 (17 维 V0.5 baseline 投影) |
| **V1131-R11** | 0.8532 | dashboard v05_total (17 维 V0.5 综合) |
| **V1136-R11** | 0.9063 | 真测引擎 (当前 9 子测度 LOCKED 实装, 历史 R11 baseline 7 子测度投影) |

**每子阶段结束 = 跑** `cargo run -p apeireth-r-measure-verify --release -- check --baseline r11`. 任何值掉 < baseline - 0.001 = fail, 阻塞 PR.

**24 维 LOCKED** 实装 (per `round10-12-asi-24-dim-9-sub-real-measurement-qa-engineer.md`):
- `apeireth-asi::V05_DIMENSION_NAMES` = 24 维 (5+5+5+5+4 分组)
- `apeireth-asi::V1136_SUBMEASURE_COUNT` = 9 (LOCKED, 编译期 const)
- `apeireth-asi::V1136_SUBMEASURE_NAMES` = 9 子测度名 (LOCKED 字符串数组)
- R-Measure verify 端做 24→17 投影 (per R-Measure 守门设计 §2.1, 主人从 v1077 抽权重)

### 1.5 跟 R20 路线图一致性

R20 路线图 §4 给的是 5 阶段任务表 + owner + 验证. **本指南把阶段 3-5 拆成 13 子阶段 + T-xxx 任务清单**, 密度比路线图高一个数量级, 给 sub-agent 照着干.

| 维度 | R20 路线图 §4 | 本指南 |
|------|------------|--------|
| 任务粒度 | 阶段级 | T-xxx 任务级 (32 个) |
| owner 建议 | 4 个 (backend/devops/fullstack/technical_writer) | 13 子阶段各自 owner |
| R-Measure 守门 | 阶段级 | **子阶段级** (13 个) |
| 风险清单 | 8 项 (R-001~R-008) | 10 项 (R-018~R-027, 阶段 3-5 专属) |
| 验收 | 阶段级 | 子阶段级 (T-xxx 验收标准) |

---

## §2 阶段 3 详细实施: API 公开 (1.8 周, 5 子阶段)

> **目标**: REST 10 端点 + WebSocket 1 端点 + OpenAPI 3.1 规范 + 鉴权限流 + 端到端守门.
>
> **总时长**: 9 天 (1.8 周).
> **owner 矩阵**:
> - backend_engineer (阶段 3.1 REST wrapper)
> - backend_engineer2 (阶段 3.2 WebSocket)
> - technical_writer + backend_engineer (阶段 3.3 OpenAPI)
> - backend_engineer + security_reviewer (阶段 3.4 鉴权限流)
> - qa_engineer (阶段 3.5 端到端守门)

### 2.1 阶段 3.1: REST API 包装 (3 天)

> **依据**: `apeireth-api` 已实装 10 端点 (4 协议 + 6 V2), 阶段 3.1 加 HTTP handler + JSON schema + 错误码 (per R-Measure verify 错误体系).

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-1001** | 10 REST 端点设计 (POST /v1/chat / POST /v1/team/spawn / GET /v1/sessions / GET /v1/sessions/:id / POST /v1/memory / GET /v1/memory/:id / GET /v1/organs / GET /v1/asi / POST /v1/sovereignty/check / POST /v1/agent/spawn) | 0 (设计) | `docs/api/rest-endpoints-2026-08-05.md` 10 行表 + 路径 + 方法 + 入参 + 出参 | R20 阶段 1 完工 (team-lead + mcp::team 已实装) |
| **T-1002** | HTTP handler (基于 `apeireth-api` axum/actix-web) — 10 端点 → 内部 trait call | 800 | `cargo build -p apeireth-api` 0 error + 10 端点 200 OK | T-1001 |
| **T-1003** | JSON schema + validator (10 端点 request/response, jsonschema crate) | 400 | 10 schema 跑通 happy path + 5 字段错误用例 fail | T-1002 |
| **T-1004** | 错误码体系 (per R-Measure verify §2.5: 7 类业务错误 + HTTP 状态码映射) | 200 | 401/403/404/409/422/429/500 7 个 case 全覆盖 | T-1002 + R-Measure verify 错误体系 |
| **T-1005** | 10 端点 unit tests (每端点 3 happy + 2 edge = 50 tests) | 500 | 50/50 PASS | T-1002~T-1004 |

**owner**: backend_engineer
**守门**: 50/50 tests pass + R-Measure V1141 ≥ 0.8682 (加 HTTP handler 不掉 ASI baseline) + 10 端点 200 OK 集成测试

**10 REST 端点清单** (per `apeireth-api` 既有 + Tauri SOP §5 资产 1):

| # | 路径 | 方法 | 调内部 trait | 关联 |
|---:|------|------|-------------|------|
| 1 | `/v1/chat` | POST | `apeireth-protocol::Adapter::send_message` | 4 协议 (OpenAI/Responses/Anthropic/Gemini) |
| 2 | `/v1/team/spawn` | POST | `apeireth-team-lead::spawn_team` | 阶段 1.2 |
| 3 | `/v1/sessions` | GET | `apeireth-session::list_sessions` | 14 工具 #12 |
| 4 | `/v1/sessions/:id` | GET | `apeireth-session::get_session` | 14 工具 #13 |
| 5 | `/v1/memory` | POST | `apeireth-session::write_memory` | ASI 北极星 V0.5 |
| 6 | `/v1/memory/:id` | GET | `apeireth-session::read_memory` | ASI 北极星 V0.5 |
| 7 | `/v1/organs` | GET | `apeireth-asi::organs_status` | 24 维 LOCKED |
| 8 | `/v1/asi` | GET | `apeireth-asi::V1136Engine::snapshot_24dim` | V1136 9 子测度 |
| 9 | `/v1/sovereignty/check` | POST | `apeireth-sovereignty::check` | V3 守门 |
| 10 | `/v1/agent/spawn` | POST | `apeireth-agent::spawn_agent` | 14 工具 #1 |

**关键技术点**:
- **HTTP framework**: axum 0.7+ (tower 中间件生态最完善, 跟 tokio 完美集成)
- **JSON schema**: `schemars` crate 派生 (跟 serde 集成, 不用手写 schema)
- **错误映射**: 内部 `SdkError` → HTTP 状态码 (1:1 映射表, per T-1004)
- **不重写协议层**: HTTP handler 只做 JSON ↔ 内部 trait call 翻译, 4 协议 LLM 真接走 `apeireth-protocol::Adapter`

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-3-1-measure-2026-08-XX.md
```

### 2.2 阶段 3.2: WebSocket 双向流 (2 天)

> **依据**: `apeireth-protocol` 已实装 `ProviderEvent` 5 变体 (R19), 阶段 3.2 加 WebSocket 端点包装.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-1101** | WS 端点 `wss://api/v1/stream` (基于 axum WebSocket upgrade) | 300 | `wscat -c wss://localhost:8080/v1/stream` 连接成功, 收到 1 帧 ProviderEvent | T-1002 (HTTP handler 基础设施) |
| **T-1102** | 双向消息协议 (server→client: ProviderEvent 流 / client→server: 控制消息: pause/resume/cancel) | 400 | 4 场景: chat 流 / team spawn 流 / agent idle 通知 / client cancel | T-1101 + `apeireth-protocol::ProviderEvent` |
| **T-1103** | 流式响应包装 (bytes_stream → WebSocket 帧) | 200 | chat 流式响应 P50 < 100ms 第一帧 | T-1101 + T-1102 |
| **T-1104** | 心跳 + 重连 (ping 每 30s / 客户端断线重连 / 服务端 idle timeout 5min) | 200 | 模拟客户端断 60s 后重连, 服务端不挂 | T-1101~T-1103 |

**owner**: backend_engineer2
**守门**: 4 场景端到端测试 PASS + R-Measure V1141 ≥ 0.8682 + WS 连接 P99 < 50ms

**WS 协议设计** (per `apeireth-protocol::ProviderEvent` 5 变体):

| 帧类型 | 方向 | 内容 | 用途 |
|--------|------|------|------|
| `StreamStart` | s→c | `{ run_id, model, timestamp }` | 流开始 |
| `StreamDelta` | s→c | `{ run_id, delta, seq }` | 流式 token |
| `StreamEnd` | s→c | `{ run_id, finish_reason, total_tokens }` | 流结束 |
| `ToolCall` | s→c | `{ run_id, tool_name, args }` | 工具调用 |
| `StreamError` | s→c | `{ run_id, error_code, message }` | 流错误 |
| `ControlPause` | c→s | `{ run_id }` | 客户端暂停 |
| `ControlResume` | c→s | `{ run_id }` | 客户端恢复 |
| `ControlCancel` | c→s | `{ run_id }` | 客户端取消 |

**关键技术点**:
- **WS library**: `axum::extract::ws` (跟 axum HTTP handler 共享状态)
- **背压**: tokio::sync::mpsc 缓冲 (避免客户端慢消费拖垮 server)
- **不阻塞**: 内部 trait call 用 `tokio::spawn` + `oneshot` channel, 不阻塞 WS event loop
- **client cancel**: 服务端收到 `ControlCancel` → 中止 `ProviderEvent` 流 → graceful shutdown

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# WS 不掉 ASI baseline (V1131 dashboard 5 Self 影响 0, V1141 影响 0)
# 报告: reports/r20-stage-3-2-measure-2026-08-XX.md
```

### 2.3 阶段 3.3: OpenAPI 3.1 规范 (2 天)

> **依据**: Tauri SOP §3 Step 2 + 阶段 3.1+3.2 端点设计.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-1201** | `docs/api/openapi.yaml` 完整定义 (10 REST + 1 WS, OpenAPI 3.1) | 800 | `npx swagger-cli validate docs/api/openapi.yaml` 0 error | T-1001~T-1104 全部完工 |
| **T-1202** | swagger-cli validate (CI 阻塞 PR) | 50 (CI yml) | PR 改 `crates/apeireth-api/src/*.rs` 但没改 openapi.yaml → 阻塞 | T-1201 + CI 集成 |
| **T-1203** | ReDoc / SwaggerUI 文档站 (`docs.apeireth.io/api/`, 静态托管) | 300 | 浏览器打开 `docs.apeireth.io/api/` 看到 10 REST + 1 WS 完整 schema + 试调 | T-1201 |
| **T-1204** | 契约测试 (per OpenAPI 自动生成 Rust 客户端 + 端到端对比) | 400 | 10 端点契约测试: OpenAPI schema vs 实际 server 响应, byte-equal | T-1201 + T-1203 |

**owner**: technical_writer (主) + backend_engineer (review)
**守门**: swagger-cli validate 0 error + 契约测试 10 端点 byte-equal + Redoc 渲染无 error + R-Measure V1141 ≥ 0.8682

**OpenAPI 3.1 关键字段** (per OpenAPI 3.1 规范):

```yaml
openapi: 3.1.0
info:
  title: Apeireth API
  version: 2.0.0-alpha
  description: |
    Apeireth v2.0.0-alpha 长程 AI 成长平台公开 API.
    10 REST 端点 (4 协议 LLM + team + memory + organs + asi + sovereignty + agent) + 1 WebSocket 双向流.
servers:
  - url: https://api.apeireth.io/v1
  - url: wss://api.apeireth.io/v1
paths:
  /chat: { post: { ... } }
  /team/spawn: { post: { ... } }
  /sessions: { get: { ... } }
  # ... 10 端点
  /stream: { get: { ... } }  # WebSocket upgrade
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
  schemas:
    ChatRequest: { ... }
    ProviderEvent: { ... }
    # ... 共 20 schemas
```

**关键技术点**:
- **OpenAPI 3.1** (不是 3.0): 支持 JSON Schema 2020-12, webhooks
- **WS 描述**: OpenAPI 3.1 还没原生 WebSocket 支持, 用 `x-websocket-protocol` extension 字段, 写明帧 schema
- **CI 阻塞**: per Tauri SOP §4.2 docs-sync-check.yml, 改 `crates/apeireth-api/src/*.rs` 必改 `docs/api/openapi.yaml`
- **契约测试**: 用 `proptest` + `openapi-types` 双向校验 (server 响应 ⊆ OpenAPI schema)

**R-Measure 守门**:
```bash
# OpenAPI 规范跟代码同步 (CI 自动跑)
npx swagger-cli validate docs/api/openapi.yaml
# 报告: reports/r20-stage-3-3-measure-2026-08-XX.md (含 swagger-cli 输出)
```

### 2.4 阶段 3.4: 鉴权 + 限流 (1 天)

> **依据**: O-5 17:58 12 急救路径 + V3 守门不假装 + R21 商业化预留.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-1301** | API key 鉴权 (Bearer token, 编译期 hardcode 鉴权头 + 401/403 测试) | 200 | 无 key → 401, 错 key → 401, 过期 key → 401, 正确 key → 200 | T-1002 (HTTP handler) |
| **T-1302** | 限流 (per `apeireth-constraint` E 急救路径, token bucket 算法) | 300 | 单 IP 100 req/s 触发 429, 30s 后自动恢复 | T-1002 + `apeireth-constraint` |
| **T-1303** | 审计日志 (per S-1 22:33 实验室, 所有 API call 写 `~/.apeireth/audit.log`) | 150 | 审计日志含 timestamp / api_key_hash / endpoint / status / duration | T-1301 |
| **T-1304** | 配额管理 (R21+ 商业化预留, 编译期 stub, 不实装) | 50 | `QuotaManager` trait stub, R21 实装 | T-1301 |

**owner**: backend_engineer + security_reviewer
**守门**: 7 个 case 全覆盖 (无 key 401 / 错 key 401 / 过期 key 401 / 限流 429 / 恢复 200 / 审计日志完整 / 配额 stub) + R-Measure V1141 ≥ 0.8682

**鉴权流程** (per O-5 17:58 12 急救):

```
client request
  ↓
HTTP handler 拦截
  ↓
Authorization: Bearer <api_key>
  ↓
keyring 查 (Linux Secret Service / macOS Keychain / Windows Credential Manager)
  ↓
匹配 → 200 + 审计日志
不匹配 → 401 + 审计日志
  ↓
限流检查 (token bucket per api_key)
  ↓
超限 → 429 + Retry-After header
未超限 → 进入业务 handler
  ↓
business logic
  ↓
审计日志 (含 duration)
  ↓
response
```

**限流策略** (per O-5 17:58 不假装 + R21 商业化预留):
- **E 急救路径**: P0 endpoint (`/v1/sovereignty/check` / `/v1/agent/spawn`) 限流 = 1000 req/s 不限 (守门 + 安全)
- **普通路径**: 100 req/s per api_key (per `apeireth-constraint` 配置)
- **WS 路径**: 连接数限 10 per api_key (避免资源耗尽)
- **R21 商业化**: 加 `QuotaManager` trait, 配 free / pro / enterprise 三档配额

**关键技术点**:
- **API key 存储**: 不存明文, 用 `keyring` crate (OS-level credential)
- **限流算法**: token bucket, 每 api_key 一个 bucket, 100 token, refill 100/s
- **审计日志**: append-only, 不删, 不修 (per S-1 22:33 实验室)
- **不假装**: T-1304 配额管理只 stub, 不实装, R21 写 ADR 后再动

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 鉴权 + 限流不影响 ASI baseline (V1141/V1131/V1136 都不动)
# 报告: reports/r20-stage-3-4-measure-2026-08-XX.md
```

### 2.5 阶段 3.5: API 集成 + R-Measure 守门 (1 天)

> **依据**: 阶段 3.1-3.4 全部完工后的端到端 + R-Measure verify 脚本.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-1401** | 端到端测试 (10 REST 端点 + 1 WS 端到端, 5 happy + 5 失败场景) | 500 | 10/10 端到端 PASS, 5 失败场景全部预期失败 | 阶段 3.1~3.4 全部完工 |
| **T-1402** | R-Measure verify 跑 (V1141 + V1131 + V1136 3 值全守) | 0 (跑脚本) | `cargo run -p apeireth-r-measure-verify -- check --baseline r11` 返 exit 0 | T-1401 |
| **T-1403** | OpenAPI 契约测试 (10 端点 server 响应 vs OpenAPI schema byte-equal) | 200 | 10 端点契约测试 PASS | T-1401 + T-1204 |
| **T-1404** | 性能测试 (P95 < 2s, 10 REST 端点 + 1 WS 连接建立) | 200 | 100 次请求, P95 < 2000ms | T-1401 |

**owner**: qa_engineer
**守门**: R-Measure baseline 3 值守 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) + 15/15 端到端 PASS + 10 端点契约测试 PASS + P95 < 2s

**端到端 10 用例**:

| # | 端点 | 场景 | 期望 |
|---:|------|------|------|
| 1 | POST /v1/chat | 4 协议各 1 (OpenAI/Responses/Anthropic/Gemini) | 200 + 流式响应 |
| 2 | POST /v1/team/spawn | spawn 2 子 agent | 200 + 2 team_id |
| 3 | GET /v1/sessions | 列 10 session | 200 + 10 session 元数据 |
| 4 | GET /v1/sessions/:id | 查具体 session | 200 + session 详情 |
| 5 | POST /v1/memory | 写 1 memory | 200 + memory_id |
| 6 | GET /v1/memory/:id | 读 1 memory | 200 + memory 内容 |
| 7 | GET /v1/organs | 查 24 维器官状态 | 200 + 24 维数值 |
| 8 | GET /v1/asi | 查 V1136 9 子测度 | 200 + 9 子测度 |
| 9 | POST /v1/sovereignty/check | 检查 1 操作 | 200 + 决策 (allow/deny) |
| 10 | POST /v1/agent/spawn | spawn 1 agent | 200 + agent_id |

**WS 端到端 1 用例**:
- 连接 `wss://localhost:8080/v1/stream` → 发送 chat 流请求 → 收到 5 帧 ProviderEvent (StreamStart + 3 StreamDelta + StreamEnd) → 客户端发 ControlCancel (无流) → 关闭

**失败场景 5 用例**:

| # | 场景 | 期望 |
|---:|------|------|
| 1 | 无 API key | 401 |
| 2 | 错 API key | 401 |
| 3 | 限流 (100 req/s 触发) | 429 + Retry-After |
| 4 | 错误 JSON schema | 422 |
| 5 | LLM 500 | 500 + 审计日志 |

**R-Measure 守门** (端到端 + 三值):
```bash
# 端到端
cargo test -p apeireth-api --test e2e
# R-Measure 三值
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-3-5-measure-2026-08-XX.md (含三值 + 15/15 tests + P95 数据)
```

**阶段 3 完工报告**: `reports/r20-stage-3-complete-2026-08-XX.md` (per r20 §4 守门要求).

---

## §3 阶段 4 详细实施: SDK 完善 (1.4-1.6 周, 4 子阶段)

> **目标**: Rust SDK 高层入口 + Python SDK + TypeScript SDK + 3 SDK 端到端守门.
>
> **总时长**: 7-8 天 (1.4-1.6 周).
> **owner 矩阵**:
> - backend_engineer + architect (阶段 4.1 Rust SDK)
> - fullstack_engineer (阶段 4.2 Python + 4.3 TypeScript)
> - backend_engineer (阶段 4.4 端到端验证)

### 3.1 阶段 4.1: apeireth-sdk 公开 API 补齐 (2 天)

> **依据**: `apeireth-sdk-gap-analysis-2026-08-05.md` §3.1 (Rust SDK 缺口清单) + `tauri-team-collab-sop` §5 资产 1 (HTTP 契约).

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-2001** | `src/client.rs` (ApeirethClient struct, 11 方法签名: chat / memory_read / memory_write / organs_status / asi_score / sovereignty_check / agent_spawn / agent_send / agent_wait_idle / team_spawn / stream) | 1000 | `cargo build -p apeireth-sdk` 0 error, 11 方法 pub fn 列出 | R20 阶段 3 完工 (10 REST + 1 WS 端点已定) |
| **T-2002** | `src/http.rs` (HTTP wrapper, 调 `apeireth-api` 10 REST 端点, reqwest client) | 700 | 10 端点 200 OK + 7 错误 case 全覆盖 | T-2001 |
| **T-2003** | `src/ws.rs` (WebSocket wrapper, 调 `/v1/stream`, tokio-tungstenite) | 500 | 4 场景: chat 流 / team spawn 流 / agent idle 通知 / client cancel | T-2001 |
| **T-2004** | 11 方法公开 + config (ApeirethConfig { base_url, api_key, timeout, max_retries }) + error (SdkError 7 错误码复用) | 300 | Config builder 跑通, 11 方法 + 7 错误码 + 4 抽象 (SdkVersion/Envelope/SdkErrorCode/C-ABI) 全在 pub API | T-2001~T-2003 |
| **T-2005** | 4 抽象保留 (SdkVersion + Envelope + SdkErrorCode + C-ABI 不动) | 0 (验证) | 4 抽象 import 路径不变, 公开 API 不破坏 | T-2001 |

**owner**: backend_engineer (主) + architect (review)
**守门**: `cargo build -p apeireth-sdk` 0 error + 11 方法签名定稿 (architect + Mavis 拍板) + 4 抽象保留 (向后兼容) + R-Measure V1141 ≥ 0.8682

**11 方法签名定稿** (per `apeireth-sdk-gap-analysis` §3.4):

```rust
// 1. chat
async fn chat(&self, messages: Vec<Message>, model: String, stream: bool) -> Result<ChatResponse, SdkError>
// 2. memory_read
async fn memory_read(&self, id: String) -> Result<Memory, SdkError>
// 3. memory_write
async fn memory_write(&self, memory: Memory) -> Result<String, SdkError>
// 4. organs_status
async fn organs_status(&self) -> Result<Vec<OrganStatus>, SdkError>
// 5. asi_score
async fn asi_score(&self) -> Result<AsiScore, SdkError>
// 6. sovereignty_check
async fn sovereignty_check(&self, action: Action) -> Result<Decision, SdkError>
// 7. agent_spawn
async fn agent_spawn(&self, agent_type: String, prompt: String) -> Result<AgentId, SdkError>
// 8. agent_send
async fn agent_send(&self, agent_id: AgentId, message: String) -> Result<SendResult, SdkError>
// 9. agent_wait_idle
async fn agent_wait_idle(&self, agent_id: AgentId, timeout: Duration) -> Result<AgentStatus, SdkError>
// 10. team_spawn
async fn team_spawn(&self, config: TeamConfig) -> Result<TeamId, SdkError>
// 11. stream
async fn stream(&self, messages: Vec<Message>, model: String) -> Result<impl Stream<Item = ProviderEvent>, SdkError>
```

**4 抽象保留约束** (per `apeireth-sdk-gap-analysis` §2.1 + ADR-0011):
- ❌ **不**改 `SdkVersion` + `negotiate` (向后兼容)
- ❌ **不**改 `Envelope` + `WireKind` (wire-format 稳定)
- ❌ **不**改 `SdkErrorCode` (7 错误码 + 数字码跨语言一致)
- ❌ **不**改 `#[no_mangle] extern "C"` 顶层 API (3 stub V2 D2 实装)

**新增抽象**:
- `ApeirethClient` (高层客户端, 11 方法)
- `ApeirethConfig` (builder 模式, 4 字段)
- `SdkError` (复用 + 加 http_error / ws_error 2 新变体)

**关键技术点**:
- **HTTP client**: reqwest (workspace 已有), 4 协议 LLM 转发走 `apeireth-api` REST 不直接调 provider
- **WebSocket client**: tokio-tungstenite (新依赖, 选 stable 版本), async stream
- **错误处理**: `Result<T, SdkError>`, 7 错误码复用 + 2 新变体 (http_error / ws_error)
- **async runtime**: tokio (workspace 已有, 双 runtime 共存)

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 加高层 SDK 不影响 ASI baseline (V1141/V1131/V1136 都不动)
# 报告: reports/r20-stage-4-1-measure-2026-08-XX.md
```

### 3.2 阶段 4.2: Python SDK (2 天)

> **依据**: `apeireth-sdk-gap-analysis` §3.2 (Python SDK 缺口) + `tauri-team-collab-sop` §5 资产 1 (HTTP 契约).

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-2101** | `src-py/apeireth_sdk/client.py` (ApeirethClient class, HTTP via httpx + async) | 600 | `from apeireth_sdk import ApeirethClient; c = ApeirethClient(base_url="http://localhost:8080", api_key="xxx")` 跑通 | 阶段 4.1 完工 (11 方法签名定稿) |
| **T-2102** | `src-py/apeireth_sdk/api.py` (11 方法实现 + 4 协议 LLM 转发) | 400 | 10 REST + 1 WS 同步 / 异步接口都跑通 | T-2101 |
| **T-2103** | `src-py/apeireth_sdk/models.py` (Pydantic 数据类, 跟 Rust SDK 字段一一对应) | 500 | Pydantic 字段跟 Rust struct 字段 byte-equal | T-2101 |
| **T-2104** | `src-py/pyproject.toml` (poetry 打包配置, 含 cdylib 引用) | 100 | `pip install -e .` 成功 + `apeireth_sdk` 模块 import OK | T-2101~T-2103 |
| **T-2105** | 5 端到端测试 (`tests_py/test_*.py`, 调本地 `apeireth-api` server) | 300 | 5/5 PASS | T-2101~T-2104 |

**owner**: fullstack_engineer
**守门**: `pip install -e .` 成功 + 5/5 端到端测试 PASS + Pydantic 字段对齐 Rust SDK + R-Measure V1141 ≥ 0.8682

**Python SDK 公开 API** (跟 Rust 11 方法一一对应, snake_case 命名):

```python
class ApeirethClient:
    def __init__(self, base_url: str, api_key: str, timeout: float = 30.0, max_retries: int = 3): ...
    async def chat(self, messages: List[Message], model: str, stream: bool = False) -> ChatResponse: ...
    async def memory_read(self, id: str) -> Memory: ...
    async def memory_write(self, memory: Memory) -> str: ...
    async def organs_status(self) -> List[OrganStatus]: ...
    async def asi_score(self) -> AsiScore: ...
    async def sovereignty_check(self, action: Action) -> Decision: ...
    async def agent_spawn(self, agent_type: str, prompt: str) -> AgentId: ...
    async def agent_send(self, agent_id: AgentId, message: str) -> SendResult: ...
    async def agent_wait_idle(self, agent_id: AgentId, timeout: float) -> AgentStatus: ...
    async def team_spawn(self, config: TeamConfig) -> TeamId: ...
    async def stream(self, messages: List[Message], model: str) -> AsyncIterator[ProviderEvent]: ...
```

**关键技术点**:
- **HTTP client**: httpx (async, 跟 FastAPI / aiohttp 生态一致)
- **WebSocket client**: websockets 库 (async iterator)
- **Pydantic**: v2 (字段跟 Rust SDK 一一对应, 自动验证)
- **错误处理**: 7 错误码复用 + 2 新变体 (`HttpError` / `WsError`)
- **cdylib 复用**: `src-py/apeireth_sdk/_ffi.py` 已生成, 复用不重做

**5 端到端测试**:
- `test_01_chat.py` — 4 协议各 1 次 chat
- `test_02_memory.py` — 写 + 读 memory
- `test_03_organs.py` — 查 24 维器官状态
- `test_04_team.py` — spawn 2 子 agent
- `test_05_streaming.py` — WS 流式 chat

**R-Measure 守门**:
```bash
# Python SDK 端到端测试跑
cd src-py && pytest tests_py/ -v
# R-Measure 三值 (跟 Rust 共享)
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-4-2-measure-2026-08-XX.md
```

### 3.3 阶段 4.3: TypeScript SDK (2 天)

> **依据**: `apeireth-sdk-gap-analysis` §3.3 (TypeScript SDK 缺口) + napi-rs 桥.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-2201** | `crates/apeireth-jsbridge/` 新 crate (napi-rs 桥, 桥接 `apeireth-sdk` C-ABI) | 900 | `cargo build -p apeireth-jsbridge` 0 error, `.node` 文件生成 | 阶段 4.1 完工 (Rust SDK 11 方法) |
| **T-2202** | `packages/typescript-sdk/` 独立 npm 包 (TypeScript 类型 + 11 方法) | 1800 | `npm install @apeireth/sdk` 成功 + `import { Apeireth } from "@apeireth/sdk"` 跑通 | T-2201 |
| **T-2203** | OpenAPI 类型自动生成 (`openapi-typescript` → `src/types.ts`) | 200 | `src/types.ts` 跟 OpenAPI 3.1 规范一致 (10 REST + 1 WS) | T-2201 + 阶段 3.3 完工 |
| **T-2204** | 10 REST + 1 WS 同步 / 异步接口 | 800 | TypeScript Promise / async iterator 跑通 | T-2201 + T-2202 |
| **T-2205** | 5 端到端测试 (vitest, 调本地 `apeireth-api` server) | 500 | 5/5 PASS | T-2201~T-2204 |

**owner**: fullstack_engineer
**守门**: `npm install @apeireth/sdk` 成功 + 5/5 端到端测试 PASS + TypeScript 类型跟 OpenAPI 一致 + R-Measure V1141 ≥ 0.8682

**TypeScript SDK 公开 API** (跟 Rust 11 方法一一对应, camelCase 命名):

```typescript
export class Apeireth {
  constructor(config: ApeirethConfig);
  async chat(messages: Message[], model: string, stream?: boolean): Promise<ChatResponse>;
  async memoryRead(id: string): Promise<Memory>;
  async memoryWrite(memory: Memory): Promise<string>;
  async organsStatus(): Promise<OrganStatus[]>;
  async asiScore(): Promise<AsiScore>;
  async sovereigntyCheck(action: Action): Promise<Decision>;
  async agentSpawn(agentType: string, prompt: string): Promise<AgentId>;
  async agentSend(agentId: AgentId, message: string): Promise<SendResult>;
  async agentWaitIdle(agentId: AgentId, timeout: number): Promise<AgentStatus>;
  async teamSpawn(config: TeamConfig): Promise<TeamId>;
  async *stream(messages: Message[], model: string): AsyncIterableIterator<ProviderEvent>;
}
```

**关键技术点**:
- **napi-rs 桥**: 桥接 `apeireth-sdk` C-ABI, 把 11 Rust 方法暴露成 Node addon
- **TypeScript 类型**: `openapi-typescript` 从 `docs/api/openapi.yaml` 自动生成
- **命名映射**: Rust snake_case (`memory_read`) → TS camelCase (`memoryRead`), 内部转换
- **错误处理**: 7 错误码 + 2 新变体 (`HttpError` / `WsError`), 跨语言一致

**5 端到端测试** (vitest):
- `test_01_chat.test.ts` — 4 协议各 1 次 chat
- `test_02_memory.test.ts` — 写 + 读 memory
- `test_03_organs.test.ts` — 查 24 维器官状态
- `test_04_team.test.ts` — spawn 2 子 agent
- `test_05_streaming.test.ts` — WS 流式 chat

**R-Measure 守门**:
```bash
# TypeScript SDK 端到端测试
cd packages/typescript-sdk && npm test
# R-Measure 三值 (跟 Rust 共享)
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-4-3-measure-2026-08-XX.md
```

### 3.4 阶段 4.4: 端到端 + 跨语言一致性 (1-2 天)

> **依据**: 阶段 4.1-4.3 全部完工后的端到端 + 3 SDK 同输入 → 同输出验证.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-2301** | 5 端到端测试 (3 SDK 同输入 → 同输出) | 600 | 3 SDK × 5 场景 = 15/15 byte-equal | 阶段 4.1~4.3 全部完工 |
| **T-2302** | 同步 vs 异步接口对比 (Rust sync vs async / Python async / TS async) | 200 | 3 SDK 在同输入下结果 byte-equal | T-2301 |
| **T-2303** | 错误处理一致性 (7 错误码 + 2 新变体, 3 SDK 映射一致) | 200 | 3 SDK 错误码 enum 字面值一致 | T-2301 |
| **T-2304** | 性能基准 (3 SDK 启动时间 + chat P50) | 200 | Rust < 50ms 启动, Python < 200ms 启动, TS < 300ms 启动 | T-2301 |
| **T-2305** | R-Measure verify 跑 (V1141 + V1131 + V1136 3 值全守) | 0 (跑脚本) | `cargo run -p apeireth-r-measure-verify -- check --baseline r11` 返 exit 0 | T-2301 |

**owner**: backend_engineer (主) + fullstack_engineer (Python/TS review)
**守门**: 15/15 跨语言测试 byte-equal + 7 错误码一致 + 性能基准达标 + R-Measure 3 值全守 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)

**5 跨语言场景**:

| # | 场景 | 输入 | 期望 3 SDK 输出 |
|---:|------|------|---------------|
| 1 | chat 简单问题 | `messages=[{"role":"user","content":"hi"}]`, `model="gpt-4o"` | 3 SDK 返 ChatResponse, content byte-equal |
| 2 | memory 写 + 读 | `memory={content:"test"}` | 3 SDK 写成功 + 读 byte-equal |
| 3 | organs 查 | (无输入) | 3 SDK 返 24 维 OrganStatus, 数值精度一致 (f64) |
| 4 | team spawn | `config={agents: 2}` | 3 SDK 返 2 TeamId, 格式一致 |
| 5 | stream chat | `messages=...`, `model="gpt-4o"` | 3 SDK 流式 ProviderEvent 5 帧 byte-equal |

**关键技术点**:
- **byte-equal**: 用 `serde_json::Value` 解析后比较, 避免浮点精度问题
- **跨语言 ABI**: 复用 4 抽象 (SdkVersion / Envelope / SdkErrorCode / C-ABI), 3 SDK 走同一套 wire format
- **CI matrix**: 3 SDK 各自 CI 跑 + 跨语言一致 CI 跑 (GitHub Actions matrix)

**R-Measure 守门** (端到端 + 跨语言 + 三值):
```bash
# 3 SDK 端到端
cargo test -p apeireth-sdk --test e2e
cd src-py && pytest tests_py/ -v
cd packages/typescript-sdk && npm test
# 跨语言一致
./scripts/cross_lang_test.sh  # 跑 5 场景, 3 SDK 对比
# R-Measure 三值
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-4-4-measure-2026-08-XX.md (含三值 + 15/15 跨语言 + 性能数据)
```

**阶段 4 完工报告**: `reports/r20-stage-4-complete-2026-08-XX.md`.

---

## §4 阶段 5 详细实施: 文档 + 营销 (1.4-1.6 周, 4 子阶段)

> **目标**: 用户文档站 + 开发者文档站 + landing + 社区基础设施.
>
> **总时长**: 7-8 天 (1.4-1.6 周).
> **owner 矩阵**:
> - technical_writer (阶段 5.1 用户文档 + 5.2 开发者文档)
> - frontend_engineer + technical_writer (阶段 5.3 landing)
> - community_manager (新角色, 阶段 5.4 社区)

### 4.1 阶段 5.1: 用户文档站 (2 天)

> **依据**: r20 §4 阶段 5 + docs/ 现状 (R17+ 文档分散, R20 整合到统一站).

| T-ID | 任务 | 估 LOC / 字数 | 验收 | 依赖 |
|------|------|-------------:|------|------|
| **T-3001** | 5 分钟快速开始 (Quick Start, `docs.apeireth.io/quickstart`) | 800 字 | 新用户 5 分钟走通: 安装 → 配 API key → 启动 → 跑 TUI | R20 阶段 2 完工 (一键安装) |
| **T-3002** | 安装指南 (Linux/macOS/Windows 4 平台, `docs.apeireth.io/install`) | 1500 字 | 4 平台安装走查, 截图 + 失败排查 | R20 阶段 2 完工 |
| **T-3003** | 教程 (10 教程覆盖 80% 用户场景, `docs.apeireth.io/tutorials`) | 5000 字 | 10 教程跑通, 覆盖 chat / memory / team / organs / sovereignty / stream / etc | R20 阶段 4 完工 (3 SDK) |
| **T-3004** | FAQ (20 常见问题, `docs.apeireth.io/faq`) | 2000 字 | 20 问答覆盖: 安装 / 启动 / API key / 限流 / 错误码 / 升级 | T-3001~T-3003 |
| **T-3005** | Docusaurus / mkdocs 静态站 (部署 `docs.apeireth.io`) | 500 (配置) | GitHub Pages 部署, 域名 DNS 解析, Lighthouse 90+ | T-3001~T-3004 |

**owner**: technical_writer
**守门**: 10 教程 + 20 FAQ 跑通 + 4 平台安装指南验证 + Docusaurus 部署成功 + Lighthouse 90+ + R-Measure V1141 ≥ 0.8682

**10 教程清单**:

| # | 标题 | 覆盖场景 |
|---:|------|---------|
| 1 | Your first chat | 4 协议 LLM chat 入门 |
| 2 | Memory management | 写读 memory, ASI 北极星 V0.5 |
| 3 | Organs dashboard | 24 维器官状态可视化 |
| 4 | ASI score | V1136 9 子测度解读 |
| 5 | Team collaboration | spawn 2 子 agent |
| 6 | Sovereignty check | V3 守门 + 12 急救路径 |
| 7 | Streaming chat | WebSocket 流式消费 |
| 8 | SDK quickstart (Python) | `pip install apeireth` 跑通 |
| 9 | SDK quickstart (TypeScript) | `npm install @apeireth/sdk` 跑通 |
| 10 | SDK quickstart (Rust) | `cargo add apeireth-sdk` 跑通 |

**关键技术点**:
- **静态站生成器**: Docusaurus 3.x (React 生态, MDX 支持, 主题丰富, 推荐) — per 风险 R-024
- **代码示例**: 5 语言 (Bash / Python / TypeScript / Rust / JSON) 都用 `<Tabs>` 标签切换
- **截图**: per 教程配 1-3 张截图, 截图用 `image_synthesize` 工具生成 (避免人肉画)
- **SEO**: meta tags + sitemap + structured data (JSON-LD)

**R-Measure 守门**:
```bash
# 文档站不直接影响 ASI baseline, 但守门要跑 (编译期 hardcode)
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-5-1-measure-2026-08-XX.md
```

### 4.2 阶段 5.2: 开发者文档站 (2 天)

> **依据**: Tauri SOP §5 资产 1-4 (HTTP 契约 / WS 协议 / 14 工具 / 11 类 IPC) + `global-architecture-map-2026-08-05.md` 13 张 Mermaid 图.

| T-ID | 任务 | 估 LOC / 字数 | 验收 | 依赖 |
|------|------|-------------:|------|------|
| **T-3101** | 架构图 (13 张 Mermaid 引用 `global-architecture-map-2026-08-05.md`, `dev.apeireth.io/architecture`) | 200 (Mermaid 引用) | 13 张图全部可渲染 + 可点开 | R20 阶段 1-2 完工 |
| **T-3102** | API 参考 (OpenAPI 3.1 自动渲染, `dev.apeireth.io/api`) | 300 (配置) | ReDoc 渲染 OpenAPI 规范, 10 REST + 1 WS 全在 | R20 阶段 3.3 完工 |
| **T-3103** | SDK 文档 (Python/TS/Rust 3 SDK, `dev.apeireth.io/sdk/{python,typescript,rust}`) | 3000 字 | 3 SDK 完整 API reference + 5 端到端示例 each | R20 阶段 4 完工 |
| **T-3104** | 扩展指南 (怎么加自定义 advisor / tool / provider, `dev.apeireth.io/extension`) | 2000 字 | 3 扩展场景跑通: 加 advisor / 加 tool / 加 provider | T-3101~T-3103 |
| **T-3105** | 性能调优 + R-Measure 守门 (`dev.apeireth.io/performance`) | 1500 字 | 性能调优 4 场景 + R-Measure verify 脚本使用 | T-3101~T-3104 |

**owner**: technical_writer (主) + backend_engineer (review)
**守门**: 13 张架构图全渲染 + OpenAPI ReDoc 全在 + 3 SDK 文档完整 + 3 扩展场景跑通 + R-Measure V1141 ≥ 0.8682

**13 张架构图清单** (per `global-architecture-map-2026-08-05.md`):
- 1 张总图 (5 层架构 overview)
- 4 张分层图 (API / Protocol / Agent / ASI / Memory)
- 4 张流程图 (chat flow / team flow / memory write / mid-task)
- 4 张子图 (4 协议 adapter / 14 工具 / 24 维 / 9 子测度)

**关键技术点**:
- **Mermaid 引用**: 用 `<Mermaid>` 组件, 引用 `global-architecture-map-2026-08-05.md` 单一来源, 改一处全改
- **OpenAPI 自动渲染**: ReDoc 静态站, 直接吃 `docs/api/openapi.yaml`, 无需手写
- **SDK 文档**: rustdoc + pdoc + TypeDoc 自动生成, 人工写 5 端到端示例
- **扩展指南**: 3 场景 (advisor / tool / provider) 走真实代码路径, 跑通 3 SDK 调用

**R-Measure 守门**:
```bash
# 开发者文档站部署后跑
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-5-2-measure-2026-08-XX.md
```

### 4.3 阶段 5.3: landing page + 营销 (2 天)

> **依据**: r20 §4 阶段 5 + R21 商业化预留.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-3201** | 落地页 (`apeireth.io`, 特征 / 案例 / 截图 / CTA) | 800 | 4 屏滚动, Lighthouse 90+, 移动端响应式 | T-3005 (Docusaurus 复用) |
| **T-3202** | feature 页面 (5 大特征 + 12 子规范, `apeireth.io/features`) | 600 | 5 特征 + 12 子规范, 每个 1 段说明 + 截图 | T-3201 |
| **T-3203** | pricing 页面 (R21+ 商业化预留, `apeireth.io/pricing`) | 300 | 3 档 (free / pro / enterprise) 占位, R21 启收费 | T-3201 |
| **T-3204** | changelog (`apeireth.io/changelog`, per CHANGELOG.md) | 200 | R17-R20 4 阶段变更日志 | T-3201 + CHANGELOG.md |
| **T-3205** | SEO 优化 (meta tags / sitemap / Open Graph / Twitter Card) | 200 | Google PageSpeed Insights 90+, 社交分享卡片正确 | T-3201~T-3204 |

**owner**: frontend_engineer (主) + technical_writer (文案)
**守门**: Lighthouse 90+ (性能 / 可访问性 / SEO / 最佳实践) + 4 屏滚动 + 5 特征 + 3 pricing 档 + changelog + R-Measure V1141 ≥ 0.8682

**landing page 4 屏**:

| 屏 | 内容 |
|---:|------|
| 1 | Hero: 价值主张 + demo gif + CTA (5 分钟快速开始) |
| 2 | 5 大特征 (24 维器官 / 4 协议 LLM / Team 协作 / R-Measure 守门 / Sovereignty V3) |
| 3 | 案例展示 (3 use case: 客服 / 代码 / 研究) |
| 4 | CTA: 立即开始 + pricing 3 档 + footer |

**关键技术点**:
- **静态站**: 跟 Docusaurus 复用 (T-3005), 不另起框架
- **图片优化**: WebP 格式 + lazy load + responsive sizes
- **CDN**: Cloudflare / Vercel 边缘缓存
- **analytics**: Plausible (隐私友好) / Google Analytics (可选)
- **不假装**: pricing 3 档 R20 阶段只占位, 实际收费 R21+ 写 ADR 后再开

**R-Measure 守门**:
```bash
# landing page 部署后跑 (验证没引入后端改动)
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-5-3-measure-2026-08-XX.md
```

### 4.4 阶段 5.4: 社区基础设施 (1-2 天)

> **依据**: r20 §4 阶段 5 + user memory #9 (周报 cron 频率).

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-3301** | GitHub Discussions 模板 (5 类: Q&A / Show and tell / Ideas / Help / Announcements) | 200 | 5 模板在 GitHub 上可见 + 可发帖 | T-3305 |
| **T-3302** | Discord 服务器 (5 频道: #general / #dev / #showcase / #help / #announcements) | 100 (配置) | Discord 服务器创建 + 5 频道就绪 + bot 接入 | T-3305 |
| **T-3303** | Twitter 账号 + 自动化发布 (per CHANGELOG, release 推文) | 200 (脚本) | @apeireth 账号 + 自动化脚本 (release 时发推) | T-3301 + T-3302 |
| **T-3304** | 周报 cron (per user memory #9: 周报频率由主人拍板, 默认周一 10:00) | 150 | cron job 设置 + 周报模板 + 自动发到 Discord #announcements | T-3302 + user memory |
| **T-3305** | issue / PR 模板 (`.github/ISSUE_TEMPLATE/` + `PULL_REQUEST_TEMPLATE.md`) | 300 | 5 issue 模板 (bug / feature / doc / question / security) + 1 PR 模板 | T-3301 |

**owner**: community_manager (新角色) + technical_writer
**守门**: 5 频道 Discord 跑通 + 5 issue 模板就绪 + Twitter 自动化发布测试 + 周报 cron 跑通 + R-Measure V1141 ≥ 0.8682

**5 频道 Discord 配置**:

| 频道 | 用途 | 权限 |
|------|------|------|
| `#general` | 社区主聊天 (含非开发) | 所有人 |
| `#dev` | 开发者讨论 (技术 / 集成) | 所有人 |
| `#showcase` | 用户展示自己的项目 | 所有人 |
| `#help` | 用户提问 + 团队回答 | 所有人 (团队 badge 优先) |
| `#announcements` | release / 周报 / 重要通知 | 仅 team + bot |

**5 issue 模板**:

| # | 模板 | 用途 |
|---:|------|------|
| 1 | `bug_report.md` | Bug 报告 (复现步骤 / 期望 / 实际 / 截图) |
| 2 | `feature_request.md` | 功能请求 (问题 / 解决方案 / 替代方案) |
| 3 | `documentation.md` | 文档问题 (位置 / 现状 / 期望) |
| 4 | `question.md` | 一般问题 |
| 5 | `security.md` | 安全问题 (私密, 走 security@apeireth.io) |

**关键技术点**:
- **GitHub Discussions**: 启用后建 5 类目, 配 issue 模板
- **Discord bot**: discord.js + webhook 接入, release 时自动发公告
- **Twitter API**: Twitter API v2 + bot, release 时发推 (含 changelog 链接)
- **周报 cron**: GitHub Actions scheduled workflow (周一 10:00 UTC), 自动生成周报 + 发 Discord
- **issue 模板**: GitHub `.github/ISSUE_TEMPLATE/` 5 文件 + `PULL_REQUEST_TEMPLATE.md`

**R-Measure 守门**:
```bash
# 社区基础设施不直接影响 ASI baseline, 但守门要跑
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-5-4-measure-2026-08-XX.md
```

**阶段 5 完工报告**: `reports/r20-stage-5-complete-2026-08-XX.md` (R20 收官报告).

---

## §5 R-Measure 守门点 (13 子阶段)

> **依据**: `r-measure-verification-design-2026-08-05.md` §2.3 (3 baseline 编译期 hardcode) + §3.4 (CLI `check` 子命令) + APEIRETH-CONVENTIONS §11.

每子阶段结束必跑 `cargo run -p apeireth-r-measure-verify --release -- check --baseline r11`, 报告路径 `reports/r20-stage-<N>-<M>-measure-<date>.md`.

| 子阶段 | 必跑值 | 容忍度 | 报告路径 | 触发场景 |
|--------|------|------:|---------|---------|
| **3.1** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-3-1-measure-<date>.md` | 加 10 REST 端点 + JSON schema + 错误码 |
| **3.2** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-3-2-measure-<date>.md` | 加 WebSocket 双向流 (V1131 dashboard 5 Self 风险高) |
| **3.3** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-3-3-measure-<date>.md` | OpenAPI 规范 (swagger-cli validate + 契约测试) |
| **3.4** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-3-4-measure-<date>.md` | 鉴权 + 限流 (E 急救路径, O-5 17:58 重点) |
| **3.5** | **V1141 ≥ 0.8682 + V1131 ≥ 0.8532 + V1136 ≥ 0.9063** | ±0.001 | `reports/r20-stage-3-5-measure-<date>.md` | 端到端 3 值全守 (阶段 3 完工) |
| **4.1** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-4-1-measure-<date>.md` | Rust SDK 高层入口 + 4 抽象保留 |
| **4.2** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-4-2-measure-<date>.md` | Python SDK 5 端到端 |
| **4.3** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-4-3-measure-<date>.md` | TypeScript SDK 5 端到端 |
| **4.4** | **V1141 ≥ 0.8682 + V1131 ≥ 0.8532 + V1136 ≥ 0.9063** | ±0.001 | `reports/r20-stage-4-4-measure-<date>.md` | 3 SDK 跨语言一致 + 端到端 (阶段 4 完工) |
| **5.1** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-5-1-measure-<date>.md` | 用户文档站部署 (10 教程 + 20 FAQ) |
| **5.2** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-5-2-measure-<date>.md` | 开发者文档站 (13 架构图 + 3 SDK 文档) |
| **5.3** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-5-3-measure-<date>.md` | landing page (Lighthouse 90+) |
| **5.4** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-5-4-measure-<date>.md` | 社区基础设施 (Discord + Twitter + 周报 cron) |
| **R20 收官** | **V1141 ≥ 0.8682 + V1131 ≥ 0.8532 + V1136 ≥ 0.9063** | ±0.001 | `reports/r20-complete-2026-08-XX.md` | R20 收官报告 (5 阶段全完工) |

**24 维 LOCKED** 实装 (per `round10-12-asi-24-dim-9-sub-real-measurement-qa-engineer.md`):
- `apeireth-asi::V05_DIMENSION_NAMES` = 24 维 (5+5+5+5+4 分组)
- `apeireth-asi::V1136_SUBMEASURE_COUNT` = 9 (LOCKED, 编译期 const)
- `apeireth-asi::V1136_SUBMEASURE_NAMES` = 9 子测度名 (LOCKED 字符串数组)
- 17 维 R11 baseline 投影在 verifier 端 (per R-Measure 守门设计 §2.1, 主人从 v1077 抽权重)

**失败处理** (per 主 17:58 不假装):
```
CI fail →
  ↓
PR 阻塞, 不允许 merge
  ↓
开发者本地: cargo run -p apeireth-r-measure-verify -- diff --before <last-green> --after HEAD
  ↓
看哪个 metric fail (v1141 / v1131 / v1136) + diff 多大
  ↓
判断:
  ① 真掉了 → 回滚或修代码, 重新跑 CI
  ② baseline 真的该升 → 写 ADR-00XX-baseline-bump, 主人拍板后才能改 fixtures/r11-baseline.json
  ❌ 绝不绕过
```

---

## §6 风险清单 (10 项, 阶段 3-5 专属)

| # | 风险 | 严重度 | 触发场景 | 缓解 |
|---|------|-------|---------|------|
| **R-018** | OpenAPI 跟代码不同步 | 🔴 高 | 改 `crates/apeireth-api/src/*.rs` 但没改 `docs/api/openapi.yaml` | CI 阻塞: `swagger-cli validate` + docs-sync-check (per Tauri SOP §4.2) + 契约测试 10 端点 byte-equal |
| **R-019** | WS 双向流 鉴权复杂度 | 🔴 高 | WS 升级后鉴权头丢失 / 中间人攻击 / 客户端断线重连鉴权失效 | (1) WS upgrade 前鉴权 (2) 服务端鉴权状态绑连接 (3) 重连必须带新 token (4) T-1301 鉴权 7 case 全覆盖 |
| **R-020** | 限流策略跟 E 急救路径冲突 | 🔴 高 | P0 endpoint (`/v1/sovereignty/check` / `/v1/agent/spawn`) 被限流 → 守门失效 | (1) P0 endpoint 限流 = 1000 req/s 不限 (per O-5 17:58) (2) 限流配置编译期 hardcode (3) 紧急情况走 admin token |
| **R-021** | Python SDK ctypes 跨平台兼容性 | 🟡 中 | macOS arm64 / Linux musl / Windows 编译冲突 (R18-2 已知) | (1) Python SDK 复用现有 `src-py/` (已生成) (2) CI matrix 3 平台 build 验证 (3) apeireth-pybridge 跨平台 issue 列表跟踪 |
| **R-022** | TypeScript SDK napi-rs Windows 编译 | 🟡 中 | napi-rs Windows MSVC toolchain 缺失 / prebuild 失败 | (1) CI 3 平台 (ubuntu/macos/windows) build 验证 (2) prebuild 自动下载机制 (3) 失败时回退到 Rust 源码编译 |
| **R-023** | 3 SDK 同输入 → 同输出测试覆盖 | 🔴 高 | 改 Rust SDK 方法签名, Python/TS 没跟 → 跨语言不一致 | (1) OpenAPI 规范 single source (2) ts-rs 自动生成 Rust 类型 (3) openapi-typescript 自动生成 TS 类型 (4) T-2301 跨语言 5 场景 byte-equal |
| **R-024** | Docusaurus 部署 vs mkdocs 选择 | 🟢 低 | Docusaurus React 生态重 / mkdocs Python 生态轻 | **拍板 A (推荐)**: Docusaurus 3.x (React 生态, MDX 支持, 主题丰富, 跟 Tauri 团队 React 一致) — 主人 Mavis 拍板时确认 |
| **R-025** | landing page 性能优化 (Lighthouse 90+) | 🟡 中 | 图大 / JS 重 / 字体慢 → Lighthouse < 90 | (1) 图片 WebP + lazy load (2) 字体 preload (3) CDN 边缘缓存 (4) Lighthouse CI 阻塞 |
| **R-026** | Discord 服务器冷启动 (早期 0 用户) | 🟡 中 | 早期 Discord 没人, 显得冷清 | (1) 团队先在 Discord 内部讨论 (2) 每周 release 公告 (3) 跟 GitHub Discussions 互链 (4) 主人 Mavis 拍板时确认冷启动策略 |
| **R-027** | 周报 cron 频率 (per user memory) | 🟢 低 | 主人没拍板周报频率, 默认周一 10:00 可能不合适 | (1) 默认周一 10:00 (2) 主人 Mavis 拍板时改 (3) 周报模板先就绪 (4) 前 4 周手动发, 跑稳后改自动 |

**继承风险** (从阶段 1-2 转过来, 阶段 3-5 仍需关注):
- R-005 (OpenAPI 规范跟代码不同步) — 已在 R-018 强化
- R-013 (签名) — 阶段 3.4 API key 鉴权, 不直接关联
- R-014 (一键安装跨平台) — 阶段 3.4 鉴权 keyring 跨平台 (跟 R-014 同)

---

## §7 不修改承诺 (跟 ADR-0011 §不修改承诺 一致)

跟 `r20-product-finalize-2026-08-05.md` §7 + ADR-0011 一致:

1. 阶段 1+2+3 LOCKED 文档
2. v2 / v4 / v4.1 LOCKED
3. 阶段 4 主文档 LOCKED (`6ca80776`)
4. 阶段 5 施工文档 LOCKED (631 行)
5. v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径)
6. R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
7. APEIRETH-CONVENTIONS / VERSIONING / GLOSSARY (顶层 3 文件)
8. START-CONSTRUCTION.md
9. workspace version 1.0.0 (Cargo.toml, semver 严格)
10. apeireth-legacy/ (物理归档, 仅增不删)
11. 现有 ADR 0001~0009
12. **24 维 V0.5 LOCKED** (`V05_DIMENSION_NAMES` per `round10-12-asi-24-dim-9-sub-real-measurement-qa-engineer.md`)
13. **V1136 9 子测度 LOCKED** (`V1136_SUBMEASURE_NAMES`, 编译期 const)

> 8 项详见 docs/stage4/8-locked-unified-2026-08-05.md §2 (本指南统一版)

**新增** (阶段 3-5 阶段成果, 也 LOCKED):
- 14. `docs/api/openapi.yaml` (R20 阶段 3 完工, swagger-cli 校验基线)
- 15. 3 SDK 公开 API (R20 阶段 4 完工, 11 方法签名基线)
- 16. 4 docs 站 (R20 阶段 5 完工, 域名 / 内容基线)

---

## §8 6 哲学 anchor 穿透 (R20 阶段 3-5)

按 APEIRETH-CONVENTIONS §9 + 主 6 哲学锚:

| 锚 | 维度 | 阶段 3-5 落地 |
|---|------|--------------|
| **S-1** (22:33) | 6 anchor ASI 完整性 | R20 阶段 3-5 是 **ASI 完整性的对外暴露** — 10 REST + 1 WS 把 24 维 + V1136 9 子测度 + 4 协议 LLM + Team 协作 完整暴露给外部 |
| **S-2** (17:43) | 6 anchor 实验室 | OpenAPI 3.1 规范是契约测试实验室基线 — 任何代码改动必先改 OpenAPI, 契约测试 byte-equal 守门 (阶段 3.3 T-1204) |
| **O-5** (17:58) | 6 anchor 12 急救 | 鉴权 + 限流是 P0 急救路径 — E 急救 endpoint (`/v1/sovereignty/check` / `/v1/agent/spawn`) 限流 = 1000 req/s 不限, 鉴权 7 case 全覆盖 (阶段 3.4 T-1301/T-1302) |
| **O-2** (19:33) | 6 anchor 4 分类 | API / SDK / 文档 / 社区 4 分类 — 阶段 3 (API) / 阶段 4 (SDK) / 阶段 5.1+5.2 (文档) / 阶段 5.4 (社区), 13 子阶段按 4 分类清晰划分 |
| **O-3** (23:44) | 6 anchor 决策清单 | 13 子阶段 = 13 决策点 — 32 个 T-xxx 任务, 每任务 1 行决策 (估 LOC + 验收 + 依赖), 不假装不漏 |
| **O-4** (00:56) | 6 anchor 12 统一 | 跟 12 子规范统一 — 阶段 3-5 是 12 子规范 (4 协议 LLM / 4 视图 dashboard / 4 工具治理) 的对外接口, 公开 API/SDK/文档全跟 12 子规范对齐 |

**关键不假装** (per 主 17:58 12 急救):
- ❌ 不假装"OpenAPI 跟代码自动同步" — 改 src 必改 openapi.yaml, CI 阻塞
- ❌ 不假装"限流不影响 E 急救" — P0 endpoint 限流 = 1000 req/s 不限, 编译期 hardcode
- ❌ 不假装"3 SDK 自动跨语言一致" — OpenAPI 单一来源 + 跨语言测试 byte-equal 守门
- ❌ 不假装"周报 cron 跑通就完事" — 前 4 周手动发验证内容质量, 跑稳后改自动

---

## §9 关联文档

**R20 主线**:
- `docs/roadmap/r20-product-finalize-2026-08-05.md` (R20 总路线图, 5 阶段) — M 标记, 不读
- `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` (R20 阶段 1-2 已写, 任务粒度模板)

**R20 协同**:
- `docs/stage4/tauri-team-collab-sop-2026-08-05.md` (Tauri 团队双边界 SOP, 5 步可执行, 阶段 3.3 OpenAPI 规范 = Tauri 团队消费契约)
- `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` (SDK 现状, ~14000 LOC 低层 FFI, 缺用户面向 SDK 入口, 阶段 4.1-4.3 全部依据)

**R-Measure 守门**:
- `docs/stage4/r-measure-verification-design-2026-08-05.md` (R-Measure 守门, 3 baseline 编译期 hardcode, 24→17 投影, 阶段 3-5 全程守门)
- `reports/round10-12-asi-24-dim-9-sub-real-measurement-qa-engineer.md` (24 维 V0.5 LOCKED + V1136 9 子测度真实测量, **注**: 用户描述里叫 `apeireth-asi-24dim-api-2026-08-05.md`, 实际文件名以 round10-12 为准)

**TUI 续**:
- `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` (TUI Step 2/3 续, 阶段 3-5 不直接涉及, 但 24 维数据从 TUI 拉)

**10 份 reports/** (背景):
- `reports/council/` `reports/crate-api/` `reports/graph-pipeline/` `reports/mcp-14-tool/` `reports/platform-modules/`
- `reports/protocol-4-adapter/` `reports/session-vector-asi/` `reports/supervisor-tool-rules/` `reports/spectrai-architecture/` `reports/tauri-roadmap/`

---

_阶段 3-5 实施指南 (technical_writer + architect)._
_13 子阶段 × T-xxx 任务清单, 总时长 23 天 (4.6 周), 紧接阶段 1-2 (4 周), 累计 R20 = 8.6 周._
_3 SDK 共享 apeireth-sdk crate 的 SdkVersion/Envelope/SdkErrorCode/C-ABI 4 抽象, 11 方法签名跨语言一致._
_R-Measure 守门 13 子阶段 + 24 维 LOCKED + 3 baseline 编译期 hardcode, 任何改动必守门._
_10 风险 (R-018~R-027) 阶段 3-5 专属, 7 项严重度高, 主人 Mavis 拍板时确认缓解策略._
_不修改承诺 13 项, 阶段 3-5 阶段成果 (OpenAPI / 3 SDK / 4 docs 站) 也 LOCKED._
