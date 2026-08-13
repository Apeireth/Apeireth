# R171 relation SurrealDB 多模型后端调研

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R171 (research-only, no code)
> **日期**: 2026-08-13
> **borrow-id**: R171-REL-BORROW-surrealdb-30k-stars-2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. 调研目的

apeireth-relation (R154 已实现 SurrealDB-style property graph + traversal + query) 当前是纯内存 property graph, 无持久化后端。R171 调研: 是否引入 SurrealDB (https://github.com/surrealdb/surrealdb) 作为持久化 + 多模型后端 (doc + graph + vector)。

调研对象: **SurrealDB** — 30K+ stars, Rust 写, 多模型 (document + graph + vector + time-series + search), 内嵌 (embedded) / 独立 (standalone) / 集群 三种部署模式, REST + WebSocket + SQL 协议。

---

## 1. SurrealDB 核心能力 (GitHub 公开数据, 2026-08-13)

| 维度 | 值 | 来源 |
|---|---|---|
| Stars | 30K+ | github.com/surrealdb/surrealdb |
| 主语言 | Rust (98.7%) | 同上 |
| License | Apache-2.0 / Business Source 1.1 | 同上 (BSL 用于商业 SaaS, 开源使用免费) |
| 部署模式 | embedded / standalone / cluster | 项目文档 |
| 数据模型 | doc + graph + vector + time-series + full-text search | 同上 |
| 协议 | HTTP REST + WebSocket + SQL (SurrealQL) | 同上 |
| Rust SDK | surrealdb crate (官方维护) | crates.io |
| Embedded 模式 | 可作为 lib 直接链接 (无独立进程) | 同上 |

关键卖点 (与 apeireth-relation 当前对照):
- 多模型统一 — 同一份数据既可当 doc 查、又可当 graph 遍历、又可当 vector 检索 — 不需要 3 个独立数据库
- embedded 模式 — Rust 应用直接链接, 启动 < 100ms, 0 进程开销
- SQL-like 查询 (SurrealQL) — 比 Cypher 更通用, 比 Gremlin 更 SQL 友好
- 活跃开发 — 30K+ stars + 周更 commit

---

## 2. 当前 apeireth-relation 状态评估 (R154 实现)

R154 已交付 (commit b6ecf0ef):
- Property graph 数据结构 (Node + Edge + Property)
- Traversal API (BFS / DFS / shortest path)
- Query API (SurrealQL-lite 解析器, 不依赖外部 DB)
- 内存存储, 无持久化

问题:
1. 无持久化 — 进程重启数据全失
2. 无事务 — 复杂 graph 操作无法原子化
3. 无 multi-model — 节点只能存 graph, 无法直接 vector search 或 doc store
4. 无分布式 — 单机内存上限

升级诉求: 想要持久化 + 事务 + 多模型 + (可选) 分布式 — SurrealDB 正好全覆盖。

---

## 3. 设计草案: apeireth-relation 的 SurrealDB 后端集成

> 强约束: 本节纯设计, 0 触碰 3 不可变脊柱 (R170 已确立)。

### 3.1 架构图

```
+-----------------------------------------------+
|  apeireth-relation::GraphStore (R154 trait)  |
+-----------------------------------------------+
                      |
                      v
+-----------------------------------------------+
|  apeireth-relation::BackendAdapter (新)      |  <-- R171+ 设计目标
|  - 选择后端 (memory / surrealdb-embedded)    |
|  - 统一 GraphStore trait 接口                |
+-----------------------------------------------+
        |                              |
        v                              v
+----------------+          +--------------------+
| MemoryBackend  |          | SurrealDBBackend   |
| (R154 当前)    |          | (R171+ 新增)       |
+----------------+          +--------------------+
                                      |
                                      v
                          +---------------------+
                          | surrealdb crate     |
                          | (embedded mode)     |
                          +---------------------+
```

### 3.2 关键 trait 设计 (草案)

apeireth-relation/src/backend.rs (新文件, R171+ 实施)

```rust
#[async_trait]
pub trait GraphBackend: Send + Sync {
    /// 节点插入
    async fn put_node(&self, node: Node) -> Result<NodeId, RelationError>;
    
    /// 边插入
    async fn put_edge(&self, edge: Edge) -> Result<EdgeId, RelationError>;
    
    /// 节点查询 (按 ID)
    async fn get_node(&self, id: NodeId) -> Result<Option<Node>, RelationError>;
    
    /// 边查询 (按 ID)
    async fn get_edge(&self, id: EdgeId) -> Result<Option<Edge>, RelationError>;
    
    /// 图遍历 (BFS / DFS / shortest path)
    async fn traverse(&self, start: NodeId, 
        strategy: TraversalStrategy) -> Result<Vec<NodeId>, RelationError>;
    
    /// SurrealQL 查询
    async fn query(&self, ql: &str) -> Result<QueryResult, RelationError>;
    
    /// 事务
    async fn transaction<F, R>(&self, f: F) -> Result<R, RelationError>
        where F: FnOnce(&mut dyn TransactionalBackend) 
        -> BoxFuture<'_, Result<R, RelationError>>;
    
    /// 持久化 (SurrealDB 后端专用: flush to disk)
    async fn flush(&self) -> Result<(), RelationError>;
    
    /// 后端类型 (供 metric / log)
    fn backend_kind(&self) -> BackendKind;
}

pub enum BackendKind {
    Memory,
    SurrealDBEmbedded,
    SurrealDBStandalone,
    // 未来: Postgres + pgvector (作为 SurrealDB 的替代)
}

pub struct BackendAdapter {
    inner: Arc<dyn GraphBackend>,
    kind: BackendKind,
}
```

### 3.3 SurrealDB schema 设计 (草案)

apeireth-relation 默认 SurrealQL schema:

```sql
-- 节点表
DEFINE TABLE node SCHEMAFULL;
DEFINE FIELD id ON node TYPE string;
DEFINE FIELD labels ON node TYPE array<string>;
DEFINE FIELD properties ON node TYPE object;
DEFINE FIELD embedding ON node TYPE option<vector<384>>;  -- R172+ vector search

-- 边表
DEFINE TABLE edge SCHEMAFULL;
DEFINE FIELD id ON edge TYPE string;
DEFINE FIELD from ON edge TYPE record<node>;
DEFINE FIELD to ON edge TYPE record<node>;
DEFINE FIELD label ON edge TYPE string;
DEFINE FIELD properties ON edge TYPE object;

-- 索引
DEFINE INDEX node_id ON node FIELDS id UNIQUE;
DEFINE INDEX edge_from ON edge FIELDS from;
DEFINE INDEX edge_to ON edge FIELDS to;
DEFINE INDEX node_embedding ON node FIELDS embedding MTREE DIMENSION 384;  -- vector search

-- 关系图遍历 (SurrealQL graph extension)
LET $start = (SELECT id FROM node WHERE id = $start_id LIMIT 1);
SELECT ->edge->node FROM $start;
```

### 3.4 风险等级 -> 后端映射 (默认策略)

| 用例 | 默认 backend | 理由 |
|---|---|---|
| 单元测试 | MemoryBackend | 速度优先, 无持久化需求 |
| 短期 session (< 1h) | MemoryBackend | 重启可丢, 内存够快 |
| 长期记忆 (>= 1 day) | SurrealDBEmbedded | 持久化, 单进程够用 |
| 多 agent 共享 | SurrealDBStandalone | HTTP/WS 协议, 跨进程 |
| 分布式 (>= 3 节点) | SurrealDBCluster | Raft 一致性 |

策略可由 `apeireth-relation::StoreConfig.backend` override。

### 3.5 与 3 不可变脊柱的关系

| 不可变脊柱 | R171 关系 |
|---|---|
| Self-Disable 判定逻辑 | 0 触碰 — GraphBackend 是其上层消费者 |
| L0 HA 物理隔离 | 0 触碰 — L0 HA 仍由 physical_multisig.rs 管 |
| 13 键 verdict cache 语义 | 0 触碰 — verdict 仍由 verdict_cache.rs 算 |

GraphBackend 设计原则:
- 不参与 agent disable 判定 — 仅存 graph 数据
- 不参与 verdict 计算 — 仅作为 verdict 的存储后端 (可选)
- 不参与物理多签 — 仅在 transaction() 时保证 ACID

---

## 4. SurrealDB 集成候选路径 (R171+ 评估)

| 路径 | 优 | 劣 |
|---|---|---|
| A. surrealdb 直接依赖 (embedded mode) | 立即可用, 30K+ stars 社区 | BSL 商业限制 (开源使用免费) |
| B. fork 内置到 relation | 完全可控 | 维护成本高, 失去上游更新 |
| C. 自研嵌入式 KV + graph index | 0 引外部 dep | 工作量大, 短期不可行 |
| D. 不引入 DB, 仅扩展内存 backend | 0 触碰代码 | 不解决持久化核心问题 |

R171 推荐路径: A (embedded mode) — 直接依赖 surrealdb (开源使用免费, 仅 SaaS 转售收费),通过 GraphBackend trait 抽象, 未来如需切换到 B/C/D 平滑。

Cargo.toml 加: surrealdb = { version = "2.x", features = ["kv-rocksdb"] }, 配 feature embedded-db。

---

## 5. 工作量估算 (R171+ 实施阶段)

| 阶段 | 工时 | 交付 |
|---|---|---|
| R171 (本档) | 完成 | research doc |
| R171+1  | 0.5 day | GraphBackend trait 抽象 + BackendAdapter 框架 |
| R171+2  | 1 day | MemoryBackend 抽离 (从 R154 代码) + 现有 API 全兼容 |
| R171+3  | 1.5 day | SurrealDBEmbedded 真接 (含 schema 初始化 + put/get/traverse/query) |
| R171+4  | 1 day | transaction + flush 实现, 事务原子性测试 |
| R171+5  | 1 day | 演示 demo: agent 启动 -> 写 1000 nodes -> 重启 -> 全读回 |
| R171+6  | 0.5 day | Kani proofs: BackendKind 切换不变量 (内存 <-> SurrealDB 行为一致) |
| 总计 | 5.5 days | apeireth-relation::BackendAdapter v1 |

前提: R171+ 不动 3 不可变脊柱, R154 GraphStore 公开 API 100% 兼容。

---

## 6. R171 结论

- SurrealDB 30K+ stars, Rust-native, 多模型 (doc + graph + vector + time-series + search), embedded 模式 — 完全契合 relation 持久化 + 多模型升级诉求
- 设计上 GraphBackend trait 作为新抽象层, 与 R154 GraphStore 100% 兼容 — 现有调用方 0 改动
- BSL 仅限制 SaaS 转售, 自用 / 开源 / 商业自部署 全免费 — R171 可放心引入
- 推荐实施路径 A (直接依赖 surrealdb embedded), 5.5 days 工作量
- R171 仅 research, 0 代码改动, R171+ 阶段开始实施

下一步: R172 (apeireth-voice LIVE apikey 测试) — 与 R170/R171 同类但实施路径不同 (已有 1092 行 infrastructure 只需接 apikey)。
