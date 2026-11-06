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
        let mut seen_in_failure: Vec<String> = Vec::new();
        for attribution in &failure.attributions {
            if attribution.memory_id.trim().is_empty() {
                continue;
            }
            if seen_in_failure
                .iter()
                .any(|id| id == &attribution.memory_id)
            {
                continue;
            }
            seen_in_failure.push(attribution.memory_id.clone());
            out.push(TuningEvidence {
                signal: crate::retrieval_tuning::EvidenceSignal::Negative,
                memory_id: attribution.memory_id.clone(),
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
        let mut seen_in_failure: Vec<String> = Vec::new();
        for attribution in &failure.attributions {
            if attribution.memory_id.trim().is_empty() {
                continue;
            }
            if seen_in_failure
                .iter()
                .any(|id| id == &attribution.memory_id)
            {
                continue;
            }
            let Some(confidence) = confidence_of(&attribution.memory_id) else {
                continue;
            };
            seen_in_failure.push(attribution.memory_id.clone());
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

    /// ⭐ **失败真的能喂权重自演化**（2026-11-05）
    ///
    /// 接线前：失败轨迹只有 `task_type` + `summary` ⇒ ⇒ **归不到任何记忆**
    /// ⇒ ⇒ ⇒ 权重自演化**永远收不到失败信号** ⇒ ⇒ ⇒ **权重只会往「被选中」漂**。
    #[test]
    fn failure_attribution_feeds_negative_tuning_evidence() {
        let store = InMemoryReflexionStore::new();
        let record = store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "database_migration",
                "迁移失败：约束冲突",
                1_900_000_000,
                vec![FailureAttribution {
                    memory_id: "ep-mem-1".to_string(),
                    dimension: Some(RetrievalDimension::Semantic),
                }],
            )
            .expect("record failure");
        assert_eq!(
            record.attributions.len(),
            1,
            "归因必须**真的落进失败记录**（不然桥是空的）"
        );

        let failures = store.list_failures().expect("list failures");
        let evidence = failures_to_tuning_evidence(&failures);
        assert_eq!(evidence.len(), 1, "一条归因 ⇒ ⇒ 一条证据");
        assert_eq!(evidence[0].memory_id, "ep-mem-1");
        assert_eq!(evidence[0].dimension, Some(RetrievalDimension::Semantic));
        assert_eq!(
            evidence[0].signal,
            crate::retrieval_tuning::EvidenceSignal::Negative,
            "失败 ⇒ ⇒ **负信号**（与命中回执的正信号成对）"
        );
    }

    /// ⚠️ **没有归因 ⇒ ⇒ 零证据**（不许瞎归因 ⇒ ⇒ 也不污染权重）
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

    /// ⭐ **结算真的会改权重**（2026-11-05）：失败 ⇒ ⇒ 负证据 ⇒ ⇒ **权重真的退**
    #[test]
    fn observe_failures_moves_the_weight() {
        let store = InMemoryReflexionStore::new();
        for index in 0..8 {
            store
                .record_failure_with_attribution(
                    FailureKind::DecisionRejected,
                    "database_migration",
                    "迁移失败",
                    1_900_000_000 + index,
                    vec![FailureAttribution {
                        memory_id: "ep-mem-1".to_string(),
                        dimension: Some(RetrievalDimension::Semantic),
                    }],
                )
                .expect("record failure");
        }
        let mut engine = RetrievalTuningEngine::new();
        let observed = observe_failures(&store, &mut engine).expect("observe failures");
        assert_eq!(observed, 8, "8 条失败 ⇒ ⇒ 8 条证据喂进去");
        let effective = engine.effective();
        assert!(
            effective.ranking.semantic_weight < 0.25,
            "连续失败 ⇒ ⇒ **语义维度权重必须退**（自 0.25 起）⇒ ⇒ 实得 {}",
            effective.ranking.semantic_weight
        );
    }

    /// ⭐ **校准观测**：失败 ⇒ ⇒ 「预测落空」的一对数据
    #[test]
    fn failure_becomes_a_calibration_observation() {
        let store = InMemoryReflexionStore::new();
        store
            .record_failure_with_attribution(
                FailureKind::DecisionRejected,
                "deploy",
                "部署失败",
                1_900_000_000,
                vec![FailureAttribution {
                    memory_id: "ep-mem-1".to_string(),
                    dimension: Some(RetrievalDimension::Confidence),
                }],
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
        // ⚠️ 校准缺口是**分箱后**的量（`CalibrationBin::calibration_gap`）
        // ⇒ ⇒ 这里量**单条观测**的可判事实：**预测 0.8、实际 0.0** ⇒ ⇒ 平方误差 > 0
        // ⇒ ⇒ ⇒ 即「它自信过头了」。
        let squared =
            crate::calibration::brier_squared(observations[0].forecast, observations[0].outcome);
        assert!(
            squared > 0.0,
            "forecast 0.8 而实际落空 ⇒ ⇒ **Brier 平方误差 > 0**（自信过头了）"
        );
        assert!(
            observations[0].forecast > 0.5 && observations[0].outcome == 0.0,
            "观测必须是「**自信却落空**」那一对 ⇒ ⇒ 实得 forecast={} outcome={}",
            observations[0].forecast,
            observations[0].outcome
        );
    }
}
