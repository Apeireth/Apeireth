# apeireth-team-lead 实施指南 (R19+ 阶段 3)

```
[Document-Meta]
Document: docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 阶段 3
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 rust-coder 接手)
```

> **性质**: R19+ 阶段 3 实施手册 — 记录 `apeireth-team-lead` 新 crate (估 850 LOC) 怎么一步步实施。给 rust-coder 接手时**照着干**,**不写实际 Rust 代码**,只写 step-by-step 步骤 + 关键决策 + 伪代码 + 验收标准。
>
> **依据**: `docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md` §决策 1-5 + `docs/adr/0012-team-lead-council-collaboration.md` §决策 1-4 + `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 第 7 行 + `docs/stage4/glossary-spectrAI-additions-2026-08-05.md` 词条 2/3/4。
>
> **约束**:
> - ❌ **不写实际 Rust 代码** — 只写伪代码 + TOML 模板 + 步骤说明
> - ❌ **不碰 M 标记文件** — Cargo.toml / Cargo.lock / CHANGELOG.md / README.md / ROADMAP.md / .github/workflows/rust-ci.yml / crates/apeireth-action/ / crates/apeireth-agent/ / crates/apeireth-api/ 全部 M 标记
> - ❌ **不碰 crates/apeireth-team-lead/** — 等 code_reviewer 完工 (改 Cargo.toml 加 workspace member + 改 apeireth-mcp 加 team 集成锚点) 后再创建
> - ❌ **不 commit / push / git add** — 写文档任务
>
> **当前状态** (2026-08-05): 文档交付,等 rust-coder 接手。

---

## §1 战略背景 (为什么)

### 1.1 决策回放 (2026-08-05 13:34 主人拍板)

| 决策项 | 内容 | 来源 |
|---|---|---|
| **A 方案命名** | `apeireth-team-lead` (新 crate, 不复用 `apeireth-supervisor`) | ADR-0011 §决策 1 |
| **1:1 翻译 supervisorPrompt.ts** | 不按 minimax 习惯改写,818 行 markdown 完整保留 | ADR-0011 §决策 2 |
| **集成位置** | `apeireth-mcp::team` 通过 trait 调 team-lead | ADR-0011 §决策 3 |
| **命名空间严格分离** | team-lead ≠ supervisor (进程) ≠ council (审议) | ADR-0011 §决策 4 |
| **7 advisor voting 通过 trait 注入** | `apeireth-protocol` 定义 `AdvisorVotingTrigger` | ADR-0011 §决策 5 + ADR-0012 §决策 1-4 |

### 1.2 角色定位 (4 分类清晰)

| crate | 层级 | 职责 | 类比 |
|---|---|---|---|
| `apeireth-team-lead` (新) | Agent-level | 构造 supervisor prompt + 触发 7 advisor voting | 项目经理 (PM) |
| `apeireth-supervisor` (550 LOC) | OS-level | 进程监督 (PID 1 / 5 sub-supervisor) | init / systemd |
| `apeireth-council` (2740 LOC) | Application-level | 7 强制 Advisor 平行审议 | 陪审团 / 安全审查委员会 |
| `apeireth-agent` (1358 LOC) | Component-level | 单 agent 行为 + AgentManager | 员工 / 工具人 |

**绝不含糊**: team-lead 是 agent role (prompt 字符串), supervisor 是 OS process (tokio task), council 是 safety gate (横向 7 席), agent 是单数 (singular worker)。

### 1.3 不依赖 `apeireth-supervisor` (强约束)

> ADR-0011 §决策 4: `apeireth-team-lead` **不依赖** `apeireth-supervisor` (避免循环依赖风险)。

**为什么**: 两者职责完全不同 (agent-level prompt vs OS-level process), 任何依赖都意味着单一职责原则被破坏。rust-coder 加 `Cargo.toml` 依赖时**严禁**加 `apeireth-supervisor`。

### 1.4 通过 trait 注入 council voting (不直接调)

> ADR-0012 §决策 1-2: team-lead 持有 `Arc<dyn AdvisorVotingTrigger>`, **不**直接 `use apeireth_council::*`。

**好处**:
- ✅ team-lead 跟 council 解耦,可独立单测
- ✅ mock / fake / 真实实现可替换 (R19 阶段 3 用 Noop, R21+ 替换 CouncilVotingTrigger)
- ✅ +150 LOC 总成本可控 (80 trait + 100 bridge + 30 mock)

---

## §2 crate 骨架设计

### 2.1 目录结构 (估 850 LOC 总)

```
crates/apeireth-team-lead/
├── Cargo.toml            (新增, 跟 code_reviewer 加 workspace member 一起做)
├── README.md             (R19 必加, 估 80 LOC)
├── src/
│   ├── lib.rs            (主入口, 估 100 LOC)
│   ├── prompt.rs         (构造 supervisor prompt, 估 350 LOC)
│   ├── awareness.rs      (构造 awareness prompt, 估 200 LOC)
│   ├── tools.rs          (14 个工具的 prompt 描述, 估 150 LOC)
│   ├── voting.rs         (AdvisorVotingTrigger impl, 估 50 LOC)
│   └── mocks.rs          (NoopVotingTrigger, 估 30 LOC)
├── tests/
│   ├── prompt_tests.rs   (估 30 tests, 估 250 LOC)
│   ├── awareness_tests.rs (估 20 tests, 估 150 LOC)
│   ├── tools_tests.rs    (14 工具各 1 happy path + edge, 估 200 LOC)
│   └── voting_tests.rs   (trait + Noop + 触发条件, 估 100 LOC)
└── examples/
    └── team_lead_demo.rs (估 50 LOC, demo 怎么 build_supervisor_prompt)
```

**LOC 累计**:
- src/: 100 + 350 + 200 + 150 + 50 + 30 = **880 LOC** (略超 ADR-0011 估 850,因为多 mocks.rs)
- tests/: 250 + 150 + 200 + 100 = **700 LOC** (含 happy + edge + integration)
- examples/: 50 LOC
- **总计 ~1630 LOC**

> **不假装**: 实际 LOC 会随翻译结果波动 ±20%,以最终 `cargo count` 为准。ADR-0011 估 850 只含 src/。

### 2.2 Cargo.toml 关键字段 (伪 TOML, 真正建 crate 时由 code_reviewer 落)

> ⚠️ 本节是伪 TOML,**不写实际文件**。等 code_reviewer 改完 `Cargo.toml` 加 workspace member 后,rust-coder 才在 `crates/apeireth-team-lead/Cargo.toml` 落下面内容。

```toml
[package]
name = "apeireth-team-lead"
version = "0.1.0"  # R19+ 阶段 3 起步
edition = "2024"
description = "R19+ 团队 leader 角色 - 1:1 翻译 SpectrAI supervisorPrompt.ts (970 LOC)"
license = "Apache-2.0 OR MIT"
repository = "https://github.com/apeireth/apeireth"
keywords = ["apeireth", "team-lead", "agent", "prompt", "spectrAI"]
categories = ["api-bindings", "asynchronous"]

[dependencies]
# 核心依赖 (按 ADR-0011 §决策 4, 不依赖 apeireth-supervisor)
apeireth-protocol = { path = "../apeireth-protocol" }   # ProviderEvent, AdapterSessionConfig, AdvisorVotingTrigger trait
apeireth-agent     = { path = "../apeireth-agent" }       # Agent, AgentManager (复用)
apeireth-mcp       = { path = "../apeireth-mcp" }         # 14 工具的 trait 定义 (等 mcp 实施)
serde              = { version = "1", features = ["derive"] }
serde_json         = "1"
tokio              = { version = "1", features = ["full"] }
tracing            = "0.1"
thiserror          = "1"
async-trait        = "0.1"

[dev-dependencies]
# 测试用 mock LLM client (等 apeireth-api::testing 提供)
tokio-test        = "0.4"
pretty_assertions  = "1"

[lints]
workspace = true   # 继承 workspace.lints (R19 新加, 等 code_reviewer 改完 Cargo.toml)
```

**关键字段说明**:

| 字段 | 为什么 | 风险 |
|---|---|---|
| `version = "0.1.0"` | R19 阶段 3 起步,跟随 workspace 0.14.0 增量 | ⚠️ semver 严格,不能跳 |
| `apeireth-protocol` | `AdvisorVotingTrigger` trait 放这里 (ADR-0012 §决策 1) | 不依赖 = 编译失败 |
| `apeireth-agent` | 复用 `Agent` / `AgentManager` 类型 | 不依赖 = `system_prompt` 字段无法引用 |
| `apeireth-mcp` | 14 工具的 trait 定义在 mcp::team (ADR-0010) | ⚠️ R19 阶段 3 同期实施,可能没到位 |
| **不依赖 `apeireth-supervisor`** | ADR-0011 §决策 4 强约束 | ❌ 严禁加,加了就违反 ADR |
| **不依赖 `apeireth-council`** | ADR-0012 §决策 2 (走 trait 注入) | ❌ 严禁加,直接调 = 紧耦合 |

### 2.3 lib.rs 公开 API 设计 (伪代码, 不写实际代码)

```rust
// ============================================================================
// lib.rs 公开 API 设计 (伪代码, 仅展示 API 形态, 不写实现)
// ============================================================================

/// 团队 leader 角色 — 构造 supervisor prompt + 触发 7 advisor voting
pub struct TeamLead {
    cfg: TeamLeadConfig,
    voting: Arc<dyn AdvisorVotingTrigger>,
    // 内部缓存: 已构造的 system_prompt (避免重复 build)
    prompt_cache: Arc<tokio::sync::RwLock<Option<String>>>,
}

/// 团队 leader 配置
pub struct TeamLeadConfig {
    /// LLM model 标识 (e.g. "claude-opus-4-1" / "minimax-m3")
    pub model: String,
    /// Persona 描述 (注入到 prompt 头部)
    pub persona: String,
    /// 完整 system prompt (调 build_supervisor_prompt 构造)
    pub system_prompt: String,  // 构造后填充
    /// 7 advisor voting 触发阈值 (默认 0.8, criticality >= 触发)
    pub voting_threshold: f32,
}

/// 14 supervisor 工具的元数据 (调 build_supervisor_prompt 注入)
pub struct ToolMeta {
    pub name: String,           // "spawn_agent" / "send_to_agent" / etc.
    pub description: String,    // 1:1 翻译自 supervisorPrompt.ts
    pub schema: serde_json::Value,  // JSON Schema (调 LLM 用)
}

// ----------------------------------------------------------------------------
// 公开 API 列表
// ----------------------------------------------------------------------------

/// 1:1 翻译 SpectrAI supervisorPrompt.ts:buildSupervisorPrompt
/// 估 350 LOC,818 行 markdown 完整保留
pub fn build_supervisor_prompt(
    cfg: &TeamLeadConfig,
    tools: &[ToolMeta],
    advisors: &[AdvisorMeta],
) -> String;

/// 1:1 翻译 SpectrAI supervisorPrompt.ts:buildAwarenessPrompt
/// 估 200 LOC,会话感知 prompt
pub fn build_awareness_prompt(
    session: &Session,
) -> String;

/// 14 个工具的 prompt 描述 (const, 编译期 hardcode)
/// 估 150 LOC,严格 1:1 翻译
pub const TOOL_DESCRIPTIONS: &[(&str, &str); 14] = &[
    ("spawn_agent",      "..."),
    ("send_to_agent",    "..."),
    ("get_output",       "..."),
    ("wait_idle",        "..."),
    ("wait",             "..."),
    ("get_status",       "..."),
    ("list",             "..."),
    ("cancel",           "..."),
    ("worktree_merge",   "..."),
    ("worktree_info",    "..."),
    ("worktree_check",   "..."),
    ("list_sessions",    "..."),
    ("get_summary",      "..."),
    ("search_sessions",  "..."),
];

/// 7 advisor voting 触发 trait (定义在 apeireth-protocol,这里 re-export)
pub use apeireth_protocol::team::AdvisorVotingTrigger;

/// Mock 实现 — 用于单测和 R19 阶段 3 早期 (council 还没 bridge)
pub fn noop_voting_trigger() -> Arc<dyn AdvisorVotingTrigger> {
    Arc::new(mocks::NoopVotingTrigger::new())
}

/// 构造 TeamLead 实例
impl TeamLead {
    pub fn new(
        cfg: TeamLeadConfig,
        voting: Arc<dyn AdvisorVotingTrigger>,
    ) -> Self;

    /// 异步构造 system prompt (含 voting 触发判断)
    pub async fn build_system_prompt(&self) -> String;
}
```

**API 风格守门** (按 APEIRETH-CONVENTIONS):
- ✅ 命名 snake_case (Rust 约定, ADR-0011 §翻译原则)
- ✅ async/await 标配 (tokio 生态)
- ✅ `Arc<dyn Trait>` 注入 (ADR-0012 §决策 2)
- ✅ `pub const TOOL_DESCRIPTIONS` 编译期 hardcode (不假装初始化)
- ✅ `thiserror` 错误类型 (不 panic)
- ❌ 不用 `unwrap()` (单测除外)
- ❌ 不用 `lazy_static` / `once_cell` (用 `const` / `tokio::sync::OnceCell`)

---

## §3 1:1 翻译 supervisorPrompt.ts 关键决策

### 3.1 翻译策略 (主哲学锚 S-2 17:43 实验室)

> **主人 2026-08-05 13:34 拍板**: 保守先, 1:1 翻译, 主人用 m3 测后迭代 prompt 敏感度。

| 维度 | SpectrAI (TypeScript) | Rust (1:1 翻译) | 备注 |
|---|---|---|---|
| **函数命名** | camelCase (`buildAwarenessPrompt`) | snake_case (`build_awareness_prompt`) | Rust 约定, 语义保持 |
| **字符串模板** | `\`...${var}...\`` 模板字符串 | `format!("...{}...", var)` 或 `write!()` | rust-coder 选 |
| **Markdown 格式** | 818 行 markdown 字符串 | 818 行 `&'static str` (用 `indoc!` 或 raw string) | 完整保留,不简化 |
| **XML 标签** | `<system>`, `<tools>`, `<advisors>` | 同上 (保留为 markdown 文本) | 不解析,纯字符串 |
| **中文 prompt** | 中文 markdown (e.g. "## 调度原则") | 1:1 保留中文 | minimax m3 认中文 |
| **prompt 工程细节** | few-shot / edge case 警告 | 1:1 保留 | 不"优化" |
| **Claude 措辞** | "You are a supervisor agent..." | 1:1 保留 (minimax m3 也认) | ADR-0011 §决策 2 |

### 3.2 关键函数翻译映射

| SpectrAI 函数 (supervisorPrompt.ts) | Rust 函数 (apeireth-team-lead) | LOC | 关键点 |
|---|---|---:|---|
| `buildAwarenessPrompt(): string` | `build_awareness_prompt(session: &Session) -> String` | 200 | 会话感知, 注入工作目录 + worktree 状态 |
| `buildSupervisorPrompt(providers: string[]): string` | `build_supervisor_prompt(cfg: &TeamLeadConfig, tools: &[ToolMeta], advisors: &[AdvisorMeta]) -> String` | 350 | 818 行 markdown 主体 |
| `injectAwarenessPrompt(workDir: string): string` | `inject_awareness_prompt(work_dir: &Path) -> String` | 30 | 文件读写 (写到 `spectrai-worktree.md` 同级) |
| `injectSupervisorPrompt(workDir, providers): void` | `inject_supervisor_prompt(work_dir: &Path, providers: &[String]) -> Result<(), Error>` | 30 | 同上, 含 7 advisor voting 触发 |
| `cleanupSupervisorPrompt(workDir): void` | `cleanup_supervisor_prompt(work_dir: &Path) -> Result<(), Error>` | 20 | 删除注入的 prompt 文件 |
| `buildWorkspaceSection(...)` | `build_workspace_section(ws: &Workspace) -> String` | 40 | 注入到 prompt 末尾 |
| `buildWorkspaceSessionSection(...)` | `build_workspace_session_section(ws: &Workspace, session: &Session) -> String` | 40 | 注入 session 元数据 |
| `injectWorkspaceSection(...)` | `inject_workspace_section(s: &mut String, ws: &Workspace)` | 20 | 字符串拼接 |
| `injectWorkspaceSessionSection(...)` | `inject_workspace_session_section(s: &mut String, ws: &Workspace, session: &Session)` | 20 | 同上 |
| `buildFileOpsPrompt(): string` | `build_file_ops_prompt() -> String` | 30 | 文件操作规范 prompt |
| `injectFileOpsRule(workDir): void` | `inject_file_ops_rule(work_dir: &Path) -> Result<(), Error>` | 20 | 注入到 .spectrai/ 目录 |

**src/ 文件对应**:

| src/ 文件 | 翻译自 | 含 | 估 LOC |
|---|---|---|---:|
| `lib.rs` | (主入口, 调下面各模块) | `TeamLead` struct + `TeamLeadConfig` + 公开 API 集合 | 100 |
| `prompt.rs` | `buildSupervisorPrompt` + `injectSupervisorPrompt` + `cleanupSupervisorPrompt` + `buildWorkspaceSection` + `buildWorkspaceSessionSection` + `injectWorkspaceSection` + `injectWorkspaceSessionSection` | 818 行 markdown 主体 + 7 advisor voting 注入 | 350 |
| `awareness.rs` | `buildAwarenessPrompt` + `injectAwarenessPrompt` + `buildFileOpsPrompt` + `injectFileOpsRule` | 会话感知 + 文件操作规范 | 200 |
| `tools.rs` | 14 工具 prompt 描述 (const TOOL_DESCRIPTIONS) | 1:1 翻译 | 150 |
| `voting.rs` | `AdvisorVotingTrigger` re-export + `should_trigger_vote` 触发判断 | trait 注入 | 50 |
| `mocks.rs` | `NoopVotingTrigger` (默认通过) | mock impl | 30 |

**合计 880 LOC** (src/ 部分)

### 3.3 14 个工具的 prompt 描述 (必须 1:1 翻译)

> SpectrAI 实际工具名 (supervisorPrompt.ts 内 14 个 `**tool_name**(...)` 块) → Rust 公开 API 命名 (用户习惯简写)。

| # | Rust 工具名 | SpectrAI 原名 | 关键参数 | prompt 描述骨架 (估 30-50 字) |
|---:|---|---|---|---|
| 1 | `spawn_agent` | `spawn_agent` | `name, prompt, workDir?, provider?, oneShot?` | 创建新 Agent 会话,返回 agentId; workDir 可指定 worktree 路径 |
| 2 | `send_to_agent` | `send_to_agent` | `agentId, message` | 给运行中的子 Agent 追加指令 (oneShot=false 时有意义) |
| 3 | `get_output` | `get_agent_output` | `agentId, lines?` | 获取子 Agent 最新终端输出 (去 ANSI, 默认 50 行) |
| 4 | `wait_idle` | `wait_agent_idle` | `agentId, timeout?` | 等待子 Agent 完成当前任务变空闲 (oneShot=true 时 Agent 不自动退) |
| 5 | `wait` | `wait_agent` | `agentId, timeout?` | 等待子 Agent 完全退出,获取最终结果 |
| 6 | `get_status` | `get_agent_status` | `agentId` | 查询子 Agent 状态 (running / idle / exited) |
| 7 | `list` | `list_agents` | `()` | 查看所有运行中的子 Agent |
| 8 | `cancel` | `cancel_agent` | `agentId` | 终止子会话 |
| 9 | `worktree_merge` | `merge_worktree` | `taskId, squash?, message?, cleanup?` | 合并 worktree 分支回主分支 |
| 10 | `worktree_info` | `get_task_info` | `taskId` | 查询任务是否启用了 worktree (worktreeEnabled 字段) |
| 11 | `worktree_check` | `check_merge` | `taskId` | 检查分支能否干净合并 (无冲突检测) |
| 12 | `list_sessions` | `list_sessions` | `status?, limit?` | 查看所有会话 (名称/状态/工作目录) |
| 13 | `get_summary` | `get_session_summary` | `sessionId?, sessionName?` | 获取某会话的 AI 答复、修改的文件、执行的动作 |
| 14 | `search_sessions` | `search_sessions` | `query, limit?` | 按关键字搜索所有会话的活动记录 |

**翻译约束** (rust-coder 必读):
- ✅ **严格 1:1 翻译**: SpectrAI 原文措辞保留 (含中文 markdown 段落)
- ✅ **保留所有参数签名**: (agentId, timeout?) 这种可选参数标注
- ✅ **保留 few-shot examples**: SpectrAI 原文的 3-5 行例子完整保留
- ✅ **保留 edge case 警告**: "**⚠️ 注意**" / "**重要**" 标记完整保留
- ❌ **不简化**: 818 行 markdown 完整保留, 不删行
- ❌ **不"优化"**: 不按 minimax 习惯改写
- ❌ **不删注释**: 中文注释 ("# 调度原则" "## Worktree 合并流程") 全保留

### 3.4 supervisorPrompt.ts 行数核对 (不假装)

> **主哲学锚 S-2 17:43 实事求是**: 实际 LOC vs ADR 估值可能有差异, rust-coder 实施时按 `wc -l` 重测。

| 来源 | LOC | 备注 |
|---|---:|---|
| SpectrAI 原文 (`supervisorPrompt.ts`) | **970** | 实际 `wc -l` (含注释 + 空行) |
| ADR-0011 §决策 2 估值 | 808 | 估 LOC (可能不含部分空行/注释) |
| 蓝图 §B.2 标注 | 808 | 沿用 ADR |
| "818 行 markdown" (蓝图附录 B) | 818 | 818 是 markdown 主体行数, 跟 970 差 152 是 TS 注释/空行 |

**rust-coder 实施时**:
1. 用 `wc -l` 重测 `supervisorPrompt.ts` 实际行数
2. 跟 `prompt.rs` 最终行数对比, 偏差 > 20% 写 `[主 S-2 17:43]` 不假装备注
3. 任何"优化" / "简化" 都要在 commit message 写 "⚠️ 偏离 1:1 翻译: <原因>"

---

## §4 7 advisor voting 触发 (trait 注入)

> 详细决策见 `docs/adr/0012-team-lead-council-collaboration.md`, 本节给 rust-coder 实施步骤。

### 4.1 5 步实施 (按 R19 阶段 3 排)

| 步 | 谁干 | 干到哪 | 估 LOC | 依赖 |
|---:|---|---|---:|---|
| **1** | architect2 | `apeireth-protocol/src/team.rs` 定义 `AdvisorVotingTrigger` trait | 80 | 无 |
| **2** | integration | `apeireth-team-lead/src/lib.rs` 构造函数接受 `voting: Arc<dyn AdvisorVotingTrigger>` | 20 | 步 1 |
| **3** | code_reviewer | `apeireth-team-lead/src/mocks.rs` 实施 `NoopVotingTrigger` | 30 | 步 2 |
| **4** | integration | `apeireth-council/src/bridge.rs` 实施 `CouncilVotingTrigger` (mock LLM) | 100 | 步 1, 等 council 实装 |
| **5** | agent-orchestrator | 集成测试: trait + 2 实现 + 端到端 | 100 | 步 1-4 |

### 4.2 触发条件 (主哲学锚 O-3 23:44 决策清单)

> **决策**: team-lead 构造 prompt 时, 如果 action.criticality >= 0.8, 调 `voting.should_trigger_vote(action)`。

**触发逻辑伪代码** (放在 `apeireth-team-lead/src/voting.rs`):

```rust
// ============================================================================
// 7 advisor voting 触发逻辑 (伪代码, 仅展示判定流程, 不写实现)
// ============================================================================

/// 判定当前 action 是否需要触发 council voting
pub async fn maybe_trigger_voting(
    action: &TeamAction,
    voting: &Arc<dyn AdvisorVotingTrigger>,
    threshold: f32,
) -> Option<VoteOutcome> {
    // 1. criticality 检查
    if action.criticality < threshold {
        return None;  // 不触发
    }

    // 2. 调 trait 方法判定
    if !voting.should_trigger_vote(action) {
        return None;
    }

    // 3. 异步请求投票
    let outcome = voting.request_vote(action.clone()).await;

    // 4. 加权 synthesis 返回
    Some(outcome)
}
```

**风险等级映射** (按 APEIRETH-CONVENTIONS §11 + ADR-0005):

| RiskGrade | 席位数 (按 ADR-0005) | 触发阈值 | team-lead 行为 |
|---|---|---:|---|
| `critical` | 7 席 | 0.95 | 强触发, 必走 council |
| `high` | 5 席 | 0.80 | 触发, 默认调用 |
| `medium` | 3 席 | 0.50 | 可选, 按场景 |
| `low` | 1 席 | 0.20 | 默认不触发, 可手动开 |
| `info` | 0 席 | 0.00 | 不触发 |

**默认阈值** (`TeamLeadConfig.voting_threshold`): `0.80` (high 风险起)。

### 4.3 集成位置 (按 ADR-0012 §实施路径 第 4 步)

> `apeireth-team-lead` 持 `Arc<dyn AdvisorVotingTrigger>`, 由 bootstrap 层注入具体实现。

**bootstrap 伪代码** (放在 `apeireth-bootstrap/src/team.rs`, 等 R19 阶段 3.4 实施):

```rust
// ============================================================================
// bootstrap 注入 (伪代码, 不在 apeireth-team-lead 实施范围)
// ============================================================================

// 1. 构造 voting trigger
let voting: Arc<dyn AdvisorVotingTrigger> = if cfg.use_real_council {
    Arc::new(apeireth_council::bridge::CouncilVotingTrigger::new(council))
} else {
    apeireth_team_lead::noop_voting_trigger()  // 默认 Noop
};

// 2. 注入到 team-lead
let team_lead = TeamLead::new(cfg, voting);

// 3. 注册到 agent manager
agent_manager.register("team_lead", Arc::new(team_lead));
```

**强约束**:
- ❌ `apeireth-team-lead` **不** `use apeireth_council::*` (避免循环依赖)
- ❌ `apeireth-team-lead` **不** 知道 council 是否存在
- ✅ team-lead 跟 council 解耦, 各干各的, 编译时无依赖

---

## §5 跟其他 crate 集成点

### 5.1 跟 `apeireth-protocol`

**用**:
- `ProviderEvent` — 订阅 agent 事件 (e.g. idle / output / exited)
- `AdapterSessionConfig` — 构造 session
- `AdvisorVotingTrigger` trait (决策 §4)
- `TeamAction` / `VoteOutcome` / `AdvisorVote` struct (决策 §4)

**不依赖**:
- ❌ 不调 `apeireth-protocol` 内部细节 (e.g. 协议路由), 只用公开 API

### 5.2 跟 `apeireth-agent`

**用**:
- `Agent::system_prompt` 字段 — team-lead 构造的 prompt 写到这里
- `AgentManager::register` — 注册 leader 角色
- `AgentManager::list` — 查询所有 agent (含 leader)

**不调**:
- ❌ `AgentManager::send_to_agent` (有 mid-task bug #2, 等 `apeireth-mcp::team` 修, 蓝图 §B.3)
- ❌ `AgentManager::cancel` 内部细节 (走 `cancel_agent` 工具)

### 5.3 跟 `apeireth-mcp`

**用** (等 R19 阶段 3.4 实施):
- 14 个工具的 trait 定义 (放在 `apeireth-mcp::team`, 等 ADR-0010 实施)
- `apeireth-mcp::team::team_message` trait 方法 (R19 实施)

**不调**:
- ❌ 不直接调 MCP 传输 (mcp 团队负责)
- ❌ 不写 `rmcp::Server` 内部细节 (调 mcp::team 公开 API)

### 5.4 跟 `apeireth-council`

**不直接调** (强约束, ADR-0012 §决策 2):
- ❌ 不 `use apeireth_council::*`
- ❌ 不调 `council.vote()` 直接投票
- ✅ 通过 `Arc<dyn AdvisorVotingTrigger>` 注入
- R19 阶段 3 用 `NoopVotingTrigger` (默认通过)
- R21+ P2 用 `CouncilVotingTrigger` (真实 LLM)

### 5.5 集成矩阵 (一键查表)

| crate | 用什么 | 不用什么 | 集成方式 |
|---|---|---|---|
| `apeireth-protocol` | `ProviderEvent` / `AdapterSessionConfig` / `AdvisorVotingTrigger` trait | 协议路由内部 | `use` (依赖) |
| `apeireth-agent` | `Agent.system_prompt` / `AgentManager.register` | `send_to_agent` (mid-task bug) | `use` (依赖) |
| `apeireth-mcp` | 14 工具 trait 定义 (`mcp::team`) | MCP 传输内部 | `use` (依赖, R19 阶段 3.4) |
| `apeireth-council` | (不直接调) | ❌ 全部 | `Arc<dyn Trait>` 注入 (不依赖) |
| `apeireth-supervisor` | (不调) | ❌ 全部 | **不依赖** (强约束) |

---

## §6 验收标准 (R-Measure baseline 守门)

> **主哲学锚 O-5 17:58 不假装**: 实施完**必须**跑以下 3 项, 全部通过才能算 R19 阶段 3.4 收尾。

### 6.1 R-Measure baseline 守门 (按 APEIRETH-CONVENTIONS §11)

| 指标 | baseline 值 | 实施后必须 |
|---|---:|---|
| **V1141-R11** | 0.8682 | ≥ 0.8682 (不能掉) |
| **V1131-R11** | 0.8532 | ≥ 0.8532 (不能掉) |
| **V1136-R11** | 0.9063 | ≥ 0.9063 (不能掉) |

**回归判定** (按 ADR-0010 + ADR-0011 §后果):
- 任一指标 < baseline → 立即回滚 + 走 R-Measure 复测
- 集成 `apeireth-team-lead` 不能掉 baseline (APEIRETH-CONVENTIONS §11)
- 集成 `apeireth-mcp::team` 14 工具不能掉 baseline
- 修 SpectrAI mid-task bug 3 处不能掉 baseline

### 6.2 必跑命令 (cargo + bench)

```bash
# 1. 单元测试 (估 30+20+14+5 = 69 tests)
cargo test -p apeireth-team-lead

# 2. 编译检查 (lints 继承 workspace)
cargo check -p apeireth-team-lead --all-targets

# 3. clippy (按 APEIRETH-CONVENTIONS §0.1)
cargo clippy -p apeireth-team-lead --all-targets -- -D warnings

# 4. benchmarks (等 R19 阶段 3.5 加)
cargo bench -p apeireth-team-lead

# 5. R-Measure 验证脚本 (独立 crate, 不在 M 标记)
cargo run -p apeireth-rmeasure -- --baseline r11
```

**通过标准**:
- ✅ `cargo test` 全过 (含 happy + edge + 集成)
- ✅ `cargo clippy` 0 warning (严格)
- ✅ `cargo bench` baseline 性能不回归
- ✅ R-Measure 3 值 ≥ baseline (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)

### 6.3 单元测试覆盖 (估 69 tests)

| 文件 | 测试数 | 覆盖 |
|---|---:|---|
| `prompt_tests.rs` | 30 | `build_supervisor_prompt` 30 种 case (含 persona / tools / advisors / empty) |
| `awareness_tests.rs` | 20 | `build_awareness_prompt` 20 种 case (含 session / worktree / provider) |
| `tools_tests.rs` | 14 | 14 工具各 1 happy path (验证 TOOL_DESCRIPTIONS 非空) |
| `voting_tests.rs` | 5 | trait + Noop + 触发条件 + criticality 边界 + async |
| **合计** | **69** | — |

**测试守门**:
- ✅ 每工具 1 happy path (14)
- ✅ persona / tools / advisors / empty 4 维度组合 (4×3×3 = 36, 取 30)
- ✅ criticality 边界 0.79 / 0.80 / 0.81 (3)
- ✅ async 投票 (2)

### 6.4 集成测试 (估 5 tests)

| 测试 | 验证 |
|---|---|
| `team_lead_with_noop_voting` | team-lead + NoopVotingTrigger 端到端 |
| `team_lead_with_council_voting` | team-lead + CouncilVotingTrigger 端到端 (mock LLM) |
| `supervisor_prompt_1to1_translation` | 818 行 markdown 跟 SpectrAI 原文 diff = 0 (主 S-2 实验室) |
| `voting_threshold_default` | 默认 0.80, criticality 0.79 不触发, 0.81 触发 |
| `worktree_tools_in_prompt` | 14 工具描述全部出现在 build_supervisor_prompt 输出 |

### 6.5 SpectrAI 原文 1:1 翻译验证 (主 S-2 17:43 实验室)

> **强验证**: `build_supervisor_prompt` 输出跟 `supervisorPrompt.ts:buildSupervisorPrompt` 输出**逐字符对比**。

**做法** (实施时):
1. 用 `supervisorPrompt.ts` 在 Node.js 跑一次, 输出 818 行 markdown 到 `tests/fixtures/spectrAI_supervisor_prompt.txt`
2. 用 `apeireth-team-lead` 跑 `build_supervisor_prompt` 输出到 `tests/fixtures/rust_supervisor_prompt.txt`
3. `diff` 两个文件 → 必须 `0` (无差异)
4. 任何"优化" / "简化" 都要在 commit message 写 "⚠️ 偏离 1:1 翻译: <原因>" (主 O-5 17:58 不假装)

**为什么**: minimax m3 对 prompt 敏感度未知, 任何字符差异都可能导致行为偏差。1:1 翻译对照可量化"我们到底翻译得多准"。

---

## §7 风险清单 (主哲学锚 O-5 17:58 不假装)

> 5 项关键风险, 实施时**必读** + 写 commit message 标注应对。

| # | 风险 | 等级 | 应对 | 监测 |
|---:|---|---|---|---|
| **1** | **prompt 翻译后 minimax m3 行为可能跟 Claude 不一样** | 🟡 中 | 主人 2026-08-05 13:34 拍板保守先, 1:1 翻译, 主人 m3 实测后迭代 prompt 敏感度 | §6.5 逐字符 diff + 主人 R-Measure 实测 |
| **2** | **7 advisor voting trait 抽象可能太宽或太窄** | 🟡 中 | ADR-0012 §中和 "trait 抽象先小后大" — 第 1 版只 2 个方法 (`should_trigger_vote` + `request_vote`), 后续按需加 | 集成测试覆盖 mock + bridge + 真实 3 路径 |
| **3** | **14 个工具的 prompt 描述可能有隐藏语义** | 🟡 中 | 严格 1:1 翻译 (含 few-shot examples + edge case 警告), §6.5 逐字符 diff | tests/tools_tests.rs 14 happy path + supervisor_prompt 集成测试 |
| **4** | **跟 `apeireth-mcp::team` 集成 (R19 阶段 3.4)** | 🟡 中 | mcp 团队按 ADR-0010 实施 14 工具 trait, team-lead 等 trait 完工后再调 | 阶段 3.4 实施时跟 mcp 团队对 trait 签名 |
| **5** | **supervisorPrompt 970 行 markdown 翻译的工作量** | 🟢 低 | §8 实施时间表 3.3 阶段估 1 天, 实际可能要 2 天 (4-6 小时含调试) | code_reviewer 实施时 daily 报告 LOC 进度 |
| **6** | **cargo count 跟 ADR 估值偏差 > 20%** | 🟢 低 | §3.4 supervisorPrompt.ts 行数核对, 偏差写 `[主 S-2 17:43]` 不假装备注 | §3.4 wc -l 重测 |
| **7** | **跟 `apeireth-supervisor` 命名混淆** | 🟢 低 | §1.2 4 分类清晰, README.md 强调 "team-lead ≠ supervisor ≠ council ≠ agent" | §2.1 README 必加 "命名空间区分" 段 |
| **8** | **`NoopVotingTrigger` 兜底可能掩盖 bug** | 🟢 低 | ADR-0012 §中和 "NoopVotingTrigger 加 warning log" — 启动时打 "WARN: using NoopVotingTrigger" | tests/voting_tests.rs 验证 warning log |

**风险守门**:
- 任一 🟡 中风险发生 → 写 commit message 标注 + 通知 Mavis 复核
- 5 项 🟡 风险全过 → R19 阶段 3.4 收尾

---

## §8 实施时间表 (8 阶段, 估 6 天)

> **主哲学锚 O-3 23:44 干到底**: 8 阶段实施, 每阶段明确 owner + 时长 + 交付物。

| 阶段 | 时长 | 任务 | Owner | 交付物 | 依赖 |
|---|---:|---|---|---|---|
| **3.1** | 0.5 天 | 等 code_reviewer 改完 Cargo.toml + 加 workspace member | code_reviewer / Mavis | Cargo.toml 含 `apeireth-team-lead` workspace member | 本文档 + ADR-0011 |
| **3.2** | 1 天 | 创建目录结构 + Cargo.toml + README + 公开 API 骨架 | rust-coder | `crates/apeireth-team-lead/` 目录 + 6 个 src/ 文件空壳 + README.md (含 §1.2 4 分类段) | 3.1 |
| **3.3** | 1 天 | 1:1 翻译 `supervisorPrompt.ts` (970 LOC) 到 `prompt.rs` (350 LOC) + `awareness.rs` (200 LOC) | rust-coder | `prompt.rs` + `awareness.rs` 实装 + §6.5 1:1 diff 通过 | 3.2 + Node.js 环境 |
| **3.4** | 0.5 天 | 14 工具 prompt 描述到 `tools.rs` (150 LOC) | rust-coder | `tools.rs` + `pub const TOOL_DESCRIPTIONS: &[(&str, &str); 14]` | 3.3 |
| **3.5** | 0.5 天 | `AdvisorVotingTrigger` trait 定义 (`apeireth-protocol/src/team.rs` 80 LOC) + `NoopVotingTrigger` (`apeireth-team-lead/src/mocks.rs` 30 LOC) + `voting.rs` 50 LOC | architect2 + code_reviewer | trait + mock + voting 触发逻辑 | 3.2 + ADR-0012 |
| **3.6** | 1 天 | 69 单元测试 + 5 集成测试 | rust-coder | `tests/` 4 文件 + `tests/fixtures/` 2 fixture + `cargo test` 全过 | 3.3 + 3.4 + 3.5 |
| **3.7** | 0.5 天 | R-Measure baseline 验证 (3 值不能掉) | Mavis | `cargo run -p apeireth-rmeasure -- --baseline r11` + 报告 | 3.6 + §6.1 baseline |
| **3.8** | 1 天 | 主人用 minimax m3 实测 + prompt 迭代 | 主人 | 实测报告 + prompt 改动 commit (如有) | 3.7 |
| **总计** | **6 天** | (1 周) | — | R19 阶段 3.4 收尾 | — |

### 关键里程碑 (M1-M4)

| 里程碑 | 阶段 | 解锁 |
|---|---|---|
| **M1** | 3.1 | Cargo.toml 完工 → 解锁 3.2 创建目录 |
| **M2** | 3.3 | prompt 核心翻译完成 → 解锁 3.4 工具描述 |
| **M3** | 3.6 | 69 单元测试 + 5 集成测试全过 → 解锁 3.7 R-Measure |
| **M4** | 3.7 | R-Measure baseline 不掉 → 解锁 3.8 主人 m3 实测 |

### 并行机会 (O-3 干到底)

- 3.2 (创建骨架) 跟 3.5 (trait 定义) 可**并行** (不同 crate, 互不依赖)
- 3.3 (prompt 翻译) 跟 3.4 (工具描述) 可**并行** (不同文件)
- 3.6 (测试) 必在 3.3+3.4+3.5 之后

**建议调度** (压缩到 4 天):
- Day 1: 3.1 + 3.2 (骨架) + 3.5 start (trait)
- Day 2: 3.3 (prompt) + 3.4 (tools) + 3.5 finish (Noop)
- Day 3: 3.6 (测试 74 个) + 3.7 start (R-Measure)
- Day 4: 3.7 finish + 3.8 (主人实测)

---

## §9 不修改承诺 (11 项 LOCKED, 跟 ADR-0011 §不修改承诺 一致)

> **主哲学锚 O-4 00:56 12 统一**: 跟现有 12 子规范统一。

| ❌ 不修改 | 原因 | 引用 |
|---|---|---|
| 阶段 1+2+3 LOCKED 文档 | 主人明确沉淀 | APEIRETH-CONVENTIONS §11 |
| v2 / v4 / v4.1 LOCKED | 哲学层纲领 | APEIRETH-CONVENTIONS §10 |
| 阶段 4 核心文档 LOCKED (`6ca80776`) | 蓝图 §10 已锁 | 蓝图 §10 |
| 阶段 5 施工文档 LOCKED (631 行) | 阶段 5 实施时再引用 | 蓝图 §10 |
| v6 基础架构 | 主 AI 团队已 LOCKED | APEIRETH-CONVENTIONS §10 |
| **R11 baseline (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)** | 主人 2026-07-31 明确不动 | APEIRETH-CONVENTIONS §11 |
| APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md | 顶层规范 | APEIRETH-CONVENTIONS §0 |
| START-CONSTRUCTION.md | 顶层手册 | APEIRETH-CONVENTIONS §0 |
| `apeireth-legacy/` | R17 finalize 后归档, 不删 | APEIRETH-CONVENTIONS §10 |
| workspace version 1.0.0 (semver 严格) | 不动 | APEIRETH-CONVENTIONS §8 |
| 现有 ADR 0001~0012 | 不动 | (本 ADR 阶段只新增 0011/0012) |
| 现有 stage4-* 文档 + README.md + CHANGELOG.md | 不动 | (本指南是新增) |
| `apeireth-supervisor` 现有 550 LOC | 本 ADR 阶段零修改 (强约束 §1.3) | ADR-0011 §决策 4 |
| `apeireth-mcp` 现有 2135 LOC | 本 ADR 阶段零修改 (强约束 §1.4) | ADR-0011 §决策 3 |
| 现有 M 标记文件 (cargo/code_reviewer 改 Cargo.toml 期间) | 等 code_reviewer 完工, 期间不碰 | (本指南 §0 约束) |

---

## §10 哲学 anchor (6 项穿透, 按 APEIRETH-CONVENTIONS §9)

| 锚 | 来源 | 穿透点 |
|---|---|---|
| **S-1** | 22:33 | 6 anchor ASI 完整性 — team-lead 是团队核心, 14 工具 prompt + 7 advisor voting 覆盖 team 协作全场景, 服务 ASI 北极星 |
| **S-2** | 17:43 | 6 anchor 实验室 — 1:1 翻译 supervisorPrompt.ts 970 行, 跟 SpectrAI 原文对照可测 (§6.5 逐字符 diff), 实验室态度 |
| **O-5** | 17:58 | 6 anchor 12 急救 — supervisorPrompt 包含 P0 mid-task bug 修法的 prompt 描述 (父进程不再"以为成功实际丢消息"), 7 不假装标注 |
| **O-2** | 19:33 | 6 anchor 4 分类 — team-lead ≠ supervisor (进程) ≠ council (审议) ≠ agent (单数), 4 分类清晰 (§1.2) |
| **O-3** | 23:44 | 6 anchor 决策清单 — 8 阶段实施 + 明确 owner + 5 决策 (ADR-0011) + 4 决策 (ADR-0012) + M1-M4 4 里程碑 |
| **O-4** | 00:56 | 6 anchor 12 统一 — 跟现有 12 子规范统一 (`apeireth-supervisor` 监督员, `apeireth-team-lead` 团队 leader, 同构命名) |

**自检清单** (rust-coder 提交 commit 前必跑):
- [ ] S-1 完整性: 14 工具 prompt 全部实装 (含 awareness + 调度 + worktree)
- [ ] S-2 实验室: §6.5 1:1 diff 通过
- [ ] O-5 不假装: 8 风险全部应对, §6.1 baseline 守住
- [ ] O-2 4 分类: README.md 含 "team-lead ≠ supervisor ≠ council ≠ agent" 段
- [ ] O-3 决策清单: commit message 含 5 决策 + 4 决策 + 8 阶段实施回执
- [ ] O-4 12 统一: 不修改承诺 15 项全守, Document-Meta 严格按 APEIRETH-CONVENTIONS §0

---

## §11 关联文档 (一键查)

### 11.1 上游 (决策来源)

| 文档 | 必读章节 | 原因 |
|---|---|---|
| `docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md` | §决策 1-5 + §实施路径 + §不修改承诺 | 本指南主要依据 |
| `docs/adr/0012-team-lead-council-collaboration.md` | §决策 1-4 + §实施路径 | trait 注入详细方案 |
| `docs/adr/0010-mcp-from-spectrai-agentmcpserver.md` | §决策 (apeireth-mcp 翻译路径) | 14 工具 trait 来源 |
| `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` | §5.2 第 7 行 + §8 决策清单 #3 + §9.2 阶段 3 路线 | 蓝图主文档 |
| `docs/stage4/glossary-spectrAI-additions-2026-08-05.md` | 词条 2 (team-lead) + 词条 3 (council) + 词条 4 (supervisor) | 4 分类清晰 |
| `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md` | §3 13 项资产清单 | 知道哪些给 Tauri 不用做 |

### 11.2 平行 (本期工程文档)

| 文档 | 必读章节 | 原因 |
|---|---|---|
| `APEIRETH-CONVENTIONS.md` | §0 Document-Meta + §8 状态标记 + §9 6 锚穿透 + §11 R-Measure baseline | 顶层规范 |
| `ARCHITECTURE.md` | §5.2 集成映射表 (team-lead 行待补) | 集成位置 |
| `docs/adr/0003-trait-interlock-22-enum.md` | 全 | trait 抽象风格统一 |
| `docs/adr/0007-compat-components-layer.md` | 全 | 兼容组件层 |
| `docs/adr/0009-integration-rebase-skip-policy.md` | 全 | rebase 策略 |
| `docs/adr/0005-risk-grade-m1-m12-thresholds.md` | 全 | 风险等级 → 席位数 映射 |

### 11.3 下游 (后续阶段引用)

| 文档 | 引用位置 | 后续阶段 |
|---|---|---|
| `apeireth-council` 7 advisor voting 真实 LLM 接入 | ADR-0012 §实施路径 | R21+ P2 |
| Tauri 阶段复用 `apeireth-team-lead` | 蓝图 §9.2 | R20+ |
| GLOSSARY.md 8 词条合并 | glossary-spectrAI-additions §拍板记录 | R19 阶段 1 启动前 |

### 11.4 源码引用 (1:1 翻译来源)

| 路径 | 用途 |
|---|---|
| `.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\supervisorPrompt.ts` | 970 LOC 1:1 翻译主源 (含 818 行 markdown) |
| `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-protocol\src\team.rs` | `AdvisorVotingTrigger` trait 定义位置 (待 R19 阶段 3.5 创建) |
| `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-team-lead\src\*.rs` | 本指南实施后产物 (估 880 LOC) |

---

## 附录 A: 实施检查清单 (rust-coder 提交 commit 前必跑)

> **主哲学锚 O-3 23:44 干到底**: 18 项检查, 全部 ✅ 才能 commit。

### A.1 必跑命令 (6 项)

- [ ] `cargo test -p apeireth-team-lead` — 全过 (69 unit + 5 integration)
- [ ] `cargo check -p apeireth-team-lead --all-targets` — 0 错
- [ ] `cargo clippy -p apeireth-team-lead --all-targets -- -D warnings` — 0 warning
- [ ] `cargo bench -p apeireth-team-lead` — baseline 性能不回归
- [ ] `cargo run -p apeireth-rmeasure -- --baseline r11` — V1141 ≥ 0.8682 / V1131 ≥ 0.8532 / V1136 ≥ 0.9063
- [ ] `wc -l crates/apeireth-team-lead/src/*.rs` — 偏差 > 20% 写 `[主 S-2 17:43]` 备注

### A.2 必查文件 (8 项)

- [ ] `Cargo.toml` — 7 依赖正确, **不依赖** apeireth-supervisor / apeireth-council
- [ ] `README.md` — §1.2 4 分类段必含
- [ ] `src/lib.rs` — 公开 API 跟 §2.3 一致
- [ ] `src/prompt.rs` — 818 行 markdown 1:1 翻译 (估 350 LOC)
- [ ] `src/awareness.rs` — 1:1 翻译 (估 200 LOC)
- [ ] `src/tools.rs` — `pub const TOOL_DESCRIPTIONS: &[(&str, &str); 14]`
- [ ] `src/voting.rs` — `maybe_trigger_voting` 伪代码实装
- [ ] `src/mocks.rs` — `NoopVotingTrigger` + 启动 warning log

### A.3 必查哲学 (6 锚)

- [ ] S-1 完整性
- [ ] S-2 实验室 (§6.5 1:1 diff 通过)
- [ ] O-5 不假装 (8 风险全部应对)
- [ ] O-2 4 分类 (README 段)
- [ ] O-3 决策清单 (commit message 含 5+4 决策 + 8 阶段回执)
- [ ] O-4 12 统一 (15 项不修改承诺全守)

### A.4 必查依赖

- [ ] 7 依赖正确: `apeireth-protocol` / `apeireth-agent` / `apeireth-mcp` / `serde` / `serde_json` / `tokio` / `tracing` / `thiserror` / `async-trait`
- [ ] **不依赖** `apeireth-supervisor` (强约束 §1.3)
- [ ] **不依赖** `apeireth-council` (强约束 §1.4)
- [ ] lints 继承 workspace (等 code_reviewer 完工)

---

## 附录 B: 拍板记录 (时序)

| 时间 | 决策 | 影响 |
|---|---|---|
| 2026-08-05 13:34 | 主人拍板 A 方案 `apeireth-team-lead` (ADR-0011) | 解锁本指南 |
| 2026-08-05 13:40 | Mavis 起草 ADR-0012 (team-lead 跟 council 协同) | trait 注入决策 |
| 2026-08-05 13:42 | Mavis 起草 8 词条 glossary 草稿 | §11.3 后续 GLOSSARY 合并 |
| 2026-08-05 14:00 | Mavis 起草本实施指南 (本文档) | rust-coder 接手 |
| 待 code_reviewer 完工 | 改 Cargo.toml 加 workspace member | 解锁 3.2 创建目录 |
| 待 rust-coder 接手 | 6 天实施 8 阶段 | R19 阶段 3.4 收尾 |
| 待 leader 复核 | §6.5 1:1 diff + §6.1 R-Measure | 实施完必跑 |
| 待 主人 m3 实测 | prompt 敏感度迭代 | R19 阶段 3.5 起 |

---

_本指南由 Mavis (technical_writer 角色) 起草, 主人 2026-08-05 14:00 拍板交付给 rust-coder 接手._

_§1-§11 全齐: 战略背景 / 骨架设计 / 1:1 翻译 / trait 注入 / 集成点 / 验收标准 / 风险清单 / 实施时间表 / 不修改承诺 / 哲学 anchor / 关联文档._
_§6 R-Measure baseline 三值守门 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)._
_§8 实施时间表 8 阶段 + 4 里程碑 + 并行机会 + 压缩调度建议._
_附录 A 18 项实施检查清单, rust-coder 提交 commit 前必跑._
_主哲学 6 锚穿透. 任何接手者能查. 不写实际 Rust 代码._
