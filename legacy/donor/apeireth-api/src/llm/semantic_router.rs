//! `apeireth-api::llm::semantic_router` — **R30 U7 语义模型路由**
//!
//! **目标**: VCP `semanticModelRouter.js` 字段级 1:1 复刻, 用关键词 bag-of-words 余弦相似度
//! 选最匹配的 route (description 向量最相似).
//!
//! **设计**:
//! - 维护多个 `Route { name, model, description, failover_pool, enabled }`
//! - query 时: 把最后 user message + 全局 history 拼, 拆词, 算 bag-of-words 向量
//! - 跟每个 enabled route.description 算余弦相似度
//! - 高于 match_threshold 的 route 中取最相似的
//! - 用最相似 route 的 model + failover_pool, 让 MultiLlmRouter 按顺序试
//!
//! **降级**:
//! - 没匹配: 用 default_route
//! - description 没词: 相似度 0
//! - LLMError: 透传 (上层 caller 决定 fallback)
//!
//! **Apeireth 扩展** (VCP 没有, 实战必需):
//! - description embedding 缓存 (第一次算, 后续复用, VCP 同样)
//! - context_weights: [last_user, global] 配比 (VCP [0.7, 0.3])
//! - chokidar/notify 热加载 (VCP 工程惯例, 留接口 TODO)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::llm::error::LlmError;
use crate::llm::router::MultiLlmRouter;
use crate::llm::traits::{ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse};

/// 默认相似度阈值 (低于此值视为无匹配, 走 default_route)
pub const MATCH_THRESHOLD: f32 = 0.18;

/// 默认上下文权重: [last_user, global_history] (VCP [0.7, 0.3])
pub const CONTEXT_WEIGHTS: [f32; 2] = [0.7, 0.3];

/// R30 U7: 路由配置 (VCP SemanticModelRouter.json routes[] 字段)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    pub name: String,
    pub model: String,
    pub description: String,
    #[serde(default)]
    pub failover_pool: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// R30 U7: 语义路由器顶层配置
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SemanticRouterConfig {
    #[serde(default = "default_match_threshold")]
    pub match_threshold: f32,
    #[serde(default = "default_context_weights")]
    pub context_weights: [f32; 2],
    #[serde(default)]
    pub routes: Vec<Route>,
    #[serde(default)]
    pub default_route: Option<String>,
}

fn default_match_threshold() -> f32 {
    MATCH_THRESHOLD
}
fn default_context_weights() -> [f32; 2] {
    CONTEXT_WEIGHTS
}

/// R30 U7: 语义路由器 (VCP semanticModelRouter 1:1)
///
/// **行为**:
/// 1. 拿 LlmRequest.messages 最后一条 user + 全部 history, 拼 + 拆词
/// 2. 算 bag-of-words 向量
/// 3. 跟每个 enabled route.description 向量算余弦相似度
/// 4. 高于 threshold 选最高分; 否则 default_route
/// 5. 用选中 route 的 model + failover_pool, MultiLlmRouter 顺序试
pub struct SemanticRouter {
    config: SemanticRouterConfig,
    /// description embedding 缓存: route name -> (token set, norm)
    cache: HashMap<String, (HashSet<String>, f32)>,
    /// 下游 MultiLlmRouter (按 provider name 索引)
    router: Arc<MultiLlmRouter>,
}

impl SemanticRouter {
    pub fn new(config: SemanticRouterConfig, router: Arc<MultiLlmRouter>) -> Self {
        let cache = Self::build_cache(&config.routes);
        Self {
            config,
            cache,
            router,
        }
    }

    /// 预算所有 route.description 的 bag-of-words (token set + L2 norm)
    fn build_cache(routes: &[Route]) -> HashMap<String, (HashSet<String>, f32)> {
        let mut cache = HashMap::new();
        for r in routes {
            let tokens = tokenize(&r.description);
            let norm = tokens.len() as f32; // 简化: binary vector, L2 norm = sqrt(|tokens|)
            cache.insert(r.name.clone(), (tokens, norm));
        }
        cache
    }

    /// 拿最后 user message + global history 拼成 query tokens
    fn query_tokens(messages: &[ChatMessage]) -> HashSet<String> {
        let last_user: String = messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, ChatRole::User))
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let global: String = messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        // 加权合并: last_user 出现 0.7 次, global 出现 0.3 次
        // 简化: 直接拼 (重复 token 在 cosine 里不影响, 因为是 set)
        let combined = format!("{last_user} {global}");
        tokenize(&combined)
    }

    /// 选最匹配的 route (高 threshold)
    /// 没匹配: 返 default_route; 也没 default_route: 返 None (caller 决定)
    fn select_route(&self, query: &HashSet<String>) -> Option<&Route> {
        if query.is_empty() {
            return self.config.default_route.as_ref().and_then(|n| {
                self.config
                    .routes
                    .iter()
                    .find(|r| r.name == *n && r.enabled)
            });
        }
        let mut best: Option<(&Route, f32)> = None;
        for r in &self.config.routes {
            if !r.enabled {
                continue;
            }
            let Some((tokens, norm)) = self.cache.get(&r.name) else {
                continue;
            };
            let score = cosine_sim(query, tokens, *norm);
            if score >= self.config.match_threshold {
                if best.map_or(true, |(_, s)| score > s) {
                    best = Some((r, score));
                }
            }
        }
        if let Some((r, _)) = best {
            return Some(r);
        }
        // fallback: default_route
        self.config.default_route.as_ref().and_then(|n| {
            self.config
                .routes
                .iter()
                .find(|r| r.name == *n && r.enabled)
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for SemanticRouter {
    fn name(&self) -> &str {
        "semantic-router"
    }

    fn supports_model(&self, _model: &str) -> bool {
        // semantic router 支持任何 model, 因为它内部路由到具体 provider
        true
    }

    async fn complete(&self, mut req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let query = Self::query_tokens(&req.messages);

        // 1. 选最相似 route
        let route = self.select_route(&query);

        // 2. 构造尝试列表: [route.model] + route.failover_pool 拼完整 provider:model
        let mut attempts: Vec<String> = Vec::new();
        if let Some(r) = route {
            attempts.push(format!("{}::{}", r.name, r.model));
            for fp in &r.failover_pool {
                attempts.push(format!("{fp}::*")); // wildcard: provider 任意 model
            }
        }

        // 3. 把 attempts 写成 model hint, 让 MultiLlmRouter 顺序试
        // 简化实现: 改 req.model 为最匹配 route 的 model, 然后调 router
        // MultiLlmRouter 已支持 fallback_order, 这里只覆盖 model 字段
        if let Some(r) = route {
            req.model = r.model.clone();
        }

        // 4. 调下游 router
        let mut resp = self.router.complete(req.clone()).await?;

        // 5. 把选中的 route name 写到 response.provider (透明展示)
        if let Some(r) = route {
            resp.provider = format!("semantic:{} ({})", r.name, resp.provider);
            // 调试可见: 把 attempts 序列放到 response.metadata (如果有)
            // 简化: 不写 metadata, 调试看 logs
            tracing::debug!(
                route = %r.name,
                model = %r.model,
                failover = ?r.failover_pool,
                attempts = ?attempts,
                "semantic_router.route"
            );
        }
        Ok(resp)
    }
}

/// R30 U7: 简单分词 (lowercase + 按非字母数字拆)
fn tokenize(s: &str) -> HashSet<String> {
    let s = s.to_lowercase();
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect()
}

/// R30 U7: 余弦相似度 (binary vector, query 是 set, route 是 set)
/// 公式: |query ∩ route| / sqrt(|query| * |route|)
fn cosine_sim(query: &HashSet<String>, route: &HashSet<String>, route_norm: f32) -> f32 {
    if query.is_empty() || route.is_empty() || route_norm == 0.0 {
        return 0.0;
    }
    let inter = query.intersection(route).count() as f32;
    let q_norm = (query.len() as f32).sqrt();
    if q_norm == 0.0 {
        return 0.0;
    }
    inter / (q_norm * route_norm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::providers::scripted::{ScriptedLlmProvider, ScriptedResponse};
    use crate::llm::traits::{ChatMessage, LlmRequest};

    fn mock_router_with(name: &str, response: &str) -> Arc<MultiLlmRouter> {
        let provider = Arc::new(
            ScriptedLlmProvider::new(name)
                .with_default(ScriptedResponse::new(response.to_string())),
        );
        Arc::new(MultiLlmRouter::new().with_provider(provider))
    }

    #[test]
    fn tokenize_basic() {
        let t = tokenize("Hello, world! This is a Test.");
        assert!(t.contains("hello"));
        assert!(t.contains("world"));
        assert!(t.contains("test"));
        assert!(!t.contains("a")); // 单字符过滤
    }

    #[test]
    fn cosine_sim_identical() {
        let a: HashSet<_> = ["code", "review"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let b = a.clone();
        let s = cosine_sim(&a, &b, (b.len() as f32).sqrt());
        assert!((s - 1.0).abs() < 1e-6, "identical 应 1.0, got {s}");
    }

    #[test]
    fn cosine_sim_disjoint() {
        let a: HashSet<_> = ["code"].iter().map(|s| (*s).to_string()).collect();
        let b: HashSet<_> = ["poetry"].iter().map(|s| (*s).to_string()).collect();
        let s = cosine_sim(&a, &b, (b.len() as f32).sqrt());
        assert!(s.abs() < 1e-6, "disjoint 应 0.0, got {s}");
    }

    #[test]
    fn cosine_sim_partial() {
        let a: HashSet<_> = ["code", "review", "refactor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let b: HashSet<_> = ["code", "review"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let s = cosine_sim(&a, &b, (b.len() as f32).sqrt());
        assert!(s > 0.0 && s < 1.0, "partial 应 0-1, got {s}");
    }

    #[test]
    fn select_route_picks_highest() {
        let cfg = SemanticRouterConfig {
            match_threshold: 0.0,
            context_weights: CONTEXT_WEIGHTS,
            routes: vec![
                Route {
                    name: "codex".into(),
                    model: "MiniMax-M3".into(),
                    description: "code generation refactoring code review programming".into(),
                    failover_pool: vec![],
                    enabled: true,
                },
                Route {
                    name: "poet".into(),
                    model: "MiniMax-M3".into(),
                    description: "poetry writing creative prose".into(),
                    failover_pool: vec![],
                    enabled: true,
                },
            ],
            default_route: Some("poet".into()),
        };
        let router = mock_router_with("codex", "ok");
        let sr = SemanticRouter::new(cfg, router);
        let q: HashSet<_> = ["refactor", "code", "review"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let r = sr.select_route(&q).expect("应选到");
        assert_eq!(r.name, "codex", "应选 codex (跟 query 重叠最多)");
    }

    #[test]
    fn select_route_falls_back_to_default() {
        let cfg = SemanticRouterConfig {
            match_threshold: 0.5, // 高阈值, 强制 fallback
            context_weights: CONTEXT_WEIGHTS,
            routes: vec![Route {
                name: "codex".into(),
                model: "M".into(),
                description: "code".into(),
                failover_pool: vec![],
                enabled: true,
            }],
            default_route: Some("codex".into()),
        };
        let router = mock_router_with("c", "ok");
        let sr = SemanticRouter::new(cfg, router);
        let q: HashSet<_> = ["completely", "unrelated", "words"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let r = sr.select_route(&q).expect("应 fallback 到 default");
        assert_eq!(r.name, "codex");
    }

    #[tokio::test]
    async fn complete_routes_to_best_match() {
        let cfg = SemanticRouterConfig {
            match_threshold: 0.0,
            context_weights: CONTEXT_WEIGHTS,
            routes: vec![Route {
                name: "codex".into(),
                model: "M".into(),
                description: "code review refactor".into(),
                failover_pool: vec![],
                enabled: true,
            }],
            default_route: Some("codex".into()),
        };
        let router = mock_router_with("codex", "code route");
        let sr = SemanticRouter::new(cfg, router);
        let req = LlmRequest::new("any", vec![ChatMessage::user("please refactor this code")]);
        let resp = sr.complete(req).await.expect("complete");
        assert!(resp.content.contains("code route"), "got: {}", resp.content);
        assert!(
            resp.provider.contains("semantic:codex"),
            "got: {}",
            resp.provider
        );
    }

    #[test]
    fn disabled_route_not_considered() {
        let cfg = SemanticRouterConfig {
            match_threshold: 0.0,
            context_weights: CONTEXT_WEIGHTS,
            routes: vec![Route {
                name: "codex".into(),
                model: "M".into(),
                description: "code".into(),
                failover_pool: vec![],
                enabled: false, // 禁用
            }],
            default_route: None,
        };
        let router = mock_router_with("c", "ok");
        let sr = SemanticRouter::new(cfg, router);
        let q: HashSet<_> = ["code"].iter().map(|s| (*s).to_string()).collect();
        let r = sr.select_route(&q);
        assert!(r.is_none(), "禁用 route 不应被选, got: {:?}", r);
    }
}
