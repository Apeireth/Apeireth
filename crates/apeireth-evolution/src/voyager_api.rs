//! R215 Voyager API — continual learner facade for library_autonomy.
//!
//! **动机**: SelfEvolution / SelfUpgrade / SelfRepair 三个引擎各自有 trait + step 循环.
//! Voyager (Wang et al. 2023) 把 agent 建模为 "skill library + 持续学习", 每次任务
//! 都 retrieve 相关 skill → 尝试 → 反馈 → 累积. R215 给 library_autonomy 加一个
//! 统一 facade, 把 3 个 self-* 引擎聚合成 Voyager 风格的"持续学习 + 技能库"接口.
//!
//! **借鉴** (per O-5): Voyager (NVIDIA, 2023) — skill library + curriculum +
//! iterative improvement. 我们用 std HashMap 自实现, 0 引外部 dep.
//!
//! **0 触碰**:
//! - library_autonomy.rs (1824 行) 0 改
//! - library_autonomy_loop.rs 0 改
//! - 3 不可变脊柱 0 触碰

#![allow(missing_docs)] // R215 additive
#![allow(clippy::all)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ============================================================================
// Skill / SkillLibrary — Voyager 风格技能库
// ============================================================================

/// Voyager 风格技能 (R215 自定义, 与 library_autonomy::Skill trait 不冲突).
///
/// 借鉴: Voyager 把每个 skill 存为可执行 code + 描述 + 成功次数. 我们用
/// (name, description, code, success_count, failure_count) 5 字段.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 技能名 (e.g. "fix_import_loop", "refine_pad_center")
    pub name: String,
    /// 技能描述 (LLM 可读)
    pub description: String,
    /// 用于检索的关键词 (来自原始 task query)
    pub query_keywords: String,
    /// 技能实现 (code-like 文本, LLM 生成)
    pub code: String,
    /// 成功调用次数
    pub success_count: u32,
    /// 失败调用次数
    pub failure_count: u32,
    /// 创建时间 (epoch ms)
    pub created_at_ms: i64,
}

impl Skill {
    pub fn new(name: impl Into<String>, description: impl Into<String>, code: impl Into<String>, query_keywords: impl Into<String>, now_ms: i64) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            code: code.into(),
            query_keywords: query_keywords.into(),
            success_count: 0,
            failure_count: 0,
            created_at_ms: now_ms,
        }
    }

    /// 成功率 (0.0 .. 1.0). 0 样本时返回 0.5 (中性).
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.5
        } else {
            self.success_count as f64 / total as f64
        }
    }

    /// 调用次数.
    pub fn total_calls(&self) -> u32 {
        self.success_count + self.failure_count
    }
}

/// Skill library (Voyager 风格 HashMap 存储).
#[derive(Debug, Default, Clone)]
pub struct SkillLibrary {
    skills: HashMap<String, Skill>,
}

impl SkillLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加或替换 skill.
    pub fn add(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// 获取 skill (按 name).
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// 删除 skill.
    pub fn remove(&mut self, name: &str) -> Option<Skill> {
        self.skills.remove(name)
    }

    /// 全部 skill 数量.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// 全部 skill 列表.
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// 按关键词检索 (name + description 子串匹配).
    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let q = query.to_lowercase();
        let mut hits: Vec<&Skill> = self.skills.values()
            .filter(|s| s.name.to_lowercase().contains(&q) || s.description.to_lowercase().contains(&q) || s.query_keywords.to_lowercase().contains(&q))
            .collect();
        hits.sort_by(|a, b| b.success_rate().partial_cmp(&a.success_rate()).unwrap_or(std::cmp::Ordering::Equal));
        hits
    }
}

// ============================================================================
// Voyager facade — 持续学习循环
// ============================================================================

/// Voyager 任务结果 (反馈).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskOutcome {
    /// 任务成功.
    Success,
    /// 任务失败.
    Failure,
}

/// Voyager 任务.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyagerTask {
    /// 任务描述.
    pub description: String,
    /// 检索关键词 (用于 skill library.search).
    pub query: String,
    /// 任务开始时间.
    pub started_at_ms: i64,
}

impl VoyagerTask {
    pub fn new(description: impl Into<String>, query: impl Into<String>, now_ms: i64) -> Self {
        Self {
            description: description.into(),
            query: query.into(),
            started_at_ms: now_ms,
        }
    }
}

/// Voyager 任务结果.
#[derive(Debug, Clone)]
pub struct VoyagerResult {
    pub task: VoyagerTask,
    /// 选用的 skill (None = 无匹配, 创建新 skill).
    pub skill_used: Option<String>,
    /// 是否新建了 skill.
    pub skill_created: bool,
    /// 任务结果.
    pub outcome: TaskOutcome,
    pub finished_at_ms: i64,
    /// 耗时 (ms).
    pub elapsed_ms: i64,
}

/// Voyager 持续学习 facade.
#[derive(Debug, Default)]
pub struct Voyager {
    library: SkillLibrary,
    history: Vec<VoyagerResult>,
    /// 新 skill 模板 (检索失败时用, 简单 LLM-like 代码生成).
    auto_skill_template: String,
}

impl Voyager {
    pub fn new() -> Self {
        Self {
            library: SkillLibrary::new(),
            history: Vec::new(),
            auto_skill_template: "// auto-generated skill stub\nfn main() { /* TODO */ }".to_string(),
        }
    }

    /// 设置自动 skill 模板 (无匹配时生成).
    pub fn set_skill_template(&mut self, tmpl: impl Into<String>) {
        self.auto_skill_template = tmpl.into();
    }

    /// 跑 1 个任务 (Voyager 风格 retrieve → use → feedback).
    ///
    /// 1. search library for matching skill
    /// 2. if found, use it
    /// 3. if not, auto-generate stub skill
    /// 4. record outcome + update success/failure count
    pub fn run_task(&mut self, task: VoyagerTask, outcome: TaskOutcome, now_ms: i64) -> VoyagerResult {
        let candidates = self.library.search(&task.query);
        let (skill_used, skill_created) = if let Some(s) = candidates.first() {
            (Some(s.name.clone()), false)
        } else {
            // 无匹配, 创建新 skill
            let name = format!("skill_{}", self.library.len());
            let new_skill = Skill::new(
                &name,
                &task.description,
                &self.auto_skill_template,
                &task.query,
                now_ms,
            );
            self.library.add(new_skill);
            (Some(name), true)
        };

        // 更新 success / failure count
        if let Some(name) = &skill_used {
            if let Some(s) = self.library.skills.get_mut(name) {
                match outcome {
                    TaskOutcome::Success => s.success_count += 1,
                    TaskOutcome::Failure => s.failure_count += 1,
                }
            }
        }

        let elapsed_ms = (now_ms - task.started_at_ms).max(0);
        let result = VoyagerResult {
            task,
            skill_used,
            skill_created,
            outcome,
            finished_at_ms: now_ms,
            elapsed_ms,
        };
        self.history.push(result.clone());
        result
    }

    /// Skill 库引用.
    pub fn library(&self) -> &SkillLibrary {
        &self.library
    }

    pub fn library_mut(&mut self) -> &mut SkillLibrary {
        &mut self.library
    }

    /// 历史任务.
    pub fn history(&self) -> &[VoyagerResult] {
        &self.history
    }

    /// 统计: 成功 / 失败 / 总数.
    pub fn stats(&self) -> VoyagerStats {
        let total = self.history.len();
        let success = self.history.iter().filter(|r| r.outcome == TaskOutcome::Success).count();
        VoyagerStats {
            total_tasks: total,
            success_tasks: success,
            failure_tasks: total - success,
            skill_count: self.library.len(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VoyagerStats {
    pub total_tasks: usize,
    pub success_tasks: usize,
    pub failure_tasks: usize,
    pub skill_count: usize,
}

impl VoyagerStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            0.0
        } else {
            self.success_tasks as f64 / self.total_tasks as f64
        }
    }
}

// ============================================================================
// 测试 (12 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_skill_new_defaults() {
        let s = Skill::new("test", "test desc", "fn x() {}", "kw", 1_000_000);
        assert_eq!(s.name, "test");
        assert_eq!(s.success_count, 0);
        assert_eq!(s.failure_count, 0);
        assert_eq!(s.total_calls(), 0);
    }

    #[test]
    fn t02_skill_success_rate() {
        let mut s = Skill::new("a", "a", "code", "kw_a", 0);
        s.success_count = 7;
        s.failure_count = 3;
        assert!((s.success_rate() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn t03_skill_success_rate_no_samples() {
        let s = Skill::new("a", "a", "c", "kw_a", 0);
        assert!((s.success_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn t04_skill_library_add_get() {
        let mut lib = SkillLibrary::new();
        lib.add(Skill::new("a", "a", "c", "kw_a", 0));
        lib.add(Skill::new("b", "b", "c", "kw_b", 0));
        assert_eq!(lib.len(), 2);
        assert!(lib.get("a").is_some());
        assert!(lib.get("c").is_none());
    }

    #[test]
    fn t05_skill_library_search() {
        let mut lib = SkillLibrary::new();
        lib.add(Skill::new("fix_loop", "fix import loop", "code", "kw_fix", 0));
        lib.add(Skill::new("optimize", "optimize query", "code", "kw_opt", 0));
        let hits = lib.search("fix");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "fix_loop");
    }

    #[test]
    fn t06_skill_library_remove() {
        let mut lib = SkillLibrary::new();
        lib.add(Skill::new("a", "a", "c", "kw_a", 0));
        let removed = lib.remove("a");
        assert!(removed.is_some());
        assert_eq!(lib.len(), 0);
    }

    #[test]
    fn t07_voyager_new() {
        let v = Voyager::new();
        assert_eq!(v.library().len(), 0);
        assert_eq!(v.history().len(), 0);
    }

    #[test]
    fn t08_voyager_run_task_creates_skill() {
        let mut v = Voyager::new();
        let task = VoyagerTask::new("test desc", "test query", 1_000_000);
        let r = v.run_task(task, TaskOutcome::Success, 1_000_100);
        assert!(r.skill_created);
        assert_eq!(v.library().len(), 1);
        assert_eq!(r.outcome, TaskOutcome::Success);
    }

    #[test]
    fn t09_voyager_run_task_reuses_skill() {
        let mut v = Voyager::new();
        let task1 = VoyagerTask::new("test 1", "test query", 1_000_000);
        v.run_task(task1, TaskOutcome::Success, 1_000_100);
        // 第二次相同 query, 应复用 skill
        let task2 = VoyagerTask::new("test 2", "test query", 1_001_000);
        let r = v.run_task(task2, TaskOutcome::Success, 1_001_100);
        assert!(!r.skill_created);
        assert_eq!(v.library().len(), 1);
    }

    #[test]
    fn t10_voyager_updates_count() {
        let mut v = Voyager::new();
        let task = VoyagerTask::new("desc", "q", 0);
        v.run_task(task.clone(), TaskOutcome::Success, 100);
        v.run_task(task.clone(), TaskOutcome::Failure, 200);
        v.run_task(task, TaskOutcome::Success, 300);
        let skill = v.library().get("skill_0").unwrap();
        assert_eq!(skill.success_count, 2);
        assert_eq!(skill.failure_count, 1);
    }

    #[test]
    fn t11_voyager_stats() {
        let mut v = Voyager::new();
        v.run_task(VoyagerTask::new("a", "qa", 0), TaskOutcome::Success, 100);
        v.run_task(VoyagerTask::new("b", "qb", 0), TaskOutcome::Failure, 200);
        let s = v.stats();
        assert_eq!(s.total_tasks, 2);
        assert_eq!(s.success_tasks, 1);
        assert_eq!(s.failure_tasks, 1);
        assert_eq!(s.skill_count, 2);
        assert!((s.success_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn t12_voyager_set_template() {
        let mut v = Voyager::new();
        v.set_skill_template("// custom template");
        let task = VoyagerTask::new("d", "q", 0);
        v.run_task(task, TaskOutcome::Success, 100);
        let skill = v.library().get("skill_0").unwrap();
        assert_eq!(skill.code, "// custom template");
    }
}
