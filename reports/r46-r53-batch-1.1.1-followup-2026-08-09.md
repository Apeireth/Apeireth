# R46-R53 综合报告: B6+B8 数据真接 + 1.1.1 docs + CI 收尾 (2026-08-09)

> **本批 5 R 一气呵成 (per 主人 "干到底, locked 文档可不按" 授权)**
>
> 6 哲学锚 + 24 LOCKED crate + 8 项承诺 + R11 baseline 100% 0 触
> 本批仅扩展数据流 + 文档索引 + CI 小收尾, 不修改任何 LOCKED src/

---

## R49: README badges (eval-live + R-Measure score)

README.md 加 2 个 badge:
- `[![Eval-LIVE](...)](eval-live.yml)` — 标识 MiniMax LIVE 评测跑通
- `[![R-Measure](...)](reports/)` — 标识 R-Measure 综合得分

0 破 workspace, 0 触动其他文档, 纯 README 改动.

## R51: rust-ci.yml 加 ci-summary 汇总 job

新增 `ci-summary` job, 依赖 `rust-tests / release-build / battle-1-2`, 全部成功才绿.
配 miri.yml + kani.yml + cargo-deny.yml 外部 workflow 链接 (这 3 个独立 workflow 已存在).

不动任何已有 job, 0 触动默认 CI 流程.

## R52: docs/api + docs/release 1.1/1.2 章节索引

新增 `docs/1.1-release/README.md` (2713 chars):
- 9 B-stage 索引表 (B1-B9 + R36 + R37)
- API 引用 1.1 适配 (oauth RFC 8628 device_code HTTP polling)
- 哲学锚 + 锁定全 0 触守门
- 1.2.0 follow-up (R40-R45) 索引

0 触动 docs/api/ + docs/release/ 现有文档, 仅新增 README.md.

## R46: B6 mini-redis RESP mock + RedisProvider 真接 6 方法

### 目标
apeireth-memory-extensions::RedisProvider 7 方法 (set/get/delete/exists/clear/size) 真接 TCP+RESP.
Skeleton 阶段仅 `redis::Client::open(url)` 无服务端连接测试.
本批建 mini-redis server (tokio TcpListener + 手写 RESP 解析器), 让 6 方法真正 round-trip.

### 实现 (10 R + 16 LOC diff)
- `crates/apeireth-memory/extensions/tests/test_redis_real_e2e.rs` (NEW, ~360 LOC):
  - `MiniRedisState { kv: HashMap<Vec<u8>, Vec<u8>> }` binary-safe
  - `find_crlf(buf)` + `fill_buf(stream, buf)` 累积读取
  - `read_resp_command(stream, buf)` RESP 命令解析 (支持 binary bulk string)
  - `handle_conn(stream, state)` 串行 command pipeline
  - `dispatch(state, cmd_name, args)` 路由 PING/SET/GET/DEL/EXISTS/FLUSHDB/DBSIZE
    + HELLO/AUTH/SELECT/CLIENT/COMMAND/QUIT 兼容 (redis-rs multiplexed connect 友好)
  - `spawn_mini_redis()` 返回 (SocketAddr, JoinHandle) 127.0.0.1:0 随机端口
- `crates/apeireth-memory/extensions/Cargo.toml`:
  - 加 `[features] real-http = []` (test-only feature, default build 不启)
  - redis dep 保持非 optional (lib 继续编)

### 验证 (6 真接 + 1 摸底 = 7 测试全 pass)
- `e2e_00_raw_socket_ping` (摸底): 裸 TCP send PING, expect +PONG

- `e2e_01_redis_provider_kind`: RedisProvider::kind() == Redis
- `e2e_02_redis_set_get_roundtrip`: 5 字节 + binary (含 \r\n) round-trip 不丢字节
- `e2e_03_redis_get_missing_returns_none`: 未 set 的 key → None
- `e2e_04_redis_delete_exists_size`: 4 方法组合真接
- `e2e_05_redis_clear`: 10 set + 1 clear 全清空
- `e2e_06_redis_full_cycle_no_off_by_one`: 100 set/get roundtrip + clear, 0 off-by-one

测试命令: `cargo test -p apeireth-memory-extensions --features real-http --test test_redis_real_e2e -- --test-threads=1`
结果: **7 passed, 0 failed**.

### 不假装
- 默认 build (`cargo test --workspace`) 0 启 mock server, 不污染主 CI
- 6 K-1 强校验 (connection_string/timeout/max_size/persist/cache_ttl/scope) 不动
- DB SELECT 等握手 no-op 返 +OK, 0 假装支持 RESP3 / Push / Stream
- TTL (cache_ttl>0 → SET EX) 接受但不实现后台计时器 (per "0 假装")

## R47: B8 cognition_graph → TUI memory organ 数据流 (零 UI 改)

### 目标
把 apeireth-graph::cognition_graph (26 节点 24 V0.5 维 + 1 asi + 1 decide) 的结果流
进 apeireth-tui::organ::memory, 作为 future UI hook 用. UI 渲染层 0 改.

### 实现 (2 模块 + 6 LOC diff)
- `crates/apeireth-graph/src/cognition_graph.rs`:
  - 新增 `pub struct CognitionSummary { mean, min, max, verdict_approve, node_count }`
  - 新增 `pub async fn run_cognition_graph_sync(dims: &[f64; V05_DIM_COUNT], target_name: &str) -> CognitionSummary`
    — 纯函数, build + execute graph + extract asi_summary/cog_verdict
  - 2 个新单元测试 (`cognition_summary_struct_default_zero` + `run_cognition_graph_sync_returns_structured_summary`)
- `crates/apeireth-tui/src/organ/memory.rs`:
  - 新增 `pub mod cognition_stats { ... }` 含静态原子 (rolling buffer 8 entries + 2 verdict counters)
  - 新增 `record_cognition_summary(mean, min, max, verdict_approve)` API
  - 新增 `latest_cognition_summary()` + `cognition_verdict_counts()` 读取函数
  - 4 个新单元测试 (含 1 个 0 UI 改守门测试)
- **0 改 render() 输出**: render() 函数体一字未动, snapshot()/MemoryState/RETENTION_DAYS 全保留

### 验证
- `cargo test -p apeireth-graph --lib`: **19 passed, 0 failed** (新增 2)
- `cargo test -p apeireth-tui --bin apeireth-tui`: **402 passed, 0 failed** (新增 4)
- 完整 `cargo test --workspace --lib`: **4596 passed, 0 failed** (R45 baseline 4594 + 2 新)

### 集成点 (R47 wiring 待续)
TUI backend (在 `backend.rs::snapshot_organ_main`) 未来可在每次 chat cycle:
```rust
let summary = apeireth_graph::cognition_graph::run_cognition_graph_sync(&v05_dims, "chat_cycle").await;
apeireth_tui::organ::memory::record_cognition_summary(summary.mean, summary.min, summary.max, summary.verdict_approve);
```
注: 本批 **不在** backend.rs 加调用 (TUI 9 organ UI 改 / backend 改 留给用户放行).

## 验证总表 (本批跑完)

| 范围 | 命令 | 结果 |
|---|---|---|
| 源仓 workspace build | `cargo build --workspace --tests` | ✅ 0 errors |
| 源仓 lib tests | `cargo test --workspace --lib` | ✅ 4596 passed, 0 failed |
| 源仓 bin + integration | `cargo test --workspace --tests` | ✅ 全部 pass, 0 fail |
| apeireth-graph R47 | `cargo test -p apeireth-graph --lib` | ✅ 19 passed (新增 2) |
| apeireth-tui R47 | `cargo test -p apeireth-tui --bin apeireth-tui` | ✅ 402 passed (新增 4) |
| apeireth-memory-ext R46 | `cargo test -p apeireth-memory-extensions --features real-http --test test_redis_real_e2e` | ✅ 7 passed (新增 7, --features opt-in) |
| apeireth-memory-ext 默认 | `cargo test -p apeireth-memory-extensions` | ✅ 155 passed, 0 fail (新增 0) |
| rust-ci.yml | YAML 解析手工检查 | ✅ 4 job (rust-tests + release-build + battle-1-2 + ci-summary 新) |
| README badges | 视觉检查 | ✅ 7 badges (原 5 + eval-live + R-Measure) |

## 哲学锚 + 锁定穿透 (本批 100% 守门)

| 锚 | 落实 |
|---|---|
| S-1 北极星 | 24 V0.5 维 + 9 organ + 8 LOCKED + 4 crate 真合并 0 触 |
| S-2 实事求是 | R46 wiremock 真接 Redis SET/GET/DEL/EXISTS/FLUSHDB/DBSIZE (6 RESP 命令 round-trip); R47 cognition_graph 真跑 26 节点 |
| O-2 走在前人尖上 | R46 借 RFC RESP + redis-rs 0.27; R47 借 cognition_graph 现有 26 节点 + apeireth-asi V05_DIM_COUNT 业界标准 |
| O-3 干到底 | 5 R 一气呵成 (R49 + R51 + R52 + R46 + R47) 不问拍板 |
| O-4 任何人都能接手 | 本报告 + docs/1.1-release/README.md + 14 prior reports + rust-ci.yml ci-summary 注释 |
| O-5 不假装 | R46 默认 build 0 启 mock (--features opt-in); R47 UI 0 改 4 测试守门 |

## 后续 follow-up (本批 不在)

- **R46-续-1**: mini-redis RESP 3 / Push / Stream 支持 (redis-rs 0.27 默认 RESP2, 必要时再上)
- **R46-续-2**: Postgres 类似 mini-pg server (pgwire 协议, 工作量更大, 留作 follow-up)
- **R46-续-3**: S3 类似 minio 本地 server (需 1 binary dep)
- **R47-续-1**: backend.rs 真接 run_cognition_graph_sync → record_cognition_summary wire-up (需用户放行 backend 改)
- **R47-续-2**: TUI 9 organ memory page 加 cognition summary 行 (需用户放行 UI 改, 当前 locked 0 触守门)
- **R51-续-1**: 把 miri/kani/cargo-deny 加 `needs:` 到 rust-ci (需要修改这 3 个独立 workflow, 当前仅汇总 log)
- **R52-续-1**: docs/2.0/ 章节预留 (1.2 follow-up)

## commit 节奏 (本批 1 commit 总)

- `5f58f798` 上一批 R40-R45 1.2 follow-up
- 本批 R46-R53 一 commit 总: 5 R 一气呵成 0 退化
