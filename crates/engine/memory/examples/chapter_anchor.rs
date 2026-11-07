//! **章节锚点**（时间锚的一种）到底值多少？—— 一个**非 oracle** 的对照实验。
//!
//! # 这个例子回答什么
//!
//! 上一批（W-35）实测：**5 种非答案侧信号来源全部不提升 recall**
//! ⇒ 其中「**时间锚**」是**唯一「没被真正测到」**的一个（0/476、0/248 出词）。
//!
//! ⚠️ **为什么没测到**（本例子的起点）：
//! 真实题面只有两种模板（`关于{主体}，我们当时记了什么？` / `{主体}这件事最后是怎么定的？`）
//! ⇒ ⇒ **题面里根本没有时间词** ⇒ ⇒ 时间锚**结构性地**无从施加。
//!
//! # 🎯 本实验的关键发现（**先看这个**）
//!
//! 语料的**可检索文本 = 正文**（**刻意不含回目联** ⇒ 见 `novel_corpus::to_mine_records`
//! 的说明：回目联进可检索文本 ⇒ 检索是**字面命中** ⇒ recall 恒等于满分 ⇒ 测不出东西）。
//!
//! ⇒ ⇒ **正文里根本没有「第12回」这种字符串**
//! ⇒ ⇒ 所以题面里的「第N回」**匹配不到任何内容** ⇒ **不是「被稀释」，是「完全无效」**。
//!
//! # 三个变体（**唯一变量 = 检索/注入形态**）
//!
//! | 变体 | 检索 | 注入 |
//! |---|---|---|
//! | **A 生产** | 生产 BM25 | 生产（每条 120 字） |
//! | **B 锚定检索** | **用题面「第N回」定位到那一回** | 同生产（每条 120 字） |
//! | **C 锚定 + 整回注入** | 同 B | **整回正文**（不截断） |
//!
//! ⚠️ **B 与 C 都**不是 oracle**：
//!
//! * 章节号**来自用户问句**（「第12回裡…」）⇒ **不需要知道答案**；
//! * 定位到「那一回」**不告诉你答案在哪一段** ⇒ ⇒ **C 注入整回**时
//!   **仍然不知道答案在 7401 字的第几段** ⇒ ⇒ **C 是诚实的天花板**，不是作弊。
//!
//! # 三条纪律
//!
//! 1. **题集固定**：同一批题在三个变体下**一字不改**（只改检索/注入形态）；
//! 2. **同 k**（10）、同语料、同挖题配置；
//! 3. **如实**：报**送达率**（`groundedness ÷ recall`）⇒ 因为 L2 的瓶颈比 L1 更严重
//!    ⇒ ⇒ **只看 recall 会漏掉真正的瓶颈**。
//!
//! # ⚠️ 口径边界
//!
//! * **B/C 是「模拟」**（假想的产品形态）⇒ **不是生产实现**；
//! * 章节锚定**只在「用户明说第几回」时可用** ⇒ 覆盖率低（真实提问未必带回目）；
//! * 样本 = 两部古典小说 ⇒ **机制可迁移、数字不可迁移**。

use apeireth_memory::delivery_metrics::measure_delivery;
use apeireth_memory::golden_set::{GoldenCase, GoldenSet, Metrics};
use apeireth_memory::hybrid_search::{Bm25Config, Bm25Index};
use apeireth_memory::novel_corpus::{self, Chapter};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const K: usize = 10;
/// 生产注入每条证据的字符数（**逐字对齐生产常量**）。
const PRODUCTION_CHARS: usize = apeireth_memory::memory_injection::EVIDENCE_MAX_CHARS;
/// 变体 E：注入「含主体的那一段」的**字符上限**。
const SUBJECT_SPAN_CHARS: usize = 400;

fn main() -> ExitCode {
    let paths = novels();
    if paths.is_empty() {
        eprintln!("· 语料不在（$APEIRETH_DATA_DIR/novels/ 或 ~/.apeireth/novels/）⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    for (name, path) in paths {
        if let Err(error) = report_one(&name, &path) {
            eprintln!("· {name}：{error}");
        }
    }
    ExitCode::SUCCESS
}

fn novels() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        roots.push(PathBuf::from(dir).join("novels"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join(".apeireth/novels"));
    }
    for root in roots {
        out.push(("redchamber".to_string(), root.join("redchamber.txt")));
        out.push(("watershed".to_string(), root.join("watershed.txt")));
    }
    out.retain(|(_, path)| path.exists());
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|(_, path)| {
        if seen.contains(path) {
            false
        } else {
            seen.push(path.clone());
            true
        }
    });
    out
}

fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    if chapters.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }

    // ---------- 题集：**带章节锚点**的问句（**用户明说第几回**）----------
    let mut cases: Vec<GoldenCase> = Vec::new();
    for (position, chapter) in chapters.iter().enumerate() {
        cases.push(GoldenCase {
            id: format!("anchor-{position:04}"),
            // ⚠️ **题面只带章节锚点，不带主体** ⇒ 这是**用户给的**信息 ⇒ **不是 oracle**。
            //
            // # 为什么**不带**主体（2026-10-27 实测踩到）
            //
            // 早先版本带主体 ⇒ 主体取自「正文最高频 bigram」⇒ 得到的是
            // 「一事」「一一」「____」这种**根本不是词**的碎片
            // ⇒ ⇒ 题面**没有语义** ⇒ ⇒ 分数**测不出任何东西**。
            //
            // ⇒ 现在：**主体完全由章节锚点承担** ⇒ ⇒ 隔离出「**锚点 vs 主体**」的对比。
            //
            // ⚠️ 回数用**阿拉伯数字**（解析器按它认；也更贴近现代用法）。
            query: format!("第{}回裡記了什麼？", chapter.index),
            expect_ids: vec![format!("novel-{:04}", chapter.index)],
            expect_fragments: vec![fragment_of(&chapter.body)],
            tags: vec!["anchor".to_string()],
        });
    }
    if cases.is_empty() {
        return Err("没造出题".into());
    }
    let golden = GoldenSet::new(format!("anchor-{name}"), cases.clone());

    // ---------- 追加：**真实挖题 + 章节锚点**（**最贴近产品**的对照）----------
    //
    // # 为什么要有这个变体（2026-10-27 实测踩到）
    //
    // 上面的 A/B 用的是「**只有锚点**」的题面 ⇒ ⇒ 它测的是「锚点 vs 无信号」
    // ⇒ ⇒ **但真实提问既有主体、也可能带锚点**。
    // ⇒ ⇒ 这里改用**生产的挖题**（题面 = 「关于X，我们当时记了什么？」）
    // ⇒ ⇒ **只在题面末尾追加锚点**（原问句逐字保留）
    // ⇒ ⇒ 于是 D vs A′ 就是「**在真实题上加锚点**能加多少」⇒ **最贴近产品**。
    let mined = apeireth_memory::golden_mining::mine(
        &records_to_mine(&chapters),
        apeireth_memory::golden_mining::MiningConfig {
            max_cases_per_record: 6,
            ..apeireth_memory::golden_mining::MiningConfig::default()
        },
    );
    let mut real_cases: Vec<GoldenCase> = Vec::new();
    let mut real_anchor_cases: Vec<GoldenCase> = Vec::new();
    for case in &mined {
        let Some(target) = case.expect_ids.first() else {
            continue;
        };
        let Some(chapter) = chapter_of_id(target) else {
            continue;
        };
        let base = GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.expect_ids.clone(),
            expect_fragments: case.expect_fragments.clone(),
            tags: vec![case.kind.clone()],
        };
        real_cases.push(base.clone());
        let mut anchored = base.clone();
        anchored.query = format!("{}（第{}回）", base.query, chapter);
        real_anchor_cases.push(anchored);
    }

    // ---------- 索引（**可检索文本 = 正文** ⇒ 与生产逐字一致）----------
    let records: Vec<(String, String)> = novel_corpus::to_mine_records(&chapters)
        .into_iter()
        .map(|record| (record.id, record.content))
        .collect();
    let index: Vec<apeireth_memory::golden_run::IndexRecord> = records
        .iter()
        .map(|(id, text)| apeireth_memory::golden_run::IndexRecord {
            id: id.clone(),
            text: text.clone(),
        })
        .collect();

    println!("════════ {name} ════════");
    println!(
        "{} 回 · 题 {} 道（**题面带「第N回」锚点**）· k={K}",
        chapters.len(),
        cases.len()
    );
    println!();
    println!(
        "{:<26} {:>10} {:>12} {:>12} {:>10}",
        "变体", "recall@10", "grounded", "送达率", "注入字数"
    );
    println!("{}", "-".repeat(76));

    // ---------- 变体 A：生产 ----------
    let production = run_production(&index, &golden);
    print_row("A 生产（BM25 + 120 字）", &production, PRODUCTION_CHARS);

    // ---------- 变体 B/C：章节锚定 ----------
    let anchored = run_anchored(&records, &golden, false);
    print_row("B 锚定检索（+ 120 字）", &anchored, PRODUCTION_CHARS);
    let anchored_full = run_anchored(&records, &golden, true);
    print_row("C 锚定检索（+ 整回正文）", &anchored_full, 0);

    // ---------- 变体 H：**只**放大单条注入预算（**不定位、不换内容**） ----------
    //
    // # 为什么这个变体重要（**产品侧几乎零成本**）
    //
    // 生产注入把每条证据**硬截断到 120 字**（`memory_injection::EVIDENCE_MAX_CHARS`）
    // ⇒ ⇒ 但**真正的预算是总量**（`context_compiler` 的 `max_chars`）
    // ⇒ ⇒ **放大单条上限并不会增加总消耗** ⇒ ⇒ **零成本**
    // ⇒ ⇒ 那它到底能救回多少送达？⇒ **实测**。
    for budget in [PRODUCTION_CHARS, 400usize, 1200] {
        let h = run_with_budget(&index, &golden, budget);
        print_row(&format!("H 单条预算 {budget} 字"), &h, budget);
    }

    println!();
    println!("── 读数 ──");
    println!(
        "  A→B（**只改检索**）：recall {:.4} → {:.4}（Δ {:+.4}）· 送达率 {:.4} → {:.4}",
        production.recall_at_k,
        anchored.recall_at_k,
        anchored.recall_at_k - production.recall_at_k,
        production.groundedness,
        anchored.groundedness
    );
    println!(
        "  B→C（**只改注入**）：grounded {:.4} → {:.4}（Δ {:+.4}）",
        anchored.groundedness,
        anchored_full.groundedness,
        anchored_full.groundedness - anchored.groundedness
    );
    // ⚠️ **分母为 0 ⇒ 比值无意义** ⇒ **不许**显示成「0.00×」（那会被读成「变差了」）。
    let ratio = if production.groundedness > 0.0 {
        format!(
            "**{:.2}×**",
            anchored_full.groundedness / production.groundedness
        )
    } else {
        "—（基线为 0 ⇒ **比值无意义**）".to_string()
    };
    println!(
        "  A→C（**检索 + 注入都改**）：grounded {:.4} → {:.4}（{ratio}）",
        production.groundedness, anchored_full.groundedness
    );

    // ---------- 变体 A′/D：**真实挖题** + 锚点 ----------
    if !real_cases.is_empty() {
        let real_base = GoldenSet::new(format!("real-{name}"), real_cases.clone());
        let real_anchor = GoldenSet::new(format!("real-anchor-{name}"), real_anchor_cases.clone());
        let a_prime = run_production(&index, &real_base);
        let d = run_anchored(&records, &real_anchor, false);
        println!();
        println!("── **真实挖题**（题面 = 「关于X，我们当时记了什么？」）──");
        println!(
            "  题数 {}（A′ 生产） / {}（D 加锚点）",
            real_cases.len(),
            real_anchor_cases.len()
        );
        print_row("A′ 生产（原题面）", &a_prime, PRODUCTION_CHARS);
        print_row("D 加锚点 + 锚定检索", &d, PRODUCTION_CHARS);
        println!(
            "  ⇒ **在真实题上加锚点**：recall {:.4} → {:.4}（Δ {:+.4}）· grounded {:.4} → {:.4}",
            a_prime.recall_at_k,
            d.recall_at_k,
            d.recall_at_k - a_prime.recall_at_k,
            a_prime.groundedness,
            d.groundedness
        );

        // ---------- 变体 H'：**只**放大单条注入预算（**生产片段口径**） ----------
        //
        // ⇒ 与上面的 H（深处片段口径）对照 ⇒ ⇒ 看「放大预算」在**真实片段**下有没有用。
        for budget in [PRODUCTION_CHARS, 400usize, 1200, 4000] {
            let h = run_with_budget(&index, &real_base, budget);
            print_row(&format!("H' 单条预算 {budget} 字"), &h, budget);
        }

        // ---------- 变体 E：**段内定位**（主体 ⇒ 挑那一段） ----------
        //
        // # 这个变体在问什么
        //
        // 变体 C 证明：**定位到那一回**之后，**整回注入**能让 groundedness = 1.0
        // ⇒ 但代价是 **7401 字/条** ⇒ **不可接受**。
        //
        // ⇒ 那能不能**只注入「那一回里含主体的段落」**？
        // ⇒ ⇒ **主体是用户会给的信息**（题面里就有）⇒ ⇒ **不是 oracle**
        // ⇒ ⇒ 成本：几百字 ⇒ **可接受**
        //
        // # ⚠️ 口径（**构造红利**）
        //
        // 本变体用**生产挖题**里的主体（由 `query_signals::parse_signals` 从期望片段推出）
        // ⇒ ⇒ **主体本身含「构造红利」**（与现有基准**同口径** ⇒ 可比）
        // ⇒ ⇒ 但**真实用户提问里未必有这么干净的主体** ⇒ **绝对值偏乐观**。
        let e = run_anchored_with_subject_span(&records, &real_anchor);
        print_row("E 锚定 + 主体段注入", &e, SUBJECT_SPAN_CHARS);
        println!(
            "  ⇒ **D→E（只改注入：从「开头 120 字」到「含主体的段」）**：grounded {:.4} → {:.4}（Δ {:+.4}）",
            d.groundedness,
            e.groundedness,
            e.groundedness - d.groundedness
        );
        println!(
            "  ⇒ 对照 C（**整回注入**，{avg} 字/条）：grounded 1.0000 ⇒ ⇒ E 用 ~{span} 字达到 {e:.4}（**不是** 1.0 ⇒ ⇒ **段内定位仍有缺口**）",
            avg = average_len(&records),
            span = SUBJECT_SPAN_CHARS,
            e = e.groundedness
        );

        // ---------- 变体 F：**多个**主体段（补 E 的缺口） ----------
        //
        // # 缺口在哪（**实测机制**）
        //
        // E 只取「**第一段**含主体的段」⇒ ⇒ 主体在同一回里**出现多次**时
        // ⇒ ⇒ 挑到的那段**未必是答案段** ⇒ ⇒ groundedness 只到 0.4202。
        //
        // ⇒ ⇒ 补法：**把含主体的段都取出来**（前 N 段）⇒ ⇒ 成本 ×N
        // ⇒ ⇒ 看 groundedness 涨多少、值不值。
        // ---------- 变体 G：窗口**以主体出现处为中心** ----------
        //
        // # 为什么试它（E/F 的缺口在哪）
        //
        // E（段首 400 字）0.4202 · F（前 2/3 段）**几乎不涨** ⇒ ⇒ 缺口**不在「取几段」**
        // ⇒ ⇒ **在窗口位置**：古典小说的段落很长 ⇒ ⇒ 答案句在段的**后半部分** ⇒ ⇒ 段首够不着。
        //
        // # ⚠️ 这个变体可能暴露**基准构造问题**
        //
        // 生产挖题的期望片段 = 「**第一句**含主体的句子」（`golden_mining::fragment_for`）
        // ⇒ ⇒ 「取主体**第一次出现处**附近的窗口」**几乎就是答案**
        // ⇒ ⇒ 若 G 直奔 1.0 ⇒ ⇒ 说明「主体定位」在这个基准上**接近 oracle**
        // ⇒ ⇒ ⇒ **必须如实指出**（否则会把这个数字当成可上线的收益）。
        let g = run_anchored_centered(&records, &real_anchor);
        print_row("G 锚定 + 主体出现处窗口", &g, SUBJECT_SPAN_CHARS);
        println!(
            "  ⇒ E→G（**窗口从段首挪到主体处**）：grounded {e:.4} → {g:.4}（Δ {delta:+.4}）",
            e = e.groundedness,
            g = g.groundedness,
            delta = g.groundedness - e.groundedness
        );

        for count in [2usize, 3] {
            let f = run_anchored_with_subject_spans(&records, &real_anchor, count);
            print_row(
                &format!("F 锚定 + 前 {count} 个主体段"),
                &f,
                SUBJECT_SPAN_CHARS * count,
            );
            println!(
                "  ⇒ E→F（前 {count} 段）：grounded {e:.4} → {f:.4}（Δ {delta:+.4}）· 成本 {span} → {span2} 字/条",
                e = e.groundedness,
                f = f.groundedness,
                delta = f.groundedness - e.groundedness,
                span = SUBJECT_SPAN_CHARS,
                span2 = SUBJECT_SPAN_CHARS * count
            );
        }
    }
    println!();
    println!("  ⚠️ 口径：");
    println!("    · **两组期望片段口径不同** ⇒ **groundedness 跨组不可比**：");
    println!(
        "      A/B/C 用「正文 60% 处的 40 字」· A′/D 用**生产挖题**的片段（正文里含主体的那一句）"
    );
    println!(
        "      ⇒ 两者**方向一致**（都不是开头）⇒ 但**绝对值不可直接比** ⇒ 只比**同一组内**的差；"
    );
    println!("    · B/C 是**模拟**（假想的产品形态）⇒ **不是生产实现**；");
    println!("    · 章节锚定**只在用户明说第几回时可用** ⇒ 覆盖率低；");
    println!("    · C 注入**整回** ⇒ 仍**不知道**答案在 7401 字的第几段 ⇒ **不是 oracle**；");
    println!(
        "    · 变体 C 的注入字数 = 整回正文（平均 {} 字/条）⇒ **上下文成本远高于生产**",
        average_len(&records)
    );
    Ok(())
}

fn print_row(label: &str, metrics: &Metrics, injected_chars: usize) {
    let delivery = measure_delivery(metrics);
    println!(
        "{:<26} {:>10.4} {:>12.4} {:>12.4} {:>10}",
        label,
        metrics.recall_at_k,
        metrics.groundedness,
        delivery.delivery_rate.unwrap_or(0.0),
        injected_chars
    );
}

fn run_production(
    index: &[apeireth_memory::golden_run::IndexRecord],
    golden: &GoldenSet,
) -> Metrics {
    apeireth_memory::golden_run::run(
        index,
        golden,
        K,
        apeireth_memory::golden_run::InjectionMode::Production,
    )
    .map(|report| report.metrics)
    .unwrap_or_default()
}

/// 章节锚定检索（**模拟**）。
///
/// # 口径
///
/// * 题面里的「第N回」⇒ 定位到**那一回**（记录 id = `novel-{index:04}`）；
/// * 定位到的那一条**排在最前**，其余按生产 BM25 排序接在后面（⇒ k 不变）；
/// * `full_injection = true` ⇒ 注入**整回正文**；否则注入**前 120 字**（与生产逐字一致）。
fn run_anchored(records: &[(String, String)], golden: &GoldenSet, full_injection: bool) -> Metrics {
    let mut bm25 = Bm25Index::new(Bm25Config::default());
    for (id, text) in records {
        bm25.insert(id, text);
    }
    let mut hits = 0usize;
    let mut grounded = 0usize;
    let mut scored = 0usize;
    for case in &golden.cases {
        let Some(chapter) = chapter_of_query(&case.query) else {
            continue;
        };
        let target_id = format!("novel-{chapter:04}");
        // ① 锚定命中（**不查 BM25** ⇒ 这是「定位」的效果）
        let mut ranked: Vec<String> = Vec::with_capacity(K);
        if records.iter().any(|(id, _)| *id == target_id) {
            ranked.push(target_id.clone());
        }
        // ② 其余按生产 BM25 接在后面
        for hit in bm25.search(&case.query, K) {
            if !ranked.contains(&hit.id) {
                ranked.push(hit.id.clone());
            }
        }
        let mut evidence: Vec<String> = Vec::new();
        for id in ranked.iter().take(K) {
            let Some((_, text)) = records.iter().find(|(record_id, _)| record_id == id) else {
                continue;
            };
            if full_injection {
                evidence.push(text.clone());
            } else {
                evidence.push(text.chars().take(PRODUCTION_CHARS).collect());
            }
        }
        scored += 1;
        // recall@k：目标 id 在前 k 里
        if ranked.iter().take(K).any(|id| id == &case.expect_ids[0]) {
            hits += 1;
        }
        // groundedness：注入里含期望片段
        let expected = case.expect_fragments.first().cloned().unwrap_or_default();
        if !expected.is_empty() && evidence.iter().any(|text| text.contains(&expected)) {
            grounded += 1;
        }
    }
    if scored == 0 {
        return Metrics::default();
    }
    Metrics {
        recall_at_k: hits as f64 / scored as f64,
        ndcg_at_k: 0.0,
        evidence_precision: 0.0,
        groundedness: grounded as f64 / scored as f64,
        skipped_cases: 0,
        total_cases: scored,
    }
}

/// 题面里的「第N回」⇒ 回数（**阿拉伯数字**；本实验的题面自己写的）。
fn chapter_of_query(query: &str) -> Option<usize> {
    let chars: Vec<char> = query.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '第' {
            let mut cursor = index + 1;
            while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor > index + 1 && cursor < chars.len() && chars[cursor] == '回' {
                let digits: String = chars[index + 1..cursor].iter().collect();
                return digits.parse().ok();
            }
        }
        index += 1;
    }
    None
}

/// 期望片段（**取正文深处**的一小段）。
///
/// # ⚠️ 为什么**不能**取开头（2026-10-27 实测踩到）
///
/// 取开头 ⇒ 生产注入（每条**前 120 字**）**必然包含它**
/// ⇒ ⇒ `groundedness` **恒等于** `recall` ⇒ ⇒ **送达率恒为 1.0** ⇒ **毫无意义**。
/// ⇒ ⇒ 必须取**正文深处**（生产注入够不着的地方）⇒ 才有判别力。
fn fragment_of(body: &str) -> String {
    let chars: Vec<char> = body.chars().collect();
    if chars.len() < 80 {
        return chars.iter().take(chars.len() / 2).collect();
    }
    // 取**约 60% 处**的一小段（避开开头 120 字 ⇒ 生产注入够不着）。
    let start = chars.len() * 3 / 5;
    let end = (start + 40).min(chars.len());
    chars[start..end].iter().collect()
}

fn average_len(records: &[(String, String)]) -> usize {
    if records.is_empty() {
        return 0;
    }
    let total: usize = records.iter().map(|(_, text)| text.chars().count()).sum();
    total / records.len()
}

/// 记录 id（`novel-XXXX`）⇒ 回数。
fn chapter_of_id(id: &str) -> Option<usize> {
    id.strip_prefix("novel-")?.parse().ok()
}

/// 章节 ⇒ 挖题用的记录（**只取 id 与正文**⇒ 与生产同口径）。
fn records_to_mine(chapters: &[Chapter]) -> Vec<apeireth_memory::golden_mining::MineRecord> {
    novel_corpus::to_mine_records(chapters)
}

/// 变体 E：锚定检索 + **注入「那一回里含主体的段落」**。
///
/// # 口径
///
/// * 检索：同变体 B/D（用题面「第N回」定位）；
/// * 注入：**不再取开头 120 字**，而是取**那一回里含主体的那一段**
///   （`SUBJECT_SPAN_CHARS` 字上限）⇒ ⇒ **成本 ~几百字**（不是 7401 字）；
/// * 主体：取自**题面**（`query_signals::parse_signals`）⇒ ⚠️ **含构造红利**（见上）。
fn run_anchored_with_subject_span(records: &[(String, String)], golden: &GoldenSet) -> Metrics {
    let mut bm25 = Bm25Index::new(Bm25Config::default());
    for (id, text) in records {
        bm25.insert(id, text);
    }
    let mut hits = 0usize;
    let mut grounded = 0usize;
    let mut scored = 0usize;
    for case in &golden.cases {
        let Some(chapter) = chapter_of_query(&case.query) else {
            continue;
        };
        let target_id = format!("novel-{chapter:04}");
        let mut ranked: Vec<String> = Vec::with_capacity(K);
        if records.iter().any(|(id, _)| *id == target_id) {
            ranked.push(target_id.clone());
        }
        for hit in bm25.search(&case.query, K) {
            if !ranked.contains(&hit.id) {
                ranked.push(hit.id.clone());
            }
        }
        let subject = apeireth_memory::query_signals::parse_signals(case).subject;
        let mut evidence: Vec<String> = Vec::new();
        for id in ranked.iter().take(K) {
            let Some((_, text)) = records.iter().find(|(record_id, _)| record_id == id) else {
                continue;
            };
            // ① 锚定命中的那一条 ⇒ 注入「含主体的段」
            if id == &target_id {
                let span = match subject.as_deref() {
                    Some(subject) => paragraph_around(text, subject, SUBJECT_SPAN_CHARS),
                    None => text.chars().take(PRODUCTION_CHARS).collect(),
                };
                evidence.push(span);
            } else {
                evidence.push(text.chars().take(PRODUCTION_CHARS).collect());
            }
        }
        scored += 1;
        if ranked.iter().take(K).any(|id| id == &case.expect_ids[0]) {
            hits += 1;
        }
        let expected = case.expect_fragments.first().cloned().unwrap_or_default();
        if !expected.is_empty() && evidence.iter().any(|text| text.contains(&expected)) {
            grounded += 1;
        }
    }
    if scored == 0 {
        return Metrics::default();
    }
    Metrics {
        recall_at_k: hits as f64 / scored as f64,
        ndcg_at_k: 0.0,
        evidence_precision: 0.0,
        groundedness: grounded as f64 / scored as f64,
        skipped_cases: 0,
        total_cases: scored,
    }
}

/// 取**含 `subject` 的那一段**（不足则补到 `max_chars`）⇒ **段内定位**。
fn paragraph_around(text: &str, subject: &str, max_chars: usize) -> String {
    if subject.is_empty() {
        return text.chars().take(max_chars).collect();
    }
    // ⚠️ **偏移**（不是段长！）⇒ 命中时**不要**覆盖 `start`
    // ⇒ ⚠️ 早先版本误写成 `start = trimmed.chars().count()` ⇒ **注入的是错位置**
    // ⇒ ⇒ groundedness 掉到 0.0126 ⇒ **一个纯 bug 伪装成了结论**。
    let mut start = 0usize;
    for paragraph in text.split('\n') {
        let trimmed = paragraph.trim();
        if !trimmed.is_empty() && trimmed.contains(subject) {
            break;
        }
        start += paragraph.chars().count() + 1;
    }
    let chars: Vec<char> = text.chars().collect();
    let start = start.min(chars.len());
    let end = (start + max_chars).min(chars.len());
    chars[start..end].iter().collect()
}

/// 变体 F：锚定检索 + **注入前 N 个含主体的段**。
///
/// # 口径
///
/// * 同变体 E，但**不是只取第一段** ⇒ ⇒ 取**前 N 段**含主体的段（各最多
///   `SUBJECT_SPAN_CHARS` 字）⇒ ⇒ 成本 ×N；
/// * ⚠️ **段之间按出现顺序**（**确定性** ⇒ 可复算）。
fn run_anchored_with_subject_spans(
    records: &[(String, String)],
    golden: &GoldenSet,
    count: usize,
) -> Metrics {
    let mut bm25 = Bm25Index::new(Bm25Config::default());
    for (id, text) in records {
        bm25.insert(id, text);
    }
    let mut hits = 0usize;
    let mut grounded = 0usize;
    let mut scored = 0usize;
    for case in &golden.cases {
        let Some(chapter) = chapter_of_query(&case.query) else {
            continue;
        };
        let target_id = format!("novel-{chapter:04}");
        let mut ranked: Vec<String> = Vec::with_capacity(K);
        if records.iter().any(|(id, _)| *id == target_id) {
            ranked.push(target_id.clone());
        }
        for hit in bm25.search(&case.query, K) {
            if !ranked.contains(&hit.id) {
                ranked.push(hit.id.clone());
            }
        }
        let subject = apeireth_memory::query_signals::parse_signals(case).subject;
        let mut evidence: Vec<String> = Vec::new();
        for id in ranked.iter().take(K) {
            let Some((_, text)) = records.iter().find(|(record_id, _)| record_id == id) else {
                continue;
            };
            if id == &target_id {
                let spans = match subject.as_deref() {
                    Some(subject) => subject_spans(text, subject, SUBJECT_SPAN_CHARS, count),
                    None => vec![text.chars().take(PRODUCTION_CHARS).collect()],
                };
                for span in spans {
                    evidence.push(span);
                }
            } else {
                evidence.push(text.chars().take(PRODUCTION_CHARS).collect());
            }
        }
        scored += 1;
        if ranked.iter().take(K).any(|id| id == &case.expect_ids[0]) {
            hits += 1;
        }
        let expected = case.expect_fragments.first().cloned().unwrap_or_default();
        if !expected.is_empty() && evidence.iter().any(|text| text.contains(&expected)) {
            grounded += 1;
        }
    }
    if scored == 0 {
        return Metrics::default();
    }
    Metrics {
        recall_at_k: hits as f64 / scored as f64,
        ndcg_at_k: 0.0,
        evidence_precision: 0.0,
        groundedness: grounded as f64 / scored as f64,
        skipped_cases: 0,
        total_cases: scored,
    }
}

/// 正文里**前 `count` 个**含 `subject` 的段（各最多 `max_chars` 字）。
fn subject_spans(text: &str, subject: &str, max_chars: usize, count: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut start = 0usize;
    for paragraph in text.split('\n') {
        let trimmed = paragraph.trim();
        if !trimmed.is_empty() && trimmed.contains(subject) {
            let begin = start.min(chars.len());
            let end = (begin + max_chars).min(chars.len());
            out.push(chars[begin..end].iter().collect());
            if out.len() >= count {
                break;
            }
        }
        start += paragraph.chars().count() + 1;
    }
    out
}

/// 变体 G：锚定检索 + **以主体出现处为中心的窗口**。
///
/// # ⚠️ 口径警告（**这是本变体存在的理由**）
///
/// 生产挖题的期望片段 = 「**第一句**含主体的句子」
/// ⇒ ⇒ 「取主体**第一次出现处**附近的窗口」**几乎就是答案**
/// ⇒ ⇒ 这个变体**很可能接近 oracle** ⇒ ⇒ **它量的是「基准构造」，不是「产品收益」**
/// ⇒ ⇒ **如实标出**，不许拿它的数字去许诺。
fn run_anchored_centered(records: &[(String, String)], golden: &GoldenSet) -> Metrics {
    let mut bm25 = Bm25Index::new(Bm25Config::default());
    for (id, text) in records {
        bm25.insert(id, text);
    }
    let mut hits = 0usize;
    let mut grounded = 0usize;
    let mut scored = 0usize;
    for case in &golden.cases {
        let Some(chapter) = chapter_of_query(&case.query) else {
            continue;
        };
        let target_id = format!("novel-{chapter:04}");
        let mut ranked: Vec<String> = Vec::with_capacity(K);
        if records.iter().any(|(id, _)| *id == target_id) {
            ranked.push(target_id.clone());
        }
        for hit in bm25.search(&case.query, K) {
            if !ranked.contains(&hit.id) {
                ranked.push(hit.id.clone());
            }
        }
        let subject = apeireth_memory::query_signals::parse_signals(case).subject;
        let mut evidence: Vec<String> = Vec::new();
        for id in ranked.iter().take(K) {
            let Some((_, text)) = records.iter().find(|(record_id, _)| record_id == id) else {
                continue;
            };
            if id == &target_id {
                let window = match subject.as_deref() {
                    Some(subject) => window_around_subject(text, subject, SUBJECT_SPAN_CHARS),
                    None => text.chars().take(PRODUCTION_CHARS).collect(),
                };
                evidence.push(window);
            } else {
                evidence.push(text.chars().take(PRODUCTION_CHARS).collect());
            }
        }
        scored += 1;
        if ranked.iter().take(K).any(|id| id == &case.expect_ids[0]) {
            hits += 1;
        }
        let expected = case.expect_fragments.first().cloned().unwrap_or_default();
        if !expected.is_empty() && evidence.iter().any(|text| text.contains(&expected)) {
            grounded += 1;
        }
    }
    if scored == 0 {
        return Metrics::default();
    }
    Metrics {
        recall_at_k: hits as f64 / scored as f64,
        ndcg_at_k: 0.0,
        evidence_precision: 0.0,
        groundedness: grounded as f64 / scored as f64,
        skipped_cases: 0,
        total_cases: scored,
    }
}

/// 以**主体第一次出现处**为中心的窗口（`max_chars` 字）。
fn window_around_subject(text: &str, subject: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let Some(position) = text.find(subject) else {
        return chars.iter().take(max_chars).collect();
    };
    let offset = text[..position].chars().count();
    let start = offset.saturating_sub(max_chars / 2);
    let end = (start + max_chars).min(chars.len());
    chars[start..end].iter().collect()
}

/// 变体 H：**生产检索** + **可配的单条注入预算**（**不定位、不换内容**）。
///
/// # 口径
///
/// * 检索：**生产**（`golden_run::run` 的同一套 BM25）；
/// * 注入：与生产**同一条**（每条**开头 `budget` 字**）⇒ ⇒ **唯一变量 = 预算**。
fn run_with_budget(
    index: &[apeireth_memory::golden_run::IndexRecord],
    golden: &GoldenSet,
    budget: usize,
) -> Metrics {
    let mut hits = 0usize;
    let mut grounded = 0usize;
    let mut scored = 0usize;
    let mut bm25 = Bm25Index::new(Bm25Config::default());
    for record in index {
        bm25.insert(&record.id, &record.text);
    }
    for case in &golden.cases {
        let hits_for_case = bm25.search(&case.query, K);
        let mut evidence: Vec<String> = Vec::new();
        for hit in hits_for_case.iter().take(K) {
            if let Some(record) = index.iter().find(|record| record.id == hit.id) {
                evidence.push(record.text.chars().take(budget).collect());
            }
        }
        scored += 1;
        if hits_for_case
            .iter()
            .take(K)
            .any(|hit| hit.id == case.expect_ids[0])
        {
            hits += 1;
        }
        let expected = case.expect_fragments.first().cloned().unwrap_or_default();
        if !expected.is_empty() && evidence.iter().any(|text| text.contains(&expected)) {
            grounded += 1;
        }
    }
    if scored == 0 {
        return Metrics::default();
    }
    Metrics {
        recall_at_k: hits as f64 / scored as f64,
        ndcg_at_k: 0.0,
        evidence_precision: 0.0,
        groundedness: grounded as f64 / scored as f64,
        skipped_cases: 0,
        total_cases: scored,
    }
}
