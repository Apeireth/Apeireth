# 阶段 2 决策 — 需要源码借鉴的项目清单

> **范围**: 用户 2026-07-30 指示 "哪些项目值得借鉴的需要源码的把名字告诉我, 我把源码下下来"
> **依据**: research/ 调研 (149 文件) + docs/stage2-decisions-*.md (14 决策) + 阶段 1 inspiration (1023 行)

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-source-projects-list.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **目的** | 用户下源码 → 真实代码细节 → 阶段 3 时配合阶段 2 + 附录使用 |

---

## 1. P0 必须下源码（核心借鉴，10 个）

### 1.1 Lumio-Research/hermes-agent-rs ⭐⭐⭐

**真实数据**: 110K+ Rust lines, 1,428 tests, 17 crates, ~16MB binary
**URL**: https://github.com/Lumio-Research/hermes-agent-rs
**借鉴什么**:
- 17-crate workspace 完整结构 (阶段 2 §3 设计参考)
- actor / 工具 / 平台 / LLM providers 实际代码
- memory system 8 backends 实现
- self-evolution engine (multi-armed bandit)
**阶段 2 引用**: §3 crate 划分 + §6 持久化 + §7 LLM 集成 + §10 智囊团

### 1.2 NousResearch/hermes-agent ⭐⭐⭐

**真实数据**: Hermes Agent Python 原版 (被 Rust port)
**URL**: https://github.com/NousResearch/hermes-agent
**借鉴什么**:
- 17 platforms (Telegram/Discord/Slack/...) 真实实现
- 30+ tools 真实代码
- 8 memory backends 真实代码
- v2026.4.13 reference baseline
**阶段 2 引用**: §9 通信总线 + §8 模块化

### 1.3 openclaw/openclaw ⭐⭐⭐

**真实数据**: Personal AI Assistant (🦞 EXFOLIATE!)
**URL**: https://github.com/openclaw/openclaw
**借鉴什么**:
- Gateway 模式 (单长生命周期进程)
- DM access security 默认沙箱
- 多 Node 接入 (Telegram/Discord/iOS/Android/macOS)
- Agent workspace + skills
- Operator quick refs
**阶段 2 引用**: §9 通信总线 (L4 WebSocket) + §15 借鉴

### 1.4 visioncortex/vcptoolbox (VCP 工具箱) ⭐⭐⭐

**真实数据**: VCP 联想网络 / 浪潮语义 (灵感 §12.3 源头)
**URL**: https://github.com/visioncortex/vcptoolbox (含 compound_eye)
**借鉴什么**:
- VCP 联想网络实际代码
- 河道能量 + 神经网络信号传播
- Point cloud + cluster 算法
**阶段 2 引用**: §6 持久化 (Phase 3 VCP 浪潮自研)

### 1.5 thedotmack/claude-mem ⭐⭐

**真实数据**: 3-layer progressive disclosure + 5 lifecycle hooks
**URL**: https://github.com/thedotmack/claude-mem
**借鉴什么**:
- 3-layer workflow (10x token 节省)
- M 层方法论 promotion 管道
- Lifecycle hooks 实际实现
**阶段 2 引用**: §6 持久化 + §10 智囊团反思

### 1.6 quickwit-oss/tantivy ⭐⭐

**真实数据**: Lucene 风格, BM25, 多语言 tokenizer
**URL**: https://github.com/quickwit-oss/tantivy
**借鉴什么**:
- 实际 API 用法
- BM25 scoring 实现
- Mmap directory 模式
**阶段 2 引用**: §6 持久化 (Tantivy 全文)

### 1.7 qdrant/qdrant ⭐⭐

**真实数据**: HNSW vector index (vector search)
**URL**: https://github.com/qdrant/qdrant
**借鉴什么**:
- HNSW 索引实现 (VCP 浪潮自研可借鉴)
- vector search 算法
**阶段 2 引用**: §6 持久化 (Qdrant 向量)

### 1.8 spacejam/sled ⭐⭐

**真实数据**: 嵌入式 KV, ACID 事务, watch_prefix
**URL**: https://github.com/spacejam/sled
**借鉴什么**:
- watch_prefix 实现 (阶段 2 §9 事件流)
- transaction 模式
**阶段 2 引用**: §6 持久化 (sled KV)

### 1.9 bytecodealliance/wasmtime ⭐

**真实数据**: WASM 沙箱运行时
**URL**: https://github.com/bytecodealliance/wasmtime
**借鉴什么**:
- WASM 沙箱隔离 API
- 资源限制 + capability-based security
**阶段 2 引用**: §8 模块化 (WASM 沙箱)

### 1.10 tokio-rs/tokio ⭐

**真实数据**: Rust 异步 runtime 工业标准
**URL**: https://github.com/tokio-rs/tokio
**借鉴什么**:
- multi-thread runtime 内部
- work-stealing scheduler
**阶段 2 引用**: §4 进程/线程/协程

---

## 2. P1 应该下源码（次要借鉴，10 个）

### 2.1 agentmemory (rohitg00/agentmemory) ⭐

**真实数据**: LLM Wiki + confidence (阶段 1 §13 候选)
**URL**: https://github.com/rohitg00/agentmemory
**借鉴什么**: A 层经验沉淀实现

### 2.2 Graphify (safishamsi/graphify) ⭐

**真实数据**: 知识图谱 + EXTRACTED/INFERRED (阶段 1 §13 候选)
**URL**: https://github.com/safishamsi/graphify
**借鉴什么**: 知识图谱实际实现

### 2.3 MemPalace ⭐

**真实数据**: 宫殿式 (wings/rooms/drawers) 物理化记忆 (阶段 1 §13 候选)
**URL**: https://github.com/MemPalace (可能多个, 见 research)
**借鉴什么**: 记忆物理化模型

### 2.4 Anthropic/claude-code ⭐

**真实数据**: claude-code CLI
**URL**: https://github.com/anthropics/claude-code
**借鉴什么**: CLI 设计 + slash commands

### 2.5 Anthropic/skills ⭐

**真实数据**: Skills 系统
**URL**: https://github.com/anthropics/skills
**借鉴什么**: 技能加载/管理/版本

### 2.6 Honcho ⭐

**真实数据**: AI agent 平台
**URL**: https://github.com/honcho
**借鉴什么**: 多 agent 协同

### 2.7 OpenHands (All-Hands-AI) ⭐

**真实数据**: AI agent 平台
**URL**: https://github.com/All-Hands-AI/OpenHands
**借鉴什么**: Agent 执行模型

### 2.8 MetaGPT (geekan/MetaGPT) ⭐

**真实数据**: 多 agent 协作框架
**URL**: https://github.com/geekan/MetaGPT
**借鉴什么**: 多 agent role + SOP

### 2.9 AladdinPaladin/launcher (claude-code launcher) ⭐

**真实数据**: codex / claude-code launcher 模式
**URL**: https://github.com/launcher (可能多个)
**借鉴什么**: CLI launcher 模式

### 2.10 openai/codex ⭐

**真实数据**: OpenAI Codex CLI
**URL**: https://github.com/openai/codex
**借鉴什么**: CLI + LLM 集成模式

---

## 3. P2 可选下源码（特定借鉴，10 个）

### 3.1 MemoryOS-Rust (TelivANT) — STM/MTM/LTM tier_manager

**URL**: https://github.com/TelivANT/memoryos-rust (推断)
**借鉴什么**: 9-crate workspace + 六边形架构

### 3.2 DeltaMemory — WAL + CRC32 + salience decay

**URL**: https://www.deltamemory.com/blog/ (blog post)
**借鉴什么**: WAL 实现 + 衰减公式

### 3.3 arXiv 2501.13956 (Zep paper) — Temporal KG

**URL**: https://arxiv.org/abs/2501.13956
**借鉴什么**: Temporal KG 架构 (阶段 2 §6 浪潮)

### 3.4 arXiv 2602.11443 等其他 7 篇

**借鉴什么**: AI agent / memory / reasoning 学术前沿

### 3.5 ComposioHQ/composio — MCP server

**URL**: https://github.com/ComposioHQ/composio
**借鉴什么**: MCP 工具集成

### 3.6 abhigyanpatwari/GitNexus — 代码检索 MCP

**URL**: https://github.com/abhigyanpatwari/GitNexus
**借鉴什么**: 代码检索 + 知识图谱

### 3.7 deusdata/codebase-memory-mcp — 代码库记忆

**URL**: https://github.com/deusdata/codebase-memory-mcp
**借鉴什么**: 代码库结构化记忆

### 3.8 microsoft/playwright-mcp — 浏览器自动化

**URL**: https://github.com/microsoft/playwright-mcp
**借鉴什么**: 浏览器 MCP 集成

### 3.9 Tavily-AI/tavily-mcp — 搜索 MCP

**URL**: https://github.com/Tavily-AI/tavily-mcp
**借鉴什么**: 网络搜索 MCP

### 3.10 system-prompts-ai-tools — 真实 system prompt 集合

**URL**: https://github.com/x1xhlol/system-prompts-ai-tools (推断)
**借鉴什么**: 阶段 2 §10 智囊团 7 persona system prompt 模板

---

## 4. 当前 R11 已落 (无需下, 看现有)

### 4.1 crates/ (9 个 R11 已落占位)

```
Apeireth-rust/crates/
├── apeireth-asi/
├── apeireth-bench/
├── apeireth-cli/
├── apeireth-core/
├── apeireth-memory/
├── apeireth-philosophy/
├── apeireth-pybridge/
├── apeireth-test/
└── apeireth-tools/
```

### 4.2 Cargo.toml + Cargo.lock + rust-toolchain.toml

```
Apeireth-rust/Cargo.toml           # workspace
Apeireth-rust/Cargo.lock           # R11 LOCKED
Apeireth-rust/rust-toolchain.toml  # Rust 1.80
```

### 4.3 research/08-rust-substrate-current/ (之前 R14 设计)

```
research/08-rust-substrate-current/
├── Cargo.toml           # 6 crates 设计 (core/ports/adapters/gateway/py/cli)
├── crates/              # 占位
├── data/                # 数据
├── gateway.log          # 运行日志
├── gateway2.log         # 第二实例
└── README.md            # 借鉴来源表
```

⚠️ **重要**：08-rust-substrate-current 是**之前 R14 设计** (6 crates), 与当前 Apeireth-rust/ (9 crates) **结构不同**。需要**融合决策**：保留哪一部分，迁移哪一部分。

---

## 5. 下源码优先级建议

```
如果你时间有限, 必下:
  P0 #1-5 (Lumio hermes-agent-rs, NousResearch hermes-agent, openclaw, vcptoolbox, claude-mem)
  
时间充裕下:
  P0 #6-10 (tantivy, qdrant, sled, wasmtime, tokio)
  
更深下:
  P1 #1-10 (agentmemory, Graphify, MemPalace, claude-code, skills, honcho, OpenHands, MetaGPT, codex, ...)
  
特定需要:
  P2 #1-10 (按阶段 3 实际需要再下)
```

---

## 6. 用户下一步

```
1. 选哪些源码下 (按优先级)
2. 下到哪个目录: research/source/ (新建)
3. 下完后告诉团队, 我们继续看真实代码
```

ponytail 标记：**149 文件调研 + 30 个候选借鉴项目**，**P0 必下 10 个已列**。**附录会持续更新**到阶段 3。

---

_主哲学 anchor 6 个全贯穿. 真实借鉴清单已沉淀. 下一步: 用户下源码后, 我们逐个深读真实代码, 补齐附录 §3 待抓项._