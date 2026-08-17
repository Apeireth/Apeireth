# R269: MultiModelAdvisorBackend (跨多 LLM 决策聚合)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 给 council 7 advisor 加跨多 LLM 决策能力, 复用 MockLlmProvider trait (跟 LlmAdvisorBackend 一致)

---

## §1 背景

R16-09 已有 `LlmAdvisorBackend` (单 LLMProvider → MockLlmProvider adapter),
但 council 每次 deliberation 只调一个 LLM. 跨多 model 决策 = ensemble/fallback.

R258 Tier A 候选: Council 跨多 model 决策 = ★★★★.

---

## §2 设计

### 2.1 AggregationStrategy (3 种)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// 第一个非空响应 (fallback chain). 默认.
    FirstNonEmpty,
    /// 最长 text 的响应 (假设最长=最有信息).
    Longest,
    /// 拼接所有非空响应, 用 "
--
" 分隔.
    ConcatAll,
}
```

### 2.2 MultiModelAdvisorBackend

```rust
pub struct MultiModelAdvisorBackend {
    backends: Vec<Arc<dyn LlmProvider>>,
    strategy: AggregationStrategy,
}

impl MultiModelAdvisorBackend {
    pub fn new(backends: Vec<Arc<dyn LlmProvider>>) -> Self;
    pub fn with_strategy(backends: Vec<Arc<dyn LlmProvider>>, strategy: AggregationStrategy) -> Self;
    pub fn backend_count(&self) -> usize;
    pub fn strategy(&self) -> AggregationStrategy;
    
    fn collect_responses(&self, prompt: &str, system: &str) -> Vec<MockLlmResponse>;
    // 内部: 在每个 backend 跑 LLM, 收集响应 (failed backend 跳过)
}

#[allow(deprecated)]
impl MockLlmProvider for MultiModelAdvisorBackend {
    fn generate(&self, prompt: &str, system: &str) -> MockLlmResponse {
        let responses = self.collect_responses(prompt, system);
        if responses.is_empty() {
            return MockLlmResponse::ok("[multi-model: all backends failed]");
        }
        match self.strategy {
            FirstNonEmpty => responses.into_iter().find(|r| !r.text.is_empty()).unwrap(),
            Longest => responses.into_iter().max_by_key(|r| r.text.len()).unwrap(),
            ConcatAll => MockLlmResponse::ok(joined.join("
--
")),
        }
    }
}
```

### 2.3 tokio Handle 复用

复用 LlmAdvisorBackend 模式: `tokio::Handle::current().block_on(...)` 调 async LLM.
fallback: 用 `tokio::runtime::Builder::new_current_thread().build().block_on(...)` 单线程 runtime (无当前 runtime 时).

### 2.4 model name 处理

```rust
let model = if backend.name() == "apeireth-api" {
    "MiniMax-M3".to_string()  // 默认 LLM
} else {
    backend.name().to_string()  // provider 自带 name
};
```

跟 R16-09 LlmAdvisorBackend 一致.

### 2.5 lib.rs mod 声明 + dev-dep

```rust
// crates/apeireth-council/src/lib.rs
pub mod llm_backend;
pub mod multi_model_backend;  // R269
```

```toml
# crates/apeireth-council/Cargo.toml [dev-dependencies]
# R269: MultiModelAdvisorBackend tests impl LlmProvider trait (defined with #[async_trait])
async-trait = { workspace = true }
```

---

## §3 测试 (6 cases)

- r269_01_first_non_empty_strategy_picks_first
- r269_02_longest_strategy_picks_longest
- r269_03_concat_all_strategy_joins_with_separator
- r269_04_failed_backend_skipped_gracefully
- r269_05_all_failed_returns_placeholder
- r269_06_strategy_accessor

**6/6 tests pass**.

测试 backend (EchoBackend / FailBackend / LongBackend) impl LlmProvider 用 #[async_trait] (LlmProvider trait 本身就是 #[async_trait] 定义).

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借 ensemble/fallback 概念 (OpenAI/Anthropic SDK 多 key 切换模式), 自实现
- **S-2 实事求是**: 真调所有 LLMProvider::complete(), 0 simulate
- **O-1 安全优先**: failed backend 跳过 (graceful degradation), 不让单点失败污染 ensemble
- **O-2 走在前人**: 复用 LlmAdvisorBackend 模式 (block_on Handle::current)
- **O-3 干到底**: 3 strategies × 2 constructors × 6 tests
- **O-5 不假装**: 真实跨多 backend aggregate, 不是 mock 单 backend 跑多次

---

## §5 后续

- 接到 council_member_deliberation: 把 `with_llm_provider(single_backend)` 改成 `with_multi_model_backends(vec![...], strategy)`
- 接到 pipeline / skills: ensemble 在跨 model 对比场景 (R8)
- 加 confidence 聚合: 当前 0.7 hardcode, 后续用真实 model 返 confidence
