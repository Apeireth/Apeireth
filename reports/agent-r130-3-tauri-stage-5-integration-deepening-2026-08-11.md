# Agent R130-3 — Tauri Stage 5 集成深化 + Stage 6+ 路线 + V1.1 minor Tauri 计划 (planning doc, 2026-08-11)

**Date**: 2026-08-11 (R130 era 调研 60 min 时间盒, 8/11 1:00 派活 per 决策 #72 cron 5 min tick)
**Author**: Mavis sub-agent R130-3 (planning-only, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push)
**任务**: Tauri 终极前端 Stage 5 集成深化 + Stage 6+ 路线调研 + V1.1 minor release Tauri 计划
**派活依据**: 决策 #71 §2 cron Section 9 Step 2 R130 era 调研 6 sub-agent (R130-1 ~ R130-6) + 决策 #72 §2.1 R130-3 派活 spec
**R129 era 续**: R129-9 Stage 2 深化 (122 tests) + R129-19 Stage 3 跨 nav 集成 (79 tests + 8 examples) + R129-31 Stage 4 实战规划 (4 维度 A/B/C/D 蓝图, 84 NEW tests 累计 163)
**报告路径**: `reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md`
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守)
**整合 #5 commit**: 估 8/11 00:38 (per 决策 #62 拆 3 commit: 5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决拍板
**整合 #6 commit**: 估 1.0 release tag 后 (per 决策 #62 + 决策 #79), Mavis 自决拍板

---

## 0. 一句话

**Tauri 终极前端 Stage 5 集成深化规划 (R130-3, 60 min, planning doc only) — Stage 5 = Tauri 2.0 完整集成 (tauri 2.11+ 跨平台打包) + 5 nav 完整 (TUI 1:1 镜像) + 9 organ 拟人化 final (1 真相源 + 5 nav 共享 + 永远循环 0 死亡 + 1 屏多卡) + 砍 7 项 UI 哲学 100% + 后端全 API 表面同步 (TUI/Tauri 共用, per 用户记忆 #8 瘦客户端). Stage 6+ 路线 = Stage 6 后端 API 集成 (apeireth-api HTTP + WebSocket 真接通) + Stage 7 实际部署 (Tauri 跨平台打包 + 1.0 release tag + GitHub release) + Stage 8 用户测试 (V1.0.0 release 后, 真用户验收 + 反馈). V1.1 minor release Tauri 计划 (估 2026-11, per 决策 #81): Tauri Stage 4 实战 (R131-4 派) + TUI 升级阶段 2 (R131-3) + ASI Python Stage 7 治理 (R131-5) + 形式化证明器 Stage 5.4 实战 (R131-6) + 借鉴 Stage 4-6 集成 (R131-7). 借鉴 Tauri 2.0 (P11-1/2 真实施, tauri v2.11.5 + tauri-macros 2.6.3) + superpowers 234 (executing-plans 5 阶段 DialoguePhase 1:1 翻译) + LangGraph 829 (stream_state_events 1:1 翻译) + VCPChat (Electron 桌面 app 借鉴, chat-first 设计模式). 0 改 src + 0 改 Cargo.toml + 0 主动 commit (Mavis 整合 #5/#6/#7 拍板) + 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑) + 0 主动 IM 主人 (per gate-discipline, 仅 done notification). 8 硬墙 0 越界 (B1 24 LOCKED / B2 1.2.0 / A1 baseline / B3 30 维 / B4 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 / C3 升 v7 / 0 push 全守). 砍 7 项 UI 哲学 (per 用户记忆 #3 严守). 9 organ 永远循环 0 死亡 (per 用户记忆 #4 严守). 0 借脑 0 装 PASS 严守 (per 决策 #33 §2.3 C2).**

---

## 1. 上下文回顾: R129 era Tauri 已做产物 (per 决策 #72 + R129-9/19/31 报告)

### 1.1 Tauri 5 阶段已 done 产物总览 (per R129-9 Stage 2 + R129-19 Stage 3 + R129-31 Stage 4 实战规划)

| Stage | 时间 | 派活 | 产物 | Tests | 0 越界 | 报告 |
|------|------|------|------|------:|------|------|
| **Stage 1 prototype** | 8/10 21:50 | P11-1 | Tauri 2.0 骨架 + 5 nav stub + 9 organ stub + 72 tests | 72 | ✅ | `agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md` |
| **Stage 2 scaffold** | 8/10 22:35 | P11-2 | cargo build PASS + cargo tauri dev 跑通 + 22 commands 拆 8 submod + 111 tests | 111 | ✅ | `agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md` |
| **Stage 2 deepening** | 8/11 00:35 | R129-9 | 5 phase 进度条 + 流式打字 + 9 健康环 + heart ECG + brain NN + 122 tests | 122 | ✅ | `agent-r129-9-tauri-stage-2-deepening-2026-08-11.md` |
| **Stage 3 跨 nav 集成** | 8/11 00:34 | R129-19 | 7 模块 J1-J7 + CrossNavStore 状态中枢 + 9 organ animator + 79 tests + 8 examples + 1 hub | 79 + 122 = 201 | ✅ | `agent-r129-19-tauri-stage-3-integration-2026-08-11.md` |
| **Stage 4 实战规划** | 8/11 00:56 | R129-31 | 4 维度 A 真后端 / B WebSocket / C 持久化 / D 真 sensor 蓝图 + 84 NEW tests 累计 163 | 0 NEW (规划) | ✅ | `agent-r129-31-tauri-stage-4-execution-2026-08-11.md` |
| **Stage 5 集成深化** | 8/11 1:00 派 | **R130-3 (本)** | Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + Stage 6+ 路线 + V1.1 计划 | 0 NEW (规划) | ✅ | **本报告** |

**Stage 1-4 累计 201 tests pass** (Stage 1 72 + Stage 2 50 = 122 [Stage 2 重复 11 单元 test]) + Stage 3 集成层 79 = 201 tests, 0 装 PASS 严守 + 8 硬墙 0 越界.

**Stage 4-5 关键差异 (per 决策 #71 §2 R130-3 派活 spec + R129-31 §2)**:
- Stage 4 (R129-31 规划) = 4 维度实战化蓝图 (A 真后端接通 / B WebSocket 流式 / C 跨 tab 持久化 / D 9 organ 真 sensor)
- Stage 5 (R130-3 规划) = 5 nav 完整 + 9 organ 拟人化 final + Tauri 2.0 完整集成 (tauri 2.11+ 跨平台打包) + 后端全 API 表面同步

### 1.2 R129-19 Stage 3 已 done 产物回顾 (per R129-19 报告 §2.1 + §3)

```
frontend/tauri-prototype/src/integration/                  # 总 32 文件 / ~128 KB
├── README.md (10 KB)                                     # 架构图 + 10 节
├── store.js (10 KB)                                      # CrossNavStore 状态中枢 (1 真相源, 14 EVT + 12 mutators)
├── index.js (3 KB)                                       # 1 行启动 (bootstrap 7 + 1 = 8 模块)
├── status_chat.js (5 KB)                                 # J1 status ↔ chat
├── status_history.js (3 KB)                              # J2 status ↔ history
├── status_tools.js (4 KB)                                # J3 status ↔ tools
├── chat_history.js (3 KB)                                # J4 chat ↔ history
├── chat_tools.js (4 KB)                                  # J5 chat ↔ tools
├── history_tools.js (4 KB)                               # J6 history ↔ tools
├── settings_global.js (4 KB)                             # J7 settings → 5 nav 全局
├── organ_animator.js (9 KB)                              # 9 organ 拟人化深化 (Stage 3)
├── __tests__/                                            # 8 test files + runner
│   ├── test-runner.js (4 KB)                             # 极简 test runner (0 装, 浏览器+Node)
│   ├── store.test.js (8 KB, 22 cases)                    # 22 cases pass
│   ├── status_chat.test.js (3 KB, 6 cases)               # 6 cases pass
│   ├── status_history.test.js (3 KB, 7 cases)            # 7 cases pass
│   ├── status_tools.test.js (3 KB, 7 cases)              # 7 cases pass
│   ├── chat_history.test.js (3 KB, 7 cases)              # 7 cases pass
│   ├── chat_tools.test.js (4 KB, 9 cases)                # 9 cases pass
│   ├── history_tools.test.js (3 KB, 8 cases)             # 8 cases pass
│   └── settings_global.test.js (5 KB, 13 cases)          # 13 cases pass
└── examples/                                             # 8 HTML examples + 1 hub
    ├── status-chat.html                                  # J1
    ├── status-history.html                               # J2
    ├── status-tools.html                                 # J3
    ├── chat-history.html                                 # J4
    ├── chat-tools.html                                   # J5
    ├── history-tools.html                                # J6
    ├── settings-global.html                              # J7
    ├── organ-animator.html                               # 9 organ 拟人化
    └── stage3-hub.html                                   # 7 集成 + 9 organ 综合 hub
```

**Stage 3 总结 (per R129-19 报告 §0)**:
- 7 集成模块 (J1-J7) + 1 CrossNavStore 状态中枢 (pub/sub, 14 EVT + 12 mutators + 5 nav 状态 + 9 organ 活动)
- 9 organ 拟人化深化 (organ_animator.js 9 KB, 5 helper: renderChatHeaderOrgans / renderToolsHeaderOrgan / renderHistoryHeaderOrgans / renderSettingsHeaderOrgan / getOrganHealthSummary)
- 79 集成 test cases (pass, per `node run-all.js` 跑通)
- 8 examples + 1 hub (stage3-hub.html)
- cargo build PASS (3.96s) + core lib 122 tests pass (102 unit + 20 integration, 0.01s)
- 0 主动 commit (写到主仓 0 git add) + 0 主动 push 严守

### 1.3 R129-31 Stage 4 4 维度蓝图回顾 (per R129-31 报告 §2)

| 维度 | ID | 实施蓝图 | NEW tests | 借鉴 |
|------|----|---------|----------:|------|
| **A 真后端接通** | A1-A6 | tauriInvoke 主路径, mock 仅 dev mode fallback (CrossNavStore 7 模块 + 9 organ animator 调 tauriInvoke) | 30 (A1-A6 各 5) | Tauri 2.0 + langgraph 829 |
| **B WebSocket 流式** | B1-B4 | 流式打字 (Stage 2 R129-9 字符级 50ms/字) → 真 WebSocket chunk append (browser native, 0 装) | 20 (B1-B4 各 5) | langgraph 829 stream_state_events |
| **C 跨 tab 持久化** | C1-C4 | settings/theme/font/layout 跨 tab 同步 (localStorage + BroadcastChannel, browser native) | 20 (C1-C4 各 5) | 浏览器原生 API |
| **D 9 organ 真 sensor** | D1-D9 + D-统一 | 9 organ 真状态接入 (heart ECG / brain NN / hand 待办 / eye 观察 / ear 聆听 / memory 沉淀 / voice 流速 / body uptime / mind 思考) | 14 (D1-D9 + D-统一) | core/src/organ.rs + TUI 6 工具 endpoint |
| **总** | | | **84 NEW** | 集成层累计 79 + 84 = 163 tests |

**HTTP 路由** (per R129-31 §2.2):
- `GET  /v1/organs` → 9 organ + activities
- `POST /v1/chat/messages` → user 消息 + AI 回复
- `GET  /v1/chat/session/{id}` → 5 DialoguePhase
- `GET  /v1/history` → history entries
- `GET  /v1/tools/results` → 6 tool results
- `GET  /v1/settings` → 14 settings
- `PATCH /v1/settings/{key}` → 改 1 setting

**WebSocket 协议** (per R129-31 §2.3 + langgraph 829 stream_state_events 1:1 翻译):
```
client → server: {"type": "send_message", "session_id": "...", "content": "..."}
server → client: {"type": "phase_change", "phase": "Streaming"}
server → client: {"type": "stream_chunk", "content": "..."}  // 累加到 AI 气泡
server → client: {"type": "stream_end", "full_content": "..."}  // 写入 history
server → client: {"type": "phase_change", "phase": "Awaiting"}
```

### 1.4 R130-3 派活 spec 关键 (per 决策 #72 §2.1 + 用户最新消息)

**R130-3 任务 spec 三大调研方向**:
1. **Stage 5 集成深化方案** (Tauri 2.0 + 5 nav 完整 + 9 organ 拟人化深化)
2. **Stage 6+ 路线图 spec** (V1.1 release 后, Stage 6/7/8 spec)
3. **V1.1 minor release Tauri 计划**

**关键约束** (per 决策 #71 + #72 + R130-3 任务 spec):
- 严格不写代码 (per 决策 #33 + #60 + 决策 #71 调研阶段)
- 调研 + 报告, 不改 src/
- 时间盒 60 min
- 0 主动 commit, 0 主动 push, 0 主动 IM 主人 (per gate-discipline)
- 8 硬墙 0 越界
- 0 借脑 0 装 PASS 严守

---

## 2. Stage 5 集成深化方案 (per 用户记忆 #3-#5 + #8 + 决策 #33 + #58)

### 2.1 Stage 5 战略定位 (per 用户记忆 #8 终极 = Tauri + 决策 #9 + 主人 8/4 23:33)

**Stage 5 终极前端 = Tauri 2.0 完整集成 (tauri 2.11+ 跨平台打包) + 5 nav 完整 (TUI 1:1 镜像) + 9 organ 拟人化 final (1 真相源 + 5 nav 共享 + 永远循环 0 死亡) + 砍 7 项 UI 哲学 100% + 后端全 API 表面同步 (TUI/Tauri 共用, 瘦客户端)**

**Stage 5 跟 Stage 4 关系**:
- Stage 4 (R129-31 规划): 4 维度实战化蓝图 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor)
- Stage 5 (R130-3 规划): 终极前端, Tauri 2.0 全面 + 5 nav 真打通 + 9 organ final + 设计团队到位

**Stage 5 跟 TUI 关系** (per 决策 #9 + 用户记忆 #8 瘦客户端):
- TUI 跟 Tauri 共享后端 API 表面 (apeireth-api)
- TUI 升级 → Tauri 升级 1:1 翻译
- 后端 0 改 (TUI/Tauri 都是 thin client)

### 2.2 Stage 5 4 大战略目标

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

### 2.3 Stage 5 5 nav 完整 (per 用户记忆 #3 严守 + TUI nav/mod.rs 1:1)

| Nav | TUI (现有) | Stage 5 完整 (R130-3 规划) | 借鉴 |
|-----|-----------|---------------------------|------|
| 0 状态 (Status) | nav/mod.rs 0 | Stage 5 = 9 organ final 1 屏多卡 (3x3 网格) + ECG + NN + 关键数字一眼看完 (per 用户记忆 #5) | TUI + Stage 2 visualization + Stage 3 J1 |
| 1 主对话 (Dialogue) | pages/dialogue.rs | Stage 5 = 真 LLM stream + WebSocket (Stage 4 B 续) + 5 phase 进度条 + 流式打字 + 0 暴露守门 | TUI + superpowers 234 + langgraph 829 + Stage 4 B |
| 2 历史 (History) | pages/history.rs | Stage 5 = 后端真 history (Stage 4 A 续) + SVG 时间线 + 按 episode 过滤 | TUI + Stage 2 timeline.js + Stage 4 A3 |
| 3 设置 (Settings) | pages/settings.rs | Stage 5 = 14 settings 真接通 (Stage 4 A5 续) + 5+5+4 分 section + 鉴权 UI + sub-control 编辑 | TUI + Stage 2 settings-editor.js + Stage 4 A5 |
| 4 工具结果 (Tools) | pages/tools.rs | Stage 5 = 6 工具真接通 (Stage 4 A4 续) + tool_call deep-link chat + 颜色编码 + 弹窗 | TUI + Stage 3 J5 + Stage 4 A4 |

**5 nav 严守 (per 用户记忆 #3)**:
- ✅ 0 加 nav (主对话是核心, 0 改 5 nav 顺序)
- ✅ 0 砍 nav (5 nav = TUI 1:1 镜像)
- ✅ 0 改 nav id (NAV_ID 0-4 严守)
- ✅ 0 加 nav 子菜单 (5 nav = 5 nav, 0 改 1:1)

**Stage 5 5 nav 实施蓝图 (per R130-3 spec + Stage 4 续)**:
- **5 nav 真打通**: CrossNavStore 状态中枢 (Stage 3 已实) + 集成层 7 模块 (J1-J7, Stage 3 已实) + tauriInvoke 主路径 (Stage 4 A 实施)
- **5 nav 跟 TUI 1:1**: nav/mod.rs (5 nav) → frontend/src/integration/ CrossNavStore.NAV_ID (5 nav 1:1 严守)
- **5 nav 严守 1 真相源**: CrossNavStore.NAV_ID 5 nav 严守, 0 加 0 砍 0 改 (per 用户记忆 #3)

### 2.4 Stage 5 9 organ 拟人化 final 1 屏多卡 (per 用户记忆 #5 + #4 + 决策 #5)

**Stage 5 9 organ 1 真相源 + 1 屏多卡 + 拟人化 + 拟物化 + 0 死亡循环**:

| ID | 英文 | 中文 | 拟物化 | Stage 5 final 深化 | 颜色 | 数据源 |
|---:|------|------|--------|-------------------|------|--------|
| 0 | heart | 心 | 跳动着 | ECG P-QRS-T (60 采样) + 实时 BPM | #ef4444 (红) | 真 sensor (Stage 4 D1) |
| 1 | brain | 脑 | 运转中 | 神经网络 9 节点 + 8 中心边 + 8 围圈边 | #a855f7 (紫) | 真 sensor (Stage 4 D2) |
| 2 | hand | 手 | 待命 | 待办工具数 + 成功率 + 0 假装 | #f59e0b (橙) | 真 sensor (Stage 4 D3) |
| 3 | eye | 眼 | 观察中 | history 新条目数 + 观察频率 | #3b82f6 (蓝) | 真 sensor (Stage 4 D4) |
| 4 | ear | 耳 | 聆听中 | chat 输入频率 + 0 假装 | #06b6d4 (青) | 真 sensor (Stage 4 D5) |
| 5 | memory | 记忆 | 沉淀中 | history 过滤数 + 沉淀速度 | #8b5cf6 (紫蓝) | 真 sensor (Stage 4 D6) |
| 6 | voice | 声 | 表达中 | stream chunk/s + 表达时长 | #22c55e (绿) | 真 sensor (Stage 4 D7) |
| 7 | body | 体 | 运行中 | 系统 uptime + theme 切换计数 | #64748b (灰) | 真 sensor (Stage 4 D8) |
| 8 | mind | 意 | 思考中 | thinking 阶段 (4 ThinkingPhase) | #ec4899 (粉) | 真 sensor (Stage 4 D9) |

**Stage 5 1 屏多卡布局 (per 用户记忆 #5 信息密度高 = 拟人化 + 拟物化)**:

```
+--- 状态主页 Stage 5 final 1 屏多卡 ---+
|                                       |
|  9 organ 卡片 (3x3 网格)              |  ← row 1: heart + brain + hand
|  +-----+ +-----+ +-----+              |     (ASCII + 中文 + 拟物化)
|  | ♥   | | B   | | H   |              |     + 健康环 + 关键数字
|  | 心  | | 脑  | | 手  |              |
|  |跳动 | |运转 | |待命 |              |
|  | 85% | | 72% | | 95% |              |
|  +-----+ +-----+ +-----+              |
|                                       |
|  +-----+ +-----+ +-----+              |  ← row 2: eye + ear + memory
|  | EYE | | EAR | | MEM |              |
|  | 眼  | | 耳  | |记忆 |              |
|  |观察 | |聆听 | |沉淀 |              |
|  | 60% | | 88% | | 75% |              |
|  +-----+ +-----+ +-----+              |
|                                       |
|  +-----+ +-----+ +-----+              |  ← row 3: voice + body + mind
|  | V   | | BDY | | MND |              |
|  | 声  | | 体  | | 意  |              |
|  |表达 | |运行 | |思考 |              |
|  | 40% | | 99% | | 68% |              |
|  +-----+ +-----+ +-----+              |
|                                       |
|  heart ECG 240x60 走纸 (拟人化)        |  ← 1 屏心电图
|  brain NN 200x200 9 节点 (拟人化)     |  ← 1 屏神经网络
|                                       |
+---------------------------------------+
```

**Stage 5 1 真相源** (per 决策 #58 §0 + Stage 3 严守):
- CrossNavStore.organ_activities 9 organ 1 真相源 (Stage 3 已实)
- 5 nav 共享 (chat 头 2 / tools 头 1 / history 头 2 / settings 头 1)
- WebSocket 推送 (Stage 4 B 维度) → CrossNavStore 实时更新 → 5 nav 同步

**0 死亡循环 (per 用户记忆 #4 AI 不会衰老病死)**:
- ✅ 9 organ 活跃度 0-100, 永远循环
- ✅ 0 显示 "已死亡 / 老化 / 终止"
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑 (ticker.js, 100ms 周期, per R129-9 §3.5)

### 2.5 Stage 5 Tauri 2.0 完整集成 (per 决策 #33 §2.3 + P11-1/2 baseline)

**Tauri 2.0 完整集成要素 (per 决策 #33 §2.3 + R129-31 §5.1)**:

| 集成项 | 状态 | Stage 5 完整 (R130-3 规划) |
|--------|------|---------------------------|
| **Tauri 2.0 invoke wrapper** | ✅ Stage 3 已实 | CrossNavStore 调 tauriInvoke, 失败回 mock (Stage 3) → tauriInvoke 主路径 (Stage 4 A) |
| **Tauri 2.0 command 注册** | ✅ P11-2 22 commands + R129-9 +5 commands visualization | Stage 5 = 总 27+ commands, 8 submod |
| **Tauri 2.0 跨平台打包** | ⏳ 1.0 release 实战 | Stage 5 = Windows + macOS + Linux 跨平台, 5 nav 窗口 |
| **Tauri 2.0 WebView** | ✅ 系统 WebView | Stage 5 = WebView2 (Windows) / WKWebView (macOS) / WebKitGTK (Linux) |
| **Tauri 2.0 IPC** | ✅ Tauri 2.0 invoke + emit | Stage 5 = cross-window emit + 9 organ 实时推送 |
| **Tauri 2.0 配置** | ✅ tauri.conf.json | Stage 5 = 5 nav 窗口 + 5 icons + capabilities/default.json |

**Tauri 2.0 跨平台打包清单 (Stage 5 实施)**:
- **Windows**: MSI / NSIS (per Tauri 2.0 bundler 官方支持)
- **macOS**: DMG / APP (per Tauri 2.0 bundler 官方支持)
- **Linux**: deb / AppImage (per Tauri 2.0 bundler 官方支持)
- **跨平台 cargo tauri build**: 1 条命令 3 平台打包
- **自动更新 (Tauri 2.0 updater)**: V1.0.0 → V1.0.1 → V1.1.0 自动推送

**Tauri 2.0 实施严守 (per 决策 #33 §2.3)**:
- ❌ 0 改 src-tauri/Cargo.toml (0 改 0.1.0)
- ❌ 0 改 core/Cargo.toml (0 改 0.1.0)
- ❌ 0 装 npm / yarn / pnpm (0 build step)
- ❌ 0 装 webpack / vite / rollup (0 build step)
- ✅ 0 装, vanilla JS + Tauri 2.0 native

### 2.6 Stage 5 后端全 API 表面同步 (per 决策 #9 TUI 升级路径一致)

**Stage 5 = TUI 跟 Tauri 升级路径一致, 瘦客户端 (per 用户记忆 #8 + 决策 #9)**:
- TUI 跟 Tauri 共享后端 API 表面 (apeireth-api)
- TUI 升级 → Tauri 升级 1:1 翻译
- 后端 0 改 (TUI/Tauri 都是 thin client)

**Stage 5 跟 TUI 1:1 镜像表 (per R129-31 §3.5 + 决策 #9)**:

| TUI 模块 | Tauri Stage 5 镜像 | 后端 API |
|---------|-------------------|---------|
| nav/mod.rs (5 nav) | frontend/src/integration/ CrossNavStore.NAV_ID | 0 改 nav |
| pages/dialogue.rs (主对话) | dialogue-stream.js + chat_history.js (J4) | POST /v1/chat/messages + WS /v1/chat/stream |
| pages/history.rs (历史) | timeline.js + status_history.js (J2) | GET /v1/history |
| pages/settings.rs (设置) | settings-editor.js + settings_global.js (J7) | GET/PATCH /v1/settings |
| pages/tools.rs (工具结果) | chat_tools.js (J5) + history_tools.js (J6) | GET /v1/tools/results |
| organ/mod.rs (9 organ) | organ_animator.js + CrossNavStore.organ_activities | GET /v1/organs + WS push |

**TUI → Tauri 升级路径** (per 决策 #9 + 用户记忆 #8 + 用户记忆 #9 8/4 23:55):
- TUI 改瘦后暂告段落 (per 用户记忆 #9 8/4 23:55)
- 优先后端 (per 主人 8/4 23:55)
- TUI 升级路线图 (per R129-15 沉淀) → Tauri Stage 5 1:1 翻译
- 后端 API 表面 0 改 (TUI/Tauri 共用)

### 2.7 Stage 5 砍 7 项 UI 哲学 100% (per 用户记忆 #3 + 决策 #33)

**Stage 5 严守 0 暴露 7 项** (per 用户记忆 #3 严守):

| 砍项 | Stage 5 实施 | 验证 |
|------|-------------|------|
| 守门 (6 重 v7) | 0 暴露, CrossNavStore 0 emit 守门事件, store.getState() 0 触碰 | per 用户记忆 #3 严守 |
| 电子环 (0 装) | 0 暴露, 9 organ 健康环是 organ 活跃度, 跟电子环不同 | per 用户记忆 #3 |
| 工具调用过程 | 0 暴露, J5 recordChatToolCall 0 暴露 process, 只暴露 result | per 用户记忆 #3 |
| 哲学锚 (8 锚) | 0 暴露, CrossNavStore.EVT 0 含哲学锚, B5 硬墙严守 | per 用户记忆 #3 + B5 硬墙 |
| 内部机制 (24 LOCKED) | 0 暴露, 0 显示 24 LOCKED fn / V0.5 30 维 / 13 键 verdict | per 用户记忆 #3 |
| AI 衰老病死 | 0 显示, 用 "活跃度" (active/idle/dormant) 非 "健康度" (healthy/sick) | per 用户记忆 #4 |
| 0 主动 IM 主人 | 0 主动 IM, 仅 done notification | per gate-discipline |

**Stage 5 只暴露 (per 用户记忆 #3 用户看结果不看哲学)**:
- ✅ 状态 (status): 9 organ 1 屏多卡 + ECG + NN
- ✅ 主对话 (dialogue): user 气泡 + AI 消息 + 5 phase 进度条 + 流式打字
- ✅ 历史 (history): 3 kind (会话/消息/工具调用) + SVG 时间线
- ✅ 设置 (settings): 14 项分 3 section (5 鉴权 + 5 Provider + 4 SDK)
- ✅ 工具结果 (tools): 6 工具 card + 颜色编码 + 弹窗

### 2.8 Stage 5 实施清单 (R131+ 派, 估 60-90 min 时间盒)

| Step | 内容 | 估计 | 状态 |
|------|------|-----:|------|
| 1 | Tauri 2.0 跨平台打包配置 (tauri.conf.json + 3 平台 bundle) | 15 min | R131+ 派 |
| 2 | 5 nav 完整集成 (Stage 4 A 续 + 验证 5 nav 真打通) | 20 min | R131+ 派 |
| 3 | 9 organ final 1 屏多卡 (Stage 4 D 续 + 验证 1 真相源 5 nav 共享) | 20 min | R131+ 派 |
| 4 | 砍 7 项 UI 哲学 100% verify | 5 min | R131+ 派 |
| 5 | cargo tauri build 跨平台 + cargo test 0 越界 verify | 15 min | R131+ 派 |
| 6 | 写报告 (Stage 5 实战) | 10 min | R131+ 派 |
| **总** | | **90 min** | **R131+ 派** |

**0 改严守** (per 决策 #33 §2.3 + 决策 #58):
- 0 改 core/src/ (24 LOCKED 入口签名 0 改)
- 0 改 src-tauri/Cargo.toml (Tauri 2.0 wrapper 0 改 0.1.0)
- 0 改 主仓 src/ (workspace.version 1.2.0 0 改)
- 0 借脑 0 装 (per 决策 #33 §2.3 C2)
- 0 主动 commit (写到主仓 0 git add, 整合 #6/#7 commit 由 Mavis 拍板)
- 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑)

---

## 3. Stage 6+ 路线图 spec (per 决策 #71 §2 R130-3 任务 spec + 决策 #80 路线图)

### 3.1 Stage 6: 后端 API 集成 (per 决策 #71 + Stage 4 续, 估 R132-2 派)

**Stage 6 = 后端 API 集成 (apeireth-api HTTP + WebSocket 真接通), 0 装严守**

**Stage 6 战略定位**:
- 1.0 release tag 后立即启动 (per 决策 #80 路线图)
- 估 R132-2 派 (per 决策 #71 §2.4 1.0 release 后路线图详细)
- Stage 4 4 维度实战化的最终落地 (per R129-31 §2 + R130-3 §2)

**Stage 6 5 实施维度 (per 决策 #80 + Stage 4 续)**:

| 维度 | Stage 4 蓝图 | Stage 6 实战 | 借鉴 | 状态 |
|------|------------|------------|------|------|
| **A 真后端接通** | tauriInvoke 主路径 + mock fallback | 6 模块真接通 + 30 tests pass (R129-31 §2.2) | Tauri 2.0 + langgraph 829 | Stage 4 → Stage 6 |
| **B WebSocket 流式** | browser native WebSocket + chunk append | 4 模块真接通 + 20 tests pass (R129-31 §2.3) | langgraph 829 stream_state_events | Stage 4 → Stage 6 |
| **C 跨 tab 持久化** | localStorage + BroadcastChannel | 4 模块真接通 + 20 tests pass (R129-31 §2.4) | 浏览器原生 API | Stage 4 → Stage 6 |
| **D 9 organ 真 sensor** | 9 organ 真状态接入 | 9 organ + 1 unified + 14 tests pass (R129-31 §2.5) | core/src/organ.rs | Stage 4 → Stage 6 |
| **E 后端 server 端** | (蓝图外) | apeireth-api Rust server 真接通 + 7 GET + 1 POST + 1 PATCH + 1 WS endpoint | Tauri 2.0 + hyper / axum | Stage 6 NEW |
| **总** | | **84 + E 30 = 114 NEW tests, 集成层 79 + 114 = 193** | | **R132-2 派** |

**Stage 6 后端 server 端 (E 维度, per 决策 #80 + 1.0 release 续)**:
- **server 端 crates**: 已有 apeireth-api (per 主仓 src/) + 7 GET + 1 POST + 1 PATCH + 1 WS endpoint
- **server 端框架**: hyper / axum (per 主仓 24 LOCKED, 0 改, 仅扩展 endpoints)
- **server 端 24 LOCKED**: 0 改 24 LOCKED crate 入口签名 (per B1 硬墙)
- **server 端 0 装**: 用现有 24 LOCKED, 0 装新 web framework (per 决策 #33 C2)

**Stage 6 风险 + 决策原则**:
- **R1**: apeireth-api server 端 7 endpoint 实施时间长 — **缓解**: 估 90 min 时间盒, 7 endpoint 各 10-15 min
- **R2**: WebSocket 长连接 稳定性 — **缓解**: 浏览器 native WebSocket, 0 装 socket.io
- **R3**: 跨 tab 持久化 浏览器差异 — **缓解**: 0 装, 浏览器原生 API
- **R4**: 9 organ 真 sensor 接入 需后端 crate — **缓解**: 已有 core/src/organ.rs 真实施 (per P11-1/2 + R129-9), 1:1 镜像
- ✅ **0 改 src 严守**: 仅 server 端扩 endpoint, 0 改 24 LOCKED
- ✅ **0 装 PASS 严守**: server 端用现有 24 LOCKED, 0 装新 framework

### 3.2 Stage 7: 实际部署 (per 决策 #80, 估 R133 派)

**Stage 7 = Tauri 跨平台打包 + 1.0 release tag + GitHub release + 自动更新**

**Stage 7 4 实施维度 (per 决策 #80 + 1.0 release 实战)**:

| 维度 | 实施 | 估计 | 状态 |
|------|------|-----:|------|
| **A Tauri 跨平台打包** | cargo tauri build 3 平台 (Windows + macOS + Linux) | 30 min | R133 派 |
| **B 1.0 release tag** | git tag v1.0.0 + gh release create | 10 min | R133 派 (主人起床后手跑) |
| **C GitHub release** | gh release create + upload 3 平台 binary + 5 icons + CHANGELOG.md | 15 min | R133 派 |
| **D Tauri 2.0 updater** | Tauri 2.0 updater 配置 + V1.0.0 → V1.0.1 → V1.1.0 自动推送 | 20 min | R133 派 |
| **总** | | **75 min** | **R133 派** |

**Stage 7 1.0 release 实战清单 (per 主人 8/11 0:43 + 决策 #78 1.0 release 实战)**:
1. ✅ 8 步 verify (R129-3 done 0 报告, 估 01:05 拍板)
2. ✅ 整合 #5 commit (5.1 src/ + 5.2 docs/ + 5.3 reports/, Mavis 拍板)
3. ✅ cargo tauri build 3 平台 (Stage 7 A)
4. ✅ git push (主人手跑, 配 GitHub remote)
5. ✅ git tag v1.0.0 + gh release create (Stage 7 B + C)
6. ✅ Tauri 2.0 updater 配置 (Stage 7 D)
7. ✅ 1.0 release 实战 done notification (per gate-discipline)

**Stage 7 风险 + 决策原则**:
- **R1**: 3 平台打包 CI 资源 — **缓解**: 本地 + GitHub Actions (per 主人 8/11 0:43)
- **R2**: 1.0 release tag 拍板权 — **缓解**: 主人起床后手跑 (per 决策 #78)
- **R3**: Tauri 2.0 updater 跨平台差异 — **缓解**: Tauri 2.0 官方支持, 0 装
- **R4**: V1.0.0 → V1.0.1 → V1.1.0 自动推送 — **缓解**: 0 装, Tauri 2.0 native updater
- ✅ **0 主动 push 严守**: 主人起床后手跑 (per 决策 #33 C1)
- ✅ **0 主动 IM 主人**: 仅 done notification (per gate-discipline)

### 3.3 Stage 8: 用户测试 (per 决策 #80 + 1.0 release 后)

**Stage 8 = V1.0.0 release 后, 真用户验收 + 反馈 + 1.0.1 patch**

**Stage 8 5 实施维度 (per 决策 #80 + 主人 8/4 23:33 团队就位)**:

| 维度 | 实施 | 估计 | 状态 |
|------|------|-----:|------|
| **A 主人手跑** | 主人起床后手跑 1.0 release 实战 (per 主人 8/11 0:43 + 0:54) | 60 min | 主人手跑 |
| **B 真用户验收** | 1-3 真用户体验 Tauri app + 反馈 (1.0 release 后 1 周内) | 7 天 | 主人组织 |
| **C 反馈收集** | GitHub Issues + 主人 IM 反馈 → 整理成 R132+ 任务 | 30 min | 主人组织 |
| **D V1.0.1 patch** | 修 1.0 release 已知小 bug + 1.0.1 patch release | 60 min | R132+ 派 |
| **E V1.1 minor 规划** | 反馈 → V1.1 minor release 计划 (per 决策 #81) | 30 min | R132-2 派 |
| **总** | | **180 min + 7 天** | **R132+ 派** |

**Stage 8 团队就位条件** (per 主人 8/4 23:33 + 决策 #9):
- ✅ Tauri 2.0 全面 (tauri 2.11+)
- ✅ 1.0 release tag 已发 (V1.0.0)
- ✅ 9 organ 拟人化 final 1 屏多卡
- ✅ 砍 7 项 UI 哲学 100%
- ✅ TUI 跟 Tauri 升级路径一致
- 🟡 **设计团队到位** (per 主人 8/4 23:33, 暂时 0 设计, 0 改 5 nav + 0 改 9 organ, 仅深化)

**Stage 8 风险 + 决策原则**:
- **R1**: 设计团队未到位 → Stage 8 仅功能验收, 0 改设计 — **缓解**: 主人 8/4 23:33 拍板
- **R2**: 真用户验收 反馈严重 — **缓解**: V1.0.1 patch 快速修复
- **R3**: V1.1 计划跟 1.0 release 冲突 — **缓解**: V1.1 估 2026-11, 跟 Stage 4-6 实战化错开
- ✅ **0 主动 push 严守**: 仅 V1.0.1 patch 推送 (主人起床后手跑)
- ✅ **0 主动 IM 主人**: 仅 done notification

### 3.4 Stage 6+ 路线总览

| Stage | 战略 | 实施 | 时间盒 | 派活 | 状态 |
|------|------|------|-------:|------|------|
| **Stage 5** | Tauri 2.0 完整 + 5 nav 完整 + 9 organ final | 1.0 release 前 | 90 min | R131+ | 蓝图就绪 (本报告 §2) |
| **Stage 6** | 后端 API 集成 (apeireth-api HTTP + WebSocket) | 1.0 release 后 | 90 min | R132-2 | 蓝图就绪 (本报告 §3.1) |
| **Stage 7** | 实际部署 (跨平台打包 + 1.0 release + updater) | 1.0 release 实战 | 75 min | R133 | 蓝图就绪 (本报告 §3.2) |
| **Stage 8** | 用户测试 + 反馈 + V1.0.1 patch + V1.1 规划 | 1.0 release 后 1 周 | 180 min + 7 天 | R132+ | 蓝图就绪 (本报告 §3.3) |
| **Stage 9+** | V1.1 minor release 实施 (per 决策 #81) | 1.0 release 后 ~3 个月 | 估 3-6 月 | R133+ | 待 V1.1 计划 |

---

## 4. V1.1 minor release Tauri 计划 (per 决策 #81 + 决策 #71 §4.1 R130-5)

### 4.1 V1.1 minor release 时间盒 + 战略 (per 决策 #81 + R129-17 §6.1)

**V1.1 minor release** (per 1.0 release 后 ~3 个月, 估 2026-11):
- **TUI 升级阶段 2** (per R131-3 派, 决策 #81)
- **Tauri 终极前端 Stage 4 实战** (per R131-4 派, 决策 #81) — **Stage 4 真实施, 1.0 release 后续**
- **ASI Python Stage 7 治理** (per R131-5 派, 决策 #81)
- **形式化证明器 Stage 5.4 实战** (per R131-6 派, 决策 #81)
- **借鉴 Stage 4-6 集成** (per R131-7 派, 决策 #81)
- **0 装 PASS 严守** (per 决策 #33 §2.3 C2)

### 4.2 V1.1 minor release Tauri 计划详 (per R130-3 spec §3 调研)

**V1.1 Tauri 计划 5 维度 (per 决策 #81 + R129-17 §6.1 + 本报告 §3)**:

| 维度 | 实施 | 估计 | 派活 | 状态 |
|------|------|-----:|------|------|
| **A Tauri Stage 4 实战 (R131-4)** | Stage 4 4 维度实战化 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor) + 84 NEW tests pass | 120 min | R131-4 | 蓝图就绪 (per R129-31 §2) |
| **B Tauri Stage 5 集成 (R130-3 续)** | Stage 5 = Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 跨平台打包 (per 本报告 §2) | 90 min | R131+ 续 | 蓝图就绪 (本报告 §2) |
| **C Tauri Stage 6 后端接通 (R132-2 续)** | Stage 6 = 后端 API 集成 (apeireth-api HTTP + WebSocket, per 本报告 §3.1) | 90 min | R132-2 | 蓝图就绪 (本报告 §3.1) |
| **D Tauri Stage 7 部署 (R133 续)** | Stage 7 = 跨平台打包 + 1.0.1 patch + Tauri 2.0 updater (per 本报告 §3.2) | 75 min | R133 | 蓝图就绪 (本报告 §3.2) |
| **E Tauri 砍 7 项 UI 哲学 100%** | 严守砍 7 项 UI 哲学 (per 用户记忆 #3 + 本报告 §2.7) | 5 min verify | R131+ ~ R133 全程 | 严守 (per R129-19 已实) |
| **总** | | **380 min + 协作** | **R131+ ~ R133** | **V1.1 计划 ready** |

### 4.3 V1.1 Tauri 计划时间线 (per 决策 #81 + 决策 #71 §4.4)

```
[1.0 release tag]  ──>  [V1.1 minor release 计划 R132-2]  ──>  [V1.1 实施 R133+]
  8/11 估 06:00-08:00    1.0 release 后 ~3 个月, 估 2026-11    V1.1 计划后 ~3-6 个月
  R130-5 实战             派 5-10 sub-agent (per 决策 #71 §2.5)  V1.1.0 tag v1.1.0
```

**V1.1 Tauri 计划 vs Stage 4-7 关系**:
- Stage 4 (R129-31 规划 + R131-4 实战) = 实战化 4 维度
- Stage 5 (R130-3 规划 + R131+ 实施) = Tauri 2.0 完整 + 5 nav 完整 + 9 organ final
- Stage 6 (R130-3 蓝图 + R132-2 实战) = 后端 API 集成
- Stage 7 (R130-3 蓝图 + R133 实战) = 实际部署 + 1.0 release + updater
- V1.1 minor release (决策 #81) = Stage 4-7 全部实战 + V1.0.1 patch + 团队反馈实施

**V1.1 Tauri 计划 8 硬墙 0 越界** (per 决策 #33 §2.3):
- B1 24 LOCKED 入口签名 0 改 (V1.1 仅扩 endpoint, 0 改入口)
- B2 workspace.version 1.1.0 (V1.1 release 时改 1.1.0, 0 改 1.2.0)
- A1 R11 baseline 3 值 0 改 (V1.1 0 触碰 integration_r_measure.rs)
- B3 V0.5 30 维 0 改 (V1.1 0 触碰 V0.5 公式)
- B4 6 重守门 v7 0 改 (V1.1 0 改 6 重守门)
- B5 8 哲学锚 0 暴露 (V1.1 0 暴露 UI)
- A3 13 键 verdict 0 改 (V1.1 0 触碰 verdict 逻辑)
- C1 0 主动 commit (V1.1 实施 0 主动 commit, Mavis 整合 #7 commit 拍板)
- C2 0 装 PASS 严守 (V1.1 0 装新 lib, 仅 Tauri 2.0 native)
- C3 升 6 重 v7 0 改 (V1.1 0 改 6 重守门)
- 0 主动 push (V1.1 release push 主人手跑)

### 4.4 V1.1 Tauri 计划 借鉴 (per 决策 #33 §2.3 C2 + 0 借脑 0 装)

**V1.1 Tauri 计划 借鉴清单 (per 决策 #33 §2.3 C2)**:

| 借鉴 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| **Tauri 2.0 跨平台打包** | cargo tauri build 3 平台 (Windows + macOS + Linux) | ❌ 0 装新 framework, 用 Tauri 2.0 native |
| **Tauri 2.0 updater** | V1.0.0 → V1.0.1 → V1.1.0 自动推送 | ❌ 0 装, Tauri 2.0 native |
| **Tauri 2.0 WebView** | WebView2 / WKWebView / WebKitGTK | ❌ 0 装, 浏览器原生 |
| **Tauri 2.0 IPC** | tauriInvoke + cross-window emit | ❌ 0 装, Tauri 2.0 native |
| **superpowers 234 executing-plans** | 5 阶段 DialoguePhase 1:1 翻译 (per R129-9 + R129-19) | ❌ 0 装, 1:1 翻译设计模式 |
| **LangGraph 829 stream_state_events** | WebSocket chunk append 1:1 翻译 (per R129-31 §2.3) | ❌ 0 装, browser native WebSocket |
| **TUI 升级路径** | TUI/Tauri 升级 1:1 翻译 (per 决策 #9) | ❌ 0 装, 0 借源码, 0 重复造轮子 |
| **VCPChat 借鉴** | Electron 桌面 app, chat-first 设计模式 (per 用户记忆, Downloads\VCPChat-main.zip) | ❌ 0 借源码, 仅借鉴 chat-first 模式 |
| **OpenCog AGPL-3.0** | 决策 #33 §2.3 已 skip | ❌ 0 装 |
| **LiteLLM / opencode / Guardrails** | 决策 #56 已 skip | ❌ 0 装 |
| **Jest / Mocha / Vitest** | 0 装, 用自家极简 test-runner.js (per R129-19) | ❌ 0 装, 集成层 0 装 |

---

## 5. 借鉴 Tauri 实战 (per 决策 #33 §2.3 C2 + R130-3 spec §3)

### 5.1 借鉴 Tauri 2.0 (per P11-1/2 真实施 + R129-9 + R129-19 + R129-31)

**Tauri 2.0 真实施** (per P11-1/2 baseline + R129-9 + R129-19 + R129-31):
- **tauri v2.11.5** (per Cargo.lock)
- **tauri-macros 2.6.3** (per Cargo.lock)
- **tauri-cli v2.11.4** (per 决策 #58 §2.1 tauri-cli 真装)
- **frontend/tauri-prototype/src-tauri/** (Tauri 2.0 Rust wrapper)
- **frontend/tauri-prototype/core/** (Rust core, 24 LOCKED 0 改)
- **frontend/tauri-prototype/src/** (frontend JS, 0 装, 0 build step)

**Tauri 2.0 文档** (per P11-1 §借鉴):
- 官方文档: https://tauri.app/v2/
- Stage 1 prototype 阶段已读 (per P11-1 §借鉴 "per web_fetch 查 Tauri 2.0 文档")
- Stage 5 续读: https://tauri.app/v2/distribute/ + https://tauri.app/v2/plugin/updater/

**Tauri 2.0 实施严守 (per 决策 #33 §2.3)**:
- ❌ 0 改 src-tauri/Cargo.toml (0 改 0.1.0)
- ❌ 0 改 core/Cargo.toml (0 改 0.1.0)
- ❌ 0 装 npm / yarn / pnpm (0 build step)
- ❌ 0 装 webpack / vite / rollup (0 build step)
- ✅ 0 装, vanilla JS + Tauri 2.0 native

### 5.2 借鉴 superpowers 234 (per R125-14 executing-plans + R129-9 + R129-19 + R129-31)

**superpowers 234 真实施** (per R125-14 + P5-1 + R129-9):
- superpowers 234 executing-plans = 5 阶段 DialoguePhase 状态机
- per P5-1 R127 Stage 4 自治 + R129-9 Stage 2 5 阶段进度条

**superpowers 234 Stage 5+ 实战化借用**:

| 借用 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| 5 阶段 DialoguePhase | New → Active → Awaiting → Streaming → Closed (per core/src/dialogue.rs) | ❌ 0 装, 1:1 翻译 superpowers 234 |
| 5 阶段进度条 | SVG 360x40 + 5 phase 圆点 + 颜色编码 (per R129-9 dialogue-stream.js) | ❌ 0 装, 纯 vanilla SVG |
| 4 ThinkingPhase | R129-4 D4 自治续 (per superpowers 234 executing-plans) | ❌ 0 装, 1:1 翻译 |
| Stream chunk | LangGraph 829 stream_state_events 1:1 翻译 (per R129-9 流式打字 + Stage 4 B + Stage 6) | ❌ 0 装, 浏览器 native WebSocket |
| 5 阶段 → 9 organ 联动 | Stage 3 status_chat.js (J1) + organ_animator.js | ❌ 0 装, CrossNavStore pub/sub |

**superpowers 234 借脑 0 借具体源码** (per 决策 #33 §2.3 C2):
- ✅ 0 装 superpowers 234 源码本身 (P11-1/2 仅借鉴设计模式, 0 借源码)
- ✅ 0 写 superpowers 234 任何代码 (R129-9 仅 1:1 翻译 DialoguePhase 5 阶段)
- ✅ 0 装 node_modules / package.json (0 build step)
- ✅ 借脑 = 借鉴设计模式, 0 借具体实现

### 5.3 借鉴 LangGraph 829 stream_state_events (per R125-13 + P11-2 + R129-31)

**LangGraph 829 真实施** (per R125-13 + P11-2 + R129-31):
- LangGraph 829 StateGraph = 流式响应 + 状态机
- per P11-2 4 StreamStatus (Idle/Streaming/Paused/Closed) + StreamChunk + StreamSession + progress_pct

**LangGraph 829 Stage 5+ 实战化借用**:

| 借用 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| 4 StreamStatus | Idle / Streaming / Paused / Closed (per core/src/streaming.rs) | ❌ 0 装, 1:1 翻译 LangGraph 829 |
| StreamChunk | 1 chunk 1 append (per R129-9 流式打字 + Stage 4 B + Stage 6) | ❌ 0 装, browser native WebSocket |
| progress_pct | 0-100 流式进度 (per R129-9 5 阶段进度条) | ❌ 0 装, 纯 vanilla SVG |
| stream_state_events | 5 phase 状态机推送 (per Stage 4 B + Stage 6) | ❌ 0 装, browser native WebSocket |
| StateGraph | 5 DialoguePhase + 4 ThinkingPhase (per R129-4 D4 自治续) | ❌ 0 装, 1:1 翻译 |

**LangGraph 829 借脑 0 借具体源码** (per 决策 #33 §2.3 C2):
- ✅ 0 装 LangGraph 829 源码本身
- ✅ 0 写 LangGraph 829 任何代码
- ✅ 借脑 = 借鉴设计模式, 0 借具体实现

### 5.4 借鉴 VCPChat (per 用户记忆, Downloads\VCPChat-main.zip)

**VCPChat 实战参考** (per 用户记忆, VCPChat 是 Electron 桌面 app, chat-first):
- **位置**: `Downloads\VCPChat-main.zip` (per 用户记忆)
- **类型**: Electron 桌面 app (非 Tauri, 但桌面 app 实战模式)
- **借鉴模式**: chat-first 设计模式 (主对话是核心, 其他 nav 围绕主对话)

**VCPChat 借鉴清单 (per 决策 #33 §2.3 C2 + 0 借源码)**:

| 借鉴 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| chat-first 设计模式 | 主对话 = 核心 nav (id 1, per Stage 1 P11-1) | ❌ 0 借源码, 1:1 翻译设计模式 |
| 5 nav 严守 | 状态 / 主对话 / 历史 / 设置 / 工具结果 (跟 VCPChat 主对话 + 副功能 1:1) | ❌ 0 借源码 |
| Electron → Tauri 2.0 迁移模式 | Tauri 2.0 替代 Electron (更小 binary, 更快启动, 跨平台 native) | ❌ 0 借源码, 0 装 Electron |
| VCPChat 实战模式 | 桌面 app UX 设计参考 (窗口尺寸 + 快捷键 + 主题) | ❌ 0 借源码, 1:1 翻译 |
| VCPChat IPC 模式 | VCPChat IPC → Tauri 2.0 invoke (per Stage 3 CrossNavStore 0 装版) | ❌ 0 借源码, 0 装 IPC lib |

**VCPChat 借脑 0 借具体源码** (per 决策 #33 §2.3 C2):
- ✅ 0 借 VCPChat Electron 源码本身
- ✅ 0 装 VCPChat 任何 dep
- ✅ 仅借鉴 chat-first 设计模式 + 桌面 app UX
- ✅ 借脑 = 借鉴设计模式, 0 借具体实现

### 5.5 借鉴 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**Stage 5+ 0 装 PASS 清单**:

| ❌ 0 装 (严守) | 替代方案 | 0 装依据 |
|--------------|---------|---------|
| D3 / visx / eCharts / Chart.js | vanilla SVG (per R129-9) | 用户记忆 #8 瘦客户端 |
| axios / fetch lib | browser native fetch + Tauri 2.0 invoke | 浏览器自带 |
| socket.io / ws lib | browser native WebSocket | 浏览器自带 |
| redux / zustand / mobx | CrossNavStore 0 装版 (Stage 3) | 集成层 0 装 |
| redux-persist / zustand persist | localStorage + BroadcastChannel (Stage 4 C) | 浏览器自带 |
| socket.io-client / eventemitter2 | CrossNavStore pub/sub (Stage 3) | 集成层 0 装 |
| Jest / Mocha / Vitest | 自家极简 test-runner.js (Stage 3) | 集成层 0 装 |
| Tailwind / Material-UI / Chakra | vanilla CSS (per R129-9 + R129-19 style.css) | 用户记忆 #8 瘦客户端 |
| 任何 Node.js / npm 依赖 | 0 build step, 浏览器原生 | 决策 #33 C2 |
| 任何 Python deps (PyO3 928) | 0 装, PyO3 真实施 (per P8-3 R127-2) | 决策 #33 C2 |
| superpowers 234 源码 | 1:1 翻译设计模式, 0 借具体实现 | 决策 #33 C2 |
| LangGraph 829 源码 | stream_state_events 1:1 翻译 | 决策 #33 C2 |
| Tauri 2.0 之外的 desktop 框架 | Tauri 2.0 唯一 (per 用户记忆 #8) | 主人 8/4 23:33 |
| VCPChat Electron 源码 | 1:1 翻译 chat-first 设计模式, 0 借源码 | 决策 #33 C2 |
| OpenCog AGPL-3.0 | 决策 #33 §2.3 已 skip | 决策 #33 C2 |
| LiteLLM / opencode / Guardrails | 决策 #56 已 skip | 决策 #56 |

**0 借脑 0 装严守 (Stage 5+)**:
- ✅ Stage 5 (R130-3 规划) 0 装: 仅规划 doc, 0 改 src, 0 触碰借鉴源码本身
- ✅ Stage 6 (R132-2 派) 0 装: 借 Tauri 2.0 native + superpowers 234 设计模式 + langgraph 829 设计模式, 0 借具体源码
- ✅ Stage 7 (R133 派) 0 装: 借 Tauri 2.0 跨平台打包 + Tauri 2.0 updater, 0 装新 framework
- ✅ Stage 8 (R132+ 派) 0 装: 仅反馈 + V1.0.1 patch, 0 装新 lib
- ✅ V1.1 minor release 0 装: 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 0 借源码
- ✅ 0 借脑: 0 装借鉴源码本身, 0 写借鉴源码任何代码

---

## 6. 0 越界 8 硬墙 (per 决策 #33 §2.3 + 决策 #58 §4 + 用户记忆 #3-#5)

| 硬墙 | 状态 | R130-3 verify |
|------|------|----------------|
| **B1 24 LOCKED 入口签名 0 改** | ✅ 0 改 | R130-3 是 planning doc, 0 触碰 24 LOCKED crate, 0 借 24 LOCKED API |
| **B2 workspace.version 1.2.0 0 改** | ✅ 0 改 | R130-3 0 触碰主仓 Cargo.toml, 0 触碰 frontend/ Cargo.toml |
| **A1 R11 baseline 3 值 0.8682/0.8532/0.9063 0 改** | ✅ 0 改 | R130-3 0 触碰 integration_r_measure.rs |
| **B3 V0.5 25 → 30 维 0 改** | ✅ 0 改 | R130-3 0 触碰 V0.5 公式 |
| **B4 6 重守门 v6 → v7 0 改** | ✅ 0 改 | R130-3 0 改 6 重守门, 0 暴露 UI (per 用户记忆 #3 砍 7 项) |
| **B5 6 → 8 哲学锚 0 暴露** | ✅ 0 暴露 | R130-3 规划 doc 不暴露 8 哲学锚, 0 在 UI 暴露 (per 用户记忆 #3) |
| **A3 12 键原 12 + PHL-07 = 13 键 0 改** | ✅ 0 改 | R130-3 0 触碰 verdict 逻辑 |
| **C1 0 主动 commit** | ✅ 0 commit | R130-3 写到主仓 0 git add, 仅写 reports/ (本报告本身, 整合 #5.3 reports/ 准备) |
| **C2 0 装 PASS 严守** | ✅ 严守 | R130-3 0 装, 借脑 0 借具体源码, 仅借鉴 Tauri 2.0 + superpowers 234 + langgraph 829 + VCPChat 设计模式 |
| **C3 升 6 重 v7** | ✅ 0 改 | R130-3 0 改 6 重守门 |
| **0 主动 push** | ✅ 0 push | 0 push (等 1.0 release 配 GitHub remote + 主人起床后手跑) |

**R130-3 总严守**:
- ✅ 0 改 src-tauri/ (Tauri 2.0 wrapper 0 改)
- ✅ 0 改 core/ (24 LOCKED 0 改)
- ✅ 0 改 主仓 src/ (workspace.version 1.2.0 0 改)
- ✅ 0 改 主仓 Cargo.toml (0 改 0 触碰)
- ✅ 0 改 8 硬墙对应任何文件
- ✅ 0 借脑 0 装 (仅规划, 0 触碰借鉴源码本身)
- ✅ 0 主动 commit (整合 #5.3 reports/ 由 Mavis 拍板)
- ✅ 0 主动 push (1.0 release 实战后)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)

---

## 7. 砍掉 UI 哲学 (per 用户记忆 #3 严守)

### 7.1 砍掉 7 项 UI 哲学元素 (P11-1/2 + R129-9/19/31 已实 + R130-3 蓝图严守)

| 砍掉项 | R129-19 实施 | R130-3 Stage 5+ 规划 |
|--------|-------------|------------------------|
| 守门 (gates) — 6 重守门 v7 | ✅ 不在 UI 展示 | ✅ R130-3 0 暴露, CrossNavStore 0 emit 守门事件 |
| 电子环 (rings) | ✅ 不在 UI 展示 | ✅ R130-3 0 暴露, 9 organ 健康环是 organ 活跃度, 跟电子环不同 |
| 工具调用过程 (process) | ✅ 只显示结果, 不显示过程 | ✅ R130-3 Stage 5 0 暴露 process, 只暴露 result |
| 哲学锚 (anchors) — 8 哲学锚 | ✅ 不在 UI 展示 | ✅ R130-3 0 暴露, CrossNavStore.EVT 0 含哲学锚 |
| 内部机制 (mechanisms) | ✅ 不在 UI 展示 | ✅ R130-3 0 暴露, brain 神经网络只显示 "AI 在思考" 姿态 |
| AI 衰老病死 (per 用户记忆 #4) | ✅ 用 "成长/活跃度" | ✅ R130-3 9 organ 永远循环 + 活跃度 0-100, 0 显示 "已死亡" |
| 0 主动 IM 主人 (per gate-discipline) | ✅ 仅 done notification | ✅ R130-3 0 主动 IM, 仅 done notification |

### 7.2 R130-3 蓝图 0 暴露 8 哲学锚 (B5 硬墙 严守)

8 哲学锚 (per 决策 #11 + 决策 #33 §2.3 B5): 主人 23:23 拍板 0 暴露 UI.

R130-3 Stage 5+ 规划 0 暴露:
- ❌ 0 在 UI 暴露: 8 哲学锚 / 6 重守门 / 24 LOCKED 内部 fn / V0.5 30 维 / 13 键 verdict
- ✅ 0 假装已接: 9 organ 全 Stub readiness (Stage 1-3) → 真 sensor 接入 (Stage 4 D) → 真 1 屏多卡 (Stage 5) → 后端接通 (Stage 6)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)

---

## 8. 9 organ 永远循环 0 死亡 (per 用户记忆 #4 AI 不会衰老病死)

**0 死亡循环严守 (per 用户记忆 #4)**:

- ✅ 9 organ 活跃度 0-100, 永远循环
- ✅ 0 显示 "已死亡 / 老化 / 终止"
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑 (ticker.js, 100ms 周期, per R129-9 §3.5)
- ✅ 活跃度 0% = "dormant" 0 "dead" (per 用户记忆 #4)
- ✅ OrganAnimator.getOrganHealthSummary 用 "活跃度" 非 "健康度" (per R129-19 §3.5)

**Stage 5+ 9 organ 永远循环 ticker** (per R129-9 ticker.js 实施):
```js
// ticker.js (per R129-9 §3.5)
function startTicker() {
  setInterval(() => {
    9 organ 心跳相位更新
    health_ring_for(activity_pct)  // 0-100 永远循环
    heart_ecg_wave(60 samples)     // 60 采样/周期, 永远走纸
    brain_neural_network(9 nodes)  // 9 节点, 永远跑
  }, 100);  // 100ms 周期
}
```

**0 显示死亡/老化/终止** (per 用户记忆 #4 严守):
- ❌ 0 显示: "heart 已死亡" / "brain 老化" / "system 终止"
- ❌ 0 用: "health" / "sick" / "critical" / "dying"
- ✅ 0 用: "activity" / "active" / "idle" / "dormant"
- ✅ 0 用: 0 装 health lib (用 activity_pct 0-100, 自循环)

**Stage 5+ 0 死亡循环严守 (per 用户记忆 #4 + 决策 #33)**:
- ✅ Stage 5 9 organ final 1 屏多卡: 活跃度 0-100 永远循环
- ✅ Stage 6 9 organ 真 sensor 接入: 真 sensor 数据 = 活跃度 (非健康度)
- ✅ Stage 7 跨平台打包: 9 organ UI 在 3 平台一致, 0 死亡
- ✅ Stage 8 用户测试: 0 死亡循环 verify (V1.0.1 patch 修复 9 organ 显示问题)
- ✅ V1.1 minor release: 0 死亡循环 + 9 organ 永远循环

---

## 9. 风险 + 决策原则 (per 决策 #33 + 用户记忆 #3-#5 + #8 + 决策 #71 + #72)

### 9.1 风险 (R130-3 规划 doc 风险评估)

- **R1**: R130-3 60 min 时间盒紧 (Stage 5 + Stage 6+ + V1.1 三大方向) — **缓解**: 0 改 src, 仅规划 doc, 0 触碰借鉴源码, 60 min 估 5 报告节 (各 12 min) + 引用 R129-9/19/31 + 决策 #71/72
- **R2**: Stage 4 (R130-1 续) 跟 R130-3 资源竞争 — **缓解**: R130-1 (cargo test 二次) 跟 R130-3 (规划 doc) 不冲突, R130-3 0 触碰 src
- **R3**: Stage 5 设计团队未到位 (per 主人 8/4 23:33) — **缓解**: R130-3 蓝图就绪, 派活等设计团队就位 (per R131+ 续)
- **R4**: Stage 6 后端 API 集成 需后端 crate — **缓解**: 已有 apeireth-api 真实施 (per 主仓 src/), 1:1 镜像
- **R5**: Stage 7 跨平台打包 CI 资源 — **缓解**: Tauri 2.0 native, 0 装新 framework, 本地 + GitHub Actions
- **R6**: Stage 8 团队就位 (per 主人 8/4 23:33) — **缓解**: 仅功能验收, 0 改设计, 0 改 5 nav + 0 改 9 organ, 仅深化
- **R7**: V1.1 minor release 跟 Stage 4-7 冲突 — **缓解**: V1.1 估 2026-11, 跟 Stage 4-7 实战化错开 (per 决策 #81)
- **R8**: VCPChat Electron 借鉴 仅 1.9MB (per R130-3 spec §3) — **缓解**: 0 借源码, 仅借鉴 chat-first 设计模式 + 桌面 app UX
- **R9**: 借鉴 Tauri 实战参考 (servers 1.9MB / 其他 Tauri 实战参考) 0 装 — **缓解**: 0 借脑 0 借具体源码, 仅借鉴设计模式
- **R10**: 8 硬墙 0 越界 verify 100% — **缓解**: 0 改 src, 0 改 Cargo.toml, 仅规划 doc, 0 触碰借鉴源码本身
- **R11**: 0 借脑 0 装 PASS 严守 — **缓解**: 0 装任何 lib, 仅 Tauri 2.0 native + superpowers 234 + langgraph 829 + VCPChat 设计模式
- **R12**: R130-3 0 触碰 src, 仅规划 doc, 整合 #5 commit reports/ 已含本报告 — **缓解**: Mavis 整合 #5.3 reports/ 拍板时 +本报告 commit
- **R13**: 9 organ 永远循环 0 死亡 (per 用户记忆 #4) 严守 — **缓解**: ticker.js 100ms 周期 (per R129-9 §3.5), activity_pct 0-100, 0 用 health/sick/dying

### 9.2 决策原则 (per 决策 #33 + 用户记忆 #3 + #4 + #5 + #8 + 决策 #71 + #72)

- ✅ **0 改 src 严守** (per 任务 spec + 决策 #33 C2): R130-3 是 planning doc, 0 触碰 src-tauri/core, 0 触碰主仓
- ✅ **0 改 Cargo.toml 严守** (per 任务 spec): 0 触碰 workspace.version 1.2.0
- ✅ **0 主动 commit 严守** (per 决策 #33 §2.3 C1): R130-3 0 主动 git add, 整合 #5.3 reports/ 由 Mavis 拍板
- ✅ **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6): 0 push, 等 1.0 release 配 GitHub remote + 主人起床后手跑
- ✅ **0 借脑 0 装 严守** (per 决策 #33 §2.3 C2): 0 装任何 lib, 借脑 0 借具体源码
- ✅ **0 越界 8 硬墙** (per 决策 #33 §2.3): B1 24 LOCKED / B2 1.2.0 / A1 baseline / B3 30 维 / B4 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 / C3 升 v7 / 0 push 全守
- ✅ **0 暴露 7 项 UI 哲学** (per 用户记忆 #3): 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM
- ✅ **9 organ 永远循环 0 死亡** (per 用户记忆 #4): ticker.js 100ms 周期, activity_pct 0-100, 0 用 health/sick/dying
- ✅ **9 organ 1 屏多卡 拟人化** (per 用户记忆 #5): 3x3 网格 + ECG + NN, 1 真相源 CrossNavStore, 5 nav 共享
- ✅ **0 主动 IM 主人** (per gate-discipline + 用户记忆 #10 + 决策 #61 §6): 仅 done notification
- ✅ **不重写 R129-19/31** (per 任务 spec): R130-3 0 触碰 Stage 3 产物, 仅做 Stage 5 蓝图 + Stage 6+ 路线 + V1.1 计划
- ✅ **TUI 跟 Tauri 升级路径一致** (per 决策 #9 + 用户记忆 #8 + #9): TUI/Tauri 升级 1:1 翻译, 后端 0 改
- ✅ **Stage 4 续接** (per 决策 #71 + R129-31 §2): Stage 4 4 维度蓝图 = Stage 5-6 实战化起点

### 9.3 R130-3 报告本身 0 触碰主仓 verify

```bash
# 假设跑 (R130-3 实际 0 跑, 仅规划):
$ cd Apeireth-rust
$ git status --porcelain
# 仅显示:
# ?? reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md
# (0 触碰主仓 src/, 0 触碰 frontend/, 0 触碰 Cargo.toml)
```

**R130-3 报告 0 触碰 verify**:
- ✅ 0 改 src-tauri/ (Tauri 2.0 wrapper 0 改)
- ✅ 0 改 core/ (24 LOCKED 0 改)
- ✅ 0 改 主仓 src/ (workspace.version 1.2.0 0 改)
- ✅ 0 改 主仓 Cargo.toml (0 改 0 触碰)
- ✅ 0 借脑 0 装 (仅规划, 0 触碰借鉴源码本身)
- ✅ 0 主动 commit (整合 #5.3 reports/ 由 Mavis 拍板)
- ✅ 0 主动 push (1.0 release 实战后)

---

## 10. 一句话 (再次强调)

**Tauri 终极前端 Stage 5 集成深化规划 (R130-3, 60 min, planning doc only) — Stage 5 = Tauri 2.0 完整集成 (tauri 2.11+ 跨平台打包) + 5 nav 完整 (TUI 1:1 镜像) + 9 organ 拟人化 final (1 真相源 + 5 nav 共享 + 永远循环 0 死亡 + 1 屏多卡) + 砍 7 项 UI 哲学 100% + 后端全 API 表面同步 (TUI/Tauri 共用, per 用户记忆 #8 瘦客户端). Stage 6+ 路线 = Stage 6 后端 API 集成 (apeireth-api HTTP + WebSocket 真接通) + Stage 7 实际部署 (Tauri 跨平台打包 + 1.0 release tag + GitHub release + updater) + Stage 8 用户测试 (V1.0.0 release 后, 真用户验收 + 反馈 + V1.0.1 patch). V1.1 minor release Tauri 计划 (估 2026-11, per 决策 #81): Tauri Stage 4 实战 (R131-4 派) + Tauri Stage 5 集成 (R130-3 续) + Tauri Stage 6 后端接通 (R132-2 续) + Tauri Stage 7 部署 (R133 续) + 砍 7 项 UI 哲学 100% verify. 借鉴 Tauri 2.0 (P11-1/2 真实施, tauri v2.11.5 + tauri-macros 2.6.3, 0 装 vanilla JS) + superpowers 234 (executing-plans 5 阶段 DialoguePhase 1:1 翻译) + LangGraph 829 (stream_state_events 1:1 翻译) + VCPChat (Electron 桌面 app 借鉴, chat-first 设计模式, 0 借源码仅借鉴设计). 0 改 src + 0 改 Cargo.toml + 0 主动 commit (Mavis 整合 #5.3 reports/ 拍板) + 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑) + 0 主动 IM 主人 (per gate-discipline, 仅 done notification). 8 硬墙 0 越界 (B1 24 LOCKED / B2 1.2.0 / A1 baseline / B3 30 维 / B4 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 / C3 升 v7 / 0 push 全守). 砍 7 项 UI 哲学 (per 用户记忆 #3 严守). 9 organ 永远循环 0 死亡 (per 用户记忆 #4 严守). 0 借脑 0 装 PASS 严守 (per 决策 #33 §2.3 C2). 不重写 R129-19/31 (0 触碰 Stage 3/4 产物).**

---

## 11. refs

### 11.1 R129 era Tauri 5 报告 (per 决策 #72 §1.2)

- P11-1 R128 tauri-frontend-prototype-final-2026-08-10: Tauri 2.0 prototype 真实施 (72 tests, 0 借脑 0 装)
  - `reports/agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md`
- P11-2 R128-2 tauri-frontend-scaffold-final-2026-08-10: Tauri 2.0 scaffold 深化 (111 tests, cargo build PASS + cargo tauri dev 跑通)
  - `reports/agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md`
- R129-9 Tauri Stage 2 深化 (2026-08-11 00:35): 5 phase 进度条 + 流式打字 + 9 健康环 + heart ECG + brain NN + 122 tests
  - `reports/agent-r129-9-tauri-stage-2-deepening-2026-08-11.md`
- R129-19 Tauri Stage 3 跨 nav 集成 (2026-08-11 00:34): 7 模块 J1-J7 + CrossNavStore 状态中枢 + 9 organ animator + 79 tests + 8 examples + 1 hub
  - `reports/agent-r129-19-tauri-stage-3-integration-2026-08-11.md`
- R129-31 Tauri Stage 4 实战规划 (2026-08-11 00:56): 4 维度 A 真后端 / B WebSocket / C 持久化 / D 真 sensor 蓝图 + 84 NEW tests 累计 163
  - `reports/agent-r129-31-tauri-stage-4-execution-2026-08-11.md`

### 11.2 决策 + 路线图 (per 决策 #71 + #72 + R130 era)

- 决策 #33: 8 硬墙 + 0 装 PASS 严守 + 0 主动 commit + 0 主动 push
- 决策 #48: 整合 #4 commit abf12243
- 决策 #55: 9 阶段路线图 + 24 LOCKED + 借鉴 ID 严格化
- 决策 #57: R128 阶段 B Tauri prototype 派活
- 决策 #58: R128-2 3 sub-agent (P11-2 scaffold 深化 spec)
- 决策 #61: 新会话接手 + R129 era 派活规划
- 决策 #62: 整合 #5 commit 拆 3 commit
- 决策 #64: 5 min tick cron 自动监督
- 决策 #65: R129 era 第 2 批 8 sub-agent 派活 (R129-9 Stage 2 + R129-19 Stage 3)
- 决策 #66: R129 era 第 3 批 7 sub-agent 派活 (R129-17 R130 + R129-19 Stage 3)
- 决策 #71: 计划内任务完成自动接续 4 步 (R130 调研 + R131 差距 + R132 计划 + R133+ 实施)
- 决策 #72: R129 era 34/35 done + R130 era 调研 派活 (本 R130-3 派活依据)
- 决策 #78: 1.0 release tag v1.0.0 实战 (主人起床后手跑)
- 决策 #79: 整合 #6 commit 拍板 (Mavis 自决, 拆 3 commit)
- 决策 #80: 1.0 release 后路线图 (TUI + Tauri + ASI + 形式化 + V1.1/V1.2)
- 决策 #81: V1.1 minor release 计划 (估 2026-11, 1.0 release 后 ~3 个月)
- 决策 #82: V1.2 minor release 计划 (估 2027-02, V1.1 后 ~3 个月)

### 11.3 R130 era 路线图 (per R130-3 spec)

- R129-17 R130 era 路线图详细: `reports/agent-r129-17-r130-roadmap-detailed-2026-08-11.md` (R130-3 Tauri Stage 3 深化 spec)
- R129-29 R130 era 路线图 final: `reports/agent-r129-29-r130-roadmap-final-2026-08-11.md`
- R129-15 TUI 升级路线图沉淀: `reports/agent-r129-15-tui-upgrade-roadmap-2026-08-11.md` (TUI 跟 Tauri 升级路径一致)
- R129-16 R129 era 决策链更新: `reports/agent-r129-16-r129-era-decision-chain-update-2026-08-11.md`

### 11.4 用户记忆 (跨 project 适用, per 决策 #10 + #33 §2.3)

- 用户记忆 #3: 用户看结果不看哲学, 砍 7 项 UI 哲学
- 用户记忆 #4: AI 不会衰老病死, 9 organ 永远循环
- 用户记忆 #5: 信息密度高 = 拟人化 + 拟物化, 1 屏多卡
- 用户记忆 #6: 派 sub-agent 干, 但要驾驭团队不重复造轮子
- 用户记忆 #7: 推技术决策要守规范, 但要诚实
- 用户记忆 #8: 前端终极 = Tauri, TUI 是过渡
- 用户记忆 #9: TUI 升级节奏, 改瘦后暂告段落, 优先后端
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志

### 11.5 主人拍板 (per 决策 #10 + 用户记忆)

- 主人 8/4 23:33: "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备."
- 主人 8/4 23:55: "测一下先, 后续的 tui 升级计划沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui"
- 主人 8/11 0:25: "全部你做主" (Mavis 升级决策权)
- 主人 8/11 0:34: "跑中 ≥ 16" (16 active 全 background 跑)
- 主人 8/11 0:43: 中断接手机制
- 主人 8/11 0:49: 编译产物清理决策矩阵
- 主人 8/11 0:54: Mavis 升级决策权 + 150 GB 强制清理
- 主人 8/11 0:57: 计划内任务完成自动接续 4 步 (调研 + 差距 + 计划 + 继续干)

### 11.6 借鉴源码 (per 决策 #33 §2.3 C2 + 0 借脑 0 装)

- Tauri 2.0 (P11-1/2 真实施): tauri v2.11.5 + tauri-macros 2.6.3 + tauri-cli v2.11.4
- superpowers 234 executing-plans (R125-14 + P5-1 真实施): 5 阶段 DialoguePhase 1:1 翻译
- LangGraph 829 stream_state_events (R125-13 + P11-2 真实施): 4 StreamStatus + progress_pct
- PyO3 928 (R125-9 + P8-3 R127-2 真实施): ASI Python 桥接
- kani 4502 (R125-10 + R129-20 真实施): 形式化证明器
- VCPChat (per 用户记忆, Downloads\VCPChat-main.zip): Electron 桌面 app 借鉴, chat-first 设计模式

### 11.7 项目结构 (per R130-3 verify)

```
Apeireth-rust/                                          # 主仓 (workspace.version 1.2.0 严守)
├── Cargo.toml                                          # workspace 1.2.0 0 改
├── Cargo.lock                                          # 0 改
├── src/                                                # 24 LOCKED 入口签名 0 改
├── tests/                                              # 跨模块守门 0 改
├── docs/                                               # 0 改 (per 决策 #62 5.2 准备)
├── frontend/                                           # Tauri 终极前端
│   └── tauri-prototype/                                # Tauri 2.0 桌面 app
│       ├── core/                                       # 122 tests pass (102 unit + 20 integration)
│       ├── src-tauri/                                  # Tauri 2.0 wrapper, 27+ commands 拆 8 submod
│       ├── src/                                        # 0 装 vanilla JS, 32 文件 + 8 examples
│       │   └── integration/                            # CrossNavStore + 7 模块 (J1-J7) + 9 organ animator
│       ├── docs/                                       # STRUCTURE.md 架构图
│       └── README.md                                   # 任何人接手指南
├── borrowed-repos/                                     # 11 借鉴源 (49.60 MB / 7,764 files, per R129-7)
├── reports/                                            # 决策 + sub-agent 报告
│   ├── decision-71-r129-to-r130-auto-continuation-2026-08-11.md
│   ├── decision-72-r130-era-dispatch-r129-3-final-wait-2026-08-11.md
│   ├── agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md
│   ├── agent-p11-2-r128-2-tauri-frontend-scaffold-final-2026-08-10.md
│   ├── agent-r129-9-tauri-stage-2-deepening-2026-08-11.md
│   ├── agent-r129-19-tauri-stage-3-integration-2026-08-11.md
│   ├── agent-r129-31-tauri-stage-4-execution-2026-08-11.md
│   ├── agent-r129-17-r130-roadmap-detailed-2026-08-11.md
│   ├── agent-r129-29-r130-roadmap-final-2026-08-11.md
│   └── agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md  ← 本报告
└── scripts/                                            # 1.0 release 实战 (R130-5 续)
```

---

**R130-3 done**: 60 min 时间盒内完成 Stage 5 集成深化方案 (Tauri 2.0 完整 + 5 nav 完整 + 9 organ final + 跨平台打包 + 后端全 API 表面同步 + 砍 7 项 UI 哲学 100%) + Stage 6+ 路线图 spec (Stage 6 后端 API 集成 + Stage 7 实际部署 + Stage 8 用户测试) + V1.1 minor release Tauri 计划 (估 2026-11, 5 维度 380 min). 借鉴 Tauri 2.0 + superpowers 234 + LangGraph 829 + VCPChat (0 借脑 0 装严守) + 0 改 src + 0 改 Cargo.toml + 0 主动 commit (整合 #5.3 reports/ 由 Mavis 拍板) + 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑) + 0 主动 IM 主人 (per gate-discipline, 仅 done notification). 8 硬墙 0 越界 (B1 24 LOCKED / B2 1.2.0 / A1 baseline / B3 30 维 / B4 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 / C3 升 v7 / 0 push 全守). 砍 7 项 UI 哲学 (per 用户记忆 #3 严守). 9 organ 永远循环 0 死亡 (per 用户记忆 #4 严守). 报告路径 `reports/agent-r130-3-tauri-stage-5-integration-deepening-2026-08-11.md`. 整合 #5.3 reports/ commit 由 Mavis 拍板.
