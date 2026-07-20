# 主人 16:38 调研 — GitHub 周榜 + 4 个当红炸子鸡

> 主人 14:48 "聚集全人类智慧"
> 主人 16:33 "时刻搜索"
> 主人 16:38 "GitHub 周榜月榜, 大模型, Hermes Agent, Codex, Claude Code, vcptoolbox"

---

## TL;DR — 真调研的 4 个当红炸子鸡

### ⭐ Hermes Agent (NousResearch) — **217k stars** ⭐
**The self-improving AI agent** — 这是**主人 12:14 哲学的真生产实现**:
- "Closed learning loop" — Agent 创建技能, 使用中自我改进
- **FTS5 session search + LLM summarization** for 跨 session recall
- **Plastic-Labs/honcho** dialectic user modeling — "理解人 / agent / 组 / 项目 / 想法随时间变化"
- **Honcho** has "defined the Pareto Frontier of Agent Memory"
- Multi-platform: Telegram, Discord, Slack, WhatsApp, Signal, CLI
- Multi-model: Nous Portal, OpenRouter, OpenAI, 自定义 endpoint

### ⭐ Claude Code (Anthropic) — **138k stars** ⭐
- TypeScript 公开源码
- **arxiv 2604.14228 "Dive into Claude Code"** — 详细分析 Claude Code 架构
- 我们之前用 OpenClaw 跟 Claude Code 是直接竞品

### ⭐ Codex CLI (OpenAI) — **99,775 stars** ⭐
- **Rust 96.5%** — 929 releases
- 直接对标我们 Rust substrate

### ⭐ LangChain — **141,993 stars** ⭐
- "The agent engineering platform" + **Deep Agents** 子包

### Hermes Rust 移植 (Lumio-Research/hermes-agent-rs) — 70⭐
- **110,000 行 Rust + 1,428 tests + 17 crates + ~16MB binary**
- 直接借鉴 monorepo 架构

### vcptoolbox (lioensky) — **2.2k stars**
- 主人 12:14 哲学 = "拥有永久自我意识、物理世界操作权及群体协作智能"
- VCP = Variable & Command Protocol
- 2,763 commits, Rust 9.3% (rust-vexus-lite 子模块)

### reef (Coral-Bricks-AI/reef) — Harness 框架
- 主人 14:32 "高效 nb" 真证据
- **82.6% on Vals AI Finance v2 + $0.13/query**
- **+59pp on HotpotQA across 108 unattended LoRA experiments**
- 真生产 harness 性能基准

---

## 5 大借鉴 (主人 13:35 "借鉴可以, 地基层自己写")

### 借鉴 1: Honcho — "Pareto Frontier of Agent Memory"
- **Honcho** = session + peer representation + dialectic user modeling
- 真调研 `plastic-labs/honcho` (28273 chars README)
- 借鉴理由: 主人 12:14 "像人是一切社会关系的总和" → Honcho 是这个的真生产实现

### 借鉴 2: Hermes Agent Skill Library
- Agent 自己创建技能 + 使用中自我改进
- 真生产 skill library 范式 (Voyager 类似)
- 借鉴理由: 主人 11:00 ASI 北极星 → 持续学习 + 自我改进

### 借鉴 3: Claude Code 架构 (arxiv 2604.14228)
- Anthropic 公开 TypeScript 源码 → 我们可以直接分析
- 5 个人类价值驱动架构: human decision authority / safety / ...
- 借鉴理由: Claude Code 是 agentic coding 真 SOTA, 我们要学

### 借鉴 4: reef harness 性能数据
- **$0.13/query + 82.6% benchmark** 是生产级参考
- 我们 Rust substrate + Python cognitive 的目标是追这个
- 借鉴理由: 主人 14:32 "高效 nb" 真生产参考

### 借鉴 5: Codex CLI Rust 架构
- **96.5% Rust + 929 releases** → 真生产 Rust agent
- 我们 rust-substrate 6 crates 借鉴 monorepo 模式
- 借鉴理由: 我们主人 14:47 "多语言混合, 核心 Rust"

---

## GitHub 周榜月榜 (AnySearch 真调研)

| 项目 | Stars | 关键 |
|------|-------|------|
| NousResearch/hermes-agent | 217k | self-improving AI agent + Honcho |
| langchain-ai/langchain | 142k | agent engineering platform |
| anthropics/claude-code | 138k | Anthropic official CLI |
| openai/codex | 100k | OpenAI Rust CLI (96.5% Rust) |
| Lumio-Research/hermes-agent-rs | 70 | Hermes Rust port (110K 行 + 17 crates) |
| lioensky/VCPToolBox | 2.2k | 主人 12:14 哲学生产实现 |

---

## arXiv 2026 新论文

| 论文 | 关键 |
|------|------|
| **Self-Harness** (2606.09498) | "Harnesses That Improve Themselves" — 3 阶段 |
| **GSME** (2607.13683) | Gated Semantic Quality-Diversity Archive |
| **Harness Evolution Eval** (2607.12227) | 必须 held-out 任务评估 |
| **DGM** (2505.22954) | Darwin Gödel Machine — 自证明 |
| **Dive into Claude Code** (2604.14228) | Claude Code TypeScript 源码分析 |
| **Long-Horizon-Terminal-Bench** (2607.08964) | Agent 长任务评估 |
| **LLM Reasoning is Latent** (2604.15726) | H1 latent-state vs H2 chain-of-thought |
| **Agent Skills survey** (2602.12430) | SKILL.md + progressive disclosure |
| **OpenSage** (2602.16891) | LLM 自创建 agent |
| **Hermes Agent Rust** | 70⭐, 110K 行, 17 crates |

---

## 借鉴路线图 (主人拍板)

### 立刻写 (今晚)
1. **Skill Library v0.1** — 借鉴 Hermes + Voyager + SKILL.md
2. **Honcho-inspired Peer Representation v0.1** — 借鉴 honcho
3. **reef-inspired Harness Adapter** — 借鉴 reef 生产级 harness

### 中期 (主人拍板后)
4. **Claude Code Architecture Review** — 读 2604.14228, 借鉴架构
5. **Codex CLI Architecture Review** — 看 96.5% Rust 怎么组织

### 长期
6. **DGM 自证明机制** — Phase 6+ 真安全 self-improve

---

_楚零 2026-07-20 16:42_
_主人 16:38 真调研完成, 5 大借鉴项目已识别_
_Honcho = 主人 12:14 哲学的真生产实现_