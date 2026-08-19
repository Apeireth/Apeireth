# SpectrAI 多 Agent 模式 → Apeireth 借鉴挖掘报告（R-side survey）

```
[Document-Meta]
Document:        reports/spectrai-multiagent-borrow-survey-2026-08-19.md
Version:         0.1-R-survey (research only, 0 改任何源码)
R-Cycle:         R-side survey (Mavis 派活前的研究性摸底)
Last-Modified:   2026-08-19
Status:          🟡 Survey (待主人/Mavis 拍板哪些借哪些不借)
Source-of-Truth: spectrai-source/src/main/{agent,session,task,bootstrap}/ + apeireth-council/src/ + apeireth-runtime/src/
0 主动 commit:   严守 (写到 reports/ 但不 commit, 等整合 #5 commit 时机拍板)
0 主动 push:     严守 (等 1.0 release 配 GitHub remote)
```

> **本报告性质**: **挖掘 + 评估**, 0 改任何源码。基于 `.minimax-agent-cn\spectrai\spectrai-source\src\main\` 真实反编译源 + 已有的 apeireth-council / runtime / team-lead 实装代码做交叉对照。
>
> **读者**: 主人/Mavis 拍板哪些值得借哪些不借。后续真实施时拆成 R 阶段任务。
>
> **诚实标注**: 借鉴 ≠ 复制。SpectrAI 是 Electron + TypeScript, Apeireth 是 Rust + tokio, 翻译不是 1:1 复刻。借鉴的是**模式/算法/状态机**, 不是具体语法。

---

## §0 TL;DR

**直接结论**: SpectrAI 的多 agent 编排**有 10 个可借鉴模式**, Apeireth 当前只借鉴了其中 **3 个** (TaskSessionCoordinator 规划进 council::task 但实际**未实施**, supervisor prompt 1:1 翻译进 team-lead, ApprovalBridge)。其余 7 个**完全没碰**, 其中 3 个是**高价值低成本**的可立即落地项。

### 推荐 Top 3 (建议立即拍板)

| 优先级 | 借鉴 ID | 模式 | 来源 | 落地路径 | 估时 |
|---|---|---|---|---|---|
| 🔴 P0 | **MA-1** | `TaskSessionCoordinator` (会话→任务状态自动联动) | `spectrai-source/src/main/task/TaskSessionCoordinator.ts:123` | 补 `apeireth-council::task` 模块 (blueprint 已规划但未实装) | 2-3 天 |
| 🔴 P0 | **MA-4** | `ConcurrencyGuard` (9 会话上限 + 内存/CPU 阈值) | `spectrai-source/src/main/session/ConcurrencyGuard.ts:188` | 加进 `apeireth-runtime` 启动路径 | 1 天 |
| 🟡 P1 | **MA-7** | **Per-task-type provider 路由 + fallback chain** | `supervisorPrompt.ts:168-185` (表格) | 加进 `apeireth-multi-model-backend` (现有 R269 升级) | 3-5 天 |

### 其余 4 个值得但需要设计判断

| 借鉴 ID | 模式 | 来源 | 落地难度 | 设计判断 |
|---|---|---|---|---|
| MA-2 | `AgentManagerV2` 父子 session 树 | `AgentManagerV2.ts:963` | 高 (3-5 周) | **已经在 team-lead 翻译过了**, 无需重复 |
| MA-3 | `turn_complete` 事件驱动 + `agentIdleFlags` 竞态修复 | `AgentManagerV2.ts:104-117` | 中 (1 周) | runtime 当前用 heartbeat, event-driven 是改进方向 |
| MA-5 | `McpSessionMode` (supervisor/member/awareness 三档) | `agent/types.ts:4` | 中 (1 周) | 哲学上跟 RoleConstitution 重叠, 需先想清楚 |
| MA-6 | `wait_agent` 轮询 vs 长等安全 | `supervisorPrompt.ts:236-247` | 低 (3 天) | 加进 team-lead + runtime |

### 不推荐 (跟架构哲学冲突)

- ❌ **AgentBridge WebSocket 桥** (MA-8) — 主人已砍 (per blueprint §3 决策 2)
- ❌ **parseInteractiveQuestion** (MA-10) — 跟现有 ApprovalBridge + onion 双层审批重复
- ❌ **进度上报强制 1 阶段 1 次** (MA-9) — 跟 Heartbeat 自带 telemetry 重复

---

## §1 SpectrAI 多 agent 全景

### 1.1 19 模块中跟多 agent 协作相关的有 6 个

| 模块 | 文件 | LOC | 跟多 agent 协作的关联度 |
|---|---|---:|---|
| **agent** | 11 文件, 5610 LOC | 🔴 P0 | `AgentManagerV2` (963 LOC) 是核心, `supervisorPrompt` (808 LOC) 是大脑, `AgentMCPServer` (700 LOC) 是工具总线 |
| **session** | 4 文件, 2862 LOC | 🔴 P0 | `SessionManagerV2` (1231 LOC) + `ConcurrencyGuard` (188 LOC) |
| **task** | 1 文件, 123 LOC | 🔴 P0 | `TaskSessionCoordinator` (整个文件就这一个 class) |
| **bootstrap** | 1 文件, 107 LOC | ⚪ 沉淀 | `shellPath.ts` macOS PATH 恢复 (跟多 agent 无关) |
| **ipc** | 14 文件, 3797 LOC | ⚪ 沉淀 | IPC handler 沉淀到 tauri-roadmap |
| **storage** | 16 文件, 3116 LOC | 🟡 P1 | DB schema 设计可借鉴 (apeireth-storage 待建) |

### 1.2 SpectrAI 多 agent 的核心抽象 (3 层)

```
┌─────────────────────────────────────────────────────┐
│ Layer 1: Supervisor (总指挥, 通常 1 个)               │
│   - supervisorPrompt 注入 buildSupervisorPrompt()  │
│   - 持有 AgentManagerV2 + SessionManagerV2          │
│   - 可以 spawn_agent / wait_agent_idle / cancel_agent│
│   - 通过 WebSocket 桥 跟子 Agent 通信               │
├─────────────────────────────────────────────────────┤
│ Layer 2: Agent Manager (orchestrator)                │
│   - AgentManagerV2 维护 agents Map                   │
│   - 父子关系: parentSessionId → childSessionId       │
│   - 状态机: pending→running→completed/failed/cancelled│
│   - oneShot vs persistent 两种生命周期               │
│   - agentIdleFlags 处理 turn_complete 竞态          │
├─────────────────────────────────────────────────────┤
│ Layer 3: Session (实际 LLM 进程)                     │
│   - SessionManagerV2 维护 sessions Map               │
│   - 每个 session 绑 1 个 AIProvider (claude/codex/..) │
│   - sessionStatus: pending/running/idle/waiting_input│
│                  /completed/error/terminated         │
│   - turn_complete 事件 + activity 事件流              │
└─────────────────────────────────────────────────────┘
```

**对比 Apeireth 当前架构**:

| 维度 | SpectrAI | Apeireth 当前 | 评估 |
|---|---|---|---|
| Supervisor 概念 | `supervisorPrompt` + AgentManagerV2 | `apeireth-team-lead` (刚实装, 14 调度工具) | ✅ 已借鉴 (1:1 翻译完成) |
| 父子 session | parentSessionId → childSessionId | `GroupChat` 是**扁平**多 agent | ⚠️ 缺失 (扁平够不够?) |
| 状态机 | 6 状态 + 事件流 | Heartbeat tick + 6 channel bus | 🟡 替代方案 |
| 资源守卫 | ConcurrencyGuard (max 9 + 内存/CPU) | 无 | ❌ 缺失 |
| 任务-会话联动 | TaskSessionCoordinator (1s debounce) | 无 | ❌ 缺失 (blueprint 规划未实装) |
| 多模型路由 | provider 选择表 + fallback chain | Multi-Model Backend (3 策略) | 🟡 部分覆盖 |
| 跨进程隔离 | WebSocket 桥 + per-session MCP | 进程内 (bus 3-channel) | ✅ 已砍 (in-process) |

---

## §2 10 个可借鉴模式详细分析

### MA-1: TaskSessionCoordinator — 会话状态→任务状态自动联动 🔴 P0

**来源**: `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\task\TaskSessionCoordinator.ts` (123 LOC, 单文件)

**核心算法**:

```typescript
// SESSION_TO_TASK: 会话状态 → 任务状态映射
const SESSION_TO_TASK = {
  running:       { target: 'in_progress', validFrom: ['todo', 'waiting'] },
  idle:          { target: 'in_progress', validFrom: ['todo', 'waiting'] },
  waiting_input: { target: 'waiting',     validFrom: ['in_progress'] },
  error:         { target: 'waiting',     validFrom: ['in_progress'] },
}

// ACTIVITY_TO_TASK: 活动事件 → 任务状态
const ACTIVITY_TO_TASK = {
  task_complete:        'done',
  error:               'waiting',
  waiting_confirmation: 'waiting',
}

// 1 秒 debounce + validFrom 检查 (合法状态迁移)
```

**亮点**:
1. **debounce 1 秒**: 同 task 多次状态变化合并成 1 次 DB 写入
2. **validFrom 守卫**: 只在合法前置状态时迁移 (避免非法状态跃迁)
3. **多会话边界处理**: session 终止时检查同 task 还有没有其他活跃 session (`handleSessionCompleted`)
4. **cleanup**: 析构时清所有 debounce 定时器

**Apeireth 现状**:
- blueprint §3 #16 明确规划进 `apeireth-council::task` (200 LOC)
- **实际**: `crates\apeireth-council\src\task*` **不存在** (我 grep 确认过)
- 当前 runtime 用 `AsyncTaskStore` (`apeireth-tool-registry`), 是单 session 单 task 模型, **缺多 session → 1 task 的联动逻辑**

**借鉴价值**: 🔴 **极高**。当前你的 `R147 runtime orchestration` 7 模块链路里, group_chat + arbitration + search 都没接 task 状态自动联动, 这是个明显的洞。

**建议落地**:
- 路径: `crates\apeireth-council\src\task\session_coordinator.rs` (Rust 翻译)
- 跟现有 `AsyncTaskStore` 集成 (publish 到 `apeireth-bus` Channel C)
- 加 `bus_bridge.rs` 监听 session status 事件, 调 `applyTaskUpdate()`
- 加 organ_kani_proofs (沿用项目惯例)
- 估时: 2-3 天 (含测试)

**Rust 翻译核心 (基于蓝图已有)**:
```rust
pub enum SessionStatus { Pending, Running, Idle, WaitingInput, Completed, Error, Terminated }
pub enum TaskStatus { Todo, InProgress, Waiting, Done, Failed, Cancelled }

pub struct SessionToTaskRule { target: TaskStatus, valid_from: Vec<TaskStatus> }
// SESSION_TO_TASK: HashMap<SessionStatus, SessionToTaskRule>

pub struct TaskSessionCoordinator {
    debounce_timers: HashMap<TaskId, tokio::time::Sleep>,
    bus: ChanneledBus,
    task_store: Arc<AsyncTaskStore>,
}

impl TaskSessionCoordinator {
    pub async fn on_session_status_change(&self, session_id: SessionId, status: SessionStatus) {
        let Some(task_id) = self.find_task_for_session(session_id) else { return };
        let Some(rule) = SESSION_TO_TASK.get(&status) else {
            if matches!(status, Completed | Terminated) {
                self.handle_session_completed(task_id, session_id).await;
            }
            return;
        };
        self.debounced_update(task_id, rule.target, &rule.valid_from).await;
    }
}
```

---

### MA-2: AgentManagerV2 父子 session 树 🟡 P1 (已借鉴, 验证实施度)

**来源**: `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\AgentManagerV2.ts` (963 LOC)

**核心数据结构**:

```typescript
interface ManagedAgent {
  info: AgentInfo                    // agentId/parentSessionId/childSessionId/status/prompt/workDir
  childSessionId: string
  parentSessionId: string
  oneShot: boolean                   // true=任务完成自动退出 / false=持久会话
  providerId: string                 // 子 Agent 用哪个 LLM provider
}

class AgentManagerV2 {
  private agents: Map<agentId, ManagedAgent>           // 所有 agent
  private childToAgent: Map<childSessionId, agentId>   // 反向索引
  private parentToAgents: Map<parentSessionId, Set<agentId>>  // 父→子 集合
  private waiters: Map<agentId, Array<{ resolve, timer }>>     // wait_agent 等待
  private idleWaiters: Map<agentId, ...>              // wait_agent_idle 等待
  private agentIdleFlags: Map<agentId, boolean>       // ★ 竞态修复
}
```

**核心方法** (8 个):
- `spawnAgent(parentSessionId, config)` → 创建子 agent + 父子映射 + DB 持久化 + MCP 配置
- `sendToAgent(agentId, message)` → 发送消息 + 清 idle flag
- `waitAgentIdle(agentId, timeout?)` → 等待 turn_complete 事件 (idle flag 竞态修复)
- `waitAgent(agentId, timeout?)` → 等待 agent 完成 (completed/failed/cancelled)
- `getAgentOutput(agentId, lines?)` → 从对话消息提取输出
- `getAgentStatus(agentId)` → 当前状态
- `listAgents(parentSessionId?)` → 列出子 agents
- `cancelAgent(agentId)` → 终止 (清 idle flag + 终止 session)

**Apeireth 现状**:
- `apeireth-team-lead` lib.rs:32-36 **明确标注**: "1:1 翻译 v0.9.21 商业版 out/main/agent/AgentMCPServer.js Orchestrator 估缺 P0, A 方案 13:34 拍板命名"
- 已实装的 14 工具白名单 (lib.rs:48-72): spawn_agent / send_to_agent / get_agent_output / wait_agent_idle / wait_agent / get_agent_status / list_agents / cancel_agent + 3 worktree + 3 感知
- supervisor_prompt.md 已 1:1 翻译完成 (含 818 行 prompt)

**借鉴评估**: ✅ **已借鉴 + 翻译完成**。但**实施度未 verify**。

**建议落地**:
- 跑 `cargo test -p apeireth-team-lead` 看 5 fixture test (SpawnAgent / SendMessage no swallow / DualAck / CancelAgent / K-1 invariant)
- 验证 8 调度工具的 rust 端 trait impl (Orchestrator trait 14 工具)
- 验证 `approval_bridge.rs` 跟 companion::approval_requests 真接
- **不要再造一遍**, 已有就 skip

---

### MA-3: turn_complete 事件驱动 + agentIdleFlags 竞态修复 🟡 P1

**来源**: `AgentManagerV2.ts:104-117` + `:170-185`

**核心问题**: wait_agent_idle 跟 task_complete 事件有**竞态**
- 如果 task_complete 在 wait_agent_idle 注册前先触发, waiter 永远等不到
- SpectrAI 修复: `agentIdleFlags: Map<agentId, boolean>` 保留已完成状态

**修复算法**:
```typescript
// 注册 wait_agent_idle 时:
if (this.agentIdleFlags.get(agentId)) {
    this.agentIdleFlags.delete(agentId)
    return { idle: true, output }  // ★ 立即返回, 不再等
}
// turn_complete 事件触发时:
this.agentIdleFlags.set(agentId, true)  // 保留状态, 即使没有 waiter
```

**Apeireth 现状**:
- `apeireth-runtime` 用 Heartbeat tick (DEFAULT_TICK_INTERVAL_SECS=10) **轮询**
- 没有 turn_complete 事件概念
- 没有 wait_agent_idle 等价物 (`AsyncTaskStore` 是 register/pending/running/completed 4 状态, 不是 turn-based)

**借鉴价值**: 🟡 **中等**。Runtime 心跳轮询适合批量任务, 但**单 agent 等待**用事件更优。

**建议落地** (可选, 看是否值得):
- 给 `AsyncTaskStore` 加 `turn_complete` 事件 (`tokio::sync::broadcast`)
- 给 `GroupChat` 加 `wait_idle` API (per session)
- 估时: 1 周
- **判断**: 如果只是后台跑批, 心跳轮询够; 如果要做实时 multi-agent 交互, 必须做

---

### MA-4: ConcurrencyGuard — 资源守卫 🔴 P0

**来源**: `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\session\ConcurrencyGuard.ts` (188 LOC)

**核心配置**:
```typescript
{
  maxSessions: 9,
  minMemoryMB: 256 on macOS, 512 elsewhere,
  maxCpuPercent: 90,
}
```

**核心 API**:
- `canCreateSession()` → bool (快速检查)
- `checkResources()` → `{ canCreate, reason?, currentSessions, memoryUsagePercent, availableMemoryMB }`
- `registerSession()` / `unregisterSession()` → 计数
- `shouldWarnResources()` → `{ warn: bool, message? }` (内存 >85% 或 session >80% 触发)

**亮点**:
- macOS 特殊: 用 `vm_stat` 解析 Pages free/inactive/speculative/purgeable 综合判断 (不只是 free)
- 默认 9 上限是**经验值** (SpectrAI 团队跑出来的)
- 阈值警告是软性的 (warn), 创建拒绝是硬性的 (block)

**Apeireth 现状**:
- `apeireth-runtime` 启动时 `Runtime::new()` 直接构造, 无资源检查
- 7 模块 (heartbeat/task/bus/arbitration/search/group_chat/emotion) 都可以无限制 spawn 任务
- **没有 maxSessions 上限**, 内存爆了只能等 OS kill

**借鉴价值**: 🔴 **极高 + 低成本**。

**建议落地**:
- 新建 `apeireth-runtime::resource_guard` 模块 (或 `apeireth-supervisor::resource_guard`)
- 平台判断: macOS 走 vm_stat, Windows 用 `GetPhysicallyInstalledSystemMemory` (kernel32), Linux 走 `/proc/meminfo`
- 用 `sysinfo` crate (blueprint 已提, 当前 0 用)
- 默认值: maxSessions=9 (对齐 SpectrAI), minMemoryMB=512, maxCpuPercent=90
- 跟 HeartbeatScheduler 集成: 每个 tick 检查 shouldWarnResources
- 估时: 1 天 (跨平台需要测试)

**Rust 翻译核心**:
```rust
pub struct ResourceGuard {
    config: ResourceConfig,
    active_sessions: AtomicUsize,
}

pub struct ResourceConfig {
    pub max_sessions: usize,          // 默认 9
    pub min_memory_mb: u64,           // 默认 512 (macOS 256)
    pub max_cpu_percent: f32,         // 默认 0.90
    pub warn_memory_percent: f32,     // 默认 0.85
    pub warn_session_percent: f32,    // 默认 0.80
}

impl ResourceGuard {
    pub fn can_create(&self) -> bool;
    pub fn check(&self) -> ResourceStatus;
    pub fn register(&self);
    pub fn unregister(&self);
    pub fn should_warn(&self) -> Option<WarnMessage>;
}
```

---

### MA-5: McpSessionMode — supervisor/member/awareness 三档 🟡 P1

**来源**: `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\types.ts:4`

```typescript
type McpSessionMode = 'supervisor' | 'member' | 'awareness'
```

**三档语义**:
| 模式 | 工具可见性 | 场景 |
|---|---|---|
| `supervisor` | 全 14 工具 (调度 + worktree + 感知) | 总指挥会话 |
| `member` | 调度工具不可见 (防止子 agent 再 spawn) | 普通子 agent |
| `awareness` | 只读感知工具 (list_sessions/get_session_summary/search_sessions) | 观察者 |

**Apeireth 现状**:
- `apeireth-team-lead::TOOL_WHITELIST` (lib.rs:48-72) 是**单一白名单** (14 工具全开)
- 没有 per-session 模式区分
- `apeireth-council::RoleConstitution` 5 字段 (`physical_isolation`/`l0_ha_required`/`jurisdiction_bounds`/`compile_time_hardcoded`/`philosophical_anchors`) 是 per-advisor 宪法, 跟 per-session 模式不冲突但**重叠**

**借鉴价值**: 🟡 **中等**。

**设计判断**:
- 如果做"子 agent 不能 spawn 子子 agent"这种**递归限制**, 这个模式很实用
- 但跟你现有的 RoleConstitution 功能重叠 — 是加进 RoleConstitution 还是新加 McpSessionMode?
- **建议**: 加进 RoleConstitution 第 6 字段 `mcp_session_mode: McpSessionMode` (编译期 hardcode enum), 不新加概念

**建议落地**:
- 改 `apeireth-council::constitution::RoleConstitution` 加 1 字段
- `apeireth-team-lead` 在 spawn_agent 时按 mode 过滤可见工具集
- 估时: 1 周 (含 team-lead 集成)

---

### MA-6: wait_agent 轮询 vs 长等安全 🟡 P1

**来源**: `supervisorPrompt.ts:236-247` (must-do addon)

**问题**: 一次 wait_agent(900000) = 15 分钟阻塞, codex 进程死了你也不知道

**SpectrAI 修法**:
```
loop {
    wait_agent_idle(60-90s)
    get_agent_output
    get_agent_status
    if running: continue another round
    if done: break
}
keep wait_agent / wait_agent_idle timeout <= 90000ms unless explicitly required
```

**Apeireth 现状**:
- `apeireth-team-lead` 14 工具白名单里有 wait_agent_idle (跟 SpectrAI 同名), 但**实现**我没看到
- `apeireth-runtime` 没有等价 API

**借鉴价值**: 🟡 **中等**。是个 prompt 级别的最佳实践, 不是核心架构借鉴。

**建议落地**:
- 给 `apeireth-team-lead::Orchestrator` 的 `wait_agent_idle` 加 max 90s 内部约束
- supervisor_prompt.md 加对应 "loop polling" 段
- 估时: 3 天

---

### MA-7: Per-task-type Provider 路由 + fallback chain 🔴 P0

**来源**: `supervisorPrompt.ts:168-185` (表格)

**核心启发式**:

| 任务类型 | 推荐 Provider | 原因 |
|---|---|---|
| 复杂架构设计、多文件重构 | claude-code | 综合推理能力最强 |
| 写代码、修 bug、加功能 | codex | 代码生成专长 |
| 大文件分析、代码审查 | gemini-cli | 上下文窗口大 |
| 文档总结、知识梳理 | gemini-cli | 擅长长文本理解 |
| 代码生成和补全、多模型切换 | opencode | 支持多模型切换 |
| 并行多个分析任务 | 混合使用 | 多样化视角 |

**Fallback 顺序** (失败自动重试):
```
claude-code → gemini-cli → codex → opencode
```

**触发条件**: AgentResult.failedProvider 字段标识 + 错误信息含"额度不足/认证失败"

**Apeireth 现状**:
- `apeireth-multi-model-backend` (R269): 3 聚合策略 (FirstNonEmpty / Longest / ConcatAll)
- 已有 `LlmProvider` trait + 5 provider (claude/codex/gemini/iFlow/openCode via `apeireth-provider`)
- **但**: 没有 per-task-type 路由, 没有 fallback chain 自动化
- 缺一个类似 `TaskProfile → ProviderRoute` 的映射表

**借鉴价值**: 🔴 **极高**。

**建议落地**:
- 改 `apeireth-multi-model-backend` 加 `ProviderRouter`:
  ```rust
  pub struct ProviderRouter {
      backends: HashMap<ProviderId, Arc<dyn LlmProvider>>,
      routes: HashMap<TaskProfile, Vec<ProviderId>>,  // fallback chain
      current: AtomicUsize,  // round-robin counter
  }
  
  pub enum TaskProfile {
      ArchitectureDesign,
      CodeGeneration,
      LargeFileAnalysis,
      DocSummarization,
      MultiModelSwitching,
      ParallelAnalysis,
  }
  
  impl ProviderRouter {
      pub async fn route(&self, profile: TaskProfile, req: LlmRequest) -> LlmResult;
      // 自动 fallback: 第一个失败 → 第二个 → ... → 最后一个失败 → 返回 error
  }
  ```
- 跟现有 `AggregationStrategy` 组合: `route(profile, strategy)`
- 估时: 3-5 天

---

### MA-8: AgentBridge WebSocket 桥 — 已砍 ❌

**来源**: `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\AgentBridge.ts` (130 LOC) + `AgentMCPServer.ts` (700 LOC)

**模式**: WebSocketServer on 127.0.0.1:63721, MCP server (per session 子进程) 注册 + 双向 RPC

**Apeireth 现状**:
- 主人 2026-08-05 决策 2: **砍 WebSocket 桥, 改 in-process**
- 已落 `apeireth-bus` L4 (3-channel) 替代

**借鉴价值**: ❌ **不借鉴**。已砍, 不重复。

---

### MA-9: Progress reporting 强制 1 阶段 1 次 🟢 P2

**来源**: `supervisorPrompt.ts:228-235` (must-do addon)

**规则**:
```
- 长跑任务主动 progress report
- 至少每个 major stage 报 1 次 (analysis / implementation / validation)
- 被 block 时清楚报 blocker + 下一步
- 每条 update 1-2 句精炼
```

**Apeireth 现状**:
- `apeireth-runtime::CycleReport` 每 tick 自动 emit
- `apeireth-supervisor::otel_metrics` 256 个 metric 点位
- 已经有 telemetry 覆盖, **不缺 progress reporting**

**借鉴价值**: 🟢 **低**。已有 telemetry 比 progress reporting 更结构化。

---

### MA-10: parseInteractiveQuestion — 跟现有冲突 ❌

**来源**: `SessionManagerV2.ts:30-130`

**算法**: 从 AI 输出解析问句 + Yes/No 或编号选项 (2-6 个), 用于 AskUserQuestion 工具

**Apeireth 现状**:
- `apeireth-companion::approval_requests` + `apeireth-team-lead::approval_bridge` 已经处理人机交互
- 双洋葱 + L0 HA 审批链覆盖

**借鉴价值**: ❌ **不借鉴**。跟现有 ApprovalBridge + 双洋葱重复。

---

## §3 借鉴决策矩阵 (项目自评视角)

| 借鉴 ID | 价值 | 成本 | 设计冲突 | **推荐** |
|---|---:|---:|---|:---:|
| **MA-1** TaskSessionCoordinator | 🔴 高 | 低 (2-3d) | 无 (blueprint 已规划) | ✅ **立即借** |
| **MA-2** AgentManagerV2 父子树 | 🟡 中 | 已完成 | 无 (team-lead 已翻译) | ⚪ skip (verify) |
| **MA-3** turn_complete 事件 | 🟡 中 | 中 (1w) | 跟 Heartbeat 并存 | ⏳ 可选 |
| **MA-4** ConcurrencyGuard | 🔴 高 | 低 (1d) | 无 (sysinfo crate 待用) | ✅ **立即借** |
| **MA-5** McpSessionMode | 🟡 中 | 中 (1w) | 跟 RoleConstitution 重叠 | ⏳ 需设计 |
| **MA-6** wait_agent 轮询 | 🟡 中 | 低 (3d) | 无 | ⏳ 加进 team-lead |
| **MA-7** Provider 路由 + fallback | 🔴 高 | 中 (3-5d) | 跟现有 Aggregation 互补 | ✅ **立即借** |
| MA-8 AgentBridge | — | — | 已砍 | ❌ skip |
| MA-9 Progress reporting | 低 | — | 已有 telemetry | ❌ skip |
| MA-10 parseInteractiveQuestion | — | — | 跟 ApprovalBridge 重 | ❌ skip |

**总工作量**: 立即借 3 项 (MA-1+MA-4+MA-7) 估 **6-9 天**, 可分 3 个 sprint。

---

## §4 落地建议 (拍板路径)

### Phase 1 (本周)
1. **MA-4 ConcurrencyGuard** → 1 天 → 加进 `apeireth-runtime`
2. **MA-1 TaskSessionCoordinator** → 2-3 天 → 补 `apeireth-council::task` (blueprint 已规划)

### Phase 2 (下周)
3. **MA-7 ProviderRouter** → 3-5 天 → 加进 `apeireth-multi-model-backend`

### Phase 3 (v1.1 候选)
4. **MA-6** + **MA-5** 设计判断 + 实施

### 不做
- MA-8 (已砍) / MA-9 (已有) / MA-10 (已有)

---

## §5 跨项目诚实标注

1. **MA-1 的 blueprint 规划未实装**: `crates\apeireth-council\src\task*` 不存在, 是 blueprint §3 标了但没做的洞 (2026-08-05 蓝图规划后还没人接手)
2. **MA-2 的 team-lead 翻译实施度未 verify**: 我看了 lib.rs + approval_bridge.rs + supervisor_prompt.md, 但 8 调度工具的 trait impl 完整度需要 cargo test 验证
3. **MA-7 的现有覆盖**: `apeireth-multi-model-backend` 已有 3 聚合策略 (R269), 加 ProviderRouter 是**补充不是替换**, 不能破坏 R269
4. **版本基线**: 本报告基于 `spectrai-source/src/main/agent/AgentManagerV2.ts` (v0.9.21 商业版从 decompile 还原的 963 LOC) + apeireth master `42a53f83` (2026-08-18)
5. **本报告 0 改任何源码**: 写在 `reports/` 不 commit, 等整合 #5 commit 时机

---

## §6 引用文件清单 (绝对路径)

### SpectrAI 源 (反编译)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\AgentManagerV2.ts` (963 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\supervisorPrompt.ts` (808 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\AgentMCPServer.ts` (700 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\AgentBridge.ts` (130 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\types.ts` (60 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\session\SessionManagerV2.ts` (1231 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\session\ConcurrencyGuard.ts` (188 LOC)
- `C:\Users\31683\.minimax-agent-cn\spectrai\spectrai-source\src\main\task\TaskSessionCoordinator.ts` (123 LOC)

### Apeireth 现状
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\lib.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\advisor.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\deliberation.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\hold.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\synthesis.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\sovereignty.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\multi_model_backend.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\group_chat.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\collaboration\types.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\constitution.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\graph_orchestration.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-council\src\delegation_matrix.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-runtime\src\lib.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-team-lead\src\lib.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-team-lead\src\approval_bridge.rs`
- `C:\Users\31683\Apeireth-rust\crates\apeireth-team-lead\src\md\supervisor_prompt.md` (1:1 翻译)

### 已有 blueprint
- `C:\Users\31683\Apeireth-rust\docs\archive\stage4\_history\spectrai\spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (63 KB)
- `C:\Users\31683\.minimax-agent-cn\spectrai\reports\spectrai-architecture-2026-08-05.md` (65 KB)
- `C:\Users\31683\.minimax-agent-cn\spectrai\reports\apeireth-council-7-advisor-analysis-2026-08-05.md` (40 KB)

---

## §7 关键引用 (代码行号定位)

| 借鉴 ID | 关键代码位置 |
|---|---|
| MA-1 | `TaskSessionCoordinator.ts:11-20` (SESSION_TO_TASK), `:21-25` (ACTIVITY_TO_TASK), `:64-90` (debounce 1s) |
| MA-2 | `AgentManagerV2.ts:29-50` (ManagedAgent struct), `:55-80` (5 个 Map), `:160-180` (waitAgentIdle 竞态修复) |
| MA-3 | `AgentManagerV2.ts:104-117` (turn_complete listener), `:160-180` (agentIdleFlags 处理) |
| MA-4 | `ConcurrencyGuard.ts:13-19` (config), `:23-55` (macOS vm_stat), `:103-122` (checkResources) |
| MA-5 | `agent/types.ts:4` (McpSessionMode type), `AgentManagerV2.ts:235-245` (注入 sessionMode 到 MCP config) |
| MA-6 | `supervisorPrompt.ts:236-247` (must-do addon) |
| MA-7 | `supervisorPrompt.ts:168-185` (provider 路由表), `:188-194` (fallback chain) |
| MA-8 | `AgentBridge.ts:36-89` (WSS startup), `AgentMCPServer.ts` (MCP server per session) |
| MA-9 | `supervisorPrompt.ts:228-235` (progress reporting) |
| MA-10 | `SessionManagerV2.ts:30-130` (parseInteractiveQuestion) |

---

*End of survey. 待主人/Mavis 拍板哪些借哪些不借.*
