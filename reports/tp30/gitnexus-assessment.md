# GitNexus 评估（**实测评估**，源码在 research/source/GitNexus/）

## 机制（What it does）

- 核心功能：把任意 codebase 索引成知识图谱（依赖/调用链/集群/执行流），通过 MCP tools 暴露给 AI agent
- 解决什么问题：AI agent 写代码时「看不到依赖、改一处破一片」→ GitNexus 让 agent 看到完整的代码架构关系
- 关键技术（**实测来源**：`research/source/GitNexus/ARCHITECTURE.md` + `gitnexus/src/`）：

| 组件 | 文件位置 | 作用 |
|---|---|---|
| **Ingestion pipeline** | `gitnexus/src/core/run-analyze.ts` + `pipeline.ts` | 15-phase DAG 流程，构建 `KnowledgeGraph` |
| **Graph store** | `gitnexus/src/core/lbug/` + `ladybug-db` | LadybugDB（图数据库，类 Neo4j）持久化到 `.gitnexus/` |
| **Query layer** | `gitnexus/src/core/search/` (`bm25-index.ts` / `hybrid-search.ts` / `fts-indexes.ts`) | BM25 + 向量混合检索 + Cypher ad-hoc |
| **MCP server** | `gitnexus/src/mcp/server.ts` + `tools.ts` | stdio + HTTP 两种 transport |
| **Embeddings** | `gitnexus/src/core/embeddings/embedder.ts` + `hf-env.ts` | HuggingFace embeddings |
| **Wiki generator** | `gitnexus/src/core/wiki/generator.ts` + `graph-queries.ts` | 自动生成代码 wiki（Mermaid 图） |
| **Staleness** | `gitnexus/src/core/git-staleness.ts` + `index-freshness.ts` | 增量更新检测（lastCommit 对比） |
| **Web UI** | `gitnexus-web/`（Vite + React） | 只读 thin client，通过 HTTP API |

**17 个 MCP tools** 实测（ARCHITECTURE.md §MCP tools）：
- `list_repos` / `query` / `cypher` / `context` / `impact` / `detect_changes` / `rename` / `api_impact` / `trace` / `route_map` / `tool_map` / `shape_check` / `explain` / `pdg_query` / `group_list` / `group_sync`
- 关键差异化：`query`/`context`/`impact` 支持 `repo: "@<group>"` group-aware 跨仓库操作

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-graph`（图编排套件）
  - `apeireth-graph-primitive`（图基元）
  - `apeireth-tool-codesearch`（代码搜索工具）
  - `apeireth-context-fold`（上下文折叠）
  - `apeireth-mcp`（V2 战区 5 MCP skeleton）
  - 缺失：APEIRETH 无 BM25 + 向量混合检索 + LadybugDB 类图存储
- 差异化优势：
  - GitNexus 是「codebase 知识图谱 + agent context」，APEIRETH `apeireth-graph` 是「运行时图编排」，场景错位
  - GitNexus 支持 CJK 分词（`cjk-segmentation.ts`），APEIRETH 当前检索默认英文
  - GitNexus 的 `route_map` / `tool_map` / `shape_check` 是「agent 写代码」专用，APEIRETH 当前不做 agent 写代码工具集成
- 可借鉴（**高价值**）：
  - **BM25 + 向量混合检索**：GitNexus 的 `hybrid-search.ts` + `bm25-index.ts` 是 APEIRETH `apeireth-tool-search` (R145) 应该吸收的模式（当前 R145 是「VSearch + aggregate + TF-IDF-like score」，可升级为真正的 BM25 + 向量）
  - **CJK 分词器**：`cjk-segmentation.ts` 在 APEIRETH 主人用中文场景下必备（主人日记/记忆以中文为主）
  - **Staleness 检测**：`git-staleness.ts` 用 lastCommit 对比 → 可迁移到 `apeireth-wiki` 的 `rebuild_index()`（当前 wiki 重建时机靠外部触发，可加自动 staleness 检测）
  - **Wiki 自动生成**：`core/wiki/generator.ts` 用 LLM + graph 生成代码 wiki → 可与 APEIRETH 主人 W3 设计意图（世界模型）结合，但 W3 已在 TP32 走 W2 因果路径，wiki 自动生成不重复
  - **`impact`/`trace` blast radius**：可借鉴到 `apeireth-action` 的「事务影响评估」（当前只有 rollback，未有事前 impact）

## 吸收建议（Action items）

- P0 立即做：**不动 GitNexus 本体**。GitNexus 是 Node.js + LadybugDB 栈，与 APEIRETH Rust 主力不兼容，直接 fork 不划算。
- P1 评估后做（按价值排序）：
  1. **CJK 分词器移植到 `apeireth-tool-search`**：参考 `research/source/GitNexus/gitnexus/src/core/search/cjk-segmentation.ts` 的算法，在 Rust 端用 `tantivy` + `jieba-rs` 重写（既有 `research/source/tantivy/` 可参考）
  2. **混合检索（BM25 + 向量）升级**：参考 `hybrid-search.ts` 设计，扩展 `apeireth-tool-search` 的 R145 算法
- P2 长期调研：
  3. **`impact` blast radius 模式**：参考 GitNexus 的 `core/graph/import-cycles.ts`（import 循环检测），加到 `apeireth-action` 的事前 impact 评估
  4. **Staleness 检测自动触发**：把 GitNexus 的 staleness 模式应用到 `apeireth-wiki`，让 wiki 在 git commit 后自动 rebuild
- 不做（重复 / 价值低）：
  - LadybugDB 图数据库迁移（APEIRETH 已有 Kùzu 图数据库，见 backlog "明确不做：图数据库自研"）
  - Wiki 自动生成（APEIRETH W3/W2 已在做主人个性化因果图，不重复 codebase wiki）
  - 17 个 MCP tools 全部照搬（APEIRETH `apeireth-mcp` 已有自己的工具集）

## 0 装 PASS 标注

- 真用：**部分是**（仅借鉴模式，不 fork 代码）
- 源：**已实测**（`research/source/GitNexus/` 源码 + `ARCHITECTURE.md` + `gitnexus/src/` 实读）
- 实测深度：ARCHITECTURE.md 全文 + src/ 目录结构 + 关键文件 `hybrid-search.ts` / `mcp/tools.ts` / `core/wiki/generator.ts` / `cjk-segmentation.ts` 文件名级确认
- 未实测：实际代码行级 review（17 个 MCP tools 的具体参数 schema、`embedder.ts` 的具体 model 选型）
- 风险：GitNexus License = PolyForm Noncommercial（非商业），如借鉴代码片段需注意 license 边界（建议仅借鉴算法思路，不复制代码段）