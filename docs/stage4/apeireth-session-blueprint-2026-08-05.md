# apeireth-session crate 完整蓝图 (R19+ 阶段 1.3-1.4)

```
[Document-Meta]
Document: docs/stage4/apeireth-session-blueprint-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 阶段 1.3-1.4
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人 17→24 维 R-Measure 投影拍板 + leader 复核)
```

> **性质**: 纯文档交付。**不写代码、不改任何文件**。给后续 rust-coder 实施 `apeireth-session` 新 crate 用。
>
> **依据**:
> - `reports/apeireth-session-vector-asi-2026-08-05.md` §2 (确认 session crate 不存在)
> - `reports/apeireth-mcp-14-tool-analysis-2026-08-05.md` §3 (3 处修法具体到 trait 方法)
> - `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §4.4 (3 处组合根因 + 修法总表)
> - `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` (team-lead 通过 mcp::team 间接调 session)
> - `docs/stage4/r-measure-verification-design-2026-08-05.md` (R-Measure 17→24 维守门)
> - `spectrai/docs/ARCHITECTURE.md` §4 (mid-task bug 根因深挖)
>
> **不修改承诺**: 阶段 1/2/3/4/5 LOCKED + v2/v4/v4.1 LOCKED + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 全部保留 (见 §11)。
>
> **范围**: 本文档**只**覆盖 `apeireth-session` crate 设计,**不**碰 apeireth-vector / apeireth-formal / apeireth-asi (那 3 crate 见 `apeireth-session-vector-asi-2026-08-05.md`)。

---

## §1 战略背景 (为什么)

### 1.1 关键事实

| 事实 | 来源 | 含义 |
|------|------|------|
| **apeireth-session crate 不存在** | `reports/apeireth-session-vector-asi-2026-08-05.md` §1.1 第 1 行 + §2.1 (`ls: cannot access`) | R19+ 阶段 3 待新建 |
| **mid-task bug 3 处修法必须改这个不存在的 crate** | `docs/ARCHITECTURE.md` §4 + `spectrAI-integration-blueprint §4.4` 修法 1 明确写"`SessionManagerV2.sendMessage` → `apeireth-session/src/manager.rs::send_message`" | P0 阻塞,3 处一起改否则撕裂状态 |
| **team-lead 通过 `apeireth-mcp::team::send_to_agent` 间接调 session** | `apeireth-mcp-14-tool-analysis §3.1` 修法 #1 落点 + `apeireth-team-lead-implementation-guide §3` 架构图 | session 永远不直接被 team-lead 调,只通过 mcp::team |
| **估时 1500-2000 LOC, 3-4 天实施** | `apeireth-session-vector-asi §2.6` 实施步骤 7 步估 1530 LOC | 跟本蓝图 §10 8 阶段 5 天一致 |

### 1.2 不修这个 crate 的代价 (诚实登记)

按主 S-2 17:43 实事求是:

| 不修的后果 | 触发场景 | 撕裂状态 |
|----------|---------|---------|
| sendMessage 在终态 throw | 父进程 wait_idle 后 send 消息到已死子 session | 子 agent 显示 running, 父进程以为发失败 |
| sendToAgent 永远 success:true | line 281 `.catch()` 吞 + line 285 无条件返 true | 父进程以为成功, 实际消息没发出去 |
| child session 状态窗口期 | 子进程崩溃到 onChildSessionEnded 调用之间 | 父进程卡死 5min 等 idle, 实际已死 |

3 处是**组合根因**,任一不改 = 撕裂状态复发。P0 必一起改。

### 1.3 战略原则 (硬约束)

| 原则 | 来源 | 落地 |
|------|------|------|
| **TS 翻译, 不 patch fork** | user memory #8 + spectrAI-integration-blueprint §1.3 | TS 源码当设计参考, Rust idiom 重写 |
| **不重复造轮子** | user memory #6 | apeireth-protocol/storage/tool-registry 已有全部复用 |
| **不依赖 apeireth-supervisor** | ADR-0011 §决策 4 | 避免循环依赖 |
| **不假装已实现** | O-5 + APEIRETH-CONVENTIONS §2 | 编译期 hardcode / Result 错误处理 / no panic |
| **6 主哲学锚穿透** | APEIRETH-CONVENTIONS §9 | 见 §12 |
| **7 LOCKED 不动** | APEIRETH-CONVENTIONS §10 | 见 §11 8 项不修改承诺 |

### 1.4 比喻

> SpectrAI `SessionManagerV2.ts` = TS 端的"长会话调度器",跑了 19 个 Electron app 验证了 Supervisor-Member 团队协作,但 3 处 bug 撕裂状态。
>
> `apeireth-session` (本蓝图) = Rust 端的"长会话调度器",**复用 3 处修法** + Rust 强类型 + tokio async + 事件总线,**不重复** TS 端的"throw + catch + 状态窗口期"老路。
>
> 集成 = 把 Rust 调度器**装进** Apeireth 41 crate,让 team-lead 通过 mcp::team 14 工具间接用, mid-task bug 在 Rust 端**根治**。

---

## §2 战略定位 (在哪)

### 2.1 41 crate 中的位置 (R19+ 阶段 3)

```
                        ┌─────────────────────────────────────┐
                        │  apeireth-team-lead (新, 850 LOC)    │
                        │  构造 supervisor prompt + voting     │
                        └─────────────┬───────────────────────┘
                                      │ 调 apeireth-mcp::team 14 工具
                                      ▼
┌──────────────────────────────────────────────────────────┐
│  apeireth-mcp::team (新模块, 14 工具)                     │
│  8 supervisor + 3 worktree + 3 认知                       │
│  ─ send_to_agent / get_output / wait_idle 走这里         │
└─────────────┬──────────────────┬─────────────────────────┘
              │                  │
              ▼                  ▼
┌────────────────────────┐  ┌────────────────────────────────┐
│  apeireth-agent        │  │  apeireth-session (本蓝图)       │
│  (1358 LOC, 已有)      │──│  1500-2000 LOC, R19+ 新建       │
│  send_to_agent sender  │  │  SessionManager + 6 状态机      │
│  mid-task bug 修法 #2  │  │  + mid-task bug 3 处一起改       │
└────────────────────────┘  └────┬────────────────────┬──────┘
                                 │                    │
                                 ▼                    ▼
                  ┌──────────────────────┐  ┌────────────────────┐
                  │  apeireth-storage    │  │  apeireth-protocol │
                  │  (新, 1300 LOC)      │  │  (已有)            │
                  │  session 持久化      │  │  AdapterSessionCfg │
                  └──────────────────────┘  └────────────────────┘
```

### 2.2 上下游依赖

| 方向 | Crate | 关系 |
|------|-------|------|
| **上游 (被调)** | `apeireth-team-lead` (新) | 调 `apeireth-mcp::team::send_to_agent` → 间接调 session |
| **上游 (被调)** | `apeireth-mcp::team` (新模块) | 14 工具直接调 session 内部 API |
| **横向 (不调)** | `apeireth-council` (2740 LOC, 已有) | 7 advisor 审议, 不直接调 session |
| **横向 (不调)** | `apeireth-agent` (1358 LOC, 已有) | 单 agent 行为, send_to_agent 时调 session (修法 #2 落点) |
| **下游 (调)** | `apeireth-storage` (新) | session 持久化 (WAL) |
| **下游 (调)** | `apeireth-protocol` (已有) | `AdapterSessionConfig` / `Message` / event |
| **依赖** | `tokio` (async) + `serde` + `thiserror` + `tracing` | workspace 已有 |

### 2.3 4 分类严格分离 (按 ADR-0011 §决策 4)

| crate | 层级 | 职责 | 类比 |
|-------|------|------|------|
| **`apeireth-session`** (本蓝图) | **Component** | **session 生命周期 + 状态机 + mid-task bug 根治** | **数据库连接池** |
| `apeireth-team-lead` (新) | Agent | supervisor prompt + voting 触发 | PM |
| `apeireth-council` (已有) | Application | 7 advisor voting | 陪审团 |
| `apeireth-supervisor` (已有) | OS | PID 1 进程监督 | init |
| `apeireth-agent` (已有) | Component | 单 agent 行为 | 员工 |

**绝不含糊**: session 是"长会话资源池",supervisor 是"OS 进程",council 是"安全审查",agent 是"单数 worker"。

---

## §3 crate 骨架设计

### 3.1 目录结构

```
crates/apeireth-session/  (新 crate, 等 code_reviewer 完工)
├── Cargo.toml            (估 100 B, 关键依赖见 §3.2)
├── README.md             (R19+ 必加, 估 80 LOC, 跟 team-lead 一致)
├── src/
│   ├── lib.rs            (主入口, 估 100 LOC, 见 §3.3)
│   ├── manager.rs        (SessionManager V2, 估 400 LOC, 见 §4.1 修法 1)
│   ├── session.rs        (Session struct, 估 200 LOC, 见 §3.3)
│   ├── state.rs          (SessionState enum, 估 100 LOC, 见 §3.3)
│   ├── mid_task.rs       (MidTaskState + AgentHandle, 估 300 LOC, 见 §4.2+§4.3 修法 2+3)
│   ├── concurrency.rs    (ConcurrencyGuard, 估 200 LOC, 限 9 session)
│   ├── storage.rs        (DB 持久化, 估 150 LOC, WAL)
│   └── error.rs          (SessionError, 估 50 LOC, thiserror)
├── tests/
│   ├── manager_tests.rs       (估 300 LOC, 10 tests)
│   ├── mid_task_tests.rs      (估 300 LOC, 8 tests, 含 4 成功 + 4 失败)
│   └── concurrency_tests.rs   (估 200 LOC, 8 tests)
└── examples/
    └── session_demo.rs   (估 50 LOC, 演示 build_supervisor prompt + 1 mid-task 场景)
```

**LOC 累计**:
- src/: 100 + 400 + 200 + 100 + 300 + 200 + 150 + 50 = **1500 LOC** (在 1500-2000 区间内)
- tests/: 300 + 300 + 200 = **800 LOC**
- examples/: 50 LOC
- **总计 ~2350 LOC** (src + tests + examples)

> **不假装 (S-2 17:43)**: src/ 部分 1500 LOC 正好压在蓝图估 1500-2000 区间下沿。如果 rust-coder 实施时需要更多,优先在 `mid_task.rs` 加 (300 → 500),不破坏区间上沿。

### 3.2 Cargo.toml 关键依赖

```toml
[package]
name = "apeireth-session"
version = "0.1.0"           # R19+ 阶段 1.3 起步
edition = "2024"
description = "Apeireth 长会话调度子系统 (SessionManager V2 + 6 状态机 + mid-task bug 3 处根治) - R19+ 阶段 3 翻译自 SpectrAI SessionManagerV2.ts"
license = "Apache-2.0 OR MIT"
repository = "https://github.com/apeireth/apeireth"
keywords = ["apeireth", "session", "lifecycle", "mid-task", "spectrAI"]
categories = ["asynchronous", "api-bindings"]

[dependencies]
# 核心依赖 (按 ADR-0011 §决策 4, 不依赖 apeireth-supervisor)
apeireth-protocol      = { path = "../apeireth-protocol" }      # AdapterSessionConfig, Message, event
apeireth-storage       = { path = "../apeireth-storage" }        # WAL 持久化 (R19+ 阶段 1.4 同期)
apeireth-tool-registry = { path = "../apeireth-tool-registry" } # Tool trait (订阅 ToolCall 事件)
tokio                  = { version = "1", features = ["full"] } # async runtime
serde                  = { version = "1", features = ["derive"] }
serde_json             = "1"
thiserror              = "1"
tracing                = "0.1"
async-trait            = "0.1"
uuid                   = { version = "1", features = ["v4", "serde"] }
parking_lot            = "0.12"   # 高性能 Mutex (替代 std::sync::Mutex)
dashmap                = "6"      # 并发 HashMap (sessions 表)

[dev-dependencies]
tokio-test         = "0.4"
pretty_assertions  = "1"
tempfile           = "3"     # 临时 DB 文件
mockall            = "0.13"  # mock Storage trait

[lints]
workspace = true    # 继承 workspace.lints (R19 新加, 等 code_reviewer 改完 Cargo.toml)
```

**关键字段说明**:

| 字段 | 为什么 | 风险 |
|------|-------|------|
| `version = "0.1.0"` | R19+ 阶段 1.3 起步, 跟 workspace 0.14.0 增量 | semver 严格 |
| `apeireth-protocol` | `AdapterSessionConfig` / `Message` / event | 不依赖 = 编译失败 |
| `apeireth-storage` | WAL 持久化 (R19+ 阶段 1.4 同期) | ⚠️ 实施时序依赖 |
| **`不依赖` apeireth-supervisor** | ADR-0011 §决策 4 强约束 | ❌ 严禁加 |
| **`不依赖` apeireth-council** | 横向, 不直接调 | ❌ 严禁加 |
| **`不依赖` apeireth-team-lead** | 反向依赖, 防止循环 | ❌ 严禁加 |
| `parking_lot` + `dashmap` | 高性能并发, 避免 std::sync::Mutex 阻塞 | ⚠️ 编译期 hardcode 守门 |

### 3.3 lib.rs 公开 API

```rust
// ============================================================================
// apeireth-session/src/lib.rs 公开 API 设计 (伪代码, 仅展示 API 形态)
// ============================================================================

// ----------------------------------------------------------------------------
// 主管理器
// ----------------------------------------------------------------------------

/// SessionManager V2 — 长会话调度器 (翻译自 SpectrAI SessionManagerV2.ts)
/// 6 状态机 + 事件总线 + 中 task 3 处根治
pub struct SessionManager {
    sessions: DashMap<SessionId, ManagedSession>,
    status_watch: HashMap<SessionId, tokio::sync::watch::Sender<SessionState>>,
    event_bus: tokio::sync::broadcast::Sender<SessionEvent>,
    concurrency_guard: ConcurrencyGuard,
    storage: Arc<dyn SessionStorage>,
}

impl SessionManager {
    /// 构造 (从 storage 恢复已有 session)
    pub async fn new(storage: Arc<dyn SessionStorage>, max_sessions: usize) -> Result<Self, SessionError>;

    /// 创建新 session (R19 阶段 1.3 主要入口)
    pub async fn create(&self, config: AdapterSessionConfig) -> Result<SessionId, SessionError>;

    /// 销毁 session (状态机保证, 不会强杀活跃 session)
    pub async fn destroy(&self, id: &SessionId) -> Result<(), SessionError>;

    /// 列出所有活跃 session
    pub async fn list_active(&self) -> Vec<SessionInfo>;

    /// 订阅 session 状态变化 (事件驱动, 修法 #3 关键)
    pub fn subscribe(&self, id: &SessionId) -> broadcast::Receiver<SessionEvent>;

    /// 订阅全局 session 事件
    pub fn subscribe_all(&self) -> broadcast::Receiver<SessionEvent>;
}

// ----------------------------------------------------------------------------
// 核心类型
// ----------------------------------------------------------------------------

/// SessionId — newtype, 强类型防混
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

/// Session (内部 struct, 不对外暴露字段)
pub struct Session {
    id: SessionId,
    state: SessionState,
    config: AdapterSessionConfig,
    messages: VecDeque<Message>,
    mid_task_state: Option<MidTaskState>,
    mid_task_seq: u64,                // 单调递增, 用于关联 mid-task 序列号
    created_at: Instant,
    last_activity: Instant,
    status_watch: tokio::sync::watch::Sender<SessionState>,
}

impl Session {
    pub fn id(&self) -> &SessionId;
    pub fn state(&self) -> SessionState;
    pub fn config(&self) -> &AdapterSessionConfig;
    pub fn messages(&self) -> &VecDeque<Message>;
    pub fn mid_task_state(&self) -> Option<&MidTaskState>;

    /// 状态转换 (内部 API, 编译期保证原子性, 修法 #3 关键)
    fn transition_to_mid_task(&mut self, cause: MessageRef) -> Result<u64, SessionError>;
    fn transition_to_idle(&mut self) -> Result<(), SessionError>;
    fn transition_to_terminal(&mut self, terminal: TerminalState) -> Result<(), SessionError>;
}

// ----------------------------------------------------------------------------
// 状态机 (6 状态 + mid-task 子状态)
// ----------------------------------------------------------------------------

/// 主状态机 (翻译自 SessionManagerV2.ts 的 6 状态)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// 初始, 配置已加载, 等消息
    Idle,
    /// 处理中 (LLM 调工具/生成中)
    Running,
    /// 等用户输入 (跟 apeireth-tui 配合)
    WaitingInput,
    /// 内部 mid-task 处理 (修法 #1+#2+#3 关键)
    MidTask,
    /// 终态 - 正常完成
    Completed,
    /// 终态 - 失败
    Failed { reason: String },
    /// 终态 - 用户取消
    Cancelled,
    /// 终态 - 异常终止 (子进程死等)
    Terminated { reason: String },
}

impl SessionState {
    /// 终态判定 (供 send_message 检查)
    pub fn is_terminal(&self) -> bool;
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

/// mid-task 子状态 (修法 #2+#3 关键)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidTaskState {
    /// mid-task 槽位空
    Idle,
    /// mid-task 进行中 (有 sub-task 在跑)
    Active { started_at: Instant, cause: MessageRef, seq: u64 },
    /// mid-task 被中断 (子 task 出错, 但 session 没死)
    Interrupted { reason: String, at: Instant, seq: u64 },
    /// mid-task 合并到下轮 (Idle 状态收到 mid-task message)
    Merged { into_session: SessionId, at: Instant, seq: u64 },
    /// mid-task 失败
    Failed { error: String, at: Instant, seq: u64 },
}

/// 终止终态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalState {
    Completed,
    Failed,
    Cancelled,
    Terminated,
}

// ----------------------------------------------------------------------------
// 修法 1 核心: send_message (替代 throw 改 return)
// ----------------------------------------------------------------------------

/// send_message 结果 (mid-task 状态时, 消息入队不抛错)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendResult {
    /// 正常发出
    Sent,
    /// mid-task 状态, 消息已入 mid_task_queue
    MidTask { queued: bool, seq: u64 },
    /// 终态消息优雅失败 (替代 throw, 修法 #1 关键)
    Failed { reason: String, current_state: SessionState },
}

impl SessionManager {
    /// 发送消息到 session (mid-task bug 修法 #1 核心)
    /// 永不 panic, 永不 throw, 全部 Result 返回
    pub async fn send_message(
        &self,
        id: &SessionId,
        message: Message,
    ) -> Result<SendResult, SessionError>;

    /// 读 session 输出 (mid-task bug 修法 #2 核心)
    /// include_mid_task=false 时跳过 mid-task 引起的 chunk
    pub async fn get_output(
        &self,
        id: &SessionId,
        since_seq: Option<u64>,
        include_mid_task: bool,
    ) -> Result<OutputChunk, SessionError>;

    /// 等 session 变 idle (mid-task bug 修法 #3 核心)
    /// mid_task_state 为 Interrupted/Merged 时不算 idle
    pub async fn wait_idle(
        &self,
        id: &SessionId,
        timeout: Option<Duration>,
    ) -> Result<IdleSignal, SessionError>;

    /// 等 session 退出 (终态)
    pub async fn wait_exit(
        &self,
        id: &SessionId,
        timeout: Option<Duration>,
    ) -> Result<TerminalState, SessionError>;

    /// 取消 session (只在非终态生效)
    pub async fn cancel(&self, id: &SessionId, reason: String) -> Result<(), SessionError>;
}

// ----------------------------------------------------------------------------
// 修法 2 核心: AgentHandle (send_to_agent 时用)
// ----------------------------------------------------------------------------

/// AgentHandle — agent 在 session 中的句柄 (修法 #2 关键)
pub struct AgentHandle {
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub mid_task_state: MidTaskState,
    pub input_seq: u64,                  // 单调递增 input 序列号
    pub output_buffer: VecDeque<OutputChunk>,
    pub state: AgentState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Running,
    Idle,
    Exited(i32),                         // exit code
    Cancelled,
}

/// 修法 #2 落点 — send_to_agent 真实返回
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendToAgentResult {
    Ok { delivered: bool, seq: u64 },
    MidTask { queued: bool, seq: u64, mid_task_state: MidTaskState },
    Failed { reason: String, current_session_state: SessionState },
    Error { error: String },
}

// ----------------------------------------------------------------------------
// 修法 3 核心: 事件总线 + 状态转换原子性
// ----------------------------------------------------------------------------

/// Session 事件 (广播, 修法 #3 关键)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    /// 状态变化 (任何变化都广播, 修法 #3 关键)
    StateChange { id: SessionId, from: SessionState, to: SessionState, at: Instant },
    /// mid-task 状态变化
    MidTaskChange { id: SessionId, mid_task: MidTaskState, caused_by_seq: u64, at: Instant },
    /// 输出 chunk 追加
    OutputChunk { id: SessionId, chunk: OutputChunk },
    /// 终态到达
    Terminal { id: SessionId, terminal: TerminalState, final_output: Option<String> },
}

/// 消息引用 (用于 mid_task_state.cause)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRef {
    pub seq: u64,
    pub kind: MessageKind,
    pub preview: String,           // 前 80 char, 供 LLM 知道 mid-task 由什么触发
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    UserInput,
    ToolResult,
    LlmResponse,
    SystemEvent,
}

/// 输出 chunk (修法 #2 关键 — caused_by_seq + is_mid_task_response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputChunk {
    pub seq: u64,
    pub content: String,
    pub at: Instant,
    pub caused_by_seq: u64,             // 哪条 input 引起的
    pub is_mid_task_response: bool,     // 是否 mid-task 引起 (修法 #2 关键)
}

/// idle signal (修法 #3 关键 — pending_mid_task 字段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleSignal {
    pub idle: bool,
    pub idle_since: Instant,
    pub pending_mid_task: bool,         // 修法 #3: 即使 idle, mid_task 还在跑也不算
}

// ----------------------------------------------------------------------------
// 并发限制 (ConcurrencyGuard)
// ----------------------------------------------------------------------------

/// 并发限制 (翻译自 ConcurrencyGuard.ts, 限 9 session)
pub trait ConcurrencyGuard: Send + Sync {
    fn try_acquire(&self) -> Result<SessionPermit, SessionError>;
    fn release(&self, permit: SessionPermit);
    fn active_count(&self) -> usize;
    fn max_count(&self) -> usize;
}

pub struct SessionPermit { /* RAII guard, drop 时自动 release */ }

/// 默认实现: semaphore + 内存/CPU 检查 (sysinfo)
pub struct DefaultConcurrencyGuard {
    semaphore: tokio::sync::Semaphore,
    max_sessions: usize,
    memory_check: bool,             // macOS 内存用 sysinfo
    cpu_check: bool,
}

// ----------------------------------------------------------------------------
// 持久化 (Storage trait)
// ----------------------------------------------------------------------------

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: &Session) -> Result<(), SessionError>;
    async fn load(&self, id: &SessionId) -> Result<Option<Session>, SessionError>;
    async fn delete(&self, id: &SessionId) -> Result<(), SessionError>;
    async fn list_active(&self) -> Result<Vec<SessionId>, SessionError>;
    async fn log_mid_task(&self, id: &SessionId, mid_task: &MidTaskState) -> Result<(), SessionError>;
}

/// SQLite WAL 实现
pub struct SqliteSessionStorage { /* rusqlite + WAL */ }

// ----------------------------------------------------------------------------
// 错误类型 (thiserror)
// ----------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(SessionId),
    #[error("session in terminal state: {state:?}, cannot accept messages")]
    Terminal { state: SessionState },
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: SessionState, to: SessionState },
    #[error("concurrency limit reached: {active}/{max}")]
    ConcurrencyLimit { active: usize, max: usize },
    #[error("storage error: {0}")]
    Storage(String),
    #[error("mid-task queue full: {active}/{max}")]
    MidTaskQueueFull { active: usize, max: usize },
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

**API 风格守门** (按 APEIRETH-CONVENTIONS):
- ✅ 命名 snake_case (Rust 约定)
- ✅ async/await 标配 (tokio 生态)
- ✅ `pub fn` + `pub struct` 显式声明 (不 wildcard re-export)
- ✅ 编译期 hardcode: `enum SessionState` 8 状态 / `enum MidTaskState` 5 状态 / `enum SendResult` 3 变体
- ✅ `thiserror` 错误类型 (不 panic, 不 unwrap)
- ✅ `Result<T, SessionError>` 永不 throw (修法 #1)
- ❌ 不用 `unwrap()` (单测除外)
- ❌ 不用 `lazy_static` / `once_cell` (用 `const` / `tokio::sync::OnceCell`)

---

## §4 mid-task bug 3 处修法 (核心交付)

> **P0 必一起改**: 3 处是组合根因,任一不改 = 撕裂状态复发。来源: `docs/ARCHITECTURE.md` §4 + `spectrAI-integration-blueprint §4.4`。

### 4.1 修法 1: send_message 状态机 (替代 throw 改 return)

**SpectrAI 根因** (`session/SessionManagerV2.ts:636-643`):

```typescript
// ❌ TS 终态 throw
if (session.status === 'error' || session.status === 'completed' || session.status === 'terminated') {
  throw new Error(`Session ${id} is in ${session.status} state and cannot accept messages`)
  // line 642 ⚠️ — throw 跟其他分支的 soft fail 风格不一致
}
```

**问题**:
- throw 风格不可恢复,调用方 `.catch()` 吞 + 误判为实现错误
- 实际是"会话语义拒绝" (子进程死了, 不能写), 应该 `Result::Err` 而非 panic
- mid-task 状态时, 消息应该入队不抛错

**Rust 修法** (`apeireth-session/src/manager.rs`):

```rust
// ============================================================================
// 修法 1: send_message 状态机 (替代 throw 改 return)
// ============================================================================

impl SessionManager {
    pub async fn send_message(
        &self,
        id: &SessionId,
        message: Message,
    ) -> Result<SendResult, SessionError> {
        // 1. 拿 session (读锁升级写锁, 锁内做状态检查)
        let mut session = self.sessions
            .get_mut(id)
            .ok_or_else(|| SessionError::NotFound(id.clone()))?;

        // 2. 状态机分支 (修法 #1 核心)
        match session.state {
            // ❌ 旧: throw new Error(...)
            // ✅ 新: 返回 SendResult::Failed
            SessionState::Completed => {
                return Ok(SendResult::Failed {
                    reason: "session already completed".into(),
                    current_state: SessionState::Completed,
                });
            }
            SessionState::Failed { .. } => {
                return Ok(SendResult::Failed {
                    reason: "session already failed".into(),
                    current_state: session.state.clone(),
                });
            }
            SessionState::Cancelled => {
                return Ok(SendResult::Failed {
                    reason: "session cancelled".into(),
                    current_state: SessionState::Cancelled,
                });
            }
            SessionState::Terminated { .. } => {
                return Ok(SendResult::Failed {
                    reason: "session terminated".into(),
                    current_state: session.state.clone(),
                });
            }

            // mid-task 状态: 写入 mid_task queue, 不抛错
            SessionState::MidTask => {
                let mid_task_seq = session.mid_task_seq;
                let item = MidTaskQueueItem {
                    message: message.clone(),
                    queued_at: Instant::now(),
                    seq: mid_task_seq + 1,
                };

                // 检查 mid_task_queue 是否已满
                if session.mid_task_queue.len() >= session.config.mid_task_queue_max {
                    return Err(SessionError::MidTaskQueueFull {
                        active: session.mid_task_queue.len(),
                        max: session.config.mid_task_queue_max,
                    });
                }

                session.mid_task_queue.push_back(item);
                session.mid_task_seq += 1;

                // 触发事件 (修法 #3 关键 — 任何状态变化都广播)
                self.event_bus.send(SessionEvent::MidTaskChange {
                    id: id.clone(),
                    mid_task: MidTaskState::Active {
                        started_at: Instant::now(),
                        cause: MessageRef::from_message(&message, mid_task_seq + 1),
                        seq: mid_task_seq + 1,
                    },
                    caused_by_seq: mid_task_seq + 1,
                    at: Instant::now(),
                })?;

                return Ok(SendResult::MidTask {
                    queued: true,
                    seq: mid_task_seq + 1,
                });
            }

            // 正常路径: Idle / Running / WaitingInput
            _ => {
                // 写消息
                session.messages.push_back(message.clone());
                session.last_activity = Instant::now();

                // 状态转换: Idle → Running (如果有 LLM 调起)
                if matches!(session.state, SessionState::Idle) {
                    Self::transition_state(
                        &mut session,
                        SessionState::Running,
                        id,
                        &self.event_bus,
                    )?;
                }

                // 触发事件
                self.event_bus.send(SessionEvent::OutputChunk {
                    id: id.clone(),
                    chunk: OutputChunk {
                        seq: session.messages.len() as u64,
                        content: message.to_string(),
                        at: Instant::now(),
                        caused_by_seq: session.messages.len() as u64,
                        is_mid_task_response: false,
                    },
                })?;

                Ok(SendResult::Sent)
            }
        }
    }

    /// 内部状态转换辅助 (原子性保证, 修法 #3 关键)
    fn transition_state(
        session: &mut Session,
        to: SessionState,
        id: &SessionId,
        event_bus: &tokio::sync::broadcast::Sender<SessionEvent>,
    ) -> Result<(), SessionError> {
        let from = session.state;
        if !is_valid_transition(from, to) {
            return Err(SessionError::InvalidStateTransition { from, to });
        }
        session.state = to;
        session.status_watch.send(to).ok();   // 修法 #3: watch 跟踪
        event_bus.send(SessionEvent::StateChange {
            id: id.clone(),
            from,
            to,
            at: Instant::now(),
        })?;
        Ok(())
    }
}

/// 状态转换合法表 (编译期 hardcode, 守门)
fn is_valid_transition(from: SessionState, to: SessionState) -> bool {
    use SessionState::*;
    matches!((from, to),
        (Idle, Running) |
        (Running, Idle) |
        (Running, WaitingInput) |
        (WaitingInput, Running) |
        (Running, MidTask) |              // mid-task 状态可从 Running 进入
        (MidTask, Running) |              // mid-task 完回 Running
        (MidTask, Idle) |                 // mid-task 完直接 Idle
        (Idle, MidTask) |                 // Idle 状态也能进入 mid-task (e.g. queue merge)
        (Running, Completed) |            // 终态
        (Running, Failed { .. }) |
        (Running, Cancelled) |
        (Running, Terminated { .. }) |
        (Idle, Cancelled) |
        (WaitingInput, Cancelled)
    )
    // ❌ 终态 → 任何: 全部非法 (除 Cancelled 是 mid-task 间的过渡)
}
```

**SendResult 完整定义** (本蓝图 §3.3 已有, 这里再清晰一次):

```rust
/// send_message 返回类型 (修法 #1 核心)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendResult {
    /// 正常发出
    Sent,
    /// mid-task 状态, 消息已入 mid_task_queue
    MidTask { queued: bool, seq: u64 },
    /// 终态消息优雅失败 (替代 throw, 修法 #1 关键)
    Failed { reason: String, current_state: SessionState },
}
```

**为什么这样改 (主 S-2 17:43 实验室)**:
1. **永不 panic**: 会话是长期资源, panic 风格 API 不可恢复
2. **编译期强制处理**: `Result<SendResult, SessionError>` 让调用方必须 `?` 或 `match`,无法吞错
3. **mid-task 状态显式**: 不再是 throw, 而是把消息入队,返回 `MidTask { queued: true, seq }` 让调用方知道
4. **终态显式分类**: 4 个终态 (Completed / Failed / Cancelled / Terminated) 分别返 `Failed { reason, current_state }`, 调用方能区分为什么失败

### 4.2 修法 2: send_to_agent 真实返回 (替代 .catch() 吞 + 永远 success:true)

**SpectrAI 根因** (`agent/AgentManagerV2.ts:269-286`):

```typescript
// ❌ TS sendToAgent 不检查终态 + 永远 success
sendToAgent(agentId: string, message: string) {
  if (agent.info.status === 'completed' || ...) return { success: false, error: ... }  // line 273-275
  this.agentIdleFlags.delete(agentId)              // line 278 — send 失败也清
  this.sessionManager.sendMessage(agent.childSessionId, message).catch(err => {  // line 281
    console.error(...)                            // 吞
  })
  return { success: true }                        // line 285 — 永远 true!
}
```

**问题**:
- line 281 `.catch()` 吞掉错误
- line 285 无条件返 `success: true`
- 父进程完全感知不到失败
- line 278 清 idle flag 即使 send 失败, 后续 `wait_agent_idle` 误判

**Rust 修法** (在 `apeireth-agent/src/manager.rs`, 但需要 `apeireth-session` 支持):

```rust
// ============================================================================
// 修法 2: send_to_agent 真实返回 (替代 .catch() 吞 + 永远 success:true)
// 位置: apeireth-agent/src/manager.rs (sender 部分)
// 依赖: apeireth-session::SessionManager (本蓝图 §3.3)
// ============================================================================

impl AgentManager {
    /// send_to_agent — 真实返回 (修法 #2 核心)
    pub async fn send_to_agent(
        &self,
        agent_id: &AgentId,
        message: AgentMessage,
    ) -> SendToAgentResult {
        // 1. 拿 agent handle
        let handle = match self.agents.read().await.get(agent_id).cloned() {
            Some(h) => h,
            None => return SendToAgentResult::Error {
                error: format!("agent not found: {}", agent_id),
            },
        };

        // 2. 检查 agent 终态 (line 273-275 同等)
        if matches!(handle.state, AgentState::Exited(_) | AgentState::Cancelled) {
            return SendToAgentResult::Failed {
                reason: format!("agent already in terminal state: {:?}", handle.state),
                current_session_state: handle.session_state(),  // 修法 #2 加: 告诉调用方 session 真实状态
            };
        }

        // 3. 调 session.send_message (修法 #1 落点)
        //    ❌ 旧: .catch(err => console.error(...)) — 吞
        //    ✅ 新: await + match
        let send_result = match self.session_manager
            .send_message(&handle.session_id, message.clone().into())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return SendToAgentResult::Error {
                    error: e.to_string(),
                };
            }
        };

        // 4. 只在 send 成功时清 idle flag (修法 #2 关键)
        match &send_result {
            SendResult::Sent => {
                self.agent_idle_flags.write().await.remove(agent_id);
                SendToAgentResult::Ok { delivered: true, seq: handle.input_seq + 1 }
            }
            SendResult::MidTask { queued, seq } => {
                // mid-task 状态: 消息已入队, 算 delivered
                self.agent_idle_flags.write().await.remove(agent_id);
                SendToAgentResult::MidTask {
                    queued: *queued,
                    seq: *seq,
                    mid_task_state: handle.mid_task_state,
                }
            }
            SendResult::Failed { reason, current_state } => {
                // ❌ 旧: 永远 return success:true
                // ✅ 新: 真实返 Failed, 让父进程知道
                // ❌ 旧: send 失败也清 idle flag
                // ✅ 新: 失败不清, 保持原 idle flag, wait_idle 行为不变
                SendToAgentResult::Failed {
                    reason: reason.clone(),
                    current_session_state: *current_state,
                }
            }
        }
    }
}

/// send_to_agent 真实返回 (修法 #2 关键)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SendToAgentResult {
    /// 消息已发出
    Ok { delivered: bool, seq: u64 },
    /// mid-task 状态, 消息已入队
    MidTask { queued: bool, seq: u64, mid_task_state: MidTaskState },
    /// 真实失败 (agent 终态 / session 终态)
    Failed { reason: String, current_session_state: SessionState },
    /// 系统错误 (storage / 锁等)
    Error { error: String },
}
```

**为什么这样改 (主 O-5 17:58 不假装)**:
1. **不假装成功**: `SendToAgentResult::Failed` 真实告诉父进程"消息没发出去", 父进程能 retry 或 abort
2. **idle flag 只在成功清**: 避免"误以为新一轮, 实际没发"的撕裂状态
3. **mid-task 显式**: 不再是 throw, 而是 `MidTask { queued, seq, mid_task_state }`, 父进程能跟踪 mid-task 状态
4. **session 状态也带上**: `Failed { current_session_state }` 让父进程知道为什么失败 (是 session 死了还是 agent 死了)

### 4.3 修法 3: 事件驱动 + 状态原子性 (替代时序竞态窗口期)

**SpectrAI 根因** (`agent/AgentManagerV2.ts:458-461` + line 88):

```typescript
// ❌ TS 反向状态不同步
private onChildSessionEnded(agentId: string, status: string) {
  const exitCode = status === 'completed' ? 0 : (status === 'error' ? 1 : -1)
  this.completeAgent(agentId, exitCode)  // line 460 — child 死了, agent 才更新
}
// 反向不同步: agent.info.status === 'running' 时, child session 可能已经 'terminated'
// 中间窗口期: child 已死, agent 还显示 running
```

**问题**:
- 子 session 状态变化时, Agent 状态也更新 (通过 onSessionStatusChange 转发)
- 但**反向不同步**: Agent `running` 时, child session 异常崩溃, session 变 `terminated`, 但 `agent.info.status` 还在 `running` 直到 `onChildSessionEnded` 调用
- **中间窗口期** (child 已死, agent 还显示 running), 任何 sendToAgent 都会走 sendMessage + 触发修法 #1 的 throw

**Rust 修法** (在 `apeireth-session/src/mid_task.rs` + `apeireth-session/src/manager.rs` 事件总线):

```rust
// ============================================================================
// 修法 3: 事件驱动 + 状态原子性 (替代时序竞态窗口期)
// 位置: apeireth-session/src/mid_task.rs (MidTaskState) +
//       apeireth-session/src/manager.rs (事件总线)
// ============================================================================

/// mid-task 子状态 (5 状态, 修法 #2+#3 关键)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidTaskState {
    /// mid-task 槽位空
    Idle,
    /// mid-task 进行中
    Active { started_at: Instant, cause: MessageRef, seq: u64 },
    /// mid-task 被中断
    Interrupted { reason: String, at: Instant, seq: u64 },
    /// mid-task 合并到下轮
    Merged { into_session: SessionId, at: Instant, seq: u64 },
    /// mid-task 失败
    Failed { error: String, at: Instant, seq: u64 },
}

impl Session {
    /// CAS 原子状态转换 (修法 #3 核心 — 替代时序竞态窗口期)
    /// ❌ 旧: agent.info.status === 'running', child session.status === 'terminated' 窗口期
    /// ✅ 新: 任何状态变化都先抢锁, CAS 失败回滚
    pub fn transition_to_mid_task(
        &mut self,
        cause: MessageRef,
    ) -> Result<u64, SessionError> {
        // 1. CAS 当前状态 (内存原子操作, 无窗口期)
        let prev = std::mem::replace(&mut self.state, SessionState::MidTask);
        if !matches!(prev, SessionState::Running | SessionState::Idle | SessionState::MidTask) {
            // 回滚
            self.state = prev;
            return Err(SessionError::InvalidStateTransition {
                from: prev,
                to: SessionState::MidTask,
            });
        }

        // 2. 分配 mid_task 序列号 (单调递增)
        self.mid_task_seq += 1;
        let new_seq = self.mid_task_seq;

        // 3. 记录 mid_task 状态
        self.mid_task_state = Some(MidTaskState::Active {
            started_at: Instant::now(),
            cause: cause.clone(),
            seq: new_seq,
        });

        // 4. 触发事件 (修法 #3 关键 — tokio::sync::broadcast)
        //    ❌ 旧: onChildSessionEnded 单向通知, 反向不同步
        //    ✅ 新: 任何状态变化都广播, 所有 listener 都收
        self.event_bus_for_self().send(SessionEvent::StateChange {
            id: self.id.clone(),
            from: prev,
            to: SessionState::MidTask,
            at: Instant::now(),
        })?;

        self.event_bus_for_self().send(SessionEvent::MidTaskChange {
            id: self.id.clone(),
            mid_task: self.mid_task_state.clone().unwrap(),
            caused_by_seq: new_seq,
            at: Instant::now(),
        })?;

        Ok(new_seq)
    }
}

// ----------------------------------------------------------------------------
// 事件总线 + watch 跟踪 (修法 #3 核心)
// ----------------------------------------------------------------------------

/// 事件总线 (修法 #3 关键 — 替代 onChildSessionEnded 单向通知)
pub struct SessionEventBus {
    /// 全局广播 (任何 session 变化都广播)
    global: tokio::sync::broadcast::Sender<SessionEvent>,
    /// 每个 session 自己的 watch (跟踪 status, 防止 race)
    per_session: DashMap<SessionId, tokio::sync::watch::Sender<SessionState>>,
}

impl SessionEventBus {
    /// 订阅全局事件
    pub fn subscribe_global(&self) -> broadcast::Receiver<SessionEvent> {
        self.global.subscribe()
    }

    /// 订阅某个 session 的 status watch
    pub fn subscribe_session(&self, id: &SessionId) -> Option<watch::Receiver<SessionState>> {
        self.per_session.get(id).map(|s| s.subscribe())
    }

    /// 广播事件 (修法 #3: 任何状态变化都广播)
    pub fn broadcast(&self, event: SessionEvent) -> Result<(), SessionError> {
        match &event {
            SessionEvent::StateChange { id, to, .. } => {
                // 同时更新 watch (修法 #3 关键)
                if let Some(sender) = self.per_session.get(id) {
                    sender.send(*to).ok();
                }
            }
            _ => {}
        }
        self.global.send(event).map_err(|_| SessionError::Other(anyhow::anyhow!("event bus closed")))?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// get_output (修法 #2 落点 — 区分 mid-task 前/后输出)
// ----------------------------------------------------------------------------

impl SessionManager {
    /// 读 session 输出 (修法 #2 关键)
    /// include_mid_task=false 时跳过 mid-task 引起的 chunk
    pub async fn get_output(
        &self,
        id: &SessionId,
        since_seq: Option<u64>,
        include_mid_task: bool,
    ) -> Result<OutputChunk, SessionError> {
        let session = self.sessions
            .get(id)
            .ok_or_else(|| SessionError::NotFound(id.clone()))?;

        let since = since_seq.unwrap_or(0);

        // 修法 #2 关键: 区分 mid-task 前/后输出
        let chunks: Vec<&OutputChunk> = session.output_buffer
            .iter()
            .filter(|c| c.seq > since)
            .filter(|c| include_mid_task || !c.is_mid_task_response)  // ← 修法 #2
            .collect();

        let next_seq = chunks.last().map(|c| c.seq).unwrap_or(since);

        Ok(OutputChunk {
            seq: next_seq,
            content: chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join(""),
            at: chunks.last().map(|c| c.at).unwrap_or_else(Instant::now),
            caused_by_seq: next_seq,
            is_mid_task_response: chunks.iter().any(|c| c.is_mid_task_response),
        })
    }
}

// ----------------------------------------------------------------------------
// wait_idle (修法 #3 落点 — mid-task 期间不算 idle)
// ----------------------------------------------------------------------------

impl SessionManager {
    /// 等 session 变 idle (修法 #3 关键)
    /// mid_task_state 为 Interrupted/Merged 时不算 idle
    pub async fn wait_idle(
        &self,
        id: &SessionId,
        timeout: Option<Duration>,
    ) -> Result<IdleSignal, SessionError> {
        let timeout = timeout.unwrap_or(Duration::from_secs(60));
        let deadline = Instant::now() + timeout;

        loop {
            // 1. 拿 session
            let session = self.sessions
                .get(id)
                .ok_or_else(|| SessionError::NotFound(id.clone()))?;

            // 2. 检查 mid_task 状态 (修法 #3 关键)
            //    ❌ 旧: 子 agent stdin 空闲就返 idle, 不管是不是 mid-task
            //    ✅ 新: mid_task_state 为 Interrupted/Merged 时不算 idle
            if let Some(mid_task) = &session.mid_task_state {
                match mid_task {
                    MidTaskState::Interrupted { .. } | MidTaskState::Merged { .. } => {
                        // mid-task 进行中, 不算 idle
                        if Instant::now() >= deadline {
                            return Err(SessionError::Timeout(timeout));
                        }
                        drop(session);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    _ => {}
                }
            }

            // 3. 检查 session state
            if matches!(session.state, SessionState::Idle) {
                return Ok(IdleSignal {
                    idle: true,
                    idle_since: session.last_activity,
                    pending_mid_task: session.mid_task_state.is_some(),
                });
            }

            // 4. 终态直接返
            if session.state.is_terminal() {
                return Ok(IdleSignal {
                    idle: false,
                    idle_since: session.last_activity,
                    pending_mid_task: false,
                });
            }

            // 5. 等事件 (事件驱动, 不是轮询)
            let mut rx = self.event_bus.subscribe(id);
            drop(session);

            let event = tokio::time::timeout_at(deadline.into(), rx.recv()).await;

            match event {
                Ok(Ok(SessionEvent::StateChange { to: SessionState::Idle, .. })) => {
                    return Ok(IdleSignal {
                        idle: true,
                        idle_since: Instant::now(),
                        pending_mid_task: false,
                    });
                }
                Ok(Ok(_)) => continue,    // 其他事件, 继续循环
                Ok(Err(_)) => return Err(SessionError::Other(anyhow::anyhow!("event bus closed"))),
                Err(_) => return Err(SessionError::Timeout(timeout)),
            }
        }
    }
}
```

**为什么这样改 (主 O-2 19:33 走在前人经验上)**:
1. **CAS 原子状态转换**: `std::mem::replace` + 立即回滚, 内存级原子性, 不会有"窗口期"
2. **事件总线 + watch 双通道**: 广播事件给全局, watch 跟踪每个 session status, 防止 race
3. **mid-task 期间不算 idle**: wait_idle 检查 `mid_task_state` 而非仅看 `state`, 修法 #3 直击根因
4. **事件驱动不轮询**: `tokio::sync::broadcast` + `watch` 让 wait_idle 不用 50ms 轮询, 事件触发即时唤醒

### 4.4 3 处修法的统一不变量 (主 S-2 17:43 实验室)

> **3 处修法一起改, 形成"3 不变量"**。任一不变量被违反 = 撕裂状态复发。

| # | 不变量 | 修法 | 违反后果 |
|---|--------|------|---------|
| **I-1** | `send_message` 永不 panic, 永不 throw, 全部 `Result<SendResult, SessionError>` | 修法 #1 | 调用方误判实现错误, 撕裂状态 |
| **I-2** | 跨 session 引用 (agent→child) 都先验状态, 真实返回结果, 不假装 success | 修法 #2 | 父进程以为成功, 实际消息没发 |
| **I-3** | 状态变更用事件驱动 (`tokio::sync::broadcast`), CAS 原子转换, 无窗口期 | 修法 #3 | child 死, agent 还显示 running, 卡死 wait_idle |

**Kani 形式化验证目标** (per `apeireth-formal` §3.2 Kani 集成, R19+ 阶段 2 实施):

```rust
// 不变量 #1 的 Kani proof (估 80 LOC, 等 apeireth-formal 扩不变量)
#[cfg(kani)]
#[kani::proof]
fn verify_send_message_never_panics() {
    let state: SessionState = kani::any();
    let message: Message = kani::any();
    let result = send_message_pure(state, message);  // 纯函数版本
    assert!(result.is_ok() || matches!(result, Err(SessionError::Terminal { .. })));
    // 永不 panic, 只返 Ok(SendResult) 或 Err(SessionError)
}

// 不变量 #3 的 Kani proof
#[cfg(kani)]
#[kani::proof]
fn verify_state_transition_atomic() {
    let from: SessionState = kani::any();
    let to: SessionState = kani::any();
    let valid = is_valid_transition(from, to);
    // 任何非法转换都返 Err, 不会"半转换"留下撕裂状态
    assert!(valid || matches!(
        transition_state_pure(from, to),
        Err(SessionError::InvalidStateTransition { .. })
    ));
}
```

---

## §5 跟 apeireth-mcp::team 集成

> 来源: `reports/apeireth-mcp-14-tool-analysis-2026-08-05.md` §3 (3 处修法具体到 trait 方法)

### 5.1 集成架构 (R19+ 阶段 3 同期)

```
apeireth-mcp::team (新模块, R19+ 阶段 3 同期)
  ├── supervisor/
  │   ├── spawn_agent.rs    → 调 session_manager.create() + apeireth-agent
  │   ├── send_to_agent.rs  → 调 session_manager.send_message() (修法 #1 落点) + apeireth-agent (修法 #2 落点)
  │   ├── get_output.rs     → 调 session_manager.get_output(include_mid_task) (修法 #2 落点)
  │   ├── wait_idle.rs      → 调 session_manager.wait_idle() (修法 #3 落点)
  │   ├── wait.rs           → 调 session_manager.wait_exit()
  │   ├── get_status.rs     → 调 session_manager.state() + mid_task_state
  │   ├── list.rs           → 调 session_manager.list_active()
  │   └── cancel.rs         → 调 session_manager.cancel()
  ├── worktree/             (3 工具, 走 apeireth-git, 不直调 session)
  └── cognitive/            (3 工具, 走 apeireth-storage 读历史, 不直调 session)
```

### 5.2 3 处修法在 mcp::team 的具体落点

> 关键 (主 S-2 17:43 实事求是): 修法 #1 + #2 + #3 落点**全部**在 mcp::team 这 3 个工具的 `call()` 方法体内。

**修法 #1 落点 — `send_to_agent::call()` (调 `session.send_message`)**:

```rust
// apeireth-mcp/src/team/supervisor/send_to_agent.rs (伪代码, 仅设计)
#[async_trait]
impl Tool for SendToAgentTool {
    async fn call(&self, args: Value) -> Result<Value, String> {
        let args: SendArgs = serde_json::from_value(args)
            .map_err(|e| format!("invalid args: {}", e))?;
        let mut handle = self.team.write().await;
        let agent = handle.get_mut(&args.agent_id)
            .ok_or_else(|| format!("agent not found: {}", args.agent_id))?;
        
        // ←——— 修法 #1 核心: 调 session.send_message (永不 throw)
        let send_result = self.session_manager
            .send_message(&agent.session_id, args.message.clone().into())
            .await
            .map_err(|e| format!("session error: {}", e))?;
        
        // 修法 #1 关键: match SendResult, 不假设成功
        let (delivered, mid_task_state, seq) = match send_result {
            SendResult::Sent => (true, "none", agent.input_seq + 1),
            SendResult::MidTask { queued, seq } => (queued, "queued", seq),
            SendResult::Failed { reason, current_state } => {
                // ❌ 旧: 永远 return { delivered: true }
                // ✅ 新: 真实返 { delivered: false, reason: ..., current_state: ... }
                return Ok(serde_json::json!({
                    "delivered": false,
                    "mid_task_state": "failed",
                    "seq": agent.input_seq,
                    "reason": reason,
                    "current_session_state": format!("{:?}", current_state),
                }));
            }
        };
        
        agent.input_queue.push_back(InputItem {
            seq,
            msg: args.message,
            mid_task: MidTaskState::from_send_result(&send_result),
        });
        
        Ok(serde_json::json!({
            "delivered": delivered,
            "mid_task_state": mid_task_state,
            "seq": seq,
        }))
    }
}
```

**修法 #2 落点 — `get_output::call()` (调 `session.get_output(include_mid_task)`)**:

```rust
// apeireth-mcp/src/team/supervisor/get_output.rs (伪代码)
#[async_trait]
impl Tool for GetOutputTool {
    async fn call(&self, args: Value) -> Result<Value, String> {
        let args: GetOutputArgs = serde_json::from_value(args)
            .map_err(|e| format!("invalid args: {}", e))?;
        let include_mid_task = args.include_mid_task.unwrap_or(true);  // 默认 true
        
        // ←——— 修法 #2 核心: 传 include_mid_task 给 session.get_output
        let chunk = self.session_manager
            .get_output(&args.agent_id, args.since_seq, include_mid_task)
            .await
            .map_err(|e| format!("session error: {}", e))?;
        
        Ok(serde_json::json!({
            "chunks": [{
                "seq": chunk.seq,
                "content": chunk.content,
                "is_mid_task_response": chunk.is_mid_task_response,  // 修法 #2 关键
                "caused_by_seq": chunk.caused_by_seq,
            }],
            "next_seq": chunk.seq,
        }))
    }
}
```

**修法 #3 落点 — `wait_idle::call()` (调 `session.wait_idle()`)**:

```rust
// apeireth-mcp/src/team/supervisor/wait_idle.rs (伪代码)
#[async_trait]
impl Tool for WaitIdleTool {
    async fn call(&self, args: Value) -> Result<Value, String> {
        let args: WaitIdleArgs = serde_json::from_value(args)
            .map_err(|e| format!("invalid args: {}", e))?;
        let timeout = args.timeout_ms.map(Duration::from_millis);
        
        // ←——— 修法 #3 核心: session.wait_idle 已修, mid-task 期间不算 idle
        let signal = self.session_manager
            .wait_idle(&args.agent_id, timeout)
            .await
            .map_err(|e| format!("session error: {}", e))?;
        
        Ok(serde_json::json!({
            "idle": signal.idle,
            "idle_since": signal.idle_since.elapsed().as_millis() as u64,
            "pending_mid_task": signal.pending_mid_task,  // 修法 #3 关键
        }))
    }
}
```

### 5.3 14 工具全集的 session 依赖矩阵

| # | 工具 | 调 session API | 修法落点 |
|---|------|----------------|---------|
| 1 | `spawn_agent` | `create()` | — |
| 2 | **`send_to_agent`** ⭐ | `send_message()` | **修法 #1 核心** |
| 3 | `get_output` | `get_output(include_mid_task)` | **修法 #2 核心** |
| 4 | `wait_idle` | `wait_idle()` | **修法 #3 核心** |
| 5 | `wait` | `wait_exit()` | — |
| 6 | `get_status` | `state()` + `mid_task_state()` | 修法 #2+#3 辅助 |
| 7 | `list` | `list_active()` | — |
| 8 | `cancel` | `cancel()` | — |
| 9-11 | 3 worktree | (不直调, 走 apeireth-git) | — |
| 12-14 | 3 认知 | (不直调, 走 apeireth-storage) | — |

**P0 必改 3 工具**: `send_to_agent` / `get_output` / `wait_idle` (修法 1+2+3 直击)

---

## §6 跟 apeireth-team-lead 集成

> 来源: `apeireth-team-lead-implementation-guide-2026-08-05.md` §3 (team-lead 不直接调 session)

### 6.1 间接调用 (team-lead → mcp::team → session)

**关键 (主 S-2 17:43 实事求是)**: team-lead **永不直接** `use apeireth_session::*`。所有 session 调用走 `apeireth-mcp::team::*` 14 工具的 `Tool::call()` 入口。

```rust
// apeireth-team-lead/src/lib.rs (伪代码, 不写实现)
impl TeamLead {
    /// 构造 awareness prompt 时, 注入 mid_task 状态信息
    /// 让 LLM 知道子 agent 状态 (per user memory #3 拟人化)
    pub async fn build_awareness_prompt(&self, session: &Session) -> String {
        // 1. 调 mcp::team::list (Tool::call) 拿活跃子 agent
        let active_agents = self.mcp_client
            .call_tool("list", json!({}))
            .await
            .map_err(|e| anyhow::anyhow!("list failed: {}", e))?;
        
        // 2. 对每个活跃 agent, 调 get_status 拿 mid_task 状态
        let mut awareness = String::from("## 当前活跃子 Agent\n");
        for agent in active_agents["agents"].as_array().unwrap_or(&vec![]) {
            let status = self.mcp_client
                .call_tool("get_status", json!({ "agent_id": agent["agent_id"] }))
                .await?;
            awareness.push_str(&format!(
                "- agent_id={} state={} mid_task={:?}\n",
                agent["agent_id"], status["state"], status["mid_task_state"]
            ));
        }
        
        awareness
    }
}
```

### 6.2 集成约束 (强约束)

| 约束 | 来源 | 违反后果 |
|------|------|---------|
| **不依赖 apeireth-session** | ADR-0011 §决策 4 (适用 team-lead, session 同理反向) | ❌ 严禁加, 加 = 循环依赖 |
| **不依赖 apeireth-mcp::team** | team-lead 通过 mcp client 间接调 | ✅ 允许, mcp 是 R19+ 已实装 |
| **不直接调 session API** | team-lead 是 agent role, 不是 session owner | ❌ 走 mcp::team 工具 |

---

## §7 跟 apeireth-storage 集成

### 7.1 持久化策略 (per user memory 持久化偏好)

| 数据 | 存储 | 策略 | 频率 |
|------|------|------|------|
| **session state** | SQLite WAL | 每次状态转换写 DB | 同步 (事务保证原子) |
| **session messages** | SQLite WAL | 每次 send_message 写 | 异步 (批 100ms flush) |
| **mid_task 状态** | 单独表 `mid_task_log` (避免主表膨胀) | 每次 mid_task 变化写 | 同步 |
| **session event** | in-memory broadcast (不持久化) | 实时广播, 不存 | — |

### 7.2 mid_task_log 表设计 (避免主表膨胀)

```sql
-- apeireth-storage/src/migrations/V7__session_mid_task.sql (估 50 LOC)

CREATE TABLE IF NOT EXISTS mid_task_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,                    -- SessionId.to_string()
    seq INTEGER NOT NULL,                        -- mid_task 序列号
    state TEXT NOT NULL,                         -- MidTaskState (serialized)
    cause_message_id INTEGER,                    -- 触发的 message ID
    started_at INTEGER NOT NULL,                 -- Unix timestamp ms
    ended_at INTEGER,                            -- Unix timestamp ms
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_mid_task_session ON mid_task_log(session_id, seq DESC);
CREATE INDEX idx_mid_task_active ON mid_task_log(session_id) WHERE ended_at IS NULL;

-- 主 session 表 (瘦身)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                         -- SessionId.to_string()
    state TEXT NOT NULL,                         -- SessionState
    config TEXT NOT NULL,                        -- AdapterSessionConfig JSON
    last_activity INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    message_count INTEGER DEFAULT 0,
    mid_task_seq INTEGER DEFAULT 0
);

CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity DESC);
```

### 7.3 事务保证 (主 O-5 17:58 不假装)

```rust
// apeireth-storage/src/session_storage.rs (伪代码)
impl SqliteSessionStorage {
    /// 状态转换 + 事件持久化, 单一事务
    pub async fn save_state_transition(
        &self,
        session_id: &SessionId,
        from: SessionState,
        to: SessionState,
        mid_task: Option<&MidTaskState>,
    ) -> Result<(), StorageError> {
        let mut tx = self.conn.begin().await?;
        
        // 1. 更新主表
        sqlx::query("UPDATE sessions SET state = ? WHERE id = ?")
            .bind(serde_json::to_string(&to)?)
            .bind(session_id.to_string())
            .execute(&mut *tx)
            .await?;
        
        // 2. 如果是 mid_task 变化, 写 mid_task_log
        if let Some(mt) = mid_task {
            sqlx::query("INSERT INTO mid_task_log (session_id, seq, state, started_at) VALUES (?, ?, ?, ?)")
                .bind(session_id.to_string())
                .bind(0)  // seq
                .bind(serde_json::to_string(mt)?)
                .bind(chrono::Utc::now().timestamp_millis())
                .execute(&mut *tx)
                .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }
}
```

---

## §8 R-Measure 守门 (17→24 维)

> 来源: `docs/stage4/r-measure-verification-design-2026-08-05.md`

### 8.1 session 实施完必跑 R-Measure verify

| 测度 | 基线 (R11 V0.5) | 目标 | 17 维 vs 24 维 |
|------|----------------|------|---------------|
| **V1141** (composite) | 0.8682 | ≥ 0.8682 | 24 维主测度, 17→24 投影在 verifier |
| **V1131** (子测度) | 0.8532 | ≥ 0.8532 | 24 维, 投影在 verifier |
| **V1136** (dashboard 真测) | 0.9063 | ≥ 0.9063 | 24 维 9 子测度, 投影在 verifier |

### 8.2 session 涉及的子测度 (V1136 9 子测度中 4 个)

| 子测度 | session 关联 | 测量方式 |
|--------|------------|---------|
| `continuity_5` (会话连续性) | 🟢 强 | session lifecycle 不中断 |
| `salience_5` (消息显著性) | 🟢 强 | message 密度 + 重要度 |
| `identity_5` (身份稳定性) | 🟡 弱 | session_id 一致性 |
| `temporal_4` (时间连续性) | 🟡 弱 | 状态转换时序 |

### 8.3 17→24 维投影公式 (需主人拍板)

```rust
// apeireth-r-measure-verify/src/project.rs (伪代码, 等主人拍板权重)
pub fn project_24_to_17(trace_24: &DimensionTrace) -> BTreeMap<&'static str, f64> {
    // 24 维 → 17 维 per R11 v1077_asi_v04_full_measurement.py
    // 主 S-2 17:43 实事求是: 投影公式需主人拍板权重
    // ❌ 旧: hardcode 17 维 (跟 R11 baseline 不一致)
    // ✅ 新: 24 维 LOCKED, 投影在 verifier, 权重可调
    todo!("等主人拍板 24→17 投影权重 (per R-Measure §2.1)")
}
```

---

## §9 验收标准

### 9.1 功能验收

- [ ] 30 unit tests (manager: 10, mid_task: 8, concurrency: 8, state: 4)
- [ ] 8 integration tests (mid_task 4 成功 + 4 失败场景, 见 §9.2)
- [ ] 编译通过 (`cargo build` + `cargo build --release`)
- [ ] cargo clippy 全过 (`cargo clippy -- -D warnings`)
- [ ] cargo fmt 全过 (`cargo fmt --check`)
- [ ] no unsafe (workspace deny 守门)
- [ ] 编译期 hardcode 守门 (per APEIRETH-CONVENTIONS §2)
- [ ] Kani 形式化验证 3 不变量 (per §4.4, R19+ 阶段 2 实施)

### 9.2 mid-task 4 成功 + 4 失败 场景

**4 成功**:
1. ✅ session 正常创建 → send → wait_idle → 终态 Completed
2. ✅ session 在 Running 时 send → 正常发出, idle flag 清
3. ✅ mid-task 状态时 send → 消息入队, 返 `MidTask { queued: true }`
4. ✅ get_output(include_mid_task=true) → 包含 mid-task 引起的 chunk

**4 失败**:
1. ❌ session 终态 (Completed) 时 send → 返 `Failed { reason, current_state }`, 不抛错
2. ❌ agent 终态 (Exited) 时 send_to_agent → 返 `Failed { reason, current_session_state }`
3. ❌ child session 异常退出时 wait_idle → 不会卡死 5min, 立刻返 `idle: false` + 终态
4. ❌ mid-task 期间 wait_idle → 不会返 idle, 继续等

### 9.3 R-Measure 守门

- [ ] R-Measure verify 脚本跑通 (`apeireth-r-measure-verify`)
- [ ] V1141 ≥ 0.8682
- [ ] V1131 ≥ 0.8532
- [ ] V1136 ≥ 0.9063
- [ ] 17→24 维投影公式主人拍板 (per §8.3)

### 9.4 工程规范

- [ ] workspace member 加 `apeireth-session` (由 code_reviewer 改 Cargo.toml, rust-coder 不动)
- [ ] README.md 写清楚 crate 职责 (估 80 LOC, 跟 team-lead 一致)
- [ ] examples/session_demo.rs 演示 mid-task 场景
- [ ] 注释完整, 公开 API 全有 `///` doc
- [ ] 任何"偏离"在 commit message 写 "⚠️ 偏离: <原因>"

---

## §10 实施时间表 (8 阶段, 5 天)

| 阶段 | 时长 | 任务 | Owner | 依赖 | 估 LOC |
|------|------|------|-------|------|------:|
| **1** | 0.5 天 | 等 code_reviewer 完工后建仓 + Cargo.toml + workspace member | rust-coder | code_reviewer 完工 | 100 |
| **2** | 1 天 | `session.rs` + `state.rs` + SessionState enum (6 状态) | rust-coder | 阶段 1 | 300 |
| **3** | 1 天 | `manager.rs` + send_message 状态机 (修法 #1) | rust-coder | 阶段 2 | 400 |
| **4** | 0.5 天 | `mid_task.rs` + MidTaskState + AgentHandle (修法 #2 + #3) | rust-coder | 阶段 3 | 300 |
| **5** | 0.5 天 | `concurrency.rs` (ConcurrencyGuard) + `storage.rs` (WAL) | rust-coder | 阶段 3 | 350 |
| **6** | 0.5 天 | `error.rs` (thiserror) + `lib.rs` 公开 API | rust-coder | 阶段 2-5 | 150 |
| **7** | 0.5 天 | 30 unit tests + 8 integration tests (mid-task 4+4) | qa_engineer | 阶段 6 | 800 |
| **8** | 0.5 天 | R-Measure verify + cargo clippy + 17→24 维投影 | qa_engineer | 阶段 7 | — |
| **总计** | **5 天** | (1 周, 1 人) | | | **2400** (含 tests) |

### 10.1 阶段详细

**阶段 1: 建仓 (0.5 天)**
- 等 code_reviewer 改 `Cargo.toml` 加 `apeireth-session` member
- 建 `crates/apeireth-session/` 目录
- 写 `Cargo.toml` (本蓝图 §3.2)
- 写 `README.md` (估 80 LOC, R19 必加)
- 写 `src/lib.rs` 空壳 (只 pub mod 声明)

**阶段 2: 基础类型 (1 天)**
- `src/state.rs`: SessionState enum (8 状态) + MidTaskState enum (5 状态) + is_terminal() + is_valid_transition()
- `src/session.rs`: Session struct (id, state, messages, mid_task_state, mid_task_seq, ...)
- 编译期 hardcode 守门: 8 + 5 = 13 个 enum variant 必须穷举 match (clippy 警告)

**阶段 3: send_message 状态机 (1 天) - P0**
- `src/manager.rs`: SessionManager struct + send_message (修法 #1 核心, 本蓝图 §4.1)
- 6 状态转换合法性表 (is_valid_transition)
- SendResult 3 变体 (Sent / MidTask / Failed)
- 单测: 4 成功 + 4 失败 (本蓝图 §9.2)

**阶段 4: mid_task + AgentHandle (0.5 天) - P0**
- `src/mid_task.rs`: MidTaskState 完整 + transition_to_mid_task CAS (修法 #3 核心, 本蓝图 §4.3)
- AgentHandle struct + SendToAgentResult 4 变体 (修法 #2 落点, 本蓝图 §4.2)
- mid_task_queue (VecDeque 限 100)
- 事件总线 (tokio::sync::broadcast)

**阶段 5: 并发 + 存储 (0.5 天)**
- `src/concurrency.rs`: ConcurrencyGuard trait + DefaultConcurrencyGuard (semaphore + sysinfo)
- `src/storage.rs`: SessionStorage trait + SqliteSessionStorage (rusqlite + WAL)
- mid_task_log 单独表 (本蓝图 §7.2)

**阶段 6: error + lib 公开 API (0.5 天)**
- `src/error.rs`: SessionError 9 变体 (thiserror)
- `src/lib.rs`: 公开 API (本蓝图 §3.3 完整)
- examples/session_demo.rs

**阶段 7: 测试 (0.5 天)**
- tests/manager_tests.rs: 10 tests
- tests/mid_task_tests.rs: 8 tests (4 成功 + 4 失败, 本蓝图 §9.2)
- tests/concurrency_tests.rs: 8 tests
- 总 30 unit + 8 integration

**阶段 8: R-Measure 守门 (0.5 天)**
- 跑 R-Measure verify 脚本
- V1141 / V1131 / V1136 三值检查
- cargo clippy + cargo fmt + Kani (R19+ 阶段 2 同步)
- 17→24 维投影公式等主人拍板 (per §8.3)

---

## §11 8 项不修改承诺

跟 ADR-0011 §不修改承诺 + APEIRETH-CONVENTIONS §10 + `spectrAI-integration-blueprint §10` 一致:

| # | 承诺 | 适用本蓝图 | 来源 |
|---|------|----------|------|
| 1 | **阶段 1/2/3/4/5 LOCKED** | ✅ 不动 R11 阶段 1-5 任何决策 | APEIRETH-CONVENTIONS §10 |
| 2 | **v2/v4/v4.1 LOCKED** | ✅ session 跟 v2/v4/v4.1 接口兼容 (R19 阶段 3 加, 不破已有) | APEIRETH-CONVENTIONS §10 |
| 3 | **12 键 LOCKED** | ✅ Document-Meta + 12 字段全保留 (本蓝图 §Document-Meta) | APEIRETH-CONVENTIONS §10 |
| 4 | **6 锚 LOCKED** | ✅ S-1/S-2/O-2/O-3/O-4/O-5 全穿透 (本蓝图 §12) | APEIRETH-CONVENTIONS §10 |
| 5 | **workspace v1.0.0** | ✅ apeireth-session 是新 member, 不破 workspace v1.0.0 兼容 (code_reviewer 改 Cargo.toml 加) | APEIRETH-CONVENTIONS §10 |
| 6 | **Document-Meta 严格** | ✅ 本蓝图 §Document-Meta 严格按 12 字段 | APEIRETH-CONVENTIONS §0.1 |
| 7 | **R11 baseline 三值** | ✅ V1141 ≥ 0.8682 / V1131 ≥ 0.8532 / V1136 ≥ 0.9063 (本蓝图 §8.1) | APEIRETH-CONVENTIONS §10 |
| 8 | **不重复造轮子** | ✅ 复用 apeireth-protocol/storage/tool-registry, 不重写 | user memory #6 |

**特别声明 (主 S-2 17:43 实事求是)**: 本蓝图 §3.3 公开 API 中的 `Cargo.toml` 是**伪 TOML**,等 code_reviewer 改完 workspace Cargo.toml 后, rust-coder 才在 `crates/apeireth-session/Cargo.toml` 落。本蓝图不预先写实际文件。

---

## §12 6 哲学 anchor 穿透

| 锚 | 时间 | 内容 | 本蓝图穿透 |
|----|------|------|----------|
| **S-1 北极星** | 22:33 | "6 anchor ASI 完整性" | §2 战略定位: session 是 ASI 完整性的核心 (6 状态机 + mid-task bug 根治) |
| **S-2 实事求是** | 17:43 | "6 anchor 实验室" | §1.2 不修的代价 + §4.4 3 不变量 + §11 特别声明 (不预先写实际文件) |
| **O-2 走在前人经验上** | 19:33 | "6 anchor 4 分类" | §2.3 4 分类严格分离 (session / team-lead / council / supervisor / agent 互不混) |
| **O-3 干到底** | 23:44 | "6 anchor 决策清单" | §10 8 阶段实施 (5 天, 1 人, 2400 LOC 含 tests) |
| **O-4 任何人都能接手** | 00:56 | "6 anchor 12 统一" | §3 骨架 + §9 验收 + §10 时间表 (rust-coder 接手照着干) |
| **O-5 不假装** | 17:58 | "6 anchor 12 急救" | §4 修法 1+2+3 (sendMessage 永不 throw, sendToAgent 真实返回, child 状态原子) |

**特别说明 (主 S-2 17:43)**:
- §4 修法是"O-5 不假装"的直接体现: sendMessage 不假装能写, sendToAgent 不假装能送达, child session 不假装同步
- §7.3 事务保证是"O-5"的存储层体现
- §9 验收 8 失败场景是"O-5"的测试层体现

---

## §13 关联文档

### 13.1 必读 (本蓝图的依据)

- `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 集成蓝图总纲)
- `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` (team-lead 实施, 间接调 session)
- `docs/stage4/r-measure-verification-design-2026-08-05.md` (R-Measure 17→24 维守门)
- `reports/apeireth-session-vector-asi-2026-08-05.md` (session/vector/formal/asi 4 crate 现状)
- `reports/apeireth-mcp-14-tool-analysis-2026-08-05.md` (3 处修法具体到 trait 方法)
- `reports/apeireth-council-7-advisor-analysis-2026-08-05.md` (7 advisor voting 集成)
- `spectrai/docs/ARCHITECTURE.md` §4 (mid-task bug 根因深挖)

### 13.2 后续产出 (待写)

- `docs/adr/0013-apeireth-session-crate-design.md` (本蓝图落 ADR, 等主人拍板)
- `docs/stage4/apeireth-storage-blueprint-2026-08-05.md` (storage 蓝图, 同期实施)
- `docs/stage4/apeireth-team-lead-session-integration-test-2026-08-05.md` (team-lead 集成测试, 阶段 7 后)

### 13.3 沉淀 (R19+ 阶段 1.4 后)

- `tauri-roadmap-2026-08-05.md` (TUI 升级路线图, 暂告段落)
- Tauri-roadmap 加 T-014: apeireth-session 在 Tauri 阶段的 UI 集成 (per user memory #8 终极 Tauri)

---

## 附录 A: 状态机 ASCII 图

```
                    ┌──────────────┐
                    │     Idle     │ ◄────────────────┐
                    └──────┬───────┘                  │
                           │ send_message / create    │
                           ▼                          │
                    ┌──────────────┐                  │
              ┌────►│   Running    │                  │
              │     └──────┬───────┘                  │
              │            │ LLM done / tool_result   │
              │            ▼                          │
              │     ┌──────────────┐                  │
              │     │ WaitingInput │ (e.g. ask user)  │
              │     └──────┬───────┘                  │
              │            │ user reply              │
              │            │                          │
              │     ┌──────┴───────┐                  │
              │     │              │ mid-task trigger  │
              │     │              ▼                  │
              │     │       ┌──────────────┐         │
              │     │       │   MidTask    │         │
              │     │       └──────┬───────┘         │
              │     │              │ mid-task done   │
              │     │              │                  │
              │     ▼              ▼                  │
              │  ┌───────────────────────┐           │
              │  │ 终态: Completed       │           │
              │  │ 终态: Failed          │ (任一可达) │
              │  │ 终态: Cancelled       │           │
              │  │ 终态: Terminated      │           │
              │  └───────────────────────┘           │
              │                                      │
              └──── Cancelled (从 Running / Idle / WaitingInput / MidTask 可达)
```

**关键不变量 (本蓝图 §4.4 I-1+I-2+I-3)**:
- 终态 → 任何: 全部非法 (除 Cancelled 是 mid-task 间过渡)
- 状态转换用 CAS, 无窗口期 (修法 #3)
- mid-task 期间 wait_idle 不算 idle (修法 #3 落点)

---

## 附录 B: 报告 (写文档任务完成)

按主 S-2 17:43 实事求是 + 00 后风格:

**文件**: `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\apeireth-session-blueprint-2026-08-05.md`

**章节**: §1 战略背景 + §2 战略定位 + §3 crate 骨架 + §4 mid-task bug 3 处修法 (核心) + §5 跟 mcp::team 集成 + §6 跟 team-lead 集成 + §7 跟 storage 集成 + §8 R-Measure 守门 + §9 验收标准 + §10 8 阶段实施时间表 + §11 8 项不修改承诺 + §12 6 哲学 anchor + §13 关联文档

**3 处修法 + Rust 伪代码**:
- §4.1 修法 1: `send_message` 状态机 (替代 throw 改 return) — 完整 Rust 伪代码
- §4.2 修法 2: `send_to_agent` 真实返回 (替代 .catch() 吞 + 永远 success:true) — 完整 Rust 伪代码
- §4.3 修法 3: 事件驱动 + 状态原子性 (替代时序竞态窗口期) — 完整 Rust 伪代码

**8 阶段实施时间表**: 5 天, 1 人, 2400 LOC (含 tests) — 详见 §10

**需要 Mavis 拍板的事**:
1. ⏸️ **17→24 维 R-Measure 投影公式** (per §8.3, 需主人拍板权重, apeireth-session 阶段 8 阻塞)
2. ⏸️ **Cargo.toml 加 workspace member 时机** (per §3.2 + §10 阶段 1, 等 code_reviewer 完工)
3. ⏸️ **apeireth-storage 实施时序** (per §7, R19+ 阶段 1.4 同期, 需 Mavis 协调)
4. ⏸️ **mcp::team 14 工具的 mid-task 修法优先级** (per §5.2, P0 必改 3 工具 = send_to_agent / get_output / wait_idle, 需 Mavis 协调 rust-coder 同步)
5. ⏸️ **Kani 形式化验证时序** (per §4.4, R19+ 阶段 2 实施, 需 Mavis 协调)

**需要 leader (主人) 拍板的事**:
1. ⏸️ 17→24 维投影权重 (per §8.3)
2. ⏸️ session 实施 LOC 上下沿 (1500-2000 区间, 阶段 4 mid_task.rs 可能超 500, 需主人确认)
3. ⏸️ session 跟 apeireth-storage 的依赖方向 (本蓝图 §3.2 session 依赖 storage, 跟其他 crate 是否一致)
