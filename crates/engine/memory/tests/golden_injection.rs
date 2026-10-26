//! `golden_injection` 的测试：注入策略 × 评测的**实验台**。
//!
//! 钉住几件事：
//! 1. **空输入不炸**：空卷 / 空检索结果 ⇒ 空结果（不 panic）；
//! 2. **窗口形态正确**：`Head` 取开头、`Tail` 取结尾、`Whole` 全文、
//!    `BestWindow` 取**查询词最多**的那一段；
//! 3. **`fragment_coverage` 语义**：包含期望片段 ⇒ 1.0；完全无关 ⇒ 0.0；
//!    部分覆盖 ⇒ 介于 0 与 1 之间；
//! 4. **`best_strategy`**：选覆盖率最高者；并列按**名字序**；空 ⇒ `None`；
//! 5. **确定性**：同一输入两次 ⇒ 逐字相等；
//! 6. **`render_table`** 含策略 id 与三个数字；
//! 7. **「检索结果固定」纪律**：换 `retrieved` ⇒ 覆盖率随之变化
//!    （证明它确实在看**注入进去的内容**，不是摆设）。
//!
//! ⚠️ 真实语料（《紅樓夢》）那条在文件末尾，标了 `#[ignore]`：
//! 它需要**仓外**语料文件 ⇒ 默认不跑（CI 门禁命令不带 `--run-ignored`），
//! 手动跑：`cargo nextest run -p apeireth-memory --run-ignored`。

use apeireth_memory::golden_injection::{
    best_strategy, compare_strategies, render_record, render_table, InjectionStrategy,
    RetrievedRecord, StrategyOutcome,
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
        tags: vec!["internal".to_string()],
    }
}

/// 一份卷子。
fn golden(cases: Vec<GoldenCase>) -> GoldenSet {
    GoldenSet::new("golden-injection-test-v1".to_string(), cases)
}

/// 常用策略集（对照表里的五档）。
fn strategies() -> Vec<InjectionStrategy> {
    vec![
        InjectionStrategy::Head { chars: 120 },
        InjectionStrategy::Head { chars: 400 },
        InjectionStrategy::Tail { chars: 120 },
        InjectionStrategy::BestWindow { chars: 160 },
        InjectionStrategy::Whole,
    ]
}

/// **同预算**的对照集（每条证据都是 120 字 ⇒ 与生产口径**同预算**）。
///
/// # 为什么要有这一档
///
/// 主表里的 `BestWindow{160}` 比生产的 `Head{120}` **多 40 字** ⇒ 覆盖更高时
/// 分不清是「窗口选得准」还是「窗口更大」⇒ 这一档把预算**钉死**。
fn same_budget_strategies() -> Vec<InjectionStrategy> {
    vec![
        InjectionStrategy::Head { chars: 120 },
        InjectionStrategy::BestWindow { chars: 120 },
        InjectionStrategy::Tail { chars: 120 },
    ]
}

/// 打印**证据族指标**（卷子自带的口径）随注入形态的变化。
///
/// # 为什么单独打印
///
/// 主表里的 `coverage` 是**逐片段**口径（本模块的主指标）；
/// 而 `evidence_precision` / `groundedness` 是**卷子自带**的证据族口径
/// ⇒ 三者一起看 ⇒ 才能判断「哪种形态让证据族指标**反映真实质量**」。
fn print_evidence_family(outcomes: &[StrategyOutcome]) {
    println!("--- 证据族指标（卷子自带口径）---");
    println!(
        "{:<20}  {:>10}  {:>12}  {:>12}",
        "策略", "coverage", "evidence_prec", "groundedness"
    );
    for outcome in outcomes {
        println!(
            "{:<20}  {:>10.4}  {:>12.4}  {:>12.4}",
            outcome.strategy_id,
            outcome.fragment_coverage,
            outcome.metrics.evidence_precision,
            outcome.metrics.groundedness
        );
    }
}

// ============================================================
// 一、空输入
// ============================================================

#[test]
fn an_empty_golden_set_yields_no_comparable_outcome() {
    let outcomes = compare_strategies(
        &golden(Vec::new()),
        &[vec![record("r1", "正文")]],
        &strategies(),
    );
    assert!(outcomes.is_empty(), "空卷 ⇒ 没有可比较的策略：{outcomes:?}");
    assert_eq!(best_strategy(&outcomes), None);
}

#[test]
fn no_retrieved_results_yields_no_comparable_outcome() {
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let outcomes = compare_strategies(&set, &[], &strategies());
    assert!(
        outcomes.is_empty(),
        "空检索结果 ⇒ 没有可比较的策略：{outcomes:?}"
    );
    assert_eq!(best_strategy(&outcomes), None);
}

#[test]
fn an_empty_strategy_list_yields_no_rows() {
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let outcomes = compare_strategies(&set, &[vec![record("r1", "限流阈值定在每秒两千次")]], &[]);
    assert!(outcomes.is_empty());
    assert_eq!(render_table(&outcomes), "", "空结果不画表头");
}

// ============================================================
// 二、窗口形态
// ============================================================

#[test]
fn head_strategy_takes_the_beginning_of_the_record() {
    let item = record("r1", "一二三四五六七八九十");
    let text = render_record(InjectionStrategy::Head { chars: 4 }, &item, "查询");
    assert_eq!(text, "一二三四");
    assert_eq!(text.chars().count(), 4, "窗口就是 N 字");
}

#[test]
fn tail_strategy_takes_the_end_of_the_record() {
    let item = record("r1", "一二三四五六七八九十");
    let text = render_record(InjectionStrategy::Tail { chars: 4 }, &item, "查询");
    assert_eq!(text, "七八九十");
}

#[test]
fn whole_strategy_keeps_the_whole_record() {
    let content = "第一段\n\n第二段";
    let item = record("r1", content);
    let text = render_record(InjectionStrategy::Whole, &item, "查询");
    assert_eq!(text, content, "整条不截断");
}

#[test]
fn best_window_strategy_picks_the_window_with_the_most_query_words() {
    // 目标词在**正文中间** ⇒ 「取开头」够不着 ⇒ 只有「最相关窗口」能命中。
    let content = format!(
        "{}限流阈值定在每秒两千次{}",
        "甲".repeat(200),
        "乙".repeat(200)
    );
    let item = record("r1", &content);
    let query = "限流阈值";

    let best = render_record(InjectionStrategy::BestWindow { chars: 160 }, &item, query);
    let head = render_record(InjectionStrategy::Head { chars: 160 }, &item, query);

    assert!(best.contains("限流阈值"), "最相关窗口必须含查询词：{best}");
    assert!(
        !head.contains("限流阈值"),
        "取开头够不着（这是对照的前提）：{head}"
    );
}

#[test]
fn best_window_falls_back_to_the_head_when_the_query_has_no_words() {
    let item = record("r1", "一二三四五六七八九十");
    let text = render_record(InjectionStrategy::BestWindow { chars: 4 }, &item, "？？？");
    assert_eq!(text, "一二三四", "查询没词 ⇒ 不猜，取开头");
}

// ============================================================
// 三、fragment_coverage 语义
// ============================================================

#[test]
fn coverage_is_one_when_the_injected_text_contains_the_expected_fragment() {
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let retrieved = vec![vec![record(
        "r1",
        "限流阈值定在每秒两千次，这是本批拍板的事。",
    )]];

    let outcomes = compare_strategies(&set, &retrieved, &[InjectionStrategy::Whole]);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0].fragment_coverage, 1.0,
        "注入文本含期望片段 ⇒ 覆盖 = 1.0：{outcomes:?}"
    );
}

#[test]
fn coverage_is_zero_when_the_injected_text_is_completely_unrelated() {
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    // ⚠️ 正文用**纯 ASCII** ⇒ 与中文期望片段没有任何公共子串 ⇒ 覆盖必然为 0。
    let retrieved = vec![vec![record(
        "r1",
        "qqqq wwww eeee rrrr tttt yyyy uuuu iiii oooo pppp",
    )]];

    let outcomes = compare_strategies(&set, &retrieved, &[InjectionStrategy::Whole]);
    assert_eq!(
        outcomes[0].fragment_coverage, 0.0,
        "完全不相关 ⇒ 覆盖 = 0.0：{outcomes:?}"
    );
}

#[test]
fn partial_coverage_lands_between_zero_and_one() {
    // 一道题两个期望片段：一个被覆盖、一个没有 ⇒ 逐片段覆盖率 = 1/2。
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次", "另一句完全不相干的答案文字"],
    )]);
    let retrieved = vec![vec![record(
        "r1",
        "限流阈值定在每秒两千次，这是本批拍板的事。",
    )]];

    let outcomes = compare_strategies(&set, &retrieved, &[InjectionStrategy::Whole]);
    let coverage = outcomes[0].fragment_coverage;
    assert!(
        coverage > 0.0 && coverage < 1.0,
        "部分覆盖 ⇒ 介于 0 与 1 之间：{coverage}"
    );
    assert!((coverage - 0.5).abs() < 1e-9, "一半 ⇒ 0.5：{coverage}");
}

#[test]
fn a_short_head_window_cannot_reach_a_fragment_in_the_middle_of_the_body() {
    // 这条钉住**本模块存在的理由**：生产口径（前 120 字）够不着中段的期望片段。
    let content = format!("{}{}", "x".repeat(400), "限流阈值定在每秒两千次");
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let retrieved = vec![vec![record("r1", &content)]];

    let outcomes = compare_strategies(
        &set,
        &retrieved,
        &[
            InjectionStrategy::Head { chars: 120 },
            InjectionStrategy::Whole,
        ],
    );
    assert_eq!(outcomes[0].strategy_id, "head-120");
    assert_eq!(outcomes[0].fragment_coverage, 0.0, "前 120 字够不着");
    assert_eq!(outcomes[1].strategy_id, "whole");
    assert_eq!(
        outcomes[1].fragment_coverage, 1.0,
        "整条够得着：{outcomes:?}"
    );
}

// ============================================================
// 四、best_strategy
// ============================================================

/// 造一行结果（只关心 coverage 与 id）。
fn outcome(id: &str, coverage: f64) -> StrategyOutcome {
    StrategyOutcome {
        strategy_id: id.to_string(),
        metrics: Default::default(),
        fragment_coverage: coverage,
        injected_chars: 0,
    }
}

#[test]
fn best_strategy_picks_the_highest_coverage() {
    let outcomes = vec![
        outcome("head-120", 0.25),
        outcome("whole", 0.90),
        outcome("tail-120", 0.50),
    ];
    assert_eq!(best_strategy(&outcomes), Some("whole".to_string()));
}

#[test]
fn best_strategy_breaks_ties_by_name_order() {
    // 并列 ⇒ 取**名字序最小**的（确定性，不看输入顺序）。
    let outcomes = vec![
        outcome("whole", 0.5),
        outcome("head-120", 0.5),
        outcome("tail-120", 0.5),
    ];
    assert_eq!(best_strategy(&outcomes), Some("head-120".to_string()));
    // 输入顺序换了 ⇒ 结果不变。
    let mut shuffled = outcomes.clone();
    shuffled.reverse();
    assert_eq!(best_strategy(&shuffled), Some("head-120".to_string()));
}

#[test]
fn best_strategy_returns_none_without_outcomes() {
    assert_eq!(best_strategy(&[]), None);
}

// ============================================================
// 五、确定性与报告
// ============================================================

#[test]
fn the_same_input_produces_identical_results() {
    let set = golden(vec![
        case("c1", "限流阈值", &["r1"], &["限流阈值定在每秒两千次"]),
        case("c2", "宝钗", &["r2"], &["宝钗原是金陵十二钗之一"]),
    ]);
    let retrieved = vec![
        vec![record("r1", "限流阈值定在每秒两千次，这是本批拍板的事。")],
        vec![record("r2", "宝钗原是金陵十二钗之一，人物性子稳。")],
    ];

    let first = compare_strategies(&set, &retrieved, &strategies());
    let second = compare_strategies(&set, &retrieved, &strategies());
    assert_eq!(first, second, "同一输入两次 ⇒ 逐字相等");
    assert_eq!(render_table(&first), render_table(&second));
}

#[test]
fn the_table_contains_the_strategy_id_and_the_three_numbers() {
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let retrieved = vec![vec![record(
        "r1",
        "限流阈值定在每秒两千次，这是本批拍板的事。",
    )]];

    let outcomes = compare_strategies(&set, &retrieved, &[InjectionStrategy::Head { chars: 120 }]);
    let table = render_table(&outcomes);

    assert!(table.contains("head-120"), "表里要有策略 id：{table}");
    assert!(table.contains("1.0000"), "表里要有 recall：{table}");
    assert!(table.contains("coverage"), "表里要有 coverage 列：{table}");
    assert!(
        table.contains(&outcomes[0].injected_chars.to_string()),
        "表里要有注入字符数：{table}"
    );
    assert_eq!(table.lines().count(), 2, "表头 + 一行：{table}");
}

// ============================================================
// 六、「检索结果固定」纪律
// ============================================================

#[test]
fn different_retrieved_results_change_the_coverage() {
    // 这条钉住「**测的是注入进去的东西**」：换一批检索结果 ⇒ 覆盖率必须变。
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);

    let hits = vec![vec![record(
        "r1",
        "限流阈值定在每秒两千次，这是本批拍板的事。",
    )]];
    let misses = vec![vec![record(
        "r2",
        "qqqq wwww eeee rrrr tttt yyyy uuuu iiii oooo pppp",
    )]];

    let hit = compare_strategies(&set, &hits, &[InjectionStrategy::Whole]);
    let miss = compare_strategies(&set, &misses, &[InjectionStrategy::Whole]);

    assert_eq!(hit[0].fragment_coverage, 1.0);
    assert_eq!(miss[0].fragment_coverage, 0.0);
}

#[test]
fn the_retrieval_metrics_do_not_move_when_only_the_injection_form_changes() {
    // 「检索结果固定」纪律的**自证**：换注入形态 ⇒ recall / nDCG **一字不变**
    // （变的是证据族与覆盖率）。
    let set = golden(vec![case(
        "c1",
        "限流阈值",
        &["r1"],
        &["限流阈值定在每秒两千次"],
    )]);
    let retrieved = vec![vec![record(
        "r1",
        "限流阈值定在每秒两千次，这是本批拍板的事。",
    )]];

    let outcomes = compare_strategies(&set, &retrieved, &strategies());
    let first = &outcomes[0].metrics;
    for outcome in &outcomes[1..] {
        assert_eq!(
            outcome.metrics.recall_at_k, first.recall_at_k,
            "召回不该随注入形态变：{outcome:?}"
        );
        assert_eq!(
            outcome.metrics.ndcg_at_k, first.ndcg_at_k,
            "排序指标不该随注入形态变：{outcome:?}"
        );
    }
}

// ============================================================
// ============================================================
// 七、真实语料对照（《紅樓夢》120 回）
// ============================================================
//
// ⚠️ 这一节需要**仓外**的真实语料 ⇒ 两条测试都标 `#[ignore]`
// ⇒ CI 门禁命令（不带 `--run-ignored`）**不会**跑它们。
// 手动跑：
// `cargo nextest run -p apeireth-memory --profile ci --run-ignored all --test golden_injection --no-capture`

/// 仓外真实语料的路径（**不入库**：版权 + 体积；与 `novel_corpus` 的口径一致）。
const REAL_CORPUS: &str = "/home/bai/.apeireth/novels/redchamber.txt";

/// 检索的 top-k（与内部卷其它实验同口径）。
const RETRIEVAL_K: usize = 10;

/// 每条记录挖几道题（内部卷的挖题配置）。
const MINING_CASES_PER_RECORD: usize = 6;

/// 真实语料实验的**共用准备**（语料 ⇒ 记录 ⇒ 题 ⇒ 检索）。
struct RealSetup {
    /// 记录（一条 = 一回）。
    records: Vec<apeireth_memory::golden_mining::MineRecord>,
    /// 卷子。
    set: GoldenSet,
    /// 每题的检索结果（**固定**：所有策略共用同一批 ⇒ 变的只有注入形态）。
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
                .filter_map(|hit| {
                    records
                        .iter()
                        .find(|record| record.id == hit.id)
                        .map(|record| RetrievedRecord {
                            id: record.id.clone(),
                            content: record.content.clone(),
                        })
                })
                .collect()
        })
        .collect();
    Some(RealSetup {
        records,
        set,
        retrieved,
    })
}

/// 真实语料上的**对照表**：同一条卷子、同一批检索结果 ⇒ 变的只有注入形态。
///
/// # 它断言什么
///
/// 1. **检索口径与 `golden_run` 一致**（recall 逐字相等）⇒ 本台子没有偷偷换检索；
/// 2. **召回与 nDCG 不随注入形态变**（检索固定）；
/// 3. **覆盖率的单调性**：整条正文必然覆盖得到 ⇒ `whole` 的覆盖 **≥** 任何窗口
///    （窗口是整条的子串 ⇒ 覆盖只会变少）。
#[test]
#[ignore = "需要仓外真实语料；用 --run-ignored all 手动跑"]
fn real_corpus_comparison_prints_the_truth_table() {
    use apeireth_memory::golden_run::{mined_to_golden, run, IndexRecord, InjectionMode};

    let Some(setup) = real_setup() else {
        return;
    };

    // --- 对照 ---
    let outcomes = compare_strategies(&setup.set, &setup.retrieved, &strategies());

    // --- 口径自证：召回必须与 `golden_run::run` 逐字相等 ---
    let index_records: Vec<IndexRecord> = setup
        .records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let report = run(
        &index_records,
        &setup.set,
        RETRIEVAL_K,
        InjectionMode::Production,
    )
    .expect("跑批");
    for outcome in &outcomes {
        assert!(
            (outcome.metrics.recall_at_k - report.metrics.recall_at_k).abs() < 1e-9,
            "本台子的检索口径必须与 golden_run 一致：{} vs {}",
            outcome.metrics.recall_at_k,
            report.metrics.recall_at_k
        );
    }

    // --- 打印（人话对照表原样输出） ---
    println!(
        "=== 语料：《紅樓夢》{} 回 · 记录 {} 条 · 题 {} 道 · k={} ===",
        setup.records.len(),
        setup.records.len(),
        setup.set.cases.len(),
        RETRIEVAL_K
    );
    println!(
        "--- golden_run 摘要 ---\n{}",
        apeireth_memory::golden_run::summarize(&report)
    );
    println!("--- 注入形态对照表 ---");
    println!("{}", render_table(&outcomes));
    print_evidence_family(&outcomes);
    println!("--- 同预算对照（每条 120 字）---");
    println!(
        "{}",
        render_table(&compare_strategies(
            &setup.set,
            &setup.retrieved,
            &same_budget_strategies()
        ))
    );
    println!(
        "--- 结论 ---\nbest = {:?}（覆盖率最高）",
        best_strategy(&outcomes)
    );

    // --- 不变量 ---
    assert!(!outcomes.is_empty());
    let whole = outcomes
        .iter()
        .find(|outcome| outcome.strategy_id == "whole")
        .expect("有 whole 行");
    for outcome in &outcomes {
        assert!(
            outcome.fragment_coverage <= whole.fragment_coverage + 1e-9,
            "整条正文覆盖得最多（窗口是子串）：{outcome:?} vs {whole:?}"
        );
    }
    for outcome in &outcomes {
        assert_eq!(outcome.metrics.recall_at_k, outcomes[0].metrics.recall_at_k);
        assert_eq!(outcome.metrics.ndcg_at_k, outcomes[0].metrics.ndcg_at_k);
    }
    let _ = mined_to_golden;
}

/// **把检索从注入形态里剥掉**：只在「目标记录确实被检索到」的题上再跑一遍对照。
///
/// # 为什么要有这条
///
/// 全量对照里，**覆盖率的上限由检索决定**（目标记录没被检索到 ⇒ 任何窗口都覆盖不到）
/// ⇒ 全量数字**混着两件事**：检索召回 + 注入形态。
/// ⇒ 只留「全部标准答案都被检索到」的题 ⇒ 覆盖率的差异**只来自注入形态**。
#[test]
#[ignore = "需要仓外真实语料；用 --run-ignored all 手动跑"]
fn real_corpus_conditional_coverage_isolates_the_injection_form() {
    let Some(setup) = real_setup() else {
        return;
    };

    // --- 期望片段在正文里的位置（诊断：窗口到底够不够得着） ---
    let mut offsets: Vec<usize> = Vec::new();
    for case in &setup.set.cases {
        for (id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let Some(record) = setup.records.iter().find(|record| &record.id == id) else {
                continue;
            };
            if let Some(byte) = record.content.find(fragment.as_str()) {
                offsets.push(record.content[..byte].chars().count());
            }
        }
    }
    offsets.sort_unstable();
    if !offsets.is_empty() {
        let percentile = |ratio: f64| -> usize {
            let index = ((offsets.len() as f64 - 1.0) * ratio).round() as usize;
            offsets[index.min(offsets.len() - 1)]
        };
        let within = |limit: usize| -> f64 {
            offsets.iter().filter(|offset| **offset < limit).count() as f64 / offsets.len() as f64
        };
        println!(
            "--- 期望片段在正文里的位置 ---\n片段 {} 个 · p25={} · p50={} · p75={} · max={}",
            offsets.len(),
            percentile(0.25),
            percentile(0.50),
            percentile(0.75),
            offsets[offsets.len() - 1]
        );
        println!(
            "落在前 120 字内 {:.1}% · 前 400 字内 {:.1}% · 前 1000 字内 {:.1}%",
            within(120) * 100.0,
            within(400) * 100.0,
            within(1000) * 100.0
        );
    }

    // --- 只留「全部标准答案都被检索到」的题 ---
    let mut cases = Vec::new();
    let mut retrieved = Vec::new();
    for (index, case) in setup.set.cases.iter().enumerate() {
        let Some(records) = setup.retrieved.get(index) else {
            continue;
        };
        let ids: Vec<&str> = records.iter().map(|record| record.id.as_str()).collect();
        if case.expect_ids.iter().all(|id| ids.contains(&id.as_str())) {
            cases.push(case.clone());
            retrieved.push(records.clone());
        }
    }
    let subset = GoldenSet::new(format!("{}-hit-only", setup.set.version), cases);
    println!(
        "--- 条件对照（只留全中题：{} / {} 道）---",
        subset.cases.len(),
        setup.set.cases.len()
    );

    let outcomes = compare_strategies(&subset, &retrieved, &strategies());
    println!("{}", render_table(&outcomes));
    print_evidence_family(&outcomes);
    println!("--- 同预算对照（每条 120 字）---");
    println!(
        "{}",
        render_table(&compare_strategies(
            &subset,
            &retrieved,
            &same_budget_strategies()
        ))
    );
    println!("best = {:?}（覆盖率最高）", best_strategy(&outcomes));
    assert!(!outcomes.is_empty());
}
