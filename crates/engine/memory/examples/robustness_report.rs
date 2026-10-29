//! L3 鲁棒/成长报告（**真实语料**）：设计稿 §6.1 的四项到底多少分。
//!
//! # 这个报告回答什么
//!
//! 设计稿 §6.1 的 L3 四项（**规模延展斜率** / **噪声鲁棒** / **确定性（逐字节）** /
//! **越权命中率**）此前**只存在于文档** ⇒ 现在用**真实语料**（《紅樓夢》《水滸傳》）
//! 把它们**跑出数字**。
//!
//! # 口径（与仓里既有口径一致，否则数字不可比）
//!
//! * 语料 = **小说正文**（仓外 `$APEIRETH_DATA_DIR/novels/`，正文一个字都不入库）；
//! * 检索 = **生产用的 BM25**（`golden_run::run` 内部用 `Bm25LexicalCandidateSource`）；
//! * 评测 = `golden_set::evaluate`（与基线同源）；`k = 10`（与基线文件一致）；
//! * 挖题 = **放宽配置**（每条记录 6 题）⇒ 与内部卷首跑同口径；
//! * **确定性比的是 `golden_run::summarize` 的输出**（逐字节）。
//!
//! # 「越权」怎么**实测**（**不许假设**）
//!
//! 设计稿 §3.5 的铁律：D4 模式层**只产标注、不产记忆、永不回写**。
//! ⇒ 实测口径（本报告**真跑**，不是写死 0）：
//!
//! 1. 取原始层（D0 = 语料记录）的 **SHA-256 摘要**；
//! 2. 取**同一份卷子**的跑批结果；
//! 3. 跑一遍**模式层**（只产标注；它拿到的是 `&dyn MemoryView` ⇒ 类型层面就没有回写口）；
//! 4. 再取 D0 摘要与跑批结果 ⇒ **必须逐字节不变**；
//! 5. **负对照**：故意改一条记录的副本 ⇒ 摘要**必须变**
//!    （否则第 4 步的检查是空转 ⇒ 本条判定不可信）。
//!
//! ⇒ `attempts` = 模式层产出的标注条数（每条标注都是一次「本可回写」的机会）；
//! ⇒ `violations` = 上面 4 个检查里**真的失败**的那些（逐条列出 ⇒ 可定位）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出真实报告（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 **0** 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在本机**真实语料**上跑过，
//!   **未在第二台机器复现** ⇒ 不宣称跨机器可复现。

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, run, IndexRecord, InjectionMode, RunReport};
use apeireth_memory::golden_set::{GoldenSet, Metrics};
use apeireth_memory::memory_view::{
    AlgoStamp, AnnotationKind, AnnotationPayload, InMemoryMemoryView, MemoryAnnotation, MemoryView,
    ProvenanceChain, Record as ViewRecord, RecordId, Scope,
};
use apeireth_memory::novel_corpus;
use apeireth_memory::robustness_metrics::{
    inject_noise, measure_determinism, measure_noise_robustness, measure_overreach,
    measure_scale_slope, render_robustness_report, DeterminismVerdict, NoiseVerdict,
    OverreachVerdict, ScaleVerdict,
};
use sha2::{Digest, Sha256};

/// 一档噪声的实测结果（判定 + **证据**）。
///
/// 为什么单独建这个结构：光有 `NoiseVerdict` 只有 recall 一项 ⇒
/// 「扛住了」这句话需要**旁证**（真的掺了多少条？证据族指标动了没？）⇒ 一并记下来。
struct NoiseRow {
    /// 噪声比例（0.1 = 10%）。
    rate: f64,
    /// 判定（相对跌幅 + 是否扛住）。
    verdict: NoiseVerdict,
    /// **真的**被掺了噪声的记录条数（证明噪声不是空转）。
    touched: usize,
    /// 干净语料的完整指标。
    clean: Metrics,
    /// 掺噪后的完整指标。
    noisy: Metrics,
}

/// 评测的 top-k（**与基线文件的 `k` 一致**）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 与内部卷基线同口径。
const MAX_CASES_PER_RECORD: usize = 6;

/// 噪声档位（**掺进去的噪声比例**）。
const NOISE_RATES: [f64; 3] = [0.10, 0.20, 0.30];

/// 噪声种子（**固定** ⇒ 噪声实验可复现）。
const NOISE_SEED: u64 = 20261027;

/// 允许的最大相对跌幅（**门限**：超过 ⇒ 判「没扛住」）。
const MAX_DROP: f64 = 0.10;

/// 规模延展的「小规模」条数（真实语料有 ~120 回 ⇒ 小规模取前 30 条）。
const SMALL_SCALE_N: usize = 30;

/// 语料清单：`（名字, 仓外文件名）`（与 `recall_gap_report.rs` 同一张表）。
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

fn main() -> ExitCode {
    let dir = novels_dir();
    let mut loaded = 0usize;

    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        match report_one(name, &path) {
            Ok(()) => loaded += 1,
            Err(error) => {
                eprintln!("· {name}：**跑失败**（{}）：{error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    if loaded == 0 {
        eprintln!(
            "一份语料都没跑到（目录 {}）⇒ **不能出结论**（不许把「没跑」当「没问题」）",
            dir.display()
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// 语料目录（`$APEIRETH_DATA_DIR/novels/`，默认 `~/.apeireth/novels/`）。
fn novels_dir() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".apeireth").join("novels")
        }
    }
}

/// 跑一份语料 ⇒ L3 四项报告。
fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = golden_mining::mine(&records, config);
    let cases = mined_to_golden(mined);
    let golden = GoldenSet::new(format!("novel-{name}-v1"), cases);

    // 可检索文本 = **正文**（与基线同口径；题面来源不进可检索文本 ⇒ 不泄漏答案）。
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    println!("════════ {name} ════════");
    println!(
        "语料 {} 条（回）· 题 {} 道 · k={K}",
        index.len(),
        golden.cases.len()
    );
    println!();

    // ---------- ① 确定性（逐字节） ----------
    let determinism = measure_determinism(&|| run_batch(&index, &golden));
    // 额外证据（比摘要**更严**）：整份跑批报告逐字段相等。
    let first = run_batch(&index, &golden);
    let second = run_batch(&index, &golden);
    println!("── 确定性：比的是哪两段文本 ──");
    println!("  比较对象 = `golden_run::summarize(&RunReport)` 的输出（两段文本逐字节相等）");
    println!(
        "  ── 第一次（{} 字）──",
        determinism.digest_a.chars().count()
    );
    for line in determinism.digest_a.lines() {
        println!("    {line}");
    }
    println!(
        "  ── 第二次（{} 字）──",
        determinism.digest_b.chars().count()
    );
    for line in determinism.digest_b.lines() {
        println!("    {line}");
    }
    println!(
        "  摘要逐字节相同 = {}（整份 `RunReport` 逐字段相等 = {}）",
        determinism.identical,
        first == second
    );
    println!();

    // ---------- ② 噪声鲁棒（10% / 20% / 30%） ----------
    let pairs: Vec<(String, String)> = index
        .iter()
        .map(|record| (record.id.clone(), record.text.clone()))
        .collect();
    let mut noise_rows: Vec<NoiseRow> = Vec::new();
    let clean_metrics = run_batch(&index, &golden).metrics;
    for rate in NOISE_RATES {
        let noised = inject_noise(&pairs, rate, NOISE_SEED);
        // 「真的被掺了多少条」——**证明噪声不是空转**（否则「扛住了」是空话）。
        let touched = noised
            .iter()
            .zip(pairs.iter())
            .filter(|((_, text), (_, original))| text != original)
            .count();
        let noisy_index: Vec<IndexRecord> = noised
            .into_iter()
            .map(|(id, text)| IndexRecord { id, text })
            .collect();
        let noisy_metrics = run_batch(&noisy_index, &golden).metrics;
        let verdict = measure_noise_robustness(
            &|| run_batch(&index, &golden),
            &|| run_batch(&noisy_index, &golden),
            MAX_DROP,
        );
        noise_rows.push(NoiseRow {
            rate,
            verdict,
            touched,
            clean: clean_metrics.clone(),
            noisy: noisy_metrics,
        });
    }
    println!(
        "── 噪声鲁棒：掺 10% / 20% / 30%（种子 {NOISE_SEED}，门限 {:.0}%）──",
        MAX_DROP * 100.0
    );
    for row in &noise_rows {
        println!(
            "  噪声 {:>4.0}%  真的被掺噪声的记录 {}/{} 条  干净 recall@{}={:.4} → 掺噪 {:.4}  相对跌幅 {:>6.2}%  ⇒ {}",
            row.rate * 100.0,
            row.touched,
            index.len(),
            K,
            row.verdict.clean_recall,
            row.verdict.noisy_recall,
            row.verdict.drop_ratio * 100.0,
            if row.verdict.survived {
                "扛住了"
            } else {
                "**没扛住**"
            }
        );
        println!(
            "            证据族：nDCG {:.4} → {:.4} · evidence_precision {:.4} → {:.4} · groundedness {:.4} → {:.4}",
            row.clean.ndcg_at_k,
            row.noisy.ndcg_at_k,
            row.clean.evidence_precision,
            row.noisy.evidence_precision,
            row.clean.groundedness,
            row.noisy.groundedness
        );
    }
    println!();

    // ---------- ③ 规模延展斜率 ----------
    let small_n = SMALL_SCALE_N.min(index.len());
    let small_index: Vec<IndexRecord> = index[..small_n].to_vec();
    let scale = measure_scale_slope(
        &|| run_batch(&small_index, &golden),
        &|| run_batch(&index, &golden),
        small_n,
        index.len(),
    );
    println!(
        "── 规模延展：同一份卷子（{} 道题），语料 {} 条 → {} 条 ──",
        golden.cases.len(),
        scale.small_n,
        scale.large_n
    );
    println!(
        "  耗时 {} ms → {} ms（墙钟）⇒ 斜率 {:.3}（1.0 = 线性；> 1.5 = 超线性）",
        scale.small_ms, scale.large_ms, scale.slope
    );
    println!("  ⚠️ **斜率是趋势，不是断言依据**（墙钟受机器负载影响 ⇒ 只同机比较）");
    println!();

    // ---------- ④ 越权命中率（**实测**，不许假设） ----------
    let (overreach, overreach_notes) = measure_overreach_in_real_run(&index, &golden);
    println!("── 越权命中率：实测口径与证据 ──");
    for note in &overreach_notes {
        println!("  · {note}");
    }
    println!();

    // ---------- 汇总：render_robustness_report 原样输出 ----------
    for row in &noise_rows {
        println!(
            "── render_robustness_report 原样输出（噪声 {:.0}%）──",
            row.rate * 100.0
        );
        println!(
            "{}",
            render_robustness_report(
                Some(&determinism),
                Some(&row.verdict),
                Some(&scale),
                Some(&overreach)
            )
        );
        println!();
    }

    Ok(())
}

/// 跑一次批（语料 + 卷子 ⇒ [`RunReport`]）。
fn run_batch(index: &[IndexRecord], golden: &GoldenSet) -> RunReport {
    run(index, golden, K, InjectionMode::Production).expect("跑批失败")
}

/// 原始层（D0）摘要：逐条 `id\0content\n` 喂进 SHA-256（**顺序敏感** ⇒ 换序也会变）。
fn corpus_digest(index: &[IndexRecord]) -> String {
    let mut hasher = Sha256::new();
    for record in index {
        hasher.update(record.id.as_bytes());
        hasher.update([0u8]);
        hasher.update(record.text.as_bytes());
        hasher.update([b'\n']);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// 建一个**只读**视图（D4 模式层能拿到的就是它）。
fn build_view(index: &[IndexRecord]) -> InMemoryMemoryView {
    let records: Vec<ViewRecord> = index
        .iter()
        .enumerate()
        .map(|(position, record)| ViewRecord {
            id: RecordId::new(record.id.clone()),
            content: record.text.clone(),
            timestamp_ms: position as i64 * 86_400_000,
            provenance: ProvenanceChain::default(),
            scope: Scope::Global,
        })
        .collect();
    InMemoryMemoryView::new(records)
}

/// 跑一遍**最小模式层**：只产标注（**不回写**）。
///
/// # 为什么用它当「模式层」
///
/// 设计稿 §3.5 定的纪律是「只产 `MemoryAnnotation`、写入 D4 标注区、可全量重算」。
/// 这里按纪律实现一个**最小**版本（标签 + 分数 + 关联）⇒ 它拿到的是
/// `&dyn MemoryView`（**类型层面就没有回写口**）⇒ 越权只可能来自「绕过契约」，
/// 而那会被下面的摘要比对当场抓住。
fn run_pattern_layer(view: &dyn MemoryView, index: &[IndexRecord]) -> Vec<MemoryAnnotation> {
    let stamp = AlgoStamp {
        algo_id: "robustness-report/minimal-pattern".to_string(),
        version: "v1".to_string(),
        params_digest: "none".to_string(),
    };
    let mut annotations: Vec<MemoryAnnotation> = Vec::new();
    for (position, record) in index.iter().enumerate() {
        let id = RecordId::new(record.id.clone());
        let content = view
            .record(&id)
            .map(|item| item.content.clone())
            .unwrap_or_default();
        // ① 标签：正文的前几个词（CJK bigram + 词，**确定性**）。
        for token in apeireth_memory::memory_view::tokenize(&content)
            .iter()
            .take(3)
        {
            annotations.push(MemoryAnnotation::new(
                AnnotationKind::Tag,
                id.clone(),
                AnnotationPayload::tag(token.clone()),
                stamp.clone(),
            ));
        }
        // ② 分数：正文长度归一（派生数据，**不是真值**）。
        let score = (content.chars().count() as f64 / 10_000.0).clamp(0.0, 1.0);
        annotations.push(MemoryAnnotation::new(
            AnnotationKind::Score,
            id.clone(),
            AnnotationPayload::score(score),
            stamp.clone(),
        ));
        // ③ 关联：与下一条记录关联（**只产标注**）。
        if let Some(next) = index.get(position + 1) {
            annotations.push(MemoryAnnotation::new(
                AnnotationKind::Association,
                id.clone(),
                AnnotationPayload::association(RecordId::new(next.id.clone()), 0.5),
                stamp.clone(),
            ));
        }
    }
    annotations
}

/// **实测**越权（**不许假设**）：跑一遍模式层 ⇒ 比对原始层与检索路径 ⇒ 外加负对照。
///
/// # 返回
///
/// `(判定, 证据行)` —— 证据行是**人话**的实测记录（报告里原样打印 ⇒ 可复核）。
fn measure_overreach_in_real_run(
    index: &[IndexRecord],
    golden: &GoldenSet,
) -> (OverreachVerdict, Vec<String>) {
    let mut notes: Vec<String> = Vec::new();
    let mut violations: Vec<String> = Vec::new();

    // ① 模式层跑之前的原始层摘要与跑批结果。
    let digest_before = corpus_digest(index);
    let report_before = run_batch(index, golden);

    // ② 跑模式层（只产标注）。
    let view = build_view(index);
    let annotations = run_pattern_layer(&view, index);

    // ③ 模式层跑之后的原始层摘要与跑批结果。
    let digest_after = corpus_digest(index);
    let report_after = run_batch(index, golden);

    // ④ 检查 1：原始层必须逐字节不变。
    if digest_before != digest_after {
        violations.push(format!(
            "原始层（D0）被改了：摘要 {digest_before} → {digest_after}"
        ));
    }
    // ⑤ 检查 2：标注不许渗进检索路径（同一份卷子 ⇒ 跑批结果必须一致）。
    if report_before != report_after {
        violations.push("检索路径被改了：模式层跑完后，同一份卷子的跑批结果与之前不同".to_string());
    }
    // ⑥ 负对照：探测器必须**看得见**变化（否则上面两条检查是空转）。
    let mut mutated = index.to_vec();
    if let Some(first) = mutated.first_mut() {
        first.text.push_str("（负对照：故意改一个字）");
    }
    let mutated_digest = corpus_digest(&mutated);
    let control_ok = mutated_digest != digest_before;
    if !control_ok {
        violations.push(
            "负对照失败：故意改了原始层，摘要却没变 ⇒ 探测器失灵（本次越权判定不可信）".to_string(),
        );
    }

    notes.push(format!(
        "模式层产出标注 {} 条（每条都是一次「本可回写」的机会）⇒ attempts = {}",
        annotations.len(),
        annotations.len()
    ));
    notes.push(format!(
        "原始层摘要：模式层跑前 = {digest_before} · 跑后 = {digest_after} ⇒ {}",
        if digest_before == digest_after {
            "逐字节未变 ✓"
        } else {
            "**变了** ✗"
        }
    ));
    notes.push(format!(
        "检索路径：模式层跑前后的跑批结果{}",
        if report_before == report_after {
            "逐字段相同 ✓（标注没渗进检索）"
        } else {
            "**不同** ✗"
        }
    ));
    notes.push(format!(
        "负对照：故意改一条记录 ⇒ 摘要 {digest_before} → {mutated_digest} ⇒ 探测器{}",
        if control_ok {
            "看得见变化 ✓"
        } else {
            "**失灵** ✗"
        }
    ));
    notes.push(
        "类型层面：模式层拿到的是 `&dyn MemoryView`（只读）⇒ 没有回写口；\
         本次实测的是「绕过契约去改 D0」是否发生"
            .to_string(),
    );

    (measure_overreach(annotations.len(), violations), notes)
}
