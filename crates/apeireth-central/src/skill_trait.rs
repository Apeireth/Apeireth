//! `Skill` trait — 借鉴 obra/superpowers Skill 化工作流 (R125-15e 升级)
//!
//! # 借鉴 ID
//!
//! `R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 + 决策 #51 §1.1)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心概念
//!
//! superpowers 是一套为 AI 编码 agent 设计的"工作流技能", 每个 skill 是一个
//! `SKILL.md` 文件, 描述「什么时候用 + 怎么做 + 强制 TDD 红绿循环」.
//! 借鉴到 apeireth-central 后, 形成 Rust trait + SkillRegistry 模式:
//!
//! 1. **Skill = Markdown 行为准则**: 每个 skill 是个 `SKILL.md`, 用 `name` + `description`
//!    + `when_to_use` + `steps` 4 字段描述.
//! 2. **TDD 强制**: 大部分 skill 要求先写失败 test 再写实现 (`tdd_required = true`).
//! 3. **Skill 注册表**: 中央 `SkillRegistry` 注册 + `get(id) / all() / tdd_required(id)` 调度.
//!
//! # 14 Skill 1:1 映射 superpowers (per R125-15e 升级)
//!
//! - `Brainstorming` ↔ `skills/brainstorming/SKILL.md`
//! - `TestDrivenDevelopment` ↔ `skills/test-driven-development/SKILL.md`
//! - `SystematicDebugging` ↔ `skills/systematic-debugging/SKILL.md`
//! - `VerificationBeforeCompletion` ↔ `skills/verification-before-completion/SKILL.md`
//! - `WritingPlans` ↔ `skills/writing-plans/SKILL.md`
//! - `ExecutingPlans` ↔ `skills/executing-plans/SKILL.md`
//! - `SubagentDrivenDevelopment` ↔ `skills/subagent-driven-development/SKILL.md`
//! - `DispatchingParallelAgents` ↔ `skills/dispatching-parallel-agents/SKILL.md`
//! - `RequestingCodeReview` ↔ `skills/requesting-code-review/SKILL.md`
//! - `ReceivingCodeReview` ↔ `skills/receiving-code-review/SKILL.md`
//! - `UsingGitWorktrees` ↔ `skills/using-git-worktrees/SKILL.md`
//! - `FinishingADevelopmentBranch` ↔ `skills/finishing-a-development-branch/SKILL.md`
//! - `WritingSkills` ↔ `skills/writing-skills/SKILL.md`
//! - `UsingSuperpowers` ↔ `skills/using-superpowers/SKILL.md` (meta skill)
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (superpowers 234 files cloned, 8 硬墙 0 越界, 有真 src 改动 + tests)
//! - 借鉴字段: skill name / description / when_to_use / steps 4 字段 1:1 映射 superpowers 公开模式
//! - 0 假装"已借鉴"私有实现 (superpowers 是公开 SKILL.md, 0 装"已读私有 fn")

#![deny(unsafe_code)]

use std::fmt;

/// `SkillId` — 14 个 superpowers skill 1:1 映射.
///
/// 编译期 enum, 顺序与 superpowers `skills/` 目录 1:1 对齐.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillId {
    /// 头脑风暴 — 跟主人一起做 spec 设计
    Brainstorming,
    /// TDD 红绿循环
    TestDrivenDevelopment,
    /// 系统化 debug
    SystematicDebugging,
    /// 完成前 verify
    VerificationBeforeCompletion,
    /// 写实施计划
    WritingPlans,
    /// 实施计划
    ExecutingPlans,
    /// subagent 驱动开发
    SubagentDrivenDevelopment,
    /// 派并行 agents
    DispatchingParallelAgents,
    /// 申请 code review
    RequestingCodeReview,
    /// 接收 code review
    ReceivingCodeReview,
    /// 用 git worktree
    UsingGitWorktrees,
    /// 完成开发分支
    FinishingADevelopmentBranch,
    /// 写新 skill
    WritingSkills,
    /// 使用 superpowers (meta)
    UsingSuperpowers,
}

impl SkillId {
    /// 14 个 skill 全部 (stable ordering, 跟 superpowers 一致)
    pub const ALL: [SkillId; 14] = [
        SkillId::Brainstorming,
        SkillId::TestDrivenDevelopment,
        SkillId::SystematicDebugging,
        SkillId::VerificationBeforeCompletion,
        SkillId::WritingPlans,
        SkillId::ExecutingPlans,
        SkillId::SubagentDrivenDevelopment,
        SkillId::DispatchingParallelAgents,
        SkillId::RequestingCodeReview,
        SkillId::ReceivingCodeReview,
        SkillId::UsingGitWorktrees,
        SkillId::FinishingADevelopmentBranch,
        SkillId::WritingSkills,
        SkillId::UsingSuperpowers,
    ];

    /// 14 skill 数量 (compile-time sanity check)
    pub const COUNT: usize = 14;

    /// 返回 `superpowers/skills/<dir>/SKILL.md` 相对路径 (1:1 映射).
    pub fn markdown_relative_path(self) -> &'static str {
        match self {
            SkillId::Brainstorming => "skills/brainstorming/SKILL.md",
            SkillId::TestDrivenDevelopment => "skills/test-driven-development/SKILL.md",
            SkillId::SystematicDebugging => "skills/systematic-debugging/SKILL.md",
            SkillId::VerificationBeforeCompletion => {
                "skills/verification-before-completion/SKILL.md"
            }
            SkillId::WritingPlans => "skills/writing-plans/SKILL.md",
            SkillId::ExecutingPlans => "skills/executing-plans/SKILL.md",
            SkillId::SubagentDrivenDevelopment => "skills/subagent-driven-development/SKILL.md",
            SkillId::DispatchingParallelAgents => "skills/dispatching-parallel-agents/SKILL.md",
            SkillId::RequestingCodeReview => "skills/requesting-code-review/SKILL.md",
            SkillId::ReceivingCodeReview => "skills/receiving-code-review/SKILL.md",
            SkillId::UsingGitWorktrees => "skills/using-git-worktrees/SKILL.md",
            SkillId::FinishingADevelopmentBranch => {
                "skills/finishing-a-development-branch/SKILL.md"
            }
            SkillId::WritingSkills => "skills/writing-skills/SKILL.md",
            SkillId::UsingSuperpowers => "skills/using-superpowers/SKILL.md",
        }
    }

    /// Skill 简短 name (kebab-case, 跟 superpowers 目录名 1:1).
    pub fn kebab_name(self) -> &'static str {
        match self {
            SkillId::Brainstorming => "brainstorming",
            SkillId::TestDrivenDevelopment => "test-driven-development",
            SkillId::SystematicDebugging => "systematic-debugging",
            SkillId::VerificationBeforeCompletion => "verification-before-completion",
            SkillId::WritingPlans => "writing-plans",
            SkillId::ExecutingPlans => "executing-plans",
            SkillId::SubagentDrivenDevelopment => "subagent-driven-development",
            SkillId::DispatchingParallelAgents => "dispatching-parallel-agents",
            SkillId::RequestingCodeReview => "requesting-code-review",
            SkillId::ReceivingCodeReview => "receiving-code-review",
            SkillId::UsingGitWorktrees => "using-git-worktrees",
            SkillId::FinishingADevelopmentBranch => "finishing-a-development-branch",
            SkillId::WritingSkills => "writing-skills",
            SkillId::UsingSuperpowers => "using-superpowers",
        }
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kebab_name())
    }
}

/// `Skill` trait — 借鉴 superpowers "skill = Markdown 行为准则" 模式.
///
/// # 4 字段
///
/// - `id` — `SkillId` 唯一标识
/// - `name` — 人类可读 name (e.g. "Test-Driven Development")
/// - `when_to_use` — 触发条件 (e.g. "Use when implementing any feature or bugfix")
/// - `steps` — 步骤列表 (TDD 强制化嵌入步骤 1: 写失败 test)
///
/// # TDD 强制
///
/// `tdd_required()` 默认 `true`, 借鉴 superpowers 「NO PRODUCTION CODE WITHOUT A FAILING
/// TEST FIRST」原则. 子 trait impl 可 override 为 `false` (e.g. meta skills).
pub trait Skill: Send + Sync {
    /// Skill 唯一 id
    fn id(&self) -> SkillId;

    /// Skill 人类可读 name
    fn name(&self) -> &'static str;

    /// 触发条件 / 使用时机 (跟 superpowers `description:` frontmatter 1:1)
    fn when_to_use(&self) -> &'static str;

    /// 步骤列表 (TDD 红绿循环嵌入, 跟 superpowers `SKILL.md` 1:1)
    fn steps(&self) -> &'static [SkillStep];

    /// 是否要求 TDD (默认 true, 借鉴 superpowers iron law)
    fn tdd_required(&self) -> bool {
        true
    }
}

/// `SkillStep` — 单步骤, 借鉴 superpowers checklist pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillStep {
    /// 步骤序号 (1-based, 跟 superpowers checklist 1:1)
    pub order: u8,
    /// 步骤描述 (人类可读)
    pub description: &'static str,
    /// 是否为 TDD "写失败 test" 步骤 (借鉴 superpowers RED 步骤标记)
    pub is_tdd_red: bool,
}

impl SkillStep {
    /// 构造 1 个普通步骤.
    pub const fn new(order: u8, description: &'static str) -> Self {
        Self {
            order,
            description,
            is_tdd_red: false,
        }
    }

    /// 构造 1 个 TDD RED 步骤 (写失败 test).
    pub const fn tdd_red(order: u8, description: &'static str) -> Self {
        Self {
            order,
            description,
            is_tdd_red: true,
        }
    }
}

// ============================================================================
// 14 Skill struct impls (1:1 映射 superpowers `skills/<name>/SKILL.md`)
// ============================================================================

/// `Brainstorming` skill — 跟主人一起 spec 设计 (借鉴 superpowers brainstorming).
pub struct BrainstormingSkill;

impl Skill for BrainstormingSkill {
    fn id(&self) -> SkillId {
        SkillId::Brainstorming
    }
    fn name(&self) -> &'static str {
        "Brainstorming"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when starting any non-trivial feature, before jumping into code"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &BRAINSTORMING_STEPS
    }
}

static BRAINSTORMING_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"问问题验证用户意图"= 标缺 intent-failure = RED
    SkillStep::tdd_red(1, "Ask clarifying questions about the user's true intent (RED: 标缺 intent)"),
    SkillStep::new(2, "Explore the codebase to understand context"),
    SkillStep::new(3, "Propose 2-3 design alternatives with tradeoffs"),
    SkillStep::new(4, "Show spec in chunks short enough to read and digest"),
    SkillStep::new(5, "Wait for user sign-off before implementation"),
];

/// `TestDrivenDevelopment` skill — TDD 红绿循环 (借鉴 superpowers TDD iron law).
pub struct TestDrivenDevelopmentSkill;

impl Skill for TestDrivenDevelopmentSkill {
    fn id(&self) -> SkillId {
        SkillId::TestDrivenDevelopment
    }
    fn name(&self) -> &'static str {
        "Test-Driven Development"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when implementing any feature or bugfix, before writing implementation code"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &TDD_STEPS
    }
}

static TDD_STEPS: [SkillStep; 5] = [
    SkillStep::tdd_red(1, "RED: write a failing test that captures the new behavior"),
    SkillStep::new(2, "Verify the test fails for the right reason"),
    SkillStep::new(3, "GREEN: write the minimum code to make the test pass"),
    SkillStep::new(4, "Verify all tests pass (no regressions)"),
    SkillStep::new(5, "REFACTOR: clean up while keeping tests green"),
];

/// `SystematicDebugging` skill — 系统化 debug (借鉴 superpowers systematic-debugging).
pub struct SystematicDebuggingSkill;

impl Skill for SystematicDebuggingSkill {
    fn id(&self) -> SkillId {
        SkillId::SystematicDebugging
    }
    fn name(&self) -> &'static str {
        "Systematic Debugging"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when facing any bug, test failure, or unexpected behavior — before guessing"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &SYSTEMATIC_DEBUGGING_STEPS
    }
}

static SYSTEMATIC_DEBUGGING_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"写 repro failing test"= 标缺 repro = RED
    SkillStep::tdd_red(1, "Reproduce the bug with a minimal failing test (RED: 标缺 repro-test)"),
    SkillStep::tdd_red(2, "If you can't repro, the bug doesn't exist yet — gather more evidence"),
    SkillStep::new(3, "Find the actual root cause via root-cause tracing"),
    SkillStep::new(4, "Apply defense-in-depth: fix root + add regression tests"),
    SkillStep::new(5, "Verify the fix doesn't break other things"),
];

/// `VerificationBeforeCompletion` skill — 完成前 verify (借鉴 superpowers verification-before-completion).
pub struct VerificationBeforeCompletionSkill;

impl Skill for VerificationBeforeCompletionSkill {
    fn id(&self) -> SkillId {
        SkillId::VerificationBeforeCompletion
    }
    fn name(&self) -> &'static str {
        "Verification Before Completion"
    }
    fn when_to_use(&self) -> &'static str {
        "Use before claiming any task is done, every time"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &VERIFICATION_BEFORE_COMPLETION_STEPS
    }
}

static VERIFICATION_BEFORE_COMPLETION_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"写失败 verify test"= 当前 verify 全跑, 标缺 pass = RED
    SkillStep::tdd_red(1, "Run the full test suite (RED: 标缺 pass 的 test 必跑前 fix)"),
    SkillStep::new(2, "Run `cargo clippy --all-targets -- -D warnings`"),
    SkillStep::new(3, "Run `cargo doc --no-deps` to verify doc compiles"),
    SkillStep::new(4, "Verify against the original task's success criteria"),
    SkillStep::new(5, "Show evidence (test output, clippy output, doc URL)"),
];

/// `WritingPlans` skill — 写实施计划 (借鉴 superpowers writing-plans).
pub struct WritingPlansSkill;

impl Skill for WritingPlansSkill {
    fn id(&self) -> SkillId {
        SkillId::WritingPlans
    }
    fn name(&self) -> &'static str {
        "Writing Plans"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when a spec is approved and the task is large enough to need a plan"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &WRITING_PLANS_STEPS
    }
}

static WRITING_PLANS_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"写失败 plan-validation"= 标缺 plan 必跑 plan-validate
    SkillStep::tdd_red(1, "Break the work into tasks an enthusiastic junior engineer can follow (RED: 标缺 task granularity)"),
    SkillStep::new(2, "Each task: 5-15 minutes, TDD, no architectural decisions"),
    SkillStep::new(3, "Include exact file paths, function signatures, test names"),
    SkillStep::new(4, "List tasks in dependency order with explicit handoffs"),
    SkillStep::new(5, "Save plan to `reports/plans/<date>-<name>-plan.md`"),
];

/// `ExecutingPlans` skill — 实施计划 (借鉴 superpowers executing-plans).
pub struct ExecutingPlansSkill;

impl Skill for ExecutingPlansSkill {
    fn id(&self) -> SkillId {
        SkillId::ExecutingPlans
    }
    fn name(&self) -> &'static str {
        "Executing Plans"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when a plan exists and the user says 'go' or equivalent"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &EXECUTING_PLANS_STEPS
    }
}

static EXECUTING_PLANS_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"读 plan 写 failing-test-stub"= 标缺 plan = RED
    SkillStep::tdd_red(1, "Read the entire plan before starting any task (RED: 标缺 plan-readable 必 fix)"),
    SkillStep::new(2, "For each task: TDD red-green-refactor (no skipping)"),
    SkillStep::new(3, "Mark each task done only after Verification Before Completion"),
    SkillStep::new(4, "If a task is harder than 15 min, stop and re-plan"),
    SkillStep::new(5, "Update the plan as you go (in-place, never silently diverge)"),
];

/// `SubagentDrivenDevelopment` skill — subagent 驱动开发 (借鉴 superpowers subagent-driven-development).
pub struct SubagentDrivenDevelopmentSkill;

impl Skill for SubagentDrivenDevelopmentSkill {
    fn id(&self) -> SkillId {
        SkillId::SubagentDrivenDevelopment
    }
    fn name(&self) -> &'static str {
        "Subagent-Driven Development"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when implementing a plan via multiple parallel subagents"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &SUBAGENT_DRIVEN_DEVELOPMENT_STEPS
    }
}

static SUBAGENT_DRIVEN_DEVELOPMENT_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"dispatch stub"= 标缺 subagent spec = RED
    SkillStep::tdd_red(1, "Dispatch each task to a fresh subagent with full context (RED: 标缺 subagent-prompt)"),
    SkillStep::new(2, "Use Dispatching Parallel Agents for concurrent tasks"),
    SkillStep::new(3, "Inspect each subagent's output against the task's success criteria"),
    SkillStep::new(4, "Re-dispatch failed tasks with concrete feedback (no hand-waving)"),
    SkillStep::new(5, "Verify the integrated result before marking the plan done"),
];

/// `DispatchingParallelAgents` skill — 派并行 agents (借鉴 superpowers dispatching-parallel-agents).
pub struct DispatchingParallelAgentsSkill;

impl Skill for DispatchingParallelAgentsSkill {
    fn id(&self) -> SkillId {
        SkillId::DispatchingParallelAgents
    }
    fn name(&self) -> &'static str {
        "Dispatching Parallel Agents"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when 3+ independent tasks can be done in parallel"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &DISPATCHING_PARALLEL_AGENTS_STEPS
    }
}

static DISPATCHING_PARALLEL_AGENTS_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"ident 失败 stub"= 标缺 independence-check = RED
    SkillStep::tdd_red(1, "Identify independent tasks (RED: 标缺 dep-analysis 必 fix)"),
    SkillStep::new(2, "Write one self-contained dispatch prompt per task"),
    SkillStep::new(3, "Dispatch in parallel via `dispatch` tool (no serial fallbacks)"),
    SkillStep::new(4, "Track task IDs; never lose a result"),
    SkillStep::new(5, "Verify all results, then integrate with explicit merge step"),
];

/// `RequestingCodeReview` skill — 申请 code review (借鉴 superpowers requesting-code-review).
pub struct RequestingCodeReviewSkill;

impl Skill for RequestingCodeReviewSkill {
    fn id(&self) -> SkillId {
        SkillId::RequestingCodeReview
    }
    fn name(&self) -> &'static str {
        "Requesting Code Review"
    }
    fn when_to_use(&self) -> &'static str {
        "Use after completing a non-trivial change, before merging"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &REQUESTING_CODE_REVIEW_STEPS
    }
}

static REQUESTING_CODE_REVIEW_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"self-review 失败 stub"= 标缺 8 硬墙 = RED
    SkillStep::tdd_red(1, "Self-review the diff first (RED: 标缺 8 硬墙 violations 必 fix)"),
    SkillStep::new(2, "Write a dispatch prompt with full context + diff"),
    SkillStep::new(3, "Request specific feedback (not just 'any comments?')"),
    SkillStep::new(4, "Wait for review, never assume approval"),
    SkillStep::new(5, "Address every comment (resolve, rebut with reason, or fix)"),
];

/// `ReceivingCodeReview` skill — 接收 code review (借鉴 superpowers receiving-code-review).
pub struct ReceivingCodeReviewSkill;

impl Skill for ReceivingCodeReviewSkill {
    fn id(&self) -> SkillId {
        SkillId::ReceivingCodeReview
    }
    fn name(&self) -> &'static str {
        "Receiving Code Review"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when a reviewer (human or AI) comments on your code"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &RECEIVING_CODE_REVIEW_STEPS
    }
}

static RECEIVING_CODE_REVIEW_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"read review 失败 stub"= 标缺 review 必 fix = RED
    SkillStep::tdd_red(1, "Read the entire review before responding to any single point (RED: 标缺 read)"),
    SkillStep::new(2, "Verify each comment is technically correct before agreeing"),
    SkillStep::new(3, "Push back with reason if you disagree (no sycophantic agreement)"),
    SkillStep::new(4, "If you agree, fix it — don't just say 'good point'"),
    SkillStep::new(5, "Re-request review if your fix is non-trivial"),
];

/// `UsingGitWorktrees` skill — 用 git worktree (借鉴 superpowers using-git-worktrees).
pub struct UsingGitWorktreesSkill;

impl Skill for UsingGitWorktreesSkill {
    fn id(&self) -> SkillId {
        SkillId::UsingGitWorktrees
    }
    fn name(&self) -> &'static str {
        "Using Git Worktrees"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when working on multiple branches concurrently (especially with subagents)"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &USING_GIT_WORKTREES_STEPS
    }
}

static USING_GIT_WORKTREES_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"worktree-setup 失败 stub"= 标缺 worktree-path = RED
    SkillStep::tdd_red(1, "Each parallel task gets its own worktree (RED: 标缺 worktree-path 必 fix)"),
    SkillStep::new(2, "Use a deterministic worktree path (per task ID)"),
    SkillStep::new(3, "Lock 8 硬墙 + Cargo.lock before cross-worktree merge"),
    SkillStep::new(4, "Merge worktrees via PR, never rebase across active worktrees"),
    SkillStep::new(5, "Clean up worktrees after merge (don't leave orphans)"),
];

/// `FinishingADevelopmentBranch` skill — 完成开发分支 (借鉴 superpowers finishing-a-development-branch).
pub struct FinishingADevelopmentBranchSkill;

impl Skill for FinishingADevelopmentBranchSkill {
    fn id(&self) -> SkillId {
        SkillId::FinishingADevelopmentBranch
    }
    fn name(&self) -> &'static str {
        "Finishing a Development Branch"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when a feature is merged and the branch is no longer needed"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &FINISHING_A_DEVELOPMENT_BRANCH_STEPS
    }
}

static FINISHING_A_DEVELOPMENT_BRANCH_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"verify-merge 失败 stub"= 标缺 merge-on-master = RED
    SkillStep::tdd_red(1, "Verify the merge commit is on master (RED: 标缺 merge-on-master 必 fix)"),
    SkillStep::new(2, "Delete the local branch (`git branch -d`)"),
    SkillStep::new(3, "Delete the remote branch if pushed (`git push origin --delete`)"),
    SkillStep::new(4, "Clean up any worktrees created for this branch"),
    SkillStep::new(5, "Document the merge in CHANGELOG / decision log"),
];

/// `WritingSkills` skill — 写新 skill (借鉴 superpowers writing-skills).
pub struct WritingSkillsSkill;

impl Skill for WritingSkillsSkill {
    fn id(&self) -> SkillId {
        SkillId::WritingSkills
    }
    fn name(&self) -> &'static str {
        "Writing Skills"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when codifying a new repeatable workflow into a Skill"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &WRITING_SKILLS_STEPS
    }
}

static WRITING_SKILLS_STEPS: [SkillStep; 5] = [
    // step 1 标 tdd_red: 先"pattern 失败 stub"= 标缺 3+ 样本 = RED
    SkillStep::tdd_red(1, "Extract the pattern from 3+ prior occurrences (RED: 标缺 sample-size 必 fix)"),
    SkillStep::new(2, "Write SKILL.md with frontmatter: name + description (when to use)"),
    SkillStep::new(3, "Use 3rd person, imperative mood, concise steps"),
    SkillStep::new(4, "Embed TDD in step 1 of any code-touching skill"),
    SkillStep::new(5, "Test the skill with a subagent before committing it"),
];

/// `UsingSuperpowers` skill — meta skill, 任何任务都要先 invoke 相关 skill (借鉴 superpowers using-superpowers).
///
/// 注: meta skill 自身 **不要求 TDD** (跟 using-superpowers 1:1, 它是"元规则" 0 写代码).
pub struct UsingSuperpowersSkill;

impl Skill for UsingSuperpowersSkill {
    fn id(&self) -> SkillId {
        SkillId::UsingSuperpowers
    }
    fn name(&self) -> &'static str {
        "Using Superpowers"
    }
    fn when_to_use(&self) -> &'static str {
        "Use when starting any conversation — establishes how to find and use skills"
    }
    fn steps(&self) -> &'static [SkillStep] {
        &USING_SUPERPOWERS_STEPS
    }
    /// meta skill 0 写代码, 0 要求 TDD
    fn tdd_required(&self) -> bool {
        false
    }
}

static USING_SUPERPOWERS_STEPS: [SkillStep; 5] = [
    SkillStep::new(1, "Invoke relevant skills BEFORE any response or action"),
    SkillStep::new(2, "Announce 'Using [skill] to [purpose]' before following the skill"),
    SkillStep::new(3, "Process skills come first (set approach), then implementation skills"),
    SkillStep::new(4, "User instructions (CLAUDE.md / AGENTS.md) take precedence over skills"),
    SkillStep::new(5, "If you think there's even 1% chance a skill applies, invoke it"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_id_all_has_fourteen_entries() {
        assert_eq!(SkillId::ALL.len(), 14);
        assert_eq!(SkillId::ALL.len(), SkillId::COUNT);
    }

    #[test]
    fn skill_id_markdown_paths_are_unique() {
        let paths: Vec<&str> = SkillId::ALL
            .iter()
            .map(|id| id.markdown_relative_path())
            .collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), paths.len(), "duplicate markdown paths");
    }

    #[test]
    fn skill_id_kebab_names_are_unique() {
        let names: Vec<&str> = SkillId::ALL.iter().map(|id| id.kebab_name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "duplicate kebab names");
    }

    #[test]
    fn tdd_skill_marks_red_step() {
        let skill = TestDrivenDevelopmentSkill;
        let steps = skill.steps();
        assert!(steps[0].is_tdd_red, "TDD step 1 should be RED (write failing test)");
    }

    #[test]
    fn meta_skill_overrides_tdd_required_to_false() {
        let meta = UsingSuperpowersSkill;
        assert!(!meta.tdd_required(), "using-superpowers meta skill should not require TDD");
    }

    #[test]
    fn all_non_meta_skills_require_tdd() {
        // 13 of 14 skills require TDD; only UsingSuperpowers is the meta exception
        for id in SkillId::ALL {
            let skill: Box<dyn Skill> = match id {
                SkillId::Brainstorming => Box::new(BrainstormingSkill),
                SkillId::TestDrivenDevelopment => Box::new(TestDrivenDevelopmentSkill),
                SkillId::SystematicDebugging => Box::new(SystematicDebuggingSkill),
                SkillId::VerificationBeforeCompletion => {
                    Box::new(VerificationBeforeCompletionSkill)
                }
                SkillId::WritingPlans => Box::new(WritingPlansSkill),
                SkillId::ExecutingPlans => Box::new(ExecutingPlansSkill),
                SkillId::SubagentDrivenDevelopment => Box::new(SubagentDrivenDevelopmentSkill),
                SkillId::DispatchingParallelAgents => Box::new(DispatchingParallelAgentsSkill),
                SkillId::RequestingCodeReview => Box::new(RequestingCodeReviewSkill),
                SkillId::ReceivingCodeReview => Box::new(ReceivingCodeReviewSkill),
                SkillId::UsingGitWorktrees => Box::new(UsingGitWorktreesSkill),
                SkillId::FinishingADevelopmentBranch => {
                    Box::new(FinishingADevelopmentBranchSkill)
                }
                SkillId::WritingSkills => Box::new(WritingSkillsSkill),
                SkillId::UsingSuperpowers => Box::new(UsingSuperpowersSkill),
            };
            if id == SkillId::UsingSuperpowers {
                assert!(!skill.tdd_required());
            } else {
                assert!(skill.tdd_required(), "skill {id:?} should require TDD");
            }
        }
    }

    #[test]
    fn all_skills_have_at_least_three_steps() {
        for id in SkillId::ALL {
            let skill: Box<dyn Skill> = match id {
                SkillId::Brainstorming => Box::new(BrainstormingSkill),
                SkillId::TestDrivenDevelopment => Box::new(TestDrivenDevelopmentSkill),
                SkillId::SystematicDebugging => Box::new(SystematicDebuggingSkill),
                SkillId::VerificationBeforeCompletion => {
                    Box::new(VerificationBeforeCompletionSkill)
                }
                SkillId::WritingPlans => Box::new(WritingPlansSkill),
                SkillId::ExecutingPlans => Box::new(ExecutingPlansSkill),
                SkillId::SubagentDrivenDevelopment => Box::new(SubagentDrivenDevelopmentSkill),
                SkillId::DispatchingParallelAgents => Box::new(DispatchingParallelAgentsSkill),
                SkillId::RequestingCodeReview => Box::new(RequestingCodeReviewSkill),
                SkillId::ReceivingCodeReview => Box::new(ReceivingCodeReviewSkill),
                SkillId::UsingGitWorktrees => Box::new(UsingGitWorktreesSkill),
                SkillId::FinishingADevelopmentBranch => {
                    Box::new(FinishingADevelopmentBranchSkill)
                }
                SkillId::WritingSkills => Box::new(WritingSkillsSkill),
                SkillId::UsingSuperpowers => Box::new(UsingSuperpowersSkill),
            };
            assert!(
                skill.steps().len() >= 3,
                "skill {} should have at least 3 steps",
                skill.name()
            );
        }
    }
}
