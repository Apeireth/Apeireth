//! `apeireth-memory::offline_embedding_provider` — **纯本地** embedder 接进
//! [`crate::scope::EmbeddingProvider`] 的**适配器**。
//!
//! # 为什么需要这个适配器（**别把它当重复实现**）
//!
//! 仓里有两个**互不相通**的 embedder 抽象：
//!
//! | 抽象 | 位置 | 形状 | 谁在用 |
//! |---|---|---|---|
//! | `apeireth_orchestration::Embedder` | foundation | **同步** `fn embed(&str) -> Vec<f32>` | `examples/semantic_ab.rs` 等对照实验 |
//! | `apeireth_memory::scope::EmbeddingProvider` | engine | **异步** `async fn embed(&str) -> Result<Vec<f32>, EmbeddingError>` | **生产检索路径**（`MemoryCoordinator`） |
//!
//! ⇒ ⇒ [`crate::offline_embed::OfflineEmbedder`] 实现的是**前者**
//! ⇒ ⇒ 生产检索要的是**后者**
//! ⇒ ⇒ **中间必须有一层适配器**（本模块）——
//! **不是**「再写一个 embedder」，而是**把已有的纯本地实现接到生产的那个口上**。
//!
//! # 口径（与 [`crate::offline_embed`] 逐字一致 ⇒ **不许漂**）
//!
//! * **确定性**：同文本 ⇒ 同向量（FNV-1a + murmur3 finalizer ⇒ 跨工具链稳定）；
//! * **不依赖网络**（离线可用 ⇒ 这正是它的存在理由）；
//! * **维度固定**（维度不匹配 ⇒ 余弦恒 0 ⇒ 静默失效 ⇒ 宁可**显式报错**）。
//!
//! # 状态（**诚实口径**，接线前后都别夸大）
//!
//! * **IMPLEMENTED** ✔：类型 + 适配 + 单测齐备（库级实现）；
//! * **PRODUCTION WIRED** ✘：**生产注入点在 `crates/adapters/cli/src/lib.rs` 与
//!   `crates/engine/runtime-assembly/src/canonical/production.rs`** ——
//!   本批**未获授权**改这两个文件（并行线占用）⇒ ⇒ **生产仍是 0 注入点**
//!   ⇒ ⇒ 对外一律按 **NOT IMPLEMENTED（新增未接线）** 表述。
//!
//! # 为什么「离线也要有向量通道」
//!
//! 生产 embedding 走 `OpenAiCompatibleEmbeddingProvider`（需 env URL + key + 联网）
//! ⇒ ⇒ **离线时语义通道整个不存在** ⇒ ⇒ 检索退化成**纯词面**
//! ⇒ ⇒ 本适配器 = 「**离线时的向量通道**」的**零件**（不是**通电**）。
//!
//! ⚠️ **它不是语义模型**：字符 n-gram 哈希投影 ⇒ 相似 = **字符面相似**
//! ⇒ ⇒ 它回答的是「**词面饱和时，字符级信号还有没有用**」
//! ⇒ ⇒ **不能**据此宣称「语义召回行不行」（那需要真模型 + 网络）。

use async_trait::async_trait;

use crate::offline_embed::{OfflineEmbedder, DEFAULT_DIMENSION};
use crate::scope::{EmbeddingError, EmbeddingProvider};

/// 本地 embedder 的**模型标识**（`EmbeddingProvider::model_id`）。
///
/// ⚠️ **必须稳定且可区分** ⇒ ⇒ 向量换代检测（`vector_rebuild`）靠它认「这批向量
/// 是哪个模型产的」⇒ ⇒ 改它 = 全部向量作废。
pub const OFFLINE_EMBEDDING_MODEL_ID: &str = "offline-char-ngram-v1";

/// **纯本地**的 [`EmbeddingProvider`] 实现（无 API key、无网络）。
///
/// # 定位（别误读）
///
/// 它是**词袋的推广**（字符级），**不是**语义模型
/// ⇒ ⇒ 相似 ⇒ **字符面相似**，不代表「意思相近」。
#[derive(Debug, Clone)]
pub struct OfflineEmbeddingProvider {
    /// 内嵌的**同步** embedder（[`OfflineEmbedder`]）。
    embedder: OfflineEmbedder,
}

impl OfflineEmbeddingProvider {
    /// 构造（`dimension = 0` ⇒ **直接拒绝**：零维向量 ⇒ 余弦恒 0 ⇒ 静默失效）。
    pub fn new(dimension: usize) -> Result<Self, String> {
        Ok(Self {
            embedder: OfflineEmbedder::new(dimension)?,
        })
    }

    /// 默认维度（**256** ⇒ 与 `offline_embed::DEFAULT_DIMENSION` 同源）。
    pub fn default_provider() -> Self {
        Self {
            embedder: OfflineEmbedder::default_embedder(),
        }
    }

    /// 当前向量维度。
    pub fn dimension(&self) -> usize {
        self.embedder.dimension
    }
}

impl Default for OfflineEmbeddingProvider {
    fn default() -> Self {
        Self::default_provider()
    }
}

#[async_trait]
impl EmbeddingProvider for OfflineEmbeddingProvider {
    /// 嵌入一段文本（**同步实现** ⇒ 直接算 ⇒ **零 await 成本**）。
    ///
    /// # 错误口径（**显式**，不许静默）
    ///
    /// * **空向量** ⇒ `EmbeddingError::InvalidVector`
    ///   （零维 / 维度被改坏 ⇒ 余弦会恒 0 ⇒ 检索全部并列 ⇒ 静默失效最伤人）；
    /// * **非有限分量**（NaN / Inf）⇒ 同上。
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let vector = self.embedder.embed(text);
        if vector.is_empty() {
            return Err(EmbeddingError::InvalidVector(format!(
                "offline embedder produced an empty vector (dimension = {})",
                self.embedder.dimension
            )));
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(EmbeddingError::InvalidVector(
                "offline embedder produced a non-finite component".to_string(),
            ));
        }
        Ok(vector)
    }

    /// 模型标识（**稳定** ⇒ 向量换代检测靠它）。
    fn model_id(&self) -> &str {
        OFFLINE_EMBEDDING_MODEL_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn embeds_deterministically_and_offline() {
        let provider = OfflineEmbeddingProvider::default_provider();
        let first = provider.embed("我偏好简洁的回答").await.expect("embed");
        let second = provider.embed("我偏好简洁的回答").await.expect("embed");
        assert_eq!(first, second, "同文本必须给出**逐字相等**的向量");
        assert_eq!(first.len(), DEFAULT_DIMENSION);
        assert!(first.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn rejects_zero_dimension() {
        assert!(OfflineEmbeddingProvider::new(0).is_err());
    }

    #[test]
    fn model_id_is_stable() {
        let provider = OfflineEmbeddingProvider::default_provider();
        assert_eq!(provider.model_id(), OFFLINE_EMBEDDING_MODEL_ID);
    }

    #[tokio::test]
    async fn empty_text_still_yields_a_zero_vector_not_an_error() {
        // ⚠️ **全 0 向量不是错误**：它表示「这段文本没有可编码的字符」
        // ⇒ ⇒ 余弦为 0 ⇒ 检索里排最后 ⇒ 语义上正确（不是静默失效）。
        let provider = OfflineEmbeddingProvider::default_provider();
        let vector = provider.embed("").await.expect("embed");
        assert_eq!(vector.len(), DEFAULT_DIMENSION);
        assert!(vector.iter().all(|value| *value == 0.0));
    }
}
