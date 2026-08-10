# `apeireth-rate-limiter`

Apeireth 专用 rate limiter (R20 阶段 6 估补, **专用 rate limiter, 比 apeireth-constraint 简单**,
1:1 翻译 v0.9.21 `@anthropic-ai/rate-limiter` 商业版)。

## 状态

- **R20 阶段 6 skeleton** (估补) — 0 真接商业版 rate-limiter SDK
- 4 算法自实现 + 5 storage (in-memory 完整, 4 stub)
- 81 tests pass (49 lib + 31 integration + 1 doc)
- `cargo check` 0 error 0 warning
- **不**进 workspace members (`[workspace]` 独立 table, 遵守 parent task spec "不**动** workspace Cargo.toml")

## 4 算法

| 算法 | 参数 (数量) | 特征 |
|---|---|---|
| **Token Bucket** | rate / burst / refill_interval / initial / max_wait (5) | 允许突发 |
| **Leaky Bucket** | rate / capacity / drip_interval / overflow_policy (4) | 严格平滑, Drop/Block 溢出 |
| **Fixed Window** | window_size / max_requests / reset_strategy (3) | 简单, 已知边界突刺 |
| **Sliding Window** | window_size / slide_interval / max_requests / precision (4) | Log 精确 / Counter 折中 |

## 5 Storage

| Storage | 状态 | 用途 |
|---|---|---|
| `InMemoryStorage` | **完整** (parking_lot + HashMap, lazy TTL) | 单进程, 默认 |
| `RedisStorage` | stub (`NotImplemented`) | R21+ 续真接 redis-rs |
| `MemcachedStorage` | stub (`NotImplemented`) | R21+ 续真接 memcache-rs |
| `FileStorage` | stub (`NotImplemented`) | R21+ 续真接 fs_err/sled |
| `DistributedStorage` | stub (`NotImplemented`) | R22+ 续真接 Raft/Paxos |

## Core API

```rust
use apeireth_rate_limiter::{RateLimiter, RateLimiterImpl, token_bucket_in_memory};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let limiter = token_bucket_in_memory(10.0, 20, Some(Duration::from_secs(1))).unwrap();

    // 1. try_acquire — 非阻塞
    assert!(limiter.try_acquire("user:42", 1).await.unwrap());

    // 2. acquire — 阻塞 + RAII
    {
        let _permit = limiter.acquire("user:42", 5).await.unwrap();
        // permit 离开 scope 时自动 release 5 tokens
    }

    // 3. reset — 清空某 key
    limiter.reset("user:42").await.unwrap();

    // 4. stats — 全局统计
    let s = limiter.stats().await;
    println!("hits={} misses={} tracked={}", s.hits, s.misses, s.tracked_keys);
}
```

## 5 K-1 强校验

构造时 `Result<RateLimiterImpl, RateLimiterError>` 失败返:

| 参数 | 错误变体 |
|---|---|
| `rate <= 0` | `RateLimiterError::ZeroRate` |
| `burst == 0` | `RateLimiterError::ZeroBurst` |
| `window_size == 0` | `RateLimiterError::ZeroWindowSize` |
| `slide_interval == 0` (显式) | `RateLimiterError::InvalidParameter` |
| `max_requests == 0` | `RateLimiterError::InvalidParameter` |
| `refill_interval == 0` | `RateLimiterError::InvalidParameter` |

## 文件结构

```
crates/apeireth-rate-limiter/
├── Cargo.toml                          # standalone package (空 [workspace])
├── README.md                           # 本文件
├── src/
│   ├── lib.rs                          # 主入口 (700+ 行, 6 哲学锚 + 8 项承诺 + trait + RateLimiterImpl)
│   ├── token_bucket.rs                 # 5 参数
│   ├── leaky_bucket.rs                 # 4 参数
│   ├── fixed_window.rs                 # 3 参数
│   ├── sliding_window.rs               # 4 参数 (Log/Counter)
│   ├── storage.rs                      # 5 storage
│   ├── error.rs                        # 9 种错误
│   └── config.rs                       # 4 段配置
├── tests/
│   └── test_rate_limiter_in_process.rs # 31 集成测试
└── examples/
    └── rate_limiter_demo.rs            # 4 算法 + storage stub 演示
```

## 跑测试

```bash
cd crates/apeireth-rate-limiter
cargo test           # 49 lib + 31 integration + 1 doc = 81 tests
cargo run --example rate_limiter_demo
```

## 6 哲学锚穿透 (per APEIRETH-CONVENTIONS §9)

1. **S-1 北极星导向**: 服务 ASI 北极星, rate limiter 是主对话 / Tool call / LLM 调用的限流基础设施
2. **S-2 实事求是**: 1:1 翻译商业版 API surface, 不重写算法理论
3. **O-5 不假装**: in-memory 完整, 4 storage stub 返 NotImplemented, 0 假装 Redis 已接
4. **O-2 走在前人经验上**: 4 大经典算法对照 stripe / ratelimit / governor
5. **O-3 干到底**: API / 4 算法 / 5 storage / 30+ 测试 / example 一次落地
6. **O-4 任何人都能接手**: 7 模块边界清晰, 每模块可独立 review

## 8 项不修改承诺 (per task spec §10)

1. **0 真接商业版 `@anthropic-ai/rate-limiter` SDK** — 4 算法全自实现
2. **0 触碰 24 LOCKED crate** — 仅新建本 crate
3. **0 改 workspace version** — 本 crate 硬编码 `version = "1.0.0"`
4. **0 改 workspace Cargo.toml** — 本 crate 不进 members, 用空 `[workspace]` table 独立
5. **0 改任何已有 crate** — 仅本 crate 新增文件
6. **8 工具白名单** — rate limiter 不暴露工具调用接口, 概念省略
7. **5 K-1 强校验** — rate / burst / window_size / slide_interval / max_wait 均 > 0
8. **0 主动 commit** — 文件落到主仓路径, **不** `git commit`

## R20 阶段 6 估补 (Roadmap)

| 阶段 | 内容 | 状态 |
|---|---|---|
| R20 阶段 6 (当前) | 4 算法 + 5 storage + 81 tests | ✅ skeleton |
| R21+ | Redis / Memcached / File 真接 (redis-rs / memcache-rs / fs_err) | ⏳ |
| R21+ | Distributed 真接 (Raft / Paxos) | ⏳ |
| R21+ | governor crate 集成 (更高质量 token bucket) | ⏳ (留口子) |
| R22+ | 跨节点一致性 (consensus), 集群限流 | ⏳ |
