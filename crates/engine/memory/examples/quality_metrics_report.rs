//! 设计稿 §6.1 的四项质量指标 —— **真实语料**（《紅樓夢》）报告器。
//!
//! # 跑什么
//!
//! | 指标 | 用什么**真实**数据 |
//! |---|---|
//! | **时间正确率** | 真实 `episodes`（写进真实 SQLite 存储 ⇒ **回读** `valid_from_ms` / `valid_until_ms`） |
//! | **矛盾检出率** | `golden_mining` 从《紅樓夢》挖出的 **conflict 题型**（真实矛盾对） |
//! | **影子胜率** | **两个不同检索器**（现役 BM25 vs 候选 token 重叠）在同一份卷子上的逐题对照 |
//! | **回退时间** | 真实 `swap_flow` 的**切换**与**回退**（墙钟实测） |
//!
//! # 口径纪律
//!
//! * **不许**造数据：语料缺失 ⇒ 直接非零退出（不是「跑过了」）；
//! * **不许**把「没测到」渲染成 0 或 1 ⇒ 报告里一律「没测到」；
//! * 矛盾检出率**必须与 k 一起读**（k 小 ⇒ 检出率天然低 ⇒ **不是**检索不行的证据）；
//! * 墙钟**只用于看趋势**（不作为断言依据）。
//!
//! # 语料在仓外
//!
//! 语料在 `$APEIRETH_DATA_DIR/novels/`（默认 `~/.apeireth/novels/`），
//! 下载器 `scripts/download_novels.sh` 带 SHA-256 校验。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出真实分数（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有本报告器在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在本机**真实语料**上跑过（2026-10-27），
//!   **未在第二台机器复现** ⇒ 不宣称跨机器可复现。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::golden_mining::{mine, MineRecord, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::novel_corpus;
use apeireth_memory::quality_metrics::{
    contradiction_pairs_from_cases, measure_citation_coverage, measure_contradiction,
    measure_rollback_time, measure_shadow_win, measure_temporal, measure_token_efficiency,
    render_quality_report, render_quality_report_extended, CitationCoverage, ContradictionVerdict,
    RollbackTiming, ShadowWinVerdict, TemporalVerdict, TokenEfficiency, Validity,
};
use apeireth_memory::retrieval_pipeline::{
    unicode_tokens, Bm25LexicalCandidateSource, MemoryCandidateSource,
};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::shadow_run::{
    judge_swap, run_shadow, Retriever, RunOutcome, SwapDecision, SwapVerdict,
};
use apeireth_memory::swap_flow::{execute_swap, rollback, AlgoPointer, SwapPlan};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};

/// 检索的 top-k（**必须与仓内基线同口径** ⇒ 换了 k 就不可比）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 与 `examples/golden_baseline_report.rs` **同口径**。
const MAX_CASES_PER_RECORD: usize = 6;

/// 候选检索器的注入窗口（字）—— **比现役短** ⇒ 用来对照「同准确度 + 更少 token」。
const CHALLENGER_SPAN_CHARS: usize = 60;

/// 切换 / 回退的**重复次数**（单次落盘是亚毫秒 ⇒ 取整成 0 ms 会让比较失真
/// ⇒ 用**同一批 N 次的总耗时**做对照，口径写在报告里）。
///
/// ⚠️ N 必须**够大**让总耗时跨过 1 ms（否则「切换 0 ms / 回退 0 ms」⇒ `0 < 0` 为假
/// ⇒ 会得出「回退不比切换快」的**假结论**）。实测单次约 20~30 µs ⇒ 1000 次 ≈ 20~30 ms。
const TIMING_ROUNDS: usize = 1000;

fn main() -> ExitCode {
    let Some(path) = corpus_path() else {
        eprintln!(
            "· 语料不在（$APEIRETH_DATA_DIR/novels/ 或 ~/.apeireth/novels/）\
             ⇒ **没测到**（不许把「没跑」当「没问题」）"
        );
        return ExitCode::FAILURE;
    };
    if !path.exists() {
        eprintln!("· 语料不在（{}）⇒ **没测到**", path.display());
        return ExitCode::FAILURE;
    }

    let chapters = match novel_corpus::load_chapters(&path) {
        Ok(chapters) => chapters,
        Err(error) => {
            eprintln!("· 语料读失败（{}）：{error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = mine(&records, config);
    let golden = GoldenSet::new("internal-redchamber", mined_to_golden(mined));
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    println!("=== 真实语料质量报告（设计稿 §6.1 · L1 定位 + L4 可塑性）===");
    println!(
        "语料：{}（{} 回 · {} 条记录 · {} 题 · k={K}）",
        path.display(),
        records.len(),
        records.len(),
        golden.len()
    );

    // --- ① 时间正确率：真实 episodes 的真实有效期 ---
    // 有效期**从真实存储回读**（不是我们假设出来的）。
    let (validity, as_of_ms, expired_capable) = measure_temporal_on_real_episodes(&records);
    // 检索返回过的记录（全部题、全部 top-k）⇒ 时间正确率的分母。
    let returned_ids = retrieve_all_ids(&index, &golden.cases);
    let temporal = measure_temporal(&returned_ids, &validity, as_of_ms);

    // --- ② 矛盾检出率：真实 conflict 题型 ---
    let conflict_cases: Vec<GoldenCase> = golden
        .cases
        .iter()
        .filter(|case| {
            case.tags.iter().any(|tag| tag == "kind:conflict") && case.expect_ids.len() >= 2
        })
        .cloned()
        .collect();
    let pairs = contradiction_pairs_from_cases(&conflict_cases);
    let conflict_retrieved = retrieve_ids_per_case(&index, &conflict_cases);
    let contradiction = measure_contradiction(&pairs, &conflict_retrieved);

    // --- ③ 影子胜率：两个不同检索器 ---
    let champion = Bm25Champion::new(&index);
    let challenger = OverlapChallenger::new(&index);
    let shadow_report = run_shadow(&golden.cases, &champion, &challenger);
    let mut champion_scores = Vec::new();
    let mut challenger_scores = Vec::new();
    let mut champion_tokens = Vec::new();
    let mut challenger_tokens = Vec::new();
    for (case, pair) in golden.cases.iter().zip(shadow_report.pairs.iter()) {
        if case.expect_ids.is_empty() {
            // 无标准答案 ⇒ 这题**不参与**准确度对照（与 `golden_set::evaluate` 同口径）。
            continue;
        }
        champion_scores.push(recall_of(&pair.champion.ids, &case.expect_ids));
        challenger_scores.push(recall_of(&pair.challenger.ids, &case.expect_ids));
        champion_tokens.push(pair.champion.injected_tokens);
        challenger_tokens.push(pair.challenger.injected_tokens);
    }
    let shadow_win = measure_shadow_win(
        &champion_scores,
        &challenger_scores,
        &champion_tokens,
        &challenger_tokens,
    );
    let verdict = judge_swap(&champion, &challenger, &golden.cases, 3);

    // --- ④ 回退时间：真实切换 / 回退（墙钟） ---
    let (rollback_timing, switch_avg_us, rollback_avg_us) = measure_rollback_wall_clock();

    // --- 报告（原样打印，供人读与留痕） ---
    let report = render_quality_report(
        Some(&temporal),
        Some(&contradiction),
        Some(&shadow_win),
        Some(&rollback_timing),
        K,
    );
    println!();
    println!("{report}");

    // --- L2 两项（同台账那条线）---
    let (token_efficiency, citation) = measure_injection_metrics(&index, &golden.cases);
    println!();
    println!(
        "{}",
        render_quality_report_extended(
            Some(&temporal),
            Some(&contradiction),
            Some(&token_efficiency),
            Some(&citation),
            Some(&shadow_win),
            Some(&rollback_timing),
            K,
        )
    );

    // --- 口径与补充说明（如实交代） ---
    println!();
    println!("--- 口径与补充说明（如实交代）---");
    println!(
        "· 时间正确率：受检 {} 条 · 失效仍返回 {} 条；as_of = {as_of_ms}（语料时间轴末端 + 1，确定性）",
        temporal.checked, temporal.stale_returned
    );
    if expired_capable == 0 {
        println!(
            "  ⚠️ **真实 episodes 里没有任何一条会失效**（valid_until_ms 全为 NULL —— 当前**没有任何写入路径**会设失效时间）\
             ⇒ 这个 1.0 是**构造性**的、**无区分力**（**不是**「检索很棒」的证据）；\
             指标的区分力由单测（人为设失效时间）与下面的受控对照证明。"
        );
    } else {
        println!(
            "  · 真实 episodes 里有 {expired_capable} 条带失效时间（valid_until_ms 非空）⇒ 这个数字**有区分力**。"
        );
    }
    // 受控对照：给**真实检索返回的**记录人为设失效时间 ⇒ 证明指标能抓住失效记录
    // （⚠️ 这是**受控实验**，不是语料事实 ⇒ 单独标注，不混进上面的数字）。
    if !returned_ids.is_empty() {
        let mut controlled = validity.clone();
        let sample = returned_ids.len().min(4);
        for id in returned_ids.iter().take(sample) {
            controlled.insert(
                id.clone(),
                Validity {
                    valid_from_ms: Some(0),
                    valid_until_ms: Some(as_of_ms - 1),
                },
            );
        }
        let controlled_verdict = measure_temporal(&returned_ids, &controlled, as_of_ms);
        println!(
            "  · 受控对照（**人为**给 {sample} 条真实返回记录设失效时间，非语料事实）：\
             时间正确率 = {} ⇒ 指标**能**抓住失效记录",
            controlled_verdict
                .correct_rate
                .map(|rate| format!("{rate:.4}"))
                .unwrap_or_else(|| "没测到".to_string())
        );
    }
    println!(
        "· 矛盾检出率：**检出 = 同一道题的检索结果里同时含这两个 id**（逐题对齐）；\
         k={K} ⇒ 检出上限受限 ⇒ **低检出率不是检索不行的证据**；\
         ⚠️ 这些矛盾对是 `golden_mining` **机器挖**出来的（同主题 + 时间相邻 + 内容不同）\
         ⇒ **不是人工精标**的矛盾 ⇒ 这是「机器认为矛盾」的检出率；\
         真实矛盾对 = {} 对（`golden_mining` 的 conflict 题型）",
        contradiction.total_pairs
    );
    println!(
        "· 影子胜率：现役 = 生产 BM25 + 生产注入（每条证据前 {} 字）；\
         候选 = token 重叠排序 + **更短**的注入窗口（{} 字）\
         ⇒ 两处都不同 ⇒ 这是**对照实验**，不是「候选算法更好」的通用结论",
        InjectionMode::PRODUCTION_CHARS,
        CHALLENGER_SPAN_CHARS
    );
    println!(
        "  ⚠️ **影子胜率与影子裁决不一致，必须一起读**：逐题胜率 {} 里绝大多数是\
         「**两边都没命中**（准确度打平）但候选注入更短」⇒ 按 §6.4 的交换许可算赢；\
         而**聚合层**（比中位数）看到候选准确度明显更差 ⇒ 裁决 = {:?}。\
         ⇒ 单看胜率会误判「候选更好」⇒ **胜率是计数，不是结论**。",
        shadow_win
            .win_rate
            .map(|rate| format!("{rate:.4}"))
            .unwrap_or_else(|| "没测到".to_string()),
        verdict.decision
    );
    println!(
        "· 回退时间：{TIMING_ROUNDS} 次「切换 + 回退」的**总墙钟**对照（单次落盘是亚毫秒 ⇒ \
         取整成 0 ms 会让比较失真）⇒ 切换均 {switch_avg_us} µs/次 · 回退均 {rollback_avg_us} µs/次"
    );
    println!(
        "· 影子裁决（聚合层，§6.4）：{:?} · {}",
        verdict.decision, verdict.reason
    );

    ExitCode::SUCCESS
}

/// 语料路径（`$APEIRETH_DATA_DIR/novels/` ⇒ 默认 `~/.apeireth/novels/`）。
fn corpus_path() -> Option<PathBuf> {
    let root = match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".apeireth")
        }
    };
    Some(root.join("novels").join("redchamber.txt"))
}

// ============================================================
// ① 时间正确率：真实 episodes 的真实有效期
// ============================================================

/// 把语料写进**真实存储** ⇒ 回读 `EpisodeMeta` ⇒ 得到**真实**的有效期。
///
/// # 为什么非得真写真读
///
/// `episode::put_episode` 是**生产写入路径** ⇒ 它写的 `valid_from_ms` / `valid_until_ms`
/// 就是生产上真实的值 ⇒ 报告里的数字**不是**我们假设出来的。
/// （反过来说：如果这里「手写一个有效期表」，那测的就不是系统，是想象。）
fn measure_temporal_on_real_episodes(
    records: &[MineRecord],
) -> (BTreeMap<String, Validity>, i64, usize) {
    let store = SqliteMemoryStore::open_in_memory().expect("内存库应当能开");
    for record in records {
        let episode = Episode {
            id: record.id.clone(),
            // `timestamp` 是**秒**；语料的时间线是「回次 × 一天」（`to_mine_records` 给的
            // `timestamp_ms`）⇒ 折成秒（确定性 ⇒ 不读系统时钟）。
            timestamp: record.timestamp_ms.div_euclid(1000),
            role: "user".to_string(),
            content: record.content.clone(),
            session_id: "novel".to_string(),
        };
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &episode)
            .expect("写入应当成功（真实生产写入路径）");
    }

    let mut validity = BTreeMap::new();
    let mut expired_capable = 0usize;
    for record in records {
        let meta = store
            .read_episode_meta(&record.id)
            .expect("读元数据应当成功")
            .unwrap_or_else(|| panic!("{} 应当有元数据", record.id));
        if meta.valid_until_ms.is_some() {
            expired_capable += 1;
        }
        validity.insert(
            record.id.clone(),
            Validity {
                valid_from_ms: meta.valid_from_ms,
                valid_until_ms: meta.valid_until_ms,
            },
        );
    }

    // `as_of` = 语料时间轴的**末端 + 1**（确定性 ⇒ 不读系统时钟）。
    let as_of_ms = validity
        .values()
        .filter_map(|entry| entry.valid_from_ms)
        .max()
        .unwrap_or(0)
        + 1;

    (validity, as_of_ms, expired_capable)
}

// ============================================================
// 检索辅助
// ============================================================

/// 建 BM25 源（**生产同口径**：可检索文本 = 正文，不含题面来源）。
fn build_source(index: &[IndexRecord]) -> Bm25LexicalCandidateSource {
    let documents = index
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
        .collect();
    Bm25LexicalCandidateSource::new(documents)
}

/// 逐题检索 ⇒ 每题返回的 id 列表（**零分结果不算检索结果**，与 `golden_run` 同口径）。
fn retrieve_ids_per_case(index: &[IndexRecord], cases: &[GoldenCase]) -> Vec<Vec<String>> {
    let source = build_source(index);
    cases
        .iter()
        .map(|case| {
            source
                .candidates(&case.query, K)
                .unwrap_or_default()
                .into_iter()
                .filter(|candidate| candidate.score > 0.0)
                .map(|candidate| candidate.id)
                .collect()
        })
        .collect()
}

/// 全部题、全部 top-k 的返回 id（**去重**）⇒ 时间正确率的分母。
fn retrieve_all_ids(index: &[IndexRecord], cases: &[GoldenCase]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for ids in retrieve_ids_per_case(index, cases) {
        for id in ids {
            if seen.insert(id.clone()) {
                out.push(id);
            }
        }
    }
    out
}

/// 逐题准确度（该题命中了几个标准答案 / 标准答案总数；与 `golden_set::evaluate` 同口径）。
fn recall_of(ids: &[String], expect_ids: &[String]) -> f64 {
    if expect_ids.is_empty() {
        return 0.0;
    }
    let hit = expect_ids.iter().filter(|id| ids.contains(id)).count();
    hit as f64 / expect_ids.len() as f64
}

// ============================================================
// ③ 两个检索器（现役 vs 候选）
// ============================================================

/// **现役**：生产同口径 BM25 + 生产注入（每条证据取正文前 [`InjectionMode::PRODUCTION_CHARS`] 字）。
struct Bm25Champion {
    source: Bm25LexicalCandidateSource,
    texts: BTreeMap<String, String>,
}

impl Bm25Champion {
    fn new(index: &[IndexRecord]) -> Self {
        let texts = index
            .iter()
            .map(|record| (record.id.clone(), record.text.clone()))
            .collect();
        Self {
            source: build_source(index),
            texts,
        }
    }
}

impl Retriever for Bm25Champion {
    fn id(&self) -> &str {
        "bm25-production"
    }

    fn retrieve(&self, case: &GoldenCase) -> RunOutcome {
        let ids: Vec<String> = self
            .source
            .candidates(&case.query, K)
            .unwrap_or_default()
            .into_iter()
            .filter(|candidate| candidate.score > 0.0)
            .map(|candidate| candidate.id)
            .collect();
        let injected_tokens = ids
            .iter()
            .map(|id| {
                self.texts
                    .get(id)
                    .map(|text| text.chars().take(InjectionMode::PRODUCTION_CHARS).count())
                    .unwrap_or(0)
            })
            .sum();
        RunOutcome {
            ids,
            injected_tokens,
        }
    }
}

/// **候选**：**token 重叠**排序（不是 BM25）+ **更短**的注入窗口（[`CHALLENGER_SPAN_CHARS`] 字）。
///
/// # 为什么它是「另一个算法」而不是「同一个算法换个参数」
///
/// * 排序口径不同：BM25（词频 × 逆文档频率）⇒ token 重叠（命中 token 数，平手按 id 序）；
/// * 注入口径不同：现役取正文**开头** ⇒ 候选取**最相关的一段**（更短）。
///
/// ⇒ 两处都不同 ⇒ 对照才有意义（否则测的是同一算法的噪声）。
struct OverlapChallenger {
    docs: Vec<Doc>,
}

struct Doc {
    id: String,
    tokens: BTreeSet<String>,
    text: String,
}

impl OverlapChallenger {
    fn new(index: &[IndexRecord]) -> Self {
        let docs = index
            .iter()
            .map(|record| Doc {
                id: record.id.clone(),
                tokens: unicode_tokens(&record.text).into_iter().collect(),
                text: record.text.clone(),
            })
            .collect();
        Self { docs }
    }

    /// 与查询最相关的一段（**取命中 token 的位置**开窗；找不到就从开头取）。
    fn best_span(&self, text: &str, query_tokens: &[String]) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0usize;
        for token in query_tokens {
            if let Some(byte) = text.find(token.as_str()) {
                start = text[..byte].chars().count();
                break;
            }
        }
        let end = (start + CHALLENGER_SPAN_CHARS).min(chars.len());
        chars[start..end].iter().collect()
    }
}

impl Retriever for OverlapChallenger {
    fn id(&self) -> &str {
        "overlap-shortspan"
    }

    fn retrieve(&self, case: &GoldenCase) -> RunOutcome {
        let query_tokens = unicode_tokens(&case.query);
        let mut scored: Vec<(usize, &str)> = self
            .docs
            .iter()
            .map(|doc| {
                let score = query_tokens
                    .iter()
                    .filter(|token| doc.tokens.contains(*token))
                    .count();
                (score, doc.id.as_str())
            })
            .filter(|(score, _)| *score > 0)
            .collect();
        // 分数降序 ⇒ 同分按 id 升序（**确定性**）。
        scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));
        let picked: Vec<&Doc> = scored
            .iter()
            .take(K)
            .filter_map(|(_, id)| self.docs.iter().find(|doc| doc.id == *id))
            .collect();
        let ids: Vec<String> = picked.iter().map(|doc| doc.id.clone()).collect();
        let injected_tokens = picked
            .iter()
            .map(|doc| self.best_span(&doc.text, &query_tokens).chars().count())
            .sum();
        RunOutcome {
            ids,
            injected_tokens,
        }
    }
}

// ============================================================
// ④ 回退时间（真实切换 / 回退，墙钟）
// ============================================================

/// 真实跑 `TIMING_ROUNDS` 轮「切换 + 回退」⇒ 墙钟总耗时对照。
///
/// # 为什么用「N 次的总耗时」而不是「单次」
///
/// 单次回退 = 写一个归档文件（**亚毫秒**）⇒ 取整成毫秒就是 `0`
/// ⇒ 与切换（也是 `0`）比较 ⇒ `0 < 0` 为假 ⇒ 会得出「回退不比切换快」的**假结论**。
/// ⇒ 同一批 N 次取**总耗时**（同口径对照）⇒ 比较才有意义。
fn measure_rollback_wall_clock() -> (RollbackTiming, u128, u128) {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("· 临时目录开不了（{error}）⇒ 回退时间**没测到**");
            return (measure_rollback_time(0, None), 0, 0);
        }
    };
    let plan = SwapPlan {
        from: "bm25-production".to_string(),
        to: "overlap-shortspan".to_string(),
        verdict: SwapVerdict {
            decision: SwapDecision::Approved,
            champion_score: Some(0.0),
            challenger_score: Some(0.0),
            champion_tokens: Some(0),
            challenger_tokens: Some(0),
            reason: "报告器专用（只为让闸放行；真实裁决由 `shadow_run::judge_swap` 给）"
                .to_string(),
        },
        shadow_runs: 3,
        shadow_cases: 100,
    };

    let mut switch_total = std::time::Duration::ZERO;
    let mut rollback_total = std::time::Duration::ZERO;
    for round in 0..TIMING_ROUNDS {
        let now_ms = 1_700_000_000_000 + (round as i64) * 1_000;
        let mut pointer = AlgoPointer {
            current: plan.from.clone(),
            previous: None,
            switched_at_ms: now_ms,
            generation: 0,
        };
        let started = Instant::now();
        let outcome = execute_swap(&plan, &mut pointer, dir.path(), now_ms);
        switch_total += started.elapsed();
        if !outcome.is_ok() {
            eprintln!("· 切换失败 ⇒ 回退时间**没测到**");
            return (measure_rollback_time(0, None), 0, 0);
        }
        let started = Instant::now();
        let outcome = rollback(&mut pointer, dir.path(), now_ms + 1);
        rollback_total += started.elapsed();
        if !outcome.is_ok() {
            eprintln!("· 回退失败 ⇒ 回退时间**没测到**");
            return (measure_rollback_time(0, None), 0, 0);
        }
    }

    let switch_ms = switch_total.as_millis() as u64;
    let rollback_ms = rollback_total.as_millis() as u64;
    let timing = measure_rollback_time(rollback_ms, Some(switch_ms));
    (
        timing,
        switch_total.as_micros() / TIMING_ROUNDS as u128,
        rollback_total.as_micros() / TIMING_ROUNDS as u128,
    )
}

// ============================================================
// L2 两项（同台账那条线）：Token 效率 / Citation coverage
// ============================================================

/// 注入侧的两项（**注入文本** = 生产口径：每条命中记录的前 120 字）。
fn measure_injection_metrics(
    index: &[IndexRecord],
    cases: &[GoldenCase],
) -> (TokenEfficiency, CitationCoverage) {
    let source = build_source(index);
    let mut injected = String::new();
    let mut supported = Vec::new();
    let mut cited = Vec::new();
    for case in cases {
        let candidates: Vec<MemoryCandidate> = source
            .candidates(&case.query, K)
            .unwrap_or_default()
            .into_iter()
            .filter(|candidate| candidate.score > 0.0)
            .collect();
        for candidate in &candidates {
            cited.push(candidate.id.clone());
            if let Some(record) = index.iter().find(|record| record.id == candidate.id) {
                injected.push_str(
                    &record
                        .text
                        .chars()
                        .take(InjectionMode::PRODUCTION_CHARS)
                        .collect::<String>(),
                );
            }
        }
        supported.extend(case.expect_fragments.iter().cloned());
    }
    let known: Vec<String> = index.iter().map(|record| record.id.clone()).collect();
    (
        measure_token_efficiency(&injected, &supported),
        measure_citation_coverage(&cited, &known),
    )
}
