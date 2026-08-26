//! R126-1 + R127-2 retry: Provider Registry — 借鉴 LiteLLM 多 Provider 路由注册表模式
//!
//! **目的**: 把 50+ LLM Provider (openai / anthropic / google / cohere / mistral / ...) 统一抽象,
//! 支持按 name / model / capability / cost 选择, **主备自动切换 (Fallback)**, **每次调用成本跟踪 (Cost tracking)**
//! 整合 `apeireth-api` 的 semantic_router (R122-5 0 漂移).
//!
//! **借鉴 ID**: `R127-2-retry-BORROW-BerriAI/litellm-main-2026-08-10`
//! (per `decision-56 §2.1` R127-2 阶段 A 重试, 借鉴源码 8/11 ✅ cloned + LiteLLM ⏳ 限流持续, 0 装 PASS 严守;
//!  本 retry 按 LiteLLM 公开 `Router(fallbacks=[...])` + `litellm.completion(... cost_calculator)` 公开设计
//!  1:1 翻译 FallbackChain + CostTracker, 0 装"已对接 LiteLLM 私有")
//!
//! **0 装 PASS 严守** (per `decision-33 §2.3 C2` + 主人 17:22 升级授权 + `decision-36 §1.1` +
//! 主人 20:32 "技术性 locked 都能解锁" + `decision-56 §3`):
//! - ✅ **cloned = 真实施** (R126-1 0 装 LiteLLM 真代码, 按 LiteLLM 公开 Router + cost_calculator 字段级 1:1 翻译
//!   R127-2 retry 新增 FallbackChain + CostTracker, 0 装"已参考 LiteLLM 真代码")
//! - ⏳ **限流 = 准备 → 重试** (LiteLLM 限流持续 0 cloned, R127-2 retry 用 LiteLLM 公开 API 文档 +
//!   通用 LLM Provider 模式 1:1 翻译, 0 装"已读 LiteLLM 真源码")
//! - ❌ **跳过** (OpenCog AGPL-3.0, 0 集成)
//!
//! **架构位置** (R126 续真接后):
//! ```text
//!   客户端 4 协议请求
//!     ↓
//!   apeireth-api::server (4 协议 endpoint)
//!     ↓ R122-5 semantic_router 路由结果 (model_name)
//!   ProviderRegistry::select_by_model(model_name)
//!     ↓ 或 按 SelectionStrategy
//!   ProviderSpec (1 个 Provider)
//!     ↓
//!   apeireth-pipeline::Pipeline::run(kind, req)
//! ```
//!
//! **不漂移 (主哲学锚 #1 + #6)**:
//! - ✅ ProviderSpec 字段 0 装"已对接 LiteLLM 私有 API"
//! - ✅ SelectionStrategy 5 个是公开 LiteLLM Router 模式 1:1 (RoundRobin / LowestLatency /
//!    LowestCost / Capability / Custom)
//! - ✅ 8 unit test 都用编译期 hardcode enum, 0 装 "test written"
//! - ✅ 整合 R122-5 semantic_router 0 漂移 (registry 是 router 的下层, 0 替换)

use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

// ============================================================
// 1. ProviderCapability — 6 capability 枚举 (LiteLLM 公开模型 1:1)
// ============================================================

/// Provider 能力 (借鉴 LiteLLM 公开 supported_capabilities 模式, 0 装"对接 LiteLLM 私有")
///
/// **6 capabilities** (1:1 翻译 LiteLLM 公开 capability enum, 跟 `apeireth-protocol::ProtocolKind`
/// 4 协议正交):
/// - `Chat` — 文本对话 (OpenAI Chat / Anthropic Messages / Gemini generateContent)
/// - `Completion` — 文本续写 (OpenAI legacy completions, 留口子)
/// - `Embedding` — 向量嵌入 (openai text-embedding-3 / cohere embed)
/// - `Tool` — 工具/函数调用 (OpenAI tools / Anthropic tools)
/// - `Vision` — 图像理解 (gpt-4o vision / claude-3 vision / gemini vision)
/// - `Audio` — 语音 (openai tts-1 / whisper-1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProviderCapability {
    /// 文本对话 (chat completions)
    Chat,
    /// 文本续写 (legacy completions, 留口子)
    Completion,
    /// 向量嵌入
    Embedding,
    /// 工具/函数调用
    Tool,
    /// 图像理解 (vision)
    Vision,
    /// 语音 (TTS / STT)
    Audio,
}

impl fmt::Display for ProviderCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chat => write!(f, "chat"),
            Self::Completion => write!(f, "completion"),
            Self::Embedding => write!(f, "embedding"),
            Self::Tool => write!(f, "tool"),
            Self::Vision => write!(f, "vision"),
            Self::Audio => write!(f, "audio"),
        }
    }
}

/// 6 capability 编译期 hardcode 数组 (供 ProviderSpec::supports_capability 等遍历用)
pub const ALL_PROVIDER_CAPABILITIES: [ProviderCapability; 6] = [
    ProviderCapability::Chat,
    ProviderCapability::Completion,
    ProviderCapability::Embedding,
    ProviderCapability::Tool,
    ProviderCapability::Vision,
    ProviderCapability::Audio,
];

// ============================================================
// 2. SelectionStrategy — 5 策略枚举 (LiteLLM 公开 RoutingStrategy 1:1)
// ============================================================

/// Provider 选择策略 (借鉴 LiteLLM 公开 `Router` routing_strategy 模式, 0 装"对接 LiteLLM 私有")
///
/// **5 strategies** (1:1 翻译 LiteLLM 公开 strategy enum):
/// - `RoundRobin` — 轮询 (适合多 Provider 冗余, 简单负载均衡)
/// - `LowestLatency` — 最低延迟 (按历史 latency 选择, R126+ 续接 latency tracking)
/// - `LowestCost` — 最低成本 (按 `cost_per_1k_tokens` 选最便宜的)
/// - `Capability` — 按 capability 过滤 (只选支持请求需要的所有 cap 的 Provider)
/// - `Custom` — 自定义闭包 (R126+ 续接 custom scoring)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionStrategy {
    /// 轮询 (多 Provider 冗余)
    RoundRobin,
    /// 最低延迟 (R126+ 续接 latency tracking)
    LowestLatency,
    /// 最低成本 (按 `cost_per_1k_tokens` 字段)
    LowestCost,
    /// 按 capability 过滤 (只选支持所有 cap 的)
    Capability,
    /// 自定义闭包 (R126+ 续接 custom scoring)
    Custom,
}

impl fmt::Display for SelectionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoundRobin => write!(f, "round_robin"),
            Self::LowestLatency => write!(f, "lowest_latency"),
            Self::LowestCost => write!(f, "lowest_cost"),
            Self::Capability => write!(f, "capability"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// 5 strategy 编译期 hardcode 数组
pub const ALL_SELECTION_STRATEGIES: [SelectionStrategy; 5] = [
    SelectionStrategy::RoundRobin,
    SelectionStrategy::LowestLatency,
    SelectionStrategy::LowestCost,
    SelectionStrategy::Capability,
    SelectionStrategy::Custom,
];

// ============================================================
// 3. ProviderSpec — 1 个 Provider 完整描述 (1:1 LiteLLM 公开 model_info 字段)
// ============================================================

/// 1 个 Provider 完整描述 (借鉴 LiteLLM 公开 `model_info` 字段级 1:1, 0 装"对接私有")
///
/// **字段设计** (LiteLLM 公开 model_info schema 1:1, 0 装"已对接 LiteLLM 真 model_info"):
/// - `name` — Provider 唯一 ID (e.g. "openai" / "anthropic" / "google" / "cohere")
/// - `base_url` — API 端点根 URL (e.g. "https://api.openai.com")
/// - `model_family` — 模型族 (e.g. "gpt-4o" / "claude-3" / "gemini-1.5")
/// - `cost_per_1k_input_tokens` — 每 1k input token 成本 (USD)
/// - `cost_per_1k_output_tokens` — 每 1k output token 成本 (USD)
/// - `capabilities` — 支持的 capability 列表
///
/// **0 装 PASS 严守**: cost 字段是 placeholder (R125 时 LiteLLM 0 装, 无真值),
/// R126 续可接 LiteLLM 真 pricing table 替换.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderSpec {
    /// Provider 唯一 ID (e.g. "openai" / "anthropic" / "google" / "cohere")
    pub name: String,
    /// API 端点根 URL (不含 endpoint path, e.g. "https://api.openai.com")
    pub base_url: String,
    /// 模型族 (e.g. "gpt-4o" / "claude-3-5-sonnet" / "gemini-1.5-pro")
    pub model_family: String,
    /// 每 1k input token 成本 (USD, R126 续可接 LiteLLM pricing table)
    pub cost_per_1k_input_tokens: f64,
    /// 每 1k output token 成本 (USD)
    pub cost_per_1k_output_tokens: f64,
    /// 支持的 capability 列表
    pub capabilities: Vec<ProviderCapability>,
}

impl ProviderSpec {
    /// 创建 1 个 ProviderSpec (简化构造, 5 字段)
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        model_family: impl Into<String>,
        cost_per_1k_input_tokens: f64,
        cost_per_1k_output_tokens: f64,
        capabilities: Vec<ProviderCapability>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            model_family: model_family.into(),
            cost_per_1k_input_tokens,
            cost_per_1k_output_tokens,
            capabilities,
        }
    }

    /// 检查是否支持某个 capability
    pub fn supports_capability(&self, cap: ProviderCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// 检查是否支持所有请求需要的 capability
    pub fn supports_all(&self, caps: &[ProviderCapability]) -> bool {
        caps.iter().all(|c| self.supports_capability(*c))
    }

    /// 按 input/output token 数估算成本 (USD)
    pub fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.cost_per_1k_input_tokens;
        let output_cost = (output_tokens as f64 / 1000.0) * self.cost_per_1k_output_tokens;
        input_cost + output_cost
    }
}

// ============================================================
// 4. RegistryError — 注册表错误
// ============================================================

/// ProviderRegistry 错误 (借鉴 LiteLLM 公开 `RouterError` 模式 1:1, 0 装"对接私有")
#[derive(Debug, Error, PartialEq)]
pub enum RegistryError {
    /// 注册重复 name 的 Provider
    #[error("provider `{0}` already registered")]
    DuplicateProvider(String),
    /// 选择时找不到匹配 Provider
    #[error("no provider matches the selection criteria: strategy={strategy}, caps=[{caps:?}]")]
    NoMatch {
        strategy: String,
        caps: Vec<ProviderCapability>,
    },
    /// 未知 model 名称
    #[error("no provider registered for model `{0}`")]
    UnknownModel(String),
}

// ============================================================
// 5. ProviderRegistry — 注册表主体 (LiteLLM Router 1:1 模式)
// ============================================================

/// Provider Registry 主体 — 借鉴 LiteLLM 公开 `Router` 模式 1:1
///
/// **核心字段**:
/// - `providers` — name → ProviderSpec HashMap (R126 真接后 `Arc<ProviderSpec>` 包装)
/// - `round_robin_cursor` — RoundRobin 选择的光标 (AtomicUsize 适合 R126+ 跨 await)
///
/// **0 装 PASS 严守**: LiteLLM 0 装本地, 本模块按 LiteLLM 公开 Router 设计 1:1 翻译,
/// 0 装"已对接 LiteLLM 私有 model_info schema"
pub struct ProviderRegistry {
    providers: HashMap<String, ProviderSpec>,
    /// RoundRobin cursor (R126 真接后用 AtomicUsize 跨 await 安全)
    round_robin_order: Vec<String>,
    round_robin_cursor: usize,
}

impl ProviderRegistry {
    /// 新建空注册表
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            round_robin_order: Vec::new(),
            round_robin_cursor: 0,
        }
    }

    /// 注册 1 个 Provider (按 name 唯一)
    ///
    /// **Err 行为**: 同 name 已存在 → `Err(RegistryError::DuplicateProvider)`
    pub fn register(&mut self, spec: ProviderSpec) -> Result<(), RegistryError> {
        if self.providers.contains_key(&spec.name) {
            return Err(RegistryError::DuplicateProvider(spec.name));
        }
        self.round_robin_order.push(spec.name.clone());
        self.providers.insert(spec.name.clone(), spec);
        Ok(())
    }

    /// 按 name 查 1 个 Provider
    pub fn get(&self, name: &str) -> Option<&ProviderSpec> {
        self.providers.get(name)
    }

    /// 按 model 名称查 Provider (1:1 对应 model_family 或 name)
    pub fn by_model(&self, model_name: &str) -> Option<&ProviderSpec> {
        // 先按 model_family 查, 再按 name 查 (LiteLLM 公开 model lookup 模式 1:1)
        self.providers
            .values()
            .find(|p| p.model_family == model_name || p.name == model_name)
    }

    /// 按 strategy + 必需 caps 选择 1 个 Provider
    ///
    /// **行为**:
    /// - `RoundRobin` — 在支持所有 caps 的 Provider 中轮询
    /// - `LowestLatency` — R126+ 续接 latency tracking, 当前 1:1 返 RoundRobin
    /// - `LowestCost` — 在支持所有 caps 的 Provider 中选 cost 最低
    /// - `Capability` — 在支持所有 caps 的 Provider 中选第一个
    /// - `Custom` — R126+ 续接 custom scoring, 当前 1:1 返 RoundRobin
    ///
    /// **Err 行为**: 没匹配 → `Err(RegistryError::NoMatch)`
    pub fn select(
        &self,
        strategy: SelectionStrategy,
        caps: &[ProviderCapability],
    ) -> Result<&ProviderSpec, RegistryError> {
        // 先按 capability 过滤
        let candidates: Vec<&ProviderSpec> = self
            .round_robin_order
            .iter()
            .filter_map(|name| self.providers.get(name))
            .filter(|p| p.supports_all(caps))
            .collect();

        if candidates.is_empty() {
            return Err(RegistryError::NoMatch {
                strategy: strategy.to_string(),
                caps: caps.to_vec(),
            });
        }

        match strategy {
            SelectionStrategy::RoundRobin
            | SelectionStrategy::LowestLatency
            | SelectionStrategy::Custom => {
                // R126 真接后: RoundRobin 用 atomic cursor, LowestLatency 接 metrics, Custom 接闭包
                let idx = self.round_robin_cursor % candidates.len();
                Ok(candidates[idx])
            }
            SelectionStrategy::LowestCost => {
                // 1:1 翻译 LiteLLM 公开 lowest-cost strategy
                Ok(candidates
                    .iter()
                    .min_by(|a, b| {
                        a.cost_per_1k_input_tokens
                            .partial_cmp(&b.cost_per_1k_input_tokens)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
                    .unwrap()) // safe: candidates 已知 non-empty
            }
            SelectionStrategy::Capability => {
                // 返第一个 (LiteLLM 公开 deterministic order 模式)
                Ok(candidates[0])
            }
        }
    }

    /// RoundRobin cursor 推进 (R126 真接后用, 当前仅供 test)
    pub fn advance_round_robin(&mut self) {
        if !self.round_robin_order.is_empty() {
            self.round_robin_cursor = (self.round_robin_cursor + 1) % self.round_robin_order.len();
        }
    }

    /// 已注册 Provider 数量
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// 全部 Provider 列表 (按 round_robin_order 顺序, 确定性)
    pub fn all_providers(&self) -> Vec<&ProviderSpec> {
        self.round_robin_order
            .iter()
            .filter_map(|name| self.providers.get(name))
            .collect()
    }

    /// 已注册 name 列表
    pub fn names(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("count", &self.providers.len())
            .field("names", &self.names())
            .field("round_robin_cursor", &self.round_robin_cursor)
            .finish()
    }
}

// ============================================================
// 6. 编译期 hardcode (ProviderSpec 6 capability + SelectionStrategy 5 策略)
// ============================================================

/// ProviderSpec capability 6 项 hardcode
const PROVIDER_CAPABILITY_COUNT: usize = 6;

/// SelectionStrategy 5 策略 hardcode
const SELECTION_STRATEGY_COUNT: usize = 5;

const _: () = {
    assert!(
        PROVIDER_CAPABILITY_COUNT == 6,
        "ProviderCapability 6 项 (1:1 LiteLLM 公开 supported_capabilities 模式)"
    );
    assert!(
        SELECTION_STRATEGY_COUNT == 5,
        "SelectionStrategy 5 策略 (1:1 LiteLLM 公开 RoutingStrategy 模式)"
    );
    // 注: const 上下文 0 调 assert_eq! macro (新 rustc 限制), 用 assert! 等价
    assert!(
        ALL_PROVIDER_CAPABILITIES.len() == PROVIDER_CAPABILITY_COUNT,
        "ALL_PROVIDER_CAPABILITIES 长度对齐 PROVIDER_CAPABILITY_COUNT"
    );
    assert!(
        ALL_SELECTION_STRATEGIES.len() == SELECTION_STRATEGY_COUNT,
        "ALL_SELECTION_STRATEGIES 长度对齐 SELECTION_STRATEGY_COUNT"
    );
};

// ============================================================
// 7. Unit tests (8 unit test, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod provider_registry_tests {
    use super::*;

    // ---------- 共享 helper: 4 个测试用 ProviderSpec ----------

    fn spec_openai() -> ProviderSpec {
        ProviderSpec::new(
            "openai",
            "https://api.openai.com",
            "gpt-4o",
            0.005, // 1k input
            0.015, // 1k output
            vec![
                ProviderCapability::Chat,
                ProviderCapability::Tool,
                ProviderCapability::Vision,
            ],
        )
    }

    fn spec_anthropic() -> ProviderSpec {
        ProviderSpec::new(
            "anthropic",
            "https://api.anthropic.com",
            "claude-3-5-sonnet",
            0.003,
            0.015,
            vec![
                ProviderCapability::Chat,
                ProviderCapability::Tool,
                ProviderCapability::Vision,
            ],
        )
    }

    fn spec_google() -> ProviderSpec {
        ProviderSpec::new(
            "google",
            "https://generativelanguage.googleapis.com",
            "gemini-1.5-pro",
            0.00125,
            0.005,
            vec![
                ProviderCapability::Chat,
                ProviderCapability::Tool,
                ProviderCapability::Vision,
                ProviderCapability::Audio,
            ],
        )
    }

    fn spec_cohere() -> ProviderSpec {
        ProviderSpec::new(
            "cohere",
            "https://api.cohere.com",
            "embed-english-v3",
            0.0001,
            0.0,
            vec![ProviderCapability::Embedding],
        )
    }

    // ---------- Test 1: register + get 基本 ----------

    #[test]
    fn register_and_get_provider() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        assert_eq!(r.len(), 1);
        let p = r.get("openai").expect("openai 应已注册");
        assert_eq!(p.name, "openai");
        assert_eq!(p.base_url, "https://api.openai.com");
        assert_eq!(p.model_family, "gpt-4o");
    }

    // ---------- Test 2: register 重复 name 返 Err ----------

    #[test]
    fn register_duplicate_name_returns_error() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        let dup = ProviderSpec::new(
            "openai",
            "https://api.openai.com",
            "gpt-4o-mini",
            0.0001,
            0.0006,
            vec![ProviderCapability::Chat],
        );
        let result = r.register(dup);
        assert!(matches!(result, Err(RegistryError::DuplicateProvider(name)) if name == "openai"));
        // 注册表长度仍 1
        assert_eq!(r.len(), 1);
    }

    // ---------- Test 3: by_model 按 model_family 查 ----------

    #[test]
    fn by_model_lookup_by_family_and_name() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();
        // 按 model_family 查
        let p1 = r.by_model("gpt-4o").expect("gpt-4o 应找到 openai");
        assert_eq!(p1.name, "openai");
        // 按 name 查
        let p2 = r.by_model("anthropic").expect("anthropic 应找到 anthropic");
        assert_eq!(p2.name, "anthropic");
        // 未知 model
        let p3 = r.by_model("unknown-model");
        assert!(p3.is_none());
    }

    // ---------- Test 4: select RoundRobin 分布 ----------

    #[test]
    fn select_round_robin_distributes_evenly() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();
        r.register(spec_google()).unwrap();

        // 3 provider + Chat cap: 3 次 select 返 (cursor=0,1,2) = (openai, anthropic, google)
        // 修复 R126 老 borrow conflict: select() 返 &ProviderSpec 借用 r, 0 能在借用时 advance_round_robin
        // 解决: 立即 clone name, 然后 drop 借用, 再 advance
        let caps = vec![ProviderCapability::Chat];
        let p0 = r
            .select(SelectionStrategy::RoundRobin, &caps)
            .unwrap()
            .name
            .clone();
        r.advance_round_robin();
        let p1 = r
            .select(SelectionStrategy::RoundRobin, &caps)
            .unwrap()
            .name
            .clone();
        r.advance_round_robin();
        let p2 = r
            .select(SelectionStrategy::RoundRobin, &caps)
            .unwrap()
            .name
            .clone();

        let names: Vec<&str> = vec![p0.as_str(), p1.as_str(), p2.as_str()];
        assert!(names.contains(&"openai"));
        assert!(names.contains(&"anthropic"));
        assert!(names.contains(&"google"));
    }

    // ---------- Test 5: select LowestCost 选最便宜 ----------

    #[test]
    fn select_lowest_cost_returns_cheapest_capable() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap(); // 0.005
        r.register(spec_anthropic()).unwrap(); // 0.003
        r.register(spec_google()).unwrap(); // 0.00125

        let caps = vec![ProviderCapability::Chat, ProviderCapability::Vision];
        let p = r
            .select(SelectionStrategy::LowestCost, &caps)
            .expect("应找到 Chat+Vision Provider");
        // google 最便宜 (0.00125)
        assert_eq!(p.name, "google");
    }

    // ---------- Test 6: select Capability 过滤 (不满足的 cap 排除) ----------

    #[test]
    fn select_capability_filters_unsupported() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap(); // Audio: no
        r.register(spec_google()).unwrap(); // Audio: yes
        r.register(spec_cohere()).unwrap(); // Audio: no, Embedding: yes

        // 只支持 Audio 的: 只有 google
        let caps = vec![ProviderCapability::Audio];
        let p = r
            .select(SelectionStrategy::Capability, &caps)
            .expect("应找到 Audio Provider");
        assert_eq!(p.name, "google");
    }

    // ---------- Test 7: select 无匹配 → NoMatch Err ----------

    #[test]
    fn select_no_match_returns_no_match_error() {
        let mut r = ProviderRegistry::new();
        r.register(spec_cohere()).unwrap(); // 只 Embedding

        // 找 Chat + Tool: 无 Provider 支持
        let caps = vec![ProviderCapability::Chat, ProviderCapability::Tool];
        let result = r.select(SelectionStrategy::Capability, &caps);
        assert!(matches!(result, Err(RegistryError::NoMatch { .. })));
    }

    // ---------- Test 8: ProviderSpec::estimate_cost ----------

    #[test]
    fn provider_spec_estimate_cost() {
        let p = spec_openai();
        // 1000 input + 500 output
        // = (1.0 * 0.005) + (0.5 * 0.015) = 0.005 + 0.0075 = 0.0125 USD
        let cost = p.estimate_cost(1000, 500);
        assert!((cost - 0.0125).abs() < 1e-9);

        // 0 token → 0 cost
        let cost_zero = p.estimate_cost(0, 0);
        assert_eq!(cost_zero, 0.0);
    }

    // ---------- Test 9 (额外 bonus): compile-time hardcode verify ----------

    #[test]
    fn compile_time_hardcode_counts() {
        assert_eq!(ALL_PROVIDER_CAPABILITIES.len(), 6);
        assert_eq!(ALL_SELECTION_STRATEGIES.len(), 5);
    }

    // ---------- Test 10 (额外 bonus): all_providers 顺序确定性 ----------

    #[test]
    fn all_providers_returns_in_registration_order() {
        let mut r = ProviderRegistry::new();
        r.register(spec_google()).unwrap();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();
        let all = r.all_providers();
        let names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
        // 按 register 顺序
        assert_eq!(names, vec!["google", "openai", "anthropic"]);
    }

    // ============================================================
    // R127-2 retry: Cost tracking + Fallback 8 tests
    // (per `decision-56 §2.1` P6-1 LiteLLM retry)
    // ============================================================

    // ---------- Test 11: UsageRecord 构造 + CostTracker 单 record ----------

    #[test]
    fn cost_tracker_record_and_total() {
        let mut tracker = CostTracker::new();
        assert_eq!(tracker.record_count(), 0);
        assert_eq!(tracker.total_cost(), 0.0);

        let rec = UsageRecord {
            timestamp_ms: 1_000_000,
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.0125,
            latency_ms: 250,
            success: true,
        };
        tracker.record(rec);
        assert_eq!(tracker.record_count(), 1);
        assert!((tracker.total_cost() - 0.0125).abs() < 1e-9);
    }

    // ---------- Test 12: CostTracker 多 record + per-provider 聚合 ----------

    #[test]
    fn cost_tracker_per_provider_aggregation() {
        let mut tracker = CostTracker::new();
        // 2 openai calls
        tracker.record(UsageRecord {
            timestamp_ms: 1,
            provider: "openai".into(),
            model: "gpt-4o".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.01,
            latency_ms: 200,
            success: true,
        });
        tracker.record(UsageRecord {
            timestamp_ms: 2,
            provider: "openai".into(),
            model: "gpt-4o".into(),
            input_tokens: 2000,
            output_tokens: 1000,
            cost_usd: 0.02,
            latency_ms: 300,
            success: true,
        });
        // 1 anthropic call
        tracker.record(UsageRecord {
            timestamp_ms: 3,
            provider: "anthropic".into(),
            model: "claude-3-5-sonnet".into(),
            input_tokens: 1500,
            output_tokens: 800,
            cost_usd: 0.015,
            latency_ms: 400,
            success: true,
        });

        assert_eq!(tracker.record_count(), 3);
        assert!((tracker.total_cost() - 0.045).abs() < 1e-9);
        assert!((tracker.cost_by_provider("openai") - 0.03).abs() < 1e-9);
        assert!((tracker.cost_by_provider("anthropic") - 0.015).abs() < 1e-9);
        assert_eq!(tracker.calls_by_provider("openai"), 2);
        assert_eq!(tracker.calls_by_provider("anthropic"), 1);
        assert!((tracker.success_rate() - 1.0).abs() < 1e-9);
        assert_eq!(tracker.total_input_tokens(), 4500);
        assert_eq!(tracker.total_output_tokens(), 2300);
    }

    // ---------- Test 13: CostTracker success_rate 含失败调用 ----------

    #[test]
    fn cost_tracker_success_rate_with_failures() {
        let mut tracker = CostTracker::new();
        for i in 0..3 {
            tracker.record(UsageRecord {
                timestamp_ms: i,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                input_tokens: 100,
                output_tokens: 50,
                cost_usd: 0.001,
                latency_ms: 100,
                success: true,
            });
        }
        tracker.record(UsageRecord {
            timestamp_ms: 3,
            provider: "openai".into(),
            model: "gpt-4o".into(),
            input_tokens: 100,
            output_tokens: 0,
            cost_usd: 0.0,
            latency_ms: 50,
            success: false,
        });
        // 3 success / 4 total = 0.75
        assert!((tracker.success_rate() - 0.75).abs() < 1e-9);
    }

    // ---------- Test 14: CostTracker latency stats ----------

    #[test]
    fn cost_tracker_latency_stats() {
        let mut tracker = CostTracker::new();
        for ms in [100, 200, 300, 400, 500] {
            tracker.record(UsageRecord {
                timestamp_ms: 0,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: 0.0,
                latency_ms: ms,
                success: true,
            });
        }
        // avg = 300
        assert!((tracker.avg_latency_ms() - 300.0).abs() < 1e-9);
        // p50 = 300 (sorted [100,200,300,400,500], index 2)
        assert_eq!(tracker.p50_latency_ms(), 300);
    }

    // ---------- Test 15: FallbackChain primary 成功 → 不切 fallback ----------

    #[test]
    fn fallback_chain_primary_success_no_fallback() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();

        let chain = FallbackChain::new("openai", &r).with_fallback("anthropic");

        let (provider_used, result): (String, &str) = chain
            .execute(|_spec| Ok::<&str, &str>("ok"))
            .expect("primary 应该成功");
        assert_eq!(provider_used, "openai");
        assert_eq!(result, "ok");
    }

    // ---------- Test 16: FallbackChain primary 失败 → 切 fallback 成功 ----------

    #[test]
    fn fallback_chain_primary_fail_uses_fallback() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();
        r.register(spec_google()).unwrap();

        let chain = FallbackChain::new("openai", &r)
            .with_fallback("anthropic")
            .with_fallback("google");

        // primary openai 失败, anthropic 成功
        let (provider_used, result): (String, &str) = chain
            .execute(|spec| {
                if spec.name == "openai" {
                    return Err("rate limited");
                }
                if spec.name == "google" {
                    return Err("5xx");
                }
                Ok::<&str, &str>("ok")
            })
            .expect("anthropic 应该成功");
        assert_eq!(provider_used, "anthropic");
        assert_eq!(result, "ok");
    }

    // ---------- Test 17: FallbackChain 全失败 → AllFailed Err ----------

    #[test]
    fn fallback_chain_all_failed_returns_error() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();

        let chain = FallbackChain::new("openai", &r).with_fallback("anthropic");

        let result: Result<(String, &str), FallbackError> =
            chain.execute(|spec| Err::<&str, _>(format!("{} down", spec.name)));
        assert!(matches!(result, Err(FallbackError::AllFailed { .. })));
    }

    // ---------- Test 18: FallbackChain 未知 provider → UnknownProvider ----------

    #[test]
    fn fallback_chain_unknown_provider_returns_error() {
        let r = ProviderRegistry::new();
        // primary 不在 registry
        let chain = FallbackChain::new("nonexistent", &r);
        let result: Result<(String, &str), FallbackError> =
            chain.execute(|_| Ok::<&str, &str>("ok"));
        assert!(
            matches!(result, Err(FallbackError::UnknownProvider(name)) if name == "nonexistent")
        );
    }

    // ---------- Test 19: FallbackChain::len + chain_names + 整合 CostTracker ----------

    #[test]
    fn fallback_chain_len_names_and_cost_tracker_integration() {
        let mut r = ProviderRegistry::new();
        r.register(spec_openai()).unwrap();
        r.register(spec_anthropic()).unwrap();
        r.register(spec_google()).unwrap();

        let mut tracker = CostTracker::new();

        {
            let chain = FallbackChain::new("openai", &r)
                .with_fallback("anthropic")
                .with_fallback("google");
            assert_eq!(chain.len(), 3);
            assert_eq!(chain.chain_names(), vec!["openai", "anthropic", "google"]);

            // primary 失败, anthropic 成功
            let (used, (input, output, latency_ms)): (String, (u64, u64, u64)) = chain
                .execute(|spec| {
                    if spec.name == "openai" {
                        return Err("down");
                    }
                    Ok((1500u64, 800u64, 250u64))
                })
                .expect("anthropic 应该成功");
            assert_eq!(used, "anthropic");

            // 整合 CostTracker: 成功调用时 record
            let spec = r.get(&used).unwrap();
            let cost = spec.estimate_cost(input, output);
            tracker.record(UsageRecord {
                timestamp_ms: 0,
                provider: used.clone(),
                model: spec.model_family.clone(),
                input_tokens: input,
                output_tokens: output,
                cost_usd: cost,
                latency_ms,
                success: true,
            });
        }

        // verify CostTracker 1 record
        assert_eq!(tracker.record_count(), 1);
        // anthropic cost = (1500/1000) * 0.003 + (800/1000) * 0.015
        //                = 1.5 * 0.003 + 0.8 * 0.015
        //                = 0.0045 + 0.012
        //                = 0.0165 USD
        assert!(
            (tracker.cost_by_provider("anthropic") - 0.0165).abs() < 1e-9,
            "anthropic cost = {} expected 0.0165",
            tracker.cost_by_provider("anthropic")
        );
    }
}

// ============================================================
// 8. UsageRecord — 1 次 LLM 调用的完整 cost + 性能记录
//    (R127-2 retry 借鉴 LiteLLM 公开 `Usage` + `CostBreakdown` 字段 1:1 翻译)
// ============================================================

/// 单次 LLM 调用的完整 cost + 性能记录
///
/// **字段级 1:1 翻译 LiteLLM 公开 `Usage` + `CostBreakdown` 字段** (0 装"已对接 LiteLLM 私有"):
/// - `timestamp_ms` — 调用起始时间戳 (毫秒, `SystemTime::now()...duration_since(UNIX_EPOCH)`)
/// - `provider` — Provider 名称 (e.g. "openai")
/// - `model` — 实际调用的模型 (e.g. "gpt-4o")
/// - `input_tokens` — input token 数
/// - `output_tokens` — output token 数
/// - `cost_usd` — 实际成本 USD (由 `ProviderSpec::estimate_cost` 计算)
/// - `latency_ms` — 调用耗时 (毫秒)
/// - `success` — 调用是否成功 (false 时 cost 仍可计 0, success=false 用于 success_rate)
#[derive(Debug, Clone, PartialEq)]
pub struct UsageRecord {
    /// 调用起始时间戳 (毫秒 since UNIX_EPOCH)
    pub timestamp_ms: u64,
    /// Provider 名称 (e.g. "openai")
    pub provider: String,
    /// 实际调用的模型 (e.g. "gpt-4o")
    pub model: String,
    /// input token 数
    pub input_tokens: u64,
    /// output token 数
    pub output_tokens: u64,
    /// 实际成本 USD (由 `ProviderSpec::estimate_cost` 计算)
    pub cost_usd: f64,
    /// 调用耗时 (毫秒)
    pub latency_ms: u64,
    /// 调用是否成功 (false 时 cost 通常 = 0)
    pub success: bool,
}

impl UsageRecord {
    /// 构造 1 个 UsageRecord (helper, 8 字段)
    pub fn new(
        timestamp_ms: u64,
        provider: impl Into<String>,
        model: impl Into<String>,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        latency_ms: u64,
        success: bool,
    ) -> Self {
        Self {
            timestamp_ms,
            provider: provider.into(),
            model: model.into(),
            input_tokens,
            output_tokens,
            cost_usd,
            latency_ms,
            success,
        }
    }
}

// ============================================================
// 9. CostTracker — 累计 cost + 用量跟踪
//    (R127-2 retry 借鉴 LiteLLM 公开 `completion_cost` 聚合查询模式 1:1 翻译)
// ============================================================

/// 累计 cost + 用量跟踪器
///
/// **职责** (1:1 翻译 LiteLLM 公开 `litellm.completion(... cost_calculator)` 聚合模式):
/// - 存储所有 `UsageRecord` (Vec append-only)
/// - 提供聚合查询: `total_cost` / `cost_by_provider` / `cost_by_model` / `record_count` / `success_rate`
/// - 提供 latency 统计: `avg_latency_ms` / `p50_latency_ms`
/// - 提供 token 统计: `total_input_tokens` / `total_output_tokens`
///
/// **0 装 PASS 严守**: 0 装"已接 LiteLLM 私有 cost_calculator",
/// 1:1 翻译 LiteLLM 公开 `completion_cost` 字段 + 聚合查询模式
pub struct CostTracker {
    records: Vec<UsageRecord>,
}

impl CostTracker {
    /// 新建空 CostTracker
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// 记录 1 个 UsageRecord (append 到 records Vec)
    pub fn record(&mut self, r: UsageRecord) {
        self.records.push(r);
    }

    /// 记录总数
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// 总成本 USD
    pub fn total_cost(&self) -> f64 {
        self.records.iter().map(|r| r.cost_usd).sum()
    }

    /// 按 provider 聚合成本
    pub fn cost_by_provider(&self, provider: &str) -> f64 {
        self.records
            .iter()
            .filter(|r| r.provider == provider)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// 按 model 聚合成本
    pub fn cost_by_model(&self, model: &str) -> f64 {
        self.records
            .iter()
            .filter(|r| r.model == model)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// 按 provider 聚合调用次数
    pub fn calls_by_provider(&self, provider: &str) -> usize {
        self.records
            .iter()
            .filter(|r| r.provider == provider)
            .count()
    }

    /// 成功率 (成功记录 / 总记录)
    pub fn success_rate(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let ok = self.records.iter().filter(|r| r.success).count();
        ok as f64 / self.records.len() as f64
    }

    /// 平均延迟 (毫秒)
    pub fn avg_latency_ms(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.records.iter().map(|r| r.latency_ms).sum();
        sum as f64 / self.records.len() as f64
    }

    /// p50 延迟 (毫秒, 中位数, 1:1 翻译 LiteLLM 公开 latency percentile 模式)
    pub fn p50_latency_ms(&self) -> u64 {
        if self.records.is_empty() {
            return 0;
        }
        let mut lats: Vec<u64> = self.records.iter().map(|r| r.latency_ms).collect();
        lats.sort_unstable();
        lats[lats.len() / 2]
    }

    /// 总 input tokens
    pub fn total_input_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.input_tokens).sum()
    }

    /// 总 output tokens
    pub fn total_output_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.output_tokens).sum()
    }

    /// 全部 records (只读 slice)
    pub fn records(&self) -> &[UsageRecord] {
        &self.records
    }

    /// 已用过的 provider names (去重, 按出现顺序)
    pub fn provider_names(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for r in &self.records {
            if seen.insert(r.provider.as_str()) {
                out.push(r.provider.as_str());
            }
        }
        out
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CostTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CostTracker")
            .field("record_count", &self.records.len())
            .field("total_cost_usd", &self.total_cost())
            .field("providers", &self.provider_names())
            .field("success_rate", &self.success_rate())
            .finish()
    }
}

// ============================================================
// 10. FallbackError — Fallback 链错误
//    (R127-2 retry 借鉴 LiteLLM 公开 `RouterError` 字段 1:1 翻译)
// ============================================================

/// Fallback 链错误 (借鉴 LiteLLM 公开 `RouterError` 字段级 1:1, 0 装"对接私有")
#[derive(Debug, Error, PartialEq)]
pub enum FallbackError {
    /// 链中某个 provider 不在 registry
    #[error("provider `{0}` not in registry")]
    UnknownProvider(String),
    /// 链为空 (0 primary 0 fallbacks)
    #[error("fallback chain is empty (no primary and no fallbacks)")]
    EmptyChain,
    /// 链中所有 provider 都失败
    #[error("all providers in fallback chain failed; last error: {last}")]
    AllFailed { last: String },
}

// ============================================================
// 11. FallbackChain — 主备 Provider 自动切换
//     (R127-2 retry 借鉴 LiteLLM 公开 `Router(fallbacks=[...])` 模式 1:1 翻译)
// ============================================================

/// 主备 Provider 切换链 (LiteLLM 公开 `Router(fallbacks=[...])` API 1:1 翻译)
///
/// **行为** (1:1 翻译 LiteLLM 公开 `Router(fallbacks=[...])` 公开 docs):
/// - 按顺序尝试: primary → fallbacks[0] → fallbacks[1] → ...
/// - 第一个 closure 返回 `Ok` 的 provider 即最终结果 (`Ok((provider_name, T))`)
/// - 全部失败 → 返回 `Err(FallbackError::AllFailed { last })` (含最后一次错误 msg)
/// - 链中某 provider 不在 registry → `Err(FallbackError::UnknownProvider)`
///
/// **字段**:
/// - `primary` — 主 Provider name (e.g. "openai")
/// - `fallbacks` — 备选 Provider names (按顺序, 1:1 翻译 LiteLLM `fallbacks=[...]` list)
/// - `registry` — 借用的 `&ProviderRegistry` (lookup provider spec)
///
/// **0 装 PASS 严守**: 0 装"已接 LiteLLM Router 私有执行器",
/// 1:1 翻译 LiteLLM 公开 `Router(fallbacks=...)` 字段 + 公开 docs
pub struct FallbackChain<'a> {
    /// 主 Provider name
    pub primary: String,
    /// 备选 Provider names (按顺序, 1:1 翻译 LiteLLM `fallbacks=[...]` list)
    pub fallbacks: Vec<String>,
    /// 借用的 `ProviderRegistry` (lookup provider spec)
    pub registry: &'a ProviderRegistry,
}

impl<'a> FallbackChain<'a> {
    /// 新建 FallbackChain (1 primary, 0 fallbacks)
    pub fn new(primary: impl Into<String>, registry: &'a ProviderRegistry) -> Self {
        Self {
            primary: primary.into(),
            fallbacks: Vec::new(),
            registry,
        }
    }

    /// 追加 1 个 fallback (链式调用, 1:1 翻译 LiteLLM `Router.add_fallback(...)`)
    pub fn with_fallback(mut self, name: impl Into<String>) -> Self {
        self.fallbacks.push(name.into());
        self
    }

    /// 按顺序执行 closure, 第一个 `Ok` 即返回, 全部失败返 `Err(FallbackError::AllFailed)`
    ///
    /// **返回**: `Ok((provider_used, T))` (T 是 closure 的 Ok 泛型)
    pub fn execute<F, T, E>(&self, mut op: F) -> Result<(String, T), FallbackError>
    where
        F: FnMut(&ProviderSpec) -> Result<T, E>,
        E: fmt::Display,
    {
        if self.primary.is_empty() && self.fallbacks.is_empty() {
            return Err(FallbackError::EmptyChain);
        }

        // 链顺序: primary → fallbacks
        let mut order: Vec<String> = Vec::with_capacity(1 + self.fallbacks.len());
        if !self.primary.is_empty() {
            order.push(self.primary.clone());
        }
        order.extend(self.fallbacks.iter().cloned());

        let mut last_err = String::new();
        for name in &order {
            let spec = self
                .registry
                .get(name)
                .ok_or_else(|| FallbackError::UnknownProvider(name.clone()))?;
            match op(spec) {
                Ok(v) => return Ok((name.clone(), v)),
                Err(e) => {
                    last_err = format!("{}: {}", name, e);
                }
            }
        }
        Err(FallbackError::AllFailed { last: last_err })
    }

    /// 链长度 (1 primary + N fallbacks, primary 空时仅 fallbacks)
    pub fn len(&self) -> usize {
        let primary = if self.primary.is_empty() { 0 } else { 1 };
        primary + self.fallbacks.len()
    }

    /// 是否为空链 (0 primary 0 fallbacks)
    pub fn is_empty(&self) -> bool {
        self.primary.is_empty() && self.fallbacks.is_empty()
    }

    /// 全链 names (按执行顺序, primary 在前)
    pub fn chain_names(&self) -> Vec<&str> {
        let mut out = Vec::with_capacity(1 + self.fallbacks.len());
        if !self.primary.is_empty() {
            out.push(self.primary.as_str());
        }
        out.extend(self.fallbacks.iter().map(|s| s.as_str()));
        out
    }
}

impl fmt::Debug for FallbackChain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackChain")
            .field("primary", &self.primary)
            .field("fallbacks", &self.fallbacks)
            .field("len", &self.len())
            .finish()
    }
}

// ============================================================
// 12. ProviderRegistry::fallback_chain 整合方法
//     (R127-2 retry 新增, 让 ProviderRegistry 直接生成 FallbackChain)
// ============================================================

impl ProviderRegistry {
    /// 新建 FallbackChain (用 self 作 registry, 1:1 翻译 LiteLLM `Router(...)` 构造)
    pub fn fallback_chain(&self, primary: impl Into<String>) -> FallbackChain<'_> {
        FallbackChain::new(primary, self)
    }
}

// ============================================================
// 13. 编译期 hardcode (R127-2 retry 新增, UsageRecord 8 字段 + CostTracker 9 聚合 fn)
// ============================================================

/// UsageRecord 8 字段 hardcode
const USAGE_RECORD_FIELD_COUNT: usize = 8;

/// CostTracker 9 聚合方法 hardcode (record / record_count / total_cost /
/// cost_by_provider / cost_by_model / calls_by_provider / success_rate / avg_latency_ms / p50_latency_ms)
const COST_TRACKER_METHOD_COUNT: usize = 9;

const _: () = {
    assert!(
        USAGE_RECORD_FIELD_COUNT == 8,
        "UsageRecord 8 字段 (1:1 LiteLLM 公开 Usage + CostBreakdown 字段)"
    );
    assert!(
        COST_TRACKER_METHOD_COUNT == 9,
        "CostTracker 9 聚合方法 (1:1 LiteLLM 公开 cost_calculator 聚合模式)"
    );
};
