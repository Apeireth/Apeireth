//! `apeireth-rate-limiter` 示例 — 4 算法演示。
//!
//! 跑: `cargo run --example rate_limiter_demo`
//!
//! 演示:
//! 1. 构造 token bucket (10 req/s, burst 20)
//! 2. 演示 `try_acquire` / `acquire` / permit drop
//! 3. 演示 `reset` / `stats`
//! 4. 演示 storage stub 错误

use apeireth_rate_limiter::{
    fixed_window_in_memory, leaky_bucket_in_memory, sliding_window_in_memory,
    token_bucket_in_memory, LeakyBucketOverflow, RateLimiter, RateLimiterStats,
    SlidingWindowPrecision, Storage, StorageConfig, StorageKind,
};
use std::time::Duration;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    println!("=== apeireth-rate-limiter demo ===\n");

    // -----------------------------------------------------------------
    // 1. Token bucket: 10 req/s, burst 20
    // -----------------------------------------------------------------
    println!("--- 1. Token Bucket (10 req/s, burst 20) ---");
    let tb = token_bucket_in_memory(10.0, 20, Some(Duration::from_secs(1))).unwrap();
    println!("  strategy: {:?}", tb.strategy_kind());
    println!("  storage:  {:?}", tb.storage_kind());

    // try_acquire 演示
    let mut hits = 0;
    for i in 0..25 {
        if tb.try_acquire("user:42", 1).await.unwrap() {
            hits += 1;
        } else if i == 20 {
            println!("  try_acquire burst drain @ iteration {}", i);
        }
    }
    println!(
        "  25 次 try_acquire('user:42', 1) -> {} hits (expect ≤ 20)",
        hits
    );

    // acquire + permit RAII
    {
        let _p1 = tb.acquire("user:42", 1).await.unwrap();
        let _p2 = tb.acquire("user:42", 1).await.unwrap();
        println!("  拿到 2 个 permit (cost=1 each), 离开 scope 时自动 release");
    }
    // 释放后, try_acquire 又能成功
    assert!(tb.try_acquire("user:42", 1).await.unwrap());
    println!("  permit drop 后, 1 次 try_acquire 成功 (证明 release 生效)");

    // reset
    tb.reset("user:42").await.unwrap();
    println!("  reset('user:42') — 桶回到初始满桶");

    // stats
    let s: RateLimiterStats = tb.stats().await;
    println!(
        "  stats: total={} hits={} misses={} hit_rate={:.2} tracked={}",
        s.total_attempts,
        s.hits,
        s.misses,
        s.hit_rate(),
        s.tracked_keys
    );

    // -----------------------------------------------------------------
    // 2. Leaky bucket: 5 capacity, Drop 溢出
    // -----------------------------------------------------------------
    println!("\n--- 2. Leaky Bucket (5 capacity, Drop overflow) ---");
    let lb = leaky_bucket_in_memory(10.0, 5, LeakyBucketOverflow::Drop).unwrap();
    for _ in 0..5 {
        assert!(lb.try_acquire("api:foo", 1).await.unwrap());
    }
    let overflow = lb.try_acquire("api:foo", 1).await.unwrap();
    println!("  桶满后第 6 个 try_acquire -> {} (expect false)", overflow);

    // -----------------------------------------------------------------
    // 3. Fixed window: 1s 窗口, 3 个请求
    // -----------------------------------------------------------------
    println!("\n--- 3. Fixed Window (1s 窗口, max 3) ---");
    let fw = fixed_window_in_memory(Duration::from_secs(1), 3).unwrap();
    for i in 0..3 {
        assert!(fw.try_acquire("metrics:push", 1).await.unwrap());
        println!("  try_acquire #{} -> ok", i + 1);
    }
    assert!(!fw.try_acquire("metrics:push", 1).await.unwrap());
    println!("  try_acquire #4 -> denied (3/3 used)");
    println!("  ⚠️  已知缺陷: fixed window 在窗口边界有 2x 突刺问题");

    // -----------------------------------------------------------------
    // 4. Sliding window: 1s 窗口, 3 个, Log 精度
    // -----------------------------------------------------------------
    println!("\n--- 4. Sliding Window (1s 窗口, max 3, Log precision) ---");
    let sw =
        sliding_window_in_memory(Duration::from_secs(1), 3, SlidingWindowPrecision::Log).unwrap();
    for i in 0..3 {
        assert!(sw.try_acquire("trace:ingest", 1).await.unwrap());
        println!("  try_acquire #{} -> ok", i + 1);
    }
    assert!(!sw.try_acquire("trace:ingest", 1).await.unwrap());
    println!("  try_acquire #4 -> denied");
    println!("  ✅ 滑窗连续滚动, 无固定窗口的 2x 突刺");

    // -----------------------------------------------------------------
    // 5. Storage stub 错误演示
    // -----------------------------------------------------------------
    println!("\n--- 5. Storage stub (Redis / Memcached / File / Distributed) ---");
    use apeireth_rate_limiter::storage::{
        DistributedStorage, FileStorage, MemcachedStorage, RedisStorage,
    };
    for (name, result) in [
        ("Redis", RedisStorage::new(None).get("k").await),
        ("Memcached", MemcachedStorage::new(None).get("k").await),
        ("File", FileStorage::new(None).get("k").await),
        ("Distributed", DistributedStorage::new(None).get("k").await),
    ] {
        match result {
            Err(e) => println!("  {:>11} -> Err({})", name, e),
            Ok(v) => println!("  {:>11} -> Ok({:?})", name, v),
        }
    }

    // -----------------------------------------------------------------
    // 6. 临时目录 demo (File storage 概念验证, 实际是 stub)
    // -----------------------------------------------------------------
    let _tmp = TempDir::new().unwrap();
    println!("\n--- 6. File storage stub (path 不影响, 永远 NotImplemented) ---");
    let cfg = StorageConfig {
        kind: StorageKind::File,
        file_path: Some(_tmp.path().to_string_lossy().to_string()),
        connection_string: None,
        shard_count: None,
    };
    println!("  StorageConfig: kind=File, path={:?}", cfg.file_path);
    println!("  ⚠️  实际读 / 写仍返 NotImplemented (R21+ 续真接 fs_err/sled)");

    println!("\n=== demo 完成 ===");
}
