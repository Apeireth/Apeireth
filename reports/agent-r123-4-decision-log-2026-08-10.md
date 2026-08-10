# R123-4 Decision Log — v2.1 P2-13 多模态生成 via MCP dispatcher template

> **作者**: R123-4 (Mavis coder 团队)
> **时间**: 2026-08-10 15:46 → 16:45
> **任务**: 1 个 v2.1 P2 缺口 dispatcher template (0 真接, O-5 标缺)
> **借鉴 ID**: `R123-4-VCP-MultimodalMCP-2026-08-10`

---

## §1. 决策时间线 (6 个关键决策点)

### 决策 #1 (15:46) — 0 触碰现有 19 文件策略

**背景**: 主人任务明示 8 墙, 必须 0 触碰现有 src.

**选项**:
- A. 在 tools.rs 末尾加 1 新 fn `multimodal_dispatch()` (改现有文件)
- B. 拆 `tools.rs` 为 `tools/` dir (跟 R123-3 同步)
- C. **新建 `src/multimodal.rs` + lib.rs 加 1 行 mod 声明** (本决策)

**选择**: C. 理由:
1. 0 触碰 `tools.rs` / `tool_bridge.rs` / `protocol.rs` / `transport/*` 等所有现有 19 文件
2. R123-3 已经拆 `tools.rs` → `tools/` dir (我 0 跟他抢)
3. lib.rs 1 行 `pub mod multimodal;` 极轻量
4. 0 改 lib.rs 现有 700 行 (mod 列表 + re-exports + McpClient + McpServer + tests)

**风险**: 0 (跟 R123-3 拆 `tools/` dir 0 同一行冲突)

---

### 决策 #2 (15:50) — multimodal.rs 走 handler_from_fn 显式注册, 0 改 Tool trait

**背景**: 主人任务说 "0 改 11 agent 公共 API 签名". Tool trait 公共 API 0 改.

**选项**:
- A. `MultimodalToolServer` impl `apeireth_tool_registry::Tool` trait (4 方法: name/kind/axes/call)
- B. **不依赖 Tool trait, 走 `tool_bridge::handler_from_fn` 显式注册** (本决策)

**选择**: B. 理由:
1. 0 触碰 Tool trait (4 方法签名 0 改)
2. multimodal.rs 0 依赖 apeireth-tool-registry, 0 触碰 ToolAxes/ToolKind
3. example 走 `McpServer::register_tool(multimodal_tool_def(), handler_from_fn(dispatch_multimodal_handler_async))` 显式注册, 跟 hello.rs 1:1 模式
4. 0 改 11 agent 公共 API (Tool/Handler/Server/Client/Transport/ToolDef/Method 等都 0 改)

**风险**: 0 (handler_from_fn 已在 tool_bridge.rs 公开 API, 0 改 1 行)

---

### 决策 #3 (15:55) — async fn 包装, R124+ 真接时只改 1 处

**背景**: 第一次跑 example 报错 `Result<Value, String>` is not a future. `handler_from_fn` 期望 `Fn(Value) -> Future<Result<Value, String>>`.

**选项**:
- A. 让 multimodal.rs 直接给 `pub async fn dispatch_multimodal_handler(args: Value) -> Result<Value, String>`
- B. 改 `gen_dispatch` 为 async fn (R124+ 真接时改 fn 签名)
- C. **保留 sync `dispatch_multimodal_handler`, 加 1 个 async wrapper `dispatch_multimodal_handler_async`** (本决策)

**选择**: C. 理由:
1. 保留 sync 入口给 lib test (`dispatch_multimodal_handler_returns_not_connected_error` 测 sync 路径)
2. async wrapper 内部调 sync 版, 0 业务逻辑重复
3. R124+ 真接时只改 sync 版内部 1 处 (`Err` → `gen_dispatch(req).and_then(|r| serde_json::to_value(r)...)`), async wrapper 0 改
4. 0 改 lib test 期望 (sync 版仍存在)

**风险**: 0 (lib test 仍测 sync 版, example 用 async 版, 2 路径都覆盖)

**修改**:
- multimodal.rs + 1 个 `pub async fn dispatch_multimodal_handler_async(args: Value) -> Result<Value, String> { dispatch_multimodal_handler(args) }`
- example import 改 `dispatch_multimodal_handler` → `dispatch_multimodal_handler_async`

---

### 决策 #4 (16:00) — GenRequest/Response 用 Option<String> × 3 url 字段, 0 假设哪种

**背景**: 9 plugin 输出类型不统一 (image / video / 3d model 3 种都可能), 0 假设具体哪种.

**选项**:
- A. `image_url: String` 强类型 (假设都是图)
- B. `output_url: String` 单 url (0 区分 image/video/3d)
- C. **`image_url: Option<String>` + `video_url: Option<String>` + `model_url: Option<String>` 3 url 字段并存** (本决策)

**选择**: C. 理由:
1. 9 plugin 都能返 (Png/Jpg/Gif/Webp → image_url, Mp4 → video_url, Glb → model_url)
2. `#[serde(skip_serializing_if = "Option::is_none")]` 让 None 字段不进 JSON, 干净
3. R124+ 真接时填对应字段即可, 0 改 Response struct
4. Client 端 0 假设, 看到哪个 url 字段有就取哪个

**风险**: 0 (3 字段都用 Option + skip_serializing_if, 0 装, 0 假设)

---

### 决策 #5 (16:10) — plugin_endpoint URL 模板, 0 真连

**背景**: 9 plugin URL 各家不同, 主人说 0 真接.

**选项**:
- A. 0 URL (仅 GenPlugin enum, 真接时再补)
- B. **9 URL 模板 (per 7 §2 P2-13 + VCP `Plugin/*Gen*/` 借鉴)** (本决策)
- C. 9 URL + reqwest 探活 (真接)

**选择**: B. 理由:
1. 9 URL 模板借鉴 VCP `Plugin/*Gen*/plugin-manifest.json` 已知 URL, 编译期 hardcode
2. 0 HTTP / 0 reqwest 调用 (R123-4 dispatcher 永远 Err)
3. R124+ 真接时 URL 模板可复用, 0 改
4. unit test `gen_plugin_9_variants_distinct_endpoints` 守 9 URL 唯一性 + http(s):// 前缀
5. URL 模板 0 真连 (R124+ 真接时由 sub-`GenPluginBackend` impl 决定 GET/POST/auth)

**风险**: 0 (URL 0 真连, R124+ 真接时由 sub-impl 决定)

---

### 决策 #6 (16:18) — 0 主动 commit, 留工作树让 Mavis / 主人拍板

**背景**: 主人明示 0 主动 commit, 仅改工作树.

**选项**:
- A. commit (R123-4 自己 commit)
- B. **0 commit, 留工作树** (本决策)
- C. push 到远端

**选择**: B. 理由:
1. 主人明示 8 墙第 7 条 "0 主动 commit"
2. R123-3 跟我并行, 都 0 commit, 等 Mavis 收尾时统一 commit
3. 等 Mavis / 主人复核报告 (readmap + final + decision log) 后再 commit
4. 0 装 "已发布", 0 越界

**风险**: 0 (工作树在, 0 commit = 0 副作用)

---

## §2. 关键取舍 (R123-4 0 触碰 vs 0 装)

| 取舍点 | 选了 | 0 选的原因 |
|--------|------|-----------|
| 0 触碰 `tool_bridge.rs` 现有 `handler_from_fn` API | ✅ 走 handler_from_fn 显式注册 | 0 改 ToolHandler / ToolDef / ToolHandlerFuture / Arc<dyn Fn> 等 |
| 0 触碰 `apeireth-tool-registry::Tool` trait | ✅ 0 impl Tool, multimodal 0 依赖 registry | 0 改 Tool trait 4 方法 / ToolAxes / ToolKind / ToolRegistry |
| 0 触碰 `McpServer` / `McpClient` 公共 API | ✅ 0 改 1 行 | multimodal 走 `McpServer::register_tool(def, handler_from_fn(...))` 显式注册 |
| 0 触碰 `Cargo.toml` [package] / [dependencies] | ✅ 仅加 1 个 [[example]] 段 | 0 改 version / edition / apeireth-tool-registry dep / tokio dep |
| 0 触碰 `protocol.rs` JsonRpcError/Request/Response | ✅ 0 用, 0 改 | multimodal 走 mcp tool handler 桥接, 0 改 JSON-RPC 基础类型 |
| 0 触碰 `transport/*` 现有 transport | ✅ 0 改 | multimodal 0 引入新 transport, 走 mcp handler |
| 0 触碰 `resources.rs` / `subscriptions.rs` / `prompts.rs` | ✅ 0 触碰 | 0 改 MCP resources / subscribe / prompts |
| 0 触碰 `telemetry_bridge.rs` | ✅ 0 触碰 | 0 改 handler call metrics (R112 0 触碰) |
| 0 触碰 `initialize.rs` | ✅ 0 触碰 | 0 改 MCP initialize handshake |
| 0 触碰 `resource_servers.rs` | ✅ 0 触碰 | 0 改 3 ResourceServer impl |

---

## §3. 0 装证据 (O-5 诚实标缺)

| 装的风险 | 0 装的证据 |
|---------|----------|
| "已接 9 Gen 插件" | ❌ `gen_dispatch` 永远 `Err("plugin X not connected (R123-4 template placeholder, R124+ 真接)")` |
| "已接 ComfyUI / Flux / Doubao" | ❌ 0 HTTP / 0 reqwest 调用, 0 鉴权, 0 token |
| "GenPlugin 9 全部能产图" | ❌ dispatcher template 0 真接, 9 plugin 全部 `not_connected` |
| "multimodal MCP 端到端跑通" | ✅ 端到端跑通 (`McpServer::register_tool` → `McpClient::call_tool` → JSON-RPC 协议 → handler 返 Err), 但 Err 是预期行为 (O-5) |
| "0 真接任何 plugin" | ✅ 全 dispatcher template, R124+ 真接 |

---

## §4. 0 范围扩散 (跟 R122-10 refactor / R123-3 browser / 其他 agent 0 冲突)

| Agent | 改了什么 | 我跟它冲突? | 防御 |
|-------|---------|-----------|------|
| R123-3 (browser MCP, P2-12) | 加 `tools/browser.rs` (22KB) + `tools/mod.rs` (14KB) + 1 行 `pub mod tools;` (dir 化) + 1 example | ❌ 0 冲突 | 我加 `multimodal.rs` 在 src/ 顶层, 加 1 行 `pub mod multimodal;` 在 `pub mod transport;` 之后 (跟 R123-3 改的 `pub mod tools;` 不同行) |
| R122-10 (refactor scan) | 0 改 src (read-only scan) | ❌ 0 冲突 | 我 0 改 refactor 建议目标 (tui-backend / keyring-platform-3 / constraint-engine 等) |
| R123-1 (clippy / doc) | (推测) | ❌ 0 冲突 | 我 0 改 clippy 规则 / doc 文件 |
| R123-2 (其他 P2) | (推测) | ❌ 0 冲突 | 我仅在 apeireth-mcp 加 1 新 mod, 0 跨 crate |
| Mavis 主 | 0 改任何文件 | ❌ 0 冲突 | 仅我自己改 |

---

## §5. 风险 & 防御 (R123-4 期间发现, 已修)

| 风险 | 防御 |
|------|------|
| 1. example 第一次 build 报 `Result<Value, String> is not a future` | 加 1 个 `pub async fn dispatch_multimodal_handler_async` 包装, 内部调 sync 版, R124+ 真接时只改 1 处 |
| 2. R123-3 同时在改 `tools.rs` → `tools/` dir | 0 同一行冲突, 我加 `pub mod multimodal;` 在 transport 之后, R123-3 改 `pub mod tools;` 在 tool_subscriptions 之后 |
| 3. `tool_bridge.rs:430` 的 `assert_eq!(*a, back)` R123-3 编译错 | R123-3 已修 (我重跑 test 时已 OK), 0 触碰 |
| 4. Windows cmd codepage 跟 UTF-8 冲突 (→ / ✓ 字符乱码) | 0 装, 0 触碰, 实际数据全对 (我用 grep 查了 log, 9 plugin 全部 call, 全部 Err "not connected") |
| 5. Cargo.toml `[[example]]` 段格式错 | 跟现有 `[[example]] name = "hello" path = "examples/hello.rs"` 1:1 格式 (path 相对 crate root, 不加引号) |

---

## §6. 0 装 (O-5) 二次守

- ✅ README / 文档 0 写 "已接 9 Gen 插件"
- ✅ example 输出明确 "R123-4 template, 0 真接, R124+ 真接"
- ✅ unit test 全部用 "not connected" 关键词
- ✅ dispatcher 永远 Err (0 HTTP / 0 reqwest / 0 token / 0 auth)
- ✅ R124+ 真接 TODO (§7 final 报告) 留档, 1 个 1 个接

---

## §7. 借鉴 ID 登记

- **借鉴 ID**: `R123-4-VCP-MultimodalMCP-2026-08-10` (per 07 §3 P2-13, VCP 9 Gen 插件架构)
- **状态**: template placeholder, R124+ 真接 9 plugin 1 个 1 个
- **0 装**: dispatcher 永远 Err, 0 真接
- **下一步**: R124+ 真接 (9 plugin sub-backend impl), 预计 9 × 1 sub-impl × 30 min ≈ 4.5h

---

## §8. 总结

R123-4 dispatcher template:
- ✅ 0 改任何现有 19 文件 (仅 1 行 mod + 1 个 [[example]] 段)
- ✅ 加 1 新 multimodal.rs (475 行) + 1 example (167 行)
- ✅ 12 unit test 全过 (8+ required)
- ✅ 195 lib test 全过 (含 R123-3 browser)
- ✅ example 端到端跑通
- ✅ 0 改 11 agent 公共 API 签名
- ✅ 0 触碰 24 LOCKED
- ✅ 0 改 workspace.version (1.1.0)
- ✅ 0 主动 commit
- ✅ 0 装 (O-5 标缺)
- ✅ 比预算提前 35 min 完成

**R123-4 决策清晰, 0 范围扩散, 0 越界 commit, Mavis / 主人随时 review.**

---

_Decision log 基于 R123-4 实施 16:00-16:30 实测过程, 6 个关键决策点 + 5 个风险防御 + 0 装二次守. 等 Mavis / 主人复核 final 报告后, 统一 commit._
