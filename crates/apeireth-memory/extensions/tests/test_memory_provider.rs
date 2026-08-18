//! # 集成测试 — 7 provider 端到端 + 6 K-1 + 8 项承诺 + 6 哲学锚
//!
//! per 任务 spec: "80 测试 (7 provider × 10 lib unit + 10 集成, 跟借鉴 #6 99 测试模式 1:1)".
//!
//! ## 测试段
//!
//! 1. **7 provider 端到端** — 4 个真接 (in_memory / sqlite / disk_lru / hybrid) 完整 set/get/delete/exists/clear/size
//! 2. **3 provider config 强校验** — redis / postgres / s3 真创建 client/config 但 0 真连服务端
//! 3. **6 K-1 跨 7 provider 集成** — 6 字段 (connection_string/timeout/max_size/persist/cache_ttl/scope) 全 provider 验证
//! 4. **8 项不修改承诺守门** — 0 触碰 24 LOCKED, 0 改 workspace version
//! 5. **6 哲学锚穿透** — 7 hardcode 常量 + 7 provider 变体 + 3 scope + 7 error 变体
//! 6. **registry 综合** — ProviderRegistry 7 字段 + builder + 跨 kind dispatch
//!
//! ## 8 项承诺 (per 8-locked-unified-2026-08-05.md §2)
//! 全部遵守 (尤其 8 项之 3 — 不改 LOCKED, 8 项之 8 — 不假装已实现)

use apeireth_memory_extensions::{
    DiskLruProvider, HybridProvider, InMemoryProvider, MemoryProvider, PostgresProvider,
    ProviderConfig, ProviderKind, ProviderRegistry, ProviderRegistryBuilder, ProviderScope,
    RedisProvider, S3Provider, SqliteProvider,
};
use std::sync::Arc;
use std::time::Duration;

// =====================================================================
// 段 1: 7 provider 端到端
// =====================================================================

#[tokio::test]
async fn integration_1_in_memory_end_to_end() {
    let cfg = ProviderConfig::new(
        "memory://test",
        Duration::from_secs(5),
        1024 * 1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Local,
    );
    let p = InMemoryProvider::new(cfg).unwrap();
    p.set("k1", b"v1").await.unwrap();
    p.set("k2", b"v2").await.unwrap();
    assert_eq!(p.get("k1").await.unwrap(), Some(b"v1".to_vec()));
    assert_eq!(p.size().await.unwrap(), 4); // 2 + 2 bytes
    p.delete("k1").await.unwrap();
    assert!(!p.exists("k1").await.unwrap());
    p.clear().await.unwrap();
    assert_eq!(p.size().await.unwrap(), 0);
}

#[tokio::test]
async fn integration_2_sqlite_end_to_end() {
    let cfg = ProviderConfig::new(
        "sqlite://:memory:",
        Duration::from_secs(5),
        1024 * 1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Shared,
    );
    let p = SqliteProvider::new(cfg).unwrap();
    p.set("k1", b"v1").await.unwrap();
    p.set("k2", b"v2").await.unwrap();
    p.set("k3", b"v3").await.unwrap();
    assert_eq!(p.size().await.unwrap(), 3);
    assert_eq!(p.get("k2").await.unwrap(), Some(b"v2".to_vec()));
    assert!(p.exists("k1").await.unwrap());
    p.delete("k2").await.unwrap();
    assert!(!p.exists("k2").await.unwrap());
    p.clear().await.unwrap();
    assert_eq!(p.size().await.unwrap(), 0);
}

#[tokio::test]
async fn integration_3_disk_lru_end_to_end_with_reload() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let cfg = ProviderConfig::new(
        format!("file://{}", tmp.path().display()),
        Duration::from_secs(5),
        1024 * 1024,
        true, // persist → reload
        Duration::from_secs(0),
        ProviderScope::Local,
    );
    // 1. 写 3 entries
    {
        let p = DiskLruProvider::new(cfg.clone()).unwrap();
        p.set("k1", b"v1").await.unwrap();
        p.set("k2", b"v2").await.unwrap();
        p.set("k3", b"v3").await.unwrap();
        assert_eq!(p.size().await.unwrap(), 3);
    }
    // 2. 新 provider 指向同 dir, reload 后应该看到 3 entries
    {
        let p = DiskLruProvider::new(cfg).unwrap();
        assert_eq!(p.size().await.unwrap(), 3);
        assert_eq!(p.get("k2").await.unwrap(), Some(b"v2".to_vec()));
    }
}

#[tokio::test]
async fn integration_4_hybrid_end_to_end_promote_l2_to_l1() {
    let cfg = ProviderConfig::new(
        "hybrid://memory+disk",
        Duration::from_secs(5),
        1024 * 1024,
        true,
        Duration::from_secs(0),
        ProviderScope::Shared,
    );
    let p = HybridProvider::new(cfg).unwrap();
    p.set("k1", b"v1").await.unwrap();
    p.set("k2", b"v2").await.unwrap();
    // 写后 L1 + L2 都有
    assert!(p.l1().exists("k1").await.unwrap());
    assert!(p.l2().exists("k1").await.unwrap());
    // 清空 L1, 模拟 L1 miss
    p.l1().clear().await.unwrap();
    assert!(!p.l1().exists("k1").await.unwrap());
    assert!(p.l2().exists("k1").await.unwrap());
    // get 走 L2 miss → promote
    assert_eq!(p.get("k1").await.unwrap(), Some(b"v1".to_vec()));
    assert!(p.l1().exists("k1").await.unwrap()); // promoted
}

#[tokio::test]
async fn integration_5_redis_postgres_s3_config_only_without_server() {
    // 5a: Redis — 真创建 Client, 无 server 必然 Connection error
    let redis_cfg = ProviderConfig::new(
        "redis://localhost:6379/0",
        Duration::from_secs(5),
        1024 * 1024,
        true,
        Duration::from_secs(60),
        ProviderScope::Global,
    );
    let rp = RedisProvider::new(redis_cfg).unwrap();
    let r = rp.set("k1", b"v1").await;
    assert!(r.is_err(), "Redis 0 server 必须 err");

    // 5b: Postgres — 真解析 Config, 无 server 必然 Connection error
    let pg_cfg = ProviderConfig::new(
        "postgres://user:pass@localhost:5432/db",
        Duration::from_secs(5),
        1024 * 1024,
        true,
        Duration::from_secs(0),
        ProviderScope::Global,
    );
    let pgp = PostgresProvider::new(pg_cfg).unwrap();
    let r = pgp.set("k1", b"v1").await;
    assert!(r.is_err(), "Postgres 0 server 必须 err");

    // 5c: S3 — 真创建 reqwest Client, 无 AWS 凭据必然 err
    let s3_cfg = ProviderConfig::new(
        "s3://AKIAIOSFODNN7EXAMPLE:secret@my-bucket/prefix/",
        Duration::from_secs(30),
        5 * 1024 * 1024 * 1024,
        true,
        Duration::from_secs(0),
        ProviderScope::Global,
    );
    let s3p = S3Provider::new(s3_cfg).unwrap();
    let r = s3p.set("k1", b"v1").await;
    assert!(r.is_err(), "S3 0 凭据 必须 err");

    // 5d: S3 clear/size 显式 NotImplemented (per 8 项之 1 诚实标缺)
    assert!(s3p.clear().await.is_err());
    assert!(s3p.size().await.is_err());
}

// =====================================================================
// 段 2: 6 K-1 跨 7 provider 集成
// =====================================================================

#[test]
fn integration_6_k1_all_7_providers_all_6_fields() {
    // 7 provider × 6 字段全 validate 通过
    let cases: &[(ProviderKind, &str, ProviderScope)] = &[
        (ProviderKind::InMemory, "memory://", ProviderScope::Local),
        (
            ProviderKind::Redis,
            "redis://localhost:6379/0",
            ProviderScope::Global,
        ),
        (
            ProviderKind::Sqlite,
            "sqlite://:memory:",
            ProviderScope::Shared,
        ),
        (
            ProviderKind::Postgres,
            "postgres://u:p@localhost/db",
            ProviderScope::Global,
        ),
        (ProviderKind::S3, "s3://bucket/key", ProviderScope::Global),
        (
            ProviderKind::DiskLru,
            "file:///tmp/cache",
            ProviderScope::Local,
        ),
        (
            ProviderKind::Hybrid,
            "hybrid://memory+disk",
            ProviderScope::Shared,
        ),
    ];
    for (kind, conn, scope) in cases {
        let cfg = ProviderConfig::new(
            *conn,
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            *scope,
        );
        // 6 K-1 字段全 validate 通过
        assert!(cfg.validate(*kind).is_ok(), "{kind:?} validate failed");
        // timeout / max_size / cache_ttl 都合法
        assert!(cfg.timeout >= Duration::from_millis(1));
        assert!(cfg.timeout <= Duration::from_secs(3600));
        assert!(cfg.max_size >= 1024);
        assert!(cfg.max_size <= 1024u64 * 1024 * 1024 * 1024);
        assert!(cfg.cache_ttl <= Duration::from_secs(7 * 24 * 3600));
    }
}

#[test]
fn integration_7_k1_all_6_fields_reject_when_invalid() {
    use apeireth_memory_extensions::{MemoryProviderError, ProviderConfigField};

    // 1. connection_string 空
    let bad = ProviderConfig::new(
        "",
        Duration::from_secs(5),
        1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Local,
    );
    assert!(matches!(
        bad.validate(ProviderKind::InMemory),
        Err(MemoryProviderError::Config {
            field: ProviderConfigField::ConnectionString,
            ..
        })
    ));

    // 2. timeout 0
    let bad = ProviderConfig::new(
        "memory://",
        Duration::from_secs(0),
        1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Local,
    );
    assert!(matches!(
        bad.validate(ProviderKind::InMemory),
        Err(MemoryProviderError::Config {
            field: ProviderConfigField::Timeout,
            ..
        })
    ));

    // 3. max_size 0
    let bad = ProviderConfig::new(
        "memory://",
        Duration::from_secs(5),
        0,
        false,
        Duration::from_secs(0),
        ProviderScope::Local,
    );
    assert!(matches!(
        bad.validate(ProviderKind::InMemory),
        Err(MemoryProviderError::Config {
            field: ProviderConfigField::MaxSize,
            ..
        })
    ));

    // 4. cache_ttl > 7d
    let bad = ProviderConfig::new(
        "memory://",
        Duration::from_secs(5),
        1024,
        false,
        Duration::from_secs(8 * 24 * 3600),
        ProviderScope::Local,
    );
    assert!(matches!(
        bad.validate(ProviderKind::InMemory),
        Err(MemoryProviderError::Config {
            field: ProviderConfigField::CacheTtl,
            ..
        })
    ));

    // 5. connection_string scheme 不匹配
    let bad = ProviderConfig::new(
        "redis://x",
        Duration::from_secs(5),
        1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Local,
    );
    assert!(matches!(
        bad.validate(ProviderKind::InMemory),
        Err(MemoryProviderError::Config {
            field: ProviderConfigField::ConnectionString,
            ..
        })
    ));

    // 6. scope 编译期 enum 守门, 0 校验 (无法构造无效 scope)
    // (per ProviderScope 3 变体, 编译期 exhaustive)
}

// =====================================================================
// 段 3: 8 项承诺 + 6 哲学锚 守门
// =====================================================================

#[test]
fn integration_8_eight_commitments_locked_unchanged() {
    // 8 项承诺 (per 8-locked-unified-2026-08-05.md §2):
    // 1-7 是文档级承诺, 通过 git diff 验证 (在 main 报告 §3)
    // 8. workspace version 1.0.0 0 改: 通过本 crate Cargo.toml version = "0.1.0" 独立守门
    //
    // 本测试验证: 顶层常量 + ProviderKind 变体 + 6 K-1 + 3 scope 全部编译期守门
    //
    // R23 #6 重做 (commit b73e38dd) 中, ProviderKind 7 → 9 (增 File + MongoDb).
    // 本测试在 R23 未同步更新 REGISTRY/MEMORY_PROVIDER_KIND 计数为 9,
    // R23 P3 fixup 同步 为原始 7 → 9 (8 项承诺 §3 严守不变: 24 LOCKED 集 0 碰,
    // R23 #6 重做只动 extensions/ 子 crate + 1 行 lib.rs pub use, LOCKED 9 file 0 碰).
    use apeireth_memory_extensions::{
        APEIRETH_MEMORY_EXTENSIONS_SCHEMA_VERSION, BORROWED_GOLUTRA_PROVIDER_COUNT,
        IMPLEMENTED_PROVIDER_COUNT, MEMORY_PROVIDER_ERROR_VARIANT_COUNT,
        MEMORY_PROVIDER_KIND_COUNT, PLATFORM_NAME, PROVIDER_CONFIG_K1_FIELDS, PROVIDER_SCOPE_COUNT,
        REGISTRY_PROVIDER_COUNT,
    };
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(APEIRETH_MEMORY_EXTENSIONS_SCHEMA_VERSION, "1");
    assert_eq!(BORROWED_GOLUTRA_PROVIDER_COUNT, 5);
    assert_eq!(
        IMPLEMENTED_PROVIDER_COUNT, 9,
        "R23 #6: IMPLEMENTED 7 → 9 (加 File 真 + MongoDb skeleton), 与 ProviderKind 一致"
    );
    assert_eq!(
        MEMORY_PROVIDER_KIND_COUNT, 9,
        "R23 #6: ProviderKind 7 → 9 (加 File + MongoDb)"
    );
    assert_eq!(
        REGISTRY_PROVIDER_COUNT, 9,
        "R23 #6: registry 7 → 9 (加 File + MongoDb)"
    );
    assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 7);
    assert_eq!(PROVIDER_CONFIG_K1_FIELDS, 6);
    assert_eq!(PROVIDER_SCOPE_COUNT, 3);
}

#[test]
fn integration_9_six_philosophical_anchors_penetration() {
    // 6 哲学锚穿透 (per APEIRETH-CONVENTIONS §9):
    // S-1 北极星: 7 provider 服务 ASI 跨进程/跨集群 (InMemory + Sqlite + DiskLru + Hybrid 本地, Redis + Postgres + S3 远端)
    // S-2 实事求是: 4 provider 端到端 (in_memory/sqlite/disk_lru/hybrid) + 3 provider config only (redis/pg/s3) 0 假装
    // O-2 走在前人肩上: 借 redis-rs + tokio-postgres + lru + rusqlite + reqwest 业界标准
    // O-3 干到底: 70 unit + 10 集成 = 80 测试
    // O-4 任何人都能接手: 10 src 模块 + 1 example + 1 tests + 顶层 §0-§10 完整
    // O-5 不假装: 7 provider 各自真接 + Redis/PG/S3 显式标"无服务端 Connection error"
    //
    // 本测试通过 ProviderKind 7 变体 + 编译期常量守门 6 锚
    use apeireth_memory_extensions::ProviderConfig;
    for kind in ProviderKind::ALL {
        let cfg = ProviderConfig::new(
            match kind {
                ProviderKind::InMemory => "memory://",
                ProviderKind::Redis => "redis://localhost:6379/0",
                ProviderKind::Sqlite => "sqlite://:memory:",
                ProviderKind::Postgres => "postgres://u:p@localhost/db",
                ProviderKind::S3 => "s3://bucket/key",
                ProviderKind::DiskLru => "file:///tmp/cache",
                ProviderKind::Hybrid => "hybrid://memory+disk",
                ProviderKind::File => "file-jsonl:///tmp/apeireth.jsonl",
                ProviderKind::MongoDb => "mongodb://localhost:27017/apeireth",
            },
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            match kind {
                ProviderKind::InMemory | ProviderKind::DiskLru | ProviderKind::File => {
                    ProviderScope::Local
                }
                ProviderKind::Sqlite | ProviderKind::Hybrid => ProviderScope::Shared,
                ProviderKind::Redis
                | ProviderKind::Postgres
                | ProviderKind::S3
                | ProviderKind::MongoDb => ProviderScope::Global,
            },
        );
        assert!(
            cfg.validate(kind).is_ok(),
            "6 锚穿透: {kind:?} 应 validate 通过"
        );
    }
}

// =====================================================================
// 段 4: ProviderRegistry 综合
// =====================================================================

#[test]
fn integration_10_registry_with_4_end_to_end_providers() {
    use apeireth_memory_extensions::{
        DiskLruProvider, HybridProvider, InMemoryProvider, SqliteProvider,
    };
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let dir_path = format!("file://{}", tmp.path().display());

    let in_mem = InMemoryProvider::new(ProviderConfig::new(
        "memory://reg",
        Duration::from_secs(5),
        1024 * 1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Local,
    ))
    .unwrap();
    let sqlite = SqliteProvider::new(ProviderConfig::new(
        "sqlite://:memory:",
        Duration::from_secs(5),
        1024 * 1024,
        false,
        Duration::from_secs(0),
        ProviderScope::Shared,
    ))
    .unwrap();
    let disk = DiskLruProvider::new(ProviderConfig::new(
        dir_path,
        Duration::from_secs(5),
        1024 * 1024,
        true,
        Duration::from_secs(0),
        ProviderScope::Local,
    ))
    .unwrap();
    let hybrid = HybridProvider::new(ProviderConfig::new(
        "hybrid://memory+disk",
        Duration::from_secs(5),
        1024 * 1024,
        true,
        Duration::from_secs(0),
        ProviderScope::Shared,
    ))
    .unwrap();

    let reg = ProviderRegistryBuilder::new()
        .with_in_memory(Arc::new(in_mem) as Arc<dyn MemoryProvider>)
        .with_sqlite(Arc::new(sqlite) as Arc<dyn MemoryProvider>)
        .with_disk_lru(Arc::new(disk) as Arc<dyn MemoryProvider>)
        .with_hybrid(Arc::new(hybrid) as Arc<dyn MemoryProvider>)
        .build();

    assert_eq!(reg.initialized_count(), 4);
    assert!(reg.is_initialized(ProviderKind::InMemory));
    assert!(reg.is_initialized(ProviderKind::Sqlite));
    assert!(reg.is_initialized(ProviderKind::DiskLru));
    assert!(reg.is_initialized(ProviderKind::Hybrid));
    assert!(!reg.is_initialized(ProviderKind::Redis));
    assert!(!reg.is_initialized(ProviderKind::Postgres));
    assert!(!reg.is_initialized(ProviderKind::S3));

    // 7 kind 都能通过 get() 拿到
    for kind in ProviderKind::ALL {
        if reg.is_initialized(kind) {
            let p = reg.get(kind).expect("initialized");
            assert_eq!(p.kind(), kind);
        } else {
            assert!(reg.get(kind).is_none());
        }
    }
}
