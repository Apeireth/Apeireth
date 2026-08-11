//! # cache_demo — LRU + Memory backend 演示
//!
//! 构造 LRU + Memory backend cache, 演示 get / put / stats.
//!
//! 跑: `cargo run --example cache_demo -p apeireth-cache`
//!
//! ## 演示流
//!
//! 1. 构造 CacheBuilder (LRU + Memory + 32 shards + max_size 100)
//! 2. put 3 个 key
//! 3. get 2 个 hit + 1 个 miss
//! 4. 显示 stats (hit_rate ≈ 0.667)
//! 5. clear 后 stats 重置 (hit_rate = 0.0)

use apeireth_cache::{Cache, CacheBuilder, EvictionPolicy, BackendKind};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-cache demo: LRU + Memory backend ===\n");

    // 1) 构造 LRU + Memory + 32 shards + max_size 100
    let config = CacheBuilder::new()
        .max_size(100)
        .default_ttl(Duration::from_secs(60))
        .policy(EvictionPolicy::Lru)
        .shards(32)
        .backend(BackendKind::Memory)
        .build();

    println!("config:");
    println!("  max_size    = {}", config.max_size);
    println!("  default_ttl = {:?}", config.default_ttl);
    println!("  policy      = {}", config.policy);
    println!("  shards      = {}", config.shards);
    println!("  backend     = {}\n", config.backend);

    let cache: std::sync::Arc<dyn Cache<String, String>> =
        apeireth_cache::build_cache(config).await?;

    // 2) put 3 个 key
    println!("--- put 3 keys ---");
    cache
        .put("user:42".to_string(), "alice".to_string(), Duration::from_secs(60))
        .await?;
    cache
        .put("user:43".to_string(), "bob".to_string(), Duration::from_secs(60))
        .await?;
    cache
        .put("user:44".to_string(), "charlie".to_string(), Duration::from_secs(60))
        .await?;
    println!("put user:42 = alice");
    println!("put user:43 = bob");
    println!("put user:44 = charlie\n");

    // 3) get 2 hit + 1 miss
    println!("--- get 2 hit + 1 miss ---");
    let v42 = cache.get(&"user:42".to_string()).await?;
    let v43 = cache.get(&"user:43".to_string()).await?;
    let v99 = cache.get(&"user:99".to_string()).await?;
    println!("get user:42 = {:?}", v42);
    println!("get user:43 = {:?}", v43);
    println!("get user:99 = {:?} (miss)\n", v99);

    // 4) stats
    println!("--- stats ---");
    let stats = cache.stats().await;
    println!("hit               = {}", stats.hit);
    println!("miss              = {}", stats.miss);
    println!("hit_rate          = {:.3}", stats.hit_rate);
    println!("put_count         = {}", stats.put_count);
    println!("size              = {}", stats.size);
    println!("max_size          = {}", stats.max_size);
    println!("avg_get_latency_us= {:.3}", stats.avg_get_latency_us);
    println!("avg_put_latency_us= {:.3}\n", stats.avg_put_latency_us);

    // 5) remove
    println!("--- remove user:42 ---");
    let removed = cache.remove(&"user:42".to_string()).await?;
    println!("removed = {:?}\n", removed);

    // 6) clear + stats reset
    println!("--- clear + stats reset ---");
    cache.clear().await?;
    let after = cache.stats().await;
    println!("after clear:");
    println!("  size  = {}", after.size);
    println!("  hit   = {}", after.hit);
    println!("  miss  = {}", after.miss);
    println!("  hit_rate = {:.3}", after.hit_rate);

    println!("\n=== demo done ===");
    Ok(())
}
