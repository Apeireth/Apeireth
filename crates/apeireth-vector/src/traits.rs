//! VectorStore trait + 关联类型定义.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::VectorError;

/// 一个向量条目 = 业务 ID + 数据 + 可选 metadata.
///
/// `data` 是**未归一化**的原始向量; backend 自行 L2 归一再做余弦.
///
/// ponytail ceiling: 不暴露 fixed-point / quantization 选项, 后续要加再开 v2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vector {
    /// 业务侧唯一 ID (复用 uuid 与 memory 子系统对齐).
    pub id: Uuid,
    /// 浮点向量.
    pub data: Vec<f32>,
    /// 可选 metadata (例如 episode_id / session_id / 摘要).
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl Vector {
    /// 构造一个不带 metadata 的向量.
    pub fn new(id: Uuid, data: Vec<f32>) -> Self {
        Self {
            id,
            data,
            metadata: None,
        }
    }

    /// 构造一个带 metadata 的向量.
    pub fn with_metadata(id: Uuid, data: Vec<f32>, metadata: serde_json::Value) -> Self {
        Self {
            id,
            data,
            metadata: Some(metadata),
        }
    }

    /// 返回向量维度.
    pub fn dim(&self) -> usize {
        self.data.len()
    }
}

/// 检索命中: ID + 余弦相似度.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    /// 业务 ID.
    pub id: Uuid,
    /// 余弦相似度 (-1.0 .. 1.0).
    pub score: f32,
    /// 命中时是否同时拿到 metadata (后端可选择不返回).
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// 仅 ID + score 的精简视图, 用于不需要 metadata 的热路径.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredId {
    /// 业务 ID.
    pub id: Uuid,
    /// 余弦相似度.
    pub score: f32,
}

/// 向量存储抽象.
///
/// backend 实现可以是 SQLite (BLOB) / sqlite-vec 扩展 / lancedb / pgvector 等.
///
/// 全部同步方法: 当前 skeleton 跑通是首要; 异步包装在调用方做.
/// ponytail ceiling: 多线程并发写需要 backend 加 Mutex; 量小 (<1w 条) 同步即可.
pub trait VectorStore: Send {
    /// 注册 / 设置该 backend 的向量维度. 后续 insert / search 都按此维度校验.
    ///
    /// 重复设置**允许** (用于 schema 迁移场景), 但仅在维度一致时通过.
    fn set_dimension(&mut self, dim: usize) -> Result<(), VectorError>;

    /// 当前 backend 维度.
    fn dimension(&self) -> usize;

    /// 当前 backend 内向量总数.
    fn len(&self) -> Result<usize, VectorError>;

    /// 是否为空.
    fn is_empty(&self) -> Result<bool, VectorError> {
        Ok(self.len()? == 0)
    }

    /// 单条插入; ID 已存在则覆盖.
    fn upsert(&mut self, v: &Vector) -> Result<(), VectorError>;

    /// 批量插入; 内部走单事务, 1000 条应 < 1s.
    fn upsert_batch(&mut self, vs: &[Vector]) -> Result<(), VectorError> {
        for v in vs {
            self.upsert(v)?;
        }
        Ok(())
    }

    /// 余弦 top-k 检索.
    ///
    /// `query` 维度必须 == self.dimension(). 返回按 score 降序, 长度 <= k.
    fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchHit>, VectorError>;

    /// 按 ID 删除; 不存在则 no-op.
    fn delete(&mut self, id: Uuid) -> Result<bool, VectorError>;

    /// 清空整个表. 返回前有多少条.
    fn clear(&mut self) -> Result<usize, VectorError>;
}
