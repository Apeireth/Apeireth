# mempalace vs apeireth-memory — 对比 + 借鉴清单

> **作者**: Codex 后端工程师
> **日期**: 2026-08-15
> **触发**: R178 后端完工 + 主人终极授权 "继续做 mempalace 对比 + 后端审计"
> **基线**: `research/source/mempalace/` HEAD (Python 3.10+ / Chroma 主仓 / 5 backend) + `crates/apeireth-memory/src/` 23 文件 (R37-2)

---

## 1. 一句话总结

mempalace 走 **Python + Chroma/5 backend + 4 层渐进加载 + 实体走廊（hallway）**；apeireth-memory 走 **Rust + SQLite + 6 历史流 + 三层 facade**。两者 **目标都是"长程记忆 + 上下文工程"**，但形态完全不同。**可借鉴 6 处**（embedder 身份校验 / normalize 版本 / 走廊图 / dedup / 原子写 / 知识图谱时序边），**不应照搬 3 处**（Chroma 依赖 / 远程 vector DB / MCP stdio 协议）。

---

## 2. mempalace 核心架构（30 秒读懂）

| 概念 | 含义 | 类比到 apeireth |
|---|---|---|
| **Palace** | 一座宫殿 = 一个项目根目录（`~/.mempalace/<project>`） | 一个 apeireth workspace |
| **Wing** | 翼 = 大分类（`diary` / `letters` / `ideas` / `code`） | 大致对应 `crate` 边界 |
| **Room** | 房间 = 翼内子主题 | 接近 `mod` |
| **Drawer** | 抽屉 = 一段 chunk（最小存储单位） | **= apeireth 的 Episode / Note / IdentityCard** |
| **Hallway** | 走廊 = **同一 wing 内** 实体之间的共现连接 | 跟 `continuity_link` 部分重叠 |
| **Tunnel** | 隧道 = **跨 wing** 房间连接 | 跟 `graph-primitive` 部分重叠 |
| **Identity** | `~/.mempalace/identity.txt` 纯文本，always-loaded | 跟 `IdentityCard` 思路接近 |
| **Knowledge Graph** | SQLite + 时序三元组（valid_from → valid_to） | 接近 `graph-primitive` + `continuity_link` |

### 2.1 4 层记忆栈（**最值得借鉴**）

```
Layer 0  Identity          ~100 tokens   always-loaded   ← 启动就吃
Layer 1  Essential Story   ~500-800      always-loaded   ← 自动从高频 drawer 压缩
Layer 2  On-Demand         ~200-500/次   by topic/wing   ← 按需取
Layer 3  Deep Search       无限          全 Chroma 检索  ← fallback
```

**核心收益**: wake-up 只花 **~600-900 tokens**，给主对话留 **95%+ 上下文**。

### 2.2 后端（5 选 1 + 抽象层）

| Backend | 文件大小 | 用途 | apeireth 对应 |
|---|---|---|---|
| `sqlite_exact` | 43 KB | 本地精确向量 + BM25 词法 | **最接近 InMemory + File** |
| `chroma` | 106 KB | 默认 / ChromaDB | 没有（依赖重） |
| `milvus` | 53 KB | 分布式 | 没有（按 stub） |
| `pgvector` | 67 KB | Postgres + pgvector | 没有 |
| `qdrant` | 57 KB | Qdrant | 没有（按 stub） |

### 2.3 走廊 / 隧道（hallway / tunnel）

```python
# 同 wing 内: 实体共现 → 走廊
if "Aya" and "Lumi" 出现在 47 个 drawers:
    → 生成 hallway (Aya, Lumi, weight=47, co_docs=...)
# 跨 wing: 走廊聚合 → 隧道
# tunnels.json 在 ~/.mempalace/<project>/
```

文件是 `JSON` + 原子写（temp-file + `os.replace`）。

---

## 3. apeireth-memory 现状（30 秒读懂）

`crates/apeireth-memory/src/` 23 文件，分两大子树：

| 子树 | 文件数 | 职责 |
|---|---|---|
| `lightmemo/` | 14 | 三层 facade + L1~L4 分层 + librarian + dream + mcp |
| `dailynote/` | 6 | 笔记存储 + 导出 + 搜索 + mcp |

**核心数据类型** (per `lib.rs` 头注释):
- **6 Append-only 历史流**: 思想/提案/行动/关系/演化/反思期 + StanceStream/GoalStream/LifeStream/MigrationStream (R22 ST-A2.4 扩展为 10)
- **`identity_cards`**: `continuity_id` UNIQUE 跨载体去重
- **`episodes`**: 按 `session_id` / 时间范围 / `continuity_id` 索引
- **`ThreeLayerMemory`**: R30 U9 加的 facade
- **3 Provider** (从 `apeireth_memory_extensions` 透明 re-export): InMemory / File / MongoDb
- **`HashEmbedder` + `SemanticIndex`**: 语义搜索（hash embedder 是降级路径）

**禁区** (per `lib.rs` 头注释):
- ❌ 不改 `apeireth-core` 已实装类型签名
- ❌ 不引入 ORM
- ❌ 不碰 R11 baseline 三值
- ❌ 不碰 `apeireth-legacy/`

---

## 4. 详细对比表

| 维度 | mempalace | apeireth-memory | 谁优 / 借鉴点 |
|---|---|---|---|
| **语言** | Python 3.10+ | Rust (no_std-friendly) | apeireth 优（性能 + 无 GC） |
| **存储** | ChromaDB (默认) / SQLite / Milvus / pgvector / Qdrant | SQLite + 文件 | **mempalace 抽象层更通用** |
| **Append-only** | 否（直接 delete） | **是**（triggers raise ABORT） | **apeireth 优**（更严格） |
| **schema 迁移** | 无显式版本，靠 `_normalize_version` 提示重 mine | `MIGRATIONS` 数组 + `MIGRATIONS` const | **apeireth 优**（更工程化） |
| **Embedder 身份校验** | **是**（RFC 001 — 记录 model_name/dim，mismatch 直接 raise） | ❌ **没有** — 换 HashEmbedder 时静默降级 | **强烈借鉴** |
| **Schema 版本化** | `NORMALIZE_VERSION` const，bump 强制重 mine | 没有 normalize 版本 | **借鉴** |
| **去重** | **是**（cosine < 0.15 + greedy keep-longest，scoped by source_file） | ❌ **没有** — `semantic_persist` 没有 dedup 步骤 | **强烈借鉴** |
| **走廊 / 隧道** | **是**（hallway 同 wing + tunnel 跨 wing，JSON 持久化） | 部分 (`continuity_link` 仅连接，无 weight/co_occurrence) | **借鉴**（weight + 共现统计） |
| **时序三元组** | **是**（KG: valid_from → valid_to + invalidate） | ❌（`graph-primitive` 无时序） | **借鉴**（Zep-killer 卖点） |
| **4 层渐进加载** | **是**（L0/L1/L2/L3，wake-up ~700 tokens） | **类似但不同**：L1_file / L2_vector / L3_tag / L4_lcm 是**存储分层**，不是**加载分层** | **借鉴**（加 L0/L1 always-loaded 设计） |
| **原子写** | **是**（temp-file + `os.replace`，POSIX 0600） | SQLite 自带 WAL；文件层 lightmemo 不原子 | **借鉴**（lightmemo 文件层加 atomic write） |
| **BM25 词法** | **是**（sqlite_exact 内置 BM25 + cosine） | 部分（`search.rs` 有但不一定 BM25） | **借鉴**（hybrid search） |
| **Embedder 警告** | **是**（legacy 未记录 embedder 的 collection 启动时 warn） | ❌ | **借鉴** |
| **MCP server** | **是**（230KB `mcp_server.py`，stdio） | **是**（`lightmemo/mcp.rs` + `dailynote/mcp.rs`） | 平手 |
| **i18n** | **是**（16 语言 JSON） | 有但分散 | mempalace 集中，apeireth 散落 |

---

## 5. 强烈推荐借鉴（6 项）

### 5.1 Embedder 身份校验（RFC 001 移植）

**为什么**: 当前 `HashEmbedder` → 真 embedder 切换时，旧 collection 静默降级到 hash 向量，**用户察觉不到但召回质量崩坏**。

**怎么做**:
```rust
// 在 SemanticIndex / PersistentSemanticIndex 加一个 metadata 表
// embedder_identity(model_name: String, dimension: usize, recorded_at: i64)
// 启动时 read → check → 不匹配 raise EmbedderMismatchError
// 老 collection (无记录) → warn 但不 fail (per mempalace 同款)
```

**工作量**: 2 天（含 5 个 cargo test + 1 Kani proof）

### 5.2 Normalize 版本 schema

**为什么**: `semantic.rs` 改 chunk 策略后，旧 chunk 不会被识别为 stale，导致混合新旧 normalize 规则的向量。

**怎么做**:
```rust
pub const SEMANTIC_NORMALIZE_VERSION: u32 = 1;
// PersistentSemanticIndex 加 normalize_version 列
// open 时: stored < current → warn + 触发后台 re-embed
```

**工作量**: 1 天

### 5.3 Dedup 模块

**为什么**: 同一 session 多次写入会产生近重复 Episode/Note，semantic search 召回噪声大。

**怎么做**:
```rust
// 新 mod: src/semantic/dedup.rs
// 默认 cosine distance threshold = 0.15 (mempalace 默认)
// greedy: 按 doc 长度降序，已保留集合里距离 < threshold 就 skip
// 暴露成 maintenance CLI: `apeireth-memory dedup [--dry-run] [--threshold 0.15]`
```

**工作量**: 2 天（含 dry-run 模式 + 10 个 cargo test）

### 5.4 走廊（hallway）结构

**为什么**: `continuity_link` 只记录"两个 ID 之间有连接"，没有 weight/co_occurrence 量化强度。

**怎么做**:
```rust
// 新 mod: src/hallway.rs
// 数据: Vec<Hallway { entity_a, entity_b, weight, co_doc_count, wing, last_seen } }
// 计算: 扫 Episode/Note，按 entity 集合做 pairwise co-occurrence
// 持久化: JSON 文件 ~/.apeireth/hallway.json (atomic write)
// API: query_entity(name) → Vec<Hallway>
```

**工作量**: 3 天（含 JSON atomic write + 8 个 cargo test）

### 5.5 时序知识图谱三元组

**为什么**: 这是 mempalace 跟 Zep 的核心差异化卖点，apeireth `graph-primitive` 完全没有时序边。

**怎么做**:
```rust
// 在 graph-primitive 加: TemporalEdge { from, to, kind, valid_from, valid_to }
// 三元组表: CREATE TABLE temporal_triple (subject, predicate, object, valid_from, valid_to, ended_at)
// API: kg.add_triple / kg.invalidate / kg.query_entity("X", as_of="2026-01-01")
// 锁: 不改 LOCKED 9 文件, 仅在 graph-primitive 加 mod temporal
```

**工作量**: 1 周（含 query language + 15 cargo test + 2 Kani proof）

### 5.6 4 层渐进加载（awareness 层）

**为什么**: 当前 `ThreeLayerMemory` 是**存储分层**（hot/warm/cold file），不是**加载分层**（always-loaded vs on-demand）。wake-up 没机制。

**怎么做**:
```rust
// 新 mod: src/awareness.rs
// L0 IdentityCard → 启动就吃 (~100 tokens)
// L1 Essential Story → 从高 weight Episode 自动压缩 (~500-800 tokens)
// L2 On-Demand → 按 topic/wing 取
// L3 Deep Search → semantic search
// 类似 mempalace layers.py
```

**工作量**: 3 天（含 L1 自动压缩算法 + 6 cargo test）

---

## 6. 不建议照搬（3 项）

### 6.1 ChromaDB 默认 backend

ChromaDB Python 依赖重（10+ MB wheels + 启动 daemon），Rust 端要么走 HTTP 要么自己实现协议。**不值得** — 当前 SQLite + 后续如需 ANN 走 usearch 即可。

### 6.2 远程 vector DB（Milvus / pgvector / Qdrant）

按主人"非云优先"原则，**先 usearch + sqlite_exact 风格自研**，远程 backend 列为 P3。

### 6.3 MCP stdio 全协议

mempalace 230KB `mcp_server.py` 是全功能 MCP server (mine/search/dedup/sync/sweeper/...)。

apeireth 已经有 `lightmemo/mcp.rs` 和 `dailynote/mcp.rs`，**两条 MCP 路径**。**统一为单入口 + command 路由**，不要照搬全命令集。

---

## 7. 总工作量估算

| 借鉴项 | 工作量 | 优先级 | 落地 R |
|---|---|---|---|
| 5.1 Embedder 身份校验 | 2 天 | **P0** | R179 |
| 5.2 Normalize 版本 | 1 天 | **P0** | R179 |
| 5.3 Dedup 模块 | 2 天 | **P1** | R180 |
| 5.4 走廊结构 | 3 天 | **P1** | R180 |
| 5.5 时序知识图谱 | 1 周 | **P2** | R181 |
| 5.6 4 层渐进加载 | 3 天 | **P1** | R180 |

合计: **~3 周**，分 3 个 R 周期落地。

---

## 8. GitHub 元信息 + 实战观察（2026-08-15 补充）

> 来源: github.com/MemPalace/mempalace + 4 个公开 issue + WebSearch 二次验证
> 目的: 把 1-7 架构对比锚定到活的社区信号上，避免纸上谈兵

### 8.1 仓库元信息

| 项 | 值 | 备注 |
|---|---|---|
| GitHub org | **MemPalace**（大小写敏感，不是 mempalace） | 抓 GitHub API 必须用对大小写，否则 404 |
| Stars / Forks | **7,423 / 280** | 健康的中型 OSS |
| Open issues | **360** | 远高于 forks 比，说明社区在用且卡住 |
| Commits (过去 12mo) | **57,532** | 平均 ~158 commit/天 — 极高活跃度（对比 Linux kernel ~50/天） |
| License | **MIT**, Copyright 2026 MemPalace Contributors | 友好，可借鉴 |
| 联合创始人 | **Milla Jovovich**（生化危机 Alice 演员跨界 AI） | 营销叙事 vs 工程实力分开看 |
| 主仓语言 | Python 3.10+ (97.4%) + TypeScript (1.8%) + Rust (0.3%) | Rust 已入侵 |

### 8.2 官方主推卖点（用于交叉验证 2.1 4 层栈）

- **96.6% R@5 raw** on LongMemEval — 零 API 调用
  - 含义: 在长程记忆基准上不调任何 LLM API，端到端命中前 5 的概率 96.6%
  - 印证 2.1「4 层渐进加载」是真有效的（Layer 0/1 不需检索就命中高频）
  - 对 apeireth: 可作为 apeireth-memory 自测基准（非 LongMemEval 全集，做 100-query 抽样即可）
- **零 lock-in** — 5 backend 可热切换，数据可导出

### 8.3 社区痛点（来自 4 个高赞 issue）

| Issue | 标题 | 性质 | apeireth 启发 |
|---|---|---|---|
| **#1669** | 提议新增 turbovecdb（轻量级独立 vector db lib） | **社区需求** | 印证 2.2 不绑死 Chroma 是正确选择；可列为 P3 探索 |
| **#1681** | HNSW 冷加载 60s/call 性能塌方 | **性能 bug** | 远程 vector DB 冷启动成本不可忽视；6.2 非云优先判断更稳 |
| **#1676** | MCP server module-level mkdir 失败 | **初始化 bug** | apeireth-lightmemo/mcp.rs 初始化必须幂等；增加 mkdir -p 等价 |
| **#1674** | Diary write 返回 opaque -32000 错误 | **错误可读性** | apeireth 所有持久化错误必须带 path + errno + operation 三元组 |

### 8.4 结论：对我们 4 项核心决策的更新

1. 非云优先 + 自研 sqlite_exact 风格：得到 #1681 的二次验证
2. backend 抽象层 + InMemory 真接：得到 #1669 turbovecdb 提议的间接支持（社区也想摆脱 Chroma）
3. 错误信息必须可读：#1674 opaque -32000 给我们立反面教材
4. MCP 入口要幂等：#1676 mkdir bug 提醒

1-7 的判断与社区信号一致，**不需要修正**。可推进 9 决策点。

## 9. 决策点（等主人拍板）

1. **借鉴范围**: 全部 6 项 vs 只 P0+P1（5 项）vs 只 5.1+5.2（P0 2 项 = 3 天）？
2. **落地节奏**: 一次性 3 周 vs 拆 3 个 R 周期？
3. **mempalace 继续研究**: 当前只读了 ~30% 代码，是否要继续深读 `palace.py` 全文件 + `entity_detector.py` 33KB？

---

_作者: Codex 后端工程师_
_基线: R178 完工 + mempalace HEAD (2026-08-15)_
_对比深度: 浅 — 仅覆盖架构层，feature parity 评估待续_
