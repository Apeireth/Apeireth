//! apeireth-skills — R23 6 module skills 子模块。
//!
//! R23 P1 #5 实质化: 加 +7 顶层 pub fn — skill registry + ID/path conventions +
//! version compare. 不假装: 真 registry + 真 ID validation (kebab-case / ascii) +
//! 真 semver comparison (3 段比较).
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.
//!
//! R63: 加 file_loader submodule — 借鉴 VCP `vcptoolbox/modules` 真扫目录 JSON.
//!
//! R125-19 (per decision-51 §1.4 P3-2): 加 skill_executor submodule — 5 phase state machines
//! 落地 obra/superpowers 14 公开 SKILL.md workflow 模式 (TDD / Plan-Verify / Parallel /
//! Review / Meta). 借鉴 ID: `R125-19-BORROW-obra/superpowers-2026-05-2026-08-10`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod descriptor;
// R177: organ invariants (5 tests + 2 Kani)
pub mod anthropic_skills;
pub mod eval_bridge; // R110: Skill descriptor → eval scenario 桥接
pub mod file_loader;
pub mod library_stage6_guardianship;
pub mod mcp_bridge;
mod organ_kani_proofs;
pub mod semver_strict;
pub mod skill_executor; // R125-19: Skill execution layer (5 phase state machines, superpowers 14 → 5 patterns)
pub mod wasm_bridge; // R174: WASM skill executor (uses apeireth-sovereignty::wasm_runtime)
pub mod watcher; // R109: 文件 watcher 热加载 (polling-based, 0 新 dep)  // R107: 严格 semver 2.0.0 (3-segment + pre-release + build metadata)  // R86: Skill → MCP ToolServer 适配器 (SkillDescriptor → Tool, call 走 dispatch) // R149: Anthropic Skills 模式 (SKILL.md + 3 层加载)
                 // R127 P5-3: Library Stage 6 守护 (借鉴 hyper 80 + PyO3 928 + servers 175)

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("skill id `{0}` is empty")]
    EmptyId(String),
    #[error("skill version `{0}` is invalid")]
    InvalidVersion(String),
    #[error("skill id `{0}` 不符合 kebab-case: 只允许 ascii lowercase + digit + `-`")]
    InvalidIdFormat(String),
    #[error("skill: id=`{0}` 重复注册")]
    DuplicateId(String),
    #[error("skill: id=`{0}` 未注册")]
    UnknownSkill(String),
    /// TP23: 试图把 capability 注册到 discipline 通道 (或反向) — 类型不匹配
    #[error("skill: id=`{0}` 类别冲突 (已是 {1}, 不能改注册为 {2})")]
    KindMismatch(String, SkillKind, SkillKind),
}
pub type SkillResult<T> = Result<T, SkillError>;

pub use anthropic_skills::{
    AnthropicSkillError, AnthropicSkillLoader, AnthropicSkillResult, SkillDocument, SkillEntry,
    SkillManifest,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub version: String,
    pub input_schema: String,
    pub output_schema: String,
}

impl Skill {
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        input_schema: impl Into<String>,
        output_schema: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            input_schema: input_schema.into(),
            output_schema: output_schema.into(),
        }
    }
    pub fn validate(&self) -> SkillResult<()> {
        let id = self.id.trim();
        if id.is_empty() {
            return Err(SkillError::EmptyId(self.id.clone()));
        }
        if !is_valid_id(id) {
            return Err(SkillError::InvalidIdFormat(self.id.clone()));
        }
        let parts: Vec<&str> = self.version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
            return Err(SkillError::InvalidVersion(self.version.clone()));
        }
        Ok(())
    }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — Registry + ID/version utilities
// ============================================================================

/// Validate ID 为 kebab-case (ascii lowercase + digit + '-', 不允许连续 `-` 不允许头尾 `-`).
pub fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.starts_with('-') || id.ends_with('-') {
        return false;
    }
    let mut prev_dash = false;
    for c in id.chars() {
        if c == '-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                return false;
            }
        }
    }
    true
}

/// Parse semver 3-segment version into (major, minor, patch).
pub fn parse_version(v: &str) -> SkillResult<(u32, u32, u32)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return Err(SkillError::InvalidVersion(v.into()));
    }
    Ok((
        parts[0]
            .parse::<u32>()
            .map_err(|_| SkillError::InvalidVersion(v.into()))?,
        parts[1]
            .parse::<u32>()
            .map_err(|_| SkillError::InvalidVersion(v.into()))?,
        parts[2]
            .parse::<u32>()
            .map_err(|_| SkillError::InvalidVersion(v.into()))?,
    ))
}

/// Compare two semver versions. Return -1 / 0 / +1 (lexicographic 3-segment).
pub fn compare_versions(a: &str, b: &str) -> SkillResult<i32> {
    let pa = parse_version(a)?;
    let pb = parse_version(b)?;
    Ok(pa.cmp(&pb) as i32)
}

/// In-memory skill registry (Vec-backed, sorted by id).
#[derive(Debug, Default, Clone)]
pub struct Registry {
    skills: Vec<Skill>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }
    /// Register a skill. Fails if id is invalid (per is_valid_id) or duplicate.
    pub fn register(&mut self, skill: Skill) -> SkillResult<()> {
        skill.validate()?;
        if self.skills.iter().any(|s| s.id == skill.id) {
            return Err(SkillError::DuplicateId(skill.id));
        }
        self.skills.push(skill);
        self.skills.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(())
    }
    pub fn get(&self, id: &str) -> SkillResult<&Skill> {
        self.skills
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| SkillError::UnknownSkill(id.into()))
    }
    pub fn len(&self) -> usize {
        self.skills.len()
    }
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
    pub fn ids(&self) -> Vec<&str> {
        self.skills.iter().map(|s| s.id.as_str()).collect()
    }
}

/// Select the highest-version skill matching a prefix (e.g. `"summarize-*"`).
pub fn select_with_prefix<'a>(reg: &'a Registry, prefix: &str) -> Option<&'a Skill> {
    reg.skills
        .iter()
        .filter(|s| s.id.starts_with(prefix.trim_end_matches('*')))
        .max_by_key(|s| parse_version(&s.version).ok())
}

// ============================================================================
// TP23: 两类技能 — Capability + Discipline (E5 扩展)
// ============================================================================
//
// 主人洞察 (per E5): 技能 ≠ 只是"可调用的功能", 还有一类叫"纪律技能":
// 可执行的原则 (如"提交前跑测试"、"代码必须有测试"）。纪律技能挂执行检查,
// 失败时阻止该执行点继续 (return Err)。
//
// 设计要点:
// - **不破坏现有 API**: `Skill` + `Registry` 字段名/方法名不变, 新类型为增量
// - **两类技能独立通道**: `SkillRegistry` 用 HashMap 分两桶, 互不混淆
// - **纪律检查可动态装载/卸载**: checker 是 `Arc<dyn DisciplineCheck>`, 与
//   `DisciplineSkill` descriptor 解耦 → 允许描述与实现分别演化
// - **不假装**: `check()` 必须返回 Result, 不允许 panic (与任务纪律对齐)
// ============================================================================

/// 技能种类 (TP23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillKind {
    /// 能力技能: 可调用的功能.
    Capability,
    /// 纪律技能: 可执行的原则 (如"提交前跑测试").
    Discipline,
}

impl std::fmt::Display for SkillKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SkillKind::Capability => "capability",
            SkillKind::Discipline => "discipline",
        })
    }
}

/// 纪律检查上下文 (传入 `check()` 的运行时信息).
#[derive(Debug, Clone)]
pub struct DisciplineContext {
    /// 操作名 (e.g. `"git-commit"`, `"code-write"`, `"pr-merge"`).
    pub operation: String,
    /// 操作主体 (e.g. 提交 hash / 文件路径 / 目标 PR id).
    pub subject: String,
    /// 扩展槽 (调用方按需填, 检查器按需读).
    pub extras: serde_json::Value,
}

impl DisciplineContext {
    pub fn new(operation: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            subject: subject.into(),
            extras: serde_json::Value::Null,
        }
    }

    pub fn with_extras(mut self, extras: serde_json::Value) -> Self {
        self.extras = extras;
        self
    }
}

/// 纪律检查实现 trait.
///
/// 契约 (任务纪律):
/// - **不能 panic** — 必须返回 `Err`, 让调用方决定处理策略
/// - **可重入** — `&self` 不持有可变借用, 允许多线程并发检查
/// - **幂等优先** — 同 ctx 多次调用结果一致 (除非 ctx 本身变)
pub trait DisciplineCheck: Send + Sync + std::fmt::Debug {
    fn check(&self, ctx: &DisciplineContext) -> Result<(), DisciplineError>;
}

/// 纪律违反错误 (检查失败原因).
#[derive(Debug, Error)]
pub enum DisciplineError {
    #[error("纪律违反 [{0}]: {1}")]
    Violation(String, String),
    #[error("纪律检查器 panic 已被捕获: {0}")]
    CheckerPanic(String),
    #[error("纪律未注册: {0}")]
    UnknownDiscipline(String),
}

/// 能力技能 (TP23): 可调用的功能. 复用 `Skill` 字段.
#[derive(Debug, Clone)]
pub struct CapabilitySkill {
    pub base: Skill,
    /// handler 名 (可选; 实际调用走外部 dispatch, 例如 `skill_executor`).
    pub handler: Option<String>,
}

impl CapabilitySkill {
    pub fn new(base: Skill, handler: Option<String>) -> Self {
        Self { base, handler }
    }
}

/// 纪律技能 (TP23): 可执行的原则. 复用 `Skill` 字段.
#[derive(Debug, Clone)]
pub struct DisciplineSkill {
    pub base: Skill,
    /// 原则说明 (自然语言, 给主人/AI 阅读).
    pub description: String,
}

impl DisciplineSkill {
    pub fn new(base: Skill, description: impl Into<String>) -> Self {
        Self {
            base,
            description: description.into(),
        }
    }
}

/// 双通道技能注册表 (TP23).
///
/// - `capabilities`: CapabilitySkill (key = id)
/// - `disciplines`:  DisciplineSkill  (key = id)
/// - `checkers`:    DisciplineCheck 实现 (key = id; 与 disciplines 同 id 配对)
///
/// ponytail: 用 std `HashMap` (与 `Registry::Vec` 同源风格, 0 新 dep);
/// 用 `parking_lot::Mutex` 与 watcher 模块保持一致.
#[derive(Debug, Default)]
pub struct SkillRegistry {
    capabilities: std::collections::HashMap<String, CapabilitySkill>,
    disciplines: std::collections::HashMap<String, DisciplineSkill>,
    checkers: std::collections::HashMap<String, std::sync::Arc<dyn DisciplineCheck>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册能力技能. id 必须 kebab-case 且未注册 (含跨通道检查: 不能与 discipline id 撞).
    pub fn register_capability(&mut self, skill: CapabilitySkill) -> SkillResult<()> {
        skill.base.validate()?;
        let id = skill.base.id.clone();
        if self.capabilities.contains_key(&id) {
            return Err(SkillError::DuplicateId(id));
        }
        if self.disciplines.contains_key(&id) {
            return Err(SkillError::KindMismatch(id, SkillKind::Discipline, SkillKind::Capability));
        }
        self.capabilities.insert(id, skill);
        Ok(())
    }

    /// 注册纪律技能 + 检查器. id 必须 kebab-case 且未注册 (含跨通道检查).
    pub fn register_discipline(
        &mut self,
        skill: DisciplineSkill,
        checker: std::sync::Arc<dyn DisciplineCheck>,
    ) -> SkillResult<()> {
        skill.base.validate()?;
        let id = skill.base.id.clone();
        if self.disciplines.contains_key(&id) {
            return Err(SkillError::DuplicateId(id));
        }
        if self.capabilities.contains_key(&id) {
            return Err(SkillError::KindMismatch(id, SkillKind::Capability, SkillKind::Discipline));
        }
        self.disciplines.insert(id.clone(), skill);
        self.checkers.insert(id, checker);
        Ok(())
    }

    /// 调用单个纪律检查. 成功 = Ok(()), 失败 = Violation.
    /// 不允许 panic (任务纪律): 用 `AssertUnwindSafe` + catch_unwind 兜底.
    pub fn check(&self, id: &str, ctx: &DisciplineContext) -> Result<(), DisciplineError> {
        let checker = self
            .checkers
            .get(id)
            .ok_or_else(|| DisciplineError::UnknownDiscipline(id.to_string()))?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            checker.check(ctx)
        }));
        match result {
            Ok(r) => r,
            Err(_) => Err(DisciplineError::CheckerPanic(id.to_string())),
        }
    }

    /// 全量纪律检查: 遍历所有纪律, 收集 (id, Result) 对. 用于"挂载点"统一跑全部纪律.
    /// 不短路: 单条违规不阻断其他纪律的检查 (主人可一次性看到所有违规).
    pub fn check_all(&self, ctx: &DisciplineContext) -> Vec<(String, Result<(), DisciplineError>)> {
        let mut out = Vec::with_capacity(self.checkers.len());
        for (id, checker) in &self.checkers {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                checker.check(ctx)
            }));
            let r = match result {
                Ok(r) => r,
                Err(_) => Err(DisciplineError::CheckerPanic(id.clone())),
            };
            out.push((id.clone(), r));
        }
        out
    }

    /// 卸载技能 (按 id). 返回 true = 卸载了某条, false = id 不存在.
    /// 纪律技能卸载 = 同时移除 descriptor + checker.
    pub fn unload(&mut self, id: &str) -> bool {
        let cap_hit = self.capabilities.remove(id).is_some();
        let disc_hit = self.disciplines.remove(id).is_some();
        let check_hit = self.checkers.remove(id).is_some();
        cap_hit || disc_hit || check_hit
    }

    /// 总条目数 (capability + discipline).
    pub fn len(&self) -> usize {
        self.capabilities.len() + self.disciplines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 能力技能数.
    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }

    /// 纪律技能数.
    pub fn discipline_count(&self) -> usize {
        self.disciplines.len()
    }

    /// 取能力技能 (按 id).
    pub fn get_capability(&self, id: &str) -> Option<&CapabilitySkill> {
        self.capabilities.get(id)
    }

    /// 取纪律技能 (按 id).
    pub fn get_discipline(&self, id: &str) -> Option<&DisciplineSkill> {
        self.disciplines.get(id)
    }

    /// 列全部能力 id (sorted).
    pub fn capability_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.capabilities.keys().map(|s| s.as_str()).collect();
        ids.sort();
        ids
    }

    /// 列全部纪律 id (sorted).
    pub fn discipline_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.disciplines.keys().map(|s| s.as_str()).collect();
        ids.sort();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_skill_passes_validation() {
        let s = Skill::new("summarize-text", "1.0.0", "{}", "{}");
        assert!(s.validate().is_ok());
    }
    #[test]
    fn empty_id_is_rejected() {
        let s = Skill::new("   ", "1.0.0", "{}", "{}");
        assert!(s.validate().is_err());
    }
    #[test]
    fn invalid_version_is_rejected() {
        let s = Skill::new("ok", "1.0", "{}", "{}");
        assert!(s.validate().is_err());
    }

    #[test]
    fn is_valid_id_kebab() {
        assert!(is_valid_id("hello"));
        assert!(is_valid_id("hello-world"));
        assert!(is_valid_id("a1b2c3"));
        assert!(is_valid_id("x-1-y-2"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("-start"));
        assert!(!is_valid_id("end-"));
        assert!(!is_valid_id("CamelCase"));
        assert!(!is_valid_id("space here"));
        assert!(!is_valid_id("under_score"));
        assert!(!is_valid_id("double--dash"));
    }
    #[test]
    fn parse_version_basic() {
        assert_eq!(parse_version("1.2.3").unwrap(), (1, 2, 3));
        assert!(parse_version("1.2").is_err());
        assert!(parse_version("a.b.c").is_err());
    }
    #[test]
    fn compare_versions_basic() {
        assert_eq!(compare_versions("1.0.0", "1.0.1").unwrap(), -1);
        assert_eq!(compare_versions("1.0.0", "1.0.0").unwrap(), 0);
        assert_eq!(compare_versions("2.0.0", "1.9.9").unwrap(), 1);
    }
    #[test]
    fn registry_register_and_get() {
        let mut r = Registry::new();
        r.register(Skill::new("a-skill", "1.0.0", "{}", "{}"))
            .unwrap();
        r.register(Skill::new("b-skill", "2.0.0", "{}", "{}"))
            .unwrap();
        assert_eq!(r.len(), 2);
        let s = r.get("a-skill").unwrap();
        assert_eq!(s.version, "1.0.0");
    }
    #[test]
    fn registry_duplicate_rejected() {
        let mut r = Registry::new();
        r.register(Skill::new("a", "1.0.0", "{}", "{}")).unwrap();
        assert!(matches!(
            r.register(Skill::new("a", "2.0.0", "{}", "{}")),
            Err(SkillError::DuplicateId(_))
        ));
    }
    #[test]
    fn registry_unknown_get() {
        let r = Registry::new();
        assert!(matches!(r.get("nope"), Err(SkillError::UnknownSkill(_))));
    }
    #[test]
    fn select_with_prefix_picks_highest() {
        let mut r = Registry::new();
        r.register(Skill::new("summarize-text", "1.0.0", "{}", "{}"))
            .unwrap();
        r.register(Skill::new("summarize-html", "1.5.0", "{}", "{}"))
            .unwrap();
        r.register(Skill::new("summarize-md", "2.0.0", "{}", "{}"))
            .unwrap();
        let s = select_with_prefix(&r, "summarize-").unwrap();
        assert_eq!(s.id, "summarize-md");
    }
    #[test]
    fn ids_sorted() {
        let mut r = Registry::new();
        r.register(Skill::new("z", "1.0.0", "{}", "{}")).unwrap();
        r.register(Skill::new("a", "1.0.0", "{}", "{}")).unwrap();
        r.register(Skill::new("m", "1.0.0", "{}", "{}")).unwrap();
        assert_eq!(r.ids(), vec!["a", "m", "z"]);
    }

    // ========================================================================
    // TP23: 两类技能 (Capability + Discipline) 测试
    // ========================================================================

    /// 测试用纪律: 操作名 = "commit" 且 subject 含 "skip-test" 时违规
    #[derive(Debug)]
    struct NoCommitWithoutTests;
    impl DisciplineCheck for NoCommitWithoutTests {
        fn check(&self, ctx: &DisciplineContext) -> Result<(), DisciplineError> {
            if ctx.operation == "commit" && ctx.subject.contains("skip-test") {
                Err(DisciplineError::Violation(
                    "no-commit-without-tests".into(),
                    "提交不能跳过测试".into(),
                ))
            } else {
                Ok(())
            }
        }
    }

    /// 测试用纪律: 总返回 Ok (基准纪律)
    #[derive(Debug)]
    struct AlwaysPass;
    impl DisciplineCheck for AlwaysPass {
        fn check(&self, _ctx: &DisciplineContext) -> Result<(), DisciplineError> {
            Ok(())
        }
    }

    /// 测试用纪律: 模拟 panic (验证 check() 不会向外传播 panic)
    #[derive(Debug)]
    struct PanicChecker;
    impl DisciplineCheck for PanicChecker {
        fn check(&self, _ctx: &DisciplineContext) -> Result<(), DisciplineError> {
            panic!("测试用 panic")
        }
    }

    fn cap(id: &str, ver: &str) -> CapabilitySkill {
        CapabilitySkill::new(
            Skill::new(id, ver, "{}", "{}"),
            Some(format!("handler_{}", id)),
        )
    }

    fn disc(id: &str, ver: &str, desc: &str) -> DisciplineSkill {
        DisciplineSkill::new(Skill::new(id, ver, "{}", "{}"), desc)
    }

    #[test]
    fn tp23_register_capability_succeeds() {
        let mut reg = SkillRegistry::new();
        reg.register_capability(cap("summarize-text", "1.0.0")).unwrap();
        assert_eq!(reg.capability_count(), 1);
        assert_eq!(reg.discipline_count(), 0);
        assert_eq!(reg.len(), 1);
        let got = reg.get_capability("summarize-text").unwrap();
        assert_eq!(got.base.id, "summarize-text");
        assert_eq!(got.handler.as_deref(), Some("handler_summarize-text"));
    }

    #[test]
    fn tp23_register_discipline_succeeds() {
        let mut reg = SkillRegistry::new();
        reg.register_discipline(
            disc("no-commit-without-tests", "1.0.0", "提交前必须跑测试"),
            Arc::new(NoCommitWithoutTests),
        )
        .unwrap();
        assert_eq!(reg.capability_count(), 0);
        assert_eq!(reg.discipline_count(), 1);
        let got = reg.get_discipline("no-commit-without-tests").unwrap();
        assert_eq!(got.base.id, "no-commit-without-tests");
        assert!(got.description.contains("提交前"));
    }

    #[test]
    fn tp23_capability_duplicate_rejected() {
        let mut reg = SkillRegistry::new();
        reg.register_capability(cap("a", "1.0.0")).unwrap();
        let r = reg.register_capability(cap("a", "2.0.0"));
        assert!(matches!(r, Err(SkillError::DuplicateId(_))));
    }

    #[test]
    fn tp23_discipline_duplicate_rejected() {
        let mut reg = SkillRegistry::new();
        reg.register_discipline(disc("a", "1.0.0", "x"), Arc::new(AlwaysPass))
            .unwrap();
        let r = reg.register_discipline(disc("a", "2.0.0", "y"), Arc::new(AlwaysPass));
        assert!(matches!(r, Err(SkillError::DuplicateId(_))));
    }

    #[test]
    fn tp23_kind_mismatch_rejected() {
        let mut reg = SkillRegistry::new();
        reg.register_capability(cap("shared", "1.0.0")).unwrap();
        // 试图把同一 id 注册为 discipline → KindMismatch
        let r = reg.register_discipline(disc("shared", "1.0.0", "x"), Arc::new(AlwaysPass));
        assert!(matches!(r, Err(SkillError::KindMismatch(_, SkillKind::Capability, SkillKind::Discipline))));
        // 反向
        let mut reg2 = SkillRegistry::new();
        reg2.register_discipline(disc("shared2", "1.0.0", "x"), Arc::new(AlwaysPass)).unwrap();
        let r2 = reg2.register_capability(cap("shared2", "1.0.0"));
        assert!(matches!(r2, Err(SkillError::KindMismatch(_, SkillKind::Discipline, SkillKind::Capability))));
    }

    #[test]
    fn tp23_invalid_id_rejected() {
        let mut reg = SkillRegistry::new();
        let bad = CapabilitySkill::new(Skill::new("BadID", "1.0.0", "{}", "{}"), None);
        assert!(reg.register_capability(bad).is_err());
    }

    #[test]
    fn tp23_check_success_path() {
        let mut reg = SkillRegistry::new();
        reg.register_discipline(
            disc("no-commit-without-tests", "1.0.0", "提交前必须跑测试"),
            Arc::new(NoCommitWithoutTests),
        )
        .unwrap();
        let ctx = DisciplineContext::new("read-file", "src/main.rs");
        assert!(reg.check("no-commit-without-tests", &ctx).is_ok());
    }

    #[test]
    fn tp23_check_failure_path() {
        let mut reg = SkillRegistry::new();
        reg.register_discipline(
            disc("no-commit-without-tests", "1.0.0", "提交前必须跑测试"),
            Arc::new(NoCommitWithoutTests),
        )
        .unwrap();
        let ctx = DisciplineContext::new("commit", "skip-test-fix.md");
        let err = reg.check("no-commit-without-tests", &ctx).unwrap_err();
        match err {
            DisciplineError::Violation(id, msg) => {
                assert_eq!(id, "no-commit-without-tests");
                assert!(msg.contains("测试"));
            }
            _ => panic!("期望 Violation, 实际 {err:?}"),
        }
    }

    #[test]
    fn tp23_check_unknown_discipline_rejected() {
        let reg = SkillRegistry::new();
        let ctx = DisciplineContext::new("any", "x");
        let err = reg.check("nope", &ctx).unwrap_err();
        assert!(matches!(err, DisciplineError::UnknownDiscipline(_)));
    }

    #[test]
    fn tp23_check_panic_caught_not_propagated() {
        // 纪律纪律: check() 不允许 panic 传出
        let mut reg = SkillRegistry::new();
        reg.register_discipline(disc("panicker", "1.0.0", "x"), Arc::new(PanicChecker))
            .unwrap();
        let ctx = DisciplineContext::new("any", "x");
        let r = reg.check("panicker", &ctx);
        assert!(matches!(r, Err(DisciplineError::CheckerPanic(_))), "panic 应被捕获, 实际 {r:?}");
    }

    #[test]
    fn tp23_check_all_collects_all_results_no_short_circuit() {
        let mut reg = SkillRegistry::new();
        reg.register_discipline(disc("d1", "1.0.0", "x"), Arc::new(NoCommitWithoutTests))
            .unwrap();
        reg.register_discipline(disc("d2", "1.0.0", "x"), Arc::new(AlwaysPass))
            .unwrap();
        reg.register_discipline(disc("d3", "1.0.0", "x"), Arc::new(PanicChecker))
            .unwrap();
        let ctx = DisciplineContext::new("commit", "skip-test.md");
        let results = reg.check_all(&ctx);
        assert_eq!(results.len(), 3);
        let by_id: std::collections::HashMap<String, _> = results.into_iter().collect();
        assert!(matches!(by_id.get("d1").unwrap(), Err(DisciplineError::Violation(_, _))));
        assert!(matches!(by_id.get("d2").unwrap(), Ok(())));
        assert!(matches!(by_id.get("d3").unwrap(), Err(DisciplineError::CheckerPanic(_))));
    }

    #[test]
    fn tp23_unload_removes_capability() {
        let mut reg = SkillRegistry::new();
        reg.register_capability(cap("c1", "1.0.0")).unwrap();
        assert_eq!(reg.capability_count(), 1);
        assert!(reg.unload("c1"), "卸载应返回 true");
        assert_eq!(reg.capability_count(), 0);
        assert!(!reg.unload("c1"), "重复卸载应返回 false");
    }

    #[test]
    fn tp23_unload_removes_discipline_and_checker() {
        let mut reg = SkillRegistry::new();
        reg.register_discipline(disc("d1", "1.0.0", "x"), Arc::new(AlwaysPass))
            .unwrap();
        assert_eq!(reg.discipline_count(), 1);
        assert!(reg.unload("d1"));
        assert_eq!(reg.discipline_count(), 0);
        // 卸载后 check 必须返回 UnknownDiscipline (checker 也应被清理)
        let ctx = DisciplineContext::new("any", "x");
        let err = reg.check("d1", &ctx).unwrap_err();
        assert!(matches!(err, DisciplineError::UnknownDiscipline(_)));
    }

    #[test]
    fn tp23_unload_unknown_returns_false() {
        let mut reg = SkillRegistry::new();
        assert!(!reg.unload("nope"));
    }

    #[test]
    fn tp23_backward_compat_existing_skill_api_unchanged() {
        // 旧 Skill + Registry API 仍可用 (不破现有测试)
        let mut reg = Registry::new();
        reg.register(Skill::new("legacy-skill", "1.0.0", "{}", "{}"))
            .unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("legacy-skill").unwrap().version, "1.0.0");
        // 新 SkillRegistry 是独立类型
        let new_reg = SkillRegistry::new();
        assert!(new_reg.is_empty());
        assert_eq!(new_reg.capability_count(), 0);
        assert_eq!(new_reg.discipline_count(), 0);
        // 旧的 select_with_prefix 等顶层函数仍可用
        assert!(select_with_prefix(&reg, "legacy-").is_some());
    }

    #[test]
    fn tp23_capability_and_discipline_lists_are_sorted() {
        let mut reg = SkillRegistry::new();
        reg.register_capability(cap("z-cap", "1.0.0")).unwrap();
        reg.register_capability(cap("a-cap", "1.0.0")).unwrap();
        reg.register_discipline(disc("z-disc", "1.0.0", "x"), Arc::new(AlwaysPass))
            .unwrap();
        reg.register_discipline(disc("a-disc", "1.0.0", "x"), Arc::new(AlwaysPass))
            .unwrap();
        assert_eq!(reg.capability_ids(), vec!["a-cap", "z-cap"]);
        assert_eq!(reg.discipline_ids(), vec!["a-disc", "z-disc"]);
    }
}

// 顶层 use 方便测试 (与文件其他 use 风格一致; 只在测试代码段使用 Arc).
#[cfg(test)]
use std::sync::Arc;
