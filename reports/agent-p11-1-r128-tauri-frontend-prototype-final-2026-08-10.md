# Agent P11-1 — R128 阶段 B Tauri 终极前端 prototype (final, 2026-08-10)

**Date**: 2026-08-10 22:00 (per 决策 #57 §2.2 21:29 派活)
**Author**: Mavis sub-agent P11-1 (root session mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**任务**: Tauri 2.0 终极前端 prototype (5 nav + 主对话 + 9 organ 拟人化 stub)
**派活依据**: 决策 #57 §2.2 R128 阶段 B 派 6 sub-agent
**整合 #4 commit**: abf12243 (19:41 done, 严守 0 重跑)
**报告路径**: `Apeireth-rust/reports/agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md`
**实现路径**: `Apeireth-rust/frontend/tauri-prototype/`

---

## 0. 一句话

**Apeireth 终极前端 prototype 完成 — Tauri 2.0 桌面 app 骨架, 5 nav (状态/主对话/历史/设置/工具结果) + 9 organ 拟人化 (心/脑/手/眼/耳/记忆/声/体/意) + 主对话 (借鉴 superpowers 234 executing-plans) + 6 工具结果 + 14 设置. core lib 72 tests PASS (61 unit + 11 integration, 0.01s 跑完, 0 依赖, 0 改主仓), Tauri 2.0 = ⏳ 限流 = 准备 (本地 cargo 缓存不含 tauri 2.x, full build pending, 0 装 PASS 严守). 0 主动 commit + 0 主动 push 严守.**

---

## 1. 借鉴 + 设计原则 (per 决策 #57 §2.2 + 决策 #36 §1.1 + 用户记忆 #3-#5)

### 1.1 借鉴源码

| 借鉴 | 状态 | 实施 |
|---|---|---|
| Tauri 2.0 desktop framework | ⏳ 限流 = 准备 | 结构完整, deps pending, per 决策 #33 §2.3 C2 + #57 §3 |
| superpowers 234 executing-plans | ✅ 已 clone | 主对话 4 phase 翻译对齐 (思考中/执行中/完成) |
| TUI nav/mod.rs (5 nav 编译期 hardcode) | ✅ 1:1 镜像 | core/src/nav.rs 严守 5 nav |
| TUI organ/mod.rs (9 organ 拟人化) | ✅ 1:1 镜像 | core/src/organ.rs 严守 9 organ |
| TUI pages/dialogue.rs (主对话 user/AI 气泡) | ✅ 借鉴 | core/src/dialogue.rs |
| TUI 6 工具 endpoint (calendar/message/contact/task/search/drive) | ✅ 严守 | core/src/tools.rs 6 工具 |
| TUI 5 鉴权 + 5 Provider + 4 SDK | ✅ 严守 | core/src/settings.rs 14 设置 |
| 用户记忆 #3 (5 nav + 砍 7 项 UI 哲学元素) | ✅ 严守 | nav 顺序 + 砍 守门/电子环/工具过程 |
| 用户记忆 #4 (AI 不会衰老病死) | ✅ 严守 | 9 organ 用"成长/活跃度"非"衰老/健康度" |
| 用户记忆 #5 (9 organ 拟人化 + 拟物化) | ✅ 严守 | 1 屏 9 cards, 拟人化 + 拟物化 (心跳/脑波/手操作) |
| 用户记忆 #8 (终极 = Tauri, TUI = 过渡) | ✅ 严守 | 瘦客户端, TUI 升级路径一致 |

### 1.2 设计原则 (per 用户记忆 #3-#5)

- **5 nav** (per 用户记忆 #3 严守): 状态 / 主对话 / 历史 / 设置 / 工具结果
- **9 organ 拟人化** (per 用户记忆 #5): 心/脑/手/眼/耳/记忆/声/体/意 + ASCII 字符 + 拟物化短语
- **1 屏多卡片** (per 用户记忆 #5): 9 organ 紧凑 3x3 网格, 关键数字一眼看完
- **砍 7 项 UI 哲学元素** (per 用户记忆 #3): 守门/电子环/工具调用过程/哲学锚/内部机制 全部不在 UI 展示
- **AI 不会衰老病死** (per 用户记忆 #4): 用"成长/活跃度"指标, 不显示死亡/老化
- **状态为主页, 不是"功能列表"** (per 用户记忆 #5): Status 是首页, 9 organ 拟人化
- **真 src 改动 + tests pass** (per 决策 #57 §6 0 装 PASS 严守): core lib 72 tests pass, 0 假装"已实施"
- **瘦客户端** (per 用户记忆 #8 + 决策 #22 §1.4): Tauri = 渲染层, 业务逻辑在 core, TUI/Tauri 升级路径一致

---

## 2. 实施成果 (per 决策 #57 §6 真 src 改动 + tests pass)

### 2.1 项目结构 (21 files + .gitignore)

```
frontend/tauri-prototype/                              # 总 21 文件 (0 改主仓)
├── README.md (7.1 KB)                                 # 入口 + 任何人接手
├── .gitignore                                         # exclude target/ + Cargo.lock + icons
├── docs/
│   └── STRUCTURE.md (8.5 KB)                          # 架构图 + 9 节详细说明
├── core/                                              # ✅ 真实施 (72 tests pass)
│   ├── Cargo.toml (525 B)                             # 独立 crate (0 workspace, 0 改主仓)
│   ├── src/
│   │   ├── lib.rs (2.5 KB)                            # re-exports + 模块总览
│   │   ├── organ.rs (13.9 KB, 14 tests)               # 9 organ 1:1 镜像 TUI organ/mod.rs
│   │   ├── nav.rs (9.1 KB, 10 tests)                  # 5 nav 严守用户记忆 #3
│   │   ├── dialogue.rs (10.9 KB, 11 tests)            # 主对话 (借鉴 superpowers 234)
│   │   ├── tools.rs (8.9 KB, 9 tests)                 # 6 工具 (per TUI 6 工具 endpoint)
│   │   ├── settings.rs (7.8 KB, 7 tests)              # 14 设置 (5+5+4)
│   │   └── history.rs (4.3 KB, 6 tests)               # 3 kind (会话/消息/工具调用)
│   └── tests/
│       └── integration_test.rs (6.0 KB, 11 tests)     # 跨模块守门
├── src-tauri/                                         # ⏳ 限流 = 准备 (Tauri 2.0 = 0 装)
│   ├── Cargo.toml (1.5 KB)                            # tauri = "2" + core path
│   ├── tauri.conf.json (1.3 KB)                       # 5 nav 窗口 + 5 icons
│   ├── build.rs (244 B)                               # tauri_build::build()
│   ├── capabilities/
│   │   └── default.json (444 B)                       # Tauri 2.0 8 permissions
│   ├── icons/
│   │   └── README.md (882 B)                          # 5 图标 placeholder (P12-1 阶段 1)
│   └── src/
│       ├── main.rs (460 B)                            # Tauri entry
│       └── lib.rs (6.2 KB, 11 commands)               # 11 Tauri commands (wrap core::*)
└── src/                                               # 前端 (HTML+JS+CSS, 0 build step)
    ├── index.html (1.4 KB)                            # 5 nav layout
    ├── app.js (22.4 KB)                               # 5 nav 路由 + 9 organ + 主对话
    └── style.css (12.8 KB)                            # 拟人化 + 拟物化 styling
```

### 2.2 核心 lib tests pass (per 决策 #57 §6)

```bash
$ cd frontend/tauri-prototype/core && cargo test
   Compiling apeireth-tauri-core v0.1.0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.21s
     Running unittests src\lib.rs
running 61 tests
... (61 tests, all pass) ...
test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\integration_test.rs
running 11 tests
... (11 tests, all pass) ...
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests apeireth_tauri_core
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**总 72 tests pass** (61 unit + 11 integration, 0 failed, 0 依赖, 0.01s 跑完):

| 模块 | tests | 守门 |
|------|------:|------|
| organ.rs | 14 | 9 organ ASCII 互异 / 中文名互异 / 拟物化互异 / 3 readiness 互异 / 跨平台 ASCII / stub honesty |
| nav.rs | 10 | 5 nav ASCII 互异 / 中文名互异 / 副标题非空 / next/prev wrap / 跨平台 ASCII |
| dialogue.rs | 11 | 3 角色互异 / 4 phase 互异 (superpowers 234 对齐) / user/AI/system 构造 / session round-trip |
| tools.rs | 9 | 6 工具 ASCII 互异 / 4 outcome 互异 / stub honesty / 跨平台 ASCII |
| settings.rs | 7 | 5+5+4 互异 / 14 keys round-trip / stub 5 鉴权 disabled + 5 Provider 0 模型 + 4 SDK installed |
| history.rs | 6 | 3 kind 互异 / stub honesty / 6 stub 0 装严守 |
| integration_test.rs | 11 | 跨模块守门: 9×5×6×14 一致性 + 4 phase superpowers 234 对齐 + 0 装 PASS 严守 + ASCII 跨平台 |

### 2.3 9 organ 拟人化 (per 用户记忆 #5)

| ID | 中文 | 英文 | ASCII | 拟物化 | 颜色 (CSS var) |
|---:|------|------|-------|--------|----------------|
| 0 | 心 | heart | [♥] | 跳动着 | 红 (心跳动画 1.2s) |
| 1 | 脑 | brain | [BRAIN] | 运转中 | 紫 |
| 2 | 手 | hand | [HAND] | 待命 | 绿 |
| 3 | 眼 | eye | [EYE] | 观察中 | 蓝 |
| 4 | 耳 | ear | [EAR] | 聆听中 | 青 |
| 5 | 记忆 | memory | [MEM] | 沉淀中 | 橙 |
| 6 | 声 | voice | [VOICE] | 表达中 | 粉 |
| 7 | 体 | body | [BODY] | 运行中 | 灰 |
| 8 | 意 | mind | [MIND] | 思考中 | 黄 |

**0 装 PASS 严守**:
- 9 organ `data_source` 字段都标 "stub: 后端未接通, 等待 Tauri command 接 apeireth-api"
- core test `tauri_prototype_all_organs_stub_phase` 守门: 9 organ readiness 应全 Stub
- core test `organ_state_data_source_marks_stub_honest` 守门: data_source 含 "stub"
- TUI 4 Ok + 5 Partial, Tauri 9 Stub (后端未接通), 诚实标

### 2.4 5 nav (per 用户记忆 #3)

| ID | 中文 | 英文 | ASCII | 副标题 |
|---:|------|------|-------|--------|
| 0 | 状态 | Status | [⌂] | 9 器官拟人化 — 心跳 / 思考 / 行动 一眼看完 |
| 1 | 主对话 | Dialogue | [DIALOG] | 主对话 — 你说, AI 想, 结果在这 (借鉴 superpowers 234) |
| 2 | 历史 | History | [HIST] | 历史 — 过去的对话, 翻回去看看 |
| 3 | 设置 | Settings | [SETUP] | 设置 — 鉴权 / Provider / SDK (5+5+4=14) |
| 4 | 工具结果 | Tools | [TOOLS] | 工具结果 — 日历/消息/联系人/任务/搜索/云盘 |

**vs TUI 5 nav (Bridge/Dialogue/Growth/History/Settings) 改造**:
- 砍 "Bridge" (舰桥, 哲学隐喻) → 替换 "状态" (per 用户记忆 #3 砍 7 项 UI 哲学元素)
- 砍 "Growth" (生长阶段, 哲学概念) → 集成到 "状态" 9 organ mind 卡片
- 加 "工具结果" (Tools, per TUI nav/mod.rs 6 工具 endpoint)
- 砍 "Help" (8 哲学锚 + 8 承诺) → 全部按用户记忆 #3 砍 7 项 UI 哲学元素

### 2.5 主对话 (借鉴 superpowers 234 executing-plans)

| ThinkingPhase | 中文 | superpowers 234 对齐 |
|---------------|------|----------------------|
| Idle | 空闲 | (主对话未启动) |
| Planning | 思考中… | Step 1: Load and Review Plan |
| Executing | 执行中… | Step 2: Execute Tasks |
| Done | 完成 | Step 3: Complete Development |

**借鉴** (per borrowed-repos/superpowers/skills/executing-plans/SKILL.md):
- 4 phase 跟 superpowers 234 executing-plans 1:1 对齐 (core test 守门)
- 用户说 → AI 思考 → AI 执行 → AI 完成
- 借鉴 TUI pages/dialogue.rs: user 气泡 vs AI 消息分块, thinking 链折叠
- frontend app.js renderDialogueMessages 走 user 气泡右对齐 + AI 消息左对齐 + thinking 折叠

### 2.6 6 工具 + 14 设置 + 3 历史 kind

- **6 工具** (per TUI nav/mod.rs 6 工具 endpoint): 日历/消息/联系人/任务/搜索/云盘, 6 ASCII 互异, 4 outcome 互异
- **14 设置** (per TUI 5+5+4): 5 鉴权 (anthropic/openai/google/azure/cohere) + 5 Provider (anthropic/openai/google/azure/local) + 4 SDK (apeireth-cli/tui/api/tauri)
- **3 历史 kind**: 会话/消息/工具调用 (per TUI pages/history.rs 6 流 + Episode 时间线)

### 2.7 Tauri 2.0 wrapper (⏳ 限流 = 准备)

| 维度 | 状态 | 说明 |
|------|------|------|
| Cargo.toml | ✅ 真声明 | tauri = "2" + serde + serde_json + core path dep |
| tauri.conf.json | ✅ 真声明 | 5 nav 窗口配置 (1280x800, 1024x720 min) |
| build.rs | ✅ 真声明 | tauri_build::build() |
| capabilities/default.json | ✅ 真声明 | 8 Tauri 2.0 permissions (core:default/window/event/app) |
| icons/README.md | ✅ 占位 | 5 图标 placeholder (P12-1 阶段 1 生成) |
| src/main.rs | ✅ 真实施 | Tauri 2.0 entry (windows_subsystem 严守) |
| src/lib.rs | ✅ 真实施 | 11 Tauri commands (get_5_nav / get_9_organs / send_user_message / 等) |
| 完整 build | ⏳ 限流 | 本地 cargo 缓存不含 tauri 2.x, build pending, 0 假装"已实施" |

---

## 3. 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #57 §3)

### 3.1 ✅ cloned = 真实施

| 借鉴 | 实施 | tests |
|------|------|------|
| core lib (organ/nav/dialogue/tools/settings/history) | ✅ 真实施 | 72 tests pass (61 unit + 11 integration, 0.01s) |
| superpowers 234 executing-plans | ✅ 借鉴 | 4 phase 翻译对齐 (思考中/执行中/完成) |
| TUI 5 nav / 9 organ / 6 tool / 14 settings | ✅ 1:1 镜像 | 各模块 test 守门数字严守 |
| 用户记忆 #3-#5 砍 7 项 + 9 organ 拟人化 | ✅ 严守 | core test stub honesty + ASCII 跨平台 |
| frontend HTML+JS+CSS | ✅ 真实施 | app.js tauriInvoke wrapper + mock fallback |
| Tauri 2.0 wrapper | ✅ 真实施 (结构) | 11 commands 完整, 等 Tauri 2.0 deps |

### 3.2 ⏳ 限流 = 准备 (诚实标)

| 借鉴 | 状态 | 诚实 disclosure |
|------|------|-----------------|
| Tauri 2.0 desktop app full build | ⏳ 限流 = 准备 | 本地 cargo 缓存不含 tauri 2.x, full `cargo build` pending, 等 P6-1/2/3 重试时一起 fetch |
| Tauri 2.0 5 icons (32x32.png/128x128/icns/ico) | ⏳ 限流 = 准备 | icons/README.md placeholder, 真实图标 P12-1 阶段 1 生成 |
| Tauri 2.0 webview integration test | ⏳ 限流 = 准备 | 等 Tauri 2.0 deps, 然后 tauri::test 写 |

### 3.3 ❌ 跳过 (本任务 0 集成)

- OpenCog AGPL-3.0: 不在本任务范围, 决策 #33 §2.3 已 skip
- LiteLLM / opencode / Guardrails (P6-1/2/3 限流重试): 不在本任务范围

---

## 4. 0 越界 8 硬墙 (per 决策 #57 §4)

| 硬墙 | 状态 | 验证 |
|------|------|------|
| **B2 workspace.version 1.2.0 0 改** | ✅ 0 改 | frontend/ 不在 workspace (core/src-tauri 各加 `[workspace]`), 主仓 Cargo.toml 0 触碰 |
| **A1 R11 baseline 3 值 0.8682/0.8532/0.9063** | ✅ 0 改 | integration_r_measure.rs 0 触碰, 17 文件原位 |
| **B1 24 LOCKED 持续更新, 入口签名 0 改** | ✅ 0 改 | 24 LOCKED crate 0 触碰, core 用 pure logic (0 借 24 LOCKED API) |
| **A3 12 键 + PHL-07 = 13 键** | ✅ 0 改 | verdict 逻辑 0 触碰 |
| **B5 6 → 8 哲学锚** | ✅ 0 改 | 哲学锚不在 UI (per 用户记忆 #3 砍 7 项) |
| **B3 V0.5 25 → 30 维** | ✅ 0 改 | V0.5 公式 0 触碰 |
| **B4 6 重守门 v6 → v7** | ✅ 0 改 | 守门不在 UI (per 用户记忆 #3 砍 7 项) |
| **C1 0 主动 commit** | ✅ 0 commit | 写到主仓 0 主动 git add/commit (git status 仅 `?? frontend/`, 0 触碰主仓) |
| **C2 0 装 PASS 严守** | ✅ 严守 | core tests pass, Tauri 2.0 = ⏳ 限流 = 准备 (诚实标) |
| **C3 升 6 重 v7** | ✅ 0 改 | 0 改 6 重守门 |
| **0 主动 push** | ✅ 0 push | 0 push (等 1.0 release 配 GitHub remote) |

---

## 5. 整合 #4 commit abf12243 严守 (per 决策 #48 + 决策 #57)

- **整合 #4 commit abf12243**: 19:41 done, 46752 file changes, master HEAD = abf12243, 0 必重跑 ✅
- **本任务 0 触碰主仓**: `git status --porcelain` 仅显示 `?? frontend/` (untracked 新 dir, 0 触碰)
- **Cargo.toml workspace.version 1.2.0**: 0 改 ✅
- **24 LOCKED 入口签名**: 0 改 ✅
- **R11 baseline 3 值 数字**: 0 改 ✅

---

## 6. 0 主动 commit + 0 主动 push 严守 (per 决策 #57 §5)

- **0 主动 commit 严守**: P11-1 写到主仓 0 主动 git add/commit, Mavis 整合 #5 commit 时机拍板
  - 整合 #5 时机 = 38 任务 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6) 全 done + 0 装 PASS verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板
- **0 主动 push 严守**: 0 push (等 1.0 release 配 GitHub remote)
- **当前 git 状态**: 仅 `?? frontend/` untracked, 0 触碰主仓 (整合 #4 commit abf12243 严守)

---

## 7. Tauri 2.0 借鉴 + 5 nav + 9 organ 设计要点

### 7.1 Tauri 2.0 项目结构 (借鉴 web_fetch 查 Tauri 2.0 文档)

```toml
# src-tauri/Cargo.toml
[workspace]                # 独立 workspace, 0 改主仓
[package]
name = "apeireth-tauri-prototype"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
apeireth-tauri-core = { path = "../core" }
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 7.2 11 Tauri commands (wrap core::\*)

| Command | 输入 | 输出 | 用途 |
|---------|------|------|------|
| `get_5_nav` | - | `[u8; 5]` | 5 nav ID 列表 |
| `get_nav_metadata` | - | `Vec<NavMetadata>` | 5 nav 完整元数据 |
| `get_9_organs` | - | `Vec<OrganState>` | 9 organ 状态快照 |
| `get_organ_state` | `organ_id: u8` | `OrganState` | 单 organ 状态 |
| `new_dialogue_session` | - | `DialogueSession` | 新主对话会话 |
| `send_user_message` | `session, content` | `DialogueSession` | 用户发送消息 |
| `get_dialogue_session` | - | `DialogueSession` | 获取当前会话 |
| `get_6_tool_results` | - | `Vec<ToolResult>` | 6 工具结果 |
| `get_settings` | - | `Settings` | 完整设置 (14 项) |
| `get_history` | - | `Vec<HistoryEntry>` | 历史记录 |

### 7.3 frontend HTML+JS+CSS (vanilla, 0 build step)

- **5 nav tabs**: 顶 bar 居中排列, 1-9 数字键 + 鼠标点击切换
- **状态页**: 9 organ cards 3x3 网格 + heartbeat 动画 (拟人化)
- **主对话页**: user 气泡右对齐 (蓝) + AI 消息左对齐 (深底) + thinking 折叠
- **历史页**: 1 列表 + kind 标签 + 时间戳
- **设置页**: 14 项分 3 section (5 鉴权 + 5 Provider + 4 SDK)
- **工具结果页**: 6 工具 card + 颜色编码
- **Tauri invoke + mock fallback**: app.js tauriInvoke() wrapper, 优先真 Tauri, fallback mock
- **0 build step**: 0 Node.js / 0 npm, 直接打开 index.html 跑

---

## 8. 风险与缓解 (per 决策 #57 §1.4)

| 风险 | 影响 | 缓解 |
|------|------|------|
| Tauri 2.0 deps 限流 | full build pending | 核心 lib tests pass (72/72), Tauri wrapper 结构完整, 等 P6-1/2/3 重试时 fetch |
| Tauri 2.0 icons 未生成 | cargo tauri build 缺图 | icons/README.md placeholder, P12-1 阶段 1 生成 |
| 前端 mock fallback 看起来 "真" | 用户可能误以为接通了 | UI 顶 status pill 显式标 "Tauri 2.0 = ⏳ 限流 = 准备", 底 status bar 严守 "0 装 PASS 严守" |
| 9 organ 全部 Stub 不实用 | 主人起床后看不到真状态 | TUI 5 nav 已 Ok 4 + Partial 5, Tauri 接通后按 TUI 表升级 |
| superpowers 234 借鉴浅 | 4 phase 翻译对了, UI 风格未深借鉴 | R129 阶段 0 真后端接通后深借鉴 (进度反馈 / 阶段切换动画) |

---

## 9. 跟 5 R128 派活任务并行 (per 决策 #57 §9)

- **P10-1 (ASI Python 整合 Stage 1)**: 跑中, 21:29 派, bg_xxx
- **P10-2 (ASI Python 整合 Stage 2)**: 跑中, 21:29 派, bg_xxx
- **P11-1 (Tauri 终极前端 prototype, 本任务)**: ✅ done, 本报告
- **P12-1 (Cargo build/test/run 实战)**: 跑中, 21:29 派, bg_xxx
- **P13-1 (LICENSE + OSS NOTICE 准备)**: 跑中, 21:29 派, bg_xxx
- **P14-1 (整合 #5 commit pre-stage 报告)**: 跑中, 21:29 派, bg_xxx

**整合 #5 commit 时机** (per 决策 #57 §0): 38 任务 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6) 全 done + 0 装 PASS verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板.

---

## 10. 任何人接手 (per 主人 00:56 任何人都能接手 + 决策 #57 §5)

### 10.1 verify core lib (Tauri 2.0 装前可跑)

```bash
$ cd Apeireth-rust/frontend/tauri-prototype/core
$ cargo test
   Compiling apeireth-tauri-core v0.1.0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.21s
running 61 tests
test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
running 11 tests
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 10.2 build Tauri 2.0 app (等 tauri 2.x 落本地 cargo 缓存)

```bash
$ cd Apeireth-rust/frontend/tauri-prototype/src-tauri
$ cargo build              # 需 tauri 2.x + tauri-build + serde 在本地 cache
$ cargo run                # 启动 desktop app (1280x800, 1024x720 min)
$ cargo tauri dev          # dev 模式 (前端热重载, Tauri 2.0 装后)
```

### 10.3 仅前端 (浏览器跑, 走 mock data fallback)

```bash
# 直接打开 src/index.html (浏览器跑, Tauri 不可用, 自动走 mock)
start Apeireth-rust/frontend/tauri-prototype/src/index.html

# 或 http server (推荐, 避免 file:// CORS)
cd Apeireth-rust/frontend/tauri-prototype/src
python -m http.server 8000
# 浏览器开 http://localhost:8000/
```

### 10.4 整合 #5 commit 时机

- 38 任务 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6) 全 done
- 0 装 PASS verify (✅ 11 cloned + ⏳ 0 限流 + ❌ 1 跳过)
- 8 硬墙 0 越界 verify (24 LOCKED 入口签名 + 0.8682/0.8532/0.9063 数字 + workspace.version 1.2.0)
- Mavis 拍板 OR 主人 8/15 拍板

---

## 11. 决策链 verify (per 决策 #57 §6)

| 决策 | 关联 | 本任务 verify |
|------|------|---------------|
| 决策 #22 (主人 16:31 最高权限 + 24 LOCKED) | B1 24 LOCKED + A1 baseline 3 值 严守 | ✅ 0 改主仓, 0 触碰 24 LOCKED |
| 决策 #33 (主人 17:22 升级授权 + 8 硬墙重置) | 8 硬墙全重置 + 0 装解除 | ✅ 8 硬墙 0 越界, 0 装 PASS 严守 |
| 决策 #41 (R125 16 sub-agent done) | R125 16 全 done | ✅ 跟本任务并行 |
| 决策 #48 (整合 #4 commit abf12243) | 19:41 done, 0 重跑 | ✅ 严守 0 触碰主仓 |
| 决策 #51 (R126+R127 16 真派 模式) | 5 min tick 监督 | ✅ 跟 38 任务并行 |
| 决策 #53 (主人 20:32 "技术性 locked 都能解锁") | 升级授权 | ✅ 严守 8 硬墙 |
| 决策 #55 (R127 整合 #5 + Library Stage 4-6) | 整合 #5 commit pre-stage | ⏳ 等 38 任务全 done |
| 决策 #56 (R127-2 10 派活 + 借鉴 3 重试) | 借鉴 3 限流重试 | ⏳ P6-1/2/3 跑中 |
| 决策 #57 (R128 6 派活) | R128 阶段 A-E 6 sub-agent | ✅ 本任务 = 阶段 B done |

---

## 12. 文件清单 (本任务创建, 21 文件 + .gitignore)

```
Apeireth-rust/frontend/tauri-prototype/
├── .gitignore                                                  # 0 装 PASS exclude
├── README.md                                                   # 入口 + 任何人接手
├── docs/
│   └── STRUCTURE.md                                            # 架构图 + 9 节详细
├── core/                                                       # ✅ 真实施 (72 tests pass)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── organ.rs                                            # 9 organ
│   │   ├── nav.rs                                              # 5 nav
│   │   ├── dialogue.rs                                         # 主对话 (superpowers 234)
│   │   ├── tools.rs                                            # 6 工具
│   │   ├── settings.rs                                         # 14 设置
│   │   └── history.rs                                          # 3 kind
│   └── tests/
│       └── integration_test.rs
├── src-tauri/                                                  # ⏳ 限流 = 准备
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   │   └── README.md
│   └── src/
│       ├── main.rs
│       └── lib.rs                                              # 11 Tauri commands
└── src/                                                        # 前端 (0 build step)
    ├── index.html
    ├── app.js
    └── style.css

Apeireth-rust/reports/
└── agent-p11-1-r128-tauri-frontend-prototype-final-2026-08-10.md   # 本报告
```

**总 22 文件** (21 创建 + 1 本报告), 0 触碰主仓, 0 主动 commit, 0 主动 push.

---

## 13. 一句话 (TL;DR)

**Apeireth 终极前端 prototype 完成 — Tauri 2.0 桌面 app 骨架 (5 nav + 9 organ 拟人化 + 主对话借鉴 superpowers 234 + 6 工具 + 14 设置), core lib 72 tests pass (61 unit + 11 integration, 0 依赖, 0.01s), Tauri 2.0 = ⏳ 限流 = 准备 (full build pending, 0 假装"已实施"), 0 越界 8 硬墙, 0 改主仓, 0 主动 commit, 0 主动 push. 整合 #5 commit 时机 = 38 任务全 done + 0 装 PASS verify + 8 硬墙 0 越界 verify, Mavis 拍板 OR 主人 8/15 拍板.**
