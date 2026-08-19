# Phase 0 — Fresh Baseline Audit（2026-08-19）

> 状态：**Phase 0 只读审计完成**。本文件是对最新 upstream 与 Pattern 资产的纯审计记录，不含任何代码改动。
> 目标：重新理解最新 Apeireth 架构，再把 Pattern 仍然有价值的能力以符合当前架构的方式重新集成。

---

## 1. Baseline

| 项 | 值 |
|---|---|
| 目标 upstream | `https://github.com/Jimmyxiao2009/apeireth-rust` |
| upstream master HEAD | `7b5528ed` (`docs: CI 修复日志 2026-08-18`) |
| 本地工作树 | `apeireth-rust/`（detached HEAD @ `993e9107`） |
| 本地 origin | `https://github.com/YintaTriss/apeireth-rust.git`（**≠ 目标远程**） |
| 关系 | `993e9107` 是 `7b5528ed` 的祖先；本地集成线全部已在 Jimmy master 历史中 |
| Pattern 参考 | `Pattern/` @ main `1a12d97`（`Jimmyxiao2009/Pattern.git`） |
| Toolchain | `rust-toolchain.toml`: channel = "1.97.1"，components = rustfmt/clippy/rust-src |
| Workspace | resolver = "2"，~83 crate（82 顶层 + memory/extensions 嵌套） |
| Workspace 版本 | 1.2.0（crate 轴）· 产品轴 v1.0.0 正式版（双轴制） |

**关键结论**：本地 `apeireth-rust/` 就是旧 integration 的产物（YintaTriss 线），而用户要求的提交目标是 Jimmy 的干净 master。**两者是同一历史的不同终点**——`993e9107` 已经在 `7b5528ed` 的祖先链里，说明 YintaTriss 的所有 integration commit 其实早已被并进 Jimmy master。

## 2. Apeireth 1.0 真实架构

### 2.1 Rust workspace（核心 crate）

- **apeireth-core** — 基础类型（RiskLevel 等）
- **apeireth-api** — 自研 API 接入平台：OpenAI + Anthropic 双协议，`/v1/chat/completions` 等端点
- **apeireth-companion** — 伙伴器官（A12.5）：Partner/Bond/Milestone/Timeline + daemon（涌现/做梦/反思/宪法/工具桥）+ `companion_serve` 示例（axum :8090，OpenAI 兼容）
- **apeireth-memory** — SQLite 记忆（SqliteMemoryStore / EpisodeStore）
- **apeireth-runtime** — 7 模块端到端 orchestration（heartbeat/task/bus/arbitration/search/group_chat/emotion）
- **apeireth-tui** — ratatui 终端（当前 dev 主线，5 nav 页 + 9 器官 + RuntimeBridge）
- **apeireth-web** — Leptos 0.7 SSR + WASM（Council advisor 面板，非对话 UI）
- **apeireth-tools / tool-*** — 工具注册表 + 9 工具子 crate + 工具运行时
- **apeireth-provider / llm-iface / pipeline / protocol / bus / config / credentials / telemetry** 等支撑层
- **apeireth-gateway / environment** — OpenClaw-mode 常驻 gateway + 6 terminal backends

### 2.2 Frontend（Apeireth 侧现状）

| 载体 | 状态 | 说明 |
|---|---|---|
| `frontend/tauri-prototype/` | **空壳冻结** | 仅剩 Cargo.lock + icons，无 Cargo.toml/lib.rs/package.json（`_frozen/` R145） |
| `crates/apeireth-web` | 活跃 | Leptos SSR，Council 面板，非对话 UI |
| `crates/apeireth-tui` | 活跃 | 当前 dev 主线 |

**决定性事实**：Apeireth 当前**没有任何可运行的对话桌面前端**。官方文档（`docs/frontend-guide.md` 2026-08-16）明确路线 A：**「主人不会写前端 → 接现成开源 Chat 前端（LobeChat/NextChat）对接 OpenAI 兼容端点」**。

### 2.3 Runtime / 对话链路

```
Companion UI (缺失)
      ↓ HTTP (OpenAI 兼容)
apeireth-api serve (:8080)  /  apeireth-companion companion_serve (:8090)
      ↓
apeireth-pipeline (5 步) → apeireth-provider → MiniMax/Anthropic
      ↓
memory 注入 + 今日摘要 + 工具桥 (ToolBridge) + 宪法评审 + 做梦/反思
```

### 2.4 Packaging

`packaging/` 有 brew/deb/docker/msi/rpm/scoop/tarball/zip。桌面打包是 `tauri-prototype` 冻结后遗留。

## 3. Pattern 资产（0.3.0，参考 @ `1a12d97`）

### 3.1 Desktop（apps/desktop）

- **Tauri 2 + Svelte 5 + Vite 6** 完整对话 shell
- 30+ 组件：App/ConversationsView/TasksView/SettingsView/Oobe/QuickWindow/RecentsSidebar/MessageContent/MemoryEditor/GoalsView/ProactiveView/ChannelsView/WorkflowsView/McpView/CompanionWidget/ProjectWorkspace/SessionAgentDocks/...
- Tauri command：persona 保存/加载/列表/激活、model config 读写、directory 列表/读文件/pick、save_session、runtime_status/runtime_connection/get_foreground_window、quick shortcut、single-instance、tray、notification、autostart
- **bridge.rs**（tiny_http 自建 HTTP server）：含 `enigo` 键鼠注入 + `xcap` 截屏 + `recovery` + `agentos` — **全部 DO NOT PORT**
- 前端 `runtime.ts`：WebSocket 连 sidecar（127.0.0.1 随机端口 + token），浏览器 fallback 用 query/localStorage

### 3.2 Mobile（apps/mobile）

- Svelte 5 + Tauri，WebDAV 中继对话 + 任务 + X25519/XChaCha20-Poly1305 安全配对 + 二维码导入
- 依赖 @pattern/relay + qr-scanner + qrcode

### 3.3 packages（8 个共享 TS 包）

`agent` `channels` `core` `memory` `proactive` `protocol` `relay` `shared`

- **protocol**：wire types（ClientMessage/ServerMessage/AgentSlot/TaskRecord/GoalState/SessionPlan...）— 前端 ↔ sidecar WS 契约
- **relay**：WebDAV 信封 + 游标 + E2E 密钥 + 设备配对
- **core**：路由（companion/executor 双槽）、安全评估、工具名归一
- **memory**：SQLite + FTS5 + Pattern Engine（candidate→active→weakening→contradicted→archived）
- **proactive**：主动性引擎（深夜/电量/健康/文件监视）
- **channels**：Telegram/SMTP/IMAP 适配器 + 插件发现
- **agent**：LLM provider 封装（resolveModel/streamChat/generate/buildTools）

### 3.4 Sidecar（sidecar/，TypeScript/Node）

自研 agent loop（`index.ts`）：WS server + MemoryEngine + ProactiveEngine + RelayClient + 工具调度 + slash 命令 + goals/session plan + Pattern Engine 管线 + Presence。

**这是 Pattern 的「第二套 AI runtime」——按 §12 核心原则，DO NOT PORT。**

## 4. Pattern → Apeireth Mapping

| Pattern 资产 | Apeireth 等价物 | 动作 |
|---|---|---|
| **Tauri 2 + Svelte 5 桌面 shell** | 无（tauri-prototype 冻结空壳） | **reuse** — 移植 shell 骨架 |
| **对话 UI 组件**（chat/conversations/message content/markdown） | 无 | **reuse** — 移植 |
| **设置/任务/目标/主动/通道/技能/MCP/记忆 UI** | 无 | **adapt** — 数据源换 Apeireth |
| **Oobe / QuickWindow / tray / 通知 / 快捷键** | 无 | **adapt** — shell 能力保留 |
| **packages/protocol**（wire types） | 无 | **rewrite** — 改为对接 Apeireth 端点 |
| **packages/relay + mobile** | 无 | **adapt** — 若保留 mobile 中继 |
| **sidecar（TS agent loop）** | `apeireth-companion` + `apeireth-runtime` + `companion_serve` | **drop** — 绝不引入第二套 runtime |
| **enigo/xcap 键鼠/截屏 bridge** | 无（上游无对应） | **drop** — DO NOT PORT |
| **recovery/AgentOS/review window** | 无 | **drop** — DO NOT PORT |
| **Packages/memory（Pattern Engine）** | `apeireth-memory` | **drop** — 用 Apeireth 记忆 |
| **packages/agent（LLM provider）** | `apeireth-api` `apeireth-provider` | **drop** |
| **packages/channels** | `apeireth-lark`（飞书/TG sink 已有） | **rewrite** — 按 Apeireth 通道语义 |

## 5. 集成形态（三方案）

### 方案 A：独立 apps（apps/desktop + apps/mobile 作为独立 Tauri 工程）

- **优点**：复用 Pattern 完整 shell，改动最小，最快出可运行桌面
- **缺点**：与 Apeireth workspace（纯 Rust，无 pnpm workspace）割裂；独立 package.json/pnpm 生态；前端如何对接 companion_serve（HTTP）要重写 runtime.ts；mobile 的中继依赖 sidecar（已 drop）需重构
- **复杂度**：新增一个独立 pnpm workspace + 一个 Rust Tauri crate，与现有 83 crate workspace 平行

### 方案 B：UI 嵌入现有 frontend（扩展 apeireth-web 或重建 frontend/）

- **优点**：统一前端载体；不新增 workspace complexity；SSR/WASM 与 Rust 同仓
- **缺点**：apeireth-web 是 Leptos，Pattern 是 Svelte，两套框架不可混；重建对话 UI 等于全部重写
- **复杂度**：Leptos 生态弱于 Svelte/React，对话 UI 开发慢

### 方案 C：Hybrid（推荐）— Pattern shell 做「薄壳 + 对话 UI」，对接 Apeireth HTTP 端点

```
frontend/
  companion-desktop/     ← Svelte 5 + Vite + Tauri 2（从 Pattern apps/desktop 移植 UI 组件）
    src/                 ← 对话/记忆/设置/任务 UI
    src-tauri/           ← 薄 Tauri shell（窗口/托盘/快捷键/通知/单实例）
crates/apeireth-companion  ← 已有后端（companion_serve :8090 OpenAI 兼容）
```

- **优点**：桌面 UI 直接复用 Pattern；后端完全用 Apeireth（companion_serve），符合 §12「UI → Apeireth Runtime Contract → Runtime/Provider/Tools」；不引入第二套 AI runtime；mobile 视成本决定
- **缺点**：需要把 Pattern 的 `runtime.ts`（WS 连 sidecar）重写为 HTTP 对接 Apeireth；新增一个独立前端 workspace
- **复杂度**：中等。前端独立 workspace + 后端零改动

## 6. 推荐

**方案 C（Hybrid）**。

理由：
1. **Apeireth 没有任何现成对话桌面前端**（tauri-prototype 冻结），Pattern 的 Svelte 对话 UI 是目前唯一完整现成的 —— 直接复用价值最高
2. **后端必须用 Apeireth**：companion_serve / apeireth-api 已提供 OpenAI 兼容端点 + 记忆 + 工具桥 + 宪法，Pattern 的 sidecar（TS 第二 runtime）必须 drop，符合 §12
3. **最少重复**：后端 0 改动，前端纯增量
4. **符合 upstream 文档**：Apeireth 官方路线就是「接前端到 OpenAI 兼容端点」，方案 C 是把官方路线从「接 LobeChat」升级为「接自家 Svelte shell」
5. 不为旧 `apps/` 目录结构服务（§7 允许重新设计），也不强制 mobile（§8 mobile 低成本才加入）

## 7. 重定义 Phase 计划

| Phase | 内容 | 验收 |
|---|---|---|
| **1a** | Fresh 分支建立 + workspace 骨架（`frontend/companion-desktop/` 最小 Tauri+Svelte 工程） | cargo check + pnpm build 通过 |
| **1b** | 移植 Pattern 桌面 shell（窗口/托盘/快捷键/通知/单实例 + 最小对话页） | Tauri 壳可运行 |
| **1c** | 移植对话 UI 核心（chat/conversations/message content/markdown/settings） | UI 可显示，HTTP 对接 companion_serve |
| **2a** | 移植任务/目标/记忆/主动/通道等辅助 UI（数据源改 Apeireth） | 各视图可用 |
| **2b** | native 集成（Oobe/QuickWindow/tray 深度） | 体验完整 |
| **3** | Legacy audit（grep enigo/xcap/review/recovery/agentos/sidecar） | 0 残留 |
| **4** | Runtime bridge（AgentRunRequest/RuntimeEvent 契约，预留 Commander/Worker） | 契约类型可编译 |

## 8. 风险

| 风险 | 等级 | 缓解 |
|---|---|---|
| **YintaTriss 与 Jimmy 两个远程** — 提交目标需确认 | 高 | 明确只用 Jimmy master；本地重建 worktree |
| **Pattern 前端深度耦合 sidecar**（runtime.ts WS 协议、slash、goals、session plan） | 高 | 重写 runtime 层为 HTTP；UI 组件逐步解耦 |
| **Svelte UI 与 Leptos 前端并存**（apeireth-web 仍在） | 中 | 方案 C 明确对话走 Svelte 壳，不碰 apeireth-web |
| **pnpm/Node 工具链进入纯 Rust 仓库** | 中 | 前端独立 workspace，不进 Cargo.toml |
| **mobile 依赖 WebDAV 中继 + sidecar** | 中 | Phase 1 先不做 mobile；后续按 Apeireth 通道重写 |
| **上游文档「不自研 UI 接 LobeChat」与自研 shell 路线冲突** | 低 | 文档是建议非硬约束；自研 shell 能力更强，属路线升级 |

---

*本文件为 Phase 0 只读审计输出。后续 commit 到 Jimmy master 的 fresh 分支。*
