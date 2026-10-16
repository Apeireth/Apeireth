//! 混合记忆检索基准: 确定性 Okapi BM25 词法路 + 稠密余弦向量路 + RRF 融合。
//!
//! 口径: 10,000 个程序生成节点 (混合 ASCII 词元与 CJK 双字词元的文本 +
//! 128 维确定性单位向量), 200 条确定性查询逐条计时 `search_rrf`
//! (recall_limit=10, rrf_k=60, 双路权重 1.0/1.0)。查询文本与向量各不相同,
//! 因此样本分布 = 混合查询工作负载的延迟分布 (含查询间方差), 不是同一查询的
//! 重复计时噪声。
//!
//! 附带记录索引构建耗时 (10K 节点双路插入, 3 次重建取值)。

use std::hint::black_box;
use std::time::Instant;

use apeireth_memory::HybridSearchEngine;

use crate::support::{measure_per_op, BenchConfig, Outcome, Rng, Target};

/// 节点数 (README 性能目标表口径)。
const NODES: usize = 10_000;
/// 向量维度。
const DIM: usize = 128;
/// 查询条数。
const QUERIES: usize = 200;

/// 程序生成语料: ASCII 词元 + CJK 双字词元混合。
fn doc_text(rng: &mut Rng, i: usize) -> String {
    let ascii: Vec<String> = (0..384).map(|k| format!("tok{k}")).collect();
    let cjk = [
        "记忆",
        "检索",
        "调度",
        "折叠",
        "溢出",
        "落盘",
        "原子",
        "冷启动",
        "配额",
        "快照",
        "回退",
        "补偿",
        "事件",
        "会话",
        "索引",
        "融合",
    ];
    let mut text = format!("node{i} ");
    let words = rng.range(24, 48);
    for _ in 0..words {
        if rng.next_u64() % 3 == 0 {
            text.push_str(rng.pick(&cjk));
            text.push(' ');
        } else {
            text.push_str(rng.pick(&ascii));
            text.push(' ');
        }
    }
    text
}

/// 单位向量 (余弦相似度输入)。
fn unit_vector(rng: &mut Rng) -> Vec<f32> {
    let mut v: Vec<f32> = (0..DIM).map(|_| rng.next_f32() - 0.5).collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in &mut v {
        *x /= norm;
    }
    v
}

/// 运行混合检索基准, 返回 [索引构建, 查询延迟] 两个结果项。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let mut rng = Rng::new(0x5EED_10C0);
    let texts: Vec<String> = (0..NODES).map(|i| doc_text(&mut rng, i)).collect();
    let vectors: Vec<Vec<f32>> = (0..NODES).map(|_| unit_vector(&mut rng)).collect();

    // 索引构建: cfg 缩放轮次重建 (10K 节点双路插入), 逐次计时。
    let mut build_samples = Vec::with_capacity(cfg.samples(3));
    for _ in 0..cfg.samples(3) {
        let mut engine = HybridSearchEngine::new(DIM).expect("构造混合检索引擎");
        let elapsed = measure_per_op(0, 1, 1, || {
            for i in 0..NODES {
                engine
                    .insert(&format!("m{i:05}"), &texts[i], Some(vectors[i].clone()))
                    .expect("插入节点");
            }
        });
        // measure_per_op 的闭包借用 engine; 此处用 black_box 防止整轮被优化掉。
        black_box(&engine);
        build_samples.push(elapsed[0]);
    }

    // 查询负载: 确定性混合查询 (文本 + 向量), 条数可由 BENCH_SCALE 缩放。
    let query_count = cfg.samples(QUERIES);
    let mut engine = HybridSearchEngine::new(DIM).expect("构造混合检索引擎");
    for i in 0..NODES {
        engine
            .insert(&format!("m{i:05}"), &texts[i], Some(vectors[i].clone()))
            .expect("插入节点");
    }
    let queries: Vec<(String, Vec<f32>)> = (0..query_count)
        .map(|_| {
            let a = format!("tok{}", rng.range(0, 384));
            let b = format!("tok{}", rng.range(0, 384));
            let vector = unit_vector(&mut rng);
            (format!("{a} {b}"), vector)
        })
        .collect();

    // warmup: 跑一遍前 16 条查询, 不计数。
    for (text, vector) in queries.iter().take(16) {
        let hits = engine
            .search_rrf(text, Some(vector), 10, 60.0, 1.0, 1.0)
            .expect("混合检索");
        black_box(hits);
    }

    // 逐查询计时 (每条查询恰好一次)。
    let mut query_samples = Vec::with_capacity(query_count);
    for (text, vector) in &queries {
        let elapsed = measure_per_op(0, 1, 1, || {
            let hits = engine
                .search_rrf(text, Some(vector), 10, 60.0, 1.0, 1.0)
                .expect("混合检索");
            black_box(hits);
        });
        query_samples.push(elapsed[0]);
    }

    vec![
        Outcome::from_ns(
            "hybrid-search-query",
            "混合记忆检索 (BM25+向量+RRF, 10K 节点)",
            "search_rrf 单次查询 (双路 recall_limit=10, rrf_k=60), 200 条确定性混合查询各计时一次",
            "ms",
            1,
            16,
            query_samples,
            Some(Target {
                label: "< 10.0 ms".to_string(),
                value: 10.0,
            }),
            "样本为不同查询的逐条延迟 (含查询间方差), 非同一查询重复计时; 索引 10,000 节点 / 向量 128 维, 程序生成",
        ),
        Outcome::from_ns(
            "hybrid-search-build",
            "混合检索索引构建 (10K 节点双路插入)",
            "HybridSearchEngine 插入 10,000 节点 (BM25 分词+词频表 + 向量索引) 全程",
            "ms",
            1,
            0,
            build_samples,
            None,
            "3 次独立重建各计时一次; 无 README 目标行, 仅记录",
        ),
    ]
}
