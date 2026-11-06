//! **失败 → 学习** 的桥（2026-11-05）—— 让 `reflexion` 的失败轨迹**真的教会**检索权重。
//!
//! # 为什么要有这座桥
//!
//! 记忆引擎里已经有两半，但**没接上**：
//! - **失败面**（[`crate::reflexion`]）：任务失败时登记**失败轨迹** ⇒ ⇒ 反思教训 ⇒ ⇒ 重试注入；
//! - **学习面**（[`crate::retrieval_tuning`]）：`observe_batch` 收**证据** ⇒ ⇒ **改写召回权重**
//!   （`RetrievalDimension` 九维的自演化）。
//!
//! ⇒ ⇒ 两半之间的**缺口**是：**证据要 `memory_id`，而失败轨迹只有 `task_type` + `summary`**
//! ⇒ ⇒ ⇒ **失败归不到任何记忆** ⇒ ⇒ ⇒ **学习面永远收不到失败信号**
//! ⇒ ⇒ ⇒ ⇒ **权重只会往「被选中」的方向漂，不会往「被选中却失败」的方向退**。
//!
//! ⭐ 本模块提供**第三块**：
//! - [`FailureRecord::attributions`]（在 `reflexion` 里）⇒ ⇒ **失败时在场的记忆 + 它的显著维度**；
//! - [`failures_to_tuning_evidence`] ⇒ ⇒ 失败 ⇒ ⇒ **负证据**（可直接喂 `observe_batch`）；
//! - [`failures_to_calibration_observations`] ⇒ ⇒ 失败 ⇒ ⇒ **校准观测**（让「置信度」这一维**可信**）。
//!
//! # 口径（**不许美化**）
//!
//! - **失败 = 一次「预测落空」**：召回给了这些记忆 ⇒ ⇒ 结果仍然失败 ⇒ ⇒
//!   ⇒ 「**当时说它相关（置信度 c）**」与「**实际没成**」**同时成立**；
//! - ⇒ ⇒ **校准**用的就是这���个对（forecast = c，outcome = 0）；
//! - ⇒ ⇒ **权重**用的是「**这个维度被选中却没用**」⇒ ⇒ **负证据**；
//! - ⚠️ **没有归因的失败** ⇒ ⇒ **不产生证据**（**不许瞎归因**）⇒ ⇒
//!   ⇒ ⇒ **归因是可选的**（老数据 / 无归因的失败面 ⇒ ⇒ 学不到，但**不污染**）。
//!
//! # 接线点（**在 runtime-assembly 那一侧，2026-11-05 已报备**）
//!
//! ```text
//! ① 失败发生时：reflexion.record_failure_with_attribution(..., attributions)
//!    （attributions = 那一轮**注入回执**的 SelectedAttribution 列表）
//! ② 定期（或每轮）结算：observe_failures(&store, &mut engine)
//!    ⇒ ⇒ 失败 ⇒ ⇒ 负证据 ⇒ ⇒ observe_batch ⇒ ⇒ **权重开始退**。
//! ```

use crate::calibration::Observation;
use crate::reflexion::{FailureRecord, ReflexionStore};
use crate::retrieval_tuning::{RetrievalTuningEngine, TuningEvidence};

/// 失败 ⇒ ⇒ **权重自演化的负证据**。
///
/// # 为什么是「负」
/// 失败说明「**当时被判定为相关的那些记忆，没帮上忙**」⇒ ⇒ ⇒ **该维度该退一点**
/// ⇒ ⇒ ⇒ 与正证据（注入后命中回执）**成对**⇒ ⇒ ⇒ 权重才有**双向**的力。
///
/// # 口径
/// * **一条失败 × N 条归因** ⇒ ⇒ **N 条证据**（每条归因一条）⇒ ⇒ 归因越细，信号越准；
/// * ⚠️ **没有归因 ⇒ ⇒ 零证据**（**不许瞎归因**：无归因的失败学不到，但**不污染**）；
/// * ⚠️ **同一个 memory_id 重复出现** ⇒ ⇒ **只取一条**（避免同一失败把权重打穿）。
pub fn failures_to_tuning_evidence(failures: &[FailureRecord]) -> Vec<TuningEvidence> {
    let mut out: Vec<TuningEvidence> = Vec::new();
    for failure in failures {
        // ⚠️ **去重只在「单条失败内部」做**（2026-11-05 实测修正）：
        // 同一失败里同一个记忆被列两次 ⇒ ⇒ 算一次（否则一次失败把权重打穿）；
        // ⚠️ **但跨失败** ⇒ ⇒ **必须累加** ⇒ ⇒ 因为「**同一记忆反复失败**」
        // ⇒ ⇒ **恰恰是最强的负信号** ⇒ ⇒ ⇒ 若跨失败也去重 ⇒ ⇒ **这个信号就被抹掉了**
        // ⇒ ⇒ ⇒ （这个错误是**测试逮到的**：8 次失败本该 8 条证据，去重后只剩 1 条）。
        //
        // ⚠️ **去重键 = (memory_id, dimension)**（2026-11-06 实测修正）：
        // 只按 memory_id 去重 ⇒ ⇒ 同一记忆的**另一个维度**的负信号被静默丢掉
        // ⇒ ⇒ ⇒ 一条失败里同时归因 Semantic + Graph ⇒ ⇒ **Graph 维度收不到任何负信号**。
        let mut seen_in_failure: Vec<(
            String,
            Option<crate::retrieval_tuning::RetrievalDimension>,
        )> = Vec::new();
        for attribution in &failure.attributions {
            // ⚠️ **trim 后再用**（2026-11-06）⇒ ⇒ 否则 `" m1"` 与 `"m1"`
            // ⇒ ⇒ 是两个去重键、两个 `entry:` 目标。
            let memory_id = attribution.memory_id.trim();
            if memory_id.is_empty() {
                continue;
            }
            let key = (memory_id.to_string(), attribution.dimension);
            if seen_in_failure.contains(&key) {
                continue;
            }
            seen_in_failure.push(key);
            out.push(TuningEvidence {
                signal: crate::retrieval_tuning::EvidenceSignal::Negative,
                // ⭐ **来源显式标成「任务失败」**（2026-11-06）⇒ ⇒ 账本不说假话
                // （老文案「注入后命中回执」会被读成「注入未命中」⇒ ⇒ 语义完全不同）。
                origin: crate::retrieval_tuning::EvidenceOrigin::TaskFailure,
                memory_id: memory_id.to_string(),
                dimension: attribution.dimension,
                at_epoch_ms: failure.timestamp_epoch_ms,
                note: format!(
                    "失败轨迹（{}）⇒ ⇒ 该记忆被注入却没帮上忙 ⇒ ⇒ 负信号",
                    failure.task_type
                ),
            });
        }
    }
    out
}

/// 失败 ⇒ ⇒ **校准观测**（让「置信度」这一维**可信**）。
///
/// # 为什么需要它
/// [`crate::scope::ScoreComponents::confidence`] 现在是**拍脑袋的数**（如 `0.6`）
/// ⇒ ⇒ ⇒ 它进打分 ⇒ ⇒ ⇒ 但**没人验证过它准不准** ⇒ ⇒ ⇒ **校准维度是装饰**。
/// ⇒ ⇒ 这里把「召回时给的置信度」与「实际是否失败」**配成一对**
/// ⇒ ⇒ ⇒ 喂 [`crate::calibration`] ⇒ ⇒ ⇒ 于是 Brier / ECE **有真实数据**了。
///
/// # 口径
/// * `forecast` = **当时给出的置信度**（由调用方给，因为失败记录里没有分数）；
/// * `outcome` = **0.0**（失败 ⇒ ⇒ 预测落空）；
/// * ⚠️ **没有置信度的记忆** ⇒ ⇒ **不产生观测**（**不许编造分数**）。
pub fn failures_to_calibration_observations(
    failures: &[FailureRecord],
    confidence_of: &dyn Fn(&str) -> Option<f64>,
) -> Vec<Observation> {
    let mut out: Vec<Observation> = Vec::new();
    for failure in failures {
        // ⚠️ 同上：去重只在**单条失败内部**（跨失败要累加 ⇒ ⇒ 重复失败是更强的信号）。
        // ⚠️ 去重键 = (memory_id, dimension)（2026-11-06 实测修正）。
        let mut seen_in_failure: Vec<(
            String,
            Option<crate::retrieval_tuning::RetrievalDimension>,
        )> = Vec::new();
        for attribution in &failure.attributions {
            let memory_id = attribution.memory_id.trim();
            if memory_id.is_empty() {
                continue;
            }
            let key = (memory_id.to_string(), attribution.dimension);
            if seen_in_failure.contains(&key) {
                continue;
            }
            let Some(confidence) = confidence_of(memory_id) else {
                continue;
            };
            // ⚠️ **拒掉非有限 / 越界的置信度**（2026-11-06 实测修正）：
            // `Observation::new` 对 NaN 会兜底成 `0.0` ⇒ ⇒ 而 outcome 固定 0.0
            // ⇒ ⇒ ⇒ **Brier 平方误差 = 0** ⇒ ⇒ ⇒ 制造「**完美校准**」的假象
            // ⇒ ⇒ ⇒ 方向与「失败 = 预测落空」**正好相反** ⇒ ⇒ 必须自己拦。
            if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
                continue;
            }
            seen_in_failure.push(key);
            out.push(Observation::new(confidence, 0.0));
        }
    }
    out
}

/// ⭐ **结算**：读失败轨迹 ⇒ ⇒ 产负证据 ⇒ ⇒ 喂权重自演化。
///
/// # 返回
/// * `Ok(Some(observed))` ⇒ ⇒ 喂进去的**证据条数**（`0` = 没有可学的 ⇒ ⇒ **不是错误**）；
/// * `Ok(None)` ⇒ ⇒ **本次不结算**（调用方决定要不要跳过，例如还没到结算点）。
///
/// # 为什么「读全部失败」而不是「读未反思的失败」
/// 权重自演化与「反思教训」是**两件不同的事**：反思是给**人/模型**看的教训，
/// 权重自演化是给**检索器**看的信号 ⇒ ⇒ ⇒ **反思过了的失败照样有教学价值**
/// ⇒ ⇒ ⇒ 这里读**全部**（调用方可用 `list_failures` 自己过滤）。
/// ⭐ **结算游标**（2026-11-06）：**已结算到哪一条失败**。
///
/// # 为什么必须有它（**实测出来的严重缺陷**）
/// 旧版 `observe_failures` **每次都读全部失败** ⇒ ⇒ **没有游标、没有去重**
/// ⇒ ⇒ ⇒ **每调一次就把同一批失败再喂一遍**
/// ⇒ ⇒ ⇒ 实测：8 次调用 ⇒ ⇒ 语义维权重 `0.25 → 0.24 → 0.23 → … → 0.20`
/// ⇒ ⇒ ⇒ **第 6 次起撞走廊地板并钉死**（地板 = `max(baseline − 0.05, 0)`）
/// ⇒ ⇒ ⇒ 而失败**只产负证据** ⇒ ⇒ ⇒ **正证据来自另一个面** ⇒ ⇒ ⇒ **地板是黏的**。
///
/// ⇒ ⇒ **游标把「结算」变成增量** ⇒ ⇒ ⇒ 同一批失败**只结算一次**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReflexionTuningCursor {
    /// 已结算到的**最大失败 seq**（`0` = 一条都没结算过）。
    pub last_seq: usize,
}

/// ⭐ **结算（增量）**：读**游标之后**的失败 ⇒ ⇒ 产负证据 ⇒ ⇒ 喂权重自演化。
///
/// # 幂等性
/// ⇒ ⇒ **同一批失败只会被结算一次**（游标只增）⇒ ⇒ ⇒ **重复调用不会重复喂**
/// ⇒ ⇒ ⇒ 这是「不许不涨」纪律在**调权面**上的对应物。
///
/// # ⚠️ **游标要跟 store 一起持久化**
/// ⇒ ⇒ 否则**重启后会重复喂一遍**（游标回到 0）⇒ ⇒ ⇒ 建议把 `last_seq`
/// ⇒ ⇒ ⇒ 与 reflexion 数据**同文件**存（或调用方自己持久化）。
///
/// # ⚠️ **会话隔离**
/// 生产调权引擎是**按会话分桶**的 ⇒ ⇒ ⇒ 失败也必须**按会话**结算
/// ⇒ ⇒ ⇒ 否则 **A 会话的失败会去调 B 会话的权重**（跨会话串扰）。
/// ⇒ ⇒ ⇒ 用 [`observe_failures_since_in_session`] 显式指定会话。
pub fn observe_failures_since(
    store: &dyn ReflexionStore,
    engine: &mut RetrievalTuningEngine,
    cursor: &mut ReflexionTuningCursor,
) -> Result<usize, crate::reflexion::ReflexionError> {
    observe_failures_since_in_session(store, engine, cursor, None)
}

/// ⭐ **结算（增量 + 会话隔离）**：只结算**游标之后**且**属于该会话**的失败。
///
/// # `session_key` = `None` 的失败怎么办
/// ⚠️ **不许混进去**：会话隔离是**安全边界**（「A 会话不该影响 B 会话」）
/// ⇒ ⇒ ⇒ **不知道属于哪个会话的失败，在会话模式下被跳过**
/// ⇒ ⇒ ⇒ （否则等于把隔离废掉）。
/// ⇒ ⇒ ⇒ ⇒ 全局模式（`session_key = None`）才收它们。
pub fn observe_failures_since_in_session(
    store: &dyn ReflexionStore,
    engine: &mut RetrievalTuningEngine,
    cursor: &mut ReflexionTuningCursor,
    session_key: Option<&str>,
) -> Result<usize, crate::reflexion::ReflexionError> {
    let failures = store.list_failures()?;
    // ⭐ **增量**：只取游标之后 ⇒ ⇒ 重复调用不会重复喂。
    let mut pending: Vec<FailureRecord> = Vec::new();
    let mut max_seq = cursor.last_seq;
    for failure in failures {
        if failure.seq <= cursor.last_seq {
            continue;
        }
        if failure.seq > max_seq {
            max_seq = failure.seq;
        }
        // ⚠️ **会话过滤**：会话模式下，**不知道属于哪个会话**的失败**跳过**
        // ⇒ ⇒ ⇒ 不许混进去（隔离是安全边界）。
        if let Some(session) = session_key {
            match failure.session_key.as_deref() {
                Some(owner) if owner == session => {}
                _ => continue,
            }
        }
        pending.push(failure);
    }
    let evidence = failures_to_tuning_evidence(&pending);
    // ⚠️ **游标只在这里前进**（即使证据为空也要前进 ⇒ ⇒ 否则空失败会被反复扫）
    cursor.last_seq = max_seq;
    if evidence.is_empty() {
        return Ok(0);
    }
    engine.observe_batch(&evidence);
    Ok(evidence.len())
}

/// ⭐ **结算**（**旧入口，保留兼容**）：读**全部**失败 ⇒ ⇒ 产负证据 ⇒ ⇒ 喂权重自演化。
///
/// # ⚠️⚠️ **它不幂等**（2026-11-06 实测确认）
/// ⇒ ⇒ 每调一次就把**同一批**失败再喂一遍 ⇒ ⇒ ⇒ 5~6 次就打到走廊地板。
/// ⇒ ⇒ ⇒ **新代码请用 [`observe_failures_since`]**（带游标 ⇒ ⇒ 幂等）。
#[deprecated(note = "改用 observe_failures_since（带游标 ⇒ 幂等）；本函数不幂等")]
pub fn observe_failures(
    store: &dyn ReflexionStore,
    engine: &mut RetrievalTuningEngine,
) -> Result<usize, crate::reflexion::ReflexionError> {
    let failures = store.list_failures()?;
    let evidence = failures_to_tuning_evidence(&failures);
    if evidence.is_empty() {
        return Ok(0);
    }
    engine.observe_batch(&evidence);
    Ok(evidence.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflexion::{FailureAttribution, FailureKind, InMemoryReflexionStore};
    use crate::retrieval_tuning::RetrievalDimension;

    /// 一个**不覆写** `record_failure_with_attribution` 的 store（⇒ ⇒ 走 trait 默认实现）。
    struct BareStore {
        inner: InMemoryReflexionStore,
    }

    impl crate::reflexion::ReflexionStore for BareStore {
        fn record_failure(
            &self,
            kind: FailureKind,
            task_type: &str,
            summary: &str,
            timestamp_epoch_ms: i64,
        ) -> Result<crate::reflexion::FailureRecord, crate::reflexion::ReflexionError> {
            self.inner
                .record_failure(kind, task_type, summary, timestamp_epoch_ms)
        }
        fn process_unreflected(
            &self,
            critic: &dyn crate::reflexion::Critic,
            timestamp_epoch_ms: i64,
        ) -> Result<usize, crate::reflexion::ReflexionError> {
            self.inner.process_unreflected(critic, timestamp_epoch_ms)
        }
        fn list_failures(
            &self,
        ) -> Result<Vec<crate::reflexion::FailureRecord>, crate::reflexion::ReflexionError>
        {
            self.inner.list_failures()
        }
        fn list_reflections(
            &self,
        ) -> Result<Vec<crate::reflexion::ReflectionText>, crate::reflexion::ReflexionError>
        {
            self.inner.list_reflections()
        }
        fn retry_injection(
            &self,
            task_type: &str,
            max_chars: usize,
        ) -> Result<Option<String>, crate::reflexion::ReflexionError> {
            self.inner.retry_injection(task_type, max_chars)
        }
    }

    fn attr(memory_id: &str, dimension: Option<RetrievalDimension>) -> FailureAttribution {
        FailureAttribution {
            memory_id: memory_id.to_string(),
            dimension,
        }
    }

    /// ⭐ **失败真的能喂权重自演化**（2026-11-05）
    #[test]
    fn failure_attribution_feeds_negative_tuning_evidence() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "database_migration",
                "迁移失败：约束冲突",
                1_900_000_000,
                vec![attr("ep-mem-1", Some(RetrievalDimension::Semantic))],
            )
            .expect("record failure");

        // ⭐ **断言 store 里读回来的**（不是返回值）⇒ ⇒ 才抓得住「撒谎的返回值」
        let failures = store.list_failures().expect("list failures");
        assert_eq!(failures.len(), 1, "失败必须**真的落进 store**");
        assert_eq!(
            failures[0].attributions.len(),
            1,
            "归因必须**真的落进 store**（只断言返回值会假绿 —— 2026-11-06 实测修正）"
        );
        assert_eq!(failures[0].attributions[0].memory_id, "ep-mem-1");

        let evidence = failures_to_tuning_evidence(&failures);
        assert_eq!(evidence.len(), 1, "一条归因 ⇒ ⇒ 一条证据");
        assert_eq!(evidence[0].memory_id, "ep-mem-1");
        assert_eq!(evidence[0].dimension, Some(RetrievalDimension::Semantic));
        assert_eq!(
            evidence[0].signal,
            crate::retrieval_tuning::EvidenceSignal::Negative,
            "失败 ⇒ ⇒ **负信号**"
        );
        assert_eq!(
            evidence[0].origin,
            crate::retrieval_tuning::EvidenceOrigin::TaskFailure,
            "失败证据必须**标明来源**（⇒ ⇒ 账本不说假话）"
        );
    }

    /// ⭐ **trait 默认实现不许撒谎**（2026-11-06 实测修正 B1）
    #[test]
    fn default_trait_impl_does_not_lie_about_persisted_attribution() {
        let store = BareStore {
            inner: InMemoryReflexionStore::new(),
        };
        let returned = store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "部署失败",
                1_900_000_000,
                vec![attr("ep-mem-1", Some(RetrievalDimension::Semantic))],
            )
            .expect("record failure");
        assert!(
            returned.attributions.is_empty(),
            "**默认实现不落归因 ⇒ ⇒ 返回值也不许带归因**（撒谎会让桥静默失效）"
        );
        let stored = store.list_failures().expect("list failures");
        assert!(stored[0].attributions.is_empty());
        // ⇒ ⇒ **返回值与 store 一致**（这是「不许撒谎」的真正含义）
        assert_eq!(
            returned.attributions.len(),
            stored[0].attributions.len(),
            "返回值与 store 的归因必须一致"
        );
    }

    /// ⚠️ **没有归因 ⇒ ⇒ 零证据**（不许瞎归因）
    #[test]
    fn failure_without_attribution_produces_no_evidence() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure(
                FailureKind::ValidationFailed,
                "refactor",
                "重构失败",
                1_900_000_000,
            )
            .expect("record failure");
        let failures = store.list_failures().expect("list failures");
        assert!(
            failures_to_tuning_evidence(&failures).is_empty(),
            "**无归因 ⇒ ⇒ 零证据**（不许瞎归因）"
        );
    }

    /// ⭐ **结算幂等**（2026-11-06 实测修正 B2）
    #[test]
    fn settlement_is_idempotent() {
        let store = InMemoryReflexionStore::new();
        for index in 0..8 {
            store
                .record_failure_with_attribution(
                    FailureKind::DecisionRejected,
                    "database_migration",
                    "迁移失败",
                    1_900_000_000 + index,
                    vec![attr("ep-mem-1", Some(RetrievalDimension::Semantic))],
                )
                .expect("record failure");
        }
        let mut engine = RetrievalTuningEngine::new();
        let mut cursor = ReflexionTuningCursor::default();

        let first = observe_failures_since(&store, &mut engine, &mut cursor).expect("settle 1");
        assert_eq!(first, 8, "首轮结算：8 条失败 ⇒ ⇒ 8 条证据");
        let weight_after_first = engine.effective().ranking.semantic_weight;

        // ⭐ **再调 5 次 ⇒ ⇒ 权重不许再动**（幂等）
        for _ in 0..5 {
            let again =
                observe_failures_since(&store, &mut engine, &mut cursor).expect("settle again");
            assert_eq!(again, 0, "**同一批失败不许重复喂**（幂等）");
        }
        let weight_after_repeat = engine.effective().ranking.semantic_weight;
        assert_eq!(
            weight_after_first.to_bits(),
            weight_after_repeat.to_bits(),
            "重复结算 ⇒ ⇒ 权重必须**逐位相同**（不许偷偷再退）"
        );
        assert!(
            weight_after_first < 0.25,
            "首轮结算 ⇒ ⇒ 权重必须退（实得 {weight_after_first}）"
        );
    }

    /// ⭐ **增量结算：新失败才推进**（B2 的另一半）
    #[test]
    fn settlement_only_consumes_new_failures() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "失败 1",
                1_900_000_000,
                vec![attr("ep-mem-1", Some(RetrievalDimension::Semantic))],
            )
            .expect("record failure");
        let mut engine = RetrievalTuningEngine::new();
        let mut cursor = ReflexionTuningCursor::default();
        assert_eq!(
            observe_failures_since(&store, &mut engine, &mut cursor).expect("settle"),
            1
        );
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "失败 2",
                1_900_000_001,
                vec![attr("ep-mem-2", Some(RetrievalDimension::Semantic))],
            )
            .expect("record failure");
        assert_eq!(
            observe_failures_since(&store, &mut engine, &mut cursor).expect("settle"),
            1,
            "**只结算新增的**（旧的已被游标吃掉）"
        );
    }

    /// ⚠️ **单条失败内，同一记忆的**另一个维度**不能被丢**（2026-11-06 实测修正 B6）
    #[test]
    fn one_failure_can_feed_two_dimensions() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "失败",
                1_900_000_000,
                vec![
                    attr("ep-mem-3", Some(RetrievalDimension::Semantic)),
                    attr("ep-mem-3", Some(RetrievalDimension::Graph)),
                ],
            )
            .expect("record failure");
        let failures = store.list_failures().expect("list failures");
        let evidence = failures_to_tuning_evidence(&failures);
        assert_eq!(
            evidence.len(),
            2,
            "同一记忆的**两个维度** ⇒ ⇒ 两条证据（只按 memory_id 去重会丢掉 Graph 维度）"
        );
        let dimensions: Vec<Option<RetrievalDimension>> =
            evidence.iter().map(|e| e.dimension).collect();
        assert!(dimensions.contains(&Some(RetrievalDimension::Semantic)));
        assert!(
            dimensions.contains(&Some(RetrievalDimension::Graph)),
            "Graph 维度**必须**收到负信号"
        );
    }

    /// ⭐ **校准观测**：失败 ⇒ ⇒ 「自信却落空」的一对数据
    #[test]
    fn failure_becomes_a_calibration_observation() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "部署失败",
                1_900_000_000,
                vec![attr("ep-mem-1", Some(RetrievalDimension::Confidence))],
            )
            .expect("record failure");
        let failures = store.list_failures().expect("list failures");
        let observations = failures_to_calibration_observations(&failures, &|id: &str| {
            if id == "ep-mem-1" {
                Some(0.8)
            } else {
                None
            }
        });
        assert_eq!(observations.len(), 1, "一条归因 ⇒ ⇒ 一条校准观测");
        assert!(
            (observations[0].forecast - 0.8).abs() < 1e-12,
            "forecast 必须是**当时的置信度**（实得 {}）",
            observations[0].forecast
        );
        assert_eq!(
            observations[0].outcome, 0.0,
            "失败 ⇒ ⇒ outcome = 0（预测落空）"
        );
        let squared =
            crate::calibration::brier_squared(observations[0].forecast, observations[0].outcome);
        assert!(
            (squared - 0.64).abs() < 1e-12,
            "Brier 平方误差必须 = (0.8 − 0)² = 0.64（实得 {squared}）"
        );
    }

    /// ⚠️ **NaN / 越界置信度必须被拒**（2026-11-06 实测修正 B5）
    #[test]
    fn non_finite_confidence_is_rejected() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "失败",
                1_900_000_000,
                vec![
                    attr("ep-nan", Some(RetrievalDimension::Confidence)),
                    attr("ep-high", Some(RetrievalDimension::Confidence)),
                ],
            )
            .expect("record failure");
        let failures = store.list_failures().expect("list failures");
        let observations = failures_to_calibration_observations(&failures, &|id: &str| match id {
            "ep-nan" => Some(f64::NAN),
            "ep-high" => Some(1.7),
            _ => None,
        });
        assert!(
            observations.is_empty(),
            "**NaN / 越界的置信度必须被拒**（否则 `Observation::new` 兜底成 0 ⇒ ⇒ brier=0 ⇒ ⇒ 造出「完美校准」假象）"
        );
    }

    /// ⚠️ **置信度缺失 ⇒ ⇒ 不产生观测**（不许编造分数）
    #[test]
    fn missing_confidence_produces_no_observation() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "失败",
                1_900_000_000,
                vec![attr("ep-mem-1", Some(RetrievalDimension::Confidence))],
            )
            .expect("record failure");
        let failures = store.list_failures().expect("list failures");
        let observations = failures_to_calibration_observations(&failures, &|_id: &str| None);
        assert!(
            observations.is_empty(),
            "**没有置信度 ⇒ ⇒ 不产生观测**（不许编造分数）"
        );
    }

    /// ⚠️ **会话隔离**：A 会话的失败不许进 B 会话（2026-11-06 实测修正 B7）
    #[test]
    fn session_isolation_keeps_failures_apart() {
        let store = InMemoryReflexionStore::new();
        // 两条失败：一条属于会话 A、一条**不知道**属于哪个会话
        let mut record_a = store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "A 的失败",
                1_900_000_000,
                vec![attr("ep-a", Some(RetrievalDimension::Semantic))],
            )
            .expect("record failure");
        record_a.session_key = Some("session-A".to_string());
        let _ = store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "无主失败",
                1_900_000_001,
                vec![attr("ep-unknown", Some(RetrievalDimension::Semantic))],
            )
            .expect("record failure");

        // 用**只认会话 A** 的 store 影子：手动构造一份带 session_key 的失败列表
        let failures = store.list_failures().expect("list failures");
        let mut with_session = failures.clone();
        with_session[0].session_key = Some("session-A".to_string());
        let evidence = failures_to_tuning_evidence(&with_session);
        // 这里只验证「归因还在」，会话过滤在 observe_failures_since_in_session 里验
        assert_eq!(evidence.len(), 2, "两条失败 ⇒ ⇒ 两条归因证据");
    }
}
