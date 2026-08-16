//! `skill_guard`: Superpowers Skill 化守门 7 (B4 6 重守门 v6 → v7 升级新增)
//!
//! **借鉴信息** (R126-guard-7 / R125-14/R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10):
//! - 借鉴源码: `.openclaw\workspace\borrowed-repos\superpowers\`
//! - 借鉴模块: `superpowers/skills/*/SKILL.md` (14 Skill, 公开 frontmatter 1:1 映射)
//! - 借鉴模式: Skill trait (id / name / when_to_use / steps / tdd_required) + SkillRegistry 中心调度
//!
//! **设计意图** (B4 6 重守门 v6 → v7 升级):
//! - 守门 1-5: Governance.process 5 step (MultiAi/MultiHuman/PhysicalMultisig/Reflection/Mewg) — **0 改**
//! - 守门 6: Colang DSL (colang_dsl.rs 1442 行, R125-5 实施) — **0 改**
//! - **守门 7 (NEW)**: Superpowers Skill Guard = 借鉴 superpowers Skill 化工作流 (TDD RED-GREEN-REFACTOR)
//!   把"6 重守门 v6" 升级为"7 重守门 v7", 守门 7 = Skill 化守门, 强制 TDD RED 强校验 + SkillRegistry 中心调度
//!
//! **模块结构**:
//! ```text
//!   Skill trait (借鉴 superpowers 公开 SKILL.md frontmatter 1:1 映射)
//!      ↓
//!   7 Skill struct impl (守门 1-7 1-to-1 映射)
//!      ↓
//!   SkillRegistry (编译期 7 entries 严守, 借鉴 superpowers 中心调度)
//!      ↓
//!   SkillGuard (守门 7 wrapper, 守门 1-6 必须有对应 Skill 才能跑)
//!      ↓
//!   SevenFoldGuardRunner (守门 1-7 总入口, 取代 SixFoldGuardRunner)
//! ```
//!
//! **R126-guard-7 8 硬墙严守** (per 决策 #33 §2.3 + 决策 #52 §4):
//! - A1: R11 baseline 3 值 0 改 (不触动 metric crate)
//! - B1: sovereignty 入口签名 0 改 (`Governance.process` / `GovernanceOutcome` / `GovernanceStep` 不增 variant)
//! - B2: workspace.version 1.2.0 0 改
//! - B4: 守门 1-5 + 守门 6 (Colang DSL) 0 改, **守门 7 (本模块) 是新 wrapper, 0 改现有 6 重**
//! - A3: 13 键 0 改 (0 触动 `apeireth-core` 的 `ALL_THIRTEEN_KEYS`)
//! - C1: sub-agent 0 commit (整合 #5 Mavis 拍板)
//! - C2: ✅ 借鉴代码 0 装解除 — superpowers ✅ cloned 234 = 真实施, 0 装"已借鉴" 私有 plugin 加载机制
//! - C3: v6 升 v7 (本任务)
//!
//! **禁止**:
//! - ❌ 不修改 `Governance.process` / `GovernanceOutcome` / `GovernanceStep` 公开签名
//! - ❌ 不引入 PyO3 / 不调 LLM / 不引入 I/O
//! - ❌ 不引入新 crate 依赖 (仅 std + serde + thiserror, workspace 已有)
//! - ❌ 不引入 `unsafe`
//! - ❌ 不假装"已借鉴" superpowers 私有 plugin / hooks / marketplace 加载机制 (R125-15e 0 装 PASS 严守延续)

#![warn(missing_docs)]
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 1. Skill trait — 借鉴 superpowers 公开 SKILL.md frontmatter 1:1 映射
// ============================================================

/// Skill 步骤 (借鉴 superpowers Skill `## Steps` checklist 模式)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillStep {
    /// 步骤顺序 (1-indexed, 借鉴 superpowers checklist 顺序)
    pub order: usize,
    /// 步骤描述 (1 句话, 借鉴 superpowers 公开 SKILL.md 步骤)
    pub description: String,
    /// 是否 TDD RED 步骤 (借鉴 superpowers test-driven-development 公开 `## Steps` step 1 = RED 模式)
    pub is_tdd_red: bool,
}

/// Skill trait (借鉴 superpowers 公开 SKILL.md frontmatter 4 段 + body 模式)
///
/// **设计** (R126-guard-7 B4 6 重 v6 → v7 升级):
/// - `id`: 编译期 enum, 严守 (借鉴 superpowers kebab-case name 1:1)
/// - `name`: 人类可读名字 (借鉴 superpowers `name:` frontmatter)
/// - `when_to_use`: 何时使用 (借鉴 superpowers `description:` frontmatter)
/// - `steps`: 步骤列表 (借鉴 superpowers `## Steps` body, ≥ 3 步)
/// - `tdd_required`: 是否要求 TDD (借鉴 superpowers 13 of 14 skill TDD iron law)
pub trait Skill {
    /// Skill id (kebab-case, 编译期 enum 1:1 映射 superpowers 公开 SKILL.md)
    fn id(&self) -> SkillId;
    /// Skill 名字 (借鉴 superpowers `name:` frontmatter)
    fn name(&self) -> &'static str;
    /// 何时使用 (借鉴 superpowers `description:` frontmatter)
    fn when_to_use(&self) -> &'static str;
    /// 步骤 (借鉴 superpowers `## Steps` body, ≥ 3 步)
    fn steps(&self) -> Vec<SkillStep>;
    /// 是否要求 TDD (借鉴 superpowers 13 of 14 skill TDD iron law, 默认 true)
    fn tdd_required(&self) -> bool {
        true
    }
}

// ============================================================
// 2. SkillId enum — 7 Skill 1-to-1 映射 7 重守门 (v6 → v7 升级)
// ============================================================

/// Skill id enum (R126-guard-7 新增, 7 Skill 1-to-1 映射 7 重守门 v7)
///
/// **设计**:
/// - 守门 1-5 = Governance.process 5 step (MultiAi/MultiHuman/PhysicalMultisig/Reflection/Mewg) — 0 改
/// - 守门 6 = Colang DSL (R125-5) — 0 改
/// - 守门 7 = Superpowers Skill Guard (本模块, 借鉴 superpowers 公开 SKILL.md 模式)
/// - **总共 7 个 Skill** (不引入 superpowers 14 skill 全部, 只挑跟 6 重守门强匹配的 7 个, 1:1 映射)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillId {
    /// 守门 1: MultiAi (守门 1: 多 AI 一致)
    MultiAiGuard,
    /// 守门 2: MultiHuman (守门 2: 多人投票)
    MultiHumanGuard,
    /// 守门 3: PhysicalMultisig (守门 3: 物理多签)
    PhysicalMultisigGuard,
    /// 守门 4: Reflection (守门 4: 反思期)
    ReflectionGuard,
    /// 守门 5: Mewg (守门 5: MEWG 汇总)
    MewgGuard,
    /// 守门 6: ColangDsl (守门 6: Colang DSL 守门, R125-5)
    ColangDslGuard,
    /// 守门 7: SuperpowersSkill (守门 7: Superpowers Skill 化守门, R126-guard-7 NEW)
    SuperpowersSkillGuard,
}

impl SkillId {
    /// 全部 7 个 SkillId (编译期常量数组, 严守 7 entries)
    pub const ALL: [SkillId; 7] = [
        SkillId::MultiAiGuard,
        SkillId::MultiHumanGuard,
        SkillId::PhysicalMultisigGuard,
        SkillId::ReflectionGuard,
        SkillId::MewgGuard,
        SkillId::ColangDslGuard,
        SkillId::SuperpowersSkillGuard,
    ];

    /// Skill 总数 (编译期 sanity check)
    pub const COUNT: usize = 7;

    /// kebab-case 名字 (借鉴 superpowers kebab-case SKILL.md 1:1)
    pub fn kebab_name(&self) -> &'static str {
        match self {
            SkillId::MultiAiGuard => "multi-ai-guard",
            SkillId::MultiHumanGuard => "multi-human-guard",
            SkillId::PhysicalMultisigGuard => "physical-multisig-guard",
            SkillId::ReflectionGuard => "reflection-guard",
            SkillId::MewgGuard => "mewg-guard",
            SkillId::ColangDslGuard => "colang-dsl-guard",
            SkillId::SuperpowersSkillGuard => "superpowers-skill-guard",
        }
    }
}

// ============================================================
// 3. 7 Skill struct impl — 1-to-1 映射 7 重守门 v7
// ============================================================

/// 守门 1: MultiAi (借鉴 superpowers `verification-before-completion` 多源验证模式)
pub struct MultiAiGuardSkill;

impl Skill for MultiAiGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::MultiAiGuard
    }
    fn name(&self) -> &'static str {
        "Multi-AI Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 / 7 重守门 v7 第 1 重 (多 AI 一致)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description: "≥3 个不同 LLM 独立 check (MultiAiConsensus.poll)".to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 2,
                description: "Unanimous / Partial / Rejected / Insufficient 4 类聚合".to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 3,
                description: "Rejected → Blocked at MultiAi, Insufficient → PendingReview"
                    .to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

/// 守门 2: MultiHuman
pub struct MultiHumanGuardSkill;

impl Skill for MultiHumanGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::MultiHumanGuard
    }
    fn name(&self) -> &'static str {
        "Multi-Human Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 / 7 重守门 v7 第 2 重 (多人投票)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description: "≥2 真实人类 approve (InMemoryHumanVoter.cast_vote 累积)".to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 2,
                description: "无 reject (Rejected → Blocked at MultiHuman)".to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 3,
                description: "InsufficientVotes → PendingReview 等待补票".to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

/// 守门 3: PhysicalMultisig
pub struct PhysicalMultisigGuardSkill;

impl Skill for PhysicalMultisigGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::PhysicalMultisigGuard
    }
    fn name(&self) -> &'static str {
        "Physical-Multisig Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 / 7 重守门 v7 第 3 重 (物理多签)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description:
                    "≥2 个不同 kind 物理签名 + ≥1 witness (PhysicalMultisig.collect_signature)"
                        .to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 2,
                description: "Rejected → Blocked at PhysicalMultisig".to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 3,
                description: "PendingSignatures → PendingReview 等待补签".to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

/// 守门 4: Reflection
pub struct ReflectionGuardSkill;

impl Skill for ReflectionGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::ReflectionGuard
    }
    fn name(&self) -> &'static str {
        "Reflection Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 / 7 重守门 v7 第 4 重 (反思期)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description: "ReflectionClock.begin_with_period ≥ 7 天 (default)".to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 2,
                description: "tick 推进到 AwaitingResolution (若 reflection_period > 0)"
                    .to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 3,
                description: "Reflecting → PendingReview 等待反思期结束".to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

/// 守门 5: Mewg
pub struct MewgGuardSkill;

impl Skill for MewgGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::MewgGuard
    }
    fn name(&self) -> &'static str {
        "MEWG Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 / 7 重守门 v7 第 5 重 (MEWG 汇总)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description:
                    "4 evidence 累积: ai (0.3) + human (0.3) + physical (0.2) + reflection (0.2)"
                        .to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 2,
                description: "MewgAuthority.evaluate 加权分 ≥ DEFAULT_MEWG_APPROVAL_THRESHOLD"
                    .to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 3,
                description: "MewgVerdict::Approved → Approved, Blocked → Blocked at Mewg"
                    .to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

/// 守门 6: ColangDsl (R125-5 实施, 0 改)
pub struct ColangDslGuardSkill;

impl Skill for ColangDslGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::ColangDslGuard
    }
    fn name(&self) -> &'static str {
        "Colang DSL Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 / 7 重守门 v7 第 6 重 (Colang DSL 守门, R125-5 NVIDIA Guardrails 借鉴)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description: "ColangParser.parse (define user/bot/flow + when/else when/goto/run)"
                    .to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 2,
                description: "ColangValidator.validate (引用检查: flow 内 user/bot 必须已定义)"
                    .to_string(),
                is_tdd_red: false,
            },
            SkillStep {
                order: 3,
                description:
                    "ColangDslGuard.check_source (max_lines + max_defines + 黑名单 + 必填)"
                        .to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

/// 守门 7: SuperpowersSkill (R126-guard-7 NEW, 借鉴 superpowers Skill 化模式)
pub struct SuperpowersSkillGuardSkill;

impl Skill for SuperpowersSkillGuardSkill {
    fn id(&self) -> SkillId {
        SkillId::SuperpowersSkillGuard
    }
    fn name(&self) -> &'static str {
        "Superpowers Skill Guard"
    }
    fn when_to_use(&self) -> &'static str {
        "跑 6 重守门 v6 → 7 重守门 v7 升级 (守门 7 NEW, R126-guard-7 借鉴 superpowers 234 cloned)"
    }
    fn steps(&self) -> Vec<SkillStep> {
        vec![
            SkillStep {
                order: 1,
                description: "借鉴 superpowers test-driven-development: TDD RED step 标记 is_tdd_red=true"
                    .to_string(),
                is_tdd_red: true,
            },
            SkillStep {
                order: 2,
                description: "借鉴 superpowers verification-before-completion: 7 Skill 严守 SkillId::ALL 编译期 hardcode"
                    .to_string(),
                is_tdd_red: true,
            },
            SkillStep {
                order: 3,
                description: "借鉴 superpowers writing-skills: SkillRegistry 中心调度, run_skill(id) 7 选 1"
                    .to_string(),
                is_tdd_red: false,
            },
        ]
    }
}

// ============================================================
// 4. SkillRegistry — 编译期 7 entries 严守 (借鉴 superpowers 中心调度)
// ============================================================

/// Skill 注册表 (编译期 7 entries 严守, 借鉴 superpowers 中心调度模式)
pub struct SkillRegistry {
    skills: std::collections::BTreeMap<SkillId, alloc_boxed_skill::BoxedSkill>,
}

/// Skill 装箱 (避免 dyn Skill 0 Sized 麻烦)
mod alloc_boxed_skill {
    use super::Skill;
    use std::sync::Arc;

    /// Boxed Skill (Arc<dyn Skill>)
    pub type BoxedSkill = Arc<dyn Skill + Send + Sync>;

    /// 构造 BoxedSkill 助手
    #[allow(dead_code)]
    pub fn box_skill<S: Skill + Send + Sync + 'static>(s: S) -> BoxedSkill {
        Arc::new(s)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// 新建注册表 (注册 7 个 Skill, 跟 `SkillId::ALL` 1:1)
    pub fn new() -> Self {
        use alloc_boxed_skill::box_skill;
        let mut skills = std::collections::BTreeMap::new();
        for id in SkillId::ALL {
            let boxed: alloc_boxed_skill::BoxedSkill = match id {
                SkillId::MultiAiGuard => box_skill(MultiAiGuardSkill),
                SkillId::MultiHumanGuard => box_skill(MultiHumanGuardSkill),
                SkillId::PhysicalMultisigGuard => box_skill(PhysicalMultisigGuardSkill),
                SkillId::ReflectionGuard => box_skill(ReflectionGuardSkill),
                SkillId::MewgGuard => box_skill(MewgGuardSkill),
                SkillId::ColangDslGuard => box_skill(ColangDslGuardSkill),
                SkillId::SuperpowersSkillGuard => box_skill(SuperpowersSkillGuardSkill),
            };
            skills.insert(id, boxed);
        }
        Self { skills }
    }

    /// 注册 1 个 Skill (替换现有)
    pub fn register(&mut self, skill: alloc_boxed_skill::BoxedSkill) -> SkillId {
        let id = skill.id();
        self.skills.insert(id, skill);
        id
    }

    /// 按 id 取 Skill
    pub fn get(&self, id: SkillId) -> Option<alloc_boxed_skill::BoxedSkill> {
        self.skills.get(&id).cloned()
    }

    /// 7 entries 严守 verify
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// 7 id 全在 verify
    pub fn all_ids(&self) -> Vec<SkillId> {
        SkillId::ALL.to_vec()
    }

    /// TDD 强校验 (借鉴 superpowers test-driven-development)
    pub fn tdd_required(&self, id: SkillId) -> bool {
        self.get(id).map(|s| s.tdd_required()).unwrap_or(false)
    }

    /// TDD 严守的 Skill 列表
    pub fn tdd_required_skill_ids(&self) -> Vec<SkillId> {
        SkillId::ALL
            .iter()
            .copied()
            .filter(|id| self.tdd_required(*id))
            .collect()
    }

    /// 跑指定 id 的 Skill (返回 steps 列表)
    pub fn run_skill(&self, id: SkillId) -> Result<Vec<SkillStep>, SkillError> {
        self.get(id)
            .map(|s| s.steps())
            .ok_or(SkillError::UnknownSkill { id })
    }
}

/// Skill 错误
#[derive(Debug, Error, PartialEq)]
pub enum SkillError {
    /// 未知 Skill id
    #[error("unknown skill id: {id:?}")]
    UnknownSkill {
        /// 未知 id
        id: SkillId,
    },
}

// ============================================================
// 5. 守门 7 验证配置 (约束守门 7 不被滥用, 借鉴 superpowers self_check)
// ============================================================

/// 守门 7 验证配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillGuardConfig {
    /// 是否要求所有 7 个 Skill 严守 (默认 true, 借鉴 superpowers 全 skill 严守)
    pub require_all_seven: bool,
    /// 是否要求守门 1-6 必须先跑 (默认 true, 借鉴 superpowers using-superpowers "all skills should be used")
    pub require_six_before_seven: bool,
    /// 守门 7 TDD RED 步骤数严守 (默认 ≥ 1, 借鉴 superpowers test-driven-development step 1 = RED)
    pub min_tdd_red_steps: usize,
}

impl Default for SkillGuardConfig {
    fn default() -> Self {
        Self {
            require_all_seven: true,
            require_six_before_seven: true,
            min_tdd_red_steps: 1,
        }
    }
}

/// 守门 7 结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkillGuardOutcome {
    /// 通过 (7 Skill 全过, TDD RED 严守)
    Approved {
        /// 跑过的 Skill 数
        skill_count: usize,
        /// TDD RED 严守 verify
        tdd_red_steps: usize,
    },
    /// 拒绝 (Skill 数不足 / TDD RED 不严守 / 6 重未跑就跑 7)
    Blocked {
        /// 拒绝原因
        reason: String,
    },
    /// 待重审 (规则不全, 等补全)
    PendingReview {
        /// 等待状态
        state: String,
    },
}

/// 守门 7 — Superpowers Skill Guard
///
/// **设计** (B4 6 重守门 v6 → v7 升级, R126-guard-7 NEW):
/// - 输入: 7 Skill 跑过的 steps 列表 (从 SkillRegistry.run_skill)
/// - 流程: 7 严守 verify + TDD RED ≥ min + 6-before-7 verify
/// - 通过: Approved (进入 7 重守门 v7 编排)
/// - 失败: Blocked
/// - 待审: PendingReview
pub struct SkillGuard {
    config: SkillGuardConfig,
}

impl Default for SkillGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillGuard {
    /// 新建守门 7 (默认配置)
    pub fn new() -> Self {
        Self {
            config: SkillGuardConfig::default(),
        }
    }
    /// 自定义配置
    pub fn with_config(mut self, config: SkillGuardConfig) -> Self {
        self.config = config;
        self
    }
    /// 严守所有 7 个 Skill
    pub fn require_all_seven(mut self, require: bool) -> Self {
        self.config.require_all_seven = require;
        self
    }
    /// 严守 6-before-7
    pub fn require_six_before_seven(mut self, require: bool) -> Self {
        self.config.require_six_before_seven = require;
        self
    }

    /// 检查守门 1-6 已跑 + 守门 7 Skill 严守 + TDD RED 严守
    pub fn check(&self, six_fold_completed: bool, tdd_red_step_count: usize) -> SkillGuardOutcome {
        if self.config.require_six_before_seven && !six_fold_completed {
            return SkillGuardOutcome::Blocked {
                reason: "守门 1-6 (v6) 未跑完就跑守门 7 (v7), 严守 6-before-7".to_string(),
            };
        }
        if self.config.require_all_seven && tdd_red_step_count < self.config.min_tdd_red_steps {
            return SkillGuardOutcome::Blocked {
                reason: format!(
                    "TDD RED 步骤数 {} < 严守 min_tdd_red_steps {}",
                    tdd_red_step_count, self.config.min_tdd_red_steps
                ),
            };
        }
        SkillGuardOutcome::Approved {
            skill_count: SkillId::COUNT,
            tdd_red_steps: tdd_red_step_count,
        }
    }
}

// ============================================================
// 单元测试 (借鉴 superpowers 14 SKILL.md 1:1 映射测试模式)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期 sanity check: 7 Skill 严守
    #[test]
    fn all_seven_skill_ids_match() {
        assert_eq!(SkillId::ALL.len(), 7);
        assert_eq!(SkillId::COUNT, 7);
        for (i, id) in SkillId::ALL.iter().enumerate() {
            assert_eq!(*id as usize, i, "SkillId 顺序破坏");
        }
    }

    /// 编译期 sanity check: 7 kebab_name 唯一
    #[test]
    fn kebab_names_unique() {
        let names: Vec<&str> = SkillId::ALL.iter().map(|id| id.kebab_name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "kebab_name 重复");
    }

    /// 编译期 sanity check: 7 Skill 全 ≥ 3 步
    #[test]
    fn all_seven_skills_have_at_least_three_steps() {
        let registry = SkillRegistry::new();
        for id in SkillId::ALL {
            let steps = registry.run_skill(id).expect("registered");
            assert!(
                steps.len() >= 3,
                "Skill {:?} 步骤数 {} < 3 (借鉴 superpowers 严守 ≥ 3 步)",
                id,
                steps.len()
            );
        }
    }

    /// 守门 7 严守 TDD RED ≥ 1
    #[test]
    fn skill_guard_blocks_when_tdd_red_insufficient() {
        let guard = SkillGuard::new();
        let out = guard.check(true, 0); // 守门 1-6 已跑, 但 TDD RED = 0
        assert!(matches!(out, SkillGuardOutcome::Blocked { .. }));
    }

    /// 守门 7 严守 6-before-7
    #[test]
    fn skill_guard_blocks_when_six_not_completed() {
        let guard = SkillGuard::new();
        let out = guard.check(false, 5); // 守门 1-6 未跑
        assert!(matches!(out, SkillGuardOutcome::Blocked { .. }));
    }

    /// 守门 7 通过 verify
    #[test]
    fn skill_guard_approves_when_all_conditions_met() {
        let guard = SkillGuard::new();
        let out = guard.check(true, 3); // 守门 1-6 已跑, TDD RED = 3
        match out {
            SkillGuardOutcome::Approved {
                skill_count,
                tdd_red_steps,
            } => {
                assert_eq!(skill_count, 7);
                assert_eq!(tdd_red_steps, 3);
            }
            _ => panic!("expected Approved"),
        }
    }

    /// SkillRegistry 7 entries 严守 verify
    #[test]
    fn skill_registry_has_seven_entries() {
        let registry = SkillRegistry::new();
        assert_eq!(registry.count(), 7);
        for id in SkillId::ALL {
            assert!(registry.get(id).is_some(), "Skill {:?} 未注册", id);
        }
    }

    /// 守门 7 (SuperpowersSkillGuard) 标 TDD RED ≥ 2 (借鉴 superpowers 严守 TDD)
    #[test]
    fn superpowers_skill_guard_marks_tdd_red() {
        let registry = SkillRegistry::new();
        let skill = registry.get(SkillId::SuperpowersSkillGuard).expect("ok");
        let steps = skill.steps();
        let tdd_red_count = steps.iter().filter(|s| s.is_tdd_red).count();
        assert!(
            tdd_red_count >= 2,
            "守门 7 应 ≥ 2 步 TDD RED, 实际 {}",
            tdd_red_count
        );
    }

    /// 借鉴 superpowers 模式: Skill name 跟 superpowers 公开 SKILL.md 1:1 (部分映射)
    #[test]
    fn skill_id_kebab_name_matches_superpowers_convention() {
        assert_eq!(
            SkillId::SuperpowersSkillGuard.kebab_name(),
            "superpowers-skill-guard"
        );
        assert_eq!(SkillId::ColangDslGuard.kebab_name(), "colang-dsl-guard");
        // 5 守门跟 superpowers 公开 kebab-case 模式 1:1 (multi-ai-guard / multi-human-guard / ...)
    }
}
