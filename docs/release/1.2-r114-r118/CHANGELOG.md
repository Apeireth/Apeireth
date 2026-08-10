# v1.2-R114-118 — R114-R118 动态运营层

```
[Document-Meta]
Document:       docs/release/1.2-r114-r118/CHANGELOG.md
Version:        R114-R118
R-Cycle:        R114-R118 (Dynamic Operations Layer)
Last-Modified:  2026-08-10 (R119-3c 索引层下沉)
Status:         🟢 已合 master (codex 5c546a84)
```

**主题**: codex 接管 R114-R118 动态运营层, 把 eval / council / cognition / protocol / skills 能力转成 MCP/CLI/runtime 可消费表面。0 改 workspace semver, 0 嵌入 credentials。

| R | 主题 | 落点 |
|---|---|---|
| R114 | Eval MCP bridge (EvalToolServer list + dispatch) | `crates/apeireth-eval/src/mcp_bridge.rs` |
| R115 | Council MCP bridge (Prompt + Resource server 共用 council 状态) | `crates/apeireth-council/src/mcp_bridge.rs` |
| R116 | CLI command families (skills / eval / council 解析 → typed command) | `crates/apeireth-cli/src/commands/` |
| R117 | TUI cognition live (summary signature 状态机 + 500ms poll) | `crates/apeireth-tui/src/cognition_live.rs` |
| R118 | Protocol transport bridges (UTF-8 chunk assembly + bounded FIFO) | `crates/apeireth-protocol/src/bridge_ext.rs:43` |

**关键数字**: 4921 tests passed / 88 suites / 0 failed (R114-R118 增量 +69), workspace.version = 1.1.0 严守, 0 触动 24 LOCKED crate。

**未纳入本批**: `crates/apeireth-integration-e2e/src/workspace_e2e.rs` (working tree 修改, codex 注明 "未纳入本批")。

## 详细资料

- **R114-R118 主报告**: [`reports/r114-r118-batch-final-2026-08-10.md`](../../../reports/r114-r118-batch-final-2026-08-10.md)
