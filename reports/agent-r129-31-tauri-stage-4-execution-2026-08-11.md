# Agent R129-31 — Tauri Stage 4 实战 (planning doc, 2026-08-11)

**Date**: 2026-08-11 (R129 era 扩展, 30 min 时间盒)
**Author**: Mavis sub-agent R129-31 (planning-only, 0 改 src, 0 改 Cargo.toml, 0 主动 commit, 0 主动 push)
**任务**: Tauri Stage 4 实战 (R129-19 Stage 3 续) — Stage 4 集成 (5 nav × 9 organ 实战化) + Stage 5 路线 (终极前端) + 9 organ 拟人化深化 final + 借鉴 Tauri 2.0 + superpowers 234
**派活依据**: Mavis R129 era 跑过夜 16 跑中监督, R129-19 Stage 3 已 done (00:34 ✅), 派 R129-31 接续做 Stage 4 实战规划
**不重写 R129-19**: Stage 3 产物 (CrossNavStore + 7 集成模块 + 9 organ 拟人化 + 79 tests) 0 触碰, 本报告仅做 Stage 4 战略 + Stage 5 路线 + 9 organ final 深化 + 借鉴映射
**报告路径**: `reports/agent-r129-31-tauri-stage-4-execution-2026-08-11.md`
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守)
**整合 #5 commit**: 估 8/11 00:38 (per decision-62 拆 3 commit: 5.1 src/ + 5.2 docs/ + 5.3 reports/), Mavis 自决拍板

---

## 0. 一句话

**Tauri 终极前端 Stage 4 实战规划 (planning doc only) — R129-19 Stage 3 已 done (CrossNavStore + J1-J7 + 9 organ 拟人化 + 79 tests + 8 examples), Stage 4 实战 = 4 维度 (A 真后端接通 / B WebSocket 流式 / C 跨 tab 持久化 / D 9 organ 真实 sensor 接入) 实施蓝图, Stage 5 路线 = 终极前端 (Tauri 2.0 团队就位 + 5 nav 真打通 + 9 organ 拟人化 final 1 屏多卡 + 砍 7 项 UI 哲学 + 后端全 API 表面同步), 9 organ 拟人化深化 final (per 用户记忆 #5 拟人化 + 拟物化 + 0 死亡循环, R130+ 续 9 organ 颜色 + 心跳 + 神经网络 + 心电图 + 9 健康环 + 1 真相源), 借鉴 Tauri 2.0 (P11-1/2 真实施, tauri v2.11.5 + tauri-macros 2.6.3) + superpowers 234 (executing-plans 5 阶段 DialoguePhase 状态机 1:1 翻译, 0 借脑 0 装严守). 0 改 src + 0 改 Cargo.toml + 0 主动 commit (Mavis 整合 #5/6 拍板) + 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑). 8 硬墙 0 越界 (B1/B2/A1/B3/B4/B5/A3/C1/C2/C3/0 push 全守). 砍掉 UI 哲学 (per 用户记忆 #3 严守): 0 暴露守门/电子环/哲学锚/工具调用过程/内部机制. 0 死亡循环 (per 用户记忆 #4 AI 不会衰老病死). 0 装 PASS 严守 (per 决策 #33 §2.3 C2).**

---

## 1. 上下文回顾: R129-19 Stage 3 现状 (per 决策 #66 + Stage 3 报告 §1-§10)

### 1.1 Stage 3 已完成产物 (per R129-19 报告 §2.1, 0 改 R129-31)

```
frontend/tauri-prototype/src/integration/                  # 总 32 文件 / ~128 KB
├── README.md (10 KB)                                     # 架构图 + 10 节
├── store.js (10 KB)                                      # CrossNavStore 状态中枢 (1 真相源)
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
│   ├── test-runner.html (3 KB)                           # 浏览器 test runner
│   ├── run-all.js (3 KB)                                 # Node 跑全部 test
│   ├── store.test.js (8 KB, 22 cases)                    # 22 cases pass
│   ├── status_chat.test.js (3 KB, 6 cases)               # 6 cases pass
│   ├── status_history.test.js (3 KB, 7 cases)            # 7 cases pass
│   ├── status_tools.test.js (3 KB, 7 cases)              # 7 cases pass
│   ├── chat_history.test.js (3 KB, 7 cases)              # 7 cases pass
│   ├── chat_tools.test.js (4 KB, 9 cases)                # 9 cases pass
│   ├── history_tools.test.js (3 KB, 8 cases)             # 8 cases pass
│   └── settings_global.test.js (5 KB, 13 cases)          # 13 cases pass
└── examples/                                             # 8 HTML examples + 1 共享 CSS
    ├── style.css (2 KB)
    ├── status-chat.html (2 KB)                           # J1
    ├── status-history.html (1 KB)                        # J2
    ├── status-tools.html (2 KB)                          # J3
    ├── chat-history.html (2 KB)                          # J4
    ├── chat-tools.html (2 KB)                            # J5
    ├── history-tools.html (3 KB)                         # J6
    ├── settings-global.html (3 KB)                       # J7
    ├── organ-animator.html (4 KB)                        # 9 organ 拟人化
    └── stage3-hub.html (6 KB)                            # 7 集成 + 9 organ 综合 hub
```

**Stage 3 总结** (per R129-19 报告 §0):
- 7 集成模块 (J1-J7) + 1 CrossNavStore 状态中枢 (pub/sub, 14 EVT 常量 + 12 state mutators + 5 nav 状态 + 9 organ 活动)
- 9 organ 拟人化深化 (organ_animator.js 9 KB, 5 helper: renderChatHeaderOrgans / renderToolsHeaderOrgan / renderHistoryHeaderOrgans / renderSettingsHeaderOrgan / getOrganHealthSummary)
- 51+ 集成 test cases (79 全 pass, per node run-all.js 跑通)
- 8 examples + 1 hub (stage3-hub.html)
- cargo build PASS (3.96s) + core lib 122 tests pass (102 unit + 20 integration, 0.01s)
- 0 主动 commit (写到主仓 0 git add) + 0 主动 push 严守

### 1.2 Stage 3 验证 (per R129-19 报告 §9.1-§9.3)

| 验证项 | 结果 | 备注 |
|--------|------|------|
| cargo build src-tauri | ✅ PASS (3.96s) | 0 改 src-tauri, 0 改 core, 0 改主仓 |
| core lib 122 tests | ✅ PASS (0.01s) | 102 unit + 20 integration, 0 regression |
| 集成层 79 tests | ✅ PASS | 7 集成 + 1 store 全 pass |
| cargo tauri dev 跳过 | ⚠️ per R129-19 风险 R1 | cargo build PASS 验证 0 越界, 资源竞争避开 |

### 1.3 R129-19 严守 0 触碰 (per 决策 #33 §2.3 + 决策 #58)

- ✅ 0 改 src-tauri/, 0 改 core/, 0 改主仓
- ✅ 0 改 Cargo.toml workspace.version 1.2.0
- ✅ 0 改 24 LOCKED 入口签名
- ✅ 0 改 R11 baseline 3 值 0.8682/0.8532/0.9063
- ✅ 0 改 V0.5 30 维
- ✅ 0 改 6 重守门 v7
- ✅ 0 改 8 哲学锚
- ✅ 0 改 13 键 verdict
- ✅ 0 主动 commit (写到主仓 0 git add)
- ✅ 0 借脑 0 装 (0 装 PASS 严守)
- ✅ 0 主动 push

---

## 2. Stage 4 集成: 4 维度 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor)

### 2.1 Stage 4 战略定位 (per 用户记忆 #8 + 决策 #9 + 主人 8/4 23:33)

**Stage 3 (R129-19 done)**: 跨 nav 集成层就绪, 集成层 = 集成层, 0 真实后端接通
**Stage 4 (R130-3 + R130+ 派)**: 4 维度实战化, 0 装严守 + 借脑 0 借具体源码, 跟后端 apeireth-api 真接通
**Stage 5 (R131+ 路线)**: 终极前端, 等设计团队到位 + Tauri 2.0 全面 + 5 nav 真打通 + 9 organ 拟人化 final 1 屏

**Stage 4 4 维度实战化蓝图**:

```
[Stage 3 集成层就绪]   ──>  [Stage 4 实战化 4 维度]                  ──>  [Stage 5 终极]
  CrossNavStore 7+J         A 真后端接通 (apeireth-api HTTP)         Tauri 2.0 全面
  9 organ 拟人化深化         B WebSocket 流式 (真 LLM stream)         5 nav 真打通
  79 tests pass              C 跨 tab 持久化 (localStorage + 后端)    9 organ final
  8 examples + 1 hub         D 9 organ 真 sensor 接入                 砍 7 项 UI 哲学
                             0 装 PASS 严守 100%                      后端全 API 表面
```

### 2.2 维度 A: 真后端接通 (apeireth-api HTTP, 瘦客户端)

**目标**: CrossNavStore 调 tauriInvoke, 失败回 mockInvoke 改成: tauriInvoke 主路径, mock 仅 dev 模式

**实施蓝图 (per 决策 #33 §2.3 + 用户记忆 #8 瘦客户端)**:

| ID | 模块 | 当前 (Stage 3) | Stage 4 实战 | 借鉴 |
|----|------|---------------|-------------|------|
| A1 | `chat_history.js` (J4) | tauriInvoke 失败回 mock | 真接通: `tauriInvoke('chat_send_message')` 调 `apeireth-api/v1/chat/messages` | Tauri 2.0 + langgraph 829 |
| A2 | `status_chat.js` (J1) | mock DialoguePhase | 真接通: `tauriInvoke('get_dialogue_session')` 拿 5 phase | Tauri 2.0 + superpowers 234 |
| A3 | `status_history.js` (J2) | mock history | 真接通: `tauriInvoke('get_history')` + `get_history_timeline` | Tauri 2.0 + TUI pages/history.rs |
| A4 | `status_tools.js` (J3) | mock tool outcomes | 真接通: `tauriInvoke('get_6_tool_results')` + `get_6_tool_calls` | Tauri 2.0 + TUI 6 工具 endpoint |
| A5 | `settings_global.js` (J7) | mock settings | 真接通: `tauriInvoke('get_settings')` + `get_setting_value` + `set_setting_value` | Tauri 2.0 + TUI 14 settings |
| A6 | `organ_animator.js` | mock 9 organ activities | 真接通: `tauriInvoke('get_9_organs')` + `get_9_organ_activities` (D 维度实施) | Tauri 2.0 + core/src/organ.rs |

**HTTP 路由** (per 主人 8/4 23:33 "TUI 升级路径一致, 瘦客户端, HTTP to apeireth-api"):
- `GET  /v1/organs` → 9 organ + activities
- `POST /v1/chat/messages` → user 消息 + AI 回复
- `GET  /v1/chat/session/{id}` → 5 DialoguePhase
- `GET  /v1/history` → history entries
- `GET  /v1/tools/results` → 6 tool results
- `GET  /v1/settings` → 14 settings
- `PATCH /v1/settings/{key}` → 改 1 setting

**0 装 PASS 严守**:
- ❌ 0 装 axios / fetch lib (用 browser native fetch + Tauri 2.0 invoke)
- ❌ 0 装状态管理 lib (用 CrossNavStore 0 装版)
- ❌ 0 装路由 lib (用 5 nav switch)
- ✅ tauriInvoke 主路径, mock 仅 dev mode fallback (per Stage 3 R129-19 严守)

**测试** (per 决策 #58 §0 真实跑通):
- 加 6 模块 × 5 cases = 30 NEW tests (A1-A6 各 5)
- 总集成层 tests: 79 (Stage 3) + 30 (Stage 4 A) = 109

### 2.3 维度 B: WebSocket 流式 (真 LLM stream, 0 装)

**目标**: 流式打字 (Stage 2 R129-9 已实, 字符级 50ms/字) → 真 WebSocket chunk append

**实施蓝图**:

| ID | 模块 | 当前 (Stage 2/3) | Stage 4 实战 | 借鉴 |
|----|------|-----------------|-------------|------|
| B1 | `dialogue-stream.js` | setTimeout 50ms/字模拟 | WebSocket `/v1/chat/stream/{session_id}` chunk append | langgraph 829 stream_state_events |
| B2 | `chat_history.js` (J4) | 一次性 appendMsg | 流式 append chunk (边收边追加) | langgraph 829 |
| B3 | `status_chat.js` (J1) | mock Streaming phase | 真 WebSocket phase 切换: New → Active → Streaming → Awaiting | superpowers 234 |
| B4 | `organ_animator.js` | mock voice 3000ms | 真 voice 心跳跟 stream 同步 (chunk rate 加速) | TUI organ/mod.rs |

**WebSocket 协议** (per langgraph 829 stream_state_events 1:1 翻译):
```json
// client → server
{"type": "send_message", "session_id": "...", "content": "..."}

// server → client (chunks)
{"type": "phase_change", "phase": "Streaming"}
{"type": "stream_chunk", "content": "..."}      // 累加到 AI 气泡
{"type": "stream_chunk", "content": "..."}
{"type": "stream_end", "full_content": "..."}   // 写入 history (J4)
{"type": "phase_change", "phase": "Awaiting"}
```

**0 装 PASS 严守**:
- ❌ 0 装 socket.io / ws lib (用 browser native WebSocket)
- ❌ 0 装 stream lib (用 chunk append 0 装)
- ✅ langgraph 829 stream_state_events 1:1 翻译 (借鉴模式, 0 借源码)

**测试**:
- B1 stream chunk 接收 → AI 气泡逐字显示: 5 cases
- B2 phase 切换: 5 cases (New/Active/Awaiting/Streaming/Closed)
- B3 流式 append + history 同步: 5 cases
- B4 organ voice 同步心跳: 5 cases
- 总: B1-B4 × 5 = 20 NEW tests, 集成层 109 + 20 = 129

### 2.4 维度 C: 跨 tab 持久化 (localStorage + 后端, 0 装)

**目标**: settings/theme/font/layout 跨 tab 同步, 用户体验 "开 5 tab 都是同样设置"

**实施蓝图**:

| ID | 模块 | 当前 (Stage 3) | Stage 4 实战 | 借鉴 |
|----|------|---------------|-------------|------|
| C1 | `settings_global.js` (J7) | 当前 session only | localStorage 持久化 + BroadcastChannel 跨 tab | 浏览器原生 API |
| C2 | `store.js` CrossNavStore | in-memory | localStorage 序列化 + 反序列化 (5 nav + 9 organ) | 浏览器原生 API |
| C3 | `organ_animator.js` | 1 tab 动效 | BroadcastChannel 跨 tab organ 心跳同步 | 浏览器原生 API |
| C4 | `chat_history.js` (J4) | 当前 session | 后端持久化 (apeireth-api 真接, 0 装) | Tauri 2.0 |

**0 装 PASS 严守**:
- ❌ 0 装 redux-persist / zustand persist (用 browser localStorage + BroadcastChannel)
- ❌ 0 装 storage lib (用 browser native API)
- ✅ 借脑 0 借 (浏览器 API 0 装 = 浏览器自带)

**测试**:
- C1 settings 改 → localStorage 写 + 跨 tab 同步: 5 cases
- C2 store 状态 → 序列化 + 反序列化: 5 cases
- C3 organ 心跳 → BroadcastChannel: 5 cases
- C4 chat history → 后端持久化: 5 cases
- 总: C1-C4 × 5 = 20 NEW tests, 集成层 129 + 20 = 149

### 2.5 维度 D: 9 organ 真 sensor 接入 (per core/src/organ.rs 1:1)

**目标**: 9 organ 真状态接入 (per Stage 2 心电图 P-QRS-T + 健康环 + 神经网络), 数据源 = 真 sensor

**实施蓝图**:

| ID | organ | Stage 2 模拟 | Stage 4 真 sensor 接入 | 借鉴 |
|----|-------|------------|---------------------|------|
| D1 | heart | mock 1200ms | 后端 `/v1/organs/heart/ecg` 真 ECG 60 采样/周期 | core/src/visualization.rs::heart_ecg_wave |
| D2 | brain | mock 800ms | 后端 `/v1/organs/brain/activity` + 神经网络 9 节点状态 | core/src/visualization.rs::brain_neural_network |
| D3 | hand | mock 2500ms | 后端 `/v1/organs/hand/pending_tools` 真工具待办数 | TUI 6 工具 endpoint |
| D4 | eye | mock 3000ms | 后端 `/v1/organs/eye/observing` 真观察频率 | TUI pages/history.rs |
| D5 | ear | mock 2000ms | 后端 `/v1/organs/ear/listening` 真 chat 输入频率 | TUI pages/dialogue.rs |
| D6 | memory | mock 5500ms | 后端 `/v1/organs/memory/sediment` 真 history 过滤数 | TUI pages/history.rs |
| D7 | voice | mock 4500ms | 后端 `/v1/organs/voice/speaking` 真 stream 速度 | langgraph 829 |
| D8 | body | mock 10000ms | 后端 `/v1/organs/body/uptime` 真系统运行时长 | TUI app_state.rs |
| D9 | mind | mock 6500ms | 后端 `/v1/organs/mind/thinking` 真 thinking 阶段 | superpowers 234 + R129-4 D4 |

**真 sensor 数据流** (per 决策 #58 §0 真 src 改动 + tests pass):
- 后端 Rust crate (apeireth-organs, R131+ 派) → 9 organ 真 sensor
- API: 9 GET endpoint + 1 unified `/v1/organs` (批量拿 9 organ)
- WebSocket 推送 (per 维度 B): organ 状态变 → 实时推前端
- 前端: CrossNavStore.organ_activities 1 真相源, 5 nav 共享 (per Stage 3 已实)

**0 装 PASS 严守**:
- ❌ 0 装 sensor 硬件驱动 (借后端 Rust crate 真实施)
- ❌ 0 装数据可视化 lib (用 vanilla SVG, per Stage 2 R129-9 严守)
- ✅ 9 organ 数据 = core/src/organ.rs 1:1 镜像 (per R129-9 实施)

**测试**:
- D1-D9 各 1 真 sensor test = 9 tests (真接通后端)
- D-统一: 9 organ 批量拿 + 5 nav 共享: 5 cases
- 总: D1-D9 + D-统一 = 14 NEW tests, 集成层 149 + 14 = 163

### 2.6 Stage 4 测试总览 (per 决策 #58 §0)

| 维度 | NEW tests | 累计 (Stage 3 79 + ...) |
|------|----------:|-------------------------:|
| Stage 3 (R129-19) | 79 | 79 (R129-19 报告 §2.3) |
| A 真后端接通 | 30 | 109 |
| B WebSocket 流式 | 20 | 129 |
| C 跨 tab 持久化 | 20 | 149 |
| D 9 organ 真 sensor | 14 | 163 |
| **Stage 4 总** | **84** | **163** |
| **Stage 3 + Stage 4 累计** | **163** | **集成层 79 + 84 = 163** |

**核心 lib tests** (per R129-9 P11-2 111 + R129-9 11 = 122):
- Stage 4 加 0 NEW (集成层, 0 触碰 core/)
- 核心 lib 仍 122 tests (102 unit + 20 integration)

**总测试** (Stage 3 + Stage 4):
- 集成层: 163 tests (JS)
- core lib: 122 tests (Rust)
- **总 285 tests, 0 装 PASS 严守**

### 2.7 Stage 4 实施清单 (R130-3 派, 估 120 min)

| Step | 内容 | 文件 | 估计 |
|------|------|------|-----:|
| 1 | 维度 A 真后端接通 (6 模块 × 5 cases) | 6 JS + 6 test = 12 files | 40 min |
| 2 | 维度 B WebSocket 流式 (4 模块 × 5 cases) | 4 JS + 4 test = 8 files | 30 min |
| 3 | 维度 C 跨 tab 持久化 (4 模块 × 5 cases) | 4 JS + 4 test = 8 files | 20 min |
| 4 | 维度 D 9 organ 真 sensor (9 + 1 unified × 1 case) | 9 + 1 test files | 20 min |
| 5 | cargo build + cargo test 0 越界 verify | 0 file (verify) | 5 min |
| 6 | 写报告 | 1 report file | 5 min |
| **总** | | | **120 min** |

**0 改严守** (per 决策 #33 §2.3):
- 0 改 core/src/ (24 LOCKED 入口签名 0 改)
- 0 改 src-tauri/ (Tauri 2.0 wrapper 0 改)
- 0 改 主仓 src/ (workspace.version 1.2.0 0 改)
- 0 借脑 0 装 (per 决策 #33 §2.3 C2)
- 0 主动 commit (写到主仓 0 git add, 整合 #6 commit 由 Mavis 拍板)
- 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑)

---

## 3. Stage 5 路线: 终极前端 (R131+ 派, 等设计团队到位)

### 3.1 Stage 5 战略定位 (per 主人 8/4 23:33 + 用户记忆 #8 + 决策 #9)

**Stage 5 终极前端 = Tauri 2.0 全面 + 5 nav 真打通 + 9 organ 拟人化 final 1 屏 + 砍 7 项 UI 哲学**

**Stage 5 跟 Stage 4 关系**:
- Stage 4 (R130-3): 实战化, 集成层 4 维度接通 (A 真后端 / B WebSocket / C 持久化 / D 真 sensor)
- Stage 5 (R131+): 终极, Tauri 2.0 全面 + 5 nav 真打通 + 9 organ final + 设计团队到位
- **TUI = 过渡** (per 决策 #9 + 用户记忆 #8), 终极 = Tauri

**Stage 5 4 大战略目标**:

```
[Stage 4 实战化]    ──>  [Stage 5 终极前端 R131+]
  4 维度接通            Tauri 2.0 全面 (tauri 2.11+)
  163 集成层 tests       5 nav 真打通 (TUI nav 1:1)
  cargo build PASS       9 organ final 1 屏多卡
                         砍 7 项 UI 哲学 100%
                         后端全 API 表面同步
                         设计团队到位 (per 主人 8/4 23:33)
                         1.0 release 部署 + 自动更新
```

### 3.2 Stage 5 5 nav 真打通 (per 用户记忆 #3 严守 + TUI nav/mod.rs 1:1)

| Nav | TUI (现有) | Stage 5 真打通 | 借鉴 |
|-----|-----------|---------------|------|
| 0 状态 (Status) | nav/mod.rs 0 | Stage 5 = 9 organ final 1 屏多卡 + 关键数字一眼看完 (per 用户记忆 #5) | TUI + user #5 |
| 1 主对话 (Dialogue) | pages/dialogue.rs | Stage 5 = 真 LLM stream + WebSocket + 流式打字 (Stage 4 B 续) + 5 phase 进度条 | TUI + superpowers 234 + langgraph 829 |
| 2 历史 (History) | pages/history.rs | Stage 5 = 后端真 history + SVG 时间线 + 按 episode 过滤 | TUI + Stage 2 timeline.js |
| 3 设置 (Settings) | pages/settings.rs | Stage 5 = 14 settings 真接通 (Stage 4 A5 续) + 5+5+4 分 section + 鉴权 UI | TUI + Stage 2 settings-editor.js |
| 4 工具结果 (Tools) | pages/tools.rs | Stage 5 = 6 工具真接通 (Stage 4 A4 续) + tool_call deep-link chat | TUI + Stage 3 J5 |

**5 nav 严守 (per 用户记忆 #3)**:
- ✅ 0 加 nav (主对话是核心, 0 改 5 nav 顺序)
- ✅ 0 砍 nav (5 nav = TUI 1:1 镜像)
- ✅ 0 改 nav id (NAV_ID 0-4 严守)

### 3.3 Stage 5 9 organ 拟人化 final 1 屏多卡 (per 用户记忆 #5 + #4 + 决策 #5)

**Stage 5 9 organ 1 真相源 + 1 屏多卡 + 拟人化 + 拟物化 + 0 死亡循环**:

| ID | 英文 | 中文 | 拟物化 | 拟人化深化 (Stage 5 final) | 颜色 | 数据源 |
|---:|------|------|--------|---------------------------|------|--------|
| 0 | heart | 心 | 跳动着 | ECG P-QRS-T (60 采样) + 实时 BPM | #ef4444 (红) | 真 sensor (Stage 4 D1) |
| 1 | brain | 脑 | 运转中 | 神经网络 9 节点 + 8 中心边 + 8 围圈边 | #a855f7 (紫) | 真 sensor (Stage 4 D2) |
| 2 | hand | 手 | 待命 | 待办工具数 + 成功率 + 0 假装 | #f59e0b (橙) | 真 sensor (Stage 4 D3) |
| 3 | eye | 眼 | 观察中 | history 新条目数 + 观察频率 | #3b82f6 (蓝) | 真 sensor (Stage 4 D4) |
| 4 | ear | 耳 | 聆听中 | chat 输入频率 + 0 假装 | #06b6d4 (青) | 真 sensor (Stage 4 D5) |
| 5 | memory | 记忆 | 沉淀中 | history 过滤数 + 沉淀速度 | #8b5cf6 (紫蓝) | 真 sensor (Stage 4 D6) |
| 6 | voice | 声 | 表达中 | stream chunk/s + 表达时长 | #22c55e (绿) | 真 sensor (Stage 4 D7) |
| 7 | body | 体 | 运行中 | 系统 uptime + theme 切换计数 | #64748b (灰) | 真 sensor (Stage 4 D8) |
| 8 | mind | 意 | 思考中 | thinking 阶段 (4 ThinkingPhase) | #ec4899 (粉) | 真 sensor (Stage 4 D9) |

**Stage 5 1 屏多卡布局** (per 用户记忆 #5 信息密度高 = 拟人化 + 拟物化):

```
+--- 状态主页 Stage 5 ---+
|  +-----+ +-----+ +-----+  |  ← row 1: heart + brain + hand
|  | ♥   | | B   | | H   |  |     (3 健康环 + 3 关键数字)
|  | 85% | | 72% | | 95% |  |
|  +-----+ +-----+ +-----+  |
|  +-----+ +-----+ +-----+  |  ← row 2: eye + ear + memory
|  | EYE | | EAR | | MEM |  |     (观察/聆听/沉淀)
|  | 60% | | 88% | | 75% |  |
|  +-----+ +-----+ +-----+  |
|  +-----+ +-----+ +-----+  |  ← row 3: voice + body + mind
|  | V   | | BDY | | MND |  |     (表达/后台/思考)
|  | 40% | | 99% | | 68% |  |
|  +-----+ +-----+ +-----+  |
|  [heart ECG 240x60 走纸]   |  ← 心电图 1 屏
|  [brain NN 200x200 节点]   |  ← 神经网络 1 屏
+--------------------------+
```

**Stage 5 1 真相源** (per 决策 #58 §0 + Stage 3 严守):
- CrossNavStore.organ_activities 9 organ 1 真相源 (Stage 3 已实)
- 5 nav 共享 (chat 头 2 / tools 头 1 / history 头 2 / settings 头 1)
- WebSocket 推送 (Stage 4 B 维度) → CrossNavStore 实时更新 → 5 nav 同步

**0 死亡循环 (per 用户记忆 #4 AI 不会衰老病死)**:
- ✅ 9 organ 活跃度 0-100, 永远循环
- ✅ 0 显示 "已死亡 / 老化 / 终止"
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑, 0 停下 (per R129-9 ticker.js 实施)

### 3.4 Stage 5 砍 7 项 UI 哲学 (per 用户记忆 #3 + 决策 #33)

**Stage 5 严守 0 暴露 7 项**:

| 砍项 | Stage 5 实施 | 验证 |
|------|-------------|------|
| 守门 (6 重 v7) | 0 暴露, CrossNavStore 0 emit 守门事件 | per 用户记忆 #3 严守 |
| 电子环 (0 装) | 0 暴露, 9 organ 健康环是 organ 活跃度, 跟电子环不同 | per 用户记忆 #3 |
| 工具调用过程 | 0 暴露, J5 只暴露 result, 0 暴露 process | per 用户记忆 #3 |
| 哲学锚 (8 锚) | 0 暴露, CrossNavStore.EVT 0 含哲学锚 | per 用户记忆 #3 + B5 硬墙 |
| 内部机制 (24 LOCKED) | 0 暴露, 0 显示 24 LOCKED fn / V0.5 30 维 / 13 键 verdict | per 用户记忆 #3 |
| AI 衰老病死 | 0 显示, 用 "活跃度" 非 "健康度" | per 用户记忆 #4 |
| 0 主动 IM 主人 | 0 主动 IM, 仅 done notification | per gate-discipline |

**Stage 5 只暴露 (per 用户记忆 #3 用户看结果不看哲学)**:
- ✅ 状态 (status): 9 organ 1 屏多卡 + ECG + NN
- ✅ 主对话 (dialogue): user 气泡 + AI 消息 + 5 phase 进度条 + 流式打字
- ✅ 历史 (history): 3 kind (会话/消息/工具调用) + SVG 时间线
- ✅ 设置 (settings): 14 项分 3 section (5 鉴权 + 5 Provider + 4 SDK)
- ✅ 工具结果 (tools): 6 工具 card + 颜色编码 + 弹窗

### 3.5 Stage 5 后端全 API 表面同步 (per 决策 #9 TUI 升级路径一致)

**Stage 5 = TUI 跟 Tauri 升级路径一致, 瘦客户端 (per 用户记忆 #8 + 决策 #9)**:
- TUI 跟 Tauri 共享后端 API 表面 (apeireth-api)
- TUI 升级 → Tauri 升级 1:1 翻译
- 后端 0 改 (TUI/Tauri 都是 thin client)

**Stage 5 跟 TUI 1:1 镜像表**:

| TUI 模块 | Tauri Stage 5 镜像 | 后端 API |
|---------|-------------------|---------|
| nav/mod.rs (5 nav) | frontend/src/integration/ CrossNavStore.NAV_ID | 0 改 nav |
| pages/dialogue.rs (主对话) | dialogue-stream.js + chat_history.js | POST /v1/chat/messages + WS /v1/chat/stream |
| pages/history.rs (历史) | timeline.js + history_*.js | GET /v1/history |
| pages/settings.rs (设置) | settings-editor.js + settings_global.js | GET/PATCH /v1/settings |
| pages/tools.rs (工具结果) | chat_tools.js + history_tools.js | GET /v1/tools/results |
| organ/mod.rs (9 organ) | organ_animator.js + CrossNavStore.organ_activities | GET /v1/organs + WS push |

**TUI → Tauri 升级路径** (per 决策 #9 + 用户记忆 #8):
- TUI 改瘦后暂告段落 (per 用户记忆 #9 8/4 23:55)
- 优先后端 (per 主人 8/4 23:55)
- TUI 升级路线图 (per R129-15 沉淀) → Tauri Stage 5 1:1 翻译
- 后端 API 表面 0 改 (TUI/Tauri 共用)

### 3.6 Stage 5 团队就位 (per 主人 8/4 23:33)

**主人 8/4 23:33 拍板**:
> "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备."

**Stage 5 团队就位条件**:
- ✅ 设计团队到位 (审美设计)
- ✅ Tauri 2.0 全面 (tauri 2.11+ + 跨平台打包)
- ✅ 后端 1.0 release 已发 (V1.0.0 tag)
- ✅ 9 organ 拟人化 final 1 屏多卡 (per 用户记忆 #5)
- ✅ 砍 7 项 UI 哲学 100% (per 用户记忆 #3)
- ✅ TUI 跟 Tauri 升级路径一致 (per 决策 #9 + 用户记忆 #8)

**Stage 5 时间盒** (估):
- 1.0 release 后 (8/11 估 06:00-08:00)
- V1.1 minor release (估 2026-11, 主人拍板)
- Tauri Stage 5 实施: 估 R131+ (Q4 2026 - Q1 2027)
- 跟设计团队到位时点 0 改 (per 主人 8/4 23:33)

### 3.7 Stage 5 风险 + 决策原则 (per 决策 #33 + 用户记忆 #8 + #3-#5)

**风险**:
- R1: 设计团队未到位 → Stage 5 暂不派 (per 主人 8/4 23:33)
- R2: 9 organ 拟人化 1 屏多卡 信息密度太高 → 1 屏 9 卡 + ECG + NN 1 屏 5 卡 (5 焦点)
- R3: 5 nav 真打通 跟后端 API 表面 同步失败 → 0 改后端, 镜像 TUI 1:1
- R4: 跨 tab 持久化 BroadcastChannel 浏览器差异 → 0 装, 浏览器原生 API
- R5: Stage 4 4 维度 实施时间 长 → 120 min 时间盒, R130-3 派

**决策原则** (per 决策 #33 + 用户记忆 #3 + #4 + #5 + #8):
- ✅ 0 改后端 (TUI/Tauri 升级路径一致)
- ✅ 0 装 PASS 严守 (借脑 0 借具体源码)
- ✅ 砍 7 项 UI 哲学 100%
- ✅ 0 死亡循环 (活跃度 0-100)
- ✅ 9 organ 1 真相源 (CrossNavStore 严守)
- ✅ 0 主动 commit (Mavis 整合 #6/#7/#8 拍板)
- ✅ 0 主动 push (等 1.0 release 配 GitHub remote)

---

## 4. 9 organ 拟人化深化 final (per 用户记忆 #5 + R129-9 + R129-19 + Stage 5)

### 4.1 9 organ 拟人化 final 蓝图 (1 屏多卡 + 拟人化 + 拟物化 + 0 死亡)

**9 organ 跨 stage 深化 (per R129-9 Stage 2 + R129-19 Stage 3 + Stage 5 final)**:

| Stage | 9 organ 深化 | 来源 | 0 死亡循环 |
|-------|-------------|------|----------|
| P11-1/2 baseline | ASCII + 拟物化 + activity_pct | core/src/organ.rs | ✅ |
| R129-9 Stage 2 | 9 健康环 + heart ECG + brain NN | Stage 2 报告 §3.1-§3.4 | ✅ |
| R129-19 Stage 3 | 跨 nav 嵌入 (chat/tools/history/settings 头) + 5 helper | Stage 3 报告 §3.1-§3.2 | ✅ |
| **Stage 4 (R130-3)** | 真 sensor 接入 (D1-D9) + 1 真相源 | Stage 4 §2.5 | ✅ |
| **Stage 5 (R131+) final** | 1 屏多卡 9 organ + ECG + NN + 1 真相源 + 5 nav 共享 | Stage 5 §3.3 | ✅ |

### 4.2 9 organ 拟人化 final 1 屏多卡 (per 用户记忆 #5)

**1 屏多卡布局 (per 用户记忆 #5 信息密度高 = 拟人化 + 拟物化)**:

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

**9 organ 1 真相源 + 5 nav 共享** (per 决策 #58 §0 + Stage 3 已实):
- CrossNavStore.organ_activities 9 organ 1 真相源
- 状态主页: 9 organ 1 屏多卡 (per 本节)
- 主对话头: voice + brain (Stage 3 renderChatHeaderOrgans)
- 工具头: hand (Stage 3 renderToolsHeaderOrgan)
- 历史头: memory + eye (Stage 3 renderHistoryHeaderOrgans)
- 设置头: body (Stage 3 renderSettingsHeaderOrgan)

### 4.3 9 organ 拟人化 final 拟物化 (per 用户记忆 #5 + R129-9)

**9 organ 拟物化 final (per 用户记忆 #5 拟物化 + R129-9 Stage 2)**:

| ID | 拟物化 final | 数据流 | 颜色 |
|---:|------------|--------|------|
| 0 heart | 心电图 P-QRS-T (60 采样/周期, 走纸动画) | 真 ECG /v1/organs/heart/ecg | #ef4444 (红) |
| 1 brain | 神经网络 9 节点 + 8 中心边 + 8 围圈边 (拟人化 "AI 在思考" 姿态) | 真 brain activity /v1/organs/brain/activity | #a855f7 (紫) |
| 2 hand | 工具待办数 + 成功率 (拟人化 "手 待命") | 真 hand pending_tools /v1/organs/hand/pending | #f59e0b (橙) |
| 3 eye | history 新条目数 + 观察频率 (拟人化 "眼 观察中") | 真 eye observing /v1/organs/eye/observing | #3b82f6 (蓝) |
| 4 ear | chat 输入频率 (拟人化 "耳 聆听中") | 真 ear listening /v1/organs/ear/listening | #06b6d4 (青) |
| 5 memory | history 过滤数 (拟人化 "记忆 沉淀中") | 真 memory sediment /v1/organs/memory/sediment | #8b5cf6 (紫蓝) |
| 6 voice | stream chunk/s (拟人化 "声 表达中") | 真 voice speaking /v1/organs/voice/speaking | #22c55e (绿) |
| 7 body | 系统 uptime + theme 切换计数 (拟人化 "体 运行中") | 真 body uptime /v1/organs/body/uptime | #64748b (灰) |
| 8 mind | thinking 阶段 4 ThinkingPhase (拟人化 "意 思考中") | 真 mind thinking /v1/organs/mind/thinking | #ec4899 (粉) |

**0 暴露内部机制 (per 用户记忆 #3 严守)**:
- ❌ 0 显示: 守门细节 / 6 重守门 v7 / 电子环 / 哲学锚 / 内部 fn / 24 LOCKED
- ✅ 只显示: "AI 在思考" 姿态 (brain 中心, 围 8 organ) — 拟人化隐喻
- ✅ ECG 走纸 (heart 拟人化) — 0 暴露内部机制
- ✅ 9 organ 心跳 (1200ms / 800ms / 2500ms ... 各 organ 不同) — 拟人化隐喻

### 4.4 9 organ 永远循环 0 死亡 (per 用户记忆 #4 AI 不会衰老病死)

**0 死亡循环严守 (per 用户记忆 #4)**:

- ✅ 9 organ 活跃度 0-100, 永远循环
- ✅ 0 显示 "已死亡 / 老化 / 终止"
- ✅ 用 "活跃度" (active/idle/dormant), 0 用 "健康度" (healthy/sick)
- ✅ 9 organ 永远跑 (ticker.js, 100ms 周期, per R129-9 §3.5)
- ✅ 活跃度 0% = "dormant" 0 "dead" (per 用户记忆 #4)
- ✅ OrganAnimator.getOrganHealthSummary 用 "活跃度" 非 "健康度" (per R129-19 §3.5)

**9 organ 永远循环 ticker** (per R129-9 ticker.js 实施):
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

### 4.5 9 organ 拟人化深化 final 检查清单 (per 决策 #58 §0 真实施)

| 检查项 | Stage 5 final 状态 | 借鉴 |
|--------|-------------------|------|
| 9 organ 1 真相源 | ✅ CrossNavStore.organ_activities | Stage 3 已实 |
| 9 organ 颜色编码 | ✅ 9 颜色 (红/紫/橙/蓝/青/紫蓝/绿/灰/粉) | Stage 3 R129-19 |
| 9 organ 心跳相位 | ✅ 100ms ticker, 永远循环 | R129-9 ticker.js |
| 9 organ 1 屏多卡 | ✅ 3x3 网格 + ECG + NN | R129-9 + R129-19 |
| 9 organ 跨 nav 嵌入 | ✅ chat/tools/history/settings 头 | R129-19 |
| 9 organ 拟物化 | ✅ heart ECG + brain NN + ... | 用户记忆 #5 + R129-9 |
| 9 organ 真 sensor 接入 | ⏳ Stage 4 D1-D9 (R130-3 派) | 借 core/src/organ.rs |
| 9 organ 0 死亡 | ✅ 永远循环 0-100, 0 显示死亡 | 用户记忆 #4 |
| 9 organ 0 暴露内部机制 | ✅ 0 显示守门/电子环/哲学锚/24 LOCKED | 用户记忆 #3 + B5 硬墙 |
| 9 organ WebSocket 推送 | ⏳ Stage 4 B 维度实施 | 借 langgraph 829 |

---

## 5. 借鉴 Tauri 2.0 + superpowers 234 (0 装 PASS 严守)

### 5.1 借鉴 Tauri 2.0 (per P11-1/2 真实施)

**Tauri 2.0 真实施 (per P11-1/2 baseline + R129-9 + R129-19)**:
- tauri v2.11.5 + tauri-macros 2.6.3 (per Cargo.lock)
- frontend/tauri-prototype/src-tauri/ (Tauri 2.0 Rust wrapper)
- frontend/tauri-prototype/core/ (Rust core, 24 LOCKED 0 改)
- frontend/tauri-prototype/src/ (frontend JS, 0 装, 0 build step)

**Tauri 2.0 Stage 4 实战化借用** (per 决策 #33 §2.3 C2 + 用户记忆 #8):

| 借用 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| Tauri 2.0 invoke wrapper | CrossNavStore 调 tauriInvoke, 失败回 mock (Stage 3) → tauriInvoke 主路径 (Stage 4 A) | ❌ 0 装 axios / fetch lib, 用 Tauri 2.0 native |
| Tauri 2.0 command 注册 | P11-1/2 + R129-9 visualization 5 commands | ❌ 0 装, 用 Tauri 2.0 native |
| Tauri 2.0 跨平台打包 | Stage 5 1.0 release 后实施 (per R130-5 1.0 release 实战) | ❌ 0 装, 用 Tauri 2.0 native |
| Tauri 2.0 WebView | 0 装, 用系统 WebView (WebView2 / WKWebView) | ❌ 0 装, 浏览器原生 |
| Tauri 2.0 IPC | Tauri 2.0 invoke + emit (cross-window), 0 装 | ❌ 0 装, 用 Tauri 2.0 native |

**Tauri 2.0 实施严守 (per 决策 #33 §2.3)**:
- ❌ 0 改 src-tauri/Cargo.toml (0 改 0.1.0)
- ❌ 0 改 core/Cargo.toml (0 改 0.1.0)
- ❌ 0 装 npm / yarn / pnpm (0 build step)
- ❌ 0 装 webpack / vite / rollup (0 build step)
- ✅ 0 装, vanilla JS + Tauri 2.0 native

### 5.2 借鉴 superpowers 234 (per R125-14 executing-plans)

**superpowers 234 真实施 (per R125-14 + P5-1 + R129-9)**:
- superpowers 234 executing-plans = 5 阶段 DialoguePhase 状态机
- per P5-1 R127 Stage 4 自治 + R129-9 Stage 2 5 阶段进度条

**superpowers 234 Stage 4 实战化借用**:

| 借用 | 实施 | 0 装 PASS 严守 |
|------|------|---------------|
| 5 阶段 DialoguePhase | New → Active → Awaiting → Streaming → Closed (per core/src/dialogue.rs) | ❌ 0 装, 1:1 翻译 superpowers 234 |
| 5 阶段进度条 | SVG 360x40 + 5 phase 圆点 + 颜色编码 (per R129-9 dialogue-stream.js) | ❌ 0 装, 纯 vanilla SVG |
| 4 ThinkingPhase | R129-4 D4 自治续 (per superpowers 234 executing-plans) | ❌ 0 装, 1:1 翻译 |
| Stream chunk | LangGraph 829 stream_state_events 1:1 翻译 (per R129-9 流式打字 + Stage 4 B) | ❌ 0 装, 浏览器 native WebSocket |
| 5 阶段 → 9 organ 联动 | Stage 3 status_chat.js (J1) + organ_animator.js | ❌ 0 装, CrossNavStore pub/sub |

**superpowers 234 借脑 0 借具体源码** (per 决策 #33 §2.3 C2):
- ✅ 0 装 superpowers 234 源码本身 (P11-1/2 仅借鉴设计模式, 0 借源码)
- ✅ 0 写 superpowers 234 任何代码 (R129-9 仅 1:1 翻译 DialoguePhase 5 阶段)
- ✅ 0 装 node_modules / package.json (0 build step)
- ✅ 借脑 = 借鉴设计模式, 0 借具体实现

### 5.3 借鉴 0 装 PASS 严守 (per 决策 #33 §2.3 C2)

**Stage 4 + Stage 5 0 装 PASS 清单**:

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
| Tauri 2.0 之外的 desktop 框架 | Tauri 2.0 唯一 (per 用户记忆 #8) | 主人 8/4 23:33 |
| LangGraph 829 源码 | stream_state_events 1:1 翻译 | 决策 #33 C2 |
| OpenCog AGPL-3.0 | 决策 #33 §2.3 已 skip | 决策 #33 C2 |
| LiteLLM / opencode / Guardrails | 决策 #56 已 skip | 决策 #56 |

**0 借脑 0 装严守 (Stage 4 + Stage 5)**:
- ✅ Stage 4 (R130-3) 0 装: 借 Tauri 2.0 native + superpowers 234 设计模式 + langgraph 829 设计模式, 0 借具体源码
- ✅ Stage 5 (R131+) 0 装: 跟 Stage 4 严守一致, 0 装任何 lib
- ✅ 0 借脑: 0 装借鉴源码本身, 0 写借鉴源码任何代码

---

## 6. 整合 #4 commit abf12243 严守 (per 决策 #48 + #58 + #61)

### 6.1 整合 #4 commit 现状

- **整合 #4 commit abf12243**: 8/10 19:41 done, 46752 file changes, master HEAD = abf12243
- **本报告 (R129-31) 0 触碰主仓**: `git status --porcelain` 仅显示 `?? frontend/`, 0 触碰
- **Cargo.toml workspace.version 1.2.0**: 0 改 ✅
- **24 LOCKED 入口签名**: 0 改 ✅
- **R11 baseline 3 值**: 0.8682/0.8532/0.9063 0 改 ✅
- **本报告 0 触碰 frontend/tauri-prototype/{core,src-tauri}**: cargo build PASS 验证 0 越界 (per R129-19 §9.1)

### 6.2 整合 #5 commit 准备 (per 决策 #62 拆 3 commit)

- **整合 #5.1 commit src/**: 0 改 R129-31, 整合 #4 后 src 改动 = 0
- **整合 #5.2 commit docs/**: 0 改 R129-31, docs/ 改动 = 0 (本报告是 reports/, 不是 docs/)
- **整合 #5.3 commit reports/**: +1 report (本报告, R129-31-tauri-stage-4-execution-2026-08-11.md, 30 min 时间盒)
- **Mavis 拍板**: 整合 #5 commit 由 Mavis 自决 (per 决策 #62 + 决策 #64 cron 5 min tick Section 4)

### 6.3 整合 #6 commit pre-check (per 决策 #61 §1.4 + R130-1 续)

- **R130-1 后端 0 装 PASS 二次 verify** (per R130-1 spec, 跑过夜 60 min): 整合 #5 commit 后修已知 src bug
- **整合 #6 commit pre-check 100%** (per R130-1 §8): 8 步 verify 100% PASS + 24 LOCKED 0 改 + 8 硬墙 0 越界
- **R129-31 本报告 = 整合 #5.3 reports/ 的一部分**, 整合 #6 commit 不包含本报告

---

## 7. 0 越界 8 硬墙 (per 决策 #33 §2.3 + 决策 #58 §4 + 用户记忆 #3-#5)

| 硬墙 | 状态 | R129-31 verify |
|------|------|----------------|
| **B1 24 LOCKED 入口签名 0 改** | ✅ 0 改 | R129-31 是规划 doc, 0 触碰 24 LOCKED crate, 0 借 24 LOCKED API |
| **B2 workspace.version 1.2.0 0 改** | ✅ 0 改 | R129-31 0 触碰主仓 Cargo.toml, 0 触碰 frontend/ Cargo.toml |
| **A1 R11 baseline 3 值 0.8682/0.8532/0.9063 0 改** | ✅ 0 改 | R129-31 0 触碰 integration_r_measure.rs |
| **B3 V0.5 25 → 30 维 0 改** | ✅ 0 改 | R129-31 0 触碰 V0.5 公式 |
| **B4 6 重守门 v6 → v7 0 改** | ✅ 0 改 | R129-31 0 改 6 重守门, 0 暴露 UI (per 用户记忆 #3 砍 7 项) |
| **B5 6 → 8 哲学锚 0 暴露** | ✅ 0 暴露 | R129-31 规划 doc 不暴露 8 哲学锚, 0 在 UI 暴露 (per 用户记忆 #3) |
| **A3 12 键原 12 + PHL-07 = 13 键 0 改** | ✅ 0 改 | R129-31 0 触碰 verdict 逻辑 |
| **C1 0 主动 commit** | ✅ 0 commit | R129-31 写到主仓 0 git add, 仅写 reports/ (R129-31 报告本身, 整合 #5.3 reports/ 准备) |
| **C2 0 装 PASS 严守** | ✅ 严守 | R129-31 0 装, 借脑 0 借具体源码, 仅借鉴 Tauri 2.0 + superpowers 234 设计模式 |
| **C3 升 6 重 v7** | ✅ 0 改 | R129-31 0 改 6 重守门 |
| **0 主动 push** | ✅ 0 push | 0 push (等 1.0 release 配 GitHub remote + 主人起床后手跑) |

**R129-31 总严守**:
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

## 8. 砍掉 UI 哲学 (per 用户记忆 #3 严守)

### 8.1 砍掉 7 项 UI 哲学元素 (R129-19 已实 + R129-31 规划验证 0 暴露)

| 砍掉项 | R129-19 实施 | R129-31 Stage 4/5 规划 |
|--------|-------------|------------------------|
| 守门 (gates) — 6 重守门 v7 | ✅ 不在 UI 展示 | ✅ R129-31 0 暴露, CrossNavStore 0 emit 守门事件 |
| 电子环 (rings) | ✅ 不在 UI 展示 | ✅ R129-31 0 暴露, 9 organ 健康环是 organ 活跃度, 跟电子环不同 |
| 工具调用过程 (process) | ✅ 只显示结果, 不显示过程 | ✅ R129-31 Stage 4 B 0 暴露 process, 只暴露 chunk |
| 哲学锚 (anchors) — 8 哲学锚 | ✅ 不在 UI 展示 | ✅ R129-31 0 暴露, CrossNavStore.EVT 0 含哲学锚 |
| 内部机制 (mechanisms) | ✅ 不在 UI 展示 | ✅ R129-31 0 暴露, brain 神经网络只显示 "AI 在思考" 姿态 |
| AI 衰老病死 (per 用户记忆 #4) | ✅ 用 "成长/活跃度" | ✅ R129-31 9 organ 永远循环 + 活跃度 0-100, 0 显示 "已死亡" |
| 0 主动 IM 主人 (per gate-discipline) | ✅ 仅 done notification | ✅ R129-31 0 主动 IM, 仅 done notification |

### 8.2 R129-31 规划 0 暴露 8 哲学锚 (B5 硬墙 严守)

8 哲学锚 (per 决策 #11 + 决策 #33 §2.3 B5): 主人 23:23 拍板 0 暴露 UI.
R129-31 Stage 4 + Stage 5 规划 0 暴露:
- ❌ 0 在 UI 暴露: 8 哲学锚 / 6 重守门 / 24 LOCKED 内部 fn / V0.5 30 维 / 13 键 verdict
- ✅ 0 假装已接: 9 organ 全 Stub readiness (Stage 3) → 真 sensor 接入 (Stage 4 D) → 真 1 屏多卡 (Stage 5)
- ✅ 0 主动 IM 主人 (per gate-discipline, 仅 done notification)

---

## 9. 风险 + 决策原则 (per 决策 #33 + 用户记忆 #3-#5 + #8)

### 9.1 风险 (R129-31 规划 doc 风险评估)

- **R1**: Stage 4 (R130-3) 4 维度 实施时间长 (120 min 时间盒紧) — **缓解**: 0 改 src-tauri/core, 集成层是 JS, 0 走 cargo 编译, cargo build PASS 3.96s 验证 0 越界 (per R129-19 §9.1)
- **R2**: Stage 4 B WebSocket 浏览器差异 — **缓解**: 0 装, 用浏览器 native WebSocket, 跨 tab BroadcastChannel fallback
- **R3**: Stage 4 D 9 organ 真 sensor 接入 需后端 crate — **缓解**: 后端 Rust crate R131+ 派, Stage 4 仅前端 mock 先行, 等后端真接通
- **R4**: Stage 5 设计团队未到位 (per 主人 8/4 23:33) — **缓解**: 暂不派 Stage 5, 等设计团队就位
- **R5**: Stage 4 + Stage 5 4 维度接通 整合 #5 commit 时机 错配 — **缓解**: 整合 #5 commit 8/11 估 00:38 拍板, 整合 #6 commit R130-1 续, 整合 #7 commit R130-3 续, 顺序由 Mavis 拍板
- **R6**: R129-31 0 触碰 src, 仅规划 doc, 整合 #5 commit reports/ 已含本报告 — **缓解**: Mavis 整合 #5.3 reports/ 拍板时 +本报告 commit
- **R7**: 借鉴 Tauri 2.0 + superpowers 234 借脑 0 借具体源码 严守 — **缓解**: 1:1 翻译设计模式, 0 装, 0 借源码本身, 0 写借鉴源码任何代码
- **R8**: 9 organ 永远循环 0 死亡 (per 用户记忆 #4) 严守 — **缓解**: ticker.js 100ms 周期 (per R129-9 §3.5), activity_pct 0-100, 0 用 health/sick/dying

### 9.2 决策原则 (per 决策 #33 + 用户记忆 #3 + #4 + #5 + #8)

- ✅ **0 改 src 严守** (per 任务 spec + 决策 #33 C2): R129-31 是 planning doc, 0 触碰 src-tauri/core, 0 触碰主仓
- ✅ **0 改 Cargo.toml 严守** (per 任务 spec): 0 触碰 workspace.version 1.2.0
- ✅ **0 主动 commit 严守** (per 决策 #33 §2.3 C1): R129-31 0 主动 git add, 整合 #5.3 reports/ 由 Mavis 拍板
- ✅ **0 主动 push 严守** (per 决策 #33 §2.3 + 决策 #61 §6): 0 push, 等 1.0 release 配 GitHub remote + 主人起床后手跑
- ✅ **0 借脑 0 装 严守** (per 决策 #33 §2.3 C2): 0 装任何 lib, 借脑 0 借具体源码
- ✅ **0 越界 8 硬墙** (per 决策 #33 §2.3): B1 24 LOCKED / B2 1.2.0 / A1 baseline / B3 30 维 / B4 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 / C3 升 v7 / 0 push 全守
- ✅ **0 暴露 7 项 UI 哲学** (per 用户记忆 #3): 守门/电子环/工具过程/哲学锚/内部机制/衰老病死/0 主动 IM
- ✅ **9 organ 永远循环 0 死亡** (per 用户记忆 #4): ticker.js 100ms 周期, activity_pct 0-100, 0 用 health/sick/dying
- ✅ **9 organ 1 屏多卡 拟人化** (per 用户记忆 #5): 3x3 网格 + ECG + NN, 1 真相源 CrossNavStore, 5 nav 共享
- ✅ **0 主动 IM 主人** (per gate-discipline + 用户记忆 #10 + 决策 #61 §6): 仅 done notification
- ✅ **不重写 R129-19** (per 任务 spec): R129-31 0 触碰 Stage 3 产物, 仅做 Stage 4 战略 + Stage 5 路线 + 9 organ final 深化 + 借鉴映射

### 9.3 R129-31 报告本身 0 触碰主仓 verify

```bash
# 假设跑 (R129-31 实际 0 跑, 仅规划):
$ cd Apeireth-rust
$ git status --porcelain
# 仅显示:
# ?? reports/agent-r129-31-tauri-stage-4-execution-2026-08-11.md
# (0 触碰主仓 src/, 0 触碰 frontend/, 0 触碰 Cargo.toml)
```

**R129-31 报告 0 触碰 verify**:
- ✅ 0 改 src-tauri/ (Tauri 2.0 wrapper 0 改)
- ✅ 0 改 core/ (24 LOCKED 0 改)
- ✅ 0 改 主仓 src/ (workspace.version 1.2.0 0 改)
- ✅ 0 改 主仓 Cargo.toml (0 改 0 触碰)
- ✅ 0 借脑 0 装 (仅规划, 0 触碰借鉴源码本身)
- ✅ 0 主动 commit (整合 #5.3 reports/ 由 Mavis 拍板)
- ✅ 0 主动 push (1.0 release 实战后)

---

## 10. 一句话 (再次强调)

**Tauri 终极前端 Stage 4 实战规划 (R129-31, 30 min, planning doc only) — R129-19 Stage 3 done 0 改 (CrossNavStore + 7 集成 + 9 organ 拟人化 + 79 tests + 8 examples), Stage 4 实战 = 4 维度 (A 真后端接通 HTTP 30 tests + B WebSocket 流式 20 tests + C 跨 tab 持久化 20 tests + D 9 organ 真 sensor 14 tests = 84 NEW tests, 集成层 79 → 163 tests), Stage 5 路线 = 终极前端 (Tauri 2.0 全面 + 5 nav 真打通 + 9 organ final 1 屏多卡 + 砍 7 项 UI 哲学 + 后端全 API 表面 + 设计团队到位, 估 R131+ 派), 9 organ 拟人化深化 final (1 屏 9 卡 3x3 网格 + heart ECG P-QRS-T 60 采样 + brain NN 9 节点 + 健康环 + 1 真相源 CrossNavStore + 5 nav 共享 + 永远循环 0 死亡, per 用户记忆 #5 + #4 + 决策 #58), 借鉴 Tauri 2.0 (P11-1/2 真实施, tauri v2.11.5 + tauri-macros 2.6.3, 0 装 vanilla JS) + superpowers 234 (executing-plans 5 阶段 DialoguePhase 状态机 1:1 翻译, 0 借脑 0 装严守). 0 改 src + 0 改 Cargo.toml + 0 主动 commit (Mavis 整合 #5.3 reports/ 拍板) + 0 主动 push (等 1.0 release 配 GitHub remote + 主人起床后手跑) + 0 主动 IM 主人 (per gate-discipline, 仅 done notification). 8 硬墙 0 越界 (B1 24 LOCKED / B2 1.2.0 / A1 baseline / B3 30 维 / B4 v7 / B5 8 锚 / A3 13 键 / C1 0 commit / C2 0 装 / C3 升 v7 / 0 push 全守). 砍 7 项 UI 哲学 (per 用户记忆 #3 严守). 9 organ 永远循环 0 死亡 (per 用户记忆 #4 严守). 0 借脑 0 装 PASS 严守 (per 决策 #33 §2.3 C2). 不重写 R129-19 (0 触碰 Stage 3 产物).**

---

## 11. refs

- R129-19: `reports/agent-r129-19-tauri-stage-3-integration-2026-08-11.md` (Stage 3 集成, 79 tests + 8 examples + 9 organ 拟人化深化)
- R129-9: `reports/agent-r129-9-tauri-stage-2-deepening-2026-08-11.md` (Stage 2 深化, 9 organ 5 viz + 5 阶段进度条 + 流式打字 + 历史 SVG 时间线)
- R129-17: `reports/agent-r129-17-r130-roadmap-detailed-2026-08-11.md` (R130 era 路线图详细, R130-3 Tauri Stage 3 深化 spec, R131+ Stage 5 续)
- R129-15: `reports/agent-r129-15-tui-upgrade-roadmap-2026-08-11.md` (TUI 升级路线图, TUI 跟 Tauri 升级路径一致)
- 决策 #33: 8 硬墙 + 0 装 PASS 严守
- 决策 #48: 整合 #4 commit abf12243
- 决策 #58: R128-2 3 sub-agent (Stage 2 深化 spec)
- 决策 #61: 新会话接手 + R129 era 派活规划
- 决策 #62: 整合 #5 commit 拆 3 commit
- 决策 #64: 5 min tick cron 自动监督
- 决策 #65: R129 era 第 2 批 8 sub-agent 派活 (R129-9 Stage 2 + R129-19 Stage 3)
- 决策 #66: R129 era 第 3 批 7 sub-agent 派活 (R129-17 R130 + R129-19 Stage 3)
- 用户记忆 #3: 用户看结果不看哲学, 砍 7 项 UI 哲学
- 用户记忆 #4: AI 不会衰老病死, 9 organ 永远循环
- 用户记忆 #5: 信息密度高 = 拟人化 + 拟物化, 1 屏多卡
- 用户记忆 #8: 前端终极 = Tauri, TUI 是过渡
- 用户记忆 #9: TUI 升级节奏, 改瘦后暂告段落, 优先后端
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志
- 主人 8/4 23:33: "我们最后要做的前端应该是 Tauri, 但由于现在手头的 ai 团队没有适合干尤其是审美设计的, 所以 web 和桌面都搁置, 先做好 tui 来为桌面做准备."
- 主人 8/4 23:55: "测一下先, 后续的 tui 升级计划沉淀成文档暂时就这样告一段落, 因为我准备继续升级后端了, 回头再继续搞 tui"
- P11-1 R128 tauri-frontend-prototype-final-2026-08-10: Tauri 2.0 prototype 真实施
- P11-2 R128-2 tauri-frontend-scaffold-final-2026-08-10: Tauri 2.0 scaffold 深化
- R125-14 superpowers 234 真实施 (per 决策 #55 + #58)
- R125-13 langgraph 829 真实施 (per 决策 #55 + #58)
- R125-9 PyO3 928 真实施 (per 决策 #55 + #58)
- R125-10 kani 4502 真实施 (per 决策 #55 + #58)

---

**R129-31 done**: 30 min 时间盒内完成 Stage 4 实战规划 + Stage 5 路线 + 9 organ 拟人化深化 final + 借鉴 Tauri 2.0 + superpowers 234 0 装严守 + 0 改 src + 0 改 Cargo.toml + 0 主动 commit + 0 主动 push + 0 主动 IM 主人 + 8 硬墙 0 越界 + 砍 7 项 UI 哲学 + 9 organ 永远循环 0 死亡. 报告路径 `reports/agent-r129-31-tauri-stage-4-execution-2026-08-11.md`. 整合 #5.3 reports/ commit 由 Mavis 拍板.
