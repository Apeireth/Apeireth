// V1190 真实端到端 criterion benchmark (R14 Phase 1 性能验证)
//
// 替换 v1130_wallclock placeholder (1+1 黑盒), 真实测 apeireth-memory 端到端:
// - put_episode 单次写入延迟 (冷启动 in-memory SQLite + migrations)
// - recent_episodes 查询延迟 (N=10 / 100 / 1000 三档)
// - bulk insert 1000 episodes 总耗时
//
// 主 17:43 实事求是: criterion 真实可测, 不刷 KPI.
// 主 19:33 走在前人经验上: SQLite + rusqlite 是经过 25 年工程验证的存储.
// 主 13:31 大胆激进: 直接用 apeireth-memory 真实 API, 不 mock.
//
// 目标 (R14 Phase 1):
// - put_episode 单条: < 50us (中等机器)
// - recent_episodes(N=100): < 1ms
// - bulk insert 1000 条: < 200ms

use apeireth_core::Episode;
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};

/// 构造一条 fake Episode.
fn make_episode(i: usize, session_id: &str) -> Episode {
    Episode {
        id: format!("ep-{i:06}"),
        timestamp: 1_700_000_000 + i as i64,
        role: if i % 2 == 0 {
            "user".into()
        } else {
            "assistant".into()
        },
        content: format!("content for episode {i} — payload size ~ 64 bytes"),
        session_id: session_id.into(),
    }
}

/// 单条 put 延迟 — 冷启动 in-memory store.
fn bench_put_episode(c: &mut Criterion) {
    c.bench_function("v1190/put_episode_single", |b| {
        b.iter_batched(
            || SqliteMemoryStore::open_in_memory().expect("open in-memory"),
            |store| {
                let ep = make_episode(0, "bench-session");
                EpisodeStore::put_episode(&store, &ep).expect("put_episode");
                store
            },
            BatchSize::SmallInput,
        );
    });
}

/// recent_episodes 查询延迟 — N=10 (真实聊天场景).
fn bench_recent_episodes_10(c: &mut Criterion) {
    let mut group = c.benchmark_group("v1190/recent_episodes");
    group.throughput(Throughput::Elements(10));

    group.bench_function("n=10", |b| {
        b.iter_batched(
            || {
                let store = SqliteMemoryStore::open_in_memory().expect("open");
                for i in 0..10 {
                    EpisodeStore::put_episode(&store, &make_episode(i, "s")).expect("put");
                }
                store
            },
            |store| {
                let _ = <SqliteMemoryStore as EpisodeStore>::recent_episodes(&store, "s", 10)
                    .expect("recent");
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// recent_episodes 查询延迟 — N=100.
fn bench_recent_episodes_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("v1190/recent_episodes");
    group.throughput(Throughput::Elements(100));

    group.bench_function("n=100", |b| {
        b.iter_batched(
            || {
                let store = SqliteMemoryStore::open_in_memory().expect("open");
                for i in 0..1000 {
                    EpisodeStore::put_episode(&store, &make_episode(i, "s")).expect("put");
                }
                store
            },
            |store| {
                let _ = <SqliteMemoryStore as EpisodeStore>::recent_episodes(&store, "s", 100)
                    .expect("recent");
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// recent_episodes 查询延迟 — N=1000 (高负载场景).
fn bench_recent_episodes_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("v1190/recent_episodes");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("n=1000", |b| {
        b.iter_batched(
            || {
                let store = SqliteMemoryStore::open_in_memory().expect("open");
                for i in 0..10_000 {
                    EpisodeStore::put_episode(&store, &make_episode(i, "s")).expect("put");
                }
                store
            },
            |store| {
                let _ = <SqliteMemoryStore as EpisodeStore>::recent_episodes(&store, "s", 1000)
                    .expect("recent");
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// bulk insert 1000 条 — 总耗时 (V1130 wallclock 核心目标).
fn bench_bulk_insert_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("v1190/bulk_insert");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("n=1000", |b| {
        b.iter_batched(
            || SqliteMemoryStore::open_in_memory().expect("open"),
            |store| {
                for i in 0..1000 {
                    EpisodeStore::put_episode(&store, &make_episode(i, "bulk")).expect("put");
                }
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_put_episode,
    bench_recent_episodes_10,
    bench_recent_episodes_100,
    bench_recent_episodes_1000,
    bench_bulk_insert_1000
);
criterion_main!(benches);
