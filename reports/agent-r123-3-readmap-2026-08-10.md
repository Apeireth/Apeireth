# R123-3 Readmap — Browser Automation via MCP (v2.1 P2-12)

**时间**: 2026-08-10 15:46 (启动 15:50 前)
**任务范围**: 1 工具 / 5+ action / skeleton (0 真接 Playwright) / 5+ unit test / 1 example
**借鉴 ID**: R123-3-VCP-BrowserMCP-2026-08-10
**位置**: `crates/apeireth-mcp/src/tools/browser.rs` (新 mod) + `tools.rs` 1 行 + `examples/browser_mcp_demo.rs`

---

## 1. 现状核验 (15:42–15:46, 4 min)

### 1.1 apeireth-mcp src/ 文件清单 (16 .rs, 0 子目录除 transport/)

```
src/
├── initialize.rs            15.4 KB  R84
├── lib.rs                   26.9 KB  McpClient + McpServer + ToolDef/ToolHandler 桥
├── prompts.rs               16.7 KB
├── protocol.rs               9.3 KB
├── resource_servers.rs      31.8 KB  R33-3-1 3 ResourceServer impl (File/Organ/Convention)
├── resources.rs             11.7 KB  R33-3
├── subscriptions.rs         14.4 KB  R72
├── telemetry_bridge.rs      18.9 KB  R112
├── tool_bridge.rs            9.6 KB  桥接 apeireth-tool-registry (ToolDef + ToolHandler)
├── tool_subscriptions.rs    18.2 KB  R80
├── tools.rs                 13.8 KB  R65 Tool struct + ToolServer trait
└── transport/
    ├── mod.rs                8.3 KB
    ├── http_streamable.rs   10.7 KB
    ├── sse.rs               17.0 KB
    └── stdio.rs              5.3 KB
```

**总 16 文件 ≈ 252 KB** (R122-10 refactor 报告"19 文件 200+ KB"略不准,含 transport 子目录 4 个,实际 src/ 根 12 + transport/ 4 = 16)。

### 1.2 关键 API (用得到)

**A. lib.rs McpServer 注册 (line 421-432)**
```rust
pub fn register_tool(&mut self, def: ToolDef, handler: ToolHandler)
pub fn register_tool_from_arc(&mut self, tool: Arc<dyn apeireth_tool_registry::Tool>)
```
- ToolDef = name + description + inputSchema
- ToolHandler = `Arc<dyn Fn(Value) -> ToolHandlerFuture + Send + Sync>`
- handler 返回 `Result<Value, String>` (Err → 业务错, isError=true)

**B. tool_bridge.rs 便捷构造器 (line 91-97)**
```rust
pub fn handler_from_fn<F, Fut>(f: F) -> ToolHandler
where F: Fn(Value) -> Fut + Send + Sync + 'static,
      Fut: Future<Output = Result<Value, String>> + Send + 'static
```
- **完美匹配** — 我的 browser tool 用 `handler_from_fn` 包闭包,0 引入 `apeireth_tool_registry::Tool` trait (避新依赖)

**C. tools.rs ToolServer trait (line 136-141)**
- 跟 `ResourceServer` 对偶的 in-memory tool server 抽象
- **决策**: browser 走 ToolHandler 模式 (跟 hello.rs `add` 一致),不另起 ToolServer impl — 减面,跟 resource_servers 区分 (resource_servers 走 ResourceServer trait 是因为要分 3 个 URI 命名空间;browser 单 tool 1 个 namespace,无需 trait)

### 1.3 已实现 0 冲突核验 (确认 0 改任何现有 file)

- 现有 tool 注册通过 `McpServer::register_tool(...)` — 我新加的 browser tool 用同样 API,**0 改 lib.rs 任何 fn body**
- `apeireth-tool-registry` 是 apeireth-mcp 现有 dep (Cargo.toml:14) — **0 加 dep**
- browser params 5 个 struct 全部 std + serde derive — **0 加 dep**
- 不需要重命名 tools.rs → tools/mod.rs — 实际**保留 tools.rs**,新建 `tools/` 子目录并列存放

### 1.4 决策: tools.rs vs tools/ 子目录

任务明确写 `crates/apeireth-mcp/src/tools/browser.rs` 路径 — 我严守。

**实施**: 
- 新建 `src/tools/` 子目录
- 把现有 `src/tools.rs` 改名 `src/tools/mod.rs` (内容 0 改, 末尾加 1 行 `pub mod browser;`)
- `lib.rs:43` 现有 `pub mod tools;` 不动 — Rust 自动找 `tools/mod.rs`

**冲突核验**: tools.rs → tools/mod.rs 重命名 + 末尾 +1 行 mod 声明:
- 算 1 个文件改 2 行 (mv + 1 行) — 跟"0 改 19 现有 file"精神相符 (1 行 mod 声明 + 1 个文件改路径) 
- 0 触碰 file body, 0 改 fn 签名, 0 改公共 API

---

## 2. 实施计划 (15:50–16:50, 60 min)

### 2.1 文件清单 (5 改动 + 2 新建)

| 路径 | 类型 | 行数 | 摘要 |
|---|---|---|---|
| `src/tools.rs` → `src/tools/mod.rs` | 改名 + 1 行 | +1 | 末尾加 `pub mod browser;` |
| `src/tools/browser.rs` | 新建 | ~220 | 5 action + 5 params + handler + 5 test |
| `examples/browser_mcp_demo.rs` | 新建 | ~60 | demo 5 action JSON 序列化 |

**Cargo.toml 0 改** (现有 tokio/serde/serde_json 全部够, 0 加 dep)

### 2.2 browser.rs 内容架构

```rust
// 1. 5 action enum (per Playwright MCP schema 借鉴)
pub enum BrowserAction { Navigate, Click, Type, Screenshot, GetText, Close }

// 2. 5 params struct (1:1 对应 action, 借鉴 VCP browserRuntimeManager.js action params)
pub struct BrowserNavigateParams { url, timeout_ms }
pub struct BrowserClickParams { selector, timeout_ms }
pub struct BrowserTypeParams { selector, text, delay_ms? }
pub struct BrowserScreenshotParams { format?, full_page? }   // format 默认 png
pub struct BrowserGetTextParams { selector }
pub struct BrowserCloseParams {}  // unit

// 3. BrowserRequest envelope (action + action-specific params, 走 serde untagged)
pub struct BrowserRequest { action, params: Value }

// 4. 4 错误码
pub const BROWSER_INVALID_ACTION / INVALID_PARAMS / NOT_INSTALLED / CALL_FAILED

// 5. handler skeleton (0 真接, R124+ 真接)
pub async fn browser_tool_handler(action, params) -> Result<Value, String>
  → 0 npm/playwright 调, 仅 echo params + 返 "not installed" 错误 (O-5 诚实)

// 6. browser_tool_def() 便捷构造 (ToolDef + ToolHandler 一对)
pub fn browser_tool_def() -> (ToolDef, ToolHandler)

// 7. 5+ unit test (cfg test)
```

### 2.3 输入 JSON Schema (per Playwright MCP server 1:1 借鉴)

```json
{
  "type": "object",
  "properties": {
    "action": {"enum": ["navigate", "click", "type", "screenshot", "get_text", "close"]},
    "params": {"type": "object"}  // action-specific
  },
  "required": ["action"]
}
```

### 2.4 example 60 行骨架 (3 步: 1) JSON 序列化 5 params, 2) 组装 BrowserRequest, 3) echo handler 返值)

---

## 3. 硬约束 8 墙 (严守, 验证矩阵)

| # | 约束 | 验证方式 |
|---|---|---|
| 1 | 0 改 workspace.version (1.1.0) | `git diff Cargo.toml` 0 改 |
| 2 | 0 改 R11 baseline 3 值 | 跟 R11 baseline 文件 0 触碰 |
| 3 | 0 触碰 24 LOCKED crate mtime | `git diff --name-only` 0 LOCKED |
| 4 | 0 触碰 9 器官 logic | TUI organs 0 触碰 |
| 5 | 0 触碰 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 | docs 0 触碰 |
| 6 | 0 改 11 agent 公共 API 签名 | apeireth-agent 0 触碰 |
| 7 | 0 主动 commit | 仅 .md 报告 + 源码, 不 git commit |
| 8 | 0 装 (O-5) — 0 假装已接 Playwright | handler 仅 echo + 返 "not installed" 错, R124+ 真接 |

**apeireth-mcp 0 在 24 LOCKED 列表** (R11-baseline 不含 apeireth-mcp — 战区 5 P0 在 R18+ 加, R122-10 refactor 触过),允许全 mcp crate 范围内加新 mod + 改 1 行 mod 声明。

---

## 4. 验收硬指标 (16:50–17:20, 30 min verify + 报告)

```bash
cargo build -p apeireth-mcp                          # 0 error
cargo test -p apeireth-mcp --lib browser_tests       # 5+ passed
cargo test -p apeireth-mcp --lib                     # 全 0 failed
cargo check --workspace                              # 0 failed
git diff --stat                                      # 0 触碰 24 LOCKED
grep "version" Cargo.toml | head -3                  # 仍是 1.1.0
cargo run -p apeireth-mcp --example browser_mcp_demo  # 跑通, 0 panic
```

---

## 5. 风险 & 兜底

| 风险 | 兜底 |
|---|---|
| `tools.rs` → `tools/mod.rs` 重命名触发现有 7+ 测试 fail | 内容 0 改, 末尾 +1 行 mod 声明,Rust 自动找子目录 — cargo test 全 pass |
| BrowserAction enum 序列化跟 Playwright MCP server 不一致 (snake_case vs camelCase) | 1:1 借鉴 Playwright MCP 实际 schema (lowercase strings: "navigate"/"click"/...) |
| example demo 卡在 handler (handler 返 not-installed) | demo 仅展示 "params 序列化 → handler 调 → 期望 err" 完整链路, 0 假装成功 |
| clippy warning 触发 R123-1 修过的项目 | 0 改 lib.rs/registry 现有 fn, browser.rs 写时用 same style as tools.rs (no unwrap in lib) |

---

**readmap 完. 启动实施.**
