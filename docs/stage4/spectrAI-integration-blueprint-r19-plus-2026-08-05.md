# SpectrAI → Apeireth 深度集成蓝图 (R19+)

```
[Document-Meta]
Document: docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 集成蓝图
Commit: <commit 4 份文档时回填>
Last-Modified: 2026-08-05
Status: ✅ A 方案已拍板 (2026-08-05)

> **版本错位诚实标注 (per sub-agent D spectrai-branch-coverage-audit-2026-08-05.md §3, 2026-08-05 19:01)**:
> 蓝图标题写 "v0.9.21"，但**实际逆向源是 v0.4.6 社区版 main branch** (git HEAD 真实版本)。
> v0.9.21 商业版 1.75M LOC 全闭源 (Teams/Workflow/Telegram/Planner 4 大模块 0 逆向)。
> 主人 NSIS 实际是 Yinta fork (new-unpacked/package.json 标 fork.fromVersion: 0.9.21 + author: chuling@local)。
> R20 阶段 1-3 实施时, 应以 v0.4.6 社区版 + Yinta fork 18 万行反编译源为基线, 不以 v0.9.21 商业版。
```

> 决策: Mavis 默认 A 方案 apeireth-team-lead, 主人 2026-08-05 13:34 拍板采纳。

> **性质**: R19+ 战略蓝图 = 把 SpectrAI v0.9.21 (TypeScript/Electron AI agent 编排, 19 模块 ~25.6K LOC) 的实战模式**深度集成**到 Apeireth v2.0.0-alpha (Rust 41 crate, R19 工程化收尾完成).
> **依据**: 3 份 sub-agent 报告 (2026-08-05), 主人 user memory 9 项决策风格 + 项目战略, APEIRETH-CONVENTIONS.md 12 子规范 + Document-Meta 格式.
> **不修改承诺**: 阶段 1/2/3/4/5 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta 全保留 (见 §10).

---

## §1 战略背景 (为什么)

### 1.1 现状两极

| 端 | 状态 | 痛点 |
|---|---|---|
| **SpectrAI v0.9.21** | TypeScript/Electron 桌面 app, 19 模块, ~25.6K LOC, 5 Provider (Claude/Codex/Gemini/iFlow/OpenCode) 全实装, Supervisor-Member 团队协作已跑通 | 原版卡死 (mid-task bug), minimax m3 48+ context 下 hallucination, mid-task bug 3 处组合 (SessionManagerV2:642 throw + AgentManagerV2:281 catch+success + child session 状态窗口期) |
| **Apeireth v2.0.0-alpha** | Rust 41 crate, R19 工程化收尾完成, 4 协议真接 (OpenAI Chat/Responses/Anthropic/Gemini), Council 7 advisor 投票表决, Pipeline 5 步管线 + Keep-Alive LIFO, MCP skeleton, TUI 改瘦 (R25) | TUI 仅主 chat, 缺团队协作实战模式, 缺 5 Provider 多样性, 缺 supervisor prompt 沉淀 |

### 1.2 改路线

❌ **不**采用 patch-fork SpectrAI (Electron 桌面已验证有 m3 hallucination + mid-task bug 撕裂状态)
✅ **改路线**: 把 SpectrAI 19 模块**实战模式** → **深度集成**到 Apeireth (TS→Rust 翻译 + Apeireth 41 crate 已有的能力复用)

### 1.3 战略原则 (硬约束)

| 原则 | 来源 | 落地 |
|---|---|---|
| **深度集成, 不 patch fork** | user memory #8 终极 Tauri / #1 先思考后动手 | TS 源码当设计参考, Rust 翻译, 不复制粘贴 |
| **不重复造轮子** | user memory #6 派 sub-agent 干 + 整合 | 41 crate 已实装的全部复用 (council/agent/graph/pipeline/mcp/tool-* /protocol/supervisor/api/bus/extension), 缺什么才新建 |
| **符合 12 子规范** | APEIRETH-CONVENTIONS.md §0.1 元信息 | Document-Meta 头 + 路径 + ADR + 锚穿透 + 不修改承诺 |
| **6 主哲学锚穿透** | APEIRETH-CONVENTIONS §9 | S-1 北极星 / S-2 实事求是 / O-5 不假装 / O-2 走在前人经验上 / O-3 干到底 / O-4 任何人都能接手 |
| **7 LOCKED + workspace v1.0.0** | APEIRETH-CONVENTIONS §10 | 见 §10 不修改承诺 8 项 |
| **00 后风格** | user memory | 直接 / 不啰嗦 / 表格 / 代码引用 / 不哲学 |

### 1.4 比喻

> Apeireth v2.0.0-alpha = R19 工程化收尾的**大型基地** (41 crate 已部署, HTTP API 表面稳定, 4 协议真接 minimax m3)
> SpectrAI v0.9.21 = 实战检验过的**工程设备** (5 Provider 抽象 + Supervisor 团队 + Git worktree + MCP 桥), 但有 3 处已知 bug + 1 个 m3 hallucination 风险
> **集成** = 把设备**装进基地**, 不是把设备外壳刷成基地颜色

---

## §2 SpectrAI 19 模块架构

### 2.1 模块总览表 (核心)

完整表见 `spectrai-architecture-2026-08-05.md` §2. 摘要 (按集成相关度):

| # | 模块 | 文件 | LOC | 集成相关度 | 集成方式 |
|---|------|------|-----|-----------|---------|
| 1 | **agent** | 11 | 5610 | 🔴 P0 核心 | AgentManagerV2 + supervisorPrompt + AgentMCPServer + MCPConfigGenerator 进 `apeireth-agent` / `apeireth-team-lead` (新命名避免冲突) / `apeireth-mcp` / `apeireth-mcp::config_gen` |
| 2 | **adapter** | 9 | 5564 | 🔴 P0 核心 | 5 Provider + toolMapping 进 `apeireth-protocol` (4 协议已实装) + `apeireth-api` (R17 重构, base_url 配置) |
| 3 | **ipc** | 14 | 3797 | ⚪ Tauri 沉淀 | TUI 不用, 11 类 IPC handler 沉淀到 `tauri-roadmap-2026-08-05.md` T-002 |
| 4 | **storage** | 16 | 3116 | 🟡 P1 | better-sqlite3 → rusqlite, 11 repo 翻译为新 `apeireth-storage` (1300 LOC) |
| 5 | **session (V2)** | 4 | 2188 | 🔴 P0 核心 | SessionManagerV2 + ConcurrencyGuard 进新 `apeireth-session` (1500-2000 LOC), **mid-task bug 3 处一起改** |
| 5b | **session (V1)** ⚠️ deprecated | — | 674 | ⚪ Tauri 沉淀 | 沉淀 T-001 (任意 CLI fallback) |
| 6 | **parser** | 10 | 1908 | 🟢 P2 简化 | V2 Adapter 架构下 OutputParser 几乎不用, StateInference 仍需, 砍 50% |
| 7 | **git** | 2 | 801 | 🟡 P1 | GitWorktreeService → 新 `apeireth-git` (1000 LOC) |
| 8 | **reader** | 3 | 532 | ⚪ Tauri 沉淀 | T-005 |
| 9 | **tracker** | 1 | 511 | ⚪ Tauri 沉淀 | T-013 |
| 10 | **skill** | 2 | 324 | 🟢 P2 | SkillEngine + builtin → 新 `apeireth-skill` (400 LOC) |
| 11 | **update** | 1 | 244 | ⚪ Tauri 沉淀 | T-003 |
| 12 | **tray** | 1 | 216 | ⚪ Tauri 沉淀 | T-003 |
| 13 | **notification** | 1 | 200 | 🟢 P3 | electron.Notification → notify-rust → 新 `apeireth-notify` (300 LOC) |
| 14 | **mcp (builtin)** | 1 | 177 | 🟢 P3 | 8 个 stdio/HTTP MCP 预置进 `apeireth-mcp::builtin` |
| 15 | **node** | 1 | 131 | 🟢 P3 | NodeVersionResolver → 新 `apeireth-node` (50 LOC 极简版, napi 桥需 Node 路径) |
| 16 | **task** | 1 | 123 | 🟡 P1 | TaskSessionCoordinator → `apeireth-council::task` (200 LOC) |
| 17 | **bootstrap** | 1 | 107 | ⚪ Tauri 沉淀 | T-004 |
| 18 | **utils** | 1 | 81 | ⚪ Tauri 沉淀 | T-006 |
| 19 | **根入口** | 3 | ~1400 | 🔴 P0 核心 | `index.ts`(851) 只取 manager 初始化 (300 LOC) → `apeireth-bootstrap` 增强 + `apeireth-tui` |

**总 LOC**: ~25,600 (root + 19 模块)
**TUI 集成目标 (P0)**: adapter + session/SessionManagerV2 + agent/AgentManagerV2 + agent/MCPConfigGenerator + agent/AgentMCPServer + agent/supervisorPrompt + storage 部分 + ConcurrencyGuard + TaskSessionCoordinator + 根入口 manager 初始化
**Tauri 阶段沉淀 (13 项)**: 见 `tauri-roadmap-2026-08-05.md` T-001 ~ T-013 (T-001 V1 PTY / T-002 IPC 11 类 / T-003 tray+notify+update / T-004 macOS shellPath / T-005 OutputReader / T-006 proxyUtils / T-011 manager 初始化 / T-012 parser 正则 / T-013 FileChangeTracker)

### 2.2 V1 / V2 双轨 (砍 V1, 节省 30%)

| 轨 | 状态 | 集成 | 节省 |
|---|---|---|---|
| **V1 PTY** (SessionManager 674 + AgentManager 1570 + node-pty + HeadlessTerminalBuffer) | ⚠️ deprecated (SpectrAI 自己标) | 沉淀 T-001 (Tauri 阶段备用) | -30% LOC |
| **V2 SDK** (SessionManagerV2 1231 + AgentManagerV2 963 + Adapter 5 Provider 5564) | ✅ 主路径 | TUI 集成 | +0 |

**砍 V1 理由**: SpectrAI 自己在 `sessionHandlers.ts` 头部注释"V1 PTY 路径已弃用"; V2 已全跑通; 集成到 Rust 端不该复活 30% 弃用代码.

### 2.3 分层架构图 (引用 sub-agent 报告)

完整 4 层架构图 (Bootstrap → Core → Adapter → External + IPC Bridge) 见 `spectrai-architecture-2026-08-05.md` §3.1.

**摘要分层**:
```
EXT (5 CLI: Claude Code / Codex / Gemini / iFlow / OpenCode)
  ↓ SDK
ADAPTER (AdapterRegistry + 5 adapter + toolMapping 5 HashMap)
  ↓ ProviderEvent 流
CORE (SessionManagerV2 / AgentManagerV2 / OutputParser / StateInference / GitWorktreeService / SkillEngine / TaskSessionCoordinator / ConcurrencyGuard)
  ↓ events
BOOTSTRAP (DatabaseManager / NotificationManager / TrayManager / UpdateManager / shellPath)
  +
BRIDGE (AgentMCPServer stdio + AgentBridge WebSocket :63721)
```

---

## §3 SpectrAI 关键数据流 (引用 sub-agent 报告 5 sequence)

5 个 Mermaid sequence 完整定义见 `spectrai-architecture-2026-08-05.md` §4. 摘要:

| # | 数据流 | 关键角色 | mid-task bug 触发点 | 集成相关 |
|---|--------|---------|-------------------|---------|
| 4.1 | **启动流程** | Electron App → index.ts → DatabaseManager → AdapterRegistry → AgentManagerV2 → SessionManagerV2 → AgentBridge (WS :63721) → IPC → Renderer | 无 | TUI 集成只取 manager 初始化 (300 LOC), 砍 BrowserWindow / tray / single-instance-lock |
| 4.2 | **LLM 调用链** | Renderer → IPC sessionHandlers → SessionManagerV2 → Adapter → Claude Code SDK → ProviderEvent → wireSessionManagerV2Events → Renderer | ⚠️ #1 (line 642 throw) | Pipeline 5 步 + Keep-Alive LIFO 替代 4 协议路径 |
| 4.3 | **工具调用链 (AI 调 MCP)** | CLI → Adapter → AgentMCPServer (stdio) → AgentBridge (WS) → AgentManagerV2 → SessionManagerV2 → Renderer | 无 | stdio MCP 翻译为 `apeireth-mcp` 服务端; WS 桥按用户决策砍 |
| 4.4 | **团队消息链 (Supervisor 等成员 idle)** | Supervisor → MCP → Bridge → AgentManagerV2 → Sub-Agent Child Session → Claude SDK | ⚠️ #2 + #3 (line 281 catch+success + 状态窗口期) | AgentManagerV2 进 `apeireth-agent` (MVP) + 监督模式借鉴 |
| 4.5 | **Git Worktree 流程** | Supervisor → MCP → Bridge → AgentManagerV2 → GitWorktreeService (withRepoLock 串行化) → DB → Sub-Agent (worktree) | 无 | 新 `apeireth-git` (P1) |

---

## §4 mid-task bug 真根因 (P0, 3 处组合, 必一起改)

### 4.1 用户表现

子 Agent 任务执行到一半时 (e.g. `wait_agent_idle` 等待中), 父进程发送新消息到子 Agent, **`sendToAgent` 报错** (从 `.catch(err => console.error)` 打印的), 但**实际子 Agent 仍在运行**, 用户看到"父进程以为发失败 + 子 Agent 正常运行"的撕裂状态.

### 4.2 3 处组合根因 (不止 1 处)

#### ① 终态用 throw 而非 return — `session/SessionManagerV2.ts:636-643`

```typescript
// SessionManagerV2.ts:636-643
async sendMessage(id: string, message: string): Promise<SendMessageDispatchResult> {
  const session = this.sessions.get(id)
  if (!session) throw new Error(`Session ${id} not found`)         // line 638

  // ★ 会话处于终态时，拒绝发送（防止向已死进程写入导致二次报错）
  if (session.status === 'error' || session.status === 'completed' || session.status === 'terminated') {
    throw new Error(`Session ${id} is in ${session.status} state and cannot accept messages`)  // line 642 ⚠️
  }
}
```

**问题**: `throw` 与函数其他分支的"soft fail" 风格不一致; 调用方 `AgentManagerV2.sendToAgent:281` 用 `.catch()` 接, 误以为是 sendMessage 实现错误; 实际是**正确的"会话语义拒绝"** (子进程死了, 不能写), 应该用 `Result::Err(SessionError::Closed)` 而非 panic 风格的 throw.

#### ② sendToAgent 不检查终态 + 永远 success — `agent/AgentManagerV2.ts:269-286`

```typescript
// AgentManagerV2.ts:269-286
sendToAgent(agentId: string, message: string): { success: boolean; error?: string } {
  const agent = this.agents.get(agentId)
  if (!agent) return { success: false, error: `Agent ${agentId} not found` }

  if (agent.info.status === 'completed' || agent.info.status === 'failed' || agent.info.status === 'cancelled') {
    return { success: false, error: `Agent ${agentId} already ${agent.info.status}` }  // line 273-275 ✓
  }

  // 发送新消息时清除 idle flag（Agent 进入新一轮处理）
  this.agentIdleFlags.delete(agentId)  // line 278

  // 通过 SessionManagerV2 发送消息
  this.sessionManager.sendMessage(agent.childSessionId, message).catch(err => {  // line 281 ⚠️
    console.error(`[AgentManagerV2] sendToAgent failed:`, err)
  })

  return { success: true }  // line 285 — 永远返回 success!
}
```

**问题**:
- Agent 状态检查在 line 273-275 ✓, 但**没有检查 child session 状态**! Agent `info.status === 'running'` 时, child session 可能已经 `terminated` (子进程异常退出但 agent 状态未同步).
- line 281 用 `.catch()` 吞掉错误, line 285 仍返回 `success: true` — 父进程完全感知不到失败.
- line 278 `agentIdleFlags.delete(agentId)` 在 sendMessage 失败时也清掉了, 导致后续 `wait_agent_idle` 误判为"刚开始新轮次", 但其实根本没发出去.

#### ③ child session 状态变化到 agent 状态变化有窗口期 — `agent/AgentManagerV2.ts:458-461`

```typescript
// AgentManagerV2.ts:458-461
private onChildSessionEnded(agentId: string, status: string): void {
  const exitCode = status === 'completed' ? 0 : (status === 'error' ? 1 : -1)
  this.completeAgent(agentId, exitCode)  // line 460
}
```

**问题**:
- 子 session 状态变化时, Agent 状态也更新 (通过 `onSessionStatusChange` 转发 line 90-105).
- 但**反向不同步**: Agent 状态是 `running` 时, child session 异常崩溃 (e.g. process.kill), session 变 `terminated`, 但 `agent.info.status` 还在 `running` 直到 `onChildSessionEnded` 调用.
- **中间窗口期** (child 已死, agent 还显示 running), 任何 sendToAgent 都会走 line 281 的 sendMessage, 然后被 line 642 throw.

### 4.3 完整根因时序

```
T0: AgentManagerV2.spawnAgent() → agent.info.status = 'pending' → child session 创建
T1: child session 状态变 'running' → onSessionStatusChange → agent.info.status = 'running'
T2: 子 Agent 工作... (e.g. spawn_agent 出去探索 5 分钟)
T3: 父进程: wait_agent_idle(agentId) → 注册 idle waiter
T4: 父进程发现任务需要调整: sendToAgent(agentId, "请改用 Y 方法")
    ↓
    ① agent.info.status === 'running' → 不触发 line 273-275 守卫
    ② agentIdleFlags.delete(agentId)  // line 278
    ③ sessionManager.sendMessage(childSessionId, msg)  // line 281
        ↓
        SessionManagerV2.sendMessage line 636
        ↓
        session.status === 'error' (子进程刚刚异常退出, 但 Agent 还没收到事件)
        ↓
        throw "Session xxx is in error state"  // line 642 ⚠️
        ↓
        .catch() 吞掉 → console.error
        ↓
    ④ return { success: true }  // line 285 父进程误以为成功
T5: 父进程: wait_agent_idle(agentId) 永远等待
    ↓
    agentIdleFlags 被 line 278 清掉了, 但子 session 已死, 永远不会有 turn_complete 事件
    ↓
    等到 5 分钟超时, return { idle: false, output: '' } (line 319)
    ↓
    父进程彻底卡死或误判
```

### 4.4 修法 (3 处一起改, 不可跳)

| 修法 | 文件 | 改法 | 编译时约束 (Apeireth 必看) |
|---|---|---|---|
| **修法 1** | `session/SessionManagerV2.ts:636-643` | `throw` → `return { dispatched: false, scheduled: false, reason: 'session_closed', error: ... }` | sendMessage 永不 panic; Rust 端 `Result<T, SessionError>` |
| **修法 2** | `agent/AgentManagerV2.ts:269-286` | 加 child session 状态检查; `.catch()` → `await`; `success: true` 改条件返回 | 所有跨 session 引用都先验状态; `await` sendMessage + 检查返回 |
| **修法 3** | `agent/AgentManagerV2.ts` 同步 child 状态 | `listenEvents` (line 88) `onSessionStatusChange` 中, 不只处理 childToAgent 映射的 child, 也要处理**所有 agent 的 child** | 状态变更用事件驱动而非轮询; `tokio::sync::broadcast` 替代每会话一个 listener |

### 4.5 m3 minimax hallucination 的 2 个具体表现

| 表现 | 触发条件 | 缓解 |
|---|---|---|
| **48+ context 失忆** | 单次会话累积 ≥ 48 messages | ① 限制单次 context ≤ 32 messages; ② `UsageEstimator` (CJK 1/1.6, ASCII 1/4) 监控 token 计数; ③ mid-task bug 修复后自动 retry 减少重复 prompt |
| **工具调用 hallucination** | m3 在 tool_use 时返回不存在的工具名 | `toolMapping.ts` 5 HashMap 统一 → Rust 端 `apeireth-protocol::tool_mapping` (T-008, 500 LOC); 兜底用 `FuzzyToolMatcher` (Levenshtein ≤ 2, `apeireth-tool-runtime`) |

### 4.6 Rust 端集成时的设计原则 (Apeireth 必看)

| 原则 | 来源 | 原因 |
|---|---|---|
| **sendMessage 永不 panic** | 修法 1 | 会话是长期资源, panic 风格 API 不可恢复 |
| **所有跨 session 引用都先验状态** | 修法 2 | 状态有"窗口期", 不能信任单一字段 |
| **状态变更用事件驱动而非轮询** | 修法 3 | 减少 race; 事件总线 (`tokio::sync::broadcast`) 比锁更可靠 |
| **Async API 统一返回 `Result<T, SessionError>`** | TS→Rust 翻译 | Rust idiomatic; `?` 强制调用方处理 |
| **`tokio::sync::watch` 跟踪 session status** | 防止 race | 替代 TS 的 `if (status === 'running')` 弱检查 |

---

## §5 SpectrAI → Apeireth 集成映射表 (核心交付)

### 5.1 命名空间冲突 (新人接手最大坑)

| SpectrAI 概念 | 主人 2026-08-05 提到 | 实际 Apeireth crate | 冲突? | 集成策略 |
|---|---|---|---|---|
| **"api 模块"** | "我记得 Apeireth 就有 api 模块吧" | **`apeireth-api`** (R17 重构, 唯一 LLM 整合点) | ✅ 已确认 | 5 Provider → `apeireth-api` (用户最新决策), 走 4 协议端点 + base_url 配置 |
| **"supervisor"** | SpectrAI 高频词 (team lead) | `apeireth-supervisor` (PID 1 进程级 supervisor) | 🔴 强冲突, 含义不同 | 新命名 `apeireth-team-lead`; supervisorPrompt 翻译为 `team_lead.md` |
| **"team / 团队"** | supervisor 调度 sub-agents | `apeireth-council` (7 强制 advisor voting) | 🟡 含义不同 | council 是"7 advisor 投票表决", 不是"supervisor 调度 sub-agents". MVP 借鉴模式, 不复用 council voting |
| **"MCP 集成"** | SpectrAI 跨进程 MCP | `apeireth-mcp` (LOCKED 0 代码) | ✅ 正好填坑 | SpectrAI AgentMCPServer 翻译为 `apeireth-mcp` 服务端 |
| **"WebSocket 桥"** | SpectrAI AgentBridge (130 LOC) | `apeireth-bus` L4 WebSocket (已有) | 用户决策: **砍**, 改 in-process | ⚠️ 跟 sub-agent 报告"主人 2 决策保留"冲突, 见 §11 风险 + §8 决策清单 #2 |
| **"扩展"** | SpectrAI 5 个 Provider Adapter | `apeireth-api` (用户决策) vs `apeireth-extension` (sub-agent 报告) | ⚠️ 双路径 | ⚠️ 见 §8 决策清单 #1, **以 apeireth-api 为主, apeireth-extension 为备** |
| **"Pipeline 5 步"** | SpectrAI 工具调用链 | `apeireth-pipeline` (1794 LOC, 已实装) | ✅ 已有, 直接复用 | SpectrAI 工具调用逻辑翻译为 `pipeline::step` |
| **"graph / 流程图"** | SpectrAI Worktree 流程 | `apeireth-graph` (565 LOC, 已实装) | ✅ 已有, 直接复用 | Worktree 流程建模为 graph DAG |
| **"tools / 工具"** | SpectrAI toolMapping 5 HashMap | `apeireth-tool-{registry,runtime,approval}` | ✅ 已有, 直接复用 | toolMapping 进 `apeireth-protocol::tool_mapping` |
| **"git worktree"** | SpectrAI GitWorktreeService (746 LOC) | 暂未找到独立 git crate | ❌ 缺 | 翻译为新 `apeireth-git` (1000 LOC, P1) |
| **"storage"** | SpectrAI SQLite + 11 repo | 暂未找到独立 storage crate (apeireth-memory 8/5 是记忆) | ❌ 缺 | 翻译为新 `apeireth-storage` (1300 LOC, P1) |
| **"tracker"** | SpectrAI FileChangeTracker (511 LOC) | 暂未找到 | ❌ 缺 | 沉淀 T-013 |

### 5.2 完整集成映射表 (TUI 集成, P0 → P2)

| SpectrAI 模块 | 关键文件 | 对应 Apeireth crate (R19+ 位置) | 集成方式 | 风险点 | 优先级 | 状态 |
|--------------|---------|--------------------------------|---------|-------|--------|------|
| **adapter (抽象)** | `adapter/types.ts`(182) → `ProviderEvent/AdapterSessionConfig/BaseProviderAdapter` | `apeireth-protocol` (已有) + `apeireth-api` (用户决策) | 复用 `apeireth-protocol::event::ProviderEvent`; 新增 5 Provider adapter, 走 `apeireth-api` HTTP 端点 (4 协议已实装); base_url + auth_token 配置 | Claude Agent SDK V1 `query()` API 是 npm-only; 必须用 napi-rs 桥 (估 200 LOC) 或自己实现 V1 协议 (估 2000 LOC) | **P0** | 🟡 部分实现 (4 协议端点已通, 5 Provider 缺 1) |
| **adapter (5 具体实现)** | `ClaudeSdkAdapter.ts`(1742) + `toolMapping.ts`(259) + `CodexAppServerAdapter.ts`(1098) + `IFlowAcpAdapter.ts`(767) + `OpenCodeSdkAdapter.ts`(696) + `GeminiHeadlessAdapter.ts`(632) | `apeireth-api` (用户决策, 走 4 协议 HTTP) | 5 Provider 全部走 `apeireth-api` 4 协议端点 + base_url 配置 (零代码改); toolMapping 进 `apeireth-protocol::tool_mapping` (5 HashMap) | 5 Provider 维护成本; napi-rs 桥只桥 Claude SDK, 其他 4 个 provider 用各自 CLI wrapper | **P0** | 🟡 部分实现 (protocol 4 个齐, 5 Provider 缺 1) |
| **session (V2)** | `SessionManagerV2.ts`(1231) | `apeireth-session` (新增, 估 1500-2000 LOC) | `ManagedSession` + `_createSession` + 状态机迁移到 Rust struct; **mid-task bug 3 处一起改**; 调度器策略 (`interrupt_now`/`queue_after_turn`) 完整保留; 事件改 `tokio::sync::broadcast` | mid-task bug 必须在 Rust 端不重复; 窗口期 race 用 `tokio::sync::watch` 跟踪 status | **P0** | ❌ 缺失 (待新增) |
| **session (V1)** ⚠️ deprecated | `SessionManager.ts`(674) | **沉淀 T-001** (tauri-roadmap-2026-08-05.md) | TUI 不集成; Tauri 阶段备用 | 不集成 | — | ⚪ 沉淀 |
| **session (并发控制)** | `ConcurrencyGuard.ts`(188) | `apeireth-agent::concurrency` (新增, 估 200 LOC) | maxSessions=9 + 内存/CPU 检查 → sysinfo crate; 跨平台 | macOS 内存用 `sysinfo::System` | **P1** | ❌ 缺失 |
| **agent (V2 编排)** | `AgentManagerV2.ts`(963) | `apeireth-agent` (1358 LOC, 已有) + 新增 supervisor 调度模式 | `ManagedAgent` / `childToAgent` / `parentToAgents` / `waiters` / `idleWaiters` / `agentIdleFlags` 全部 Map → Rust `HashMap`; `spawnAgent/sendToAgent/waitAgentIdle/waitAgent/cancelAgent` 改 async fn; **mid-task bug #2 修法** | 父-子关系 + wait_idle 回调 + oneShot vs persistent 三处需对齐; 跟 `apeireth-council` (7 advisor voting) 模式不混 | **P0** | 🟡 部分实现 (AgentManager 基础有, supervisor 模式缺) |
| **agent (V1)** ⚠️ deprecated | `AgentManager.ts`(1570) | **沉淀 T-001** | Tauri 备用 | 不集成 | — | ⚪ 沉淀 |
| **agent (Supervisor 提示词)** | `supervisorPrompt.ts`(808) | **新命名 `apeireth-team-lead`** (新 crate, 估 850 LOC) | **1:1 翻译** (用户决策 3); 818 行 markdown 完整保留; `buildAwarenessPrompt` + `buildSupervisorPrompt` 不优化 | 跟 `apeireth-supervisor` 命名冲突, 必须新命名; "Claude Code"/"Claude" 字样保留, minimax m3 也认 "Claude Code" 工具名 | **P0** | ❌ 缺失 (待新增 crate) |
| **agent (MCP 配置生成)** | `MCPConfigGenerator.ts`(485) | `apeireth-mcp::config_gen` (新增, 估 600 LOC) | 3 个分支都翻译: Claude/iFlow (JSON 临时文件) + Codex (CODEX_HOME 目录) + OpenCode (OPENCODE_CONFIG 路径) | 3 个非 Claude provider 注入机制可能改变 | **P0** | ❌ 缺失 (apeireth-mcp 整体待填坑) |
| **agent (MCP Server 实现)** | `AgentMCPServer.ts`(893) | **`apeireth-mcp` (填 LOCKED 0 代码缺口!)** | 893 LOC 翻译为 `apeireth-mcp` 服务端, 用 `rmcp` crate (官方 Rust MCP SDK); 8 调度工具 (spawn/send_to/get_output/wait_idle/wait/get_status/list/cancel) + 3 supervisor worktree 工具 (merge/info/check) + 3 感知工具 (list_sessions/get_summary/search_sessions) | 8+3+3=14 工具逐个测试, 不能丢 | **P0** | ❌ 缺失 (LOCKED 0 代码) |
| **agent (WebSocket 桥)** | `AgentBridge.ts`(130) | **🗑️ 砍, 改 in-process** (用户决策 2) | 不实现 WebSocket 桥, AgentMCPServer 走 stdio, AgentManagerV2 走直接函数调用 | ⚠️ 失去 apeireth-bus L4 已实装的 WebSocket 复用; ⚠️ 失去未来第三方 MCP 客户端跨进程兼容 | **P0** | ❌ 砍 (用户决策) |
| **agent (Provider 可用性检测)** | `providerAvailability.ts`(110) | `apeireth-protocol::availability` (新增, 估 200 LOC) | `checkCommand` 用 `which` crate 跨平台; `prependNodeVersionToEnvPath` 调 `apeireth-node`; 5 Provider 全测 (用户决策 1) | nvm Windows 路径复杂, 简化只看 `NVM_HOME` | **P0** | ❌ 缺失 |
| **agent (类型)** | `types.ts` (AgentInfo/AgentResult/BridgeRequest/BridgeResponse) | `apeireth-agent::types` (估 100 LOC) | struct 翻译; 字段对齐 (parentSessionId 保留); 砍 BridgeRequest/Response (用户决策 2) | 命名 | **P1** | ❌ 缺失 |
| **ipc (11 类 handler)** | `ipc/index.ts`(82) + 14 个 handler 文件 (3797 LOC) | **沉淀 T-002** | TUI 不用 IPC; 11 类划分作为 trait 表面沉淀, Tauri 阶段复用为 `#[tauri::command]` | 11 类稳定, Tauri command 命名空间映射 | — | ⚪ 沉淀 |
| **storage (Database + 11 repo)** | `Database.ts`(319) + 11 repository | `apeireth-storage` (新增, 估 1300 LOC) | `better-sqlite3` → `rusqlite` (含 `feature="fts5"`); 11 张表全量迁移; WAL 模式 + 内存降级 + FTS5 全文搜索 完整保留; `cleanupOrphanedSessions` / `cleanupOldLogs` 保留 | FTS5 中文分词 (改 `tokenize='trigram'`) | **P1** | ❌ 缺失 |
| **parser (OutputParser + StateInference + UsageEstimator)** | `OutputParser.ts`(534) + `StateInference.ts`(413) + `UsageEstimator.ts`(163) | `apeireth-parser` (新增, 估 1500-2000 LOC) | V2 Adapter 架构下 OutputParser 几乎不用 (结构化事件直接 emit); StateInference 仍需; UsageEstimator token 估算公式 (CJK 1/1.6, ASCII 1/4) | V1 fallback 不做, 砍 50% parser 代码 | **P2** | ❌ 缺失 |
| **parser (正则规则)** | `rules.ts`(180) + `geminiRules.ts`(188) + `genericRules.ts`(175) + `ConfirmationDetector.ts` | **沉淀 T-012** | TUI 不做; Tauri 阶段如果做任意 CLI 接入 (T-001) 才做 | — | — | ⚪ 沉淀 |
| **reader (结构化输出)** | `OutputReaderManager.ts`(75) + `ClaudeJsonlReader.ts`(402) | **沉淀 T-005** | TUI 不做; Tauri 阶段审计/回放用 | Claude Code JSONL 格式可能变 | — | ⚪ 沉淀 |
| **git (Worktree)** | `GitWorktreeService.ts`(746) | `apeireth-git` (新增, 估 1000 LOC) | `execFile git` → `tokio::process::Command`; `withRepoLock` → `Arc<Mutex<HashMap<RepoPath, Semaphore>>>`; 746 LOC 全量翻译 | Windows `where git` 路径; 长路径 `\\?\` 前缀 | **P1** | ❌ 缺失 |
| **tracker (FileChangeTracker)** | `FileChangeTracker.ts`(511) | **沉淀 T-013** | TUI 暂不做; Tauri 阶段 UI 显示时做 | notify crate 跨平台测 | — | ⚪ 沉淀 |
| **skill (SkillEngine + builtin)** | `SkillEngine.ts`(83) + `builtinSkills.ts`(241) | `apeireth-skill` (新增, 估 400 LOC) | `expand` + `parseVariables` 翻译; 324 LOC 全量; 内置 skill 硬编码 fixture | V1 fallback 砍 | **P2** | ❌ 缺失 |
| **mcp (内置 MCP 预置)** | `builtinMcps.ts`(177) | `apeireth-mcp::builtin` (新增, 估 200 LOC) | 8 个 stdio/HTTP MCP server 预置 JSON 翻译为 Rust struct | 实际启用由用户配置 | **P3** | ❌ 缺失 |
| **task (TaskSessionCoordinator)** | `TaskSessionCoordinator.ts`(123) | `apeireth-council::task` (新增, 估 200 LOC) | SESSION_TO_TASK + ACTIVITY_TO_TASK + 1s debounce + 多会话边界 完整翻译 | 状态名需对齐 | **P1** | ❌ 缺失 |
| **node (NodeVersionResolver)** | `NodeVersionResolver.ts`(131) | `apeireth-node` (新增, 估 50 LOC 极简版) | TUI 是 Rust, 只需 Claude Agent SDK napi 桥的 Node 路径; `which` crate | nvm Windows 简化为只读 `NVM_HOME` | **P3** | ❌ 缺失 |
| **bootstrap (shellPath)** | `shellPath.ts`(107) | **沉淀 T-004** | TUI 不需要; Tauri macOS 启动时复用 | 极低 | — | ⚪ 沉淀 |
| **update (electron-updater)** | `UpdateManager.ts`(244) | **沉淀 T-003** | TUI 走自己的更新流程; Tauri 用 `tauri-plugin-updater` | 不集成 | — | ⚪ 沉淀 |
| **tray (TrayManager)** | `TrayManager.ts`(216) | **沉淀 T-003** | TUI 没有托盘; Tauri 阶段用内置 tray API | 不集成 | — | ⚪ 沉淀 |
| **notification (NotificationManager)** | `NotificationManager.ts`(200) | `apeireth-notify` (新增, 估 300 LOC) | `electron.Notification` → `notify-rust` crate; 4 类型 + 免打扰逻辑 完整 | TUI 可选 stderr 输出 | **P3** | ❌ 缺失 |
| **utils (proxyUtils)** | `proxyUtils.ts`(81) | **沉淀 T-006** | TUI 不需要; Tauri webview 启动时复用 | minimax m3 可能不需要 | — | ⚪ 沉淀 |
| **根入口 (index.ts)** | `index.ts`(851) | `apeireth-bootstrap` (107 LOC 已有) + `apeireth-tui` (新增, 估 600 LOC) | **只取 manager 初始化 (300 LOC)** (用户决策 4); `initializeManagers` line 235-379 + `wireEvents` line 385+; 砍 BrowserWindow / tray / single-instance-lock | Tauri 阶段用 `tauri::Builder::default()` 替代 Electron | **P0** | 🟡 部分实现 (bootstrap 基础有, tui 入口缺) |
| **logger** | `logger.ts` | `apeireth-telemetry::log` (估 50 LOC) | electron-log → `tracing` crate + `tracing-subscriber` | — | **P1** | ❌ 缺失 |
| **migration (legacy)** | `migration.ts` | **跳过** | SpectrAI 旧用户数据迁移, Apeireth 全新安装 | — | — | ⚪ 跳过 |

### 5.3 总新增/增强 LOC 估算

| 新 crate | 估 LOC | 来源 |
|---------|-------|------|
| `apeireth-api` (5 Provider 增强) | +500 | base_url + auth_token 配置 5 个 |
| `apeireth-protocol` (增强) | +2300 | 5 Provider + tool_mapping + availability |
| `apeireth-session` (新增) | 1500-2000 | SessionManagerV2 + mid-task bug 修复 |
| `apeireth-agent` (增强) | +400 | supervisor 调度模式 + concurrency + types |
| `apeireth-mcp` (从 0 实装) | 1500 | AgentMCPServer + config_gen + builtin |
| `apeireth-team-lead` (新增, 避免命名冲突) | 850 | supervisorPrompt 1:1 翻译 |
| `apeireth-storage` (新增) | 1300 | Database + 11 repo + FTS5 |
| `apeireth-git` (新增) | 1000 | GitWorktreeService |
| `apeireth-council::task` (新增) | 200 | TaskSessionCoordinator |
| `apeireth-skill` (新增) | 400 | SkillEngine + builtin |
| `apeireth-parser` (新增, 简化) | 800-1500 | StateInference + UsageEstimator |
| `apeireth-notify` (新增) | 300 | NotificationManager |
| `apeireth-bootstrap` (增强) | +300 | manager 初始化 + 事件接线 |
| `apeireth-tui` (新增) | 600 | TUI 启动入口 |
| **新增总计** | **~11500-12450** | |

**复用现有 crate**: `apeireth-protocol` + `apeireth-agent` + `apeireth-council` + `apeireth-supervisor` (PID 1, 不动) + `apeireth-mcp` (填坑) + `apeireth-tool-{registry,runtime,approval}` + `apeireth-memory` + `apeireth-cognition` + `apeireth-bus` (L4 WebSocket 已实装, 但用户决策砍) + `apeireth-extension` (VCP 6 类) + `apeireth-pipeline` + `apeireth-graph` + `apeireth-api` (R17 重构, 4 协议真接)

**Tauri 阶段沉淀**: `tauri-roadmap-2026-08-05.md` T-001 ~ T-013 (~10,500 LOC 备用, 默认不实现)

---

## §6 Apeireth 已有能力 (R19 后, 41 crate 总览)

### 6.1 41 crate 总览 (按 R 状态 + 集成相关度)

完整 crate 列表见 `apeireth-crate-api-2026-08-05.md` §1 + `apeireth-platform-modules-2026-08-05.md` §1. 摘要 (按集成相关度):

| Crate | LOC | 状态 | 集成相关度 | 用途 |
|---|---|---|---|---|
| **`apeireth-api`** | ~2700 (27 .rs) | 🟢 R17 重构 | 🔴 P0 核心 | **LLM provider 整合平台, 唯一对外入口**; 4 协议真接 (OpenAI Chat/Responses/Anthropic/Gemini) + Keep-Alive LIFO + 5 步 pipeline; 7 HTTP endpoint (4 协议 + /health + /council/advise + /verdict) + R25 Step 2 6 类 V2 端点 |
| **`apeireth-protocol`** | 1365 | 🟢 战区 5 P0 | 🔴 P0 核心 | 4 协议归一化 (OpenAI Chat/Responses/Anthropic/Gemini) + NormalizedRequest/Response + ProtocolRouter |
| **`apeireth-pipeline`** | 1794 | 🟢 R17 战役 1-3 | 🔴 P0 核心 | 5 步主 chat 管线 + 流式; ⚠️ `run_streaming` 是 simulate (按 50 字符切块) |
| **`apeireth-council`** | 1711 | 🟢 7 advisor | 🔴 P0 核心 | 7 强制 Advisor + 按住机制 + 拟人化 synthesis + Sovereignty hook 接口 + `LlmAdvisorBackend` (真 LLM 适配) |
| **`apeireth-agent`** | 1358 | 🟢 Agent 注册 | 🔴 P0 核心 | Agent 注册 + alias 解析 + LRU cache + notify 热加载 (字段级引用 VCP `agentManager.js`) |
| **`apeireth-mcp`** | 1128 | 🔴 LOCKED 0 代码 | 🔴 P0 填坑 | MCP skeleton (client/server + JSON-RPC 2.0 + stdio/SSE/HTTP Streamable transport) |
| **`apeireth-graph`** | 565 | 🟢 DAG 编排 | 🟡 P1 | 同步节点 DAG 编排 + 拓扑执行 + 版本化 checkpoint |
| **`apeireth-tool-registry`** | 1838 | 🟢 6 类 + 5 轴 | 🟡 P1 | 工具注册中心 (6 类 enum + 5 轴正交 + token 预算 + notify 热加载) |
| **`apeireth-tool-runtime`** | 2363 | 🟢 解析+执行 | 🟡 P1 | LLM 输出解析 + 模糊匹配 + 真执行 + 隐私脱敏 + 调用记录 |
| **`apeireth-tool-approval`** | 1782 | 🟢 5 规则 | 🟡 P1 | 5 审批规则 (Trust/Risk/Frequency/Whitelist/Blacklist) + 5min 窗口 |
| **`apeireth-supervisor`** | 641 | 🟢 5 sub-supervisor | ⚪ 命名冲突, 不复用 | 进程级监督树 (5 sub-supervisor + 21 child + actor 模型) |
| **`apeireth-bus`** | ~900 (9 .rs) | 🟢 5 层 | ⚠️ 砍 (用户决策 2) | 5 层通信总线 (L0 inproc / L1 UDS / L2 pipe / L3 gRPC / L4 WebSocket) |
| **`apeireth-extension`** | ~1900 (19 .rs) | 🟢 VCP 6 类 | 🟡 备选路径 | VCP 6 类插件平台 (sync/async/static/service/messagePreprocessor/hybrid) + extension.toml schema + 沙盒 |
| 其他 28 个 crate | — | 🟢 各种 | ⚪ 各按需 | memory / cognition / action / motivation / value / consciousness / relation / life-force / upgrade / tui / pybridge / sdk / onion / formal / central / telemetry / etc. |

### 6.2 9 个核心 crate 详细 (按 spectrai-architecture §5 集成点)

详细 API 表面见 `apeireth-crate-api-2026-08-05.md` §2. 关键集成点摘要:

| Crate | 关键集成点 |
|---|---|
| `apeireth-council` | ① `LlmAdvisorBackend` 已是真 LLM 入口 (SpectrAI 多模型路由可直接走); ② `SovereigntyHook` trait 留空, MVP 用 `NoopSovereigntyHook`; ③ `CouncilEvent::OpinionIssued` 事件流 (SpectrAI mid-task 显示每个 advisor 意见); ④ `Council::deliberate()` **同步阻塞**, async 环境需 `tokio::task::spawn_blocking` |
| `apeireth-agent` | ① `Agent::tools: Vec<String>` 是工具名不是 `Arc<dyn Tool>`, 集成时需自己用 `ToolRegistry::get(name)` 查; ② `AgentManager` 用 `parking_lot::RwLock` (同步), async 环境需 `spawn_blocking`; ③ 事件流 `AgentEvent` 是同步 push 模式, 没 callback/SSE, 需自己 wrap; ④ `watch_dir` 用 `RecursiveMode::NonRecursive`, 不递归 |
| `apeireth-graph` | ① `Node` trait 是**同步 `run()`**, async 操作需要 `tokio::task::block_in_place`; ② 强拓扑校验, 失败返 `GraphError::Cycle`; ③ `State` 用 `serde_json::Value` 跨节点传数据灵活, 无类型; ④ 没看到 supervisor 集成 hook (`SupervisorSnapshot` 注释有, 没深入) |
| `apeireth-mcp` | ① 3 transport 都真实现 (stdio spawn 子进程 / SSE skeleton / HTTP Streamable skeleton); ② `McpServer::from_registry()` 一行代码桥接全部 tool (SpectrAI 接 MCP 客户端时 server 端零改造); ③ `ToolHandler` 是 `Box<dyn Fn(Value) -> Pin<Box<dyn Future<...>>>>` 灵活; ④ **单飞 (in-flight at a time)**: skeleton 简化, 生产环境要 outstanding 改造; ⑤ MCP 协议版本 hardcode `"2025-03-26"` |
| `apeireth-pipeline` | ① **`run_streaming` 是 simulate (按 50 字符切块)**, 真流式走 `stream_to_sender` + reqwest `bytes_stream`; ② `PipelineConfig::base_url` 默认 `"https://api.minimaxi.com"`; ③ `RetrySuppression` 用 model + first user msg 做 fingerprint, 15s 窗口; ④ 5 步固定不可改顺序 |
| `apeireth-tool-runtime` | ① `ToolCallParser` 解析 VCP `<<<[TOOL_REQUEST]>>>` 标记; ② `FuzzyToolMatcher` 距离 ≤ 2 简单 Levenshtein; ③ `PrivacyGuard` mask 用 `[VCP_PRIVACY_REDACTED]` 替换; ④ `RecordStore` 写 SQLite (`apeireth-memory::SqliteMemoryStore`) |
| `apeireth-tool-registry` | ① 6 类 enum + 5 轴 1:1 对应 VCP `pluginType`; ② `Tool::call(args: Value) -> Result<Value, String>` 错误是 String 不是 thiserror Error; ③ `ToolRegistry` 用 `Arc<dyn Tool>`, 跨线程共享; ④ `ToolKind::as_vcp_str()` 1:1 映射 VCP |
| `apeireth-tool-approval` | ① 5 规则按顺序 (Blacklist > Trust > Risk > Frequency > Whitelist), 第一个非 NoMatch 生效; ② `ApprovalHandler` 是外部注册 (SpectrAI 接 UI 弹窗实现 `set_handler`); ③ 5min 窗口 hardcode; ④ `FrequencyRule` 1min/3 次反刷 (VCP 没有, Apeireth 扩展) |
| `apeireth-protocol` | ① 4 协议都真实现 (非 mock), Anthropic `max_tokens` 必填, Gemini URL 含 `{model}` 占位符; ② `NormalizedRequest` 是内部统一形态; ③ `ProtocolKind::parse` 支持多别名 (`openai` / `openai_chat` / `openai-chat` / `chat_completions`); ④ `ContentPart` enum 设计支持多模态 + tool_use + tool_result; ⑤ `is_tool_result_error` 5 字段判断 (success/ok/status/code/httpStatus) |

### 6.3 双层 LLM 抽象 (2 套共存)

`apeireth-api` **有 2 套 LLM 抽象**共存 (见 `apeireth-platform-modules-2026-08-05.md` §2.4):

| 抽象 | 位置 | 状态 | 用途 |
|---|---|---|---|
| **抽象 A: `LlmProvider` trait** | `apeireth-api/src/llm/traits.rs` | ⚠️ **DEPRECATE** (战役 1-4), 仅 `/council/advise` legacy 兼容 | 4 concrete providers (ApeirethApiProvider / AnthropicCompatibleProvider / OpenAiCompatibleProvider / ScriptedLlmProvider) + MultiLlmRouter (NewAPI 风格) + 协议无关简化类型 (LlmRequest/LlmResponse/ChatMessage/TokenUsage) |
| **抽象 B: `ProtocolRouter`** | `apeireth-protocol/src/router.rs` | 🟢 **当前主路径** | 4 zero-sized adapters (OpenAiChat / OpenAiResponses / AnthropicMessages / Gemini) + ProtocolKind enum + NormalizedRequest/Response + 真接 4 协议走 5 步 pipeline + Keep-Alive LIFO |

**关键洞察**: 抽象 A 是 "**Provider 概念**" (base URL identity + 协议无关), 抽象 B 是 "**协议概念**" (协议 shape + base URL 拼接). 两者不在同一抽象层, 设计目标也不同.

**5 Provider 映射 (用户决策 = apeireth-api)**: 5 Provider 全部走 `apeireth-api` 4 协议端点 + base_url + auth_token 配置, 零代码改. 如果有 minimaxi 之外的真 Anthropic 直连 / 真 OpenAI 直连 → 用未验证但兼容的 base URL 配置 (`AppState::new(base_url, auth_token, llm_provider)` 一行).

### 6.4 5 Provider base URL 配置能力 (R17 战役 1-4 已实装)

**已验证** (R17 验收, 真接 minimax m3):
- `https://api.minimaxi.com` — 4 协议全开 (OpenAI Chat / Responses / Anthropic / Gemini 都能走)
- 模型: `MiniMax-M3`

**未验证但协议兼容** (加 base URL + auth token 即可接入, 零代码改):
- OpenAI 直连: `https://api.openai.com/v1`
- Anthropic 直连: `https://api.anthropic.com/v1`
- Ollama 本地: `http://localhost:11434/v1` (OpenAI 协议)
- Together / vLLM / LMStudio / Azure OpenAI: 均走 OpenAI 协议
- Google Gemini 直连: `https://generativelanguage.googleapis.com`

---

## §7 关键缺口 (4 处, 必看)

| 缺口 | 位置 | 集成影响 | 修法 (用户决策) |
|---|---|---|---|
| **1. `Pipeline::run_streaming` 是 simulate** | `apeireth-pipeline::Pipeline::run_streaming` line 注释明确 "按 50 字符切块" | SpectrAI 真 SSE 推流受阻 | **Mavis 亲自改 1 file** (用户决策 7), 改写为 `stream_to_sender` + `reqwest::Response::bytes_stream`; 改后真 SSE 贯通 |
| **2. `SovereigntyHook` 是 noop** | `apeireth-council::NoopSovereigntyHook` | SpectrAI 想"主 AI 守门"无入口 | **MVP Noop** (用户决策 9); R20+ 阶段实现主权仲裁时再 implement `SovereigntyHook` trait |
| **3. `SseTransport` + `HttpStreamableTransport` 是 skeleton** | `apeireth-mcp::transport::*` Week 1 后实装注释 | SpectrAI stdio MCP 翻译不受影响 (MCP 主走 stdio) | 集成时**只用 stdio transport**; SseTransport/HttpStreamable 待 Week 1 后实装; 不影响 P0 阶段 |
| **4. `graph ↔ supervisor` 集成是空** | `apeireth-graph` 注释提 `SupervisorSnapshot` 但没展开 | 蓝图 TODO, 当前只能 in-process | **不在 P0/P1 范围**; R21+ 阶段补; 不影响 TUI 集成 |

其他次要缺口 (可忽略或沉淀):
- `Council::deliberate()` 同步阻塞 → 集成时 wrap `tokio::task::spawn_blocking`
- Anthropic `max_tokens` 必填 → `PipelineConfig` 默认 `DEFAULT_ANTHROPIC_MAX_TOKENS=1024`
- Gemini URL 占位符 `{model}` → router 自动替换, 集成时 base URL 不要带 `/v1beta`
- 7 advisor MVP 共享 1 provider (用户决策 8) → `LlmAdvisorBackend::new(shared_llm)` 7 个 advisor 用同一 `Arc<dyn LlmProvider>`

---

## §8 决策清单 (10 项, 2026-08-05 A 方案已拍板: apeireth-team-lead)

| # | 决策项 | 主人最新指令 | sub-agent 报告旧决策 | 我的建议 | 状态 |
|---|--------|------------|-------------------|---------|------|
| **1** | **Provider 范围** | **保留 5 个, 深度集成到 `apeireth-api`** (R17 重构, base_url 配置) | sub-agent 报告"主人 1 决策" = 5 Provider 进 `apeireth-extension` (VCP 6 类 plugin) | **采纳主人最新指令** = `apeireth-api`; 理由: `apeireth-platform-modules-2026-08-05.md` 明确 `apeireth-api` 是**唯一 LLM 整合点**; base_url + auth_token 配置零代码改覆盖 5 Provider | 🟡 待确认 |
| **2** | **WebSocket 桥 + stdio MCP** | **砍, 改 in-process** (用户决策 2) | sub-agent 报告"主人 2 决策" = 保留: WebSocket → `apeireth-bus` L4 (已有), stdio MCP → `apeireth-mcp` (填坑) | **采纳主人最新指令** = 砍 WebSocket 桥; 但**保留 stdio MCP 翻译为 `apeireth-mcp`** (集成需要); 风险: 失去 apeireth-bus L4 已实装复用 + 失去未来第三方 MCP 客户端跨进程兼容 | 🟡 待确认 (高风险) |
| **3** | **supervisorPrompt** | **1:1 翻译** (用户决策 3) | 一致 | 818 行 markdown 完整保留; `buildAwarenessPrompt` + `buildSupervisorPrompt` 不优化; 翻译到新命名 `apeireth-team-lead::prompt` (避免与 PID 1 supervisor 冲突) | ✅ 采纳 |
| **4** | **启动 stage 0-7** | **只取 manager 初始化 (300 LOC)** (用户决策 4) | 一致 | Tauri 资产 (BrowserWindow / tray / single-instance-lock) 沉淀到 `tauri-roadmap-2026-08-05.md` T-011; TUI 入口从 `apeireth-bootstrap` 增强 + `apeireth-tui` 新增 (估 600 LOC) | ✅ 采纳 |
| **5** | **TUI 风格** | **并排** (用户决策 5) | sub-agent 报告 = "SpectrAI 24 panel 并排 + Apeireth 9 器官拟人化叠加" (D 方案) | **采纳主人最新指令** = 并排; 不采纳 D 方案的 9 器官拟人化叠加 (避免过度设计); 24 panel 简单并排 (按 4×6 网格) | ✅ 采纳 |
| **6** | **5 阶段顺序 vs 并行** | **阶段 1 单独 sprint** (用户决策 6) | sub-agent 报告 11 周按 R1-R6 顺序 | 阶段 1 (R18 P0, 1-2 周) 单独 sprint, 因为 protocol + tool-registry 是纯类型, 不依赖其他模块, 可以快速完成 + 立竿见影验证 R17 重构成果 | ✅ 采纳 |
| **7** | **Pipeline 真 SSE 修复** | **Mavis 亲自改 1 file** (用户决策 7) | sub-agent 报告建议 R21+ 补 | Mavis 亲自改 `apeireth-pipeline/src/lib.rs` 中 `Pipeline::run_streaming` 方法, 改写为 `stream_to_sender` + `reqwest::Response::bytes_stream`; 影响最小, 立竿见影 | ✅ 采纳 |
| **8** | **Council 7 advisor LLM** | **MVP 共享 1 provider** (用户决策 8) | sub-agent 报告未明确 | MVP 阶段 7 advisor 共享 1 个 `Arc<dyn LlmProvider>` (minimax m3); `philosophy_advisor(llm) / ethics_advisor(llm) / history_advisor() / safety_advisor() / performance_advisor() / strategy_advisor() / legal_advisor()` 7 个工厂统一注入同一 LLM; R20+ 阶段支持 7 advisor 不同 provider | ✅ 采纳 |
| **9** | **SovereigntyHook** | **MVP Noop** (用户决策 9) | sub-agent 报告建议 R20+ 补 | MVP 阶段用 `NoopSovereigntyHook` (已有); 不 implement; R20+ 阶段主权仲裁时再 implement | ✅ 采纳 |
| **10** | **Mock 改造** | **保留当文档示例** (用户决策 10) | sub-agent 报告未明确 | Mock 集中在 3 处: `apeireth-council/mock_llm.rs` (trait 抽象) + `apeireth-tool-registry` (6 类示例) + `apeireth-api/llm/providers/scripted.rs` (测试); 保留作为**文档示例 + 单元测试 fixture**, 不删除; 真 LLM 路径独立存在 | ✅ 采纳 |

---

## §9 5 阶段集成路线 (对齐 R18+ 实际规划)

### 9.1 阶段总览 (5 阶段, 跨 R18 → R21+)

| 阶段 | R 周期 | 时长 | 优先级 | 核心交付 | 砍/留/沉淀 |
|------|--------|------|--------|---------|-----------|
| **阶段 1** | R18 P0 | 1-2 周 | 🔴 P0 单独 sprint | 6 象限 LLM API 深化 (5 trait 接 VCP 真实实现 + 6 message API) | 复用 + 新增 |
| **阶段 2** | R18 P0 | 1 天 | 🔴 P0 修 bug | mid-task bug 3 处修法 + `Pipeline::run_streaming` 真 SSE | 复用 + 改 1 file |
| **阶段 3** | R19 P1 | 1-2 周 | 🟡 P1 深化 | TUI 9 命令深化 + 团队功能借鉴到 `apeireth-agent` | 翻译 + 新增 |
| **阶段 4** | R20 P1 | 2-4 周 | 🟡 P1 收产品 | 收产品 + Tauri 团队接入 + OTA 实装 | 全栈 + Tauri 阶段 |
| **阶段 5** | R21+ P2 | 4-8 周 | 🟢 P2 补缺口 | Council 真实 LLM / SseTransport / graph↔supervisor / WebAuthn / OTA | 补缺口 + 战略 |

### 9.2 阶段 1 (R18 P0, 1-2 周) — 6 象限 LLM API 深化

**为什么单独 sprint**: protocol + tool-registry 是**纯类型**, 不依赖其他模块, 可以快速完成 + 立竿见影验证 R17 重构成果 (4 协议真接 + Keep-Alive LIFO + 5 步 pipeline).

**6 象限 LLM API 深化**:
1. **trait 深化**: `LlmProvider` + `ProtocolAdapter` 各 5 个 impl (5 Provider × 2 抽象)
2. **message API**: 6 类 message (system/user/assistant/tool/tool_result/error) 完整 NormalizedMessage 实现
3. **error 归一化**: 4 协议错误码 → 统一 `LlmError` enum
4. **retry 策略**: `RetrySuppression` 15s 窗口 + exponential backoff 跨协议
5. **stream 真实**: `Pipeline::run_streaming` 改 `stream_to_sender` (Mavis 亲自改 1 file, 决策 7)
6. **capabilities bitmap**: `ProviderCapabilities` 9 bits (CHAT/STREAMING/TOOLS/VISION/JSON_MODE/SYSTEM_PROMPT/THINKING/LONG_CONTEXT/CUSTOM_TEMPERATURE) 实战验证

**交付**:
- `apeireth-api` 5 Provider × 4 协议 = 20 条 HTTP 路径真接
- `apeireth-protocol` 4 协议 × 6 message = 24 字段级覆盖
- 集成测试覆盖 4 协议 × 5 Provider × 6 message = 120 组合
- example 覆盖 11 个 (已有) + 加 5 Provider × 4 协议 = 20 个

### 9.3 阶段 2 (R18 P0, 1 天) — mid-task bug 3 处修法

**为什么 1 天**: bug 修法明确 (§4.4), 3 处一起改, 不可跳. 1 天包含翻译到 Rust 端 + 单元测试 + 集成测试.

**3 处修法**:
1. `session/SessionManagerV2.ts:636-643` → Rust 端 `apeireth-session::SessionManager::send_message()` 永不 panic
2. `agent/AgentManagerV2.ts:269-286` → Rust 端 `apeireth-agent::AgentManager::send_to_agent()` 加 child session 状态检查 + `await`
3. `agent/AgentManagerV2.ts:458-461` → Rust 端 `apeireth-agent::AgentManager::on_child_session_ended()` 状态变更用 `tokio::sync::broadcast` 事件驱动

**交付**:
- 3 处 Rust 翻译完整
- 3 个单元测试 + 3 个集成测试 + 1 个 e2e 测试 (mid-task 场景)
- m3 minimax hallucination 2 个具体表现 (§4.5) 的缓解措施 (context ≤ 32 messages + UsageEstimator + toolMapping 5 HashMap)

### 9.4 阶段 3 (R19 P1, 1-2 周) — TUI 9 命令深化 + 团队功能借鉴

**TUI 9 命令深化** (基于 R19 已有的 5 nav + 主对话 + 9 器官):
- 1-9 数字键直跳 nav
- a-x 字母键直跳 agent panel (24 个 agent 缩略图)
- Tab 切换 focus
- Enter 进入 panel 全屏
- Esc 返回并排
- / 搜索会话
- ? 帮助
- : 命令面板
- q 退出

**团队功能借鉴到 `apeireth-agent`** (不混 council voting):
- `AgentManager` 新增 `supervisor_mode: bool` 配置项
- `ManagedAgent` struct (从 `AgentManagerV2.ts:963` 翻译)
- `childToAgent` / `parentToAgents` / `waiters` / `idleWaiters` / `agentIdleFlags` 5 个 Map → `HashMap`
- `spawn_agent` / `send_to_agent` / `wait_agent_idle` / `get_agent_output` / `cancel_agent` 5 个调度工具
- **新命名 `apeireth-team-lead`** (避免与 `apeireth-supervisor` PID 1 冲突)

**交付**:
- TUI 9 命令全实装
- `apeireth-agent` 团队协作模式 (MVP 共享 1 LLM, 决策 8)
- `apeireth-team-lead` 新 crate (1:1 翻译 supervisorPrompt, 决策 3)
- 24 agent 并排 TUI 截图 (文字版)

### 9.5 阶段 4 (R20 P1, 2-4 周) — 收产品 + Tauri 团队接入 + OTA 实装

**收产品**:
- `apeireth-storage` (1300 LOC, 11 repo + FTS5)
- `apeireth-git` (1000 LOC, GitWorktreeService 翻译)
- `apeireth-council::task` (200 LOC, TaskSessionCoordinator)
- `apeireth-skill` (400 LOC, SkillEngine + builtin)
- `apeireth-mcp` 从 0 → 1500 LOC (AgentMCPServer 翻译, 8+3+3=14 工具)
- `apeireth-bootstrap` 增强 (manager 初始化 300 LOC)
- `apeireth-tui` 新增 (600 LOC, TUI 启动入口)

**Tauri 团队接入**:
- Tauri 资产沉淀 13 项 T-001 ~ T-013 (`tauri-roadmap-2026-08-05.md`)
- 11 类 IPC handler 翻译为 `#[tauri::command]`
- V1 PTY 备用 (T-001)
- macOS shellPath (T-004)
- electron-updater → tauri-plugin-updater (T-003)

**OTA 实装**:
- Tauri 桌面 app 自动更新
- 增量更新 (JSON patch + binary diff)
- 灰度发布 (10% → 50% → 100%)
- 回滚机制 (snapshot + revert)

**交付**:
- 41 crate 完整产品
- Tauri 桌面 app v0.1.0
- OTA pipeline 文档

### 9.6 阶段 5 (R21+ P2, 4-8 周) — 补缺口

**5 处补缺口**:
1. **Council 真实 LLM** (7 advisor 各不同 provider): R20 阶段后, 7 advisor 可独立配置 LLM (按主 vs 按住 vs 拟人化)
2. **SseTransport 实装**: `apeireth-mcp::transport::SseTransport` skeleton → 完整实现
3. **graph ↔ supervisor 集成**: `apeireth-graph` 注释的 `SupervisorSnapshot` 展开, 子进程级 flow 跑起来
4. **WebAuthn**: 主人 R20 收产品时定, 阶段 5 实现
5. **OTA 完善**: 增量算法优化 (zstd chunk + 签名验证)

**Tauri 终极** (user memory #8 战略):
- 主人干 TUI/后端, AI 团队干 Tauri 设计
- TUI 是 Tauri 的"集成测试床", 后端 API 表面 / 集成模式 / 用户流都在 TUI 跑稳
- Tauri 来了无缝换 UI 层 (TUI HTTP 瘦客户端模式)

**交付**:
- Council 7 advisor 独立 LLM 配置
- MCP SSE/HTTP Streamable 实装
- graph ↔ supervisor 完整集成
- WebAuthn 认证
- OTA zstd chunk + 签名
- Tauri 桌面 app v1.0.0

---

## §10 不修改承诺 (8 项, 主人硬约束 100% 守住)

| ❌ 不修改 | 原因 / 引用 |
|---------|-----------|
| **1. 阶段 1+2+3 文档** (LOCKED) | 主人明确沉淀, 阶段 4 仅引用不重写 |
| **2. v2 / v4 / v4.1 LOCKED** | 哲学层纲领 (BF896EEF / af0d1957 / 4aa3c5b0) |
| **3. 阶段 4 核心文档 LOCKED** (`6ca80776`) | 我们**新增** `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` 不冲突 |
| **4. 阶段 5 施工文档 LOCKED** (631 行) | 阶段 5 实施时再引用 |
| **5. v6 基础架构** | 主 AI 团队已 LOCKED |
| **6. R11 baseline** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 主人 2026-07-31 明确不动 |
| **7. APEIRETH-CONVENTIONS / VERSIONING / GLOSSARY** (顶层 3 文件) | 不动 |
| **8. START-CONSTRUCTION.md** | 不动 |
| **附加: `apeireth-legacy`** | R17 finalize 后归档, 不删 |
| **附加: workspace version 1.0.0** | semver 严格, 不动 |

**6 主哲学锚穿透检查** (按 APEIRETH-CONVENTIONS §9):
- [x] S-1 主 22:33 北极星导向 — 服务 ASI 北极星 (R0.5 V2 24 维 → trait sketch)
- [x] S-2 主 17:43 实事求是 — 3 份 sub-agent 报告 + 实际现状, 不重写 LOCKED
- [x] O-5 主 17:58 不假装 — mid-task bug 3 处一起改, 不假装已实现
- [x] O-2 主 19:33 走在前人经验上 — 借 R11 baseline + R17 4 协议 + R19 41 crate 既有能力
- [x] O-3 主 23:44 干到底 — 5 阶段路线 + 12 表格 + 拍板项立即落
- [x] O-4 主 00:56 任何人都能接手 — 12 章节 + 完整映射表 + 附录 A/B 引用

---

## §11 风险清单

| 风险 | 影响 | 概率 | 缓解 | 优先级 |
|------|------|------|------|--------|
| **mid-task bug 修法 3 处一起改** | 高 — 改一处不完整会引入新 bug | 中 | ① 3 处一起改 PR; ② 单元测试 + 集成测试 + e2e 测试 3 套覆盖; ③ 阶段 2 单独 sprint 1 天专注 | **P0** |
| **WebSocket 砍掉后失去未来第三方 MCP 客户端兼容** | 中 — 失去跨进程扩展能力 | 中 | ① R20+ 阶段评估; ② Tauri 阶段如需要再实装 `apeireth-bus` L4 (已有 feature `full-bus`); ③ 文档明示砍的理由 | **P1** |
| **5 Provider 集成的工作量** | 高 — 5x 工作量, 估 5x 1000 LOC | 中 | ① 5 Provider 走 `apeireth-api` 4 协议端点 + base_url 配置 (零代码改); ② napi-rs 桥只桥 Claude SDK, 其他 4 个 provider 用各自 CLI wrapper; ③ toolMapping 5 HashMap 统一 | **P0** |
| **TUI 改瘦后 SpectrAI 工具覆盖可能不完整** | 中 — 某些 SpectrAI 高级功能缺失 | 中 | ① SpectrAI 19 模块中 7 个沉淀到 Tauri (T-001 ~ T-013); ② TUI MVP 跑通核心 (chat + team + worktree); ③ R20+ 阶段补 parser / skill / notify / builtin mcp | **P1** |
| **`apeireth-mcp` LOCKED 0 代码填坑风险** | 中 — 填坑可能改 LOCKED 设想 | 低 | ① 用 `rmcp` crate (官方 Rust MCP SDK) 严格按 MCP 协议; ② 不修改 ADR 0007; ③ 新增 ADR 记录 "apeireth-mcp 来自 SpectrAI AgentMCPServer 翻译" | **P0** |
| **minimax m3 48+ context hallucination** | 高 — 影响 LLM 调用链 | 高 | ① 限制单次 context ≤ 32 messages; ② 强化 mid-task bug 修复; ③ UsageEstimator 监控 token 计数 | **P0** |
| **命名空间冲突: SpectrAI "supervisor" ≠ Apeireth `apeireth-supervisor`** | 高 — 接手者必踩坑 | 高 | ① **新命名 `apeireth-team-lead`** (§5.1); ② supervisorPrompt 1:1 翻译到 `team_lead.md`; ③ 写 ADR 说明命名决策 | **P0** |
| **V1 PTY 路径在 Rust 重写中复活** | 中 — 双轨并存 | 中 | ① 架构文档明确写 "V1 PTY 不集成 TUI, 只 Tauri 备用"; ② Review 检查; ③ 沉淀到 T-001 | **P0** |
| **`Pipeline::run_streaming` 真 SSE 改 1 file** | 中 — 改坏影响所有流式 | 低 | ① Mavis 亲自改 (主人决策 7); ② 复用 `stream_to_sender` + `bytes_stream` (已有); ③ 集成测试覆盖 4 协议 × 5 Provider × 流式 | **P0** |
| **FTS5 中文分词差** | 低 — 搜索结果可能不一致 | 低 | 改用 `tokenize='trigram'` (SQLite 3.34+) | **P2** |
| **GitWorktreeService 在 Windows 长路径失败** | 中 — worktree 路径长 | 中 | `\\?\` 前缀; `apeireth-git` 跨平台包装 | **P1** |
| **AgentMCPServer 893 LOC 重写丢失工具** | 中 — 团队能力缺失 | 中 | 完整列出 8 调度 + 3 worktree + 3 感知 = 14 工具, 逐个测试 | **P0** |
| **Tauri 阶段沉淀项可能未及时回看** | 中 — 用了找不到 | 中 | 每项有唯一 ID (T-001 ~ T-013) + 触发条件 | **P2** |
| **FileChangeTracker 排除规则不完整** | 低 | 低 | 复制 EXCLUDE_PATTERNS 数组; `notify` crate 替代 `fs.FSWatcher` | **P2** |
| **m3 minimax API 与 Anthropic 协议差异** | 高 — 核心功能 | 高 | ① `apeireth-protocol` 已有 minimax 适配; ② ClaudeSdkAdapter 翻译时协议层用 minimax, SDK 壳仍走 Claude Code; ③ minimax 的 `path` 错误要在 validation 层拦截 | **P0** |

---

## §12 下一步行动

| 行动 | 责任 | 时间 | 拍板? |
|------|------|------|--------|
| **A 方案命名已拍板** (§8 决策 3 = supervisorPrompt → `apeireth-team-lead`) | 主人 | 2026-08-05 13:34 | ✅ 已拍板 |
| **派 sub-agent 实施阶段 1** (R18 P0, 1-2 周) | Mavis → sub-agent | 阶段 1 sprint 启动 | 🟢 拍板后启动 |
| **Mavis 亲自改 `Pipeline::run_streaming` 真 SSE** (决策 7) | Mavis | 阶段 1 内 | 🟢 拍板后启动 |
| **Tauri 团队同步 SpectrAI Electron 资产** | Tauri team | 阶段 4 启动前 (R20) | ⚪ 战略级 |
| **R20 收产品时把 SpectrAI 整合作为 case study** | Mavis → Apeireth leader | 2026 Q4 | ⚪ 战略级 |
| **Apeireth leader 复核本文档** | Apeireth leader | 2026-08-06 | ⚠️ 必复核 |
| **写 ADR: `apeireth-mcp` 来自 SpectrAI AgentMCPServer 翻译** | Mavis | 阶段 2 启动前 | 🟡 待写 |
| **写 ADR: `apeireth-team-lead` 新命名决策** | Mavis | 阶段 1 启动前 | 🟡 待写 |
| **更新 GLOSSARY.md** 加 spectrAI / SpectrAI / supervisor / team-lead 词条 | Mavis | 阶段 1 启动前 | 🟡 待写 |
| **更新 APEIRETH-CONVENTIONS.md** §5 报告路径加 `spectrAI-integration-blueprint` 模板 | Mavis | 阶段 1 启动前 | 🟡 待写 |

---

## 附录 A: 引用 sub-agent 报告 (3 份, 2026-08-05)

| 报告 | 路径 | 大小 | 章节 | 核心 |
|---|---|---|---|---|
| **spectrai-architecture** | `.minimax-agent-cn\spectrai\reports\spectrai-architecture-2026-08-05.md` | 61.2 KB | §0-§11 + 附录 A (34 文件清单) | 19 模块架构图 + 5 sequence + 集成映射表 (§5) + mid-task bug 根因 (§6) + 主人 5 决策 (§8) + TUI 风格推荐 (§9) + 11 周路线 (§5.3) |
| **apeireth-crate-api** | `.minimax-agent-cn\spectrai\reports\apeireth-crate-api-2026-08-05.md` | 45.4 KB | §0-§5 + 关键发现 | 9 crate API surface 详细 (council/agent/graph/mcp/pipeline/tool-runtime/tool-registry/tool-approval/protocol/supervisor) + 跨 crate 集成现状 (§3) + 真实 LLM 集成缺口清单 (§4) + 推荐集成顺序 (§5) |
| **apeireth-platform-modules** | `.minimax-agent-cn\spectrai\reports\apeireth-platform-modules-2026-08-05.md` | 26.2 KB | §0-§8 | **apeireth-api 是 LLM 整合点** (R17 重构, 唯一) + 41 crate 总览 (§1) + `apeireth-api` 详细 (§2) + compat/平台/桥接映射 (§3) + HTTP client + 5 Provider 映射 (§4-§5) + 跨模块集成现状 (§6) + 关键发现 (§7) + 需 Mavis 拍板 (§8) |

**配套沉淀文档** (Tauri 阶段用):
- `tauri-roadmap-2026-08-05.md` (32.5 KB) — TUI 不需要但 Tauri 阶段需要的 13 项资产沉淀 (T-001 ~ T-013)

---

## 附录 B: 关键源码引用 (SpectrAI, file:line 完整清单)

### B.1 mid-task bug 3 处 (必改)

| 文件:行 | 描述 |
|---------|------|
| `session/SessionManagerV2.ts:636-643` | ① 终态用 throw 而非 return (line 642 ⚠️) |
| `agent/AgentManagerV2.ts:269-286` | ② sendToAgent 不检查终态 + 永远 success (line 281 ⚠️ + 285) |
| `agent/AgentManagerV2.ts:458-461` | ③ child session 状态变化到 agent 状态变化窗口期 (line 460) |
| `agent/AgentManagerV2.ts:88-105` | `listenEvents` `onSessionStatusChange` 事件转发 (修法 3 涉及) |
| `agent/AgentManagerV2.ts:90-96` | child session 'running' → agent.info.status = 'running' (line 96) |
| `agent/AgentManagerV2.ts:319` | `wait_agent_idle` 5 分钟超时返回 `{ idle: false, output: '' }` |
| `session/SessionManagerV2.ts:600-700, 700-1000` | V2 核心 (状态机 + 调度器策略) |

### B.2 团队消息 (P0 翻译)

| 文件:行 | 描述 |
|---------|------|
| `agent/AgentMCPServer.ts` (893 LOC) | 8 调度 + 3 supervisor worktree + 3 感知 = 14 工具 |
| `agent/AgentManagerV2.ts` (963 LOC) | `ManagedAgent` / `childToAgent` / `parentToAgents` / `waiters` / `idleWaiters` / `agentIdleFlags` |
| `agent/MCPConfigGenerator.ts` (485 LOC) | 3 分支: Claude/iFlow (JSON 临时文件) + Codex (CODEX_HOME 目录) + OpenCode (OPENCODE_CONFIG 路径) |
| `agent/AgentBridge.ts` (130 LOC) | WebSocket 桥 (用户决策 2: 砍) |
| `agent/supervisorPrompt.ts` (808 LOC) | 818 行 markdown (用户决策 3: 1:1 翻译) |
| `agent/providerAvailability.ts` (110 LOC) | 5 Provider 可用性检测 |
| `agent/types.ts` | AgentInfo / AgentResult / BridgeRequest / BridgeResponse (BridgeRequest/Response 砍) |

### B.3 启动 (P0 翻译, 只取 manager 初始化 300 LOC)

| 文件:行 | 描述 |
|---------|------|
| `main/index.ts` (851 LOC) | 启动编排 |
| `main/index.ts:235-379` | `initializeManagers` (manager 初始化, 取 300 LOC) |
| `main/index.ts:385+` | `wireEvents` (事件接线) |
| `main/index.ts:851` | 完整 bootstrap |
| `main/ipc.ts` (4 LOC) | shim (沉淀 T-002) |
| `main/ipc/index.ts` (82 LOC) | IPC handler 总注册 (沉淀 T-002) |

### B.4 Adapter (P0 翻译, 5 Provider)

| 文件:行 | 描述 |
|---------|------|
| `adapter/types.ts` (182 LOC) | `ProviderEvent` / `AdapterSessionConfig` / `BaseProviderAdapter` |
| `adapter/AdapterRegistry.ts` (96 LOC) | 路由表 |
| `adapter/toolMapping.ts` (259 LOC) | 5 Provider 工具名 → ActivityEventType 统一 |
| `adapter/ClaudeSdkAdapter.ts` (1742 LOC) | Claude Agent SDK V1 `query()` (P0 napi-rs 桥) |
| `adapter/CodexAppServerAdapter.ts` (1098 LOC) | Codex AppServer 协议 |
| `adapter/IFlowAcpAdapter.ts` (767 LOC) | iFlow ACP 协议 |
| `adapter/OpenCodeSdkAdapter.ts` (696 LOC) | OpenCode HTTP server |
| `adapter/GeminiHeadlessAdapter.ts` (632 LOC) | Gemini headless mode |

### B.5 其他 (P1-P3 / 沉淀)

| 文件:行 | 描述 |
|---------|------|
| `storage/Database.ts` (319 LOC) | SQLite + 11 repo (P1 `apeireth-storage`) |
| `storage/migrations.ts` (463 LOC) | 11 迁移 (P1) |
| `git/GitWorktreeService.ts` (746 LOC) | Worktree 服务 (P1 `apeireth-git`) |
| `parser/OutputParser.ts` (534 LOC) | 终端输出正则解析 (P2 简化) |
| `parser/StateInference.ts` (413 LOC) | 状态推断 (P2) |
| `parser/UsageEstimator.ts` (163 LOC) | token 用量估算 (P2) |
| `parser/rules.ts` (180 LOC) | 正则规则 (沉淀 T-012) |
| `reader/OutputReaderManager.ts` (75 LOC) | 结构化输出 (沉淀 T-005) |
| `reader/ClaudeJsonlReader.ts` (402 LOC) | JSONL 审计 (沉淀 T-005) |
| `tracker/FileChangeTracker.ts` (511 LOC) | FS 改动追踪 (沉淀 T-013) |
| `skill/SkillEngine.ts` (83 LOC) | 模板引擎 (P2 `apeireth-skill`) |
| `skill/builtinSkills.ts` (241 LOC) | 内置 skill (P2) |
| `update/UpdateManager.ts` (244 LOC) | electron-updater (沉淀 T-003) |
| `tray/TrayManager.ts` (216 LOC) | 系统托盘 (沉淀 T-003) |
| `notification/NotificationManager.ts` (200 LOC) | 系统通知 (P3 `apeireth-notify`) |
| `mcp/builtinMcps.ts` (177 LOC) | 内置 MCP 预置 (P3 `apeireth-mcp::builtin`) |
| `node/NodeVersionResolver.ts` (131 LOC) | nvm Node 解析 (P3 `apeireth-node`) |
| `task/TaskSessionCoordinator.ts` (123 LOC) | 任务-会话联动 (P1 `apeireth-council::task`) |
| `bootstrap/shellPath.ts` (107 LOC) | macOS PATH 恢复 (沉淀 T-004) |
| `utils/proxyUtils.ts` (81 LOC) | 工具函数 (沉淀 T-006) |
| `session/ConcurrencyGuard.ts` (188 LOC) | 并发控制 (P1 `apeireth-agent::concurrency`) |

---

**报告结束**。

> **附**: 任何接手者快速路径:
> - Mavis 整合: §1 战略 + §5 映射表 + §8 决策清单
> - 阶段 1 sub-agent 实施: §9.2 + §6 + §4 mid-task 原则
> - 阶段 2 sub-agent 修 bug: §4.4 3 处修法 + §9.3
> - 阶段 3 sub-agent TUI + 团队: §9.4 + §5.1 命名冲突
> - 阶段 4 sub-agent 收产品: §9.5 + §6 41 crate 总览
> - 阶段 5 sub-agent 补缺口: §9.6 + §7 4 处缺口
> - Apeireth leader 复核: §10 不修改承诺 + §1.3 战略原则 + §8 决策清单

---

## 附录 C: 拍板记录

- **2026-08-05 13:34** - 主人拍板 A 方案：`apeireth-team-lead`（新 crate 命名）
  - 理由：明确"团队 + leader"角色，跟 `apeireth-supervisor`（进程监督）区分
  - 决策者：Mavis 默认 + 主人拍板
  - 影响：解锁 ADR-0011, ADR-0010 §8 决策清单, ARCHITECTURE.md §5.2 映射表
  - 后续：等 Cargo.toml 完工（code_reviewer 在改），立即创建 `crates/apeireth-team-lead/`
