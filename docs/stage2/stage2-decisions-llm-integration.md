# 阶段 2 决策：LLM 集成 (2026-07-30)

> **范围**: R14 Rust 重写 LLM 集成决策 (阶段 2 第七项)
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: 阶段 1 灵感 §1.2 (性能极致) + §6 (LLM 友好语义层) + 城堡底线 (本地推理 "管家") + Hermes 10 LLM providers 借鉴

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-llm-integration.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 7/12) |
| **决策** | **多 LLM + 路由 + 本地管家 + fallback + token 经济** |
| **候选 crate** | `apeireth-prompt` + `apeireth-asi` 增强 (阶段 2 §3 已列) |

---

## 1. 决策总览

```
5 大机制:
  1. LlmProvider trait 抽象 (统一接口, 8+ providers)
  2. LlmRouter (路由策略: Fixed/Cost/Latency/Capability/RoundRobin/Bandit)
  3. 本地管家 (apeireth-asi 内置 Ollama/llama.cpp)
  4. Fallback chain (主→次→本地→拒绝)
  5. Token 经济 (缓存/压缩/批处理/预算/降级)
```

---

## 2. LlmProvider trait 抽象

```rust
// apeireth-prompt/src/provider.rs

use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 同步 completion
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, LlmError>;
    
    /// 流式 completion
    async fn stream(&self, req: &CompletionRequest) -> Result<CompletionStream, LlmError>;
    
    /// Provider 类型
    fn provider_type(&self) -> ProviderType;
    
    /// 模型列表
    fn models(&self) -> Vec<ModelInfo>;
    
    /// 成本估算
    fn cost_estimate(&self, req: &CompletionRequest) -> Cost;
    
    /// 健康检查
    async fn health(&self) -> HealthStatus;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    DeepSeek,
    Mistral,
    Ollama,        // 本地
    LlamaCpp,      // 本地
    VLLM,          // 本地 (高性能)
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Vec<ToolSpec>,  // function calling
    pub stop: Vec<String>,
    pub metadata: HashMap<String, Value>,
}

pub struct CompletionResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub model: String,
}

pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}
```

### 2.1 8+ provider 实现

| Provider | 实现 | 备注 |
|----------|------|------|
| OpenAI | `OpenAiProvider` | GPT-4o / GPT-4-turbo / o1 |
| Anthropic | `AnthropicProvider` | Claude 3.5/4 Sonnet/Opus |
| Google | `GoogleProvider` | Gemini 2.0 |
| DeepSeek | `DeepSeekProvider` | DeepSeek-V3 |
| Mistral | `MistralProvider` | Mistral Large |
| Ollama | `OllamaProvider` | 本地 Qwen/Llama/Phi |
| LlamaCpp | `LlamaCppProvider` | 直接调 .so |
| VLLM | `VllmProvider` | 高吞吐本地 |

### 2.2 Cargo.toml

```toml
[dependencies]
async-openai = "0.21"     # OpenAI + 兼容 (DeepSeek/Mistral)
anthropic-sdk = "0.1"     # Anthropic
reqwest = { version = "0.12", features = ["json", "stream"] }  # 自定义
ollama-rs = "0.2"          # Ollama
```

---

## 3. LlmRouter (智能路由)

```rust
// apeireth-prompt/src/router.rs

pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    default_policy: RoutingPolicy,
    fallback_chain: Vec<String>,  // 主→次→本地
    metrics: Arc<RwLock<RouterMetrics>>,
}

#[derive(Debug, Clone)]
pub enum RoutingPolicy {
    Fixed(String),                  // 固定 provider (如 "openai")
    CostOptimized,                  // 选 cost 最低
    LatencyOptimized,               // 选 latency 最低
    CapabilityBased { needs: Vec<Capability> }, // 按能力 (vision/long-context/code)
    RoundRobin,                     // 轮询
    MultiArmedBandit,               // UCB1 强化学习 (Hermes 借鉴)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    Vision,
    LongContext,    // 1M+ tokens
    CodeGeneration,
    FunctionCalling,
    Json,
    Reasoning,      // o1 / extended thinking
    Multilingual,
    Local,           // 本地模型 (隐私/无网)
}

impl LlmRouter {
    pub async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let primary = self.pick_provider(req)?;
        
        match primary.complete(req).await {
            Ok(resp) => {
                self.metrics.write().await.record_success(&primary.provider_type());
                Ok(resp)
            }
            Err(e) => {
                self.metrics.write().await.record_failure(&primary.provider_type());
                
                // fallback chain
                for fallback in &self.fallback_chain {
                    if let Ok(provider) = self.providers.get(fallback).ok_or(()) {
                        if let Ok(resp) = provider.complete(req).await {
                            return Ok(resp);
                        }
                    }
                }
                Err(e)
            }
        }
    }
    
    fn pick_provider(&self, req: &CompletionRequest) -> Result<Arc<dyn LlmProvider>, LlmError> {
        match &self.default_policy {
            RoutingPolicy::Fixed(name) => self.providers.get(name).cloned()
                .ok_or_else(|| LlmError::ProviderNotFound(name.clone())),
            RoutingPolicy::CostOptimized => {
                // 选 cost 最低的可用 provider
                todo!()
            }
            RoutingPolicy::LatencyOptimized => {
                todo!()
            }
            RoutingPolicy::CapabilityBased { needs } => {
                // 选能力匹配的
                todo!()
            }
            RoutingPolicy::RoundRobin => {
                todo!()
            }
            RoutingPolicy::MultiArmedBandit => {
                // UCB1 算法
                todo!()
            }
        }
    }
}
```

### 3.1 路由策略决策表

| 场景 | 推荐策略 | 理由 |
|------|---------|------|
| 默认日常对话 | CapabilityBased | 自动匹配最佳 |
| 成本敏感 | CostOptimized | 省钱 |
| 延迟敏感 (实时对话) | LatencyOptimized | 快 |
| 高可用 (生产) | RoundRobin + fallback | 不挂 |
| 自我进化 | MultiArmedBandit | 学到最优 (Hermes 启发) |
| 隐私敏感 | Fixed("local") | 本地推理 |

---

## 4. 本地"城堡内置管家"

### 4.1 设计

`apeireth-asi` 内部集成本地推理：

```rust
// apeireth-asi/src/local_steward.rs

pub struct LocalSteward {
    backend: LocalBackend,    // Ollama / llama.cpp / vLLM
    model: String,            // 默认 "qwen2.5:7b"
    endpoint: String,         // "http://localhost:11434"
    capabilities: Vec<Capability>,
}

#[async_trait]
impl LlmProvider for LocalSteward { /* 本地推理 */ }

impl LocalSteward {
    /// 启动时检测本地推理是否可用
    pub async fn detect() -> Option<Self> {
        // 1. 探测 Ollama (11434)
        if let Ok(client) = OllamaClient::new("http://localhost:11434") {
            if client.health().await.is_ok() {
                return Some(LocalSteward {
                    backend: LocalBackend::Ollama,
                    model: "qwen2.5:7b".into(),
                    endpoint: "http://localhost:11434".into(),
                    capabilities: vec![Capability::CodeGeneration, Capability::Json],
                });
            }
        }
        
        // 2. 探测 llama.cpp (本地 .so)
        if let Ok(steward) = Self::detect_llamacpp().await {
            return Some(steward);
        }
        
        // 3. 探测 vLLM
        // ...
        
        None
    }
    
    /// 本地推理能力边界 (E-3 守门)
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
}
```

### 4.2 推荐模型

| 模型 | 用途 | 能力 | 大小 |
|------|------|------|------|
| **Qwen 2.5 7B** | 默认管家 | 代码 + 推理 + 中文 | 4-8GB |
| Llama 3.2 7B | 英文为主 | 通用 | 4-8GB |
| Phi-3 mini | 轻量管家 | 基础 | 2-4GB |
| DeepSeek-Coder 6.7B | 代码专精 | 代码 | 4-8GB |
| Gemma 2 9B | 推理专精 | 推理 | 6-10GB |

### 4.3 能力边界 (E-3 守门)

```
本地管家能力边界:
  ✅ 写作 (创意/技术)
  ✅ 推理 (基本逻辑)
  ✅ 分类 (文本分类)
  ✅ 代码生成 (单文件)
  ✅ 翻译 (中英)
  ✅ 摘要

  ❌ 长上下文 (>32k)
  ❌ 视觉 (本地小模型不支持)
  ❌ 高级推理 (o1 级别)
  ❌ 多步规划 (需要大模型)

→ 这些能力自动 fallback 到远程 provider
```

---

## 5. Fallback 机制

### 5.1 链式 fallback

```
主 provider (e.g. OpenAI GPT-4)
  ↓ timeout / rate limit / error
次 provider (e.g. Anthropic Claude)
  ↓ timeout / rate limit / error
本地 provider (Ollama + Qwen 7B)
  ↓ unavailable
优雅降级:
  - 简单任务用本地 (无需 fallback)
  - 复杂任务 fallback 到本地降级版本 (4-bit 量化)
  - 终极: 返回错误 (拒绝服务)
```

### 5.2 Circuit Breaker

```rust
// apeireth-prompt/src/circuit_breaker.rs

pub struct CircuitBreaker {
    failure_threshold: u32,       // 失败 N 次开路
    success_threshold: u32,      // 成功后 N 次闭路
    open_duration: Duration,      // 开路持续时间
    state: Arc<RwLock<BreakerState>>,
}

#[derive(Debug, Clone)]
pub enum BreakerState {
    Closed,        // 正常
    Open { until: Instant },  // 开路 (拒绝请求)
    HalfOpen,      // 半开 (试探)
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T, LlmError>
    where
        F: Future<Output = Result<T, LlmError>>,
    {
        // 1. 检查 state
        match *self.state.read().await {
            BreakerState::Open { until } if Instant::now() < until => {
                return Err(LlmError::CircuitOpen);
            }
            _ => {}
        }
        
        // 2. 调用
        match f.await {
            Ok(t) => {
                self.record_success().await;
                Ok(t)
            }
            Err(e) => {
                self.record_failure().await;
                Err(e)
            }
        }
    }
    
    async fn record_failure(&self) {
        let mut state = self.state.write().await;
        // 失败计数, 达到阈值就 Open
        // ...
    }
}
```

### 5.3 Retry with exponential backoff

```rust
pub async fn retry_with_backoff<F, Fut, T>(
    mut f: F,
    max_retries: u32,
) -> Result<T, LlmError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, LlmError>>,
{
    let mut attempt = 0;
    loop {
        match f().await {
            Ok(t) => return Ok(t),
            Err(e) if attempt < max_retries => {
                let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 6. Token 经济

### 6.1 5 个机制

| 机制 | 作用 | 实现位置 |
|------|------|---------|
| **缓存** | 语义相似 prompt 复用 | `apeireth-prompt/src/cache.rs` |
| **压缩** | context 压缩 (历史摘要) | `apeireth-prompt/src/compactor.rs` |
| **批处理** | 多请求合并 | `apeireth-prompt/src/batcher.rs` |
| **预算控制** | 每日/每月 token 限制 | `apeireth-prompt/src/budget.rs` |
| **模型降级** | token 不够换小模型 | 路由策略 |

### 6.2 语义缓存

```rust
// apeireth-prompt/src/cache.rs

pub struct SemanticCache {
    embeddings: Arc<dyn DataBackend>,  // 用 Qdrant 存 embeddings
    responses: Arc<dyn DataBackend>,   // 用 sled 存响应
    similarity_threshold: f32,         // 0.95 触发
}

impl SemanticCache {
    /// 查缓存 (语义相似)
    pub async fn get(&self, prompt: &str) -> Option<CompletionResponse> {
        let embedding = self.embed(prompt).await?;
        let similar = self.embeddings.query(&Query::Vector(embedding, 1)).await?;
        let (cached_prompt, cached_response) = similar.first()?;
        
        // 计算相似度
        let sim = cosine_similarity(embedding, cached_prompt);
        if sim >= self.similarity_threshold {
            return Some(cached_response.deserialize()?);
        }
        None
    }
    
    /// 写缓存
    pub async fn put(&self, prompt: &str, response: &CompletionResponse) -> Result<(), LlmError> {
        let embedding = self.embed(prompt).await?;
        self.embeddings.put(&Key(format!("emb:{}", hash(prompt)).into_bytes()), &embedding).await?;
        self.responses.put(&Key(format!("resp:{}", hash(prompt)).into_bytes()), &serialize(response)?).await?;
        Ok(())
    }
}
```

### 6.3 Context 压缩

```rust
// apeireth-prompt/src/compactor.rs

pub struct ContextCompactor {
    summarizer: Arc<dyn LlmProvider>,
    max_context_tokens: u32,
    preserve_recent: u32,    // 保留最近 N 条消息
}

impl ContextCompactor {
    /// 压缩 context (旧消息摘要, 保留最近消息)
    pub async fn compact(&self, messages: Vec<Message>) -> Result<Vec<Message>, LlmError> {
        if self.total_tokens(&messages) <= self.max_context_tokens {
            return Ok(messages);
        }
        
        // 拆分: 旧的 + 最近的
        let split_at = messages.len().saturating_sub(self.preserve_recent as usize);
        let (old, recent) = messages.split_at(split_at);
        
        // 摘要旧的
        let summary_prompt = format!(
            "请摘要以下对话:\n{}",
            old.iter().map(|m| format!("{:?}: {}", m.role, m.content)).collect::<Vec<_>>().join("\n")
        );
        let summary = self.summarizer.complete(&CompletionRequest {
            model: "gpt-4o-mini".into(),  // 用便宜模型
            messages: vec![Message::user(summary_prompt)],
            ..Default::default()
        }).await?;
        
        // 重构: [system, summary_message, ...recent]
        let mut compacted = vec![
            Message::system(format!("之前的对话摘要: {}", summary.content))
        ];
        compacted.extend_from_slice(recent);
        
        Ok(compacted)
    }
}
```

### 6.4 预算控制

```rust
// apeireth-prompt/src/budget.rs

pub struct TokenBudget {
    daily_limit: u64,
    monthly_limit: u64,
    current_usage: Arc<RwLock<Usage>>,
}

impl TokenBudget {
    /// 检查是否超预算
    pub async fn can_proceed(&self, estimated_tokens: u64) -> Result<(), LlmError> {
        let usage = self.current_usage.read().await;
        if usage.today + estimated_tokens > self.daily_limit {
            return Err(LlmError::DailyBudgetExhausted);
        }
        if usage.this_month + estimated_tokens > self.monthly_limit {
            return Err(LlmError::MonthlyBudgetExhausted);
        }
        Ok(())
    }
    
    /// 记录使用
    pub async fn record(&self, usage: Usage) {
        let mut current = self.current_usage.write().await;
        current.today += usage.input_tokens + usage.output_tokens;
        current.this_month += usage.input_tokens + usage.output_tokens;
        current.cost += usage.cost;
    }
}
```

### 6.5 模型降级

```rust
// 路由策略: 降级链
let downgrade_chain = vec![
    "gpt-4o",          // 主力
    "claude-3.5-sonnet", // 次主
    "gpt-4o-mini",     // 降级 1
    "claude-3-haiku",  // 降级 2
    "qwen2.5:7b",      // 本地降级
];

// token 不够或预算耗尽时, 自动降级
match budget.can_proceed(req.tokens_estimate()).await {
    Ok(()) => router.complete(req).await,
    Err(BudgetExhausted) => router.complete_with_model(req, downgrade_chain[2]).await,
}
```

---

## 7. 与智囊团协作

智囊团 7 顾问**每个有默认 provider**:

```rust
pub struct AdvisorConfig {
    pub advisor: AdvisorType,        // safety / performance / ...
    pub provider: ProviderType,
    pub model: String,
    pub system_prompt: String,
    pub requires_council: bool,       // 是否需要 7 个都通过
}

pub fn default_council_configs() -> Vec<AdvisorConfig> {
    vec![
        AdvisorConfig {
            advisor: AdvisorType::Safety,
            provider: ProviderType::Anthropic,    // Claude 在 safety 强
            model: "claude-3.5-sonnet".into(),
            // ...
        },
        AdvisorConfig {
            advisor: AdvisorType::Performance,
            provider: ProviderType::DeepSeek,    // DeepSeek 在代码/性能强
            model: "deepseek-coder".into(),
            // ...
        },
        // ...
    ]
}
```

---

## 8. 阶段 2 第七项收尾判定

LLM 集成已沉淀: **多 LLM + 路由 + 本地管家 + fallback + token 经济**。

**关键设计**:
- ✅ `LlmProvider` trait 抽象 (8+ providers)
- ✅ `LlmRouter` 6 种路由策略 (Fixed/Cost/Latency/Capability/RoundRobin/Bandit)
- ✅ 本地管家 (`LocalSteward` in `apeireth-asi`)
- ✅ Fallback chain (主→次→本地→拒绝)
- ✅ Circuit Breaker + Retry with backoff
- ✅ Token 经济 5 机制 (缓存/压缩/批处理/预算/降级)
- ✅ 智囊团 7 顾问默认 provider 配置

**R14 增量**:
- 新增 `apeireth-prompt` crate (阶段 2 §3 已列)
- 增强 `apeireth-asi` 含 `LocalSteward`

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (LLM 服务 ASI 方向)
- 主 17:43 S-2 (基于 Hermes/调研, 不凭空)
- 主 17:58 O-5 (本地管家是 E 层守门)
- 主 19:33 O-2 (Hermes 10 providers 借鉴)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查)

**下一步**: 阶段 2 第八项 — **模块化**

---

## 9. 决策对比表

| 方案 | 灵活性 | 复杂度 | 推荐 |
|------|--------|--------|------|
| 单 LLM (只用 GPT-4) | ❌ | 低 | ❌ |
| 手动切换 LLM | ⚠️ | 低 | ❌ |
| **多 LLM + 自动路由** | ✅ | 中 | ✅✅ |
| 多 LLM + 强化学习路由 | ✅ | 高 | ✅ 自我进化 |

**Apeireth 选多 LLM + 路由**:
- Phase 1: Fixed 策略 (2-3 个 provider)
- Phase 2: CapabilityBased 策略
- Phase 3: MultiArmedBandit 策略 (自我进化)

---

_主哲学 anchor 6 个全贯穿. LLM 集成已沉淀. 下一步等用户确认进入阶段 2 第八项 (模块化)._