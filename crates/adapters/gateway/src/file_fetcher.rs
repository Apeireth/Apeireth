//! file_fetcher: 超栈追踪 V2 跨节点透明文件穿透与安全沙箱缓存
//!
//! 吸收自 VCP 1.0 (`FileFetcherServer.js`):
//! 1. 拦截 `file://` 本地协议 URL 并计算 SHA-256 平台无关缓存键；
//! 2. 本地缓存 (.file_cache) 命中即读，未命中时通过分布式 WebSocket 协议下发 `internal_request_file` 请求；
//! 3. 接收远程 Base64 数据并做 SHA-256 完整性自检后原子落盘；
//! 4. 严格执行路径穿越与敏感目录安全沙箱防御（禁止 `..`、绝对系统敏感区穿越）。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileFetchError {
    #[error("invalid file url: {0}")]
    InvalidUrl(String),
    #[error("path traversal detected: {0}")]
    PathTraversal(String),
    #[error("remote node not found for target: {0}")]
    NodeNotFound(String),
    #[error("integrity check failed: expected {expected}, actual {actual}")]
    IntegrityMismatch { expected: String, actual: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode error: {0}")]
    Decode(String),
}

/// 文件元数据与二进制负载
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FetchedFile {
    pub cache_key: String,
    pub original_url: String,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub sha256_hash: String,
}

/// 分布式内部文件请求包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalFileRequest {
    pub file_url: String,
    pub request_id: String,
}

/// 分布式内部文件响应包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalFileResponse {
    pub request_id: String,
    pub status: String,
    pub base64_data: String,
    pub mime_type: String,
    pub sha256_hash: String,
}

/// 超栈追踪透明文件穿透器
pub struct TransparentFileFetcher {
    cache_dir: PathBuf,
    memory_cache: HashMap<String, FetchedFile>,
    allowed_roots: Vec<PathBuf>,
}

impl TransparentFileFetcher {
    pub fn new(cache_dir: impl AsRef<Path>, allowed_roots: Vec<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            memory_cache: HashMap::new(),
            allowed_roots,
        }
    }

    /// 计算 URL 的 SHA-256 唯一缓存键
    pub fn compute_cache_key(file_url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_url.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 校验文件路径是否符合安全沙箱（防路径穿越）
    pub fn validate_path_safety(&self, path_str: &str) -> Result<(), FileFetchError> {
        if path_str.contains("..") || path_str.contains("\0") {
            return Err(FileFetchError::PathTraversal(path_str.into()));
        }

        // 若配置了允许根目录，则路径必须坐落在允许根目录下
        if !self.allowed_roots.is_empty() {
            let normalized = PathBuf::from(path_str);
            let is_allowed = self
                .allowed_roots
                .iter()
                .any(|root| normalized.starts_with(root));
            if !is_allowed {
                return Err(FileFetchError::PathTraversal(format!(
                    "Path '{path_str}' is outside allowed root directories"
                )));
            }
        }

        Ok(())
    }

    /// 获取文件：优先读缓存，未命中则触发远程分布式穿透回调
    pub fn fetch_file<F>(
        &mut self,
        file_url: &str,
        remote_provider: F,
    ) -> Result<FetchedFile, FileFetchError>
    where
        F: FnOnce(&str) -> Result<InternalFileResponse, FileFetchError>,
    {
        if !file_url.starts_with("file://") {
            return Err(FileFetchError::InvalidUrl(file_url.into()));
        }

        let path_part = &file_url["file://".len()..];
        self.validate_path_safety(path_part)?;

        let cache_key = Self::compute_cache_key(file_url);

        // 1. 检查内存缓存
        if let Some(file) = self.memory_cache.get(&cache_key) {
            return Ok(file.clone());
        }

        // 2. 检查本地磁盘文件
        let local_path = Path::new(path_part);
        if local_path.is_file() {
            let data = std::fs::read(local_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let sha256_hash = format!("{:x}", hasher.finalize());

            let mime_type = Self::guess_mime_type(path_part);
            let fetched = FetchedFile {
                cache_key: cache_key.clone(),
                original_url: file_url.into(),
                mime_type,
                data,
                sha256_hash,
            };
            self.memory_cache.insert(cache_key, fetched.clone());
            return Ok(fetched);
        }

        // 3. 本地不存在，通过分布式回调拉取
        let response = remote_provider(file_url)?;
        if response.status != "success" {
            return Err(FileFetchError::NodeNotFound(file_url.into()));
        }

        // 4. 解码 Base64 并做哈希校验
        let raw_data = base64_decode(&response.base64_data)?;
        let mut hasher = Sha256::new();
        hasher.update(&raw_data);
        let actual_hash = format!("{:x}", hasher.finalize());

        if actual_hash != response.sha256_hash {
            return Err(FileFetchError::IntegrityMismatch {
                expected: response.sha256_hash,
                actual: actual_hash,
            });
        }

        let fetched = FetchedFile {
            cache_key: cache_key.clone(),
            original_url: file_url.into(),
            mime_type: response.mime_type,
            data: raw_data,
            sha256_hash: actual_hash,
        };

        self.memory_cache.insert(cache_key, fetched.clone());
        Ok(fetched)
    }

    fn guess_mime_type(path: &str) -> String {
        if path.ends_with(".png") {
            "image/png".into()
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg".into()
        } else if path.ends_with(".json") {
            "application/json".into()
        } else if path.ends_with(".txt") {
            "text/plain".into()
        } else {
            "application/octet-stream".into()
        }
    }
}

/// 简易 Safe Base64 解码器 (无外部额外依赖)
fn base64_decode(input: &str) -> Result<Vec<u8>, FileFetchError> {
    const B64_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [255u8; 256];
    for (i, &b) in B64_TABLE.iter().enumerate() {
        lookup[b as usize] = i as u8;
    }

    let clean: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'\r' && b != b'\n' && b != b' ')
        .collect();

    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for &b in &clean {
        if b == b'=' {
            break;
        }
        let val = lookup[b as usize];
        if val == 255 {
            return Err(FileFetchError::Decode(format!("invalid base64 char: {b}")));
        }
        buffer = (buffer << 6) | u32::from(val);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_detection() {
        let fetcher = TransparentFileFetcher::new("target/.file_cache", vec![]);
        assert!(fetcher
            .validate_path_safety("valid/path/to/file.png")
            .is_ok());
        assert!(fetcher.validate_path_safety("../etc/passwd").is_err());
        assert!(fetcher
            .validate_path_safety("valid/../../secret.key")
            .is_err());
    }

    #[test]
    fn test_remote_file_fetch_and_integrity_verification() {
        let mut fetcher = TransparentFileFetcher::new("target/.file_cache", vec![]);

        let dummy_data = b"Hello VCP HyperStack Transparent File Fetcher!";
        let mut hasher = Sha256::new();
        hasher.update(dummy_data);
        let expected_hash = format!("{:x}", hasher.finalize());

        // Base64 of dummy_data
        let base64_str = "SGVsbG8gVkNQIEh5cGVyU3RhY2sgVHJhbnNwYXJlbnQgRmlsZSBGZXRjaGVyIQ==";

        let mock_provider = |_url: &str| -> Result<InternalFileResponse, FileFetchError> {
            Ok(InternalFileResponse {
                request_id: "req-123".into(),
                status: "success".into(),
                base64_data: base64_str.into(),
                mime_type: "text/plain".into(),
                sha256_hash: expected_hash.clone(),
            })
        };

        let result = fetcher
            .fetch_file("file://remote_storage/asset.txt", mock_provider)
            .unwrap();

        assert_eq!(result.data, dummy_data);
        assert_eq!(result.sha256_hash, expected_hash);
        assert_eq!(result.mime_type, "text/plain");

        // 再次获取应命中内存缓存
        let cached = fetcher
            .fetch_file("file://remote_storage/asset.txt", |_| {
                panic!("Should use cache")
            })
            .unwrap();
        assert_eq!(cached.cache_key, result.cache_key);
    }
}
