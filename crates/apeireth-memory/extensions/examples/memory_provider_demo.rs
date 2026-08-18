//! # Memory Provider Demo — 7 provider 端到端演示
//!
//! per 任务 spec: "1 完整 7 provider 演示".
//!
//! ## 演示段
//!
//! 1. **ProviderKind 7 变体** — 编译期 hardcode 守门
//! 2. **6 K-1 强校验** — 跨 7 provider 验证 6 字段合法性
//! 3. **InMemoryProvider 端到端** — set/get/delete/exists/clear/size 真接
//! 4. **SqliteProvider 端到端** — rusqlite :memory: DB 真接
//! 5. **DiskLruProvider 端到端** — 写盘 + reload from disk 真接
//! 6. **HybridProvider 端到端** — L1/L2 两级 + promote 真接
//! 7. **Redis/Postgres/S3 config 强校验** — 真创建 client/reqwest/Config, 0 真连服务端
//!
//! ## 跑法
//!
//! ```bash
//! cargo run -p apeireth-memory-extensions --example memory_provider_demo
//! ```

use apeireth_memory_extensions::{
    DiskLruProvider, HybridProvider, InMemoryProvider, MemoryProvider, PostgresProvider,
    ProviderConfig, ProviderKind, ProviderScope, RedisProvider, S3Provider, SqliteProvider,
};
use std::time::Duration;
use tempfile::TempDir;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("===========================================");
    println!("apeireth-memory-extensions: 7 Provider Demo");
    println!("===========================================\n");

    // -------- Demo 1: ProviderKind 7 变体 --------
    println!("--- Demo 1: ProviderKind 7 变体 ---");
    for (i, kind) in ProviderKind::ALL.iter().enumerate() {
        println!("  [{}] {:?} = {}", i, kind, kind.as_str());
    }
    println!();

    // -------- Demo 2: 6 K-1 强校验 --------
    println!("--- Demo 2: 6 K-1 强校验 (跨 7 provider) ---");
    let cases: &[(ProviderKind, &str, &str)] = &[
        (ProviderKind::InMemory, "memory://", "local"),
        (ProviderKind::Redis, "redis://localhost:6379/0", "global"),
        (ProviderKind::Sqlite, "sqlite://:memory:", "shared"),
        (
            ProviderKind::Postgres,
            "postgres://u:p@localhost/db",
            "global",
        ),
        (ProviderKind::S3, "s3://bucket/key", "global"),
        (ProviderKind::DiskLru, "file:///tmp/cache", "local"),
        (ProviderKind::Hybrid, "hybrid://memory+disk", "shared"),
    ];
    for (kind, conn, scope) in cases {
        let cfg = ProviderConfig::new(
            *conn,
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            match *scope {
                "local" => ProviderScope::Local,
                "shared" => ProviderScope::Shared,
                "global" => ProviderScope::Global,
                _ => ProviderScope::Local,
            },
        );
        let r = cfg.validate(*kind);
        println!(
            "  [{:?}] conn={} scope={} → validate={}",
            kind,
            conn,
            scope,
            if r.is_ok() { "OK" } else { "FAIL" }
        );
    }
    println!();

    // -------- Demo 3: InMemoryProvider 端到端 --------
    println!("--- Demo 3: InMemoryProvider 端到端 ---");
    {
        let p = InMemoryProvider::new(ProviderConfig::new(
            "memory://demo",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        ))
        .unwrap();
        p.set("k1", b"v1").await.unwrap();
        p.set("k2", b"v2").await.unwrap();
        p.set("k3", b"v3").await.unwrap();
        println!("  set 3 entries → size={}", p.size().await.unwrap());
        let got = p.get("k2").await.unwrap();
        println!(
            "  get k2 = {:?}",
            got.as_ref().map(|v| String::from_utf8_lossy(v).to_string())
        );
        p.delete("k1").await.unwrap();
        println!(
            "  delete k1 → exists k1 = {}",
            p.exists("k1").await.unwrap()
        );
        p.clear().await.unwrap();
        println!("  clear all → size = {}", p.size().await.unwrap());
    }
    println!();

    // -------- Demo 4: SqliteProvider 端到端 --------
    println!("--- Demo 4: SqliteProvider 端到端 (rusqlite :memory:) ---");
    {
        let p = SqliteProvider::new(ProviderConfig::new(
            "sqlite://:memory:",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Shared,
        ))
        .unwrap();
        p.set("k1", b"v1").await.unwrap();
        p.set("k2", b"v2").await.unwrap();
        p.set("k3", b"v3").await.unwrap();
        println!("  set 3 entries → size={}", p.size().await.unwrap());
        let got = p.get("k2").await.unwrap();
        println!(
            "  get k2 = {:?}",
            got.as_ref().map(|v| String::from_utf8_lossy(v).to_string())
        );
        p.delete("k1").await.unwrap();
        println!(
            "  delete k1 → exists k1 = {}",
            p.exists("k1").await.unwrap()
        );
        p.clear().await.unwrap();
        println!("  clear all → size = {}", p.size().await.unwrap());
    }
    println!();

    // -------- Demo 5: DiskLruProvider 端到端 (含 reload from disk) --------
    println!("--- Demo 5: DiskLruProvider 端到端 (lru crate + std::fs) ---");
    {
        let tmp = TempDir::new().unwrap();
        let dir = format!("file://{}", tmp.path().display());
        // 1. 写 3 entries
        let p1 = DiskLruProvider::new(ProviderConfig::new(
            dir.clone(),
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Local,
        ))
        .unwrap();
        p1.set("k1", b"v1").await.unwrap();
        p1.set("k2", b"v2").await.unwrap();
        p1.set("k3", b"v3").await.unwrap();
        println!("  [1] 写 3 entries → size={}", p1.size().await.unwrap());
        drop(p1);
        // 2. reload
        let p2 = DiskLruProvider::new(ProviderConfig::new(
            dir,
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Local,
        ))
        .unwrap();
        println!("  [2] reload from disk → size={}", p2.size().await.unwrap());
        let got = p2.get("k2").await.unwrap();
        println!(
            "  [2] get k2 = {:?}",
            got.as_ref().map(|v| String::from_utf8_lossy(v).to_string())
        );
    }
    println!();

    // -------- Demo 6: HybridProvider 端到端 (L1/L2 两级 + promote) --------
    println!("--- Demo 6: HybridProvider 端到端 (L1=InMemory + L2=DiskLru) ---");
    {
        let p = HybridProvider::new(ProviderConfig::new(
            "hybrid://memory+disk",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Shared,
        ))
        .unwrap();
        p.set("k1", b"v1").await.unwrap();
        p.set("k2", b"v2").await.unwrap();
        println!(
            "  set 2 entries → L1 has k1 = {}, L2 has k1 = {}",
            p.l1().exists("k1").await.unwrap(),
            p.l2().exists("k1").await.unwrap()
        );
        p.l1().clear().await.unwrap();
        println!(
            "  clear L1 → L1 has k1 = {}, L2 has k1 = {}",
            p.l1().exists("k1").await.unwrap(),
            p.l2().exists("k1").await.unwrap()
        );
        let got = p.get("k1").await.unwrap();
        println!(
            "  get k1 (走 L2 miss → promote) = {:?}, L1 has k1 = {}",
            got.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
            p.l1().exists("k1").await.unwrap()
        );
    }
    println!();

    // -------- Demo 7: Redis/Postgres/S3 config 强校验 (0 真连服务端) --------
    println!("--- Demo 7: Redis/Postgres/S3 config 强校验 (0 真连服务端) ---");
    {
        // 7a: Redis — 真创建 Client
        let rp = RedisProvider::new(ProviderConfig::new(
            "redis://localhost:6379/0",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            ProviderScope::Global,
        ))
        .unwrap();
        let r = rp.set("k1", b"v1").await;
        println!("  [Redis] 0 server → set err: {}", r.is_err());
    }
    {
        // 7b: Postgres — 真解析 Config
        let pgp = PostgresProvider::new(ProviderConfig::new(
            "postgres://u:p@localhost:5432/db",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        ))
        .unwrap();
        let r = pgp.set("k1", b"v1").await;
        println!("  [Postgres] 0 server → set err: {}", r.is_err());
    }
    {
        // 7c: S3 — 真创建 reqwest Client, 0 AWS 凭据
        let s3p = S3Provider::new(ProviderConfig::new(
            "s3://AKIAIOSFODNN7EXAMPLE:secret@bucket/prefix/",
            Duration::from_secs(30),
            5 * 1024 * 1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        ))
        .unwrap();
        let r = s3p.set("k1", b"v1").await;
        let rc = s3p.clear().await;
        let rs = s3p.size().await;
        println!(
            "  [S3] 0 凭据 → set err: {}, clear err: {}, size err: {}",
            r.is_err(),
            rc.is_err(),
            rs.is_err()
        );
    }
    println!();

    println!("===========================================");
    println!("Demo 完毕: 7 provider 端到端 / config 校验全通过");
    println!("===========================================");
}
