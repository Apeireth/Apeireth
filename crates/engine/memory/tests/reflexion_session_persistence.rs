//! ⭐ **失败会话键真落盘**（2026-11-06 根治）的验收。
//!
//! # 缺口（已实测）
//! `FailureRecord.session_key` 字段在，但**四个落盘点全都写 `None`**
//! ⇒ ⇒ 结算侧 [`observe_failures_since_in_session`] 的会话过滤
//! ⇒ ⇒ **把所有失败都跳过** ⇒ ⇒ 「失败 → 学习桥」= **永远 0 证据 = 静默失效**。
//!
//! # 本批钉住的四件事
//! ① ⭐ **「重启后仍能结算」**（**验收核心**）：
//!    用 `FileReflexionStore` 写入**带 `session_key`** 的失败
//!    ⇒ ⇒ **重新打开同一个文件**（模拟重启）⇒ ⇒ 再结算 ⇒ ⇒ **拿到证据**;
//! ② **默认实现不落 `session_key`**（不许撒谎）：默认实现下**行为等于旧方法**;
//! ③ **老 JSON 文件（没有 `session_key` 字段）读得开**、不报错、`session_key == None`;
//! ④ **老数据**（没有会话键的失败）在会话模式下**被跳过**、全局模式下**被收**。
//!
//! ⚠️ **诚实口径**：默认实现**不落**会话键 ⇒ ⇒ 断言的是「**没落**」，
//! **不是**「假装落了」⇒ ⇒ 想要真落盘必须**覆写**（内存版 / 文件版已覆写）。

use apeireth_memory::reflexion::{
    FailureAttribution, FailureKind, FileReflexionStore, InMemoryReflexionStore, ReflexionError,
    ReflexionStore,
};
use apeireth_memory::reflexion_tuning::{
    observe_failures_since, observe_failures_since_in_session, ReflexionTuningCursor,
};
use apeireth_memory::retrieval_tuning::{
    RetrievalDimension, RetrievalTuningEngine, DIMENSION_STEP,
};

/// 测试用会话键（桶键口径）。
const SESSION_A: &str = "session-persist-a";
const SESSION_B: &str = "session-persist-b";

fn attr(memory_id: &str, dimension: Option<RetrievalDimension>) -> FailureAttribution {
    FailureAttribution {
        memory_id: memory_id.to_string(),
        dimension,
    }
}

/// 一个**不覆写**新方法（走 trait 默认实现）的 store。
struct BareStore {
    inner: InMemoryReflexionStore,
}

impl ReflexionStore for BareStore {
    fn record_failure(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
    ) -> Result<apeireth_memory::reflexion::FailureRecord, ReflexionError> {
        self.inner
            .record_failure(kind, task_type, summary, timestamp_epoch_ms)
    }

    fn process_unreflected(
        &self,
        critic: &dyn apeireth_memory::reflexion::Critic,
        timestamp_epoch_ms: i64,
    ) -> Result<usize, ReflexionError> {
        self.inner.process_unreflected(critic, timestamp_epoch_ms)
    }

    fn list_failures(
        &self,
    ) -> Result<Vec<apeireth_memory::reflexion::FailureRecord>, ReflexionError> {
        self.inner.list_failures()
    }

    fn list_reflections(
        &self,
    ) -> Result<Vec<apeireth_memory::reflexion::ReflectionText>, ReflexionError> {
        self.inner.list_reflections()
    }

    fn retry_injection(
        &self,
        task_type: &str,
        budget_chars: usize,
    ) -> Result<Option<String>, ReflexionError> {
        self.inner.retry_injection(task_type, budget_chars)
    }
}

/// ① ⭐ **验收核心**：「重启后仍能结算」。
///
/// # 「重启」怎么模拟
/// 进程内记账（接线层的 `owned_failures`）**重启后就没了**
/// ⇒ ⇒ 所以「重启」= **换一个全新的 store 实例读同一个文件**
/// ⇒ ⇒ 若结算仍能拿到证据 ⇒ ⇒ 证据**只能**来自**落盘的 `session_key`**
/// ⇒ ⇒ 这就是「真落盘」的证明。
#[test]
fn session_key_survives_restart_and_failures_still_settle() {
    let dir = tempfile::tempdir().expect("临时目录");
    let root = dir.path().join("reflexion");

    // ── 写（进程 1）────────────────────────────────────────────
    {
        let store = FileReflexionStore::new(root.clone());
        let record = store
            .record_failure_with_attribution_and_session(
                FailureKind::DecisionRejected,
                "deploy",
                "部署失败: 约束冲突",
                2_000,
                vec![attr("mem-1", Some(RetrievalDimension::Semantic))],
                Some(SESSION_A),
            )
            .expect("失败登记");
        assert_eq!(
            record.session_key.as_deref(),
            Some(SESSION_A),
            "返回值必须带会话键（不许撒谎）"
        );

        // ⚠️ 断言**真 store 里读回来的**（不是返回值）。
        let stored = store.list_failures().expect("读失败轨迹");
        assert_eq!(stored.len(), 1, "失败必须**真的落进真 store**");
        assert_eq!(
            stored[0].session_key.as_deref(),
            Some(SESSION_A),
            "会话键必须**真的落进真 store**（只断言返回值会假绿）"
        );
    }

    // ── 「重启」：全新实例读同一个文件（进程内状态全丢）──────────
    let reopened = FileReflexionStore::new(root.clone());
    let after_restart = reopened.list_failures().expect("重启后读失败轨迹");
    assert_eq!(
        after_restart[0].session_key.as_deref(),
        Some(SESSION_A),
        "⭐ **重启后会话键必须还在**（落盘了 ⇒ 才算根治；只在返回值里 = 没根治）"
    );

    // ⚠️ **落盘文件里真的有这个字段**（不是内存里有的）。
    let raw = std::fs::read_to_string(root.join("reflexions.json")).expect("读原始 JSON");
    assert!(
        raw.contains("session_key"),
        "落盘 JSON 必须含 `session_key` 字段: {raw}"
    );

    // ── 结算：本会话**拿得到证据** ─────────────────────────────
    let mut engine = RetrievalTuningEngine::new();
    let mut cursor = ReflexionTuningCursor::default();
    let settled =
        observe_failures_since_in_session(&reopened, &mut engine, &mut cursor, Some(SESSION_A))
            .expect("结算");
    assert_eq!(
        settled, 1,
        "⭐ **重启后仍能结算**（落盘会话键 ⇒ 会话过滤不再把这条跳过）"
    );

    // ⚠️ **权重真的退**（不是「函数被调用了」）。
    let baseline = apeireth_memory::MemoryRankingConfig::default();
    let after = engine.ranking();
    assert!(
        (after.semantic_weight - (baseline.semantic_weight - DIMENSION_STEP)).abs() < 1e-9,
        "语义维权重必须退一步（基线 {} ⇒ 实得 {}）",
        baseline.semantic_weight,
        after.semantic_weight
    );
}

/// ①-补 **重启后仍然隔离**：别的会话**不许**结算走这条失败。
#[test]
fn session_isolation_survives_restart() {
    let dir = tempfile::tempdir().expect("临时目录");
    let root = dir.path().join("reflexion");
    {
        let store = FileReflexionStore::new(root.clone());
        store
            .record_failure_with_attribution_and_session(
                FailureKind::DecisionRejected,
                "deploy",
                "A 的失败",
                2_000,
                vec![attr("mem-a", Some(RetrievalDimension::Semantic))],
                Some(SESSION_A),
            )
            .expect("失败登记");
    }

    let reopened = FileReflexionStore::new(root.clone());
    let mut engine = RetrievalTuningEngine::new();
    let mut cursor = ReflexionTuningCursor::default();
    let settled_b =
        observe_failures_since_in_session(&reopened, &mut engine, &mut cursor, Some(SESSION_B))
            .expect("B 结算");
    assert_eq!(
        settled_b, 0,
        "⭐ **B 会话不许结算 A 的失败**（落盘会话键 ⇒ 跨会话串扰被真隔离）"
    );
    let baseline = apeireth_memory::MemoryRankingConfig::default();
    assert!(
        (engine.ranking().semantic_weight - baseline.semantic_weight).abs() < 1e-12,
        "B 的权重不许被 A 的失败改"
    );
}

/// ② **默认实现不落 `session_key`**（不许撒谎）。
///
/// ⚠️ 断言的是「**没落**」⇒ ⇒ 默认实现**行为等于旧方法**
/// ⇒ ⇒ 未覆写的 store **不会**让桥假装学得到。
#[test]
fn default_trait_impl_does_not_persist_session_key() {
    let store = BareStore {
        inner: InMemoryReflexionStore::new(),
    };
    let returned = store
        .record_failure_with_attribution_and_session(
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败",
            2_000,
            vec![attr("mem-1", Some(RetrievalDimension::Semantic))],
            Some(SESSION_A),
        )
        .expect("失败登记");
    assert!(
        returned.session_key.is_none(),
        "**默认实现不落会话键，返回值也不许带**（撒谎会让桥静默失效）"
    );
    let stored = store.list_failures().expect("读失败轨迹");
    assert!(
        stored[0].session_key.is_none(),
        "**默认实现不落会话键**（老数据路径仍可用 ⇒ 不炸）"
    );
    assert_eq!(
        returned.session_key, stored[0].session_key,
        "返回值与 store 的会话键必须一致（不许撒谎）"
    );
}

/// ②-补 **默认实现 = 旧方法逐字一致**：不覆写新方法的 store，
/// 它的行为与**旧方法**完全相同（归因照旧由旧方法决定）。
#[test]
fn default_trait_impl_behaves_exactly_like_the_old_method() {
    let store = BareStore {
        inner: InMemoryReflexionStore::new(),
    };
    let via_new = store
        .record_failure_with_attribution_and_session(
            FailureKind::ValidationFailed,
            "refactor",
            "重构失败",
            2_000,
            vec![attr("mem-1", Some(RetrievalDimension::Semantic))],
            Some(SESSION_A),
        )
        .expect("失败登记");
    let via_old = store
        .record_failure_with_attribution(
            FailureKind::ValidationFailed,
            "refactor",
            "重构失败",
            2_000,
            vec![attr("mem-1", Some(RetrievalDimension::Semantic))],
        )
        .expect("失败登记");
    assert_eq!(
        via_new.attributions, via_old.attributions,
        "默认实现下，新方法与旧方法的**归因行为必须一致**"
    );
    assert_eq!(via_new.kind, via_old.kind);
    assert_eq!(via_new.task_type, via_old.task_type);
    assert_eq!(via_new.summary, via_old.summary);
    assert_eq!(via_new.timestamp_epoch_ms, via_old.timestamp_epoch_ms);
    assert!(
        via_new.session_key.is_none() && via_old.session_key.is_none(),
        "旧方法本来就不带会话键 ⇒ ⇒ 两边都是 None"
    );
}

/// ②-补 **内存版覆写 ⇒ 真落**（内存版是文件版的同款语义）。
#[test]
fn in_memory_store_overrides_and_persists_session_key() {
    let store = InMemoryReflexionStore::new();
    let record = store
        .record_failure_with_attribution_and_session(
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败",
            2_000,
            vec![attr("mem-1", Some(RetrievalDimension::Semantic))],
            Some(SESSION_A),
        )
        .expect("失败登记");
    assert_eq!(record.session_key.as_deref(), Some(SESSION_A));
    let stored = store.list_failures().expect("读失败轨迹");
    assert_eq!(
        stored[0].session_key.as_deref(),
        Some(SESSION_A),
        "内存版覆写了 ⇒ ⇒ 会话键**真的落进 store**"
    );
    assert_eq!(stored[0].attributions.len(), 1, "归因也要落");

    // 空会话键 ⇒ ⇒ `None`（不落**假归属**）。
    let blank = store
        .record_failure_with_attribution_and_session(
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败 2",
            3_000,
            Vec::new(),
            Some("   "),
        )
        .expect("失败登记");
    assert!(
        blank.session_key.is_none(),
        "空白会话键 ⇒ ⇒ **不许落假归属**（不知道属于哪个会话就是不知道）"
    );
}

/// ③ ⭐ **老 JSON 文件（没有 `session_key` 字段）读得开**、不报错、`session_key == None`。
///
/// # 老文件长什么样
/// 手写一份**只有老字段**的 `reflexions.json`（连 `attributions` 都没有）
/// ⇒ ⇒ 这正是「会话键字段引入前」的落盘形状。
#[test]
fn legacy_json_without_session_key_still_loads() {
    let dir = tempfile::tempdir().expect("临时目录");
    let root = dir.path().join("reflexion");
    std::fs::create_dir_all(&root).expect("建目录");
    let legacy = r#"{
  "failures": [
    {
      "seq": 1,
      "kind": "decision_rejected",
      "task_type": "deploy",
      "summary": "老失败",
      "timestamp_epoch_ms": 1000
    }
  ],
  "reflections": [],
  "reflected_until": 0
}"#;
    std::fs::write(root.join("reflexions.json"), legacy).expect("写老文件");

    let store = FileReflexionStore::new(root.clone());
    let failures = store
        .list_failures()
        .expect("**老文件必须读得开**（不报错）");
    assert_eq!(failures.len(), 1, "老文件里的失败必须读出来");
    assert!(
        failures[0].session_key.is_none(),
        "老文件没有会话键 ⇒ ⇒ `None`（**不编造归属**）"
    );
    assert!(
        failures[0].attributions.is_empty(),
        "老文件没有归因字段 ⇒ ⇒ 空（**不编造归因**）"
    );

    // ⚠️ **老文件上继续写不许炸**（向后兼容的另一面）。
    store
        .record_failure_with_attribution_and_session(
            FailureKind::ValidationFailed,
            "deploy",
            "新失败",
            2_000,
            vec![attr("mem-2", Some(RetrievalDimension::Semantic))],
            Some(SESSION_A),
        )
        .expect("老文件上继续写");
    let after = store.list_failures().expect("读回");
    assert_eq!(after.len(), 2);
    assert_eq!(after[1].session_key.as_deref(), Some(SESSION_A));
}

/// ④ **老数据**（没有会话键的失败）在**会话模式**下被跳过、在**全局模式**下被收。
///
/// ⚠️ 这是**兜底路径**的诚实口径：老数据靠**接线层进程内记账**补标签，
/// 真 store 自己**不知道**它属于哪个会话 ⇒ ⇒ 会话模式下**必须**跳过（不串扰）。
#[test]
fn legacy_record_without_session_key_is_skipped_in_session_mode() {
    let store = InMemoryReflexionStore::new();
    store
        .record_failure(FailureKind::DecisionRejected, "deploy", "老失败", 1_000)
        .expect("老口径登记");

    let mut engine = RetrievalTuningEngine::new();
    let mut cursor = ReflexionTuningCursor::default();
    let in_session =
        observe_failures_since_in_session(&store, &mut engine, &mut cursor, Some(SESSION_A))
            .expect("会话结算");
    assert_eq!(
        in_session, 0,
        "**不知道属于哪个会话的失败不许混进会话结算**（隔离是安全边界）"
    );

    let mut engine = RetrievalTuningEngine::new();
    let mut cursor = ReflexionTuningCursor::default();
    let global = observe_failures_since(&store, &mut engine, &mut cursor).expect("全局结算");
    assert_eq!(global, 0, "老失败没有归因 ⇒ ⇒ 零证据（不许瞎归因）");
}
