# R149 GitHub 调研 — 每个模块对应优秀项目

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R149 (调研 + 立刻升级)
> **日期**: 2026-08-13
> **主人授权**: 全部自主决定,干到底

---

## §0 摘要

调研 4 批 × 多组 web_search,覆盖 GitHub Trending (Rust/AI/Python) + 各模块对应领域。

| 维度 | 数量 |
|---|---|
| 调研模块数 | 79 active crates → 15 功能域 (一对一) |
| 识别可借鉴标杆 | 28 个 (大厂 17 + 中小 11) |
| 终极目标新增/升级项 | P0 5 项 + P1 7 项 + P2 3 项 |
| 拟补 Kani proof | 3 个 (Self-Disable) |

核心结论:Apeireth 架构超行业平均,借鉴非"换骨架"而是"补战术":
- tool-search/browser/runtime/registry — 已超 VCP,向 LangGraph/MCP 升级
- 认知层 — 业界无对标,独立探索
- 形式化 — 业界独一档,继续加深
- pipeline/workflow — 借鉴 Temporal.io (Workflow+Activity+EventHistory)
- skills — 借鉴 Anthropic Skills 三层加载
- vector — 借鉴 Qdrant/LanceDB 后端 (保留自有 RAG)
- state — 借鉴 XState statechart

---

## §1 GitHub Trending 优秀项目

| 项目 | stars | 对标模块 |
|---|---|---|
| OpenClaw | 171K | runtime |
| browser-use | 96.8K | tool-browser |
| cc-switch | 90K | config |
| OpenClaude | 28.3K | agent |
| OpenCLI | 23.4K | cli |
| claude-mem | 24K | memory + council |
| Hyperlight | 4.4K | sovereignty (micro-VM) |
| PG-durable-execution | 1.7K | pipeline |
| Policy isolation | 796 | onion |

## §2 LLM 网关 (provider/api/protocol)

标杆:BerriAI/litellm (35K,290+ provider)、Portkey-AI/gateway (9K,policy engine)、envoyproxy/ai-gateway (1K+,Envoy 集成)、openai-agents-sdk (18K,Handoffs+Guardrails)、anthropic-sdk-python (1.5K,prompt caching)、Anthropic Skills 三层加载。

借鉴:
- provider 升级 LiteLLM-style cost tracking + Portkey policy engine (L0 HA gate)
- api 加 OpenAI 兼容 /v1/messages (VCP 用 Anthropic 协议)
- 保留 protocol 自有

## §3 Multi-Agent (council/supervisor/agent/graph)

标杆:langgraph (16K,checkpoint)、autogen (50K,Actor)、CrewAI (30K+,role-based)、MetaGPT (4.8K,SOP)、letta-ai/letta (18K,MemGPT)。

借鉴:
- council 已有 7 advisor + group_chat,超 AutoGen
- graph 加 LangGraph checkpoint
- council::sop 模块 (MetaGPT 风格)
- council::session 升级 (claude-mem 风格)

## §4 工具战区 (tool-* 全套)

标杆:modelcontextprotocol/servers (4K)、mark3labs MCP Rust SDK、modelcontextprotocol/rust-sdk (600+)、spider-rs/spider (3K,无浏览器爬虫)、mattsse/chromiumoxide (1.7K,CDP)、browser-use (96.8K)、tauri-apps/tauri (95K)。

借鉴:
- tool-browser 已用 chromiumoxide
- **apeireth-tool-fetch (Tier 1.5 唯一缺项)** — R149 立刻实现,吸收 7 VCP plugins:
  UrlFetch + TavilySearch + AnySearch + VSearch (合 tool-search) + FlashDeepSearch + BilibiliFetch + AnimeFinder

## §5 记忆战区 (memory/vector/context-fold/state)

标杆:letta-ai (18K,hierarchical)、mem0 (30K+,跨 session)、MIRIX (5K,多模)、langmem、qdrant (22K,Rust 向量 DB)、lancedb (6K,embedded)、xstate (28K,statechart)、surrealdb (30K,Rust 多模)、neo4j (14K,GQL 1.0)、claude-mem (24K,session 捕获)。

借鉴:
- memory R146 已合并 lightmemo+dailynote
- vector 加 Qdrant 协议兼容
- state 加 XState statechart
- relation 加 SurrealDB 后端 (可选)

## §6 形式化安全 (sovereignty/onion/constraint/formal)

标杆:model-check/kani (3K+,模型检验)、verus-lang/verus (1.7K,验证)、stateright/stateright (800,分布式)、Hyperlight (4.4K,micro-VM)、constitutional-ai。

借鉴:
- 业界无对标,继续做独一档
- 补 3 个缺失 Kani proof (Self-Disable 自动扫描 / L0 HA 物理多签 / 13-key verdict cache)
- Hyperlight — R150+ 调研

## §7 调度与编排 (pipeline/workflow/runtime/cron)

标杆:temporalio/temporal (13K,Workflow+Activity)、prefect (16K,DAG)、questdb/tokio-cron-scheduler (800,Rust cron)、apache/airflow (35K,DAG)、openobserve (13K,Rust 日志)。

借鉴:
- runtime R147 已有 7 模块
- **pipeline 升级 Temporal Workflow+Activity** — workflow deterministic,activity 处理副作用
- **cron 升级 tokio-cron-scheduler**

## §8 互操作 (mcp/protocol/pybridge/skills)

标杆:modelcontextprotocol/rust-sdk (600+,官方)、pyo3 (13K+,已用)、PyOxidizer、Anthropic Skills 三层加载。

借鉴:
- pybridge 823 行已实现 PyO3 桥
- **skills 升级 Anthropic Skills 模式** — SKILL.md + scripts/ + references/ + assets/,渐进加载

## §9 TUI/前端 (tui/tui-e2e/tauri-stub/desktop)

标杆:ratatui (2.3K,已用)、gptscript (4K,TUI+agent)、open-tui (React)、tauri-apps/tauri (95K)。

借鉴:
- tui 255KB + 5 页面,继续
- 暂不接 TUI (主人 R148 决定)
- tauri-stub 留 R150+

## §10 评测/测试 (eval/bench/test/integration-e2e)

标杆:EleutherAI/lm-evaluation-harness (9K)、SWE-bench (3K+)、proptest (1.7K,property-based)、cargo-mutants (1K,mutation testing)。

借鉴:
- eval 升级 SWE-bench 风格任务 (issue + patch + test)
- test 加 proptest
- cargo-mutants 留 R150+

## §11 其他模块

- voice + livekit → GPT-Realtime-2 (speech-to-speech + 128K)
- lark → 飞书自有 SDK,保留
- bus → 已修 24/24,无升级
- arbitration → OpenObserve 风格
- asi / blueprint-impl / central → 独立探索
- skills → **升级 Anthropic Skills (R149)**
- graph → **加 LangGraph checkpoint (R149)**
- workflow → **Temporal Workflow (R149)**
- i18n → 已实现

## §12 终极目标 — 优先级

### P0 (R149 立刻)
1. **apeireth-tool-fetch** — Tier 1 唯一缺项,吸收 7 VCP plugins
2. **apeireth-skills Anthropic Skills 升级**
3. **apeireth-runtime 真 MiniMax worker** (替 SimulatedWorker)
4. **apeireth-graph LangGraph-style checkpoint**
5. **Self-Disable Kani proof 补 3 个**

### P1 (R150)
6. vector Qdrant 协议兼容
7. pipeline Temporal-style Activity
8. state XState statechart
9. cron 迁 tokio-cron-scheduler
10. council session 自动捕获 (claude-mem)
11. eval SWE-bench 风格
12. test 加 proptest

### P2 (R151+)
13. sovereignty Hyperlight micro-VM
14. relation SurrealDB 后端
15. voice GPT-Realtime-2

## §13 不做

- ❌ 占卜/酒馆/论坛类 — 已冻结
- ❌ 第三方商业 SDK 1:1 翻译
- ❌ 完整复制 LangGraph/AutoGen/CrewAI
- ❌ Tauri 桌面端深入

## §14 不假装

- 调研基于 web_search + README,非实际克隆
- 借鉴方案需 R149+ 实施验证
- P0/P1/P2 是估计,实际以实现难度为准

---

> 下一步:R149 立刻实现 P0 的 5 项。