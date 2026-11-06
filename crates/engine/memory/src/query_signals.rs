//! `apeireth-memory::query_signals` — 题面**信号增强**的量化（查询判别力的体检）。
//!
//! # 为什么
//!
//! 上一批（W-20R）用严格 A/B 测出：段落化**反而**让 recall@10 从 0.2983 掉到 0.0693，
//! 机制是「**TF 本身就是判别信号**」⇒ **真瓶颈 = 查询的判别力**（题面只有主体词一个信号，
//! 而主体在 73% 的记录里都出现）。
//!
//! ⇒ 本模块**量化**「题面多带一个信号能多判别多少」，让「加什么信号」变成**有数字的决定**。
//!
//! # ⚠️ 实测结论（2026-10-27，真实语料 + 生产 BM25，k=10）
//!
//! **「题面多带一个判别信号」⇒ recall@10 真涨**（四个方案**都涨**，但涨多少不一样）：
//!
//! | 方案 | 《紅樓夢》(476 题) | 《水滸傳》(248 题) | 选词口径 |
//! |---|---|---|---|
//! | 基线（原题面） | 0.2983 | 0.6875 | —— |
//! | 仅主体（**自检行**） | 0.2983 | 0.6875 | 不追加任何词 ⇒ 必须与基线**逐字同分** |
//! | 主体+谓词 | 0.5756 (+0.2773) | 0.7802 (+0.0927) | 主体**之后**的第一个实词（位置） |
//! | 主体+上下文锚 | **0.6113 (+0.3130)** | 0.7903 (+0.1028) | 片段里**出现次数最多**的实词（词频） |
//! | 主体+第二主体 | 0.4412 (+0.1429) | **0.8065 (+0.1190)** | **别的题也拿它当主体**的词（题集身份） |
//!
//! ⇒ **「加什么信号」是**有差别**的决定**（不是「加不加」的问题）；
//! ⇒ ⚠️ **这是上界**：加进去的信号**来自答案片段** ⇒ 带答案侧信息 ⇒ 不是端到端结论；
//! ⇒ ⚠️ **判别力 ≠ 效果**：水滸傳里「两主体」判别力**最低**（0.8440）却 recall **最高**（0.8065）
//!   ⇒ 判别力只是**信息量**，**排序能不能用上**才是效果。
//!
//! # 两个要分清的东西（**别混**）
//!
//! | | 是什么 | 本模块怎么测 |
//! |---|---|---|
//! | **判别力**（distinctiveness） | **信息量**：加了信号 ⇒ 候选集缩到多小 | 数语料里有多少条记录提到该信号 |
//! | **效果**（recall@k） | **排序能不能用上**：候选缩小了，目标能排进 top-k 吗 | **真的跑检索**（`golden_run::run`） |
//!
//! ⇒ 判别力**不等于** recall 的提升量 ⇒ 报告里**必须**说清这个区别。
//!
//! # 口径（钉死，别自创）
//!
//! * **分词**：CJK bigram + ASCII 词（≥2 字符），与 [`crate::golden_mining::tokenize`]
//!   **同口径**（本模块**自己实现**一份，见 [`tokenize`]；测试里有逐字相等的断言）。
//! * **实词**（content word）= 分词后**再去掉**：停用词 / 泛称 / 虚词碎片 / 数量短语
//!   / 跨词边界碎片。
//!   ⚠️ 比 `golden_mining` 的噪声过滤**更严** —— 因为本模块要的是「**实词**」，
//!   不是「任何 token」（「後來」「怎麼」「一個」这种词当不得信号）。
//! * **主体** = 题面里**最长**的实词（挖题时用的那个）。
//!   ⚠️ 同长度时：**先看它是不是出现在答案片段里**（答案片段是围绕主体裁的 ⇒
//!   这是「这个主体对不对」最硬的证据），再取**题面里最靠前**的。
//!   ⇒ 认不出 ⇒ `None`（**不硬凑**）。
//! * **候选记录**：一条记录算「提到某信号」= 记录正文**含有该词**（子串口径，
//!   与 `chunking::competitor_density` 同口径 ⇒ 与「主体出现在 88/120 条」的旧数字可比）。
//!
//! # ⚠️ 「改写题面」是**对照实验**，不是「改完就能上线」
//!
//! `compare_schemes` 会把题面**改写成该方案的样子**（加信号）⇒
//! ① **题目内容变了** ⇒ 分数**不可与基线直接比**（基线也跑一遍放同一张表，只为同口径）；
//! ② 加进去的信号**来自答案片段** ⇒ 那是**答案侧的信息** ⇒ 跑出来的 recall 是
//! **「信号到位之后能涨多少」的**上界**，**不是**「端到端可用」的结论。
//! ⇒ 报告里**必须**同时说清这两条。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、计算、报告齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（生产 0 调用点）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✔（部分）：已在《紅樓夢》《水滸傳》**真实语料**上跑过
//!   （2026-10-27，见 `examples/query_signal_report.rs`），**未在第二台机器复现**。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::{BTreeMap, BTreeSet};

use crate::golden_run::{IndexRecord, InjectionMode};
use crate::golden_set::{GoldenCase, GoldenSet};

/// 一个**信号**（题面里可用于判别的一项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    /// 信号名（如 `主体` / `谓词` / `对象` / `时间锚` / `第二主体`）。
    pub kind: String,
    /// 信号词。
    pub term: String,
}

/// 一道题的**信号集合**（从题面 + 期望片段里解析出来）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySignals {
    /// 题 id。
    pub case_id: String,
    /// 题面的实词（去停用词）。
    pub terms: Vec<String>,
    /// **主体词**（判别力的主战场）。
    pub subject: Option<String>,
    /// 期望片段里的实词（**不含**主体）⇒ 「**本来就有但没写进题面**」的信号。
    pub answer_terms: Vec<String>,
}

/// 从题面 + 期望片段解析出信号集合。
///
/// # 口径
///
/// * 题面实词 = 分词后去停用词（**照 `golden_mining::tokenize` 的同口径**，自己实现一份）；
/// * **主体** = 题面里**最长**的实词（挖题时用的那个）⇒ 认不出 ⇒ `None`（**不硬凑**）；
/// * `answer_terms` = 期望片段的实词**去掉**主体词 ⇒ **主体之外**还能带什么进题面。
///
/// # `answer_terms` 的排序（确定性）
///
/// 按 **（片段内出现次数降序，长度降序，字典序升序）** ⇒
/// ⇒ 第一个 = 「答案里反复提的那个词」（**上下文锚**）；
/// ⇒ 最长的那个 = 「答案里最像对象/状态名的词」（**谓词/对象**）。
pub fn parse_signals(case: &GoldenCase) -> QuerySignals {
    let terms = content_terms(&case.query);
    let subject = detect_subject(&case.query, &case.expect_fragments, &terms);
    let answer_terms = answer_terms_of(&case.expect_fragments, subject.as_deref());
    QuerySignals {
        case_id: case.id.clone(),
        terms,
        subject,
        answer_terms,
    }
}

/// 一个信号方案的**判别力**。
///
/// ⚠️ **派生说明**：契约里写的是 `Copy`，但 `scheme: String` **不可能**是 `Copy`
/// ⇒ 这里保留**字段类型逐字不变**，只去掉 `Copy` 派生（`Clone` / `PartialEq` 照契约）。
#[derive(Debug, Clone, PartialEq)]
pub struct Discriminability {
    /// 方案名（如 `仅主体` / `主体+谓词` / `主体+第二主体`）。
    pub scheme: String,
    /// 信号数。
    pub signal_count: usize,
    /// **候选记录里含该信号的记录数**（越少越有判别力）。
    pub matching_records: usize,
    /// **候选记录总数**。
    pub total_records: usize,
    /// **判别力** = 1 - 匹配占比（0 = 到处都是 ⇒ 无判别力；1 = 独一份）。
    pub distinctiveness: f64,
}

/// 量化一个方案在**一批题**上的平均判别力。
///
/// # 口径
///
/// * 对每题：把该方案**实际会写进题面**的词列出来；
/// * 对每个词：数**语料里有多少条记录提到它**；
/// * ⇒ **判别力** = 1 - 匹配占比；**一批题取平均**。
///
/// # 「提到它」的合取口径（**重要**）
///
/// 一题有多个信号时 ⇒ 一条记录算「匹配」的条件是**同时含有全部信号**
/// ⇒ `matching_records` = **该方案下的候选集大小**（不是「每个词各自的命中数之和」）。
///
/// # 字段口径
///
/// * `matching_records` = 逐题匹配数的**平均**（四舍五入）；
/// * `signal_count` = 逐题信号数的**平均**（四舍五入）；
/// * `distinctiveness` = `1 - 平均匹配数 / 语料条数`
///   （**分母为 0 ⇒ 记 0.0**，不 panic）。
///
/// # ⚠️ **这是「信息量」不是「效果」**
///
/// ⇒ 它回答「**加这个信号能缩小多少候选集**」
/// ⇒ **不等于** recall 会涨多少（还要看排序能不能用上）⇒ 报告里**必须**说清这个区别。
pub fn measure_discriminability(
    cases: &[GoldenCase],
    records: &[(String, String)],
    scheme: &str,
) -> Discriminability {
    let total_records = records.len();
    if cases.is_empty() {
        return Discriminability {
            scheme: scheme.to_string(),
            signal_count: 0,
            matching_records: 0,
            total_records,
            distinctiveness: 0.0,
        };
    }

    let subjects = batch_subjects(cases);
    let mut matching_sum = 0usize;
    let mut signal_sum = 0usize;
    for case in cases {
        let signals = scheme_signals_with(case, scheme, &subjects);
        signal_sum += signals.len();
        let matched = records
            .iter()
            .filter(|(_, text)| {
                signals
                    .iter()
                    .all(|signal| !signal.term.is_empty() && text.contains(&signal.term))
            })
            .count();
        matching_sum += matched;
    }

    let mean_matching = matching_sum as f64 / cases.len() as f64;
    let distinctiveness = if total_records == 0 {
        0.0
    } else {
        (1.0 - mean_matching / total_records as f64).clamp(0.0, 1.0)
    };

    Discriminability {
        scheme: scheme.to_string(),
        signal_count: (signal_sum as f64 / cases.len() as f64).round() as usize,
        matching_records: mean_matching.round() as usize,
        total_records,
        distinctiveness,
    }
}

/// 方案清单（**四个候选**，照本模块头部的编号）。
///
/// * `subject` —— 只有主体（现状）；
/// * `subject+predicate` —— 主体 + 期望片段里的**谓词/对象**（取一个）；
/// * `subject+answer-term` —— 主体 + 期望片段里的**另一个实词**；
/// * `two-subjects` —— 主体 + **第二主体**（若题面/片段里有）。
pub const SCHEMES: &[&str] = &[
    "subject",
    "subject+predicate",
    "subject+answer-term",
    "two-subjects",
];

/// 基线方案名（**原题面**，不改写）—— 放进同一张表只为**同口径**。
pub const BASELINE_SCHEME: &str = "baseline";

/// 一个方案在一批题上的**效果**（**真的跑检索** ⇒ 不是推算）。
///
/// ⚠️ **派生说明**：同 [`Discriminability`] —— `scheme: String` ⇒ 不能 `Copy`。
#[derive(Debug, Clone, PartialEq)]
pub struct SchemeOutcome {
    pub scheme: String,
    /// 该方案下的 `recall@k`（**真的检索**出来的）。
    pub recall_at_k: f64,
    /// 平均判别力（信息量）。
    pub distinctiveness: f64,
}

/// 逐个方案**真的跑检索**并对照。
///
/// # 口径
///
/// * 对每个方案：把题面**改写成该方案的样子** ⇒ 跑 `golden_run::run` ⇒ 取 `recall@k`；
/// * ⚠️ **改写题面会改变「题目内容」** ⇒ 这是**对照实验**
///   ⇒ 报告里**必须**说明「题面被改写了 ⇒ 分数不可与基线直接比」
///   ⇒ 且**基线**（原题面）也要**跑一遍**放进同一张表（**同口径**）。
///
/// # 返回
///
/// * **第 0 项永远是基线**（`scheme = "baseline"`，题面**逐字未改**）；
/// * 之后是 [`SCHEMES`] 里的四个方案，**顺序固定**；
/// * **空卷 ⇒ 空 `Vec`**（不 panic、不返回占位行）。
///
/// # 改写规则（**保持原题不变，只加信号**）
///
/// 题面追加 `（信号1、信号2）` ⇒ **原问句逐字保留** ⇒ 与基线的差异**只有新增的词**
/// ⇒ 这是「加信号」的**消融实验**（不是「换题」）。
/// 已经在原题面里的信号**不重复追加**（例如主体）⇒
/// 「仅主体」方案与基线**逐字同题** ⇒ 它的分数**应当与基线完全相同**（**实验台自检**）。
pub fn compare_schemes(
    golden: &GoldenSet,
    records: &[(String, String)],
    k: usize,
) -> Vec<SchemeOutcome> {
    if golden.cases.is_empty() {
        return Vec::new();
    }
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|(id, text)| IndexRecord {
            id: id.clone(),
            text: text.clone(),
        })
        .collect();

    let mut outcomes: Vec<SchemeOutcome> = Vec::new();

    // --- 基线（原题面，逐字不改）---
    let baseline_recall = recall_of(&index, golden, k);
    outcomes.push(SchemeOutcome {
        scheme: BASELINE_SCHEME.to_string(),
        recall_at_k: baseline_recall,
        distinctiveness: measure_discriminability(&golden.cases, records, "subject")
            .distinctiveness,
    });

    // --- 四个方案（题面改写 ⇒ 真的跑检索）---
    for scheme in SCHEMES {
        let rewritten = rewrite_cases(golden, scheme);
        let set = GoldenSet::new(format!("{}-{}", golden.version, scheme), rewritten);
        let recall = recall_of(&index, &set, k);
        outcomes.push(SchemeOutcome {
            scheme: (*scheme).to_string(),
            recall_at_k: recall,
            distinctiveness: measure_discriminability(&golden.cases, records, scheme)
                .distinctiveness,
        });
    }
    outcomes
}

/// 人话报告（方案对照表 + **口径提醒**）。
///
/// # 报告里**必须**出现的两条口径提醒
///
/// ① **「题面被改写 ⇒ 分数不可与基线直接比」**
///    （加进去的信号来自答案片段 ⇒ 这是**上界**，不是端到端结论）；
/// ② **「判别力是信息量不是效果」**
///    （判别力只回答「候选集缩小多少」，recall 还得看排序能不能用上）。
pub fn render_signal_report(outcomes: &[SchemeOutcome], baseline_recall: f64) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("══ 题面信号增强 · 方案对照（recall@k + 平均判别力）══".to_string());
    lines.push(format!("基线 recall = {baseline_recall:.4}"));
    lines.push(String::new());
    lines.push(format!(
        "  {:<22} {:>10} {:>12} {:>12}",
        "方案", "recall@k", "平均判别力", "相对基线"
    ));
    for outcome in outcomes {
        lines.push(format!(
            "  {:<22} {:>10.4} {:>12.4} {:>+12.4}",
            outcome.scheme,
            outcome.recall_at_k,
            outcome.distinctiveness,
            outcome.recall_at_k - baseline_recall
        ));
    }
    lines.push(String::new());
    lines.push("⚠️ 口径提醒（读数前必看）：".to_string());
    lines.push(
        "  ① 题面被改写 ⇒ 分数不可与基线直接比：每个方案都往题面里**追加了信号词**\n     ⇒ 原问句变了 ⇒ 这是**对照实验**（基线也跑了一遍，放同一张表只为同口径）；\n     ⇒ 加进去的信号**来自答案片段** ⇒ 跑出来的是「信号到位之后能涨多少」的**上界**，\n     ⇒ **不是**「端到端可用」的结论（真上线还需要「信号从哪来」这条链路）。"
            .to_string(),
    );
    lines.push(
        "  ② 判别力是信息量不是效果：判别力只回答「**加这个信号能缩小多少候选集**」\n     （1 - 命中记录占比）⇒ recall 能不能涨，还要看**排序**能不能用上这个信号。\n     ⇒ 两个数必须**分开读**：判别力涨了而 recall 没涨 ⇒ 是**排序**没跟上，不是信号没用。"
            .to_string(),
    );
    lines.join("\n")
}

/// 一个方案在这道题里**实际会写进题面**的信号。
///
/// # 口径（选词规则是**固定规则**，不按语料挑最稀有的词）
///
/// | 方案 | 第二个信号怎么选 | 语义 |
/// |---|---|---|
/// | `subject` | ——（只有主体） | 现状 |
/// | `subject+predicate` | 期望片段里**紧接主体之后**的**第一个**实词（位置口径） | 「这件事**是什么**」 |
/// | `subject+answer-term` | 期望片段里**出现次数最多**的实词（词频口径） | 上下文锚 |
/// | `two-subjects` | **另一个题的主体词**（题集口径）；题面有「A和B」时取 B | 多主体对照 |
///
/// # 为什么三个增强方案要**挑不同的词**
///
/// 若三个方案都挑同一个词 ⇒ 三个分数**必然相同** ⇒ 对照实验**测不出差别**
/// ⇒ 「哪种信号更值钱」这个问题就答不了 ⇒ 所以三条规则**口径必须不同**
/// （位置 / 词频 / 题集身份）。
///
/// ⚠️ **为什么不按语料频次挑词**：若按「语料里最稀有的词」挑 ⇒
/// 挑词与测判别力**用同一份语料** ⇒ 循环论证（判别力必然虚高）
/// ⇒ 这里用**与语料无关**的固定规则，判别力是**选完之后**再测的。
///
/// # 单题口径 vs 批口径
///
/// 本函数是**单题口径**（不知道别的题）⇒ 「两主体」方案拿不到「另一个题的主体」
/// ⇒ 退回兜底（见 [`scheme_signals_in_batch`] 的说明）。
/// 批口径（[`measure_discriminability`] / [`compare_schemes`] 内部用的）
/// 才有完整的「题集身份」信息。
pub fn scheme_signals(case: &GoldenCase, scheme: &str) -> Vec<Signal> {
    scheme_signals_with(case, scheme, &BTreeSet::new())
}

/// **批口径**的信号选择（比 [`scheme_signals`] 多一张牌：**别的题的主体**）。
///
/// # 为什么要批口径
///
/// 「多主体对照」方案要挑的必须是「**另一个主体**」⇒ 光看一道题认不出来
/// （答案片段里任何一个词都可能是）⇒ 必须看**整批题**：某个词**别的题也拿它当主体**
/// ⇒ 它就是货真价实的另一个主体（人名/地名跨题复用）。
///
/// 批口径与单题口径**只在 `two-subjects` 方案上不同**
/// （其余方案的选择规则与题集无关 ⇒ 两口径结果一致）。
pub fn scheme_signals_in_batch(
    cases: &[GoldenCase],
    case: &GoldenCase,
    scheme: &str,
) -> Vec<Signal> {
    let subjects = batch_subjects(cases);
    scheme_signals_with(case, scheme, &subjects)
}

/// 批口径的信号选择（**多了「别的题的主体」这张牌**）。
///
/// * `batch_subjects` = 这一批题里**所有**主体词（题集口径）。
///   ⇒ 「两主体」方案若在答案片段里找到一个**别的题也拿它当主体**的词
///   ⇒ 那就是货真价实的「**另一个主体**」⇒ 比「随便挑个长词」有意义得多。
///
/// ⚠️ `batch_subjects` 为空（单题口径）⇒ 「两主体」退回兜底：
/// 题面有「A和B」取 B；否则取期望片段里**最长的**实词。
fn scheme_signals_with(
    case: &GoldenCase,
    scheme: &str,
    batch_subjects: &BTreeSet<String>,
) -> Vec<Signal> {
    let parsed = parse_signals(case);
    let mut signals: Vec<Signal> = Vec::new();
    if let Some(subject) = &parsed.subject {
        signals.push(Signal {
            kind: "主体".to_string(),
            term: subject.clone(),
        });
    }
    match scheme {
        "subject" => {}
        "subject+predicate" => {
            if let Some(term) = predicate_term(&case.expect_fragments, &parsed) {
                signals.push(Signal {
                    kind: "谓词".to_string(),
                    term,
                });
            }
        }
        "subject+answer-term" => {
            if let Some(term) = parsed.answer_terms.first() {
                signals.push(Signal {
                    kind: "上下文锚".to_string(),
                    term: term.clone(),
                });
            }
        }
        "two-subjects" => {
            if let Some(term) = second_subject(case, &parsed, batch_subjects) {
                signals.push(Signal {
                    kind: "第二主体".to_string(),
                    term,
                });
            }
        }
        _ => {}
    }
    signals
}

// ============================================================
// 内部：改写与跑批
// ============================================================

/// 跑一次批 ⇒ `recall@k`（跑失败 ⇒ 记 0.0，**不 panic**）。
fn recall_of(index: &[IndexRecord], golden: &GoldenSet, k: usize) -> f64 {
    crate::golden_run::run(index, golden, k, InjectionMode::Production)
        .map(|report| report.metrics.recall_at_k)
        .unwrap_or(0.0)
}

/// 把一批题改写成某个方案的样子（**只加信号，不动原问句**）。
fn rewrite_cases(golden: &GoldenSet, scheme: &str) -> Vec<GoldenCase> {
    let subjects = batch_subjects(&golden.cases);
    golden
        .cases
        .iter()
        .map(|case| {
            let signals = scheme_signals_with(case, scheme, &subjects);
            let extra: Vec<String> = signals
                .iter()
                .filter(|signal| !case.query.contains(&signal.term))
                .map(|signal| signal.term.clone())
                .collect();
            let mut rewritten = case.clone();
            rewritten.query = rewrite_query(&case.query, &extra);
            rewritten
        })
        .collect()
}

/// 题面追加信号词（**原问句逐字保留**）。
fn rewrite_query(query: &str, extra: &[String]) -> String {
    if extra.is_empty() {
        return query.to_string();
    }
    let joined = extra.join("、");
    match query.strip_suffix('？') {
        Some(stem) => format!("{stem}（{joined}）？"),
        None => format!("{query}（{joined}）"),
    }
}

/// **谓词/对象**：期望片段里**紧接主体之后**的**第一个**实词。
///
/// # 为什么用「位置」口径
///
/// 任务书里这一招叫「加谓词/对象：题面带上『这件事**是什么**』」
/// ⇒ 「是什么」在答案里就是**紧跟在主体后面**的那个词 ⇒ 位置口径最贴题意。
///
/// # 口径细节
///
/// * 从**主体在片段里的位置之后**开始取（**不与主体重叠**）
///   ⇒ 「主体尾字 + 下一个字」那个跨词 bigram（如「玉帶」）**不算**
///   ⇒ 否则挑到的是碎片，不是「是什么」；
/// * 主体**不在片段里** ⇒ 兜底取期望片段里**最长的**实词（**如实**降级）。
fn predicate_term(fragments: &[String], parsed: &QuerySignals) -> Option<String> {
    let subject = parsed.subject.as_deref()?;
    for fragment in fragments {
        let Some(position) = fragment.find(subject) else {
            continue;
        };
        let tail = &fragment[position + subject.len()..];
        for term in content_terms(tail) {
            if term != subject {
                return Some(term);
            }
        }
    }
    longest_term(&parsed.answer_terms)
}

/// 第二主体：题面里「A和B」的 B；题面没有 ⇒ **别的题也拿它当主体**的词；
/// 再没有 ⇒ 期望片段里**最长的**实词（兜底）。
fn second_subject(
    case: &GoldenCase,
    parsed: &QuerySignals,
    batch_subjects: &BTreeSet<String>,
) -> Option<String> {
    let subject = parsed.subject.as_deref();
    // 题面里的连接词（「和」是挖题模板用的那个；「與」是繁体写法）。
    for connector in ["和", "與"] {
        if let Some(position) = case.query.find(connector) {
            let right = &case.query[position + connector.len()..];
            for term in content_terms(right) {
                if Some(term.as_str()) != subject {
                    return Some(term);
                }
            }
        }
    }
    // 题集口径：答案片段里那个**别的题也拿它当主体**的词 ⇒ 货真价实的「另一个主体」。
    // （`answer_terms` 已去掉主体 ⇒ 命中的必然是**别的**主体。）
    if !batch_subjects.is_empty() {
        for term in &parsed.answer_terms {
            if batch_subjects.contains(term) {
                return Some(term.clone());
            }
        }
    }
    // 兜底：从答案片段借一个（**如实**标注来源是片段）。
    longest_term(&parsed.answer_terms)
}

/// 一批题的**主体词集合**（题集口径 ⇒ 供「两主体」方案挑「另一个主体」）。
fn batch_subjects(cases: &[GoldenCase]) -> BTreeSet<String> {
    let mut subjects = BTreeSet::new();
    for case in cases {
        if let Some(subject) = parse_signals(case).subject {
            subjects.insert(subject);
        }
    }
    subjects
}

/// 期望片段里的实词（去掉主体）⇒ 按（次数降序，长度降序，字典序升序）排。
fn answer_terms_of(fragments: &[String], subject: Option<&str>) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for fragment in fragments {
        for term in content_terms(fragment) {
            if Some(term.as_str()) == subject {
                continue;
            }
            *counts.entry(term).or_insert(0) += 1;
        }
    }
    let mut items: Vec<(String, usize)> = counts.into_iter().collect();
    items.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.0.chars().count().cmp(&left.0.chars().count()))
            .then_with(|| left.0.cmp(&right.0))
    });
    items.into_iter().map(|(term, _)| term).collect()
}

/// 最长的词（同长 ⇒ 取 `answer_terms` 里**靠前**的那个 ⇒ 片段里次数多的）。
///
/// ⚠️ **不用 `max_by_key`**：它在并列时返回**最后一个** ⇒ 与
/// `answer_terms` 的排序（次数降序）相反 ⇒ 这里手写折叠，保留**第一个**最大值。
fn longest_term(terms: &[String]) -> Option<String> {
    let mut best: Option<&String> = None;
    for term in terms {
        let length = term.chars().count();
        let better = match best {
            Some(current) => length > current.chars().count(),
            None => true,
        };
        if better {
            best = Some(term);
        }
    }
    best.cloned()
}

/// 主体识别：题面里**最长**的实词（同长度时先看答案片段、再看位置）。
///
/// # 为什么先看答案片段
///
/// 挖题时答案片段是**围绕主体裁**的 ⇒ 主体**一定在片段里** ⇒
/// 「候选词里谁出现在片段里」是「这个主体对不对」最硬的证据。
/// （纯按位置挑会踩坑：「关于宝钗，……」的 bigram 里有「于宝」这个跨词碎片，
/// 它的位置比「宝钗」还靠前 ⇒ 纯位置会挑错。）
fn detect_subject(query: &str, fragments: &[String], terms: &[String]) -> Option<String> {
    if terms.is_empty() {
        return None;
    }
    let longest = terms
        .iter()
        .map(|term| term.chars().count())
        .max()
        .unwrap_or(0);
    let candidates: Vec<&String> = terms
        .iter()
        .filter(|term| term.chars().count() == longest)
        .collect();

    let in_fragment: Vec<&String> = candidates
        .iter()
        .copied()
        .filter(|term| {
            fragments
                .iter()
                .any(|fragment| fragment.contains(term.as_str()))
        })
        .collect();
    let pool: Vec<&String> = if in_fragment.is_empty() {
        candidates
    } else {
        in_fragment
    };

    pool.iter()
        .min_by_key(|term| {
            (
                query.find(term.as_str()).unwrap_or(usize::MAX),
                term.as_str(),
            )
        })
        .map(|term| (*term).clone())
}

// ============================================================
// 内部：分词与实词过滤
// ============================================================

/// 中文分词口径：**连续 CJK 字符两两组合（bigram）** + ASCII 词（字母数字下划线，≥2 字符）。
///
/// # 与 `golden_mining::tokenize` 的关系
///
/// **同口径**（本模块自己实现一份，不依赖同事模块的内部细节 ⇒ 将来那边改了也不会漂）。
/// 测试里有「两边逐字相等」的断言钉住这一点。
pub fn tokenize(text: &str) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<String> = Vec::new();
    for token in raw_tokens(text) {
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }
    out
}

/// 原始 token 流（**保留重复**，计数用）。
fn raw_tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if is_cjk(ch) {
            let start = index;
            while index < chars.len() && is_cjk(chars[index]) {
                index += 1;
            }
            let run = &chars[start..index];
            if run.len() >= 2 {
                for window in run.windows(2) {
                    out.push(window.iter().collect());
                }
            }
        } else if is_word_char(ch) {
            let start = index;
            while index < chars.len() && is_word_char(chars[index]) {
                index += 1;
            }
            if index - start >= 2 {
                out.push(chars[start..index].iter().collect());
            }
        } else {
            index += 1;
        }
    }
    out
}

/// 是不是 CJK 汉字（基本区 + 扩展 A + 兼容区 + 扩展 B–F）。
fn is_cjk(ch: char) -> bool {
    let code = ch as u32;
    (0x3400..=0x4DBF).contains(&code)
        || (0x4E00..=0x9FFF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0x20000..=0x2FA1F).contains(&code)
}

/// 是不是 ASCII 词字符（字母、数字、下划线）。
fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// 题面/片段里的**实词**（去重保序）。
fn content_terms(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|token| is_content_word(token))
        .collect()
}

/// 是不是**实词**（能当判别信号的那种词）。
///
/// # 过滤链（为什么每一道都要）
///
/// 1. **太短**（<2 字）⇒ 单字没有判别力（「宝」「玉」到处都是）；
/// 2. **停用词** ⇒ 问句模板的词（「后来」「怎么」「什么」）—— 模板每道题都一样 ⇒ 零判别力；
/// 3. **泛称**（「一個」「众人」「先生」）⇒ 高频且不指事；
/// 4. **虚词碎片**（含虚词字的 bigram，如「们需」「定把」）⇒ 跨词边界的碎片；
/// 5. **数量短语**（以数词开头，如「兩千」「三次」）⇒ 数量不是话题；
/// 6. **纯数字** ⇒ 年份/编号不是话题；
/// 7. **非字母数字**（如 OCR 出来的 `____`）⇒ 那不是词。
fn is_content_word(token: &str) -> bool {
    let length = token.chars().count();
    if length < 2 {
        return false;
    }
    if STOPWORDS.contains(&token) || GENERIC_TERMS.contains(&token) {
        return false;
    }
    if token.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if is_cjk_token(token) {
        if token.chars().any(|ch| FUNCTION_CHARS.contains(ch)) {
            return false;
        }
        let first = token.chars().next().unwrap_or('\0');
        if CHINESE_NUMERALS.contains(first) {
            return false;
        }
    }
    token.chars().all(|ch| ch.is_alphanumeric())
}

/// 是不是 CJK token（全部字符都是 CJK 汉字 ⇒ 视为 bigram 候选）。
fn is_cjk_token(token: &str) -> bool {
    !token.is_empty() && token.chars().all(is_cjk)
}

/// 高频虚词字：出现在 CJK bigram 里 ⇒ 该 bigram 多半是**跨词边界的碎片**。
///
/// ⚠️ 与 `golden_mining` 的那张表**同源**，另加繁体虚词「麼」/「麽」/「與」
/// （繁体语料里「怎麼」「這麼」「與黛玉」不带这些字 ⇒ 会被切出来当「实词」⇒ 主体认错）。
const FUNCTION_CHARS: &str = "的了着过是在和与就都也而及或但却还又再更很太把被让使给对从向于并之其有无不要会能可说我你他她们这那些什么怎样呢吗吧啊呀嘛哪谁因所果如麼麽與";

/// 中文数词（数量短语的核心字）。
const CHINESE_NUMERALS: &str = "零一二三四五六七八九十百千万亿两兩";

/// 停用词（**整词**命中才丢）。
///
/// ⚠️ **比 `golden_mining::STOPWORDS` 更全**：那里没有「后来」，
/// 而「后来」是**弱时间锚**（每道题都有 ⇒ 零判别力）⇒ 本模块必须丢掉它。
const STOPWORDS: &[&str] = &[
    // 中文虚词（简体 + 繁体）
    "我们",
    "我們",
    "你们",
    "你們",
    "他们",
    "他們",
    "这个",
    "這個",
    "那个",
    "那個",
    "这些",
    "這些",
    "那些",
    "一个",
    "一個",
    "一些",
    "什么",
    "什麼",
    "怎么",
    "怎麼",
    "怎样",
    "怎樣",
    "这样",
    "這樣",
    "那样",
    "那樣",
    "就是",
    "不是",
    "但是",
    "因为",
    "因為",
    "所以",
    "如果",
    "虽然",
    "然後",
    "然后",
    "已经",
    "已經",
    "还是",
    "還是",
    "可以",
    "不能",
    "没有",
    "沒有",
    "这里",
    "這裡",
    "那里",
    "那裡",
    "时候",
    "時候",
    "现在",
    "現在",
    "之后",
    "之後",
    "之前",
    "以及",
    "而且",
    "或者",
    "不过",
    "不過",
    "于是",
    "於是",
    "因此",
    "由于",
    "由於",
    "关于",
    "關於",
    "对于",
    "對於",
    "今天",
    "明天",
    "昨天",
    "自己",
    "大家",
    "东西",
    "東西",
    "事情",
    // **弱时间锚 / 模板词**（每道题都一样 ⇒ 零判别力 ⇒ 必须丢）
    //
    // ⚠️ **注意 bigram 的字序**：分词是「相邻两字」⇒ 挖题模板里的「后来」
    // 在 bigram 口径下是 **「来后」**（不是「后来」）⇒ 停用词必须照**字序**收。
    "来后",
    "後來",
    "后来",
    "以后",
    "以後",
    "当时",
    "當時",
    "最后",
    "最後",
    "定下",
    "定下來的",
    "下来",
    "下來",
    "件事",
    "事最",
    "时记",
    "時記",
    "这件事",
    "這件事",
    "两件事",
    "兩件事",
    "这两件事",
    "這兩件事",
    "关系",
    "關係",
    "是什么",
    "是什麼",
    "是什么关系",
    "是什麼關係",
    "改的",
    "记了",
    "記了",
    "定了",
    "什么关系",
    "什麼關係",
    // 英文虚词
    "the",
    "and",
    "for",
    "that",
    "with",
    "this",
    "from",
    "have",
    "has",
    "had",
    "was",
    "were",
    "are",
    "but",
    "not",
    "you",
    "your",
    "our",
    "its",
    "his",
    "her",
    "they",
    "them",
    "their",
    "which",
    "what",
    "when",
    "where",
    "would",
    "could",
    "should",
    "been",
    "being",
    "than",
    "then",
    "also",
    "of",
    "is",
    "in",
    "it",
    "to",
    "as",
    "at",
    "by",
    "on",
    "or",
    "an",
    "be",
    "no",
    "so",
    "do",
    "we",
    "he",
    "my",
];

/// 泛称（数量泛称 + 称谓 + 泛指名词）—— 高频且**不指事** ⇒ 当不得信号。
const GENERIC_TERMS: &[&str] = &[
    "一個",
    "兩個",
    "三個",
    "四個",
    "五個",
    "幾個",
    "許多",
    "很多",
    "一个",
    "两个",
    "三个",
    "几个",
    "许多",
    "很多",
    "先生",
    "夫人",
    "老爺",
    "老爷",
    "公子",
    "姑娘",
    "大爺",
    "大爷",
    "和尚",
    "道人",
    "小人",
    "在下",
    "奴家",
    "小二",
    "哥哥",
    "兄弟",
    "頭領",
    "头领",
    "衆人",
    "众人",
    "眾人",
    "那人",
    "此人",
    "有人",
    "那裏",
    "這裏",
    "那里",
    "这里",
    "如何",
    "怎樣",
    "怎样",
    "為何",
    "爲何",
    "为什么",
    "甚麼",
    "說道",
    "说道",
    "只見",
    "只见",
    "聽得",
    "听得",
    "去了",
    "起來",
    "起来",
    "出來",
    "出来",
    "來了",
    "来了",
    "不知",
    "也不",
    "我們",
    "我们",
    "他們",
    "他们",
    "你們",
    "你们",
    "咱們",
    "咱们",
    "那裡",
    "那里",
    "這裡",
    "這里",
    "這樣",
    "这样",
    "那樣",
    "那样",
    "如今",
    "知道",
    "聽見",
    "听见",
    "一面",
    "不得",
    "沒有",
    "没有",
    "東西",
    "东西",
    "就是",
    "告訴",
    "告诉",
    "進來",
    "进来",
    "回來",
    "回来",
    "丫頭",
    "丫头",
    "奶奶",
    "太太",
    "姐儿",
    "姐兒",
    "的人",
    "這個",
    "这个",
    "了說",
    "说了",
    "看著",
    "看着",
];
