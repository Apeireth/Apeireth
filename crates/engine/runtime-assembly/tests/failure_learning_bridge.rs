//! 「失败 → 学习桥」生产接线验收 (2026-11-06)。
//!
//! 桥的两端都在**生产真件**上验 (不许假 store / 假 engine):
//! - **失败登记侧**: 真 store (`FileReflexionStore` —— 生产 CLI 用的同款)
//!   **真的落盘归因**（不是只改返回值）;
//! - **学习侧**: 真引擎（`RetrievalTuningEngine`, 接线层持有）+ 真生效值槽
//!   ⇒ ⇒ **权重真的退**，不是「函数被调用了」。
//!
//! 钉住三件事:
//! ① **有归因的失败 ⇒ 负证据 ⇒ 权重真的退**（逐位对账基线与步长）;
//! ② **没有归因的失败 ⇒ 零证据 ⇒ 权重一动不动**（不许瞎归因）;
//! ③ **会话隔离**: A 会话的失败**不许**去调 B 会话的权重（跨会话串扰）。
//!
//! ⚠️ **测试隔离**: 生效检索权重落在**进程级全局槽**（按会话分桶）
//! ⇒ ⇒ 每个用例用**自己的会话键**（互不撞桶）⇒ ⇒ 并行跑也不串。

use std::path::PathBuf;

use apeireth_memory::reflexion::{FailureKind, FileReflexionStore, ReflexionStore};
use apeireth_memory::retrieval_tuning::{
    clear_effective_for_session, effective_ranking_or_for_session, enter_session_scope,
    entry_salience_for_session, RetrievalDimension, SelectedAttribution, DIMENSION_STEP,
    ENTRY_STEP,
};
use apeireth_memory::MemoryRankingConfig;
use apeireth_runtime_assembly::{read_retrieval_tuning_log, RetrievalTuningWire};

/// 每个用例**自己的**会话键 (生效值槽是进程级全局 ⇒ 键不撞 ⇒ 不串)。
const SESSION_ATTRIBUTED: &str = "session-attributed";
const SESSION_UNATTRIBUTED: &str = "session-unattributed";
const SESSION_ISO_A: &str = "session-iso-a";
const SESSION_ISO_B: &str = "session-iso-b";
const SESSION_IDEMPOTENT: &str = "session-idempotent";

/// 生产同款: 真 store (文件版) + 真接线层 (账落在同一目录)。
struct Fixture {
    _dir: tempfile::TempDir,
    store: FileReflexionStore,
    wire: RetrievalTuningWire,
    log: PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let dir = tempfile::tempdir().expect("临时目录");
    let root = dir.path().join(label);
    std::fs::create_dir_all(&root).expect("建目录");
    let store = FileReflexionStore::new(root.join("reflexion"));
    let log = root.join("retrieval_tuning.jsonl");
    let wire = RetrievalTuningWire::new(log.clone());
    Fixture {
        _dir: dir,
        store,
        wire,
        log,
    }
}

/// 清掉本会话键的生效值 (生效值槽是**进程级全局** ⇒ 用例从干净基线开始)。
fn fresh_bucket(session_key: &str) {
    clear_effective_for_session(session_key);
}

/// 一轮召回的**注入回执** (生产同款: 选中条目 + 它的显著维度)。
fn receipt(id: &str, dimension: Option<RetrievalDimension>) -> SelectedAttribution {
    SelectedAttribution::new(id, dimension)
}

/// ① ⭐ **有归因的失败 ⇒ 负证据 ⇒ 权重真的退**。
///
/// # 为什么不是「函数被调用了」
/// 断言的是**三层真值**: ① 真 store 里读回来的**归因**; ② 结算喂进去的
/// **证据条数**; ③ **生效权重**（逐位对账基线 − 步长）。
#[test]
fn attributed_failure_produces_negative_evidence_and_recedes_the_weight() {
    let fixture = fixture("attributed");
    let baseline = MemoryRankingConfig::default();
    fresh_bucket(SESSION_ATTRIBUTED);

    // ① 那一轮的注入回执（生产同款: 召回选中集 ⇒ 回执面）。
    {
        let _scope = enter_session_scope(SESSION_ATTRIBUTED);
        fixture.wire.record_injection_round(
            &[receipt("mem-1", Some(RetrievalDimension::Semantic))],
            1_000,
        );
    }

    // ② 失败发生（归因 = 那一轮的注入回执 ⇒ 真 store 落盘）。
    fixture
        .wire
        .record_failure_with_injection_attribution(
            &fixture.store,
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败: 约束冲突",
            2_000,
            SESSION_ATTRIBUTED,
        )
        .expect("失败登记");

    // ⚠️ 断言**真 store 里读回来的**（不是返回值）⇒ 才抓得住「撒谎的返回值」。
    let stored = fixture.store.list_failures().expect("读失败轨迹");
    assert_eq!(stored.len(), 1, "失败必须**真的落进真 store**");
    assert_eq!(
        stored[0].attributions.len(),
        1,
        "归因必须**真的落进真 store**（只断言返回值会假绿）"
    );
    assert_eq!(stored[0].attributions[0].memory_id, "mem-1");
    assert_eq!(
        stored[0].attributions[0].dimension,
        Some(RetrievalDimension::Semantic),
        "归因必须带**显著维度**（否则学不到维度这一维）"
    );

    // ③ 结算（轮次边界）⇒ 负证据 ⇒ 权重退。
    let settled = fixture
        .wire
        .settle_failures(&fixture.store, SESSION_ATTRIBUTED)
        .expect("结算");
    assert_eq!(settled, 1, "一条归因 ⇒ 一条负证据（**不许瞎归因**）");

    // ⚠️ **权重真的退**（生效值槽 = 生产检索真正吃的那份）。
    let after = effective_ranking_or_for_session(SESSION_ATTRIBUTED, baseline);
    assert!(
        (after.semantic_weight - (baseline.semantic_weight - DIMENSION_STEP)).abs() < 1e-9,
        "语义维权重必须退一步（基线 {} ⇒ 实得 {}）",
        baseline.semantic_weight,
        after.semantic_weight
    );
    assert!(
        (after.lexical_weight - baseline.lexical_weight).abs() < 1e-9,
        "**没归因的维度不许动**（实得 {}）",
        after.lexical_weight
    );
    assert!(
        (entry_salience_for_session(SESSION_ATTRIBUTED, "mem-1") + ENTRY_STEP).abs() < 1e-12,
        "条目显著性必须退一步（实得 {}）",
        entry_salience_for_session(SESSION_ATTRIBUTED, "mem-1")
    );

    // ④ 自进化账: 负信号真的落账（条目 + 维度各一行）。
    let summary = read_retrieval_tuning_log(&fixture.log).expect("读回自进化账");
    assert_eq!(summary.records.len(), 2, "条目 + 维度各一行: {summary:?}");
    assert!(
        summary
            .records
            .iter()
            .all(|record| record.signal == "negative"),
        "失败 ⇒ 账上必须是**负信号**: {summary:?}"
    );
}

/// ② ⚠️ **没有归因的失败 ⇒ 零证据 ⇒ 权重一动不动**（不许瞎归因）。
///
/// 两种「没有归因」都验:
/// - **那一轮什么都没注入**（空轮）⇒ 归因为空;
/// - **老口径登记**（`record_failure`, 本来就没有归因）⇒ 归因为空。
#[test]
fn failure_without_attribution_produces_no_evidence_and_no_weight_move() {
    let fixture = fixture("no-attribution");
    let baseline = MemoryRankingConfig::default();
    fresh_bucket(SESSION_UNATTRIBUTED);

    // ① 那一轮**什么都没注入**（空轮）⇒ 失败没有归因。
    {
        let _scope = enter_session_scope(SESSION_UNATTRIBUTED);
        fixture.wire.record_empty_round(1_000);
    }
    let record = fixture
        .wire
        .record_failure_with_injection_attribution(
            &fixture.store,
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败",
            2_000,
            SESSION_UNATTRIBUTED,
        )
        .expect("失败登记");
    assert!(
        record.attributions.is_empty(),
        "**空轮没有回执 ⇒ 零归因**（不许拿上一轮的回执瞎归因）"
    );

    // ② 老口径登记（没有归因字段的那条路）⇒ 同样零归因。
    fixture
        .store
        .record_failure(FailureKind::ValidationFailed, "deploy", "校验失败", 3_000)
        .expect("失败登记");

    // ③ 结算 ⇒ **零证据**。
    let settled = fixture
        .wire
        .settle_failures(&fixture.store, SESSION_UNATTRIBUTED)
        .expect("结算");
    assert_eq!(settled, 0, "**无归因 ⇒ 零证据**（不许瞎归因）");

    // ④ 权重一动不动。
    let after = effective_ranking_or_for_session(SESSION_UNATTRIBUTED, baseline);
    assert!(
        (after.semantic_weight - baseline.semantic_weight).abs() < 1e-12,
        "零证据 ⇒ 权重不许动（基线 {} ⇒ 实得 {}）",
        baseline.semantic_weight,
        after.semantic_weight
    );
    assert_eq!(
        entry_salience_for_session(SESSION_UNATTRIBUTED, "mem-1"),
        0.0,
        "零证据 ⇒ 条目显著性不许动"
    );
    assert!(!fixture.log.exists(), "零证据 ⇒ 不造账（不落假移动）");
}

/// ③ ⚠️ **会话隔离**: A 会话的失败**不许**去调 B 会话的权重（跨会话串扰）。
#[test]
fn failures_do_not_cross_sessions() {
    let fixture = fixture("session-isolation");
    let baseline = MemoryRankingConfig::default();
    fresh_bucket(SESSION_ISO_A);

    // A 会话: 注入 + 失败。
    {
        let _scope = enter_session_scope(SESSION_ISO_A);
        fixture.wire.record_injection_round(
            &[receipt("mem-a", Some(RetrievalDimension::Semantic))],
            1_000,
        );
    }
    fixture
        .wire
        .record_failure_with_injection_attribution(
            &fixture.store,
            FailureKind::DecisionRejected,
            "deploy",
            "A 的失败",
            2_000,
            SESSION_ISO_A,
        )
        .expect("失败登记");

    // B 会话结算 ⇒ **拿不到** A 的失败 ⇒ 零证据 ⇒ B 的权重不动。
    fresh_bucket(SESSION_ISO_B);
    let settled_b = fixture
        .wire
        .settle_failures(&fixture.store, SESSION_ISO_B)
        .expect("B 结算");
    assert_eq!(settled_b, 0, "**B 会话不许结算 A 的失败**（跨会话串扰）");
    let ranking_b = effective_ranking_or_for_session(SESSION_ISO_B, baseline);
    assert!(
        (ranking_b.semantic_weight - baseline.semantic_weight).abs() < 1e-12,
        "B 的权重不许被 A 的失败改（实得 {}）",
        ranking_b.semantic_weight
    );

    // A 会话结算 ⇒ 自己的失败自己吃 ⇒ A 的权重退。
    let settled_a = fixture
        .wire
        .settle_failures(&fixture.store, SESSION_ISO_A)
        .expect("A 结算");
    assert_eq!(settled_a, 1, "A 会话结算自己的失败");
    let ranking_a = effective_ranking_or_for_session(SESSION_ISO_A, baseline);
    assert!(
        (ranking_a.semantic_weight - (baseline.semantic_weight - DIMENSION_STEP)).abs() < 1e-9,
        "A 的权重退一步（实得 {}）",
        ranking_a.semantic_weight
    );
}

/// ④ ⭐ **结算幂等**: 同一批失败**只结算一次**（不许重复喂 ⇒ 不抖）。
#[test]
fn settlement_is_idempotent() {
    let fixture = fixture("idempotent");
    let baseline = MemoryRankingConfig::default();
    fresh_bucket(SESSION_IDEMPOTENT);

    {
        let _scope = enter_session_scope(SESSION_IDEMPOTENT);
        fixture.wire.record_injection_round(
            &[receipt("mem-1", Some(RetrievalDimension::Semantic))],
            1_000,
        );
    }
    fixture
        .wire
        .record_failure_with_injection_attribution(
            &fixture.store,
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败",
            2_000,
            SESSION_IDEMPOTENT,
        )
        .expect("失败登记");

    let first = fixture
        .wire
        .settle_failures(&fixture.store, SESSION_IDEMPOTENT)
        .expect("结算 1");
    assert_eq!(first, 1);
    let after_first =
        effective_ranking_or_for_session(SESSION_IDEMPOTENT, baseline).semantic_weight;

    // 再结算 3 次 ⇒ **零证据**（游标只增）。
    for _ in 0..3 {
        let again = fixture
            .wire
            .settle_failures(&fixture.store, SESSION_IDEMPOTENT)
            .expect("重复结算");
        assert_eq!(again, 0, "**同一批失败不许重复喂**（幂等）");
    }
    let after_repeat =
        effective_ranking_or_for_session(SESSION_IDEMPOTENT, baseline).semantic_weight;
    assert_eq!(
        after_first.to_bits(),
        after_repeat.to_bits(),
        "重复结算，权重必须**逐位相同**（不许偷偷再退）"
    );
}

/// ⑤ ⭐ **桶键口径只有一处**: 会话 id ≠ 桶键 (画像分区路径) ⇒ ⇒
/// 失败面**按登记的映射问桶** ⇒ ⇒ 归因仍接得上（不用各算各的）。
#[test]
fn the_registered_bucket_mapping_resolves_the_session_bucket() {
    let fixture = fixture("bucket-mapping");
    let baseline = MemoryRankingConfig::default();

    // ① 记忆召回侧登记: 会话 id ⇒ 桶键（画像分区路径）。
    fixture
        .wire
        .register_session_bucket("session-raw", "partition-bucket");
    assert_eq!(
        fixture
            .wire
            .bucket_key_for_session("session-raw")
            .as_deref(),
        Some("partition-bucket"),
        "登记过的会话必须能问回自己的桶"
    );
    assert_eq!(
        fixture.wire.bucket_key_for_session("session-unknown"),
        None,
        "没登记过的会话**问不到桶**（⇒ 回落 ⇒ 零归因, 不瞎归因）"
    );

    // ② 回执注入**那个桶**。
    {
        let _scope = enter_session_scope("partition-bucket");
        fixture.wire.record_injection_round(
            &[receipt("mem-partition", Some(RetrievalDimension::Semantic))],
            1_000,
        );
    }

    // ③ 按**桶**结算 ⇒ 归因接得上 ⇒ 权重退。
    fixture
        .wire
        .record_failure_with_injection_attribution(
            &fixture.store,
            FailureKind::DecisionRejected,
            "deploy",
            "部署失败",
            2_000,
            "partition-bucket",
        )
        .expect("失败登记");
    let settled = fixture
        .wire
        .settle_failures(&fixture.store, "partition-bucket")
        .expect("结算");
    assert_eq!(settled, 1, "分区桶的失败要能结算（键对不上就学不到）");
    let after = effective_ranking_or_for_session("partition-bucket", baseline);
    assert!(
        (after.semantic_weight - (baseline.semantic_weight - DIMENSION_STEP)).abs() < 1e-9,
        "分区桶的权重必须退（实得 {}）",
        after.semantic_weight
    );
}
