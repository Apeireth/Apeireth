//! R110: Skill descriptor → eval scenario 桥接 (动态运营层 → 评测层)
//!
//! **目标**: 把 SkillDescriptor 转成 `EvalScenario` 给 apeireth-eval 注册, 实现
//! "skill 上线自动生成 eval scenario" 闭环. 不引入新跨 crate dep, scenario struct 自定义.
//!
//! **Apeireth 真接 (本 module)**:
//! - `EvalScenario` struct — id (跟 skill.id) + prompt (从 input_example 提) + expected (output_example) + source + tags
//! - `descriptor_to_eval_scenario(desc) -> EvalScenario` — 单 descriptor 转 scenario
//! - `descriptors_to_eval_scenarios(descs) -> Vec<EvalScenario>` — 批量
//! - `descriptors_to_eval_scores(descs) -> Vec<EvalScore>` — 包装成 apeireth-eval 6 module 的 EvalScore
//!   (需要 apeireth-eval 在 caller 端 import, 本 module 0 改 apeireth-eval)
//! - `scenario_to_eval_score(scenario, value) -> EvalScore` — 单 scenario 配 score
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `apeireth-skills/src/descriptor.rs` 已有 SkillDescriptor 7 字段
//! - 0 改 `apeireth-skills/src/lib.rs` 已有 8 pub fn / Skill / Registry
//! - 0 改 `apeireth-eval` 任何 (caller 用我们的 scenario 自行包装 EvalScore, 0 强制跨 crate dep)
//!
//! **借鉴锚 (S-8)**:
//! - LangChain `EvaluatorCallbackHandler` (每个 tool 跑完自动 eval)
//! - VCP `vcptoolbox/modules/eval` (模块自带 eval scenario)
//! - OpenAI Evals §composite (从 registry 抽 scenario)

use serde::{Deserialize, Serialize};

use crate::descriptor::SkillDescriptor;

// ============================================================
// EvalScenario
// ============================================================

/// **Skill 派生的 eval scenario** (per skill 一个 scenario)
///
/// 跟 apeireth-eval 的 `SmokeReport` / `RealLlmSmokeReport` 正交, 1:1 包装 SkillDescriptor.
/// caller 自行决定怎么用 (e.g. 包 EvalScore / 注册到 eval pool).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalScenario {
    /// scenario id (跟 skill.id 1:1, e.g. "summarize-text")
    pub id: String,
    /// 人类可读 prompt (从 skill.description + input_example 拼)
    pub prompt: String,
    /// 期望 output (从 output_example 直接拿)
    pub expected: String,
    /// 来源标识 (跟 skill.source 1:1, 给报告用)
    pub source: String,
    /// 路由 tags (跟 skill.tags 1:1)
    pub tags: Vec<String>,
    /// skill version (给 scenario 版本用)
    pub skill_version: String,
}

impl EvalScenario {
    /// **便利构造** (5 字段)
    pub fn new(
        id: impl Into<String>,
        prompt: impl Into<String>,
        expected: impl Into<String>,
        source: impl Into<String>,
        skill_version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            expected: expected.into(),
            source: source.into(),
            tags: Vec::new(),
            skill_version: skill_version.into(),
        }
    }

    /// 加 tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.tags = tags.into_iter().collect();
        self
    }

    /// **prompt 是否非空** (空 prompt 不能跑 eval, 跳过)
    pub fn is_runnable(&self) -> bool {
        !self.prompt.trim().is_empty() && !self.expected.trim().is_empty()
    }
}

// ============================================================
// 转换函数
// ============================================================

/// **单个 SkillDescriptor → EvalScenario**
///
/// prompt 拼装规则: "{description}\n\nInput: {input_example}"
/// expected: 直接 output_example
pub fn descriptor_to_eval_scenario(desc: &SkillDescriptor) -> EvalScenario {
    let prompt = format!("{}\n\nInput: {}", desc.description, desc.input_example);
    let mut scenario = EvalScenario::new(
        desc.id.clone(),
        prompt,
        desc.output_example.clone(),
        desc.source.clone(),
        desc.version.clone(),
    );
    scenario.tags = desc.tags.iter().cloned().collect();
    scenario
}

/// **批量 descriptors → scenarios**
pub fn descriptors_to_eval_scenarios(descs: &[SkillDescriptor]) -> Vec<EvalScenario> {
    descs.iter().map(descriptor_to_eval_scenario).collect()
}

/// **过滤可跑 scenario** (prompt 和 expected 都非空)
pub fn runnable_scenarios(scenarios: &[EvalScenario]) -> Vec<&EvalScenario> {
    scenarios.iter().filter(|s| s.is_runnable()).collect()
}

/// **统计 scenario 来源分布** (返回 {source: count})
pub fn scenarios_by_source(
    scenarios: &[EvalScenario],
) -> std::collections::BTreeMap<String, usize> {
    let mut map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for s in scenarios {
        *map.entry(s.source.clone()).or_insert(0) += 1;
    }
    map
}

// ============================================================
// EvalScore 镜像 (给 caller 用, 0 强制跨 crate dep)
// ============================================================

/// **EvalScore-like 镜像** (本 module 独立定义, 跟 apeireth-eval::EvalScore 字段 1:1)
///
/// caller 端 import apeireth-eval 后, 可直接用 `scenario_to_eval_score(scenario, value)`
/// 构 apeireth_eval::EvalScore. 本镜像方便 0 dep 单元测试.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalScoreMirror {
    pub dimension: String,
    pub value: f64,
}

impl EvalScoreMirror {
    pub fn new(dimension: impl Into<String>, value: f64) -> Self {
        Self {
            dimension: dimension.into(),
            value,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.value.is_finite() && (0.0..=1.0).contains(&self.value)
    }
}

/// **scenario + value → EvalScore mirror** (per scenario 一个 score, dimension=scenario.id)
pub fn scenario_to_eval_score(scenario: &EvalScenario, value: f64) -> EvalScoreMirror {
    EvalScoreMirror::new(scenario.id.clone(), value)
}

/// **批量 descriptors → EvalScore mirrors** (全 1.0 表示 "scenario 配出来了", 实际值 caller 算)
pub fn descriptors_to_eval_score_mirrors(
    descs: &[SkillDescriptor],
    default_value: f64,
) -> Vec<EvalScoreMirror> {
    descs
        .iter()
        .map(|d| EvalScoreMirror::new(d.id.clone(), default_value))
        .collect()
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn make_desc(
        id: &str,
        version: &str,
        description: &str,
        tags: &[&str],
        source: &str,
    ) -> SkillDescriptor {
        SkillDescriptor::new(
            id,
            version,
            description,
            tags.iter().map(|s| (*s).to_string()),
            source,
        )
        .with_examples(
            r#"{"input": "test"}"#.to_string(),
            r#"{"output": "ok"}"#.to_string(),
        )
    }

    fn make_empty_desc(id: &str) -> SkillDescriptor {
        SkillDescriptor::new(id, "1.0.0", "desc", vec![] as Vec<String>, "test")
            .with_examples("".to_string(), "".to_string())
    }

    #[test]
    fn eval_scenario_new_and_with_tags() {
        let s = EvalScenario::new("s", "prompt", "expected", "vcptoolbox", "1.0.0")
            .with_tags(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(s.id, "s");
        assert_eq!(s.tags.len(), 2);
    }

    #[test]
    fn eval_scenario_is_runnable() {
        let s = EvalScenario::new("s", "prompt", "expected", "test", "1.0.0");
        assert!(s.is_runnable());

        let empty_prompt = EvalScenario::new("s", "", "expected", "test", "1.0.0");
        assert!(!empty_prompt.is_runnable());

        let whitespace_prompt = EvalScenario::new("s", "  \n  ", "expected", "test", "1.0.0");
        assert!(!whitespace_prompt.is_runnable());
    }

    #[test]
    fn descriptor_to_eval_scenario_basic() {
        let desc = make_desc(
            "summarize-text",
            "1.5.0",
            "Summarize a text",
            &["summarize", "text"],
            "vcptoolbox",
        );
        let s = descriptor_to_eval_scenario(&desc);
        assert_eq!(s.id, "summarize-text");
        assert_eq!(s.expected, r#"{"output": "ok"}"#);
        assert_eq!(s.source, "vcptoolbox");
        assert_eq!(s.skill_version, "1.5.0");
        assert!(s.prompt.contains("Summarize a text"));
        assert!(s.prompt.contains(r#"{"input": "test"}"#));
        assert_eq!(s.tags.len(), 2);
        assert!(s.tags.contains(&"summarize".to_string()));
    }

    #[test]
    fn descriptor_to_eval_scenario_without_tags() {
        let desc = make_desc("plain", "1.0.0", "Plain skill", &[], "local");
        let s = descriptor_to_eval_scenario(&desc);
        assert_eq!(s.tags.len(), 0);
        assert!(s.is_runnable());
    }

    #[test]
    fn descriptor_to_eval_scenario_with_empty_examples_not_runnable() {
        let desc = make_empty_desc("empty");
        let s = descriptor_to_eval_scenario(&desc);
        assert!(!s.is_runnable());
    }

    #[test]
    fn descriptors_to_eval_scenarios_batch() {
        let descs = vec![
            make_desc("a", "1.0.0", "A", &[], "local"),
            make_desc("b", "1.0.0", "B", &[], "local"),
            make_empty_desc("empty"),
        ];
        let scenarios = descriptors_to_eval_scenarios(&descs);
        assert_eq!(scenarios.len(), 3);
        assert!(scenarios[0].is_runnable());
        assert!(scenarios[1].is_runnable());
        assert!(!scenarios[2].is_runnable());
    }

    #[test]
    fn runnable_scenarios_filters_empty() {
        let descs = vec![
            make_desc("a", "1.0.0", "A", &[], "local"),
            make_empty_desc("empty1"),
            make_empty_desc("empty2"),
        ];
        let scenarios = descriptors_to_eval_scenarios(&descs);
        let runnable = runnable_scenarios(&scenarios);
        assert_eq!(runnable.len(), 1);
        assert_eq!(runnable[0].id, "a");
    }

    #[test]
    fn scenarios_by_source_groups() {
        let descs = vec![
            make_desc("a", "1.0.0", "A", &[], "vcptoolbox"),
            make_desc("b", "1.0.0", "B", &[], "local"),
            make_desc("c", "1.0.0", "C", &[], "vcptoolbox"),
            make_desc("d", "1.0.0", "D", &[], "apeireth-eval"),
        ];
        let scenarios = descriptors_to_eval_scenarios(&descs);
        let grouped = scenarios_by_source(&scenarios);
        assert_eq!(grouped.get("vcptoolbox"), Some(&2));
        assert_eq!(grouped.get("local"), Some(&1));
        assert_eq!(grouped.get("apeireth-eval"), Some(&1));
    }

    #[test]
    fn eval_score_mirror_basic() {
        let s = EvalScoreMirror::new("x", 0.85);
        assert_eq!(s.dimension, "x");
        assert_eq!(s.value, 0.85);
        assert!(s.is_valid());
    }

    #[test]
    fn eval_score_mirror_invalid() {
        assert!(!EvalScoreMirror::new("x", 1.5).is_valid());
        assert!(!EvalScoreMirror::new("x", -0.1).is_valid());
        assert!(!EvalScoreMirror::new("x", f64::NAN).is_valid());
    }

    #[test]
    fn scenario_to_eval_score_uses_id_as_dimension() {
        let s = EvalScenario::new("scenario-x", "p", "e", "test", "1.0.0");
        let score = scenario_to_eval_score(&s, 0.9);
        assert_eq!(score.dimension, "scenario-x");
        assert_eq!(score.value, 0.9);
    }

    #[test]
    fn descriptors_to_eval_score_mirrors_uses_default() {
        let descs = vec![
            make_desc("a", "1.0.0", "A", &[], "local"),
            make_desc("b", "1.0.0", "B", &[], "local"),
        ];
        let scores = descriptors_to_eval_score_mirrors(&descs, 1.0);
        assert_eq!(scores.len(), 2);
        assert!(scores[0].is_valid());
        assert!(scores[1].is_valid());
        assert_eq!(scores[0].dimension, "a");
    }

    #[test]
    fn eval_scenario_serde_round_trip() {
        let s = EvalScenario::new("id", "prompt", "expected", "src", "1.0.0")
            .with_tags(vec!["t1".to_string()]);
        let json = serde_json::to_string(&s).unwrap();
        let back: EvalScenario = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn eval_scenario_prompt_includes_description_and_input() {
        let desc = make_desc("x", "1.0.0", "Custom description", &["t"], "local");
        let s = descriptor_to_eval_scenario(&desc);
        // prompt = "{description}\n\nInput: {input_example}"
        assert!(s.prompt.starts_with("Custom description\n\nInput: "));
    }
}
