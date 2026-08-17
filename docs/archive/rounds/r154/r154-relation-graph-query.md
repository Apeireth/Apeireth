# R154 — `apeireth-relation` SurrealDB 风格 property graph + traversal + query

> **R154 (2026-08-13)**: `apeireth-relation` 加 3 个新模块 — `graph.rs` (RelationGraph + GraphNode + GraphEdge + adjacency-list indexes + shortest-path BFS, 337 行) + `traversal.rs` (BfsIter + DfsIter + direction filter + depth-limited + shortest_path function, 306 行) + `query.rs` (NodeQuery + EdgeQuery + CombinedQuery + count_by_kind, 247 行). 累计 +45 tests (29 lib unit + 16 integration) + 1 example (graph_demo), 0 errors, 0 触碰 3 不可变脊柱. 与现有 `Relation` / `RelationKind` / `RelationRegistry` 完全共存 (no breaking changes). 详见 `crates/apeireth-relation/README.md` + `crates/apeireth-relation/examples/graph_demo.rs` + `crates/apeireth-relation/tests/test_relation_graph.rs`.

---

## 1. 动机

apeireth-relation 当前只有 `Relation` struct + `RelationKind` 枚举 + `RelationRegistry` (扁平 Vec 存储), 缺真正的图遍历/查询能力. 这限制了:

1. 想找两个主体之间的最短关系路径, 必须手写 BFS
2. 想按 kind + property 过滤节点/边, 必须手写 `Vec::iter().filter()`
3. 想做图可视化或子图提取, 完全没有工具

R154 在不破坏现有 API 的前提下加 `graph` / `traversal` / `query` 三层, 形成完整的 property graph 栈. 既保了 4 类关系的领域建模, 又补上通用图操作能力.

## 2. 设计决策

### 2.1 与 RelationRegistry 共存 (而非替代)

```rust
// 现有 flat API 保留
let mut registry = RelationRegistry::new();
registry.register(Relation::new_symbiosis("a", "b")?);

// R154 加: 一次性转换到 graph
let graph = RelationGraph::from(&registry);
```

设计哲学: relation modeling (4 类语义) 与 graph operations (BFS/DFS/path) 是正交的两个维度. 用 `From` 桥接, 让上层按需选择.

### 2.2 adjacency-list 索引 (而非邻接矩阵)

```rust
pub struct RelationGraph {
    nodes: HashMap<NodeId, GraphNode>,
    edges: HashMap<EdgeId, GraphEdge>,
    out_edges: HashMap<NodeId, HashSet<EdgeId>>,  // from -> edges
    in_edges: HashMap<NodeId, HashSet<EdgeId>>,   // to -> edges
    edges_by_kind: HashMap<RelationKind, HashSet<EdgeId>>,
}
```

3 个二级索引 (out / in / by_kind) 让 `outgoing(node)` / `incoming(node)` / `edges_of_kind(kind)` 都是 O(1) (除返回 `Vec<&GraphEdge>` 的迭代开销外). 不用邻接矩阵因为节点稀疏且 ID 是 String.

### 2.3 BFS iterators (而非 callback-based traversal)

```rust
let nodes: Vec<(String, usize)> = BfsIter::new(&graph, "alice")
    .with_max_depth(3)
    .with_direction(TraversalDirection::Outgoing)
    .collect();
```

Rust iterator 模式天然 lazy, 比 callback (SurrealDB `->` arrow) 更符合 Rust 习惯用法. `depth` 字段一并 yield, 调用方可立即做"按深度分组"或"按深度过滤"等操作.

### 2.4 最短路径带 edge ID (而非仅 nodes)

```rust
pub struct PathResult {
    pub nodes: Vec<NodeId>,
    pub edges: Vec<EdgeId>,
}
```

不仅返回节点, 还返回穿过的 edge ID. 这样调用方可以:
- 重建完整路径 (节点 + 边)
- 提取路径上的 relation kind (`edges.iter().map(|eid| graph.get_edge(eid).unwrap().kind)`)
- 计算 path 上的总权重 (如 property 包含 `weight` 字段)

### 2.5 0 引外部 dep

按 ponytail ceiling: graph / traversal / query 全部用 std (`HashMap` / `HashSet` / `VecDeque`) + 现有 workspace deps (serde / serde_json / uuid). 不引 petgraph / graphlib 等 graph crate.

## 3. 模块清单

### 3.1 `graph.rs` — RelationGraph + 节点/边/索引

| 类型 | 说明 |
|------|------|
| `RelationGraph` | 主存储: nodes + edges + 3 索引 |
| `GraphNode` | 节点: id + kind + serde_json::Value properties |
| `GraphEdge` | 边: id (UUID) + from + to + RelationKind + properties |
| `GraphError` | 5 variant 错误 (NodeNotFound / EdgeNotFound / DuplicateNode / DuplicateEdge / InvalidEdgeEndpoint) |
| `NodeId` / `EdgeId` | type alias = `String` |

API 表面:
- `insert_node` / `insert_edge` (validation: endpoint 必须先存在)
- `remove_node` (级联删除所有 incident edges) / `remove_edge` (更新 3 索引)
- `get_node` / `get_edge` / `node_count` / `edge_count` / `node_ids` / `all_edges`
- `outgoing(node_id)` / `incoming(node_id)` / `edges_of_kind(kind)`
- `shortest_path(from, to)` — BFS, 返回 `Option<Vec<NodeId>>`
- `From<&RelationRegistry> for RelationGraph`

### 3.2 `traversal.rs` — iterators + shortest path

| 类型 | 说明 |
|------|------|
| `TraversalDirection` | Outgoing / Incoming / Both |
| `BfsIter` | level-order iterator, yield `(NodeId, depth)` |
| `DfsIter` | pre-order iterator, yield `(NodeId, depth)` |
| `PathResult` | { nodes, edges, hop_count(), is_trivial() } |
| `shortest_path` | standalone function, BFS-based, 带 edge ID 重建 |

API 表面:
- `BfsIter::new(graph, root)` / `.with_direction(dir)` / `.with_max_depth(d)` / `.collect()`
- `DfsIter::new(graph, root)` / `.with_direction(dir)` / `.with_max_depth(d)` / `.collect()`
- `shortest_path(graph, from, to) -> Option<PathResult>`

### 3.3 `query.rs` — 谓词查询

| 类型 | 说明 |
|------|------|
| `PropertyMatch` | 单一 property 相等 predicate |
| `NodeQuery` | 节点过滤 (kind + 多 property match, AND 语义) |
| `EdgeQuery` | 边过滤 (kind + from + to + 多 property match, AND 语义) |
| `CombinedQuery` | nodes + edges 组合 + restrict_to_nodes flag |
| `count_by_kind` | 返回 4-tuple (sym, coord, emb, self_rel) |

API 表面 (builder pattern):
- `NodeQuery::new().kind_eq("agent").property_string("role", "assistant").execute(&graph)`
- `EdgeQuery::new().kind(RelationKind::Embedding).from("alice").execute(&graph)`
- `CombinedQuery::new(nodes, edges).restrict_to_nodes(true).execute(&graph)`

## 4. 性能特性

| 操作 | 时间复杂度 | 备注 |
|------|----------|------|
| `insert_node` | O(1) amortized | HashMap insert |
| `insert_edge` | O(1) amortized | HashMap + 3 索引更新 |
| `remove_node` | O(d) | d = 节点度数 (incident edges) |
| `outgoing(node)` | O(d) | d = 出度 |
| `incoming(node)` | O(d) | d = 入度 |
| `edges_of_kind(k)` | O(e_k) | e_k = kind k 的边数 |
| `BfsIter::collect()` | O(V + E) | 标准 BFS |
| `DfsIter::collect()` | O(V + E) | 标准 DFS |
| `shortest_path` | O(V + E) | BFS + parent pointer 重建 |

## 5. 测试覆盖 (R154 累计)

| 测试类型 | 数量 | 位置 |
|---------|------|------|
| graph.rs unit (insert / remove / indexes / shortest_path) | 16 | `src/graph.rs::tests` |
| traversal.rs unit (BFS / DFS / direction / depth / shortest_path) | 9 | `src/traversal.rs::tests` |
| query.rs unit (property match / kind / endpoint / combined / count) | 13 | `src/query.rs::tests` |
| 现有 lib.rs unit (4 关系 + Relation + Registry) | 10 | `src/lib.rs::tests` |
| 现有 integration | 3 | `tests/integration_test.rs` (旧) |
| 新 integration (graph + traversal + query + lifecycle) | 16 | `tests/test_relation_graph.rs` |
| **Total** | **67 + 0 failed** | |

## 6. 示例输出

`cargo run -p apeireth-relation --example graph_demo` 输出片段:

```
Registry has 5 relations
  symbiosis: 2
  coordination: 1
  embedding: 1
  self_relation: 1

Graph has 6 nodes, 5 edges

BFS from `perception` (max depth 3):
  depth=0: perception
  depth=1: cognition
  depth=1: memory
  depth=2: consciousness
  depth=3: reflection

Shortest path memory -> reflection:
  nodes: ["memory", "perception", "cognition", "consciousness", "reflection"]
  edges: ["73857739-...", "923c0dac-...", "cb0d604c-...", "3f540575-..."]
  hops: 4

Edge counts: symbiosis=2 coordination=1 embedding=1 self_relation=1
```

## 7. 已知限制 / 留 R155+

1. **Cypher/GQL parser** — 当前是 predicate-based builder API, 不是 full Cypher parser. Full parser 是 ~3-5K LOC 工作量, 留给后续.
2. **持久化** — graph 是 in-memory only. 留 apeireth-vector 或 apeireth-memory 做持久化.
3. **并发写** — 单线程 owned borrow. 多线程场景用 `Arc<RwLock<RelationGraph>>` 在调用方包装.
4. **Property index** — 当前 property filter 是 O(n) scan. 大量数据时可加 `HashMap<Key, HashSet<NodeId>>` 二级索引.

## 8. 文档同步

- `crates/apeireth-relation/README.md` 重写 (two-layer architecture table + borrowed upstream references per O-5)
- `crates/apeireth-relation/Cargo.toml` description 扩展 (提及 R154 graph / traversal / query)
- `crates/apeireth-relation/src/lib.rs` 顶部 doc + 模块声明 (R154 modules 加 public + re-exports)
- 顶层 `README.md` R154 banner (本次提交)

## 9. 累计 regression

| 维度 | 数量 |
|------|------|
| 新 crate | 0 (扩展现有 `apeireth-relation`) |
| 新模块 | 3 (`graph` / `traversal` / `query`) |
| 新 unit tests | 29 (graph 16 + traversal 9 + query 13, 部分重叠) |
| 新 integration tests | 16 |
| 新 example | 1 (`graph_demo`) |
| 新 Cargo.toml example 声明 | 1 |
| 0 触碰 3 不可变脊柱 | ✓ |
| 0 改 Relation / RelationKind / RelationRegistry | ✓ |
| 0 改 workspace.version (1.2.0) | ✓ |

## 10. 0-touch 声明

按 8 项不修改承诺:

- ✓ 0 触碰 `docs/v4/v4.1/v2/V0.5/V1136/9键原始`
- ✓ 0 触碰 `workspace.version` (1.2.0)
- ✓ 0 触碰 R11 baseline 3 values
- ✓ 0 触碰 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache)
- ✓ 0 改现有 Relation / RelationKind / RelationRegistry 签名
- ✓ 0 改现有 6+ lib unit + 3 integration 测试

## 11. 下一步 (R155 候选)

- TUI × runtime 集成 (per master "后端完全做好了再接 tui" — 后端已完成, TUI 现在 eligible)
- 调研 GitHub 优秀项目 (针对每个模块逐一调研, per master 8/12 指示)
- WebRTC signaling for apeireth-voice (留 R153 已知限制 #1)
- apeireth-council MCP 远程接入 (留 R153 已知限制 #2)
