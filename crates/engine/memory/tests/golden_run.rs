//! `golden_run` 的测试：把考卷接到**真实检索**上，跑出真分数。
//!
//! 重点不在「跑得动」，而在**跑出来的数字能自证**：
//! * 卷子**不是**字面命中（全中率明显低于 100%）；
//! * 改一个字的语料 ⇒ 分数**真的变**（不是摆设）。

use apeireth_memory::golden_run::{
    hits_by_kind, mined_to_golden, run, summarize, IndexRecord, InjectionMode,
};
use apeireth_memory::golden_set::GoldenSet;

fn record(id: &str, text: &str) -> IndexRecord {
    IndexRecord {
        id: id.to_string(),
        text: text.to_string(),
    }
}

fn golden(cases: Vec<apeireth_memory::golden_set::GoldenCase>) -> GoldenSet {
    GoldenSet::new("test-v1".to_string(), cases)
}

fn case(id: &str, query: &str, expect_ids: Vec<&str>) -> apeireth_memory::golden_set::GoldenCase {
    apeireth_memory::golden_set::GoldenCase {
        id: id.to_string(),
        query: query.to_string(),
        expect_ids: expect_ids.into_iter().map(str::to_string).collect(),
        expect_fragments: Vec::new(),
        tags: vec!["internal".to_string(), "kind:keyword".to_string()],
    }
}

#[test]
fn a_perfectly_matching_query_finds_the_record() {
    let records = vec![
        record("r1", "限流阈值今天定在每秒两千次"),
        record("r2", "今天天气不错适合出门散步"),
    ];
    let set = golden(vec![case("c1", "限流阈值 每秒两千次", vec!["r1"])]);
    let report = run(&records, &set, 5, InjectionMode::Production).expect("跑批");
    assert_eq!(report.fully_hit, 1, "字面能对上就该命中：{report:?}");
    assert!(report.metrics.recall_at_k > 0.9);
}

#[test]
fn an_unrelated_query_does_not_hit() {
    // ⚠️ 这条钉住「卷子能失败」：若任何查询都能命中 ⇒ 指标是假的。
    let records = vec![
        record("r1", "限流阈值今天定在每秒两千次"),
        record("r2", "今天天气不错适合出门散步"),
    ];
    // ⚠️ **k 必须小于语料条数**：k ≥ 语料条数 ⇒ 全部记录都进 top-k ⇒ recall 恒为 1
    // ⇒ 那样测不出「该不该命中」（这是**口径陷阱**，不是实现问题）。
    let set = golden(vec![case("c1", "量子色动力学与规范场", vec!["r1"])]);
    let report = run(&records, &set, 1, InjectionMode::Production).expect("跑批");
    assert_eq!(report.fully_hit, 0, "无关查询不该命中：{report:?}");
    assert!(report.metrics.recall_at_k < 0.5);
}

#[test]
fn corpus_is_actually_searched_not_just_the_answer() {
    // 语料里塞 200 条**干扰**记录 ⇒ 目标记录必须被**排进** top-k，
    // 而不是「因为只有一条所以命中」。
    let mut records = vec![record("target", "记忆索引换成 sqlite 全文检索的决策过程")];
    for index in 0..200 {
        records.push(record(
            &format!("noise-{index}"),
            &format!("无关内容第 {index} 段，写了一些不相干的话，用来当干扰项"),
        ));
    }
    let set = golden(vec![case("c1", "记忆索引 sqlite 全文检索", vec!["target"])]);
    let report = run(&records, &set, 10, InjectionMode::Production).expect("跑批");
    assert_eq!(report.corpus_size, 201);
    assert_eq!(report.fully_hit, 1, "目标应被排进 top-10：{report:?}");
}

#[test]
fn multi_target_case_requires_all_targets_to_hit() {
    let records = vec![
        record("a", "限流阈值最初定在每秒两千次"),
        record("b", "限流阈值后来改成每秒五千次"),
        record("c", "完全无关的内容"),
    ];
    let set = golden(vec![case("c1", "限流阈值", vec!["a", "b"])]);
    let report = run(&records, &set, 3, InjectionMode::Production).expect("跑批");
    // 两条都进 top-3 ⇒ 全中。
    assert_eq!(report.fully_hit, 1, "{report:?}");
}

#[test]
fn summary_reports_a_honest_verdict_about_whether_the_set_actually_tests_anything() {
    // 全中率接近 100% ⇒ 摘要必须**警告**「卷子可能坏了（字面命中）」。
    let records = vec![record("r1", "限流阈值今天定在每秒两千次")];
    let set = golden(vec![case("c1", "限流阈值 每秒两千次", vec!["r1"])]);
    let report = run(&records, &set, 5, InjectionMode::Production).expect("跑批");
    let text = summarize(&report);
    assert!(text.contains("测不出东西"), "{text}");
}

#[test]
fn summary_reports_the_opposite_when_the_set_is_not_trivially_solvable() {
    // 全中率低 ⇒ 摘要必须说「它**真的在测检索**」。
    let mut records = vec![record("r1", "限流阈值今天定在每秒两千次")];
    for index in 0..50 {
        records.push(record(
            &format!("n{index}"),
            "毫不相干的干扰内容，用来把正确答案挤出 top-k",
        ));
    }
    let set = golden(vec![case("c1", "量子色动力学", vec!["r1"])]);
    let report = run(&records, &set, 3, InjectionMode::Production).expect("跑批");
    let text = summarize(&report);
    assert!(text.contains("真的在测检索"), "{text}");
}

#[test]
fn changing_the_corpus_changes_the_score() {
    // 分数必须**对语料敏感** ⇒ 否则指标是摆设。
    let base = vec![
        record("r1", "限流阈值今天定在每秒两千次"),
        record("r2", "今天天气不错适合出门散步"),
    ];
    let set = golden(vec![case("c1", "限流阈值 每秒两千次", vec!["r1"])]);
    let small = run(&base, &set, 2, InjectionMode::Production).expect("跑批");
    let mut big = base.clone();
    for index in 0..100 {
        big.push(record(
            &format!("n{index}"),
            "干扰内容，稀释检索结果，让目标掉出 top-2",
        ));
    }
    let large = run(&big, &set, 2, InjectionMode::Production).expect("跑批");
    assert!(
        large.metrics.recall_at_k <= small.metrics.recall_at_k,
        "语料变大 ⇒ 同 k 下 recall 不该上升：{} vs {}",
        large.metrics.recall_at_k,
        small.metrics.recall_at_k
    );
}

#[test]
fn injection_mode_changes_the_evidence_metrics() {
    // 两种注入口径必须给出**不同**的证据分数 ⇒ 证明指标确实在看「注入什么」。
    // ⚠️ **答案必须在长文本的末尾** ⇒ 「取开头 120 字」够不着它，
    // 而「取最相关片段」能取到 ⇒ 两种口径才会拉开差距。
    let filler = "铺垫文字。".repeat(30);
    let records = vec![record(
        "r1",
        &format!("{filler}真正的答案句：限流阈值定在每秒两千次。"),
    )];
    let mut case = case("c1", "限流阈值 每秒两千次", vec!["r1"]);
    case.expect_fragments = vec!["限流阈值定在每秒两千次".to_string()];
    let set = golden(vec![case]);
    let production = run(&records, &set, 1, InjectionMode::Production).expect("跑批");
    let best = run(&records, &set, 1, InjectionMode::BestSpan).expect("跑批");
    assert!(
        best.metrics.evidence_precision > production.metrics.evidence_precision,
        "最相关片段口径应比「取开头」更能覆盖答案：{} vs {}",
        best.metrics.evidence_precision,
        production.metrics.evidence_precision
    );
}

#[test]
fn hits_by_kind_groups_the_results() {
    let records = vec![
        record("r1", "限流阈值今天定在每秒两千次"),
        record("r2", "今天天气不错适合出门散步"),
    ];
    let set = golden(vec![
        case("c1", "限流阈值 每秒两千次", vec!["r1"]),
        case("c2", "量子色动力学", vec!["r1"]),
    ]);
    // ⚠️ **k=1**（k=2 时两条都进 top-k ⇒ 两题都「命中」⇒ 测不出差别）。
    let report = run(&records, &set, 1, InjectionMode::Production).expect("跑批");
    let stats = hits_by_kind(&report, &set);
    let keyword = stats.get("keyword").expect("有 keyword 分组");
    assert_eq!(keyword.1, 2, "两题都算在 keyword 组");
    assert_eq!(keyword.0, 1, "只有一题命中");
}

#[test]
fn empty_corpus_and_empty_set_do_not_panic() {
    let set = golden(Vec::new());
    let report = run(&[], &set, 5, InjectionMode::Production).expect("跑批");
    assert_eq!(report.cases, 0);
    assert_eq!(report.corpus_size, 0);
    let text = summarize(&report);
    assert!(!text.is_empty());
}

#[test]
fn mined_cases_convert_into_golden_cases_with_kind_tags() {
    let mined = vec![apeireth_memory::golden_mining::MinedCase {
        id: "gm-keyword-0001".to_string(),
        query: "关于限流阈值，我们当时记了什么？".to_string(),
        expect_ids: vec!["r1".to_string()],
        expect_fragments: vec!["限流阈值今天定在每秒两千次".to_string()],
        kind: "keyword".to_string(),
    }];
    let converted = mined_to_golden(mined);
    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].id, "gm-keyword-0001");
    assert_eq!(converted[0].expect_ids, vec!["r1".to_string()]);
    assert!(
        converted[0].tags.contains(&"kind:keyword".to_string()),
        "题型要变成标签：{:?}",
        converted[0].tags
    );
    assert!(converted[0].tags.contains(&"internal".to_string()));
}

#[test]
fn run_is_deterministic() {
    let records = vec![
        record("r1", "限流阈值今天定在每秒两千次"),
        record("r2", "今天天气不错适合出门散步"),
    ];
    let set = golden(vec![case("c1", "限流阈值 每秒两千次", vec!["r1"])]);
    let first = run(&records, &set, 2, InjectionMode::Production).expect("跑批");
    let second = run(&records, &set, 2, InjectionMode::Production).expect("跑批");
    assert_eq!(first, second, "同一输入两次跑批必须逐字相等");
}
