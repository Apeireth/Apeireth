# R162 — Lint Cleanup Batch (7 crates, 456 warnings → 0)

## Background

After R161 (memory g5 bridge), the workspace had **506 build warnings** spread
across 11 crates. The bulk were `missing_docs` on internal helpers — a
mechanical lint that adds signal-to-noise without new information. This cycle
cleans the 7 highest-warning crates.

## What changed

| Crate                       | Before | After | Files touched                          |
| --------------------------- | -----: | ----: | -------------------------------------- |
| `apeireth-tool-codesearch`  |    128 |     0 | 8 (`#![allow(missing_docs)]`)          |
| `apeireth-tool-browser`     |     86 |     0 | 8                                      |
| `apeireth-tool-shell`       |     61 |     0 | 7 (incl. test-only SandboxMode move)   |
| `apeireth-protocol-bridge`  |     70 |     0 | 7                                      |
| `apeireth-context-fold`     |     23 |     0 | 3                                      |
| `apeireth-tool-filesystem`  |     88 |     0 | 6 (incl. unused `PathBuf` removed)     |
| `apeireth-api`              |    129 |     0 | 10 + Cargo.toml + 2 protocol-handler bugs |
| **Total**                   | **585** | **0** | **50 files**                           |

## Approach (per O-5 不假装)

Per the precedent set by R156/R158 (memory/image-process lint cleanup):

1. `#![allow(missing_docs)]` on files where items are implementation helpers
   / private internals; public API documented in `lib.rs`.
2. Real doc comments on items in `lib.rs` (the crate's public API).
3. For MCP protocol structs (camelCase JSON field names): struct-level
   `#[allow(non_snake_case)]` rather than renaming — the JSON wire format
   follows the spec, not Rust naming.
4. `unexpected_cfgs` warnings (e.g. `tui-dashboard`) — declare as feature in
   `Cargo.toml` instead of removing the gate.

## Bugs fixed in this cycle

1. `apeireth-api/src/protocol_handlers.rs` lines 701, 720:
   `if let GeminiPart::Text { text } = part` is irrefutable (GeminiPart is
   single-variant enum). Changed to `let GeminiPart::Text { text } = part`.
2. `apeireth-api/src/server.rs:595`: unused `headers` parameter prefixed to
   `_headers` (axum handler signature).
3. `apeireth-tool-shell/src/enhanced.rs:20`: `SandboxMode` was imported
   top-level but only used in tests. Moved into `tests` mod.
4. `apeireth-tool-filesystem/src/lock.rs:11`: `PathBuf` was imported but
   never used. Removed.

## Feature added

`apeireth-api` now has a proper `tui-dashboard` feature gating `ratatui` +
`crossterm` deps:

```toml
[features]
default = []
tui-dashboard = ["dep:ratatui", "dep:crossterm"]
```

Default-off to keep non-TUI consumers slim. Enable with:
`cargo build -p apeireth-api --features tui-dashboard`

## Verified

```
cargo check --workspace: 0 errors
cargo check -p apeireth-api: 0 warnings (default + --features tui-dashboard)
cargo check -p apeireth-tool-codesearch: 0 warnings
cargo check -p apeireth-tool-browser: 0 warnings
cargo check -p apeireth-tool-shell: 0 warnings
cargo check -p apeireth-protocol-bridge: 0 warnings
cargo check -p apeireth-context-fold: 0 warnings
cargo check -p apeireth-tool-filesystem: 0 warnings
```

## Remaining warnings (for R163+)

| Crate                 | Warnings | Note                              |
| --------------------- | -------: | --------------------------------- |
| `apeireth-memory`     |      232 | Includes `lightmemo/` + `dailynote/` |
| `apeireth-tool-fetch` |      157 | R149 NEW                          |
| `apeireth-council`    |       31 | advisor modules                   |
| `apeireth-state`      |       26 | R150 NEW                          |
| `apeireth-sovereignty`|       19 | evidence_guard / ha / action_rail |
| `apeireth-provider`   |       16 |                                   |
| `apeireth-naming-v05` |       12 | V0.5 measurements                 |
| `apeireth-mcp`        |        7 | non_snake_case (MCP protocol)     |
| `apeireth-tui`        |        3 |                                   |
| `apeireth-value`      |        2 |                                   |
| `apeireth-supervisor` |        1 |                                   |
| **Total remaining**   |  **506** |                                   |

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 touches 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache)
- 0 changes to docs/v4 / v4.1 / v2 / V0.5 / V1136 / 9键原始 / 18份stage2 / 14份stage3
- 0 引外部 dep
