//! `apeireth-tools-canonical::spill` — 工具结果溢出存储.
//!
//! 工具输出可能超大 (grep/读大文件/代码检索), 直接塞入 LLM 上下文会导致上下文膨胀.
//! 超过阈值的结果会溢出到会话私有文件；模型只得到一个**会话内相对引用**，读取时必须
//! 同时提供同一个会话标识。`SpillStore` 不拥有会话生命周期，它只把调用方给出的会话
//! 标识作为存储授权边界。
//!
//! ## 安全和生命周期边界
//!
//! - 会话目录由会话 UTF-8 字节的无碰撞十六进制编码组成，读取 API 接受
//!   `(session_id, relative_reference)`，绝不接受一个可跨会话复用的绝对路径。
//! - spill 文件通过 `create_new` 独占创建；Unix 上文件以 `0o600`、新建目录以
//!   `0o700` 的模式创建，不存在“先创建再 chmod”的权限窗口。创建失败会返回错误。
//! - Windows 没有在这里伪造 `0600` 等价 ACL：文件和目录依赖 `root` 的继承 ACL。
//!   部署者必须将 `root` 放在适当受限的位置；本模块不声称抵御能够修改该目录 ACL 的
//!   本地账户。
//! - 每个会话由 [`SpillPolicy`] 限制单文件大小、会话总大小、文件数和 TTL。清理是显式
//!   的，且在新写入前懒执行；没有后台守护线程。
//! - 会话本地文件锁把配额检查和创建串行化，因而多个线程或进程不会都通过同一次配额
//!   检查。

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// 溢出阈值: 序列化结果超过该字符数 → spill (默认 2000 字符).
pub const SPILL_THRESHOLD_CHARS: usize = 2000;

/// `SpillStore` 接受的最大会话标识字节数。
///
/// 会话目录使用可逆的十六进制编码，以避免净化字符串造成的目录碰撞；这个上限同时避免
/// 用户控制的会话标识生成超长路径组件。
pub const MAX_SESSION_ID_BYTES: usize = 96;

const SPILL_FILE_PREFIX: &str = "spill-";
const SESSION_PREFIX: &str = "session-";
const SESSION_LOCK_FILE: &str = ".apeireth-spill.lock";
const MAX_CREATE_ATTEMPTS: usize = 32;

/// 溢出文件的有界生命周期策略。
///
/// `ttl` 按文件最后修改时间计算。超期文件仅在调用 [`SpillStore::cleanup_expired`] 或下次
/// 同会话 [`SpillStore::spill`] 时被删除。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpillPolicy {
    /// 单个 spill 文件允许的最大 UTF-8 字节数。
    pub max_file_bytes: usize,
    /// 单个会话所有受管理 spill 文件允许的最大总字节数。
    pub max_session_bytes: usize,
    /// 单个会话允许的受管理 spill 文件数，避免零字节 spill 绕过字节配额。
    pub max_session_files: usize,
    /// spill 文件保留时间。
    pub ttl: Duration,
}

impl Default for SpillPolicy {
    fn default() -> Self {
        Self {
            max_file_bytes: 8 * 1024 * 1024,
            max_session_bytes: 64 * 1024 * 1024,
            max_session_files: 1024,
            ttl: Duration::from_secs(24 * 60 * 60),
        }
    }
}

/// 会话私有、有界的溢出存储。
#[derive(Debug, Clone)]
pub struct SpillStore {
    root: PathBuf,
    policy: SpillPolicy,
    sequence: Arc<AtomicU64>,
}

/// 文件名净化: 只保留字母数字与安全符号, 移除路径分隔符与 `..`.
pub fn safe_segment(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('_');
    if cleaned.is_empty() {
        "spill".to_string()
    } else {
        cleaned.chars().take(60).collect()
    }
}

impl Default for SpillStore {
    fn default() -> Self {
        Self::new_private()
    }
}

impl SpillStore {
    /// 构造系统临时目录下的私有随机子目录 (进程生命周期隔离).
    pub fn new_private() -> Self {
        let root = std::env::temp_dir().join(format!(
            "apeireth-spill-{}-{}",
            std::process::id(),
            timestamp_nanos()
        ));
        Self::with_policy(root, SpillPolicy::default())
    }

    /// 显式指定 root 根目录，并使用默认的有界策略。
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self::with_policy(root, SpillPolicy::default())
    }

    /// 显式指定 root 和有界生命周期策略。
    ///
    /// 在 Windows 上，`root` 必须位于 ACL 已限制为预期服务身份的目录中。本模块只使用
    /// 继承 ACL，未实现或声称实现逐文件 `0600` 等价的 ACL 隔离。
    pub fn with_policy(root: impl Into<PathBuf>, policy: SpillPolicy) -> Self {
        Self {
            root: root.into(),
            policy,
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 获取根目录路径引用.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 获取当前生命周期策略。
    pub fn policy(&self) -> &SpillPolicy {
        &self.policy
    }

    /// 溢出写入，返回**会话内相对** spill 引用，而不是绝对路径。
    ///
    /// 该引用只能通过 [`Self::read_for_session`] 并携带相同的 `session_id` 读取。对于一
    /// 个会话，配额检查、过期清理和 `create_new` 写入在同一个文件锁内完成。
    pub fn spill(
        &self,
        session_id: &str,
        suggested_name: &str,
        content: &str,
    ) -> Result<String, String> {
        let content_bytes = content.len();
        if content_bytes > self.policy.max_file_bytes {
            return Err(format!(
                "溢出内容超过单文件上限: {content_bytes} bytes > {} bytes",
                self.policy.max_file_bytes
            ));
        }

        let session_dir = self.session_dir(session_id)?;
        let _lock = self.acquire_session_lock(&session_dir)?;
        self.cleanup_expired_in_session(&session_dir, SystemTime::now())?;

        let usage = self.session_usage(&session_dir)?;
        if usage.files >= self.policy.max_session_files {
            return Err(format!(
                "溢出文件数超过会话上限: current={} files, max={} files",
                usage.files, self.policy.max_session_files
            ));
        }
        let available_bytes = self.policy.max_session_bytes.saturating_sub(usage.bytes);
        if content_bytes > available_bytes {
            return Err(format!(
                "溢出内容超过会话上限: current={} bytes, requested={content_bytes} bytes, max={} bytes",
                usage.bytes,
                self.policy.max_session_bytes
            ));
        }

        let safe_name = safe_segment(suggested_name);
        for _ in 0..MAX_CREATE_ATTEMPTS {
            let reference = self.next_reference(&safe_name);
            let file = session_dir.join(&reference);
            match open_private_new_file(&file) {
                Ok(mut handle) => {
                    if let Err(write_error) = handle.write_all(content.as_bytes()) {
                        drop(handle);
                        return match fs::remove_file(&file) {
                            Ok(()) => Err(format!("写溢出内容失败: {write_error}")),
                            Err(cleanup_error) => Err(format!(
                                "写溢出内容失败: {write_error}; 删除部分写入文件也失败: {cleanup_error}"
                            )),
                        };
                    }
                    return Ok(reference);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(format!("独占写溢出文件失败: {error}")),
            }
        }

        Err("无法在限定次数内创建唯一溢出文件".to_string())
    }

    /// 在给定会话中读取一个会话内相对 spill 引用。
    ///
    /// 绝对路径、多组件路径、`..` 和符号链接都会被拒绝。即使调用方知道另一个会话的文件
    /// 名称，也只能在自己的会话目录中查找，不能将该引用解释为根目录或另一会话的路径。
    pub fn read_for_session(&self, session_id: &str, reference: &str) -> Result<String, String> {
        validate_spill_reference(reference)?;
        let session_dir = self.session_dir(session_id)?;
        let _lock = self.acquire_session_lock(&session_dir)?;
        let target = session_dir.join(reference);

        let metadata = fs::symlink_metadata(&target)
            .map_err(|error| format!("会话内 spill 引用不可读取: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("拒绝读取符号链接 spill 引用".to_string());
        }
        if !metadata.is_file() {
            return Err("拒绝读取非文件 spill 引用".to_string());
        }

        let session_canonical = fs::canonicalize(&session_dir)
            .map_err(|error| format!("canonicalize 会话目录失败: {error}"))?;
        let target_canonical = fs::canonicalize(&target)
            .map_err(|error| format!("canonicalize 会话 spill 文件失败: {error}"))?;
        if target_canonical.parent() != Some(session_canonical.as_path()) {
            return Err("越权读取: spill 引用不属于请求的会话目录".to_string());
        }

        fs::read_to_string(&target_canonical).map_err(|error| format!("读取溢出文件失败: {error}"))
    }

    /// 删除一个会话中已经过 TTL 的受管理 spill 文件，返回删除数量。
    ///
    /// 清理只枚举该会话目录的直接子项，只删除名字符合本模块生成格式的普通文件；未知文件、
    /// 目录和符号链接会被保留。调用者可在其自身生命周期钩子中显式调用本方法，无后台服务。
    pub fn cleanup_expired(&self, session_id: &str) -> Result<usize, String> {
        let session_dir = self.session_dir(session_id)?;
        let _lock = self.acquire_session_lock(&session_dir)?;
        self.cleanup_expired_in_session(&session_dir, SystemTime::now())
    }

    /// 若内容超出阈值则溢出并返回提示文本, 否则原样返回.
    ///
    /// 当有界策略拒绝 spill 时，不会将原始的大内容回退到上下文中，从而绕过大小限制。
    pub fn maybe_spill(
        &self,
        session_id: &str,
        tool_name: &str,
        content: &str,
        threshold: usize,
    ) -> String {
        if content.chars().count() > threshold {
            match self.spill(session_id, tool_name, content) {
                Ok(reference) => {
                    let preview: String = content.chars().take(200).collect();
                    format!(
                        "[工具输出过大 (共 {} 字符), 已安全溢出；会话内引用: {}]\n预览内容:\n{}\n...",
                        content.chars().count(),
                        reference,
                        preview
                    )
                }
                Err(error) => {
                    format!("[工具输出过大，溢出存储拒绝该内容: {error}; 原始内容未注入上下文]")
                }
            }
        } else {
            content.to_string()
        }
    }

    fn session_dir(&self, session_id: &str) -> Result<PathBuf, String> {
        let root_canonical = self.ensure_root()?;
        let segment = session_segment(session_id)?;
        let candidate = self.root.join(&segment);

        match create_private_directory(&candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("创建会话 spill 目录失败: {error}")),
        }

        let metadata = fs::symlink_metadata(&candidate)
            .map_err(|error| format!("检查会话 spill 目录失败: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("会话 spill 目录不是受信任的普通目录".to_string());
        }

        let canonical = fs::canonicalize(&candidate)
            .map_err(|error| format!("canonicalize 会话 spill 目录失败: {error}"))?;
        let expected = root_canonical.join(segment);
        if canonical != expected {
            return Err("会话 spill 目录未严格位于配置根目录下".to_string());
        }
        Ok(canonical)
    }

    fn ensure_root(&self) -> Result<PathBuf, String> {
        create_private_directory_all(&self.root)
            .map_err(|error| format!("创建 spill 根目录失败: {error}"))?;
        let metadata = fs::symlink_metadata(&self.root)
            .map_err(|error| format!("检查 spill 根目录失败: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("spill 根目录不是受信任的普通目录".to_string());
        }
        fs::canonicalize(&self.root)
            .map_err(|error| format!("canonicalize spill 根目录失败: {error}"))
    }

    fn acquire_session_lock(&self, session_dir: &Path) -> Result<File, String> {
        let lock_path = session_dir.join(SESSION_LOCK_FILE);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err("会话 spill 锁不是普通文件".to_string());
            }
        }

        let handle = open_private_lock_file(&lock_path)
            .map_err(|error| format!("打开会话 spill 锁失败: {error}"))?;
        handle
            .lock()
            .map_err(|error| format!("获取会话 spill 锁失败: {error}"))?;
        Ok(handle)
    }

    fn cleanup_expired_in_session(
        &self,
        session_dir: &Path,
        now: SystemTime,
    ) -> Result<usize, String> {
        let cutoff = now.checked_sub(self.policy.ttl).unwrap_or(UNIX_EPOCH);
        let entries = fs::read_dir(session_dir)
            .map_err(|error| format!("枚举会话 spill 目录失败: {error}"))?;
        let mut removed = 0usize;

        for entry in entries {
            // A concurrently removed or malformed directory entry is not a reason to panic or
            // to traverse elsewhere. It will be reconsidered on a later cleanup pass.
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !metadata.is_file() || !is_spill_file_name(&entry.file_name()) {
                continue;
            }
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            if modified > cutoff {
                continue;
            }

            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("删除过期 spill 文件失败: {error}")),
            }
        }

        Ok(removed)
    }

    fn session_usage(&self, session_dir: &Path) -> Result<SessionUsage, String> {
        let entries = fs::read_dir(session_dir)
            .map_err(|error| format!("枚举会话 spill 目录失败: {error}"))?;
        let mut usage = SessionUsage::default();

        for entry in entries {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !metadata.is_file() || !is_spill_file_name(&entry.file_name()) {
                continue;
            }
            let file_bytes = usize::try_from(metadata.len())
                .map_err(|_| "单个 spill 文件大小无法表示为 usize".to_string())?;
            usage.bytes = usage
                .bytes
                .checked_add(file_bytes)
                .ok_or_else(|| "会话 spill 文件大小总和溢出".to_string())?;
            usage.files = usage
                .files
                .checked_add(1)
                .ok_or_else(|| "会话 spill 文件数量溢出".to_string())?;
        }

        Ok(usage)
    }

    fn next_reference(&self, safe_name: &str) -> String {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        format!(
            "{SPILL_FILE_PREFIX}{}-{}-{sequence}-{safe_name}",
            timestamp_nanos(),
            std::process::id(),
        )
    }
}

#[derive(Debug, Default)]
struct SessionUsage {
    bytes: usize,
    files: usize,
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn session_segment(session_id: &str) -> Result<String, String> {
    if session_id.len() > MAX_SESSION_ID_BYTES {
        return Err(format!(
            "会话标识超过最大长度: {} bytes > {MAX_SESSION_ID_BYTES} bytes",
            session_id.len()
        ));
    }

    let mut encoded = String::with_capacity(SESSION_PREFIX.len() + session_id.len() * 2);
    encoded.push_str(SESSION_PREFIX);
    for byte in session_id.bytes() {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(encoded)
}

fn validate_spill_reference(reference: &str) -> Result<(), String> {
    let path = Path::new(reference);
    let mut components = path.components();
    let component = components.next();
    if components.next().is_some() {
        return Err("spill 引用必须是会话内单文件相对路径".to_string());
    }

    let Component::Normal(name) = component.ok_or_else(|| "spill 引用不能为空".to_string())?
    else {
        return Err("spill 引用必须是会话内单文件相对路径".to_string());
    };
    if !is_spill_file_name(name) {
        return Err("spill 引用不是受管理的 spill 文件名".to_string());
    }
    Ok(())
}

fn is_spill_file_name(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    name.starts_with(SPILL_FILE_PREFIX)
        && name.len() > SPILL_FILE_PREFIX.len()
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '-' || character == '_')
}

fn create_private_directory_all(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
    }
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir(path)
    }
}

fn open_private_new_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.mode(0o600);
    }
    options.open(path)
}

fn open_private_lock_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashSet,
        sync::{Arc, Barrier},
        thread,
    };

    fn restrictive_policy(
        max_file_bytes: usize,
        max_session_bytes: usize,
        ttl: Duration,
    ) -> SpillPolicy {
        SpillPolicy {
            max_file_bytes,
            max_session_bytes,
            max_session_files: 64,
            ttl,
        }
    }

    fn spill_path(store: &SpillStore, session_id: &str, reference: &str) -> PathBuf {
        store.session_dir(session_id).unwrap().join(reference)
    }

    #[test]
    fn safe_segment_cleans_and_sanitizes() {
        assert_eq!(safe_segment("valid_name-123"), "valid_name-123");
        assert_eq!(safe_segment("../../../etc/passwd"), "etc_passwd");
        assert_eq!(safe_segment(""), "spill");
    }

    #[test]
    fn session_read_requires_matching_session_and_relative_reference() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let content = "这是一段非常长的工具输出内容，需要溢出落盘保存。";
        let reference = store.spill("session_a", "grep_tool", content).unwrap();

        assert_eq!(
            store.read_for_session("session_a", &reference).unwrap(),
            content
        );
        assert!(store.read_for_session("session_b", &reference).is_err());

        let absolute_a_path = spill_path(&store, "session_a", &reference);
        assert!(store
            .read_for_session("session_b", &absolute_a_path.to_string_lossy())
            .is_err());
    }

    #[test]
    fn traversal_and_root_wide_references_are_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let reference = store.spill("session_a", "grep_tool", "contents").unwrap();
        let path = spill_path(&store, "session_a", &reference);

        assert!(store.read_for_session("session_a", "../outside").is_err());
        assert!(store
            .read_for_session("session_a", &path.to_string_lossy())
            .is_err());
        assert!(store
            .read_for_session("session_a", "session-other/spill-known")
            .is_err());
    }

    #[test]
    fn oversized_spill_is_rejected_before_creating_a_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_policy(
            temp_dir.path(),
            restrictive_policy(4, 16, Duration::from_secs(60)),
        );

        assert!(store.spill("session", "tool", "12345").is_err());
        let session_dir = store.session_dir("session").unwrap();
        let spills = fs::read_dir(session_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| is_spill_file_name(&entry.file_name()))
            .count();
        assert_eq!(spills, 0);
    }

    #[test]
    fn session_quota_rejects_new_content_without_deleting_active_spill() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_policy(
            temp_dir.path(),
            restrictive_policy(10, 5, Duration::from_secs(60)),
        );
        let first = store.spill("session", "first", "abc").unwrap();

        assert!(store.spill("session", "second", "def").is_err());
        assert_eq!(store.read_for_session("session", &first).unwrap(), "abc");
    }

    #[test]
    fn session_file_limit_blocks_zero_byte_spill_growth() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut policy = restrictive_policy(10, 10, Duration::from_secs(60));
        policy.max_session_files = 1;
        let store = SpillStore::with_policy(temp_dir.path(), policy);

        let first = store.spill("session", "first", "").unwrap();
        assert!(store.spill("session", "second", "").is_err());
        assert_eq!(store.read_for_session("session", &first).unwrap(), "");
    }

    #[test]
    fn expired_cleanup_removes_old_files_but_keeps_active_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_policy(
            temp_dir.path(),
            restrictive_policy(100, 100, Duration::from_secs(60)),
        );
        let expired = store.spill("session", "old", "old").unwrap();
        let active = store.spill("session", "active", "active").unwrap();
        let expired_path = spill_path(&store, "session", &expired);

        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(61))
            .unwrap();
        OpenOptions::new()
            .write(true)
            .open(&expired_path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(old_time))
            .unwrap();

        assert_eq!(store.cleanup_expired("session").unwrap(), 1);
        assert!(!expired_path.exists());
        assert_eq!(
            store.read_for_session("session", &active).unwrap(),
            "active"
        );
    }

    #[test]
    fn cleanup_skips_unexpected_entries_and_never_crosses_sessions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_policy(
            temp_dir.path(),
            restrictive_policy(100, 100, Duration::from_secs(60)),
        );
        let a_reference = store.spill("session_a", "old", "old").unwrap();
        let b_reference = store.spill("session_b", "active", "active").unwrap();
        let a_path = spill_path(&store, "session_a", &a_reference);
        let b_dir = store.session_dir("session_b").unwrap();
        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(61))
            .unwrap();
        OpenOptions::new()
            .write(true)
            .open(&a_path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(old_time))
            .unwrap();
        fs::create_dir(b_dir.join("spill-unexpected-directory")).unwrap();

        assert_eq!(store.cleanup_expired("session_b").unwrap(), 0);
        assert!(a_path.exists());
        assert!(b_dir.join("spill-unexpected-directory").is_dir());
        assert_eq!(
            store.read_for_session("session_b", &b_reference).unwrap(),
            "active"
        );
    }

    #[test]
    fn concurrent_same_session_writers_receive_unique_references() {
        let temp_dir = tempfile::tempdir().unwrap();
        let policy = restrictive_policy(1024, 16 * 1024, Duration::from_secs(60));
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = Vec::new();

        for index in 0..8 {
            let root = temp_dir.path().to_path_buf();
            let policy = policy.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let store = SpillStore::with_policy(root, policy);
                barrier.wait();
                let content = format!("payload-{index}");
                let reference = store.spill("same-session", "parallel", &content).unwrap();
                (store, reference, content)
            }));
        }

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        let references: HashSet<_> = results
            .iter()
            .map(|(_, reference, _)| reference.clone())
            .collect();
        assert_eq!(references.len(), results.len());
        for (store, reference, content) in results {
            assert_eq!(
                store.read_for_session("same-session", &reference).unwrap(),
                content
            );
        }
    }

    #[test]
    fn concurrent_different_sessions_stay_isolated() {
        let temp_dir = tempfile::tempdir().unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let policy = restrictive_policy(1024, 1024, Duration::from_secs(60));
        let mut handles = Vec::new();

        for (session, content) in [("session_a", "a"), ("session_b", "b")] {
            let root = temp_dir.path().to_path_buf();
            let barrier = Arc::clone(&barrier);
            let policy = policy.clone();
            handles.push(thread::spawn(move || {
                let store = SpillStore::with_policy(root, policy);
                barrier.wait();
                let reference = store.spill(session, "parallel", content).unwrap();
                (store, session.to_string(), reference, content.to_string())
            }));
        }

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0]
                .0
                .read_for_session(&results[0].1, &results[0].2)
                .unwrap(),
            results[0].3
        );
        assert!(results[0]
            .0
            .read_for_session(&results[1].1, &results[0].2)
            .is_err());
    }

    #[test]
    fn quota_race_has_exactly_one_winner_across_store_instances() {
        let temp_dir = tempfile::tempdir().unwrap();
        let policy = restrictive_policy(10, 10, Duration::from_secs(60));
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();

        for name in ["one", "two"] {
            let root = temp_dir.path().to_path_buf();
            let policy = policy.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let store = SpillStore::with_policy(root, policy);
                barrier.wait();
                store.spill("quota-session", name, "123456")
            }));
        }

        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes.iter().filter(|outcome| outcome.is_err()).count(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_spill_file_is_restrictive_from_creation() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let reference = store.spill("session", "tool", "contents").unwrap();
        let permissions = fs::metadata(spill_path(&store, "session", &reference))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(permissions & 0o077, 0);
    }

    #[test]
    fn maybe_spill_triggers_above_threshold_without_exposing_absolute_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SpillStore::with_root(temp_dir.path());
        let big_content = "x".repeat(100);
        let small_content = "hello";

        let result_small = store.maybe_spill("s1", "cat", small_content, 50);
        assert_eq!(result_small, "hello");

        let result_big = store.maybe_spill("s1", "cat", &big_content, 50);
        assert!(result_big.contains("工具输出过大"));
        assert!(result_big.contains("会话内引用"));
        assert!(!result_big.contains(temp_dir.path().to_string_lossy().as_ref()));
    }
}
