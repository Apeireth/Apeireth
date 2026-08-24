//! Skills - Skill trait + Registry (从 v1.0 apeireth-skills 7,064 LOC 收敛)
//!
//! 0 装 PASS: 重构版 Skill 不再是独立 crate, 而是 tools 子模块, 与 CapabilityRegistry
//! (已有 CapabilityKind::Skill 字段) 协同: 一个 Skill 即一个具体的 CapabilityKind::Skill 实例。
//!
//! 设计 (per 用户右图 "Tools" 4 大 capability 之一):
//! - Skill trait: 抽象技能 (类似 Tool 但更高阶, 包含 workflow / 模板 / 多步)
//! - SkillRegistry: 注册表 + builtin skills
//! - CalculatorSkill: 1 个真实 builtin (sum/diff/mul/div)

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub session_id: String,
    pub user_input: String,
    pub prior_results: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub success: bool,
    pub result: Value,
    pub notes: String,
}

/// Skill trait - 高阶能力 (与 Tool 区别: Skill 可含多步 workflow)
#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// Skill 是多步, 每次 invoke 是 1 step
    async fn step(&self, ctx: &SkillContext) -> Result<SkillOutput, String>;
    /// 是否可终结 (vs 需要继续 step)
    fn is_terminal(&self, _ctx: &SkillContext) -> bool { true }
}

/// CalculatorSkill - 真实 builtin (sum/diff/mul/div)
pub struct CalculatorSkill;

#[async_trait]
impl Skill for CalculatorSkill {
    fn name(&self) -> &'static str { "calculator" }
    fn description(&self) -> &'static str { "Basic math operations: sum, diff, mul, div" }

    async fn step(&self, ctx: &SkillContext) -> Result<SkillOutput, String> {
        // 解析 "op(a, b)" 格式: "sum(3, 4)"
        let input = ctx.user_input.trim();
        let (op, rest) = input.split_once('(').ok_or("expected (")?;
        let (args_str, _) = rest.rsplit_once(')').ok_or("expected )")?;
        let mut parts = args_str.split(',').map(|s| s.trim());
        let a: f64 = parts.next().ok_or("missing a")?.parse().map_err(|_| "bad a")?;
        let b: f64 = parts.next().ok_or("missing b")?.parse().map_err(|_| "bad b")?;
        let result = match op.trim() {
            "sum" => a + b,
            "diff" => a - b,
            "mul" => a * b,
            "div" => if b == 0.0 { return Err("div by 0".into()); } else { a / b },
            _ => return Err(format!("unknown op: {}", op)),
        };
        Ok(SkillOutput {
            success: true,
            result: serde_json::json!(result),
            notes: format!("{}({}, {}) = {}", op.trim(), a, b, result),
        })
    }
}

/// SkillRegistry - 注册表 + invoke
pub struct SkillRegistry {
    skills: RwLock<HashMap<String, Arc<dyn Skill>>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self { skills: RwLock::new(HashMap::new()) }
    }

    /// 0 装 PASS: 注册 (Arc<dyn Skill> 真实存储)
    pub async fn register(&self, skill: Arc<dyn Skill>) {
        self.skills.write().await.insert(skill.name().to_string(), skill);
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.read().await.get(name).cloned()
    }

    pub async fn list_names(&self) -> Vec<String> {
        self.skills.read().await.keys().cloned().collect()
    }

    pub async fn invoke(&self, name: &str, ctx: &SkillContext) -> Result<SkillOutput, String> {
        self.get(name).await
            .ok_or_else(|| format!("skill not found: {}", name))?
            .step(ctx).await
    }
}

impl Default for SkillRegistry {
    fn default() -> Self { Self::new() }
}

/// builtin skills 注册 helper
/// 0 装 PASS: 用 Arc<SkillRegistry> 避免借用问题, 调用方需在 tokio runtime 中
pub fn register_builtin_skills(reg: Arc<SkillRegistry>) {
    let calc: Arc<dyn Skill> = Arc::new(CalculatorSkill);
    tokio::spawn(async move { reg.register(calc).await });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculator_skill_sum() {
        let s = CalculatorSkill;
        let ctx = SkillContext {
            session_id: "s1".into(), user_input: "sum(3, 4)".into(),
            prior_results: HashMap::new(),
        };
        let out = s.step(&ctx).await.unwrap();
        assert!(out.success);
        assert_eq!(out.result, serde_json::json!(7.0));
    }

    #[tokio::test]
    async fn test_calculator_skill_div() {
        let s = CalculatorSkill;
        let ctx = SkillContext {
            session_id: "s1".into(), user_input: "div(10, 0)".into(),
            prior_results: HashMap::new(),
        };
        assert!(s.step(&ctx).await.is_err()); // div by 0
    }

    #[tokio::test]
    async fn test_skill_registry_invoke() {
        let reg = SkillRegistry::new();
        reg.register(Arc::new(CalculatorSkill)).await;
        assert_eq!(reg.list_names().await, vec!["calculator".to_string()]);
        let ctx = SkillContext {
            session_id: "s2".into(), user_input: "mul(6, 7)".into(),
            prior_results: HashMap::new(),
        };
        let out = reg.invoke("calculator", &ctx).await.unwrap();
        assert_eq!(out.result, serde_json::json!(42.0));
    }

    #[tokio::test]
    async fn test_skill_not_found() {
        let reg = SkillRegistry::new();
        let ctx = SkillContext {
            session_id: "s3".into(), user_input: "x".into(),
            prior_results: HashMap::new(),
        };
        assert!(reg.invoke("nonexistent", &ctx).await.is_err());
    }
}
