//! `robustness_metrics` 的测试：设计稿 §6.1 **L3 鲁棒/成长**四项。
//!
//! # 这批测试钉住什么
//!
//! * **确定性**：同一输入跑两次 ⇒ 摘要**逐字节**相同（且**比的是哪段文本**要钉死）；
//! * **噪声鲁棒**：跌幅是**相对**的（不是绝对分差）⇒ 超过门限 ⇒ `survived = false`；
//! * **规模延展**：斜率公式用**已知耗时**直接验算（不许靠墙钟碰运气）；
//! * **越权**：恒 0 ⇒ 通过；非 0 ⇒ **失败 + 明细能定位**；
//! * **噪声注入**：**同种子逐字相等**、`rate = 0` ⇒ 原样不动。

use std::cell::Cell;
use std::time::Duration;

use apeireth_memory::golden_run::{summarize, RunReport};
use apeireth_memory::golden_set::Metrics;
use apeireth_memory::robustness_metrics::{
    inject_noise, measure_determinism, measure_noise_robustness, measure_overreach,
    measure_scale_slope, render_robustness_report, scale_slope_from_ms, DeterminismVerdict,
    NoiseVerdict, OverreachVerdict, ScaleVerdict, NOISE_FRAGMENT,
};

/// 造一份跑批报告（`recall` 决定分数；其余字段保持一致）。
fn report(corpus: usize, recall: f64) -> RunReport {
    RunReport {
        corpus_size: corpus,
        cases: 10,
        metrics: Metrics {
            recall_at_k: recall,
            ndcg_at_k: recall,
            evidence_precision: recall,
            groundedness: recall,
            skipped_cases: 0,
            total_cases: 10,
        },
        per_case_hit: Vec::new(),
        k: 10,
        fully_hit: (recall * 10.0).round() as usize,
    }
}

fn corpus() -> Vec<(String, String)> {
    vec![
        ("r1".to_string(), "限流阈值今天定在每秒两千次".to_string()),
        ("r2".to_string(), "今天天气不错适合出门散步".to_string()),
        (
            "r3".to_string(),
            "记忆索引换成了 sqlite 全文检索".to_string(),
        ),
    ]
}

// ============================================================
// ① 确定性（逐字节）
// ============================================================

#[test]
fn determinism_is_true_when_two_runs_agree() {
    let verdict = measure_determinism(&|| report(3, 0.8));
    assert!(verdict.identical, "同一输入两次跑批必须逐字节相同");
    assert_eq!(verdict.digest_a, verdict.digest_b);
    assert!(
        !verdict.digest_a.is_empty(),
        "摘要不能是空串（否则「相同」没有意义）"
    );
}

#[test]
fn determinism_compares_the_run_summary_byte_for_byte() {
    // ⚠️ **钉住「比的是哪段文本」**：判据 = `golden_run::summarize` 的输出。
    let source = report(3, 0.8);
    let verdict = measure_determinism(&|| source.clone());
    assert_eq!(verdict.digest_a, summarize(&source));
    assert_eq!(verdict.digest_b, summarize(&source));
}

#[test]
fn determinism_is_false_when_the_runner_oscillates() {
    // 一个**故意**不稳定的 runner（两次跑出不同分数）⇒ 判据必须抓到。
    let counter = Cell::new(0usize);
    let verdict = measure_determinism(&|| {
        let index = counter.get();
        counter.set(index + 1);
        report(3, if index % 2 == 0 { 0.8 } else { 0.7 })
    });
    assert!(!verdict.identical, "不稳定的 runner 不许被判成确定");
    assert_ne!(verdict.digest_a, verdict.digest_b);
}

#[test]
fn determinism_on_an_empty_run_report_does_not_panic() {
    let empty = RunReport {
        corpus_size: 0,
        cases: 0,
        metrics: Metrics::default(),
        per_case_hit: Vec::new(),
        k: 10,
        fully_hit: 0,
    };
    let verdict = measure_determinism(&|| empty.clone());
    assert!(verdict.identical);
}

// ============================================================
// ② 噪声鲁棒
// ============================================================

#[test]
fn noise_drop_ratio_is_relative_to_the_clean_score() {
    // 干净 0.8 → 掺噪 0.6 ⇒ 相对跌幅 = (0.8-0.6)/0.8 = **25%**（不是掉 0.2）。
    let verdict = measure_noise_robustness(&|| report(3, 0.8), &|| report(3, 0.6), 0.10);
    assert!((verdict.drop_ratio - 0.25).abs() < 1e-9, "{verdict:?}");
    assert!((verdict.clean_recall - 0.8).abs() < 1e-9);
    assert!((verdict.noisy_recall - 0.6).abs() < 1e-9);
}

#[test]
fn noise_survives_when_the_drop_is_within_the_threshold() {
    let verdict = measure_noise_robustness(&|| report(3, 1.0), &|| report(3, 0.95), 0.10);
    assert!(verdict.survived, "跌幅 5% ≤ 门限 10% ⇒ 扛住了：{verdict:?}");
}

#[test]
fn noise_fails_when_the_drop_exceeds_the_threshold() {
    let verdict = measure_noise_robustness(&|| report(3, 1.0), &|| report(3, 0.5), 0.10);
    assert!(
        !verdict.survived,
        "跌幅 50% > 门限 10% ⇒ 没扛住：{verdict:?}"
    );
    assert!((verdict.drop_ratio - 0.5).abs() < 1e-9);
}

#[test]
fn noise_with_zero_clean_score_does_not_divide_by_zero() {
    // 干净分就是 0 ⇒ **没有可掉的分** ⇒ 记 0.0（不许 NaN，也不许 panic）。
    let verdict = measure_noise_robustness(&|| report(3, 0.0), &|| report(3, 0.0), 0.10);
    assert_eq!(verdict.drop_ratio, 0.0);
    assert!(verdict.survived);
    assert!(verdict.drop_ratio.is_finite());
}

#[test]
fn noise_drop_is_clamped_to_zero_when_the_noisy_run_scores_higher() {
    // 掺噪后反而更高 ⇒ 跌幅是 **0**（不是负数）⇒ 不许把「变好」写成「变差」。
    let verdict = measure_noise_robustness(&|| report(3, 0.5), &|| report(3, 0.9), 0.10);
    assert_eq!(verdict.drop_ratio, 0.0);
    assert!(verdict.survived);
}

// ============================================================
// ③ 规模延展斜率
// ============================================================

#[test]
fn scale_slope_from_known_durations_is_exactly_linear() {
    // 耗时 ×4、规模 ×4 ⇒ 斜率 = 1.0（线性）。
    let slope = scale_slope_from_ms(100, 400, 100, 400);
    assert!((slope - 1.0).abs() < 1e-9, "{slope}");
}

#[test]
fn scale_slope_from_known_durations_flags_superlinear_growth() {
    // 耗时 ×8、规模 ×4 ⇒ 斜率 = 2.0（超线性 ⇒ 要警惕）。
    let slope = scale_slope_from_ms(100, 800, 100, 400);
    assert!((slope - 2.0).abs() < 1e-9, "{slope}");
    // 耗时 ×2、规模 ×4 ⇒ 斜率 = 0.5（亚线性 ⇒ 反而更划算）。
    let slope = scale_slope_from_ms(100, 200, 100, 400);
    assert!((slope - 0.5).abs() < 1e-9, "{slope}");
}

#[test]
fn measure_scale_slope_records_the_two_runs_and_a_trend() {
    // ⚠️ **墙钟** ⇒ 只断言「量级与趋势」，不拿它当精确断言依据。
    let small = || {
        std::thread::sleep(Duration::from_millis(30));
        report(10, 0.8)
    };
    let large = || {
        std::thread::sleep(Duration::from_millis(120));
        report(40, 0.8)
    };
    let verdict = measure_scale_slope(&small, &large, 10, 40);
    assert_eq!(verdict.small_n, 10);
    assert_eq!(verdict.large_n, 40);
    assert!(verdict.small_ms >= 1, "耗时不能记 0：{verdict:?}");
    assert!(
        verdict.large_ms >= verdict.small_ms,
        "大语料不该更快：{verdict:?}"
    );
    // 耗时 ×4、规模 ×4 ⇒ 斜率 ≈ 1.0（给抖动留余量）。
    assert!(
        (0.7..=1.4).contains(&verdict.slope),
        "斜率应接近线性（1.0）：{verdict:?}"
    );
}

// ============================================================
// ④ 越权命中率
// ============================================================

#[test]
fn overreach_passes_when_there_is_no_violation() {
    let verdict = measure_overreach(10, Vec::new());
    assert_eq!(verdict.hits, 0);
    assert!(verdict.passed);
    assert!(
        verdict.details.is_empty(),
        "没有越权 ⇒ 明细必须为空：{verdict:?}"
    );
}

#[test]
fn overreach_fails_and_the_details_locate_the_violation() {
    let verdict = measure_overreach(
        10,
        vec![
            "模式层把摘要写回了原始记录 novel-0007".to_string(),
            "模式层改了原始记录 novel-0031 的正文".to_string(),
        ],
    );
    assert_eq!(verdict.hits, 2);
    assert!(!verdict.passed, "有越权就不许判通过");
    assert_eq!(verdict.details.len(), 2);
    assert!(verdict
        .details
        .iter()
        .any(|line| line.contains("novel-0007")));
    assert!(verdict
        .details
        .iter()
        .any(|line| line.contains("novel-0031")));
}

#[test]
fn overreach_counts_violations_not_attempts() {
    // ⚠️ **命中数是「越权次数」**，不是「尝试次数」⇒ 尝试 10 次、越权 2 次 ⇒ 命中 2。
    let verdict = measure_overreach(10, vec!["越权一".to_string(), "越权二".to_string()]);
    assert_eq!(verdict.hits, 2);
    assert!(!verdict.passed);
}

// ============================================================
// ⑤ 噪声注入（确定性）
// ============================================================

#[test]
fn inject_noise_is_deterministic_for_the_same_seed() {
    let source = corpus();
    let first = inject_noise(&source, 0.5, 42);
    let second = inject_noise(&source, 0.5, 42);
    assert_eq!(first, second, "同种子 ⇒ 逐字相等（否则噪声实验不可复现）");
}

#[test]
fn inject_noise_with_rate_zero_leaves_the_corpus_untouched() {
    let source = corpus();
    let noisy = inject_noise(&source, 0.0, 7);
    assert_eq!(noisy, source, "rate = 0 ⇒ 原样不动（一个字都不许改）");
}

#[test]
fn inject_noise_with_rate_one_touches_every_record() {
    let source = corpus();
    let noisy = inject_noise(&source, 1.0, 7);
    assert_eq!(noisy.len(), source.len());
    for ((id, original), (noisy_id, text)) in source.iter().zip(noisy.iter()) {
        assert_eq!(id, noisy_id, "id 不许被噪声改掉");
        assert_ne!(original, text, "rate = 1 ⇒ 每条都要被掺噪声");
        // ⚠️ 噪声是「**插入**」⇒ 去掉那段噪声后必须**逐字**还原原文（原正文一字不少）。
        assert_eq!(
            text.replace(NOISE_FRAGMENT, ""),
            *original,
            "噪声只是插入 ⇒ 原文必须一字不少"
        );
    }
}

#[test]
fn inject_noise_keeps_the_record_count_and_the_ids() {
    let source = corpus();
    let noisy = inject_noise(&source, 0.5, 99);
    assert_eq!(noisy.len(), source.len());
    let source_ids: Vec<&String> = source.iter().map(|(id, _)| id).collect();
    let noisy_ids: Vec<&String> = noisy.iter().map(|(id, _)| id).collect();
    assert_eq!(source_ids, noisy_ids);
}

#[test]
fn different_seeds_usually_produce_different_noise() {
    // ⚠️ **概率性**：不同种子**通常**给出不同的噪声，但**不许**断言「必不同」
    // （样本少时可能撞上）⇒ 这里只断言「多种子里至少有两种不同」。
    let source = corpus();
    let mut outputs: Vec<Vec<(String, String)>> = Vec::new();
    for seed in 0..8u64 {
        outputs.push(inject_noise(&source, 0.9, seed));
    }
    let distinct = outputs
        .iter()
        .enumerate()
        .any(|(index, output)| outputs[..index].iter().any(|other| other != output));
    assert!(
        distinct,
        "种子必须真的影响噪声（否则「同种子可复现」是空话）"
    );
}

// ============================================================
// ⑥ 报告渲染
// ============================================================

#[test]
fn report_contains_all_four_items() {
    let determinism = DeterminismVerdict {
        identical: true,
        digest_a: "A".to_string(),
        digest_b: "A".to_string(),
    };
    let noise = NoiseVerdict {
        clean_recall: 0.8,
        noisy_recall: 0.78,
        drop_ratio: 0.025,
        survived: true,
    };
    let scale = ScaleVerdict {
        small_n: 100,
        large_n: 1000,
        small_ms: 100,
        large_ms: 400,
        slope: 1.0,
    };
    let overreach = OverreachVerdict {
        hits: 0,
        details: Vec::new(),
        passed: true,
    };
    let text = render_robustness_report(
        Some(&determinism),
        Some(&noise),
        Some(&scale),
        Some(&overreach),
    );
    assert!(text.contains("确定性"), "{text}");
    assert!(text.contains("噪声鲁棒"), "{text}");
    assert!(text.contains("规模延展"), "{text}");
    assert!(text.contains("越权"), "{text}");
    assert!(text.contains("恒 0"), "{text}");
}

#[test]
fn report_raises_the_alarm_when_overreach_is_not_zero() {
    let overreach = OverreachVerdict {
        hits: 2,
        details: vec![
            "模式层把摘要写回了 novel-0007".to_string(),
            "模式层改了 novel-0031".to_string(),
        ],
        passed: false,
    };
    let text = render_robustness_report(None, None, None, Some(&overreach));
    assert!(
        text.contains("事故"),
        "越权非 0 ⇒ 必须报警（不粉饰）：{text}"
    );
    assert!(text.contains("novel-0007"), "报警要能定位：{text}");
    assert!(text.contains("novel-0031"), "报警要能定位：{text}");
}

#[test]
fn report_marks_missing_measurements_as_not_measured() {
    let text = render_robustness_report(None, None, None, None);
    assert_eq!(
        text.matches("没测到").count(),
        4,
        "四项都没测 ⇒ 四处都要写明：{text}"
    );
    // ⚠️ 「没测到」**不许**写成「通过」⇒ 四项都没测时，报告里不许出现任何 ✓。
    assert!(!text.contains('✓'), "没测到 ≠ 通过：{text}");
}

#[test]
fn report_states_the_slope_is_a_trend_not_an_assertion_basis() {
    let scale = ScaleVerdict {
        small_n: 100,
        large_n: 1000,
        small_ms: 100,
        large_ms: 900,
        slope: 2.25,
    };
    let text = render_robustness_report(None, None, Some(&scale), None);
    assert!(
        text.contains("斜率是趋势，不是断言依据"),
        "墙钟口径必须标明：{text}"
    );
}

#[test]
fn report_flags_a_broken_determinism_and_a_collapsed_noise_score() {
    let determinism = DeterminismVerdict {
        identical: false,
        digest_a: "语料 3 条 · 题 10 道".to_string(),
        digest_b: "语料 4 条 · 题 10 道".to_string(),
    };
    let noise = NoiseVerdict {
        clean_recall: 0.8,
        noisy_recall: 0.2,
        drop_ratio: 0.75,
        survived: false,
    };
    let text = render_robustness_report(Some(&determinism), Some(&noise), None, None);
    assert!(text.contains("不一致"), "{text}");
    assert!(text.contains("没扛住"), "{text}");
}
