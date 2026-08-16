//! `apeireth-gateway::semantic_router` — 语义模型路由适配件 (N12 ①, VCP semanticModelRouter 吸收)
//!
//! **机制** (调研依据: research/source/vcptoolbox/modules/semanticModelRouter.js +
//! docs/SEMANTIC_MODEL_ROUTER.md; 任务锚点: team-work-doc §8.3/§8.4, backlog N12):
//!
//! 1. **虚拟模型名**: 客户端把 `model` 填成 `auto_model_name` (默认 `ApeirethModelAuto`)
//!    或某个 preset 名即进入路由; 其余模型名原样放行 (inactive, 0 改写)。
//! 2. **意图选模型**: 上下文向量 = 最后用户消息向量 × w0 + 最后 AI 消息向量 × w1
//!    (默认 `[0.7, 0.3]`), 与每条 route 的 description 向量做余弦相似度,
//!    按阈值 (默认 0.18) 过滤后降序排位。
//! 3. **容灾链**: 命中 route (受 `failover_pool` 约束) → `default_model` →
//!    `fallback_models`, 全链去重。`dispatch` 按链序逐个调 `ModelExecutor`,
//!    首个成功即返回; 全链失败 → `AllCandidatesFailed` (附全链尝试明细, 可行动)。
//!
//! **trait 口 (任务要求③, 实现注入不在本 crate)**:
//! - `Embedder` — 文本向量化注入点 (真实现 = MiniMax embedding 等, 留部署层)
//! - `ModelExecutor` — LLM 执行注入点 (真实现 = provider dispatch, 留部署层)
//!
//! **0 假装**:
//! - 未接 Gateway 帧管线: gateway.rs 只做 node/session/transport 准入, LLM 请求
//!   路径在上游 (apeireth-api / companion_serve), 本模块是可注入适配件,
//!   真接线留部署层 (验收标准为 mock 路由可测, 已满足)。
//! - 无配置热加载 (VCP 有 fs.watch; 本 crate 无文件 watch 基建, 不造轮子)。
//! - 描述向量缓存仅进程内 (VCP 另有 SQLite 持久缓存, 未移植)。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 默认虚拟模型名 (VCP: VCPModelAuto → 本项目命名空间)。
pub const DEFAULT_AUTO_MODEL_NAME: &str = "ApeirethModelAuto";
/// 默认预设名。
pub const DEFAULT_PRESET_NAME: &str = "default";
/// 全局默认相似度阈值 (VCP 同值 0.18)。
pub const DEFAULT_MATCH_THRESHOLD: f32 = 0.18;
/// 默认上下文权重 [user, assistant] (VCP 同值)。
pub const DEFAULT_CONTEXT_WEIGHTS: [f32; 2] = [0.7, 0.3];

/// 路由/派发错误 (诚实失败: 每个变体带可行动信息)。
#[derive(Debug, Error)]
pub enum RouterError {
    /// 配置 JSON 解析失败。
    #[error("semantic-router: config parse failed: {0}")]
    Config(String),
    /// 向量化失败 (resolve 内部已降级为默认计划, 此错误仅在调用方直接要求向量时出现)。
    #[error("semantic-router: embedding failed: {0}")]
    Embed(String),
    /// 容灾链全部候选失败 (附全链尝试明细)。
    #[error("semantic-router: all candidates failed, chain=[{chain}]: {detail}")]
    AllCandidatesFailed {
        /// 去重后的候选链 (逗号拼接, 用于错误展示)。
        chain: String,
        /// 每个候选的失败原因明细。
        detail: String,
    },
}

/// 路由决策原因 (审计留痕, 对照 VCP reason 枚举 + Rust 侧细分)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteReason {
    /// 路由器总开关关闭。
    RouterDisabled,
    /// 请求模型不是虚拟模型名/预设名 → 原样放行。
    NotRoutingModel,
    /// 预设名解析成功但预设不存在 (normalize 后理论不可达, 防御保留)。
    PresetNotFound,
    /// 语义命中 (≥ 阈值)。
    SemanticMatch,
    /// 相似度全部低于阈值 → 走默认计划。
    BelowThresholdDefault,
    /// 用户/AI 消息都为空, 无法构造上下文向量 → 默认计划。
    ContextUnavailable,
    /// 向量化失败 → 默认计划。
    EmbeddingUnavailable,
}

impl RouteReason {
    /// 机器可读字符串 (日志/审计用)。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RouterDisabled => "router_disabled",
            Self::NotRoutingModel => "not_routing_model",
            Self::PresetNotFound => "preset_not_found",
            Self::SemanticMatch => "semantic_match",
            Self::BelowThresholdDefault => "below_threshold_default",
            Self::ContextUnavailable => "context_unavailable",
            Self::EmbeddingUnavailable => "embedding_unavailable",
        }
    }
}

/// 路由项 (serde 版; `failover_pool`/`enabled` 缺省为 true, 对齐 VCP)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSpec {
    /// 路由名 (缺省回退 model, 再缺省 "unnamed")。
    #[serde(default)]
    pub name: String,
    /// 真实模型 ID (空 = 该路由在 normalize 时丢弃)。
    #[serde(default)]
    pub model: String,
    /// 语义描述 (空 = 该路由在 normalize 时丢弃)。
    #[serde(default)]
    pub description: String,
    /// 是否参与容灾池 (默认 true; false = 仅语义命中时用, 不进链)。
    #[serde(default = "default_true", rename = "failoverPool")]
    pub failover_pool: bool,
    /// 是否启用 (默认 true; false = normalize 时丢弃)。
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// normalize 后的有效路由项。
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// 路由名。
    pub name: String,
    /// 真实模型 ID。
    pub model: String,
    /// 语义描述 (向量化对象)。
    pub description: String,
    /// 是否参与容灾池。
    pub failover_pool: bool,
}

/// 预设 (serde 版)。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresetSpec {
    /// /v1/models 展示名。
    #[serde(default, rename = "displayName")]
    pub display_name: String,
    /// 兜底模型 (向量化失败/低于阈值时使用)。
    #[serde(default, rename = "defaultModel")]
    pub default_model: String,
    /// 容灾模型列表 (defaultModel 也失败时按序尝试)。
    #[serde(default, rename = "fallbackModels")]
    pub fallback_models: Vec<String>,
    /// 本预设阈值 (覆盖顶层)。
    #[serde(default, rename = "matchThreshold")]
    pub match_threshold: Option<f32>,
    /// 本预设上下文权重 (覆盖顶层)。
    #[serde(default, rename = "contextWeights")]
    pub context_weights: Option<Vec<f32>>,
    /// 候选路由列表。
    #[serde(default)]
    pub routes: Vec<RouteSpec>,
}

/// normalize 后的预设。
#[derive(Debug, Clone)]
pub struct Preset {
    /// 展示名。
    pub display_name: String,
    /// 兜底模型。
    pub default_model: String,
    /// 容灾模型 (已去重)。
    pub fallback_models: Vec<String>,
    /// 本预设阈值。
    pub match_threshold: f32,
    /// 本预设上下文权重。
    pub context_weights: Vec<f32>,
    /// 有效路由 (model/description 非空且 enabled)。
    pub routes: Vec<Route>,
}

/// 顶层配置 (serde 版, 字段命名对齐 VCP JSON 规范)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfigSpec {
    /// 总开关 (缺省 true)。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 默认预设对外的虚拟模型名。
    #[serde(default, rename = "autoModelName")]
    pub auto_model_name: String,
    /// 默认预设名。
    #[serde(default, rename = "defaultPreset")]
    pub default_preset: String,
    /// 全局相似度阈值。
    #[serde(default, rename = "matchThreshold")]
    pub match_threshold: Option<f32>,
    /// 全局上下文权重 [user, assistant]。
    #[serde(default, rename = "contextWeights")]
    pub context_weights: Option<Vec<f32>>,
    /// 预设字典 (键 = 预设名, 同时也是对外暴露的模型 ID)。
    #[serde(default)]
    pub presets: BTreeMap<String, PresetSpec>,
}

/// normalize 后的路由器配置。
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// 总开关。
    pub enabled: bool,
    /// 虚拟模型名。
    pub auto_model_name: String,
    /// 默认预设名。
    pub default_preset: String,
    /// 全局阈值。
    pub match_threshold: f32,
    /// 全局上下文权重。
    pub context_weights: Vec<f32>,
    /// 预设字典。
    pub presets: BTreeMap<String, Preset>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from_spec(RouterConfigSpec {
            enabled: true,
            auto_model_name: String::new(),
            default_preset: String::new(),
            match_threshold: None,
            context_weights: None,
            presets: BTreeMap::new(),
        })
    }
}

impl RouterConfig {
    /// 从 JSON 解析 (解析失败 → `RouterError::Config`)。
    pub fn from_json(json: &str) -> Result<Self, RouterError> {
        let spec: RouterConfigSpec =
            serde_json::from_str(json).map_err(|e| RouterError::Config(e.to_string()))?;
        Ok(Self::from_spec(spec))
    }

    /// normalize (对齐 VCP normalizeConfig 语义):
    /// - 空名回退默认值; 阈值/权重非法回退默认
    /// - 路由: model+description 非空且 enabled 才保留
    /// - fallbackModels 去重去空
    /// - defaultPreset 缺失 → 回退第一个预设; 无预设 → 建空 default 预设
    pub fn from_spec(spec: RouterConfigSpec) -> Self {
        let auto_model_name = nonempty_or(spec.auto_model_name, DEFAULT_AUTO_MODEL_NAME);
        let match_threshold = spec.match_threshold.unwrap_or(DEFAULT_MATCH_THRESHOLD);
        let context_weights = sanitize_weights(spec.context_weights);

        let mut presets = BTreeMap::new();
        for (name, p) in spec.presets {
            let name = name.trim().to_string();
            if name.is_empty() {
                continue;
            }
            let routes: Vec<Route> = p
                .routes
                .into_iter()
                .filter(|r| r.enabled)
                .map(|r| Route {
                    name: nonempty_or(r.name, &r.model),
                    model: r.model.trim().to_string(),
                    description: r.description.trim().to_string(),
                    failover_pool: r.failover_pool,
                })
                .filter(|r| !r.model.is_empty() && !r.description.is_empty())
                .collect();
            let default_model = p.default_model.trim().to_string();
            let display_name = nonempty_or(
                p.display_name,
                if name == spec.default_preset || spec.default_preset.is_empty() {
                    &auto_model_name
                } else {
                    &name
                },
            );
            presets.insert(
                name,
                Preset {
                    display_name,
                    default_model,
                    fallback_models: unique_nonempty(p.fallback_models),
                    match_threshold: p.match_threshold.unwrap_or(match_threshold),
                    context_weights: sanitize_weights(p.context_weights),
                    routes,
                },
            );
        }

        let mut default_preset = nonempty_or(spec.default_preset, DEFAULT_PRESET_NAME);
        if !presets.contains_key(&default_preset) {
            if let Some(first) = presets.keys().next().cloned() {
                default_preset = first;
            } else {
                presets.insert(
                    DEFAULT_PRESET_NAME.into(),
                    Preset {
                        display_name: auto_model_name.clone(),
                        default_model: String::new(),
                        fallback_models: Vec::new(),
                        match_threshold,
                        context_weights: context_weights.clone(),
                        routes: Vec::new(),
                    },
                );
                default_preset = DEFAULT_PRESET_NAME.into();
            }
        }

        Self {
            enabled: spec.enabled,
            auto_model_name,
            default_preset,
            match_threshold,
            context_weights,
            presets,
        }
    }

    /// 虚拟模型清单 (enabled=false → 空; 默认预设以 auto_model_name 对外, 其余预设为自身名)。
    pub fn virtual_models(&self) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }
        let mut out = Vec::new();
        out.push(self.auto_model_name.clone());
        for name in self.presets.keys() {
            if *name != self.default_preset {
                out.push(name.clone());
            }
        }
        out
    }
}

/// 文本向量化注入点 (任务要求③: 真实现留部署层, 本 crate 只定义契约 + mock 可测)。
#[async_trait]
pub trait Embedder: Send + Sync {
    /// 文本 → 向量。空文本/服务故障 → `Err` (调用方降级处理)。
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError>;
}

/// LLM 执行注入点 (任务要求③: 真实现 = provider dispatch, 留部署层)。
#[async_trait]
pub trait ModelExecutor: Send + Sync {
    /// 对指定真实模型执行一次请求。失败 → `Err` (触发容灾链下一候选)。
    async fn execute(
        &self,
        model: &str,
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

/// 单次语义匹配的排位结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredRoute {
    /// 路由名。
    pub name: String,
    /// 真实模型 ID。
    pub model: String,
    /// 是否参与容灾池。
    pub failover_pool: bool,
    /// 余弦相似度。
    pub similarity: f32,
}

/// 路由决策 (resolve 产物, 完整审计信息)。
#[derive(Debug, Clone)]
pub struct RouteDecision {
    /// 路由是否生效 (false = 请求模型原样放行)。
    pub active: bool,
    /// 客户端请求的模型名。
    pub requested_model: String,
    /// 命中的预设名 (未生效为 None)。
    pub preset_name: Option<String>,
    /// 本次选定的真实模型。
    pub selected_model: String,
    /// 容灾候选链 (去重; 重试时按序切换)。
    pub candidates: Vec<String>,
    /// 最佳语义匹配 (低于阈值/无上下文时为 None)。
    pub best_match: Option<ScoredRoute>,
    /// 全部排位结果 (降序, 审计用)。
    pub ranked_routes: Vec<ScoredRoute>,
    /// 决策原因。
    pub reason: RouteReason,
}

/// 语义模型路由器 (VCP semanticModelRouter 的 Rust 吸收版)。
pub struct SemanticModelRouter {
    config: RouterConfig,
    embedder: Arc<dyn Embedder>,
    /// 描述向量进程内缓存 (VCP descriptionVectorCache 对应物)。
    desc_cache: Mutex<HashMap<String, Vec<f32>>>,
}

impl SemanticModelRouter {
    /// 构造 (注入配置 + 嵌入实现)。
    pub fn new(config: RouterConfig, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            config,
            embedder,
            desc_cache: Mutex::new(HashMap::new()),
        }
    }

    /// 当前生效配置 (只读)。
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// 请求模型是否为虚拟模型/预设名。
    pub fn is_routing_model(&self, requested_model: &str) -> bool {
        self.resolve_preset_name(requested_model).is_some()
    }

    /// 解析预设名: auto_model_name → default_preset; 预设名自身 → 自身; 否则 None。
    pub fn resolve_preset_name(&self, requested_model: &str) -> Option<String> {
        let name = requested_model.trim();
        if name.is_empty() || !self.config.enabled {
            return None;
        }
        if name == self.config.auto_model_name {
            return Some(self.config.default_preset.clone());
        }
        if self.config.presets.contains_key(name) {
            return Some(name.to_string());
        }
        None
    }

    /// 路由决策 (纯决策, 不执行 LLM 调用)。
    ///
    /// `user_text`/`assistant_text` = 上下文中最后一条用户/AI 消息文本
    /// (调用方负责提取; 两者皆空 → ContextUnavailable 降级默认计划)。
    pub async fn resolve(
        &self,
        requested_model: &str,
        user_text: &str,
        assistant_text: &str,
    ) -> RouteDecision {
        let requested_model = requested_model.trim().to_string();

        if !self.config.enabled {
            return self.inactive(requested_model, RouteReason::RouterDisabled);
        }
        let preset_name = match self.resolve_preset_name(&requested_model) {
            Some(p) => p,
            None => return self.inactive(requested_model, RouteReason::NotRoutingModel),
        };
        let preset = match self.config.presets.get(&preset_name) {
            Some(p) => p,
            None => return self.inactive(requested_model, RouteReason::PresetNotFound),
        };

        let user_text = user_text.trim();
        let assistant_text = assistant_text.trim();
        if user_text.is_empty() && assistant_text.is_empty() {
            return self.default_plan(
                &requested_model,
                &preset_name,
                preset,
                RouteReason::ContextUnavailable,
            );
        }

        let context = match self
            .build_context_vector(user_text, assistant_text, &preset.context_weights)
            .await
        {
            Ok(v) => v,
            Err(_) => {
                return self.default_plan(
                    &requested_model,
                    &preset_name,
                    preset,
                    RouteReason::EmbeddingUnavailable,
                )
            }
        };

        // 排位: 每条 route 的 description 向量与上下文向量余弦相似度。
        let mut ranked: Vec<ScoredRoute> = Vec::new();
        for route in &preset.routes {
            let desc_vec = match self.description_vector(&route.description).await {
                Ok(v) => v,
                Err(_) => continue, // 单条路由向量化失败不影响其余路由 (诚实降级)
            };
            ranked.push(ScoredRoute {
                name: route.name.clone(),
                model: route.model.clone(),
                failover_pool: route.failover_pool,
                similarity: cosine_similarity(&context, &desc_vec),
            });
        }
        ranked.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));

        let threshold = preset.match_threshold;
        let matched: Vec<ScoredRoute> = ranked
            .iter()
            .filter(|r| r.similarity >= threshold)
            .cloned()
            .collect();

        let candidates = build_fallback_plan(preset, &matched);
        let selected = candidates
            .first()
            .cloned()
            .unwrap_or_else(|| requested_model.clone());
        let reason = if matched.is_empty() {
            RouteReason::BelowThresholdDefault
        } else {
            RouteReason::SemanticMatch
        };

        RouteDecision {
            active: true,
            requested_model,
            preset_name: Some(preset_name),
            selected_model: selected.clone(),
            candidates: if candidates.is_empty() {
                vec![selected]
            } else {
                candidates
            },
            best_match: matched.first().cloned(),
            ranked_routes: ranked,
            reason,
        }
    }

    /// 按容灾链派发: resolve → 逐候选调 executor → 首个成功即返回 (决策 + 结果)。
    /// 全链失败 → `RouterError::AllCandidatesFailed` (附全链尝试明细)。
    pub async fn dispatch(
        &self,
        requested_model: &str,
        user_text: &str,
        assistant_text: &str,
        request: &serde_json::Value,
        executor: &dyn ModelExecutor,
    ) -> Result<(RouteDecision, serde_json::Value), RouterError> {
        let decision = self
            .resolve(requested_model, user_text, assistant_text)
            .await;
        let mut failures: Vec<String> = Vec::new();
        for candidate in &decision.candidates {
            match executor.execute(candidate, request).await {
                Ok(out) => return Ok((decision, out)),
                Err(e) => failures.push(format!("{candidate}: {e}")),
            }
        }
        Err(RouterError::AllCandidatesFailed {
            chain: decision.candidates.join(", "),
            detail: failures.join(" | "),
        })
    }

    fn inactive(&self, requested_model: String, reason: RouteReason) -> RouteDecision {
        RouteDecision {
            active: false,
            requested_model: requested_model.clone(),
            preset_name: None,
            selected_model: requested_model.clone(),
            candidates: vec![requested_model],
            best_match: None,
            ranked_routes: Vec::new(),
            reason,
        }
    }

    fn default_plan(
        &self,
        requested_model: &str,
        preset_name: &str,
        preset: &Preset,
        reason: RouteReason,
    ) -> RouteDecision {
        let candidates = build_fallback_plan(preset, &[]);
        let selected = candidates
            .first()
            .cloned()
            .unwrap_or_else(|| requested_model.to_string());
        RouteDecision {
            active: true,
            requested_model: requested_model.to_string(),
            preset_name: Some(preset_name.to_string()),
            selected_model: selected.clone(),
            candidates: if candidates.is_empty() {
                vec![selected]
            } else {
                candidates
            },
            best_match: None,
            ranked_routes: Vec::new(),
            reason,
        }
    }

    async fn build_context_vector(
        &self,
        user_text: &str,
        assistant_text: &str,
        weights: &[f32],
    ) -> Result<Vec<f32>, RouterError> {
        let user_vec = if user_text.is_empty() {
            None
        } else {
            Some(self.embedder.embed(user_text).await?)
        };
        let ai_vec = if assistant_text.is_empty() {
            None
        } else {
            Some(self.embedder.embed(assistant_text).await?)
        };
        match (user_vec, ai_vec) {
            (Some(u), Some(a)) => Ok(weighted_average(&u, &a, weights)),
            (Some(u), None) => Ok(u),
            (None, Some(a)) => Ok(a),
            (None, None) => Err(RouterError::Embed("empty context".into())),
        }
    }

    async fn description_vector(&self, description: &str) -> Result<Vec<f32>, RouterError> {
        let key = description.trim().to_string();
        if let Some(v) = self.desc_cache.lock().get(&key) {
            return Ok(v.clone());
        }
        let v = self.embedder.embed(&key).await?;
        self.desc_cache.lock().insert(key.clone(), v.clone());
        Ok(v)
    }
}

/// 容灾链构造 (VCP buildFallbackPlan 1:1):
/// 命中首选 → (首选允许入池时) 其余命中按相似度序 → default_model → fallback_models, 去重。
pub fn build_fallback_plan(preset: &Preset, matched: &[ScoredRoute]) -> Vec<String> {
    let mut models: Vec<String> = Vec::new();
    if let Some(primary) = matched.first() {
        if !primary.model.is_empty() {
            models.push(primary.model.clone());
        }
        if primary.failover_pool {
            for route in matched.iter().skip(1) {
                if route.model.is_empty() || !route.failover_pool {
                    continue;
                }
                models.push(route.model.clone());
            }
        }
    }
    if !preset.default_model.is_empty() {
        models.push(preset.default_model.clone());
    }
    models.extend(preset.fallback_models.iter().cloned());
    unique_nonempty(models)
}

/// 余弦相似度 (零向量/维数 0 → 0.0)。
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn weighted_average(a: &[f32], b: &[f32], weights: &[f32]) -> Vec<f32> {
    let w0 = weights
        .first()
        .copied()
        .unwrap_or(DEFAULT_CONTEXT_WEIGHTS[0]);
    let w1 = weights
        .get(1)
        .copied()
        .unwrap_or(DEFAULT_CONTEXT_WEIGHTS[1]);
    let len = a.len().max(b.len());
    let mut out = vec![0.0f32; len];
    for (i, slot) in out.iter_mut().enumerate() {
        let va = a.get(i).copied().unwrap_or(0.0);
        let vb = b.get(i).copied().unwrap_or(0.0);
        *slot = va * w0 + vb * w1;
    }
    out
}

fn nonempty_or(value: String, fallback: &str) -> String {
    let t = value.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

fn sanitize_weights(weights: Option<Vec<f32>>) -> Vec<f32> {
    let clean: Vec<f32> = weights
        .unwrap_or_default()
        .into_iter()
        .filter(|w| w.is_finite() && *w >= 0.0)
        .collect();
    if clean.is_empty() {
        DEFAULT_CONTEXT_WEIGHTS.to_vec()
    } else {
        clean
    }
}

fn unique_nonempty(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    values
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && seen.insert(v.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- mock 注入 (验收: mock 路由可测) ----------

    /// 关键词正交向量 embedder: 文本含某关键词 → 该关键词的正交单位基向量。
    struct KeywordEmbedder;

    #[async_trait]
    impl Embedder for KeywordEmbedder {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
            let keywords = ["coding", "creative", "general"];
            for (i, kw) in keywords.iter().enumerate() {
                if text.contains(kw) {
                    let mut v = vec![0.0f32; keywords.len()];
                    v[i] = 1.0;
                    return Ok(v);
                }
            }
            // 未识别文本 → 全零向量 (余弦 0, 低于阈值)
            Ok(vec![0.0; keywords.len()])
        }
    }

    /// 总是失败的 embedder (降级路径测试)。
    struct FailingEmbedder;

    #[async_trait]
    impl Embedder for FailingEmbedder {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, RouterError> {
            Err(RouterError::Embed("mock embedder offline".into()))
        }
    }

    /// 脚本化 executor: 每模型按脚本逐个返回 (pop_front), 脚本空 → Err。
    struct ScriptExecutor {
        scripts: Mutex<HashMap<String, std::collections::VecDeque<Result<(), String>>>>,
    }

    impl ScriptExecutor {
        fn new(scripts: Vec<(&str, Vec<Result<(), String>>)>) -> Self {
            let mut map = HashMap::new();
            for (model, outcomes) in scripts {
                map.insert(model.to_string(), outcomes.into());
            }
            Self {
                scripts: Mutex::new(map),
            }
        }
    }

    #[async_trait]
    impl ModelExecutor for ScriptExecutor {
        async fn execute(
            &self,
            model: &str,
            _request: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            let next = self
                .scripts
                .lock()
                .get_mut(model)
                .and_then(|q| q.pop_front())
                .unwrap_or(Err(format!("no script for model {model}")));
            next.map(|_| serde_json::json!({ "model": model, "ok": true }))
        }
    }

    fn test_config() -> RouterConfig {
        let json = r#"{
            "enabled": true,
            "autoModelName": "AutoModel",
            "defaultPreset": "default",
            "matchThreshold": 0.18,
            "contextWeights": [0.7, 0.3],
            "presets": {
                "default": {
                    "defaultModel": "model-default",
                    "fallbackModels": ["model-fb1", "model-fb2"],
                    "routes": [
                        {"name": "coding", "model": "model-code", "description": "coding tasks"},
                        {"name": "creative", "model": "model-creative", "description": "creative writing"}
                    ]
                },
                "fast": {
                    "defaultModel": "model-fast-default",
                    "routes": [
                        {"name": "general", "model": "model-fast", "description": "general chat"}
                    ]
                }
            }
        }"#;
        RouterConfig::from_json(json).unwrap()
    }

    // ---------- 路由命中 ----------

    #[tokio::test]
    async fn resolve_semantic_hit_selects_matched_model() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let d = router
            .resolve("AutoModel", "please do some coding", "")
            .await;
        assert!(d.active);
        assert_eq!(d.selected_model, "model-code");
        assert_eq!(d.reason, RouteReason::SemanticMatch);
        assert_eq!(d.best_match.as_ref().unwrap().name, "coding");
        assert!(d.best_match.as_ref().unwrap().similarity >= 0.18);
    }

    #[tokio::test]
    async fn resolve_preset_name_directly_is_routing_model() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let d = router.resolve("fast", "general question", "").await;
        assert!(d.active);
        assert_eq!(d.selected_model, "model-fast");
        assert_eq!(d.preset_name.as_deref(), Some("fast"));
    }

    // ---------- 低于阈值 → default ----------

    #[tokio::test]
    async fn resolve_below_threshold_falls_back_to_default_model() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let d = router.resolve("AutoModel", "完全无关的文本", "").await;
        assert_eq!(d.reason, RouteReason::BelowThresholdDefault);
        assert_eq!(d.selected_model, "model-default");
        assert!(d.best_match.is_none());
    }

    // ---------- 容灾链 ----------

    #[tokio::test]
    async fn fallback_chain_order_matched_then_default_then_fallbacks() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let d = router.resolve("AutoModel", "coding please", "").await;
        assert_eq!(
            d.candidates,
            vec!["model-code", "model-default", "model-fb1", "model-fb2"]
        );
    }

    #[tokio::test]
    async fn fallback_chain_dedups() {
        let json = r#"{
            "presets": {
                "default": {
                    "defaultModel": "model-code",
                    "fallbackModels": ["model-code", "model-fb1", "model-fb1"],
                    "routes": [
                        {"name": "coding", "model": "model-code", "description": "coding"}
                    ]
                }
            }
        }"#;
        let router = SemanticModelRouter::new(
            RouterConfig::from_json(json).unwrap(),
            Arc::new(KeywordEmbedder),
        );
        let d = router.resolve("ApeirethModelAuto", "coding", "").await;
        assert_eq!(d.candidates, vec!["model-code", "model-fb1"]);
    }

    #[tokio::test]
    async fn failover_pool_false_excludes_secondary_matches_from_chain() {
        let json = r#"{
            "presets": {
                "default": {
                    "defaultModel": "model-default",
                    "routes": [
                        {"name": "coding", "model": "model-code", "description": "coding"},
                        {"name": "creative", "model": "model-creative", "description": "creative coding", "failoverPool": false}
                    ]
                }
            }
        }"#;
        let router = SemanticModelRouter::new(
            RouterConfig::from_json(json).unwrap(),
            Arc::new(KeywordEmbedder),
        );
        // "coding" 同时命中两条描述; creative 相似度与 coding 相同 (都含 coding)
        let d = router.resolve("ApeirethModelAuto", "coding", "").await;
        assert_eq!(d.reason, RouteReason::SemanticMatch);
        // failoverPool=false 的 route 不进链; 命中首选保留
        assert!(!d.candidates.contains(&"model-creative".to_string()));
        assert!(d.candidates.contains(&"model-code".to_string()));
        assert!(d.candidates.contains(&"model-default".to_string()));
    }

    // ---------- inactive 放行 ----------

    #[tokio::test]
    async fn non_virtual_model_passes_through() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let d = router.resolve("gpt-real-1", "coding", "").await;
        assert!(!d.active);
        assert_eq!(d.reason, RouteReason::NotRoutingModel);
        assert_eq!(d.selected_model, "gpt-real-1");
        assert_eq!(d.candidates, vec!["gpt-real-1"]);
    }

    #[tokio::test]
    async fn disabled_router_passes_through() {
        let json = r#"{"enabled": false, "presets": {"default": {"defaultModel": "m1"}}}"#;
        let router = SemanticModelRouter::new(
            RouterConfig::from_json(json).unwrap(),
            Arc::new(KeywordEmbedder),
        );
        let d = router.resolve("ApeirethModelAuto", "coding", "").await;
        assert!(!d.active);
        assert_eq!(d.reason, RouteReason::RouterDisabled);
        assert!(router.config().virtual_models().is_empty());
    }

    // ---------- 失败路径 (0 装 PASS: 降级且原因诚实) ----------

    #[tokio::test]
    async fn empty_context_degrades_to_default_plan() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let d = router.resolve("AutoModel", "", "   ").await;
        assert_eq!(d.reason, RouteReason::ContextUnavailable);
        assert_eq!(d.selected_model, "model-default");
    }

    #[tokio::test]
    async fn embedder_failure_degrades_to_default_plan() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(FailingEmbedder));
        let d = router.resolve("AutoModel", "coding", "").await;
        assert_eq!(d.reason, RouteReason::EmbeddingUnavailable);
        assert_eq!(d.selected_model, "model-default");
        assert_eq!(
            d.candidates,
            vec!["model-default", "model-fb1", "model-fb2"]
        );
    }

    // ---------- dispatch 容灾切换 ----------

    #[tokio::test]
    async fn dispatch_switches_to_next_candidate_on_failure() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let executor = ScriptExecutor::new(vec![
            ("model-code", vec![Err("upstream 500".into())]),
            ("model-default", vec![Ok(())]),
        ]);
        let (decision, out) = router
            .dispatch(
                "AutoModel",
                "coding",
                "",
                &serde_json::json!({"prompt": "x"}),
                &executor,
            )
            .await
            .unwrap();
        assert_eq!(decision.selected_model, "model-code"); // 决策仍为首选
        assert_eq!(out["model"], "model-default"); // 实际由第二候选完成
    }

    #[tokio::test]
    async fn dispatch_all_candidates_failed_reports_chain() {
        let router = SemanticModelRouter::new(test_config(), Arc::new(KeywordEmbedder));
        let executor = ScriptExecutor::new(vec![
            ("model-code", vec![Err("500".into())]),
            ("model-default", vec![Err("429".into())]),
            ("model-fb1", vec![Err("timeout".into())]),
            ("model-fb2", vec![Err("auth".into())]),
        ]);
        let err = router
            .dispatch("AutoModel", "coding", "", &serde_json::json!({}), &executor)
            .await
            .unwrap_err();
        match err {
            RouterError::AllCandidatesFailed { chain, detail } => {
                assert_eq!(chain, "model-code, model-default, model-fb1, model-fb2");
                assert!(detail.contains("500") && detail.contains("auth"));
            }
            other => panic!("expected AllCandidatesFailed, got {other:?}"),
        }
    }

    // ---------- 配置 normalize ----------

    #[test]
    fn normalize_drops_invalid_routes_and_dedups_fallbacks() {
        let json = r#"{
            "presets": {
                "default": {
                    "defaultModel": "dm",
                    "fallbackModels": ["fb", "fb", "", "fb2"],
                    "routes": [
                        {"name": "ok", "model": "m1", "description": "d1"},
                        {"model": "", "description": "no-model"},
                        {"model": "m2", "description": ""},
                        {"model": "m3", "description": "d3", "enabled": false}
                    ]
                }
            }
        }"#;
        let cfg = RouterConfig::from_json(json).unwrap();
        let preset = cfg.presets.get("default").unwrap();
        assert_eq!(preset.routes.len(), 1);
        assert_eq!(preset.routes[0].model, "m1");
        assert_eq!(preset.fallback_models, vec!["fb", "fb2"]);
        assert_eq!(cfg.auto_model_name, DEFAULT_AUTO_MODEL_NAME);
        assert_eq!(cfg.match_threshold, DEFAULT_MATCH_THRESHOLD);
    }

    #[test]
    fn normalize_missing_default_preset_falls_back() {
        let json = r#"{"defaultPreset": "ghost", "presets": {"only": {"defaultModel": "m"}}}"#;
        let cfg = RouterConfig::from_json(json).unwrap();
        assert_eq!(cfg.default_preset, "only");
    }

    #[test]
    fn normalize_empty_presets_creates_default_stub() {
        let cfg = RouterConfig::from_json("{}").unwrap();
        assert_eq!(cfg.default_preset, DEFAULT_PRESET_NAME);
        assert!(cfg.presets.contains_key(DEFAULT_PRESET_NAME));
        assert_eq!(cfg.presets[DEFAULT_PRESET_NAME].routes.len(), 0);
    }

    #[test]
    fn invalid_json_is_config_error() {
        let err = RouterConfig::from_json("{nope").unwrap_err();
        assert!(matches!(err, RouterError::Config(_)));
        assert!(err.to_string().contains("config parse failed"));
    }

    #[test]
    fn virtual_models_lists_auto_plus_non_default_presets() {
        let cfg = test_config();
        let models = cfg.virtual_models();
        assert!(models.contains(&"AutoModel".to_string()));
        assert!(models.contains(&"fast".to_string()));
        assert!(!models.contains(&"default".to_string())); // 默认预设以 AutoModel 对外
    }

    // ---------- 几何函数 ----------

    #[test]
    fn cosine_similarity_orthogonal_and_zero_vectors() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]), 1.0);
        assert_eq!(cosine_similarity(&[], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn weighted_average_applies_weights() {
        let out = weighted_average(&[1.0, 0.0], &[0.0, 1.0], &[0.7, 0.3]);
        assert!((out[0] - 0.7).abs() < 1e-6);
        assert!((out[1] - 0.3).abs() < 1e-6);
    }
}
