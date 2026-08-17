# v1.2-candidate — R63-R68 1.2 candidate

```
[Document-Meta]
Document:       docs/release/1.2-candidate/CHANGELOG.md
Version:        R63-R68
R-Cycle:        R63-R68 (1.2 candidate)
Last-Modified:  2026-08-10 (R119-3c 索引层下沉)
Status:         🟡 candidate (未 release, 1.2-patch-LIVE 取代)
```

**主题**: 5 既有 crate 加 submodule, 借 VCP/LangGraph/AutoGen/MCP/LSP/MemSave 真接。1 commit 总, 6 R 一气呵成。

| R | 主题 | 落点 |
|---|---|---|
| R63 | apeireth-skills file_loader (借 VCP vcptoolbox) | `crates/apeireth-skills/src/file_loader.rs` |
| R64 | cognition_graph 真接 checkpoint persistence (借 LangGraph MemorySaver) | `crates/apeireth-graph/src/cognition_graph.rs` |
| R65 | MCP tools/list 真接 | `crates/apeireth-mcp/src/tools.rs` |
| R66 | cargo-audit SARIF | `.github/workflows/cargo-audit.yml` |
| R67 | cross_model_benchmark tier (HELM 范式) | `crates/apeireth-eval/src/cross_model_benchmark.rs` |
| R68 | council deliberation stress (AutoGen GroupChat) | `crates/apeireth-council/` |

**关键数字**: 0 触动 24 LOCKED crate, 0 触动 8 项承诺, 0 触动 R11 baseline, 0 改 workspace 1.1.0。

## 详细资料

- **R63-R68 主报告**: [`reports/r63-r68-batch-1.2-candidate-2026-08-09.md`](../../../reports/r63-r68-batch-1.2-candidate-2026-08-09.md)
- **Desktop 同步**: [`reports/r63-r68-desktop-sync-2026-08-09.md`](../../../reports/r63-r68-desktop-sync-2026-08-09.md)
- **R70-R72 FINAL-CHECK (后续 1.2 patch LIVE)**: [`docs/final-check/r70-r72-2026-08-09.md`](../../final-check/r70-r72-2026-08-09.md)
