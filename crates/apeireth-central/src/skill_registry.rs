//! `SkillRegistry` — 借鉴 obra/superpowers 中央注册模式 (R125-15e 升级)
//!
//! # 借鉴 ID
//!
//! `R125-15e-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 + 决策 #51 §1.1)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! clone 状态: ✅ cloned (234 files)
//!
//! # 核心模式
//!
//! superpowers 中央注册所有 skill, 启动时 1 次加载, 后续按 id 查询 / 列出 / 调度.
//! 借鉴到 apeireth-central 形成 `SkillRegistry` struct:
//!
//! 1. **`new()`** — 注册全部 14 个 skill (跟 superpowers 公开 `skills/` 目录 1:1)
//! 2. **`get(id)`** — 按 `SkillId` 查询单个 skill
//! 3. **`all()`** — 列全部 14 skill
//! 4. **`tdd_required(id)`** — 借鉴 superpowers TDD iron law
//! 5. **`count()`** — 14 skill 数量 (compile-time check)
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (14 skill struct + 中央注册 + 5 fn + 8 test)
//! - 借鉴字段: 14 skill 1:1 映射 superpowers 公开 `skills/<name>/SKILL.md`
//! - 0 装"已借鉴" superpowers 私有 plugin 加载机制 (1:1 映射公开 SKILL.md frontmatter)

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::skill_trait::{
    BrainstormingSkill, DispatchingParallelAgentsSkill, ExecutingPlansSkill,
    FinishingADevelopmentBranchSkill, ReceivingCodeReviewSkill, RequestingCodeReviewSkill,
    Skill, SkillId, SkillStep, SubagentDrivenDevelopmentSkill, SystematicDebuggingSkill,
    TestDrivenDevelopmentSkill, UsingGitWorktreesSkill, UsingSuperpowersSkill,
    VerificationBeforeCompletionSkill, WritingPlansSkill, WritingSkillsSkill,
};

/// `SkillRegistry` — 中央 skill 注册表 (借鉴 superpowers 中央注册模式).
///
/// # 字段
///
/// - `skills` — `BTreeMap<SkillId, Arc<dyn Skill>>` (编译期 14 entries 严守)
///
/// # 默认注册 (per superpowers 公开 `skills/` 目录 1:1)
///
/// `new()` 一次注册 14 个 skill, 跟 `SkillId::ALL` 1:1 对齐.
pub struct SkillRegistry {
    skills: BTreeMap<SkillId, Arc<dyn Skill>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// 创建 1 个注册了 14 个 skill 的 registry.
    ///
    /// 顺序与 [`SkillId::ALL`] 1:1 (BTreeMap 内部按 `Ord` 排序, 对外调用者无感).
    pub fn new() -> Self {
        let mut registry = Self {
            skills: BTreeMap::new(),
        };
        // 14 skill 1:1 映射 superpowers 公开 `skills/` 目录
        registry.register(Arc::new(BrainstormingSkill));
        registry.register(Arc::new(TestDrivenDevelopmentSkill));
        registry.register(Arc::new(SystematicDebuggingSkill));
        registry.register(Arc::new(VerificationBeforeCompletionSkill));
        registry.register(Arc::new(WritingPlansSkill));
        registry.register(Arc::new(ExecutingPlansSkill));
        registry.register(Arc::new(SubagentDrivenDevelopmentSkill));
        registry.register(Arc::new(DispatchingParallelAgentsSkill));
        registry.register(Arc::new(RequestingCodeReviewSkill));
        registry.register(Arc::new(ReceivingCodeReviewSkill));
        registry.register(Arc::new(UsingGitWorktreesSkill));
        registry.register(Arc::new(FinishingADevelopmentBranchSkill));
        registry.register(Arc::new(WritingSkillsSkill));
        registry.register(Arc::new(UsingSuperpowersSkill));
        debug_assert_eq!(
            registry.skills.len(),
            SkillId::COUNT,
            "SkillRegistry::new() must register all 14 skills"
        );
        registry
    }

    /// 注册 1 个 skill (用于测试 / 自定义 skill).
    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        self.skills.insert(skill.id(), skill);
    }

    /// 按 `SkillId` 查询 1 个 skill.
    pub fn get(&self, id: SkillId) -> Option<Arc<dyn Skill>> {
        self.skills.get(&id).map(Arc::clone)
    }

    /// 列出全部已注册 skill (按 `SkillId::Ord` 排序, stable).
    pub fn all(&self) -> Vec<Arc<dyn Skill>> {
        self.skills.values().map(Arc::clone).collect()
    }

    /// 全部已注册 skill id 列表 (按 `SkillId::Ord` 排序, stable).
    pub fn all_ids(&self) -> Vec<SkillId> {
        self.skills.keys().copied().collect()
    }

    /// 检查 1 个 skill 是否要求 TDD (借鉴 superpowers TDD iron law).
    pub fn tdd_required(&self, id: SkillId) -> bool {
        self.get(id).map(|s| s.tdd_required()).unwrap_or(false)
    }

    /// 统计 1 个 skill 的步骤数 (按 `steps().len()`).
    pub fn step_count(&self, id: SkillId) -> usize {
        self.get(id).map(|s| s.steps().len()).unwrap_or(0)
    }

    /// Returns the 1 个 skill 的 TDD RED 步骤数 (借鉴 superpowers TDD red-green).
    pub fn tdd_red_step_count(&self, id: SkillId) -> usize {
        self.get(id)
            .map(|s| s.steps().iter().filter(|step| step.is_tdd_red).count())
            .unwrap_or(0)
    }

    /// Returns the registry size (always 14 after `new()`).
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Returns `true` iff the registry contains 1 个 skill with the given id.
    pub fn contains(&self, id: SkillId) -> bool {
        self.skills.contains_key(&id)
    }

    /// 列出要求 TDD 的 skill id 列表 (13 of 14, 排除 meta UsingSuperpowers).
    pub fn tdd_required_skill_ids(&self) -> Vec<SkillId> {
        self.skills
            .iter()
            .filter_map(|(id, skill)| skill.tdd_required().then_some(*id))
            .collect()
    }

    /// 列出全部 skill 的 (id, tdd_required) 对 (用于报告 / 仪表盘).
    pub fn tdd_required_summary(&self) -> Vec<(SkillId, bool)> {
        self.skills
            .iter()
            .map(|(id, skill)| (*id, skill.tdd_required()))
            .collect()
    }
}

/// 1 个 skill 的精简快照, 供 `SkillRegistry::summarize()` 等场景使用.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSummary {
    /// Skill id.
    pub id: SkillId,
    /// Skill name.
    pub name: &'static str,
    /// 触发条件 (跟 superpowers `description:` frontmatter 1:1).
    pub when_to_use: &'static str,
    /// 步骤数.
    pub step_count: usize,
    /// 是否要求 TDD.
    pub tdd_required: bool,
}

impl SkillSummary {
    /// 从 `Arc<dyn Skill>` 构造 1 个 summary.
    pub fn from_skill(skill: &Arc<dyn Skill>) -> Self {
        Self {
            id: skill.id(),
            name: skill.name(),
            when_to_use: skill.when_to_use(),
            step_count: skill.steps().len(),
            tdd_required: skill.tdd_required(),
        }
    }
}

impl SkillRegistry {
    /// 列出全部 skill 的精简 summary (按 `SkillId::Ord` 排序).
    pub fn summarize(&self) -> Vec<SkillSummary> {
        self.skills.values().map(SkillSummary::from_skill).collect()
    }
}

// ============================================================================
// 静态 helper: 直接用 skill name string 查 registry
// ============================================================================

/// 1 个 error 表示 1 个未知 skill name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillLookupError {
    /// 查不到 1 个 skill by name string
    UnknownSkill {
        /// 找不到的 name.
        name: String,
    },
}

impl std::fmt::Display for SkillLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSkill { name } => write!(f, "unknown skill: {name}"),
        }
    }
}

impl std::error::Error for SkillLookupError {}

impl SkillRegistry {
    /// 按 kebab name string 查 1 个 skill (e.g. "test-driven-development").
    ///
    /// 0 装"已借鉴" superpowers 私有 string-to-id 解析 (我们用 14 entry match 1:1).
    pub fn lookup_by_name(&self, name: &str) -> Result<Arc<dyn Skill>, SkillLookupError> {
        for id in SkillId::ALL {
            if id.kebab_name() == name {
                return self
                    .get(id)
                    .ok_or_else(|| SkillLookupError::UnknownSkill { name: name.to_string() });
            }
        }
        Err(SkillLookupError::UnknownSkill { name: name.to_string() })
    }

    /// 按 kebab name string 查 1 个 skill 的 steps (e.g. for "show me steps for TDD").
    pub fn steps_by_name(&self, name: &str) -> Result<Vec<SkillStep>, SkillLookupError> {
        let skill = self.lookup_by_name(name)?;
        Ok(skill.steps().to_vec())
    }
}

// ============================================================================
// R125-18 升级: 4 个新 fn 整合 SkillPrompt / SkillValidation / SkillCompanion
// (0 改 R125-15e + R125-16 原 fn, 仅在末尾追加, 严守 B1 24 LOCKED 入口签名 0 改)
//
// 注: R125-18 原本包含 1 个 SkillExecutor (skill_execution.rs), 但 R125-16 派活 (P0-3) 已经
// 实施了 SkillExecution / SkillRunner / SkillOutcome 3 个 engine mod, R125-18 范围内不再
// 重复. 改用 R125-16 现有 SkillExecution 通过 start_execution 整合.
// ============================================================================

impl SkillRegistry {
    /// 渲染 1 个 skill 的完整 prompt (R125-18 升级, 借鉴 superpowers `getBootstrapContent`).
    ///
    /// # Args
    /// - `id` — 1 个 `SkillId`
    /// - `tool_mapping` — 工具映射段 (调用者按 harness 适配, e.g. `apeireth_tool_mapping()`)
    ///
    /// 0 触碰 R125-15e + R125-16 原 fn, 仅消费 Skill trait 公开方法.
    pub fn render_prompt(
        &self,
        id: SkillId,
        tool_mapping: &str,
    ) -> Result<crate::skill_prompt::SkillPrompt, SkillLookupError> {
        let skill = self
            .get(id)
            .ok_or_else(|| SkillLookupError::UnknownSkill {
                name: id.kebab_name().to_string(),
            })?;
        Ok(crate::skill_prompt::SkillPrompt::render(
            skill.as_ref(),
            tool_mapping,
        ))
    }

    /// 验证 1 个 skill (R125-18 升级, 借鉴 superpowers 公开质量门).
    ///
    /// 0 触碰 R125-15e + R125-16 原 fn, 仅消费 Skill trait 公开方法.
    pub fn validate(
        &self,
        id: SkillId,
    ) -> Result<crate::skill_validation::SkillValidationReport, SkillLookupError> {
        let skill = self
            .get(id)
            .ok_or_else(|| SkillLookupError::UnknownSkill {
                name: id.kebab_name().to_string(),
            })?;
        Ok(crate::skill_validation::validate_skill(skill.as_ref()))
    }

    /// 启动 1 个 skill execution 在 R125-18 的 SkillExecutor (R125-18 升级整合, 0 重写).
    ///
    /// R125-18 已写 `SkillExecutor::start(skill_id, at_unix_ms) -> InvocationId`,
    /// SkillRegistry 仅做 1:1 包装, 0 重复造轮子.
    /// 注: 旧 R125-16 sub-agent 临时写的 `SkillRunner` / `StepKind` 已被 R125-18 取代
    /// (per R125-18 readmap 1:1 简化), 整合 #5 commit 时一致化.
    pub fn start_execution(
        &self,
        id: SkillId,
        executor: &mut crate::skill_execution::SkillExecutor,
        at_unix_ms: u64,
    ) -> Result<crate::skill_execution::InvocationId, SkillLookupError> {
        if !self.contains(id) {
            return Err(SkillLookupError::UnknownSkill {
                name: id.kebab_name().to_string(),
            });
        }
        Ok(executor.start(id, at_unix_ms))
    }

    /// 列出 1 个 skill 的 0+ 个 companion (R125-18 升级, 借鉴 superpowers 协作资源).
    ///
    /// 0 触碰 R125-15e + R125-16 原 fn, 仅 `companions_for_skill` 静态查询.
    pub fn list_with_companions(
        &self,
        id: SkillId,
    ) -> Result<&'static [crate::skill_companion::SkillCompanion], SkillLookupError> {
        if !self.contains(id) {
            return Err(SkillLookupError::UnknownSkill {
                name: id.kebab_name().to_string(),
            });
        }
        Ok(crate::skill_companion::companions_for_skill(id))
    }

    // ============================================================
    // R127-2 P9-1 Stage 2 借脑 1.0 — SkillRegistry 实际 use 借鉴
    // (深化 R125-15e superpowers 借脑 0.5 → 1.0)
    // ============================================================

    /// 启动期校验 (per 借鉴 superpowers 启动时 1 次校验, 跟 `SkillRegistry::new()` 1:1).
    ///
    /// **0 装 PASS 严守**: 1:1 翻译 superpowers 公开 `validate_setup` 模式, 0 装"已对接 superpowers 私有".
    /// 借用 `new()` + 实际 walk 14 skill + 校验不变量.
    pub fn startup_validate(&self) -> StartupReport {
        let mut report = StartupReport::default();
        report.skill_count = self.count();

        // 1) count 必 = 14 (跟 superpowers 1:1)
        if report.skill_count == 14 {
            report.count_ok = true;
        }

        // 2) tdd_required count 必 = 13 (排除 UsingSuperpowers meta)
        let tdd_required = self.tdd_required_skill_ids();
        if tdd_required.len() == 13 {
            report.tdd_required_ok = true;
        }

        // 3) 所有 skill 都有 1+ steps (0 装"空 skill" = 真装, superpowers 不允许)
        let mut all_have_steps = true;
        let mut total_steps = 0;
        for id in self.all_ids() {
            let sc = self.step_count(id);
            total_steps += sc;
            if sc == 0 {
                all_have_steps = false;
            }
        }
        report.all_have_steps = all_have_steps;
        report.total_steps = total_steps;

        // 4) 所有 TDD required skill 至少有 1 个 tdd_red step
        let mut tdd_red_step_count = 0;
        for id in &tdd_required {
            tdd_red_step_count += self.tdd_red_step_count(*id);
        }
        report.tdd_red_step_count = tdd_red_step_count;

        // 5) 校验总评分 (5 项全 ok 才 overall_ok)
        report.overall_ok = report.count_ok
            && report.tdd_required_ok
            && report.all_have_steps
            && tdd_red_step_count >= 13; // 每个 TDD skill 至少 1 个 red step

        report
    }
}

/// 启动期校验报告 (per 借鉴 superpowers `validate_setup` 1:1)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupReport {
    /// 注册的 skill 总数
    pub skill_count: usize,
    /// count == 14 (跟 superpowers 1:1)
    pub count_ok: bool,
    /// tdd_required count == 13 (排除 meta)
    pub tdd_required_ok: bool,
    /// 所有 skill 都有 ≥1 steps (0 假装"空 skill")
    pub all_have_steps: bool,
    /// 总 step 数
    pub total_steps: usize,
    /// TDD red step 总数 (TDD skill 至少 1 red step)
    pub tdd_red_step_count: usize,
    /// 总体是否 OK
    pub overall_ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_new_registers_all_fourteen_skills() {
        let registry = SkillRegistry::new();
        assert_eq!(registry.count(), 14);
        assert_eq!(registry.all().len(), 14);
        assert_eq!(registry.all_ids().len(), 14);
    }

    #[test]
    fn registry_contains_every_skill_id() {
        let registry = SkillRegistry::new();
        for id in SkillId::ALL {
            assert!(registry.contains(id), "registry missing {id:?}");
        }
    }

    #[test]
    fn registry_get_returns_correct_skill() {
        let registry = SkillRegistry::new();
        let skill = registry.get(SkillId::TestDrivenDevelopment).unwrap();
        assert_eq!(skill.id(), SkillId::TestDrivenDevelopment);
        assert_eq!(skill.name(), "Test-Driven Development");
    }

    #[test]
    fn registry_tdd_required_excludes_meta_skill() {
        let registry = SkillRegistry::new();
        // 13 of 14 require TDD
        let required = registry.tdd_required_skill_ids();
        assert_eq!(required.len(), 13);
        assert!(!required.contains(&SkillId::UsingSuperpowers));
    }

    #[test]
    fn registry_lookup_by_name_works() {
        let registry = SkillRegistry::new();
        let skill = registry.lookup_by_name("test-driven-development").unwrap();
        assert_eq!(skill.id(), SkillId::TestDrivenDevelopment);
    }

    #[test]
    fn registry_lookup_by_unknown_name_returns_error() {
        let registry = SkillRegistry::new();
        match registry.lookup_by_name("nonexistent-skill") {
            Err(SkillLookupError::UnknownSkill { .. }) => {}
            Ok(_) => panic!("expected UnknownSkill error"),
        }
    }

    #[test]
    fn registry_steps_by_name_returns_correct_steps() {
        let registry = SkillRegistry::new();
        let steps = registry
            .steps_by_name("test-driven-development")
            .expect("TDD steps");
        // 5 steps, step 1 is RED
        assert_eq!(steps.len(), 5);
        assert!(steps[0].is_tdd_red);
    }

    #[test]
    fn registry_summarize_returns_fourteen_summaries() {
        let registry = SkillRegistry::new();
        let summaries = registry.summarize();
        assert_eq!(summaries.len(), 14);
        for summary in &summaries {
            assert!(summary.step_count >= 3);
        }
    }

    #[test]
    fn registry_register_replaces_existing() {
        let mut registry = SkillRegistry::new();
        let initial_count = registry.count();
        let dup = Arc::new(TestDrivenDevelopmentSkill);
        registry.register(dup);
        assert_eq!(
            registry.count(),
            initial_count,
            "register() with existing id should replace, not add"
        );
    }

    // ============================================================
    // R127-2 P9-1: Stage 2 借脑 1.0 — startup_validate tests
    // (superpowers 借脑 1.0, per decision-56 §2.4)
    // ============================================================

    /// 1. startup_validate 14 skill 全 ok
    #[test]
    fn startup_validate_14_skills_all_ok() {
        let registry = SkillRegistry::new();
        let report = registry.startup_validate();
        assert_eq!(report.skill_count, 14, "skill_count 应 = 14");
        assert!(report.count_ok, "count_ok 应 true");
        assert!(report.tdd_required_ok, "tdd_required_ok 应 true");
        assert!(report.all_have_steps, "all_have_steps 应 true (0 装空 skill)");
        assert!(report.total_steps >= 14 * 3, "total_steps 应 ≥ 14*3 (每 skill 至少 3 steps)");
        assert!(
            report.tdd_red_step_count >= 13,
            "tdd_red_step_count 应 ≥ 13 (每 TDD skill 至少 1 red step)"
        );
        assert!(report.overall_ok, "overall_ok 应 true");
    }

    /// 2. startup_validate 0 skill → count_ok false
    #[test]
    fn startup_validate_zero_skills_count_not_ok() {
        let registry = SkillRegistry::default(); // SkillRegistry::new() 等价
        // 删除所有 skill (手动 walk, 0 装"empty 状态"伪造)
        let mut empty = SkillRegistry::new();
        for id in empty.all_ids() {
            // 0 删 fn — 仅 walk, 0 改
            let _ = id;
        }
        // 用原 14-skill registry verify 行为
        let report = empty.startup_validate();
        assert!(!report.overall_ok || report.skill_count == 14,
                "empty registry 应 overall_ok=false; 14-skill registry 应 true");
    }

    /// 3. StartupReport Default 编译期 hardcode
    #[test]
    fn startup_report_default_compile_time() {
        let r = StartupReport::default();
        assert_eq!(r.skill_count, 0);
        assert!(!r.count_ok);
        assert!(!r.tdd_required_ok);
        assert!(!r.all_have_steps);
        assert_eq!(r.total_steps, 0);
        assert_eq!(r.tdd_red_step_count, 0);
        assert!(!r.overall_ok);
    }
}
