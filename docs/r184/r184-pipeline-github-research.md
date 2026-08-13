# R184 GitHub 优秀项目调研 — pipeline 模块 (工作流引擎 / DAG / LLM 管道)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R184
> **日期**: 2026-08-13
> **范围**: apeireth-pipeline (226KB) + apeireth-pipeline-g5 (76KB) 升级路径
> **状态**: 调研为升级预备.

---

## 0. 现状

### apeireth-pipeline (12 文件 226KB)
- lib.rs (28KB) — 主入口
- model_router.rs (30KB) — 模型路由
- provider_registry.rs (46KB) — provider 注册表
- role_divider.rs (25KB) — 角色分割
- force_translate / g5_chat_bridge / retry_suppression / streaming / tiktoken_counter / token_budget / tool_loop / placeholder

### apeireth-pipeline-g5 (10 文件 76KB)
- pipeline.rs (13KB) — 5 阶段管道
- reliability.rs (10KB) — 重试/降级
- policy.rs (8KB) — 策略
- dispatch / error / message / normalize / stage / throttle / lib

两个 pipeline crate 已经形成: **model_router + provider_registry + role_divider + token_budget + g5 5 阶段 + reliability** 的完整 LLM 管道体系.

---

## 1. Rust 工作流 / DAG 引擎

### 1.1 daggy (sfrijters/daggy) — **RECOMMENDED 当前用**

- **GitHub**: https://github.com/sfrijters/daggy
- **License**: MPL-2.0
- **定位**: 有向无环图 + 拓扑排序
- **现状**: 我们 pipeline 大概率已用
- **强化**: 加可视化 (dot / mermaid 输出)

### 1.2 petgraph (再次) — 算法 SOTA

- 我们 graph/traversal 已经在用
- pipeline 可借鉴其算法库

### 1.3 tokio (再次) — 运行时

- 我们整个异步生态都基于 tokio
- pipeline 调度是 tokio task 编排

### 1.4 bollard (fussybeaver/bollard) — 备选

- Docker API client
- 我们 sandbox 当前用, pipeline 不直接相关

### 1.5 temporalio/sdk-core / restate / cadence — **学习 (Rust 重写)**

- Rust SDK for Temporal (分布式工作流)
- 借鉴 long-running workflow 设计
- **不集成**: Temporal 是 Java 主导, Rust SDK 不成熟

---

## 2. LLM 管道框架 (Python 生态 SOTA, 不集成学设计)

### 2.1 LangChain (langchain-ai/langchain) — **学习 + 警惕**

- **Stars**: 90K+
- **License**: MIT
- **核心**: chains / agents / retrieval / memory
- **学习点**: chain 抽象 + LCEL (LangChain Expression Language)
- **警惕**: 复杂度爆炸, 我们不要重蹈覆辙

### 2.2 LlamaIndex (run-llama/llama_index) — **学习**

- **Stars**: 35K+
- **License**: MIT
- **核心**: RAG 数据框架
- **学习点**: QueryEngine / Retriever / Reader 三件套
- **价值**: 我们 memory RAG 可借鉴

### 2.3 Haystack (deepset-ai/haystack) — 学习

- **License**: Apache 2.0
- **核心**: Pipeline / Component / DocumentStore
- **学习点**: Pipeline YAML 定义 (DAG as code)

### 2.4 Semantic Kernel (microsoft/semantic-kernel) — 学习

- **License**: MIT
- **核心**: SDK + Planner + Memory
- **学习点**: Planner 自动编排 (我们 council planner_executor 借鉴)

### 2.5 Mirascope (Mirascope/mirascope) — **学习 (Rust 写, 不用学)**

- Rust 写的 LLM 框架
- **学习点**: type-safe prompt
- **借鉴价值**: 我们 provider_registry 可以借鉴其类型化设计

### 2.6 DSPy (stanfordnlp/dSPy) — **RECOMMENDED 学习**

- **Stars**: 24K+
- **License**: Apache 2.0
- **定位**: 编译 LLM 管道 (prompt as code, 自动优化)
- **核心能力**:
  - Signatures (类型化 prompt)
  - Modules (Predict / ChainOfThought / ReAct / ProgramOfThought)
  - Optimizers (自动 prompt 调优)
  - 评估框架
- **学习点**:
  - Signatures -> 我们的 provider_registry 类型化增强
  - Optimizers -> 我们的 role_divider 自动角色选择
  - 评估 -> 我们 council deliberation 评估

### 2.7 Instructor (jxnelkar/instructor-rs) — **学习**

- **License**: MIT
- **定位**: 结构化 LLM 输出 (JSON / type-safe)
- **学习点**: schema 验证 + 自动 retry
- **价值**: 我们 role_divider 输出结构化

### 2.8 guidance (microsoft/guidance) — 学习

- **License**: MIT
- **核心**: 模板化 prompt + token-level control
- **学习点**: 控制 LLM 输出的 token 级精度

### 2.9 outlines (outlines-dev/outlines) — 学习

- **License**: Apache 2.0
- **核心**: 保证 LLM 输出符合 JSON / regex / grammar
- **学习点**: grammar-guided generation

---

## 3. Reliability / 弹性模式 SOTA

### 3.1 retry / circuit-breaker (Hystrix / resilience4j 类比)

- 我们 retry_suppression.rs 已有
- **强化**:
  - **Circuit Breaker** (断路器): 失败 N 次后熔断, M 秒后半开
  - **Bulkhead** (舱壁): 隔离不同 provider 的资源池
  - **Timeout**: 每步超时
  - **Retry with backoff**: 指数退避 + 抖动

**借鉴实现 (Rust 生态)**:
- **failsafe-rs** (Layer 5 Networks/failsafe-rs) — RECOMMENDED
  - Circuit Breaker / Retry / Rate Limiter / Fallback
  - Rust 异步原生
  - MIT
- **backoff** (joon-github/backoff) — 退避算法
- **tokio-retry** (tokio-rs/tokio-retry) — tokio 官方 retry

### 3.2 OpenTelemetry (open-telemetry/opentelemetry-rust) — 可观测性

- Rust 官方 OTel 实现
- 我们 observability.rs 可加 OTel 导出
- **学习点**: distributed tracing

---

## 4. 缓存 / 优化

### 4.1 GPTCache (zilliztech/gptcache) — **学习**

- **License**: Apache 2.0
- **核心**: LLM 响应缓存 (语义级, 不只精确匹配)
- **学习点**: embedding-based cache lookup
- **价值**: 我们的 model_router 可加 cache

### 4.2 Semantic Router (aurelio-ai/semantic-router) — 学习

- **License**: Apache 2.0
- **核心**: 语义级请求路由
- **学习点**: 我们的 role_divider 已经类似, 可借鉴更多

---

## 5. 评估 / 监控

### 5.1 PromptTools (hegelai/prompttools) — 学习

- **License**: Apache 2.0
- **核心**: prompt A/B 测试 + 评估
- **学习点**: 系统化评估

### 5.2 Ragas (explodinggradients/ragas) — 学习

- **License**: Apache 2.0
- **核心**: RAG 评估
- **学习点**: faithfulness / relevance 指标

### 5.3 Langfuse (langfuse/langfuse) — **学习 (可自托管)**

- **License**: MIT
- **核心**: LLM 可观测性平台 (self-hostable)
- **学习点**: trace / metric / prompt management
- **价值**: 我们 observability.rs 可对接 Langfuse 作为外部 sink

---

## 6. 升级方案 (最终阶段执行)

### 6.1 短期 (1-2 days)

1. **加 failsafe-rs**: Circuit Breaker / Bulkhead / Timeout
2. **加 backoff crate**: 指数退避
3. **加 sem-cache**: embedding-based LLM 响应缓存
4. **强化 tiktoken_counter**: 更精确 token 预算

### 6.2 中期 (3-5 days)

5. **Type-safe Signatures**: 借鉴 DSPy, provider_registry 类型化增强
6. **Semantic Router**: 我们的 role_divider 加 embedding 路由
7. **结构化输出**: 借鉴 Instructor, role_divider 输出结构化 JSON
8. **Langfuse 集成**: 可观测性导出

### 6.3 长期 (持续)

9. **Prompt Optimizer**: 借鉴 DSPy Optimizers, 自动 prompt 调优
10. **OpenTelemetry**: 分布式 tracing 标准化
11. **DAG 可视化**: pipeline dot/mermaid 输出

---

## 7. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| daggy (当前) | ~0 | MPL-2.0 | 是 |
| failsafe-rs | ~30KB | MIT | 短期 |
| backoff | ~10KB | MIT/Apache-2.0 | 短期 |
| tokio-retry (已有) | ~0 | MIT | 是 |
| opentelemetry (按需) | ~5MB | Apache-2.0 | 长期 |

**总增加**: 短期 < 50KB, 长期 OTel ~5MB

---

## 8. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| council (R180) | planner_executor 借鉴 Semantic Kernel Planner |
| tool (R140) | tool_loop 是 pipeline 的一类 stage |
| memory (R146) | GPTCache 风格的语义级 cache |
| tui (R183) | observability 显示 |
| observability | OTel 导出 |

---

## 9. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- pipeline 公开 API: 0 改 (新能力在 stage / policy 子模块内)

---

## 10. 参考链接

- daggy: https://github.com/sfrijters/daggy
- temporalio/sdk-core: https://github.com/temporalio/sdk-core
- LangChain: https://github.com/langchain-ai/langchain
- LlamaIndex: https://github.com/run-llama/llama_index
- Haystack: https://github.com/deepset-ai/haystack
- Semantic Kernel: https://github.com/microsoft/semantic-kernel
- Mirascope: https://github.com/Mirascope/mirascope
- DSPy: https://github.com/stanfordnlp/dspy
- Instructor: https://github.com/instructor-ai/instructor
- guidance: https://github.com/microsoft/guidance
- outlines: https://github.com/outlines-dev/outlines
- failsafe-rs: https://github.com/l5-rs/failsafe-rs
- backoff: https://github.com/joon-github/backoff
- tokio-retry: https://github.com/tokio-rs/tokio-retry
- OpenTelemetry Rust: https://github.com/open-telemetry/opentelemetry-rust
- GPTCache: https://github.com/zilliztech/gptcache
- Semantic Router: https://github.com/aurelio-ai/semantic-router
- PromptTools: https://github.com/hegelai/prompttools
- Ragas: https://github.com/explodinggradients/ragas
- Langfuse: https://github.com/langfuse/langfuse