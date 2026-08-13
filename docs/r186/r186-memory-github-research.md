# R186 GitHub 优秀项目调研 — memory 模块 (长期记忆)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R186
> **日期**: 2026-08-13
> **范围**: apeireth-memory 35 文件 + lightmemo 14 子文件 + dailynote 6 子文件
> **状态**: 调研为升级预备.

---

## 0. 现状

apeireth-memory **35 文件, ~300KB+**:

### 核心 (15 文件)
- lib.rs (21KB) — 入口
- episode.rs (13KB) — episode 记忆
- session_note.rs (23KB) — session 笔记
- identity.rs (20KB) — 身份记忆
- user_profile.rs (16KB) — 用户画像
- three_layer.rs (20KB) — 三层记忆模型
- streams.rs (20KB) — 流式记忆
- semantic_persist.rs (25KB) — 语义持久化
- semantic.rs (12KB) — 语义检索
- append_only.rs (12KB) — append-only 日志
- history_streams.rs (6KB) — 历史流
- continuity_link.rs (6KB) — 连续性链接
- migrations.rs (13KB) — 迁移
- llm_analysis.rs (3KB) — LLM 增强分析
- g5_memory_bridge.rs (9KB) — g5 桥

### lightmemo (14 文件, L1-L4 四层)
- L1: l1_file.rs (3KB) — 文件层
- L2: l2_vector.rs (3KB) — 向量层
- L3: l3_tag.rs (2KB) — 标签层
- L4: l4_lcm.rs (3KB) — LCM 层 (Latent Context Model)
- decay.rs (2KB) — 衰减
- dream.rs (2KB) — 梦境 (类似 VCP 凌晨整理)
- sleep_cycle.rs (4KB) — 睡眠周期
- librarian.rs (3KB) — 图书管理员 (自动归档?)
- manager.rs (5KB) — 管理层
- pipe.rs (5KB) — 管道
- search.rs (3KB) — 搜索
- adapter.rs (5KB) — 适配器
- mcp.rs (6KB) — MCP 桥
- enhanced.rs / compat.rs / mod.rs

### dailynote (6 文件)
- 日记系统: store / note / search / export / enhanced / compat / mcp / mod

**已实现能力**:
- 4 层记忆 (L1 file / L2 vector / L3 tag / L4 LCM)
- 三层记忆模型
- 衰减 + 梦境 + 睡眠周期 (类似 VCP 凌晨整理)
- Session / Episode / Identity / UserProfile 4 类
- append-only + 迁移
- MCP 桥 (memory 是 MCP server)

**已经领先很多项目**:
- L1-L4 四层比 MemGPT/Letta 二层 (core + archival) 细
- 梦境 + 睡眠周期设计有 VCP 凌晨整理的影子
- 衰减 + librarian 自动归档
- g5 桥接

---

## 1. LLM 长期记忆 SOTA

### 1.1 Letta (letta-ai/letta) — **RECOMMENDED 学习**

- **GitHub**: https://github.com/letta-ai/letta
- **Stars**: 17K+ (前 MemGPT)
- **License**: Apache 2.0
- **定位**: 状态化 LLM agent + 分层记忆
- **核心能力**:
  - Core memory (in-context) + Archival memory (out-of-context) + Recall memory
  - Self-editing memory (agent 决定何时写什么)
  - 多 agent 编排
  - Tool calling + 消息队列
  - REST API + Python SDK
- **关键设计**:
  - MemGPT 论文: \"Operating System for LLM\" 比喻
  - 分页机制: 记忆像虚拟内存分页
  - Agent loop: 用户消息 -> 工具调用 -> 记忆更新 -> 回复

**为什么必须学**:
- 长期记忆 SOTA, 学术 + 工业双验证
- 我们 L1-L4 比它细, 但自我编辑机制可借鉴
- **多 agent 编排** 直接借鉴到我们 council

**借鉴方案**:
`
ust
// apeireth-memory/src/self_edit.rs
pub trait MemorySelfEdit: Send + Sync {
    async fn should_edit(&self, context: &Context) -> Result<EditDecision, Error>;
    async fn apply_edit(&self, decision: EditDecision) -> Result<(), Error>;
}

// apeireth-memory/src/paging.rs
pub struct MemoryPaging {
    core: Arc<CoreMemory>,
    archival: Arc<ArchivalMemory>,
    page_size: usize,
}

impl MemoryPaging {
    pub async fn page_in(&self, query: &str) -> Result<Vec<MemoryPage>, Error>;
    pub async fn page_out(&self, page: &MemoryPage) -> Result<(), Error>;
}
`

### 1.2 Mem0 (mem0ai/mem0) — **学习**

- **Stars**: 33K+
- **License**: Apache 2.0
- **定位**: Self-improving memory layer for AI
- **核心能力**:
  - LLM 自动从对话提取记忆
  - 向量检索 + LLM 过滤
  - 多用户隔离
  - 自我遗忘 (衰减)

**学习点**: 我们的 llm_analysis.rs 已经类似, 可强化

### 1.3 Cognee (topoteretes/cognee) — **学习**

- **Stars**: 8K+
- **License**: Apache 2.0
- **定位**: Knowledge graph + vector + LLM 整合
- **核心能力**:
  - ECL (Extract, Cognify, Load) pipeline
  - Knowledge graph 自动构建
  - 时序推理
  - 多模态

**学习点**: 知识图谱 + 向量混合检索

### 1.4 Zep (getzep/zep) — **学习**

- **Stars**: 7K+
- **License**: Apache 2.0
- **定位**: Long-term memory for AI assistants (生产级)
- **核心能力**:
  - Fact extraction
  - Episode summarization
  - 时序推理
  - Graphiti (知识图谱)
  - 企业级 (Slack/Discord/Notion 集成)

**学习点**: 时序推理 + 企业级集成模式

### 1.5 Supermemory (supermemoryai/supermemory) — 学习

- **License**: AGPL-3.0
- **定位**: Infinite memory for AI apps
- **不集成**: AGPL

### 1.6 Memvid (OvidijusParmas/memvid) — **学习 (轻量)**

- **License**: MIT
- **定位**: Video-based memory, MP4 存储向量
- **独特**: 把向量编码到视频帧, 压缩比高
- **价值**: 我们的 lightmemo L2 长期存储可借鉴

### 1.7 LangMem (langchain-ai/langmem) — 学习

- **License**: MIT
- **定位**: LangChain 的长期记忆模块
- **学习点**: 整合到 LangGraph 的 memory pattern

---

## 2. 向量 / 嵌入基础设施

### 2.1 Qdrant (qdrant/qdrant) — **RECOMMENDED 备选**

- 28K+, Apache 2.0
- 我们 L2 向量层
- 高性能 + payload filter

### 2.2 Milvus (milvus-io/milvus) — 学习

- 33K+, Apache 2.0
- 大规模场景, 工业级
- 不集成 (Go/C++ 重)

### 2.3 Weaviate (weaviate/weaviate) — 学习

- 14K+, BSD-3
- 内置 vectorization
- 不集成 (Go 重)

### 2.4 LanceDB (lancedb/lance) — **学习 (Rust 原生)**

- 6K+, Apache 2.0
- Rust 实现的列存 + 向量
- **价值**: 我们 L2 可以用 lance 列存
- **性能**: 比 Qdrant 快 10x (列式扫描)

### 2.5 HNSW 索引实现 (hnswlib / vqcode) — 备选

- 我们可能已经在用 hnsw_rs

---

## 3. 知识图谱 (与 R182 互补)

### 3.1 Graphiti (getzep/graphiti) — **RECOMMENDED**

- **License**: Apache 2.0
- **定位**: 时序知识图谱 for AI
- **核心能力**:
  - 时序边 (边带时间戳)
  - 实体提取 + 关系推理
  - 与 LLM agent 整合
  - Zep 团队出品
- **学习点**: 时序图谱
- **集成方案**: 我们 relation crate (R182 调研) 可以借鉴 Graphiti

### 3.2 Neo4j (再) — 不集成

### 3.3 Memgraph (再) — 不集成

---

## 4. 自我编辑 / 反思 SOTA

### 4.1 Reflexion (noahshinn/reflexion) — 学习

- 论文: \"Reflexion: Language Agents with Verbal Reinforcement Learning\"
- **学习点**: 自我反思 + 短期反思 vs 长期反思
- **价值**: 我们 lightmemo dream.rs 已经类似, 可强化

### 4.2 Voyager (MineDojo/Voyager) — 学习

- Minecraft 终身学习 agent
- **学习点**: skill library 累积
- **价值**: 我们的 librarian.rs 可以借鉴 skill 库

### 4.3 Generative Agents (Stanford) — 学习

- 论文: \"Generative Agents: Interactive Simulacra of Human Behavior\"
- **学习点**: 记忆流 + 反思 + 规划
- **价值**: 我们的 lightmemo 设计哲学完全契合

### 4.4 MemGPT 论文 (Packer et al. 2023) — 必读

- \"MemGPT: Towards LLMs as Operating Systems\"
- 我们 lightmemo 已经是 MemGPT 风格
- **学习点**: 操作系统比喻 + 分页 + 工具调用

---

## 5. 衰减 / 遗忘机制 SOTA

### 5.1 Ebbinghaus Forgetting Curve — 经典

- 1885 心理学经典
- 我们 decay.rs 已经在用类似
- **学习点**: R = e^(-t/S) 公式

### 5.2 ACT-R (Adaptive Control of Thought—Rational) — 学术

- 认知科学, 记忆激活衰减模型
- **学习点**: 基础激活 + 关联激活 + 衰减

### 5.3 MemoryBank (zhongshsh/MemoryBank) — 学习

- Ebbinghaus + 神经科学启发的 LLM 记忆
- **价值**: 我们 decay 可以借鉴更精细的激活模型

---

## 6. 升级方案 (R186+ 实施)

### 6.1 短期 (1-2 days)

1. **Self-edit 机制**: 借鉴 Letta, agent 决定何时写什么
2. **Vector LanceDB 评估**: 我们 L2 升级到 lance 列存
3. **Graphiti 时序图谱**: 评估, 加进 relation crate

### 6.2 中期 (3-5 days)

4. **Mem0 风格自动提取**: 强化 llm_analysis
5. **Cognee ECL pipeline**: 整合到 three_layer
6. **时序推理**: 借鉴 Zep, 增强 streams.rs
7. **Reflexion 风格反思**: 强化 dream.rs

### 6.3 长期 (持续)

8. **Voyager 风格 skill 库**: librarian.rs 升级
9. **Generative Agents 风格反思 + 规划**: 整合进 council
10. **Memvid 视频压缩存储**: 长期归档备选

---

## 7. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| lancedb (Rust 绑定) | ~5MB | Apache 2.0 | 短期 |
| graphiti (评估) | ~0 (Python 评估) | Apache 2.0 | 中期 |
| (其余均为借鉴, 不引入) | | | |

**总增加**: 短期 ~5MB (lancedb), 中期 0 (评估型)

---

## 8. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| lightmemo (L1-L4) | 本模块核心, 升级目标 |
| relation (R182) | Graphiti 风格时序图谱 |
| cognition | 记忆为 cognition 提供上下文 |
| council | 记忆为 council deliberation 提供历史 |
| tui | dailynote 显示 + 编辑 |
| consciousness | identity 记忆是 consciousness 基础 |

---

## 9. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- memory 公开 API: 0 改 (新能力在 lightmemo 子模块内)

---

## 10. 参考链接

- Letta (MemGPT): https://github.com/letta-ai/letta
- MemGPT 论文: https://arxiv.org/abs/2310.08560
- Mem0: https://github.com/mem0ai/mem0
- Cognee: https://github.com/topoteretes/cognee
- Zep: https://github.com/getzep/zep
- Graphiti: https://github.com/getzep/graphiti
- Supermemory: https://github.com/supermemoryai/supermemory
- Memvid: https://github.com/OvidijusParmas/memvid
- LangMem: https://github.com/langchain-ai/langmem
- Qdrant: https://github.com/qdrant/qdrant
- Milvus: https://github.com/milvus-io/milvus
- Weaviate: https://github.com/weaviate/weaviate
- LanceDB: https://github.com/lancedb/lance
- Reflexion: https://github.com/noahshinn/reflexion
- Voyager: https://github.com/MineDojo/Voyager
- Generative Agents: https://github.com/joonspk-research/generative_agents
- MemoryBank: https://github.com/zhongshsh/MemoryBank
- MemGPT 论文: https://arxiv.org/abs/2310.08560