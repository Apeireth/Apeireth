# V2 memory × vector 性能基准报告（2026-08-05）

> 结果状态：**四档真实运行完成，不是预测值**。运行完成后 integration 工作目录发生外部基线回退，`apeireth-vector/src` 与 `memory/src/semantic.rs` 一度从当前目录消失；下列数字来自回退前已成功退出（exit code 0）的 Criterion 运行。本任务保留可复跑 bench，源集成恢复后使用同一命令即可验证。

基准文件：`crates/apeireth-memory/benches/v2-memory-vector-bench.rs`  
命令：`cargo bench -p apeireth-memory --bench v2-memory-vector-bench --features semantic -- --noplot`

## 1. 摘要

`SemanticIndex::search` 在 100、1K、10K、100K episode 上的 P50 为 **0.1187、0.8801、10.0830、145.5459 ms**。100K P95/P99 为 **166.5775/167.0322 ms**。当前 skeleton 在 10K 内适合轻量本地语义检索；到 100K 时已明确进入 brute-force 线性扫描瓶颈，若目标为 P95 < 50 ms，应切换 sqlite-vec/HNSW/专用向量 backend，而不是继续增加普通 SQLite B-tree。

当前 vector backend 对 `vec_items` 执行 `SELECT id, vec, metadata WHERE dim=?`，在 Rust 端解 BLOB、计算全部 cosine、解析全部 metadata、全量排序，再按 top-k episode ID 回读 memory。因此主复杂度为：

```text
T(N,D,K) = Tembed(D)
         + N × (Trow + Tunpack(D) + Tcosine(D) + Tmetadata)
         + Tsort(N log N)
         + K × Tepisode_pk_lookup
```

`idx_vec_items_dim` 只过滤不同维度，不能加速同维度最近邻。

## 2. 方法和边界

### 被计时路径

每次调用 `SemanticIndex::search("sqlite vector semantic memory benchmark", 10)`，包含 query embedding、SQLite candidate scan、BLOB 解码、cosine、JSON metadata 解码、top-k ranking 及 10 次 episode 主键回读。这是 memory→vector→memory 的公开端到端查询，不是单独 dot-product 微循环。

### 排除项

数据生成、episode 插入、向量索引写入、连接打开和 migration 均在计时区外。memory/vector 使用独立 `:memory:` SQLite，排除了 SSD、文件系统 flush、WAL checkpoint 和杀毒软件噪声。因此本结果用于比较算法 scaling，不等价于生产文件 DB 的冷缓存/并发尾延迟。

### 数据设置

* 数据规模：100、1,000、10,000、100,000。任务文字称“三档”但明确列出四个规模，本次按四个规模执行。
* 维度：32；top-k：10。
* embedding：确定性 FNV-1a hash bucket + L2 normalize，无网络、无真实模型、无随机依赖。
* percentile：每档预热 5 次，随后 `Instant` 采集 31 次，nearest-rank 得到 P50/P95/P99。
* Criterion：20 samples、1 秒 warm-up、2 秒 measurement；100K 自动扩展到约 2.9 秒以完成 20 samples。

## 3. 环境

| 项目 | 值 |
|---|---|
| CPU | AMD Ryzen 9 9955HX 16-Core Processor |
| RAM | 33,485,037,568 bytes（约 31.2 GiB） |
| OS | Microsoft Windows 11 build 10.0.26200 |
| Rust | rustc 1.97.1, LLVM 22.1.6 |
| Cargo | 1.97.1 |
| target | x86_64-pc-windows-msvc |
| rusqlite | workspace 0.32 + bundled |
| 数据库 | 两个独立 SQLite `:memory:` |

## 4. 真实结果

### 4.1 墙钟 percentile

| episodes | P50 | P95 | P99 | P50/episode | P50 相对上一档 |
|---:|---:|---:|---:|---:|---:|
| 100 | 0.1187 ms | 0.1322 ms | 0.1377 ms | 1.187 µs | — |
| 1,000 | 0.8801 ms | 1.1418 ms | 1.1678 ms | 0.880 µs | 7.41× |
| 10,000 | 10.0830 ms | 14.1249 ms | 21.2374 ms | 1.008 µs | 11.46× |
| 100,000 | 145.5459 ms | 166.5775 ms | 167.0322 ms | 1.455 µs | 14.43× |

原始输出：

```text
V2_MEMORY_VECTOR_PERCENTILES dataset=100 samples=31 p50_ns=118700 p95_ns=132200 p99_ns=137700
V2_MEMORY_VECTOR_PERCENTILES dataset=1000 samples=31 p50_ns=880100 p95_ns=1141800 p99_ns=1167800
V2_MEMORY_VECTOR_PERCENTILES dataset=10000 samples=31 p50_ns=10083000 p95_ns=14124900 p99_ns=21237400
V2_MEMORY_VECTOR_PERCENTILES dataset=100000 samples=31 p50_ns=145545900 p95_ns=166577500 p99_ns=167032200
```

### 4.2 Criterion 估计区间

| episodes | time interval | central | candidate throughput central |
|---:|---:|---:|---:|
| 100 | 94.438–97.849 µs | 96.148 µs | 1.0401 M/s |
| 1,000 | 691.32–725.33 µs | 707.01 µs | 1.4144 M/s |
| 10,000 | 8.3607–9.0049 ms | 8.6743 ms | 1.1528 M/s |
| 100,000 | 142.06–152.11 ms | 146.77 ms | 681.35 K/s |

10K 有 1/20 high severe outlier，100K 有 1/20 high mild outlier。10K 的独立 P99 21.237 ms 也显示 OS 调度、临时分配或 cache miss 会拉高尾延迟。Criterion central 值比独立 P50 略低是合理的：Criterion 批量迭代并做统计校正；SLO 看独立 percentile，代码回归看 Criterion baseline/change。

## 5. Scaling 分析

100→1K 只增长 7.41×，因为 statement、query embedding 和 top-k 回读等固定成本在小集合占比较高。1K→10K 增长 11.46×，开始接近/超过线性。10K→100K 增长 14.43×，明显差于理想 10×：若用 10K P50 线性外推，100K 应约 100.83 ms，实测 145.55 ms，多约 44%。候选吞吐也从 10K 的 1.15M/s 降至 100K 的 0.68M/s，符合全量分配、JSON decode、排序和 CPU cache 压力共同放大的判断。

本次维度仅 32。换成 384/768/1536 时，BLOB bytes、解码与 dot product 会随 D 增长，而 row/JSON/sort 固定部分不会严格成比例，所以不能简单把总延迟乘 12/24/48；真实模型维度必须复跑。不过 100K×768 brute-force 几乎不可能稳定满足交互式 P95 < 50 ms。

降低 K 对当前算法帮助有限，因为先为全部 N 构造 hit 并排序、再 truncate。K 只影响最后 episode 回读。换成 K 大小有界堆可把 ranking 从 `O(N log N)` 降为 `O(N log K)`，仍保留 `O(N·D)` scan。

## 6. 优化路线

### P0：保留 SQLite BLOB backend

1. **top-k 有界堆**：扫描时仅保留 K 个最佳项，避免全量排序。
2. **metadata 延迟解析**：保留 raw metadata，只解析 top-k；目前 N 条全部 JSON decode，最终只返回 10 条。
3. **零/少分配 cosine**：直接迭代 BLOB `chunks_exact(4)`，不要每 row 构造 `Vec<f32>`。
4. **预归一化写入**：写入 generation 标记已归一化，查询只 dot。旧 row 必须 fallback normalize 并后台渐进迁移，不得原地解释成新格式。
5. **批量回读 episode**：一个 `WHERE id IN (...)` 获取 top-k，再按 hit order 重排。

这些优化适合把 100K P95 从 166 ms 压向 80–120 ms；若 SLO 是 50 ms，不应以微优化替代 ANN。

### P1：batch 与 cache

* `SemanticIndex::index_batch` 应最终透传 `VectorStore::upsert_batch`，用单事务批量 256–1,000 条；避免每 row commit，也避免 100K 单个巨大 WAL。
* query embedding cache key 必须含 `model_id + model_revision + normalized_query`，防模型升级复用旧向量。
* search result cache key 必须含 query hash、K、filter、`index_generation`；成功提交 batch 后递增 generation，保证新增 episode 可见。
* episode LRU 对 append-only 数据安全，但出现删除/修订时必须失效。先测命中率，不为小集合新增分布式缓存。

### P1/P2：HNSW 与 IVFFlat

| 方案 | 优势 | 代价 | 建议场景 |
|---|---|---|---|
| sqlite-vec | 保持单文件和 SQLite 运维 | 扩展装载/平台构建需验证 | 10K–数十万本地数据 |
| HNSW | 高 recall、低 latency、增量友好 | 内存和构建成本较高 | 10K–百万，低延迟优先 |
| IVFFlat | 内存较省、批量成熟 | 需训练；分布漂移会降召回 | 大规模稳定数据 |
| pgvector | SQL filter/备份/HA 成熟 | 网络与服务运维 | 多用户、已有 PostgreSQL |
| LanceDB/专用引擎 | 向量/列式能力完整 | 新存储与恢复流程 | 超过单机 SQLite 舒适区 |

HNSW spike 必须同时测 recall@10 与 latency，以 exact backend 为 truth，初始比较 `efSearch=32/64/128`，目标 recall@10 ≥ 0.95。IVFFlat 可从 lists≈`sqrt(N)`、多档 probes 起步，不应硬编码参数后只报告速度。

## 7. 容量与 CI 守护

建议普通 PR 只跑 100/1K smoke，夜间/release 跑四档：

* 1K Criterion central baseline 0.707 ms，退化 >20% 报警；
* 10K P95 目标 ≤20 ms；
* 100K exact baseline P95 166.58 ms，未切 ANN 前报警线 200 ms；
* ANN 除 P50/P95/P99 外强制报告 recall@10；
* 记录 N、D、K、backend/schema/model generation、CPU、Rust target。

生产前还需文件 DB 热/冷 cache、1/4/16 并发、写入与 checkpoint 同时发生、真实 768 维四类基准。单线程 in-memory 数字是可靠的 scaling baseline，但不是生产容量承诺。

## 8. 复现与当前基线说明

```bash
cargo check -p apeireth-memory --bench v2-memory-vector-bench --features semantic
cargo bench -p apeireth-memory --bench v2-memory-vector-bench --features semantic -- --noplot
```

预期出现四行 `V2_MEMORY_VECTOR_PERCENTILES`，每行 `samples=31`。本次上述命令已在 integration 回退前成功运行；回退后若提示缺少 `apeireth-vector/src` 或 `apeireth_memory::semantic`，应先恢复已 merged 的 memory×vector 集成，而不是修改基准绕开公开 API。恢复后本 bench 无需改变即可复跑。
