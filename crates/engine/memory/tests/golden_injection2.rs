//! `golden_injection2` 的测试：注入形态对照的**第二轮**（消偏置 + 总预算固定）。
//!
//! 钉住几件事：
//! 1. **总量封顶**：`injected_chars <= total_chars`（逐个预算值测，硬不变式）；
//! 2. **`total_chars = 0` ⇒ 不限总量**（只受 per_item × max_items 约束）；
//! 3. **`max_items` 生效**（条数封顶）；
//! 4. **确定性**：同输入两次 ⇒ 逐字相等；
//! 5. **空输入**：空卷 / 空检索 / 空方案 ⇒ 空结果、不 panic；
//! 6. **均匀采样**：片段位置**不再偏前**（**本批的核心断言**）
//!    + 只改 `expect_fragments` + 片段仍是正文子串 + 短正文退化不炸；
//! 7. **`best_plan`**：覆盖率最高者；并列按名字序；空 ⇒ `None`；
//! 8. **检索结果固定**：只改预算/形态 ⇒ recall 一字不变；
//! 9. **窗口形态真的在测位置**：head / tail / best-window 的覆盖**必须不同**。
//!
//! ⚠️ 真实语料（《紅樓夢》）那几条在文件末尾，标了 `#[ignore]`：
//! 它需要**仓外**语料文件 ⇒ 默认不跑（CI 门禁命令不带 `--run-ignored`），
//! 手动跑：
//! `cargo nextest run -p apeireth-memory --profile ci --run-ignored all --test golden_injection2 --no-capture`

use apeireth_memory::golden_injection::RetrievedRecord;
use apeireth_memory::golden_injection2::{
    best_plan, compare_under_budget, render_budget_table, resample_fragments_uniformly, BudgetPlan,
    UNIFORM_BUCKETS,
};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};

/// 一条命中记录。
fn record(id: &str, content: &str) -> RetrievedRecord {
    RetrievedRecord {
        id: id.to_string(),
        content: content.to_string(),
    }
}

/// 一道题。
fn case(id: &str, query: &str, expect_ids: &[&str], fragments: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: query.to_string(),
        expect_ids: expect_ids.iter().map(|value| value.to_string()).collect(),
        expect_fragments: fragments.iter().map(|value| value.to_string()).collect(),
        tags: vec!["internal".to_string(), "kind:keyword".to_string()],
    }
}

/// 一份卷子。
fn golden(cases: Vec<GoldenCase>) -> GoldenSet {
    GoldenSet::new("golden-injection2-test-v1".to_string(), cases)
}

/// 造一段**多句**正文（每句 30 字左右，句子之间用 `。` 分隔）。
///
/// # 为什么要有这个
///
/// 偏置 ① 的要害是「片段落在正文的哪儿」⇒ 合成正文必须**够长、够多句**
/// ⇒ 否则分段采样会退化成「只有一句」⇒ 测不出位置分布。
fn synthetic_body(sentences: usize, subject: &str) -> String {
    let filler = "甲乙丙丁戊己庚辛壬癸子丑寅卯辰巳午未申酉戌亥";
    let parts: Vec<String> = (0..sentences)
        .map(|index| {
            if index == 0 {
                format!("{subject}在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰")
            } else {
                format!("第{index}段有{subject}甲乙丙丁戊己庚辛壬癸子丑")
            }
        })
        .collect();
    let _ = filler;
    parts.join("。")
}

// ============================================================
// 一、总量封顶（硬不变式）
// ============================================================

#[test]
fn total_budget_is_a_hard_cap() {
    let body = synthetic_body(40, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1", "r2", "r3"],
        &["寶玉在這裡"],
    )]);
    let retrieved = vec![vec![
        record("r1", &body),
        record("r2", &body),
        record("r3", &body),
    ]];

    // 逐个预算值测 ⇒ 每一个都必须**装得下**（`injected_chars <= total_chars`）。
    for total in [1usize, 30, 120, 200, 400, 1000, 5_000, 100_000] {
        let plan = BudgetPlan::new(120, 10, total);
        let outcomes = compare_under_budget(&set, &retrieved, &[plan]);
        assert_eq!(outcomes.len(), 3, "一个方案 ⇒ 三种形态");
        for outcome in &outcomes {
            assert!(
                outcome.injected_chars <= total,
                "总预算是硬上限：{} > {}（{outcome:?}）",
                outcome.injected_chars,
                total
            );
            assert!(
                outcome.injected_items <= plan.max_items,
                "条数封顶生效：{outcome:?}"
            );
        }
    }
}

#[test]
fn the_total_cap_actually_bites() {
    // 反向断言：预算越小 ⇒ 装进去的字越少（否则「封顶」是摆设）。
    let body = synthetic_body(40, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1", "r2", "r3"],
        &["寶玉在這裡"],
    )]);
    let retrieved = vec![vec![
        record("r1", &body),
        record("r2", &body),
        record("r3", &body),
    ]];

    let mut previous = 0usize;
    for total in [120usize, 240, 360, 480] {
        let outcomes = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 10, total)]);
        let chars = outcomes[0].injected_chars;
        assert!(
            chars >= previous,
            "预算变大 ⇒ 装进去的字不该变少：{chars} < {previous}"
        );
        previous = chars;
    }
    assert_eq!(previous, 360, "三条 × 120 字 = 360（预算 480 装得下）");
}

// ============================================================
// 二、total_chars = 0 ⇒ 不限总量
// ============================================================

#[test]
fn zero_total_budget_means_no_total_cap() {
    let body = synthetic_body(40, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1", "r2", "r3"],
        &["寶玉在這裡"],
    )]);
    let retrieved = vec![vec![
        record("r1", &body),
        record("r2", &body),
        record("r3", &body),
    ]];

    let uncapped = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 10, 0)]);
    assert_eq!(
        uncapped[0].injected_chars, 360,
        "不限总量 ⇒ 三条 × 120 字全装下：{uncapped:?}"
    );
    assert_eq!(uncapped[0].injected_items, 3);

    let capped = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 10, 200)]);
    assert!(
        capped[0].injected_chars < uncapped[0].injected_chars,
        "有上限时装得更少 ⇒ 「不限」确实是另一种行为：{capped:?} vs {uncapped:?}"
    );
}

// ============================================================
// 三、max_items 生效
// ============================================================

#[test]
fn max_items_caps_the_number_of_injected_items() {
    let body = synthetic_body(40, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1", "r2", "r3", "r4", "r5"],
        &["寶玉在這裡"],
    )]);
    let retrieved = vec![vec![
        record("r1", &body),
        record("r2", &body),
        record("r3", &body),
        record("r4", &body),
        record("r5", &body),
    ]];

    let outcomes = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 2, 0)]);
    for outcome in &outcomes {
        assert_eq!(outcome.injected_items, 2, "条数封顶生效：{outcome:?}");
        assert_eq!(outcome.injected_chars, 240, "两条 × 120 字");
    }

    // `max_items = 0` ⇒ 不限条数（与 `injection_renderer::Budget` 同口径）。
    let unlimited = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 0, 0)]);
    assert_eq!(unlimited[0].injected_items, 5, "不限条数 ⇒ 五条全装");
}

// ============================================================
// 四、确定性与空输入
// ============================================================

#[test]
fn the_same_input_produces_identical_results() {
    let body = synthetic_body(40, "寶玉");
    let set = golden(vec![
        case(
            "c1",
            "关于寶玉，我们当时记了什么？",
            &["r1"],
            &["寶玉在這裡"],
        ),
        case("c2", "关于寶玉，后来是怎么改的？", &["r2"], &["寶玉在這裡"]),
    ]);
    let retrieved = vec![vec![record("r1", &body)], vec![record("r2", &body)]];
    let plans = vec![
        BudgetPlan::new(120, 10, 0),
        BudgetPlan::new(300, 4, 1200),
        BudgetPlan::new(1200, 1, 1200),
    ];

    let first = compare_under_budget(&set, &retrieved, &plans);
    let second = compare_under_budget(&set, &retrieved, &plans);
    assert_eq!(first, second, "同一输入两次 ⇒ 逐字相等");
    assert_eq!(render_budget_table(&first), render_budget_table(&second));
}

#[test]
fn empty_inputs_yield_no_comparable_outcome() {
    let body = synthetic_body(4, "寶玉");
    let plans = vec![BudgetPlan::new(120, 10, 0)];

    // 空卷。
    let empty_set = golden(Vec::new());
    let retrieved = vec![vec![record("r1", &body)]];
    assert!(compare_under_budget(&empty_set, &retrieved, &plans).is_empty());

    // 空检索结果。
    let set = golden(vec![case("c1", "关于寶玉？", &["r1"], &["寶玉在這裡"])]);
    assert!(compare_under_budget(&set, &[], &plans).is_empty());

    // 空方案表。
    assert!(compare_under_budget(&set, &retrieved, &[]).is_empty());

    assert_eq!(best_plan(&[]), None);
}

// ============================================================
// 五、窗口形态真的在测位置
// ============================================================

#[test]
fn the_window_form_changes_the_coverage() {
    // 期望片段在正文**中间** ⇒ 「取开头」「取结尾」都够不着，只有「最相关窗口」够得着。
    let content = format!(
        "{}限流阈值定在每秒两千次{}",
        "甲".repeat(400),
        "乙".repeat(400)
    );
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let retrieved = vec![vec![record("r1", &content)]];

    let outcomes = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 10, 0)]);
    let head = outcomes
        .iter()
        .find(|outcome| outcome.strategy_id == "head")
        .expect("有 head 行");
    let tail = outcomes
        .iter()
        .find(|outcome| outcome.strategy_id == "tail")
        .expect("有 tail 行");
    let best = outcomes
        .iter()
        .find(|outcome| outcome.strategy_id == "best-window")
        .expect("有 best-window 行");

    assert_eq!(head.fragment_coverage, 0.0, "取开头够不着：{head:?}");
    assert_eq!(tail.fragment_coverage, 0.0, "取结尾够不着：{tail:?}");
    assert_eq!(best.fragment_coverage, 1.0, "最相关窗口够得着：{best:?}");
}

// ============================================================
// 六、检索结果固定（纪律自证）
// ============================================================

#[test]
fn recall_does_not_move_when_only_the_budget_changes() {
    let body = synthetic_body(40, "寶玉");
    let set = golden(vec![
        case(
            "c1",
            "关于寶玉，我们当时记了什么？",
            &["r1"],
            &["寶玉在這裡"],
        ),
        case("c2", "关于賈母，后来是怎么改的？", &["r2"], &["賈母在這裡"]),
    ]);
    let retrieved = vec![
        vec![record("r1", &body), record("r2", &body)],
        vec![record("r2", &body), record("r1", &body)],
    ];
    let plans = vec![
        BudgetPlan::new(120, 10, 0),
        BudgetPlan::new(1200, 1, 1200),
        BudgetPlan::new(300, 4, 1200),
    ];

    let outcomes = compare_under_budget(&set, &retrieved, &plans);
    let first = &outcomes[0].metrics;
    for outcome in &outcomes {
        assert_eq!(
            outcome.metrics.recall_at_k, first.recall_at_k,
            "召回不该随注入预算变：{outcome:?}"
        );
        assert_eq!(
            outcome.metrics.ndcg_at_k, first.ndcg_at_k,
            "排序指标不该随注入预算变：{outcome:?}"
        );
    }
}

// ============================================================
// 七、均匀采样（消解偏置 ①）
// ============================================================

#[test]
fn resampled_fragments_are_not_all_at_the_head() {
    // ⚠️ **本批的核心断言**：偏置 ① 说的是「期望片段系统性偏前」
    // ⇒ 钉住「重采样之后片段位置**跨遍正文**」。
    //
    // 构造：主体只出现在**第一句** ⇒ 挖题器的 `fragment_for` 会把期望片段
    // 放在**正文开头**（偏置的成因）⇒ 重采样后必须**出现尾部片段**。
    let body = synthetic_body(24, "寶玉");
    let head_fragment = "寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰";
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &[head_fragment],
    )]);
    let records = vec![record("r1", &body)];

    // 前提：原片段确实在**头部**（偏置成立）。
    let original_offset = body.find(head_fragment).expect("原片段在正文里");
    assert!(
        original_offset < body.len() / 4,
        "前提：原片段落在正文前 1/4（偏置成立）：{original_offset}"
    );

    let resampled = resample_fragments_uniformly(&set, &records);
    let fragments = &resampled.cases[0].expect_fragments;
    assert!(
        fragments.len() >= 2,
        "重采样后应当有多条片段：{fragments:?}"
    );

    let offsets: Vec<usize> = fragments
        .iter()
        .filter_map(|fragment| body.find(fragment.as_str()))
        .collect();
    assert_eq!(
        offsets.len(),
        fragments.len(),
        "每条片段都得是正文子串：{fragments:?}"
    );
    let last = offsets.iter().copied().max().expect("有片段");
    assert!(
        last > body.len() / 2,
        "重采样后必须有**尾部**片段（偏置真的被消解了）：max offset = {last} / {}：{fragments:?}",
        body.len()
    );
    // 位置分布要**散开**（不是全挤在一段里）。
    assert!(
        offsets.iter().any(|offset| *offset < body.len() / 4)
            && offsets.iter().any(|offset| *offset > body.len() * 3 / 4),
        "片段位置要跨遍正文：{offsets:?}（body {} 字）",
        body.len()
    );
}

#[test]
fn resampling_changes_only_the_expect_fragments() {
    let body = synthetic_body(24, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰"],
    )]);
    let records = vec![record("r1", &body)];

    let resampled = resample_fragments_uniformly(&set, &records);
    assert_eq!(resampled.cases.len(), set.cases.len(), "题数不变");
    assert_eq!(resampled.cases[0].id, set.cases[0].id, "题 id 不变");
    assert_eq!(resampled.cases[0].query, set.cases[0].query, "题面不变");
    assert_eq!(
        resampled.cases[0].expect_ids, set.cases[0].expect_ids,
        "标准答案 id 不变"
    );
    assert_eq!(resampled.cases[0].tags, set.cases[0].tags, "标签不变");
    assert_ne!(
        resampled.cases[0].expect_fragments, set.cases[0].expect_fragments,
        "期望片段应当被重采样（否则重采样没干活）"
    );
    assert!(
        resampled.version != set.version,
        "改题必须换版本（机械保证）：{} vs {}",
        resampled.version,
        set.version
    );
    // 原卷**不能被改**（纯函数）。
    assert_eq!(
        set.cases[0].expect_fragments,
        vec!["寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰".to_string()],
        "原卷子不许被改"
    );
}

#[test]
fn resampled_fragments_are_substrings_of_the_body() {
    // 硬契约：期望片段必须是正文的**子串**（否则「有据」无从判定）。
    let body = synthetic_body(24, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰"],
    )]);
    let records = vec![record("r1", &body)];

    let resampled = resample_fragments_uniformly(&set, &records);
    for fragment in &resampled.cases[0].expect_fragments {
        assert!(
            body.contains(fragment.as_str()),
            "片段必须是正文子串：{fragment}"
        );
        assert!(
            fragment.chars().count() <= 60,
            "片段长度不得超过 60 字（与挖题器同口径）：{fragment}"
        );
    }
}

#[test]
fn a_short_body_degrades_without_panicking() {
    // 退化路径：正文太短（放不下 N 段）⇒ 按句均匀取 ⇒ 不炸、不空。
    let body = "寶玉在這裡。后面还有一句。";
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在這裡"],
    )]);
    let records = vec![record("r1", body)];

    let resampled = resample_fragments_uniformly(&set, &records);
    let fragments = &resampled.cases[0].expect_fragments;
    assert!(!fragments.is_empty(), "短正文也要采得到片段：{fragments:?}");
    for fragment in fragments {
        assert!(body.contains(fragment.as_str()), "仍是正文子串：{fragment}");
    }

    // 更极端：正文是空的 ⇒ 采不到 ⇒ 保留原片段（不丢标注）。
    let empty_set = golden(vec![case("c2", "关于寶玉？", &["r2"], &["寶玉在這裡"])]);
    let empty_records = vec![record("r2", "")];
    let resampled_empty = resample_fragments_uniformly(&empty_set, &empty_records);
    assert_eq!(
        resampled_empty.cases[0].expect_fragments,
        vec!["寶玉在這裡".to_string()],
        "正文为空 ⇒ 保留原片段"
    );

    // 记录根本不存在 ⇒ 同样保留原片段。
    let resampled_missing = resample_fragments_uniformly(&empty_set, &[]);
    assert_eq!(
        resampled_missing.cases[0].expect_fragments,
        vec!["寶玉在這裡".to_string()],
        "找不到记录 ⇒ 保留原片段"
    );
}

#[test]
fn resampling_is_deterministic() {
    let body = synthetic_body(24, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰"],
    )]);
    let records = vec![record("r1", &body)];
    let first = resample_fragments_uniformly(&set, &records);
    let second = resample_fragments_uniformly(&set, &records);
    assert_eq!(first, second, "同一输入两次 ⇒ 逐字相等");
}

#[test]
fn the_uniform_bucket_count_is_eight() {
    // 口径钉死：段数是 8（改它 ⇒ 换口径 ⇒ 必须改测试）。
    assert_eq!(UNIFORM_BUCKETS, 8);
}

// ============================================================
// 八、best_plan 与对照表
// ============================================================

#[test]
fn best_plan_picks_the_highest_coverage_and_breaks_ties_by_name() {
    let body = synthetic_body(24, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰"],
    )]);
    let retrieved = vec![vec![record("r1", &body)]];
    let outcomes = compare_under_budget(
        &set,
        &retrieved,
        &[
            BudgetPlan::new(120, 10, 0),
            BudgetPlan::new(1200, 1, 1200),
            BudgetPlan::new(300, 4, 1200),
        ],
    );
    assert_eq!(outcomes.len(), 9, "三个方案 × 三种形态");

    let winner = best_plan(&outcomes).expect("有胜出方案");
    let best_coverage = outcomes
        .iter()
        .map(|outcome| outcome.fragment_coverage)
        .fold(f64::MIN, f64::max);
    let winner_outcome = outcomes
        .iter()
        .find(|outcome| outcome.label() == winner)
        .expect("胜出方案在表里");
    assert_eq!(winner_outcome.fragment_coverage, best_coverage);

    // 并列 ⇒ 按**标签序**最小（确定性，不看输入顺序）。
    let mut tied = outcomes.clone();
    tied.reverse();
    assert_eq!(
        best_plan(&tied),
        Some(winner),
        "并列按名字序 ⇒ 与输入顺序无关"
    );
}

#[test]
fn the_budget_table_shows_the_key_numbers() {
    let body = synthetic_body(24, "寶玉");
    let set = golden(vec![case(
        "c1",
        "关于寶玉，我们当时记了什么？",
        &["r1"],
        &["寶玉在這裡甲乙丙丁戊己庚辛壬癸子丑寅卯辰"],
    )]);
    let retrieved = vec![vec![record("r1", &body)]];
    let outcomes = compare_under_budget(&set, &retrieved, &[BudgetPlan::new(120, 10, 0)]);
    let table = render_budget_table(&outcomes);

    assert!(table.contains("head-120x10@0"), "表里要有方案标签：{table}");
    assert!(table.contains("coverage"), "表里要有 coverage 列：{table}");
    assert!(
        table.contains("注入字符数"),
        "表里要有注入字符数列：{table}"
    );
    assert!(table.contains("注入条数"), "表里要有注入条数列：{table}");
    assert_eq!(table.lines().count(), 4, "表头 + 三行形态：{table}");
    assert_eq!(render_budget_table(&[]), "", "空结果不画表头");
}

// ============================================================
// 九、真实语料对照（《紅樓夢》120 回）
// ============================================================
//
// ⚠️ 这一节需要**仓外**的真实语料 ⇒ 全部标 `#[ignore]`
// ⇒ CI 门禁命令（不带 `--run-ignored`）**不会**跑它们。
// 手动跑：
// `cargo nextest run -p apeireth-memory --profile ci --run-ignored all --test golden_injection2 --no-capture`

/// 仓外真实语料的路径（**不入库**：版权 + 体积；与 `novel_corpus` 的口径一致）。
const REAL_CORPUS: &str = "/home/bai/.apeireth/novels/redchamber.txt";

/// 检索的 top-k（与内部卷其它实验同口径）。
const RETRIEVAL_K: usize = 10;

/// 每条记录挖几道题（内部卷的挖题配置，与第一轮同口径）。
const MINING_CASES_PER_RECORD: usize = 6;

/// 真实语料实验的**共用准备**（语料 ⇒ 记录 ⇒ 题 ⇒ 检索）。
struct RealSetup {
    /// 全部记录（一条 = 一回；供重采样查正文）。
    corpus: Vec<RetrievedRecord>,
    /// 卷子（**原样**：第一轮口径）。
    set: GoldenSet,
    /// 每题的检索结果（**固定**：所有方案共用同一批 ⇒ 变的只有注入形态与预算）。
    retrieved: Vec<Vec<RetrievedRecord>>,
}

/// 准备真实语料实验（语料不在 ⇒ `None` ⇒ 测试自行跳过）。
fn real_setup() -> Option<RealSetup> {
    use apeireth_memory::golden_mining::MiningConfig;
    use apeireth_memory::golden_run::mined_to_golden;
    use apeireth_memory::novel_corpus::{load_chapters, mine_novel, to_mine_records};
    use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
    use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};

    let path = std::path::Path::new(REAL_CORPUS);
    if !path.exists() {
        println!("跳过：仓外语料不在（{REAL_CORPUS}）");
        return None;
    }
    let chapters = load_chapters(path).expect("读小说文件");
    let config = MiningConfig {
        max_cases_per_record: MINING_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = mine_novel(&chapters, config);
    let set = GoldenSet::new("redchamber-internal-v1".to_string(), mined_to_golden(mined));
    let records = to_mine_records(&chapters);
    let corpus: Vec<RetrievedRecord> = records
        .iter()
        .map(|record| RetrievedRecord {
            id: record.id.clone(),
            content: record.content.clone(),
        })
        .collect();

    // 检索：**与 `golden_run` 同口径**（BM25 + 只留分数 > 0 的命中）。
    let documents: Vec<MemoryCandidate> = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.content.clone(),
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
    let source = Bm25LexicalCandidateSource::new(documents);
    let retrieved: Vec<Vec<RetrievedRecord>> = set
        .cases
        .iter()
        .map(|case| {
            source
                .candidates(&case.query, RETRIEVAL_K)
                .expect("检索")
                .into_iter()
                .filter(|hit| hit.score > 0.0)
                .filter_map(|hit| corpus.iter().find(|record| record.id == hit.id).cloned())
                .collect()
        })
        .collect();
    Some(RealSetup {
        corpus,
        set,
        retrieved,
    })
}

/// 期望片段在正文里的**字符位置**（诊断：片段到底偏不偏前）。
fn fragment_offsets(set: &GoldenSet, corpus: &[RetrievedRecord]) -> Vec<usize> {
    let mut out = Vec::new();
    for case in &set.cases {
        for fragment in &case.expect_fragments {
            let mut found = None;
            for id in &case.expect_ids {
                let Some(record) = corpus.iter().find(|record| &record.id == id) else {
                    continue;
                };
                if let Some(byte) = record.content.find(fragment.as_str()) {
                    found = Some(record.content[..byte].chars().count());
                    break;
                }
            }
            if let Some(offset) = found {
                out.push(offset);
            }
        }
    }
    out
}

/// 打印位置分布（人话）。
fn print_offsets(label: &str, offsets: &[usize]) {
    if offsets.is_empty() {
        println!("{label}: 没有片段");
        return;
    }
    let mut sorted = offsets.to_vec();
    sorted.sort_unstable();
    let percentile = |ratio: f64| -> usize {
        let index = ((sorted.len() as f64 - 1.0) * ratio).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    };
    let within = |limit: usize| -> f64 {
        sorted.iter().filter(|offset| **offset < limit).count() as f64 / sorted.len() as f64
    };
    println!(
        "{label}: 片段 {} 个 · p25={} · p50={} · p75={} · max={}",
        sorted.len(),
        percentile(0.25),
        percentile(0.50),
        percentile(0.75),
        sorted[sorted.len() - 1]
    );
    println!(
        "  落在前 120 字内 {:.1}% · 前 400 字内 {:.1}% · 前 1000 字内 {:.1}%",
        within(120) * 100.0,
        within(400) * 100.0,
        within(1000) * 100.0
    );
}

/// 第二轮对照：**消偏置 + 总预算固定**（《紅樓夢》）。
#[test]
#[ignore = "需要仓外真实语料；用 --run-ignored all 手动跑"]
fn real_corpus_second_round_prints_the_debiased_tables() {
    let Some(setup) = real_setup() else {
        return;
    };

    println!(
        "=== 语料：《紅樓夢》{} 回 · 题 {} 道 · k={} ===",
        setup.corpus.len(),
        setup.set.cases.len(),
        RETRIEVAL_K
    );

    // --- 偏置诊断：片段位置分布（改之前 / 改之后） ---
    let resampled = resample_fragments_uniformly(&setup.set, &setup.corpus);
    print_offsets(
        "--- 期望片段位置（原卷子 = 第一轮口径）---",
        &fragment_offsets(&setup.set, &setup.corpus),
    );
    print_offsets(
        "--- 期望片段位置（均匀重采样 = 本轮口径）---",
        &fragment_offsets(&resampled, &setup.corpus),
    );

    // --- 主表：与第一轮同窗口（每条 120/400/160 字，条数 = k = 10，总量不限） ---
    let main_plans = vec![
        BudgetPlan::new(120, RETRIEVAL_K, 0),
        BudgetPlan::new(400, RETRIEVAL_K, 0),
        BudgetPlan::new(160, RETRIEVAL_K, 0),
        BudgetPlan::new(1_000_000, RETRIEVAL_K, 0),
    ];
    println!("--- 主表（原卷子 = 第一轮口径）---");
    println!(
        "{}",
        render_budget_table(&compare_under_budget(
            &setup.set,
            &setup.retrieved,
            &main_plans
        ))
    );
    println!("--- 主表（均匀重采样 = 消偏置后）---");
    let debiased = compare_under_budget(&resampled, &setup.retrieved, &main_plans);
    println!("{}", render_budget_table(&debiased));

    // --- 同总预算对照：1200 字总预算，少条大窗 vs 多条小窗 ---
    let budget_plans = vec![
        BudgetPlan::new(1200, 1, 1200),
        BudgetPlan::new(300, 4, 1200),
        BudgetPlan::new(120, 10, 1200),
    ];
    println!("--- 同总预算对照（总预算 1200 字 · 消偏置后）---");
    let fixed = compare_under_budget(&resampled, &setup.retrieved, &budget_plans);
    println!("{}", render_budget_table(&fixed));
    println!("--- 同总预算对照（总预算 1200 字 · 原卷子）---");
    println!(
        "{}",
        render_budget_table(&compare_under_budget(
            &setup.set,
            &setup.retrieved,
            &budget_plans
        ))
    );

    println!(
        "--- 结论 ---\n消偏置后 best = {:?}\n原卷子 best = {:?}",
        best_plan(&debiased),
        best_plan(&compare_under_budget(
            &setup.set,
            &setup.retrieved,
            &main_plans
        ))
    );

    // --- 不变量：总量封顶 + 检索固定 ---
    for outcome in fixed.iter().chain(debiased.iter()) {
        assert!(
            outcome.injected_chars <= outcome.plan.total_chars,
            "总预算硬上限：{outcome:?}"
        );
        assert!(
            outcome.injected_items <= outcome.plan.max_items,
            "条数封顶：{outcome:?}"
        );
    }
    for outcome in &debiased {
        assert_eq!(
            outcome.metrics.recall_at_k, debiased[0].metrics.recall_at_k,
            "recall 不随注入形态/预算变：{outcome:?}"
        );
    }
    assert!(!debiased.is_empty());
}
