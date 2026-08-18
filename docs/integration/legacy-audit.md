# Phase 3 — Legacy Audit（2026-08-19）

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
