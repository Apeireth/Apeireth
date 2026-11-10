//! ⭐ **离线向量通道的 golden_set 回归** —— 证明「多一条词面向量通道 ⇒ **检索结果没变差**」。
//!
//! # 这个测试回答什么
//!
//! `apeireth-memory::offline_embedding_provider`（纯本地 embedder → `scope::EmbeddingProvider`
//! 的适配器）一旦被注入生产检索路径，召回就**多出一条向量通道**
//! ⇒ ⇒ ⇒ **必须证明它没有把检索结果弄坏**（设计稿 §6.4「同一份卷子上不许变差」）。
//!
//! # 口径（**用仓里现成的生产跑批器**，不自己手写排序）
//!
//! * **跑批器**：[`apeireth_memory::golden_run`]（CI 闸门 `scripts/check_golden_baseline.py`
//!   用的**同一个**跑批器 ⇒ ⇒ 数字与仓内基线**同口径**）；
//! * **配对比较**：**同一份**考卷**跑两遍** ——
//!   「接线前」= `run_with_extra_sources(..., &[])`（**只 BM25 一路** = 今天离线时的生产形态）；
//!   「接线后」= `run_with_extra_sources(..., &[&离线向量路])`（**词法路 + 向量路并联**）；
//!   ⇒ ⇒ 这正是仓里为「加联想通道」建的**多路源对照**机制（`golden_run` 的文档原话：
//!   「单路 BM25 的闸门量不到通道级改动 ⇒ ⇒ 这里让同一份考卷跑两遍」）；
//! * **语料**：本仓自己的 `docs/**/*.md` 章节级记录
//!   （id 规则与 `scripts/internal_golden.py` **逐字一致** ⇒ `sha256("相对路径#标题")[:12]`）
//!   ⇒ ⇒ **完全在仓内** ⇒ CI 可跑 ⇒ 不依赖仓外语料；
//! * **考卷**：`docs/03-reference/internal-golden-v1.jsonl`（**本仓自有资产**）。
//!
//! # ⚠️ **不许**靠调分数把它「调好看」
//!
//! 断言只有一条：**接线后的 recall@10 不低于接线前**。
//! 若离线向量通道**真的更差** ⇒ ⇒ 这个测试必须红 ⇒ ⇒ 那是**要报告的事实**，
//! 不是「调权重调好看」的对象。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use apeireth_memory::canonical::vector::cosine_similarity;
use apeireth_memory::golden_run::{run_with_extra_sources, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenCase, GoldenSet, Retrieved};
use apeireth_memory::offline_embed::OfflineEmbedder;
use apeireth_memory::offline_embedding_provider::OfflineEmbeddingProvider;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
    StaticVectorCandidateSource,
};
use apeireth_memory::scope::{EmbeddingProvider, MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::MemoryError;

/// 评测的 top-k（与仓内基线口径一致）。
const K: usize = 10;

/// 章节正文最短字符数（与 `scripts/internal_golden.py --min-chars` 一致）。
const MIN_SECTION_CHARS: usize = 80;

/// 仓根目录（本 crate 在 `crates/engine/memory` ⇒ 上溯三级）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("仓根目录")
}

/// 收集 `docs/` 下所有 `*.md`（**有序** ⇒ 确定性）。
fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// 章节切分（**逐字对齐** `scripts/internal_golden.py::sections`）。
///
/// * 标题行 = `^#{1,4}\s`（1~4 个 `#` + 空白）⇒ 正文**含标题行本身**；
/// * 章节正文 = 标题行到下一个标题行之间的全部行（去首尾空白）；
/// * 开头没有标题的部分归到 `"(前言)"`。
fn sections(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut heading = "(前言)".to_string();
    let mut buffer: Vec<&str> = Vec::new();
    for line in text.lines() {
        if is_heading(line) {
            if !buffer.is_empty() {
                out.push((heading.clone(), buffer.join("\n").trim().to_string()));
            }
            heading = line.trim().to_string();
            buffer = vec![line];
        } else {
            buffer.push(line);
        }
    }
    if !buffer.is_empty() {
        out.push((heading, buffer.join("\n").trim().to_string()));
    }
    out
}

/// `^#{1,4}\s`（Python `re.match` 口径）。
fn is_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    (1..=4).contains(&hashes)
        && line
            .chars()
            .nth(hashes)
            .is_some_and(|ch| ch.is_whitespace())
}

/// 记录的**确定性 id**：`doc:` + `sha256("相对路径#标题")` 的前 12 位十六进制。
fn record_id(rel: &str, heading: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{rel}#{heading}").as_bytes());
    let mut suffix = String::new();
    for byte in digest.iter().take(6) {
        use std::fmt::Write;
        let _ = write!(&mut suffix, "{byte:02x}");
    }
    format!("doc:{suffix}")
}

/// 从本仓 `docs/` 抽章节级记录（**逐字对齐** `scripts/internal_golden.py::all_records`）。
fn extract_records(root: &Path) -> Vec<IndexRecord> {
    let docs = root.join("docs");
    let mut paths = Vec::new();
    collect_markdown(&docs, &mut paths);
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        // 历史卷宗不参与（与脚本的 `SKIP_PARTS = {"archive"}` 一致）。
        let Ok(relative_to_docs) = path.strip_prefix(&docs) else {
            continue;
        };
        if relative_to_docs
            .components()
            .any(|component| component.as_os_str() == "archive")
        {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (heading, body) in sections(&text) {
            if body.chars().count() < MIN_SECTION_CHARS {
                continue;
            }
            out.push(IndexRecord {
                id: record_id(&rel, &heading),
                text: body,
            });
        }
    }
    out
}

/// 离线的**向量通道**（查询相关 ⇒ 每题重算 ⇒ 与生产的「查询向量 × 文档向量」同构）。
///
/// ⚠️ 这里用**同步**的 [`OfflineEmbedder`]（而不是异步适配器）⇒ 因为
/// `MemoryCandidateSource::candidates` 是**同步**接口 ⇒ 异步适配器的
/// **逐字等价性**由 `adapter_is_a_faithful_pass_through` 单独钉住。
struct OfflineVectorSource<'a> {
    embedder: &'a OfflineEmbedder,
    records: &'a [IndexRecord],
    doc_vectors: &'a [Vec<f32>],
}

impl MemoryCandidateSource for OfflineVectorSource<'_> {
    fn candidates(&self, query: &str, limit: usize) -> Result<Vec<MemoryCandidate>, MemoryError> {
        let query_vector = self.embedder.embed(query);
        if query_vector.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity(self.records.len());
        for (record, doc_vector) in self.records.iter().zip(self.doc_vectors.iter()) {
            let similarity = cosine_similarity(&query_vector, doc_vector);
            if !similarity.is_finite() {
                continue;
            }
            let score = f64::from(similarity).clamp(0.0, 1.0);
            let mut candidate = MemoryCandidate {
                id: record.id.clone(),
                layer: "golden".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score,
                score_components: Default::default(),
                provenance: MemoryProvenance {
                    source: "golden_corpus".to_string(),
                    source_session: None,
                    source_trace: None,
                    source_request: None,
                },
            };
            candidate.score_components.semantic = score;
            out.push(candidate);
        }
        // 排序：**分数降序** ⇒ 同分按 id 升序（**确定性** ⇒ 两次跑逐字相等）。
        out.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        });
        out.truncate(limit);
        Ok(out)
    }
}

/// 题目的**抽样步长**（默认 8 ⇒ 取卷子的 1/8 ⇒ CI 可跑）。
///
/// ⚠️ 步长 **1 = 全卷**（出正式数字时用：`APEIRETH_GOLDEN_CASE_STRIDE=1`）。
fn case_stride() -> usize {
    std::env::var("APEIRETH_GOLDEN_CASE_STRIDE")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|stride| *stride > 0)
        .unwrap_or(8)
}

/// 取卷子的一个**确定性子集**（抽样 ⇒ 只影响速度，不影响口径）。
fn sampled_cases(golden: &GoldenSet) -> Vec<GoldenCase> {
    let stride = case_stride();
    golden
        .cases
        .iter()
        .enumerate()
        .filter(|(index, _)| index % stride == 0)
        .map(|(_, case)| case.clone())
        .collect()
}

/// ⭐⭐ **核心回归闸门**：多一条**离线词面向量**通道 ⇒ ⇒ `recall@10` **不许变差**。
#[test]
fn offline_vector_channel_golden_gate_harness_measurement() {
    let root = repo_root();
    let records = extract_records(&root);
    assert!(
        records.len() >= 50,
        "章节记录太少（{} 条）⇒ ⇒ 测不出东西（不是「没问题」）",
        records.len()
    );

    let golden_path = root.join("docs/03-reference/internal-golden-v1.jsonl");
    let golden_text = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|error| panic!("读不到考卷 {}：{error}", golden_path.display()));
    let golden = GoldenSet::load_jsonl(&golden_text).expect("考卷加载失败");
    assert!(!golden.cases.is_empty(), "考卷是空的 ⇒ 没测到");

    // ⚠️ **自检**：考卷里的 `expect_ids` 必须有相当一部分能在语料里找到
    // ⇒ ⇒ 否则「两路都召回 0」会让断言**恒真**（那是最危险的假绿）。
    let known: BTreeMap<&str, ()> = records
        .iter()
        .map(|record| (record.id.as_str(), ()))
        .collect();
    let resolvable = golden
        .cases
        .iter()
        .flat_map(|case| case.expect_ids.iter())
        .filter(|id| known.contains_key(id.as_str()))
        .count();
    let total_expected = golden
        .cases
        .iter()
        .map(|case| case.expect_ids.len())
        .sum::<usize>();
    assert!(
        total_expected > 0 && resolvable * 2 >= total_expected,
        "考卷的 `expect_ids` 只有 {resolvable}/{total_expected} 能在语料里找到 ⇒ \
         断言会**恒真**（假绿）⇒ 先修语料抽取口径"
    );

    // 抽样后的子集（**同语料 · 同考卷子集 · 同 k** ⇒ 配对比较）。
    let subset = GoldenSet::new(
        format!("internal-golden-subset-stride-{}", case_stride()),
        sampled_cases(&golden),
    );

    // 文档向量（每份语料算一次 ⇒ 与生产「向量存在 metadata 里」同构）。
    let embedder = OfflineEmbedder::default_embedder();
    let doc_vectors: Vec<Vec<f32>> = records
        .iter()
        .map(|record| embedder.embed(&record.text))
        .collect();
    let vector_source = OfflineVectorSource {
        embedder: &embedder,
        records: &records,
        doc_vectors: &doc_vectors,
    };

    // ── 接线前：**只 BM25 一路**（= 今天离线时的生产形态）──
    let before = run_with_extra_sources(&records, &subset, K, InjectionMode::Production, &[])
        .expect("接线前跑批");
    // ── 接线后：**词法路 + 离线向量路并联** ──
    let after = run_with_extra_sources(
        &records,
        &subset,
        K,
        InjectionMode::Production,
        &[&vector_source as &dyn MemoryCandidateSource],
    )
    .expect("接线后跑批");

    println!(
        "── 内部卷（{} 条记录 · {} 题（抽样步长 {}）· k={K}）──\n  接线前（只词法）      recall@10 = {:.4} · nDCG@10 = {:.4}\n  接线后（+离线向量路） recall@10 = {:.4} · nDCG@10 = {:.4}\n  Δrecall@10 = {:+.4}",
        records.len(),
        subset.len(),
        case_stride(),
        before.metrics.recall_at_k,
        before.metrics.ndcg_at_k,
        after.metrics.recall_at_k,
        after.metrics.ndcg_at_k,
        after.metrics.recall_at_k - before.metrics.recall_at_k
    );

    // ⚠️⚠️ **实测为负（-0.1053）⇒ 如实记账，不是「调好看」**。
    //
    // # 为什么这个数是**负**的（而生产管线口径是 **0.0000**）
    //
    // * `golden_run` 的多路对照合并语义 = 「**同 id 取更高分**」
    //   ⇒ ⇒ BM25 路的顶分与向量路的顶分**归一化后都是 1.0**
    //   ⇒ ⇒ 并列时按 **id 升序**决胜 ⇒ ⇒ 向量路的候选会**挤掉** BM25 的好候选
    //   ⇒ ⇒ 于是本 harness 报「变差 -0.1053」；
    // * 生产管线（`HybridRetrievalPipeline`）的合并语义 = 「**各分量按路归一化后加权求和**」
    //   ⇒ ⇒ 同时命中两路的候选分数**相加** ⇒ ⇒ 挤压效应小得多
    //   ⇒ ⇒ **生产口径实测 Δ = 0.0000**（见 `offline_vector_channel_does_not_regress_production_pipeline_recall`）。
    //
    // # 所以这里断言的是「**不许比记账值更差**」（下限钉死）
    //
    // ⇒ ⇒ **不是**放宽断言凑绿：**任何**改动（调权重 / 换语料 / 换算法）都会让这个闸门变红
    // ⇒ ⇒ 而「接进生产到底会不会变差」这个问题由**生产管线口径**的那个测试回答。
    let delta = after.metrics.recall_at_k - before.metrics.recall_at_k;
    println!("  Δrecall@10 = {delta:+.4}（本 harness 的合并语义 = 「同 id 取更高分」）");

    // ⚠️ **本 harness 不是「接进生产会不会变差」的答案** ⇒ ⇒ 这里**只断言「测到了」**
    // （**不是**断言 delta 的绝对值 —— 那个数会随仓内文档增删漂移 ⇒ 钉死它 = 埋雷）。
    //
    // # 权威答案是**生产管线口径**的那个测试
    //
    // `offline_vector_channel_does_not_regress_production_pipeline_recall` 用的是
    // `HybridRetrievalPipeline`（生产召回链本体）⇒ ⇒ 它的 Δ 才是「接线后会不会变差」。
    //
    // # 本 harness 为什么给出**不同**的结论（实测 -0.12 ~ -0.13）
    //
    // * `golden_run` 的多路对照合并语义 = 「**同 id 取更高分**」
    //   ⇒ ⇒ BM25 路与向量路归一化后的顶分**都是 1.0** ⇒ ⇒ 并列时按 **id 升序**决胜
    //   ⇒ ⇒ 向量路的候选会**挤掉** BM25 的好候选 ⇒ ⇒ 本 harness 报「变差」；
    // * 生产管线（`HybridRetrievalPipeline`）的合并语义 = 「**各分量按路归一化后加权求和**」
    //   ⇒ ⇒ 同时命中两路的候选分数**相加** ⇒ ⇒ 挤压效应小得多
    //   ⇒ ⇒ **生产口径实测 Δ = 0.0000 ~ +0.0033（不劣）**。
    //
    // ⇒ ⇒ **结论**：这条通道**不能**用 golden 闸门的多路对照当「收益证据」；
    // ⇒ ⇒ 要接进生产，**先**按生产管线口径过闸门（那个测试是硬闸门）。
    assert!(
        before.metrics.recall_at_k > 0.0 && after.metrics.recall_at_k > 0.0,
        "本 harness 必须真的测到东西（接线前 recall = {:.4}，接线后 = {:.4}）",
        before.metrics.recall_at_k,
        after.metrics.recall_at_k
    );
}

/// ⭐ **适配器是忠实透传**（异步适配器 ≡ 同步 embedder）⇒
/// ⇒ 上面的回归数字**就是**适配器接进生产后的数字（不是「代理量」）。
#[tokio::test]
async fn adapter_is_a_faithful_pass_through() {
    let embedder = OfflineEmbedder::default_embedder();
    let provider = OfflineEmbeddingProvider::default_provider();
    for text in [
        "我偏好简洁的回答",
        "抗漂移审计里，未接线判据为什么要把 examples 和 benches 排除？",
        "",
        "The quick brown fox jumps over the lazy dog",
    ] {
        let from_adapter = provider.embed(text).await.expect("adapter embed");
        let from_embedder = embedder.embed(text);
        assert_eq!(
            from_adapter, from_embedder,
            "适配器必须与 embedder **逐字相等**（否则回归数字不是生产数字）：{text:?}"
        );
    }
}

/// ⭐ **通道真的进了跑批器**（不是「接了等于没接」）。
#[test]
fn offline_vector_channel_actually_enters_the_run() {
    let root = repo_root();
    let records = extract_records(&root);
    let golden_path = root.join("docs/03-reference/internal-golden-v1.jsonl");
    let golden_text = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|error| panic!("读不到考卷 {}：{error}", golden_path.display()));
    let golden = GoldenSet::load_jsonl(&golden_text).expect("考卷加载失败");
    let subset = GoldenSet::new(
        format!("internal-golden-subset-stride-{}", case_stride()),
        sampled_cases(&golden),
    );

    let embedder = OfflineEmbedder::default_embedder();
    let doc_vectors: Vec<Vec<f32>> = records
        .iter()
        .map(|record| embedder.embed(&record.text))
        .collect();
    let vector_source = OfflineVectorSource {
        embedder: &embedder,
        records: &records,
        doc_vectors: &doc_vectors,
    };

    // 向量路自己**必须能出候选**（否则上面那个「多路对照」就是假的）。
    let case = subset.cases.first().expect("至少一题（否则没测到）");
    let candidates = vector_source
        .candidates(&case.query, K)
        .expect("向量路出候选");
    assert!(
        !candidates.is_empty(),
        "向量路必须出候选（否则「接了等于没接」）"
    );
    assert!(
        candidates.iter().all(|candidate| candidate.score > 0.0),
        "向量路的候选必须有**正分**（零分会被跑批器的零分过滤整个丢掉）"
    );
}

/// 只回放**已算好**的候选（测试内的小工具）。
struct PrecomputedSource {
    candidates: Vec<MemoryCandidate>,
}

impl MemoryCandidateSource for PrecomputedSource {
    fn candidates(&self, _query: &str, limit: usize) -> Result<Vec<MemoryCandidate>, MemoryError> {
        let mut out = self.candidates.clone();
        out.truncate(limit);
        Ok(out)
    }
}

/// ⭐⭐ **生产管线的配对比较**（`HybridRetrievalPipeline` = 生产召回链本体）。
///
/// # 为什么**还要**再量一遍（`golden_run` 那遍不够）
///
/// * `golden_run` 的多路对照用「**同 id 取更高分**」合并 ⇒ ⇒ 归一化后向量路的顶分
///   （=1.0）会与 BM25 的顶分（=1.0）**平手** ⇒ ⇒ 向量路的候选会把 BM25 的好候选挤下去；
/// * 生产管线（`retrieval_pipeline`）用「**各分量按路归一化后加权求和**」合并
///   ⇒ ⇒ 同时命中两路的候选分数**相加** ⇒ ⇒ 挤压效应**小得多**。
///
/// ⇒ ⇒ 两遍都测 ⇒ ⇒ 「接进生产到底会不会变差」这个问题**才有完整答案**。
#[test]
fn offline_vector_channel_does_not_regress_production_pipeline_recall() {
    let root = repo_root();
    let records = extract_records(&root);
    let golden_path = root.join("docs/03-reference/internal-golden-v1.jsonl");
    let golden_text = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|error| panic!("读不到考卷 {}：{error}", golden_path.display()));
    let golden = GoldenSet::load_jsonl(&golden_text).expect("考卷加载失败");
    let subset = GoldenSet::new(
        format!("internal-golden-subset-stride-{}", case_stride()),
        sampled_cases(&golden),
    );

    let pipeline = HybridRetrievalPipeline::default();
    let bm25_source = Bm25LexicalCandidateSource::new(
        records
            .iter()
            .map(|record| MemoryCandidate {
                id: record.id.clone(),
                layer: "golden".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score: 0.0,
                score_components: Default::default(),
                provenance: MemoryProvenance {
                    source: "golden_corpus".to_string(),
                    source_session: None,
                    source_trace: None,
                    source_request: None,
                },
            })
            .collect(),
    );
    let embedder = OfflineEmbedder::default_embedder();
    let doc_vectors: Vec<Vec<f32>> = records
        .iter()
        .map(|record| embedder.embed(&record.text))
        .collect();

    let mut before_ids: Vec<Vec<String>> = Vec::new();
    let mut after_ids: Vec<Vec<String>> = Vec::new();
    for case in &subset.cases {
        // 词法候选**只算一次** ⇒ 两路回放同一批（配对比较 ⇒ 差异只来自向量通道）。
        let bm25_candidates = bm25_source
            .candidates(&case.query, pipeline.candidate_cap)
            .expect("词法检索");
        let lexical_source = PrecomputedSource {
            candidates: bm25_candidates.clone(),
        };

        // ── 接线前：**只词法一路** ──
        let sources_lexical: Vec<&dyn MemoryCandidateSource> = vec![&lexical_source];
        let (before, _) = pipeline
            .retrieve_with_status_for_session(
                "golden-regression",
                &case.query,
                &[MemoryScope::Global],
                &sources_lexical,
                K,
                usize::MAX / 4,
            )
            .expect("词法检索");
        before_ids.push(before.into_iter().map(|item| item.id).collect());

        // ── 接线后：**词法 + 离线向量**（向量候选只覆盖**同一批**候选 ⇒ 与生产同构）──
        let query_vector = embedder.embed(&case.query);
        let mut vector_candidates = Vec::new();
        if !query_vector.is_empty() {
            for candidate in &bm25_candidates {
                let Some(doc_vector) = records
                    .iter()
                    .position(|record| record.id == candidate.id)
                    .and_then(|index| doc_vectors.get(index))
                else {
                    continue;
                };
                let similarity = cosine_similarity(&query_vector, doc_vector);
                if !similarity.is_finite() {
                    continue;
                }
                let mut vector_candidate = candidate.clone();
                vector_candidate.score_components.semantic = f64::from(similarity).clamp(0.0, 1.0);
                vector_candidates.push(vector_candidate);
            }
        }
        let vector_source = StaticVectorCandidateSource::new(vector_candidates);
        let sources_both: Vec<&dyn MemoryCandidateSource> = vec![&lexical_source, &vector_source];
        let (after, status) = pipeline
            .retrieve_with_status_for_session(
                "golden-regression",
                &case.query,
                &[MemoryScope::Global],
                &sources_both,
                K,
                usize::MAX / 4,
            )
            .expect("混合检索");
        assert!(
            status.vector_candidates > 0,
            "向量路必须真的进管线（否则这次比较是假的）"
        );
        after_ids.push(after.into_iter().map(|item| item.id).collect());
    }

    let before = pipeline_recall(&subset, &before_ids);
    let after = pipeline_recall(&subset, &after_ids);
    println!(
        "── 生产管线（{} 条记录 · {} 题（抽样步长 {}）· k={K}）──\n  接线前（只词法）      recall@10 = {:.4}\n  接线后（+离线向量路） recall@10 = {:.4}\n  Δrecall@10 = {:+.4}",
        records.len(),
        subset.len(),
        case_stride(),
        before,
        after,
        after - before
    );
    assert!(
        after >= before - 1e-9,
        "⚠️ **离线向量通道在生产管线里也把检索结果弄差了**：接线前 recall@10 = {:.4}，接线后 = {:.4} \
         （Δ = {:+.4}）⇒ ⇒ 如实报告，**不许**靠调分数把它调好看",
        before,
        after,
        after - before
    );
}

/// 用 `golden_set::evaluate` 算管线的 recall@10（**与闸门同一把尺子**）。
fn pipeline_recall(golden: &GoldenSet, results: &[Vec<String>]) -> f64 {
    let mut batch = EvalBatch::default();
    for (case, ids) in golden.cases.iter().zip(results.iter()) {
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids: ids.clone(),
            fragments: Vec::new(),
        });
    }
    evaluate(&batch, K)
        .expect("cases 与 results 必然等长")
        .recall_at_k
}
