//! 错误类型定义.

use thiserror::Error;

/// 顶层错误: 所有 vector 子系统的 fallback error.
#[derive(Debug, Error)]
pub enum VectorError {
    /// SQLite 底层错误.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON 编/解码错误 (不影响主路径; 用于可选 metadata).
    #[error("serde_json error: {0}")]
    Json(#[from] serde_json::Error),

    /// std I/O 错误 (创建 db 目录时).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// 向量维度与已注册的 dimension 不一致.
    #[error("vector dim mismatch: expected {expected}, got {actual}")]
    DimMismatch {
        /// backend 注册/初始化的维度.
        expected: usize,
        /// 实际传入的维度.
        actual: usize,
    },

    /// 维度非法 (<=0).
    #[error("invalid vector dim: {0}")]
    InvalidDim(usize),

    /// 向量为空.
    #[error("empty vector")]
    EmptyVector,

    /// 向量值有 NaN / Inf.
    #[error("non-finite vector value at index {index}: {value}")]
    NonFinite {
        /// 出错的维度下标.
        index: usize,
        /// 实际值.
        value: f32,
    },

    /// 通用错误兜底.
    #[error("vector backend error: {0}")]
    Other(String),
}
