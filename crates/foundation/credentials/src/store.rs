//! **TP3/N21 / 存储层 — 按服务名读写凭据**
//!
//! **统一接口** [`CredentialsStore`] + **文件形态后端** [`FileCredentialsStore`]。
//!
//! **0 假装'安全存储'边界 (任务纪律, 如实标注)**:
//! 本层是**凭据存取抽象** — 统一读写接口 + 文件后端 + 脱敏输出 + 权限 600 语义。
//! **它不是加密保险库**: 文件后端在磁盘上是**明文静态存储** (靠 OS 文件权限收敛访问),
//! 加密静态存储 (KMS / age / OS keyring) 属**后续层**, 此处如实标注不假装。
//!
//! **文件权限 600 语义**: unix 下凭据文件以 `mode(0o600)` **创建**并收敛到精确值
//! (统一存储基础件保证) — 凭据自落盘起即 0600, 无"先写后 chmod"的短暂暴露窗口;
//! 非 unix (Windows) 无 unix mode 语义, 权限依赖默认 ACL,
//! 语义等价由部署保证, 此处标注 (0 假装边界).
//!
//! **原子写 + 串行化 (M1 修复)**: `save` 走统一存储基础件的**持久档原子写**
//! (独占临时文件 0600 + `sync_all` + rename, 崩溃不留半写, 不丢全表);
//! `set`/`delete` 的 load-modify-save 由进程内 [`std::sync::Mutex`] + 跨进程
//! 文件锁 (`with_file_lock`) 双层串行化
//! (trait 方法是 `&self`, 调用方无需自行同步; 跨进程并发不丢更新).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use apeireth_core::storage_atomic::{self, with_file_lock, OWNER_ONLY_MODE};

use crate::error::{CredentialsError, Result};
use crate::secret::SecretString;

/// 统一凭据存取接口 (按服务名)。
///
/// 实现: [`FileCredentialsStore`] (文件后端)。加密后端属后续层。
pub trait CredentialsStore {
    /// 读取服务凭据; 服务不存在 → [`CredentialsError::UnknownService`]。
    fn get(&self, service: &str) -> Result<SecretString>;

    /// 写入/覆盖服务凭据 (写入前做合法性检查, 见 [`validate_service_name`])。
    fn set(&self, service: &str, secret: SecretString) -> Result<()>;

    /// 删除服务凭据; 服务不存在 → [`CredentialsError::UnknownService`]。
    fn delete(&self, service: &str) -> Result<()>;

    /// 列出已存服务名 (仅名称, 不含明文)。
    fn list(&self) -> Result<Vec<String>>;

    /// 是否存在该服务凭据 (不取明文)。
    fn contains(&self, service: &str) -> Result<bool>;
}

/// 服务名合法性校验 (防注入/空名/控制字符)。
///
/// 允许: 非空, 仅 `A-Za-z0-9 . _ -`, 长度 ≤ 128。拒绝路径分隔符 (防穿越),
/// 拒绝 `.` / `..` / 点开头 (防路径穿越与隐藏文件)。
pub fn validate_service_name(service: &str) -> Result<()> {
    if service.is_empty() || service.len() > 128 {
        return Err(CredentialsError::InvalidServiceName(service.to_string()));
    }
    if service == "." || service == ".." || service.starts_with('.') {
        return Err(CredentialsError::InvalidServiceName(service.to_string()));
    }
    let ok = service
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !ok {
        return Err(CredentialsError::InvalidServiceName(service.to_string()));
    }
    Ok(())
}

/// 文件形态凭据后端。
///
/// 单个 JSON 文件承载 `服务名 -> 明文` 映射。**明文静态存储** (0 假装边界见模块头),
/// 靠文件权限 600 语义收敛访问。加密后端属后续层。
///
/// `set`/`delete` 的读-改-写由进程内写锁 + 跨进程文件锁双层串行化; 写路径为
/// 统一存储基础件的持久档原子写 (unix 创建即 0600, 见 [`FileCredentialsStore::save`])。
pub struct FileCredentialsStore {
    path: PathBuf,
    /// 进程内写锁: 串行化 `set`/`delete` 的 load-modify-save (M1②);
    /// poison 后取守卫值继续 (凭据表状态完整性优先于线程 unwind 传播).
    write_lock: Mutex<()>,
}

impl FileCredentialsStore {
    /// 以存储文件路径构造 (父目录不存在则创建)。
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| CredentialsError::Io {
                    service: "<store>".into(),
                    source,
                })?;
            }
        }
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    /// 存储文件路径 (元信息)。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 读取全表 (文件不存在 → 空表)。
    fn load(&self) -> Result<BTreeMap<String, String>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let raw = std::fs::read_to_string(&self.path).map_err(|source| CredentialsError::Io {
            service: "<store>".into(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|e| CredentialsError::Format {
            service: "<store>".into(),
            message: e.to_string(),
        })
    }

    /// 原子写回全表 (统一存储基础件持久档: 独占临时文件 0600 + `sync_all` + rename)。
    ///
    /// M1①③: unix 下凭据**自落盘起**即 0600 (无"先 write 后 chmod"的 umask
    /// 0644 暴露窗口), 且替换原子 — 崩溃不会留下半写 JSON 丢全表, 亦不回退。
    /// 非 unix: 无 unix mode 语义 (权限由 OS 默认 ACL 承载), 原子替换与落盘照常。
    fn save(&self, map: &BTreeMap<String, String>) -> Result<()> {
        let json = serde_json::to_string_pretty(map).map_err(|e| CredentialsError::Format {
            service: "<store>".into(),
            message: e.to_string(),
        })?;
        storage_atomic::write_atomic_durable(&self.path, json.as_bytes(), OWNER_ONLY_MODE).map_err(
            |source| CredentialsError::Io {
                service: "<store>".into(),
                source,
            },
        )
    }

    /// `set`/`delete` 的 load-modify-save 在「进程内写锁 + 跨进程文件锁」内执行。
    fn mutate_locked(
        &self,
        op: impl FnOnce(&mut BTreeMap<String, String>) -> Result<()>,
    ) -> Result<()> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let lock_path = storage_atomic::lock_path_for(&self.path);
        let inner = with_file_lock(&lock_path, || {
            let mut map = self.load()?;
            op(&mut map)?;
            self.save(&map)
        })
        .map_err(|source| CredentialsError::Io {
            service: "<store>".into(),
            source,
        })?;
        inner
    }
}

impl CredentialsStore for FileCredentialsStore {
    fn get(&self, service: &str) -> Result<SecretString> {
        validate_service_name(service)?;
        let map = self.load()?;
        match map.get(service) {
            Some(v) => Ok(SecretString::new(v.clone())),
            None => Err(CredentialsError::UnknownService(service.to_string())),
        }
    }

    fn set(&self, service: &str, secret: SecretString) -> Result<()> {
        validate_service_name(service)?;
        // M1②: load-modify-save 在进程内写锁 + 跨进程文件锁内串行化 (防并发丢更新).
        self.mutate_locked(|map| {
            map.insert(service.to_string(), secret.expose().to_string());
            Ok(())
        })
    }

    fn delete(&self, service: &str) -> Result<()> {
        validate_service_name(service)?;
        // M1②: 同 set — 整个 load-modify-save 在双层锁内.
        self.mutate_locked(|map| {
            if map.remove(service).is_none() {
                return Err(CredentialsError::UnknownService(service.to_string()));
            }
            Ok(())
        })
    }

    fn list(&self) -> Result<Vec<String>> {
        Ok(self.load()?.into_keys().collect())
    }

    fn contains(&self, service: &str) -> Result<bool> {
        validate_service_name(service)?;
        Ok(self.load()?.contains_key(service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store(name: &str) -> FileCredentialsStore {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-credentials-test-{}-{}",
            std::process::id(),
            name
        ));
        FileCredentialsStore::new(dir.join("creds.json")).expect("store")
    }

    #[test]
    fn set_then_get_roundtrip() {
        let s = tmp_store("roundtrip");
        s.set("openai", SecretString::new("sk-test-123")).unwrap();
        let got = s.get("openai").unwrap();
        assert_eq!(got.expose(), "sk-test-123");
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn get_unknown_service_errors() {
        let s = tmp_store("unknown");
        let e = s.get("nonexistent-service").unwrap_err();
        assert!(matches!(e, CredentialsError::UnknownService(_)));
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn delete_unknown_service_errors() {
        let s = tmp_store("del-unknown");
        assert!(matches!(
            s.delete("ghost").unwrap_err(),
            CredentialsError::UnknownService(_)
        ));
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn delete_then_get_is_unknown() {
        let s = tmp_store("delete");
        s.set("github", SecretString::new("ghp_x")).unwrap();
        s.delete("github").unwrap();
        assert!(matches!(
            s.get("github").unwrap_err(),
            CredentialsError::UnknownService(_)
        ));
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn list_and_contains() {
        let s = tmp_store("list");
        s.set("a", SecretString::new("1")).unwrap();
        s.set("b", SecretString::new("2")).unwrap();
        let mut names = s.list().unwrap();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
        assert!(s.contains("a").unwrap());
        assert!(!s.contains("zz").unwrap());
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn invalid_service_name_rejected() {
        let s = tmp_store("invalid");
        for bad in ["", "a/b", "a\\b", "a b", "a:b", "..", ".", ".hidden"] {
            assert!(
                s.set(bad, SecretString::new("x")).is_err(),
                "应拒绝非法名: {bad:?}"
            );
        }
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn error_messages_do_not_leak_secret() {
        let s = tmp_store("noleak");
        let e = s.get("missing-svc").unwrap_err();
        let msg = format!("{e}");
        assert!(!msg.contains("sk-"), "错误不得含明文");
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn validate_name_rules() {
        assert!(validate_service_name("openai").is_ok());
        assert!(validate_service_name("my-service.v2_prod").is_ok());
        assert!(validate_service_name("").is_err());
        assert!(validate_service_name("a/b").is_err());
    }

    #[test]
    fn concurrent_sets_do_not_lose_updates() {
        // M1② 回归: 多线程并发 set/delete 不得丢更新 (进程内写锁串行化).
        let s = std::sync::Arc::new(tmp_store("concurrent"));
        let mut handles = Vec::new();
        for i in 0..8 {
            let s = s.clone();
            handles.push(std::thread::spawn(move || {
                for j in 0..20 {
                    let name = format!("svc-{i}-{j}");
                    s.set(&name, SecretString::new("v")).unwrap();
                }
            }));
        }
        for h in handles {
            h.join().expect("thread");
        }
        // 8 * 20 = 160 条必须全部在 (无锁旧实现会大规模丢更新).
        assert_eq!(s.list().unwrap().len(), 160, "并发 set 不应丢更新");
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[test]
    fn corrupted_file_refuses_set_instead_of_overwriting() {
        // M1③/H5 同类语义: 文件损坏 (非 JSON) 时 set 必须报错拒绝,
        // 不得"空表起步"把损坏文件里可能恢复的内容静默清掉.
        let s = tmp_store("corrupt");
        std::fs::write(s.path(), "{ this is not json").unwrap();
        let before = std::fs::read(s.path()).unwrap();
        let e = s.set("svc", SecretString::new("v"));
        assert!(
            matches!(e, Err(CredentialsError::Format { .. })),
            "损坏文件上 set 应拒绝: {e:?}"
        );
        assert_eq!(
            std::fs::read(s.path()).unwrap(),
            before,
            "损坏文件不得被静默覆盖"
        );
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn file_permission_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let s = tmp_store("perm");
        s.set("svc", SecretString::new("v")).unwrap();
        let mode = std::fs::metadata(s.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "应为 600, 实际 {mode:o}");
        let _ = std::fs::remove_dir_all(s.path().parent().unwrap());
    }
}
