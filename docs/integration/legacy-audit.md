# Phase 3 — Legacy Audit（2026-08-19）

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。补充：文中"Legacy 残留 0"仅指当时 frontend 内部，不含现 `legacy/` 86-crate 归档。

> 对已迁入的 companion-desktop 做 legacy 残留审计。旧 Computer Use / AgentOS 本就不应迁入，
> 这里确认无残留并记录。

## 审计范围

`frontend/companion-desktop/`（前端 + Tauri 薄壳）。

## Grep 结果

| 关键词 | 命中 | 分类 | 处理 |
|---|---|---|---|
| `computer_use` / `Computer Use` | `ExecutionTimeline.svelte` action 映射 | B (runtime-invalid UI) | **已删** — 新 runtime 无此 action |
| `agentos` / `AgentOS` | 无 | — | — |
| `show_review` | 无 | — | — |
| `ReviewWindow` | 无 | — | — |
| `enigo` | 无 | — | — |
| `xcap` | 无 | — | — |
| `riskTier` / `tier` | `ChatMessageEvent.tier` 保留 | D (工具透明 UI) | 保留 — 工具风险等级展示 |
| `screenshotPath` | 无 | — | — |
| `recovery` | 无 | — | — |
| `emergency` | 无 | — | — |

## invoke/backend cross-check（§14）

- 前端 **0 个 `invoke()` 调用** — runtime 层用纯 HTTP fetch，不依赖 Tauri IPC
- 后端 command：`ping` / `open_settings`，均已注册，无死命令
- **结论：无 runtime-invalid UI，无死按钮**

## 结论

**Legacy 残留 0**（除 `ChatMessageEvent.tier` 作为工具透明 UI 字段保留，符合 §13 D 类）。
旧 Pattern 的 Computer Use / AgentOS / ReviewWindow / recovery / enigo / xcap 未进入主路径。

> **补充（2026-08-27, v2 工程重构）**：本文 §1 "Legacy 残留 0" 仅指当时 `frontend/companion-desktop/` 内部相对 pattern 残留（pre-1.0 的 Computer Use / AgentOS 等）。**这不等于 v2 工作区无 legacy**：reconstruct_v2 收敛（commit `72088f61` 起）把 86-crate 旧工作区整体搬入 `legacy/`，当前 `legacy/{donor 77 / archived 15 / frozen 13}` 共 105 个目录，旧代码**全部 reference-only**，由根 `Cargo.toml` `exclude = ["legacy"]` 排除构建；详见根 [ARCHITECTURE.md](../../ARCHITECTURE.md) "Historical material" 节与根 [ROADMAP.md](../../ROADMAP.md) §4。
