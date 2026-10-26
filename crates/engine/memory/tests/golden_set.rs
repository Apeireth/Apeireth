//! 记忆质量考卷（golden set）集成测试。
//!
//! 钉住几件事（每条都是「不许悄悄变」的判据）：
//! 1. **卷子格式**：JSONL 能读、空行忽略、坏行报**行号**、缺字段报**字段名**；
//!    改题必须改版本号（版本 = 内容摘要，机器保证，不靠人自觉）。
//! 2. **指标口径**：
//!    - Recall@k：全命中 = 1.0，部分命中按比例，k 截断生效，
//!      **无标准答案的题跳过**（不进分母、计入 `skipped_cases`，不是记 0 分）；
//!    - nDCG@k：完美排序 = 1.0，命中被压到后面明显 < 1.0，
//!      且与**人工算出来的数**一致（写死具体数字）；
//!    - Evidence Precision / Groundedness：只对**有 `expect_fragments`** 的题统计，
//!      无标注题跳过（**不是 0 分**）。
//! 3. **闸门**：`evaluate` 长度不匹配 ⇒ Err；`check_no_regression` 完全相同 ⇒ Ok，
//!    掉分 ⇒ Err 且**消息里含指标名**，1% 以内的微小劣化放行（边界：恰好 1% 通过）。
//! 4. **确定性**：同一输入两次 `evaluate` 逐字节相等（设计稿 §6.4「比稳定分不比单次分」）。

use apeireth_memory::golden_set::{
    check_no_regression, evaluate, EvalBatch, GoldenCase, GoldenSet, GoldenSetError, Metrics,
    Retrieved,
};

/// 构造一条考卷题目（无答案片段标注 ⇒ 不参与 Groundedness 统计）。
fn case(id: &str, expect_ids: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: format!("问题 {id}"),
        expect_ids: expect_ids.iter().map(|s| s.to_string()).collect(),
        expect_fragments: Vec::new(),
        tags: Vec::new(),
    }
}

/// 构造一条带答案片段标注的考卷题目。
fn case_with_fragments(id: &str, expect_ids: &[&str], expect_fragments: &[&str]) -> GoldenCase {
    GoldenCase {
        id: id.to_string(),
        query: format!("问题 {id}"),
        expect_ids: expect_ids.iter().map(|s| s.to_string()).collect(),
        expect_fragments: expect_fragments.iter().map(|s| s.to_string()).collect(),
        tags: Vec::new(),
    }
}

/// 构造一次检索返回（只有 id 候选）。
fn retrieved(ids: &[&str]) -> Retrieved {
    Retrieved {
        ids: ids.iter().map(|s| s.to_string()).collect(),
        fragments: Vec::new(),
    }
}

/// 构造一次带注入片段的检索返回。
fn retrieved_with_fragments(ids: &[&str], fragments: &[&str]) -> Retrieved {
    Retrieved {
        ids: ids.iter().map(|s| s.to_string()).collect(),
        fragments: fragments.iter().map(|s| s.to_string()).collect(),
    }
}

/// 近似相等（浮点比较给一点容差）。
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

// ============================================
// 1. 卷子格式：load_jsonl / filtered
// ============================================

#[test]
fn load_jsonl_parses_cases_and_derives_version() {
    let text = concat!(
        r#"{"id":"c1","query":"量子纠缠的实验叫什么","expect_ids":["r1","r2"],"expect_fragments":["贝尔不等式"],"tags":["retrieval"]}"#,
        "\n",
        r#"{"id":"c2","query":"记忆的遗忘语义是什么","expect_ids":["r3"],"tags":["forgetting"]}"#,
        "\n",
    );
    let set = GoldenSet::load_jsonl(text).expect("JSONL 应解析成功");

    assert_eq!(set.len(), 2);
    assert!(!set.is_empty());
    assert_eq!(set.cases[0].id, "c1");
    assert_eq!(set.cases[0].query, "量子纠缠的实验叫什么");
    assert_eq!(
        set.cases[0].expect_ids,
        vec!["r1".to_string(), "r2".to_string()]
    );
    assert_eq!(
        set.cases[0].expect_fragments,
        vec!["贝尔不等式".to_string()]
    );
    assert_eq!(set.cases[0].tags, vec!["retrieval".to_string()]);
    // 第二题没写 expect_fragments ⇒ 缺省为空（不报错）；tags 是显式写的。
    assert!(set.cases[1].expect_fragments.is_empty());
    assert_eq!(set.cases[1].tags, vec!["forgetting".to_string()]);
    // 版本号是内容摘要（机器保证「改题必须换版本」）。
    assert!(
        set.version.starts_with("sha256:"),
        "版本号应为内容摘要，实际 = {}",
        set.version
    );
}

#[test]
fn load_jsonl_ignores_blank_lines() {
    let text = concat!(
        "\n",
        r#"{"id":"c1","query":"q1","expect_ids":["r1"]}"#,
        "\n",
        "   \n",
        "\t\n",
        r#"{"id":"c2","query":"q2","expect_ids":["r2"]}"#,
        "\n",
    );
    let set = GoldenSet::load_jsonl(text).expect("JSONL 应解析成功");
    assert_eq!(set.len(), 2, "空行/空白行必须被忽略");
    assert_eq!(set.cases[0].id, "c1");
    assert_eq!(set.cases[1].id, "c2");
}

#[test]
fn load_jsonl_reports_broken_json_with_line_number() {
    let text = concat!(
        r#"{"id":"c1","query":"q1","expect_ids":["r1"]}"#,
        "\n",
        r#"{"id":"c2","query":"q2","expect_ids":["r2"]}"#,
        "\n",
        "{ this is not json }\n",
    );
    let err = GoldenSet::load_jsonl(text).expect_err("坏 JSON 必须报错");
    match err {
        GoldenSetError::Json { line, source } => {
            assert_eq!(line, 3, "必须报出出错行号（1 起）");
            assert!(!source.is_empty(), "错误描述不能为空");
        }
        other => panic!("应报 Json 变体，实际 = {other:?}"),
    }
}

#[test]
fn load_jsonl_reports_missing_field_name() {
    // 缺 `query`（必填字段之一）。
    let text = r#"{"id":"c1","expect_ids":["r1"]}"#;
    let err = GoldenSet::load_jsonl(text).expect_err("缺必填字段必须报错");
    match err {
        GoldenSetError::MissingField { field, line } => {
            assert_eq!(field, "query", "必须报出缺失字段名");
            assert_eq!(line, 1);
        }
        other => panic!("应报 MissingField 变体，实际 = {other:?}"),
    }
}

#[test]
fn load_jsonl_version_changes_when_content_changes() {
    let a = GoldenSet::load_jsonl(r#"{"id":"c1","query":"q1","expect_ids":["r1"]}"#).unwrap();
    let b = GoldenSet::load_jsonl(r#"{"id":"c1","query":"q1","expect_ids":["r2"]}"#).unwrap();
    assert_ne!(
        a.version, b.version,
        "改了题（标准答案变了）必须换版本，否则历史分数不可比"
    );

    let same = GoldenSet::load_jsonl(r#"{"id":"c1","query":"q1","expect_ids":["r1"]}"#).unwrap();
    assert_eq!(
        a.version, same.version,
        "同样的题必须给出同样的版本（可复现）"
    );
}

#[test]
fn filtered_returns_only_matching_tag() {
    let mut retrieval = case("c1", &["r1"]);
    retrieval.tags = vec!["retrieval".to_string()];
    let mut forgetting = case("c2", &["r2"]);
    forgetting.tags = vec!["forgetting".to_string()];
    let mut both = case("c3", &["r3"]);
    both.tags = vec!["retrieval".to_string(), "temporal".to_string()];

    let set = GoldenSet::new("v1", vec![retrieval, forgetting, both]);

    let only_retrieval = set.filtered("retrieval");
    assert_eq!(only_retrieval.len(), 2);
    assert_eq!(only_retrieval.cases[0].id, "c1");
    assert_eq!(only_retrieval.cases[1].id, "c3");
    assert_eq!(only_retrieval.version, "v1", "子集沿用同一版本号");

    // 空 tag ⇒ 返回全部。
    let all = set.filtered("");
    assert_eq!(all.len(), 3);
    assert_eq!(all.cases, set.cases);
}

// ============================================
// 2. 指标口径
// ============================================

#[test]
fn recall_perfect_hit_is_one() {
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1", "r2"])],
        results: vec![retrieved(&["r1", "r2", "noise"])],
    };
    let m = evaluate(&batch, 3).expect("长度一致");
    assert!(approx(m.recall_at_k, 1.0), "全命中 ⇒ recall = 1.0");
    assert_eq!(m.total_cases, 1);
    assert_eq!(m.skipped_cases, 0);
}

#[test]
fn recall_partial_hit_is_proportional() {
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1", "r2"])],
        results: vec![retrieved(&["r1", "noise"])],
    };
    let m = evaluate(&batch, 2).expect("长度一致");
    assert!(approx(m.recall_at_k, 0.5), "命中 1/2 ⇒ recall = 0.5");
}

#[test]
fn recall_respects_k_truncation() {
    // 两条标准答案都在候选里，但 k=1 只看第一个 ⇒ 只能命中 1 条。
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1", "r2"])],
        results: vec![retrieved(&["r1", "r2"])],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert!(approx(m.recall_at_k, 0.5), "k=1 时 recall = 1/2");
}

#[test]
fn recall_skips_cases_without_expect_ids() {
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"]), case("c2", &[])],
        results: vec![retrieved(&["r1"]), retrieved(&["r1"])],
    };
    let m = evaluate(&batch, 1).expect("长度一致");

    assert!(
        approx(m.recall_at_k, 1.0),
        "无标准答案的题不进 Recall 分母（不是记 0 分）"
    );
    assert_eq!(m.total_cases, 2);
    assert_eq!(m.skipped_cases, 1, "无标准答案的题必须计入 skipped_cases");
}

#[test]
fn ndcg_perfect_ranking_is_one() {
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"])],
        results: vec![retrieved(&["r1", "noise"])],
    };
    let m = evaluate(&batch, 2).expect("长度一致");
    assert!(approx(m.ndcg_at_k, 1.0), "标准答案排第一 ⇒ nDCG = 1.0");
}

#[test]
fn ndcg_hit_at_second_position_matches_hand_computed_value() {
    // 人工计算：DCG = 1/log2(2) * 0 + 1/log2(3) * 1 = 1/log2(3)；
    // IDCG = 1/log2(2) = 1 ⇒ nDCG = 1/log2(3) ≈ 0.6309297535714575。
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"])],
        results: vec![retrieved(&["noise", "r1"])],
    };
    let m = evaluate(&batch, 2).expect("长度一致");
    assert!(
        approx(m.ndcg_at_k, 0.630_929_753_571_457_5),
        "nDCG 必须等于人工算出的 1/log2(3)，实际 = {}",
        m.ndcg_at_k
    );
}

#[test]
fn ndcg_noisy_ranking_is_clearly_below_one() {
    // 命中被噪声压到第 3 位：DCG = 1/log2(4) = 0.5 ⇒ nDCG = 0.5（明显 < 1.0）。
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"])],
        results: vec![retrieved(&["n1", "n2", "r1"])],
    };
    let m = evaluate(&batch, 3).expect("长度一致");
    assert!(
        m.ndcg_at_k < 0.6 && m.ndcg_at_k > 0.4,
        "命中排到第 3 位 ⇒ nDCG ≈ 0.5，实际 = {}",
        m.ndcg_at_k
    );
    assert!(approx(m.ndcg_at_k, 0.5), "nDCG = 1/log2(4) = 0.5");
}

#[test]
fn ndcg_counts_cases_without_expect_ids_as_zero() {
    // 保守方向：无标准答案的题在 nDCG 上记 0 并**计入分母**
    // （不许靠「跳过难题」把分刷高）。
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"]), case("c2", &[])],
        results: vec![retrieved(&["r1"]), retrieved(&["r1"])],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert!(
        approx(m.ndcg_at_k, 0.5),
        "两题分母：满分题 1.0 + 无标准答案题 0.0 ⇒ 平均 0.5，实际 = {}",
        m.ndcg_at_k
    );
}

#[test]
fn evidence_and_groundedness_scored_on_annotated_cases() {
    let batch = EvalBatch {
        cases: vec![
            case_with_fragments("c1", &["r1"], &["贝尔不等式"]),
            case_with_fragments("c2", &["r2"], &["三段式处置"]),
        ],
        results: vec![
            retrieved_with_fragments(&["r1"], &["这里提到了贝尔不等式", "无关内容"]),
            retrieved_with_fragments(&["r2"], &["三段式处置的说明"]),
        ],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert!(
        approx(m.evidence_precision, 0.75),
        "第 1 题 1/2、第 2 题 1/1 ⇒ 平均 0.75，实际 = {}",
        m.evidence_precision
    );
    // ⚠️ **口径拆分（2026-10-26）**：Groundedness 现在是**题级**
    // （「这道题拿到了至少一条有据证据吗」）⇒ 与 Evidence Precision（片段级）**分母不同**。
    // 本例：两题都拿到了有据证据 ⇒ 题级 = 2/2 = **1.0**（而片段级是 0.75）。
    assert!(
        approx(m.groundedness, 1.0),
        "Groundedness 是题级覆盖（两题都有有据证据 ⇒ 1.0），实际 = {}",
        m.groundedness
    );
}

#[test]
fn evidence_precision_and_groundedness_have_different_denominators() {
    // 第 1 题：注了 2 段，只有 1 段有据 ⇒ 片段级 = 0.5，题级 = 1.0；
    // 第 2 题：注了 2 段，**都没**有据 ⇒ 片段级 = 0.0，题级 = 0.0。
    // ⇒ 片段级平均 = 0.25；题级平均 = 0.5。
    // ⇒ **两个指标确实在测两件事**（之前它们同口径 ⇒ 一个指标测两件事）。
    let batch = EvalBatch {
        cases: vec![
            case_with_fragments("c1", &["r1"], &["贝尔不等式"]),
            case_with_fragments("c2", &["r2"], &["三段式处置"]),
        ],
        results: vec![
            retrieved_with_fragments(&["r1"], &["这里提到了贝尔不等式", "无关内容"]),
            retrieved_with_fragments(&["r2"], &["完全无关", "依然无关"]),
        ],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert!(
        approx(m.evidence_precision, 0.25),
        "片段级：(0.5 + 0.0)/2 = 0.25，实际 = {}",
        m.evidence_precision
    );
    assert!(
        approx(m.groundedness, 0.5),
        "题级：(1.0 + 0.0)/2 = 0.5，实际 = {}",
        m.groundedness
    );
}

#[test]
fn a_fragment_cut_by_the_window_still_counts_as_grounded() {
    // ⚠️ 这条钉住**口径修正的核心**：期望片段是**句子里的一个切片**，
    // 注入窗口切过那句话 ⇒ **逐字包含必然失败** ⇒ 指标对窗口对齐过敏。
    // ⇒ 新口径按**覆盖**（最长公共子串 ≥ 60% 且 ≥ 4 字）判定 ⇒ 切了一半**仍算有据**。
    let expected = "贝尔不等式是量子信息学的核心工具";
    let injected = "这里提到了贝尔不等式是量子信息学的"; // 被窗口切掉尾巴
    let m = EvalBatch {
        cases: vec![case_with_fragments("c1", &["r1"], &[expected])],
        results: vec![retrieved_with_fragments(&["r1"], &[injected])],
    };
    let metrics = evaluate(&m, 1).expect("长度一致");
    assert!(
        approx(metrics.evidence_precision, 1.0),
        "切了一半仍算有据（覆盖判定），实际 = {}",
        metrics.evidence_precision
    );
}

#[test]
fn an_unrelated_injection_does_not_count_as_grounded() {
    // 覆盖判定的**下界**：完全无关 ⇒ 不算有据。
    let batch = EvalBatch {
        cases: vec![case_with_fragments("c1", &["r1"], &["贝尔不等式"])],
        results: vec![retrieved_with_fragments(
            &["r1"],
            &["今天天气不错适合出门散步"],
        )],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert!(approx(m.evidence_precision, 0.0));
    assert!(approx(m.groundedness, 0.0));
}

#[test]
fn evidence_metrics_skip_unannotated_cases() {
    // 第 2 题没有 expect_fragments ⇒ **跳过**（不是记 0 分）。
    let batch = EvalBatch {
        cases: vec![
            case_with_fragments("c1", &["r1"], &["关键结论"]),
            case("c2", &["r2"]),
        ],
        results: vec![
            retrieved_with_fragments(&["r1"], &["关键结论在这里", "无关"]),
            retrieved_with_fragments(&["r2"], &["随便注入"]),
        ],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert!(
        approx(m.evidence_precision, 0.5),
        "只有有标注的第 1 题参与统计 ⇒ 1/2 = 0.5，实际 = {}",
        m.evidence_precision
    );
    assert!(
        approx(m.groundedness, 1.0),
        "Groundedness 是题级 ⇒ 有标注的第 1 题拿到了有据证据 ⇒ 1.0，实际 = {}",
        m.groundedness
    );
    assert_eq!(
        m.skipped_cases, 0,
        "缺证据标注的题对证据族是「不适用」，不计 skipped_cases（那一栏专记整题无法评的题）"
    );
    assert_eq!(m.total_cases, 2);
}

#[test]
fn evidence_metrics_are_zero_when_nothing_is_annotated() {
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"])],
        results: vec![retrieved(&["r1"])],
    };
    let m = evaluate(&batch, 1).expect("长度一致");
    assert_eq!(m.evidence_precision, 0.0, "分母为 0 ⇒ 该指标记 0.0");
    assert_eq!(m.groundedness, 0.0, "分母为 0 ⇒ 该指标记 0.0");
    assert!(approx(m.recall_at_k, 1.0));
}

#[test]
fn evaluate_rejects_length_mismatch() {
    let batch = EvalBatch {
        cases: vec![case("c1", &["r1"]), case("c2", &["r2"])],
        results: vec![retrieved(&["r1"])],
    };
    let err = evaluate(&batch, 3).expect_err("长度不一致必须报错");
    match err {
        GoldenSetError::LengthMismatch { cases, results } => {
            assert_eq!(cases, 2);
            assert_eq!(results, 1);
        }
        other => panic!("应报 LengthMismatch 变体，实际 = {other:?}"),
    }
}

#[test]
fn evaluate_on_empty_batch_is_all_zero() {
    let batch = EvalBatch::default();
    let m = evaluate(&batch, 5).expect("空批不报错");
    assert_eq!(m.total_cases, 0);
    assert_eq!(m.skipped_cases, 0);
    assert_eq!(m.recall_at_k, 0.0);
    assert_eq!(m.ndcg_at_k, 0.0);
    assert_eq!(m.evidence_precision, 0.0);
    assert_eq!(m.groundedness, 0.0);
}

// ============================================
// 3. 回归闸门
// ============================================

#[test]
fn check_no_regression_accepts_identical_metrics() {
    let baseline = Metrics {
        recall_at_k: 0.8,
        ndcg_at_k: 0.7,
        evidence_precision: 0.6,
        groundedness: 0.5,
        skipped_cases: 1,
        total_cases: 10,
    };
    let current = baseline.clone();
    assert!(
        check_no_regression(&baseline, &current).is_ok(),
        "完全相同必须放行"
    );
}

#[test]
fn check_no_regression_reports_metric_name_when_dropped() {
    let baseline = Metrics {
        recall_at_k: 0.8,
        ndcg_at_k: 0.7,
        evidence_precision: 0.6,
        groundedness: 0.5,
        skipped_cases: 0,
        total_cases: 10,
    };
    let current = Metrics {
        recall_at_k: 0.79, // 掉 0.01 = 1.25% > 1% 容忍度
        ..baseline.clone()
    };
    let err = check_no_regression(&baseline, &current).expect_err("掉分必须拦下");
    assert!(
        err.contains("recall_at_k"),
        "错误消息必须含指标名，实际 = {err}"
    );
    assert!(
        err.contains("0.01"),
        "错误消息必须说清掉了多少，实际 = {err}"
    );
}

#[test]
fn check_no_regression_flags_each_compared_metric() {
    let baseline = Metrics {
        recall_at_k: 0.8,
        ndcg_at_k: 0.7,
        evidence_precision: 0.6,
        groundedness: 0.5,
        skipped_cases: 0,
        total_cases: 10,
    };

    let ndcg_drop = Metrics {
        ndcg_at_k: 0.5,
        ..baseline.clone()
    };
    let err = check_no_regression(&baseline, &ndcg_drop).expect_err("nDCG 掉分必须拦下");
    assert!(err.contains("ndcg_at_k"), "实际 = {err}");

    let grounded_drop = Metrics {
        groundedness: 0.4,
        ..baseline.clone()
    };
    let err =
        check_no_regression(&baseline, &grounded_drop).expect_err("groundedness 掉分必须拦下");
    assert!(err.contains("groundedness"), "实际 = {err}");
}

#[test]
fn check_no_regression_ignores_metrics_outside_the_gate() {
    // 契约钦定入闸的只有 recall / nDCG / groundedness 三项；
    // evidence_precision（以及将来的 token 类指标）只作观察量，不参与比较。
    let baseline = Metrics {
        recall_at_k: 0.8,
        ndcg_at_k: 0.7,
        evidence_precision: 0.6,
        groundedness: 0.5,
        skipped_cases: 0,
        total_cases: 10,
    };
    let current = Metrics {
        evidence_precision: 0.0,
        ..baseline.clone()
    };
    assert!(
        check_no_regression(&baseline, &current).is_ok(),
        "不入闸的指标变化不应拦下"
    );
}

#[test]
fn check_no_regression_allows_one_percent_tolerance() {
    let baseline = Metrics {
        recall_at_k: 0.5,
        ndcg_at_k: 0.5,
        evidence_precision: 0.5,
        groundedness: 0.5,
        skipped_cases: 0,
        total_cases: 10,
    };

    // 恰好 1%（0.5 → 0.495）⇒ 放行（边界）。
    let at_boundary = Metrics {
        recall_at_k: 0.495,
        ..baseline.clone()
    };
    assert!(
        check_no_regression(&baseline, &at_boundary).is_ok(),
        "恰好 1% 的劣化应当放行"
    );

    // 超过 1%（0.5 → 0.4949）⇒ 拦下。
    let beyond = Metrics {
        recall_at_k: 0.4949,
        ..baseline.clone()
    };
    let err = check_no_regression(&baseline, &beyond).expect_err("超过 1% 必须拦下");
    assert!(err.contains("recall_at_k"), "实际 = {err}");
}

#[test]
fn check_no_regression_allows_improvement() {
    let baseline = Metrics {
        recall_at_k: 0.5,
        ndcg_at_k: 0.5,
        evidence_precision: 0.5,
        groundedness: 0.5,
        skipped_cases: 0,
        total_cases: 10,
    };
    let better = Metrics {
        recall_at_k: 0.9,
        ndcg_at_k: 0.9,
        evidence_precision: 0.9,
        groundedness: 0.9,
        skipped_cases: 0,
        total_cases: 10,
    };
    assert!(
        check_no_regression(&baseline, &better).is_ok(),
        "变好永远放行"
    );
}

// ============================================
// 4. 确定性
// ============================================

#[test]
fn evaluate_is_deterministic_across_runs() {
    let batch = EvalBatch {
        cases: vec![
            case_with_fragments("c1", &["r1", "r2"], &["甲"]),
            case_with_fragments("c2", &["r3"], &["乙", "丙"]),
            case("c3", &[]),
        ],
        results: vec![
            retrieved_with_fragments(&["r2", "r1", "noise"], &["含甲的片段", "无关"]),
            retrieved_with_fragments(&["n1", "r3"], &["含乙的片段"]),
            retrieved_with_fragments(&["n2"], &["随便"]),
        ],
    };

    let first = evaluate(&batch, 2).expect("长度一致");
    let second = evaluate(&batch, 2).expect("长度一致");
    assert_eq!(first, second, "同一输入两次评测必须完全相等");
    assert_eq!(
        format!("{first:?}"),
        format!("{second:?}"),
        "同一输入两次评测必须逐字节相等"
    );
}
