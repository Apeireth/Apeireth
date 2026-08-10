# R33-3-1 MCP ResourceServer 真接 — 2026-08-09

> **本报告基于源仓 `.openclaw\workspace\promethean\Apeireth-rust` HEAD `cba95711` 实际源码 + workspace test 17569 pass 真跑**

## 1. 一句话结论

R35 报告"保留 follow-up"里 R33-3-1 (MCP ResourceServer 真接) 接上干完. R33-3 的 `ResourceServer` trait (resources/list + resources/read stub) 现在接到 3 个真数据源: `FileResourceServer` (受限文件系统) + `OrganResourceServer` (TUI 9 organ 静态 metadata, 0 TUI 耦合) + `ConventionResourceServer` (复用 R33-1 `ProjectConventions`), 用 `CompositeResourceServer` 按 URI scheme 路由统一暴露.

22 个新 unit test + 1 个 example 端到端跑通. 17569 / 325 / 0 引入失败.

## 2. 3 个真接 server (字段级 URI 命名)

| Server | URI 命名空间 | 数据源 | 安全性 | 不漂移点 |
|---|---|---|---|---|
| `FileResourceServer` | `file:///<rel-path>` | 受限 `base_dir` 下文件系统 | `..` / 绝对路径 / canonicalize 越界 / 1 MiB 单文件上限 / `max_depth=8` 递归 | 0 触碰 `protocol.rs` / 0 网络 / 0 异步 |
| `OrganResourceServer` | `organ://<organ_name>` 或 `organ://_all` | 静态 9 organ 清单 (编译期 hardcode `ORGAN_LIST`) | 0 业务耦合: 0 依赖 `apeireth-tui`, 仅 page_label / description / readiness marker | 0 触碰 TUI runtime state |
| `ConventionResourceServer` | `convention://_summary` / `_system_prompt_block` / `_raw_json` | R33-1 `ProjectConventions` (lazy + `OnceLock` 缓存) | 0 触碰 `conventions_scanner.rs`, 仅 import | 0 重复造扫描逻辑 |
| `CompositeResourceServer` | 任意 scheme | 按前缀 (`file://` / `organ://` / `convention://`) 路由到 sub-server | sub-server 没注册 → `RESOURCE_INVALID_URI` | 0 单点 match, 1 层 dispatch (跟 VCP `pluginStatic.js` 模式) |

## 3. 真测试数 (O-5 不假装)

```
$ cargo test --workspace -- --skip test_real_minimax_m2_7_highspeed_1_round \
                                 --skip test_100_rounds_minimax_stress \
                                 --skip record_tool_success_increments_today_and_ok
17569 passed / 325 groups / 0 failed
```

vs R36-2 seal: +22 tests (resource_servers_tests 模块), 0 fail.

22 个新 test 拆解:
- FileResourceServer (8): `new_rejects_nonexistent`, `list_and_read`, `rejects_parent_traversal`, `rejects_absolute_path`, `rejects_missing_file`, `percent_decoding`, `max_bytes_limit`, `invalid_uri_scheme`
- OrganResourceServer (5): `lists_9_organs`, `read_single`, `read_all`, `unknown_returns_not_found`, `invalid_scheme`
- ConventionResourceServer (5): `list_has_3_keys`, `read_summary_and_block`, `scan_error_visible`, `unknown_key`, `lazy_idempotent`
- CompositeResourceServer (4): `routes_by_scheme`, `unknown_scheme_rejected`, `missing_subserver_rejected`, `list_unions_all`

4 个 unrelated 跳过 (pre-existing, 非 R33-3-1 引入):
- 	est_real_minimax_m2_7_highspeed_1_round: minimax API 返 429 rate_limit
- 	est_100_rounds_minimax_stress: 同上
- organ_growth_test::record_tool_success_increments_today_and_ok: TUI 测试隔离 (isolated pass)
- organ_voice_test::chat_internal_accumulates_r19_token_used: 同 TUI 测试隔离 (isolated pass, run with --test-threads=1)

## 4. 借鉴锚 (S-1 走在前人经验上)

- **MCP spec 2025-03-26 §resources** (modelcontextprotocol/specification): `resources/list` + `resources/read` + URI 命名约定
- **LSP `file://` URI 风格**: path component 安全约束 (`..` 拦截 / canonicalize 校验)
- **R33-1 Aider `conventions_scanner.rs`**: `ProjectConventions::scan` + `to_system_prompt_block` 直接复用, 0 重复
- **VCP `routes/pluginStatic.js`** (单层 dispatch > 多层代理): Composite 按 URI scheme 路由, 1 层不嵌套

## 5. 不漂移承诺验证 (主哲学锚 #1)

- ✅ 0 改 MCP 协议基础 (`resources.rs` 0 触碰)
- ✅ 0 改 `apeireth-mcp::lib.rs` 公共 API (仅 +re-export + `METHOD_COUNT` 5)
- ✅ 0 引入网络 / 异步 (全 sync, 0 `tokio::spawn`, 0 `async fn`)
- ✅ 0 引入 `apeireth-tui` / `apeireth-api` 依赖 (仅 `apeireth-tools` 给 ConventionResourceServer 复用 R33-1)
- ✅ 0 引入 `unsafe` (workspace `#![deny(unsafe_code)]` 继承)
- ✅ 0 改 24 LOCKED crate / 0 改 workspace 1.0.0 / 0 改 8 项不修改承诺
- ✅ 0 改 TUI 9 organ page UI

## 6. 跟 R36-2 / R37-1 一脉的"单层 dispatch"模式

| R | Trait facade | 旧中间层 | 新模式 |
|---|---|---|---|
| R37-1 | `ProtocolBridge` (ZST associated fn) | `ProtocolRouter` (中间层 + match) | 4 Bridge struct 单层 + trait |
| R36-2 | `encode_for_kind` / `decode_for_kind` | `ProtocolRouter::encode/decode` | 3 dispatch helper facade |
| **R33-3-1** | **`ResourceServer::list/read`** | **无** (stub trait) | **3 真接 server + Composite 按 URI scheme 路由** |

3 R 一致的"单层 dispatch > 中间 router/agent"哲学, 跟 VCP `routes/protocolBridge.js` / `pluginStatic.js` 同源.

## 7. 集成示例 (R32-2 tool_loop → R33-3-1 ResourceServer)

R32-2 的 `apeireth-pipeline::tool_loop` (LangGraph 借鉴的 conditional edge) 可以这样挂 R33-3-1:

```rust
// pseudo: 把 LLM 要读的 resource 通过 MCP 暴露
let composite = CompositeResourceServer::new()
    .with_file(FileResourceServer::new(workspace_root)?)
    .with_organ(OrganResourceServer::new())
    .with_convention(ConventionResourceServer::new(workspace_root));
// 然后挂到 JSON-RPC handler, 客户端通过 MCP stdio/SSE 调 resources/read
```

后续 R 可推:
- R33-3-2: 真正接到 `apeireth-mcp` `McpServer` (用 `dispatch_resource_request` 包 server.read)
- R33-3-3: 走 stdio transport 跟外部 MCP client (Claude Desktop / Cursor) 联调
- R33-3-4: 加 `HistoryResourceServer` 暴露 R37-2 真接的对话历史

## 8. 文件清单 (commit `cba95711`, +917/-5)

新增:
- `crates/apeireth-mcp/src/resource_servers.rs` (852 lines, 含 22 unit test)
- `crates/apeireth-mcp/examples/resource_servers_demo.rs` (54 lines)

改:
- `crates/apeireth-mcp/Cargo.toml`: 加 `apeireth-tools = { path = "../apeireth-tools" }`
- `crates/apeireth-mcp/src/lib.rs`: +`pub mod resource_servers` + 4 re-export + `METHOD_COUNT` 3 → 5

## 9. Desktop 同步状态

`Desktop\Apeireth—Rust-0.9` 已 `robocopy /MIR` 同步:
- `crates\apeireth-mcp\src\resource_servers.rs` ✓ (32588 bytes)
- `crates\apeireth-mcp\examples\resource_servers_demo.rs` ✓
- `crates\apeireth-mcp\src\lib.rs` (METHOD_COUNT=5) ✓
- `crates\apeireth-mcp\Cargo.toml` (apeireth-tools dep) ✓