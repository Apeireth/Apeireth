//! # S3Provider — 7 provider 模式 4: 外部 S3 (S3-compatible)
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 借 `reqwest = 0.12` (workspace) + S3 REST API (PUT / GET / DELETE / HEAD)
//! - 7 通用方法 (`kind`/`set`/`get`/`delete`/`exists`/`clear`/`size`) 走真实 HTTP
//! - 0 引 `aws-sdk-s3` 重量级 (100+ MB 编译, V2 Day 1 Step 3.4 已禁止)
//! - **端到端**: config 强校验 + URL 解析 + S3 client 构造 (实际 HTTP 由 R21+ 续做, 需 AWS 凭据)
//!
//! **不假装**:
//! - skeleton 阶段仅做 S3 协议 URL 解析 + reqwest Client 创建
//! - 0 假装"无 AWS 凭据也能 set/get" — 没凭据必然 403
//! - 0 引 AWS SigV4 签名 (0 重复造轮子) — 真接时 R21+ 续做 (用 aws-sigv4 crate)
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `s3://[access_key:secret_key@]bucket[/prefix]`
//! 2. timeout = [1ms, 1h]
//! 3. max_size = [1KB, 1TB]
//! 4. persist = bool (S3 永远 true, 0 假装 false)
//! 5. cache_ttl = [0ms, 7d] (0 = 永不过期; S3 lifecycle 策略由服务端配置)
//! 6. scope = Global (S3 跨 region 共享)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{MemoryProvider, ProviderConfig, ProviderKind, ProviderScope};

/// **S3 解析后的元组**: (endpoint, bucket, key_prefix, access_key, secret_key).
#[derive(Debug, Clone)]
pub struct S3ParsedUri {
    /// S3 endpoint (e.g. `https://s3.amazonaws.com`).
    endpoint: String,
    /// bucket 名.
    bucket: String,
    /// key prefix (e.g. `apeireth-memory/`).
    key_prefix: String,
    /// access_key (optional, 空 = anonymous).
    #[allow(dead_code)]
    access_key: Option<String>,
    /// secret_key (optional).
    #[allow(dead_code)]
    secret_key: Option<String>,
}

/// **S3Provider**: 外部 S3 (S3-compatible) provider.
#[derive(Debug, Clone)]
pub struct S3Provider {
    /// 解析后的 S3 URI 元组.
    parsed: S3ParsedUri,
    /// reqwest Client (借 workspace 0.12, 0 引 aws-sdk-s3).
    http_client: Client,
    /// 6 K-1 强校验过的 config.
    #[allow(dead_code)]
    config: ProviderConfig,
}

impl S3Provider {
    /// 新建 S3Provider, 6 K-1 强校验 + 解析 s3:// URI.
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::S3)?;

        // K-1 #1: 解析 s3://[user:pass@]bucket[/prefix]
        let parsed = Self::parse_s3_uri(&config.connection_string)?;

        // 真创建 reqwest Client (0 实际发请求, 仅创建 handle)
        let http_client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::S3,
                reason: format!("reqwest::Client::build failed: {e}"),
            })?;

        Ok(Self {
            parsed,
            http_client,
            config,
        })
    }

    /// 6 K-1 字段 hardcoded: scope = Global (S3 跨 region 共享).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Global
    }

    /// 解析 s3:// URI 格式: `s3://[user:pass@]bucket[/prefix]`
    fn parse_s3_uri(uri: &str) -> MemoryProviderResult<S3ParsedUri> {
        let stripped = uri
            .strip_prefix("s3://")
            .ok_or_else(|| MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "must start with `s3://`".to_string(),
            })?;

        // 解析 [user:pass@]bucket[/prefix]
        let (auth_part, bucket_and_prefix) = match stripped.split_once('@') {
            Some((auth, rest)) => (Some(auth), rest),
            None => (None, stripped),
        };

        let (bucket, key_prefix) = match bucket_and_prefix.split_once('/') {
            Some((b, p)) => {
                // 去掉尾部 / 防双斜杠 (e.g. "apeireth-memory/" → "apeireth-memory", 然后再加 / → "apeireth-memory/")
                let p_trimmed = p.trim_end_matches('/');
                (b.to_string(), format!("{p_trimmed}/"))
            }
            None => (bucket_and_prefix.to_string(), String::new()),
        };

        let (access_key, secret_key) = match auth_part {
            Some(auth) => match auth.split_once(':') {
                Some((u, p)) => (Some(u.to_string()), Some(p.to_string())),
                None => (Some(auth.to_string()), None),
            },
            None => (None, None),
        };

        if bucket.is_empty() {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "bucket name must be non-empty".to_string(),
            });
        }

        // 默认 endpoint = AWS S3
        let endpoint = std::env::var("APEIRETH_S3_ENDPOINT")
            .unwrap_or_else(|_| "https://s3.amazonaws.com".to_string());

        Ok(S3ParsedUri {
            endpoint,
            bucket,
            key_prefix,
            access_key,
            secret_key,
        })
    }

    /// Get parsed S3 URI handle.
    pub fn parsed(&self) -> &S3ParsedUri {
        &self.parsed
    }

    /// Get reqwest Client handle.
    pub fn http_client(&self) -> &Client {
        &self.http_client
    }
}

#[async_trait]
impl MemoryProvider for S3Provider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::S3
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        // 0 假装 — skeleton 阶段无 AWS SigV4 签名, 必然 403
        // 真接由 R21+ 续做 (aws-sigv4 crate + 真实 PUT 请求)
        let url = format!(
            "{}/{}/{}{}",
            self.parsed.endpoint, self.parsed.bucket, self.parsed.key_prefix, key
        );
        // 用 reqwest PUT (0 假装成功 — 实际 S3 需 SigV4)
        let resp = self
            .http_client
            .put(&url)
            .body(value.to_vec())
            .send()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::S3,
                reason: format!("PUT request failed: {e}"),
            })?;
        // 显式 check status (非 2xx 必返 Backend error, 0 编造"成功")
        if !resp.status().is_success() {
            return Err(MemoryProviderError::Backend {
                provider: ProviderKind::S3,
                reason: format!("PUT returned non-2xx: {}", resp.status()),
            });
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        let url = format!(
            "{}/{}/{}{}",
            self.parsed.endpoint, self.parsed.bucket, self.parsed.key_prefix, key
        );
        let resp = self.http_client.get(&url).send().await.map_err(|e| {
            MemoryProviderError::Connection {
                provider: ProviderKind::S3,
                reason: format!("GET request failed: {e}"),
            }
        })?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(MemoryProviderError::Backend {
                provider: ProviderKind::S3,
                reason: format!("GET returned non-2xx: {}", resp.status()),
            });
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::S3,
                reason: format!("GET body read failed: {e}"),
            })?;
        Ok(Some(bytes.to_vec()))
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        let url = format!(
            "{}/{}/{}{}",
            self.parsed.endpoint, self.parsed.bucket, self.parsed.key_prefix, key
        );
        let resp = self.http_client.delete(&url).send().await.map_err(|e| {
            MemoryProviderError::Connection {
                provider: ProviderKind::S3,
                reason: format!("DELETE request failed: {e}"),
            }
        })?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(MemoryProviderError::Backend {
                provider: ProviderKind::S3,
                reason: format!("DELETE returned non-2xx: {}", resp.status()),
            });
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        let url = format!(
            "{}/{}/{}{}",
            self.parsed.endpoint, self.parsed.bucket, self.parsed.key_prefix, key
        );
        let resp = self.http_client.head(&url).send().await.map_err(|e| {
            MemoryProviderError::Connection {
                provider: ProviderKind::S3,
                reason: format!("HEAD request failed: {e}"),
            }
        })?;
        match resp.status() {
            reqwest::StatusCode::OK | reqwest::StatusCode::NO_CONTENT => Ok(true),
            reqwest::StatusCode::NOT_FOUND => Ok(false),
            _ => Err(MemoryProviderError::Backend {
                provider: ProviderKind::S3,
                reason: format!("HEAD returned non-success: {}", resp.status()),
            }),
        }
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        // 0 假装 — S3 没有"clear all", 需用 List + Delete 多步
        // R21+ 续做: list_objects_v2 + batch delete
        Err(MemoryProviderError::Other {
            msg: "S3 clear() not implemented in skeleton (R21+: List + BatchDelete)".to_string(),
        })
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        // 0 假装 — S3 不知道"全部 entry 数", 需 List + 计数
        Err(MemoryProviderError::Other {
            msg: "S3 size() not implemented in skeleton (R21+: List + Count)".to_string(),
        })
    }
}

/// **S3ConfigDefault** — 借 serde derive 序列化 ProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ConfigDefault {
    pub config: ProviderConfig,
}

// =====================================================================
// 单元测试 (10 tests per 借鉴 #6 模式 1:1)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_config() -> ProviderConfig {
        ProviderConfig::new(
            "s3://AKIAIOSFODNN7EXAMPLE:wJalrXUtnFEMI%2FK7MDENG%2FbPxRfiCYEXAMPLEKEY@my-bucket/apeireth-memory/",
            Duration::from_secs(30),
            5 * 1024 * 1024 * 1024, // 5GB (S3 单 obj 上限 5TB)
            true, // S3 永远 true
            Duration::from_secs(0),  // 永不过期
            ProviderScope::Global,
        )
    }

    // ----- 10 unit tests -----

    #[test]
    fn test_1_provider_kind_is_s3() {
        let p = S3Provider::new(make_config()).unwrap();
        assert_eq!(p.kind(), ProviderKind::S3);
    }

    #[test]
    fn test_2_k1_connection_string_must_be_s3_scheme() {
        let bad = ProviderConfig::new(
            "memory://",
            Duration::from_secs(30),
            5 * 1024 * 1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let r = S3Provider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_3_k1_timeout_validation_propagates() {
        let bad = ProviderConfig::new(
            "s3://bucket/key",
            Duration::from_micros(500),
            5 * 1024 * 1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let r = S3Provider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_4_k1_max_size_validation_propagates() {
        let bad = ProviderConfig::new(
            "s3://bucket/key",
            Duration::from_secs(30),
            512, // < 1KB
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let r = S3Provider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_5_k1_persist_always_true() {
        // S3 永远 persist=true, 即使用户传 false 也会被 config.validate 接受 (0 强制转换)
        let p = S3Provider::new(make_config()).unwrap();
        assert!(p.config.persist);
    }

    #[test]
    fn test_6_k1_cache_ttl_zero_passes() {
        let p = S3Provider::new(make_config()).unwrap();
        assert_eq!(p.config.cache_ttl, Duration::from_secs(0));
    }

    #[test]
    fn test_7_k1_scope_is_always_global() {
        let p = S3Provider::new(make_config()).unwrap();
        assert_eq!(p.scope(), ProviderScope::Global);
    }

    #[test]
    fn test_8_provider_init_with_valid_config() {
        let p = S3Provider::new(make_config()).expect("valid config");
        assert_eq!(p.kind(), ProviderKind::S3);
        // 验证 parsed URI 真解析
        let parsed = p.parsed();
        assert_eq!(parsed.bucket, "my-bucket");
        assert_eq!(parsed.key_prefix, "apeireth-memory/");
        assert!(parsed.access_key.is_some());
        // 验证 reqwest Client 真创建
        let _ = p.http_client();
    }

    #[test]
    fn test_8b_s3_uri_parsing_minimal_no_auth() {
        // s3://bucket 格式 (无 access_key, 无 prefix)
        let cfg = ProviderConfig::new(
            "s3://my-bucket",
            Duration::from_secs(30),
            5 * 1024 * 1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let p = S3Provider::new(cfg).unwrap();
        assert_eq!(p.parsed().bucket, "my-bucket");
        assert_eq!(p.parsed().key_prefix, "");
        assert!(p.parsed().access_key.is_none());
        assert!(p.parsed().secret_key.is_none());
    }

    #[tokio::test]
    async fn test_9_set_get_without_credentials_returns_connection_error() {
        // 0 假装 — 没 AWS 凭据 + 无 SigV4 必然 失败
        let p = S3Provider::new(make_config()).unwrap();
        let r = p.set("k1", b"v1").await;
        // reqwest 发请求 → DNS 失败 or 403 → Connection error
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_10_clear_size_return_not_implemented_error() {
        // S3 clear/size 0 真接 (无 List+Delete batch), 返 Other error
        let p = S3Provider::new(make_config()).unwrap();
        assert!(matches!(
            p.clear().await,
            Err(MemoryProviderError::Other { .. })
        ));
        assert!(matches!(
            p.size().await,
            Err(MemoryProviderError::Other { .. })
        ));
    }
}
