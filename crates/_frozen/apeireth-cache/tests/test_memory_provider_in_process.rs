//! Fixture: in-process memory_provider 7 provider + 3 流 (capture / query / clear) (per
//! R20 借鉴 Golutra #4 战役 task spec §3 + §4 + §5).
//!
//! 9 测试覆盖 (per task spec §5 "5+ 总" 要求, 本 fixture 给到 9):
//! 1. `test_provider_kind_7_variants` — 7 ProviderKind 1:1 列表 (K-1 守门)
//! 2. `test_provider_3_implemented_4_stub` — 3 真接 / 4 stub 守门
//! 3. `test_in_memory_capture_query_clear` — InMemoryProvider 端到端
//! 4. `test_in_memory_query_invalid` — 无过滤返 InvalidQuery
//! 5. `test_sqlite_capture_query_clear` — SqliteProvider 端到端 (in-memory mode)
//! 6. `test_sqlite_persistence_on_disk` — SqliteProvider 落盘 + reopen 持久化
//! 7. `test_hybrid_write_through` — HybridProvider L1+L2 双写
//! 8. `test_hybrid_read_fill` — HybridProvider L1 miss → L2 hit 触发 read-fill
//! 9. `test_4_stubs_all_return_not_implemented` — 4 stub 全部守门
//!
//! 4 stub (Redis / Postgres / S3 / DiskLru) 各自有 in-module 单元测试 (per
//! `src/memory_provider/{redis,postgres,s3,disk_lru}.rs` 的 `mod tests`),
//! 本 fixture 额外做一次跨 stub 守门 (4 个 stub 全部经 `build_provider` 返 Err).

use apeireth_cache::memory_provider::{
    build_provider, DiskLruProvider, HybridProvider, InMemoryProvider, MemoryEntry,
    MemoryProvider, MemoryProviderError, MemoryQuery, PostgresProvider, ProviderKind,
    RedisProvider, S3Provider, SqliteProvider, MEMORY_PROVIDER_ERROR_VARIANT_COUNT,
    PROVIDER_KIND_VARIANT_COUNT,
};
use std::collections::HashMap;

// ============================================================================
// K-1 #1: 7 ProviderKind 1:1 列表 (per task spec §3)
// ============================================================================

#[test]
fn test_provider_kind_7_variants() {
    assert_eq!(PROVIDER_KIND_VARIANT_COUNT, 7);
    assert_eq!(ProviderKind::ALL.len(), 7);
    assert!(ProviderKind::ALL.contains(&ProviderKind::InMemory));
    assert!(ProviderKind::ALL.contains(&ProviderKind::Redis));
    assert!(ProviderKind::ALL.contains(&ProviderKind::Sqlite));
    assert!(ProviderKind::ALL.contains(&ProviderKind::Postgres));
    assert!(ProviderKind::ALL.contains(&ProviderKind::S3));
    assert!(ProviderKind::ALL.contains(&ProviderKind::DiskLru));
    assert!(ProviderKind::ALL.contains(&ProviderKind::Hybrid));
}

#[test]
fn test_provider_kind_7_strs() {
    assert_eq!(ProviderKind::InMemory.as_str(), "IN_MEMORY");
    assert_eq!(ProviderKind::Redis.as_str(), "REDIS");
    assert_eq!(ProviderKind::Sqlite.as_str(), "SQLITE");
    assert_eq!(ProviderKind::Postgres.as_str(), "POSTGRES");
    assert_eq!(ProviderKind::S3.as_str(), "S3");
    assert_eq!(ProviderKind::DiskLru.as_str(), "DISK_LRU");
    assert_eq!(ProviderKind::Hybrid.as_str(), "HYBRID");
}

#[test]
fn test_provider_kind_from_str_works() {
    assert_eq!(
        "IN_MEMORY".parse::<ProviderKind>().unwrap(),
        ProviderKind::InMemory
    );
    assert_eq!(
        "in-memory".parse::<ProviderKind>().unwrap(),
        ProviderKind::InMemory
    );
    assert_eq!(
        "redis".parse::<ProviderKind>().unwrap(),
        ProviderKind::Redis
    );
    assert_eq!(
        "postgresql".parse::<ProviderKind>().unwrap(),
        ProviderKind::Postgres
    );
    assert_eq!(
        "DISK_LRU".parse::<ProviderKind>().unwrap(),
        ProviderKind::DiskLru
    );
    assert!("NOPE".parse::<ProviderKind>().is_err());
}

// ============================================================================
// K-1 #2: 3 真接 / 4 stub 守门 (per task spec §2 + §3)
// ============================================================================

#[test]
fn test_provider_3_implemented_4_stub() {
    // 3 真接
    assert!(ProviderKind::InMemory.is_implemented());
    assert!(ProviderKind::Sqlite.is_implemented());
    assert!(ProviderKind::Hybrid.is_implemented());

    // 4 stub
    assert!(!ProviderKind::Redis.is_implemented());
    assert!(!ProviderKind::Postgres.is_implemented());
    assert!(!ProviderKind::S3.is_implemented());
    assert!(!ProviderKind::DiskLru.is_implemented());

    // 8 error variant 守门
    assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 8);
}

#[test]
fn test_provider_check_implemented() {
    // 3 真接
    assert!(ProviderKind::InMemory.check_implemented().is_ok());
    assert!(ProviderKind::Sqlite.check_implemented().is_ok());
    assert!(ProviderKind::Hybrid.check_implemented().is_ok());

    // 4 stub 全部返 BackendNotImplemented
    for stub in [
        ProviderKind::Redis,
        ProviderKind::Postgres,
        ProviderKind::S3,
        ProviderKind::DiskLru,
    ] {
        let err = stub.check_implemented().unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
    }
}

// ============================================================================
// 真接 #1: InMemoryProvider 端到端 (capture / query / clear)
// ============================================================================

#[tokio::test]
async fn test_in_memory_capture_query_clear() {
    let p = InMemoryProvider::new();
    assert!(p.is_empty());
    assert_eq!(p.kind(), ProviderKind::InMemory);
    assert!(p.is_implemented());

    // capture
    let mut md = HashMap::new();
    md.insert("tag".to_string(), "t1".to_string());
    let id = p
        .capture(MemoryEntry::new("u1", "hello in-memory", md.clone()))
        .await
        .unwrap();
    assert_eq!(id, "u1");
    assert_eq!(p.len(), 1);

    // query by id
    let r = p.query(MemoryQuery::by_id("u1")).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].content, "hello in-memory");
    assert_eq!(r[0].metadata.get("tag").map(|s| s.as_str()), Some("t1"));

    // query by content_contains
    let r = p.query(MemoryQuery::by_content_contains("in-memory")).await.unwrap();
    assert_eq!(r.len(), 1);

    // query miss
    let r = p.query(MemoryQuery::by_id("nope")).await.unwrap();
    assert_eq!(r.len(), 0);

    // clear 单条
    p.clear(Some("u1")).await.unwrap();
    assert!(p.is_empty());

    // clear 不存在返 NotFound
    let err = p.clear(Some("nope")).await.unwrap_err();
    assert!(matches!(err, MemoryProviderError::NotFound(_)));

    // clear 全部 (无 entry 不会返错)
    p.clear(None).await.unwrap();
    assert!(p.is_empty());
}

#[tokio::test]
async fn test_in_memory_query_invalid() {
    let p = InMemoryProvider::new();
    // 无过滤条件: 返 InvalidQuery (K-1 强校验)
    let err = p.query(MemoryQuery::new()).await.unwrap_err();
    assert!(matches!(err, MemoryProviderError::InvalidQuery(_)));
}

// ============================================================================
// 真接 #2: SqliteProvider 端到端 (in-memory mode)
// ============================================================================

#[tokio::test]
async fn test_sqlite_capture_query_clear() {
    let p = SqliteProvider::in_memory().unwrap();
    assert!(p.is_empty().unwrap());
    assert_eq!(p.kind(), ProviderKind::Sqlite);
    assert!(p.is_implemented());

    // capture
    p.capture(MemoryEntry::with_id_and_content("s1", "sqlite entry"))
        .await
        .unwrap();
    assert_eq!(p.len().unwrap(), 1);

    // query by id
    let r = p.query(MemoryQuery::by_id("s1")).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].content, "sqlite entry");

    // query by content_contains
    let r = p.query(MemoryQuery::by_content_contains("sqlite")).await.unwrap();
    assert_eq!(r.len(), 1);

    // clear
    p.clear(Some("s1")).await.unwrap();
    assert!(p.is_empty().unwrap());

    // clear 全部
    p.capture(MemoryEntry::with_id_and_content("s2", "x"))
        .await
        .unwrap();
    p.capture(MemoryEntry::with_id_and_content("s3", "y"))
        .await
        .unwrap();
    assert_eq!(p.len().unwrap(), 2);
    p.clear(None).await.unwrap();
    assert!(p.is_empty().unwrap());
}

// ============================================================================
// 真接 #2 续: SqliteProvider 落盘 + reopen 持久化
// ============================================================================

#[tokio::test]
async fn test_sqlite_persistence_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mem.db");
    let path_str = path.to_str().unwrap();

    // 1) 写
    {
        let p = SqliteProvider::open(path_str).unwrap();
        p.capture(MemoryEntry::with_id_and_content("persist-1", "durable"))
            .await
            .unwrap();
        assert_eq!(p.len().unwrap(), 1);
    }

    // 2) 重新打开, 数据应在
    {
        let p = SqliteProvider::open(path_str).unwrap();
        assert_eq!(p.len().unwrap(), 1);
        let r = p.query(MemoryQuery::by_id("persist-1")).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "durable");
    }
}

// ============================================================================
// 真接 #3: HybridProvider write-through (L1 + L2)
// ============================================================================

#[tokio::test]
async fn test_hybrid_write_through() {
    let l2 = SqliteProvider::in_memory().unwrap();
    let h = HybridProvider::new(l2);
    assert_eq!(h.kind(), ProviderKind::Hybrid);
    assert!(h.is_implemented());
    assert_eq!(h.l1().len(), 0);
    assert_eq!(h.l2().len().unwrap(), 0);

    // capture → L1 + L2 双写
    h.capture(MemoryEntry::with_id_and_content("h1", "hybrid entry"))
        .await
        .unwrap();
    assert_eq!(h.l1().len(), 1);
    assert_eq!(h.l2().len().unwrap(), 1);

    // query 走 L1 (L1 优先)
    let r = h.query(MemoryQuery::by_id("h1")).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].content, "hybrid entry");

    // clear 双删
    h.clear(Some("h1")).await.unwrap();
    assert_eq!(h.l1().len(), 0);
    assert_eq!(h.l2().len().unwrap(), 0);

    // clear 全部
    h.capture(MemoryEntry::with_id_and_content("h2", "x"))
        .await
        .unwrap();
    h.capture(MemoryEntry::with_id_and_content("h3", "y"))
        .await
        .unwrap();
    h.clear(None).await.unwrap();
    assert_eq!(h.l1().len(), 0);
    assert_eq!(h.l2().len().unwrap(), 0);
}

// ============================================================================
// 真接 #3 续: HybridProvider read-fill (L1 miss → L2 hit → L1 回填)
// ============================================================================

#[tokio::test]
async fn test_hybrid_read_fill() {
    let l2 = SqliteProvider::in_memory().unwrap();
    // 1) 直接往 L2 写 (跳过 L1)
    l2.capture(MemoryEntry::with_id_and_content("only-l2", "L2-only"))
        .await
        .unwrap();

    // 2) 构造 hybrid, L1 空, L2 有 1 条
    let h = HybridProvider::new(l2);
    assert_eq!(h.l1().len(), 0);
    assert_eq!(h.l2().len().unwrap(), 1);

    // 3) query 触发 L1 miss → L2 hit → read-fill
    let r = h.query(MemoryQuery::by_id("only-l2")).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].content, "L2-only");

    // 4) L1 被回填
    assert_eq!(h.l1().len(), 1);

    // 5) 再次 query 走 L1 (热路径)
    let r = h.query(MemoryQuery::by_id("only-l2")).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(h.l1().len(), 1); // L1 仍 1 条, 没重复回填
}

// ============================================================================
// 4 stub 跨模块守门 (per task spec §3 "4-5 stub 显式标 TODO R21")
// ============================================================================

#[tokio::test]
async fn test_4_stubs_all_return_not_implemented() {
    // 4 stub provider 各自构造
    let redis = RedisProvider::new();
    let postgres = PostgresProvider::new();
    let s3 = S3Provider::new();
    let disk_lru = DiskLruProvider::new();

    // 4 stub 都不是 implemented
    assert!(!redis.is_implemented());
    assert!(!postgres.is_implemented());
    assert!(!s3.is_implemented());
    assert!(!disk_lru.is_implemented());

    // 4 stub 全部方法返 BackendNotImplemented
    for provider in [&redis as &dyn MemoryProvider, &postgres, &s3, &disk_lru] {
        let err = provider
            .capture(MemoryEntry::with_id_and_content("a", "x"))
            .await
            .unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
        let err = provider.query(MemoryQuery::by_id("a")).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
        let err = provider.clear(None).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
    }
}

// ============================================================================
// build_provider factory 跨模块守门 (3 真接 / 4 stub)
// ============================================================================

#[tokio::test]
async fn test_build_provider_dispatch_cross() {
    // 3 真接
    let p1 = build_provider(ProviderKind::InMemory, None).unwrap();
    assert_eq!(p1.kind(), ProviderKind::InMemory);
    let p2 = build_provider(ProviderKind::Sqlite, None).unwrap();
    assert_eq!(p2.kind(), ProviderKind::Sqlite);
    let p3 = build_provider(ProviderKind::Hybrid, None).unwrap();
    assert_eq!(p3.kind(), ProviderKind::Hybrid);

    // 4 stub 全部返 Err
    for stub in [
        ProviderKind::Redis,
        ProviderKind::Postgres,
        ProviderKind::S3,
        ProviderKind::DiskLru,
    ] {
        let result = build_provider(stub, None);
        assert!(result.is_err(), "stub {stub:?} should return Err");
        let err = result.err().unwrap();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
    }
}
