//! # FileProvider — 9 provider 模式 7: 本地 JSON-Lines append-only 文件
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 内部用 `Arc<Mutex<HashMap<String, Vec<u8>>>>` 持内存索引 (跟 DiskLru 同模式)
//! - `set` 走 HashMap insert + JSON-Lines append (`fs_err::OpenOptions::append`)
//! - `get` 走 HashMap get clone (启动时 replay JSON-Lines 重建索引)
//! - `delete` 走 HashMap remove + JSON-Lines append `del` marker
//! - `clear` 走 truncate 文件 + 清 HashMap
//! - **端到端可测**: 集成测试用 `tempfile::TempDir` 真写真读
//!
//! **不假装**:
//! - 跟 DiskLru 的区别: 不用 LRU 淘汰策略 (full retention until clear), append-only 日志式
//! - skeleton 阶段 sync 锁, 0 引 tokio
//! - `connection_string = "file-jsonl://<dir>/<file>.jsonl"` 必须以 `file-jsonl://` 开头
//! - 0 假装 "thread-safe concurrent flush", 1 次 fsync/append 是 std 默认 (R23+ 续做 tokio fs)
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `file-jsonl://<path>` (本地路径, 必须可创建)
//! 2. timeout = [1ms, 1h]
//! 3. max_size = [1KB, 1TB] (文件总 bytes, 超限返 Capacity)
//! 4. persist = bool (true = 文件持久化, false = 仅写入不读回)
//! 5. cache_ttl = [0ms, 7d] (File 不主动 expire, 仅供接口一致)
//! 6. scope = Local (单进程文件)

use fs_err::{File, OpenOptions};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{MemoryProvider, ProviderConfig, ProviderKind, ProviderScope};

/// **FileProvider**: 本地 JSON-Lines append-only 文件 provider (per R23 #6 派工).
#[derive(Debug)]
pub struct FileProvider {
    /// 内部 `Arc<Mutex<HashMap<String, Vec<u8>>>>` (内存索引, 启动时从文件 replay).
    inner: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// JSON-Lines 文件路径 (从 connection_string 解析).
    file_path: PathBuf,
    /// 6 K-1 强校验过的 config.
    config: ProviderConfig,
    /// 当前文件占用 bytes.
    current_size: Arc<Mutex<u64>>,
}

/// **FileConfig 编译期守门**: connection_string 必须以 `file-jsonl://` 开头.
const FILE_SCHEME: &str = "file-jsonl://";

/// JSON-Lines 文件中每一行的 JSON 记录.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileLine {
    /// 操作类型 (put / del).
    op: FileOp,
    /// key.
    key: String,
    /// value (base64 编码的字节, None = del).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

/// 操作类型.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum FileOp {
    /// 写入.
    #[serde(rename = "put")]
    Put,
    /// 删除.
    #[serde(rename = "del")]
    Delete,
}

/// **FileProvider 编译期守门**: base64 字母表 (RFC 4648 §4 standard alphabet).
const B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let b0 = input[i];
        let b1 = input[i + 1];
        let b2 = input[i + 2];
        out.push(B64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(B64_ALPHABET[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        out.push(B64_ALPHABET[((b1 & 0x0F) << 2 | (b2 >> 6)) as usize] as char);
        out.push(B64_ALPHABET[(b2 & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let b0 = input[i];
        out.push(B64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(B64_ALPHABET[((b0 & 0x03) << 4) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = input[i];
        let b1 = input[i + 1];
        out.push(B64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(B64_ALPHABET[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        out.push(B64_ALPHABET[((b1 & 0x0F) << 2) as usize] as char);
        out.push('=');
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut inv = [255u8; 256];
    for (i, &c) in B64_ALPHABET.iter().enumerate() {
        inv[c as usize] = i as u8;
    }
    inv[b'=' as usize] = 0;
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err(format!("length {} not multiple of 4", bytes.len()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i < bytes.len() {
        let s = [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]];
        let mut vals = [0u8; 4];
        let mut pad = 0;
        for (j, &c) in s.iter().enumerate() {
            if c == b'=' {
                vals[j] = 0;
                pad += 1;
            } else {
                let v = inv[c as usize];
                if v == 255 {
                    return Err(format!("invalid char {} at pos {}", c as char, i + j));
                }
                vals[j] = v;
            }
        }
        out.push((vals[0] << 2) | (vals[1] >> 4));
        if pad < 2 {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if pad < 1 {
            out.push((vals[2] << 6) | vals[3]);
        }
        i += 4;
    }
    Ok(out)
}

impl FileProvider {
    /// 新建 FileProvider, 6 K-1 强校验 + replay JSON-Lines 文件.
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::File)?;
        let conn = &config.connection_string;
        if !conn.starts_with(FILE_SCHEME) {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: format!("must start with `{FILE_SCHEME}`, got `{conn}`"),
            });
        }
        let path_str = &conn[FILE_SCHEME.len()..];
        let file_path = PathBuf::from(path_str);
        if let Some(parent) = file_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs_err::create_dir_all(parent).map_err(|e| MemoryProviderError::Connection {
                    provider: ProviderKind::File,
                    reason: format!("create_dir_all({}): {e}", parent.display()),
                })?;
            }
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&file_path)
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::File,
                reason: format!("open({}): {e}", file_path.display()),
            })?;
        let mut index: HashMap<String, Vec<u8>> = HashMap::new();
        let mut size: u64 = 0;
        if config.persist {
            let file = File::open(&file_path).map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::File,
                reason: format!("replay open({}): {e}", file_path.display()),
            })?;
            let reader = BufReader::new(file);
            for (lineno, line) in reader.lines().enumerate() {
                let line = line.map_err(|e| MemoryProviderError::Backend {
                    provider: ProviderKind::File,
                    reason: format!("read line {}: {e}", lineno + 1),
                })?;
                if line.trim().is_empty() {
                    continue;
                }
                let rec: FileLine = serde_json::from_str(&line).map_err(|e| {
                    MemoryProviderError::Serialization {
                        provider: ProviderKind::File,
                        reason: format!("line {}: {e}", lineno + 1),
                    }
                })?;
                size = size.saturating_add(line.len() as u64 + 1);
                match rec.op {
                    FileOp::Put => {
                        if let Some(b64) = rec.value {
                            let bytes = base64_decode(&b64).map_err(|e| {
                                MemoryProviderError::Serialization {
                                    provider: ProviderKind::File,
                                    reason: format!("base64 decode {}: {e}", rec.key),
                                }
                            })?;
                            index.insert(rec.key, bytes);
                        }
                    }
                    FileOp::Delete => {
                        index.remove(&rec.key);
                    }
                }
            }
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(index)),
            file_path,
            config,
            current_size: Arc::new(Mutex::new(size)),
        })
    }

    /// 6 K-1 字段 hardcoded: persist = true (File 持久化默认).
    pub fn is_persistent(&self) -> bool {
        self.config.persist
    }

    /// 6 K-1 字段 hardcoded: scope = Local (File 仅 Local).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Local
    }

    /// 当前文件占用 bytes.
    pub fn file_size(&self) -> u64 {
        let g = self.current_size.lock().ok();
        match g {
            Some(g) => *g,
            None => 0,
        }
    }

    /// 文件路径.
    pub fn path(&self) -> &std::path::Path {
        &self.file_path
    }

    /// 追加一行 JSON-Lines 到文件.
    fn append_line(&self, line: &FileLine) -> MemoryProviderResult<()> {
        let json = serde_json::to_string(line).map_err(|e| MemoryProviderError::Serialization {
            provider: ProviderKind::File,
            reason: format!("serialize: {e}"),
        })?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.file_path)
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("append open: {e}"),
            })?;
        file.write_all(json.as_bytes())
            .and_then(|()| {
                file.write_all(
                    b"
",
                )
            })
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("append write: {e}"),
            })?;
        let mut size = self
            .current_size
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("size lock poisoned: {e}"),
            })?;
        *size = size.saturating_add(json.len() as u64 + 1);
        Ok(())
    }

    /// 校验容量上限 (6 K-1 #3).
    fn check_capacity(&self, incoming: u64) -> MemoryProviderResult<()> {
        let current = *self
            .current_size
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("capacity lock poisoned: {e}"),
            })?;
        if current.saturating_add(incoming) > self.config.max_size {
            return Err(MemoryProviderError::Capacity {
                provider: ProviderKind::File,
                max_size: self.config.max_size,
                current,
            });
        }
        Ok(())
    }
}

#[async_trait]
impl MemoryProvider for FileProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::File
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        if key.is_empty() {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "key must be non-empty".to_string(),
            });
        }
        self.check_capacity(value.len() as u64)?;
        let b64 = base64_encode(value);
        self.append_line(&FileLine {
            op: FileOp::Put,
            key: key.to_string(),
            value: Some(b64),
        })?;
        let mut index = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("index lock poisoned: {e}"),
            })?;
        index.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        if key.is_empty() {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "key must be non-empty".to_string(),
            });
        }
        let index = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("index lock poisoned: {e}"),
            })?;
        Ok(index.get(key).cloned())
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        if key.is_empty() {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "key must be non-empty".to_string(),
            });
        }
        self.append_line(&FileLine {
            op: FileOp::Delete,
            key: key.to_string(),
            value: None,
        })?;
        let mut index = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("index lock poisoned: {e}"),
            })?;
        index.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        if key.is_empty() {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "key must be non-empty".to_string(),
            });
        }
        let index = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("index lock poisoned: {e}"),
            })?;
        Ok(index.contains_key(key))
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        fs_err::write(&self.file_path, b"").map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::File,
            reason: format!("truncate: {e}"),
        })?;
        let mut index = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("index lock poisoned: {e}"),
            })?;
        index.clear();
        let mut size = self
            .current_size
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("size lock poisoned: {e}"),
            })?;
        *size = 0;
        Ok(())
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        let index = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::File,
                reason: format!("index lock poisoned: {e}"),
            })?;
        Ok(index.len() as u64)
    }
}

/// **FileConfigDefault**: FileProvider 6 K-1 默认 config 构造器.
pub struct FileConfigDefault;

impl FileConfigDefault {
    /// 默认 config (connection_string = `file-jsonl://./apeireth-file.jsonl`,
    /// timeout = 1s, max_size = 1MB, persist = true, cache_ttl = 0, scope = Local).
    pub fn build() -> ProviderConfig {
        ProviderConfig::new(
            "file-jsonl://./apeireth-file.jsonl",
            std::time::Duration::from_secs(1),
            1024 * 1024,
            true,
            std::time::Duration::ZERO,
            ProviderScope::Local,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_path(tag: &str) -> (TempDir, String) {
        let dir = TempDir::new().expect("create tempdir");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = dir
            .path()
            .join(format!("apeireth-file-{tag}-{nanos}.jsonl"));
        let conn = format!("{}{}", FILE_SCHEME, path.display());
        (dir, conn)
    }

    fn make_config(conn: &str) -> ProviderConfig {
        ProviderConfig::new(
            conn.to_string(),
            std::time::Duration::from_millis(100),
            1024 * 1024,
            true,
            std::time::Duration::ZERO,
            ProviderScope::Local,
        )
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn kind_is_file() {
        let (_dir, conn) = temp_path("kind");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        assert_eq!(p.kind(), ProviderKind::File);
    }

    #[test]
    fn set_then_get() {
        let (_dir, conn) = temp_path("set-get");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            p.set("k1", b"hello").await.unwrap();
            assert_eq!(p.get("k1").await.unwrap(), Some(b"hello".to_vec()));
        });
    }

    #[test]
    fn get_missing_returns_none() {
        let (_dir, conn) = temp_path("miss");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            assert_eq!(p.get("nope").await.unwrap(), None);
        });
    }

    #[test]
    fn delete_then_missing() {
        let (_dir, conn) = temp_path("del");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            p.set("k", b"v").await.unwrap();
            p.delete("k").await.unwrap();
            assert_eq!(p.get("k").await.unwrap(), None);
            assert!(!p.exists("k").await.unwrap());
        });
    }

    #[test]
    fn exists_returns_bool() {
        let (_dir, conn) = temp_path("exists");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            assert!(!p.exists("nope").await.unwrap());
            p.set("yes", b"v").await.unwrap();
            assert!(p.exists("yes").await.unwrap());
        });
    }

    #[test]
    fn size_after_operations() {
        let (_dir, conn) = temp_path("size");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            assert_eq!(p.size().await.unwrap(), 0);
            p.set("a", b"1").await.unwrap();
            p.set("b", b"2").await.unwrap();
            assert_eq!(p.size().await.unwrap(), 2);
            p.delete("a").await.unwrap();
            assert_eq!(p.size().await.unwrap(), 1);
        });
    }

    #[test]
    fn clear_empties_file_and_index() {
        let (_dir, conn) = temp_path("clear");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            p.set("a", b"1").await.unwrap();
            p.set("b", b"2").await.unwrap();
            p.clear().await.unwrap();
            assert_eq!(p.size().await.unwrap(), 0);
            assert_eq!(p.get("a").await.unwrap(), None);
        });
    }

    #[test]
    fn empty_key_rejected() {
        let (_dir, conn) = temp_path("empty");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            assert!(p.set("", b"v").await.is_err());
            assert!(p.get("").await.is_err());
            assert!(p.delete("").await.is_err());
            assert!(p.exists("").await.is_err());
        });
    }

    #[test]
    fn persistence_across_reopen() {
        let (_dir, conn) = temp_path("persist");
        {
            let p = FileProvider::new(make_config(&conn)).unwrap();
            runtime().block_on(async {
                p.set("persist-key", b"persist-value").await.unwrap();
            });
        }
        let p2 = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            assert_eq!(
                p2.get("persist-key").await.unwrap(),
                Some(b"persist-value".to_vec())
            );
            assert_eq!(p2.size().await.unwrap(), 1);
        });
    }

    #[test]
    fn binary_value_roundtrip() {
        let (_dir, conn) = temp_path("bin");
        let p = FileProvider::new(make_config(&conn)).unwrap();
        runtime().block_on(async {
            let bytes: Vec<u8> = (0u8..=255).cycle().take(513).collect();
            p.set("binary", &bytes).await.unwrap();
            assert_eq!(p.get("binary").await.unwrap(), Some(bytes));
        });
    }

    #[test]
    fn base64_roundtrip_known_vector() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64_decode("Zm8=").unwrap(), b"fo");
        assert_eq!(base64_decode("Zm9v").unwrap(), b"foo");
        assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn file_provider_default_config() {
        let cfg = FileConfigDefault::build();
        assert!(cfg.connection_string.starts_with(FILE_SCHEME));
        assert_eq!(cfg.scope, ProviderScope::Local);
    }
}
