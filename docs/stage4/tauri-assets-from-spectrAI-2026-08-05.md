# Tauri 阶段 SpectrAI 资产沉淀 (R19+)

```
[Document-Meta]
Document: docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+
Commit: <待 Mavis 拍板>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 leader 复核)
```

> **性质**: R19+ 资产沉淀文档 — 把 SpectrAI v0.9.21 (TypeScript/Electron AI agent 编排) 19 模块中**TUI 阶段不用但 Tauri 阶段可能用**的 13 项资产系统化登记为"项目资产"，避免"隐形资产"。
>
> **依据**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 + §B.5 + APEIRETH-CONVENTIONS §2 路径系统 + §9 6 锚穿透。
>
> **不修改承诺**: 阶段 1+2+3+4+5 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta 全保留 (见 §4)。

---

## §1 战略背景 (为什么)

### 1.1 现状

| 端 | 状态 | 痛点 |
|---|---|---|
| **TUI 阶段 (R19+)** | Rust 41 crate 已部署, HTTP API 表面稳定, 4 协议真接 minimax m3, TUI 改瘦 (R25) | TUI 仅主 chat; 缺窗口/tray/单实例锁/auto-update 等桌面 app 必备 |
| **Tauri 阶段 (R20+)** | 终极前端 (user memory #8); 主人干 TUI/后端, AI 团队干 Tauri 设计 | Tauri 团队接手时需快速找到 SpectrAI 19 模块中的 13 项 Electron 桌面特性资产 |
| **SpectrAI v0.9.21** | TypeScript/Electron 桌面 app, 19 模块 ~25.6K LOC, 已实装窗口/tray/auto-update/IPC 11 类等 | TS 源码是 Tauri 团队的"参考实现"; 但分散在 19 模块目录里, 接手者不知从哪开始 |

### 1.2 为什么沉淀

1. **避免"隐形资产"**: Tauri 阶段需要某个特性 (e.g. auto-update), 不知道 SpectrAI 已有 `UpdateManager.ts` 244 LOC 可参考
2. **避免重新发明轮子**: Tauri 团队不知道 `OutputReaderManager.ts` 已解决 Claude Code JSONL 解析
3. **避免架构决策走偏**: Tauri 团队不知道 SpectrAI 11 类 IPC handler 划分, 自己随便设计 namespace
4. **降低接手成本**: Tauri 团队接手时, 1 张表查 "我要 X → T-XXX → SpectrAI 哪个文件 → 怎么翻译"

### 1.3 沉淀范围 (13 项)

> **不含**: P0 核心 (adapter/session/agent/storage/git) — 蓝图 §5.2 标 "P0 翻译进 TUI"; Tauri 阶段直接复用 TUI 已实装的 `apeireth-*` crate, 不需要 SpectrAI 源码。
>
> **含**: TUI 阶段**不集成**但 Tauri 阶段可能用上的 13 项 (T-001 ~ T-013), 全部 SpectrAI Electron 桌面特性。

### 1.4 战略原则 (硬约束)

| 原则 | 来源 | 落地 |
|---|---|---|
| **TUI 阶段不动这些资产** | user memory #9 TUI 升级节奏 | TUI 仅用主 chat + 团队 + worktree; 13 项沉淀待 Tauri |
| **Tauri 阶段优先复用, 不重写** | user memory #6 不重复造轮子 | 13 项每项都列 SpectrAI 原始文件 + LOC, Tauri 团队 1:1 翻译思路 |
| **符合 12 子规范** | APEIRETH-CONVENTIONS.md §0.1 | Document-Meta 头 + 路径 + 锚穿透 + 不修改承诺 |
| **6 主哲学锚穿透** | APEIRETH-CONVENTIONS §9 | S-1 / S-2 / O-5 / O-2 / O-3 / O-4 |

---

## §2 SpectrAI → Tauri 资产映射表 (13 项)

> 格式参考 `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 (P0 集成映射表)。
> SpectrAI 原始路径: `.minimax-agent-cn\spectrai\spectrai-source\src\main\`

| # | Tauri 资产 | SpectrAI 原始 | 文件 | LOC | 何时需要 | 怎么用 | 风险 |
|---|-----------|--------------|------|----:|---------|--------|------|
| **T-001** | Tauri command 映射 (11 类 IPC → `#[tauri::command]`) | `ipc/index.ts` 82 LOC + 14 个 handler 文件 | `.minimax-agent-cn\spectrai\spectrai-source\src\main\ipc\` (14 文件: `index.ts` / `shared.ts` / `agentHandlers.ts` / `fileManagerHandlers.ts` / `gitHandlers.ts` / `mcpHandlers.ts` / `providerHandlers.ts` / `registryHandlers.ts` / `sessionHandlers.ts` / `skillHandlers.ts` / `systemHandlers.ts` / `taskHandlers.ts` / `updateHandlers.ts` / `workspaceHandlers.ts`) | ~3797 (14 文件合计) | Tauri 阶段做 `#[tauri::command]` 时 | 14 个 IPC handler → 14 个 Tauri command (1:1 翻译); 复用 TUI 已实装的 `apeireth-*` crate 作为后端 (TUI 是 Tauri 的"集成测试床", user memory #8) | 11 类 IPC 分类的 command 命名空间冲突; Tauri command 命名建议 `domain_action` 模式 (e.g. `agent_spawn` / `session_send` / `git_worktree_create`) |
| **T-002** | Electron BrowserWindow 多窗口管理 | `index.ts:851` bootstrap + theme overlay | `.minimax-agent-cn\spectrai\spectrai-source\src\main\index.ts` | ~1400 (root 入口合计) | Tauri 阶段做多窗口 (聊天 + 设置 + 通知) | Electron 窗口逻辑 → Tauri `WebviewWindow` API; `tauri::WindowBuilder` 创建; `window.on_window_event` 监听 | theme overlay 跨平台兼容性 (Windows/macOS/Linux); Tauri 主题切换走 CSS variables 更干净 |
| **T-003** | 系统托盘 + 应用更新 | `TrayManager.ts` 216 LOC + `UpdateManager.ts` 244 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\tray\` + `update\` | 460 (216+244) | Tauri 阶段做 tray + auto-update | 全部用 `tauri-plugin-tray` + `tauri-plugin-updater` 重写; 托盘菜单 4 项 (打开/退出/更新/关于) + auto-update 增量签名 | auto-update 服务器配置 (Tauri 需 HTTPS endpoint + 公钥); 增量算法用 zstd chunk (蓝图 §9.6 OTA 完善) |
| **T-004** | macOS/Linux PATH 引导 | `shellPath.ts` 107 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\bootstrap\shellPath.ts` | 107 | Tauri macOS 开发 | `tauri-plugin-shell` + 环境变量; macOS .app bundle 不继承用户 shell PATH, 启动时调 `/bin/zsh -ilc 'echo $PATH'` 恢复 | Windows 行为差异 (Windows .exe 继承系统 PATH, 不需要); 仅 macOS/Linux 需要 |
| **T-005** | 结构化日志读取器 | `OutputReaderManager.ts` 75 LOC + `ClaudeJsonlReader.ts` 402 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\reader\` | 477 (75+402) | Tauri 阶段 UI 实时显示 agent 日志 | Claude Code JSONL 解析 → Tauri 前端 stream; `tauri::ipc::Channel<T>` 推流 + 前端 `EventSource` 消费 | Claude Code JSONL 格式可能变 (官方 weekly 升级); 解析器需 version-aware 兼容 |
| **T-006** | 单实例锁 | `index.ts` 里的 `app.requestSingleInstanceLock` | `.minimax-agent-cn\spectrai\spectrai-source\src\main\index.ts` (引用, ~20 LOC) | ~20 | Tauri 阶段 | `tauri-plugin-single-instance` 一行集成; 默认行为: 第二实例退出 + 唤醒第一实例 | 默认 vs 自定义行为 (自定义 callback 可做"将文件路径传给第一实例"); SpectrAI 用默认 |
| **T-007** | 主题覆盖层 | theme overlay (CSS + system theme detection) | `.minimax-agent-cn\spectrai\spectrai-source\src\main\index.ts` (引用, ~50 LOC) + `renderer/src/styles/` | ~50 | Tauri 阶段 | Tauri CSS theme switcher + `window.matchMedia('(prefers-color-scheme: dark)')`; 3 主题 (light/dark/auto) | 跨平台 theme 兼容 (Windows 10/11 高对比度模式); SpectrAI 用 `nativeTheme.shouldUseDarkColors` |
| **T-008** | 启动 Stage 0-7 | `bootstrap/index.ts` 107 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\bootstrap\` | 107 | Tauri 阶段 | 7 stage 启动模式 (DB init → Adapter load → Manager init → Bridge init → Event wire → IPC ready → Window show) → Tauri `setup` hook | Electron vs Tauri 启动模型差异 (Electron `app.whenReady` 异步 vs Tauri `setup` 同步 hook); 7 stage 切分思路可借鉴 |
| **T-009** | 文件管理 IPC | `ipc/fileManagerHandlers.ts` (~250 LOC) | `.minimax-agent-cn\spectrai\spectrai-source\src\main\ipc\fileManagerHandlers.ts` | ~250 | Tauri 阶段 | `ipcMain.handle('file:read', ...)` → `#[tauri::command] file_read(path: String)`; 复用 TUI 已实装的 `apeireth-storage` (蓝图 §5.2 P1) | 路径权限 (`std::path::Path` 校验, 防 `..\` 越权); Tauri 有 `tauri-plugin-fs` 可直接用 |
| **T-010** | 工作区 IPC | `ipc/workspaceHandlers.ts` (~200 LOC) | `.minimax-agent-cn\spectrai\spectrai-source\src\main\ipc\workspaceHandlers.ts` | ~200 | Tauri 阶段 | → Tauri command; 复用 TUI 已实装的 `apeireth-git` worktree 操作 (蓝图 §5.2 P1) | worktree 路径处理 (Windows `\` vs POSIX `/`); 用 `PathBuf` 统一 |
| **T-011** | Git 操作 IPC | `ipc/gitHandlers.ts` (~200 LOC) + `GitWorktreeService.ts` 746 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\ipc\gitHandlers.ts` + `git\GitWorktreeService.ts` | 946 (200+746) | Tauri 阶段 (如果 Tauri 也要 git worktree 集成) | GitWorktreeService 746 LOC 翻译为 Rust crate `apeireth-git` (蓝图 §5.2 P1, 估 1000 LOC); `ipc/gitHandlers.ts` → Tauri command 8 个 | Windows git 路径 (`where git` / `\\?\` 长路径); 跨平台 exec 用 `tokio::process::Command` |
| **T-012** | Confirmation 检测 + 解析器 | `parser/rules.ts` 180 LOC + `geminiRules.ts` 188 LOC + `genericRules.ts` 175 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\parser\` | 543 (180+188+175) | Tauri 阶段 (如果做 CLI 命令审批 UI) | 543 LOC 解析器 → `apeireth-parser` crate (蓝图 §5.2 P2, 估 1500-2000 LOC); 5 Provider 各自 CLI 格式 | 5 Provider 各自 CLI 格式 (Claude/Codex/Gemini/iFlow/OpenCode); 正则规则易碎, 季度需更新 |
| **T-013** | 文件变化追踪 UI | `FileChangeTracker.ts` 511 LOC | `.minimax-agent-cn\spectrai\spectrai-source\src\main\tracker\` | 511 | Tauri 阶段做文件变化显示 | `notify` crate (Rust) → Tauri `emit('file-changed', ...)` event; 前端监听 + diff 展示 | 跨平台 file watcher (Windows ReadDirectoryChangesW / macOS FSEvents / Linux inotify); notify 5.x 已统一 |

### 资产分类总览

| 类别 | 资产 ID | 累计 LOC | 翻译目标 |
|---|---|---:|---|
| **窗口 / UI 框架** | T-002 / T-006 / T-007 | ~1470 | Tauri WebviewWindow + plugin |
| **系统集成** | T-003 / T-004 | 567 | Tauri 官方 plugins |
| **流式 / 日志** | T-005 | 477 | Tauri Channel<T> |
| **IPC 翻译** | T-001 / T-008 / T-009 / T-010 / T-011 | ~2704 | `#[tauri::command]` + 复用 TUI crate |
| **解析器** | T-012 | 543 | `apeireth-parser` crate |
| **Watcher** | T-013 | 511 | `notify` crate + Tauri event |
| **总计** | 13 项 | **~6272** | — |

---

## §3 Tauri 资产使用流程

> Tauri 团队接手 T-XXX 时, 严格按 5 步走。

### 5 步走 (Tauri 团队 SOP)

1. **查表**: 看本表 `T-XXX` 行, 确认 Tauri 阶段是否真的需要
2. **找源**: 按 `SpectrAI 原始` 路径找 `AgentMCPServer.ts` 同级模块 (e.g. `parser/rules.ts` → `spectrai-source/src/main/parser/rules.ts`)
3. **翻译**: 1:1 翻译思路 (按 `蓝图 §5.2` 的映射表风格; TS interface → Rust struct, TS class → Rust struct + impl, TS async function → Rust async fn)
4. **复用**: 复用 TUI 已实装的 `apeireth-*` crate (e.g. `apeireth-git` / `apeireth-storage` / `apeireth-parser`); 不重写
5. **守门**: R-Measure baseline 三值守门 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063); 集成测试覆盖翻译后代码

### 关键引用 (Tauri 团队必读)

| 文档 | 必读章节 | 原因 |
|---|---|---|
| `spectrAI-integration-blueprint-r19-plus-2026-08-05.md` | §5.2 集成映射表 | 知道 TUI 已实装什么, Tauri 不用重写 |
| `apeireth-crate-api-2026-08-05.md` | §1 41 crate 总览 + §2 关键 API 表面 | 知道 Rust 端可调什么 |
| `apeireth-platform-modules-2026-08-05.md` | §1 41 crate + §2 apeireth-api 详细 | 知道 5 Provider 怎么配 |
| `spectrai-architecture-2026-08-05.md` | §2 19 模块 + §3 4 层架构 + §4 5 sequence | 知道 SpectrAI 设计意图 |

### 复用 vs 重写判断矩阵

| 情况 | 决策 | 理由 |
|---|---|---|
| TUI 已实装 `apeireth-X` (P0/P1) | **复用** | user memory #6 不重复造轮子; 1 个后端多 UI |
| TUI 没实装, SpectrAI 有 (P2/P3) | **翻译 + 沉淀为本表** | 蓝图 §5.2 已规划; 本表给具体路径 |
| TUI 没实装, SpectrAI 也没有 | **新设计** | 不在本表范围; 走 12 子规范新建 ADR |

---

## §4 不修改承诺

跟 `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §10 一致:

| ❌ 不修改 | 原因 |
|---|---|
| 阶段 1+2+3 文档 (LOCKED) | 主人明确沉淀 |
| v2 / v4 / v4.1 LOCKED | 哲学层纲领 |
| 阶段 4 核心文档 LOCKED (`6ca80776`) | 蓝图已锁 |
| 阶段 5 施工文档 LOCKED (631 行) | 阶段 5 实施时再引用 |
| v6 基础架构 | 主 AI 团队已 LOCKED |
| R11 baseline (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 主人 2026-07-31 明确不动 |
| APEIRETH-CONVENTIONS / VERSIONING / GLOSSARY (顶层 3 文件) | 不动 |
| START-CONSTRUCTION.md | 不动 |
| 附加: `apeireth-legacy/` | R17 finalize 后归档, 不删 |
| 附加: workspace version 1.0.0 | semver 严格, 不动 |
| 现有 ADR 0001~0009 | 不动 |
| 现有 stage4-* 文档 | 不动 |

---

## §5 维护

### 5.1 增改规则

- **新增资产**: 加一行 T-XXX (T-014, T-015, ...), 必填 8 列 (Tauri 资产 / SpectrAI 原始 / 文件 / LOC / 何时需要 / 怎么用 / 风险 / 累计 LOC 更新)
- **使用过的项**: 标 ✅ (在某次 commit message 或 reports 里用了)
- **弃用的项**: 标 ❌ + 原因 (e.g. ❌ Tauri 2.0 不再需要, 已用内置 plugin)
- **LOC 重测**: SpectrAI 升级时 (e.g. v0.9.22), 用 `wc -l` 重测 LOC, 更新本表

### 5.2 沉淀更新节奏

| 触发 | 更新 | Owner |
|---|---|---|
| SpectrAI 升级 (v0.9.22+) | 重测 LOC, 改 `文件` 列路径 (如有变化) | Tauri 团队 |
| Tauri 阶段启动 (R20+) | 全表逐项 ✅/❌ 标 | Tauri 团队 lead |
| Tauri 阶段完成 (R21+) | 本表归档, 改名 `tauri-assets-archive-*.md` | technical_writer |
| 主人拍板新资产 | 加新行 T-XXX | technical_writer |

### 5.3 与蓝图 §5.2 的关系

- **蓝图 §5.2**: TUI 集成 P0 映射 (20+ 行, 含 crate + 文件 + LOC + 风险 + 优先级 + 状态)
- **本表 §2**: Tauri 阶段沉淀 13 项 (13 行, 含 Tauri 资产 + SpectrAI 原始 + 文件 + LOC + 何时需要 + 怎么用 + 风险)
- **关系**: 蓝图 §5.2 标 "⚪ 沉淀" 的行 = 本表 T-XXX 的来源; 本表是蓝图 §5.2 "⚪ 沉淀" 行的具体展开

### 5.4 6 主哲学锚穿透 (按 APEIRETH-CONVENTIONS §9)

- [x] **S-1 主 22:33** 北极星导向 — 13 项资产服务 ASI 北极星 (Tauri 阶段复用)
- [x] **S-2 主 17:43** 实事求是 — 13 项 SpectrAI 真实文件 + LOC, 不假装已实装
- [x] **O-5 主 17:58** 不假装 — 标记 "TUI 阶段不动" + "何时需要" 列, 不假装 TUI 已用
- [x] **O-2 主 19:33** 走在前人经验上 — SpectrAI v0.9.21 实战检验, 1:1 翻译思路
- [x] **O-3 主 23:44** 干到底 — 5 步走 SOP + 复用 vs 重写判断矩阵
- [x] **O-4 主 00:56** 任何人都能接手 — 13 行表 + 4 章节 + §5 维护规则, Tauri 团队接手零学习成本

---

## §6 关联文档

- **蓝图**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 战略, §5.2 集成映射, §B.5 其他沉淀)
- **架构报告**: `.minimax-agent-cn\spectrai\reports\spectrai-architecture-2026-08-05.md` §2 (19 模块架构) + §3 (4 层架构)
- **Crate API**: `.minimax-agent-cn\spectrai\reports\apeireth-crate-api-2026-08-05.md` §1 (41 crate 总览)
- **平台模块**: `.minimax-agent-cn\spectrai\reports\apeireth-platform-modules-2026-08-05.md` §1 (41 crate) + §2 (apeireth-api 详细)
- **ADR 0010**: `docs/adr/0010-mcp-from-spectrai-agentmcpserver.md` (apeireth-mcp 来自 SpectrAI 翻译路径, 同期 ADR)
- **规范**: `APEIRETH-CONVENTIONS.md` §2 路径系统 + §9 锚穿透 + §10 不修改承诺

---

_Tauri 阶段 SpectrAI 资产沉淀 13 项 (technical_writer)._
_TUI 阶段不动, Tauri 阶段接手时 1 张表查 T-XXX._
_每项 8 列: 资产 / 原始 / 文件 / LOC / 何时 / 怎么用 / 风险 / 累计._
_主哲学 6 锚穿透. 任何接手者能查._
