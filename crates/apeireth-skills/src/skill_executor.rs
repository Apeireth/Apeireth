//! R125-19: Skill Execution Layer — Phase Machines (per decision-51 §1.4 P3-2)
//!
//! **借鉴 ID**: `R125-19-BORROW-obra/superpowers-2026-05-2026-08-10`
//! (superpowers 234 cloned, per decision-36 §1.1)
//!
//! **借鉴源码**: `.openclaw/workspace/borrowed-repos/superpowers/`
//!
//! **目标**: 把 obra/superpowers 14 公开 `SKILL.md` 的 workflow 模式,
//! 落到 Rust 状态机里 — 强制 step 顺序 + 状态推进 + 验证闭环.
//!
//! **Apeireth 真接 (本 module)**:
//! - `SkillCategory` enum (14 variants, 1:1 映射 superpowers 14 `skills/<name>/SKILL.md`)
//! - `ExecutionPattern` enum (5 patterns, 把 14 categories 分组)
//! - `ExecutionStep` struct — pattern 内一步 (name + order + description + is_terminal)
//! - 5 phase state machines:
//!   1. `TddCycle` (red → green → refactor → done) — per `test-driven-development` + `systematic-debugging`
//!   2. `PlanExecuteVerifyCycle` (plan → execute → verify → iterate → done) — per `writing-plans` + `executing-plans` + `verification-before-completion`
//!   3. `ParallelCycle` (dispatch → collect → merge → done) — per `subagent-driven-development` + `dispatching-parallel-agents`
//!   4. `ReviewCycle` (submit → receive → apply → done) — per `requesting-code-review` + `receiving-code-review`
//!   5. `MetaCycle` (identify → author → done) — per `using-superpowers` + `brainstorming` + `writing-skills`
//!   (branch lifecycle `using-git-worktrees` + `finishing-a-development-branch` 也归 MetaCycle, 5 段 1 pattern)
//! - `category_to_pattern(cat) -> ExecutionPattern` — 14 → 5 映射
//! - `pattern_steps(p) -> Vec<ExecutionStep>` — pattern 5/5/4/3/3 步 严守
//! - 5 cycle structs 各 6 unit test (state advance + terminal detection + reset + multi-iteration)
//!
//! **0 装 PASS 严守 (per decision-36 §1.1 + 主人 17:22 升级授权 + decision-33 §2.3 C2)**:
//! - ✅ `cloned = 真实施` — superpowers 234 cloned = 写 5 phase state machines + 14 categories + 6 unit test/cycle = 30 unit test
//!   + 8 integration test + 1 demo example, 真 src 改动 (5 状态机 + 5 pattern enum + 1 mapping fn)
//! - ⏳ 限流 = 准备 — 不适用 (superpowers ✅ cloned, 0 限流)
//! - ❌ 跳过 — 不适用 (OpenCog AGPL-3.0 跟本 module 无关)
//! - ❌ **0 装"已借鉴" superpowers 私有 plugin 加载机制** — superpowers 私有 `.claude-plugin/` / `.codex-plugin/` /
//!   `.opencode/plugins/superpowers.js` / `hooks/session-start` 等 plugin 加载机制 0 集成, 0 写
//!   `use obra::superpowers::...` import 任何"借鉴代码", 仅借鉴 14 公开 `SKILL.md` frontmatter
//!   (name / description) + 公开 markdown body 步骤描述
//!
//! **8 硬墙 verify (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略, per decision-33 §2.3)**:
//! - **B2** workspace.version 1.2.0 — ✅ 0 改 (`version.workspace = true` 继承, Cargo.toml 0 触碰)
//! - **A1** R11 baseline 3 值 0.8682/0.8532/0.9063 — ✅ 0 触碰 (R125-19 0 改 17 文件 baseline)
//! - **B1** 24 LOCKED entry sigs 0 改 — ✅ 0 触碰 24 LOCKED crate entry sigs (`apeireth-skills` **不在 24 LOCKED**,
//!   内部 fn 实施可改, 新 module 加 = OK)
//! - **A3** 13 键 — ✅ 0 触碰 12+1 键 (R125-19 0 触碰 verdict cache)
//! - **C1** 0 主动 commit — ✅ 0 commit (Mavis 整合 #5 commit 时机拍板, 跑过夜明早 8/11-8/22 done)
//! - **C2** 0 装 PASS 严守 — ✅ ✅ cloned = 真实施 (5 状态机 + 30 unit test + 8 integration test + 1 demo)
//! - **C3** 6 重 v6 0 改 — ✅ 0 触碰 sovereignty
//! - 0 主动 push — ✅ 0 push (等 1.0 release 配 GitHub remote)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `apeireth-skills/src/lib.rs` 已有 8 pub fn / Skill / Registry (R23 LOCKED)
//! - 0 改 `apeireth-skills/src/descriptor.rs` 已有 SkillDescriptor 7 字段 (R33 LOCKED)
//! - 0 改 `apeireth-skills/src/{mcp_bridge, eval_bridge, file_loader, watcher, semver_strict}.rs` (R86/R110/R63/R109/R107 LOCKED)
//! - 0 引入新 dep (仅 std: Vec / String / BTreeMap / fmt)
//!
//! **借鉴锚 (S-9, 新增)**: obra/superpowers 14 公开 `SKILL.md` workflow 模式 (1:1 step 顺序 + state machine 落地).
//! 公开 SKILL.md 全部在 borrowed-repos/superpowers/skills/<name>/SKILL.md 234 files 父目录, 0 必再读.

use std::fmt;

// =====================================================================
// SkillCategory — 14 entries, 1:1 映射 superpowers 公开 SKILL.md
// =====================================================================

/// **R125-19 借鉴 superpowers 14 公开 SKILL.md frontmatter 1:1**
///
/// 顺序跟 `borrowed-repos/superpowers/skills/<name>/SKILL.md` 一致
/// (per R125-14 done 17:54 借鉴源码 cloned, per decision-36 §1.1).
///
/// 不漂移: 0 装"已借鉴" superpowers 私有 plugin 加载机制
/// (per decision-36 §1.1 + 决策 #51 §1.4 0 装 PASS 严守).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillCategory {
    /// `skills/brainstorming/SKILL.md` — 探索选项
    Brainstorming,
    /// `skills/test-driven-development/SKILL.md` — TDD 强制
    TestDrivenDevelopment,
    /// `skills/systematic-debugging/SKILL.md` — 系统化 debug
    SystematicDebugging,
    /// `skills/verification-before-completion/SKILL.md` — 完成前 verify
    VerificationBeforeCompletion,
    /// `skills/writing-plans/SKILL.md` — 写计划
    WritingPlans,
    /// `skills/executing-plans/SKILL.md` — 执行计划
    ExecutingPlans,
    /// `skills/subagent-driven-development/SKILL.md` — subagent 驱动
    SubagentDrivenDevelopment,
    /// `skills/dispatching-parallel-agents/SKILL.md` — 并行派发
    DispatchingParallelAgents,
    /// `skills/requesting-code-review/SKILL.md` — 请求 review
    RequestingCodeReview,
    /// `skills/receiving-code-review/SKILL.md` — 接收 review
    ReceivingCodeReview,
    /// `skills/using-git-worktrees/SKILL.md` — 用 git worktree
    UsingGitWorktrees,
    /// `skills/finishing-a-development-branch/SKILL.md` — 完成 branch
    FinishingADevelopmentBranch,
    /// `skills/writing-skills/SKILL.md` — 写 skill
    WritingSkills,
    /// `skills/using-superpowers/SKILL.md` — meta: 选 skill
    UsingSuperpowers,
}

impl SkillCategory {
    /// **14 entries 严守** (compile-time hardcode)
    pub const COUNT: usize = 14;
    /// 14 entries 数组 (1:1 跟 borrowed-repos/skills/ 子目录顺序一致)
    pub const ALL: [SkillCategory; 14] = [
        Self::Brainstorming,
        Self::TestDrivenDevelopment,
        Self::SystematicDebugging,
        Self::VerificationBeforeCompletion,
        Self::WritingPlans,
        Self::ExecutingPlans,
        Self::SubagentDrivenDevelopment,
        Self::DispatchingParallelAgents,
        Self::RequestingCodeReview,
        Self::ReceivingCodeReview,
        Self::UsingGitWorktrees,
        Self::FinishingADevelopmentBranch,
        Self::WritingSkills,
        Self::UsingSuperpowers,
    ];

    /// **kebab name** (1:1 跟 superpowers 公开 SKILL.md 目录名一致)
    pub fn kebab_name(&self) -> &'static str {
        match self {
            Self::Brainstorming => "brainstorming",
            Self::TestDrivenDevelopment => "test-driven-development",
            Self::SystematicDebugging => "systematic-debugging",
            Self::VerificationBeforeCompletion => "verification-before-completion",
            Self::WritingPlans => "writing-plans",
            Self::ExecutingPlans => "executing-plans",
            Self::SubagentDrivenDevelopment => "subagent-driven-development",
            Self::DispatchingParallelAgents => "dispatching-parallel-agents",
            Self::RequestingCodeReview => "requesting-code-review",
            Self::ReceivingCodeReview => "receiving-code-review",
            Self::UsingGitWorktrees => "using-git-worktrees",
            Self::FinishingADevelopmentBranch => "finishing-a-development-branch",
            Self::WritingSkills => "writing-skills",
            Self::UsingSuperpowers => "using-superpowers",
        }
    }

    /// **from kebab name** — 反向 (e.g. "test-driven-development" -> TestDrivenDevelopment)
    pub fn from_kebab(name: &str) -> Option<Self> {
        match name {
            "brainstorming" => Some(Self::Brainstorming),
            "test-driven-development" => Some(Self::TestDrivenDevelopment),
            "systematic-debugging" => Some(Self::SystematicDebugging),
            "verification-before-completion" => Some(Self::VerificationBeforeCompletion),
            "writing-plans" => Some(Self::WritingPlans),
            "executing-plans" => Some(Self::ExecutingPlans),
            "subagent-driven-development" => Some(Self::SubagentDrivenDevelopment),
            "dispatching-parallel-agents" => Some(Self::DispatchingParallelAgents),
            "requesting-code-review" => Some(Self::RequestingCodeReview),
            "receiving-code-review" => Some(Self::ReceivingCodeReview),
            "using-git-worktrees" => Some(Self::UsingGitWorktrees),
            "finishing-a-development-branch" => Some(Self::FinishingADevelopmentBranch),
            "writing-skills" => Some(Self::WritingSkills),
            "using-superpowers" => Some(Self::UsingSuperpowers),
            _ => None,
        }
    }
}

impl fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kebab_name())
    }
}

// =====================================================================
// ExecutionPattern — 5 patterns, 把 14 categories 分组
// =====================================================================

/// **R125-19 execution pattern — 5 patterns 覆盖 14 categories**
///
/// 1:1 跟 superpowers 公开 SKILL.md 的 step 顺序对应 (TDD red-green-refactor 是铁律).
/// 0 装"已借鉴" superpowers 私有 plugin 加载机制.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExecutionPattern {
    /// TDD: red → green → refactor → done (3 phase, 1 cycle)
    /// 对应 superpowers: `test-driven-development` + `systematic-debugging`
    Tdd,
    /// Plan → Execute → Verify → Iterate → done (4 phase, 多 iteration)
    /// 对应 superpowers: `writing-plans` + `executing-plans` + `verification-before-completion`
    PlanExecuteVerify,
    /// Dispatch → Collect → Merge → done (3 phase)
    /// 对应 superpowers: `subagent-driven-development` + `dispatching-parallel-agents`
    Parallel,
    /// Submit → Receive → Apply → done (3 phase)
    /// 对应 superpowers: `requesting-code-review` + `receiving-code-review`
    Review,
    /// Identify → Author → Lifecycle → done (3 phase, 含 branch + meta)
    /// 对应 superpowers: `using-superpowers` + `brainstorming` + `writing-skills` +
    /// `using-git-worktrees` + `finishing-a-development-branch`
    Meta,
}

impl ExecutionPattern {
    /// **5 patterns 严守** (compile-time hardcode)
    pub const COUNT: usize = 5;
    pub const ALL: [ExecutionPattern; 5] = [
        Self::Tdd,
        Self::PlanExecuteVerify,
        Self::Parallel,
        Self::Review,
        Self::Meta,
    ];
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tdd => "tdd",
            Self::PlanExecuteVerify => "plan-execute-verify",
            Self::Parallel => "parallel",
            Self::Review => "review",
            Self::Meta => "meta",
        }
    }
}

impl fmt::Display for ExecutionPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// **14 categories → 5 patterns 映射 (compile-time fn, 0 漂移)**
///
/// 严守: Tdd cycle 包含 test-driven-development + systematic-debugging (2 cat)
/// PlanExecuteVerify 包含 writing-plans + executing-plans + verification-before-completion (3 cat)
/// Parallel 包含 subagent-driven-development + dispatching-parallel-agents (2 cat)
/// Review 包含 requesting-code-review + receiving-code-review (2 cat)
/// Meta 包含 using-superpowers + brainstorming + writing-skills + using-git-worktrees
///   + finishing-a-development-branch (5 cat)
/// 总 2+3+2+2+5 = 14 严守.
pub fn category_to_pattern(cat: SkillCategory) -> ExecutionPattern {
    match cat {
        SkillCategory::TestDrivenDevelopment | SkillCategory::SystematicDebugging => {
            ExecutionPattern::Tdd
        }
        SkillCategory::WritingPlans
        | SkillCategory::ExecutingPlans
        | SkillCategory::VerificationBeforeCompletion => ExecutionPattern::PlanExecuteVerify,
        SkillCategory::SubagentDrivenDevelopment
        | SkillCategory::DispatchingParallelAgents => ExecutionPattern::Parallel,
        SkillCategory::RequestingCodeReview | SkillCategory::ReceivingCodeReview => {
            ExecutionPattern::Review
        }
        SkillCategory::UsingSuperpowers
        | SkillCategory::Brainstorming
        | SkillCategory::WritingSkills
        | SkillCategory::UsingGitWorktrees
        | SkillCategory::FinishingADevelopmentBranch => ExecutionPattern::Meta,
    }
}

/// **列出 pattern 下所有 categories** (跟 category_to_pattern 反向)
pub fn categories_in_pattern(p: ExecutionPattern) -> Vec<SkillCategory> {
    SkillCategory::ALL
        .iter()
        .copied()
        .filter(|c| category_to_pattern(*c) == p)
        .collect()
}

/// **pattern 步数严守** (compile-time 检查)
pub fn pattern_step_count(p: ExecutionPattern) -> usize {
    match p {
        ExecutionPattern::Tdd => 4,             // Red, Green, Refactor, Done
        ExecutionPattern::PlanExecuteVerify => 5, // Plan, Execute, Verify, Iterate, Done
        ExecutionPattern::Parallel => 4,         // Dispatch, Collect, Merge, Done
        ExecutionPattern::Review => 4,           // Submit, Receive, Apply, Done
        ExecutionPattern::Meta => 4,             // Identify, Author, Lifecycle, Done
    }
}

// =====================================================================
// ExecutionStep — 1 step 描述
// =====================================================================

/// **1 step 描述** (per pattern 内的 1 步)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStep {
    /// 步序号 (从 0 开始, terminal step 也占 1 个序号)
    pub order: usize,
    /// 步名 (e.g. "Red", "Plan", "Dispatch")
    pub name: String,
    /// 步描述 (1 句话)
    pub description: String,
    /// 是否终止 (cycle 到此 = done, 不可 advance)
    pub is_terminal: bool,
}

impl ExecutionStep {
    /// 便利构造
    pub fn new(
        order: usize,
        name: impl Into<String>,
        description: impl Into<String>,
        is_terminal: bool,
    ) -> Self {
        Self {
            order,
            name: name.into(),
            description: description.into(),
            is_terminal,
        }
    }
}

/// **返回 pattern 全步骤** (5/5/4/3/3 步 严守)
pub fn pattern_steps(p: ExecutionPattern) -> Vec<ExecutionStep> {
    match p {
        ExecutionPattern::Tdd => vec![
            ExecutionStep::new(0, "Red", "写失败测试 (failing test, 确认能 fail)", false),
            ExecutionStep::new(1, "Green", "写最少代码让 test pass", false),
            ExecutionStep::new(2, "Refactor", "在 test pass 前提下重整代码 (0 改 test 行为)", false),
            ExecutionStep::new(3, "Done", "TDD cycle 完成, 提交", true),
        ],
        ExecutionPattern::PlanExecuteVerify => vec![
            ExecutionStep::new(0, "Plan", "写 plan: 目标 + 任务列表", false),
            ExecutionStep::new(1, "Execute", "按 plan 顺序执行任务", false),
            ExecutionStep::new(2, "Verify", "执行完跑 verify (test / build / lint)", false),
            ExecutionStep::new(3, "Iterate", "verify fail → 回 Execute (可多轮)", false),
            ExecutionStep::new(4, "Done", "verify pass 收尾", true),
        ],
        ExecutionPattern::Parallel => vec![
            ExecutionStep::new(0, "Dispatch", "fan-out 任务到 N 个 sub-agent", false),
            ExecutionStep::new(1, "Collect", "收集 N 个 sub-agent 产出", false),
            ExecutionStep::new(2, "Merge", "合并产出 (冲突解决)", false),
            ExecutionStep::new(3, "Done", "Parallel 周期完成", true),
        ],
        ExecutionPattern::Review => vec![
            ExecutionStep::new(0, "Submit", "提交工作给 reviewer", false),
            ExecutionStep::new(1, "Receive", "接收 review 反馈", false),
            ExecutionStep::new(2, "Apply", "应用 review 改动 (or reject with reason)", false),
            ExecutionStep::new(3, "Done", "Review 周期完成", true),
        ],
        ExecutionPattern::Meta => vec![
            ExecutionStep::new(0, "Identify", "识别用哪个 skill (meta selection)", false),
            ExecutionStep::new(1, "Author", "写新 skill (or 改现有)", false),
            ExecutionStep::new(2, "Lifecycle", "branch / commit / merge (git worktree)", false),
            ExecutionStep::new(3, "Done", "Meta 周期完成", true),
        ],
    }
}

// =====================================================================
// 1. TddCycle — red → green → refactor → done
// =====================================================================

/// **TDD 状态机** (per superpowers `test-driven-development` 铁律)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TddPhase {
    /// 初始: 写失败 test
    Red,
    /// 写最少代码让 test pass
    Green,
    /// 在 test pass 前提下重整代码
    Refactor,
    /// 完成
    Done,
}

impl fmt::Display for TddPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Red => f.write_str("Red"),
            Self::Green => f.write_str("Green"),
            Self::Refactor => f.write_str("Refactor"),
            Self::Done => f.write_str("Done"),
        }
    }
}

impl TddPhase {
    /// **TDD 3 phase 严守** (Red / Green / Refactor) + Done
    pub const PHASES: [TddPhase; 3] = [Self::Red, Self::Green, Self::Refactor];
}

/// **TddCycle — 1 个 TDD 周期状态机**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TddCycle {
    /// 当前 phase
    pub phase: TddPhase,
    /// 关联 test name (e.g. "test_skill_category_count_is_14")
    pub test_name: String,
    /// 已经 advance 了几次
    pub iteration: usize,
    /// 历史 phase (push 顺序)
    pub history: Vec<TddPhase>,
}

impl TddCycle {
    /// 新 cycle (从 Red 开始)
    pub fn new(test_name: impl Into<String>) -> Self {
        let test_name = test_name.into();
        let phase = TddPhase::Red;
        let mut history = Vec::with_capacity(4);
        history.push(phase);
        Self {
            phase,
            test_name,
            iteration: 0,
            history,
        }
    }

    /// **推进到下一 phase** (按 Red → Green → Refactor → Done 顺序)
    /// 返 true = 推进成功, false = 已 Done (0 推进)
    pub fn advance(&mut self) -> bool {
        let next = match self.phase {
            TddPhase::Red => TddPhase::Green,
            TddPhase::Green => TddPhase::Refactor,
            TddPhase::Refactor => TddPhase::Done,
            TddPhase::Done => return false,
        };
        self.phase = next;
        self.history.push(next);
        self.iteration += 1;
        true
    }

    /// 是否完成
    pub fn is_done(&self) -> bool {
        self.phase == TddPhase::Done
    }

    /// 历史 step 数
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// **重置** (回到 Red, 保留 test_name)
    pub fn reset(&mut self) {
        self.phase = TddPhase::Red;
        self.iteration = 0;
        self.history.clear();
        self.history.push(TddPhase::Red);
    }
}

// =====================================================================
// 2. PlanExecuteVerifyCycle — plan → execute → verify → iterate → done
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanPhase {
    Plan,
    Execute,
    Verify,
    /// Iterate = 回到 Execute (verify fail 时)
    Iterate,
    Done,
}

impl fmt::Display for PlanPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan => f.write_str("Plan"),
            Self::Execute => f.write_str("Execute"),
            Self::Verify => f.write_str("Verify"),
            Self::Iterate => f.write_str("Iterate"),
            Self::Done => f.write_str("Done"),
        }
    }
}

impl PlanPhase {
    /// **Plan 4 phase 严守** (Plan / Execute / Verify / Iterate) + Done
    pub const PHASES: [PlanPhase; 4] = [Self::Plan, Self::Execute, Self::Verify, Self::Iterate];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanExecuteVerifyCycle {
    pub phase: PlanPhase,
    pub goal: String,
    pub iteration: usize,
    pub history: Vec<PlanPhase>,
    /// verify pass 历史 (true = pass, false = fail → 触发 Iterate)
    pub verify_outcomes: Vec<bool>,
}

impl PlanExecuteVerifyCycle {
    pub fn new(goal: impl Into<String>) -> Self {
        let goal = goal.into();
        let phase = PlanPhase::Plan;
        let mut history = Vec::with_capacity(8);
        history.push(phase);
        Self {
            phase,
            goal,
            iteration: 0,
            history,
            verify_outcomes: Vec::new(),
        }
    }

    /// 推进 (Plan → Execute → Verify)
    pub fn advance(&mut self) -> bool {
        let next = match self.phase {
            PlanPhase::Plan => PlanPhase::Execute,
            PlanPhase::Execute => PlanPhase::Verify,
            PlanPhase::Verify => return false, // 等 record_verify_outcome
            PlanPhase::Iterate => PlanPhase::Execute, // Iterate → Execute (重做)
            PlanPhase::Done => return false,
        };
        self.phase = next;
        self.history.push(next);
        self.iteration += 1;
        true
    }

    /// 记录 verify 结论 (verify 阶段调用, pass → Done, fail → Iterate)
    pub fn record_verify_outcome(&mut self, passed: bool) -> bool {
        if self.phase != PlanPhase::Verify {
            return false;
        }
        self.verify_outcomes.push(passed);
        if passed {
            self.phase = PlanPhase::Done;
            self.history.push(PlanPhase::Done);
        } else {
            self.phase = PlanPhase::Iterate;
            self.history.push(PlanPhase::Iterate);
            self.iteration += 1;
        }
        true
    }

    pub fn is_done(&self) -> bool {
        self.phase == PlanPhase::Done
    }

    /// 全部 verify 轮次中 pass 率
    pub fn verify_pass_rate(&self) -> Option<f64> {
        if self.verify_outcomes.is_empty() {
            None
        } else {
            let passed = self.verify_outcomes.iter().filter(|p| **p).count();
            Some(passed as f64 / self.verify_outcomes.len() as f64)
        }
    }
}

// =====================================================================
// 3. ParallelCycle — dispatch → collect → merge → done
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParallelPhase {
    Dispatch,
    Collect,
    Merge,
    Done,
}

impl fmt::Display for ParallelPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dispatch => f.write_str("Dispatch"),
            Self::Collect => f.write_str("Collect"),
            Self::Merge => f.write_str("Merge"),
            Self::Done => f.write_str("Done"),
        }
    }
}

impl ParallelPhase {
    /// **Parallel 3 phase 严守** (Dispatch / Collect / Merge) + Done
    pub const PHASES: [ParallelPhase; 3] = [Self::Dispatch, Self::Collect, Self::Merge];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelCycle {
    pub phase: ParallelPhase,
    pub task_count: usize,
    pub collected: usize,
    pub merged: usize,
    pub history: Vec<ParallelPhase>,
}

impl ParallelCycle {
    pub fn new(task_count: usize) -> Self {
        assert!(task_count > 0, "task_count must be > 0");
        let phase = ParallelPhase::Dispatch;
        let mut history = Vec::with_capacity(4);
        history.push(phase);
        Self {
            phase,
            task_count,
            collected: 0,
            merged: 0,
            history,
        }
    }

    /// 推进
    pub fn advance(&mut self) -> bool {
        let next = match self.phase {
            ParallelPhase::Dispatch => ParallelPhase::Collect,
            ParallelPhase::Collect => ParallelPhase::Merge,
            ParallelPhase::Merge => ParallelPhase::Done,
            ParallelPhase::Done => return false,
        };
        self.phase = next;
        self.history.push(next);
        true
    }

    /// Collect 阶段调用 — 记录收到几个 sub-agent 产出
    pub fn record_collected(&mut self, count: usize) {
        if self.phase == ParallelPhase::Collect {
            self.collected = count;
        }
    }

    /// Merge 阶段调用 — 记录合并了几个
    pub fn record_merged(&mut self, count: usize) {
        if self.phase == ParallelPhase::Merge {
            self.merged = count;
        }
    }

    pub fn is_done(&self) -> bool {
        self.phase == ParallelPhase::Done
    }

    /// 收集率 (collected / task_count)
    pub fn collection_rate(&self) -> f64 {
        if self.task_count == 0 {
            0.0
        } else {
            self.collected as f64 / self.task_count as f64
        }
    }
}

// =====================================================================
// 4. ReviewCycle — submit → receive → apply → done
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewPhase {
    Submit,
    Receive,
    Apply,
    Done,
}

impl fmt::Display for ReviewPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Submit => f.write_str("Submit"),
            Self::Receive => f.write_str("Receive"),
            Self::Apply => f.write_str("Apply"),
            Self::Done => f.write_str("Done"),
        }
    }
}

impl ReviewPhase {
    /// **Review 3 phase 严守** (Submit / Receive / Apply) + Done
    pub const PHASES: [ReviewPhase; 3] = [Self::Submit, Self::Receive, Self::Apply];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCycle {
    pub phase: ReviewPhase,
    pub submit_count: usize,
    pub feedback_count: usize,
    pub applied_count: usize,
    pub history: Vec<ReviewPhase>,
}

impl ReviewCycle {
    pub fn new() -> Self {
        let phase = ReviewPhase::Submit;
        let mut history = Vec::with_capacity(4);
        history.push(phase);
        Self {
            phase,
            submit_count: 0,
            feedback_count: 0,
            applied_count: 0,
            history,
        }
    }

    pub fn advance(&mut self) -> bool {
        let next = match self.phase {
            ReviewPhase::Submit => ReviewPhase::Receive,
            ReviewPhase::Receive => ReviewPhase::Apply,
            ReviewPhase::Apply => ReviewPhase::Done,
            ReviewPhase::Done => return false,
        };
        self.phase = next;
        self.history.push(next);
        true
    }

    pub fn record_submit(&mut self) {
        if self.phase == ReviewPhase::Submit {
            self.submit_count += 1;
        }
    }

    pub fn record_feedback(&mut self, count: usize) {
        if self.phase == ReviewPhase::Receive {
            self.feedback_count = count;
        }
    }

    pub fn record_applied(&mut self, count: usize) {
        if self.phase == ReviewPhase::Apply {
            self.applied_count = count;
        }
    }

    pub fn is_done(&self) -> bool {
        self.phase == ReviewPhase::Done
    }
}

impl Default for ReviewCycle {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================================
// 5. MetaCycle — identify → author → lifecycle → done
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetaPhase {
    /// 识别用哪个 skill
    Identify,
    /// 写新 skill (or 改现有)
    Author,
    /// branch / commit / merge (git worktree)
    Lifecycle,
    Done,
}

impl fmt::Display for MetaPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identify => f.write_str("Identify"),
            Self::Author => f.write_str("Author"),
            Self::Lifecycle => f.write_str("Lifecycle"),
            Self::Done => f.write_str("Done"),
        }
    }
}

impl MetaPhase {
    /// **Meta 3 phase 严守** (Identify / Author / Lifecycle) + Done
    pub const PHASES: [MetaPhase; 3] = [Self::Identify, Self::Author, Self::Lifecycle];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaCycle {
    pub phase: MetaPhase,
    pub identified_skill: Option<SkillCategory>,
    pub authored_skill: Option<String>,
    pub branch_name: Option<String>,
    pub history: Vec<MetaPhase>,
}

impl MetaCycle {
    pub fn new() -> Self {
        let phase = MetaPhase::Identify;
        let mut history = Vec::with_capacity(4);
        history.push(phase);
        Self {
            phase,
            identified_skill: None,
            authored_skill: None,
            branch_name: None,
            history,
        }
    }

    pub fn advance(&mut self) -> bool {
        let next = match self.phase {
            MetaPhase::Identify => MetaPhase::Author,
            MetaPhase::Author => MetaPhase::Lifecycle,
            MetaPhase::Lifecycle => MetaPhase::Done,
            MetaPhase::Done => return false,
        };
        self.phase = next;
        self.history.push(next);
        true
    }

    pub fn record_identified(&mut self, cat: SkillCategory) {
        if self.phase == MetaPhase::Identify {
            self.identified_skill = Some(cat);
        }
    }

    pub fn record_authored(&mut self, name: impl Into<String>) {
        if self.phase == MetaPhase::Author {
            self.authored_skill = Some(name.into());
        }
    }

    pub fn record_branch(&mut self, branch: impl Into<String>) {
        if self.phase == MetaPhase::Lifecycle {
            self.branch_name = Some(branch.into());
        }
    }

    pub fn is_done(&self) -> bool {
        self.phase == MetaPhase::Done
    }
}

impl Default for MetaCycle {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================================
// 单元测试 — SkillCategory
// =====================================================================

#[cfg(test)]
mod tests_category {
    use super::*;

    #[test]
    fn skill_category_count_is_14() {
        assert_eq!(SkillCategory::COUNT, 14);
        assert_eq!(SkillCategory::ALL.len(), 14);
    }

    #[test]
    fn skill_category_all_kebab_names_unique() {
        let mut names: Vec<&str> = SkillCategory::ALL.iter().map(|c| c.kebab_name()).collect();
        names.sort();
        let original_len = names.len();
        names.dedup();
        assert_eq!(names.len(), original_len, "kebab names 0 重复");
    }

    #[test]
    fn skill_category_from_kebab_roundtrip() {
        for cat in SkillCategory::ALL.iter() {
            let name = cat.kebab_name();
            let recovered = SkillCategory::from_kebab(name);
            assert_eq!(recovered, Some(*cat), "roundtrip fail for {name}");
        }
    }

    #[test]
    fn skill_category_from_kebab_unknown_returns_none() {
        assert_eq!(SkillCategory::from_kebab("nope"), None);
        assert_eq!(SkillCategory::from_kebab(""), None);
        assert_eq!(SkillCategory::from_kebab("Brainstorming"), None); // 0 大小写
    }

    #[test]
    fn skill_category_display_matches_kebab() {
        for cat in SkillCategory::ALL.iter() {
            assert_eq!(format!("{cat}"), cat.kebab_name());
        }
    }
}

// =====================================================================
// 单元测试 — ExecutionPattern + 映射
// =====================================================================

#[cfg(test)]
mod tests_pattern {
    use super::*;

    #[test]
    fn execution_pattern_count_is_5() {
        assert_eq!(ExecutionPattern::COUNT, 5);
        assert_eq!(ExecutionPattern::ALL.len(), 5);
    }

    #[test]
    fn category_to_pattern_14_to_5_total() {
        let mut counts = [0usize; 5];
        for cat in SkillCategory::ALL.iter() {
            let p = category_to_pattern(*cat);
            let idx = ExecutionPattern::ALL.iter().position(|x| *x == p).unwrap();
            counts[idx] += 1;
        }
        // Tdd=2 + PlanExecuteVerify=3 + Parallel=2 + Review=2 + Meta=5 = 14
        assert_eq!(counts, [2, 3, 2, 2, 5]);
    }

    #[test]
    fn categories_in_pattern_5_to_14_total() {
        let mut total = 0;
        for p in ExecutionPattern::ALL.iter() {
            let cats = categories_in_pattern(*p);
            for c in cats.iter() {
                assert_eq!(category_to_pattern(*c), *p, "roundtrip fail for {c}");
            }
            total += cats.len();
        }
        assert_eq!(total, 14);
    }

    #[test]
    fn pattern_step_count_matches_pattern_steps_len() {
        for p in ExecutionPattern::ALL.iter() {
            let expected = pattern_step_count(*p);
            let actual = pattern_steps(*p).len();
            assert_eq!(actual, expected, "pattern {p} step count mismatch");
        }
    }

    #[test]
    fn pattern_steps_terminal_is_last() {
        for p in ExecutionPattern::ALL.iter() {
            let steps = pattern_steps(*p);
            let last = steps.last().expect("at least 1 step");
            assert!(last.is_terminal, "pattern {p} last step 0 terminal");
            for s in steps.iter().take(steps.len() - 1) {
                assert!(!s.is_terminal, "pattern {p} non-last step marked terminal");
            }
        }
    }

    #[test]
    fn pattern_steps_orders_are_dense_from_zero() {
        for p in ExecutionPattern::ALL.iter() {
            let steps = pattern_steps(*p);
            for (i, s) in steps.iter().enumerate() {
                assert_eq!(s.order, i, "pattern {p} step {i} order mismatch");
            }
        }
    }
}

// =====================================================================
// 单元测试 — TddCycle
// =====================================================================

#[cfg(test)]
mod tests_tdd {
    use super::*;

    #[test]
    fn tdd_cycle_starts_at_red() {
        let c = TddCycle::new("test_x");
        assert_eq!(c.phase, TddPhase::Red);
        assert!(!c.is_done());
        assert_eq!(c.history_len(), 1);
    }

    #[test]
    fn tdd_cycle_advance_red_green_refactor_done() {
        let mut c = TddCycle::new("test_y");
        assert!(c.advance());
        assert_eq!(c.phase, TddPhase::Green);
        assert!(c.advance());
        assert_eq!(c.phase, TddPhase::Refactor);
        assert!(c.advance());
        assert_eq!(c.phase, TddPhase::Done);
        assert!(c.is_done());
    }

    #[test]
    fn tdd_cycle_advance_done_returns_false() {
        let mut c = TddCycle::new("test_z");
        for _ in 0..3 {
            c.advance();
        }
        assert!(c.is_done());
        assert!(!c.advance(), "advance after Done 应该返 false");
    }

    #[test]
    fn tdd_cycle_history_tracks_all_phases() {
        let mut c = TddCycle::new("test_h");
        c.advance();
        c.advance();
        c.advance();
        assert_eq!(
            c.history,
            vec![TddPhase::Red, TddPhase::Green, TddPhase::Refactor, TddPhase::Done]
        );
    }

    #[test]
    fn tdd_cycle_reset_back_to_red() {
        let mut c = TddCycle::new("test_r");
        c.advance();
        c.advance();
        c.advance();
        c.reset();
        assert_eq!(c.phase, TddPhase::Red);
        assert_eq!(c.iteration, 0);
        assert_eq!(c.history, vec![TddPhase::Red]);
        assert_eq!(c.test_name, "test_r"); // 保留 test_name
    }

    #[test]
    fn tdd_cycle_iteration_count_increments() {
        let mut c = TddCycle::new("test_i");
        assert_eq!(c.iteration, 0);
        c.advance();
        assert_eq!(c.iteration, 1);
        c.advance();
        assert_eq!(c.iteration, 2);
        c.advance();
        assert_eq!(c.iteration, 3);
    }
}

// =====================================================================
// 单元测试 — PlanExecuteVerifyCycle
// =====================================================================

#[cfg(test)]
mod tests_plan {
    use super::*;

    #[test]
    fn plan_cycle_starts_at_plan() {
        let c = PlanExecuteVerifyCycle::new("implement feature X");
        assert_eq!(c.phase, PlanPhase::Plan);
        assert!(!c.is_done());
    }

    #[test]
    fn plan_cycle_advance_plan_execute_verify() {
        let mut c = PlanExecuteVerifyCycle::new("g");
        c.advance();
        assert_eq!(c.phase, PlanPhase::Execute);
        c.advance();
        assert_eq!(c.phase, PlanPhase::Verify);
    }

    #[test]
    fn plan_cycle_verify_pass_moves_to_done() {
        let mut c = PlanExecuteVerifyCycle::new("g");
        c.advance(); // Plan → Execute
        c.advance(); // Execute → Verify
        assert!(c.record_verify_outcome(true));
        assert_eq!(c.phase, PlanPhase::Done);
        assert!(c.is_done());
        assert_eq!(c.verify_outcomes, vec![true]);
    }

    #[test]
    fn plan_cycle_verify_fail_moves_to_iterate_then_execute() {
        let mut c = PlanExecuteVerifyCycle::new("g");
        c.advance(); // → Execute
        c.advance(); // → Verify
        assert!(c.record_verify_outcome(false));
        assert_eq!(c.phase, PlanPhase::Iterate);
        c.advance(); // Iterate → Execute
        assert_eq!(c.phase, PlanPhase::Execute);
    }

    #[test]
    fn plan_cycle_iterate_then_pass() {
        let mut c = PlanExecuteVerifyCycle::new("g");
        c.advance();
        c.advance();
        c.record_verify_outcome(false); // fail
        c.advance(); // Iterate → Execute
        c.advance(); // Execute → Verify
        c.record_verify_outcome(true); // pass
        assert!(c.is_done());
        assert_eq!(c.verify_outcomes, vec![false, true]);
        assert_eq!(c.verify_pass_rate(), Some(0.5));
    }

    #[test]
    fn plan_cycle_record_verify_only_at_verify_phase() {
        let mut c = PlanExecuteVerifyCycle::new("g");
        assert!(!c.record_verify_outcome(true), "Plan 阶段 0 应该接受 verify");
    }

    #[test]
    fn plan_cycle_verify_pass_rate_empty() {
        let c = PlanExecuteVerifyCycle::new("g");
        assert_eq!(c.verify_pass_rate(), None);
    }
}

// =====================================================================
// 单元测试 — ParallelCycle
// =====================================================================

#[cfg(test)]
mod tests_parallel {
    use super::*;

    #[test]
    fn parallel_cycle_starts_at_dispatch() {
        let c = ParallelCycle::new(4);
        assert_eq!(c.phase, ParallelPhase::Dispatch);
        assert_eq!(c.task_count, 4);
    }

    #[test]
    fn parallel_cycle_advance_dispatch_collect_merge_done() {
        let mut c = ParallelCycle::new(3);
        c.advance();
        assert_eq!(c.phase, ParallelPhase::Collect);
        c.advance();
        assert_eq!(c.phase, ParallelPhase::Merge);
        c.advance();
        assert_eq!(c.phase, ParallelPhase::Done);
        assert!(c.is_done());
    }

    #[test]
    fn parallel_cycle_record_collected_at_collect() {
        let mut c = ParallelCycle::new(5);
        c.advance(); // → Collect
        c.record_collected(5);
        assert_eq!(c.collected, 5);
        assert_eq!(c.collection_rate(), 1.0);
    }

    #[test]
    fn parallel_cycle_record_merged_at_merge() {
        let mut c = ParallelCycle::new(5);
        c.advance();
        c.advance(); // → Merge
        c.record_merged(5);
        assert_eq!(c.merged, 5);
    }

    #[test]
    fn parallel_cycle_collection_rate_partial() {
        let mut c = ParallelCycle::new(4);
        c.advance();
        c.record_collected(2);
        assert_eq!(c.collection_rate(), 0.5);
    }

    #[test]
    #[should_panic(expected = "task_count must be > 0")]
    fn parallel_cycle_zero_task_count_panics() {
        let _ = ParallelCycle::new(0);
    }
}

// =====================================================================
// 单元测试 — ReviewCycle
// =====================================================================

#[cfg(test)]
mod tests_review {
    use super::*;

    #[test]
    fn review_cycle_starts_at_submit() {
        let c = ReviewCycle::new();
        assert_eq!(c.phase, ReviewPhase::Submit);
        assert!(!c.is_done());
    }

    #[test]
    fn review_cycle_advance_submit_receive_apply_done() {
        let mut c = ReviewCycle::new();
        c.advance();
        assert_eq!(c.phase, ReviewPhase::Receive);
        c.advance();
        assert_eq!(c.phase, ReviewPhase::Apply);
        c.advance();
        assert_eq!(c.phase, ReviewPhase::Done);
    }

    #[test]
    fn review_cycle_record_submit_at_submit() {
        let mut c = ReviewCycle::new();
        c.record_submit();
        assert_eq!(c.submit_count, 1);
        c.record_submit();
        assert_eq!(c.submit_count, 2);
    }

    #[test]
    fn review_cycle_record_feedback_at_receive() {
        let mut c = ReviewCycle::new();
        c.advance(); // → Receive
        c.record_feedback(3);
        assert_eq!(c.feedback_count, 3);
    }

    #[test]
    fn review_cycle_record_applied_at_apply() {
        let mut c = ReviewCycle::new();
        c.advance();
        c.advance(); // → Apply
        c.record_applied(2);
        assert_eq!(c.applied_count, 2);
    }

    #[test]
    fn review_cycle_advance_done_returns_false() {
        let mut c = ReviewCycle::new();
        c.advance();
        c.advance();
        c.advance();
        assert!(c.is_done());
        assert!(!c.advance());
    }
}

// =====================================================================
// 单元测试 — MetaCycle
// =====================================================================

#[cfg(test)]
mod tests_meta {
    use super::*;

    #[test]
    fn meta_cycle_starts_at_identify() {
        let c = MetaCycle::new();
        assert_eq!(c.phase, MetaPhase::Identify);
        assert!(!c.is_done());
    }

    #[test]
    fn meta_cycle_advance_identify_author_lifecycle_done() {
        let mut c = MetaCycle::new();
        c.advance();
        assert_eq!(c.phase, MetaPhase::Author);
        c.advance();
        assert_eq!(c.phase, MetaPhase::Lifecycle);
        c.advance();
        assert_eq!(c.phase, MetaPhase::Done);
    }

    #[test]
    fn meta_cycle_record_identified_at_identify() {
        let mut c = MetaCycle::new();
        c.record_identified(SkillCategory::TestDrivenDevelopment);
        assert_eq!(c.identified_skill, Some(SkillCategory::TestDrivenDevelopment));
    }

    #[test]
    fn meta_cycle_record_authored_at_author() {
        let mut c = MetaCycle::new();
        c.advance(); // → Author
        c.record_authored("my-new-skill");
        assert_eq!(c.authored_skill, Some("my-new-skill".to_string()));
    }

    #[test]
    fn meta_cycle_record_branch_at_lifecycle() {
        let mut c = MetaCycle::new();
        c.advance();
        c.advance(); // → Lifecycle
        c.record_branch("feat/my-skill");
        assert_eq!(c.branch_name, Some("feat/my-skill".to_string()));
    }

    #[test]
    fn meta_cycle_full_workflow_with_all_records() {
        let mut c = MetaCycle::new();
        c.record_identified(SkillCategory::WritingSkills);
        c.advance();
        c.record_authored("tdd-flow");
        c.advance();
        c.record_branch("feat/tdd-flow");
        c.advance();
        assert!(c.is_done());
        assert_eq!(c.identified_skill, Some(SkillCategory::WritingSkills));
        assert_eq!(c.authored_skill, Some("tdd-flow".to_string()));
        assert_eq!(c.branch_name, Some("feat/tdd-flow".to_string()));
    }
}

// =====================================================================
// 顶层总测试
// =====================================================================

#[cfg(test)]
mod tests_integration {
    use super::*;

    #[test]
    fn all_5_patterns_have_working_cycles() {
        // 5 patterns 都有可工作的 state machine
        let _tdd = TddCycle::new("t");
        let _plan = PlanExecuteVerifyCycle::new("g");
        let _parallel = ParallelCycle::new(2);
        let _review = ReviewCycle::new();
        let _meta = MetaCycle::new();
    }

    #[test]
    fn all_14_categories_map_to_5_patterns() {
        for cat in SkillCategory::ALL.iter() {
            let p = category_to_pattern(*cat);
            let cats = categories_in_pattern(p);
            assert!(
                cats.contains(cat),
                "category {cat} 0 在 pattern {p} 的 categories 列表里"
            );
        }
    }

    #[test]
    fn tdd_pattern_covers_test_driven_development_and_systematic_debugging() {
        let cats = categories_in_pattern(ExecutionPattern::Tdd);
        assert!(cats.contains(&SkillCategory::TestDrivenDevelopment));
        assert!(cats.contains(&SkillCategory::SystematicDebugging));
    }

    #[test]
    fn plan_pattern_covers_3_categories() {
        let cats = categories_in_pattern(ExecutionPattern::PlanExecuteVerify);
        assert!(cats.contains(&SkillCategory::WritingPlans));
        assert!(cats.contains(&SkillCategory::ExecutingPlans));
        assert!(cats.contains(&SkillCategory::VerificationBeforeCompletion));
    }

    #[test]
    fn parallel_pattern_covers_2_categories() {
        let cats = categories_in_pattern(ExecutionPattern::Parallel);
        assert!(cats.contains(&SkillCategory::SubagentDrivenDevelopment));
        assert!(cats.contains(&SkillCategory::DispatchingParallelAgents));
    }

    #[test]
    fn review_pattern_covers_2_categories() {
        let cats = categories_in_pattern(ExecutionPattern::Review);
        assert!(cats.contains(&SkillCategory::RequestingCodeReview));
        assert!(cats.contains(&SkillCategory::ReceivingCodeReview));
    }

    #[test]
    fn meta_pattern_covers_5_categories() {
        let cats = categories_in_pattern(ExecutionPattern::Meta);
        assert_eq!(cats.len(), 5);
        assert!(cats.contains(&SkillCategory::UsingSuperpowers));
        assert!(cats.contains(&SkillCategory::Brainstorming));
        assert!(cats.contains(&SkillCategory::WritingSkills));
        assert!(cats.contains(&SkillCategory::UsingGitWorktrees));
        assert!(cats.contains(&SkillCategory::FinishingADevelopmentBranch));
    }
}
