# MCP 集成点识别自检报告

- 任务: 自检-MCP1: MCP集成点识别 (19809d9e-de75-4d6e-9c22-9bd5c66a0293)
- 角色: MCP 集成专家 | 方式: 只读检索 + `cargo test -p apeireth-mcp` 实测
- 结论: ⚠️ 核心 crate 健康可用，但存在文档/规划引用悬空

## 集成点清单与现状

| # | 集成点 | 现状 | 评估 |
|---|--------|------|------|
| 1 | `crates/apeireth-mcp` (workspace 成员, V2 战区5 P0, LOCKED) | client/server + JSON-RPC 2.0; transport: stdio/SSE/HTTP-streamable/memory; initialize/tools/resources/prompts/subscriptions/multimodal/telemetry | ✅ 实测 `cargo test -p apeireth-mcp` 18/18 通过 (conformance 9 + multi_transport 9, exit 0) |
| 2 | `apeireth-cli` (commands.rs) | 依赖 apeireth-mcp; `eval_list_tools` 经 `apeireth_eval::mcp_bridge::EvalToolServer` + `ToolServer` 列出 MCP eval 工具 | ✅ 代码级真实接线 |
| 3 | `apeireth-api` 协议层 | `ProtocolKind::Mcp` 已定义并参与路由 (routing.rs:206)，但 Mcp gateway 与 Acp/OpenClawGateway 同为未注册 → dispatch 返回 Err（设计内的 skeleton 门） | ✅ 符合 skeleton 设计 |
| 4 | `apeireth-tool-registry` 桥接 | `tool_bridge.rs` import + bridge，0 改 registry 源码（不修改承诺） | ✅ |
| 5 | examples/tests | 4 examples (hello/browser/multimodal/resource_servers) + 2 集成测试套件 | ✅ |

## 发现的问题

1. ⚠️ **CODEOWNERS 悬空引用**: 声明 `crates/apeireth-mcp-ssh/`、`apeireth-mcp-winrm/`、`apeireth-mcp-relay-image/` (P0 R20 阶段1)，但 crates/ 下不存在这些目录。
2. ⚠️ **策略文档缺失**: `apeireth-mcp/src/lib.rs` 与 `Cargo.toml` 注释引用 `docs/v2-strategy/05`，该目录下仅有 README.md，05 号文档缺失。
3. ⚠️ **lib.rs 头部文档过时**: 声称 "SSE 真实实现未做 / resources·prompts 未实现"，实际 `transport/sse.rs`、`http_streamable.rs`、`resources.rs`、`prompts.rs` 均已实现且有通过的端到端测试。
4. ℹ️ 无顶层 MCP 运行时配置文件 (如 `.mcp.json`/`mcp.json`)，集成均为代码级；doc-tests 4 处 ignored（有意为之，非失败）。

## 建议

- 清理 CODEOWNERS 中未落地 crate 条目（或补建 crate）；补齐/迁移 `docs/v2-strategy/05`。
- 更新 `apeireth-mcp/src/lib.rs` 头部注释与实现状态对齐（LOCKED crate，需走对应流程）。
