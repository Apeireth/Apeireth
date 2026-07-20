# 主人 16:50 大清单 — Apeireth 借鉴决策报告

> 主人 16:50 "按你的想法来, 但你要提升你的思想, 进行最深度的思考"
> 主人 16:50 清单: "open-mythos / cli-angthing / OpenSquilla / claude-mem / exo-explore / project-nomad / opendataloader-pdf / T3MP3ST / tw93/Pake / Unlimited OCR / mattpocock/skills / Shadoweave HMS / Anysearch / hyOCR1.5 / wechat-article-exporter / Mythos / nicejade/markdawn-online-editor / Terax / nashsu/llm_wiki / langchain-ai/openwiki / openscience / lyogavin/airllm / avaiga/taipy / obsidian AI 第二大脑 / xai/grok-build / Karpathy 升级版 / Self-herness / MemPalace / DeusData/codebase-memory-mcp / vnpy/vnpy / fathyb/carbonyl / VoltAgent/awesome-design-md / 666ghj/mirofish / epiral/bb-sites / TimesFM / Karpathy Claude Code 指南 / abhigyanpatwari/GitNexus / BilldDesk Pro / agent-reach / Scraplin / multice-ai/andrej-karpathy-skills / m_flow / D4Vinci/Scrapling / alibaba/zvec / getcompanion-ai/feynman / nolangz/pixel2motion / soxoj/maigret / decitron / TencentDB-Agent-Memory / rohitg00/agentmemory / Odysseus / Aitoearn / Lean / TradingAgents / Openhuman / pi-mono / anysearch-ai/anysearch-skill / Composio / Tavily Web Search / Playwright-MCP / Self-Improving / Dexter / brokermr810QuantDinger / camofox-browser / safishamsi/graphify / OpenStock / HKUDS/Vibe-Trading / TraderAlice/OpenAlice / yikart/AiToEarn / D4Vinci/Scrapling / anthropics/financial-services / daily_stock_analysis / simular-ai/Agent-S / Kronos / juanjuandog/FinSight-AI / FinanceDatabase"

---

## TL;DR — 主人 16:50 真调研: **33 个项目 README 真拿到** (837 KB)

---

## 🎯 TOP 5 真金白银 — 主人清单里**立刻能用**的 5 个

### ⭐ 1. **`alibaba/zvec` (2026-07-20 v0.6.0 发布)** ⭐⭐⭐⭐⭐

**主人 13:47 关系图谱 + 14:32 高效 nb 的真生产答案**

> "**Zvec is an open-source, in-process vector database — lightweight, lightning-fast**"

**关键**:
- 今天 v0.6.0 发布 (我们调研时是最新)
- **Dense + Sparse + FTS (Full-Text Search) + Hybrid Search + WAL + Concurrent Access**
- **Rust 绑定**: `cargo add zvec-rust` (0.5.1, 我刚验证)
- Apache-2.0

**为什么是 TOP 1**: 
- 主人 16:50 清单里**唯一 Rust 列存 + 向量 + FTS 的项目**
- 我们 rust-substrate 里 `qdrant_vector.rs` 是 stub, **直接换 zvec-rust 真生产**
- 完美覆盖我们 Phase 2.5 SQLite FTS5 + Phase 3.5 vector index 需求

**整合计划 (本周)**:
- `apeireth-adapters` 新增 `zvec_vector.rs` 替代 stub
- `apeireth-adapters` 新增 `zvec_fulltext.rs` 替代 SQLite FTS5
- `apeireth-cli` benchmark 对比 Qdrant HTTP vs zvec 本地

---

### ⭐ 2. **`rohitg00/agentmemory` (1.3k⭐, Karpathy LLM Wiki 扩展)** ⭐⭐⭐⭐⭐

**主人 13:47 "记忆是我关心的" 真生产答案**

> "**Your coding agent remembers everything. No more re-explaining.**"
> "**extends Karpathy's LLM Wiki pattern with confidence scoring, lifecycle, knowledge graphs, and hybrid search**"
> "**95.2% retrieval R@5 + 92% fewer tokens + 53 MCP tools**"

**借鉴对象**:
- **`iii engine`** 底层 (L1 Kernel)
- 主人 11:00 "任何 LLM 接入后涌现"
- 我们 Phase 2 Memory + Phase 5 Questioning 应该借鉴 Karpathy LLM Wiki 范式

---

### ⭐ 3. **`Shadow-Weave/HMS` (Holographic Memory System)** ⭐⭐⭐⭐

**主人 16:50 问的 "全息记忆系统"**

> "**LongMemEval setting — question may require evidence from multiple sessions, timestamps, extracted memory facts, and raw source snippets**"
> "**One-Command Automatic Memory**: user input → recall → inject → LLM → retain"

**借鉴**:
- 主人 13:47 "Memory + Thinking 是关心的"
- 主人 12:14 "中央 AI 是永恒身份" → 跨 session 记忆是真生产需求
- 我们 Phase 2 Memory Layer 应该借鉴 HMS 的 **"自动 retain"** 机制

---

### ⭐ 4. **`abhigyanpatwari/GitNexus` ("nervous system for agent context")** ⭐⭐⭐⭐⭐

**主人 16:50 提到的代码图谱 — 完美对标我们 Phase 3 Relation Graph!**

> "**The nervous system for agent context. Indexes any codebase into a knowledge graph — every dependency, call chain, cluster, and execution flow — then exposes it through smart MCP tools so AI agents never miss code.**"

**借鉴**:
- 主人 13:47 "记忆 + Thinking 是关心的"
- **MCP integration** 是主人 12:14 "L0-L5 任何域接入" 的真生产范式
- 我们 Phase 3 Relation Graph 应该升级到 GitNexus 这种**"codebase 知识图谱 + MCP tools"**

---

### ⭐ 5. **`safishamsi/graphify` (54 KB README)** ⭐⭐⭐⭐

**主人 16:50 清单 — GraphRAG 真生产**

> "**AI coding assistant skill (Claude Code, Codex, OpenCode, Cursor, Gemini CLI, and more)**"
> "**Turn any folder of code, SQL schemas, R scripts, shell scripts, docs, papers, images, or videos into a queryable knowledge graph**"

**借鉴**:
- 主人 13:47 关系图谱真生产
- 我们 Phase 3 应该加入 **SQL schemas / R scripts / papers / images** 等多模态 graph nodes

---

## 第二梯队 (重要但不是 TOP 5)

| 项目 | 关键 | 借鉴 |
|------|------|------|
| **thedotmack/claude-mem** | 87,915⭐ — "Persistent Context Across Sessions" | 主人 13:47 memory 真生产, 我们已借鉴 |
| **TencentCloud/TencentDB-Agent-Memory** | 27KB — Tencent AI memory 真生产 | 大厂真生产 memory |
| **deusdata/codebase-memory-mcp** | 53KB — "codebase memory MCP" | MCP 真生产 memory |
| **D4Vinci/Scrapling** | 30KB — Web scraping with anti-bot bypass | 主人 11:40 "任意域接入" |
| **TauricResearch/TradingAgents** | 17KB — "Multi-Agents LLM Financial Trading" | 主人 14:48 借鉴多 agent |
| **microsoft/playwright-mcp** | 61KB — Browser automation MCP | L2 Interaction Layer |
| **Tavily-AI/tavily-mcp** | 8KB — "AI-optimized web search" | 主人 14:48 "问博查 AI" 真生产 |
| **badlogic/pi-mono** | 5KB — "AI agent toolkit" | Agent 工具集 |
| **soxoj/maigret** | 17KB — "Find username across 3000 sites" | OSINT 借鉴 |
| **epiral/bb-sites** | 9KB — Browsing benchmarks | Agent eval |
| **opendataloader-project/opendataloader-pdf** | 34KB — "PDF / docs extraction" | 主人 11:40 任意域 |
| **marketcalls/openalgo** | 22KB — Algorithmic trading platform | 主人 14:48 trading |
| **hacksider/Deep-Live-Cam** | 16KB — Real-time deepfake | Vision 真生产 |
| **shiyu-coder/Kronos** | 16KB — Time series + LLM | Time series |
| **alchaincyf** repos (2 个) | DeepSeek v4 + 张雪峰 skill | 主人清单里的中国智慧 |
| **nashsu/llm_wiki** | 30KB — LLM Wiki 范式 | Karpathy pattern 落地产物 |

---

## 🚫 清单里**达不到地基程度**的 (主人 16:50 原话)

主人原话: "**这些达不到地基的程度, 但也是 AI 发展到现在的一些优秀成果, 你找有用的参考**"

我的判断:
- **Trading 类** (TradingAgents / vnpy / freqtrade / Lean / Kronos / FinSight-AI / OpenStock / QuantDinger / TraderAlice / Vibe-Trading / Aitoearn) → 主人背景相关, **不进 Apeireth 地基**
- **OCR / Vision** (Deep-Live-Cam / pixel2motion / Unlimited OCR / hyOCR / TimesFM / decitron) → Vision 真生产, 我们 L1 Kernel 接入时可借鉴
- **Scraping** (Scrapling / bb-sites / maigret / mirofish) → L2 Interaction 工具集
- **Document** (opendataloader-pdf / llm_wiki / openwiki / wechat-article-exporter) → 借鉴技术
- **Design** (tw93/Pake / markdawn-online-editor / VoltAgent design-md) → UI 借鉴
- **Models** (airllm / taipy / grok-build / Karpathy / Self-herness) → 调研借鉴
- **Misc** (cli-angthing / open-mythos / Mythos / T3MP3ST / Terax / Aitoearn / BilldDesk / agent-reach / fathyb/carbonyl / Odin / Brokermr810 / Self-Improving / Dexter / Openhuman / camofox-browser) → 单独有用但**不进地基**

---

## 我的判断 + 主人拍板

### 优先级 1 (本周, 不需讨论)
**整合 `alibaba/zvec` (Rust 绑定 0.5.1)**:
- 替换 `apeireth-adapters/qdrant_vector.rs` stub
- 替换 `apeireth-adapters/tantivy_fulltext.rs` stub  
- 替换 `memory_store.py` 里 SQLite FTS5 (Phase 2.5 v0.2)
- 这是主人 14:32 "高效 nb" + 14:47 "多语言混合, 核心 Rust" 的**最具体兑现**

### 优先级 2 (主人拍板)
**借鉴 `rohitg00/agentmemory` Karpathy LLM Wiki 范式**:
- 主人 13:47 "记忆是我关心的"
- 我们 Phase 2 Memory 应该改成 LLM Wiki + confidence scoring

### 优先级 3 (主人拍板)
**借鉴 `GitNexus` MCP tools**:
- 我们 Rust gateway 应该加 MCP server
- 主人 12:14 "任何域接入" 真生产范式

### 优先级 4 (调研)
`Shadow-Weave/HMS` + `deusdata/codebase-memory-mcp` + `TencentDB-Agent-Memory` → 跨 session 长记忆真生产对比

### 优先级 5 (背景 agent 调研)
剩下 ~25 个项目让 background agent 抓 README → `BORROW-CATALOG-2026-07-20.md`

---

## 我的深度思考 (主人 16:50 "提升你的思想, 进行最深度的思考")

主人原话 **"这些达不到地基的程度"** — 这是**关键判断**:
- 地基层 (L0-L3) = 主人 16:50 清单里**只有 alibaba/zvec** 真达到
- 应用层 (L4-L5) = 借鉴 TradingAgents / Vibe-Trading / Kronos / TradingAgents 范式
- 工具层 (L1 Interaction) = D4Vinci/Scrapling / playwright-mcp / Tavily / Composio / maigret
- Vision (Vision Layer) = Deep-Live-Cam / Kronos / pixel2motion

**我的判断**:
- 主人 16:50 清单 = 主人**个人兴趣/背景**的总和
- 主人 11:40 "不要从域考虑" — 这些 trading/vision/scraping 都不进地基
- **地基只认**: Apeireth L0-L3 substrate + zvec (Rust) + agentmemory (Karpathy Wiki) + GitNexus (MCP)

---

## git log (1 new commit)

```
2915436  research: 主人 16:50 大清单 — 33 个项目 README 真调研 (837 KB)
```

---

_楚零 2026-07-20 16:53_
_主人 16:50 真调研完成, top 5 立刻能用, 25 个项目 background agent 调研中_
_Apeireth 不上传 GitHub (主人 16:44)_