# apeireth-tools

> **职责**: 工具集成 (5 trait 真实现: web_search / file_ops / git_ops / code_exec / tool_result)
> **状态**: R17 战役 2-5 完工 (替 R11 占位 + 借 VCP `FileOperator.js` 68KB 真代码字段级复刻)
> **对应文档**: 阶段 2 §8 模块化 + §3 兼容组件层 + `borrowed-from-projects.md` §6.2.2

---

## 5 Trait 真实现

| Trait | 字段级 VCP 借鉴 | Rust impl | 操作数 |
|-------|----------------|-----------|--------|
| `WebSearch` | VCP `WebReadFile` (plugin-manifest.json:59) HTTP GET 模式 | `HttpWebSearch` (借战役 1-2 `HttpClient` 5 字段 keep-alive) | search |
| `FileOps` | VCP `FileOperator` 19 命令中核心 6 个 (manifest 55/63/79/99/91/103) | `StdFileOps` (tokio::fs 异步) | 6 ops |
| `GitOps` | (VCP 没有, 我们加, git CLI 真跑) | `GitCliOps` (tokio::process + timeout) | 3 ops |
| `CodeExec` | (VCP 没有, 我们加, tokio::time::timeout 包裹) | `ShellCodeExec` (cmd / sh 真跑) | 1 op |
| `ToolResult` | VCP `isToolResultError` 5 字段 + `toolExecutor.js:475-482 _createErrorResult` | enum 统一返回类型 | (返回值) |

## 4 impl + 4 Tool 适配器 + register_all

每个 impl 配套一个 `*Tool` 适配器 (借战役 2-1 `Tool` trait 4 方法), 通过 `register_all(registry)` 一行塞进战役 2-1 `ToolRegistry`:

```rust
use apeireth_tool_registry::ToolRegistry;
use apeireth_tools::register_all;

let registry = ToolRegistry::new();
register_all(&registry).expect("register_all");
// 现在 registry 里有 4 个工具: WebSearch / FileOperator / Git / ShellExec
// 都可通过 `Tool::call(args)` 路由调
```

## VCP FileOperator 字段级引用 (per `borrowed-from-projects.md` + 真读)

| VCP 真字段 (FileOperator.js) | Rust const | 真值 |
|------------------------------|-----------|------|
| `MAX_FILE_SIZE` (line 24) | `MAX_FILE_SIZE` | 20 * 1024 * 1024 (20MB) |
| `MAX_DIRECTORY_ITEMS` (line 25) | `MAX_DIRECTORY_ITEMS` | 1000 |
| `MAX_SEARCH_RESULTS` (line 26) | `MAX_SEARCH_RESULTS` | 100 |
| `ReadFile` (plugin-manifest.json:55) | `FileOps::read` | (操作映射) |
| `WriteFile` (line 63) | `FileOps::write` | (操作映射) |
| `ListDirectory` (line 79) | `FileOps::list` | (操作映射) |
| `DeleteFile` (line 99) | `FileOps::delete` | (操作映射) |
| `MoveFile` (line 91) | `FileOps::move_path` | (操作映射) |
| `CreateDirectory` (line 103) | `FileOps::mkdir` | (操作映射) |
| `WebReadFile` (line 59) | `WebSearch::search` (HTTP GET 模式借鉴) | (模式借鉴) |

**不抄 VCP 业务代码**: 借鉴字段名 + 行为 (path 白名单 / MAX_FILE_SIZE 截断 / HTTP GET + JSON 解析),
不抄 fs / process / HTTP 实现.

## 端到端 demo

```bash
cargo run -p apeireth-tools --example tools_demo
```

**6 步全跑通**:
1. `ToolResult` enum 演示 (Ok / Err / serde round-trip)
2. `WebSearch` 真接 (起本地 HTTP echo server + 真发 GET + 验 query 透传)
3. `FileOps` 6 操作 (write / read / mkdir / list / move / delete) 全在 tempdir
4. `GitOps` 3 操作 (status / log / diff) 真建 git 仓库
5. `CodeExec` 4 case (echo / exit 7 / timeout 50ms / stderr 捕获)
6. `register_all` 一行注册 4 工具 + `Tool::call` 路由 端到端

## 跨 crate 集成

- **`apeireth-http-client`** (战役 1-2) — `HttpWebSearch` 内部用 (5 字段 keep-alive + LIFO 池)
- **`apeireth-tool-registry`** (战役 2-1) — 4 个 `*Tool` 适配器 + `register_all` 一行注册
- **`apeireth-tool-runtime`** (战役 2-2) — `ToolResult` 设计模式借鉴 (本地 enum 跟 `ExecutionResult` 平行)
- **`apeireth-core`** (R11) — 保留 (兼容)

## Cargo.toml

```toml
[dependencies]
apeireth-http-client = { path = "../apeireth-http-client" }
apeireth-tool-registry = { path = "../apeireth-tool-registry" }
apeireth-core = { path = "../apeireth-core" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
tracing = "0.1"
parking_lot = "0.12"

[dev-dependencies]
tempfile = "3"
```

---

_主哲学 anchor: 主 19:33 走在前人经验上 (VCP FileOperator 字段级复刻) + 主 17:43 实事求是 (5 trait 真实现, 不只 mock) + 主 17:58 不漂移 (LOCKED 全保) + 主 23:44 干到底 (5 trait + 4 impl + 4 适配器 + 57 tests + 端到端 example)._
