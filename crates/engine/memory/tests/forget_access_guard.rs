//! 「**先记访问史，再忘记**」的顺序纪律契约测试。
//!
//! # 这批钉住什么
//!
//! * **空回执不是通行证**：`record` 对空事件批次返回 `None` ⇒ 拿不到令牌。
//! * **令牌留痕**：令牌里带回执的事件 ⇒ 事后可查「销毁前记了什么」。
//! * **覆盖判定从严**：3 个目标缺 1 个 ⇒ 必须报出那 1 个（**不许「大部分覆盖就算」**）。
//! * **顺序不可绕过**（本批核心）：没有访问史 ⇒ **拿不到** `ForgetPermit`
//!   ⇒ 假 forgets 入口**无法**执行销毁；记了 ⇒ 令牌到手 ⇒ 可执行。
//!
//! # 状态口径
//!
//! **NOT IMPLEMENTED（新增未接线）**：本守卫**尚未**接进生产 forgets 入口
//! （`universal_forget` / `memory_governance` / `forget_coordinator`）⇒
//! 这里的「假 forgets 入口」是**本模块内的替身**，用于证明顺序纪律**本身**成立，
//! **不代表**生产路径已经有这道闸门。

use apeireth_memory::access_history::AccessEvent;
use apeireth_memory::forget_access_guard::{
    missing_access_cover, AccessGuard, AccessReceipt, ForgetPermit,
};

/// 造一条访问事件（字段照 `access_history::AccessEvent` 的真实定义）。
fn event(memory_id: &str, accessed_at_ms: i64) -> AccessEvent {
    AccessEvent {
        id: format!("evt-{memory_id}-{accessed_at_ms}"),
        memory_id: memory_id.to_owned(),
        subject_id: Some("user-1".to_owned()),
        session_id: Some("session-1".to_owned()),
        accessed_at_ms,
        query: None,
        access_kind: "recall".to_owned(),
        rank: Some(0),
        score: Some(0.5),
        metadata: serde_json::json!({"origin": "forget_access_guard_test"}),
    }
}

fn ids(events: &[AccessEvent]) -> Vec<String> {
    events.iter().map(|e| e.memory_id.clone()).collect()
}

/// **假 forgets 入口**：只接受「访问史守卫签发的令牌」。
///
/// # 为什么这么写
///
/// 令牌不可从外部手工构造（私有签发标记）⇒ 走到这里的令牌**只可能**来自
/// `AccessGuard::record` → `AccessGuard::permit` ⇒ 「先记访问史」被钉成顺序纪律，
/// 而不是靠人记得调。
#[derive(Debug, Default)]
struct FakeForgetSink {
    forgotten: Vec<String>,
}

impl FakeForgetSink {
    /// 执行「降权/归档/销毁」——必须出示令牌。
    fn forget(&mut self, permit: ForgetPermit, targets: &[String]) -> Result<(), String> {
        assert!(permit.is_issued(), "令牌必须由守卫签发");
        let missing = missing_access_cover(&permit.events, targets);
        if !missing.is_empty() {
            return Err(format!("访问史未覆盖待忘记对象: {missing:?}"));
        }
        self.forgotten.extend(targets.iter().cloned());
        Ok(())
    }
}

// ============================================
// AccessGuard::record —— 空批次不是通行证
// ============================================

#[test]
fn record_with_empty_events_returns_none() {
    let mut guard = AccessGuard::new(1_000);
    let receipt = guard.record(Vec::new());
    assert!(
        receipt.is_none(),
        "空访问批次必须返回 None: 不许拿空回执当通行证"
    );
}

#[test]
fn empty_record_does_not_increase_recorded_count() {
    let mut guard = AccessGuard::new(1_000);
    assert_eq!(guard.recorded_count(), 0);
    assert!(guard.record(Vec::new()).is_none());
    assert_eq!(
        guard.recorded_count(),
        0,
        "一次空的 record 不是一次访问 ⇒ 不许记账"
    );
}

#[test]
fn record_keeps_events_verbatim_in_the_receipt() {
    let mut guard = AccessGuard::new(2_000);
    let first = event("mem-a", 100);
    let second = event("mem-b", 200);
    let receipt = guard
        .record(vec![first.clone(), second.clone()])
        .expect("非空批次必须给出回执");
    assert_eq!(
        receipt.events,
        vec![first, second],
        "回执里事件必须逐条一致"
    );
    assert_eq!(receipt.recorded_at_ms, 2_000, "回执带注入的记录时刻");
    assert!(receipt.is_issued(), "回执必须自证由守卫签发");
}

// ============================================
// AccessGuard::permit —— 令牌留痕
// ============================================

#[test]
fn permit_carries_the_receipt_events_and_timestamp() {
    let mut guard = AccessGuard::new(3_000);
    let recorded = event("mem-a", 100);
    let receipt = guard
        .record(vec![recorded.clone()])
        .expect("非空批次必须给出回执");
    let permit = guard.permit(receipt);
    assert_eq!(
        permit.events,
        vec![recorded],
        "令牌必须带回执里的事件（留痕可查）"
    );
    assert_eq!(permit.recorded_at_ms, 3_000);
    assert!(permit.is_issued(), "令牌必须自证由守卫签发");
}

#[test]
fn recorded_count_accumulates_across_batches() {
    let mut guard = AccessGuard::new(4_000);
    guard
        .record(vec![event("mem-a", 1), event("mem-b", 2)])
        .expect("批次一");
    assert_eq!(guard.recorded_count(), 2);
    guard.record(vec![event("mem-c", 3)]).expect("批次二");
    assert_eq!(
        guard.recorded_count(),
        3,
        "已记录事件总数必须跨批次累加（审计口径）"
    );
}

// ============================================
// missing_access_cover —— 覆盖判定从严
// ============================================

#[test]
fn full_coverage_yields_no_missing_target() {
    let events = vec![event("mem-a", 1), event("mem-b", 2)];
    let targets = vec!["mem-a".to_owned(), "mem-b".to_owned()];
    let missing = missing_access_cover(&events, &targets);
    assert!(
        missing.is_empty(),
        "全覆盖时必须返回空 (实际返回 {missing:?})"
    );
}

#[test]
fn one_missing_target_is_reported() {
    let events = vec![event("mem-a", 1), event("mem-b", 2)];
    let targets = vec!["mem-a".to_owned(), "mem-c".to_owned()];
    let missing = missing_access_cover(&events, &targets);
    assert_eq!(missing, vec!["mem-c".to_owned()], "必须精确报出缺的那一个");
}

#[test]
fn all_missing_targets_are_reported() {
    let events = vec![event("mem-a", 1)];
    let targets = vec!["mem-x".to_owned(), "mem-y".to_owned(), "mem-z".to_owned()];
    let missing = missing_access_cover(&events, &targets);
    assert_eq!(
        missing,
        vec!["mem-x".to_owned(), "mem-y".to_owned(), "mem-z".to_owned()],
        "全缺时必须全列出"
    );
}

#[test]
fn partial_coverage_is_not_accepted_as_coverage() {
    // 3 个目标缺 1 个 ⇒ **必须**返回那 1 个 (不许返回空 ⇒ 不许「大部分覆盖就算」)。
    let events = vec![event("mem-a", 1), event("mem-b", 2)];
    let targets = vec!["mem-a".to_owned(), "mem-b".to_owned(), "mem-c".to_owned()];
    let missing = missing_access_cover(&events, &targets);
    assert_eq!(
        missing,
        vec!["mem-c".to_owned()],
        "缺一个就不算覆盖: 不许返回空"
    );
    assert!(
        !missing.is_empty(),
        "3 缺 1 必须判为不覆盖（部分覆盖不算覆盖）"
    );
}

#[test]
fn empty_target_list_is_empty_and_does_not_panic() {
    let events = vec![event("mem-a", 1)];
    let missing = missing_access_cover(&events, &[]);
    assert!(missing.is_empty(), "空目标列表必须返回空");
}

#[test]
fn empty_event_list_covers_nothing() {
    let targets = vec!["mem-a".to_owned()];
    let missing = missing_access_cover(&[], &targets);
    assert_eq!(missing, targets, "没有访问事件 ⇒ 一个目标都覆盖不了");
}

#[test]
fn duplicate_targets_are_reported_once() {
    let events = vec![event("mem-a", 1)];
    let targets = vec!["mem-x".to_owned(), "mem-x".to_owned()];
    let missing = missing_access_cover(&events, &targets);
    assert_eq!(
        missing,
        vec!["mem-x".to_owned()],
        "重复的缺失目标只报一次（集合语义不变）"
    );
}

// ============================================
// 端到端：顺序纪律不可绕过（本批核心断言）
// ============================================

#[test]
fn forget_without_recorded_access_history_is_impossible() {
    let mut guard = AccessGuard::new(5_000);
    let mut sink = FakeForgetSink::default();
    let targets = vec!["mem-a".to_owned()];

    // 没记访问史 ⇒ 守卫不给回执 ⇒ **拿不到令牌** ⇒ 无法执行 forgets。
    let receipt: Option<AccessReceipt> = guard.record(Vec::new());
    assert!(receipt.is_none(), "没访问过 ⇒ 没有回执 ⇒ 拿不到令牌");

    // 令牌只能由守卫签发 ⇒ 假 forgets 入口只在拿到令牌时才可能执行。
    let outcome = match receipt {
        Some(receipt) => sink.forget(guard.permit(receipt), &targets),
        None => Err("no permit: 访问史未记录".to_owned()),
    };
    assert!(outcome.is_err(), "没记访问史就不许执行 forgets");
    assert!(
        sink.forgotten.is_empty(),
        "假 forgets 入口必须什么都没做 (实际 {:?})",
        sink.forgotten
    );
}

#[test]
fn forget_executes_once_access_history_is_recorded() {
    let mut guard = AccessGuard::new(6_000);
    let mut sink = FakeForgetSink::default();
    let targets = vec!["mem-a".to_owned(), "mem-b".to_owned()];

    let receipt = guard
        .record(vec![event("mem-a", 10), event("mem-b", 20)])
        .expect("记了访问史 ⇒ 有回执");
    let permit = guard.permit(receipt);
    sink.forget(permit, &targets)
        .expect("记了访问史 ⇒ 可以执行 forgets");

    assert_eq!(sink.forgotten, targets, "forgets 必须真的执行了");
}

#[test]
fn forget_is_rejected_when_access_history_does_not_cover_the_target() {
    let mut guard = AccessGuard::new(7_000);
    let mut sink = FakeForgetSink::default();
    let targets = vec!["mem-a".to_owned(), "mem-z".to_owned()];

    let receipt = guard
        .record(vec![event("mem-a", 10)])
        .expect("记了访问史 ⇒ 有回执");
    let permit = guard.permit(receipt);
    let outcome = sink.forget(permit, &targets);

    assert!(
        outcome.is_err(),
        "访问史未覆盖目标 ⇒ 必须拒绝 (实际 {outcome:?})"
    );
    assert!(
        sink.forgotten.is_empty(),
        "被拒绝的 forgets 不得产生任何副作用"
    );
}

#[test]
fn permit_can_be_minted_only_through_record_then_permit() {
    // 「顺序不可绕过」的形态断言: 令牌**只**来自 record → permit 这一条路径。
    let mut guard = AccessGuard::new(8_000);
    assert_eq!(guard.recorded_count(), 0);

    let receipt = guard
        .record(vec![event("mem-a", 10)])
        .expect("记了访问史 ⇒ 有回执");
    let permit = guard.permit(receipt);
    assert_eq!(guard.recorded_count(), 1, "记账只发生在 record 这一步");
    assert!(permit.is_issued());
    assert_eq!(ids(&permit.events), vec!["mem-a"]);
}
