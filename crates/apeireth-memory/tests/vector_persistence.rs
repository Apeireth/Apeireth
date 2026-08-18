//! R19 P2 战区 4 续 (A-3): Vector Long-Term Persistence 跨 daemon 集成测试
// 54ed4c7d: 向量路径集成测试, 挂 semantic feature (no-default 构建下整体跳过)
#![cfg(feature = "semantic")]
//!
//! ## 验证目标
//! - **真跨 daemon 持久化**: 写一段 → 关闭 (drop 所有 Arc) → 重开 (新 Arc) → 数据仍在
//! - 0 改 A 公开 API: 跟 `SqliteMemoryStore::semantic_search` (一次性) 行为一致但用真 disk
//! - 0 触碰 A 已写 `SqliteVecBackend` (在 apeireth-vector)
//!
//! ## 跟 unit test 区别
//! - unit test (`semantic_persist::tests`) 用 in-memory mem + disk vec, 模拟 in-process
//!   drop/reopen (Arc 引用计数共享)
//! - 本 integration test **真用 path-based mem + path-based vec**, 模拟不同进程
//!
//! ## 跨进程模拟方式
//! 1. 阶段 1: `Arc::new(SqliteMemoryStore::open(mem_path))` + `PersistentSemanticIndex::open(vec_path, e)`
//! 2. 写 N 条 episode
//! 3. 显式 `drop` 所有 Arc (force close)
//! 4. 阶段 2: 重新 `Arc::new(SqliteMemoryStore::open(mem_path))` + `PersistentSemanticIndex::open(vec_path, e)`
//! 5. 验证数据 + search 仍能命中
//!
//! 注: 同一进程也能模拟 — 因为 SQLite WAL NORMAL write-through, 跨连接独立.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_memory::semantic::HashEmbedder;
use apeireth_memory::{EmbedFn, EpisodeStore, PersistentSemanticIndex, SqliteMemoryStore};

/// 生成 pair (mem_path, vec_path) 在 std::env::temp_dir() 下, 测试结束清理.
fn temp_paths(tag: &str) -> (PathBuf, PathBuf) {
    let unique = uuid::Uuid::new_v4().to_string();
    let base = std::env::temp_dir().join(format!("apeireth-a3-int-{tag}-{unique}"));
    let mem_path = base.with_extension("mem.db");
    let vec_path = base.with_extension("vec.db");
    (mem_path, vec_path)
}

fn cleanup_pair(mem: &PathBuf, vec: &PathBuf) {
    let _ = std::fs::remove_file(mem);
    let _ = std::fs::remove_file(vec);
    // WAL / SHM 副文件 (SqliteMemoryStore 也走 WAL)
    let _ = std::fs::remove_file(format!("{}-wal", mem.display()));
    let _ = std::fs::remove_file(format!("{}-shm", mem.display()));
    let _ = std::fs::remove_file(format!("{}-wal", vec.display()));
    let _ = std::fs::remove_file(format!("{}-shm", vec.display()));
}

fn make_episode(id: &str, ts: i64, content: &str) -> Episode {
    Episode {
        id: id.into(),
        timestamp: ts,
        role: "user".into(),
        content: content.into(),
        session_id: "s1".into(),
    }
}

fn open_mem_arc(path: &PathBuf) -> Arc<SqliteMemoryStore> {
    Arc::new(SqliteMemoryStore::open(path).expect("open mem"))
}

fn open_embedder() -> Arc<dyn EmbedFn> {
    Arc::new(HashEmbedder::new(32))
}

// =====================================================================
// 跨 daemon 持久化集成测试
// =====================================================================

#[test]
fn cross_daemon_persistence_100_episodes() {
    let (mem_path, vec_path) = temp_paths("100ep");
    let embedder = open_embedder();

    // ===== 阶段 1: daemon 1 写 100 条 =====
    {
        let mem = open_mem_arc(&mem_path);
        // 先把 100 episode 写进 memory
        for i in 0..100 {
            let ep = make_episode(
                &format!("e{i:03}"),
                i64::from(i),
                &format!("episode {i} about topic {}", i % 7),
            );
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
        }
        // 打开 persistent index + 索引全部
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        for i in 0..100 {
            let ep = <SqliteMemoryStore as EpisodeStore>::get_episode(&mem, &format!("e{i:03}"))
                .unwrap()
                .expect("episode must exist");
            idx.index_episode(&ep).unwrap();
        }
        idx.flush_noop().unwrap();
        assert_eq!(idx.len().unwrap(), 100);

        // 显式 drop — 模拟 daemon 关闭
        drop(idx);
        drop(mem);
    }

    // ===== 阶段 2: daemon 2 重开, 验证持久化 =====
    {
        let mem = open_mem_arc(&mem_path);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        assert_eq!(idx.len().unwrap(), 100, "100 条应跨 daemon 持久");
        // search 仍能命中
        let hits = idx.search("topic", 5).unwrap();
        assert!(!hits.is_empty(), "重开后 search 应能命中");
        assert!(hits.len() <= 5, "k=5 应返 ≤ 5 条");
    }

    cleanup_pair(&mem_path, &vec_path);
}

#[test]
fn cross_daemon_persistence_1000_episodes() {
    let (mem_path, vec_path) = temp_paths("1000ep");
    let embedder = open_embedder();

    {
        let mem = open_mem_arc(&mem_path);
        for i in 0..1000 {
            let ep = make_episode(
                &format!("k{i:04}"),
                i64::from(i),
                &format!("corpus item {i} with content about rust sql vector"),
            );
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
        }
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        for i in 0..1000 {
            let ep = <SqliteMemoryStore as EpisodeStore>::get_episode(&mem, &format!("k{i:04}"))
                .unwrap()
                .expect("must exist");
            idx.index_episode(&ep).unwrap();
        }
        idx.flush_noop().unwrap();
        assert_eq!(idx.len().unwrap(), 1000);
        drop(idx);
        drop(mem);
    }

    {
        let mem = open_mem_arc(&mem_path);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        assert_eq!(idx.len().unwrap(), 1000, "1000 条应跨 daemon 持久");
        let hits = idx.search("rust sql", 10).unwrap();
        assert!(!hits.is_empty(), "1000 corpus search 应能命中");
        // 验证返回 Episode 字段完整 (跨 daemon 反查正确)
        for h in &hits {
            assert!(!h.id.is_empty(), "episode id 不能为空");
            assert!(!h.content.is_empty(), "episode content 不能为空");
        }
    }

    cleanup_pair(&mem_path, &vec_path);
}

#[test]
fn incremental_persistence_two_writes() {
    // 验证: 写 5 条 → drop → 重开 → 写 5 条 (累计 10) → drop → 重开 → 验证 10
    let (mem_path, vec_path) = temp_paths("incr");
    let embedder = open_embedder();

    // 阶段 1: 写 5 条
    {
        let mem = open_mem_arc(&mem_path);
        for i in 0..5 {
            let ep = make_episode(&format!("i{i}"), i64::from(i), &format!("content {i}"));
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
        }
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        for i in 0..5 {
            let ep = <SqliteMemoryStore as EpisodeStore>::get_episode(&mem, &format!("i{i}"))
                .unwrap()
                .unwrap();
            idx.index_episode(&ep).unwrap();
        }
        assert_eq!(idx.len().unwrap(), 5);
    }

    // 阶段 2: 重开, 追加 5 条
    {
        let mem = open_mem_arc(&mem_path);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        assert_eq!(idx.len().unwrap(), 5, "阶段 2 重开应见 5 条");
        for i in 5..10 {
            let ep = make_episode(&format!("i{i}"), i64::from(i), &format!("content {i}"));
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
            let ep2 = <SqliteMemoryStore as EpisodeStore>::get_episode(&mem, &format!("i{i}"))
                .unwrap()
                .unwrap();
            idx.index_episode(&ep2).unwrap();
        }
        assert_eq!(idx.len().unwrap(), 10, "阶段 2 累计 10 条");
    }

    // 阶段 3: 再重开, 验证累计 10
    {
        let mem = open_mem_arc(&mem_path);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        assert_eq!(idx.len().unwrap(), 10, "阶段 3 累计 10 条");
        let hits = idx.search("content", 5).unwrap();
        assert!(!hits.is_empty());
    }

    cleanup_pair(&mem_path, &vec_path);
}

#[test]
fn concurrent_persistent_indexes_dont_conflict() {
    // 2 个 PersistentSemanticIndex 实例同时持 (各自 Arc<Mutex>), 写同一 path.
    // 期望: 不 panic; 后写者覆盖前写者 (read-your-write semantics).
    let (mem_path, vec_path) = temp_paths("concurrent");
    let embedder = open_embedder();

    let mem = open_mem_arc(&mem_path);
    // 写 1 条
    let ep = make_episode("e1", 1, "shared content");
    <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();

    // idx1 + idx2 (不同进程语义: 同 path, 不同 connection, 不同 Arc<Mutex>)
    let idx1 =
        PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder)).unwrap();
    idx1.index_episode(&ep).unwrap();
    assert_eq!(idx1.len().unwrap(), 1);

    let idx2 =
        PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder)).unwrap();
    // idx2 reload 应该看到 idx1 写的 (write-through WAL)
    let n2 = idx2.len().unwrap();
    assert!(n2 >= 1, "idx2 应见 idx1 写入 (>= 1), got {n2}");

    // idx2 追加 1 条
    let ep2 = make_episode("e2", 2, "second content");
    <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep2).unwrap();
    idx2.index_episode(&ep2).unwrap();
    assert_eq!(idx2.len().unwrap(), 2);

    // idx1 reload: 重新打开 → 应见 2 条
    drop(idx1);
    let idx1_v2 =
        PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder)).unwrap();
    assert_eq!(idx1_v2.len().unwrap(), 2, "idx1 重开应见 idx2 写入");

    cleanup_pair(&mem_path, &vec_path);
}

#[test]
fn persistent_index_search_after_daemon_restart_preserves_ranking() {
    // 验证: 同一 query 在 daemon 1 / daemon 2 跑出的 ranking 一致 (write-through + reload 无损)
    let (mem_path, vec_path) = temp_paths("ranking");
    let embedder = open_embedder();

    let daemon1_hits: Vec<String>;
    {
        let mem = open_mem_arc(&mem_path);
        // 写 3 条: 2 sql 主题 + 1 rust 主题
        let eps = vec![
            make_episode("sql1", 1, "SQL database query tutorial"),
            make_episode("sql2", 2, "advanced SQL joins"),
            make_episode("rust1", 3, "rust borrow checker is hard"),
        ];
        for ep in &eps {
            <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, ep).unwrap();
        }
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        idx.index_episodes(&eps).unwrap();
        idx.flush_noop().unwrap();
        daemon1_hits = idx
            .search("SQL database query", 3)
            .unwrap()
            .iter()
            .map(|e| e.id.clone())
            .collect();
    }

    let daemon2_hits: Vec<String>;
    {
        let mem = open_mem_arc(&mem_path);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        daemon2_hits = idx
            .search("SQL database query", 3)
            .unwrap()
            .iter()
            .map(|e| e.id.clone())
            .collect();
    }

    // 两次 search 顺序应一致 (HashEmbedder deterministic)
    assert_eq!(daemon1_hits, daemon2_hits, "跨 daemon ranking 应一致");
    assert!(!daemon1_hits.is_empty(), "search 应至少 1 hit");
    assert!(daemon1_hits.len() <= 3, "k=3 应返 ≤ 3 条");
    // HashEmbedder 不理解语义, 这里只验证"不空"和"顺序跨 daemon 一致", 不验证排除
    // (排除验证是 LLM embedder 的事, R21+ 续)

    cleanup_pair(&mem_path, &vec_path);
}

#[test]
fn persistent_index_extract_profile_after_restart() {
    // 验证: extract_profile 跨 daemon 仍能工作
    let (mem_path, vec_path) = temp_paths("profile");
    let embedder = open_embedder();

    // 阶段 1: 写 10 条 user role
    {
        let mem = open_mem_arc(&mem_path);
        let eps: Vec<Episode> = (0..10)
            .map(|i| {
                let ep = make_episode(
                    &format!("p{i}"),
                    i64::from(i),
                    "rust sql database vector topic",
                );
                <SqliteMemoryStore as EpisodeStore>::put_episode(&mem, &ep).unwrap();
                ep
            })
            .collect();
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        idx.index_episodes(&eps).unwrap();
        idx.flush_noop().unwrap();
    }

    // 阶段 2: 重开 + extract
    {
        let mem = open_mem_arc(&mem_path);
        let idx = PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder))
            .unwrap();
        let profile = idx.extract_profile().unwrap();
        assert!(
            profile.interaction_count >= 10,
            "重开 extract 应见 ≥ 10 interaction"
        );
    }

    cleanup_pair(&mem_path, &vec_path);
}

#[test]
fn persistent_index_with_different_embedder_dim_rejects() {
    // 验证: 持久化 db 32 维, embedder 64 维 → 第二次 open 报错
    let (_mem_path, vec_path) = temp_paths("dim");
    // 显式标 Arc<dyn EmbedFn>, 避免 Arc::clone 时类型推导到 Arc<HashEmbedder>
    let embedder_32: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(32));
    let embedder_64: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(64));

    // 阶段 1: 32 维 open + close
    {
        let mem = open_mem_arc(&_mem_path);
        let idx =
            PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder_32))
                .unwrap();
        drop(idx);
    }

    // 阶段 2: 64 维 embedder open → 报错
    {
        let mem = open_mem_arc(&_mem_path);
        let result =
            PersistentSemanticIndex::open(Arc::clone(&mem), &vec_path, Arc::clone(&embedder_64));
        assert!(result.is_err(), "dim 不一致应报错");
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("dim") || err.contains("dimension"),
            "错误信息应提 dim: {err}"
        );
    }

    cleanup_pair(&_mem_path, &vec_path);
}
