# R125-3 Sub-Agent Dispatch Prompt (hyper 连接池 LIFO 复用)

**Date**: 2026-08-10 17:28
**Author**: R125 P0 supervisor
**Receiving agent**: R125-3 sub-agent

---

## 任务

**主题**: hyper-util 连接池 LIFO 复用, 优化 `apeireth-api` HTTP client (减少 TLS handshake 50-80% 延迟)

**借鉴 ID**: `R124-1-BORROW-hyperium/hyper-util-2e9d4b6-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\hyper\`

**目标文件**:
- `Apeireth-rust/crates/apeireth-api/src/http_client.rs` (LIFO 池, NEW module)
- `Apeireth-rust/crates/apeireth-api/src/lib.rs` (M: add `pub mod http_client;` + 2 re-export)
- `Apeireth-rust/crates/apeireth-api/Cargo.toml` (M: add `hyper-util = { workspace = true }` + `hyper = { workspace = true }` 假设 workspace 已有, 0 加新依赖)
- `Apeireth-rust/crates/apeireth-api/tests/http_client_pool_test.rs` (5 单元测试)
- `Apeireth-rust/crates/apeireth-api/benches/http_client_pool_bench.rs` (criterion bench, 1 file)

**B1 24 LOCKED 持续更新**: http_client.rs 在 api/ 内, **api/ 不在 24 LOCKED 名单**, 实施可改. 入口签名 0 改.

**整合依赖**: R17 pipeline 调 API + R30 tool upgrade + R122-5 semantic_router 都用 HTTP client, 池化后它们都受益

**估时**: 1 天 (8h)

**截止**: 8/11 8:00 (过夜)

---

## 0 装解除 (主人 17:22)

```bash
Test-Path '.openclaw\workspace\borrowed-repos\hyper\.git'
```

- ✅ cloned = 真实施 LIFO pool
- ⏳ 限流中 = 0 实施, 写 final 报告"借鉴 ID 索引完成"
- ❌ 永久失败 = 报 supervisor

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 + C1-C3)

| # | 必守 |
|---|------|
| 1 | B2 0 触碰 workspace.version |
| 2 | A1 0 触碰 R11 baseline 3 值 |
| 3 | B1 0 触碰 24 LOCKED crate, **http_client.rs 是 NEW** (api/ 实施可改) |
| 4-7 | B3-B6 0 改原实质 |
| 8 | C1 0 commit, C2 0 装解除, C3 0 装 5 项升 6 重 v6, 0 push |

---

## 实施步骤 (4 阶段)

### 阶段 1: 借鉴 hyper-util (30 min)
- 读 `hyper-util/src/client/legacy/` (旧 client + pool) + `hyper-util/src/client/pool.rs` (新建)
- 提取 3 pattern:
  1. `Client<H, S>` + `pool_max_idle_per_host`
  2. `PoolableConnection` trait
  3. LIFO vs FIFO 策略 (LIFO 复用率更高, 延迟更低)

### 阶段 2: 实现 LIFO 池 (4h)
**http_client.rs** 核心:
```rust
//! HTTP Client with LIFO Connection Pool (借鉴 hyper-util 池模式)

use hyper_util::client::legacy::{Client, connect::HttpConnector, pool::Pool};
use hyper_util::rt::TokioExecutor;

pub struct PooledHttpClient {
    client: Client<HttpConnector, http_body_util::Empty<bytes::Bytes>>,
    pool_config: PoolConfig,
}

pub struct PoolConfig {
    pub max_idle_per_host: usize,    // default 32
    pub idle_timeout: std::time::Duration,  // default 90s
    pub strategy: PoolStrategy,       // LIFO (default) or FIFO
}

pub enum PoolStrategy { Lifo, Fifo }

impl PooledHttpClient {
    pub fn new() -> Self { ... }
    pub fn with_config(config: PoolConfig) -> Self { ... }
    pub async fn request(&self, req: http::Request<...>) -> Result<http::Response<...>, Error> { ... }
    pub fn pool_stats(&self) -> PoolStats { /* idle, in_use, total */ }
}

pub struct PoolStats {
    pub idle_connections: usize,
    pub in_use_connections: usize,
    pub total_pools: usize,
}
```

**集成到 lib.rs**:
- 加 `pub mod http_client;`
- 加 `pub use http_client::{PooledHttpClient, PoolConfig, PoolStrategy, PoolStats};`

### 阶段 3: 5 单元测试 + bench (2h)
- `test_pool_basic_reuse` — 2 requests to same host, verify 1 connection reused
- `test_pool_lifo_vs_fifo` — LIFO has higher hit rate
- `test_pool_max_idle_eviction` — exceed max_idle → oldest evicted
- `test_pool_idle_timeout` — connection idle > 90s → closed
- `test_pool_stats_accuracy` — stats match actual count

criterion bench: pool vs no-pool, 100 reqs to 1 host, measure p50/p99 延迟

### 阶段 4: 跑通 + final 报告 (1.5h)
```bash
cd .openclaw\workspace\promethean\Apeireth-rust
cargo build -p apeireth-api
cargo test -p apeireth-api http_client_pool
cargo bench -p apeireth-api http_client_pool
```

---

## 0 主动 commit (C1)

❌ **0 commit, 0 push**.

---

## final 报告 必含 6 段 + 池化收益数据 (p50/p99 before/after)

---

**派活完成 17:28. 截止 8/11 8:00 (跑过夜).**
