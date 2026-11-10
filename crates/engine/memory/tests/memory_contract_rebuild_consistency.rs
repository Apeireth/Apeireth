//! **B2 批：`vector_rebuild` 的运维入口 + 「重建前后检索结果一致」真跑证据**。
//!
//! # 这个文件证明什么
//!
//! 1. ⭐ **重建前后检索结果逐项相同**（同一 embedder ⇒ 同一查询向量 ⇒ 同一条 top-k 序列）
//!    ⇒ ⇒ 重建**没有改变召回**（比「向量逐条相同」更强的证据）；
//! 2. ⭐ **派生索引可删了重建**：重建入口走的是既有 `rebuild_vector_index`
//!    （先读旧向量 ⇒ 再 embed ⇒ 失败则**索引一个字节都没动** ⇒ 最后回读落盘结果再比对）；
//! 3. ⭐ **模型变了 ⇒ 不假装一致**（换代检测）⇒ 结论写成 `ModelChanged`，不写「一致」。
//!
//! # 为什么重建是「运维入口」而不是「读路径」
//! 重建是**运维动作**（删了重建 + 比对）⇒ ⇒ 放进每请求的读路径会把一次运维变成
//! 「每次召回都重建一遍」⇒ ⇒ 读路径只读、重建走 [`rebuild_and_verify_retrieval`]。
//!
//! # 中性表述
//!
//! 只描述**机制**（派生索引 / 重建 / 换代检测 / 检索一致性），不引用任何外部产品名。

use apeireth_memory::scope::EmbeddingProvider;
use apeireth_memory::vector_rebuild::{
    rebuild_and_verify_retrieval, rebuild_vector_index, RebuildVerdict, SourceRecord,
};
use std::path::PathBuf;

/// 确定性 embedder（**同文本 ⇒ 同向量** ⇒ 重建前后可比）。
struct Deterministic;

#[async_trait::async_trait]
impl EmbeddingProvider for Deterministic {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, apeireth_memory::scope::EmbeddingError> {
        // 4 维确定性向量：按字符码归一化 ⇒ 稳定、可复现。
        let mut vector = vec![0.0_f32; 4];
        for (index, ch) in text.chars().enumerate() {
            vector[index % 4] += ch as u32 as f32;
        }
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in vector.iter_mut() {
                *value /= norm;
            }
        }
        Ok(vector)
    }

    fn model_id(&self) -> &str {
        "deterministic-test-model"
    }
}

fn records() -> Vec<SourceRecord> {
    vec![
        SourceRecord {
            id: "a".to_string(),
            text: "记忆系统 v2 的设计要点".to_string(),
        },
        SourceRecord {
            id: "b".to_string(),
            text: "记忆系统的检索管线与预算纪律".to_string(),
        },
        SourceRecord {
            id: "c".to_string(),
            text: "完全无关的一段文字".to_string(),
        },
    ]
}

fn temp_index_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-rebuild-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir.join("vectors.usearch")
}

/// ⭐⭐ **重建前后检索结果逐项相同**（真跑）。
#[tokio::test]
async fn rebuilding_the_derived_index_keeps_the_retrieval_results_identical() {
    let path = temp_index_path("same");
    let embedder = Deterministic;

    // ① 首次建索引（没有「重建前」可比）。
    let first = rebuild_vector_index(&path, &records(), &embedder)
        .await
        .expect("首次建索引");
    assert_eq!(
        first.verdict,
        RebuildVerdict::NoBaseline,
        "首次建索引没有旧索引可比"
    );

    // ② 重建 + 检索一致性复核。
    let check = rebuild_and_verify_retrieval(&path, &records(), &embedder, "记忆系统", 3)
        .await
        .expect("重建 + 复核");
    assert_eq!(
        check.verdict,
        RebuildVerdict::RebuiltIdentical,
        "同模型重建 ⇒ 向量逐条一致：{}",
        check.detail
    );
    assert!(
        check.identical,
        "重建前后 top-k 检索结果必须**逐项相同**：\n{}",
        check.detail
    );
    assert_eq!(check.before, check.after, "两条序列必须逐项相同");
    assert!(!check.after.is_empty(), "重建后必须检索得到结果");
}

/// ⭐ **模型变了 ⇒ 报「换代」，不假装一致**。
#[tokio::test]
async fn a_changed_model_is_reported_as_a_generation_change_not_as_consistency() {
    struct OtherModel;
    #[async_trait::async_trait]
    impl EmbeddingProvider for OtherModel {
        async fn embed(
            &self,
            text: &str,
        ) -> Result<Vec<f32>, apeireth_memory::scope::EmbeddingError> {
            Deterministic.embed(text).await
        }
        fn model_id(&self) -> &str {
            "another-model"
        }
    }

    let path = temp_index_path("model");
    let _ = rebuild_vector_index(&path, &records(), &Deterministic)
        .await
        .expect("先建索引");

    let check = rebuild_and_verify_retrieval(&path, &records(), &OtherModel, "记忆系统", 3)
        .await
        .expect("重建 + 复核");
    assert_eq!(
        check.verdict,
        RebuildVerdict::ModelChanged,
        "模型不同 ⇒ 报换代，不假装一致：{}",
        check.detail
    );
}
