//! **瓶颈诊断**：L1 检索的 38% 漏检，是「**排序**」问题还是「**候选集**」问题？
//!
//! # 这个例子为什么重要
//!
//! 真实语料上 `recall@10 = 0.6221` ⇒ ⇒ **38% 的答案检索不到**。
//! 但「检索不到」有两种**完全不同**的病，治法相反：
//!
//! | 病 | 表现 | 治法 |
//! |---|---|---|
//! | **排序**（rerank） | 答案在候选集里，但没进 top-10 | 加大 k / 重排 / 加信号 |
//! | **候选**（recall 天花板） | 答案**压根不在**候选集 | 换索引 / 加语义 / 改切分 |
//!
//! ⇒ ⇒ 判据：**扫 k**。若 `recall@100` 也很低 ⇒ ⇒ 是**候选**问题（排序怎么调都没用）。
//!
//! # 口径
//!
//! * 只用**单证据题**（1,122 题）⇒ ⇒ 每题只有 1 条答案 ⇒ 「命中」是 0/1，可直接数；
//! * **逐类别**报（LoCoMo 的 `category`：1 多跳 / 2 时间 / 3 开放 / 4 单跳）；
//! * 用 `RunReport.per_case_hit`（逐题命中）⇒ ⇒ **一次跑就能按类别拆**；
//! * **如实**：任何一类 0 题 ⇒ 打印「无题」，不假装有数。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example locomo_bottleneck --locked
//! ```

use apeireth_memory::golden_run::{run, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::locomo_corpus::{self, LocomoSample};
use std::collections::BTreeMap;
use std::process::ExitCode;

/// 扫的 k 值（**从 1 到 100** ⇒ 看曲线形状）。
const KS: [usize; 7] = [1, 3, 5, 10, 20, 50, 100];

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

    // ① 只取单证据题（0/1 ⇒ 可直接数）。
    let single: Vec<(usize, Vec<GoldenCase>)> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let cases: Vec<GoldenCase> = sample
                .cases
                .iter()
                .filter(|case| case.expect_ids.len() == 1)
                .cloned()
                .collect();
            (index, cases)
        })
        .filter(|(_, cases)| !cases.is_empty())
        .collect();
    let total: usize = single.iter().map(|(_, cases)| cases.len()).sum();
    if total == 0 {
        eprintln!("· 单证据题一题都没有 ⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    println!("════════ 瓶颈诊断：排序 vs 候选集 ════════");
    println!("单证据题 **{}** 题 · 扫 k = {:?}", total, KS);
    println!();

    // ② 对每个 k 跑一遍 ⇒ 拿逐题命中。
    //    `hits[k]` = 该 k 下命中的题 id 集合。
    let mut hits: BTreeMap<usize, std::collections::BTreeSet<String>> = BTreeMap::new();
    for &k in &KS {
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (index, cases) in &single {
            let sample = &samples[*index];
            let golden = GoldenSet::new(sample.sample_id.clone(), cases.clone());
            let Ok(report) = run(&sample.records, &golden, k, InjectionMode::Production) else {
                continue;
            };
            for (case_id, hit) in &report.per_case_hit {
                if *hit {
                    set.insert(case_id.clone());
                }
            }
        }
        hits.insert(k, set);
    }

    // ③ 总曲线。
    println!("── 总曲线（单证据题）──");
    println!("{:>8} {:>10} {:>10}", "k", "命中题数", "recall@k");
    println!("{}", "-".repeat(32));
    for &k in &KS {
        let hit = hits.get(&k).map(|set| set.len()).unwrap_or(0);
        println!("{:>8} {:>10} {:>10.4}", k, hit, hit as f64 / total as f64);
    }
    println!();

    // ④ 按类别拆（case id 里带 `::catN`）。
    println!("── 按类别拆（LoCoMo：1 多跳 · 2 时间 · 3 开放 · 4 单跳）──");
    let mut category_case_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (index, cases) in &single {
        let sample = &samples[*index];
        for case in cases {
            let _ = sample;
            for tag in &case.tags {
                if tag.starts_with("cat:") {
                    category_case_ids
                        .entry(tag.clone())
                        .or_default()
                        .push(case.id.clone());
                }
            }
        }
    }
    print!("{:<10} {:>7}", "类别", "题数");
    for &k in &KS {
        print!(" {:>8}", format!("k={k}"));
    }
    println!();
    println!("{}", "-".repeat(10 + 7 + 9 * KS.len()));
    for (label, ids) in &category_case_ids {
        if ids.is_empty() {
            continue;
        }
        print!("{:<10} {:>7}", label, ids.len());
        for &k in &KS {
            let hit = hits
                .get(&k)
                .map(|set| ids.iter().filter(|id| set.contains(*id)).count())
                .unwrap_or(0);
            print!(" {:>8.4}", hit as f64 / ids.len() as f64);
        }
        println!();
    }
    println!();

    // ⑤ 判据（**跑前写死的规则**）。
    let r10 = hits.get(&10).map(|s| s.len()).unwrap_or(0) as f64 / total as f64;
    let r50 = hits.get(&50).map(|s| s.len()).unwrap_or(0) as f64 / total as f64;
    let r100 = hits.get(&100).map(|s| s.len()).unwrap_or(0) as f64 / total as f64;
    println!("── 判据 ──");
    println!("  recall@10 = {r10:.4} · recall@50 = {r50:.4} · recall@100 = {r100:.4}");
    let headroom_k = r100 - r10;
    let ceiling = 1.0 - r100;
    println!(
        "  ⇒ 「加大 k」能捞回的余量 = **{:+.4}**（{:.1}% 的题）",
        headroom_k,
        headroom_k * 100.0
    );
    println!(
        "  ⇒ 「候选集里根本没有」的占比 = **{:.4}**（{:.1}% 的题）⇒ 加大 k **救不了**",
        ceiling,
        ceiling * 100.0
    );
    println!();
    if ceiling > headroom_k {
        println!("  🎯 **结论：这是「候选集」问题，不是排序问题**");
        println!(
            "     ⇒ 加大 k / 重排 / 加信号 都**只能捞回 {:.1}%**",
            headroom_k * 100.0
        );
        println!("     ⇒ 要动的是**索引与语义**（换检索器 / 加语义召回 / 改切分）");
    } else {
        println!("  🎯 **结论：这是「排序」问题，不是候选集问题**");
        println!("     ⇒ 答案**就在候选集里**，只是没排进前 10");
        println!("     ⇒ 要动的是**排序**（加信号 / 重排 / 加大 k）");
    }
    ExitCode::SUCCESS
}
