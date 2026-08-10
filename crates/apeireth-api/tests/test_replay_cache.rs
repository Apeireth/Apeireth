//! R122-1 (v2.1 P1 #1): Response Replay Cache 集成测试 (VCP 借鉴)
//!
//! **目的**: 验证 `replay_cache::*` 在外部 crate 视角正常工作
//! (跟 R120 `cache.rs` 集成测试 1:1 模式).
//!
//! **2+ 集成测试要求** (per spec):
//! 1. cache hit 路径: 同 (method, url, body) 二次 record/lookup 一致
//! 2. cache miss 路径: 不同 (method, url, body) 各自独立
//!
//! **R122-1-retry 适配**: 适配 R122-1-retry 的 `replay_cache.rs` API
//! - `global()` 进程级 singleton (替代第一波的 `REPLAY_CACHE` global)
//! - `ResponsePayload` struct (含 body/content/model/created_at_secs/status 字段)
//! - `record` 返 `Result<(), ReplayError>` (用 `.expect("...")`)
//!
//! **不漂移 1.0 行为**:
//! - 0 起 HTTP server (跟 R120 cache 集成测试 1:1, 测 unit 行为)
//! - 0 触碰 protocol_handlers (那个在 unit test 测)

use apeireth_api::replay_cache::{
    global, hash_request, ResponsePayload, ResponseReplayCache, ReplayStats,
};
use std::time::Duration;

/// 构造一个测试用 `ResponsePayload` (跟 R122-1-retry `replay_cache.rs:500-508` 1:1 模式)
fn make_payload(content: &str) -> ResponsePayload {
    ResponsePayload {
        body: serde_json::json!({"content": content, "id": format!("resp-{content}")}),
        content: content.to_string(),
        model: "gpt-4o".to_string(),
        created_at_secs: 0,
        status: 200,
    }
}

// ============================================================
// 1. cache hit 路径: 同 (method, url, body) 二次 lookup 一致
// ============================================================

#[test]
fn integration_replay_cache_hit_path_returns_same_response() {
    // 用本地 cache 实例, 0 污染全局 (跟 unit test 一致)
    let cache = ResponseReplayCache::new(100, Duration::from_secs(60));

    let method = "POST";
    let url = "https://api.minimaxi.com/v1/chat/completions";
    let body = br#"{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}"#;

    // 1. 算 hash
    let hash1 = hash_request(method, url, body);
    assert_eq!(hash1.len(), 64, "SHA-256 hex must be 64 chars");

    // 2. miss (空 cache)
    assert!(cache.lookup(&hash1).is_none(), "first lookup should miss");

    // 3. record (R122-1-retry API: record 返 Result)
    let payload1 = make_payload("hello back");
    cache
        .record(hash1.clone(), payload1)
        .expect("record should succeed");

    // 4. hit (二次 lookup)
    let got = cache.lookup(&hash1);
    assert!(got.is_some(), "second lookup should hit");
    let entry = got.unwrap();
    assert_eq!(entry.request_hash, hash1);
    assert_eq!(entry.response.content, "hello back");
    assert_eq!(entry.hit_count, 1, "first hit should +1 hit_count");

    // 5. 第三次 lookup → hit_count = 2
    let got2 = cache.lookup(&hash1).unwrap();
    assert_eq!(got2.hit_count, 2, "second hit should +1 hit_count");
    assert_eq!(got2.response.content, "hello back", "content must be 1:1 same");

    // 6. stats 跟踪正确
    let stats: ReplayStats = cache.stats();
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1); // 第一次 lookup miss
    assert_eq!(stats.evictions, 0);
}

// ============================================================
// 2. cache miss 路径: 不同 (method, url, body) 各自独立
// ============================================================

#[test]
fn integration_replay_cache_miss_path_isolates_different_requests() {
    let cache = ResponseReplayCache::new(100, Duration::from_secs(60));

    // 3 个不同 request (改 body, 改 url, 改 method)
    let body1 = br#"{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}"#;
    let body2 = br#"{"model":"gpt-4o","messages":[{"role":"user","content":"hello"}]}"#;
    let body3 = br#"{"model":"claude-sonnet-4","messages":[{"role":"user","content":"hi"}]}"#;

    let hash1 = hash_request("POST", "https://api.minimaxi.com/v1/chat/completions", body1);
    let hash2 = hash_request("POST", "https://api.minimaxi.com/v1/chat/completions", body2);
    let hash3 = hash_request("POST", "https://api.minimaxi.com/v1/messages", body3);
    let hash4 = hash_request("GET", "https://api.minimaxi.com/v1/chat/completions", body1); // GET 跟 POST 不同

    // 4 个 hash 都不同
    assert_ne!(hash1, hash2, "different body → different hash");
    assert_ne!(hash1, hash3, "different url → different hash");
    assert_ne!(hash1, hash4, "different method → different hash");

    // 全部 record (R122-1-retry API: record 返 Result)
    cache
        .record(hash1.clone(), make_payload("ans-1"))
        .expect("record hash1");
    cache
        .record(hash2.clone(), make_payload("ans-2"))
        .expect("record hash2");
    cache
        .record(hash3.clone(), make_payload("ans-3"))
        .expect("record hash3");

    // 各自 lookup 拿到各自 payload (隔离)
    let e1 = cache.lookup(&hash1).unwrap();
    let e2 = cache.lookup(&hash2).unwrap();
    let e3 = cache.lookup(&hash3).unwrap();

    assert_eq!(e1.response.content, "ans-1");
    assert_eq!(e2.response.content, "ans-2");
    assert_eq!(e3.response.content, "ans-3");

    // GET 那个没 record, lookup miss
    assert!(
        cache.lookup(&hash4).is_none(),
        "unrecorded hash should miss"
    );

    // cache len = 3 (hash4 没 record)
    assert_eq!(cache.len(), 3);

    // stats: 3 hits + 1 miss
    let stats = cache.stats();
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 1);
}

// ============================================================
// bonus: 全局 `global()` singleton 可访问性 + VCP 借鉴 ID 字段级核验
// ============================================================

#[test]
fn integration_replay_cache_global_singleton_is_accessible() {
    // 全局 `global()` singleton 能从外部 crate 访问 (R122-1-retry 1:1 跟 R120 cache 集成测试)
    // 用 unique hash 避免污染 (R122-1-retry 没 clear 方法, 用 unique key 隔离)
    let cache = global();
    let hash = format!(
        "integration-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );

    cache
        .record(hash.clone(), make_payload("global-ans"))
        .expect("record global");
    let got = cache.lookup(&hash);
    assert!(got.is_some(), "global() singleton should be accessible");
    assert_eq!(got.unwrap().response.content, "global-ans");
}

#[test]
fn integration_replay_cache_vcp_field_correspondence() {
    // 字段级核验 VCP `ResponseReplayCache` (`chatCompletionHandler.js:73-156`) 1:1 翻译:
    // 1. `cache: Map` → `entries: Arc<RwLock<HashMap>>` ✓
    // 2. `maxEntries` → `max_entries` ✓
    // 3. `cachedAt: Date.now()` → `created_at: SystemTime` ✓
    // 4. `get(key)` → `lookup(hash)` ✓
    // 5. `set(key, entry)` → `record(hash, response)` ✓
    // 6. VCP LRU 容量清理 → `evict_lru(max)` ✓
    // 7. VCP 0 TTL → `evict_expired(now)` (super-set) ✓
    let cache = ResponseReplayCache::new(2, Duration::from_secs(60));

    // VCP 字段 (1) + (2): 写入 3 个, 触发 VCP `while (size > maxEntries)` 1:1
    cache
        .record("k1".to_string(), make_payload("ans-1"))
        .expect("record k1");
    std::thread::sleep(Duration::from_millis(5));
    cache
        .record("k2".to_string(), make_payload("ans-2"))
        .expect("record k2");
    std::thread::sleep(Duration::from_millis(5));
    cache
        .record("k3".to_string(), make_payload("ans-3"))
        .expect("record k3");

    // VCP 字段 (6): LRU 删最旧的 (k1)
    assert!(cache.lookup("k1").is_none(), "k1 should be evicted (LRU)");
    assert!(cache.lookup("k2").is_some());
    assert!(cache.lookup("k3").is_some());
    assert_eq!(cache.len(), 2);

    // VCP 字段 (3): created_at 是 SystemTime
    let entry = cache.lookup("k2").unwrap();
    let _: std::time::SystemTime = entry.created_at; // type 核验

    // VCP 字段 (7): TTL 主动清理
    let cache_ttl = ResponseReplayCache::new(100, Duration::from_millis(50));
    cache_ttl
        .record("ttl1".to_string(), make_payload("ttl-ans"))
        .expect("record ttl1");
    std::thread::sleep(Duration::from_millis(100));
    let evicted = cache_ttl.evict_expired(std::time::SystemTime::now());
    assert_eq!(evicted, 1, "TTL expired entry should be evicted");
}
