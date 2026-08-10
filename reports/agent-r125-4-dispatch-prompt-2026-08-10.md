# R125-4 Sub-Agent Dispatch Prompt (MCP servers 协议对齐 + primitive namespace)

**Date**: 2026-08-10 17:28
**Author**: R125 P0 supervisor
**Receiving agent**: R125-4 sub-agent

---

## 任务

**主题**: MCP servers 协议对齐 — 借鉴 modelcontextprotocol/servers 仓库的 primitive namespace 设计, 重构 `apeireth-mcp` 内部 fn 实施 (主人 17:22 升级授权 + B1 24 LOCKED 持续更新 + 内部 fn 可改, 入口签名 0 改)

**借鉴 ID**: `R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\servers\` ✅ **cloned (145 files)**

**目标文件** (per B1 24 LOCKED 持续更新 + 主人 17:22 升级授权):
- `Apeireth-rust/crates/apeireth-mcp/src/lib.rs` (M: re-exports 0 改, 内部 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/protocol.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/initialize.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/multimodal.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/resources.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/subscriptions.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/resource_servers.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/tool_subscriptions.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/telemetry_bridge.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/tool_bridge.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/prompts.rs` (M: 内部 fn 实施可改, 公共 API 0 改)
- `Apeireth-rust/crates/apeireth-mcp/src/tools/` (NEW subdir, 借鉴 servers/src 的 tools/ 拆 module 设计, 替换原 tools.rs)

**B1 24 LOCKED 持续更新 (per 主人 17:22)**: **apeireth-mcp 在 24 LOCKED 名单**, 主人已知 #8 风险. **R125-4 例外**: 内部 fn 实施可改 (per 主人 17:22 升级授权 + B1 持续更新). **crate 入口签名 0 改** (`apeireth_mcp::server::run()`, `apeireth_mcp::protocol::Handler`, 等).

**估时**: 1-2 天

**截止**: 8/12 8:00 (跑过夜明早)

---

## 0 装解除 (主人 17:22) — 这次有真实源码

**借鉴源码**: `.openclaw\workspace\borrowed-repos\servers\` ✅ **cloned (145 files)** = **真实施**, 0 装 PASS.

你 (R125-4) 是 **唯一一个有真实施条件** 的 P0 sub-agent (servers 已 cloned, 0 装解除). 期望你 8/12 8:00 前出实施.

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 + C1-C3)

| # | 必守 (R125-4 特殊) |
|---|------|
| 1 | B2 0 触碰 workspace.version |
| 2 | A1 0 触碰 R11 baseline 3 值 |
| 3 | **B1 apeireth-mcp 在 24 LOCKED 名单**: **内部 fn 实施可改, crate 入口签名 0 改** |
| 4-7 | B3-B6 0 改原实质 |
| 8 | C1 0 commit, C2 0 装解除 ✅ (借鉴已 cloned = 真实施), C3 0 装 5 项升 6 重 v6, 0 push |

---

## 实施步骤 (5 阶段)

### 阶段 1: 借鉴 servers 仓库 (45 min)
- 读 `servers/src/` 全部 + `servers/src/tools/` 拆 module
- 提取 4 pattern:
  1. **Primitive namespace** (resources/tools/prompts/sampling/roots/logging 共 6 primitive)
  2. **Tools 拆 module** (1 文件 → tools/<tool_name>.rs 多文件, per tool 1 file)
  3. **JSON-RPC 2.0 envelope** (jsonrpc="2.0" + id + method + params)
  4. **Capability negotiation** (initialize request/response + capabilities dict)

### 阶段 2: apeireth-mcp 现状盘点 (30 min)
读全部 src/*.rs, 列出:
- 哪些 fn 是 **crate 入口签名** (lib.rs pub use + pub fn) → 0 改
- 哪些 fn 是 **内部 fn** (pub(crate) / pub(super) / private) → 可改
- 哪些是 **跨 crate 公共 API** (被 apeireth-api/apeireth-tui 等调) → 0 改

**目标**: 找 5-10 个内部 fn, 用 servers 借鉴的 pattern 重构

### 阶段 3: 内部 fn 重构 (8-12h, 跨夜)
**3 个重点重构**:
1. `tools.rs` → `tools/` 拆 module (1 大文件 → 6 小文件, per tool)
2. JSON-RPC envelope 统一用 1 个 macro (减少 5+ 处重复)
3. primitive namespace 抽 `Primitive` enum (6 个 variant, 编译期 hardcode)

**约束**:
- 公共 API 0 改 (re-exports 保持)
- 内部 fn 0 改语义 (仅改实现)
- 0 改 mtime 24 LOCKED baseline (虽然 LOCKED, 主人 17:22 授权)

### 阶段 4: 测试 (4h)
- apeireth-mcp 已有 test suite, 加 5 个新 test:
  - `test_tools_module_split_works` — 拆后所有 tool 仍可调
  - `test_jsonrpc_macro_generates_correct_envelope`
  - `test_primitive_enum_exhaustive`
  - `test_capability_negotiation_roundtrip`
  - `test_no_public_api_breaks` (snapshot test, 公共 API 名字集合 0 改)

```bash
cd .openclaw\workspace\promethean\Apeireth-rust
cargo build -p apeireth-mcp
cargo test -p apeireth-mcp
cargo build --workspace  # 验证 0 破其他 crate
```

### 阶段 5: final 报告 (1h)
- `Apeireth-rust/reports/agent-r125-4-final-2026-08-10.md`

---

## 0 主动 commit (C1)

❌ **0 commit, 0 push**.

---

## final 报告 必含 6 段 + 借鉴 ID 严格化 (3 primitive 借鉴摘要 + 0 改入口签名 verify)

---

**派活完成 17:28. 截止 8/12 8:00 (跑过夜明早). 这是 P0 4 任务中唯一有真借鉴源码的, 期望你 8/12 8:00 前出实施.**
