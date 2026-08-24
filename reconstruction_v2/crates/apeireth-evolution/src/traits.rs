//! 4 演化 trait — Learning / Abstraction / SelfModification / Extension
//!
//! **设计**: 所有 trait 由 `BasicEvolution` mock 实现, 不依赖真实 LLM/外部 SDK.

use crate::{EvolutionError, EvolutionResult};
use serde::{Deserialize, Serialize};

// ============================================================
// Episode (Learning 输入)
// ============================================================

/// 演化学习输入 — 一段行为历史。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Episode {
    /// Episode ID (全局唯一)
    pub id: String,
    /// 上下文 (领域描述)
    pub context: String,
    /// 观察信号 (键值)
    pub observations: Vec<(String, String)>,
    /// 实际行为
    pub action: String,
    /// 结果 (success / failure)
    pub outcome: String,
    /// 时间戳 (epoch ms)
    pub at_ms: i64,
}

impl Episode {
    /// 构造 episode。
    pub fn new(
        id: impl Into<String>,
        context: impl Into<String>,
        action: impl Into<String>,
        outcome: impl Into<String>,
        at_ms: i64,
    ) -> Self {
        Self {
            id: id.into(),
            context: context.into(),
            observations: Vec::new(),
            action: action.into(),
            outcome: outcome.into(),
            at_ms,
        }
    }

    /// 添加观察。
    pub fn with_observation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.observations.push((key.into(), value.into()));
        self
    }

    /// 是否为失败 episode。
    pub fn is_failure(&self) -> bool {
        self.outcome.to_lowercase().contains("fail")
            || self.outcome.to_lowercase().contains("error")
            || self.outcome.to_lowercase().contains("reject")
    }

    /// 验证 (空 context / action → 失败)。
    pub fn validate(&self) -> EvolutionResult<()> {
        if self.id.trim().is_empty() {
            return Err(EvolutionError::InvalidEpisode("id is empty".into()));
        }
        if self.context.trim().is_empty() {
            return Err(EvolutionError::InvalidEpisode("context is empty".into()));
        }
        if self.action.trim().is_empty() {
            return Err(EvolutionError::InvalidEpisode("action is empty".into()));
        }
        Ok(())
    }
}

// ============================================================
// Concept (Abstraction 产出)
// ============================================================

/// 抽象概念。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Concept {
    /// 概念 ID
    pub id: String,
    /// 概念名
    pub name: String,
    /// 关键特征 (提取自 examples)
    pub features: Vec<String>,
    /// 覆盖 examples 数
    pub example_count: usize,
}

impl Concept {
    /// 构造概念。
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        features: Vec<String>,
        example_count: usize,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            features,
            example_count,
        }
    }
}

// ============================================================
// Patch (SelfModification 产出)
// ============================================================

/// 系统补丁提案。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    /// 补丁 ID
    pub id: String,
    /// 目标层 ("L1" / "L2" / "L3" / "L4" / "L5" / "L0")
    pub target_layer: String,
    /// 修改描述
    pub description: String,
    /// 风险评估 ("low" / "medium" / "high" / "nuclear")
    pub risk: String,
}

impl Patch {
    /// 构造 patch。
    pub fn new(
        id: impl Into<String>,
        target_layer: impl Into<String>,
        description: impl Into<String>,
        risk: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            target_layer: target_layer.into(),
            description: description.into(),
            risk: risk.into(),
        }
    }

    /// 是否触及 L0 (硬件锚定层 — 演化拒绝)。
    pub fn targets_l0(&self) -> bool {
        self.target_layer.eq_ignore_ascii_case("L0") || self.target_layer == crate::L0_ANCHOR
    }
}

// ============================================================
// Plugin / PluginRegistry (Extension 用)
// ============================================================

/// 6 类插件类型 (与 apeireth-extension 协同)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginKind {
    /// 工具
    Tool,
    /// 传感器
    Sensor,
    /// 动作
    Action,
    /// 记忆
    Memory,
    /// 推理
    Reason,
    /// 渲染
    Render,
}

/// 插件抽象。
pub trait Plugin: Send + Sync {
    /// 插件名
    fn name(&self) -> &str;
    /// 插件类型
    fn kind(&self) -> PluginKind;
    /// 执行 (输入 → 输出字符串)
    fn execute(&self, input: &str) -> EvolutionResult<String>;
}

/// 插件注册表。
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugin_count", &self.plugins.len())
            .field("names", &self.plugins_names())
            .finish()
    }
}

impl PluginRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// 注册插件。
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// 计数。
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// 查找按名。
    pub fn find(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins
            .iter()
            .find(|p| p.name() == name)
            .map(|b| b.as_ref())
    }

    /// 按类型列出名。
    pub fn names_of_kind(&self, kind: PluginKind) -> Vec<&str> {
        self.plugins
            .iter()
            .filter(|p| p.kind() == kind)
            .map(|p| p.name())
            .collect()
    }

    fn plugins_names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }
}

// ============================================================
// SystemState (SelfModification 上下文)
// ============================================================

/// 系统状态快照 (供 SelfModification 读取)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemState {
    /// 当前 epoch ms
    pub at_ms: i64,
    /// 当前活跃 layer
    pub active_layer: String,
    /// 健康度 0.0-1.0
    pub health_score: u32,
    /// 备注
    pub note: String,
}

impl SystemState {
    /// 便利构造。
    pub fn new(active_layer: impl Into<String>, health_score: u32) -> Self {
        Self {
            at_ms: crate::current_time_ms(),
            active_layer: active_layer.into(),
            health_score,
            note: String::new(),
        }
    }
}

// ============================================================
// 4 trait
// ============================================================

/// Learning trait — 从 episode 更新内部状态。
pub trait Learning {
    /// 学习一段 episode。
    fn learn(&mut self, episode: &Episode) -> EvolutionResult<()>;
    /// 当前知识强度 (0.0-1.0)。
    fn knowledge_score(&self) -> f64;
    /// 重置。
    fn reset(&mut self);
}

/// Abstraction trait — 从 episodes 抽出概念。
pub trait Abstraction {
    /// 抽象一组 episode → Concept (空集返回 None)。
    fn abstract_concept(&self, examples: &[Episode]) -> Option<Concept>;
}

/// SelfModification trait — 产出系统补丁提案。
pub trait SelfModification {
    /// 基于当前状态产出补丁。
    fn propose_patch(&self, current: &SystemState) -> Patch;
    /// 默认 L0 拒绝 (派生 trait 时可重写)。
    fn reject_l0(&self) -> bool {
        true
    }
}

/// Extension trait — 拓展能力 (注册插件)。
pub trait Extension {
    /// 注册插件到内部 registry。
    fn extend_capability(&mut self, plugin: Box<dyn Plugin>) -> EvolutionResult<()>;
    /// 列出插件数。
    fn plugin_count(&self) -> usize;
}

// ============================================================
// BasicEvolution — mock 实现 (无 LLM / 外部 SDK)
// ============================================================

/// 4 trait 的示例实现 (测试 + 演示用, 不依赖真实 LLM)。
#[derive(Debug, Default)]
pub struct BasicEvolution {
    knowledge: f64,
    plugins: PluginRegistry,
    learn_log: Vec<String>,
}

impl BasicEvolution {
    /// 构造示例实现。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前学习日志 (只读视图)。
    pub fn learn_log(&self) -> &[String] {
        &self.learn_log
    }

    /// 注册过的概念 (仅 BasicEvolution 内部; 真实实现可改为外部存储)。
    pub fn registered_concepts(&self) -> &[String] {
        &[]
    }
}

impl Learning for BasicEvolution {
    fn learn(&mut self, episode: &Episode) -> EvolutionResult<()> {
        episode.validate()?;
        // 失败 episode 减权; 成功 episode 增权 (clamp 0-1)。
        let delta: f64 = if episode.is_failure() { -0.10 } else { 0.05 };
        let new_score = (self.knowledge + delta).clamp(0.0, 1.0);
        self.knowledge = if new_score.is_finite() {
            new_score
        } else {
            0.0
        };
        self.learn_log.push(format!(
            "{}::{} -> {:.3}",
            episode.id, episode.action, self.knowledge
        ));
        Ok(())
    }

    fn knowledge_score(&self) -> f64 {
        self.knowledge
    }

    fn reset(&mut self) {
        self.knowledge = 0.0;
        self.learn_log.clear();
    }
}

impl Abstraction for BasicEvolution {
    fn abstract_concept(&self, examples: &[Episode]) -> Option<Concept> {
        let valid: Vec<&Episode> = examples.iter().filter(|e| e.validate().is_ok()).collect();
        if valid.is_empty() {
            return None;
        }
        // 共同前缀作为概念名 (确定性算法)
        let first = &valid[0].context;
        let mut boundary = first.len();
        for ep in &valid[1..] {
            let matched: usize = first
                .chars()
                .zip(ep.context.chars())
                .take_while(|(a, b)| a == b)
                .map(|(c, _)| c.len_utf8())
                .sum();
            boundary = boundary.min(matched);
        }
        if boundary == 0 {
            return None;
        }
        let name = first[..boundary].trim().to_string();
        if name.is_empty() {
            return None;
        }
        Some(Concept::new(
            format!("concept-{}", name.replace(' ', "-")),
            name,
            vec![format!("examples={}", valid.len())],
            valid.len(),
        ))
    }
}

impl SelfModification for BasicEvolution {
    fn propose_patch(&self, current: &SystemState) -> Patch {
        let risk = if current.health_score >= 80 {
            "low"
        } else if current.health_score >= 50 {
            "medium"
        } else {
            "high"
        };
        Patch::new(
            format!("patch-{}", current.at_ms),
            current.active_layer.clone(),
            format!(
                "Auto-proposed patch from BasicEvolution (health={})",
                current.health_score
            ),
            risk,
        )
    }

    fn reject_l0(&self) -> bool {
        true
    }
}

impl Extension for BasicEvolution {
    fn extend_capability(&mut self, plugin: Box<dyn Plugin>) -> EvolutionResult<()> {
        if plugin.name().trim().is_empty() {
            return Err(EvolutionError::PluginLoadFailed("plugin name empty".into()));
        }
        if self.plugins.find(plugin.name()).is_some() {
            return Err(EvolutionError::PluginLoadFailed(format!(
                "plugin '{}' already registered",
                plugin.name()
            )));
        }
        self.plugins.register(plugin);
        Ok(())
    }

    fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

// ============================================================
// Mock Plugin — 测试用
// ============================================================

/// 测试用 mock plugin。
pub struct MockPlugin {
    name: String,
    kind: PluginKind,
    response: String,
}

impl MockPlugin {
    /// 构造。
    pub fn new(name: impl Into<String>, kind: PluginKind, response: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind,
            response: response.into(),
        }
    }
}

impl Plugin for MockPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> PluginKind {
        self.kind
    }
    fn execute(&self, input: &str) -> EvolutionResult<String> {
        if input.trim().is_empty() {
            return Err(EvolutionError::PluginLoadFailed("empty input".into()));
        }
        Ok(format!("{}::{}", self.response, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(id: &str, ctx: &str, action: &str, outcome: &str) -> Episode {
        Episode::new(id, ctx, action, outcome, crate::current_time_ms())
    }

    #[test]
    fn episode_validate_rejects_empty_context() {
        let e = Episode::new("a", "", "act", "ok", 0);
        assert!(e.validate().is_err());
    }

    #[test]
    fn episode_validate_rejects_empty_id() {
        let e = Episode::new("", "ctx", "act", "ok", 0);
        assert!(e.validate().is_err());
    }

    #[test]
    fn episode_validate_rejects_empty_action() {
        let e = Episode::new("a", "ctx", "", "ok", 0);
        assert!(e.validate().is_err());
    }

    #[test]
    fn episode_validate_accepts_valid() {
        let e = ep("a", "ctx", "act", "ok");
        assert!(e.validate().is_ok());
    }

    #[test]
    fn episode_is_failure_recognises_keywords() {
        assert!(ep("a", "ctx", "act", "failure").is_failure());
        assert!(ep("a", "ctx", "act", "ERROR").is_failure());
        assert!(ep("a", "ctx", "act", "rejected").is_failure());
        assert!(!ep("a", "ctx", "act", "ok").is_failure());
    }

    #[test]
    fn episode_with_observation_appends() {
        let e = ep("a", "ctx", "act", "ok").with_observation("k1", "v1");
        assert_eq!(e.observations.len(), 1);
    }

    #[test]
    fn patch_targets_l0_recognises() {
        assert!(Patch::new("p", "L0", "d", "low").targets_l0());
        assert!(Patch::new("p", "l0", "d", "low").targets_l0());
        assert!(Patch::new("p", crate::L0_ANCHOR, "d", "low").targets_l0());
        assert!(!Patch::new("p", "L1", "d", "low").targets_l0());
    }

    #[test]
    fn plugin_registry_register_and_find() {
        let mut reg = PluginRegistry::new();
        assert!(reg.is_empty());
        let p: Box<dyn Plugin> = Box::new(MockPlugin::new("p1", PluginKind::Tool, "out"));
        reg.register(p);
        assert_eq!(reg.len(), 1);
        assert!(reg.find("p1").is_some());
        assert!(reg.find("missing").is_none());
    }

    #[test]
    fn plugin_registry_filter_by_kind() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(MockPlugin::new("t1", PluginKind::Tool, "x")));
        reg.register(Box::new(MockPlugin::new("s1", PluginKind::Sensor, "y")));
        reg.register(Box::new(MockPlugin::new("t2", PluginKind::Tool, "z")));
        assert_eq!(reg.names_of_kind(PluginKind::Tool), vec!["t1", "t2"]);
        assert_eq!(reg.names_of_kind(PluginKind::Sensor), vec!["s1"]);
        assert!(reg.names_of_kind(PluginKind::Action).is_empty());
    }

    #[test]
    fn mock_plugin_execute_rejects_empty() {
        let p = MockPlugin::new("p", PluginKind::Tool, "r");
        assert!(p.execute("").is_err());
        assert_eq!(p.execute("hi").unwrap(), "r::hi");
    }

    #[test]
    fn learning_trait_increases_then_clamps() {
        let mut e = BasicEvolution::new();
        let ep = ep("a", "ctx", "act", "ok");
        for _ in 0..50 {
            e.learn(&ep).unwrap();
        }
        assert!(e.knowledge_score() <= 1.0);
        assert!(e.knowledge_score() > 0.0);
    }

    #[test]
    fn learning_trait_decreases_on_failure() {
        let mut e = BasicEvolution::new();
        // 先涨
        for _ in 0..10 {
            e.learn(&ep("a", "ctx", "act", "ok")).unwrap();
        }
        let peak = e.knowledge_score();
        // 再跌
        for _ in 0..10 {
            e.learn(&ep("a", "ctx", "act", "failure")).unwrap();
        }
        assert!(e.knowledge_score() < peak);
        assert!(e.knowledge_score() >= 0.0);
    }

    #[test]
    fn learning_trait_rejects_invalid_episode() {
        let mut e = BasicEvolution::new();
        let bad = Episode::new("", "", "", "", 0);
        assert!(e.learn(&bad).is_err());
    }

    #[test]
    fn learning_trait_reset_clears_state() {
        let mut e = BasicEvolution::new();
        e.learn(&ep("a", "ctx", "act", "ok")).unwrap();
        e.reset();
        assert_eq!(e.knowledge_score(), 0.0);
        assert!(e.learn_log().is_empty());
    }

    #[test]
    fn abstraction_returns_none_for_empty() {
        let e = BasicEvolution::new();
        assert!(e.abstract_concept(&[]).is_none());
    }

    #[test]
    fn abstraction_finds_common_prefix() {
        let e = BasicEvolution::new();
        let examples = vec![
            ep("1", "auth.login", "act", "ok"),
            ep("2", "auth.refresh", "act", "ok"),
        ];
        let c = e.abstract_concept(&examples).unwrap();
        assert!(c.name.starts_with("auth"));
        assert_eq!(c.example_count, 2);
    }

    #[test]
    fn abstraction_returns_none_for_unrelated() {
        let e = BasicEvolution::new();
        let examples = vec![ep("1", "alpha", "act", "ok"), ep("2", "beta", "act", "ok")];
        assert!(e.abstract_concept(&examples).is_none());
    }

    #[test]
    fn abstraction_skips_invalid_episodes() {
        let e = BasicEvolution::new();
        let examples = vec![
            Episode::new("", "", "act", "ok", 0), // invalid
            ep("2", "auth.refresh", "act", "ok"),
        ];
        let c = e.abstract_concept(&examples);
        assert!(c.is_some()); // 至少 1 个 valid
        assert_eq!(c.unwrap().example_count, 1);
    }

    #[test]
    fn self_modification_patch_classifies_risk() {
        let e = BasicEvolution::new();
        let p_high = e.propose_patch(&SystemState::new("L3", 30));
        assert_eq!(p_high.risk, "high");
        let p_mid = e.propose_patch(&SystemState::new("L3", 60));
        assert_eq!(p_mid.risk, "medium");
        let p_low = e.propose_patch(&SystemState::new("L3", 90));
        assert_eq!(p_low.risk, "low");
    }

    #[test]
    fn self_modification_rejects_l0_by_default() {
        let e = BasicEvolution::new();
        assert!(e.reject_l0());
    }

    #[test]
    fn extension_register_and_dedup() {
        let mut e = BasicEvolution::new();
        let p: Box<dyn Plugin> = Box::new(MockPlugin::new("p1", PluginKind::Tool, "x"));
        e.extend_capability(p).unwrap();
        assert_eq!(e.plugin_count(), 1);

        // duplicate
        let p2: Box<dyn Plugin> = Box::new(MockPlugin::new("p1", PluginKind::Tool, "y"));
        assert!(e.extend_capability(p2).is_err());
        assert_eq!(e.plugin_count(), 1);
    }

    #[test]
    fn extension_rejects_empty_name() {
        let mut e = BasicEvolution::new();
        let p: Box<dyn Plugin> = Box::new(MockPlugin::new("", PluginKind::Tool, "x"));
        assert!(e.extend_capability(p).is_err());
    }

    #[test]
    fn plugin_kind_has_six_variants() {
        // 6 类 (Tool / Sensor / Action / Memory / Reason / Render) — 编译时 hardcode
        let kinds = [
            PluginKind::Tool,
            PluginKind::Sensor,
            PluginKind::Action,
            PluginKind::Memory,
            PluginKind::Reason,
            PluginKind::Render,
        ];
        assert_eq!(kinds.len(), 6);
        let mut seen = std::collections::HashSet::new();
        for k in kinds {
            assert!(seen.insert(k));
        }
    }

    #[test]
    fn basic_evolution_default_is_zero() {
        let e = BasicEvolution::default();
        assert_eq!(e.knowledge_score(), 0.0);
        assert_eq!(e.plugin_count(), 0);
    }
}
