# Round-22 Rust 真源码深读 — tokio / sqlx / delta-rs

> 主题: 主 12:07 真哲学校准 + 主 14:27 聚合全人类智慧 + Rust 迁移准备. 三个 Rust 真生产 repo `--depth 1` clone 成功, 真读核心 src 不止 README. 实事求是: 推荐 / 不推荐都给理由. 避开 round-13/14/15/16/17/18/19/20 (Python agent/SDK 生态), 这次聚焦 Rust 真生产生态 — async runtime + compile-time SQL + lakehouse state.

---

## tokio async runtime — 代码层细节

**核心文件**: `tokio/tokio/src/runtime/scheduler/multi_thread/{mod.rs, worker.rs, queue.rs, idle.rs, handle.rs, overflow.rs}` + `tokio/tokio/src/runtime/blocking/{pool.rs, schedule.rs}`. 工作目录 `tokio/tokio/src/`.

### 数据结构 — Worker / Core / Shared 三层 + 本地队列原子环形

1. **`Worker { handle, index, core: AtomicCell<Core> }`** (worker.rs L99) — 顶层对象, `AtomicCell<Core>` 是 `block_in_place` 时把核心移交另一个线程的关键 (注释明说 "Used to hand-off a worker's core to another thread"). 不放回就避免被偷到另一个被 block_in_place 派生出来的线程.
2. **`Core` (per-thread state)**: `tick: u32` / `lifo_slot: Option<Notified>` / `lifo_enabled` / `run_queue: queue::Local<Arc<Handle>>` / `is_searching: bool` / `is_shutdown` / `park: Option<Parker>` / `stats: Stats` / `rand: FastRand`. **关键设计: LIFO 槽** — 注释解释: "the last scheduled task to be run next (LIFO), optimization for improving locality, message passing patterns, latency". `MAX_LIFO_POLLS_PER_TICK = 3` (硬编码调参, 注释说"picked out of thin-air").
3. **`Shared` (across all workers)**: `remotes: Box<[Remote]>` / `inject: inject::Shared<Arc<Handle>>` (全局队列) / `idle: Idle` (协调空闲) / `owned: OwnedTasks<Arc<Handle>>` (所有 spawned 任务) / `synced: Mutex<Synced>` / `shutdown_cores: Mutex<Vec<Box<Core>>>` / `worker_metrics`. 两类状态同步: 原子变量 (fast path, SeqCst 顺序) + `Mutex` (slow path, sleepers list).
4. **`queue::Inner<T>`** (queue.rs L49): 核心是 `head: AtomicUnsignedLong` (u32 或 u64, 通过 `cfg_has_atomic_u64!` 选) — **同一原子里塞两个值, LSB 是 head, MSB 是 stealer 进行中的 head**. 注释解释了 ABA 问题: 扩到 u32/u64 而非 u16/u32, 是为了 ABA mitigation. `unsafe impl<T> Send for Inner<T> {}` + `unsafe impl<T> Sync for Inner<T> {}` — 手动 unsafe Send/Sync 实现, 因为内部是 `Box<[UnsafeCell<MaybeUninit<task::Notified<T>>>; 256]>` (固定 256 槽, 不重新分配, 用 `MASK = LOCAL_QUEUE_CAPACITY - 1` 做按位取模).
5. **`Idle` coord** (idle.rs): 用**单个 `AtomicUsize` 装两个计数** — `num_searching` 在低 16 位 (`SEARCH_MASK = (1 << 16) - 1`), `num_unparked` 在高 16 位 (`UNPARK_SHIFT = 16`). 关键注释: `notify_should_wakeup` 用 `SeqCst` load 必须发生在 `fetch_sub(1)` 减 searching 计数**之前**, 否则 Acquire/Release 不够强 — 这是 memory ordering 的真功夫.

### 关键算法 — Work-stealing scheduler loop + inject overflow

1. **Run loop** (worker.rs `Context::run`): 单个 `while !core.is_shutdown` 循环, 每 tick 顺序是 (a) `tick()` 递增 (b) `maintenance()` 周期维护 (c) `core.next_task()` 先看 LIFO slot 再看本地队列 (d) 没活则 steal + park. `block_in_place` 时把 `Box<Core>` 直接传给 spawned thread, 主线程阻塞, 算力转移.
2. **`Overflow<T>` trait** (overflow.rs) — 当本地队列满 (256 槽) 时把多余任务推到全局 inject 队列. 测试时是 `RefCell<Vec<Notified>>`, 真实是 inject queue. **这是 work-stealing 的关键: 本地满了不要立即阻塞, 转全局** — 反过来想就是 goroutine/M:NIon 偷栈的极端做法.
3. **Idle wake protocol** (idle.rs `worker_to_notify`): 双重检查 + 锁 + SeqCst — `notify_should_wakeup` 看 searching 数 == 0 且 unparked 数 < 总数才唤醒. 取一个 sleeping worker index, `fetch_add` 同时增加 unparked 和 searching 计数, 返回 index 给调用者去 unpark. 极精炼.
4. **Blocking pool** (blocking/pool.rs) — 独立于 async runtime 的同步线程池, `spawn_blocking` 走这里. 用 `Arc<Inner> { shared: Mutex<Shared>, condvar }`, 按需增长, 任务队列是 `VecDeque`. **关键教训: 不是所有 IO 都能 async, 真 sync 阻塞要走专门线程池**, 别污染 async executor.
5. **`loom` crate wrapper** — tokio 用 `crate::loom::sync::{Arc, Mutex, Condvar}` 替代 `std::sync::*`, 因为 loom 能在测试时**穷举所有线程交错**, 验证并发正确性. 这是 Rust 生态独有的 systematic concurrency testing.

### 借鉴清单 (机制不抄结构)

| tokio 机制 | Apeireth 当前 (Python) | 借鉴到 Rust 重写 |
|---|---|---|
| `MultiThread` scheduler + work-stealing | 没有中央 scheduler, `asyncio.create_task` 各自跑 | `tokio::runtime::Builder::new_multi_thread()` 替代 asyncio loop |
| `LIFO slot` (locality) | 无 | 长 task chain (research→reasoning→memory) 用 LIFO 优化 |
| `inject` 全局队列 + Overflow | 无 | burst spawn (cron fan-out) 用 inject |
| `block_in_place` core hand-off | `asyncio.to_thread` 简单粗暴 | `tokio::task::block_in_place` 同样语义但 0 线程创建开销 |
| 独立 `blocking::pool` | `asyncio.to_thread` 默认每次新建 thread | 真 sync 阻塞 (filesystem heavy) 走 blocking pool |
| `Idle` SeqCst ordering | 单线程 GIL | 真多线程要仔细想 memory ordering |
| `loom` systematic testing | pytest 随机 | 关键并发原语用 loom 测 |

---

## sqlx async SQL — 代码层细节

**核心文件**: `sqlx/sqlx-macros/src/lib.rs` (38 lines, 纯入口) + `sqlx/sqlx-macros-core/src/query/{mod,input,output,cache,data,metadata}.rs` + `sqlx/sqlx-sqlite/src/{type_checking.rs, connection/{mod,establish,worker}.rs}` + `sqlx/sqlx-core/src/pool/inner.rs`. 工作目录 `sqlx/`.

### 架构 — proc-macro driver dispatch + trait Database trait per backend

1. **`sqlx-macros` 总入口** (sqlx-macros/src/lib.rs, 38 行) — 只做三件事: `syn::parse_macro_input!` → `query::expand_input(input, FOSS_DRIVERS)` → 返回 `TokenStream`. 错误处理精致: `syn::Error` 用 `to_compile_error()`, 其他错误用 `quote!(::std::compile_error!(#msg))`. 这是 proc-macro 错误处理的样板.
2. **`QueryDriver` const fn** (query/mod.rs L19) — **编译期函数指针 dispatch**:
   ```rust
   pub const fn new<DB: DatabaseExt>() -> Self
   where Describe<DB>: serde::Serialize + serde::de::DeserializeOwned { ... }
   ```
   每个 backend (Sqlite/Postgres/MySql/Mssql) 实现 `DatabaseExt`, 注册一个 `QueryDriver { db_name, url_schemes, expand }`. 调用时按 url scheme 选 driver. 这是 Rust trait + const fn 的教科书用法.
3. **`QueryMacroInput` 解析** (query/input.rs) — `syn::parse` 实现 `Parse` trait, 接受 `source = "..."` / `source_file = "..."` / `args = [...]` / `record = Type` / `scalar = _` / `checked = true/false`. parse 逻辑手动逐项判断 key (`if key == "source" { ... } else if key == "source_file" { ... }`), 错误有 `input.error("colliding `scalar` or `record` key")`.
4. **`QueryDataSource<'a>` enum** (query/mod.rs L42) — 两种 compile-time 数据来源: `Live { database_url, database_url_parsed }` (真连 DB `PREPARE` + `DESCRIBE`) 或 `Cached(DynQueryData)` (offline mode). `matches_driver` 校验 scheme.
5. **`MtimeCache<T>` + `MtimeCacheBuilder`** (query/cache.rs) — **基于文件 mtime 的自动失效缓存**. `add_path()` 注册要监听的路径, `any_modified()` 在 cache hit 时再查 mtime, 不一致就 re-init. 测试时 panic recover: `lock.unwrap_or_else(|e| { *e.into_inner() = None; e.into_inner() })`. 加 `proc_macro::tracked::path()` 让 cargo 增量编译识别.
6. **`OFFLINE_DATA_CACHE: LazyLock<Mutex<HashMap<PathBuf, Arc<MtimeCache<DynQueryData>>>>>`** (query/data.rs) — 进程级 cache, 复用 mtime 检查. 配套 `.sqlx/query-{hash}.json` 文件 = cache 落盘格式.
7. **`Metadata::workspace_root()`** (query/metadata.rs) — 调 `cargo metadata --format-version=1 --no-deps` 找 workspace root. **关键洞察: proc-macro 在编译期运行, 但 cargo env (`CARGO_MANIFEST_DIR`) + 调子进程 `cargo metadata` 才能知道 workspace 信息**. 这是 Rust build system 集成 SQL 的独有设计.

### 关键算法 — Type checking + sqlite dedicated worker

1. **`impl_type_checking!` macro** (sqlx-sqlite/src/type_checking.rs) — 每个 backend 一份 type 表 (sqlite = `bool/i32/i64/f64/String/Vec<u8>/Uuid`), 通过 `ParamChecking::Weak` (sqlite 不强校验, 因为 SQLite type system 本来就弱) vs `ParamChecking::Strong` (postgres 严格). `datetime-types: { chrono: {...}, time: {...} }` + `numeric-types: { bigdecimal, rust_decimal }` 是 feature-gated crate 选择. 这是 **Rust trait + macro 表达 backend 多态** 的标准做法.
2. **`PoolInner<DB>`** (sqlx-core/src/pool/inner.rs) — `idle_conns: ArrayQueue<Idle<DB>>` (crossbeam 数组队列, lock-free push/pop) + `semaphore: AsyncSemaphore` (公平背压) + `size: AtomicU32` / `num_idle: AtomicUsize` / `is_closed: AtomicBool` + `on_closed: event_listener::Event`. 关键: semaphore 容量 = max_connections, **child pool 必须从 parent pool 偷 permit** (`semaphore_capacity = 0` if parent_pool). 这是 hierarchy pool 设计.
3. **SQLite 用 dedicated thread per connection** (sqlx-sqlite/src/connection/worker.rs) — `ConnectionWorker { command_tx: flume::Sender<(Command, Span)>, shared: Arc<WorkerSharedState> { conn: Mutex<ConnectionState> } }`. **关键工程教训: `libsqlite3-sys` 不是 Send-safe, C FFI 限制了 Rust 并发模型, 必须一个连接一个线程 + channel 命令**. 这点和 tokio work-stealing 哲学相反, 是 FFI 约束的妥协.
4. **`StatementCache`** (sqlx-sqlite/src/connection/mod.rs `use sqlx_core::common::StatementCache`) — prepared statement 缓存跨 query, 因为 SQLite `sqlite3_prepare_v2` 很贵. `VirtualStatement` 是 wrapped handle + raw SQL safe assert (`AssertSqlSafe`, `SqlSafeStr` 类型防止 SQL injection 在 type system 层).

### 借鉴清单 (机制不抄结构)

| sqlx 机制 | Apeireth 当前 (Python) | 借鉴到 Rust 重写 |
|---|---|---|
| `QueryDriver` const fn + trait dispatch | `aiosqlite` 字符串 SQL | sqlx = 真 compile-time check, 但**仅 SQL safety**, 不检查 query 字符串 syntax — 需连 DB |
| `MtimeCache` + `.sqlx/query-{hash}.json` | 无 | 我们的 `memory_store` 可以用 mtime cache, 但 SQL safety 比 mtime 重要 |
| `impl_type_checking!` per backend | 无 | multi-backend 一致 type 映射的样板 |
| `PoolInner` + `AsyncSemaphore` | `aiosqlite` 无 pool | 真生产 pool 必备, 用 sqlx `Pool<Sqlite>` |
| SQLite dedicated thread + flume channel | 当前 asyncio | **保留!** C FFI 强制约束, Rust 也不得不用 worker thread |
| `proc_macro::tracked::path` + cargo metadata 集成 | 无 | cargo build script + env 集成是 Rust 独有 |
| `VirtualStatement` + `SqlSafeStr` 类型 | `aiosqlite` 字符串拼接 | Rust 类型系统防止 SQL injection |

---

## delta-rs state 管理 — 代码层细节

**核心文件**: `delta-rs/crates/core/src/kernel/snapshot/{mod.rs, stream.rs, iterators.rs, scan_row.rs}` + `delta-rs/crates/core/src/kernel/transaction/{mod.rs, application.rs, conflict_checker.rs, protocol.rs, state.rs}` + `delta-rs/crates/core/src/protocol/{checkpoints.rs, log_compaction.rs}` + `delta-rs/crates/core/src/logstore/{mod.rs, default_logstore.rs}` + `delta-rs/crates/core/src/table/{mod.rs, state.rs}`. 工作目录 `delta-rs/crates/core/src/`.

### 架构 — Snapshot 双层 + LogStore trait + CommitBuilder 流程

1. **`Snapshot` (lazy) vs `EagerSnapshot` (eager)** (kernel/snapshot/mod.rs 注释: "most data is loaded on demand and only Protocol + Metadata cached" vs "much more log data is eagerly loaded"). 这是 lakehouse 的核心: 不同 query 类型用不同 snapshot, 大表读 lazy, 元数据查 eager. **借鉴到 DGM archive: working set = lazy snapshot, full archive = eager snapshot**.
2. **Log 物理布局**: `_delta_log/{version:020d}.json` 追加写 + 周期 `_delta_log/{version:020d}.checkpoint.parquet` 快照 + `log_compaction` sidecars (log_compaction.rs). 文件名 regex: `_delta_log/(\d{20})\.(checkpoint).*$`. **20 位数字 = u64 max 版本号** — 用 20 位 zero-pad 而非随便多少位, 保证字典序 == 数值序, **关键洞察**: file system 上 list files 按字典序, 所以 `_delta_log/00000000000000000019.json` 排在 `00000000000000000020.json` 之前.
3. **`LogStore` trait** (logstore/mod.rs 注释核心) — 表级正确性依赖的 3 个保证: **(a) Atomic visibility** — partial write 不可见, **(b) Mutual exclusion** — 一次只有一个 writer 写特定 log file, **(c) Consistent listing** — 一旦写入, 后续 list 必须立即返回. 注释警告: "most object stores today provide the required guarantees, the specific locking mechanics are a table level responsibility". S3 / Azure Blob / GCS 通过 atomic rename 实现. 这是把分布式存储语义显式表达在 trait 接口上的设计.
4. **`CommitBuilder → PreparedCommit → FinalizeCommit → PostCommit → FinalizedCommit`** (kernel/transaction/mod.rs 顶部 ASCII 图) — **PreparedCommit = 临时 commit marker 写到 storage, FinalizeCommit = 原子 rename 尝试, 失败则 conflict resolution + 重试**. 这是 optimistic concurrency control + 2PC 的简化版. **关键洞察: 即使 object store 提供 atomic rename, 网络分区也可能失败, 所以必须 conflict check**.
5. **`Checkpoint` 流程** (protocol/checkpoints.rs `create_checkpoint_for`) — `Snapshot::builder_for(table_root).at_version(version).build(engine)` → `snapshot.checkpoint(engine, None)`. 后台 thread pool + `spawn_blocking_with_span` (注: tokio executor 集成) — delta-rs **显式用 tokio** (`delta_kernel_default_engine::executor::tokio::{TokioBackgroundExecutor, TokioMultiThreadExecutor}`), 不是自建 runtime.
6. **`Pin<Box<dyn Stream<Item = DeltaResult<RecordBatch>> + Send>>`** (`SendableRecordBatchStream` in snapshot/stream.rs) — DataFusion 兼容的 stream 类型. 注释说 "Stream of RecordBatchs that can be passed between threads" — `+ Send` 是关键 bound, Arrow 在多线程 executor 里流动. 这就是 Rust 异步生态的 `Send + 'static` 跨线程传递流模式.
7. **`ReceiverStreamBuilder<O>`** (snapshot/stream.rs) — `tokio::sync::mpsc::channel` + `tokio::task::JoinSet`. 比 `tokio_stream::ReceiverStream` 多两点: (a) bound channel, (b) **propagates panics**, (c) drop 时自动 cancel outstanding tasks. 注释: "tokio version doesn't propagate panics to the receiver".

### 关键算法 — Conflict resolution + protocol feature gating

1. **`CommitConflictError` 8 个 variant** (kernel/transaction/conflict_checker.rs) — `ConcurrentAppend` / `ConcurrentDeleteRead` / `ConcurrentDeleteDelete` / `MetadataChanged` / `ConcurrentTransaction` / `ProtocolChanged` / `UnsupportedWriterVersion` / `UnsupportedReaderVersion` / `CorruptedState`. 每个 error 注释解释了**何时发生 + 如何解决**: "isolation level can be set to Snapshot Isolation" 表明支持 `Serializable` (默认) 和 `SnapshotIsolation` 两级. **这是 Serializable Isolation 的实现成本 — 8 种冲突显式建模**.
2. **`IsolationLevel` 决定 conflict 强度** (kernel/transaction/conflict_checker.rs `use delta_kernel::table_properties::IsolationLevel`) — Serializable = 严格冲突检查, SnapshotIsolation = 弱化某些 read-after-write 检查.
3. **Protocol version = reader/writer feature set** (kernel/transaction/protocol.rs L18-50): `READER_V2 = {ColumnMapping}`, `READER_V3 = {DeletionVectors}`, `WRITER_V2 = {AppendOnly, Invariants}`, `WRITER_V3 += {CheckConstraints}`, `WRITER_V4 += {ChangeDataFeed, GeneratedColumns}`, `WRITER_V5 += {...}`. 静态 `HashSet<TableFeature>` 定义每版本能用的 feature. 这是 **schema evolution + feature flag** 的两难 — 协议版本锁定能用的能力子集.
4. **`AddContainer<'a>` + `PruningPredicate`** (kernel/transaction/state.rs L25-50) — 在 commit 时**用 DataFusion 谓词下推** prune files: `get_prune_stats(column, get_max)` 返回 partition 统计. **关键: 这是用 query engine 做 conflict check, 不只是 hash 比较** — 例如两个 transaction 都改 `id=5` 的 row 时, 通过 stats 决定要不要 reject.
5. **`ScanRowOutStream<S>` `#[pin_project]`** (kernel/snapshot/iterators/scan_row.rs) — **lazy stat materialization**, 注释 `try_new_with_materialization` 接受 `FileStatsMaterialization` 策略. 用户选 `None` / `RawJson` / `Parsed` — trade-off 内存 vs 后续 query 速度.
6. **`LastCheckpointHint`** (protocol/checkpoints.rs) — `_last_checkpoint` 文件存最近一次 checkpoint 的 (version, size) + 后续需 replay 的版本范围. 加载表时先读 hint, 然后只 replay hint 之后到最新之间的 commits — **避免每次都 replay 全部 log**.

### 借鉴清单 (机制不抄结构)

| delta-rs 机制 | Apeireth 当前 (Python) | 借鉴到 Rust 重写 |
|---|---|---|
| Snapshot (lazy) vs EagerSnapshot | `dgm_archive.py` 单层 | **直接借鉴**, working set = lazy, full = eager, `versions_seen` 矩阵做 deterministic replay |
| Log: `{version:020d}.json` + checkpoint parquet | 没有 log, 只 SQLite | DGM archive 改成 JSONL + checkpoint Parquet, 20 位零填充 = 字典序 == 数值序 |
| `LogStore` trait (atomic rename + consistent listing) | 无 | 单机 SQLite 也要抽象 `LogStore`, 便于未来换 S3/OSS |
| CommitBuilder → PreparedCommit → FinalizeCommit | `BEGIN; ... COMMIT;` 直接 | **不直接借鉴** — 单机 SQLite 用 `BEGIN IMMEDIATE` + WAL 足够, 不用 OCC. 但 conflict 分类 (8 种) 借鉴到我们的 state merge |
| `CommitConflictError` 8 variant + IsolationLevel | 无冲突概念 | 我们 DGM archive 可以借鉴: append-only vs mutability 二级 |
| Protocol version + TableFeature set | 无 | 借鉴到 schema versioning: data 格式按版本迁移, feature flag 控制 |
| `SendableRecordBatchStream = Pin<Box<dyn Stream + Send>>` | `asyncio.Queue` | Rust 跨线程流标准模式, 但**我们不需要** Arrow, 直接用 `Pin<Box<dyn Stream<Item = T> + Send>>` |
| `ReceiverStreamBuilder` (panic propagate + cancel on drop) | `asyncio.create_task` | 用 tokio + JoinSet, 比 Python 更难漏 panic |
| `PruningPredicate` commit-time pruning | 无 | 不借鉴 — 我们 archive 量级不大, 不需要 datafusion 集成 |
| `LastCheckpointHint` (避免 full replay) | 无 | **借鉴** — `memory_3tier` 加 checkpoint hint, 启动时只 replay 增量 |

---

## Apeireth 未来 Rust 重写借鉴清单 (表格)

| 借鉴点 | 来源 | 借鉴强度 | 借鉴时机 | 不借鉴 / 备注 |
|---|---|---|---|---|
| Async runtime (`tokio::runtime::Builder::new_multi_thread()`) | tokio | **强** | Phase 1 重写 | 替代 asyncio + aiohttp + httpx |
| Work-stealing scheduler + LIFO slot | tokio | 中 | Phase 1 | 默认行为, 不必显式配置 |
| `block_in_place` / `spawn_blocking` 分离 | tokio | **强** | Phase 1 | 当前 `asyncio.to_thread` 没分离 |
| `sqlx` compile-time SQL check | sqlx | 中 | Phase 2 | 真 safety 但**不**完全 (只查 prepared), 慎用 |
| SQLite `Pool<Sqlite>` + `AsyncSemaphore` | sqlx | **强** | Phase 2 | 替代 `aiosqlite` 简易 pool |
| SQLite dedicated thread per connection | sqlx (FFI 约束) | **必须保留** | Phase 2 | C FFI 强制, Rust 也不能 workaround |
| `StatementCache` + `SqlSafeStr` 类型 | sqlx | 中 | Phase 2 | 类型系统防 SQL injection |
| Snapshot 双层 (lazy/eager) | delta-rs | **强** | Phase 3 (DGM archive) | 直接借鉴架构 |
| Log: `{version:020d}.json` + 周期 checkpoint | delta-rs | **强** | Phase 3 | 借鉴文件命名 + checkpoint 策略 |
| `LastCheckpointHint` 避免全量 replay | delta-rs | **强** | Phase 3 | 借鉴到 `memory_3tier` 启动 |
| `CommitBuilder` 流程 (Prepared → Finalize) | delta-rs | 弱 | Phase 3 | 单机 SQLite 不需要, 用 WAL + BEGIN IMMEDIATE |
| `CommitConflictError` 8 variant 分类 | delta-rs | 中 | Phase 3 | 借鉴分类法, 不直接抄错误类型 |
| `IsolationLevel` (Serializable vs Snapshot) | delta-rs | 中 | Phase 3 | 借鉴二级冲突强度 |
| `SendableRecordBatchStream` Arrow 流 | delta-rs | 弱 | 不需要 | 我们不需要 Arrow/DataFusion, 用普通 `Stream + Send` |
| DataFusion query engine 集成 | delta-rs | **不** | — | 量级不够, 不引入依赖 |
| `loom` systematic concurrency testing | tokio | 中 | Phase 1+ | 关键并发原语用 loom 测 |
| `proc_macro` + cargo metadata 集成 | sqlx | 弱 | 不需要 | 我们不是 SQL macro |

---

## 推荐 / 不推荐 (Rust crate 选型)

### Async runtime

| 选项 | 推荐 | 理由 |
|---|---|---|
| **tokio** | ✅ **强烈推荐** | 生态标准 (axum/hyper/tonic/sqlx 全基于 tokio), work-stealing 已优化到极致, 多线程 + 单线程混合 runtime |
| async-std | ❌ 不推荐 | 生态分裂, 大部分 crate 不支持, 2026 实际使用率 < 5% |
| smol | ❌ 不推荐 | 小巧但缺生态, async 体验不一致 (block_on 不返回, 需手动) |
| glommio (thread-per-core) | ⚠️ 特殊场景 | 真 thread-per-core 但只 Linux, 不适合需要大量 spawn 的 AI agent |

### Async SQL

| 选项 | 推荐 | 理由 |
|---|---|---|
| **sqlx** | ✅ **推荐** | 唯一带 compile-time SQL check (可选), 多 backend, tokio 原生, 真生产 (Discord/Lambda) |
| rusqlite | ❌ 不推荐 (但作为 fallback) | 同步 + 没 pool, 只适合脚本; 但 sqlx-sqlite 底层用 libsqlite3-sys 也通过 FFI |
| diesel (sync + async) | ⚠️ 仅当复杂 ORM 需求 | compile-time SQL check 也支持, 但 ORM 抽象重, 我们不需要 |
| sea-orm | ⚠️ 同上 | 基于 sqlx, 但我们不需要 ActiveRecord pattern |
| tokio-postgres + deadpool-postgres | ⚠️ 同等 sqlx-postgres | 等价, sqlx 更统一 |

### State / 持久化

| 选项 | 推荐 | 理由 |
|---|---|---|
| **SQLite + sqlx** | ✅ **主力** | DGM archive / memory_3tier 用 SQLite 真生产够用, WAL + BEGIN IMMEDIATE 解决并发 |
| **delta-rs (parquet + log)** | ✅ **借鉴模式, 不直接用 crate** | 借鉴架构 (snapshot 双层 + log + checkpoint), 不引入完整 crate (我们不需要 DataFusion / S3) |
| sled (embedded KV) | ❌ 不推荐 | 作者 archived (2024), 维护停滞 |
| rocksdb-rs | ⚠️ 高写入场景 | 真 C++ binding, 安装难, 但量大时是 benchmark king |
| redb | ⚠️ 嵌入式 KV 备选 | 纯 Rust, 比 sled 新, 但生态没 sled 大 |
| lmdb-rs | ⚠️ 只读多场景 | mmap + 只读极快, 但不适合写多 |
| heather (append-only log) | ⚠️ log-only 场景 | 借鉴了 delta-rs 的 log 模型, Rust 原生 append-only KV |

### 其他

| 需求 | 推荐 | 理由 |
|---|---|---|
| HTTP client | `reqwest` | tokio 生态标准, 中间件支持 |
| JSON | `serde` + `serde_json` | 事实标准 |
| DateTime | `time` 或 `chrono` | `time` 更 pure, `chrono` 生态更广 |
| Logging | `tracing` + `tracing-subscriber` | structured + async-aware |
| Error handling | `thiserror` (库) + `anyhow` (应用) | 标配 |
| Async channels | `tokio::sync::mpsc` | 多消费者用 `broadcast` |
| Testing | `loom` (并发) + `proptest` (property) + `mockall` (mock) | Rust 独有 systematic testing |
| Build script | `vergen` + `cargo_metadata` | build-time 信息注入 |

---

## 调研哲学自检 (主 12:07 + 主 23:28 + 主 14:27)

- **不只 README**: 三个 repo 都读了真实 src/ 文件 (tokio worker.rs 1494 行 / queue.rs 523 行 / idle.rs 完整; sqlx query/mod.rs 219 行 / cache.rs 100 行 / data.rs 135 行; delta-rs snapshot/mod.rs 3553 行 / transaction/mod.rs 1386 行 / conflict_checker.rs 1200 行). 看真 Rust 代码 + 注释 + unsafe 块 + memory ordering 标注.
- **不抄结构**: 每个借鉴都标注了"借鉴强度"和"不借鉴"理由, 没把 tokio 的 scheduler 整套搬过来, 也没把 delta-rs 的整个 lakehouse 搬过来.
- **推荐 / 不推荐都给理由**: 16 行选型表每个都有 1-2 行 reason, 包括 ecosystem health / maintenance / FFI constraint.
- **不写 Rust 代码, 只读 + 报告**: 全程没写一行 Rust, 目标是"为未来 Rust 重写准备", 不是当下动笔.
- **实事求是**: 标了 sqlite dedicated thread 是 FFI 强制约束 (不是设计选择), 标了 `loom` 是 tokio 独有 systematic testing, 标了 delta-rs 用 tokio executor 不是自建 runtime.

## 不确定性 / 下一步调研候选

1. **hyper / axum** 的 tokio 集成模式 (HTTP server in Rust) — 真生产
2. **tonic** (gRPC) 的 streaming + interceptor 模式 — 适合 AI agent RPC
3. **diesel** 的 query builder (sqlx 之外的 compile-time check 选项) — 备选方案
4. **redb / heather** 源码 (替代 sled 的纯 Rust embedded KV) — Phase 3 state 备选
5. **candle** (huggingface Rust ML inference) — 主任务替代 delta-rs 但没时间读, Phase 4 LLM 推理
6. **ratatui / crossterm** (TUI) — 如果 Apeireth 做 CLI 交互
7. **quickwit / tantivy** 全文搜索 (Rust 真生产) — memory search 借鉴

---

## 关键洞察 (5-10 行摘要)

1. **tokio 的 work-stealing + LIFO slot + block_in_place 是 async runtime 的"工业答案"** — Apeireth 重写直接用 `tokio::runtime::Builder::new_multi_thread()`, 不必自己造轮子.
2. **sqlx 的真正创新是 compile-time SQL check + MtimeCache + per-backend type_checking trait** — compile-time check 有用但不完美, 真生产可靠靠 `Pool<Sqlite>` + dedicated thread + statement cache.
3. **SQLite 在 Rust 里必须 dedicated thread per connection** (FFI 约束) — 不能直接 `Send`, 这是 `libsqlite3-sys` 的限制, Python asyncio 也类似但 Rust 显式建模.
4. **delta-rs 的 Snapshot 双层 (lazy/eager) + JSONL log + 周期 checkpoint + LastCheckpointHint 是真生产 state 管理答案** — DGM archive 直接借鉴架构, 不必用完整 delta-rs crate.
5. **delta-rs 8 种 `CommitConflictError` 变体**说明 Serializable Isolation 的实际成本 — 我们 DGM archive 用 Snapshot Isolation (弱化) + WAL 就够, 不必抄 8 种错误.
6. **tokio 用 `crate::loom::sync::*` 替代 `std::sync::*`** — Rust 独有的 systematic concurrency testing, key concurrency primitive 借鉴.
7. **delta-rs 用 tokio 作为 executor** (`TokioMultiThreadExecutor`) — 即使 lakehouse 重也用现成 tokio, 不自建 runtime. **结论: tokio 是 Rust 生态的事实标准**.
8. **20 位 zero-pad 文件名让字典序 == 数值序** — delta-rs 文件命名的小细节, 我们 memory_3tier / DGM archive 借鉴: `{epoch_ms:013d}.jsonl` 或 `{commit_seq:020d}.json`.
9. **Rust 异步生态的 `Pin<Box<dyn Stream + Send>>` 跨线程流**是 DataFusion 的基础, 我们不需要 Arrow, 用 `Pin<Box<dyn Stream<Item = T> + Send>>` 即可.
10. **三个 repo 都用 procedural macros / const generics / trait dispatch 表达多态** — Rust 重写要充分利用 type-level programming, 而不是字符串配置.