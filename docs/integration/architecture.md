# 集成架构（2026-08-19）

> 方案 C (Hybrid): Pattern Svelte 壳做薄壳 + 对话 UI, 对接 Apeireth HTTP 端点, 后端 0 改动.
> 完整背景见 `phase0-audit.md`.

## 目标形态（§12）

```text
Companion UI (frontend/companion-desktop)
     ↓  Agent Runtime Contract (runtime.ts)
     ↓  HTTP/SSE (OpenAI 兼容)
apeireth-companion :8090  /  apeireth-api :8080
     ↓
Runtime / Provider / Tools / Memory / 宪法
```

**单一 AI runtime**：Pattern 的 TS sidecar（第二套 runtime）未迁入，全部走 Apeireth。

## 目录结构

```
frontend/companion-desktop/        ← 独立 pnpm workspace (不进 Cargo.toml members)
  src/App.svelte                    ← 对话壳 (chat/conversations/memory/settings 4 视图)
  src/lib/runtime.ts                ← Agent Runtime Contract + HTTP/SSE adapter
  src/lib/MemoryView.svelte         ← 记忆/工具/器官 (V2 端点)
  src/lib/{MessageContent,TaskCard,ExecutionTimeline,ConversationsView,PageHeader,StatusDot}.svelte  ← 移植自 Pattern
  src/lib/markdown.ts               ← markdown + KaTeX + 代码高亮 (移植)
  src/styles.css + app.css          ← Pattern 主题体系
  src-tauri/                        ← 薄 Tauri 壳 (窗口/托盘/通知, 独立 [workspace])
    src/lib.rs                      ← main 窗 (conf 声明) + quick 窗 (setup) + 托盘
```

## 分层职责

| 层 | 内容 |
|---|---|
| Tauri shell | 窗口/托盘/通知/关闭隐藏到托盘 — **不含 Agent runtime** |
| Svelte UI | 对话/会话/记忆/设置视图, 纯展示 + 事件订阅 |
| Runtime Contract | `AgentRunRequest`/`RuntimeEvent`/`AgentRuntime` 接口 |
| HTTP adapter | SSE 流式, OpenAI 兼容, 对接 Apeireth |
| Apeireth 后端 | companion_serve / apeireth-api — 0 改动 |

## 与 Pattern 原架构的差异

| Pattern 原 | Apeireth 集成 | 说明 |
|---|---|---|
| sidecar (TS agent loop) | apeireth-companion 后端 | 第二套 runtime **drop** |
| WS 协议 (runtime.ts) | HTTP/SSE OpenAI 兼容 | 协议重写 |
| packages/* TS 共享 | 前端独立, 无跨包 | UI 自洽 |
| enigo/xcap bridge | 无 | Computer Use **drop** |
| review/recovery window | 无 | **drop** |

## 关键决策记录

1. **前端框架 Svelte 5**（复用 Pattern UI），非 Leptos（apeireth-web）
2. **前端独立 workspace**（`frontend/companion-desktop`），不进 Cargo.toml（§9 边界）
3. **src-tauri 独立 [workspace]** — 避免被 root Cargo.toml 捕获
4. **runtime 重写为 HTTP** — Pattern WS → Apeireth HTTP/SSE
5. **记忆/工具/器官视图用 V2 端点** — 对应 docs/frontend-guide P1-2/P1-3
