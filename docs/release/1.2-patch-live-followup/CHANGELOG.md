# v1.2-patch-LIVE-续 — R78-R113 1.2 patch LIVE 续

```
[Document-Meta]
Document:       docs/release/1.2-patch-live-followup/CHANGELOG.md
Version:        R78-R113
R-Cycle:        R78-R113 (1.2 patch LIVE 续)
Last-Modified:  2026-08-10 (R119-3c 索引层下沉)
Status:         🟢 LIVE
```

**主题**: 12 R + 1 LIVE 一气呵成 (per 主人 8/9 拍板, 1 commit 也行)。涵盖 skills / graph / MCP / live / TUI 真接。

| R | 主题 | 落点 |
|---|---|---|
| R78 | TUI cognition summary 显示 + render 0 假装小修 | `crates/apeireth-tui/src/organ/memory.rs` |
| R80 | MCP tools/subscribe 双向 push | `crates/apeireth-mcp/src/tool_subscriptions.rs` |
| **R82** | **LIVE MiniMax 7 model 真接** (env-gated) | `reports/r82-live-minimax-8model-results.md` |
| R84 | MCP `initialize` + `prompts` 真接 | `crates/apeireth-mcp/src/{initialize,prompts}.rs` |
| R86 | `apeireth-skills` 真接 `apeireth-mcp` ToolServer | `crates/apeireth-skills/src/mcp_bridge.rs` |
| R89 | `apeireth-graph` 真接 `apeireth-mcp` ResourceServer | `crates/apeireth-graph/src/mcp_resource.rs` |
| R107 | `apeireth-skills` 严格 semver 2.0.0 | `crates/apeireth-skills/src/semver_strict.rs` |
| R109 | `apeireth-skills` 文件 watcher 热加载 | `crates/apeireth-skills/src/watcher.rs` |
| R110 | `apeireth-skills` 真接 `apeireth-eval` | `crates/apeireth-skills/src/eval_bridge.rs` |
| R111 | `apeireth-council` 真接 bus | `crates/apeireth-council/src/bus_bridge.rs` |
| R112 | `apeireth-mcp` handler call metrics | `crates/apeireth-mcp/src/telemetry_bridge.rs` |
| R113 | `apeireth-council` 真接 `apeireth-graph` | `crates/apeireth-council/src/graph_bridge.rs` |

**关键数字**: 4683 → 4852 tests (+169, 0 fail), 0 触动 24 LOCKED crate, 0 触动 8 项承诺, 0 触动 R11 baseline, workspace.version 1.1.0 严守。

## 详细资料

- **R78-R113 主报告**: [`reports/r78-r113-batch-final-2026-08-10.md`](../../../reports/r78-r113-batch-final-2026-08-10.md)
- **R82 LIVE 证据**: [`reports/r82-live-minimax-8model-results.md`](../../../reports/r82-live-minimax-8model-results.md)
