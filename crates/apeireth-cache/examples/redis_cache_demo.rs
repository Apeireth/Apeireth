//! # redis_cache_demo — Redis backend 真接演示
//!
//! R121r-4 (工程化战区 R121 续): 演示如何用 `build_cache_redis()` 跟真 Redis 端点.
//!
//! 跑: `cargo run --example redis_cache_demo -p apeireth-cache`
//!
//! **环境变量**:
//! - `APEIRETH_REDIS_URL` (可选): Redis 连接 URL, 缺省 `redis://127.0.0.1:6379/0`
//!
//! **示例**:
//! ```bash
//! # 起 redis-server
//! docker run --rm -d -p 6379:6379 redis:7-alpine
//!
//! # 跑 example
//! cargo run --example redis_cache_demo -p apeireth-cache
//!
//! # 或自定义 URL
//! APEIRETH_REDIS_URL=redis://:secret@redis.example.com:6379/0 \
//!   cargo run --example redis_cache_demo -p apeireth-cache
//! ```
//!
//! **不假装** (主哲学锚 #1):
//! - 0 连接真实 Redis 端点时, 0 假装"已连" — 返 Err 给用户看
//! - 5 EvictionPolicy 在 Memory 模式生效; Redis 模式 0 客户端侧 eviction
//! - 真连需要本地起 redis-server (R121r-4 example 仅演示 API 用法)
//!
//! **决策日志**: `reports/agent-r121r-decision-log-2026-08-10.md` 决策 4 (Redis stub 决策)

use std::time::Duration;

use apeireth_cache::Cache;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-cache demo: Redis backend (R121r-4) ===\n");

    // 1) 构造 RedisCache (lazy connect, 失败返 Err)
    let cache: std::sync::Arc<dyn Cache<String, Vec<u8>>> =
        apeireth_cache::build_cache_redis().await?;

    println!("Connected to Redis (via APEIRETH_REDIS_URL or default redis://127.0.0.1:6379/0)");

    // 2) put 3 个 key (TTL = 60s, 跟 Memory backend 1:1)
    println!("\n--- put 3 keys ---");
    cache
        .put(
            "apeireth:demo:user:42".to_string(),
            b"alice".to_vec(),
            Duration::from_secs(60),
        )
        .await?;
    cache
        .put(
            "apeireth:demo:user:43".to_string(),
            b"bob".to_vec(),
            Duration::from_secs(60),
        )
        .await?;
    cache
        .put(
            "apeireth:demo:user:44".to_string(),
            b"charlie".to_vec(),
            Duration::from_secs(60),
        )
        .await?;

    // 3) get 2 个 hit + 1 个 miss
    println!("\n--- get 3 keys (2 expected hit, 1 expected miss) ---");
    let v1: Option<Vec<u8>> = cache
        .get(&"apeireth:demo:user:42".to_string())
        .await?;
    let v2: Option<Vec<u8>> = cache
        .get(&"apeireth:demo:user:43".to_string())
        .await?;
    let v3: Option<Vec<u8>> = cache
        .get(&"apeireth:demo:nonexistent".to_string())
        .await?;

    println!("user:42       = {:?}", v1.as_ref().map(|b| String::from_utf8_lossy(b)));
    println!("user:43       = {:?}", v2.as_ref().map(|b| String::from_utf8_lossy(b)));
    println!("user:nonexist = {:?} (expected None)", v3);

    // 4) TTL expiry (R121r-4: 1s TTL, sleep 1.5s, expected None)
    println!("\n--- TTL expiry test (1s TTL + 1.5s sleep) ---");
    cache
        .put(
            "apeireth:demo:ttl_key".to_string(),
            b"expires_soon".to_vec(),
            Duration::from_secs(1),
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let v_ttl: Option<Vec<u8>> = cache
        .get(&"apeireth:demo:ttl_key".to_string())
        .await?;
    println!(
        "ttl_key after 1.5s = {:?} (expected None — TTL expired)",
        v_ttl
    );

    // 5) clear 显式 0 实现 (FLUSHDB 不可逆, 留 R21+)
    println!("\n--- clear (intentionally not implemented) ---");
    let clear_result = cache.clear().await;
    println!("clear result: {:?} (expected Err BackendNotImplemented)", clear_result);

    // 6) stats 返空 snapshot (0 客户端侧 stats, R21+ 接服务端 INFO)
    let stats = cache.stats().await;
    println!("\n--- stats (empty snapshot, R21+ 续) ---");
    println!("hit={} miss={} size={} max_size={}", stats.hit, stats.miss, stats.size, stats.max_size);
    println!("hit_rate={} evictions={}", stats.hit_rate, stats.evictions);

    println!("\n=== demo done ===");
    Ok(())
}
