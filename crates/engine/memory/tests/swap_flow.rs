//! `swap_flow` 的测试：**算法切换流程**（影子双跑 → 裁决 → 切换 + 归档）+ `Swap cost` 闸。
//!
//! 重点不在「跑得动」，而在**三道闸不许被悄悄放水**：
//! * 裁决不过 ⇒ **什么都不做**（指针逐字不变 + **不落盘**）；
//! * **回退不比切换快 ⇒ 不许换**（「可回退」是假的 ⇒ 零风险上线是假的）；
//! * 影子样本不足 ⇒ 不许换（一轮只是运气，题太少结论没意义）；
//! * 切换/回退**都要留痕**（评测报告归档 ⇒ 事后可查「凭什么换的」）；
//! * 同一输入两次 ⇒ 归档**逐字相同**（可复现 ⇒ 可 diff ⇒ 可复核）。

use std::path::Path;

use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::shadow_run::{
    judge_swap, run_shadow, Retriever, RunOutcome, SwapDecision, SwapVerdict,
};
use apeireth_memory::swap_flow::{
    execute_swap, list_archives, rollback, AlgoPointer, SwapCost, SwapOutcome, SwapPlan,
    MIN_SHADOW_CASES, MIN_SHADOW_RUNS,
};
use tempfile::tempdir;

// ============================================
// 测试用的假件
// ============================================

/// 假检索器：**每 `hit_every` 题命中一道**（`1` = 全命中，`2` = 命中一半，`3` = 命中三分之一）。
///
/// # 为什么按题号命中而不是随机
///
/// 随机 ⇒ 分数会抖 ⇒ 裁决结果不可复现 ⇒ 测试没法钉死口径。
struct FakeRetriever {
    id: String,
    hit_every: usize,
    tokens: usize,
}

impl FakeRetriever {
    fn new(id: &str, hit_every: usize, tokens: usize) -> Self {
        FakeRetriever {
            id: id.to_string(),
            hit_every,
            tokens,
        }
    }
}

impl Retriever for FakeRetriever {
    fn id(&self) -> &str {
        &self.id
    }

    fn retrieve(&self, case: &GoldenCase) -> RunOutcome {
        let index = case
            .id
            .rsplit('-')
            .next()
            .and_then(|tail| tail.parse::<usize>().ok())
            .unwrap_or(0);
        let ids = if index % self.hit_every == 0 {
            case.expect_ids.iter().take(1).cloned().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        RunOutcome {
            ids,
            injected_tokens: self.tokens,
        }
    }
}

/// 一批题（每题都有标准答案 ⇒ 能进准确度分母）。
fn cases(count: usize) -> Vec<GoldenCase> {
    (0..count)
        .map(|i| GoldenCase {
            id: format!("case-{i:03}"),
            query: format!("关于第 {i} 个话题的问题"),
            expect_ids: vec![format!("doc-{i:03}")],
            expect_fragments: Vec::new(),
            tags: vec!["internal".to_string()],
        })
        .collect()
}

/// 初始指针（还没换过 ⇒ `previous = None`）。
fn initial_pointer(current: &str) -> AlgoPointer {
    AlgoPointer {
        current: current.to_string(),
        previous: None,
        switched_at_ms: 0,
        generation: 0,
    }
}

/// 手搓一份裁决（用于**边界**用例：裁决由 `judge_swap` 产出的用例另有其测试）。
fn verdict_with(decision: SwapDecision, reason: &str) -> SwapVerdict {
    SwapVerdict {
        decision,
        champion_score: Some(0.5),
        challenger_score: Some(0.5),
        champion_tokens: Some(1200),
        challenger_tokens: Some(960),
        reason: reason.to_string(),
    }
}

/// 一份**通过**的计划（样本充分：轮数 ≥ 2、题数 ≥ 10）。
fn ok_plan(from: &str, to: &str) -> SwapPlan {
    SwapPlan {
        from: from.to_string(),
        to: to.to_string(),
        verdict: verdict_with(
            SwapDecision::Approved,
            "准确度未劣化且 token 未增加 ⇒ 允许换",
        ),
        shadow_runs: 3,
        shadow_cases: 12,
    }
}

/// 读归档文件（**逐字** ⇒ 确定性用例要比字节）。
fn read_archive(path: &str) -> String {
    std::fs::read_to_string(path).expect("归档文件应可读")
}

// ============================================
// `Swap cost` 的闸
// ============================================

#[test]
fn allows_swap_true_for_approved_with_enough_samples_and_faster_rollback() {
    let cost = SwapCost {
        switch_ms: 50,
        rollback_ms: 5,
        shadow_runs: 3,
        shadow_cases: 12,
    };
    assert!(cost.allows_swap(SwapDecision::Approved));
    assert!(cost.allows_swap(SwapDecision::ApprovedWithGain));
}

#[test]
fn allows_swap_false_for_rejected() {
    let cost = SwapCost {
        switch_ms: 50,
        rollback_ms: 5,
        shadow_runs: 3,
        shadow_cases: 12,
    };
    assert!(!cost.allows_swap(SwapDecision::Rejected));
}

#[test]
fn allows_swap_false_when_rollback_is_not_faster() {
    // 相等：回退不比切换快 ⇒「可回退」是假的。
    let equal = SwapCost {
        switch_ms: 20,
        rollback_ms: 20,
        shadow_runs: 3,
        shadow_cases: 12,
    };
    assert!(!equal.allows_swap(SwapDecision::Approved));
    // 更慢：更不许。
    let slower = SwapCost {
        switch_ms: 20,
        rollback_ms: 21,
        shadow_runs: 3,
        shadow_cases: 12,
    };
    assert!(!slower.allows_swap(SwapDecision::Approved));
}

#[test]
fn allows_swap_false_for_single_shadow_run() {
    let cost = SwapCost {
        switch_ms: 50,
        rollback_ms: 5,
        shadow_runs: 1,
        shadow_cases: 12,
    };
    assert!(!cost.allows_swap(SwapDecision::Approved));
}

#[test]
fn allows_swap_false_for_too_few_shadow_cases() {
    let cost = SwapCost {
        switch_ms: 50,
        rollback_ms: 5,
        shadow_runs: 3,
        shadow_cases: MIN_SHADOW_CASES - 1,
    };
    assert!(!cost.allows_swap(SwapDecision::Approved));
}

#[test]
fn allows_swap_true_at_the_sample_boundary() {
    let cost = SwapCost {
        switch_ms: 50,
        rollback_ms: 5,
        shadow_runs: MIN_SHADOW_RUNS,
        shadow_cases: MIN_SHADOW_CASES,
    };
    assert!(cost.allows_swap(SwapDecision::Approved));
}

// ============================================
// `execute_swap`：不通过 ⇒ 什么都不做
// ============================================

#[test]
fn execute_swap_with_rejected_verdict_keeps_pointer_untouched() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    let before = pointer.clone();
    let mut plan = ok_plan("bm25", "hybrid");
    plan.verdict = verdict_with(SwapDecision::Rejected, "准确度劣化 12% ⇒ 不许换");

    let outcome = execute_swap(&plan, &mut pointer, dir.path(), 1_760_000_000_000)
        .expect("裁决不通过不是 IO 错误 ⇒ 应当 Ok");

    assert!(!outcome.switched, "Rejected ⇒ 绝不许切");
    assert_eq!(pointer, before, "指针必须逐字不变");
    assert_eq!(outcome.pointer, before);
    assert!(outcome.archive_path.is_empty(), "不落盘 ⇒ 路径为空");
    assert!(
        outcome.detail.contains("Rejected") && outcome.detail.contains("准确度劣化 12%"),
        "detail 必须写清是哪道闸 + 裁决原话：{}",
        outcome.detail
    );
    assert!(
        list_archives(dir.path()).expect("列目录").is_empty(),
        "不通过 ⇒ 目录里不许多出文件"
    );
}

#[test]
fn execute_swap_with_insufficient_shadow_runs_does_not_switch() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    let before = pointer.clone();
    let mut plan = ok_plan("bm25", "hybrid");
    plan.shadow_runs = 1;

    let outcome = execute_swap(&plan, &mut pointer, dir.path(), 1_760_000_000_000)
        .expect("样本不足不是 IO 错误 ⇒ 应当 Ok");

    assert!(!outcome.switched);
    assert_eq!(pointer, before);
    assert!(
        outcome.detail.contains("影子"),
        "detail 要说明样本不足：{}",
        outcome.detail
    );
    assert!(list_archives(dir.path()).expect("列目录").is_empty());
}

#[test]
fn execute_swap_with_missing_archive_dir_is_err() {
    let dir = tempdir().expect("temp dir");
    let missing = dir.path().join("no-such-dir");
    let mut pointer = initial_pointer("bm25");
    let before = pointer.clone();

    let err = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        &missing,
        1_760_000_000_000,
    )
    .expect_err("归档目录不存在 ⇒ 必须 Err（不许 panic、不许静默跳过归档）");

    assert!(
        err.contains("归档目录"),
        "错误要说清是归档目录的问题：{err}"
    );
    assert_eq!(pointer, before, "出错 ⇒ 指针不动");
}

#[test]
fn execute_swap_with_stale_plan_is_err() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    let before = pointer.clone();
    // 计划说从 "old" 换，但现役是 "bm25" ⇒ 影子比的不是现在在用的那个 ⇒ 计划过期。
    let plan = ok_plan("old", "hybrid");

    let err = execute_swap(&plan, &mut pointer, dir.path(), 1_760_000_000_000)
        .expect_err("计划与指针对不上 ⇒ 必须 Err");

    assert!(err.contains("过期"), "错误要说清是计划过期：{err}");
    assert_eq!(pointer, before, "出错 ⇒ 指针不动");
}

// ============================================
// `execute_swap`：通过 ⇒ 切指针 + 归档
// ============================================

#[test]
fn execute_swap_updates_pointer_fields() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");

    let outcome = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_123_456,
    )
    .expect("应当切换成功");

    assert!(outcome.switched);
    assert_eq!(pointer.current, "hybrid");
    assert_eq!(
        pointer.previous,
        Some("bm25".to_string()),
        "必须记下回退目标"
    );
    assert_eq!(pointer.generation, 1, "切换序号要 +1");
    assert_eq!(
        pointer.switched_at_ms, 1_760_000_123_456,
        "切换时刻用注入值"
    );
    assert_eq!(outcome.pointer, pointer);
    assert!(!outcome.archive_path.is_empty());
    assert!(
        Path::new(&outcome.archive_path).exists(),
        "归档文件必须真的落盘"
    );
}

#[test]
fn execute_swap_writes_archive_containing_verdict_reason() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    let mut plan = ok_plan("bm25", "hybrid");
    plan.verdict = verdict_with(SwapDecision::ApprovedWithGain, "准确度提升 0.1250 ⇒ 允许换");

    let outcome =
        execute_swap(&plan, &mut pointer, dir.path(), 1_760_000_000_000).expect("应当切换成功");

    let body = read_archive(&outcome.archive_path);
    assert!(
        body.contains("准确度提升 0.1250"),
        "归档必须含裁决理由（事后可查「凭什么换的」）：{body}"
    );
    let json: serde_json::Value = serde_json::from_str(&body).expect("归档必须是合法 JSON");
    assert_eq!(json["kind"], "swap");
    assert_eq!(json["verdict"]["decision"], "ApprovedWithGain");
    assert_eq!(json["plan"]["from"], "bm25");
    assert_eq!(json["plan"]["to"], "hybrid");
    assert_eq!(json["plan"]["shadow_runs"], 3);
    assert_eq!(json["plan"]["shadow_cases"], 12);
    assert_eq!(json["pointer_after"]["current"], "hybrid");
    assert_eq!(json["pointer_before"]["current"], "bm25");
    assert!(
        json["cost"]["switch_ms"].as_u64().unwrap() > json["cost"]["rollback_ms"].as_u64().unwrap()
    );
}

#[test]
fn execute_swap_twice_keeps_both_archives() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");

    let first = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_000_000,
    )
    .expect("第一次切换");
    let second = execute_swap(
        &ok_plan("hybrid", "hybrid-v2"),
        &mut pointer,
        dir.path(),
        1_760_000_001_000,
    )
    .expect("第二次切换");

    assert_ne!(
        first.archive_path, second.archive_path,
        "两次切换不许互相覆盖"
    );
    assert!(Path::new(&first.archive_path).exists());
    assert!(Path::new(&second.archive_path).exists());
    let names = list_archives(dir.path()).expect("列目录");
    assert_eq!(names.len(), 2, "两次切换 ⇒ 两份归档：{names:?}");
    assert_eq!(pointer.generation, 2);
}

#[test]
fn list_archives_returns_names_sorted() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    // 时间戳乱序写入 ⇒ 列目录必须按**文件名序**（⇒ 顺序确定，与写入顺序无关）。
    execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_003_000,
    )
    .expect("第一次切换");
    execute_swap(
        &ok_plan("hybrid", "hybrid-v2"),
        &mut pointer,
        dir.path(),
        1_760_000_001_000,
    )
    .expect("第二次切换");
    execute_swap(
        &ok_plan("hybrid-v2", "hybrid-v3"),
        &mut pointer,
        dir.path(),
        1_760_000_002_000,
    )
    .expect("第三次切换");

    let names = list_archives(dir.path()).expect("列目录");
    assert_eq!(names.len(), 3);
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "必须按文件名序返回（确定性）");
    assert!(
        names[0].contains("1760000001000"),
        "时间戳序 = 时序：{names:?}"
    );
    assert!(
        names[2].contains("1760000003000"),
        "时间戳序 = 时序：{names:?}"
    );
}

#[test]
fn list_archives_on_missing_dir_is_err() {
    let dir = tempdir().expect("temp dir");
    let missing = dir.path().join("nope");
    assert!(
        list_archives(&missing).is_err(),
        "目录不存在 ⇒ Err（不 panic）"
    );
}

// ============================================
// `rollback`
// ============================================

#[test]
fn rollback_without_previous_is_err() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    let before = pointer.clone();

    let err = rollback(&mut pointer, dir.path(), 1_760_000_000_000)
        .expect_err("没换过 ⇒ 没有回退目标 ⇒ 必须 Err（不假装成功）");

    assert!(err.contains("回退"), "错误要说清是没得回退：{err}");
    assert_eq!(pointer, before, "出错 ⇒ 指针不动");
    assert!(list_archives(dir.path()).expect("列目录").is_empty());
}

#[test]
fn rollback_swaps_current_and_previous_and_keeps_generation() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_000_000,
    )
    .expect("先切一次");
    assert_eq!(pointer.generation, 1);

    let outcome = rollback(&mut pointer, dir.path(), 1_760_000_009_000).expect("应当回退成功");

    assert!(outcome.switched);
    assert_eq!(pointer.current, "bm25", "回到上一个算法");
    assert_eq!(
        pointer.previous,
        Some("hybrid".to_string()),
        "对调 ⇒ 再回退一次能回去"
    );
    assert_eq!(pointer.generation, 1, "回退是撤销 ⇒ generation 不变");
    assert_eq!(pointer.switched_at_ms, 1_760_000_009_000);
}

#[test]
fn rollback_archives_report_with_kind_rollback() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_000_000,
    )
    .expect("先切一次");

    let outcome = rollback(&mut pointer, dir.path(), 1_760_000_009_000).expect("应当回退成功");

    let body = read_archive(&outcome.archive_path);
    let json: serde_json::Value = serde_json::from_str(&body).expect("归档必须是合法 JSON");
    assert_eq!(json["kind"], "rollback", "回退也要留痕（kind = rollback）");
    assert_eq!(json["plan"]["from"], "hybrid");
    assert_eq!(json["plan"]["to"], "bm25");
    assert!(
        json["verdict"].is_null(),
        "回退没有裁决 ⇒ 显式 null（不假装有裁决）"
    );
    let names = list_archives(dir.path()).expect("列目录");
    assert_eq!(names.len(), 2, "一次切换 + 一次回退 ⇒ 两份归档：{names:?}");
}

#[test]
fn rollback_twice_returns_to_the_original_algorithm() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_000_000,
    )
    .expect("先切一次");

    rollback(&mut pointer, dir.path(), 1_760_000_009_000).expect("第一次回退");
    assert_eq!(pointer.current, "bm25");
    rollback(&mut pointer, dir.path(), 1_760_000_009_500).expect("第二次回退");

    assert_eq!(
        pointer.current, "hybrid",
        "对调 ⇒ 再回退一次能回去（不丢历史）"
    );
    assert_eq!(pointer.previous, Some("bm25".to_string()));
    assert_eq!(pointer.generation, 1, "回退不增加 generation");
}

// ============================================
// 确定性
// ============================================

#[test]
fn archive_is_byte_identical_for_same_input() {
    let dir_a = tempdir().expect("temp dir");
    let dir_b = tempdir().expect("temp dir");
    let mut pointer_a = initial_pointer("bm25");
    let mut pointer_b = initial_pointer("bm25");

    let outcome_a = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer_a,
        dir_a.path(),
        1_760_000_000_000,
    )
    .expect("第一次切换");
    let outcome_b = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer_b,
        dir_b.path(),
        1_760_000_000_000,
    )
    .expect("第二次切换");

    let name_a = Path::new(&outcome_a.archive_path)
        .file_name()
        .expect("文件名")
        .to_string_lossy()
        .into_owned();
    let name_b = Path::new(&outcome_b.archive_path)
        .file_name()
        .expect("文件名")
        .to_string_lossy()
        .into_owned();
    assert_eq!(name_a, name_b, "同一输入 ⇒ 归档文件名逐字相等");
    assert_eq!(
        read_archive(&outcome_a.archive_path),
        read_archive(&outcome_b.archive_path),
        "同一输入 ⇒ 归档内容逐字相等（可 diff ⇒ 可复核）"
    );
}

#[test]
fn archive_in_same_dir_is_stable_across_repeated_runs() {
    let dir = tempdir().expect("temp dir");
    let mut pointer = initial_pointer("bm25");
    let first = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer,
        dir.path(),
        1_760_000_000_000,
    )
    .expect("第一次切换");
    let body_first = read_archive(&first.archive_path);

    // 同一个目录里再跑一次 ⇒ 文件名相同、内容相同 ⇒ 不产生第二份报告（也不矛盾）。
    let mut pointer2 = initial_pointer("bm25");
    let second = execute_swap(
        &ok_plan("bm25", "hybrid"),
        &mut pointer2,
        dir.path(),
        1_760_000_000_000,
    )
    .expect("第二次切换");

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(body_first, read_archive(&second.archive_path));
    assert_eq!(list_archives(dir.path()).expect("列目录").len(), 1);
}

// ============================================
// 三步流程整合（影子双跑 → 裁决 → 切换 + 归档）
// ============================================

#[test]
fn full_flow_shadow_judge_then_switch_and_archive() {
    let dir = tempdir().expect("temp dir");
    let cases = cases(12);
    let champion = FakeRetriever::new("bm25", 2, 100);
    let challenger = FakeRetriever::new("hybrid", 1, 80);

    // ① 影子双跑（结果不外泄 ⇒ 只比对，不返回给用户）。
    let shadow = run_shadow(&cases, &champion, &challenger);
    assert_eq!(shadow.pairs.len(), 12);
    assert_eq!(shadow.champion_id, "bm25");
    assert_eq!(shadow.challenger_id, "hybrid");

    // ② 裁决（比中位数 ⇒ 跑 3 轮）。
    let verdict = judge_swap(&champion, &challenger, &cases, 3);
    assert_eq!(
        verdict.decision,
        SwapDecision::ApprovedWithGain,
        "候选全命中且更省 token ⇒ 允许换并说明提升：{}",
        verdict.reason
    );

    // ③ 切换 + 归档。
    let plan = SwapPlan {
        from: "bm25".to_string(),
        to: "hybrid".to_string(),
        verdict,
        shadow_runs: 3,
        shadow_cases: cases.len(),
    };
    let mut pointer = initial_pointer("bm25");
    let outcome: SwapOutcome =
        execute_swap(&plan, &mut pointer, dir.path(), 1_760_000_000_000).expect("应当切换成功");

    assert!(outcome.switched);
    assert_eq!(pointer.current, "hybrid");
    assert_eq!(pointer.previous, Some("bm25".to_string()));
    assert!(
        outcome.cost.rollback_ms < outcome.cost.switch_ms,
        "回退必须比切换快"
    );
    assert!(outcome.cost.shadow_runs >= MIN_SHADOW_RUNS);
    assert!(outcome.cost.shadow_cases >= MIN_SHADOW_CASES);

    let body = read_archive(&outcome.archive_path);
    assert!(body.contains("ApprovedWithGain"));
    assert!(body.contains(&outcome.detail), "归档要含人话说明");
    assert_eq!(list_archives(dir.path()).expect("列目录").len(), 1);
}

#[test]
fn full_flow_rejected_challenger_leaves_production_untouched() {
    let dir = tempdir().expect("temp dir");
    let cases = cases(12);
    let champion = FakeRetriever::new("bm25", 1, 100);
    // 候选只命中三分之一 ⇒ 准确度掉 ⇒ 裁决拒绝 ⇒ 生产一个字节都不许动。
    let challenger = FakeRetriever::new("hybrid", 3, 60);

    let verdict = judge_swap(&champion, &challenger, &cases, 3);
    assert_eq!(verdict.decision, SwapDecision::Rejected);

    let plan = SwapPlan {
        from: "bm25".to_string(),
        to: "hybrid".to_string(),
        verdict,
        shadow_runs: 3,
        shadow_cases: cases.len(),
    };
    let mut pointer = initial_pointer("bm25");
    let before = pointer.clone();
    let outcome = execute_swap(&plan, &mut pointer, dir.path(), 1_760_000_000_000)
        .expect("拒绝不是 IO 错误 ⇒ 应当 Ok");

    assert!(!outcome.switched);
    assert_eq!(pointer, before);
    assert!(list_archives(dir.path()).expect("列目录").is_empty());
}
