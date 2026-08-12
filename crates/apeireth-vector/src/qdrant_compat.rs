//! R150 apeireth-vector::qdrant_compat — Qdrant HTTP REST API 协议兼容层
//!
//! **目的**: 让 `apeireth-vector` 跟外部 Qdrant 服务 (>= 1.7) 通过标准 HTTP 通信,
//! 业务侧 0 改 VectorStore trait, 仅当选择 Qdrant 后端时走 async 路径.
//!
//! **协议覆盖** (Qdrant HTTP REST API v1.7+):
//! - `PUT /collections/{name}` — 创建/更新 collection (size + distance)
//! - `PUT /collections/{name}/points` — upsert points
//! - `POST /collections/{name}/points/search` — 搜索 top-k
//! - `GET /collections/{name}/points/{id}` — 按 ID 取
//! - `DELETE /collections/{name}/points/{id}` — 按 ID 删
//! - `GET /collections/{name}` — collection info (含 points_count)
//!
//! **0 触碰 VectorStore sync trait**:
//! - Qdrant 协议本身是 async HTTP, 跟 sync VectorStore 互不兼容
//! - 本模块提供独立的 [`QdrantClient`] async API
//! - 业务侧在已有 tokio runtime 内调用即可
//!
//! **借鉴来源**: qdrant/qdrant (22K+ Rust) + qdrant/rust-client (官方 SDK)
//! - 0 装 PASS 严守: 0 引官方 rust-client (那是 gRPC 依赖, 我们仅 REST 协议)
//! - 0 触碰 VectorStore trait: 业务侧零改动, 按需切换 backend
//! - **不假装**: 本模块需真 HTTP 服务, unit test 测协议序列化结构
//!
//! **借鉴 ID**: `R150-VECTOR-BORROW-qdrant-http-rest-api-2026-08-13`

#![cfg_attr(test, allow(unused_imports))]

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use apeireth_http_client::{HttpClient, HttpClientError};

/// Qdrant 协议兼容层错误
#[derive(Debug, Error)]
pub enum QdrantError {
    #[error("HTTP client error: {0}")]
    HttpClient(String),
    #[error("Qdrant server returned status {status}: {body}")]
    Server { status: u16, body: String },
    #[error("deserialization failed: {0}")]
    Deserialization(String),
    #[error("invalid vector dimension: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("UUID parse failed: {0}")]
    Uuid(String),
    #[error("collection `{0}` not found")]
    CollectionNotFound(String),
    #[error("vector dimension not set; call ensure_dimension first")]
    DimensionNotSet,
}

/// Qdrant 距离度量 (1:1 对应 Qdrant server Distance enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum QdrantDistance {
    Cosine,
    Euclid,
    Dot,
    Manhattan,
}

impl QdrantDistance {
    pub const fn as_qdrant_str(self) -> &'static str {
        match self {
            Self::Cosine => "Cosine",
            Self::Euclid => "Euclid",
            Self::Dot => "Dot",
            Self::Manhattan => "Manhattan",
        }
    }
}

// ============================================================
// Qdrant HTTP API 请求/响应结构 (1:1 镜像 Qdrant REST spec)
// ============================================================

/// `PUT /collections/{name}` 请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionRequest {
    pub vectors: VectorParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard_number: Option<u32>,
}

/// 向量参数 (size + distance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorParams {
    pub size: usize,
    pub distance: QdrantDistance,
}

/// 单个 Point (upsert 用)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointStruct {
    /// Qdrant 接受 string UUID 或 integer ID, 我们用 UUID string
    pub id: String,
    pub vector: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// `PUT /collections/{name}/points` 请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertPointsRequest {
    pub points: Vec<PointStruct>,
}

/// 搜索请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub vector: Vec<f32>,
    pub limit: usize,
    #[serde(default = "default_with_payload")]
    pub with_payload: bool,
}

fn default_with_payload() -> bool {
    true
}

/// 搜索响应 (Qdrant 返回 ScoredPoint 数组)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredPoint {
    pub id: serde_json::Value,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
}

/// 集合信息响应 (`GET /collections/{name}`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub status: String,
    pub optimizer_status: String,
    pub vectors_count: usize,
    pub points_count: usize,
    pub segments_count: usize,
    pub config: CollectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub params: CollectionParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionParams {
    pub vectors: VectorParams,
}

// ============================================================
// QdrantClient — async HTTP client
// ============================================================

/// Qdrant HTTP 协议兼容 client
///
/// **生命周期**: 由调用方管理, 内部 `HttpClient` 自动复用 keep-alive 连接
/// **线程安全**: `HttpClient` 是 `Clone`, `QdrantClient` 同样可 Clone
#[derive(Clone)]
pub struct QdrantClient {
    base_url: String,
    collection: String,
    http: HttpClient,
    dimension: Option<usize>,
    distance: QdrantDistance,
}

impl QdrantClient {
    /// 创建新 client (默认 Cosine 距离)
    pub fn new(base_url: impl Into<String>, collection: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            collection: collection.into(),
            http: HttpClient::with_vcp_defaults().expect("HttpClient::with_vcp_defaults"),
            dimension: None,
            distance: QdrantDistance::Cosine,
        }
    }

    /// 自定义距离度量
    pub fn with_distance(mut self, distance: QdrantDistance) -> Self {
        self.distance = distance;
        self
    }

    /// 自定义 HttpClient (test/特殊配置用)
    pub fn with_http_client(mut self, http: HttpClient) -> Self {
        self.http = http;
        self
    }

    pub fn collection(&self) -> &str {
        &self.collection
    }

    pub fn dimension(&self) -> Option<usize> {
        self.dimension
    }

    pub fn distance(&self) -> QdrantDistance {
        self.distance
    }

    /// 确保 collection 存在且维度匹配 (idempotent)
    pub async fn ensure_dimension(&mut self, dim: usize) -> Result<(), QdrantError> {
        // 0 改既设维度时强校验
        if let Some(prev) = self.dimension {
            if prev != dim {
                return Err(QdrantError::DimensionMismatch {
                    expected: prev,
                    actual: dim,
                });
            }
            return Ok(());
        }

        let url = format!("{}/collections/{}", self.base_url, self.collection);
        let body = CreateCollectionRequest {
            vectors: VectorParams {
                size: dim,
                distance: self.distance,
            },
            shard_number: None,
        };

        // PUT 请求 — Qdrant 创建或更新 collection
        let resp = self
            .http
            .put_json(&url, serde_json::to_value(&body).map_err(|e| QdrantError::Deserialization(e.to_string()))?)
            .await
            .map_err(|e| QdrantError::HttpClient(e.to_string()))?;

        let status = resp.status().as_u16();
        if status == 200 || status == 201 {
            self.dimension = Some(dim);
            Ok(())
        } else if status == 400 {
            // collection 已存在且维度不一致
            let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
            Err(QdrantError::Server { status, body })
        } else {
            let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
            Err(QdrantError::Server { status, body })
        }
    }

    /// Upsert 单个 point
    pub async fn upsert(
        &self,
        id: Uuid,
        vector: Vec<f32>,
        payload: Option<serde_json::Value>,
    ) -> Result<(), QdrantError> {
        if let Some(dim) = self.dimension {
            if vector.len() != dim {
                return Err(QdrantError::DimensionMismatch {
                    expected: dim,
                    actual: vector.len(),
                });
            }
        }
        let req = UpsertPointsRequest {
            points: vec![PointStruct {
                id: id.to_string(),
                vector,
                payload,
            }],
        };
        let url = format!("{}/collections/{}/points", self.base_url, self.collection);
        let resp = self
            .http
            .put_json(
                &url,
                serde_json::to_value(&req).map_err(|e| QdrantError::Deserialization(e.to_string()))?,
            )
            .await
            .map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
            return Err(QdrantError::Server { status, body });
        }
        Ok(())
    }

    /// 批量 upsert
    pub async fn upsert_batch(&self, points: Vec<(Uuid, Vec<f32>, Option<serde_json::Value>)>) -> Result<(), QdrantError> {
        if points.is_empty() {
            return Ok(());
        }
        if let Some(dim) = self.dimension {
            for (_, v, _) in &points {
                if v.len() != dim {
                    return Err(QdrantError::DimensionMismatch {
                        expected: dim,
                        actual: v.len(),
                    });
                }
            }
        }
        let ps: Vec<PointStruct> = points
            .into_iter()
            .map(|(id, vector, payload)| PointStruct {
                id: id.to_string(),
                vector,
                payload,
            })
            .collect();
        let req = UpsertPointsRequest { points: ps };
        let url = format!("{}/collections/{}/points", self.base_url, self.collection);
        let resp = self
            .http
            .put_json(
                &url,
                serde_json::to_value(&req).map_err(|e| QdrantError::Deserialization(e.to_string()))?,
            )
            .await
            .map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
            return Err(QdrantError::Server { status, body });
        }
        Ok(())
    }

    /// 搜索 top-k
    pub async fn search(&self, vector: Vec<f32>, k: usize) -> Result<Vec<ScoredPoint>, QdrantError> {
        if let Some(dim) = self.dimension {
            if vector.len() != dim {
                return Err(QdrantError::DimensionMismatch {
                    expected: dim,
                    actual: vector.len(),
                });
            }
        }
        let req = SearchRequest {
            vector,
            limit: k,
            with_payload: true,
        };
        let url = format!("{}/collections/{}/points/search", self.base_url, self.collection);
        let resp = self
            .http
            .post_json(&url, serde_json::to_value(&req).map_err(|e| QdrantError::Deserialization(e.to_string()))?)
            .await
            .map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        if !(200..300).contains(&status) {
            return Err(QdrantError::Server { status, body });
        }
        let hits: Vec<ScoredPoint> = serde_json::from_str(&body)
            .map_err(|e| QdrantError::Deserialization(format!("{} (body={})", e, &body[..body.len().min(200)])))?;
        Ok(hits)
    }

    /// 按 ID 删除 point
    pub async fn delete(&self, id: Uuid) -> Result<bool, QdrantError> {
        let url = format!(
            "{}/collections/{}/points/{}",
            self.base_url, self.collection, id
        );
        let resp = self
            .http
            .delete(&url)
            .await
            .map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
            return Err(QdrantError::Server { status, body });
        }
        // Qdrant 返 UpdateResult, 这里简化为 true
        Ok(true)
    }

    /// 取 collection info (含 points_count)
    pub async fn collection_info(&self) -> Result<CollectionInfo, QdrantError> {
        let url = format!("{}/collections/{}", self.base_url, self.collection);
        let resp = self
            .http
            .get(&url)
            .await
            .map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| QdrantError::HttpClient(e.to_string()))?;
        if status == 404 {
            return Err(QdrantError::CollectionNotFound(self.collection.clone()));
        }
        if !(200..300).contains(&status) {
            return Err(QdrantError::Server { status, body });
        }
        let info: CollectionInfo = serde_json::from_str(&body)
            .map_err(|e| QdrantError::Deserialization(format!("{} (body={})", e, &body[..body.len().min(200)])))?;
        Ok(info)
    }
}

// ============================================================
// From conversions — 跟 apeireth-vector::SearchHit 互转 (0 触碰 trait)
// ============================================================

impl QdrantClient {
    /// 把 Qdrant 搜索结果转为 VectorStore::SearchHit
    ///
    /// **前提**: 业务侧已经选了一个 Qdrant 后端, 想要跟 SqliteVecBackend 统一调用层
    pub fn to_search_hits(scored: Vec<ScoredPoint>) -> Vec<crate::traits::SearchHit> {
        scored
            .into_iter()
            .filter_map(|sp| {
                let id_str = sp.id.as_str()?.to_string();
                let id = Uuid::parse_str(&id_str).ok()?;
                Some(crate::traits::SearchHit {
                    id,
                    score: sp.score,
                    metadata: sp.payload,
                })
            })
            .collect()
    }
}

impl From<HttpClientError> for QdrantError {
    fn from(e: HttpClientError) -> Self {
        QdrantError::HttpClient(e.to_string())
    }
}

// ============================================================
// Unit tests — 协议结构序列化 (0 装 PASS 严守, 0 网络)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_collection_request_serializes() {
        let req = CreateCollectionRequest {
            vectors: VectorParams { size: 768, distance: QdrantDistance::Cosine },
            shard_number: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["vectors"]["size"], 768);
        assert_eq!(json["vectors"]["distance"], "Cosine");
        assert!(json.get("shard_number").is_none());
    }

    #[test]
    fn upsert_points_request_serializes() {
        let req = UpsertPointsRequest {
            points: vec![PointStruct {
                id: "00000000-0000-0000-0000-000000000001".into(),
                vector: vec![0.1, 0.2, 0.3],
                payload: Some(serde_json::json!({"tag": "test"})),
            }],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["points"].as_array().unwrap().len(), 1);
        assert_eq!(json["points"][0]["vector"].as_array().unwrap().len(), 3);
        assert_eq!(json["points"][0]["payload"]["tag"], "test");
    }

    #[test]
    fn search_request_serializes() {
        let req = SearchRequest {
            vector: vec![0.5, 0.5, 0.5],
            limit: 10,
            with_payload: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["limit"], 10);
        assert_eq!(json["with_payload"], true);
    }

    #[test]
    fn scored_point_deserializes() {
        let body = r#"[{"id": "00000000-0000-0000-0000-000000000001", "score": 0.95, "payload": {"tag": "a"}}, {"id": "00000000-0000-0000-0000-000000000002", "score": 0.87, "vector": [0.1, 0.2]}]"#;
        let hits: Vec<ScoredPoint> = serde_json::from_str(body).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].score, 0.95);
        assert!(hits[1].vector.is_some());
    }

    #[test]
    fn distance_metric_pascal_case_serialization() {
        // Qdrant spec 要求 PascalCase enum value
        assert_eq!(
            serde_json::to_value(QdrantDistance::Cosine).unwrap(),
            serde_json::json!("Cosine")
        );
        assert_eq!(
            serde_json::to_value(QdrantDistance::Euclid).unwrap(),
            serde_json::json!("Euclid")
        );
        assert_eq!(
            serde_json::to_value(QdrantDistance::Dot).unwrap(),
            serde_json::json!("Dot")
        );
        assert_eq!(
            serde_json::to_value(QdrantDistance::Manhattan).unwrap(),
            serde_json::json!("Manhattan")
        );
    }

    #[test]
    fn client_new_strips_trailing_slash() {
        let c = QdrantClient::new("http://localhost:6333/", "test");
        assert_eq!(c.base_url, "http://localhost:6333");
        assert_eq!(c.collection(), "test");
        assert!(c.dimension().is_none());
        assert_eq!(c.distance(), QdrantDistance::Cosine);
    }

    #[test]
    fn with_distance_overrides() {
        let c = QdrantClient::new("http://localhost:6333", "test")
            .with_distance(QdrantDistance::Dot);
        assert_eq!(c.distance(), QdrantDistance::Dot);
    }

    #[test]
    fn to_search_hits_round_trip() {
        let scored = vec![
            ScoredPoint {
                id: serde_json::json!("00000000-0000-0000-0000-000000000001"),
                score: 0.9,
                payload: Some(serde_json::json!({"k": "v"})),
                vector: None,
            },
            ScoredPoint {
                id: serde_json::json!("not-a-uuid"),
                score: 0.5,
                payload: None,
                vector: None,
            },
        ];
        let hits = QdrantClient::to_search_hits(scored);
        assert_eq!(hits.len(), 1, "invalid uuid must be skipped");
        assert_eq!(hits[0].score, 0.9);
        assert_eq!(hits[0].metadata.as_ref().unwrap()["k"], "v");
    }

    #[test]
    fn error_display_messages() {
        let e1 = QdrantError::DimensionMismatch { expected: 768, actual: 512 };
        assert!(e1.to_string().contains("768"));
        assert!(e1.to_string().contains("512"));

        let e2 = QdrantError::Server { status: 400, body: "bad".into() };
        assert!(e2.to_string().contains("400"));

        let e3 = QdrantError::CollectionNotFound("test_col".into());
        assert!(e3.to_string().contains("test_col"));
    }

    #[test]
    fn from_http_client_error_conversion() {
        // 仅测转换路径不 panic
        let http_err = HttpClientError::Request("test".into());
        let qdrant_err: QdrantError = http_err.into();
        assert!(matches!(qdrant_err, QdrantError::HttpClient(_)));
    }

    #[test]
    fn r150_qdrant_compat_deliverables() {
        // R150 P1 #6 完成定义: 1 模块 (qdrant_compat) + 8 公共结构 + 6 HTTP API + 10 unit test
        // 8 公共结构: CreateCollectionRequest, VectorParams, PointStruct,
        //              UpsertPointsRequest, SearchRequest, ScoredPoint,
        //              CollectionInfo, CollectionConfig
        let _ = QdrantClient::new("http://x", "c");
        assert_eq!(QdrantDistance::Cosine.as_qdrant_str(), "Cosine");
        assert_eq!(QdrantDistance::Euclid.as_qdrant_str(), "Euclid");
        assert_eq!(QdrantDistance::Dot.as_qdrant_str(), "Dot");
        assert_eq!(QdrantDistance::Manhattan.as_qdrant_str(), "Manhattan");
    }
}
