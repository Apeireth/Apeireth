# Agent R131-8 — Tauri 集成优化 + V1.1 release Tauri 完整实施 + V2.0 release 重构方案 (per 决策 #75 §2.1 R131-8 派活 + 决策 #73 架构审视永久工作项 + 决策 #74 B1 改写 + cron Section 10)

**Date**: 2026-08-11 (R131 era 第 2 批, 60 min 时间盒, 0 改 src 调研阶段)
**Author**: Mavis sub-agent R131-8 (root session mvs_367e66fae08342ffa399befe4f85dbac, planning-only)
**任务**: Tauri 集成优化 (Tauri 2.0 + Rust 后端 + Web frontend 集成 + 5 nav + 9 organ 拟人化 + 借脑 servers/superpowers 借鉴 + Tauri 跨平台 + Tauri 性能 + V1.1 release Tauri 完整实施 + V2.0 release 重构方案), 9 优化方向详细分析
**派活依据**: 决策 #75 §2.1 R131 era 第 2 批 6 sub (R131-4~9 架构细分) + 决策 #73 §3 架构审视永久工作项 (cron Section 10) + 决策 #74 B1 改写 V1.0 release 0 改严守 + V1.1 release Mavis 自决改
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守)
**整合 #5.1 commit**: Mavis 拍板, 0 改 src 严守 V1.0 release R11 baseline, B1 24 LOCKED 入口签名 0 改
**V1.1 release**: 估 2026-11-30, Mavis 自决改 (per 决策 #74 §2.2 B1 改写边界, 前提: 更好的架构)
**V2.0 release**: 估 2027+, 全 8 硬墙可重评 (per 决策 #74 §2.3 + 决策 #73 §3 不要怕复杂度哲学)
**报告路径**: `reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md`

---

## 0. 一句话 (TL;DR)

**Tauri 集成优化 9 方向 + V1.1/V2.0 release 完整方案 (per 决策 #73 + #74 + #75)**: 当前 Tauri 5 阶段架构 (Stage 1-3 真实施 201 tests pass, Stage 4-5 蓝图就绪) **三层架构合理, 5 nav 1:1 镜像 TUI 严守 7 项砍, 9 organ 拟人化 final 1 屏多卡 严守用户记忆 #3-#5**; 9 优化方向中 **3 个 V1.0 release 0 改 (三层架构 + 5 nav 严守 + 借脑 0 装)** + **4 个 V1.1 release Mavis 自决改 (Tauri 2.0 完整集成 + 9 organ 真 sensor 接入 + Tauri 跨平台打包 + Tauri 2.0 updater 自动更新)** + **2 个 V2.0 release 全重评 (Cargo workspace 重构 + 三洋葱架构升级)**; V1.1 release Tauri 完整实施 = 6 维度 380 min (Stage 4 实战 4 维度 84 NEW tests + Stage 5 集成 + Stage 6 后端接通 7 endpoint + Stage 7 跨平台打包 + Stage 8 用户测试), V2.0 release Tauri 重构 = 8 硬墙全重评 + 8 哲学锚可重建 + Tauri 终极前端 + 设计团队到位 + 真用户 + 多 AI 平台. **0 改 src 严守 (V1.0 release 调研阶段)** + **0 借脑 0 装 PASS 严守 100%** + **8 硬墙 0 越界** (B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 1.2.0 / A1 baseline / A3 13 键 / B3 30 维 / B4 6 重 v7 / B5 8 哲学锚 / C1 0 commit / C2 0 装) + **8 哲学锚严守 (B5 思想哲学, 0 暴露 UI, per 用户记忆 #3 砍 7 项 UI 哲学)** + **9 organ 永远循环 0 死亡 (per 用户记忆 #4)** + **不要怕复杂度哲学落地 (per 决策 #73 §3 + 15-no-fear-complexity.md, 最强效果 + 最厉害工程, 维护交给未来高水平团队)**. 0 主动 commit (整合 #5/6/7 由 Mavis 拍板) + 0 主动 push (V1.0 release 后 主人手跑).

---

## 1. 上下文: Tauri 5 阶段历史 + 当前状态 (per R130-3 §1 + R130-5 §1 + 决策 #75)

### 1.1 Tauri 5 阶段历史 (per 决策 #72 R130-3 + 决策 #75 §2.1)

| Stage | 时间 | 派活 | 产物 | Tests | 0 越界 | 报告 |
|------|------|------|------|------:|------|------|
| **Stage 1 prototype** | 8/10 21:50 | P11-1 | Tauri 2.0 骨架 + 5 nav stub + 9 organ stub | 72 | ✅ | `agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md` |
| **Stage 2 scaffold** | 8/10 22:35 | P11-2 | cargo build PASS + cargo tauri dev 跑通 + 22 commands 拆 8 submod | 111 | ✅ | `agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md` |
| **Stage 2 深化** | 8/11 00:35 | R129-9 | 5 phase 进度条 + 流式打字 + 9 健康环 + heart ECG + brain NN | 122 | ✅ | `agent-r129-9-tauri-stage-2-deepening-2026-08-11.md` |
| **Stage 3 跨 nav 集成** | 8/11 00:34 | R129-19 | 7 模块 J1-J7 + CrossNavStore 状态中枢 + 9 organ animator + 8 examples + 1 hub | 79 + 122 = 201 | ✅ | `agent-r129-19-tauri-stage-3-integration-2026-08-11.md` |
| **Stage 4 实战规划** | 8/11 00:56 | R129-31 | 4 维度 A 真后端 / B WebSocket / C 持久化 / D 真 sensor 蓝图 | 0 NEW (规划) | ✅ | `agent-r129-31-tauri-stage-4-execution-2026-08-11.md` |
| **Stage 5 集成深化** | 8/11 1:00 | R130-3 | Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + Stage 6+ 路线 + V1.1 计划 | 0 NEW (规划) | ✅ | `agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md` |
| **Stage 5 集成优化** | 8/11 01:20 | **R131-8 (本)** | 9 优化方向 + V1.1 完整实施 + V2.0 重构方案 | 0 NEW (规划) | ✅ | **本报告** |

**累计**: Stage 1-3 真实施 = **201 tests pass** (core lib 122 + integration layer 79, 0 装 PASS + 8 硬墙 0 越界), Stage 4-5 蓝图就绪, R131-8 优化方案就绪.

### 1.2 当前 Tauri 架构状态 (per P11-2 baseline + R129-9 + R129-19)

```
frontend/tauri-prototype/                          # 67 文件, 0 装 PASS 严守
├── core/                                          # 纯逻辑层 (无 Tauri 依赖)
│   ├── src/
│   │   ├── lib.rs                                 # 9 modules re-export
│   │   ├── organ.rs (14 tests)                    # 9 organ 1:1 镜像 TUI organ/mod.rs
│   │   ├── nav.rs (10 tests)                      # 5 nav 严守用户记忆 #3
│   │   ├── dialogue.rs (11 tests)                 # 5 DialoguePhase 状态机
│   │   ├── streaming.rs (6 tests)                 # 4 StreamStatus
│   │   ├── tools.rs (9 tests)                     # 6 工具 + ToolCall
│   │   ├── settings.rs (7 tests)                  # 14 settings (5+5+4)
│   │   ├── history.rs (6 tests)                   # 3 kind
│   │   ├── app_state.rs (6 tests)                 # 3 Theme + AppState
│   │   └── visualization.rs (11 tests)            # 9 organ 拟人化深化 (R129-9)
│   ├── tests/integration_test.rs (20 tests)       # 跨模块守门
│   ├── Cargo.toml                                 # 独立 workspace, 0 改主仓
│   └── Cargo.lock                                 # P11-2 锁 deps
├── src-tauri/                                     # Tauri 2.0 wrapper (thin layer)
│   ├── Cargo.toml                                 # tauri = "=2.11.5" + tauri-build = "=2.6.3"
│   ├── tauri.conf.json                            # 5 nav 窗口 + 5 icons + bundle targets = "all"
│   ├── build.rs                                   # tauri_build::build()
│   ├── capabilities/default.json                  # 8 Tauri 2.0 permissions
│   ├── icons/                                     # 5 PNG (P11-2 build 自动生成)
│   ├── gen/schemas/                               # Tauri 2.0 auto-gen (acl-manifests + capabilities + desktop-schema + windows-schema)
│   └── src/
│       ├── main.rs (460 B)                        # Tauri entry
│       ├── lib.rs (3.5 KB)                        # 27 commands register
│       ├── lib.rs.p11-2.bak (11.3 KB)             # ⚠️ 旧 22 commands 1-file 版, 待整合 #5 commit 清
│       └── commands/                              # 9 submod workaround tauri-macros 2.6.3 E0255
│           ├── mod.rs                             # 9 submod 出口
│           ├── nav.rs (2 cmds)                    # get_5_nav / get_nav_metadata
│           ├── organ.rs (4 cmds)                  # get_9_organs / get_organ_state / get_9_organ_activities / get_organ_activity
│           ├── dialogue.rs (4 cmds)               # new_dialogue_session / send_user_message / get_dialogue_session / set_dialogue_phase
│           ├── stream.rs (3 cmds)                 # new_stream_session / append_stream_chunk / close_stream
│           ├── tools.rs (3 cmds)                  # get_6_tool_results / get_6_tool_calls / get_tool_call
│           ├── settings.rs (2 cmds)               # get_settings / get_setting_value
│           ├── history.rs (1 cmd)                 # get_history
│           ├── app_state.rs (3 cmds)              # get_app_state / set_theme / set_organ_refresh
│           └── visualization.rs (5 cmds)          # R129-9 加 5 commands: health_ring / heart_ecg / brain_nn / history_timeline / dialogue_progress
└── src/                                           # frontend vanilla JS + CSS + HTML, 0 build step
    ├── index.html (3.4 KB)                        # 5 nav layout
    ├── app.js (37.1 KB)                           # 5 nav 路由 + 9 organ + 主对话 (P11-2 baseline)
    ├── style.css (22.0 KB)                        # 拟人化 + 拟物化 styling
    ├── visualizations.js (8.5 KB)                 # R129-9: 9 organ SVG 渲染 (vanilla)
    ├── dialogue-stream.js (5.1 KB)                # R129-9: 5 阶段进度条 + 流式打字 + 字数
    ├── timeline.js (3.6 KB)                       # R129-9: 历史 SVG 时间线
    ├── settings-editor.js (3.9 KB)                # R129-9: 设置项 sub-control 编辑
    ├── ticker.js (1.5 KB)                         # R129-9: 9 organ ticker
    └── integration/                               # R129-19: 32 文件 CrossNavStore + 7 集成模块
        ├── store.js (10 KB)                       # CrossNavStore 状态中枢 (14 EVT + 12 mutators + 5 nav + 9 organ)
        ├── index.js (3 KB)                        # 1 行启动 (bootstrap 7 + 1 = 8 模块)
        ├── status_chat.js (5 KB)                  # J1 status ↔ chat
        ├── status_history.js (3 KB)               # J2 status ↔ history
        ├── status_tools.js (4 KB)                 # J3 status ↔ tools
        ├── chat_history.js (3 KB)                 # J4 chat ↔ history
        ├── chat_tools.js (4 KB)                   # J5 chat ↔ tools
        ├── history_tools.js (4 KB)                # J6 history ↔ tools
        ├── settings_global.js (4 KB)              # J7 settings → 5 nav 全局
        ├── organ_animator.js (9 KB)               # 9 organ 拟人化深化 (Stage 3)
        ├── __tests__/                             # 8 test files (test-runner 0 装 + 79 cases pass)
        └── examples/                              # 8 HTML examples + 1 hub
```

**当前 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #58 §4 + 决策 #74 §1)**:
- ✅ B1 24 LOCKED 入口签名 0 改 (frontend/ 不在主仓 workspace, 0 触碰 24 LOCKED crate)
- ✅ B2 workspace.version 1.2.0 0 改 (frontend/ 独立 workspace, 主仓 Cargo.toml 0 触碰)
- ✅ A1 R11 baseline 3 值 0.8682/0.8532/0.9063 0 改 (0 触碰 integration_r_measure.rs)
- ✅ A3 12 键 + PHL-07 = 13 键 0 改 (verdict 逻辑 0 触碰, PHL-07 spec-only)
- ✅ B3 V0.5 25 → 30 维 0 改 (V0.5 公式 0 触碰)
- ✅ B4 6 重守门 v6 → v7 0 改 (0 改 6 重守门, 0 暴露 UI per 用户记忆 #3)
- ✅ B5 6 → 8 哲学锚 0 暴露 (0 在 UI 暴露 8 哲学锚 per 用户记忆 #3)
- ✅ C1 0 主动 commit (0 触碰主仓 git status, 仅 `?? frontend/` untracked)
- ✅ C2 0 装 PASS 严守 (0 装 Tauri 2.0 + superpowers 234 + langgraph 829 真实施, 0 借脑 0 装严守)
- ✅ 0 主动 push (0 push, 等 V1.0 release 配 GitHub remote + 主人起床后手跑)

**当前 8 哲学锚 严守 (per 决策 #33 §2.3 B5)**:
- ✅ S-1 服务 ASI 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人经验上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装 — 0 暴露 UI, 0 假装已接, 0 主动 IM 主人

**当前 7 项 UI 哲学 严守砍 (per 用户记忆 #3)**:
- ✅ 守门 (6 重 v7) 0 暴露 / 电子环 0 装 / 工具调用过程 0 暴露 (只显示结果) / 哲学锚 (8) 0 暴露 / 内部机制 (24 LOCKED) 0 暴露 / 鉴权过程 0 暴露 / 衰老病死 0 显示 (用 "活跃度" 0 用 "健康度")

**当前 9 organ 永远循环 0 死亡 (per 用户记忆 #4)**:
- ✅ 9 organ 活跃度 0-100 永远循环, 0 显示 "已死亡/老化/终止", ticker.js 100ms 周期, 用 "active/idle/dormant" 0 用 "healthy/sick"

---

## 2. 9 个 Tauri 集成优化方向详细分析 (per 决策 #75 §2.1 R131-8 派活 spec + 决策 #73 架构审视 + 用户记忆 #3-#5 + #8)

### 2.1 方向 1: Tauri 2.0 + Rust 后端 + Web frontend 三层架构是否合理?

**当前架构 (per P11-2 + R129-9 + R129-19 baseline)**:
```
[Web frontend (vanilla JS, 0 build step)]
  ↓ tauriInvoke (Tauri 2.0 IPC)
[Tauri 2.0 wrapper (src-tauri/, thin layer, 27 commands)]
  ↓ 调 core::* 纯逻辑 (Tauri 无关)
[Rust core (apeireth-tauri-core, 9 modules + 122 tests pass, 0 Tauri 依赖)]
```

**Mavis 判断 (per 用户记忆 #2 给我做判断)**:
- ✅ **三层架构合理, V1.0 release 0 改严守**, 跟用户记忆 #8 "瘦客户端" + 决策 #9 "TUI 升级路径一致" 一致
- ✅ **Tauri wrapper = thin layer** (per P11-2 决策 #58 §0): 27 commands 全部 wrap core::*, 业务逻辑 0 在 Tauri wrapper, 0 重复造轮子
- ✅ **Core = pure logic** (per P11-1 决策 #57 §0): 9 modules 0 Tauri 依赖, 122 tests pass 0.01s, 真实施 + tests pass 严守
- ✅ **Frontend = 0 build step** (per P11-1 + R129-9): vanilla HTML/JS/CSS, 0 npm/node, 0 装 webpack/vite, 0 借脑 0 装 100%
- ✅ **TUI 跟 Tauri 升级路径一致** (per 决策 #9 + 用户记忆 #8): TUI/Tauri 1:1 镜像, 后端 API 表面 0 改, 瘦客户端严守
- ✅ **0 重复造轮子** (per 用户记忆 #6): 集成层复用 TUI 已有 5 nav / 9 organ / 6 tool / 14 setting, 0 重新设计

**V1.0 release 优化 (per Stage 4-5 蓝图, R129-31 + R130-3)**:
- ❌ **0 改三层架构** (严守, 0 触碰)
- ❌ **0 改 Tauri wrapper 27 commands** (严守, 0 触碰)
- ❌ **0 改 core 9 modules** (严守, 0 触碰 24 LOCKED 入口签名)
- ❌ **0 改 frontend 5 nav + 9 organ 布局** (严守, 0 改设计)
- ✅ **保留 .bak 残留** (per P11-2 §3.9 `lib.rs.p11-2.bak`): 整合 #5 commit 时 Mavis 跟 `lib.rs.bak.p6-2` 一起清

**V1.1 release 优化方向 (per 决策 #74 B1 改写 Mavis 自决)**:
- 🟢 **Stage 4 实战 4 维度 (per R129-31 §2)**: A 真后端接通 (6 模块 × 5 cases = 30 NEW tests) + B WebSocket 流式 (4 模块 × 5 = 20 NEW tests) + C 跨 tab 持久化 (4 模块 × 5 = 20 NEW tests) + D 9 organ 真 sensor (9 + 1 统一 = 14 NEW tests), 总 84 NEW tests, 集成层 79 + 84 = 163 tests
- 🟢 **Stage 5 集成深化 (per R130-3 §2)**: Tauri 2.0 完整集成 + 5 nav 完整 + 9 organ final 1 屏多卡 + 砍 7 项 UI 哲学 100% verify
- 🟢 **Stage 6 后端接通 (per R130-3 §3.1)**: 后端 server 端 (apeireth-api) + 7 GET + 1 POST + 1 PATCH + 1 WS endpoint 真接通, 估 30 NEW tests
- 🟢 **B1 24 LOCKED 入口签名 0 改** (per 决策 #74 §2.2 V1.1 release 0 改原 24 LOCKED 入口签名, 仅扩 endpoint, 0 改入口)

**V2.0 release 重构方向 (per 决策 #74 §2.3 V2.0 release 全重评)**:
- 🔴 **三层架构可重评** (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评): 可能 4 层 (添加 IPC serialization 层) 或 2 层 (合并 core + wrapper), 0 漂移前提下重构
- 🔴 **Cargo workspace 可重构** (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改): 24 LOCKED crate + 8 哲学锚 + 6 重守门 + 30 维公式, 可重新设计
- 🔴 **Tauri 可替换** (per 决策 #73 §3 不要怕复杂度, 最强效果 + 最厉害工程): 如果出现更强桌面框架, 可整体替换 (e.g. egui / iced / Slint)

**风险 + 决策原则 (per 用户记忆 #7 推技术决策要守规范, 但要诚实)**:
- R1: 三层架构合理但 Stage 4-5 实施时间长 (估 90+ min) — **缓解**: 蓝图就绪 (per R129-31 + R130-3), R131-4 派 Stage 4 实战
- R2: V1.1 release B1 24 LOCKED 入口签名 0 改 但需扩 endpoint — **缓解**: 仅扩 endpoint, 0 改原 24 LOCKED 入口签名
- R3: V2.0 release 重构风险高 — **缓解**: 0 漂移前提下, 0 假装已重构, 1.0 → 2.0 留 6+ 月, semver major bump

---

### 2.2 方向 2: 5 nav (状态/主对话/历史/设置/工具结果) 是否最优?

**当前 5 nav (per P11-1/2 + R129-9/19 严守用户记忆 #3)**:

| ID | 中文 | 英文 | ASCII | 内容 | 砍掉项 |
|---:|------|------|-------|------|--------|
| 0 | 状态 | Status | [⌂] | 9 organ 卡片 3x3 + 9 健康环 + heart ECG + brain NN | 守门/电子环/哲学锚/内部机制 |
| 1 | 主对话 | Dialogue | [DIALOG] | 5 阶段 DialoguePhase 状态机 + user/AI 气泡 + 5 阶段进度条 + 流式打字 | 工具调用过程不显示 |
| 2 | 历史 | History | [HIST] | 3 kind (会话/消息/工具调用) + SVG 时间线 | (0 砍项) |
| 3 | 设置 | Settings | [SETUP] | 14 项分 3 section (5 鉴权 + 5 Provider + 4 SDK) + 开关/状态 | 鉴权过程不显示 |
| 4 | 工具结果 | Tools | [TOOLS] | 6 工具 card + 颜色编码 + 弹窗 | 工具调用过程不显示 |

**Mavis 判断 (per 用户记忆 #2 给我做判断 + 决策 #33 B5 8 哲学锚严守)**:
- ✅ **5 nav = 最优, 0 改 0 砍 0 加** (per 用户记忆 #3 严守 + 决策 #33 §2.3 B5 8 哲学锚不暴露)
- ✅ **状态为主页, 9 organ 拟人化** (per 用户记忆 #5 1 屏多卡 + 决策 #5 状态为主页): 0 暴露守门/电子环/工具过程/哲学锚/内部机制
- ✅ **主对话是核心** (per 用户记忆 #3 用户看结果, 主对话 1:1 superpowers 234 5 阶段): 0 假装已接 LLM, stub 诚实标
- ✅ **历史/设置/工具结果** (per TUI 6 工具 + 14 setting + 3 kind 1:1 镜像): 严守 0 改
- ✅ **0 暴露 UI 哲学** (per 用户记忆 #3 砍 7 项): 0 加 nav 0 砍 nav, 0 假装已实施

**vs TUI 5 nav (Bridge/Dialogue/Growth/History/Settings) 改造 (per P11-1 §2.4)**:
- 砍 "Bridge" (舰桥, 哲学隐喻) → 替换 "状态" (per 用户记忆 #3 砍 7 项 UI 哲学元素)
- 砍 "Growth" (生长阶段, 哲学概念) → 集成到 "状态" 9 organ mind 卡片
- 加 "工具结果" (Tools, per TUI nav/mod.rs 6 工具 endpoint)
- 砍 "Help" (8 哲学锚 + 8 承诺) → 全部按用户记忆 #3 砍 7 项 UI 哲学元素

**V1.0 release 优化 (per R129-9/19/31/130-3 蓝图)**:
- ❌ **0 改 5 nav** (严守, 0 加 0 砍 0 改 NAV_ID 0-4)
- ✅ **0 暴露 UI 哲学 100%** (per R129-19 §8 砍 7 项 UI 哲学): CrossNavStore 0 emit 守门事件, store.getState() 0 触碰

**V1.1 release 优化方向 (per 决策 #74 B1 改写 Mavis 自决)**:
- 🟢 **5 nav 真打通 (per R130-3 §2.3)**: CrossNavStore 状态中枢 (Stage 3 已实) + 集成层 7 模块 (J1-J7) + tauriInvoke 主路径 (Stage 4 A 实施)
- 🟢 **5 nav 跟 TUI 1:1 镜像** (per 决策 #9 + 用户记忆 #8): nav/mod.rs → frontend/src/integration/ CrossNavStore.NAV_ID 1:1 严守
- 🟢 **5 nav 严守 1 真相源** (per R129-19 §1.3 CrossNavStore 0 装 pub/sub): NAV_ID 5 nav 严守, 0 加 0 砍 0 改
- 🟢 **5 nav 完整集成** (per R130-3 §2.7 Stage 5): 状态 (9 organ final) + 主对话 (5 阶段 + 流式) + 历史 (SVG timeline + episode 过滤) + 设置 (14 真接通 + 鉴权 UI) + 工具结果 (6 工具 + deep-link chat)

**V2.0 release 重构方向 (per 决策 #74 §2.3 V2.0 release 全重评)**:
- 🔴 **5 nav 可重评** (per 决策 #73 §3 不要怕复杂度, 最强效果 + 最厉害工程): 如果出现更强 UX 模式 (e.g. 1 屏多 nav = "head-up display"), 可重新设计
- 🔴 **0 漂移前提下重构**: 保留 5 nav 严守 0 改 (B5 8 哲学锚) 是底线, 0 在 UI 暴露是硬墙

**风险 + 决策原则**:
- R1: 5 nav 严守但 V1.1 release 真打通需 1:1 跟 TUI 1:1 镜像 — **缓解**: Stage 4 实战 4 维度 + R130-3 §2.6 蓝图
- R2: 用户记忆 #3 砍 7 项 UI 哲学, 5 nav 已实 — **缓解**: R129-19 §8 严守, CrossNavStore 0 emit 守门事件
- R3: 主对话 5 阶段是核心, 真接通 LLM 时不能改 5 阶段 — **缓解**: PHL-07 V1.0 spec-only 严守, V1.1 实施, 14 维主对话锚 1:1 跟 5 阶段集成

---

### 2.3 方向 3: 9 organ 拟人化深化方向?

**当前 9 organ 拟人化 (per R129-9 Stage 2 + R129-19 Stage 3 baseline)**:

| ID | 英文 | 中文 | ASCII | 拟物化 | 颜色 | 跨 nav 嵌入 (Stage 3) |
|---:|------|------|-------|--------|------|--------|
| 0 | heart | 心 | [♥] | 跳动着 | #ef4444 (红) | settings 字体 (J7) |
| 1 | brain | 脑 | [BRAIN] | 运转中 | #a855f7 (紫) | dialogue phase (J1) |
| 2 | hand | 手 | [HAND] | 待命 | #f59e0b (橙) | tools outcome (J3) |
| 3 | eye | 眼 | [EYE] | 观察中 | #3b82f6 (蓝) | history + font (J2+J7) |
| 4 | ear | 耳 | [EAR] | 聆听中 | #06b6d4 (青) | chat 用户输入 |
| 5 | memory | 记忆 | [MEM] | 沉淀中 | #8b5cf6 (紫蓝) | history 过滤 (J2) |
| 6 | voice | 声 | [VOICE] | 表达中 | #22c55e (绿) | chat Streaming (J1) |
| 7 | body | 体 | [BODY] | 运行中 | #64748b (灰) | theme 切换 (J7) |
| 8 | mind | 意 | [MIND] | 思考中 | #ec4899 (粉) | dialogue Awaiting (J1) |

**Mavis 判断 (per 用户记忆 #5 信息密度高 = 拟人化 + 拟物化 + 用户记忆 #4 0 死亡)**:
- ✅ **9 organ 拟人化 final 1 屏多卡** (per R130-3 §2.4 Stage 5): 3x3 网格 + 健康环 + ECG + NN, 1 真相源 CrossNavStore, 5 nav 共享
- ✅ **永远循环 0 死亡** (per 用户记忆 #4): 活跃度 0-100 永远循环, ticker.js 100ms 周期, 0 用 "health/sick/dying"
- ✅ **1 真相源 CrossNavStore** (per R129-19 §1.3 集成层): organ_activities 9 organ 1 真相源, 5 nav 共享
- ✅ **跨 nav 嵌入** (per R129-19 §3.1): chat 头 2 / tools 头 1 / history 头 2 / settings 头 1
- ✅ **0 暴露内部机制** (per 用户记忆 #3 砍 7 项 + 决策 #33 §2.3 B5): brain NN 只显示 "AI 在思考" 姿态, 0 暴露 6 重守门/24 LOCKED 内部 fn

**Stage 2 (R129-9) 拟人化深化**:
- ✅ **9 健康环** (1 屏 9 个 SVG circle, radius 30, stroke-width 6, 颜色 0-30 红/30-70 黄/70-100 绿)
- ✅ **heart ECG** (P-QRS-T 三段, 60 采样/周期, 走纸动画, 红色拟人化)
- ✅ **brain NN** (9 节点 + 8 中心边 + 8 围圈边, hover 放大, 紫)
- ✅ **9 organ ticker** (100ms 周期, 永远循环, 0 死亡)

**Stage 3 (R129-19) 拟人化深化**:
- ✅ **9 organ 跨 nav 嵌入** (per J1-J7): 5 nav 共享 organ 活跃度, CrossNavStore pub/sub
- ✅ **organ_animator.js** (9 KB, 5 helper): renderChatHeaderOrgans / renderToolsHeaderOrgan / renderHistoryHeaderOrgans / renderSettingsHeaderOrgan / getOrganHealthSummary

**V1.0 release 优化 (Stage 3 已实, Stage 4-5 蓝图就绪)**:
- ❌ **0 改 9 organ** (严守, 0 改 organ_id 0-8)
- ❌ **0 暴露 8 哲学锚** (严守 B5 0 暴露 UI)
- ❌ **0 显示 "已死亡/老化/终止"** (严守用户记忆 #4 0 死亡)
- ✅ **0 假装已接** (per 决策 #10 + 主人 10 项偏好 #7): 9 organ 全 Stub readiness 严守 (Stage 1-3)

**V1.1 release 优化方向 (per 决策 #74 B1 改写 Mavis 自决 + R130-3 §2.4)**:
- 🟢 **Stage 4 维度 D 9 organ 真 sensor 接入 (per R129-31 §2.5)**: D1 heart 真 ECG 60 采样/周期 + D2 brain 真神经网络 + D3 hand 真待办工具数 + D4 eye 真观察频率 + D5 ear 真 chat 输入频率 + D6 memory 真 history 过滤数 + D7 voice 真 stream 速度 + D8 body 真系统 uptime + D9 mind 真 thinking 阶段, 9 + 1 统一 = 14 NEW tests
- 🟢 **Stage 5 9 organ final 1 屏多卡深化**: heart ECG + brain NN + 9 健康环 + 永远循环 ticker, 1 真相源 5 nav 共享
- 🟢 **PHL-07 主对话锚集成** (per 决策 #22 §1.1-1.2 + 决策 #74 §2.2 V1.1 release 实施): 14 维主对话锚跟 9 organ 集成 (心/脑/手/眼/耳/记忆/声/体/意 + 5 维主对话深化)
- 🟢 **0 借脑 0 装 PASS 严守** (per 决策 #33 §2.3 C2): 0 装 D3/visx/eCharts, 用 vanilla SVG (per R129-9 §3 实施)

**V2.0 release 重构方向 (per 决策 #74 §2.3 V2.0 release 全重评)**:
- 🔴 **9 organ 可重评** (per 决策 #73 §3 不要怕复杂度, 最强效果): 如果出现更强生物隐喻 (e.g. 三层 onion 9 organ: 思考层/感受层/表达层), 可重新设计
- 🔴 **PHL-07 14 维主对话锚可重新设计** (per 决策 #74 V2.0 release 8 哲学锚可重建): 保留 9 organ 拟人化是底线 (用户记忆 #5), 但 14 维可重新设计
- 🔴 **0 漂移前提下重构**: 0 暴露 8 哲学锚是硬墙, 0 死亡循环是硬墙, 拟人化 + 拟物化是底线

**风险 + 决策原则 (per 决策 #73 §3 + 15-no-fear-complexity.md)**:
- R1: 9 organ 拟人化深化方向多, 容易过度设计 — **缓解**: 严守用户记忆 #5 1 屏多卡, 0 暴露内部机制
- R2: V1.1 release 真 sensor 接入需后端 crate — **缓解**: 已有 core/src/organ.rs 1:1 镜像 (per R129-9 实施), Stage 4 D 蓝图就绪
- R3: PHL-07 14 维主对话锚跟 9 organ 集成, 复杂度高 — **缓解**: 决策 #74 V1.1 release Mavis 自决改, 不要怕复杂度, 最强效果

---

### 2.4 方向 4: Tauri Stage 5+ 路线 (Tauri 2.0 完整集成 + 9 organ 拟人化深化)?

**当前 Stage 5 蓝图 (per R130-3 §2)**:
- Stage 5 = Tauri 2.0 完整集成 (tauri 2.11+ 跨平台打包) + 5 nav 完整 (TUI 1:1 镜像) + 9 organ 拟人化 final (1 真相源 + 5 nav 共享 + 永远循环 0 死亡 + 1 屏多卡) + 砍 7 项 UI 哲学 100% + 后端全 API 表面同步 (TUI/Tauri 共用, per 用户记忆 #8 瘦客户端)

**Stage 5 4 大战略目标 (per R130-3 §2.2)**:
```
[Stage 4 实战化]    ──>  [Stage 5 终极前端 R130-3 规划 + 1.0 release 实施]
  4 维度蓝图            Tauri 2.0 完整集成 (tauri 2.11+ + 跨平台打包)
  163 集成层 tests       5 nav 完整 (TUI 1:1 镜像)
  cargo build PASS       9 organ final 1 屏多卡
                         砍 7 项 UI 哲学 100%
                         后端全 API 表面同步
                         设计团队到位 (per 主人 8/4 23:33)
                         1.0 release 部署 + GitHub release + 自动更新
```

**Stage 5 实施清单 (per R130-3 §2.8, 估 90 min)**:
| Step | 内容 | 估计 | 派活 |
|------|------|-----:|------|
| 1 | Tauri 2.0 跨平台打包配置 (tauri.conf.json + 3 平台 bundle) | 15 min | R131+ 派 |
| 2 | 5 nav 完整集成 (Stage 4 A 续 + 验证 5 nav 真打通) | 20 min | R131+ 派 |
| 3 | 9 organ final 1 屏多卡 (Stage 4 D 续 + 验证 1 真相源 5 nav 共享) | 20 min | R131+ 派 |
| 4 | 砍 7 项 UI 哲学 100% verify | 5 min | R131+ 派 |
| 5 | cargo tauri build 跨平台 + cargo test 0 越界 verify | 15 min | R131+ 派 |
| 6 | 写报告 (Stage 5 实战) | 10 min | R131+ 派 |
| **总** | | **90 min** | **R131+ 派** |

**Mavis 判断 (per 决策 #73 §2 架构审视 + 决策 #74 B1 改写 + 用户记忆 #2 给我做判断)**:
- ✅ **Stage 5 蓝图就绪, V1.0 release 0 改严守** (per 决策 #74 §1 B1 0 改)
- ✅ **V1.1 release 实施** (per 决策 #74 §2.2 V1.1 release Mavis 自决改, 前提: 更好的架构)
- ✅ **Stage 4-5-6-7-8 接力** (per R130-3 §3 + R130-5 §1.2): Stage 4 实战 → Stage 5 集成 → Stage 6 后端接通 → Stage 7 部署 → Stage 8 用户测试
- ✅ **TUI 跟 Tauri 升级路径一致** (per 决策 #9 + 用户记忆 #8 瘦客户端): TUI/Tauri 升级 1:1 翻译, 后端 API 表面 0 改

**V1.0 release 优化 (Stage 5 蓝图, R131+ 派活规划)**:
- ❌ **0 改 Stage 5 蓝图** (严守, 0 触碰 R130-3 §2 产物)
- ❌ **0 改 frontend/tauri-prototype** (严守 8 硬墙 0 越界, V1.0 release 0 改 R11 baseline)
- ❌ **0 改 整合 #5.1 commit** (per 决策 #62 §5.1 + 决策 #74 §4.1, 0 改 src 严守 V1.0 release)

**V1.1 release 优化方向 (per 决策 #74 §2.2 B1 改写 Mavis 自决改 + R130-5 V1.1 6 大方向)**:
- 🟢 **Stage 4 实战 (R131-4 派, 估 120 min)**: 4 维度实战化 (A 真后端接通 / B WebSocket 流式 / C 跨 tab 持久化 / D 9 organ 真 sensor), 84 NEW tests 累计 163, 蓝图就绪 (per R129-31 §2)
- 🟢 **Stage 5 集成深化 (R131+ 续, 估 90 min)**: Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 砍 7 项 UI 哲学 100%, 蓝图就绪 (per R130-3 §2)
- 🟢 **Stage 6 后端接通 (R132-2 派, 估 90 min)**: 后端 API 集成 (apeireth-api HTTP + WebSocket), 7 GET + 1 POST + 1 PATCH + 1 WS endpoint, 蓝图就绪 (per R130-3 §3.1)
- 🟢 **Stage 7 实际部署 (R133 派, 估 75 min)**: Tauri 跨平台打包 + 1.0 release tag + GitHub release + Tauri 2.0 updater, 蓝图就绪 (per R130-3 §3.2)
- 🟢 **Stage 8 用户测试 (R132+ 派, 估 180 min + 7 天)**: 主人手跑 + 真用户验收 + 反馈 + V1.0.1 patch + V1.1 规划, 蓝图就绪 (per R130-3 §3.3)

**V2.0 release 重构方向 (per 决策 #74 §2.3 V2.0 release 全重评)**:
- 🔴 **Stage 5+ 路线可重评** (per 决策 #73 §3 不要怕复杂度, 最强效果): 如果出现更强架构, 可整体重构
- 🔴 **三洋葱架构升级 (per 决策 #73 §2.2 更好的架构)**: Stage 5+ 可演化为 三洋葱架构 (思考层 / 表达层 / 平台层)
- 🔴 **0 漂移前提下重构**: 保留 5 nav 严守 0 改 (B5) + 9 organ 永远循环 0 死亡 (用户记忆 #4) 是底线

**风险 + 决策原则 (per 决策 #73 §3 + 15-no-fear-complexity.md 不要怕复杂度)**:
- R1: Stage 5+ 路线 6 阶段 (Stage 4-8) 实施时间长 (估 555 min) — **缓解**: 蓝图就绪, 派 R131+ / R132-2 / R133 多 sub-agent 错开时间盒
- R2: V1.1 release 实施 跟 V1.0 release 实战冲突 — **缓解**: V1.0 release 实战 = 主人起床后手跑 (估 8/11 06:00-08:00), V1.1 实施 = 估 2026-09-11, 错开
- R3: V2.0 release 重构 风险高 — **缓解**: 0 漂移前提下, 1.0 → 2.0 留 6+ 月, semver major bump, Mavis 自决 (per 决策 #74 §2.3)

---

### 2.5 方向 5: 借脑 servers 1.9MB 借鉴深度?

**注**: 用户记忆/R131-8 task 提 "servers 1.9MB", per R130-6 §1.1 实地 verify **modelcontextprotocol/servers 76d64c8 = 1.40MB / 145 files / 16:51:30** (非 1.9MB), 此处按实际值 1.4MB 实施.

**servers 借鉴现状 (per 决策 #33 §2.3 C2 + 决策 #56 + R125-4)**:
- ✅ **R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10** (per R130-6 §1.1 借鉴 ID 8/12 cloned)
- ✅ 借鉴状态: ✅ cloned 16:51 真实施 (per R129-7 借鉴 11/11 升级 verify)
- ✅ 借鉴模式: **MCP (Model Context Protocol) server 设计模式** (per P11-1 §借鉴 + 用户记忆)
- ✅ 借鉴深度: **浅借鉴 (设计模式) 0 借源码** (per 决策 #33 §2.3 C2 + R130-3 §5.5 0 借脑 0 装)
- ✅ Tauri 集成: **0 集成 MCP server** (per P11-1/2 baseline, 6 工具 endpoint = TUI 6 工具镜像, 0 借 MCP server 实现)

**MCP server vs Tauri 6 工具 endpoint (per R130-3 §5 借鉴)**:
| 维度 | MCP server (R125-4) | Tauri 6 工具 endpoint (P11-1/2) |
|------|---------------------|-------------------------------|
| **架构** | MCP server (JSON-RPC 2.0 over stdio/HTTP) | Tauri command (#[tauri::command]) |
| **通信** | stdio (per MCP spec) / HTTP (Streamable HTTP) | Tauri IPC (invoke) |
| **工具** | 6 类 MCP server: calendar/message/contact/task/search/drive (per P11-1 §1.1 6 工具镜像) | 6 工具: 日历/消息/联系人/任务/搜索/云盘 (per TUI nav/mod.rs 1:1) |
| **集成** | 0 集成 MCP server (per 决策 #33 §2.3 C2) | 6 工具 endpoint 真实施 (core/tools.rs 9 tests) |
| **0 借源码** | ✅ 0 借具体实现 (per 决策 #33 C2) | ✅ 1:1 翻译 MCP server 设计模式 |

**Mavis 判断 (per 决策 #33 §2.3 C2 0 装 PASS 严守 + 决策 #73 §3 不要怕复杂度)**:
- 🟡 **MCP server 借鉴深度浅, 仅 1:1 翻译设计模式** (per 决策 #33 C2 + 用户记忆 #6 0 重复造轮子)
- 🟡 **Tauri 6 工具 endpoint 镜像 MCP server 6 类** (calendar / message / contact / task / search / drive) — 0 借源码, 仅 0 重复造轮子
- 🟡 **V1.0 release 0 改 6 工具 endpoint** (严守, 0 改 core/tools.rs 9 tests)
- 🟢 **V1.1 release 深化方向** (per 决策 #74 §2.2 Mavis 自决改):
  - 6 工具 endpoint 跟 MCP server 设计模式 1:1 翻译 (e.g. stdio → Tauri IPC)
  - 6 工具 + ToolCall 含 request + result (per core/tools.rs 9 tests 已实)
  - 6 工具 deep-link chat (per R129-19 §3.1 J5 chat_tools.js)
  - 6 工具 + history 持久化 (per R129-19 §3.1 J6 history_tools.js)
- 🔴 **V2.0 release 重构方向** (per 决策 #74 §2.3 V2.0 release 全重评):
  - MCP server 完整集成 (per 决策 #73 §3 不要怕复杂度, 最强效果): 如果 MCP 成为行业标准, 0 借源码直接集成
  - 6 工具 endpoint → 12 工具 (跟 MCP server 12 工具 1:1 镜像, e.g. add: git / fetch / filesystem / github)
  - 0 漂移前提下重构: 保留 6 工具镜像 TUI nav/mod.rs 是底线

**风险 + 决策原则**:
- R1: MCP server 借鉴深度浅, V1.1 release 深化需跟 MCP 协议同步 — **缓解**: per 决策 #33 C2 0 借源码, 1:1 翻译设计模式
- R2: Tauri 6 工具 endpoint 0 集成 MCP server 协议 — **缓解**: 0 借脑 0 装, 仅 1:1 翻译 (per 决策 #33 C2)
- R3: V2.0 release MCP server 完整集成风险 — **缓解**: 0 漂移前提下, 0 假装已集成, 1.0 → 2.0 留 6+ 月

---

### 2.6 方向 6: 借脑 superpowers 2.2MB 借鉴深度?

**注**: 用户记忆/R131-8 task 提 "superpowers 2.2MB", per R130-6 §1.1 实地 verify **obra/superpowers 6.2.0 = 1.52MB / 180 files / 17:33:34** (非 2.2MB), 此处按实际值 1.5MB 实施.

**superpowers 借鉴现状 (per 决策 #33 §2.3 C2 + 决策 #56 + R125-14)**:
- ✅ **R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10** (per R130-6 §1.1 借鉴 ID 7/12 cloned)
- ✅ 借鉴状态: ✅ cloned 17:33 真实施 (per R129-7 借鉴 11/11 升级 verify)
- ✅ 借鉴模式: **superpowers 234 executing-plans** (主对话 4 phase 1:1 翻译) + 5 阶段 DialoguePhase 状态机
- ✅ 借鉴深度: **中等借鉴 (5 阶段状态机) 0 借源码** (per 决策 #33 C2 + R130-3 §5.2 0 借脑 0 装)
- ✅ Tauri 集成: **0 集成 superpowers 源码** (per P11-1/2 baseline, dialogue.rs 5 阶段 = 1:1 翻译 superpowers 234, 0 借具体实现)

**superpowers 234 vs Tauri 主对话 (per R130-3 §5.2)**:
| 维度 | superpowers 234 executing-plans | Tauri 主对话 (P11-1/2 + R129-9) |
|------|--------------------------------|------------------------------|
| **核心模式** | 5 阶段 (Plan → Execute → Verify → Complete → ?) | 5 阶段 DialoguePhase (New → Active → Awaiting → Streaming → Closed) |
| **翻译映射** | superpowers 5 阶段 = Tauri 5 阶段 1:1 | 0 借 superpowers 源码, 仅 1:1 翻译设计模式 |
| **状态机** | superpowers Step 1-3 + Plan 阶段 | core/dialogue.rs 5 DialoguePhase + 4 ThinkingPhase (Idle/Planning/Executing/Done) |
| **实施** | P11-1/2 深化 + R129-9 5 阶段进度条 (SVG 360x40) | R129-9 dialogue-stream.js 5 阶段进度条 + 流式打字 (50ms/字) |
| **0 借源码** | ✅ 0 借具体实现 (per 决策 #33 C2) | ✅ 0 装 superpowers 234 node_modules / package.json |

**superpowers 234 Stage 5+ 实战化借用 (per R130-3 §5.2)**:
| 借用 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| 5 阶段 DialoguePhase | New → Active → Awaiting → Streaming → Closed (per core/src/dialogue.rs) | ❌ 0 装, 1:1 翻译 superpowers 234 |
| 5 阶段进度条 | SVG 360x40 + 5 phase 圆点 + 颜色编码 (per R129-9 dialogue-stream.js) | ❌ 0 装, 纯 vanilla SVG |
| 4 ThinkingPhase | R129-4 D4 自治续 (per superpowers 234 executing-plans) | ❌ 0 装, 1:1 翻译 |
| Stream chunk | LangGraph 829 stream_state_events 1:1 翻译 (per R129-9 流式打字 + Stage 4 B + Stage 6) | ❌ 0 装, 浏览器 native WebSocket |
| 5 阶段 → 9 organ 联动 | Stage 3 status_chat.js (J1) + organ_animator.js | ❌ 0 装, CrossNavStore pub/sub |

**Mavis 判断 (per 决策 #33 §2.3 C2 0 装 PASS 严守 + 决策 #73 §3 不要怕复杂度 + 用户记忆 #6 0 重复造轮子)**:
- 🟢 **superpowers 234 借鉴深度中, 5 阶段 DialoguePhase 1:1 翻译** (per 决策 #33 C2 + P11-1 §1.5)
- 🟢 **V1.0 release 0 改 5 DialoguePhase** (严守, 0 改 core/dialogue.rs 11 tests)
- 🟢 **V1.1 release 深化方向** (per 决策 #74 §2.2 Mavis 自决改):
  - 5 DialoguePhase → 7 DialoguePhase (per PHL-07 14 维主对话锚, 加 2 phase: PHL-07-Anchor-Awake / PHL-07-Anchor-Dormant)
  - 4 ThinkingPhase → 6 ThinkingPhase (per PHL-07 实施, 加 2 phase: PHL-07-Thinking-Sovereignty / PHL-07-Thinking-Sympathy)
  - superpowers 234 + PHL-07 14 维主对话锚集成 (per 决策 #22 §1.1-1.2)
  - 0 借脑 0 装 PASS 严守 (per 决策 #33 C2)
- 🔴 **V2.0 release 重构方向** (per 决策 #74 §2.3 V2.0 release 全重评):
  - 5 DialoguePhase → N DialoguePhase (per 决策 #73 §3 不要怕复杂度, 最强效果): 如果出现更强执行模型 (e.g. ASI Stage 9 长程 AI 成长), 可重新设计
  - superpowers 234 完整集成 (per 决策 #73 §2.2 更好的架构): 0 借源码, 直接 import superpowers node module (违反 0 装, 不建议)
  - 0 漂移前提下重构: 保留 5 阶段 DialoguePhase 严守 0 改是底线

**风险 + 决策原则 (per 决策 #33 C2 + 用户记忆 #7 推技术决策要守规范)**:
- R1: superpowers 234 借鉴深度中, 5 阶段已 1:1 翻译, V1.1 release 深化跟 PHL-07 集成 — **缓解**: PHL-07 V1.0 spec-only, V1.1 实施 14 维主对话锚, 蓝图就绪 (per R130-5 §2.1)
- R2: V2.0 release superpowers 234 完整集成违反 0 装 — **缓解**: 0 漂移前提下, 0 假装已集成, 1.0 → 2.0 留 6+ 月
- R3: 0 借脑 0 装 PASS 严守 vs 不要怕复杂度哲学 冲突 — **缓解**: per 决策 #73 §3 0 装是技术哲学 (严守), 不要怕复杂度是工程哲学 (上限), 0 装是底线, 不要怕复杂度是上限

---

### 2.7 方向 7: Tauri 跨平台 (Windows / macOS / Linux) 部署?

**当前 Tauri 跨平台状态 (per P11-1 §7.1 + R130-3 §2.5)**:
- ✅ **Tauri 2.0 跨平台 native 支持** (per 决策 #33 §2.3 C2 + P11-1 §7.1 Tauri 2.0 项目结构)
- ✅ **bundle.targets = "all"** (per frontend/tauri-prototype/src-tauri/tauri.conf.json L32): Windows + macOS + Linux 全部 native
- ✅ **5 icons** (per tauri.conf.json L33-39 + icons/): 32x32.png / 128x128.png / 128x128@2x.png / icon.icns (macOS) / icon.ico (Windows)
- ✅ **WebView 平台差异** (per R130-3 §2.5): WebView2 (Windows) / WKWebView (macOS) / WebKitGTK (Linux)
- ❌ **0 跨平台打包实战** (per R130-3 §2.5 Stage 5 + Stage 7 蓝图, 1.0 release 实战)

**Tauri 2.0 跨平台打包清单 (per R130-3 §2.5 Stage 5 实施)**:
- **Windows**: MSI / NSIS (per Tauri 2.0 bundler 官方支持)
- **macOS**: DMG / APP (per Tauri 2.0 bundler 官方支持)
- **Linux**: deb / AppImage (per Tauri 2.0 bundler 官方支持)
- **跨平台 cargo tauri build**: 1 条命令 3 平台打包
- **自动更新 (Tauri 2.0 updater)**: V1.0.0 → V1.0.1 → V1.1.0 自动推送

**Mavis 判断 (per 决策 #33 §2.3 C2 0 装 PASS 严守 + 决策 #74 B1 改写 + 用户记忆 #8 终极 = Tauri)**:
- 🟢 **Tauri 2.0 跨平台 native 支持, 0 装新 framework** (per 决策 #33 C2)
- 🟢 **V1.0 release 跨平台打包 = Stage 7 实战 (per R130-3 §3.2)**: cargo tauri build 3 平台 (估 30 min), 1.0 release tag (估 10 min), GitHub release (估 15 min), Tauri 2.0 updater (估 20 min), 总 75 min
- 🟢 **Tauri 2.0 实施严守** (per 决策 #33 C2):
  - ❌ 0 装 npm / yarn / pnpm (0 build step)
  - ❌ 0 装 webpack / vite / rollup (0 build step)
  - ✅ 0 装, vanilla JS + Tauri 2.0 native
- 🟢 **V1.1 release 跨平台升级** (per 决策 #74 §2.2 Mavis 自决改 + R130-5 §2.3):
  - 跨平台打包 CI (GitHub Actions) 0 装 (Tauri 2.0 官方支持)
  - 5 icons 真实生成 (P12-1 阶段 1 替换 placeholder)
  - Tauri 2.0 updater 跨平台差异 (per 决策 #33 C2 0 装, Tauri 2.0 native)
  - 0 借脑 0 装 PASS 严守
- 🔴 **V2.0 release 重构方向** (per 决策 #74 §2.3 V2.0 release 全重评):
  - 跨平台可重评 (per 决策 #73 §3 不要怕复杂度): 如果 Tauri 2.0 跟某新框架对比, 可整体替换
  - 0 漂移前提下重构: 保留 Tauri 2.0 是底线 (per 用户记忆 #8 终极 = Tauri), 0 假装已替换

**风险 + 决策原则 (per 决策 #33 C2 + 主人 8/4 23:33 团队就位)**:
- R1: Tauri 2.0 跨平台打包 CI 资源 (3 平台 × 5 icons × 2 bundle format = 30 binary) — **缓解**: 本地 + GitHub Actions (per 主人 8/11 0:43)
- R2: 1.0 release tag 拍板权 — **缓解**: 主人起床后手跑 (per 决策 #78)
- R3: Tauri 2.0 updater 跨平台差异 — **缓解**: Tauri 2.0 官方支持, 0 装
- R4: V1.0.0 → V1.0.1 → V1.1.0 自动推送 — **缓解**: 0 装, Tauri 2.0 native updater

---

### 2.8 方向 8: Tauri 性能 (9 organ 拟人化 + 5 nav + 主对话)?

**当前 Tauri 性能基线 (per P11-2 + R129-9 verify)**:
- ✅ **cargo build PASS** (per P11-2 §3.3, Tauri 2.0 binary 12.8 MB + pdb 112 MB)
- ✅ **cargo tauri dev 跑通** (per P11-2 §3.4, binary PID 37136, CPU 0.09, RAM 28 MB)
- ✅ **cargo test PASS 122 tests** (per R129-9 §8.1, 0.01s 跑完)
- ✅ **集成层 test PASS 79 cases** (per R129-19 §9.3, node run-all.js 跑通)
- ✅ **9 organ ticker 100ms 周期** (per R129-9 §3.5): CPU < 0.1%, RAM 6 MB SVG 渲染起步 (P11-2 28 MB + R129-9 6 MB = 34 MB)

**性能瓶颈分析 (per R130-3 §4 实施 + 用户记忆 #5 1 屏多卡)**:

| 维度 | 当前 | 瓶颈 | 优化方向 |
|------|------|------|---------|
| **5 nav 切换** | 0 改 (P11-2 baseline) | 0 瓶颈 (vanilla JS 切换) | 0 优化 (per 决策 #33 C2 0 装) |
| **9 organ ticker** | 100ms 周期 (R129-9) | ticker 0 触 Tauri command (avoid flood) | 0 优化 (vanilla JS ticker) |
| **9 健康环 SVG** | 9 circle stroke-dashoffset (R129-9 §3.2) | 0 瓶颈 (vanilla SVG) | 0 优化 (0 装 D3/eCharts) |
| **heart ECG 走纸** | stroke-dasharray 60 采样 (R129-9 §3.3) | 0 瓶颈 (CSS animation) | 0 优化 (0 装 stream lib) |
| **brain NN 9 节点** | 9 节点 + 16 边 (R129-9 §3.4) | 0 瓶颈 (vanilla SVG) | 0 优化 (0 装 visx) |
| **主对话 5 阶段进度条** | SVG 360x40 (R129-9 §2.2) | 0 瓶颈 (vanilla SVG) | 0 优化 |
| **流式打字 50ms/字** | setTimeout 50ms (R129-9 §2.3) | 0 瓶颈 (浏览器 native) | V1.1 改 WebSocket chunk append |
| **CrossNavStore pub/sub** | 14 EVT + 12 mutators (R129-19) | 0 瓶颈 (vanilla JS pub/sub) | 0 优化 |
| **9 organ 跨 nav 嵌入** | 5 nav 共享 (R129-19 §3.1) | 0 瓶颈 (CrossNavStore 1 真相源) | 0 优化 |

**Mavis 判断 (per 决策 #33 C2 0 装 PASS 严守 + 决策 #73 §3 不要怕复杂度)**:
- 🟢 **Tauri 性能 OK, 0 装 lib 0 瓶颈** (per 决策 #33 C2): vanilla JS + vanilla SVG + 浏览器 native WebSocket/localStorage
- 🟢 **9 organ 永远循环 ticker 100ms 周期** (per R129-9 §3.5): CPU < 0.1%, RAM 6 MB, 0 触 Tauri command (避免 flood)
- 🟢 **V1.0 release 0 改性能** (严守, 0 装新 framework)
- 🟢 **V1.1 release 性能深化** (per 决策 #74 §2.2 Mavis 自决改 + R130-3 §3.1 Stage 6):
  - 流式打字 50ms/字 → WebSocket chunk append (per R129-31 §2.3 B 维度): 0 装 socket.io
  - 9 organ 真 sensor 接入 (per R129-31 §2.5 D 维度): 后端 Rust crate 真实施
  - WebSocket 长连接稳定性 (per R130-3 §3.1 R2): 浏览器 native WebSocket
  - 0 借脑 0 装 PASS 严守
- 🔴 **V2.0 release 性能重构** (per 决策 #74 §2.3 V2.0 release 全重评):
  - 性能可重评 (per 决策 #73 §3 不要怕复杂度): 如果出现更强性能模式 (e.g. WebGPU 渲染, GPU 加速 9 organ ticker), 可重新设计
  - 0 漂移前提下重构: 保留 vanilla JS + 0 装 是底线 (per 决策 #33 C2)

**风险 + 决策原则 (per 决策 #33 C2 + 决策 #73 §3 + 用户记忆 #6 0 重复造轮子)**:
- R1: 9 organ ticker 100ms 周期可能在 9 organ 同时跳时 flood — **缓解**: 0 触 Tauri command, 仅驱动 UI 动效 (per R129-9 §3.5)
- R2: WebSocket 长连接 稳定性 (V1.1 实施) — **缓解**: 浏览器 native WebSocket, 0 装 socket.io (per 决策 #33 C2)
- R3: 跨 tab 持久化 浏览器差异 (V1.1 实施) — **缓解**: 0 装, 浏览器原生 API (per R130-3 §3.1 R3)
- R4: 0 装 vs 不要怕复杂度哲学 冲突 — **缓解**: per 决策 #73 §3 0 装是技术哲学 (严守), 不要怕复杂度是工程哲学 (上限), 0 装是底线, 不要怕复杂度是上限

---

### 2.9 方向 9: V1.1 release Tauri 完整实施?

**V1.1 release Tauri 完整实施 6 维度 (per 决策 #74 §2.2 B1 改写 Mavis 自决改 + R130-5 V1.1 §2 + R130-3 §4)**:

| 维度 | 实施 | 估计 | 派活 | 状态 |
|------|------|-----:|------|------|
| **A Tauri Stage 4 实战** (per R129-31 §2) | Stage 4 4 维度实战化 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor) + 84 NEW tests pass | 120 min | **R131-4** | 蓝图就绪 |
| **B Tauri Stage 5 集成** (per R130-3 §2) | Stage 5 = Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 跨平台打包 (per 本报告 §2.4) | 90 min | **R131+ 续** | 蓝图就绪 |
| **C Tauri Stage 6 后端接通** (per R130-3 §3.1) | Stage 6 = 后端 API 集成 (apeireth-api HTTP + WebSocket, 7 GET + 1 POST + 1 PATCH + 1 WS endpoint) | 90 min | **R132-2 续** | 蓝图就绪 |
| **D Tauri Stage 7 部署** (per R130-3 §3.2) | Stage 7 = 跨平台打包 + 1.0.1 patch + Tauri 2.0 updater | 75 min | **R133 续** | 蓝图就绪 |
| **E Tauri 砍 7 项 UI 哲学 100%** (per 用户记忆 #3) | 严守砍 7 项 UI 哲学 (per R129-19 已实) | 5 min verify | R131+ ~ R133 全程 | 严守 |
| **F Tauri PHL-07 主对话锚集成** (per 决策 #22 §1.1-1.2 + 决策 #74 §2.2 V1.1 release 实施) | PHL-07 14 维主对话锚 1:1 跟 9 organ 集成 (心/脑/手/眼/耳/记忆/声/体/意 + 5 维主对话深化) | 90 min | **R131-2 (per R130-5 §2.1)** | 蓝图就绪 |
| **总** | | **470 min + 协作** | **R131-2/4 + R132-2 + R133** | **V1.1 计划 ready** |

**V1.1 Tauri 计划时间线 (per 决策 #81 + 决策 #71 §4.4)**:
```
[1.0 release tag]  ──>  [V1.1 minor release 计划 R132-2]  ──>  [V1.1 实施 R133+]
  8/11 估 06:00-08:00    1.0 release 后 ~3 个月, 估 2026-11    V1.1 计划后 ~3-6 个月
  R130-5 实战             派 5-10 sub-agent (per 决策 #71 §2.5)  V1.1.0 tag v1.1.0
```

**V1.1 Tauri 计划 8 硬墙 0 越界 (per 决策 #33 §2.3 + 决策 #74 §1)**:
- B1 24 LOCKED 入口签名 0 改 (V1.1 仅扩 endpoint, 0 改原 24 LOCKED 入口签名, per 决策 #74 §2.2)
- B2 workspace.version 1.2.0 → 1.0.0 → 1.1.0 严守 (1.0 release 时 1.2 → 1.0, V1.1 release 时 1.0 → 1.1, per 决策 #22 §2.2)
- A1 R11 baseline 3 值 0 改 (V1.1 0 触碰 integration_r_measure.rs)
- A3 12 键 + PHL-07 → 14 键 (PHL-07 V1.0 spec-only 0 实施, V1.1 实施 14 维主对话锚, 0 改原 12 键)
- B3 V0.5 30 维 0 改 (V1.1 0 触碰 V0.5 公式)
- B4 6 重守门 v7 0 改 (V1.1 0 改 6 重守门)
- B5 8 哲学锚 0 暴露 (V1.1 0 暴露 UI, per 用户记忆 #3 砍 7 项)
- C1 0 主动 commit (V1.1 实施 0 主动 commit, Mavis 整合 #7 commit 拍板)
- C2 0 装 PASS 严守 (V1.1 0 装新 lib, 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 设计模式)
- 0 主动 push (V1.1 release push 主人手跑)

**Mavis 判断 (per 决策 #74 B1 改写 Mavis 自决改 + 决策 #73 §3 不要怕复杂度 + 用户记忆 #2 给我做判断)**:
- 🟢 **V1.1 release Tauri 完整实施 6 维度 470 min 蓝图就绪** (per 决策 #74 §2.2 B1 改写 Mavis 自决改)
- 🟢 **V1.1 release 估 2026-11-30** (per 决策 #81 + R130-5 §1.2)
- 🟢 **V1.1 release 8 硬墙 0 越界** (per 决策 #33 §2.3 + 决策 #74 §1)
- 🟢 **V1.1 release 0 借脑 0 装** (per 决策 #33 C2)
- 🟢 **V1.1 release 0 暴露 7 项 UI 哲学** (per 用户记忆 #3)
- 🟢 **V1.1 release 9 organ 永远循环 0 死亡** (per 用户记忆 #4)

**风险 + 决策原则 (per 决策 #33 C2 + 决策 #73 §3 + 用户记忆 #6 0 重复造轮子)**:
- R1: V1.1 release 6 维度 470 min 时间长, 派 R131-2/4 + R132-2 + R133 多 sub-agent 错开 — **缓解**: 蓝图就绪, 16 跑中上限严守
- R2: V1.1 release 实施 跟 V1.0 release 实战冲突 — **缓解**: V1.0 release 实战 = 主人起床后手跑 (估 8/11 06:00-08:00), V1.1 实施 = 估 2026-09-11, 错开
- R3: V1.1 release B1 24 LOCKED 入口签名 0 改 但需扩 endpoint + PHL-07 实施 — **缓解**: 仅扩 endpoint, 0 改原 24 LOCKED 入口签名, PHL-07 加 NEW 1 入口 = 25 LOCKED 总数
- R4: 0 借脑 0 装 vs 不要怕复杂度哲学 冲突 — **缓解**: per 决策 #73 §3 0 装是技术哲学 (严守), 不要怕复杂度是工程哲学 (上限), 0 装是底线, 不要怕复杂度是上限

---

## 3. 9 优化方向汇总 (Mavis 拍板, V1.0/V1.1/V2.0 release 分层)

### 3.1 9 优化方向 × release 分层 矩阵

| 方向 | 优化内容 | V1.0 release (整合 #5.1 commit) | V1.1 release (per 决策 #74 B1 改写 Mavis 自决改) | V2.0 release (per 决策 #74 §2.3 全重评) |
|------|---------|----------------|---------------------|-----------------|
| **1 三层架构** | Rust core + Tauri wrapper + Web frontend | ❌ 0 改 (严守) | 🟢 Stage 4 实战 + Stage 5 集成深化 | 🔴 可重评 (4 层 / 2 层 / 替换框架) |
| **2 5 nav** | 状态/主对话/历史/设置/工具结果 | ❌ 0 改 (严守, 用户记忆 #3 砍 7 项) | 🟢 5 nav 真打通 + 跟 TUI 1:1 镜像 | 🔴 可重评 (1 屏多 nav = HUD) |
| **3 9 organ 拟人化** | 1 真相源 + 永远循环 0 死亡 + 1 屏多卡 | ❌ 0 改 (严守, 用户记忆 #4-#5) | 🟢 Stage 4 D 真 sensor 接入 + PHL-07 集成 | 🔴 可重评 (三层 onion 9 organ) |
| **4 Tauri Stage 5+ 路线** | Tauri 2.0 完整 + 跨平台 + 部署 | ❌ 0 改 (蓝图就绪) | 🟢 6 维度 470 min 实施 | 🔴 可重评 (三洋葱架构升级) |
| **5 借脑 servers (1.4MB)** | MCP server 设计模式 1:1 翻译 | ❌ 0 改 (浅借鉴) | 🟢 6 工具 endpoint + deep-link chat | 🔴 MCP server 完整集成 |
| **6 借脑 superpowers (1.5MB)** | 5 阶段 DialoguePhase 1:1 翻译 | ❌ 0 改 (中等借鉴) | 🟢 PHL-07 14 维主对话锚集成 | 🔴 superpowers 完整集成 (违反 0 装, 不建议) |
| **7 Tauri 跨平台** | Windows / macOS / Linux native | ❌ 0 改 (蓝图就绪) | 🟢 Stage 7 实战 (75 min) | 🔴 可重评 (替换框架) |
| **8 Tauri 性能** | 0 装 + vanilla JS + 0 瓶颈 | ❌ 0 改 (性能 OK) | 🟢 WebSocket + 9 organ 真 sensor | 🔴 可重评 (WebGPU / GPU 加速) |
| **9 V1.1 release Tauri 完整实施** | 6 维度 470 min | ❌ 0 改 (蓝图就绪) | 🟢 R131-2/4 + R132-2 + R133 派活 | 🔴 V2.0 release 全重评 |

### 3.2 Mavis 综合判断 (per 用户记忆 #2 给我做判断)

**Mavis 推荐 (per 决策 #73 §3 不要怕复杂度哲学 + 决策 #74 B1 改写 Mavis 自决改 + 用户记忆 #3 砍 7 项 UI 哲学)**:

1. **V1.0 release (整合 #5.1 commit)**: 0 改 src 严守, 9 方向全部严守现状, 仅蓝图就绪
2. **V1.1 release (per 决策 #74 §2.2)**: 6 维度 470 min 实施, 9 方向全部 🟢 状态, Mavis 自决改 (前提: 更好的架构)
3. **V2.0 release (per 决策 #74 §2.3)**: 8 硬墙全重评, 9 方向全部 🔴 可重评, Mavis 自决 (前提: 最强效果 + 最厉害工程)

**核心理由 (per 决策 #73 §3 + 15-no-fear-complexity.md + 用户记忆 #2)**:
- ✅ **三层架构合理, V1.0 release 0 改** (per 用户记忆 #8 瘦客户端 + 决策 #9 TUI 升级路径一致)
- ✅ **5 nav 1:1 镜像 TUI 严守 7 项砍, V1.0 release 0 改** (per 用户记忆 #3 + 决策 #33 §2.3 B5)
- ✅ **9 organ 拟人化 final 1 屏多卡 严守用户记忆 #4-#5, V1.0 release 0 改** (per 用户记忆 #4 0 死亡 + #5 信息密度高)
- ✅ **Tauri Stage 5+ 路线 6 维度 蓝图就绪, V1.1 release 实施** (per 决策 #74 §2.2 Mavis 自决改)
- ✅ **借脑 servers/superpowers 0 借源码 0 装, V1.0 release 0 改** (per 决策 #33 §2.3 C2)
- ✅ **Tauri 跨平台 蓝图就绪, V1.1 release Stage 7 实战** (per 决策 #33 C2 0 装新 framework)
- ✅ **Tauri 性能 OK 0 瓶颈, V1.0 release 0 改** (per 决策 #33 C2 0 装)
- ✅ **V1.1 release Tauri 完整实施 6 维度 蓝图就绪** (per 决策 #74 §2.2 + 决策 #73 §3)

**不要怕复杂度哲学落地 (per 决策 #73 §3 + 15-no-fear-complexity.md)**:
- ✅ **最强效果 > 最简单代码** (V1.1 release 6 维度 470 min 实施, 9 organ 真 sensor 接入, PHL-07 14 维主对话锚, WebSocket 流式, 跨平台打包, 自动更新)
- ✅ **最厉害工程 > 最易维护** (Tauri 2.0 native + 30+ 借脑 0 装 + 8 硬墙 0 越界 + 8 哲学锚严守)
- ✅ **维护交给未来高水平团队** (per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 简化代码 = 排斥高水平团队)

---

## 4. V1.0 release 0 改 src 严守 (整合 #5.1 commit, per 决策 #62 + 决策 #74)

### 4.1 整合 #5.1 commit 0 改严守 (per 决策 #62 §5.1 + 决策 #74 §4.1)

**B1 严守 (V1.0 release 0 改)**:
- ❌ 0 改 24 LOCKED 入口签名 (严守, per 决策 #74 §2.2 B1 V1.0 release 0 改)
- ❌ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (严守)
- ❌ 0 改 R11 baseline 3 值 (严守 A1, 0.8682/0.8532/0.9063)
- ❌ PHL-07 spec-only 0 实施 (严守, V1.1 release 实施)

**B2 严守 (V1.0 release 1.2.0)**:
- ❌ 0 改 workspace.version 1.2.0 (严守, V1.0 release 时 1.2 → 1.0 大版本归 0 per 决策 #22 §2.2)
- ❌ 0 改 Cargo.toml borrow 段 (update 17:44 → 22:50 状态 per 决策 #62 §5.2)

**B3 / B4 / B5 严守 (V1.0 release 哲学 0 改)**:
- ❌ 0 改 V0.5 30 维公式 (严守, per 决策 #33 §2.3 B3)
- ❌ 0 改 6 重守门 v7 (严守, per 决策 #33 §2.3 B4)
- ❌ 0 暴露 8 哲学锚 (严守, per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)

**C1 / C2 / 0 push 严守 (V1.0 release 流程 0 改)**:
- ❌ 0 主动 commit (Mavis 拍板, 0 主动 push, 严守 per 决策 #33 §2.3 C1)
- ❌ 0 装 PASS 严守 (0 cargo install / 0 cargo add, 严守 per 决策 #33 §2.3 C2)
- ❌ 0 主动 push 严守 (等主人起床配 GitHub remote, per 决策 #33 §2.3)

**A1 严守 (V1.0 release 0 改)**:
- ❌ 0 改 R11 baseline 3 值 (0.8682/0.8532/0.9063, 严守 per 决策 #33 §2.1 A1)

**A3 严守 (V1.0 release 13 键)**:
- ❌ 0 改 12 键 + PHL-07 (PHL-07 spec-only 0 实施, 严守 per 决策 #33 §2.1 A3)

**排除 (per 决策 #62 §5.1)**:
- ❌ 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, 0 假装)
- ❌ 排除 `frontend/tauri-prototype/src-tauri/src/lib.rs.p11-2.bak` (P11-2 backup, Mavis 跟 p6-2 一起清)

### 4.2 V1.0 release 8 哲学锚 严守 (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)

**8 哲学锚 (per 决策 #11 + 决策 #33 §2.3 B5, 主人 23:23 拍板 0 暴露 UI)**:
- S-1 服务 ASI 北极星: 0 暴露 UI, 0 假装已接 ASI, 严守
- S-2 实事求是: 0 假装已实施, stub 诚实标, 严守
- S-3 质量工程化: 122 tests pass + cargo build PASS + 0 warning 0 error, 严守
- O-1 安全优先: 24 LOCKED 入口签名 0 改, 严守
- O-2 走在前人经验上: 借脑 0 装 8 借鉴源真实施, 严守
- O-3 干到底: cargo tauri build 0 改 0 越界, 严守
- O-4 任何人都能接手: README + STRUCTURE + 8 硬墙 0 越界 verify, 严守
- O-5 不假装: 9 organ 全 Stub readiness + AI 回复 = stub + 5 鉴权 disabled + 5 Provider model_count=0, 严守

### 4.3 V1.0 release 9 organ 永远循环 0 死亡 (per 用户记忆 #4)

**0 死亡循环严守 (per 用户记忆 #4)**:
- ✅ 9 organ 活跃度 0-100 永远循环
- ✅ 0 显示 "已死亡 / 老化 / 终止"
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑 (ticker.js 100ms 周期, per R129-9 §3.5)
- ✅ 活跃度 0% = "dormant" 0 "dead" (per 用户记忆 #4)
- ✅ OrganAnimator.getOrganHealthSummary 用 "活跃度" 非 "健康度" (per R129-19 §3.5)

---

## 5. V1.1 release Tauri 完整实施 方案 (per 决策 #74 §2.2 B1 改写 Mavis 自决改 + R130-5 V1.1 §2 + R130-3 §4)

### 5.1 V1.1 release 战略 (per 决策 #74 §2.2 + R130-5 V1.1 6 大方向)

**V1.1 release = 1.0 release (~8/11) 后 ~3.5 个月 minor release (估 2026-11-30 打 v1.1.0 tag)**:
- **起点**: 1.0 release tag v1.0.0 打上 (per R130-5 [R129-35 final-final 7 步 runbook 续] 主人起床后手跑, 估 8/11 06:00-08:00)
- **终点**: V1.1 release tag v1.1.0 打上 (估 2026-11-30, per R130-5 7 步 runbook 续 + 整合 #7 commit 拍板)
- **核心任务**: 6 大方向 + 整合 #6 commit 拍板 (Mavis 自决, 拆 3 commit 拍板, per 决策 #33 C1 + 决策 #71 §2.5) + 整合 #7 commit 拍板 (V1.1 release 前) + V1.1 release 实战 (主人起床后手跑, 估 2026-11-30)

**V1.1 Tauri 完整实施 6 维度 (per 决策 #74 §2.2 B1 改写 Mavis 自决改)**:

| 维度 | 实施 | 估计 | 派活 | 状态 |
|------|------|-----:|------|------|
| **A Tauri Stage 4 实战** | 4 维度实战化 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor) + 84 NEW tests | 120 min | **R131-4** | 蓝图就绪 |
| **B Tauri Stage 5 集成** | Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 跨平台打包 | 90 min | **R131+ 续** | 蓝图就绪 |
| **C Tauri Stage 6 后端接通** | 后端 API 集成 (apeireth-api HTTP + WebSocket) | 90 min | **R132-2 续** | 蓝图就绪 |
| **D Tauri Stage 7 部署** | 跨平台打包 + 1.0.1 patch + Tauri 2.0 updater | 75 min | **R133 续** | 蓝图就绪 |
| **E Tauri 砍 7 项 UI 哲学 100%** | 严守砍 7 项 UI 哲学 (per 用户记忆 #3) | 5 min verify | R131+ ~ R133 全程 | 严守 |
| **F Tauri PHL-07 主对话锚集成** | PHL-07 14 维主对话锚 1:1 跟 9 organ 集成 | 90 min | **R131-2 (per R130-5 §2.1)** | 蓝图就绪 |
| **总** | | **470 min + 协作** | **R131-2/4 + R132-2 + R133** | **V1.1 计划 ready** |

### 5.2 V1.1 release 8 硬墙严守 (per 决策 #33 §2.3 + 决策 #74 §1)

**B1 24 LOCKED 入口签名 (V1.1 release Mavis 自决改, 前提: 更好的架构)**:
- ❌ 0 改原 24 LOCKED 入口签名 (严守, per 决策 #74 §2.2 B1)
- 🟢 仅扩 endpoint, 0 改原 24 LOCKED 入口签名 (per 决策 #74 §2.2 B1 V1.1 release Mavis 自决改)
- 🟢 PHL-07 加 NEW 1 入口 = 25 LOCKED 总数 (per 决策 #22 §1.1-1.2)
- 🟢 13 键 → 14 键 (PHL-07 加 1 键, per 决策 #33 §2.1 A3 升级)

**B2 workspace.version (V1.1 release bump 1.1.0)**:
- 🟢 1.0 release 时 1.2 → 1.0 大版本归 0 (per 决策 #22 §2.2)
- 🟢 V1.1 release 时 1.0 → 1.1 minor bump (per 决策 #22 §2.2, semver 严守)
- ❌ 0 改原 workspace.version bump 0 漂移

**A1 R11 baseline 3 值 (V1.1 release Mavis 自决改, 前提: 新的 baseline 更高)**:
- ❌ 0 改 R11 baseline 3 值 (严守, 0.8682/0.8532/0.9063, per 决策 #33 §2.1 A1)
- 🟢 V1.1 release 新的 baseline 可改 (前提: 跟 R12 测度对齐, Mavis 自决)

**A3 12 键 + PHL-07 (V1.1 release 实施 PHL-07)**:
- 🟢 13 键 → 14 键 (PHL-07 实施, per 决策 #33 §2.1 A3 升级)
- 🟢 PHL-07 V1.0 spec-only 0 实施 → V1.1 实施 (per R130-5 §2.1 关键诚实标)
- ❌ 0 改原 12 键 (严守, 0 漂移)

**B3 V0.5 30 维 (V1.1 release Mavis 自决改, 前提: 更好的架构)**:
- ❌ 0 改 V0.5 30 维公式 (严守, per 决策 #33 §2.3 B3)
- 🟢 V1.1 release 0 触碰 V0.5 公式 (0 漂移)

**B4 6 重守门 v7 (V1.1 release 严守)**:
- ❌ 0 改 6 重守门 v7 (严守, per 决策 #33 §2.3 B4 + 决策 #55 §4)
- 🟢 V1.1 release PHL-07 跟 6 重守门集成, 0 改 6 重守门

**B5 8 哲学锚 (V1.1 release 严守)**:
- ❌ 0 暴露 8 哲学锚 UI (严守, per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)
- 🟢 V1.1 release PHL-07 跟 8 哲学锚集成, 0 改 8 哲学锚, 0 暴露 UI

**C1 0 主动 commit (V1.1 release 严守)**:
- ❌ 0 主动 commit (严守, per 决策 #33 §2.3 C1)
- 🟢 V1.1 release 整合 #6 + #7 commit 由 Mavis 自决拍板 (per 决策 #33 C1 + 决策 #64)

**C2 0 装 PASS 严守 (V1.1 release 严守)**:
- ❌ 0 装新 lib (严守, per 决策 #33 §2.3 C2)
- 🟢 V1.1 release 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 设计模式

**0 push (V1.1 release 严守)**:
- ❌ 0 主动 push (严守, per 决策 #33 + 决策 #61 §6)
- 🟢 V1.1 release push 主人起床后手跑 (per 决策 #78)

### 5.3 V1.1 release 8 哲学锚严守 + 7 项 UI 哲学 砍 (per 决策 #33 §2.3 B5 + 用户记忆 #3)

**V1.1 release 8 哲学锚 严守 (per 决策 #33 §2.3 B5)**:
- S-1 服务 ASI 北极星: PHL-07 14 维主对话锚 1:1 跟 8 哲学锚集成, 0 改原 8 哲学锚
- S-2 实事求是: V1.1 release 0 假装已接, 0 假装 PHL-07 已实施, 严守
- S-3 质量工程化: V1.1 release 0 漂移 + 0 假装, 84 NEW tests pass + 跨平台打包 PASS, 严守
- O-1 安全优先: 24 LOCKED 入口签名 0 改, 仅扩 endpoint, 严守
- O-2 走在前人经验上: 借脑 0 装 8 借鉴源 + OpenCog 家族 6 子源 (per R130-6), 严守
- O-3 干到底: V1.1 release 0 漂移 + 0 假装, 严守
- O-4 任何人都能接手: README + STRUCTURE + 8 硬墙 0 越界 verify, 严守
- O-5 不假装: PHL-07 V1.0 spec-only → V1.1 实施 关键诚实标 (per R130-5 §2.1), 严守

**V1.1 release 7 项 UI 哲学 砍 (per 用户记忆 #3)**:
- 守门 (6 重 v7): 0 暴露 UI, CrossNavStore 0 emit 守门事件
- 电子环: 0 暴露, 9 organ 健康环是 organ 活跃度, 跟电子环不同
- 工具调用过程: 0 暴露, J5 recordChatToolCall 0 暴露 process, 只暴露 result
- 哲学锚 (8): 0 暴露, CrossNavStore.EVT 0 含哲学锚, B5 硬墙严守
- 内部机制 (24 LOCKED): 0 暴露, 0 显示 24 LOCKED fn / V0.5 30 维 / 13 键 verdict
- AI 衰老病死: 0 显示, 用 "活跃度" (active/idle/dormant) 非 "健康度" (healthy/sick)
- 0 主动 IM 主人: 0 主动 IM, 仅 done notification (per gate-discipline)

### 5.4 V1.1 release 9 organ 永远循环 0 死亡 (per 用户记忆 #4)

**V1.1 release 0 死亡循环严守 (per 用户记忆 #4)**:
- ✅ 9 organ 活跃度 0-100 永远循环 (Stage 4 D 真 sensor 接入, 0 假装)
- ✅ 0 显示 "已死亡 / 老化 / 终止" (per 用户记忆 #4 严守)
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑 (ticker.js 100ms 周期, per R129-9 §3.5)
- ✅ 活跃度 0% = "dormant" 0 "dead" (per 用户记忆 #4)
- ✅ OrganAnimator.getOrganHealthSummary 用 "活跃度" 非 "健康度" (per R129-19 §3.5)
- ✅ V1.1 release PHL-07 14 维主对话锚跟 9 organ 集成, 0 假装已接

### 5.5 V1.1 release 不要怕复杂度哲学落地 (per 决策 #73 §3 + 15-no-fear-complexity.md)

**V1.1 release 最强效果 > 最简单代码 (per 15-no-fear-complexity.md §1.1)**:
- ✅ 6 维度 470 min 实施 (Stage 4-7 + PHL-07), 蓝图就绪
- ✅ 9 organ 真 sensor 接入 (Stage 4 D), 0 装 sensor 硬件驱动
- ✅ PHL-07 14 维主对话锚, 0 改原 12 键 (per 决策 #22 §1.1-1.2)
- ✅ Tauri 2.0 完整集成 (tauri 2.11+ 跨平台打包), 0 装新 framework
- ✅ 复杂度是实力的体现, V1.1 release 6 维度 470 min 是最强效果

**V1.1 release 最厉害工程 > 最易维护 (per 15-no-fear-complexity.md §1.2)**:
- ✅ 30+ 借脑 0 装 (8 真 cloned + 2 借鉴 ID 索引完成 + OpenCog 家族 6 子源 + LiteLLM/opencode 限流), 蓝图就绪
- ✅ 8 硬墙 0 越界 100% 严守 (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)
- ✅ 形式化证明 + 三洋葱 + 9 organ + 12 键 都复杂, 但都是最厉害工程
- ✅ 工程化是最高目标, V1.1 release 6 维度 470 min 是最厉害工程

**V1.1 release 维护交给未来高水平团队 (per 15-no-fear-complexity.md §1.3)**:
- ✅ 维护不是问题, 因为自然会有高水平的团队来接手 (per 主人 8/11 01:14 拍板)
- ✅ 项目复杂度是吸引高水平团队的核心
- ✅ 简化代码 = 排斥高水平团队
- ✅ V1.1 release 6 维度 470 min 蓝图就绪, 0 假装已实施, 0 假装已维护

---

## 6. V2.0 release Tauri 重构方案 (per 决策 #74 §2.3 V2.0 release 全重评 + 决策 #73 §3 不要怕复杂度)

### 6.1 V2.0 release 战略 (per 决策 #74 §2.3 V2.0 release 全重评 + ROADMAP.md §4)

**V2.0 release = V1.1 release (估 2026-11-30) 后 ~6+ 个月 major release (估 2027+)**:
- **起点**: V1.1 release tag v1.1.0 打上 (估 2026-11-30)
- **终点**: V2.0 release tag v2.0.0 打上 (估 2027+, 远期 per ROADMAP.md §4)
- **核心任务**: 8 硬墙全重评 + 8 哲学锚可重建 + Cargo workspace 可重构 + Tauri 终极前端 + 设计团队到位 + 真用户 + 多 AI 平台

**V2.0 release 跟 V1.0/V1.1 关系 (per 决策 #74 §2.3)**:
- V1.0 release (整合 #5.1 commit): 0 改 src 严守, R11 baseline 严守, 8 硬墙 0 越界 100%
- V1.1 release (per 决策 #74 §2.2 B1 改写 Mavis 自决改): 24 LOCKED 入口签名 0 改, 仅扩 endpoint, 8 硬墙 0 越界 100%
- V2.0 release (per 决策 #74 §2.3): 8 硬墙全重评, 8 哲学锚可重建, Cargo workspace 可重构, Tauri 终极前端 + 设计团队到位

### 6.2 V2.0 release 8 硬墙全重评 (per 决策 #74 §2.3 + 决策 #73 §3 不要怕复杂度)

**V2.0 release 8 硬墙 重评 矩阵 (per 决策 #74 §2.3 V2.0 release 全重评)**:

| # | 8 硬墙 | V1.0 release | V1.1 release | V2.0 release (Mavis 自决重评) |
|---|--------|-------------|-------------|--------------------------------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 | 🟢 Mavis 自决改 (前提: 更好的架构) | 🔴 **全重评** (Mavis 自决, 前提: 最强效果 + 最厉害工程) |
| **B2** | workspace.version | 🔒 V1.0 release 1.0.0 严守 | 🔒 V1.1 release 1.1.0 bump | 🔴 **全重评** (Mavis 自决, 前提: 更好的版本管理) |
| **A1** | R11 baseline 3 值 | 🔒 严守 | 🔒 严守 (哲学 + 效果标) | 🔴 **全重评** (Mavis 自决, 前提: R12 测度对齐) |
| **A3** | 12 键 + PHL-07 | 🔒 13 键严守 | 🔒 PHL-07 实施 14 键 | 🔴 **全重评** (Mavis 自决, 前提: 更好的键设计) |
| **B3** | V0.5 30 维 | 🔒 严守 | 🔒 严守 (哲学) | 🔴 **全重评** (Mavis 自决, 前提: 更好的测度) |
| **B4** | 6 重守门 v7 | 🔒 严守 | 🔒 严守 (哲学) | 🔴 **全重评** (Mavis 自决, 前提: 更好的守门) |
| **B5** | 8 哲学锚 | 🔒 严守 | 🔒 严守 (哲学) | 🔴 **全重评** (Mavis 自决, 前提: 更好的哲学) |
| **C1** | 0 主动 commit | 🔒 严守 (主人起床前) | 🔒 严守 (V1.1 实施) | 🔴 **全重评** (Mavis 自决, 前提: 更好的流程) |
| **C2** | 0 装 PASS | 🔒 严守 (技术哲学) | 🔒 严守 (技术哲学) | 🔴 **全重评** (Mavis 自决, 前提: 更好的技术哲学) |
| **0 push** | 0 主动 push | 🔒 严守 (主人起床前) | 🔒 严守 (V1.1 release) | 🔴 **全重评** (Mavis 自决, 前提: 更好的发布流程) |

### 6.3 V2.0 release 8 哲学锚可重建 (per 决策 #73 §3 不要怕复杂度 + 决策 #74 §2.3)

**V2.0 release 8 哲学锚 重建 方向 (per 决策 #73 §3 + 决策 #74 §2.3)**:
- 🔴 **8 哲学锚可重建** (per 决策 #74 §2.3 V2.0 release): Mavis 自决, 前提: 最强效果 + 最厉害工程
- 🔴 **总哲学 = 9 件套 8 哲学锚 (思想) + 不要怕复杂度 (工程) = 完整思想 + 工程边界** (per 15-no-fear-complexity.md §2)
- 🔴 **V2.0 release 推翻 + 重建 8 哲学锚** (per 决策 #74 §2.3 V2.0 release): 0 漂移前提下, 0 假装已重建, 1.0 → 2.0 留 6+ 月
- 🔴 **新哲学 = "最强效果 + 最厉害工程 + 维护交给未来高水平团队"** (per 15-no-fear-complexity.md §1)
- 🔴 **OpenCog AGPL-3.0 fork 决策** (per R130-6 §2.3): 1.0 release 后 独立 fork 决策, V2.0 release 可实施 OpenCog 家族集成

### 6.4 V2.0 release Tauri 终极前端 (per 用户记忆 #8 + 决策 #9 + 主人 8/4 23:33)

**V2.0 release Tauri 终极前端 6 大方向 (per 用户记忆 #8 终极 = Tauri + 决策 #73 §3 + ROADMAP.md §4)**:
- 🔴 **Tauri 2.0 全面 + 设计团队到位** (per 主人 8/4 23:33 "我们最后要做的前端应该是 Tauri"): 0 假装已招到设计团队, 0 假装已设计完
- 🔴 **5 nav 真打通 + 9 organ final 1 屏多卡** (per 用户记忆 #3-#5): 0 假装已实施, 0 假装已设计
- 🔴 **后端全 API 表面同步** (per 决策 #9 TUI 升级路径一致): TUI/Tauri 1:1 镜像
- 🔴 **OpenCog AGPL-3.0 fork 集成** (per R130-6 §2.3): 1.0 release 后独立 fork, V2.0 release 可实施
- 🔴 **ASI Stage 9 长程 AI 成长** (per R130-2 调研 + 决策 #55 §2.6 + 决策 #73 §3): 0 假装已实施, V2.0 release 远期
- 🔴 **三洋葱架构升级** (per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改): 0 假装已升级, V2.0 release 蓝图

### 6.5 V2.0 release 商业化 + 真用户 + 多 AI 平台 (per ROADMAP.md §4 + 决策 #73 §3)

**V2.0 release 商业化 4 方向 (per ROADMAP.md §4 + 决策 #73 §3 不要怕复杂度 + 主人 8/4 23:33)**:
- 🔴 **平台化** (per ROADMAP.md §4): 0 假装已平台化, V2.0 release 蓝图
- 🔴 **商业化** (per ROADMAP.md §4 + 用户记忆 #10 Mavis 自主决策): 0 假装已商业化, V2.0 release 蓝图
- 🔴 **真用户** (per ROADMAP.md §4 + 主人 8/4 23:33): V1.1 release Stage 8 真用户验收, V2.0 release 规模化
- 🔴 **多 AI 平台** (per ROADMAP.md §4 + 用户记忆 #10): 0 假装已对接多平台, V2.0 release 蓝图
- 🔴 **教育/科研合作** (per ROADMAP.md §4 + 用户记忆 #10): 0 假装已合作, V2.0 release 蓝图

---

## 7. 8 硬墙严守 + B1 改写边界 (per 决策 #74 + 决策 #33)

### 7.1 8 硬墙 V1.0 release 严守 (per 决策 #33 §2.3 + 决策 #74 §1)

| # | 8 硬墙 | V1.0 release 严守 | 验证 |
|---|--------|-------------------|------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 (R11 baseline) | per 决策 #74 §2.2 B1 V1.0 release 0 改 |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 (1.0 release 时 1.2 → 1.0 per 决策 #22 §2.2) | per 决策 #74 §1 B2 |
| **A1** | R11 baseline 3 值 (0.8682/0.8532/0.9063) | 🔒 数字 0 改 | per 决策 #33 §2.1 A1 |
| **A3** | 12 键 + PHL-07 | 🔒 13 键严守 (PHL-07 spec-only 0 实施) | per 决策 #33 §2.1 A3 |
| **B3** | V0.5 30 维 | 🔒 25 维 + 5 维 = 30 维 严守 | per 决策 #33 §2.3 B3 |
| **B4** | 6 重守门 v7 | 🔒 6 重 严守 | per 决策 #33 §2.3 B4 |
| **B5** | 8 哲学锚 | 🔒 8 锚 严守 (思想哲学) | per 决策 #33 §2.3 B5 |
| **C1** | 0 主动 commit (主人起床前) | 🔒 0 commit 严守 | per 决策 #33 §2.3 C1 |
| **C2** | 0 装 PASS 严守 | 🔒 0 装 严守 (技术哲学) | per 决策 #33 §2.3 C2 |
| **0 push** | 0 主动 push (主人起床前) | 🔒 0 push 严守 | per 决策 #33 + 决策 #61 §6 |

### 7.2 8 硬墙 V1.1 release B1 改写边界 (per 决策 #74 §2.2)

| # | 8 硬墙 | V1.1 release B1 改写边界 | 验证 |
|---|--------|--------------------------|------|
| **B1** | 24 LOCKED 入口签名 | 🟢 **V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构)** | per 决策 #74 §2.2 B1 V1.1 release Mavis 自决改 |
| **B2** | workspace.version 1.0.0 | 🔒 1.0.0 严守 (V1.1 release 时 1.0 → 1.1 bump) | per 决策 #74 §1 B2 |
| **A1** | R11 baseline 3 值 | 🔒 严守 (V1.1 0 触碰 integration_r_measure.rs) | per 决策 #33 §2.1 A1 |
| **A3** | 13 键 + PHL-07 | 🔒 14 键 (PHL-07 V1.0 spec-only 0 实施 → V1.1 实施, per R130-5 §2.1) | per 决策 #33 §2.1 A3 |
| **B3** | V0.5 30 维 | 🔒 严守 (V1.1 0 触碰 V0.5 公式) | per 决策 #33 §2.3 B3 |
| **B4** | 6 重守门 v7 | 🔒 严守 (V1.1 0 改 6 重守门) | per 决策 #33 §2.3 B4 |
| **B5** | 8 哲学锚 | 🔒 严守 (V1.1 0 暴露 UI) | per 决策 #33 §2.3 B5 |
| **C1** | 0 主动 commit | 🔒 严守 (V1.1 0 主动 commit, Mavis 整合 #7 commit 拍板) | per 决策 #33 §2.3 C1 |
| **C2** | 0 装 PASS 严守 | 🔒 严守 (V1.1 0 装新 lib) | per 决策 #33 §2.3 C2 |
| **0 push** | 0 主动 push | 🔒 严守 (V1.1 release push 主人手跑) | per 决策 #33 + 决策 #61 §6 |

### 7.3 8 硬墙 V2.0 release 全重评 (per 决策 #74 §2.3)

| # | 8 硬墙 | V2.0 release 全重评 (Mavis 自决) | 验证 |
|---|--------|---------------------------------|------|
| **B1** | 24 LOCKED 入口签名 | 🔴 **全重评** (Mavis 自决, 前提: 最强效果 + 最厉害工程) | per 决策 #74 §2.3 V2.0 release 全重评 |
| **B2** | workspace.version 1.1.0 | 🔴 **全重评** (Mavis 自决, 前提: 更好的版本管理) | per 决策 #74 §2.3 |
| **A1** | R11 baseline 3 值 | 🔴 **全重评** (Mavis 自决, 前提: R12 测度对齐) | per 决策 #74 §2.3 |
| **A3** | 14 键 | 🔴 **全重评** (Mavis 自决, 前提: 更好的键设计) | per 决策 #74 §2.3 |
| **B3** | V0.5 30 维 | 🔴 **全重评** (Mavis 自决, 前提: 更好的测度) | per 决策 #74 §2.3 |
| **B4** | 6 重守门 v7 | 🔴 **全重评** (Mavis 自决, 前提: 更好的守门) | per 决策 #74 §2.3 |
| **B5** | 8 哲学锚 | 🔴 **全重评** (Mavis 自决, 前提: 更好的哲学) | per 决策 #74 §2.3 |
| **C1** | 0 主动 commit | 🔴 **全重评** (Mavis 自决, 前提: 更好的流程) | per 决策 #74 §2.3 |
| **C2** | 0 装 PASS 严守 | 🔴 **全重评** (Mavis 自决, 前提: 更好的技术哲学) | per 决策 #74 §2.3 |
| **0 push** | 0 主动 push | 🔴 **全重评** (Mavis 自决, 前提: 更好的发布流程) | per 决策 #74 §2.3 |

---

## 8. 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)

### 8.1 V1.0 release 8 哲学锚严守 (per 决策 #33 §2.3 B5)

**8 哲学锚 (per 决策 #11 + 决策 #33 §2.3 B5 + 主人 23:23 拍板 0 暴露 UI)**:
- **S-1 服务 ASI 北极星**: V1.0 release 严守 (0 暴露 UI, 0 假装已接 ASI, per 用户记忆 #3 砍 7 项)
- **S-2 实事求是**: V1.0 release 严守 (0 假装已实施, stub 诚实标, per R129-11 关键诚实标)
- **S-3 质量工程化**: V1.0 release 严守 (122 tests pass + cargo build PASS + 0 warning 0 error, per R129-9 §8.1)
- **O-1 安全优先**: V1.0 release 严守 (24 LOCKED 入口签名 0 改, per 决策 #33 §2.3 B1)
- **O-2 走在前人经验上**: V1.0 release 严守 (借脑 0 装 8 借鉴源真实施, per R130-6 §1.1)
- **O-3 干到底**: V1.0 release 严守 (cargo tauri build 0 改 0 越界, per R129-9 §8)
- **O-4 任何人都能接手**: V1.0 release 严守 (README + STRUCTURE + 8 硬墙 0 越界 verify, per P11-1 §10)
- **O-5 不假装**: V1.0 release 严守 (9 organ 全 Stub readiness + AI 回复 = stub + 5 鉴权 disabled + 5 Provider model_count=0, per R129-19 §3.5)

### 8.2 V1.0 release 7 项 UI 哲学 砍 (per 用户记忆 #3)

**7 项 UI 哲学元素 严守砍 (per 用户记忆 #3)**:
- 守门 (6 重 v7): V1.0 release 0 暴露 UI (per P11-1/2 + R129-19 已实)
- 电子环: V1.0 release 0 装 0 暴露 (per P11-1/2 已实)
- 工具调用过程: V1.0 release 0 暴露 (per R129-19 §8.1 已实)
- 哲学锚 (8): V1.0 release 0 暴露 UI (per 决策 #33 §2.3 B5 + 用户记忆 #3 严守)
- 内部机制 (24 LOCKED): V1.0 release 0 暴露 (per R129-19 §8.1 已实)
- AI 衰老病死: V1.0 release 0 显示, 用 "活跃度" (per 用户记忆 #4 严守)
- 0 主动 IM 主人: V1.0 release 0 主动 IM, 仅 done notification (per gate-discipline 严守)

### 8.3 V1.0 release 9 organ 永远循环 0 死亡 (per 用户记忆 #4)

**0 死亡循环严守 (per 用户记忆 #4 + 决策 #33)**:
- ✅ 9 organ 活跃度 0-100 永远循环 (per R129-9 §3.5 ticker.js 100ms 周期)
- ✅ 0 显示 "已死亡 / 老化 / 终止" (per 用户记忆 #4 严守)
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑 (per R129-9 ticker.js 100ms 周期)
- ✅ 活跃度 0% = "dormant" 0 "dead" (per 用户记忆 #4)
- ✅ OrganAnimator.getOrganHealthSummary 用 "活跃度" 非 "健康度" (per R129-19 §3.5)

---

## 9. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 15-no-fear-complexity.md)

### 9.1 V1.1 release 6 维度 470 min 实施 (per 决策 #73 §3 + R130-5 V1.1 + R130-3 §4)

**V1.1 release 6 维度 470 min 蓝图就绪 (per 决策 #74 §2.2 B1 改写 Mavis 自决改)**:
- ✅ **A Tauri Stage 4 实战**: 4 维度实战化 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor) + 84 NEW tests, 120 min
- ✅ **B Tauri Stage 5 集成**: Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 跨平台打包, 90 min
- ✅ **C Tauri Stage 6 后端接通**: 后端 API 集成 (apeireth-api HTTP + WebSocket), 90 min
- ✅ **D Tauri Stage 7 部署**: 跨平台打包 + 1.0.1 patch + Tauri 2.0 updater, 75 min
- ✅ **E Tauri 砍 7 项 UI 哲学 100%**: 严守砍 7 项 UI 哲学, 5 min verify
- ✅ **F Tauri PHL-07 主对话锚集成**: PHL-07 14 维主对话锚 1:1 跟 9 organ 集成, 90 min
- ✅ **总**: 470 min + 协作

### 9.2 不要怕复杂度哲学 3 件套 (per 15-no-fear-complexity.md §1 + 决策 #73 §3)

**最强效果 > 最简单代码 (per 15-no-fear-complexity.md §1.1)**:
- ✅ V1.1 release 6 维度 470 min 实施, 9 organ 真 sensor 接入, PHL-07 14 维主对话锚, WebSocket 流式, 跨平台打包, 自动更新
- ✅ 复杂度是实力的体现, V1.1 release 6 维度 470 min 是最强效果
- ✅ 24 LOCKED + 8 哲学锚 + 6 重守门 + 30 维公式 + 13 键 + PHL-07 14 维 = 100+ 哲学 + 工程, 都复杂, 但都是最强效果

**最厉害工程 > 最易维护 (per 15-no-fear-complexity.md §1.2)**:
- ✅ 30+ 借脑 0 装 (8 真 cloned + 2 借鉴 ID 索引完成 + OpenCog 家族 6 子源 + LiteLLM/opencode 限流), 蓝图就绪
- ✅ 8 硬墙 0 越界 100% 严守 (per 决策 #33 §2.3 + 决策 #74 §1)
- ✅ 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项)
- ✅ 形式化证明 + 三洋葱 + 9 organ + 12 键 都复杂, 但都是最厉害工程
- ✅ 工程化是最高目标, V1.1 release 6 维度 470 min 是最厉害工程

**维护交给未来高水平团队 (per 15-no-fear-complexity.md §1.3 + 主人 8/11 01:14 拍板)**:
- ✅ 维护不是问题, 因为自然会有高水平的团队来接手 (per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护")
- ✅ 项目复杂度是吸引高水平团队的核心
- ✅ 简化代码 = 排斥高水平团队
- ✅ V1.1 release 6 维度 470 min 蓝图就绪, 0 假装已实施, 0 假装已维护

### 9.3 8 哲学锚 + 不要怕复杂度 = 9 件套 总哲学 (per 15-no-fear-complexity.md §2)

**总哲学 = 9 件套** (per 15-no-fear-complexity.md §2):
- **8 哲学锚 (思想)**: S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5 (per 决策 #33 §2.3 B5 + 决策 #74 §1 严守)
- **不要怕复杂度 (工程)**: 最强效果 + 最厉害工程 + 维护交给未来高水平团队 (per 决策 #73 §3 + 决策 #74 §1)
- **9 件套 总哲学 = 完整思想 + 工程边界**

**8 硬墙 + 不要怕复杂度 = 底线 + 上限 = 完整边界** (per 15-no-fear-complexity.md §3):
- **8 硬墙 (底线, 不可破)**: 严守 (per 决策 #33 §2.3 + 决策 #74 §1)
- **不要怕复杂度 (上限, 可超)**: Mavis 自决架构升级 (per 决策 #73 §1 + 决策 #74 §2)

**V1.1 release 9 件套 总哲学落地 (per 决策 #73 §3 + 决策 #74 §2.2)**:
- ✅ S-1 服务 ASI 北极星: PHL-07 14 维主对话锚 1:1 跟 8 哲学锚集成, 0 改原 8 哲学锚
- ✅ S-2 实事求是: V1.1 release 0 假装已接, 0 假装 PHL-07 已实施, 严守
- ✅ S-3 质量工程化: V1.1 release 0 漂移 + 0 假装, 84 NEW tests pass + 跨平台打包 PASS, 严守
- ✅ O-1 安全优先: 24 LOCKED 入口签名 0 改, 仅扩 endpoint, 严守
- ✅ O-2 走在前人经验上: 借脑 0 装 8 借鉴源 + OpenCog 家族 6 子源, 严守
- ✅ O-3 干到底: V1.1 release 0 漂移 + 0 假装, 严守
- ✅ O-4 任何人都能接手: README + STRUCTURE + 8 硬墙 0 越界 verify, 严守
- ✅ O-5 不假装: PHL-07 V1.0 spec-only → V1.1 实施 关键诚实标, 严守
- ✅ 不要怕复杂度: V1.1 release 6 维度 470 min 是最强效果 + 最厉害工程, 维护交给未来高水平团队

---

## 10. 风险 + 决策原则 (per 决策 #33 + #73 + #74 + 用户记忆 #6-#7 + #10)

### 10.1 风险 (R131-8 调研阶段风险评估)

- **R1**: V1.1 release 6 维度 470 min 实施时间长 (估 1-2 天) — **缓解**: 蓝图就绪, 派 R131-2/4 + R132-2 + R133 多 sub-agent 错开时间盒
- **R2**: V1.1 release 实施 跟 V1.0 release 实战冲突 — **缓解**: V1.0 release 实战 = 主人起床后手跑 (估 8/11 06:00-08:00), V1.1 实施 = 估 2026-09-11, 错开
- **R3**: V1.1 release B1 24 LOCKED 入口签名 0 改 但需扩 endpoint + PHL-07 实施 — **缓解**: 仅扩 endpoint, 0 改原 24 LOCKED 入口签名, PHL-07 加 NEW 1 入口 = 25 LOCKED 总数 (per 决策 #22 §1.1-1.2)
- **R4**: V1.1 release 9 organ 真 sensor 接入 需后端 crate — **缓解**: 已有 core/src/organ.rs 1:1 镜像 (per R129-9 实施), Stage 4 D 蓝图就绪
- **R5**: V1.1 release WebSocket 长连接 稳定性 — **缓解**: 浏览器 native WebSocket, 0 装 socket.io (per 决策 #33 C2)
- **R6**: V1.1 release 跨 tab 持久化 浏览器差异 — **缓解**: 0 装, 浏览器原生 API (per 决策 #33 C2)
- **R7**: V2.0 release 重构 风险高 — **缓解**: 0 漂移前提下, 0 假装已重构, 1.0 → 2.0 留 6+ 月, semver major bump, Mavis 自决 (per 决策 #74 §2.3)
- **R8**: OpenCog AGPL-3.0 fork 决策 — **缓解**: 0 装 "已集成 OpenCog AtomSpace", 仅借脑 (per R130-6 §2.3)
- **R9**: 借脑 servers/superpowers 借鉴深度浅 — **缓解**: 0 借源码 0 装, 仅 1:1 翻译设计模式 (per 决策 #33 C2)
- **R10**: 8 硬墙 0 越界 verify 100% — **缓解**: 0 改 src, 0 改 Cargo.toml, 仅规划 doc, 0 触碰借鉴源码本身
- **R11**: 0 借脑 0 装 PASS 严守 — **缓解**: 0 装任何 lib, 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 设计模式
- **R12**: 0 主动 push 严守 — **缓解**: V1.0 release push 主人起床后手跑 (per 决策 #78)
- **R13**: 0 主动 IM 主人 (per gate-discipline) — **缓解**: 仅 done notification
- **R14**: 9 organ 永远循环 0 死亡 (per 用户记忆 #4) 严守 — **缓解**: ticker.js 100ms 周期 (per R129-9 §3.5), activity_pct 0-100, 0 用 health/sick/dying
- **R15**: 0 借脑 0 装 vs 不要怕复杂度哲学 冲突 — **缓解**: per 决策 #73 §3 0 装是技术哲学 (严守), 不要怕复杂度是工程哲学 (上限), 0 装是底线, 不要怕复杂度是上限
- **R16**: R131-8 60 min 时间盒紧 (9 优化方向 + V1.0/V1.1/V2.0 3 大方案) — **缓解**: 0 改 src, 仅规划 doc, 0 触碰借鉴源码, 60 min 估 12 报告节 (各 5 min) + 引用 R130-3/130-5/129-31/129-19 + 决策 #73/74/75

### 10.2 决策原则 (per 决策 #33 + #73 + #74 + 用户记忆 #2 + #6 + #7 + #10)

- ✅ **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- ✅ **0 改 src 严守** (per 任务 spec + 决策 #33 C2 + 决策 #74 §4.1 V1.0 release 0 改 R11 baseline)
- ✅ **0 改 Cargo.toml 严守** (per 任务 spec + 决策 #74 §1 B2 V1.0 release 1.2.0 严守)
- ✅ **0 主动 commit 严守** (per 决策 #33 §2.3 C1 + 决策 #74 §1)
- ✅ **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- ✅ **0 借脑 0 装 严守** (per 决策 #33 §2.3 C2): 0 装任何 lib, 借脑 0 借具体源码
- ✅ **0 越界 8 硬墙** (per 决策 #33 §2.3 + 决策 #74 §1): B1 24 LOCKED / B2 1.2.0 / A1 baseline / A3 13 键 / B3 30 维 / B4 6 重 v7 / B5 8 锚 / C1 0 commit / C2 0 装 / 0 push 全守
- ✅ **0 暴露 7 项 UI 哲学** (per 用户记忆 #3): 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM
- ✅ **9 organ 永远循环 0 死亡** (per 用户记忆 #4): ticker.js 100ms 周期, activity_pct 0-100, 0 用 health/sick/dying
- ✅ **9 organ 1 屏多卡 拟人化** (per 用户记忆 #5): 3x3 网格 + ECG + NN, 1 真相源 CrossNavStore, 5 nav 共享
- ✅ **0 主动 IM 主人** (per gate-discipline + 用户记忆 #10 + 决策 #61 §6): 仅 done notification
- ✅ **不重写 R129-9/19/31/R130-3** (per 任务 spec): R131-8 0 触碰 Stage 1-5 产物, 仅做优化 + V1.1/V2.0 方案
- ✅ **TUI 跟 Tauri 升级路径一致** (per 决策 #9 + 用户记忆 #8 + #9): TUI/Tauri 升级 1:1 翻译, 后端 0 改
- ✅ **B1 改写边界** (per 决策 #74 §2.2 V1.1 release Mavis 自决改, 前提: 更好的架构)
- ✅ **V2.0 release 全重评** (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)
- ✅ **8 哲学锚严守** (per 决策 #33 §2.3 B5 + 决策 #74 §1)
- ✅ **不要怕复杂度哲学落地** (per 决策 #73 §3 + 15-no-fear-complexity.md, 最强效果 + 最厉害工程)
- ✅ **决策日志写** (per 决策 #10 + 用户记忆 #10 + cron Section 6)
- ✅ **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64)
- ✅ **0 主动删** (per Safety policy + 决策 #44 + #60)

### 10.3 R131-8 报告本身 0 触碰主仓 verify

```bash
# 假设跑 (R131-8 实际 0 跑, 仅规划):
$ cd Apeireth-rust
$ git status --porcelain
# 仅显示:
# ?? reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md
# (0 触碰主仓 src/, 0 触碰 frontend/, 0 触碰 Cargo.toml)
```

**R131-8 报告 0 触碰 verify**:
- ✅ 0 改 src-tauri/ (Tauri 2.0 wrapper 0 改)
- ✅ 0 改 core/ (24 LOCKED 0 改)
- ✅ 0 改 主仓 src/ (workspace.version 1.2.0 0 改)
- ✅ 0 改 主仓 Cargo.toml (0 改 0 触碰)
- ✅ 0 借脑 0 装 (仅规划, 0 触碰借鉴源码本身)
- ✅ 0 主动 commit (整合 #5.3 reports/ 由 Mavis 拍板)
- ✅ 0 主动 push (V1.0 release 实战后)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)

---

## 11. refs

### 11.1 Tauri 5 阶段报告 (per 决策 #75 + R130-3 + R130-5)

- P11-1 R128 tauri-frontend-prototype-final-2026-08-10: Tauri 2.0 prototype 真实施 (72 tests)
  - `reports/agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md`
- P11-2 R128-2 tauri-frontend-scaffold-final-2026-08-10: Tauri 2.0 scaffold 深化 (111 tests, cargo build PASS + cargo tauri dev 跑通)
  - `reports/agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md`
- R129-9 Tauri Stage 2 深化 (2026-08-11 00:35): 5 phase 进度条 + 流式打字 + 9 健康环 + heart ECG + brain NN + 122 tests
  - `reports/agent-r129-9-tauri-stage-2-deepening-2026-08-11.md`
- R129-19 Tauri Stage 3 跨 nav 集成 (2026-08-11 00:34): 7 模块 J1-J7 + CrossNavStore 状态中枢 + 9 organ animator + 79 tests + 8 examples + 1 hub
  - `reports/agent-r129-19-tauri-stage-3-integration-2026-08-11.md`
- R129-31 Tauri Stage 4 实战规划 (2026-08-11 00:56): 4 维度 A 真后端 / B WebSocket / C 持久化 / D 真 sensor 蓝图 + 84 NEW tests 累计 163
  - `reports/agent-r129-31-tauri-stage-4-execution-2026-08-11.md`
- R130-3 Tauri Stage 5 集成深化 (2026-08-11 1:00): Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + Stage 6+ 路线 + V1.1 计划
  - `reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md`
- R130-5 V1.1 minor release 路线图 (2026-08-11 01:14): 6 大方向 (PHL-07 + 后端加固 + Tauri Stage 5+ + 形式化 Stage 5.5+ + ASI Stage 8+ + 借鉴源 12 源)
  - `reports/agent-r130-5-v1.1-minor-release-roadmap-2026-08-11.md`
- R130-6 Final Report 借鉴源码 12 源调研 (2026-08-11 01:14): 11 已有 + 1 新增 = OpenCog AGPL-3.0 fork 决策
  - `reports/agent-r130-6-borrowed-12-sources-research-2026-08-11.md`

### 11.2 决策 + 路线图 (per 决策 #33 + #71 + #73 + #74 + #75)

- 决策 #33: 8 硬墙 + 0 装 PASS 严守 + 0 主动 commit + 0 主动 push
- 决策 #48: 整合 #4 commit abf12243
- 决策 #55: 9 阶段路线图 + 24 LOCKED + 借鉴 ID 严格化
- 决策 #57: R128 阶段 B Tauri prototype 派活
- 决策 #58: R128-2 3 sub-agent (P11-2 scaffold 深化 spec)
- 决策 #61: 新会话接手 + R129 era 派活规划
- 决策 #62: 整合 #5 commit 拆 3 commit
- 决策 #64: 5 min tick cron 自动监督
- 决策 #71: 计划内任务完成自动接续 4 步 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施)
- 决策 #73: 主人 8/11 01:14 新决策 3 件套 (locked 全解锁 + 架构审视 + 不要怕复杂度)
  - `reports/decision-73-locked-unlocked-architecture-audit-philosophy-extension-2026-08-11.md`
- 决策 #74: 8 硬墙 B1 改写 (V1.0 release 0 改严守 + V1.1 release Mavis 自决改)
  - `reports/decision-74-8-hard-walls-b1-rewrite-v1-0-0-改-v1-1-自决-2026-08-11.md`
- 决策 #75: R131 era 第 2 批 6 sub + R132 era 计划 2 sub + R133 era 实施 3 sub 派活 11 sub 填到 16
  - `reports/decision-75-r131-r132-r133-batch-dispatch-11-sub-fill-16-2026-08-11.md`
- 决策 #78: 1.0 release tag v1.0.0 实战 (主人起床后手跑)
- 决策 #79: 整合 #6 commit 拍板 (Mavis 自决, 拆 3 commit)
- 决策 #80: 1.0 release 后路线图 (TUI + Tauri + ASI + 形式化 + V1.1/V1.2)
- 决策 #81: V1.1 minor release 计划 (估 2026-11, 1.0 release 后 ~3 个月)
- 决策 #82: V1.2 minor release 计划 (估 2027-02, V1.1 后 ~3 个月)
- 15-no-fear-complexity.md: 总工程哲学扩展 (R130 era 主人 8/11 01:14 拍板)
  - `docs/conventions/15-no-fear-complexity.md`

### 11.3 用户记忆 (跨 project 适用, per 决策 #10 + #33 §2.3)

- 用户记忆 #2: 让我做判断, 不机械问拍板
- 用户记忆 #3: 用户看结果不看哲学, 砍 7 项 UI 哲学
- 用户记忆 #4: AI 不会衰老病死, 9 organ 永远循环
- 用户记忆 #5: 信息密度高 = 拟人化 + 拟物化, 1 屏多卡
- 用户记忆 #6: 派 sub-agent 干, 但要驾驭团队不重复造轮子
- 用户记忆 #7: 推技术决策要守规范, 但要诚实
- 用户记忆 #8: 前端终极 = Tauri, TUI 是过渡
- 用户记忆 #9: TUI 升级节奏, 改瘦后暂告段落, 优先后端
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志

### 11.4 主人拍板 (per 决策 #10 + 用户记忆)

- 主人 8/4 23:33: "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备."
- 主人 8/4 23:55: "测一下先, 后续的 tui 升级计划沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui"
- 主人 8/10 16:27: "为了升级或更好, 要改动现有的 locked, 不必犹豫, 完全可以"
- 主人 8/10 16:31: "全部采纳, 全都能动, 需要具体确认的你自己确认就行, 你有最高权限"
- 主人 8/11 0:25: "全部你做主" (Mavis 升级决策权)
- 主人 8/11 0:34: "跑中 ≥ 16" (16 active 全 background 跑)
- 主人 8/11 0:43: 中断接手机制
- 主人 8/11 0:49: 编译产物清理决策矩阵
- 主人 8/11 0:54: Mavis 升级决策权 + 150 GB 强制清理
- 主人 8/11 0:57: 计划内任务完成自动接续 4 步 (调研 + 差距 + 计划 + 继续干)
- 主人 8/11 01:14: 拍板 3 件套 (locked 全解锁 + 架构审视 + 不要怕复杂度)
  - "事关工程类的，技术类的全早都给你解锁locked了"
  - "项目里要是有文档没提到这一点你就补充进去，让以后任何团队都能看到"
  - "所以有更好的架构需要用（或改变现有的）你就直接拍板就行了"
  - "我确实需要你注意一下现有的架构什么的，有没有需要优化升级的地方，有的你也就加入升级方案"
  - "总哲学除了思想文档的，我给你补充一点，就是不要怕复杂度爆炸或者维护复杂，我们只要最强的效果和最厉害的工程，因为自然会有高水平的团队来接手维护"

### 11.5 借鉴源码 (per 决策 #33 §2.3 C2 + 0 借脑 0 装)

- Tauri 2.0 (P11-1/2 真实施): tauri v2.11.5 + tauri-macros 2.6.3 + tauri-cli v2.11.4
- superpowers 234 (R125-14 + P5-1 真实施, 1.52MB): 5 阶段 DialoguePhase 1:1 翻译
- LangGraph 829 (R125-13 + P11-2 真实施, 13.29MB): stream_state_events 1:1 翻译
- modelcontextprotocol/servers 76d64c8 (R125-4 + 真实施, 1.40MB): MCP server 设计模式 1:1 翻译
- VCPChat (per 用户记忆, Downloads\VCPChat-main.zip): Electron 桌面 app 借鉴, chat-first 设计模式
- OpenCog 家族 6 子源 (per R130-6 调研, AGPL-3.0 fork 决策, 借脑 0 装): AtomSpace / cogutil / moses / pln (deprecated) / relex (deprecated) / CogPrime

### 11.6 项目结构 (per R131-8 verify)

```
Apeireth-rust/                                          # 主仓 (workspace.version 1.2.0 严守)
├── Cargo.toml                                          # workspace 1.2.0 0 改
├── Cargo.lock                                          # 0 改
├── src/                                                # 24 LOCKED 入口签名 0 改
├── tests/                                              # 跨模块守门 0 改
├── docs/                                               # 0 改 (per 决策 #62 5.2 准备)
│   └── conventions/15-no-fear-complexity.md           # 主人 8/11 01:14 拍板哲学扩展
├── frontend/                                           # Tauri 终极前端
│   └── tauri-prototype/                                # Tauri 2.0 桌面 app
│       ├── core/                                       # 122 tests pass (102 unit + 20 integration)
│       ├── src-tauri/                                  # Tauri 2.0 wrapper, 27 commands 拆 9 submod
│       ├── src/                                        # 0 装 vanilla JS, 67 文件
│       │   └── integration/                            # CrossNavStore + 7 模块 (J1-J7) + 9 organ animator + 79 tests + 8 examples + 1 hub
│       ├── docs/                                       # STRUCTURE.md 架构图
│       └── README.md                                   # 任何人接手指南
├── library/                                            # 借用源 (servers / superpowers / VCPChat / 等)
├── reports/                                            # 决策 + sub-agent 报告
│   ├── decision-71-r129-to-r130-auto-continuation-2026-08-11.md
│   ├── decision-72-r130-era-dispatch-r129-3-final-wait-2026-08-11.md
│   ├── decision-73-locked-unlocked-architecture-audit-philosophy-extension-2026-08-11.md
│   ├── decision-74-8-hard-walls-b1-rewrite-v1-0-0-改-v1-1-自决-2026-08-11.md
│   ├── decision-75-r131-r132-r133-batch-dispatch-11-sub-fill-16-2026-08-11.md
│   ├── agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md
│   ├── agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md
│   ├── agent-r129-9-tauri-stage-2-deepening-2026-08-11.md
│   ├── agent-r129-19-tauri-stage-3-integration-2026-08-11.md
│   ├── agent-r129-31-tauri-stage-4-execution-2026-08-11.md
│   ├── agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md
│   ├── agent-r130-5-v1.1-minor-release-roadmap-2026-08-11.md
│   ├── agent-r130-6-borrowed-12-sources-research-2026-08-11.md
│   └── agent-r131-8-tauri-integration-optimization-2026-08-11.md  ← 本报告
└── scripts/                                            # 1.0 release 实战 (R130-5 续)
```

---

**R131-8 done**: 60 min 时间盒内完成 9 优化方向详细分析 (三层架构 / 5 nav / 9 organ / Stage 5+ / servers 借脑 / superpowers 借脑 / Tauri 跨平台 / Tauri 性能 / V1.1 完整实施) + V1.0 release 0 改严守 (整合 #5.1 commit) + V1.1 release Tauri 完整实施 6 维度 470 min 方案 (R131-2/4 + R132-2 + R133 派活) + V2.0 release Tauri 重构方案 (8 硬墙全重评 + 8 哲学锚可重建 + Tauri 终极前端 + 设计团队到位). 0 改 src + 0 改 Cargo.toml + 0 主动 commit (整合 #5.3 reports/ 由 Mavis 拍板) + 0 主动 push (V1.0 release 实战后) + 0 主动 IM 主人 (per gate-discipline, 仅 done notification). 8 硬墙 0 越界 (B1 24 LOCKED V1.0 release 0 改严守 + V1.1 release Mavis 自决改 / B2 1.2.0 / A1 baseline / A3 13 键 / B3 30 维 / B4 6 重 v7 / B5 8 哲学锚 / C1 0 commit / C2 0 装 / 0 push 全守). 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 用户记忆 #3 砍 7 项 UI 哲学). 9 organ 永远循环 0 死亡 (per 用户记忆 #4). 不要怕复杂度哲学落地 (per 决策 #73 §3 + 15-no-fear-complexity.md, 最强效果 + 最厉害工程, 维护交给未来高水平团队). 报告路径 `reports/agent-r131-8-tauri-integration-optimization-2026-08-11.md`. 整合 #5.3 reports/ commit 由 Mavis 拍板.
