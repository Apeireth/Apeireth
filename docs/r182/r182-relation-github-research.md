# R182 GitHub 优秀项目调研 — relation (graph DB / 多模型 DB) 模块

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R182
> **日期**: 2026-08-13
> **范围**: apeireth-relation 升级路径 + SurrealDB 真接方案
> **状态**: 调研为升级预备.

---

## 0. 现状

apeireth-relation 65KB, 4 文件:
- lib.rs (16KB) — 4 类关系 (共生 / 协调 / 嵌入 / 与自身) + RelationKind 枚举
- graph.rs (19KB) — property-graph 存储 + adjacency list 索引
- query.rs (13KB) — predicate filter
- traversal.rs (15KB) — BFS/DFS / shortest path

**已借鉴 (in lib.rs 注释)**: SurrealDB (RELATE / -> 箭头) / Neo4j / Memgraph (BFS/DFS 语义) / Cypher (MATCH pattern 灵感)

**当前 in-memory** — 0 持久化, 0 真实 DB backend. R175 路线图: 5.5 days SurrealDB 真接.

---

## 1. 专用 Graph DB

### 1.1 Neo4j (neo4j/neo4j) — **行业标杆学习**

- **GitHub**: https://github.com/neo4j/neo4j (只读, 闭源核心)
- **License**: GPL-3.0 (社区版)
- **定位**: Property graph 标杆
- **核心能力**:
  - Cypher query language
  - 数十亿节点/边 scale
  - ACID 事务
  - 丰富图算法 (中心性 / 路径 / 社区发现)
  - Bolt 协议

**为什么必须学**:
- Cypher 是事实标准查询语言
- 我们的 query.rs predicate filter 可以借鉴 Cypher pattern 风格

**不集成 (GPL-3.0 + 闭源 + Java)** — 仅学设计

### 1.2 Memgraph (memgraph/memgraph) — **学习**

- **License**: BSL (Business Source License) -> Apache 2.0 after 3 years
- **定位**: in-memory graph DB, C++ 实现
- **核心能力**:
  - 兼容 Bolt + Cypher
  - 实时流处理
  - 动态图算法
- **学习点**: streaming graph algorithms

**不集成 (BSL)** — 仅学设计

### 1.3 Kùzu (kuzudb/kuzu) — **RECOMMENDED 嵌入**

- **GitHub**: https://github.com/kuzudb/kuzu
- **License**: MIT
- **定位**: Embedded graph DB, C++ 实现
- **核心能力**:
  - 嵌入式 (类似 SQLite, link 到进程)
  - Cypher 兼容
  - 列式存储 + 向量化查询
  - 性能 SOTA (LDBC SNB benchmark)
  - 10K+ stars, 活跃
- **Rust binding**: 官方 (kuzu crate) + 社区维护

**为什么推荐 (备选 SurrealDB)**:
- 嵌入式 0 服务, 单 binary 部署
- Cypher 全兼容
- License 友好 (MIT)
- 性能 SOTA

**集成草案**:
`
ust
// apeireth-relation/src/backend/kuzu.rs
use kuzu::{Database, Connection};

pub struct KuzuBackend {
    db: Database,
    conn: Connection,
}

impl GraphBackend for KuzuBackend {
    async fn connect(path: &Path) -> Result<Self, BackendError> {
        let db = Database::new(path)?;
        let conn = Connection::new(&db)?;
        Ok(Self { db, conn })
    }
    async fn query(&self, cypher: &str) -> Result<Vec<Row>, BackendError> {
        let mut result = self.conn.query(cypher)?;
        Ok(result.collect())
    }
}
`

**优先级**: 与 SurrealDB 并列, 选哪个看最终场景:
- SurrealDB: 多模型 (relational + document + graph), 单 binary 服务
- Kùzu: 纯图, 嵌入式, 性能极致

### 1.4 Cozo (cozodb/cozo) — **学习 (Datalog 路线)**

- **GitHub**: https://github.com/cozodb/cozo
- **License**: Apache 2.0
- **定位**: Datalog-based graph DB, Rust 实现
- **核心能力**:
  - Datalog 查询 (递归友好)
  - 嵌入式 + 服务两种模式
  - 图算法 library
  - 时间旅行 (历史回溯)
  - Rust 一等公民

**为什么必须看**:
- 唯一 Rust 实现的图 DB (其他主流是 C++/Java)
- Datalog 比 Cypher 更适合 recursive queries
- 时间旅行设计 → 我们 deliberation 历史可借鉴

**集成方案**:
- 短期: 学习 Datalog 思想, 不集成
- 长期: 如果我们 RelationKind 扩展到 5+ 类, Datalog 表达力比 Cypher 强, 考虑换

### 1.5 SurrealDB (surrealdb/surrealdb) — **R171 调研, 真接候选**

- **GitHub**: https://github.com/surrealdb/surrealdb
- **Stars**: 28K+
- **License**: BSL (Business Source License) -> Apache 2.0 after 4 years
- **定位**: Multi-model DB (relational + document + graph + time-series + vector)
- **核心能力**:
  - 单 binary 服务 (embedded 模式也有)
  - SurrealQL (类 SQL + graph pattern)
  - 实时订阅 (LIVE queries)
  - 权限系统内置
  - Rust 客户端 (surrealdb crate)
  - 嵌入式 (R2024+) — 单进程

**为什么真接候选**:
- 已经 R171 调研过, 决定为首选 backend
- 嵌入式模式 (R2024+) 可以替代 Kùzu
- License 即将转 Apache 2.0 (1-2 年内)

**集成方案 (R171+1~6 实施)**:
- Week 1: embedded mode 集成 (替代 Kùzu 候选)
- Week 2: RelationKind -> SurrealQL schema
- Week 3: query.rs 适配 SurrealQL
- Week 4: LIVE queries 集成 (council deliberation 实时推送)
- Week 5: 性能调优 + 内存回收
- Week 6: Kani proofs 验证图不变量

### 1.6 Apache AGE (apache/age) — 学习

- **License**: Apache 2.0
- **定位**: PostgreSQL 扩展, 图能力
- **不集成**: 引入 PG 依赖过重

---

## 2. Rust 生态图库 (in-memory)

### 2.1 petgraph (petgraph/petgraph) — **RECOMMENDED 当前**

- **Stars**: 3.3K+
- **License**: MIT/Apache-2.0
- **定位**: Rust 数据结构库, graph 算法 SOTA
- **核心能力**:
  - Graph / DiGraph / GraphMap
  - BFS / DFS / Dijkstra / A* / Bellman-Ford
  - 拓扑排序 / 强连通分量
  - 多种 dot 输出
- **现状**: 我们 graph.rs / traversal.rs 大概率已用 petgraph

**强化方向**:
- 升级到最新 petgraph (0.7+) 拿到 stable 算法 API
- 加 dot 序列化用于 TUI 可视化

### 2.2 graph-rs / ego-tree / pathfinding — 备选

- 各有侧重, 不如 petgraph 完整

### 2.3 nalgebra (nalgebra) — 互补 (张量)

- 图嵌入 (node2vec / graphSAGE) 需要线性代数
- 我们如果做关系网络嵌入, nalgebra 是基础

---

## 3. 向量 / 嵌入视角 (memory + relation 交叉)

### 3.1 Qdrant (qdrant/qdrant) — 学习

- **License**: Apache 2.0
- **定位**: 向量 DB, 也支持 payload filtering (近 graph)
- **学习点**: HNSW 索引 + payload filter
- **价值**: memory 嵌入索引可以借鉴

### 3.2 pgvector (pgvector/pgvector) — 学习

- PostgreSQL 扩展
- **不集成**: 不依赖 PG

### 3.3 surrealdb (再次) — **同时有 vector 索引**

- 我们如果走 SurrealDB 一站式, 关系网络 + 嵌入 + vector 都覆盖

---

## 4. 图算法 (networkx / igraph 类比)

### 4.1 rustworkx (Qiskit/rustworkx) — 学习

- **License**: Apache 2.0
- **定位**: 高性能 Rust 图算法 (Qiskit 用)
- **学习点**: 大规模图算法 (PageRank / 中心性)

### 4.2 graph — Rust NetworkX 等价

- 不如 petgraph 成熟

---

## 5. 升级方案 (最终阶段执行)

### 5.1 短期 (1-2 days) — R182+1

1. **真接 SurrealDB embedded 模式** (替代 in-memory, 单 binary)
2. **RelationKind -> SurrealQL schema** (建表 DDL)
3. **query.rs 适配 SurrealQL** (predicate -> SurrealQL)
4. **LIVE queries 集成** (council deliberation 实时推送)

### 5.2 中期 (3-5 days) — R182+2

5. **Kuzu 备选 backend** (如果 SurrealDB BSL 风险触发, 切换)
6. **Cozo Datalog 评估** (如果 RelationKind 扩展到 5+ 类)
7. **图可视化** (TUI 集成 petgraph dot 输出)

### 5.3 长期 (持续) — R182+

8. **Node2Vec / GraphSAGE** (关系网络嵌入, 进 memory)
9. **图算法** (中心性 / 社区发现 / 路径)
10. **时间旅行** (借鉴 Cozo, 记录 deliberation 历史可回放)

---

## 6. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| petgraph (当前在用) | ~0 | MIT/Apache-2.0 | 是 |
| surrealdb (embedded) | ~10MB 编译 | BSL -> Apache 2.0 | 中期 |
| kuzu (备选) | ~5MB 编译 | MIT | 备选 |
| cozo (备选) | ~3MB 编译 | Apache 2.0 | 备选 |

**总增加**: ~10MB 编译产物 (surrealdb), 0 外部 service 依赖 (embedded 模式)

---

## 7. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| memory (R146) | 关系网络可作为 memory 索引 |
| council (R180) | deliberation 历史用 relation 存储 + LIVE queries 推送 |
| consciousness (R84) | SelfRelation 是 4 类之一, 自我关系建模 |
| tool-browser (R179) | 独立 |
| pipeline | relation 决策树可作为 pipeline step |

---

## 8. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- relation 公开 API: 0 改 (新 backend 在 trait impl 内)

---

## 9. 参考链接

- Neo4j: https://github.com/neo4j/neo4j
- Memgraph: https://github.com/memgraph/memgraph
- Kùzu: https://github.com/kuzudb/kuzu
- Cozo: https://github.com/cozodb/cozo
- SurrealDB: https://github.com/surrealdb/surrealdb
- Apache AGE: https://github.com/apache/age
- petgraph: https://github.com/petgraph/petgraph
- nalgebra: https://github.com/dimforge/nalgebra
- Qdrant: https://github.com/qdrant/qdrant
- pgvector: https://github.com/pgvector/pgvector
- rustworkx: https://github.com/Qiskit/rustworkx
- LDBC SNB: https://ldbcouncil.org/ldbc/snb/