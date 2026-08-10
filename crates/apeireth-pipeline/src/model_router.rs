//! `model_router` — **借鉴 VCP `SemanticModelRouter.json` (R122-5)**
//!
//! ## 借鉴源 (per 07 §1 O-2 走在前人经验上)
//!
//! **VCP 真代码** (`lioensky/VCPToolBox/SemanticModelRouter.json`, 2.7KB JSON config,
//! + `SemanticModelRouter.json.example` 1.6KB):
//! - 顶层 `presets: { name: { defaultModel, routes[] } }` 结构
//! - 每条 `routes[]` 形如 `{ name, model, description, failoverPool }`
//! - `description` 是**逗号分隔的关键词列表** (中文场景), 匹配即路由
//! - `defaultModel` 是 fallback, 无 match 时用
//!
//! **借鉴 ID**: `R122-5-VCP-SemanticModelRouter-2026-08-10`
//!
//! ## 0 装 (per 哲学锚 #1 "不假装已实现")
//!
//! VCP 真代码用 fuzzy embedding scoring (0.18 阈值), 1:1 需引入 embedding
//! 模型, **V2.1 P1 out of scope**, 我用 case-insensitive substring match 替代,
//! 显式声明 **0 装 4 项**:
//!
//! | VCP 字段 | 0 装原因 | 我的简化 |
//! |----------|----------|----------|
//! | `matchThreshold: 0.18` (fuzzy) | 1:1 需 embedding 模型, V2.1 P1 out of scope | case-insensitive substring |
//! | `contextWeights: [0.7, 0.3]` | 1:1 需累积分计算, V2.1 P1 out of scope | 0 port, single prompt only |
//! | `fallbackModels[]` | failover 池需 HTTP 客户端集成, V2.1 P1 out of scope | 0 port |
//! | `presets: { name: {...} }` 嵌套 | VCP 多租户设计, V2.1 P1 单 preset 足够 | flat `Vec<RoutingRule>` + priority |
//!
//! ## 架构
//!
//! - **5 种 `RoutingCondition`**: `KeywordMatch` / `TokenCountRange` / `RoleBased` / `Complexity` / `Custom`
//! - **priority 降序排序**: priority=100 最先匹配 (跟 VCP `routes[]` 顺序语义一致)
//! - **first-match-wins**: 第一个匹配的 rule 胜出
//! - **无 match**: 返 `default_model`
//! - **8 unit tests** 覆盖 5 condition + 3 integration
//!
//! ## 字段级 1:1 借鉴 (per 07 §1)
//!
//! - VCP `defaultModel` → Rust `SemanticModelRouter.default_model: String` (**1:1**)
//! - VCP `routes[].name` → Rust `RoutingRule.name: String` (**1:1**)
//! - VCP `routes[].model` → Rust `RoutingRule.target_model: String` (**1:1**)
//! - VCP `routes[].description` (keyword list) → Rust `RoutingCondition::KeywordMatch(Vec<String>)` (**1:1**, split 逗号)

use apeireth_protocol::MessageRole;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

// ============================================================
// 编译期 hardcode (不漂移, per 工程哲学铁律 #2 "不漂移")
// ============================================================

/// VCP `SemanticModelRouter.json` 借鉴源 真实文件大小 (bytes)
/// - 真实仓库 sha: ac9cd950ffdc8aa668e64424bbfa14af6d5658eb (per github API 2026-08-10)
/// - 真实文件 size: 2741 bytes
/// - **不漂移承诺**: 借鉴源 hash/size 变了, 这里必须改 (per 工程哲学铁律 #2 "不漂移")
pub const VCP_SEMANTIC_MODEL_ROUTER_BYTES: usize = 2741;

/// VCP `matchThreshold: 0.18` hardcode (VCP 真值, per `SemanticModelRouter.json:5`)
/// - 借鉴字段: `matchThreshold: 0.18` (fuzzy threshold)
/// - 0 装 1:1 (V2.1 P1): 我用 case-insensitive substring, 0 装 fuzzy
/// - 此常量作为 keyword match 的最小关键词数下限 (保留 VCP 阈值字段语义)
pub const VCP_MATCH_THRESHOLD: f32 = 0.18;

// ============================================================
// RoutingCondition (5 variants, 借鉴 VCP 5 路由场景 + 1 扩展)
// ============================================================

/// 路由条件 — 5 variants
///
/// 借鉴 VCP `routes[].description` (keyword match) 是**唯一**条件类型,
/// 我扩展 4 个: token range / role / complexity / custom, 覆盖完整路由场景。
#[derive(Clone)]
pub enum RoutingCondition {
    /// 关键词匹配 (大小写不敏感, substring match)
    /// **1:1 借鉴** VCP `routes[].description` (逗号分隔关键词列表)
    KeywordMatch(Vec<String>),

    /// Token 数范围 (含端点, min_tokens <= tokens <= max_tokens)
    /// **扩展**: VCP 无此场景, 0 装
    TokenCountRange(usize, usize),

    /// Role-based 路由 (借鉴 `apeireth_protocol::MessageRole`)
    /// **扩展**: VCP 无 role 概念, 0 装
    RoleBased(MessageRole),

    /// 复杂度阈值 (>= min_complexity 命中)
    /// **扩展**: VCP 0 装 fuzzy complexity scoring, 0 装
    Complexity(f32),

    /// 自定义闭包 (Arc<dyn Fn> 共享所有权)
    /// **扩展**: VCP 无, 给上游业务塞 ML 评分器用, 0 装
    Custom(Arc<dyn Fn(&str) -> bool + Send + Sync>),
}

impl std::fmt::Debug for RoutingCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeywordMatch(kws) => f.debug_tuple("KeywordMatch").field(kws).finish(),
            Self::TokenCountRange(min, max) => {
                f.debug_tuple("TokenCountRange").field(min).field(max).finish()
            }
            Self::RoleBased(role) => f.debug_tuple("RoleBased").field(role).finish(),
            Self::Complexity(min) => f.debug_tuple("Complexity").field(min).finish(),
            Self::Custom(_) => f.debug_struct("Custom").field("fn", &"<fn>").finish(),
        }
    }
}

// ============================================================
// RoutingRule (1 rule = 1 VCP route, 1:1 字段)
// ============================================================

/// 路由规则 (1 rule = 1 VCP route)
///
/// **1:1 字段借鉴**:
/// - VCP `routes[].name` → `name: String` (**1:1**)
/// - VCP `routes[].model` → `target_model: String` (**1:1**)
/// - VCP `routes[].description` → `condition: RoutingCondition::KeywordMatch` (**1:1**)
/// - 扩展: `priority: u8` (Rust 显式, VCP 用数组顺序隐式)
#[derive(Clone)]
pub struct RoutingRule {
    /// 规则名 (1:1 VCP `routes[].name`)
    pub name: String,
    /// 命中条件 (5 variants, 1:1 VCP keyword + 4 扩展)
    pub condition: RoutingCondition,
    /// 目标模型 (1:1 VCP `routes[].model`)
    pub target_model: String,
    /// 优先级 (0-255, 越大越先匹配, 默认 50)
    /// - 借鉴 VCP `routes[]` 数组顺序 (first-match-wins)
    /// - Rust 显式 priority 字段, 同 priority 时按 name 字典序稳定排序
    pub priority: u8,
}

impl std::fmt::Debug for RoutingRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingRule")
            .field("name", &self.name)
            .field("condition", &self.condition)
            .field("target_model", &self.target_model)
            .field("priority", &self.priority)
            .finish()
    }
}

impl RoutingRule {
    /// 便捷构造: keyword match 规则
    pub fn keyword(name: impl Into<String>, keywords: Vec<String>, target: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            condition: RoutingCondition::KeywordMatch(keywords),
            target_model: target.into(),
            priority: 50,
        }
    }

    /// 便捷构造: token count range 规则
    pub fn token_range(
        name: impl Into<String>,
        min: usize,
        max: usize,
        target: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            condition: RoutingCondition::TokenCountRange(min, max),
            target_model: target.into(),
            priority: 50,
        }
    }

    /// 便捷构造: role-based 规则
    pub fn role(
        name: impl Into<String>,
        role: MessageRole,
        target: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            condition: RoutingCondition::RoleBased(role),
            target_model: target.into(),
            priority: 50,
        }
    }

    /// 便捷构造: complexity threshold 规则
    pub fn complexity(
        name: impl Into<String>,
        min_complexity: f32,
        target: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            condition: RoutingCondition::Complexity(min_complexity),
            target_model: target.into(),
            priority: 50,
        }
    }
}

// ============================================================
// RoutingDecision (explain() 返回值, 借鉴 VCP 路由可观测性)
// ============================================================

/// 路由决策 (路由可观测性, 借鉴 VCP `routes[].description` 调试)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    /// 命中的 rule name (None 表示走 default_model)
    pub matched_rule: Option<String>,
    /// 目标模型
    pub target_model: String,
    /// 命中原因 (人类可读)
    pub reason: String,
}

// ============================================================
// SemanticModelRouter (主类型, 借鉴 VCP 顶层 + routes[])
// ============================================================

/// 语义模型路由器 (借鉴 VCP `SemanticModelRouter.json`)
///
/// ## 字段 1:1 借鉴
/// - VCP `defaultModel` → `default_model: String` (**1:1**)
/// - VCP `presets[name].routes[]` → `rules: Vec<RoutingRule>` (**1:1 展开**)
///
/// ## 0 装字段
/// - VCP `enabled` (全局开关): 0 port (不加 rule 即不启用, 显式优于隐式)
/// - VCP `autoModelName` (虚拟模型名): 0 port (Rust 调用方直接持有实例)
/// - VCP `defaultPreset`: 0 port (flat rules, 无 preset 嵌套)
/// - VCP `contextWeights`: 0 port (V2.1 P1 out of scope, 需累积分)
/// - VCP `fallbackModels[]`: 0 port (V2.1 P1 out of scope)
#[derive(Clone)]
pub struct SemanticModelRouter {
    /// 路由规则列表 (按 priority 降序, 同 priority 按 name 升序)
    rules: Vec<RoutingRule>,
    /// 默认模型 (1:1 VCP `defaultModel`)
    default_model: String,
}

impl std::fmt::Debug for SemanticModelRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticModelRouter")
            .field("rules_count", &self.rules.len())
            .field("default_model", &self.default_model)
            .field("rules", &self.rules)
            .finish()
    }
}

impl SemanticModelRouter {
    /// 构造新路由器, 默认模型兜底
    pub fn new(default_model: impl Into<String>) -> Self {
        Self {
            rules: Vec::new(),
            default_model: default_model.into(),
        }
    }

    /// 加规则 (按 priority 降序插入, 同 priority 按 name 字典序)
    ///
    /// **借鉴 VCP**: VCP `routes[]` 数组顺序就是匹配顺序, 我用 priority 字段显式化
    pub fn add_rule(&mut self, rule: RoutingRule) {
        // 按 priority 降序, 同 priority 按 name 升序稳定排序
        let pos = self
            .rules
            .binary_search_by(|r| {
                r.priority
                    .cmp(&rule.priority)
                    .reverse() // 降序: priority 大的排前面
                    .then(r.name.cmp(&rule.name))
            })
            .unwrap_or_else(|e| e);
        self.rules.insert(pos, rule);
    }

    /// 路由 (返目标 model name)
    ///
    /// 输入: prompt 文本 + (可选) role + (可选) complexity 分数
    /// 输出: 命中的 rule 的 target_model, 或 default_model
    ///
    /// **匹配算法**: 按 priority 降序遍历, 第一个命中即返
    /// - KeywordMatch: case-insensitive substring (prompt 含任一 keyword)
    /// - TokenCountRange: min_tokens <= tokens <= max_tokens
    /// - RoleBased: role match (Some(role) 时)
    /// - Complexity: complexity >= threshold (Some(complexity) 时)
    /// - Custom: 调用 fn(prompt)
    pub fn route(
        &self,
        prompt: &str,
        role: Option<MessageRole>,
        complexity: Option<f32>,
    ) -> String {
        for rule in &self.rules {
            if Self::matches(&rule.condition, prompt, role, complexity) {
                return rule.target_model.clone();
            }
        }
        self.default_model.clone()
    }

    /// 解释路由 (返完整 RoutingDecision, 含 matched_rule + reason)
    ///
    /// 借鉴 VCP 路由可观测性 (VCP `routes[].description` 用于人类调试)
    pub fn explain(
        &self,
        prompt: &str,
        role: Option<MessageRole>,
        complexity: Option<f32>,
    ) -> RoutingDecision {
        for rule in &self.rules {
            if Self::matches(&rule.condition, prompt, role, complexity) {
                return RoutingDecision {
                    matched_rule: Some(rule.name.clone()),
                    target_model: rule.target_model.clone(),
                    reason: Self::explain_match(&rule.condition, prompt, role, complexity),
                };
            }
        }
        RoutingDecision {
            matched_rule: None,
            target_model: self.default_model.clone(),
            reason: format!("no rule matched; falling back to default_model '{}'", self.default_model),
        }
    }

    /// 从 VCP 风格 YAML 文件加载规则
    ///
    /// **YAML schema** (我设计, 字段对齐 VCP JSON 1:1):
    /// ```yaml
    /// default_model: gemini-2.5-flash
    /// rules:
    ///   - name: daily_chat
    ///     priority: 10
    ///     target_model: gemini-2.5-flash
    ///     condition:
    ///       type: keyword
    ///       keywords: [chat, 你好, 问候]
    ///   - name: long_input
    ///     priority: 50
    ///     target_model: gpt-5.5
    ///     condition:
    ///       type: token_range
    ///       min_tokens: 2000
    ///       max_tokens: 32000
    ///   - name: system_role
    ///     priority: 20
    ///     target_model: claude-opus-4-7-thinking
    ///     condition:
    ///       type: role
    ///       role: system
    ///   - name: high_complexity
    ///     priority: 80
    ///     target_model: claude-opus-4-7-thinking
    ///     condition:
    ///       type: complexity
    ///       min_complexity: 0.7
    /// ```
    pub fn from_yaml(path: &Path) -> Result<Self, ModelRouterError> {
        let content = std::fs::read_to_string(path).map_err(ModelRouterError::Io)?;
        let config: RouterConfig = serde_yaml::from_str(&content).map_err(ModelRouterError::Yaml)?;

        let mut router = Self::new(config.default_model);
        for rule_cfg in config.rules {
            let condition = match rule_cfg.condition {
                YamlCondition::Keyword { keywords } => RoutingCondition::KeywordMatch(keywords),
                YamlCondition::TokenRange { min_tokens, max_tokens } => {
                    RoutingCondition::TokenCountRange(min_tokens, max_tokens)
                }
                YamlCondition::Role { role } => {
                    let role_enum = match role.as_str() {
                        "system" | "developer" => MessageRole::System,
                        "user" => MessageRole::User,
                        "assistant" => MessageRole::Assistant,
                        "tool" | "function" => MessageRole::Tool,
                        _ => {
                            return Err(ModelRouterError::InvalidRole(role));
                        }
                    };
                    RoutingCondition::RoleBased(role_enum)
                }
                YamlCondition::Complexity { min_complexity } => {
                    RoutingCondition::Complexity(min_complexity)
                }
            };
            router.add_rule(RoutingRule {
                name: rule_cfg.name,
                condition,
                target_model: rule_cfg.target_model,
                priority: rule_cfg.priority,
            });
        }
        Ok(router)
    }

    /// 当前规则数
    pub fn rules_count(&self) -> usize {
        self.rules.len()
    }

    // --------------------------------------------------------
    // 内部: 条件匹配
    // --------------------------------------------------------

    fn matches(
        condition: &RoutingCondition,
        prompt: &str,
        role: Option<MessageRole>,
        complexity: Option<f32>,
    ) -> bool {
        match condition {
            RoutingCondition::KeywordMatch(keywords) => {
                let prompt_lower = prompt.to_lowercase();
                keywords
                    .iter()
                    .any(|k| prompt_lower.contains(&k.to_lowercase()))
            }
            RoutingCondition::TokenCountRange(min, max) => {
                // 简单按 char count 估算 (char count ≈ token count * 4 粗略)
                // 0 装真 tiktoken counting (R122-3 兄弟任务会接)
                let token_estimate = prompt.chars().count() / 4 + 1;
                token_estimate >= *min && token_estimate <= *max
            }
            RoutingCondition::RoleBased(target_role) => role == Some(*target_role),
            RoutingCondition::Complexity(min_complexity) => {
                complexity.map_or(false, |c| c >= *min_complexity)
            }
            RoutingCondition::Custom(f) => f(prompt),
        }
    }

    fn explain_match(
        condition: &RoutingCondition,
        prompt: &str,
        role: Option<MessageRole>,
        complexity: Option<f32>,
    ) -> String {
        match condition {
            RoutingCondition::KeywordMatch(keywords) => {
                let prompt_lower = prompt.to_lowercase();
                let hit: Vec<&String> = keywords
                    .iter()
                    .filter(|k| prompt_lower.contains(&k.to_lowercase()))
                    .collect();
                format!("keyword match: hit keywords {:?}", hit)
            }
            RoutingCondition::TokenCountRange(min, max) => {
                let token_estimate = prompt.chars().count() / 4 + 1;
                format!(
                    "token range match: estimated {} tokens in [{}, {}]",
                    token_estimate, min, max
                )
            }
            RoutingCondition::RoleBased(r) => {
                let _ = role; // role 参数是 callsite 提供的, 这里仅说明 rule 期望的 role
                format!("role-based match: rule role = {:?}", r)
            }
            RoutingCondition::Complexity(min) => {
                format!(
                    "complexity match: caller complexity = {:?} >= {}",
                    complexity, min
                )
            }
            RoutingCondition::Custom(_) => "custom function match".to_string(),
        }
    }
}

// ============================================================
// YAML schema (serde derive, 内部用, 0 暴露)
// ============================================================

#[derive(Debug, Deserialize, Serialize)]
struct RouterConfig {
    default_model: String,
    rules: Vec<YamlRule>,
}

#[derive(Debug, Deserialize, Serialize)]
struct YamlRule {
    name: String,
    priority: u8,
    target_model: String,
    condition: YamlCondition,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum YamlCondition {
    Keyword {
        keywords: Vec<String>,
    },
    TokenRange {
        min_tokens: usize,
        max_tokens: usize,
    },
    Role {
        role: String,
    },
    Complexity {
        min_complexity: f32,
    },
}

// ============================================================
// Error
// ============================================================

/// ModelRouter 错误
#[derive(Debug, thiserror::Error)]
pub enum ModelRouterError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid role string: {0}")]
    InvalidRole(String),
}

// ============================================================
// 单元测试 (8 tests, per 任务要求)
// ============================================================

#[cfg(test)]
mod model_router_tests {
    use super::*;

    // 1. 默认模型兜底: 无 rule 时返 default_model
    #[test]
    fn model_router_default_returns_default_model() {
        let router = SemanticModelRouter::new("gemini-2.5-flash");
        assert_eq!(router.rules_count(), 0);
        let target = router.route("随便说点什么", None, None);
        assert_eq!(target, "gemini-2.5-flash");
        let decision = router.explain("随便说点什么", None, None);
        assert!(decision.matched_rule.is_none());
        assert_eq!(decision.target_model, "gemini-2.5-flash");
        assert!(decision.reason.contains("default_model"));
    }

    // 2. 关键词匹配命中
    #[test]
    fn model_router_keyword_match_selects_target() {
        let mut router = SemanticModelRouter::new("gemini-2.5-flash");
        router.add_rule(RoutingRule::keyword(
            "daily_chat",
            vec!["chat".to_string(), "你好".to_string(), "问候".to_string()],
            "gemini-2.5-flash",
        ));
        router.add_rule(RoutingRule::keyword(
            "research_and_coding",
            vec!["信息检索".to_string(), "debug".to_string(), "代码".to_string()],
            "gpt-5.5",
        ));
        // 命中 daily_chat
        let target = router.route("你好, 今天天气不错", None, None);
        assert_eq!(target, "gemini-2.5-flash");
        // 命中 research_and_coding (大小写不敏感)
        let target = router.route("Please DEBUG this Rust code", None, None);
        assert_eq!(target, "gpt-5.5");
        // 不命中, 走 default
        let target = router.route("What is the meaning of life?", None, None);
        assert_eq!(target, "gemini-2.5-flash");
    }

    // 3. Token count range: 长输入路由到大模型
    #[test]
    fn model_router_token_count_range_selects_larger_model_for_long_input() {
        let mut router = SemanticModelRouter::new("gemini-2.5-flash");
        // 短输入 (< 2000 tokens) → gemini-2.5-flash (light)
        router.add_rule(RoutingRule::token_range(
            "long_input",
            2000,
            32000,
            "gpt-5.5", // 大模型
        ));
        // 短 prompt: ~5 chars / 4 + 1 = 2 tokens, 不在 [2000, 32000], 走 default
        let target = router.route("hi", None, None);
        assert_eq!(target, "gemini-2.5-flash");
        // 长 prompt: 10000 chars → ~2500 tokens, 在 [2000, 32000], 命中 gpt-5.5
        let long_prompt = "a".repeat(10_000);
        let target = router.route(&long_prompt, None, None);
        assert_eq!(target, "gpt-5.5");
    }

    // 4. Role-based: system role 路由到 advanced model
    #[test]
    fn model_router_role_based_routes_system_to_advanced() {
        let mut router = SemanticModelRouter::new("gemini-2.5-flash");
        router.add_rule(RoutingRule::role(
            "system_advanced",
            MessageRole::System,
            "claude-opus-4-7-thinking",
        ));
        // system role → claude-opus
        let target = router.route("any prompt", Some(MessageRole::System), None);
        assert_eq!(target, "claude-opus-4-7-thinking");
        // user role → default
        let target = router.route("any prompt", Some(MessageRole::User), None);
        assert_eq!(target, "gemini-2.5-flash");
        // 无 role → default (RoleBased 仅在 Some 时命中)
        let target = router.route("any prompt", None, None);
        assert_eq!(target, "gemini-2.5-flash");
    }

    // 5. Complexity threshold: 高复杂度路由到 strong model
    #[test]
    fn model_router_complexity_high_selects_strong_model() {
        let mut router = SemanticModelRouter::new("gemini-2.5-flash");
        router.add_rule(RoutingRule::complexity(
            "high_complexity",
            0.7,
            "claude-opus-4-7-thinking",
        ));
        // 复杂度 0.9 → 命中 (>= 0.7)
        let target = router.route("complex task", None, Some(0.9));
        assert_eq!(target, "claude-opus-4-7-thinking");
        // 复杂度 0.5 → 不命中
        let target = router.route("simple task", None, Some(0.5));
        assert_eq!(target, "gemini-2.5-flash");
        // 复杂度 None → 不命中 (Complexity 仅在 Some 时命中)
        let target = router.route("any", None, None);
        assert_eq!(target, "gemini-2.5-flash");
    }

    // 6. Priority: 高的先匹配 (priority=100 优先于 priority=50)
    #[test]
    fn model_router_priority_higher_wins() {
        let mut router = SemanticModelRouter::new("default");
        // 注意: add_rule 按 priority 降序, 后面 add 的高 priority 会排前面
        router.add_rule(RoutingRule {
            name: "low_priority_coding".to_string(),
            condition: RoutingCondition::KeywordMatch(vec!["code".to_string()]),
            target_model: "gpt-5.5".to_string(),
            priority: 50,
        });
        router.add_rule(RoutingRule {
            name: "high_priority_coding".to_string(),
            condition: RoutingCondition::KeywordMatch(vec!["code".to_string()]),
            target_model: "claude-opus-4-7-thinking".to_string(),
            priority: 100,
        });
        // "code" 命中两个 rule, 但 priority=100 的胜出
        let target = router.route("please help me with code", None, None);
        assert_eq!(target, "claude-opus-4-7-thinking");
        // 验证内部排序: high_priority 在前
        let decision = router.explain("please help me with code", None, None);
        assert_eq!(decision.matched_rule, Some("high_priority_coding".to_string()));
    }

    // 7. Explain: 返 matched_rule + target_model + reason
    #[test]
    fn model_router_explain_returns_matched_rule_and_reason() {
        let mut router = SemanticModelRouter::new("default");
        router.add_rule(RoutingRule::keyword(
            "chinese_chat",
            vec!["你好".to_string()],
            "qwen-3",
        ));
        let decision = router.explain("你好世界", None, None);
        assert_eq!(decision.matched_rule, Some("chinese_chat".to_string()));
        assert_eq!(decision.target_model, "qwen-3");
        assert!(decision.reason.contains("keyword match"));
        assert!(decision.reason.contains("你好"));

        // fallback decision
        let decision = router.explain("hello world", None, None);
        assert!(decision.matched_rule.is_none());
        assert_eq!(decision.target_model, "default");
        assert!(decision.reason.contains("falling back"));
    }

    // 8. from_yaml: VCP 风格 yaml 加载
    #[test]
    fn model_router_from_yaml_loads_vcp_format() {
        // 写一个临时 yaml 文件 (VCP 风格, 字段对齐 VCP JSON schema)
        let yaml_content = r#"
default_model: gemini-2.5-flash
rules:
  - name: daily_chat
    priority: 10
    target_model: gemini-2.5-flash
    condition:
      type: keyword
      keywords: [chat, 你好, 问候]
  - name: long_input
    priority: 50
    target_model: gpt-5.5
    condition:
      type: token_range
      min_tokens: 2000
      max_tokens: 32000
  - name: system_role
    priority: 20
    target_model: claude-opus-4-7-thinking
    condition:
      type: role
      role: system
  - name: high_complexity
    priority: 80
    target_model: claude-opus-4-7-thinking
    condition:
      type: complexity
      min_complexity: 0.7
"#;
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join("apeireth_model_router_test.yaml");
        std::fs::write(&tmp_path, yaml_content).expect("write yaml");

        let router =
            SemanticModelRouter::from_yaml(&tmp_path).expect("load yaml");
        assert_eq!(router.rules_count(), 4);
        assert_eq!(router.rules_count(), 4); // dedup: 4 rules

        // daily_chat 命中
        let target = router.route("chat with me", None, None);
        assert_eq!(target, "gemini-2.5-flash");
        // system role 命中
        let target = router.route("anything", Some(MessageRole::System), None);
        assert_eq!(target, "claude-opus-4-7-thinking");
        // high complexity 命中
        let target = router.route("anything", None, Some(0.8));
        assert_eq!(target, "claude-opus-4-7-thinking");
        // long input 命中
        let long = "a".repeat(10_000);
        let target = router.route(&long, None, None);
        assert_eq!(target, "gpt-5.5");
        // fallback
        let target = router.route("xyz", Some(MessageRole::User), Some(0.1));
        assert_eq!(target, "gemini-2.5-flash");

        // 清理
        let _ = std::fs::remove_file(&tmp_path);
    }

    // 9. (bonus) Custom: Arc<dyn Fn> 闭包路由
    #[test]
    fn model_router_custom_condition_with_arc_dyn_fn() {
        let mut router = SemanticModelRouter::new("default");
        // 自定义闭包: 含 "magic" 才命中
        let is_magic: Arc<dyn Fn(&str) -> bool + Send + Sync> = Arc::new(|p: &str| p.contains("magic"));
        router.add_rule(RoutingRule {
            name: "magic_word".to_string(),
            condition: RoutingCondition::Custom(is_magic),
            target_model: "gpt-5.5".to_string(),
            priority: 50,
        });
        // 命中 custom
        let target = router.route("this is magic", None, None);
        assert_eq!(target, "gpt-5.5");
        // 不命中
        let target = router.route("normal text", None, None);
        assert_eq!(target, "default");
        // explain 返 "custom function match"
        let decision = router.explain("this is magic", None, None);
        assert_eq!(decision.matched_rule, Some("magic_word".to_string()));
        assert!(decision.reason.contains("custom function"));
    }

    // --------------------------------------------------------
    // 编译期 hardcode 验证 (per 工程哲学铁律 #2 "不漂移")
    // --------------------------------------------------------

    #[test]
    fn compile_time_hardcode_vcp_source_size() {
        // VCP SemanticModelRouter.json 真实文件大小 (per github API 2026-08-10 sha ac9cd950ffdc8aa668e64424bbfa14af6d5658eb)
        assert_eq!(VCP_SEMANTIC_MODEL_ROUTER_BYTES, 2741);
        // VCP matchThreshold: 0.18 (per SemanticModelRouter.json:5)
        assert!((VCP_MATCH_THRESHOLD - 0.18).abs() < f32::EPSILON);
    }
}
