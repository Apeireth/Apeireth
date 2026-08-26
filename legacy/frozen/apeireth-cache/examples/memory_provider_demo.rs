//! # memory_provider_demo — 7 provider 演示 (3 真接 + 4 stub)
//!
//! 演示 7 provider 1:1 翻译 Golutra memory gateway 的用法:
//! - 3 真接 (InMemory / Sqlite / Hybrid) 端到端
//! - 4 stub (Redis / Postgres / S3 / DiskLru) 显式标 TODO R21, 全部返 BackendNotImplemented
//!
//! 运行: `cargo run -p apeireth-cache --example memory_provider_demo`
//!
//! ## 预期输出
//!
//! ```text
//! === InMemoryProvider 演示 ===
//! capture 'demo-1' 成功, len=1
//! query by_id 'demo-1' 命中 1 条: "hello in-memory"
//!
//! === SqliteProvider 演示 (落盘到 temp dir) ===
//! capture 'demo-2' 成功, len=1
//! reopen 后 len=1, 数据持久化
//!
//! === HybridProvider 演示 (L1 in-memory + L2 sqlite write-through) ===
//! capture 'demo-3' 成功, L1=1, L2=1
//! query 走 L1 (热路径), 命中 1 条
//! clear 双删, L1=0, L2=0
//!
//! === 4 Stub 演示 (R21 续接) ===
//! Redis: 返 Err(BackendNotImplemented) ✓
//! Postgres: 返 Err(BackendNotImplemented) ✓
//! S3: 返 Err(BackendNotImplemented) ✓
//! DiskLru: 返 Err(BackendNotImplemented) ✓
//! ```

use apeireth_cache::memory_provider::{
    build_provider, DiskLruProvider, HybridProvider, InMemoryProvider, MemoryEntry, MemoryProvider,
    MemoryProviderError, MemoryQuery, PostgresProvider, ProviderKind, RedisProvider, S3Provider,
    SqliteProvider,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    println!("=== memory_provider_demo (R20 借鉴 Golutra #4) ===\n");

    // -----------------------------------------------------------------------
    // 1) InMemoryProvider 端到端
    // -----------------------------------------------------------------------
    println!("=== InMemoryProvider 演示 ===");
    let p1 = InMemoryProvider::new();
    let mut md = HashMap::new();
    md.insert("demo".to_string(), "in-memory".to_string());
    let id = p1
        .capture(MemoryEntry::new("demo-1", "hello in-memory", md))
        .await
        .unwrap();
    println!("capture '{id}' 成功, len={}", p1.len());

    let entries = p1.query(MemoryQuery::by_id("demo-1")).await.unwrap();
    println!(
        "query by_id 'demo-1' 命中 {} 条: {:?}\n",
        entries.len(),
        entries[0].content
    );

    // -----------------------------------------------------------------------
    // 2) SqliteProvider 端到端 (落盘到 temp dir + reopen 持久化)
    // -----------------------------------------------------------------------
    println!("=== SqliteProvider 演示 (落盘 + reopen) ===");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.db");
    let path_str = path.to_str().unwrap();

    {
        let p2 = SqliteProvider::open(path_str).unwrap();
        p2.capture(MemoryEntry::with_id_and_content("demo-2", "sqlite durable"))
            .await
            .unwrap();
        println!("capture 'demo-2' 成功, len={}", p2.len().unwrap());
    }

    {
        let p2 = SqliteProvider::open(path_str).unwrap();
        println!(
            "reopen 后 len={}, 数据持久化 ✓",
            p2.len().unwrap()
        );
    }
    println!();

    // -----------------------------------------------------------------------
    // 3) HybridProvider 端到端 (L1 in-memory + L2 sqlite write-through)
    // -----------------------------------------------------------------------
    println!("=== HybridProvider 演示 (L1 in-memory + L2 sqlite write-through) ===");
    let l2 = SqliteProvider::in_memory().unwrap();
    let h = HybridProvider::new(l2);
    h.capture(MemoryEntry::with_id_and_content("demo-3", "hybrid entry"))
        .await
        .unwrap();
    println!("capture 'demo-3' 成功, L1={}, L2={}", h.l1().len(), h.l2().len().unwrap());

    let entries = h.query(MemoryQuery::by_id("demo-3")).await.unwrap();
    println!(
        "query 走 L1 (热路径), 命中 {} 条: {:?}",
        entries.len(),
        entries[0].content
    );

    h.clear(Some("demo-3")).await.unwrap();
    println!("clear 双删, L1={}, L2={}\n", h.l1().len(), h.l2().len().unwrap());

    // -----------------------------------------------------------------------
    // 4) build_provider factory (3 真接 / 4 stub 守门)
    // -----------------------------------------------------------------------
    println!("=== build_provider 演示 (3 真接 / 4 stub 守门) ===");
    for kind in ProviderKind::ALL.iter() {
        match build_provider(*kind, None) {
            Ok(p) => println!("{:?} → OK ({:?}, implemented={})", kind, p.kind(), p.is_implemented()),
            Err(e) => println!("{:?} → Err({})", kind, short_err(&e)),
        }
    }
    println!();

    // -----------------------------------------------------------------------
    // 5) 4 stub 各自方法守门
    // -----------------------------------------------------------------------
    println!("=== 4 Stub 演示 (R21 续接) ===");
    let redis = RedisProvider::new();
    let postgres = PostgresProvider::new();
    let s3 = S3Provider::new();
    let disk_lru = DiskLruProvider::new();
    for (name, provider) in [
        ("Redis", &redis as &dyn MemoryProvider),
        ("Postgres", &postgres),
        ("S3", &s3),
        ("DiskLru", &disk_lru),
    ] {
        let err = provider
            .capture(MemoryEntry::with_id_and_content("x", "y"))
            .await
            .unwrap_err();
        println!("{name}: capture → {} ✓", short_err(&err));
    }
    println!("\n=== 演示结束 ===");
}

/// MemoryProviderError 简化为单行字符串.
fn short_err(e: &MemoryProviderError) -> String {
    format!("{e:?}")
        .split_whitespace()
        .next()
        .unwrap_or("Error")
        .trim_end_matches('(')
        .to_string()
}
