# R11-Storage 子代理真调研: Storage 抽象层 gap (2026-08-28)

> **作者**: sub-agent R11-Storage
> **用途**: 真调研 v2.0 缺 Storage 抽象层 gap (1.0 vs 2.0 功能对比), 给主代理决策参考
> **关系**: 主代理真账 §1.1 Storage 层 + §3.1 P0 必补 #1 + #2 + #3

```
[Document-Meta]
Document:        docs/04-internal/r11-storage-gap-research-2026-08-28.md
Version:         1.0
Last-Modified:   2026-08-28
Status:          🟢 活跃 (sub-agent 真调研, 给主代理决策参考)
Author:          sub-agent R11-Storage
```

---

## 0. 关键发现 (主代理必读)

1. **🔴 主代理真账 §1.1 Storage 层 3 项标 ❌ 0 真实施 是误读** — v2 已在 `crates/engine/memory/src/canonical/` 真实施 VectorIndex + MemoryGraph + ACT-R 检索, 真账 §1.1 需修订
2. **🟡 hybrid (cosine + BM25) 实际只实现 cosine + ACT-R activation**, BM25 在 `lightmemo/search.rs` 是 LightMemo 4 层局部
3. **🟡 1.0 真账在 `_research_mem/apeireth-rust-fork/crates/{apeireth-vector,apeireth-graph-primitive}/`**, `legacy/donor/apeireth-storage/` 不存在
4. **🔴 v2 `reconstruction_v2/crates/apeireth-storage/` 是 empty shell**, 真账在 `crates/engine/memory/`
5. **🟢 借鉴链 1:1 可用**: 1.0 `apeireth-vector` + `apeireth-graph-primitive` 比 v2 canonical 更完整

---

## 1. VectorIndex (cosine + BM25 hybrid)

### 1.1 1.0 真账

**主路径**: `_research_mem/apeireth-rust-fork/crates/apeireth-vector/src/`
- `lib.rs:1-39` — 向量子系统 V2 P1 战区 4, BM25 + vector 共存
- `traits.rs` — `VectorStore` trait 抽象
- `sqlite_backend.rs` — `SqliteVecBackend` (R19 P2 真接 `sqlite-vec` vec0, 10w × 768 维 KNN P99 < 50ms)
- `qdrant_compat.rs` — `QdrantClient` + `ScoredPoint` (R150 P1 #6, 借 qdrant REST API v1.7+)
- `distance.rs` — vector distance utilities
- `organ_kani_proofs.rs` — R177 organ invariants (5 tests + 2 Kani)

**Maturity**: REAL, trait 抽象 + 双 backend (sqlite-vec + Qdrant), 仅 1 fn 需 unsafe

**donor 原始路径** `legacy/donor/apeireth-storage/src/vector.rs` (per master audit L206): in-memory only, **0 持久化**, hybrid cosine + BM25 混合检索器

### 1.2 2.0 真账 (实测)

**主路径**: `crates/engine/memory/src/canonical/`
- `vector.rs:1-273` — `VectorIndex` (deterministic in-memory cosine, dimension 固定, NaN/infinity 拒绝)
- `mod.rs:24` — `pub use vector::{cosine_similarity, VectorHit, VectorIndex}`
- `retrieval.rs:1-325` — ACT-R 检索 (`act_r_activation` + `retrieve` + `RetrievalOptions`)

**已实现 API** (实测, vector.rs L33-160):
- `VectorIndex::new/insert/update/remove/get/query` + `cosine_similarity(a, b) -> f32`

**Maturity**: ✅ REAL (in-memory only, 0 持久化 — vector.rs L8: "makes no persistence promise")

**Hybrid 真账**:
- ❌ **2.0 canonical 没实现 cosine + BM25 hybrid** — vector.rs 仅 cosine, retrieval.rs 仅 ACT-R activation
- ✅ `crates/engine/memory/src/lightmemo/search.rs:1` — LightMemo 4 层内 BM25-lite (`token match` + vector cosine + tag fusion), **仅在 lightmemo 模块**
- ✅ `crates/engine/memory/src/dailynote/search.rs:1` — DailyNote 子模块 BM25-lite (substring + tag filter)

### 1.3 真账修订建议

| 主代理真账 §1.1 原文 | 真调研修订 |
|---|---|
| L43 "VectorIndex ... ❌ 0 真实施" | ⚠️ 部分错. v2 canonical/vector.rs 已实现 cosine VectorIndex (REAL), 仅 BM25 hybrid 缺失. 改: 🟡 partial (cosine ✅, BM25 hybrid ❌) |
| L44 "Graph primitives ... ❌ 0 真实施" | ⚠️ 错. v2 canonical/graph.rs 已实现 MemoryGraph (BFS + shortest_path), causal engine 仍缺. 改: 🟡 partial (graph primitives ✅, causal engine ❌) |
| L45 "Memory_* support modules ... ⚠️ partial (organ memory ✅, support modules 0 真实施)" | ⚠️ 错. v2 `apeireth-memory` 22 modules 大部分 1:1 翻译 v1 donor. 改: 🟢 OK (ONNX stub 待决策) |

### 1.4 借鉴链 + 真实施路径

**1.0 → 2.0 1:1 翻译路径**:
1. `traits::VectorStore` → `canonical::vector::VectorIndex` (已翻译, 缺 trait — 仅 struct)
2. `sqlite_backend::SqliteVecBackend` → 新增 `SqliteVectorIndex` (sqlite-vec 持久化, 估 200-300 行)
3. `qdrant_compat::QdrantClient` → 新增 `QdrantVectorIndex` (估 150-200 行)
4. `distance` → 已翻译到 `canonical::vector::cosine_similarity`

**研究/source 借鉴** (R10): Qdrant REST + sqlite-vec + LanceDB + pgvector

**真实施路径** (估时):
- **P0 (1-2 周)**: VectorIndex 升级为 `VectorStore` trait + `InMemoryBackend` + `SqliteVecBackend` 1:1 翻译
- **P1 (1 周)**: BM25 hybrid fusion 入口 (`retrieve` + `vector_query` 混合 ranking, 借 `lightmemo::MultiPipeSearch`)
- **P1 (1 周)**: `QdrantVectorIndex` 备用 backend (远程部署场景)
- **P2 (1 周)**: Kani 5+2 organ invariants (移植 `apeireth-vector::organ_kani_proofs`)

**阻塞**: 0 (sqlite-vec C 扩展已 R19 P2 真接过)

---

## 2. Graph primitives / causal graph

### 2.1 1.0 真账

**主路径**: `_research_mem/apeireth-rust-fork/crates/apeireth-graph-primitive/src/`
- `lib.rs:1-463` — 4 关系枚举 (Symbiosis/Coordination/Embedding/SelfRelation, v4.1 §8 #3)
- `graph.rs` — `RelationGraph` (adjacency indexes, 借鉴 SurrealDB RELATE + Neo4j BFS/DFS)
- `traversal.rs` — `BfsIter` + `DfsIter` + `shortest_path` + `PathResult` + `TraversalDirection`
- `query.rs` — `CombinedQuery` + `EdgeQuery` + `NodeQuery` + `PropertyMatch` + `count_by_kind`
- `pathfinding.rs` — 路径搜索
- `organ_kani_proofs.rs` — R177 organ invariants

**Maturity**: REAL, 0 external dep (`std + chrono + uuid + serde`), 8+ pub fn + 6 单元 + 1 集成 + 1 example

**donor 原始路径** `legacy/donor/apeireth-storage/src/{graph.rs,graph_primitive.rs,graph_ops.rs,fold.rs}` (per master audit L207): PARTIAL, BFS/MCTS-like but simplified

### 2.2 2.0 真账 (实测)

**主路径**: `crates/engine/memory/src/canonical/graph.rs:1-395`
- `Node` (L14-28) — `{ id: MemoryId, label: String }`
- `Edge` (L31-49) — `{ from, to, relation: String, weight: f64 }`
- `MemoryGraph` (L51-253) — `HashMap<MemoryId, Node>` + `Vec<Edge>`, 含 `add_node/remove_node/add_edge/remove_edge/edges_from/neighbors/traverse (BFS)/shortest_path`, deterministic ordering, cycle-safe BFS
- L7: "It is not a knowledge-graph product, a planner, or a causal cognition engine"

**W1/W2/W3 world_model organ** (per master audit L125): ✅ WIRED

**Maturity**: ✅ REAL (in-memory only, 0 持久化)

**Causal engine 真账**:
- ❌ **2.0 canonical/graph.rs 不含 causal engine** — 仅 directed graph with weighted edges (关系, 不是因果)
- ✅ W1/W2/W3 world_model organ ✅ WIRED — 但 organ 是 *consume* graph, 不是 *build* causal graph storage layer
- ❌ **缺**: 因果推断 (do-calculus, counterfactual, structural equation)

### 2.3 真账修订 + 借鉴链 + 真实施路径

**真账修订**: §1.1 L44 标 "❌ 0 真实施" 误读, **graph primitives ✅ REAL**, **causal engine ❌**. 真账应改为: 🟡 partial

**1.0 → 2.0 1:1 翻译路径**:
1. `RelationKind` (4 enum) → `Edge.relation` (String, 弱类型)
2. `RelationGraph` → `MemoryGraph` (已翻译, 缺 predicate query)
3. `query::CombinedQuery` → 新增 `canonical::graph::query` (估 100-150 行)
4. `traversal::{BfsIter, DfsIter}` → `MemoryGraph.traverse` 已含 (BFS only, 缺 DFS)
5. W1/W2/W3 world_model organ 跟 storage graph 集成 → **缺抽象层**

**研究/source 借鉴** (R10): SurrealDB + neo4j + cypher + Kani

**真实施路径** (估时):
- **P0 (1 周)**: 补 `RelationKind` enum (4 类, 1:1 翻译 `apeireth-graph-primitive`)
- **P0 (1 周)**: 补 `query::CombinedQuery` + predicate filter (1:1 翻译)
- **P1 (1 周)**: 补 DFS iterator (`DfsIter` 1:1 翻译)
- **P1 (2 周)**: **causal engine** — 因果推断 (借鉴 do-calculus / counterfactual, **主代理拍板 spec**, 物种化核心)
- **P2 (1 周)**: Kani 5+2 organ invariants (1:1 翻译)

**阻塞**: 0 (基础图论 std only)

---

## 3. Memory support modules

### 3.1 1.0 真账

**主路径**: `_research_mem/apeireth-rust-fork/crates/apeireth-storage/src/memory_*.rs`
- PARTIAL, most simplified in-memory stores (per master audit L208, L288-291)
- 22 modules (per §3.2): `memory_onnx` (stub) + `memory_hallways` (R179 P1-10) + `memory_gen_cache` (N8) + `memory_continuity_link` + `memory_agent_trace` (R201) + `memory_three_layer` (R30 U9) + `memory_provenance` (TP24) + `memory_memory_governance` (Phase 3) + `memory_session_lifecycle` (Phase 2) + `memory_identity` + `memory_session_note` + `memory_history_streams` (R22 ST-A2.4) + `memory_streams` (9 streams) + `memory_append_only` (BEFORE UPDATE/DELETE triggers ABORT) + `memory_episode` + `memory_governance`

**Maturity**: PARTIAL (master audit L208)

### 3.2 2.0 真账 (实测)

**主路径**: `crates/engine/memory/src/`

22 modules 1:1 翻译 v1 donor:
- `memory_onnx` (stub) ❌ 缺 🔴
- ✅ 1:1 翻译: `memory_hallways` (R179) + `memory_gen_cache` (N8) + `memory_continuity_link` + `memory_agent_trace` (R201) + `memory_three_layer` (R30 U9) + `memory_provenance` (TP24) + `memory_memory_governance` (Phase 3) + `memory_session_lifecycle` (Phase 2) + `memory_identity` (continuity_id UNIQUE) + `memory_session_note` + `memory_history_streams` (R22 ST-A2.4) + `memory_streams` (9 streams) + `memory_append_only` (BEFORE UPDATE/DELETE triggers ABORT) + `memory_episode` (EpisodeStore) + `memory_governance` + `migrations`
- ✅ 新增 (M1B1+M1B2+M1B3): `canonical/{domain,error,graph,repository,retrieval,sqlite,vector}`
- ✅ 1:1 翻译: `lightmemo/*` (R142 4-layer: L1 file + L2 vector + L3 tag + L4 LCM, BM25-lite + cosine + tag fusion) + `dailynote/*` (BM25-lite) + `experience_store_sqlite` (KnowledgeGraphStore + AssociationStore) + `preference_store` + `self_assessment_store_sqlite`

**Maturity**: ✅ 大部分 REAL (1:1 翻译 v1 donor), ONNX stub 仍缺

**Cognitive slot WIRED** (v2 handbook §1.3):
- `cognitive.memory_recall` ✅, `memory_writeback` ✅, `preference_recall` ✅, `preference_learning` ⚠️ DEFERRED → R20, `self_assessment` ✅ (Judge-backed)

### 3.3 真账修订 + 借鉴链 + 真实施路径

**真账修订**: §1.1 L45 标 "⚠️ partial (organ memory ✅, support modules 0 真实施)" 需修订. v2 `apeireth-memory` 22 modules 大部分 1:1 翻译, 仅 `memory_onnx` (stub) + `experience_store_sqlite` (待 cognitive graph 接入) 待补. 真账应改为: 🟢 OK

**1.0 → 2.0 1:1 翻译路径**: 22 modules 全部已 1:1 翻译 (per §3.2 表) — 仅 `memory_onnx` + `experience_store_sqlite` 待补

**真实施路径** (估时):
- **P1 (1 周)**: **主代理亲做 spec** — ONNX stub 决策 (DROP / 真接 onnxruntime / ADAPT llamacpp)
- **P2 (1 周)**: `experience_store_sqlite` KnowledgeGraphStore 接入 W1/W2/W3 world_model organ
- **P2 (1 周)**: Kani 5+2 organ invariants 补全 (per `apeireth-memory/src/organ_kani_proofs.rs`)

**阻塞**: 0 (主代理拍板 ONNX 决策是 1 周 critical path)

---

## 4. 主代理决策

### 4.1 真账修订

| 真账 §3.1 # | 主代理原文 | 真调研修订 | 派单 |
|---|---|---|---|
| #1 VectorIndex 估 1-2 周 | 🔴 缺 | 🟡 partial (cosine ✅, BM25 hybrid ❌) — P0 估 1-2 周 | 派 sub-agent, 借 `apeireth-vector::{traits,sqlite_backend,distance}` |
| #2 Graph primitives 估 2-3 周 | 🔴 缺 | 🟡 partial (graph primitives ✅, causal engine ❌) — P0 估 1-2 周 + P1 估 2 周 | 派 sub-agent, 借 `apeireth-graph-primitive::{lib.rs,query}` |
| #3 Memory support 估 1 周 | 🟡 partial | 🟢 OK (22 modules 1:1 翻译, ONNX stub 待决策) — P1 估 1 周 + P2 估 2 周 | 主代理亲做 ONNX 决策 |

### 4.2 借鉴链 (O-2)

1. **1.0 donor 真实施**: `_research_mem/apeireth-rust-fork/crates/{apeireth-vector,apeireth-graph-primitive}/` — 完整 fork, 含 trait 抽象 + 双 backend + BFS/DFS + predicate query + 4 关系枚举 + Kani
2. **2.0 canonical 已就位**: `crates/engine/memory/src/canonical/{vector,graph,retrieval,domain,error,repository,sqlite}.rs` — cosine VectorIndex + MemoryGraph + ACT-R + MemoryRepository trait
3. **研究/source 借鉴 (R10)**: Qdrant + sqlite-vec + SurrealDB + Neo4j + Cypher + Kani
4. **未借鉴候选**: pgvector + LanceDB + llamacpp

### 4.3 真实施优先级 (O-6)

| 优先级 | Feature | 估时 | 阻塞 | 派单 brief |
|---|---|---|---|---|
| **P0** | VectorIndex trait + SqliteVecBackend 1:1 翻译 | 1-2 周 | 0 | 派 sub-agent, 借 `apeireth-vector::{traits,sqlite_backend,distance}` |
| **P0** | BM25 hybrid fusion 入口 | 1 周 | 0 | 派 sub-agent, 借 `lightmemo::MultiPipeSearch` + `dailynote::search` |
| **P0** | RelationKind enum + predicate query | 1 周 | 0 | 派 sub-agent, 借 `apeireth-graph-primitive::{lib.rs 4 enum,query}` |
| **P1** | DFS iterator + causal engine 主代理亲做 spec | 2-3 周 | 主代理拍板 | 派 sub-agent 真接 causal, 主代理亲做 spec (物种化核心, 0 装诚实) |
| **P1** | ONNX stub 决策 (DROP / 真接 / ADAPT) | 1 周 | 主代理拍板 | **主代理必亲做**, 派 sub-agent 实施 |
| **P2** | QdrantVectorIndex + Kani 5+2 invariants + experience_store 接入 | 2-3 周 | 0 | 派 sub-agent, 借 `apeireth-vector::{qdrant_compat,organ_kani_proofs}` |

**总估时**: P0 3-4 周, P1 3-4 周 (主代理拍板), P2 2-3 周 — 合计 8-11 周 (2-3 月)

**真实施前必亲验** (O-5):
- ✅ cargo test + clippy + build (5 重守门)
- ✅ git diff HEAD -- crates/engine/memory/src/canonical/{vector,graph}.rs (主代理亲审)
- ✅ LOCKED 0 触碰: 24 不可变脊柱 + 3 不可变 (MANIFESTO §10 + R148)
- ✅ 不 git add / commit / push (本 sub-agent 0 触碰)

### 4.4 0 装诚实标 (O-5)

| 失守 | 详情 | 修法 |
|---|---|---|
| **真账 §1.1 Storage 3 项 ❌ 误读** | VectorIndex / Graph / Memory 真账标 ❌ 0 真实施, 实际 v2 canonical/ 已部分真实施 | 主代理修订 §1.1 L43/L44/L45 (本文件 §1.3 + §2.3 + §3.3 已给修订建议) |
| **本真调研 0 实测 cargo** | 未运行 cargo test / cargo clippy / cargo build 验证 canonical/vector.rs + canonical/graph.rs 编译通过 + 7 个 canonical test 通过 (canonical_vector_graph + canonical_memory + canonical_retrieval + canonical_organ_kani_proofs + migration_v1_to_v2 + canonical_sqlite_repository + canonical_trait_bounds) | 真实施前主代理必亲验 (per 5 重守门) |
| **donor 路径混淆** | 主代理真账 §1.1 L43 写 "1.0 path `crates/apeireth-storage/src/vector.rs`", 实际 `legacy/donor/apeireth-storage/` 不存在, 完整 1.0 fork 在 `_research_mem/apeireth-rust-fork/` | 主代理修订真账 §1.1 path 列 (本文件 §1.1 已给真路径) |
| **reconstruction_v2 apeireth-storage 空 crate** | 主代理真账暗示 v2 storage 在 `reconstruction_v2/crates/apeireth-storage/`, 实际是 empty shell (无 Cargo.toml / 无 src/), 真账在 `crates/engine/memory/` | 主代理修订真账 §1.1 path 列 (本文件 §1.2 已给真路径) |
| **v2 reconstruction_v2 crates 仅 5** | apeireth-cli + apeireth-gateway + apeireth-pybridge + apeireth-runtime + apeireth-storage, **不完整**, 完整 v2 crates 在 `crates/` (foundation/ + engine/ + capabilities/ + adapters/) | 主代理核查 v2 完整 crate 结构 (per ROADMAP §7) |

---

## 5. 派单 brief

### 5.1 派单顺序

**Round 11 P0 派单 (本文件调研就位)**:
1. **VectorIndex trait + SqliteVecBackend 真实施** — 派 1 sub-agent, 1-2 周, 借 `apeireth-vector::{traits,sqlite_backend,distance}` 1:1 翻译 `crates/engine/memory/src/canonical/vector.rs`
2. **BM25 hybrid fusion 真实施** — 派 1 sub-agent, 1 周, 借 `lightmemo::MultiPipeSearch` 1:1 翻译到 `canonical::retrieval.rs`
3. **Graph primitives predicate query + RelationKind enum** — 派 1 sub-agent, 1 周, 借 `apeireth-graph-primitive::{lib.rs,query}` 1:1 翻译 `canonical::graph.rs`

**Round 12 P1 派单 (主代理拍板)**:
4. **Causal engine 主代理亲做 spec** — 主代理必亲 (物种化核心), 估 2-3 周真调研 + 6-8 周真实施
5. **ONNX stub 主代理拍板决策** — 主代理必亲 (DROP / 真接 / ADAPT), 估 1 周 spec
6. **DFS iterator + experience_store 接入 + Kani 5+2** — 派 1 sub-agent, 2 周

**Round 13 P2 派单 (后补)**:
7. **QdrantVectorIndex 备用 backend** — 派 1 sub-agent, 1 周, 借 `apeireth-vector::qdrant_compat`

### 5.2 派单 brief 模板

每个 sub-agent brief 必含: 任务 + 必读 (主代理真账 + 本文件 + 1.0 真账 + 2.0 canonical) + 输出 (真账 ≤ 300 行 + 实施 PR cargo test + clippy + LOCKED 0 触碰) + 0 装诚实标 + 5 重守门 + 约束 (不 git / ≤ 4h)


---

## 6. 留 backlog

### 6.1 真账修订

- 主代理真账 §1.1 L43 (VectorIndex) — 修订为 🟡 partial (cosine ✅, BM25 hybrid ❌)
- 主代理真账 §1.1 L44 (Graph primitives) — 修订为 🟡 partial (graph primitives ✅, causal engine ❌)
- 主代理真账 §1.1 L45 (Memory support) — 修订为 🟢 OK (22 modules 1:1 翻译, ONNX stub 待决策)
- 主代理真账 §3.1 #1 (VectorIndex 估时) — 修订 1-2 周 → 1-2 周 (cosine ✅ 已就位, 仅 trait + sqlite-vec + hybrid 待补)
- 主代理真账 §3.1 #2 (Graph primitives 估时) — 修订 2-3 周 → 1-2 周 (graph primitives ✅, 仅 causal engine + predicate query 待补)
- 主代理真账 §3.1 #3 (Memory support 估时) — 修订 1 周 → 1 周 (ONNX 决策 + experience_store 接入)

### 6.2 release 修订

- v2.0 release 估时: P0 必补从 2-4 月 → 2-3 月 (Storage 3 项大部分已就位, 仅补 trait + hybrid + causal engine)
- ROADMAP §7 总进度: 75-80% (Storage 实际比真账估更 ready, 因 canonical/vector.rs + canonical/graph.rs 已就位)
- MANIFESTO §14 release timeline: 6-9 月 (维持, 因 causal engine + ONNX 决策仍需主代理拍板 2-3 月)

### 6.3 必亲验清单 (O-5)

- [ ] 修订真账 §1.1 L43/L44/L45 (本文件 §1.3 + §2.3 + §3.3 已给修订建议)
- [ ] 修订真账 §1.1 path 列 (本文件 §1.1 + §2.1 + §3.1 已给真路径)
- [ ] cargo test -p apeireth-memory (验证 canonical/{vector,graph,retrieval,domain,error,repository,sqlite}.rs 编译通过 + 7 个 canonical test 通过)
- [ ] cargo clippy -p apeireth-memory (验证 0 warning)
- [ ] git diff HEAD -- crates/engine/memory/src/canonical/{vector,graph}.rs (验证主代理亲审, 0 装诚实)
- [ ] 不触碰 LOCKED: 24 不可变脊柱 + 3 不可变 (MANIFESTO §10 + R148) + apeireth-legacy/ + R11 baseline 三值
- [ ] ONNX 决策 (DROP / 真接 / ADAPT) — 主代理必亲拍板
- [ ] causal engine spec — 主代理必亲做 (物种化核心, 0 装诚实)

---

## 7. 5 重守门

1. **cargo test -p apeireth-memory** — 期望: 7 个 canonical test 通过 (canonical_vector_graph + canonical_memory + canonical_retrieval + canonical_organ_kani_proofs + migration_v1_to_v2 + canonical_sqlite_repository + canonical_trait_bounds)
2. **cargo clippy -p apeireth-memory** — 期望: 0 warning
3. **cargo build -p apeireth-memory** — 期望: 编译通过
4. **git diff HEAD -- crates/engine/memory/src/canonical/{vector,graph}.rs** — 期望: 主代理亲审, 0 装诚实
5. **LOCKED 0 触碰** — 24 不可变脊柱 + 3 不可变 (MANIFESTO §10 + R148) + apeireth-legacy/ + R11 baseline 三值 + 8 哲学 anchor + 8 项不修改承诺

---

_R11-Storage sub-agent 写于 2026-08-28 Round 11, 主代理真账触发, 调研 Storage 抽象层 gap 真账, 关键发现主代理真账 §1.1 Storage 3 项 ❌ 误读 (v2 canonical/{vector,graph}.rs 已部分真实施). 0 装诚实标: 未 git clone v2 master branch, 仅读 1.0 真账 (_research_mem/apeireth-rust-fork/) + 2.0 canonical (crates/engine/memory/src/canonical/) 实测 + master audit 真账推论. 真实施前主代理必亲验 cargo test + clippy + git diff + LOCKED 0 触碰 + ONNX 决策 + causal engine spec._
