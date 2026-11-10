//! `vector_rebuild` 集成测试 —— 派生索引的**重建**/**换代检测**/**一致性验证**。
//!
//! ⚠️ 全部用**确定性假 embedder**（不调外部 API）⇒ 逐位可复现 ⇒ 能断言「一致」。
//! ⚠️ B2 的核心验收在 `delete_index_then_rebuild_keeps_search_results`：
//! **建索引 → 删索引文件 → 从源重建 → 检索结果一致**。

use std::path::Path;

use apeireth_memory::persistent_vector::PersistentVectorIndex;
use apeireth_memory::scope::{EmbeddingError, EmbeddingProvider};
use apeireth_memory::vector_rebuild::{
    compare_vectors, read_snapshot, rebuild_vector_index, RebuildVerdict, SourceRecord,
    DEFAULT_TOLERANCE,
};
use async_trait::async_trait;
use tempfile::tempdir;

/// 确定性假 embedder：向量**只由文本内容决定**（不调外部 API ⇒ 逐位可复现）。
#[derive(Debug, Clone, Default)]
struct DeterministicEmbedder {
    model: String,
    dim: usize,
}

impl DeterministicEmbedder {
    fn new(model: &str, dim: usize) -> Self {
        Self {
            model: model.to_string(),
            dim,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for DeterministicEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if self.dim == 0 {
            return Err(EmbeddingError::InvalidVector(
                "deterministic embedder needs dim > 0".into(),
            ));
        }
        // 只用整数倍增的字节权重 ⇒ 同一文本 ⇒ 逐位相同的向量。
        let mut vector = vec![0.0f32; self.dim];
        for (i, byte) in text.bytes().enumerate() {
            let slot = i % self.dim;
            vector[slot] += (f32::from(byte) + 1.0) * ((i as f32) + 1.0);
        }
        // 归一化（避免零向量 ⇒ cosine 未定义）；归一化是确定的浮点运算 ⇒ 仍逐位可复现。
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in vector.iter_mut() {
                *value /= norm;
            }
        }
        Ok(vector)
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// 同维度但**向量内容不同**的 embedder（模拟「换代后 embed 结果确实变了」）。
#[derive(Debug, Clone, Default)]
struct SaltedEmbedder {
    model: String,
    dim: usize,
}

#[async_trait]
impl EmbeddingProvider for SaltedEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let base = DeterministicEmbedder::new(&self.model, self.dim);
        let mut vector = base.embed(text).await?;
        // 加一个与模型名绑定的常数偏移 ⇒ 同维度、同文本，但数值不同。
        let salt = self
            .model
            .bytes()
            .fold(0.25f32, |acc, b| acc + f32::from(b) * 1e-3);
        for value in vector.iter_mut() {
            *value += salt;
        }
        Ok(vector)
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// 永远失败的 embedder（验证「失败 ⇒ Err，不跳过、不 panic」）。
#[derive(Debug, Clone, Default)]
struct FailingEmbedder;

#[async_trait]
impl EmbeddingProvider for FailingEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::Unavailable(
            "test embedder always fails".into(),
        ))
    }

    fn model_id(&self) -> &str {
        "failing-model"
    }
}

fn records() -> Vec<SourceRecord> {
    vec![
        SourceRecord {
            id: "a".into(),
            text: "苹果派的做法".into(),
        },
        SourceRecord {
            id: "b".into(),
            text: "香蕉船的来历".into(),
        },
        SourceRecord {
            id: "c".into(),
            text: "橙子与柠檬的区别".into(),
        },
    ]
}

async fn search_ids(path: &Path, embedder: &DeterministicEmbedder, query: &str) -> Vec<String> {
    let query_vector = embedder.embed(query).await.expect("query embed");
    let index = PersistentVectorIndex::open(path).expect("open index");
    index
        .search(&query_vector, 8, None)
        .expect("search")
        .into_iter()
        .map(|hit| hit.id)
        .collect()
}

/// 手搓一个索引（绕过 `rebuild_vector_index` ⇒ **没有模型标识**）。
fn handbuilt_index(path: &Path, items: &[(&str, Vec<f32>)]) {
    let dim = items.first().map(|(_, v)| v.len()).unwrap_or(2);
    let mut index = PersistentVectorIndex::open(path).expect("open index");
    index.set_dimension(dim).expect("set dim");
    for (id, vector) in items {
        index.upsert(*id, vector, None).expect("upsert");
    }
}

// ===========================================================================
// rebuild_vector_index：判定四档
// ===========================================================================

/// 首次建索引 ⇒ 无旧索引可比 ⇒ `NoBaseline`。
#[tokio::test]
async fn first_build_reports_no_baseline() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    let outcome = rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("rebuild");

    assert_eq!(outcome.verdict, RebuildVerdict::NoBaseline);
    assert_eq!(outcome.entries, 3);
    assert!(
        outcome.detail.contains("首次建索引"),
        "detail 要说人话: {}",
        outcome.detail
    );
}

/// 同模型重建 ⇒ `RebuiltIdentical`（真的一致，不是「大概一致」）。
#[tokio::test]
async fn same_model_rebuild_is_identical() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    let first = rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("first rebuild");
    assert_eq!(first.verdict, RebuildVerdict::NoBaseline);

    let second = rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("second rebuild");
    assert_eq!(second.verdict, RebuildVerdict::RebuiltIdentical);
    assert_eq!(second.entries, 3);
    assert!(
        second.detail.contains("一致"),
        "detail 要说人话: {}",
        second.detail
    );
}

/// 同模型 ⇒ 两次独立建出来的索引**逐位一致**（容差 0）。
#[tokio::test]
async fn same_model_two_indexes_are_bitwise_identical() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.db");
    let right = dir.path().join("right.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&left, &records(), &embedder)
        .await
        .expect("left rebuild");
    rebuild_vector_index(&right, &records(), &embedder)
        .await
        .expect("right rebuild");

    // 容差 0 ⇒ 只有逐位相同才算一致。
    assert_eq!(
        compare_vectors(&left, &right, 0.0).expect("compare"),
        (3, 0, 0)
    );
}

/// ⚠️ 换模型 ⇒ **必须报换代**（哪怕向量碰巧一样）——不许假装一致。
#[tokio::test]
async fn model_change_is_flagged_not_faked() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let first_embedder = DeterministicEmbedder::new("model-a", 4);
    let second_embedder = DeterministicEmbedder::new("model-b", 4);

    rebuild_vector_index(&path, &records(), &first_embedder)
        .await
        .expect("first rebuild");

    // ⚠️ 两个 embedder 的向量**完全一样**（向量只由文本决定），
    // 但模型标识变了 ⇒ 判定必须仍然是「换代」。
    let outcome = rebuild_vector_index(&path, &records(), &second_embedder)
        .await
        .expect("second rebuild");

    assert_eq!(outcome.verdict, RebuildVerdict::ModelChanged);
    assert_ne!(outcome.verdict, RebuildVerdict::RebuiltIdentical);
    assert!(
        outcome.detail.contains("换代"),
        "detail: {}",
        outcome.detail
    );
}

/// 换代 + 向量确实变了 ⇒ 仍然是「换代」（且向量确实被重写）。
#[tokio::test]
async fn model_change_with_different_vectors_is_flagged() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let before = dir.path().join("before.db");
    let first_embedder = DeterministicEmbedder::new("model-a", 4);
    let second_embedder = SaltedEmbedder {
        model: "model-b".into(),
        dim: 4,
    };

    rebuild_vector_index(&path, &records(), &first_embedder)
        .await
        .expect("first rebuild");
    // 留一份旧索引副本 ⇒ 比对「换代前后」向量确实变了。
    std::fs::copy(&path, &before).expect("copy index");

    let outcome = rebuild_vector_index(&path, &records(), &second_embedder)
        .await
        .expect("second rebuild");

    assert_eq!(outcome.verdict, RebuildVerdict::ModelChanged);
    assert_eq!(outcome.entries, 3, "换代也要把索引重建出来");
    assert_eq!(
        compare_vectors(&before, &path, DEFAULT_TOLERANCE).expect("compare"),
        (0, 3, 0),
        "换代后向量必须确实变了（否则「换代」只是标签）"
    );
}

/// embed 失败 ⇒ `Err`（不 panic、不跳过）；且**旧索引一个字节都没动**。
#[tokio::test]
async fn embed_failure_returns_err_without_touching_index() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("first rebuild");

    let outcome = rebuild_vector_index(&path, &records(), &FailingEmbedder).await;
    assert!(outcome.is_err(), "embed 失败必须如实报 Err");

    // 失败发生在任何写入之前 ⇒ 旧索引仍在。
    let index = PersistentVectorIndex::open(&path).expect("open index");
    assert_eq!(index.len().expect("len"), 3);
}

/// 空源记录 ⇒ 条目数 0、不 panic。
#[tokio::test]
async fn empty_records_produce_empty_index() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    let outcome = rebuild_vector_index(&path, &[], &embedder)
        .await
        .expect("rebuild");

    assert_eq!(outcome.entries, 0);
    assert_eq!(outcome.verdict, RebuildVerdict::NoBaseline);
    let index = PersistentVectorIndex::open(&path).expect("open index");
    assert_eq!(index.len().expect("len"), 0);
}

/// 空源记录 ⇒ 把上一轮的派生向量**清掉**（重建 = 清空后重新派生）。
#[tokio::test]
async fn empty_records_clear_previous_index() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("first rebuild");

    let outcome = rebuild_vector_index(&path, &[], &embedder)
        .await
        .expect("second rebuild");

    assert_eq!(outcome.entries, 0);
    assert_eq!(outcome.verdict, RebuildVerdict::RebuiltDifferent);
    let index = PersistentVectorIndex::open(&path).expect("open index");
    assert_eq!(index.len().expect("len"), 0);
}

/// 旧索引**没有模型标识** ⇒ 「模型身份未知」**不等于**「模型变了」。
#[tokio::test]
async fn unknown_model_baseline_is_not_reported_as_model_changed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    // 手搓索引 ⇒ 没有 `embed_model_id` ⇒ 快照里模型标识为空。
    handbuilt_index(
        &path,
        &[
            ("a", embedder.embed("苹果派的做法").await.unwrap()),
            ("b", embedder.embed("香蕉船的来历").await.unwrap()),
            ("c", embedder.embed("橙子与柠檬的区别").await.unwrap()),
        ],
    );

    let outcome = rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("rebuild");

    assert_ne!(
        outcome.verdict,
        RebuildVerdict::ModelChanged,
        "模型标识未知时不能谎报「换代」: {}",
        outcome.detail
    );
    assert_eq!(outcome.verdict, RebuildVerdict::RebuiltIdentical);
    assert!(
        outcome.detail.contains("未知"),
        "detail 要说明模型身份未知: {}",
        outcome.detail
    );
}

// ===========================================================================
// read_snapshot
// ===========================================================================

/// 索引文件不存在 ⇒ `Ok(None)`（首次建索引是正常路径，不是错误）。
#[tokio::test]
async fn read_snapshot_missing_file_is_none() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing.db");

    assert!(read_snapshot(&path).expect("read snapshot").is_none());
}

/// 索引存在 ⇒ 快照正确（模型标识 + 条目数）。
#[tokio::test]
async fn read_snapshot_reports_model_and_entries() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("rebuild");

    let snapshot = read_snapshot(&path)
        .expect("read snapshot")
        .expect("snapshot exists");
    assert_eq!(snapshot.stamp.model_id, "model-a");
    assert_eq!(snapshot.entries, 3);
}

/// 手搓索引（没有模型标识）⇒ 快照条目数对、模型标识为空（未知）。
#[tokio::test]
async fn read_snapshot_of_handbuilt_index_has_unknown_model() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");

    handbuilt_index(&path, &[("x", vec![1.0, 0.0]), ("y", vec![0.0, 1.0])]);

    let snapshot = read_snapshot(&path)
        .expect("read snapshot")
        .expect("snapshot exists");
    assert_eq!(snapshot.entries, 2);
    assert!(
        snapshot.stamp.model_id.is_empty(),
        "没有标识就是没有 ⇒ 不能编一个模型名"
    );
}

// ===========================================================================
// compare_vectors
// ===========================================================================

/// 两个相同索引 ⇒ `(全一致, 0, 0)`。
#[tokio::test]
async fn compare_vectors_same_index_is_all_identical() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.db");
    let right = dir.path().join("right.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&left, &records(), &embedder)
        .await
        .expect("left rebuild");
    rebuild_vector_index(&right, &records(), &embedder)
        .await
        .expect("right rebuild");

    assert_eq!(
        compare_vectors(&left, &right, DEFAULT_TOLERANCE).expect("compare"),
        (3, 0, 0)
    );
    // 跟**自己**比也必须是全一致。
    assert_eq!(
        compare_vectors(&left, &left, DEFAULT_TOLERANCE).expect("compare"),
        (3, 0, 0)
    );
}

/// 删掉一条 ⇒ 报「只在一边存在」。
#[tokio::test]
async fn compare_vectors_reports_entry_only_on_one_side() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.db");
    let right = dir.path().join("right.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&left, &records(), &embedder)
        .await
        .expect("left rebuild");
    let two = vec![records()[0].clone(), records()[1].clone()];
    rebuild_vector_index(&right, &two, &embedder)
        .await
        .expect("right rebuild");

    assert_eq!(
        compare_vectors(&left, &right, DEFAULT_TOLERANCE).expect("compare"),
        (2, 0, 1),
        "少一条必须报「只在一边存在」，不能算成「一致」"
    );
}

/// 向量微小差异 ⇒ 容差内算一致、超容差算不一致。
#[tokio::test]
async fn compare_vectors_respects_tolerance() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.db");
    let right = dir.path().join("right.db");

    handbuilt_index(&left, &[("x", vec![1.0, 0.0]), ("y", vec![0.0, 1.0])]);
    handbuilt_index(
        &right,
        &[("x", vec![1.0000001, 0.0]), ("y", vec![0.0, 1.001])],
    );

    // 默认容差 1e-6：x 的差 (~1.2e-7) 在容差内，y 的差 (~1e-3) 超了。
    assert_eq!(
        compare_vectors(&left, &right, DEFAULT_TOLERANCE).expect("compare"),
        (1, 1, 0)
    );
    // 放宽容差 ⇒ 两条都在容差内。
    assert_eq!(
        compare_vectors(&left, &right, 1e-2).expect("compare"),
        (2, 0, 0)
    );
    // 容差收到 0 ⇒ 微小差异也不放过了。
    assert_eq!(
        compare_vectors(&left, &right, 0.0).expect("compare"),
        (0, 2, 0)
    );
}

/// 索引文件不存在 ⇒ `Err`（不能拿「空索引」冒充「没有差异」）。
#[tokio::test]
async fn compare_vectors_missing_file_is_error() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing.db");
    let existing = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    rebuild_vector_index(&existing, &records(), &embedder)
        .await
        .expect("rebuild");

    assert!(compare_vectors(&missing, &existing, DEFAULT_TOLERANCE).is_err());
    assert!(compare_vectors(&existing, &missing, DEFAULT_TOLERANCE).is_err());
}

// ===========================================================================
// B2 端到端验收：删派生索引 → 重建 → 检索结果一致
// ===========================================================================

/// ⚠️ **B2 核心验收**：建索引 → **删索引文件** → 从源重建 → **检索结果一致**。
#[tokio::test]
async fn delete_index_then_rebuild_keeps_search_results() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);
    let source = records();

    rebuild_vector_index(&path, &source, &embedder)
        .await
        .expect("first rebuild");
    let before = search_ids(&path, &embedder, "橙子与柠檬的区别").await;
    assert_eq!(before.len(), 3, "重建前必须有命中可对比");

    // 删派生索引（P1：原始记录还在，丢的只是派生的向量）。
    std::fs::remove_file(&path).expect("remove index");
    assert!(!path.exists());
    // 索引没了 ⇒ 没有旧索引可比（不许拿旧模型冒充）。
    assert!(read_snapshot(&path).expect("read snapshot").is_none());

    let outcome = rebuild_vector_index(&path, &source, &embedder)
        .await
        .expect("rebuild");
    assert_eq!(outcome.verdict, RebuildVerdict::NoBaseline);
    assert_eq!(outcome.entries, 3);

    let after = search_ids(&path, &embedder, "橙子与柠檬的区别").await;
    assert_eq!(before, after, "删派生索引 → 重建 ⇒ 检索结果必须一致");
}

/// ⚠️ **B2 核心验收（不删文件版）**：原地重建 ⇒ `RebuiltIdentical` ⇒ 检索结果一致。
#[tokio::test]
async fn rebuild_in_place_keeps_search_results() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vec.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);
    let source = records();

    rebuild_vector_index(&path, &source, &embedder)
        .await
        .expect("first rebuild");
    let before = search_ids(&path, &embedder, "苹果派的做法").await;

    let outcome = rebuild_vector_index(&path, &source, &embedder)
        .await
        .expect("second rebuild");
    assert_eq!(outcome.verdict, RebuildVerdict::RebuiltIdentical);

    let after = search_ids(&path, &embedder, "苹果派的做法").await;
    assert_eq!(before, after);
}

// ===========================================================================
// 确定性
// ===========================================================================

/// 同一输入两次 ⇒ `RebuildOutcome` 除 `elapsed_ms` 外**逐字相等**。
#[tokio::test]
async fn outcome_is_deterministic_across_runs() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.db");
    let right = dir.path().join("right.db");
    let embedder = DeterministicEmbedder::new("model-a", 4);

    let first = rebuild_vector_index(&left, &records(), &embedder)
        .await
        .expect("first rebuild");
    let second = rebuild_vector_index(&right, &records(), &embedder)
        .await
        .expect("second rebuild");

    assert_eq!(first.verdict, second.verdict);
    assert_eq!(first.entries, second.entries);
    assert_eq!(first.detail, second.detail, "detail 必须可复现");
    // 除 elapsed_ms 外逐字相等（elapsed_ms 是非确定量 ⇒ 只做记录，不参与判定）。
    assert_eq!(
        (first.verdict, first.entries, first.detail.as_str()),
        (second.verdict, second.entries, second.detail.as_str())
    );

    // 同一个索引连续重建两次 ⇒ 第 2 次与第 3 次的判定与说明也必须逐字相等。
    let third = rebuild_vector_index(&left, &records(), &embedder)
        .await
        .expect("third rebuild");
    let fourth = rebuild_vector_index(&left, &records(), &embedder)
        .await
        .expect("fourth rebuild");
    assert_eq!(third.verdict, fourth.verdict);
    assert_eq!(third.detail, fourth.detail);
    assert_eq!(third.entries, fourth.entries);
}
