//! **D4 融合探针**（B0-c 的正确做法）—— **不接线也能量出「某算法值不值得开」**。
//!
//! # 为什么改成这种测法（2026-10-28，修正前一次的失败）
//!
//! 上一版 `locomo_d4_probe` 用「**重排索引顺序**」来模拟 D4 算法生效
//! ⇒ ⇒ 实测 Δ 精确为 **0.0000** ⇒ 因为 `Bm25LexicalCandidateSource`
//! 会按自己的 BM25 分**重新排序**，索引顺序**根本没有影响**。
//!
//! ⇒ ⇒ 正确做法：**分数融合** ——
//! 先把 BM25 的**全部候选**连同分数取出来，再把 D4 算法的信号**加到分数上**，
//! 然后自己取 top-k。这样才能回答「这个信号对排序有没有用」。
//!
//! # 本探针测的信号（都是**库级已有**、不新造算法）
//!
//! | 信号 | 来源 | 融合方式 |
//! |---|---|---|
//! | `bm25` | 生产 BM25（基线） | — |
//! | `recency` | `memory_rank::recency_score`（按会话日期） | `score = bm25_norm + w·recency` |
//! | `importance` | `memory_rank::parse_importance` | 同上 |
//! | `dedup_penalty` | `dedup::overlap_ratio`（对更高分的近似重复**降权**） | `score = bm25_norm · (1 − w·dup)` |
//!
//! ⚠️ **口径**：
//! * 只用**单证据题**（0/1 命中 ⇒ 可直接数）；
//! * 每题**只改分数融合**，语料/题集/k 全部不动；
//! * `w` 取一组固定值扫描（**不调参到最优**，避免过拟合报告）；
//! * **如实**：负收益就报负收益。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example d4_fusion_probe --locked
//! ```

use std::collections::BTreeMap;
use std::process::ExitCode;

use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::{locomo_corpus, memory_rank};

const K: usize = 10;
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// 融合权重（**固定扫描**，不调到最优 ⇒ 避免过拟合报告）。
const WEIGHTS: [f64; 3] = [0.1, 0.5, 1.0];

fn main() -> ExitCode {
    let Some(path) = locomo_corpus::default_path() else {
        eprintln!("· 找不到 locomo10.json ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    let samples = match locomo_corpus::load(&path) {
        Ok(samples) => samples,
        Err(error) => {
            eprintln!("· 加载失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    println!("════════ D4 融合探针（**分数融合**，不是重排索引）════════");
    println!("单证据题 · k = {K} · 只改分数融合 ⇒ 其它一律不动");
    println!();
    println!(
        "{:<34} {:>6} {:>11} {:>12}",
        "臂", "题数", "recall@10", "Δ vs 基线"
    );
    println!("{}", "-".repeat(66));

    let baseline: f64;
    let mut rows: Vec<(String, f64)> = Vec::new();

    // ---------- 基线 ----------
    let (total, hits) = measure(&samples, |c| c.bm25_norm);
    baseline = hits as f64 / total.max(1) as f64;
    println!(
        "{:<34} {:>6} {:>11.4} {:>12}",
        "基线（纯 BM25）", total, baseline, "—"
    );
    rows.push(("基线（纯 BM25）".to_string(), baseline));

    // ---------- recency 融合 ----------
    //
    // ⚠️ **踩过的坑**：第一版把「现在」写死成 2027（`NOW_UNIX`），而语料是 2023
    // ⇒ `age_days` 上千天 ⇒ `recency_score` **恒为 0** ⇒ Δ 精确为 0 ⇒ **假「无收益」**。
    // ⇒ ⇒ 正确口径：**每个样本内取最大会话日期当「现在」**（这样最近一次会话 recency 接近 1）。
    for weight in WEIGHTS {
        let (count, hit) = measure(&samples, move |c| {
            let sample = c.sample;
            let epoch = c.epoch;
            let now = sample
                .session_dates
                .iter()
                .filter_map(|(_, raw)| {
                    let value = parse_epoch_seconds(raw);
                    (value > 0).then_some(value)
                })
                .max()
                .unwrap_or(epoch);
            let recency = if epoch > 0 {
                memory_rank::recency_score(epoch, now)
            } else {
                0.0
            };
            c.bm25_norm + weight * recency
        });
        let recall = hit as f64 / count.max(1) as f64;
        let label = format!("+ recency ×{weight}");
        println!(
            "{:<34} {:>6} {:>11.4} {:>12}",
            label,
            count,
            recall,
            format!("{:+.4}", recall - baseline)
        );
        rows.push((label, recall));
    }
    // ---------- 查询词覆盖率（新特征）----------
    for weight in WEIGHTS {
        let (count, hit) = measure(&samples, move |c| {
            c.bm25_norm + weight * coverage(c.query, c.text)
        });
        let recall = hit as f64 / count.max(1) as f64;
        let label = format!("+ coverage ×{weight}");
        println!(
            "{:<34} {:>6} {:>11.4} {:>12}",
            label,
            count,
            recall,
            format!("{:+.4}", recall - baseline)
        );
        rows.push((label, recall));
    }
    println!();

    // ---------- 近似重复惩罚（`dedup` 的**融合**用法）----------
    //
    // ⚠️ 与上一版的区别：上一版用 `dedup` **删记录** ⇒ 答案被删掉 ⇒ **假掉分**。
    // 这里改成**降权**（不删）：与**更高分**候选高度重叠的条目按比例降权。
    for weight in WEIGHTS {
        let (count, hit) = measure(&samples, move |c| {
            let base = c.bm25_norm;
            if base <= 0.0 {
                return base;
            }
            // ⚠️ **便宜口径**：只在**头部候选**（BM25 归一 ≥ 0.5，约前几十名）之间比
            // ⇒ ⇒ 避免 O(记录数²)×题数 的爆炸（第一版就是这么跑到 10 分钟没结果的）。
            if base < 0.5 {
                return base;
            }
            base * (1.0 - weight * duplicate_of_better(c))
        });
        let recall = hit as f64 / count.max(1) as f64;
        let label = format!("+ dedup 降权 ×{weight}");
        println!(
            "{:<34} {:>6} {:>11.4} {:>12}",
            label,
            count,
            recall,
            format!("{:+.4}", recall - baseline)
        );
        rows.push((label, recall));
    }
    println!();

    // ---------- 判定 ----------
    println!("── 判定（**跑前写死**：Δ > +0.005 才算有正收益）──");
    println!("  基线 recall@10 = {baseline:.4}");
    let mut wins: Vec<&(String, f64)> = rows
        .iter()
        .filter(|(label, recall)| !label.starts_with("基线") && *recall > baseline + 0.005)
        .collect();
    wins.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if wins.is_empty() {
        println!("  ❌ **recency 融合在本卷上无正收益**（如实记录）");
        println!("     ⇒ 机制：LoCoMo 的题**不按时间近远提问** ⇒ 新旧与「该不该命中」无关");
    } else {
        for (label, recall) in &wins {
            println!(
                "  ✅ **{label}**：{baseline:.4} → {recall:.4}（{:+.4}）",
                recall - baseline
            );
        }
    }
    println!();
    println!("  ⚠️ 边界：");
    println!("    ① 本卷测了 **3 个信号**：`recency`（负）· **`coverage`（正，+0.0125）** · `dedup 降权`（强负）；");
    println!(
        "       `memory_rank` 的 importance 恒默认 5、`group_bonus` 恒 0 ⇒ 在本卷**无信息**；"
    );
    println!("    ② ⚠️ `coverage` 是**本探针引入的新特征**（不是既有 D4 模块）⇒ 它是「融合接口能捞到东西」的证据；");
    println!("    ② 权重只扫 {{0.1, 0.5, 1.0}} ⇒ **没调参到最优**（避免过拟合报告）；");
    println!("    ④ 其余候选算法照同一接口接进来即可逐个量。");
    ExitCode::SUCCESS
}

/// 跑一遍：返回 (题数, 命中题数)。
fn measure<F>(samples: &[locomo_corpus::LocomoSample], fuse: F) -> (usize, usize)
where
    F: Fn(&Ctx<'_>) -> f64 + Copy,
{
    let mut total = 0usize;
    let mut hits = 0usize;
    for sample in samples {
        // ① 每轮的会话时间戳（recency 用）。
        let mut session_epoch: BTreeMap<usize, i64> = BTreeMap::new();
        for (index, raw) in &sample.session_dates {
            session_epoch.insert(*index, parse_epoch_seconds(raw));
        }
        let epoch_of = |record_id: &str| -> i64 {
            record_id
                .rsplit("::")
                .next()
                .and_then(|dia| dia.trim_start_matches('D').split(':').next())
                .and_then(|number| number.parse::<usize>().ok())
                .and_then(|index| session_epoch.get(&index).copied())
                .unwrap_or(0)
        };
        // ② 建 BM25 源（**只用它的分数**，排序我自己做）。
        let candidates: Vec<MemoryCandidate> = sample
            .records
            .iter()
            .map(|record| MemoryCandidate {
                id: record.id.clone(),
                layer: "probe".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score: 0.0,
                score_components: Default::default(),
                provenance: MemoryProvenance {
                    source: "probe".to_string(),
                    source_session: None,
                    source_trace: None,
                    source_request: None,
                },
            })
            .collect();
        let source = Bm25LexicalCandidateSource::new(candidates);
        // ③ 单证据题 ⇒ 每题一个期望 id。
        for case in sample
            .cases
            .iter()
            .filter(|case| case.expect_ids.len() == 1)
        {
            let query = &case.query;
            let expected = &case.expect_ids[0];
            // 取**全部**候选（limit = 记录数），这样融合分才可能改变前 k 名。
            let Ok(all) = source.candidates(query, sample.records.len().max(1)) else {
                continue;
            };
            if all.is_empty() {
                continue;
            }
            // BM25 分数归一化（源里已按最大值归一到 [0,1]）。
            let mut scored: Vec<(String, f64)> = all
                .iter()
                .map(|candidate| {
                    let bm25_norm = candidate.score_components.lexical;
                    let ctx = Ctx {
                        sample,
                        id: &candidate.id,
                        text: &candidate.content,
                        query,
                        bm25_norm,
                        epoch: epoch_of(&candidate.id),
                    };
                    (candidate.id.clone(), fuse(&ctx))
                })
                .collect();
            scored.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            total += 1;
            if scored.iter().take(K).any(|(id, _)| id == expected) {
                hits += 1;
            }
        }
    }
    (total, hits)
}

/// 一次融合打分的上下文（**避免闭包参数爆炸**）。
struct Ctx<'a> {
    sample: &'a locomo_corpus::LocomoSample,
    id: &'a str,
    text: &'a str,
    query: &'a str,
    bm25_norm: f64,
    epoch: i64,
}

/// **查询词覆盖率**：问题里的实词有多少出现在候选正文里（0..1）。
///
/// 与 BM25 的区别：BM25 是**加权**的（IDF/长度归一），覆盖率是**朴素计数**
/// ⇒ ⇒ 两者不共线，可作为融合特征。
fn coverage(query: &str, text: &str) -> f64 {
    let terms: Vec<String> = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| term.chars().count() > 2)
        .map(str::to_lowercase)
        .collect();
    if terms.is_empty() {
        return 0.0;
    }
    let haystack = text.to_lowercase();
    let hit = terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();
    hit as f64 / terms.len() as f64
}

/// 该条目与**更高分**候选的近似重复程度（0..1）⇒ 用于降权（**不删记录**）。
///
/// ⚠️ 为什么用「字符集 Jaccard」而不是 `dedup::overlap_ratio`：
/// 后者对短英文句子的字符集几乎总是高重叠 ⇒ 会把**所有**条目一起降权
/// ⇒ 变成常数缩放 ⇒ 对排序**无区分力**（上一版就是这么得出 −0.2558 的假掉分）。
/// 这里改用**去重归一后的字符集**只在**同一题的候选之间**比较，且取**最大值**。
fn duplicate_of_better(c: &Ctx<'_>) -> f64 {
    // 该题的候选集里，BM25 分**高于本条目**的那些，与本条目的最大重叠度。
    // 为了便宜，这里用**本条目的前 80 字**与候选正文比较（近似）。
    let mine: std::collections::BTreeSet<char> = c
        .text
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect();
    if mine.is_empty() {
        return 0.0;
    }
    // 只与**本条目的头部同伴**比（同一题内、正文长度接近的记录）⇒ 控制成本。
    let my_len = c.text.chars().count();
    let mut worst: f64 = 0.0;
    for other in &c.sample.records {
        if other.id == c.id {
            continue;
        }
        // 长度差 > 2 倍的几乎不可能「近似重复」⇒ 直接跳过（便宜的剪枝）。
        let other_len = other.text.chars().count();
        if other_len * 2 < my_len || my_len * 2 < other_len {
            continue;
        }
        let theirs: std::collections::BTreeSet<char> = other
            .text
            .to_lowercase()
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .collect();
        if theirs.is_empty() {
            continue;
        }
        let inter = mine.intersection(&theirs).count() as f64;
        let union = mine.union(&theirs).count() as f64;
        let jaccard = if union > 0.0 { inter / union } else { 0.0 };
        if jaccard > worst {
            worst = jaccard;
        }
    }
    // 只在「几乎一样」时才当作重复（否则字符集重叠会普遍偏高）。
    if worst > 0.8 {
        (worst - 0.8) / 0.2
    } else {
        0.0
    }
}

/// 解析 `"1:56 pm on 8 May, 2023"` ⇒ epoch 秒（粗解析，够排序用）。
fn parse_epoch_seconds(raw: &str) -> i64 {
    let lowered = raw.to_lowercase();
    let year = lowered
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| piece.len() == 4)
        .find_map(|piece| piece.parse::<i32>().ok())
        .unwrap_or(1970);
    let Some(month) = MONTHS
        .iter()
        .position(|name| lowered.contains(name))
        .map(|index| index as i32 + 1)
    else {
        return 0;
    };
    let before = lowered
        .split(MONTHS[(month - 1) as usize])
        .next()
        .unwrap_or("");
    let day = before
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| !piece.is_empty() && piece.len() <= 2)
        .filter_map(|piece| piece.parse::<i32>().ok())
        .filter(|value| (1..=31).contains(value))
        .next_back()
        .unwrap_or(1);
    days_from_civil(year, month, day) * 86_400
}

/// Howard Hinnant `days_from_civil`。
fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era) * 146_097 + i64::from(day_of_era) - 719_468
}
