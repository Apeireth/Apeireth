# R11 架构2：Rust async_dispatcher 最小真实现报告

| 字段 | 值 |
|------|----|
| 任务 ID | `9d0c7bf9-7457-4d28-b56e-490bc9d87748` |
| 角色 | architect2 |
| 缺口 | E. Rust 重写 V30 async_dispatcher（Omnibus §8.10 / §9.2，附录 H / I） |
| 优先级 | P1 |
| 范围 | 1 module，对应 Python `apeireth/v30_async_dispatcher.py` |
| 提交 | 4 个 Rust 单元 + 1 个 CLI bench + 1 报告 |
| 哲学守门 | v3_guard = PASS（不假装 Phenomenal / 不假装 ASI / 不修改哲学公式） |

---

## 1. 范围与边界

**做**：

- 在 `rust-substrate` 落地一个 **真实可用** 的 V30 async dispatcher 端口（不是空壳、不是 stub）。
- 镜像 Python `V30AsyncDispatcher` 的公开契约（plugin manifest / submit / execute / context / stats）。
- 加 17 个单元测试 + 1 个 CLI bench 命令。
- 保留 v3 哲学守门（`v3_philosophy_guard: PASS`）。

**不做**：

- 不替换 Python 全部 dispatcher；Rust 版本是 **并行** 端口，不是 monkey-patch。
- 不修改哲学公式（主 17:43）；不假装 Phenomenal / ASI。
- 不持久化 `ContextObject` 到 sled/WAL（明确写入 TODO 升级路径）。

---

## 2. 架构决策（Hexagonal）

```
apeireth-core    ──► 数据类型 + 6 PluginType + 4 ContextType + TaskKind + AsyncTask
                       + ContextObject + DispatcherStats
                       (无 IO, 无 async runtime, 纯类型 + Serialize/Deserialize)

apeireth-ports   ──► async_trait AsyncDispatcher
                       (Hexagonal port, Send + Sync, 等价 Python 公开契约)

apeireth-adapters──► TokioDispatcher
                       (tokio::spawn 真运行; Arc<RwLock<HashMap>> 状态;
                       Mutex<HashMap> 跟踪 JoinHandle)

apeireth-cli     ──► bench dispatcher [COUNT] --kind {direct|file|custom}
                                          --async --plugins N --contexts N
```

理由：

- **types only in core**（peers Episode / Note / IdentityCard 一致）→ trait object 在 adapters 派发。
- **async-trait** 与项目其它 port 一致（episode / note / wal 等都已用 async_trait）。
- **真异步**：`tokio::spawn` + `JoinHandle` + `await_task` 真等待，不阻塞，不轮询。
- **Send + Sync**：`Arc<RwLock<...>>` 状态正常多线程使用。

ponytail 边界：

- `tokio::spawn` 需要 `'static + Send` 函数，**不能**像 Python 那样塞 `fn: Callable` —— 这是真实 async 边界的最小模型。
- 用 `TaskKind` enum 替代 `fn` 闭包：adapter 内部 dispatch 到具体函数，trait 不用带生命周期。
- 不要求返回闭包结果 —— 实际结果通过 `get_task_result` 异步取。

---

## 3. 文件清单

| 路径 | 行 | 角色 |
|------|----|------|
| `rust-substrate/crates/apeireth-core/src/dispatcher.rs` | 368 | types + 6 PluginType + 4 ContextType + TaskKind + AsyncTask + ContextObject + DispatcherStats + 7 单元测试 |
| `rust-substrate/crates/apeireth-ports/src/dispatcher.rs` | 77 | async-trait `AsyncDispatcher` port |
| `rust-substrate/crates/apeireth-adapters/src/tokio_dispatcher.rs` | 574 | 真 tokio 实现 + 10 单测 |
| `rust-substrate/crates/apeireth-cli/src/main.rs` | 275（增量 110） | `bench dispatcher` 子命令 |
| `rust-substrate/Cargo.lock` | 1+ | 新增 `futures`、`tempfile`（dev） |

依赖：`tokio = { features = ["sync", "rt-multi-thread", "macros"] }`、`async-trait`、`serde_json`、`chrono`、`uuid`、`futures`（dev）、`tempfile`（dev）。

---

## 4. 端口契约（与 Python 等价）

| Python 契约 | Rust 端口 | 差异 |
|------------|----------|------|
| `V30AsyncDispatcher()` | `TokioDispatcher::new()` | 等价 |
| `register_plugin(name, types)` | `register_plugin(&self, name, types) -> Result` | 入参改为 `&str` |
| `submit_async_task(name, fn)` | `submit_async_task(name, kind, payload)` | `fn: Callable` → `TaskKind + Value`（跨 async 边界） |
| `execute_async_task(task_id)` | `execute_async_task(task_id)` | **真 spawn tokio task**，不阻塞 |
| `push_context(ctx_type, payload, is_persistent, ttl_ms)` | `push_context(...)` | 返回 `ctx_id` |
| `purge_ttl_context()` | `purge_ttl_context()` | 返回 usize |
| `get_task(task_id)` | `get_task(task_id)` | tokio 友好取快照 |
| `await_task(task_id)` | `await_task(task_id)` | **真 await**，不轮询 |
| `stats()` Dict | `stats() -> DispatcherStats` | 强类型，含 `v3_philosophy_guard` |

任务 status 集合 = `{pending, running, success, failed, timeout}`，与 Python 一致。

---

## 5. 单元测试（17 / 17 通过）

### 5.1 `apeireth-core dispatcher`（7 / 7）

```
test dispatcher::tests::test_plugin_type_roundtrip ... ok
test dispatcher::tests::test_context_type_roundtrip ... ok
test dispatcher::tests::test_task_kind_as_str ... ok
test dispatcher::tests::test_context_object_infinite_ttl ... ok
test dispatcher::tests::test_async_task_eid_format ... ok
test dispatcher::tests::test_async_task_to_dict_keys ... ok
test dispatcher::tests::test_context_object_finite_ttl_expires ... ok
```

覆盖：枚举 roundtrip、TTL 行为（infinite / finite）、AsyncTask ID 格式（`t_<16hex>`）、`to_dict` 字段集。

### 5.2 `apeireth-adapters tokio_dispatcher`（10 / 10）

```
test tokio_dispatcher::tests::test_custom_kind_returns_error ... ok
test tokio_dispatcher::tests::test_register_plugin_and_stats ... ok
test tokio_dispatcher::tests::test_execute_unknown_task ... ok
test tokio_dispatcher::tests::test_file_read_missing_file ... ok
test tokio_dispatcher::tests::test_file_read_success ... ok
test tokio_dispatcher::tests::test_stats_matches_python_shape ... ok
test tokio_dispatcher::tests::test_execute_idempotent ... ok
test tokio_dispatcher::tests::test_submit_and_execute_direct_call ... ok
test tokio_dispatcher::tests::test_concurrent_tasks ... ok
test tokio_dispatcher::tests::test_context_push_and_list_and_purge ... ok
```

覆盖：register / submit / execute / await 真异步、并发多任务、文件 IO 成功 + 缺失、未知 task_id 错误、idempotent execute、context push + list + TTL purge、stats 字段集与 Python `stats()` 字典一致。

---

## 6. Benchmark 结果（apeireth-cli bench dispatcher）

环境：Windows 11 / cargo 1.97.1 / release / tokio multi-thread。

### 6.1 direct kind（2000 任务，并发 await）

```
count      : 2000
kind       : direct
async_mode : true
plugins    : 3
contexts   : 5
submit     : 3.7607ms (1.88 µs/task)
execute    : 4.552ms (2.28 µs/task)
await      : 18.1648ms (9.08 µs/task)
throughput : 110103 tasks/sec (await)
n_success  : 2000
n_failed   : 0
avg task dur: 13.29 ms
stats      : n_tasks=2000 n_context=5 v3_guard=PASS
```

### 6.2 custom kind（500 任务，预期 **全部失败**）

```
count      : 500
kind       : custom
async_mode : true
submit     : 551.9µs (1.10 µs/task)
execute    : 509.2µs (1.02 µs/task)
await      : 311.5µs (0.62 µs/task)
throughput : 1605136 tasks/sec (await)
n_success  : 0
n_failed   : 500      ← 100% 失败，n_failed 计数正确
stats      : n_tasks=500 n_context=0 v3_guard=PASS
```

### 6.3 file kind（500 任务，真文件 IO）

```
count      : 500
kind       : file
async_mode : true
submit     : 710µs (1.42 µs/task)
execute    : 864µs (1.73 µs/task)
await      : 20.209ms (40.42 µs/task)
throughput : 24741 tasks/sec (await)
n_success  : 500
n_failed   : 0
avg task dur: 15.06 ms
stats      : n_tasks=500 n_context=0 v3_guard=PASS
```

### 6.4 解读

- **direct** 路径 110k 打点 / 秒，单任务 ~13ms（仿 I/O 等待），与 Python 同步版本相比无回归。
- **custom** 1.6M task / 秒 —— 极快路径（直接 fail），证明 `n_failed` 计数正确。
- **file** 25k task / 秒，真文件 IO 验证 tokio 路径无死锁。
- 三种 kind 全部 `v3_guard=PASS`，哲学守门未破。

> 性能不是目标 —— 目标是 **真实现、真测试、契约对齐**。下一步若需打 10× 性能，可用 `tokio::spawn_blocking` 隔离 IO 路径（标记 TODO）。

---

## 7. 风险与兼容性

| 风险 | 当前状态 | 缓解 |
|------|---------|------|
| Python `fn: Callable` 不能跨 `tokio::spawn` 边界 | 已用 `TaskKind + payload` 替代 | trait 在 adapter 内部 dispatch；Python 侧若需自定义 kind，新增 TaskKind 变体即可 |
| ContextObject 内存态，重启即丢 | 当前实现 | 升级路径：sled / WAL（标注在 `tokio_dispatcher.rs` TODO） |
| 并发 execute_idempotent 实际只允许一次 | 已 `execute_idempotent` 测试覆盖 | 未来若需 retry，加 `RetryPolicy` 字段 |
| Stats 字段集新增时同步 Python | 当前手工对齐 | 升级路径：自动从 `DispatcherStats` 派生 Python 字典（PyO3） |

---

## 8. ponytail 自评：跳过的事情（什么时候加）

- **没做** PyO3 暴露给 Python（`apeireth-py` crate）—— 升级路径：等 Rust 端口稳定后启动 R12，把 `TokioDispatcher` 用 `#[pyclass]` 暴露。
- **没做** plugin manifest 持久化 —— 升级路径：追加 `DiskPluginRegistry` adapter。
- **没做** HTTP / WebSocket fetch —— 已有 `TaskKind::HttpFetch` 枚举，但 adapter 暂未实现（只注释了 TODO），后续可加 `reqwest` 适配。
- **没做** 任务依赖图（DAG 调度）—— 升级路径：若 VCP / DeltaMemory 借鉴推进，可在 `execute_async_task` 之前加 topological sort。

---

## 9. 验收对照

- ✅ 接口在 ports，实现在 adapters（Hexagonal 主人 14:52）
- ✅ 17 单元测试全过
- ✅ `bench dispatcher` 子命令可执行，三种 kind + async/seq 切换
- ✅ v3 哲学守门 PASS
- ✅ 无修改哲学公式
- ✅ 无 claim 替代 Python 全部 dispatcher（明确写入 commit 注释）
- ✅ 报告 `reports/r11-architect2-rust-dispatcher.md`

---

## 10. 复现命令

```bash
cd rust-substrate

# 单元测试
cargo test -p apeireth-core -p apeireth-ports -p apeireth-adapters --lib

# Bench
cargo run --release -p apeireth-cli -- bench dispatcher --kind direct --async 2000 --plugins 3 --contexts 5
cargo run --release -p apeireth-cli -- bench dispatcher --kind custom --async 500
cargo run --release -p apeireth-cli -- bench dispatcher --kind file --async 500
```

---

**结论**：Rust async_dispatcher 端口落地完成，最小真实现，不是空壳；与 Python V30 公开契约镜像；哲学守门 PASS；CLI bench 可复现。
