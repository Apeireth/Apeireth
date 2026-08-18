//! V2 memory × vector semantic-search benchmark.
//!
//! Run:
//! `cargo bench -p apeireth-memory --bench v2-memory-vector-bench --features semantic`
//!
//! Dataset construction and indexing are outside the timed region. Measurements cover the public
//! `SemanticIndex::search` path: query embedding, SQLite vector scan, cosine ranking, metadata
//! decoding, and top-k episode lookups.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_core::Episode;
use apeireth_memory::semantic::{EmbedFn, SemanticIndex};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use apeireth_vector::{SqliteVecBackend, VectorStore};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

const DIMENSION: usize = 32;
const TOP_K: usize = 10;
const DATASET_SIZES: [usize; 4] = [100, 1_000, 10_000, 100_000];
const QUERY: &str = "sqlite vector semantic memory benchmark";

struct DeterministicEmbedder;

impl EmbedFn for DeterministicEmbedder {
    fn dim(&self) -> usize {
        DIMENSION
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let mut values = vec![0.0_f32; DIMENSION];
        // Stable FNV-1a buckets avoid an extra random-number dependency.
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for (position, byte) in text.bytes().enumerate() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            values[(hash as usize ^ position) % DIMENSION] += 1.0 + f32::from(byte) / 255.0;
        }
        let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            values.iter_mut().for_each(|value| *value /= norm);
        }
        values
    }
}

fn episode(index: usize) -> Episode {
    Episode {
        id: format!("v2-bench-episode-{index:06}"),
        timestamp: index as i64,
        role: if index % 2 == 0 { "user" } else { "assistant" }.into(),
        content: format!(
            "episode {index} about sqlite memory vector retrieval topic {} shard {}",
            index % 97,
            index % 13
        ),
        session_id: format!("bench-session-{}", index % 128),
    }
}

fn percentile(sorted_nanos: &[u128], quantile: f64) -> u128 {
    let rank = (quantile * sorted_nanos.len() as f64).ceil() as usize;
    sorted_nanos[rank.saturating_sub(1).min(sorted_nanos.len() - 1)]
}

fn print_wallclock_percentiles(index: &SemanticIndex<'_>, dataset_size: usize) {
    for _ in 0..5 {
        black_box(
            index
                .search(black_box(QUERY), TOP_K)
                .expect("warm-up search"),
        );
    }

    let mut samples = Vec::with_capacity(31);
    for _ in 0..31 {
        let started = Instant::now();
        black_box(
            index
                .search(black_box(QUERY), TOP_K)
                .expect("sample search"),
        );
        samples.push(started.elapsed().as_nanos());
    }
    samples.sort_unstable();
    eprintln!(
        "V2_MEMORY_VECTOR_PERCENTILES dataset={} samples={} p50_ns={} p95_ns={} p99_ns={}",
        dataset_size,
        samples.len(),
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        percentile(&samples, 0.99),
    );
}

fn memory_vector_semantic_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("v2_memory_vector_semantic_search");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(2));

    for dataset_size in DATASET_SIZES {
        eprintln!("building memory×vector fixture: {dataset_size} episodes");
        let memory = SqliteMemoryStore::open_in_memory().expect("in-memory episode store");
        let mut vector = SqliteVecBackend::open_in_memory().expect("in-memory vector store");
        vector
            .set_dimension(DIMENSION)
            .expect("set vector dimension");
        let embedder: Arc<dyn EmbedFn> = Arc::new(DeterministicEmbedder);
        let mut index = SemanticIndex::new(&memory, Box::new(vector), embedder);

        for item_index in 0..dataset_size {
            let item = episode(item_index);
            memory.put_episode(&item).expect("insert episode");
            index.index_episode(&item).expect("index episode vector");
        }

        print_wallclock_percentiles(&index, dataset_size);
        group.throughput(Throughput::Elements(dataset_size as u64));
        group.bench_with_input(
            BenchmarkId::new("episodes", dataset_size),
            &dataset_size,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        index
                            .search(black_box(QUERY), black_box(TOP_K))
                            .expect("semantic search"),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, memory_vector_semantic_search);
criterion_main!(benches);
