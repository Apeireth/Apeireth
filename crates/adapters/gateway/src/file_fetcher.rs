//! file_fetcher: 跨节点透明文件获取与内容寻址安全缓存
//!
//! 本模块为独立实现，属通用工程模式（URL 解析 → 本地/远端解析 → 内容寻址缓存
//! → 完整性校验 → 路径沙箱），不含任何来源措辞：
//! 1. 拦截 `file://` 协议 URL，以 SHA-256(URL) 作为平台无关缓存键；
//! 2. 三级命中：内存缓存 → **磁盘缓存**（`cache_dir`，两级内容寻址：
//!    `<url_key>.meta.json` 记 URL→内容哈希，`<sha256>.bin` 存不可变字节）
//!    → 本地文件直读；
//! 3. 本地缺失时经分布式内部请求通道获取 Base64 负载，
//!    做 SHA-256 完整性自检后原子入库；
//! 4. 路径沙箱防御：拒绝 `..` / NUL 穿越，可选白名单根目录约束。
//! 编码规范见 RFC 4648（Base64）与 FIPS 180-4（SHA-256）。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// cfg(kani) 证明面容器: 见 research/verification/kani/src/lib.rs kani_collections 注。
#[cfg(kani)]
use super::kani_collections::HashMap;
#[cfg(not(kani))]
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

/// `file://` 协议前缀。
const FILE_SCHEME: &str = "file://";
/// MIME 后缀表（按优先级排列，首个命中生效）。
const MIME_BY_SUFFIX: &[(&str, &str)] = &[
    (".png", "image/png"),
    (".jpg", "image/jpeg"),
    (".jpeg", "image/jpeg"),
    (".json", "application/json"),
    (".txt", "text/plain"),
];
/// 未知类型的兜底 MIME。
const DEFAULT_MIME: &str = "application/octet-stream";

/// 透明文件穿透器（本地优先 + 分布式回调兜底 + 内存缓存）
pub struct TransparentFileFetcher {
    cache_dir: PathBuf,
    memory_cache: HashMap<String, FetchedFile>,
    allowed_roots: Vec<PathBuf>,
    /// 磁盘缓存写失败累计（缓存是优化，失败不影响正确性，但要能看见）。
    cache_write_failures: u64,
}

impl TransparentFileFetcher {
    pub fn new(cache_dir: impl AsRef<Path>, allowed_roots: Vec<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            memory_cache: HashMap::new(),
            allowed_roots,
            cache_write_failures: 0,
        }
    }

    /// 计算 URL 的 SHA-256 唯一缓存键
    pub fn compute_cache_key(file_url: &str) -> String {
        sha256_hex(file_url.as_bytes())
    }

    /// 词面路径校验；实际获取还会核对当前文件系统的根目录解析。
    pub fn validate_path_safety(&self, path_str: &str) -> Result<(), FileFetchError> {
        if path_str.contains("..") || path_str.contains('\0') {
            return Err(FileFetchError::PathTraversal(path_str.into()));
        }

        // 配置了白名单根目录时，路径必须坐落在其一之下
        if !self.allowed_roots.is_empty() {
            let normalized = PathBuf::from(path_str);
            let within_roots = self
                .allowed_roots
                .iter()
                .any(|root| normalized.starts_with(root));
            if !within_roots {
                return Err(FileFetchError::PathTraversal(format!(
                    "Path '{path_str}' is outside allowed root directories"
                )));
            }
        }

        Ok(())
    }

    /// 获取文件：内存缓存 → 本地磁盘 → 分布式回调，逐级回退。
    pub fn fetch_file<F>(
        &mut self,
        file_url: &str,
        remote_provider: F,
    ) -> Result<FetchedFile, FileFetchError>
    where
        F: FnOnce(&str) -> Result<InternalFileResponse, FileFetchError>,
    {
        let path_part = file_url
            .strip_prefix(FILE_SCHEME)
            .ok_or_else(|| FileFetchError::InvalidUrl(file_url.into()))?;
        self.validate_path_safety(path_part)?;
        self.validate_local_resolution(Path::new(path_part))?;

        let cache_key = Self::compute_cache_key(file_url);

        if let Some(file) = self.memory_cache.get(&cache_key) {
            return Ok(file.clone());
        }

        // 磁盘缓存命中（跨进程重启仍有效）。**每次命中都重算内容哈希并与
        // 元数据比对** —— 缓存是磁盘上的字节，可能被截断/篡改/写一半。
        // 校验不过 = 当作未命中并顺手删掉坏条目（不静默返回坏数据）。
        if let Some(file) = self.read_disk_cache(file_url, &cache_key)? {
            self.memory_cache.insert(cache_key, file.clone());
            return Ok(file);
        }

        let fetched = match self.read_local(path_part, file_url, &cache_key)? {
            Some(file) => file,
            None => Self::fetch_remote(file_url, &cache_key, remote_provider)?,
        };

        // 回填磁盘缓存。**写盘失败不影响本次获取**（缓存是优化不是正确性），
        // 但要能被看见：`cache_write_failures` 计数（自述面可读）。
        if self.write_disk_cache(&fetched).is_err() {
            self.cache_write_failures = self.cache_write_failures.saturating_add(1);
        }

        self.memory_cache.insert(cache_key, fetched.clone());
        Ok(fetched)
    }

    /// 磁盘缓存写失败累计次数（缓存坏了要能被告状，不静默）。
    pub fn cache_write_failures(&self) -> u64 {
        self.cache_write_failures
    }

    fn validate_local_resolution(&self, path: &Path) -> Result<(), FileFetchError> {
        if self.allowed_roots.is_empty() {
            return Ok(());
        }
        let resolved = resolve_existing_prefix(path).map_err(|_| {
            FileFetchError::PathTraversal(
                "file path cannot be resolved within allowed roots".into(),
            )
        })?;
        if self
            .allowed_roots
            .iter()
            .any(|root| std::fs::canonicalize(root).is_ok_and(|root| resolved.starts_with(root)))
        {
            Ok(())
        } else {
            Err(FileFetchError::PathTraversal(
                "resolved file path is outside allowed roots".into(),
            ))
        }
    }

    /// 元数据文件名：`<url_cache_key>.meta.json`。
    fn meta_path(&self, cache_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{cache_key}.meta.json"))
    }

    /// 内容文件名：`<sha256>.bin`（**按内容寻址**，同一份字节只存一份）。
    fn blob_path(&self, sha256_hash: &str) -> PathBuf {
        self.cache_dir.join(format!("{sha256_hash}.bin"))
    }

    /// 读磁盘缓存：元数据 → 内容 → **重算哈希校验** → 命中。
    ///
    /// 任何一步失败都当作未命中（`Ok(None)`），并尽力清理坏条目：
    /// 缓存层的错误不该冒泡成用户可见失败，但也绝不返回未校验的数据。
    fn read_disk_cache(
        &self,
        file_url: &str,
        cache_key: &str,
    ) -> Result<Option<FetchedFile>, FileFetchError> {
        let meta_path = self.meta_path(cache_key);
        if !is_regular_file(&meta_path) {
            return Ok(None);
        }
        let Ok(meta_text) = std::fs::read_to_string(&meta_path) else {
            return Ok(None);
        };
        let Ok(meta) = serde_json::from_str::<CacheMeta>(&meta_text) else {
            let _ = std::fs::remove_file(&meta_path);
            return Ok(None);
        };
        if meta.original_url != file_url
            || meta.sha256_hash.len() != 64
            || !meta.sha256_hash.bytes().all(|c| c.is_ascii_hexdigit())
        {
            let _ = std::fs::remove_file(&meta_path);
            return Ok(None);
        }
        if !is_regular_file(&self.blob_path(&meta.sha256_hash)) {
            let _ = std::fs::remove_file(&meta_path);
            return Ok(None);
        }
        let Ok(data) = std::fs::read(self.blob_path(&meta.sha256_hash)) else {
            // 元数据在但内容丢了 = 坏条目，清掉元数据重取
            let _ = std::fs::remove_file(&meta_path);
            return Ok(None);
        };
        // **完整性校验**：内容哈希必须与元数据一致
        if sha256_hex(&data) != meta.sha256_hash {
            let _ = std::fs::remove_file(&meta_path);
            return Ok(None);
        }
        Ok(Some(FetchedFile {
            cache_key: cache_key.to_string(),
            original_url: file_url.to_string(),
            mime_type: meta.mime_type,
            data,
            sha256_hash: meta.sha256_hash,
        }))
    }

    /// 写磁盘缓存（先写内容、再原子落元数据）。
    ///
    /// 顺序刻意如此：**先内容后元数据** —— 元数据是「这份内容可用」的承诺，
    /// 内容没落盘就写元数据会造出「元数据在、内容缺」的坏条目。
    /// 两步都走 `tmp + rename` 原子替换。
    fn write_disk_cache(&self, file: &FetchedFile) -> Result<(), FileFetchError> {
        std::fs::create_dir_all(&self.cache_dir)?;
        let blob_path = self.blob_path(&file.sha256_hash);
        let valid_blob = is_regular_file(&blob_path)
            && std::fs::read(&blob_path).is_ok_and(|data| sha256_hex(&data) == file.sha256_hash);
        if !valid_blob {
            atomic_write(&blob_path, &file.data)?;
        }
        let meta = CacheMeta {
            original_url: file.original_url.clone(),
            mime_type: file.mime_type.clone(),
            sha256_hash: file.sha256_hash.clone(),
        };
        let meta_bytes = serde_json::to_vec(&meta)
            .map_err(|error| FileFetchError::Decode(format!("meta 序列化失败: {error}")))?;
        atomic_write(&self.meta_path(&file.cache_key), &meta_bytes)?;
        Ok(())
    }

    /// 本地直读：存在即读、计哈希、推断 MIME；不存在返回 `None`，读失败报错。
    fn read_local(
        &self,
        path_part: &str,
        file_url: &str,
        cache_key: &str,
    ) -> Result<Option<FetchedFile>, FileFetchError> {
        let local_path = Path::new(path_part);
        if !local_path.is_file() {
            return Ok(None);
        }
        let data = std::fs::read(local_path)?;
        Ok(Some(FetchedFile {
            cache_key: cache_key.to_string(),
            original_url: file_url.into(),
            mime_type: Self::guess_mime_type(path_part),
            sha256_hash: sha256_hex(&data),
            data,
        }))
    }

    /// 远端回退：发起内部请求 → Base64 解码 → 哈希完整性校验。
    fn fetch_remote<F>(
        file_url: &str,
        cache_key: &str,
        remote_provider: F,
    ) -> Result<FetchedFile, FileFetchError>
    where
        F: FnOnce(&str) -> Result<InternalFileResponse, FileFetchError>,
    {
        let response = remote_provider(file_url)?;
        if response.status != "success" {
            return Err(FileFetchError::NodeNotFound(file_url.into()));
        }

        let data = base64_decode(&response.base64_data)?;
        let actual_hash = sha256_hex(&data);
        if actual_hash != response.sha256_hash {
            return Err(FileFetchError::IntegrityMismatch {
                expected: response.sha256_hash,
                actual: actual_hash,
            });
        }

        Ok(FetchedFile {
            cache_key: cache_key.to_string(),
            original_url: file_url.into(),
            mime_type: response.mime_type,
            data,
            sha256_hash: actual_hash,
        })
    }

    fn guess_mime_type(path: &str) -> String {
        MIME_BY_SUFFIX
            .iter()
            .find(|(suffix, _)| path.ends_with(suffix))
            .map(|(_, mime)| (*mime).to_string())
            .unwrap_or_else(|| DEFAULT_MIME.to_string())
    }
}

/// 磁盘缓存元数据（`<url_key>.meta.json`）。
///
/// 只记「这份 URL 对应哪份内容」，内容本身按 `<sha256>.bin` 存 ——
/// 同一份字节被多个 URL 命中时只落一份盘。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CacheMeta {
    original_url: String,
    mime_type: String,
    sha256_hash: String,
}

/// 原子写文件（同目录 `tmp` + rename）。
///
/// 缓存文件必须「要么是旧版本、要么是完整新版本」——
/// 直接覆写会留下写一半的条目，下次命中即坏数据。
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), FileFetchError> {
    // The shared primitive uses exclusive temporary creation and replacement.
    // Cache data is owner-only on Unix; normal restart persistence is sufficient.
    apeireth_core::storage_atomic::write_atomic(path, bytes, 0o600).map_err(FileFetchError::Io)
}

fn is_regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file())
}

fn resolve_existing_prefix(path: &Path) -> std::io::Result<PathBuf> {
    let mut prefix = path.to_path_buf();
    let mut tail = Vec::new();
    loop {
        match std::fs::symlink_metadata(&prefix) {
            Ok(_) => {
                let mut resolved = std::fs::canonicalize(prefix)?;
                for part in tail.into_iter().rev() {
                    resolved.push(part);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = prefix.file_name().map(|n| n.to_os_string()) else {
                    return Err(error);
                };
                tail.push(name);
                prefix.pop();
                if prefix.as_os_str().is_empty() {
                    prefix.push(".");
                }
            }
            Err(error) => return Err(error),
        }
    }
}

/// SHA-256 十六进制摘要（小写）。
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// 简易 Safe Base64 解码器 (RFC 4648，无外部额外依赖)：
/// 忽略 `\r` / `\n` / 空格，遇 `=` 提前收尾，非法字符报错。
fn base64_decode(input: &str) -> Result<Vec<u8>, FileFetchError> {
    const B64_ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut decode_table = [255u8; 256];
    for (value, &symbol) in B64_ALPHABET.iter().enumerate() {
        decode_table[symbol as usize] = value as u8;
    }

    let mut output = Vec::with_capacity(input.len() * 3 / 4 + 3);
    let mut accumulator = 0u32;
    let mut pending_bits = 0u32;

    for byte in input.bytes() {
        match byte {
            b'\r' | b'\n' | b' ' => continue,
            b'=' => break,
            _ => {}
        }
        let sextet = decode_table[byte as usize];
        if sextet == 255 {
            // cfg(kani): format! 插值是 CBMC 引雷点 (Display/format 机器拖
            // memchr 7881 次, 实测 40m+ 跑不完; 换静态文案后 <420s 证完)。
            // 命题只断言失败模式 (Decode 枚举) 不含消息文本, 语义无损。
            #[cfg(kani)]
            let message = "invalid base64 char".to_string();
            #[cfg(not(kani))]
            let message = format!("invalid base64 char: {byte}");
            return Err(FileFetchError::Decode(message));
        }
        accumulator = (accumulator << 6) | u32::from(sextet);
        pending_bits += 6;
        if pending_bits >= 8 {
            pending_bits -= 8;
            output.push((accumulator >> pending_bits) as u8);
            accumulator &= (1 << pending_bits) - 1;
        }
    }

    Ok(output)
}

// ===== Kani harness (cfg(kani) 门控: 生产零参与) =====
// base64_decode 为模块私有函数, harness 必须与被测函数同模块可见,
// 故此证明段随源文件携带 (与 research_approval_sm.rs 的 kani_proofs 段同构);
// 其余 file_fetcher 性质族 harness 在 research/verification/kani/src/ 下。

#[cfg(kani)]
mod kani_base64_proofs {
    use super::*;

    /// 证明: base64_decode 对任意 ≤8 字符串不 panic —— 非法字符唯一失败模式
    /// 是 Decode 错误, 合法解码输出长度不超过输入长 (≤3/4 膨胀界)。
    /// 边界: 输入 8 字节 (字节 0..=127, 含 `=` 提前收尾、\r\n/空白、非法字符;
    /// 口径修订 2026-10-11: 字节数组直构 + from_utf8_unchecked (构造即合法,
    /// 验证机/panic 路径退出证明面, 见 research/verification/kani/src/
    /// harness_panic_freedom.rs 同名注); unwind 128 —— 由 48 修正: decode_table
    /// 初始化循环 64 次 > 48, 展开断言必败 (命题没变, 是上限没盖住字面量循环)。
    #[kani::proof]
    #[kani::unwind(128)]
    fn kani_panic_free_base64_decode() {
        // 代表值域 8 档 (NUL/\n/空格/'='/'+'/'A'/0x7F/'!'): 全分支类覆盖
        // (跳过/截断/合法/非法), 值域量词口径见 harness_panic_freedom.rs::rep_byte。
        let mut raw = [0u8; 8];
        for slot in raw.iter_mut() {
            *slot = match kani::any::<u8>() % 8 {
                0 => 0x00,
                1 => 0x0A,
                2 => 0x20,
                3 => 0x3D,
                4 => 0x2B,
                5 => 0x41,
                6 => 0x7F,
                _ => 0x21,
            };
        }
        // SAFETY: 全部字节 < 0x80 (ASCII), 必为合法 UTF-8。
        let input: &str = unsafe { std::str::from_utf8_unchecked(&raw) };
        match base64_decode(&input) {
            Ok(out) => assert!(out.len() <= input.len(), "解码输出长度 ≤ 输入长"),
            Err(err) => assert!(
                matches!(err, FileFetchError::Decode(_)),
                "唯一失败模式是 Decode 错误"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 磁盘缓存（IMPLEMENTED；生产消费点仍未接线）=====

    /// 进程内唯一的临时目录（本 crate 无 `tempfile` 依赖，不为此新增依赖）。
    ///
    /// 用「进程 id + 原子计数 + 纳秒」命名：并行测试不会撞同一目录。
    fn scratch_dir(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "apeireth-file-fetcher-{label}-{}-{seq}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        path
    }

    /// 写一个本地文件，返回它的 `file://` URL。
    fn local_file_url(dir: &Path, name: &str, content: &[u8]) -> String {
        let path = dir.join(name);
        std::fs::write(&path, content).expect("write fixture");
        format!("file://{}", path.display())
    }

    fn no_remote(_url: &str) -> Result<InternalFileResponse, FileFetchError> {
        panic!("本用例不该走远端回退")
    }

    /// **跨进程重启仍命中**：新实例（内存缓存空）应直接吃磁盘缓存，
    /// 既不再读原文件、也不走远端。
    #[test]
    fn disk_cache_survives_a_new_fetcher_instance() {
        let dir = scratch_dir("src");
        let cache = scratch_dir("cache");
        let url = local_file_url(dir.as_path(), "a.txt", b"hello cache");

        {
            let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
            let fetched = fetcher.fetch_file(&url, no_remote).expect("first fetch");
            assert_eq!(fetched.data, b"hello cache");
            assert_eq!(fetcher.cache_write_failures(), 0);
        }

        // 把原文件删掉：若还命中，只可能来自磁盘缓存
        std::fs::remove_file(dir.as_path().join("a.txt")).expect("remove source");

        let mut reopened = TransparentFileFetcher::new(cache.as_path(), Vec::new());
        let fetched = reopened.fetch_file(&url, no_remote).expect("cached fetch");
        assert_eq!(fetched.data, b"hello cache", "必须从磁盘缓存命中");
    }

    /// 缓存内容被篡改 ⇒ **不得返回坏数据**，应视为未命中并回源。
    #[test]
    fn corrupted_cache_blob_is_rejected_not_served() {
        let dir = scratch_dir("src");
        let cache = scratch_dir("cache");
        let url = local_file_url(dir.as_path(), "b.txt", b"original bytes");

        let hash = {
            let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
            let fetched = fetcher.fetch_file(&url, no_remote).expect("fetch");
            fetched.sha256_hash
        };

        // 篡改内容 blob（哈希不再匹配）
        let blob = cache.as_path().join(format!("{hash}.bin"));
        std::fs::write(&blob, b"TAMPERED").expect("tamper");

        // 原文件还在 ⇒ 回源拿到正确内容（而不是返回被篡改的字节）
        let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
        let fetched = fetcher.fetch_file(&url, no_remote).expect("refetch");
        assert_eq!(fetched.data, b"original bytes", "绝不能返回被篡改的缓存");
    }

    #[test]
    fn rejected_corruption_is_repaired_for_the_next_restart() {
        let dir = scratch_dir("repair-source");
        let cache = scratch_dir("repair-cache");
        let url = local_file_url(&dir, "item.txt", b"original");
        let first = TransparentFileFetcher::new(&cache, vec![])
            .fetch_file(&url, no_remote)
            .unwrap();
        std::fs::write(cache.join(format!("{}.bin", first.sha256_hash)), b"corrupt").unwrap();
        assert_eq!(
            TransparentFileFetcher::new(&cache, vec![])
                .fetch_file(&url, no_remote)
                .unwrap()
                .data,
            b"original"
        );
        std::fs::remove_file(dir.join("item.txt")).unwrap();
        assert_eq!(
            TransparentFileFetcher::new(&cache, vec![])
                .fetch_file(&url, no_remote)
                .unwrap()
                .data,
            b"original"
        );
    }

    #[test]
    fn metadata_for_a_different_url_cannot_substitute_cached_content() {
        let dir = scratch_dir("binding-source");
        let cache = scratch_dir("binding-cache");
        let a = local_file_url(&dir, "a.txt", b"first");
        let b = local_file_url(&dir, "b.txt", b"second");
        let mut first = TransparentFileFetcher::new(&cache, vec![]);
        first.fetch_file(&a, no_remote).unwrap();
        first.fetch_file(&b, no_remote).unwrap();
        let meta_a = first.meta_path(&TransparentFileFetcher::compute_cache_key(&a));
        let meta_b = first.meta_path(&TransparentFileFetcher::compute_cache_key(&b));
        std::fs::copy(meta_b, meta_a).unwrap();
        std::fs::remove_file(dir.join("a.txt")).unwrap();
        let error = TransparentFileFetcher::new(&cache, vec![])
            .fetch_file(&a, |_| Err(FileFetchError::NodeNotFound("offline".into())))
            .unwrap_err();
        assert!(matches!(error, FileFetchError::NodeNotFound(_)));
        assert_eq!(
            TransparentFileFetcher::new(&cache, vec![])
                .fetch_file(&b, no_remote)
                .unwrap()
                .data,
            b"second"
        );
    }

    #[test]
    fn cache_write_failure_preserves_the_fetch_and_is_counted() {
        let dir = scratch_dir("write-failure");
        let url = local_file_url(&dir, "source.txt", b"available");
        let blocked = dir.join("not-a-directory");
        std::fs::write(&blocked, b"occupied").unwrap();
        let mut fetcher = TransparentFileFetcher::new(&blocked, vec![]);
        assert_eq!(
            fetcher.fetch_file(&url, no_remote).unwrap().data,
            b"available"
        );
        assert_eq!(fetcher.cache_write_failures(), 1);
        assert_eq!(std::fs::read(blocked).unwrap(), b"occupied");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_cache_records_are_not_trusted() {
        for metadata in [true, false] {
            let dir = scratch_dir("symlink-source");
            let cache = scratch_dir("symlink-cache");
            let url = local_file_url(&dir, "source.txt", b"payload");
            let mut first = TransparentFileFetcher::new(&cache, vec![]);
            let file = first.fetch_file(&url, no_remote).unwrap();
            let entry = if metadata {
                first.meta_path(&file.cache_key)
            } else {
                first.blob_path(&file.sha256_hash)
            };
            let outside = dir.join("moved-cache-record");
            std::fs::rename(&entry, &outside).unwrap();
            std::os::unix::fs::symlink(&outside, &entry).unwrap();
            std::fs::remove_file(dir.join("source.txt")).unwrap();
            assert!(TransparentFileFetcher::new(&cache, vec![])
                .fetch_file(&url, |_| Err(FileFetchError::NodeNotFound(
                    "offline".into()
                )))
                .is_err());
            assert!(outside.is_file());
        }
    }

    #[cfg(unix)]
    #[test]
    fn root_symlink_escape_is_rejected_even_after_a_memory_hit() {
        let root = scratch_dir("allowed-root");
        let outside = scratch_dir("outside-root");
        let cache = scratch_dir("root-cache");
        let inside = root.join("inside");
        std::fs::create_dir(&inside).unwrap();
        std::fs::write(inside.join("item.txt"), b"inside").unwrap();
        std::fs::write(outside.join("item.txt"), b"outside").unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&inside, &link).unwrap();
        let url = format!("file://{}", link.join("item.txt").display());
        let mut fetcher = TransparentFileFetcher::new(&cache, vec![root]);
        assert_eq!(fetcher.fetch_file(&url, no_remote).unwrap().data, b"inside");
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(matches!(
            fetcher.fetch_file(&url, no_remote),
            Err(FileFetchError::PathTraversal(_))
        ));
    }

    /// 元数据在但内容 blob 缺失（坏条目）⇒ 视为未命中，不报错。
    #[test]
    fn missing_blob_with_present_meta_falls_back_to_source() {
        let dir = scratch_dir("src");
        let cache = scratch_dir("cache");
        let url = local_file_url(dir.as_path(), "c.txt", b"payload");

        let hash = {
            let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
            fetcher
                .fetch_file(&url, no_remote)
                .expect("fetch")
                .sha256_hash
        };
        std::fs::remove_file(cache.as_path().join(format!("{hash}.bin"))).expect("drop blob");

        let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
        let fetched = fetcher.fetch_file(&url, no_remote).expect("refetch");
        assert_eq!(fetched.data, b"payload");
    }

    /// 同一份内容被两个 URL 引用时，内容 blob **只落一份**（按内容寻址去重）。
    #[test]
    fn identical_content_is_stored_once() {
        let dir = scratch_dir("src");
        let cache = scratch_dir("cache");
        let one = local_file_url(dir.as_path(), "d1.txt", b"same bytes");
        let two = local_file_url(dir.as_path(), "d2.txt", b"same bytes");

        let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
        fetcher.fetch_file(&one, no_remote).expect("fetch one");
        fetcher.fetch_file(&two, no_remote).expect("fetch two");

        let blobs = std::fs::read_dir(cache.as_path())
            .expect("read cache")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".bin"))
            .count();
        assert_eq!(blobs, 1, "同内容只应有一个 blob（内容寻址去重）");
    }

    /// 远端回退的内容同样**入库**，下次重启可离线命中。
    #[test]
    fn remote_fetch_is_cached_for_offline_reuse() {
        let cache = scratch_dir("cache");
        let url = "file:///definitely/not/present/remote.png";
        let payload = b"PNGDATA";
        let expected_hash = sha256_hex(payload);
        let response = InternalFileResponse {
            request_id: "r1".into(),
            status: "success".into(),
            base64_data: {
                // 简易 base64 编码（只为本用例构造负载）
                const ALPHABET: &[u8; 64] =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                let mut out = String::new();
                for chunk in payload.chunks(3) {
                    let b = [
                        chunk[0],
                        *chunk.get(1).unwrap_or(&0),
                        *chunk.get(2).unwrap_or(&0),
                    ];
                    let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
                    // 按 RFC 4648 处理尾部：1 字节 → 2 字符 + "=="，2 字节 → 3 字符 + "="
                    let emit = match chunk.len() {
                        1 => 2,
                        2 => 3,
                        _ => 4,
                    };
                    for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
                        if index < emit {
                            out.push(ALPHABET[((n >> shift) & 0x3F) as usize] as char);
                        }
                    }
                    for _ in emit..4 {
                        out.push('=');
                    }
                }
                out
            },
            mime_type: "image/png".into(),
            sha256_hash: expected_hash.clone(),
        };
        {
            let mut fetcher = TransparentFileFetcher::new(cache.as_path(), Vec::new());
            let fetched = fetcher
                .fetch_file(url, move |_| Ok(response))
                .expect("remote fetch");
            assert_eq!(fetched.data, payload);
        }
        // 重启后离线命中（远端 provider 会 panic，证明没走远端）
        let mut reopened = TransparentFileFetcher::new(cache.as_path(), Vec::new());
        let fetched = reopened
            .fetch_file(url, no_remote)
            .expect("offline cache hit");
        assert_eq!(fetched.data, payload);
        assert_eq!(fetched.mime_type, "image/png");
    }

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

        let dummy_data = b"Hello Apeireth Transparent File Fetcher!";
        let expected_hash = sha256_hex(dummy_data);

        // Base64 of dummy_data (RFC 4648)
        let base64_str = "SGVsbG8gQXBlaXJldGggVHJhbnNwYXJlbnQgRmlsZSBGZXRjaGVyIQ==";

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
