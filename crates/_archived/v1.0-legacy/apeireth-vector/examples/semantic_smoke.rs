//! semantic_smoke: 1000 条 mock 向量 → 写入 SqliteVecBackend → top-k 余弦检索 → 打印报告.
//!
//! 用法: `cargo run -p apeireth-vector --example semantic_smoke`
//!
//! 验收目标 (V2 P1 战区 4 skeleton):
//! - 1000 条 / 256 维写入 + 多次检索跑通
//! - 单次检索 P99 < 200ms (skeleton 阶段不追求极致, 但要可见)
//! - 输出可读报告: 前 5 个最近邻 / 时间 / 统计
//!
//! ponytail ceiling: 真实语义检索需接 embedding 模型 + sqlite-vec C 扩展; 当前
//! 用 deterministic mock 生成器 (前 5 维 block 编码类别), 用以证明管线通.

use std::path::PathBuf;
use std::time::Instant;

use apeireth_vector::{SqliteVecBackend, Vector, VectorStore};
use serde_json::json;
use uuid::Uuid;

const N_VECTORS: usize = 1000;
const DIM: usize = 256;
const TOP_K: usize = 5;
const N_QUERIES: usize = 50;

fn main() -> anyhow::Result<()> {
    // 用工作目录下临时 db, 跑完即弃.
    let db_path: PathBuf = std::env::temp_dir().join("apeireth-vector-smoke.db");
    if db_path.exists() {
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
    }

    println!("apeireth-vector semantic_smoke");
    println!("  db_path    = {}", db_path.display());
    println!("  n_vectors  = {N_VECTORS}");
    println!("  dim        = {DIM}");
    println!("  top_k      = {TOP_K}");
    println!("  n_queries  = {N_QUERIES}");
    println!();

    let mut backend = SqliteVecBackend::open(&db_path)?;
    backend.set_dimension(DIM)?;
    println!("schema ready (dim={})", backend.dimension());

    // ------------------------------------------------------------------
    // (1) 生成 1000 条 mock 向量: 前 5 维 = 类别标签 (block), 后 251 维 = 噪声.
    // 这样前 5 维 block 值相同的向量彼此"语义接近".
    // ------------------------------------------------------------------
    let t_build = Instant::now();
    let mut batch: Vec<Vector> = Vec::with_capacity(N_VECTORS);
    for i in 0..N_VECTORS {
        let cat = (i % 10) as f32; // 10 个类别
        let mut data = vec![0.0f32; DIM];
        // 后 (DIM - BLOCK_RANGE) 维: 确定性噪声 (种子 = i)
        // 注意: 噪声从 d=BLOCK_RANGE 写入, 不要覆盖前面的 block 信号.
        const BLOCK_RANGE: usize = 10;
        for d in BLOCK_RANGE..DIM {
            let s = (i as f32).sin() + (d as f32).cos();
            data[d] = s * 0.01;
        }
        // 类中心: 前 BLOCK_RANGE 维中 block 位置 = 5.0 (写在噪声之后, 不会被覆盖)
        let block = (cat as usize) % 10;
        data[block] = 5.0;
        let id = Uuid::new_v4();
        let meta = json!({
            "category": cat,
            "index": i,
        });
        batch.push(Vector::with_metadata(id, data, meta));
    }
    backend.upsert_batch(&batch)?;
    let build_ms = t_build.elapsed().as_millis();
    println!(
        "inserted {} vectors in {build_ms} ms (batch tx)",
        backend.len()?
    );

    // ------------------------------------------------------------------
    // (2) 用 50 次随机 query 测检索时间.
    // 每个 query = 类中心 + 一点噪声; top-1 期望命中所属类别.
    // ------------------------------------------------------------------
    let mut latencies_ms: Vec<u128> = Vec::with_capacity(N_QUERIES);
    let mut top1_correct = 0usize;

    for q in 0..N_QUERIES {
        let cat = (q % 10) as f32;
        let mut query = vec![0.0f32; DIM];
        const BLOCK_RANGE: usize = 10;
        // 噪声先写 (BLOCK_RANGE 起), 再写 block 信号, 与 insert 一致.
        for d in BLOCK_RANGE..DIM {
            let s = ((q + 1000) as f32).sin() + (d as f32).cos();
            query[d] = s * 0.01;
        }
        let block = (cat as usize) % 10;
        query[block] = 1.0;
        let t0 = Instant::now();
        let hits = backend.search(&query, TOP_K)?;
        let dt = t0.elapsed().as_millis();
        latencies_ms.push(dt);

        // 检查 top-1 hit 的 metadata.category 与 query 相同 (近似)
        if let Some(top) = hits.first() {
            if let Some(md) = &top.metadata {
                if let Some(c) = md.get("category").and_then(|v| v.as_f64()) {
                    if (c as i64) == (cat as i64) {
                        top1_correct += 1;
                    }
                }
            }
        }
    }

    latencies_ms.sort_unstable();
    let p50 = latencies_ms[latencies_ms.len() / 2];
    let p99 = latencies_ms[(latencies_ms.len() * 99) / 100];
    let max = *latencies_ms.last().unwrap();
    let min = latencies_ms[0];
    println!();
    println!("search latency over {N_QUERIES} queries (k={TOP_K}):");
    println!("  min  = {min:>4} ms");
    println!("  p50  = {p50:>4} ms");
    println!("  p99  = {p99:>4} ms");
    println!("  max  = {max:>4} ms");
    println!();
    println!("top-1 category match: {top1_correct}/{N_QUERIES}  (mock-corpus expected ≥ 90%)");

    // ------------------------------------------------------------------
    // (3) 演示: 取一个 query, 打印它 top-5 命中.
    // ------------------------------------------------------------------
    println!();
    println!("--- sample query: category=7 (block=7) ---");
    let mut sample_q = vec![0.0f32; DIM];
    sample_q[7] = 5.0; // 强烈信号
    let t0 = Instant::now();
    let hits = backend.search(&sample_q, TOP_K)?;
    let dt = t0.elapsed().as_millis();
    println!("retrieved in {dt} ms:");
    for (rank, hit) in hits.iter().enumerate() {
        let cat = hit
            .metadata
            .as_ref()
            .and_then(|m| m.get("category"))
            .and_then(|v| v.as_f64())
            .map(|c| c as i64)
            .unwrap_or(-1);
        println!(
            "  #{} id={} score={:.4}  category={cat}",
            rank + 1,
            hit.id,
            hit.score
        );
    }

    // ------------------------------------------------------------------
    // (4) 演示 delete / clear.
    // ------------------------------------------------------------------
    let first_id = batch[0].id;
    let removed = backend.delete(first_id)?;
    println!();
    println!("delete test: removed {removed} (expect true)");
    let after_remove = backend.len()?;
    println!("len after remove = {after_remove} (expect {N_VECTORS} - 1)");

    // 清理临时 db.
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));

    println!();
    println!("semantic_smoke: OK");
    Ok(())
}
