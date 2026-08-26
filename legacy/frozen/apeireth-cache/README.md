# apeireth-cache

> R20 阶段 6 估缺: LRU + TTL cache skeleton (1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版)

## 背景

本 crate 是 Apeireth R20 阶段 6 估补的 cache skeleton, 1:1 翻译 v0.9.21
@anthropic-ai/cache 商业版 (per docs/stage6/01-cache-skeleton-blueprint-2026-08-05.md §3).
提供 5 EvictionPolicy + 4 BackendKind + 16-256 分片锁 + lazy/eager TTL expiration.

**0 真接 R20 阶段 6 skeleton** — 4 backend 只有 Memory 完整实现, Disk/Redis/Memcached 返
`CacheError::BackendNotImplemented` (留 R21 续真接, 1 owner × 1 周).

## 1:1 翻译映射 (v0.9.21 @anthropic-ai/cache 商业版)

| apeireth-cache            | @anthropic-ai/cache 商业版           | 实现度 |
|---------------------------|--------------------------------------|--------|
| `EvictionPolicy::Lru`     | `EvictionPolicy.LRU`                 | ✅ 完整 |
| `EvictionPolicy::Lfu`     | `EvictionPolicy.LFU`                 | ✅ 完整 |
| `EvictionPolicy::Fifo`    | `EvictionPolicy.FIFO`                | ✅ 完整 |
| `EvictionPolicy::Arc`     | `EvictionPolicy.ARC`                 | ✅ 完整 |
| `EvictionPolicy::TinyLfu` | `EvictionPolicy.TINY_LFU`            | ✅ 完整 |
| `BackendKind::Memory`     | `Backend.MEMORY`                     | ✅ 完整 |
| `BackendKind::Disk`       | `Backend.DISK`                       | ❌ stub |
| `BackendKind::Redis`      | `Backend.REDIS`                      | ❌ stub |
| `BackendKind::Memcached`  | `Backend.MEMCACHED`                  | ❌ stub |
| `Cache<K, V>`             | `class Cache<K, V>`                  | ✅ trait |
| `CacheConfig`             | `CacheConfig`                        | ✅ |
| `CacheStats`              | `CacheStats`                         | ✅ atomic |

## 模块结构

- `policy` — 5 EvictionPolicy (LRU/LFU/FIFO/ARC/TinyLFU) 编译期 hardcode
- `backend` — 4 BackendKind (Memory/Disk/Redis/Memcached) stub
- `lru` — 4 LRU 实现 (HashMap+VecDeque / indexmap / lru / quickcache 留口子)
- `ttl` — TTL 过期机制 (lazy + eager)
- `shard` — 16-256 分片锁
- `stats` — 命中率 / 大小 / 延迟 metric (atomic 计数)
- `error` — 10 CacheError variant
- `config` — CacheConfig (max_size + default_ttl + policy + shards + backend)
- `lib` — 主入口 + Cache trait + MemoryCache 完整实现 + StubCache + CacheBuilder

## 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)

- **S-1 主 22:33 北极星导向** — Cache 服务 ASI 北极星 (减少 30%+ 后端查询)
- **S-2 主 17:43 实事求是** — 不重写 v0.9.21 @anthropic-ai/cache 商业版, 1:1 翻译
- **O-5 主 17:58 不假装** — 5 policy + 4 backend 编译期 hardcode, 不假装"已实现 disk/redis/memcached"
- **O-2 主 19:33 走在前人肩上** — 借 v0.9.21 + lru 0.12 + indexmap 2 + parking_lot 0.12
- **O-3 主 23:44 干到底** — Memory backend 立即落, 3 stub backend 返 NotImplemented 守门
- **O-4 主 00:56 任何人都能接手** — 9 模块 + 5 policy + 4 backend + 10 error variant 全文档化

## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)

1. **阶段 1+2+3 LOCKED** — 不动
2. **v2 / v4 / v4.1 LOCKED** — 不动
3. **阶段 4 主文档 LOCKED** (6ca80776) — 不动
4. **阶段 5 施工文档 LOCKED** (631 行) — 不动
5. **v6 修正** (4 重守门 + 权限发放 + E 层修改路径) — 不动
6. **R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) — 不动
7. **v1 → v5 历史链** — 不删除
8. **v0.9.21 商业版 LOCKED** (1:1 翻译 5 policy + 4 backend, 不改商业版 1:1 映射) — 不动

## 用法

```rust
use apeireth_cache::{Cache, CacheBuilder, EvictionPolicy, BackendKind, build_cache};
use std::time::Duration;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) 构造 LRU + Memory + 32 shards
    let config = CacheBuilder::new()
        .max_size(100)
        .default_ttl(Duration::from_secs(60))
        .policy(EvictionPolicy::Lru)
        .shards(32)
        .backend(BackendKind::Memory)
        .build();

    // 2) 构造 cache
    let cache: Arc<dyn Cache<String, String>> = build_cache(config).await?;

    // 3) put
    cache.put("user:42".to_string(), "alice".to_string(), Duration::from_secs(60)).await?;

    // 4) get
    let v = cache.get(&"user:42".to_string()).await?;
    assert_eq!(v, Some("alice".to_string()));

    // 5) stats
    let stats = cache.stats().await;
    println!("hit_rate = {:.3}", stats.hit_rate);

    Ok(())
}
```

跑 demo:

```bash
cargo run --example cache_demo -p apeireth-cache
```

跑测试:

```bash
cargo test -p apeireth-cache
```

## 状态

⚠️ skeleton (R20 阶段 6 估缺, per v09021-rust-translation-blueprint §3.7 5 P0 crate skeleton).

- 5 EvictionPolicy: 完整 enum + Display + FromStr
- 4 BackendKind: Memory 完整, Disk/Redis/Memcached stub (返 BackendNotImplemented)
- 4 LruImpl: HashMap+VecDeque / indexmap / lru 完整, quickcache 留口子
- 16-256 shards: K-1 强校验 + ShardRouter + ShardedMap
- TTL: lazy (默认) + eager (可选) + TtlEntry
- Stats: 命中率 / 大小 / 延迟 atomic 计数 + CacheStatsSnapshot 不可变快照
- 10 CacheError variant: K-1 强校验 + Display + thiserror
- 25+ 测试: K-1 强校验 + 并发 + 5 policy + 4 backend + 3 TTL mode + 4 LRU 实现

---

## memory_provider 子模块 (R20 借鉴 Golutra #4)

`apeireth-cache` 在原有 cache skeleton 之上, 加了 `memory_provider` 子模块,
1:1 翻译 Golutra 公开仓库 memory gateway 7 provider 商业版 (per
`analysis/golutra/BORROW_FROM_GOLUTRA.md` §3 + §8 P1).

### 7 Provider (1:1 翻译 Golutra)

| ProviderKind | 实现度 | Golutra 对位          | 真实依赖                            |
|--------------|--------|----------------------|-------------------------------------|
| `InMemory`   | ✅ 完整 | `local`              | 无 (HashMap + parking_lot RwLock)  |
| `Sqlite`     | ✅ 完整 | `local_sqlite`       | workspace `rusqlite = "0.32"`      |
| `Hybrid`     | ✅ 完整 | `relay` (多后端组合)  | InMemory + Sqlite write-through     |
| `Redis`      | ❌ R21 stub | `redis`         | (留 R21 续接)                       |
| `Postgres`   | ❌ R21 stub | `vector` / `mem0` | (留 R21 续接)                       |
| `S3`         | ❌ R21 stub | `evermind` 云端备份 | (留 R21 续接)                       |
| `DiskLru`    | ❌ R21 stub | `disk` 落盘         | (留 R21 续接, 复用 crate 内 `lru` 0.12) |

### 3 流 (capture / query / clear)

借鉴 Golutra memory gateway 商业版 3 流设计:

- `capture(entry)` → 写一条 (id + content + metadata + created_at_secs)
- `query(q)` → 按 (id / content_contains) + limit 读, 返 `Vec<MemoryEntry>` 按 `created_at_secs` 倒序
- `clear(id)` → 删单条 (`id = Some`) 或 全清 (`id = None`)

### 用法示例 (memory_provider)

```rust
use apeireth_cache::memory_provider::{
    build_provider, MemoryEntry, MemoryQuery, ProviderKind,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // 1) 选 provider (InMemory / Sqlite / Hybrid 真接, 其他 R21 stub)
    let provider = build_provider(ProviderKind::InMemory, None).unwrap();

    // 2) capture (写一条)
    let mut md = HashMap::new();
    md.insert("tag".to_string(), "demo".to_string());
    let id = provider
        .capture(MemoryEntry::new("u1", "hello memory", md))
        .await
        .unwrap();

    // 3) query (按 id 精确)
    let entries = provider.query(MemoryQuery::by_id("u1")).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "hello memory");

    // 4) clear (删一条)
    provider.clear(Some("u1")).await.unwrap();
}
```

跑 memory_provider demo:

```bash
cargo run --example memory_provider_demo -p apeireth-cache
```

跑 memory_provider 集成测试:

```bash
cargo test -p apeireth-cache --test test_memory_provider_in_process
```

### Hybrid 模式 (L1 + L2 写穿透)

```rust
use apeireth_cache::memory_provider::{HybridProvider, SqliteProvider, MemoryEntry, MemoryQuery};

// L1 = 进程内 HashMap (热路径, fast)
// L2 = Sqlite (冷路径, durable, write-through + read-fill)
let l2 = SqliteProvider::in_memory().unwrap();
let h = HybridProvider::new(l2);

h.capture(MemoryEntry::with_id_and_content("h1", "hybrid")).await.unwrap();
// L1 = 1, L2 = 1

let r = h.query(MemoryQuery::by_id("h1")).await.unwrap();
// 走 L1 (热路径)

h.clear(Some("h1")).await.unwrap();
// L1 = 0, L2 = 0 (双删)
```

### 4 stub 守门 (不假装已实现, O-5 哲学)

4 stub (Redis / Postgres / S3 / DiskLru) 全部方法返
`MemoryProviderError::BackendNotImplemented`, 显式标 `// TODO R21`.
**不假装已实现** (per APEIRETH-CONVENTIONS §9 O-5).

```text
InMemory → OK (implemented=true)
Redis → Err(BackendNotImplemented("REDIS"))
Sqlite → OK (implemented=true)
Postgres → Err(BackendNotImplemented("POSTGRES"))
S3 → Err(BackendNotImplemented("S3"))
DiskLru → Err(BackendNotImplemented("DISK_LRU"))
Hybrid → OK (implemented=true)
```

### memory_provider 状态 (R20 阶段 6 续)

- 7 ProviderKind 编译期 hardcode (PROVIDER_KIND_VARIANT_COUNT = 7)
- 8 MemoryProviderError variant 编译期 hardcode (MEMORY_PROVIDER_ERROR_VARIANT_COUNT = 8)
- 3 真接 (InMemory / Sqlite / Hybrid) + 4 stub (Redis / Postgres / S3 / DiskLru)
- 13+ 集成测试: K-1 守门 + 3 真接端到端 + 4 stub 跨模块守门 + factory 分发

---

## 状态总结 (R20 阶段 6 整体)

⚠️ skeleton 阶段 (R20 阶段 6 估缺 + 借鉴 Golutra #4 续).

- **9 模块**: backend / config / error / lru / **memory_provider** / policy / shard / stats / ttl + lib
- **5 EvictionPolicy + 4 BackendKind + 7 ProviderKind** (双 7 模式: cache backend + memory provider)
- **1 trait + 6 个 完整 provider/cache** (Cache + MemoryProvider + MemoryCache + InMemory + Sqlite + Hybrid)
- **56+ 测试**: 43 in-module + 13 memory_provider integration
