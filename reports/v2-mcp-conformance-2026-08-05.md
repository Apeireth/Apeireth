# V2 MCP Conformance & Multi-Transport Validation Report

**Task ID**: `v2-mcp-step2-conformance` (V2 战区 5 P0 — Week 1 后立即开始的事 #1)
**Date**: 2026-08-05
**Author**: backend_engineer2 (MCP-integration)
**Branch**: `rebase/d7d8-into-integration`
**Reference**: `docs/v2-strategy/05-EXECUTION-NOW.md` §Step 2 + `docs/v2-strategy/03-CHARTER.md` §6.5 MCP §

---

## TL;DR — 干完了

3 个 transport (Memory / SSE / HTTP-streamable) 端到端跑通; **47 tests pass** (38 unit + 9 integration), 0 fail; rusqlite 0.32 workspace lock commit `8b5874c8`; `apeireth-mcp` 注册到 workspace (`crates/apeireth-mcp` member)。

**唯一不假装**: stdio transport 集成测试不跑 (spawn 不可控, 已有 `examples/hello.rs` 演示)。

---

## 1. 范围 & 验收

| Acceptance criterion | Status | Evidence |
|---|---|---|
| **fix code_reviewer-audit §3.2 rusqlite conflict** | ✅ done | commit `8b5874c8` — workspace 锁 0.32, 注释禁各 crate 自钉 minor/patch |
| **register `apeireth-mcp` in workspace** | ✅ done | commit `8b5874c8` — Cargo.toml members + `"crates/apeireth-mcp"` |
| **real SSE transport** | ✅ done | `crates/apeireth-mcp/src/transport/sse.rs` — 真 SSE 帧解析 (event/data/id/retry/comment) |
| **real HTTP-streamable transport** | ✅ done | `crates/apeireth-mcp/src/transport/http_streamable.rs` — JSON sync + SSE stream 响应 |
| **Multi-transport facade** | ✅ done | `TransportKind` enum + `connect()` factory (Stdio/StdioCurrent/Sse/HttpStreamable/Memory) |
| **≥3 conformance tests pass** | ✅ done | 9 integration tests in `crates/apeireth-mcp/tests/multi_transport.rs` |

---

## 2. 实装细节

### 2.1 rusqlite 0.32 workspace 锁 (commit `8b5874c8`)

**问题**: code_reviewer-audit-2026-08-05.md §3.2 标 workspace 编译冲突:
- `apeireth-memory` / `apeireth-vector` 依赖 rusqlite 0.32 → libsqlite3-sys 0.30.x
- `apeireth-api` 钉 rusqlite 0.31 → libsqlite3-sys 0.28.0

**修复**: workspace-level 锁定到 0.32, 所有 SQLite 用 crate 必须 `rusqlite = { workspace = true }`。

```toml
# Cargo.toml [workspace.dependencies]
# rusqlite 锁到 0.32 (workspace 级硬锁, 防 V2 各 crate libsqlite3-sys links 冲突)。
# 所有用 SQLite 的 crate (memory / vector / api / mcp) 必须写 `rusqlite = { workspace = true }`,
# 禁止各 crate 钉自己的 minor/patch 版本。fix(R19/V1266 code_reviewer-audit): 统一 workspace 锁定。
rusqlite = { version = "0.32", features = ["bundled"] }
```

```toml
# crates/apeireth-api/Cargo.toml
# V2 Step 2 自包含 6 类 stub (EpisodeStore SQLite / ASI Registry / SelfDisableGuard / AgentManager / LRU cache)
rusqlite = { workspace = true }
```

**验收**: `cargo check -p apeireth-memory -p apeireth-vector -p apeireth-mcp` 不再报 libsqlite3-sys links 冲突。

### 2.2 SSE Transport (`crates/apeireth-mcp/src/transport/sse.rs`)

**字段级参考**: MCP 2025-03-26 §Transport / SSE + WHATWG HTML §9.2 Server-Sent Events。

**wire protocol**:
```text
Client → Server (HTTP POST {endpoint}):
    POST {endpoint} HTTP/1.1
    Content-Type: application/json
    Accept: application/json, text/event-stream
    {jsonrpc request body}

Server → Client (SSE stream from initial GET {url}):
    HTTP/1.1 200 OK
    Content-Type: text/event-stream
    Cache-Control: no-cache

    event: endpoint
    data: /messages?sessionId=xxx

    event: message
    data: {"jsonrpc":"2.0","id":1,"result":{...}}
```

**实现要点**:
- `SseTransport::connect(url)` → GET {url}, 解析 SSE 帧, 读首帧 `event:endpoint` 拿 POST endpoint URL
- `send(line)` → POST 到 endpoint, Content-Type: application/json
- `recv()` → 读下一帧 `event:message` 的 `data:` 字段 (返回 `Ok(None)` 表示 EOF)
- `close()` → 清理 SSE stream + reqwest client

**SSE 帧解析状态机** (`FrameStream`):
- 累积行到 `pending: String`
- 找 `\n\n` 或 `\r\n\r\n` 分隔符
- 按行解析 `event:value` / `data:value` / `id:value` / `retry:value` / `:comment`
- WHATWG spec: `data:` 字段值去掉一个前导空格 (多空格保留)
- 非 `event:message` 帧跳过 (如心跳 comment)

**Tests (9)**:
- `parse_sse_frame_message_basic` — 单 message 帧
- `parse_sse_frame_endpoint` — endpoint 帧
- `parse_sse_frame_comment_and_multiline_data` — 注释 + 多行 data
- `parse_sse_frame_data_leading_space_stripped` — 单空格剥离
- `parse_sse_frame_data_multiple_leading_spaces_kept` — 多空格保留 (WHATWG 仅剥一个)
- `absolutize_endpoint_relative` / `absolutize_endpoint_already_absolute` — endpoint URL 绝对化
- `find_frame_sep_lf_lf` / `find_frame_sep_crlf_crlf` — 帧分隔符

### 2.3 HTTP-Streamable Transport (`crates/apeireth-mcp/src/transport/http_streamable.rs`)

**字段级参考**: MCP 2025-03-26 §Transport / Streamable HTTP (2025-06-18 revision)。

**wire protocol**:
```text
Client → Server (HTTP POST {endpoint}):
    POST {endpoint} HTTP/1.1
    Content-Type: application/json
    Accept: application/json, text/event-stream
    Mcp-Session-Id: {optional session id}
    {jsonrpc request body}

Server → Client:
    HTTP/1.1 200 OK
    Content-Type: application/json
    {jsonrpc response body}

    或:
    HTTP/1.1 200 OK
    Content-Type: text/event-stream
    event: message
    data: {jsonrpc response}
    event: message
    data: {jsonrpc notification}
```

**实现要点**:
- `HttpStreamableTransport::connect(endpoint)` → 设 endpoint URL (skeleton 不强制 GET)
- `send(line)` → POST + 解析响应:
  - `Content-Type: application/json` → 整个 body 入 `response_buffer`
  - `Content-Type: text/event-stream` → 解析 SSE 帧, 所有 `event:message` 的 `data:` 入 buffer
  - 服务端 `Mcp-Session-Id` 头自动持久化
- `recv()` → 从 `response_buffer` 弹一个 (同步 RPC 模式)
- `close()` → 清理

**V2 skeleton 范围**:
- ✅ 单端点 POST + JSON 同步响应
- ✅ SSE 流式响应 (单次响应内多帧)
- ✅ `Mcp-Session-Id` 头透传
- ❌ Server-initiated 长连接推送 (后续 task 引入独立 SSE 通道)
- ❌ Reconnection / resumability (后续 task)

**Tests (6)**:
- `connect_succeeds` — 构造成功
- `send_after_close_errors` / `recv_after_close_returns_none` — closed 状态检查
- `recv_empty_buffer_returns_none` — 同步 RPC 模式行为
- `session_id_roundtrip` — session_id 字段级
- `find_frame_sep_basic` — SSE 帧分隔符

### 2.4 Multi-Transport Facade (`crates/apeireth-mcp/src/transport/mod.rs`)

**`TransportKind` enum**:
```rust
pub enum TransportKind {
    Stdio { cmd: String, args: Vec<String> },
    StdioCurrent,
    Sse { url: String },
    HttpStreamable { url: String },
    Memory,
}

pub async fn connect(kind: TransportKind) -> Result<Box<dyn Transport>, TransportError>;
```

**设计**:
- 上层 `McpClient::connect_*` 根据配置选 `TransportKind`, 工厂 `connect()` 产出 `Box<dyn Transport>`
- 协议层不感知底层是 stdio / SSE / HTTP-streamable / 内存管道
- `Memory` 返回 `NotImplemented` (实际场景应直接用 `tokio::io::duplex` + `MemoryTransport::new`)

---

## 3. Conformance 测试 (`crates/apeireth-mcp/tests/multi_transport.rs`)

| # | Test | Transport | Coverage |
|---|---|---|---|
| 1 | `memory_transport_end_to_end` | Memory | initialize / tools/list / tools/call 全流程 |
| 2 | `sse_transport_end_to_end_via_mock_server` | SSE | 真 TCP server mock; endpoint URL 解析 + 多 message 帧 |
| 3 | `http_streamable_transport_end_to_end_via_mock_server` | HTTP-streamable | 真 TCP server mock; JSON 同步响应 |
| 4 | `transport_kind_factory_http_streamable` | TransportKind | 工厂 dispatch |
| 5 | `transport_kind_variants_construct` | enum | 5 个变体可构造 |
| 6 | `transport_kind_memory_factory_errors` | TransportKind | Memory 返回 NotImplemented |
| 7 | `serverinfo_fields_match_mcp_spec` | protocol | ServerInfo 字段级对齐 MCP §Initialize |
| 8 | `sse_transport_absolutize_endpoint_in_mock` | SSE | endpoint 相对路径绝对化 |
| 9 | `http_streamable_session_id_header` | HTTP-streamable | `Mcp-Session-Id` 头透传 |

**Test count summary**:
- `cargo test -p apeireth-mcp --lib`: **38 passed** (含原 23 + sse 9 + http_streamable 6)
- `cargo test -p apeireth-mcp --test multi_transport`: **9 passed**
- **总计: 47 tests, 0 fail**

---

## 4. 文件清单

**Modified**:
- `Cargo.toml` — workspace.dependencies 注释 + members 加 `apeireth-mcp` + 移除 deprecated crates
- `Cargo.lock` — cargo update -p rusqlite
- `crates/apeireth-api/Cargo.toml` — rusqlite → workspace = true

**Added/Modified in `crates/apeireth-mcp/`**:
- `Cargo.toml` — 加 `reqwest` + `bytes` 依赖
- `src/transport/sse.rs` — 重写: 真 SSE 解析 + 9 测试
- `src/transport/http_streamable.rs` — 新建: HTTP-streamable 解析 + 6 测试
- `src/transport/mod.rs` — 加 `TransportKind` enum + `connect()` factory
- `tests/multi_transport.rs` — 新建: 9 集成测试

---

## 5. 跟其他 agent 协作

### 5.1 给 architect / R-Cycle v2-strategy

**未决问题 (供 architect 决策)**:
1. **graph orchestration 绑定**: apeireth-graph 当前是 skeleton, 跟 apeireth-mcp 的 graph tool (V2 战区 3 Step 3) 绑定接口未定义。需要 architect 出 GraphOrchestration trait + 字段级 JSON schema。
2. **SDK 统一测试入口**: apeireth-sdk 当前是 skeleton, 多语言 (Python/Go/TS) SDK 跟 apeireth-mcp client 的绑定约定 (JSON-RPC 版本协商 + transport 协商) 未定。

**建议** (不假装决策权):
- graph 绑定走"apeireth-mcp tools/list + tools/call, 后端实现 dispatch 到 graph orchestrator"模式 (跟现有 Tool interface 一致, 无新概念)。
- SDK 绑定走"SDK 端实现 JSON-RPC 2.0 + 至少 stdio transport", apeireth-mcp 不需 SDK 端依赖 (避免循环)。

### 5.2 给 backend_engineer / api

**不破坏**: `crates/apeireth-api/Cargo.toml` 的 `rusqlite = { workspace = true }` 改动是为了跟 memory/vector 一致。如果后续 V2 Step 2 还要加其他 SQLite 用 crate (e.g. EpisodeStore), 同样用 `workspace = true`。

---

## 6. 验收 — `cargo` 命令序列

```bash
# 1. rusqlite workspace lock 验收 (无 libsqlite3-sys links 冲突)
cargo check -p apeireth-memory -p apeireth-vector -p apeireth-api -p apeireth-mcp

# 2. 全 unit + integration 测试
cargo test -p apeireth-mcp
# expected: 47 passed, 0 failed

# 3. workspace 编译 (确认 workspace Cargo.toml 合法)
cargo build -p apeireth-mcp
```

---

## 7. 后续 task 候选

按 `docs/v2-strategy/05 §Week 1 后立即开始的事` 排:

| Task | P | 依赖 |
|---|---|---|
| apeireth-mcp server-side 路由 (POST endpoint + SSE stream 分发) | P1 | 本任务 (transport) |
| apeireth-graph ↔ apeireth-mcp tool 绑定 (graph orchestration) | P1 | architect GraphOrchestration trait |
| apeireth-sdk 模板生成 (Python/Go/TS) | P1 | architect SDK 绑定约定 |
| Kani 验证 transport state machine (FrameStream 状态机) | P2 | 本任务 (transport) |
| 真 e2e 测试 vs Anthropic MCP reference server | P2 | 本任务 + server-side 路由 |

---

**Report complete**: 2026-08-05
**Total LOC added**: sse.rs 459 lines + http_streamable.rs 280 lines + mod.rs +62 lines + multi_transport.rs 410 lines + Cargo.toml 6 lines ≈ **1200 lines**
**Total tests added**: 24 (9 sse unit + 6 http_streamable unit + 9 integration)
**Rusqlite fix commit**: `8b5874c8`