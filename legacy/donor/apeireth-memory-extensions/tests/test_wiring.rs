//! # 集成测试 — telemetry cache 接线 (registry + factory + cache 语义, 全走公开 API)
//!
//! 验证接线入口 (per crate `//!` "接线现状"):
//! 1. `MemoryProviderRegistry` — 按名称注册/查询/列表/未知报错/fallback 到默认
//! 2. `ProviderFactory::from_env` — 真实 env 装配 (含 `MemoryProviderRegistry::from_env`)
//! 3. `CachedMemoryProvider` — 持久化后端 + 内存 cache 层语义 (写穿/读填充/失效 + 跨实例持久化)
//!
//! 0 假装: 所有断言用真实 provider 实例 (InMemory / Sqlite 文件), 0 mock.
//! env 测试用静态 Mutex 串行化 (防同二进制内并行测试竞争 env).

use apeireth_memory_extensions::{
    is_persistent, CachedMemoryProvider, InMemoryProvider, MemoryProvider, MemoryProviderError,
    MemoryProviderRegistry, ProviderConfig, ProviderFactory, ProviderKind, ProviderScope,
    SqliteProvider,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// env 串行化锁 (集成测试二进制内并行测试竞争 env 的防护).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn in_memory() -> Arc<dyn MemoryProvider> {
    Arc::new(
        InMemoryProvider::new(ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        ))
        .unwrap(),
    )
}

fn sqlite_mem() -> Arc<dyn MemoryProvider> {
    Arc::new(
        SqliteProvider::new(ProviderConfig::new(
            "sqlite://:memory:",
            Duration::from_secs(5),
            1024 * 1024,
            false, // persist=false → 强制 :memory:
            Duration::from_secs(0),
            ProviderScope::Shared,
        ))
        .unwrap(),
    )
}

fn sqlite_file(path: &std::path::Path) -> Arc<dyn MemoryProvider> {
    Arc::new(
        SqliteProvider::new(ProviderConfig::new(
            format!("sqlite://{}", path.display()),
            Duration::from_secs(5),
            1024 * 1024,
            true, // persist → 文件 DB
            Duration::from_secs(0),
            ProviderScope::Shared,
        ))
        .unwrap(),
    )
}

// =====================================================================
// 1. MemoryProviderRegistry (公开 API)
// =====================================================================

#[test]
fn wiring_registry_register_query_list_unknown_and_fallback() {
    let mut reg = MemoryProviderRegistry::new();
    reg.register("in_memory", in_memory()).unwrap();
    reg.register("sqlite", sqlite_mem()).unwrap();
    reg.set_default("in_memory").unwrap();

    // 查询 (按名称)
    let p = reg.get("sqlite").unwrap();
    assert_eq!(p.kind(), ProviderKind::Sqlite);

    // 列表
    assert_eq!(reg.list(), vec!["in_memory", "sqlite"]);

    // 未知 provider 报错
    let err = reg.get("redis").err().unwrap();
    assert!(matches!(err, MemoryProviderError::Other { .. }));
    assert!(err.to_string().contains("redis"));

    // select: 注册过用它, 未知 fallback
    let fallback = in_memory();
    assert_eq!(
        reg.select("sqlite", fallback.clone()).kind(),
        ProviderKind::Sqlite
    );
    assert_eq!(
        reg.select("nonexistent", fallback.clone()).kind(),
        ProviderKind::InMemory
    );

    // select_or_default: 注册过优先, 未知 → registry 默认
    assert_eq!(
        reg.select_or_default("sqlite").unwrap().kind(),
        ProviderKind::Sqlite
    );
    assert_eq!(
        reg.select_or_default("nonexistent").unwrap().kind(),
        ProviderKind::InMemory
    );
}

#[test]
fn wiring_registry_duplicate_register_errors() {
    let mut reg = MemoryProviderRegistry::new();
    reg.register("sqlite", in_memory()).unwrap();
    let err = reg.register("sqlite", in_memory()).unwrap_err();
    assert!(matches!(err, MemoryProviderError::Other { .. }));
}

// =====================================================================
// 2. ProviderFactory::from_env (真实 env)
// =====================================================================

#[test]
fn wiring_factory_from_env_real_env_selects_sqlite() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 备份 + 设 env
    let old = std::env::var("APEIRETH_MEMORY_PROVIDER").ok();
    std::env::set_var("APEIRETH_MEMORY_PROVIDER", "sqlite");
    let result = ProviderFactory::from_env();
    // 恢复
    match old {
        Some(v) => std::env::set_var("APEIRETH_MEMORY_PROVIDER", v),
        None => std::env::remove_var("APEIRETH_MEMORY_PROVIDER"),
    }

    let (kind, p) = result.unwrap();
    assert_eq!(kind, ProviderKind::Sqlite);
    assert_eq!(p.kind(), ProviderKind::Sqlite);
}

#[test]
fn wiring_registry_from_env_real_env_registers_default() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let old = std::env::var("APEIRETH_MEMORY_PROVIDER").ok();
    std::env::set_var("APEIRETH_MEMORY_PROVIDER", "in_memory");
    let reg = MemoryProviderRegistry::from_env();
    match old {
        Some(v) => std::env::set_var("APEIRETH_MEMORY_PROVIDER", v),
        None => std::env::remove_var("APEIRETH_MEMORY_PROVIDER"),
    }

    let reg = reg.unwrap();
    assert!(reg.contains("in_memory"));
    assert_eq!(reg.default_name(), Some("in_memory"));
    // fallback 到默认可用
    assert_eq!(
        reg.select_or_default("unknown").unwrap().kind(),
        ProviderKind::InMemory
    );
}

// =====================================================================
// 3. CachedMemoryProvider — cache 语义 (公开 API)
// =====================================================================

#[tokio::test]
async fn wiring_cached_provider_persists_across_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("wiring.db");

    // 写: in_memory cache + sqlite 文件后端 (write-through)
    {
        let cached = CachedMemoryProvider::new(in_memory(), sqlite_file(&db_path));
        cached.set("k1", b"persisted-value").await.unwrap();
        assert!(
            cached.cache().exists("k1").await.unwrap(),
            "cache 层应有镜像"
        );
        assert!(
            cached.backend().exists("k1").await.unwrap(),
            "后端应权威落盘"
        );
    }

    // 新开: 同文件 sqlite (persist=true) → 数据仍在 (证明后端权威持久化)
    {
        let backend2 = sqlite_file(&db_path);
        assert_eq!(
            backend2.get("k1").await.unwrap(),
            Some(b"persisted-value".to_vec())
        );
        assert_eq!(backend2.size().await.unwrap(), 1);
    }
}

#[tokio::test]
async fn wiring_cached_provider_read_fill_and_invalidate() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("wiring2.db");
    let cache = in_memory();
    let backend = sqlite_file(&db_path);
    let cached = CachedMemoryProvider::new(cache.clone(), backend.clone());

    // read-fill: 后端直写 (绕过 cache) → get 命中后回填 cache
    backend.set("cold", b"data").await.unwrap();
    assert!(!cache.exists("cold").await.unwrap());
    assert_eq!(cached.get("cold").await.unwrap(), Some(b"data".to_vec()));
    assert!(cache.exists("cold").await.unwrap(), "miss 后应回填");

    // invalidate: delete 后两层都无
    cached.delete("cold").await.unwrap();
    assert!(!cache.exists("cold").await.unwrap());
    assert!(!backend.exists("cold").await.unwrap());
    assert_eq!(cached.get("cold").await.unwrap(), None);
}

#[tokio::test]
async fn wiring_cache_semantics_persistent_direct_vs_cache_layer() {
    // "若 provider 支持持久化则直接读写, 否则经主 store 中转缓存 (内存 provider 视为 cache 层)"
    // 分类表: sqlite = 持久化后端; in_memory = cache 层
    assert!(is_persistent(ProviderKind::Sqlite));
    assert!(!is_persistent(ProviderKind::InMemory));

    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("wiring3.db");

    // 直接读写: 裸持久化 provider (不走装饰器) 也能跨实例存活
    {
        let direct = sqlite_file(&db_path);
        direct.set("direct", b"yes").await.unwrap();
    }
    let direct2 = sqlite_file(&db_path);
    assert_eq!(direct2.get("direct").await.unwrap(), Some(b"yes".to_vec()));

    // cache 层单独 (InMemory) 不持久化: 新实例读不到 (0 假装)
    let mem1 = in_memory();
    mem1.set("volatile", b"gone").await.unwrap();
    let mem2 = in_memory();
    assert_eq!(
        mem2.get("volatile").await.unwrap(),
        None,
        "in_memory 死亡即失"
    );
}
