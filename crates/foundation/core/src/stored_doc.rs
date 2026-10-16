//! 存储文档描述符 + 版本拒开两级容错 (存储基础件).
//!
//! 问题: JSON 配置/记录层「读坏用默认」会静默吞掉损坏。本模块给出两级明确语义:
//!
//! - **单文档类** (配置文件): [`open_single`] 打开即校验版本与结构, 不符/畸形
//!   **拒绝打开**并报结构化错误 ([`StoredDocError`]) —— 不静默迁移, 不回退默认。
//! - **逐记录类** (jsonl / 记录流): [`scan_records`] 把单条坏行**读作不存在**
//!   (跳过并计数), 版本戳不符的记录同样跳过, 不砖化整个单元; 好行照读。
//!
//! 迁移**必须由调用方显式触发**: [`migrate`] (纯转换) 与 [`migrate_file`]
//! (读 → 转换 → 持久写回 + 审计留痕)。迁移失败不落盘。
//!
//! 落盘形态是 [`StoredDoc`] 信封 JSON:
//! `{ "name", "version", "compatible_versions", "body" }`, 其中
//! `compatible_versions` 是写端声明的「语义兼容的格式版本集合」, 必须包含
//! `version` (自洽性校验, 不自洽按畸形拒开/跳过)。
//!
//! 版本契约由读端 [`DocCompat`] 描述: 当前格式版本 + 本构建可读的历史版本集。
//! `version` 大于读端当前版本 = 未知未来版本, 单独报错, 不当作普通不符。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::storage_atomic::{self, DEFAULT_FILE_MODE};

/// 存储文档信封: 描述符字段 + 主体, 整体落盘为 JSON。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDoc<T> {
    /// 文档身份 (稳定字符串, 防止串档打开)。
    pub name: String,
    /// 本文件 body 的格式版本戳。
    pub version: u32,
    /// 写端声明的语义兼容格式版本集合; 必须包含 `version`。
    pub compatible_versions: Vec<u32>,
    /// 文档主体。
    pub body: T,
}

impl<T> StoredDoc<T> {
    /// 构造文档信封 (写端用)。
    pub fn new(
        name: impl Into<String>,
        version: u32,
        compatible_versions: Vec<u32>,
        body: T,
    ) -> Self {
        Self {
            name: name.into(),
            version,
            compatible_versions,
            body,
        }
    }

    /// 信封自洽性: `compatible_versions` 必须包含 `version`。
    pub fn is_self_consistent(&self) -> bool {
        self.compatible_versions.contains(&self.version)
    }
}

/// 读端版本契约: 一个构建如何认一个存储文档。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocCompat {
    /// 期望的文档身份。
    pub name: String,
    /// 读端当前格式版本 (写出时使用)。
    pub current_version: u32,
    /// 读端可读的历史版本集合; 应包含 `current_version`。
    pub readable_versions: Vec<u32>,
}

impl DocCompat {
    /// 构造读端契约。
    pub fn new(name: impl Into<String>, current_version: u32, readable_versions: Vec<u32>) -> Self {
        Self {
            name: name.into(),
            current_version,
            readable_versions,
        }
    }

    /// 仅接受当前版本的契约 (历史版本一律拒开)。
    pub fn exact(name: impl Into<String>, current_version: u32) -> Self {
        Self::new(name, current_version, vec![current_version])
    }
}

/// 版本拒开/迁移的结构化错误。
#[derive(Debug)]
pub enum StoredDocError {
    /// 磁盘 IO 失败。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 底层错误。
        source: std::io::Error,
    },
    /// JSON 畸形或结构不符 (含信封自洽性失败)。
    Malformed {
        /// 文档名 (能解析出时)。
        name: String,
        /// 具体原因。
        reason: String,
    },
    /// 文档身份不符 (串档)。
    NameMismatch {
        /// 期望身份。
        expected: String,
        /// 实际身份。
        found: String,
    },
    /// 版本戳不在读端可读集合内 (普通版本不符)。
    VersionUnsupported {
        /// 文档名。
        name: String,
        /// 实际版本。
        found: u32,
        /// 读端可读版本。
        readable_versions: Vec<u32>,
    },
    /// 未知未来版本 (大于读端当前版本)。
    FutureVersion {
        /// 文档名。
        name: String,
        /// 实际版本。
        found: u32,
        /// 读端当前版本。
        current_version: u32,
    },
    /// 显式迁移的转换步骤失败 (不落盘)。
    MigrationFailed {
        /// 文档名。
        name: String,
        /// 迁移前版本。
        from_version: u32,
        /// 失败原因。
        reason: String,
    },
}

impl std::fmt::Display for StoredDocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "存储文档 IO 失败 {}: {source}", path.display())
            }
            Self::Malformed { name, reason } => {
                write!(f, "存储文档 {name} 畸形或结构不符: {reason}")
            }
            Self::NameMismatch { expected, found } => {
                write!(f, "存储文档身份不符: 期望 {expected}, 实际 {found}")
            }
            Self::VersionUnsupported {
                name,
                found,
                readable_versions,
            } => write!(
                f,
                "存储文档 {name} 版本 {found} 不在可读集合 {readable_versions:?} 内"
            ),
            Self::FutureVersion {
                name,
                found,
                current_version,
            } => write!(
                f,
                "存储文档 {name} 版本 {found} 大于读端当前版本 {current_version} (未知未来版本)"
            ),
            Self::MigrationFailed {
                name,
                from_version,
                reason,
            } => write!(
                f,
                "存储文档 {name} 迁移失败 (来自版本 {from_version}): {reason}"
            ),
        }
    }
}

impl std::error::Error for StoredDocError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl StoredDocError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// 逐记录扫描结果: 好行照读, 坏行/版本不符行读作不存在 (跳过并计数)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordScan<T> {
    /// 解析成功且版本契约相符的记录 (保持文件顺序)。
    pub records: Vec<StoredDoc<T>>,
    /// 被跳过的行数 (坏 JSON / 身份不符 / 自洽性失败 / 版本不符, 合计计数)。
    pub skipped: usize,
}

// ---------------------------------------------------------------------------
// 单文档类: 打开即校验, 不符/畸形拒绝打开
// ---------------------------------------------------------------------------

/// 打开单文档: 解析 + 身份 + 信封自洽性 + 版本契约, 任一不符**拒绝打开**。
///
/// 不静默迁移, 不回退默认。文件不存在按 IO 错误上报 (严格打开, 是否可缺省由
/// 调用方决定)。
pub fn open_single<T: DeserializeOwned>(
    path: &Path,
    compat: &DocCompat,
) -> Result<StoredDoc<T>, StoredDocError> {
    let bytes = std::fs::read(path).map_err(|e| StoredDocError::io(path, e))?;
    let doc: StoredDoc<T> =
        serde_json::from_slice(&bytes).map_err(|e| StoredDocError::Malformed {
            name: compat.name.clone(),
            reason: format!("JSON 解析失败: {e}"),
        })?;
    validate_doc(&doc, compat)?;
    Ok(doc)
}

/// 保存单文档 (持久档原子写): 盖当前版本戳, `compatible_versions` 恰为当前版本。
pub fn save_single<T: Serialize>(
    path: &Path,
    compat: &DocCompat,
    body: T,
    mode: u32,
) -> Result<StoredDoc<T>, StoredDocError> {
    let doc = StoredDoc::new(
        compat.name.clone(),
        compat.current_version,
        vec![compat.current_version],
        body,
    );
    write_doc(path, &doc, mode)?;
    Ok(doc)
}

/// 按原样写出文档信封 (持久档原子写)。信封必须自洽, 否则拒绝写出。
pub fn write_doc<T: Serialize>(
    path: &Path,
    doc: &StoredDoc<T>,
    mode: u32,
) -> Result<(), StoredDocError> {
    if !doc.is_self_consistent() {
        return Err(StoredDocError::Malformed {
            name: doc.name.clone(),
            reason: format!(
                "信封不自洽: version {} 不在 compatible_versions {:?} 内",
                doc.version, doc.compatible_versions
            ),
        });
    }
    let bytes = serde_json::to_vec_pretty(doc).map_err(|e| StoredDocError::Malformed {
        name: doc.name.clone(),
        reason: format!("JSON 序列化失败: {e}"),
    })?;
    storage_atomic::write_atomic_durable(path, &bytes, mode)
        .map_err(|e| StoredDocError::io(path, e))
}

/// 单文档校验: 身份 → 自洽性 → 版本契约 (未来版本单独报错)。
fn validate_doc<T>(doc: &StoredDoc<T>, compat: &DocCompat) -> Result<(), StoredDocError> {
    if doc.name != compat.name {
        return Err(StoredDocError::NameMismatch {
            expected: compat.name.clone(),
            found: doc.name.clone(),
        });
    }
    if !doc.is_self_consistent() {
        return Err(StoredDocError::Malformed {
            name: doc.name.clone(),
            reason: format!(
                "信封不自洽: version {} 不在 compatible_versions {:?} 内",
                doc.version, doc.compatible_versions
            ),
        });
    }
    if doc.version > compat.current_version {
        return Err(StoredDocError::FutureVersion {
            name: doc.name.clone(),
            found: doc.version,
            current_version: compat.current_version,
        });
    }
    if !compat.readable_versions.contains(&doc.version) {
        return Err(StoredDocError::VersionUnsupported {
            name: doc.name.clone(),
            found: doc.version,
            readable_versions: compat.readable_versions.clone(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 逐记录类: 单条坏行读作不存在 (跳过并计数), 不砖化整个单元
// ---------------------------------------------------------------------------

/// 扫描逐记录文件 (jsonl: 每行一个 [`StoredDoc`] 信封 JSON)。
///
/// - 单条坏行 (JSON 解析失败/身份不符/自洽性失败/版本戳不符/未知未来版本)
///   **读作不存在**: 跳过并计入 `skipped`, 不影响其它行。
/// - 好行照读, 保持文件顺序。
/// - 文件不存在视为空流 (记录不存在即读不到)。
/// - 空白行不算记录也不计数 (容忍行尾换行)。
pub fn scan_records<T: DeserializeOwned>(
    path: &Path,
    compat: &DocCompat,
) -> Result<RecordScan<T>, StoredDocError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RecordScan {
                records: Vec::new(),
                skipped: 0,
            });
        }
        Err(e) => return Err(StoredDocError::io(path, e)),
    };
    let mut records = Vec::new();
    let mut skipped = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<StoredDoc<T>>(line) {
            Ok(doc) => {
                if validate_doc(&doc, compat).is_ok() {
                    records.push(doc);
                } else {
                    skipped += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }
    Ok(RecordScan { records, skipped })
}

// ---------------------------------------------------------------------------
// 显式迁移入口: 必须由调用方显式触发, 成功留审计, 失败不落盘
// ---------------------------------------------------------------------------

/// 迁移审计记录 (返回给调用方留档, 并尽力写入旁侧审计文件)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationAudit {
    /// 文档身份。
    pub name: String,
    /// 迁移前格式版本。
    pub from_version: u32,
    /// 迁移后格式版本。
    pub to_version: u32,
    /// 被迁移的文件路径。
    pub path: PathBuf,
    /// 迁移时刻 (epoch 毫秒)。
    pub at_epoch_ms: u64,
    /// 旁侧审计文件是否写入成功。
    pub audit_logged: bool,
}

/// 纯迁移转换: 旧信封 → 新信封 (目标版本戳)。不触碰磁盘。
///
/// 转换失败返回 [`StoredDocError::MigrationFailed`], 由调用方保证不落盘。
pub fn migrate<T, U, F>(
    doc: StoredDoc<T>,
    target: &DocCompat,
    transform: F,
) -> Result<StoredDoc<U>, StoredDocError>
where
    T: DeserializeOwned,
    U: Serialize,
    F: FnOnce(T) -> Result<U, String>,
{
    if doc.name != target.name {
        return Err(StoredDocError::NameMismatch {
            expected: target.name.clone(),
            found: doc.name.clone(),
        });
    }
    let from_version = doc.version;
    let body = transform(doc.body).map_err(|reason| StoredDocError::MigrationFailed {
        name: target.name.clone(),
        from_version,
        reason,
    })?;
    Ok(StoredDoc::new(
        target.name.clone(),
        target.current_version,
        vec![target.current_version],
        body,
    ))
}

/// 显式文件级迁移: 读旧文档 (版本不限, 迁移本就是为版本差而生) → 转换 →
/// 持久写回新版本戳 → 追加审计行。
///
/// 失败语义: 转换失败/写回失败均**不落盘**(目标文件保持原样); 转换成功但
/// 审计行写入失败时文档已迁移, `audit_logged = false` 如实回报, 不假装已审计。
pub fn migrate_file<T, U, F>(
    path: &Path,
    target: &DocCompat,
    mode: u32,
    transform: F,
) -> Result<MigrationAudit, StoredDocError>
where
    T: DeserializeOwned,
    U: Serialize,
    F: FnOnce(T) -> Result<U, String>,
{
    let bytes = std::fs::read(path).map_err(|e| StoredDocError::io(path, e))?;
    let old: StoredDoc<T> =
        serde_json::from_slice(&bytes).map_err(|e| StoredDocError::Malformed {
            name: target.name.clone(),
            reason: format!("JSON 解析失败: {e}"),
        })?;
    if old.name != target.name {
        return Err(StoredDocError::NameMismatch {
            expected: target.name.clone(),
            found: old.name.clone(),
        });
    }
    let from_version = old.version;
    let new_doc = migrate(old, target, transform)?;
    write_doc(path, &new_doc, mode)?;

    let mut audit = MigrationAudit {
        name: target.name.clone(),
        from_version,
        to_version: target.current_version,
        path: path.to_path_buf(),
        at_epoch_ms: epoch_ms(),
        audit_logged: false,
    };
    audit.audit_logged = append_audit_line(path, &audit).is_ok();
    Ok(audit)
}

/// 审计文件路径: 与文档同目录的 `<文件名>.migrate-audit.jsonl`。
fn audit_path(path: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(path.as_os_str());
    name.push(".migrate-audit.jsonl");
    PathBuf::from(name)
}

/// 追加一行迁移审计 (JSONL, 追加写 + 落盘)。
fn append_audit_line(path: &Path, audit: &MigrationAudit) -> std::io::Result<()> {
    use std::io::Write;
    let line = serde_json::to_vec(audit).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("审计序列化失败: {e}"),
        )
    })?;
    let audit_file = audit_path(path);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&audit_file)?;
    file.write_all(&line)?;
    file.write_all(b"\n")?;
    file.sync_all()
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 默认文档权限 (调用方未指定时)。
pub const DEFAULT_DOC_MODE: u32 = DEFAULT_FILE_MODE;

// ---------------------------------------------------------------------------
// 旧裸体格式兼容 (叠加入口): 默认打开只读兼容, 迁移必须显式且留审计
// ---------------------------------------------------------------------------
//
// 前信封时代, body 直接落盘 (无 name/version/compatible_versions 字段)。存量
// 文件必须保持可读, 但**默认打开路径不得静默迁移** (不回写、不升级版本戳):
// 读取兼容以 [`OpenedDoc::LegacyBare`] 如实回报格式, 升级一律走
// [`migrate_file_from_legacy`] / [`migrate_records_from_legacy`] 显式迁移并留审计。

/// 前信封裸体格式的版本号 (body 直接落盘, 无信封字段)。
pub const LEGACY_DOC_VERSION: u32 = 0;

/// 兼容打开结果: 标准信封文档, 或旧裸体 body (只读兼容, 不回写)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenedDoc<T> {
    /// 信封文档 (身份 / 自洽性 / 版本契约已全部校验通过)。
    Envelope(StoredDoc<T>),
    /// 前信封裸体 body (无版本戳; 只读兼容, 升级需显式迁移)。
    LegacyBare(T),
}

impl<T> OpenedDoc<T> {
    /// 文档主体 (信封或旧裸体)。
    pub fn body(&self) -> &T {
        match self {
            Self::Envelope(doc) => &doc.body,
            Self::LegacyBare(body) => body,
        }
    }

    /// 取出文档主体。
    pub fn into_body(self) -> T {
        match self {
            Self::Envelope(doc) => doc.body,
            Self::LegacyBare(body) => body,
        }
    }

    /// 是否为旧裸体格式 (提示调用方可显式迁移)。
    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::LegacyBare(_))
    }
}

/// 逐记录扫描结果 (旧裸体行兼容版): 见 [`scan_records_compat`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordScanCompat<T> {
    /// 解析成功的记录 (信封记录已校验, 旧裸体行读作无戳记录), 保持文件顺序。
    pub records: Vec<OpenedDoc<T>>,
    /// 被跳过的行数 (坏 JSON / 信封畸形 / 身份不符 / 自洽性失败 / 版本不符)。
    pub skipped: usize,
}

/// 信封形状判定: 同时含 `name`/`version`/`body` 键即视为信封。
///
/// 判定只看键的存在 (不看类型), 因此「信封字段写坏」的文件按信封走严格校验并
/// **拒开**, 不会退化成旧裸体解析而静默取默认值。
fn looks_like_envelope(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|obj| {
        obj.contains_key("name") && obj.contains_key("version") && obj.contains_key("body")
    })
}

/// 兼容打开单文档: 信封文件严格校验 (不符拒开); 旧裸体 body 只读兼容
/// (不回写、不迁移); 两者皆不可解析 → 拒开 (不回退默认)。
pub fn open_single_compat<T: DeserializeOwned>(
    path: &Path,
    compat: &DocCompat,
) -> Result<OpenedDoc<T>, StoredDocError> {
    let bytes = std::fs::read(path).map_err(|e| StoredDocError::io(path, e))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| StoredDocError::Malformed {
            name: compat.name.clone(),
            reason: format!("JSON 解析失败: {e}"),
        })?;
    if looks_like_envelope(&value) {
        let doc: StoredDoc<T> =
            serde_json::from_value(value).map_err(|e| StoredDocError::Malformed {
                name: compat.name.clone(),
                reason: format!("信封结构或 body 解析失败: {e}"),
            })?;
        validate_doc(&doc, compat)?;
        return Ok(OpenedDoc::Envelope(doc));
    }
    let body: T = serde_json::from_value(value).map_err(|e| StoredDocError::Malformed {
        name: compat.name.clone(),
        reason: format!("旧裸体 JSON 解析失败: {e}"),
    })?;
    Ok(OpenedDoc::LegacyBare(body))
}

/// 显式迁移「前信封裸体」旧文件 → 当前信封格式: 读裸体 → 转换 → 持久写回新
/// 版本戳 → 追加审计行。文件已是信封格式时**拒绝** (版本迁移请用 [`migrate_file`])。
///
/// 失败语义与 [`migrate_file`] 一致: 转换失败/写回失败均不落盘; 审计行写入
/// 失败时 `audit_logged = false` 如实回报。
pub fn migrate_file_from_legacy<T, U, F>(
    path: &Path,
    target: &DocCompat,
    mode: u32,
    transform: F,
) -> Result<MigrationAudit, StoredDocError>
where
    T: DeserializeOwned,
    U: Serialize,
    F: FnOnce(T) -> Result<U, String>,
{
    let bytes = std::fs::read(path).map_err(|e| StoredDocError::io(path, e))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| StoredDocError::Malformed {
            name: target.name.clone(),
            reason: format!("JSON 解析失败: {e}"),
        })?;
    if looks_like_envelope(&value) {
        return Err(StoredDocError::MigrationFailed {
            name: target.name.clone(),
            from_version: LEGACY_DOC_VERSION,
            reason: "文件已是信封格式: 版本迁移请用 migrate_file".to_string(),
        });
    }
    let legacy: T = serde_json::from_value(value).map_err(|e| StoredDocError::Malformed {
        name: target.name.clone(),
        reason: format!("旧裸体 JSON 解析失败: {e}"),
    })?;
    let body = transform(legacy).map_err(|reason| StoredDocError::MigrationFailed {
        name: target.name.clone(),
        from_version: LEGACY_DOC_VERSION,
        reason,
    })?;
    let doc = StoredDoc::new(
        target.name.clone(),
        target.current_version,
        vec![target.current_version],
        body,
    );
    write_doc(path, &doc, mode)?;

    let mut audit = MigrationAudit {
        name: target.name.clone(),
        from_version: LEGACY_DOC_VERSION,
        to_version: target.current_version,
        path: path.to_path_buf(),
        at_epoch_ms: epoch_ms(),
        audit_logged: false,
    };
    audit.audit_logged = append_audit_line(path, &audit).is_ok();
    Ok(audit)
}

/// 兼容扫描逐记录文件 (jsonl): 信封记录严格校验; 旧裸体行读作无戳记录
/// (只读兼容); 真正坏行/版本不符行跳过并计入 `skipped`。不回写、不迁移。
pub fn scan_records_compat<T: DeserializeOwned>(
    path: &Path,
    compat: &DocCompat,
) -> Result<RecordScanCompat<T>, StoredDocError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RecordScanCompat {
                records: Vec::new(),
                skipped: 0,
            });
        }
        Err(e) => return Err(StoredDocError::io(path, e)),
    };
    let mut records = Vec::new();
    let mut skipped = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if looks_like_envelope(&value) {
            match serde_json::from_value::<StoredDoc<T>>(value) {
                Ok(doc) if validate_doc(&doc, compat).is_ok() => {
                    records.push(OpenedDoc::Envelope(doc))
                }
                _ => skipped += 1,
            }
        } else {
            match serde_json::from_value::<T>(value) {
                Ok(body) => records.push(OpenedDoc::LegacyBare(body)),
                Err(_) => skipped += 1,
            }
        }
    }
    Ok(RecordScanCompat { records, skipped })
}

/// 显式迁移逐记录文件 (jsonl): 旧裸体行 → 当前版本信封行; 信封行与不可解析行
/// **原样保留** (逐记录版本戳由读端裁决, 迁移不代行裁决也不丢行), 整档原子写回
/// + 追加审计行。文件全为信封行时同样重写并留审计 (幂等可重入)。
pub fn migrate_records_from_legacy<T>(
    path: &Path,
    target: &DocCompat,
    mode: u32,
) -> Result<MigrationAudit, StoredDocError>
where
    T: Serialize + DeserializeOwned,
{
    let text = std::fs::read_to_string(path).map_err(|e| StoredDocError::io(path, e))?;
    let mut out = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let wrapped = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) if !looks_like_envelope(&value) => match serde_json::from_value::<T>(value) {
                Ok(body) => {
                    let doc = StoredDoc::new(
                        target.name.clone(),
                        target.current_version,
                        vec![target.current_version],
                        body,
                    );
                    serde_json::to_string(&doc).map_err(|e| StoredDocError::Malformed {
                        name: target.name.clone(),
                        reason: format!("JSON 序列化失败: {e}"),
                    })?
                }
                Err(_) => line.to_string(),
            },
            _ => line.to_string(),
        };
        out.push_str(&wrapped);
        out.push('\n');
    }
    storage_atomic::write_atomic_durable(path, out.as_bytes(), mode)
        .map_err(|e| StoredDocError::io(path, e))?;

    let mut audit = MigrationAudit {
        name: target.name.clone(),
        from_version: LEGACY_DOC_VERSION,
        to_version: target.current_version,
        path: path.to_path_buf(),
        at_epoch_ms: epoch_ms(),
        audit_logged: false,
    };
    audit.audit_logged = append_audit_line(path, &audit).is_ok();
    Ok(audit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_dir(tag: &str) -> PathBuf {
        let n = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-stored-doc-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建测试目录");
        dir
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Config {
        label: String,
        retries: u32,
    }

    fn compat_v2_readable_1_2() -> DocCompat {
        DocCompat::new("demo-config", 2, vec![1, 2])
    }

    #[test]
    fn single_doc_roundtrip_via_save_and_open() {
        let dir = test_dir("roundtrip");
        let path = dir.join("config.json");
        let compat = compat_v2_readable_1_2();
        save_single(
            &path,
            &compat,
            Config {
                label: "a".into(),
                retries: 3,
            },
            DEFAULT_DOC_MODE,
        )
        .unwrap();
        let doc: StoredDoc<Config> = open_single(&path, &compat).unwrap();
        assert_eq!(doc.version, 2);
        assert_eq!(doc.compatible_versions, vec![2]);
        assert_eq!(doc.body.retries, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_doc_rejects_malformed_json_without_default_fallback() {
        // 坏 JSON 必须拒开并报结构化错误, 不得「读坏用默认」。
        let dir = test_dir("malformed");
        let path = dir.join("config.json");
        std::fs::write(&path, "{ this is not json").unwrap();
        let err = open_single::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(
            matches!(err, StoredDocError::Malformed { .. }),
            "坏 JSON 必须报 Malformed: {err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_doc_rejects_version_not_readable() {
        // 版本戳不在读端可读集合 (且非未来版本) → 拒开 (不静默迁移)。
        let dir = test_dir("version-mismatch");
        let path = dir.join("config.json");
        let doc = StoredDoc::new(
            "demo-config",
            1,
            vec![1],
            Config {
                label: "x".into(),
                retries: 0,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
        // 读端只认版本 2: 版本 1 已超出可读集合。
        let err = open_single::<Config>(&path, &DocCompat::exact("demo-config", 2)).unwrap_err();
        assert!(
            matches!(err, StoredDocError::VersionUnsupported { found: 1, .. }),
            "版本不符必须拒开: {err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_doc_rejects_unknown_future_version() {
        // 未知未来版本单独报错, 与普通版本不符区分开。
        let dir = test_dir("future-version");
        let path = dir.join("config.json");
        let doc = StoredDoc::new(
            "demo-config",
            99,
            vec![99],
            Config {
                label: "x".into(),
                retries: 0,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
        let err = open_single::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(
            matches!(
                err,
                StoredDocError::FutureVersion {
                    found: 99,
                    current_version: 2,
                    ..
                }
            ),
            "未知未来版本必须单独拒开: {err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_doc_rejects_name_mismatch_and_inconsistent_stamp() {
        let dir = test_dir("identity");
        let path = dir.join("config.json");
        // 身份不符 (串档)。
        let other = StoredDoc::new(
            "other-doc",
            2,
            vec![2],
            Config {
                label: "x".into(),
                retries: 0,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&other).unwrap()).unwrap();
        let err = open_single::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(
            matches!(err, StoredDocError::NameMismatch { .. }),
            "{err:?}"
        );
        // 信封不自洽 (version 不在 compatible_versions 内) → 按畸形拒开。
        let inconsistent = StoredDoc::new(
            "demo-config",
            2,
            vec![1],
            Config {
                label: "x".into(),
                retries: 0,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&inconsistent).unwrap()).unwrap();
        let err = open_single::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(matches!(err, StoredDocError::Malformed { .. }), "{err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_scan_skips_bad_lines_and_counts() {
        // 单条坏行读作不存在: 跳过并计数, 不砖化整个单元。
        let dir = test_dir("skip-bad");
        let path = dir.join("records.jsonl");
        let good = serde_json::to_string(&StoredDoc::new(
            "demo-config",
            2,
            vec![2],
            Config {
                label: "good".into(),
                retries: 1,
            },
        ))
        .unwrap();
        let content = format!("{good}\n{{ not json at all\n{good}\n");
        std::fs::write(&path, content).unwrap();
        let scan = scan_records::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert_eq!(scan.records.len(), 2, "好行照读: {:#?}", scan.records);
        assert_eq!(scan.skipped, 1, "坏行必须跳过并计数");
        assert!(scan.records.iter().all(|r| r.body.label == "good"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_scan_skips_version_mismatched_records_only() {
        // 版本戳不符的记录同样跳过 (含未知未来版本), 其余好行照读。
        let dir = test_dir("skip-version");
        let path = dir.join("records.jsonl");
        let v1 = serde_json::to_string(&StoredDoc::new(
            "demo-config",
            1,
            vec![1],
            Config {
                label: "v1".into(),
                retries: 1,
            },
        ))
        .unwrap();
        let v2 = serde_json::to_string(&StoredDoc::new(
            "demo-config",
            2,
            vec![2],
            Config {
                label: "v2".into(),
                retries: 2,
            },
        ))
        .unwrap();
        let v99 = serde_json::to_string(&StoredDoc::new(
            "demo-config",
            99,
            vec![99],
            Config {
                label: "v99".into(),
                retries: 9,
            },
        ))
        .unwrap();
        let wrong_name = serde_json::to_string(&StoredDoc::new(
            "other-doc",
            2,
            vec![2],
            Config {
                label: "other".into(),
                retries: 0,
            },
        ))
        .unwrap();
        std::fs::write(&path, format!("{v1}\n{v2}\n{v99}\n{wrong_name}\n\n")).unwrap();
        let scan = scan_records::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert_eq!(scan.records.len(), 2, "v1/v2 都可读: {:#?}", scan.records);
        assert_eq!(scan.skipped, 2, "v99 与串档记录必须跳过并计数");
        let labels: Vec<&str> = scan.records.iter().map(|r| r.body.label.as_str()).collect();
        assert_eq!(labels, vec!["v1", "v2"], "好行照读且保序");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_scan_missing_file_reads_as_empty() {
        let dir = test_dir("missing");
        let path = dir.join("no-such.jsonl");
        let scan = scan_records::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert!(scan.records.is_empty());
        assert_eq!(scan.skipped, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_migration_succeeds_and_logs_audit() {
        // 显式迁移成功路径: 新版本戳落盘 + 审计留痕。
        let dir = test_dir("migrate-ok");
        let path = dir.join("config.json");
        let old = StoredDoc::new(
            "demo-config",
            1,
            vec![1],
            Config {
                label: "keep".into(),
                retries: 1,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&old).unwrap()).unwrap();
        let target = compat_v2_readable_1_2();
        let audit = migrate_file::<Config, Config, _>(&path, &target, DEFAULT_DOC_MODE, |body| {
            Ok(Config {
                retries: body.retries + 1,
                ..body
            })
        })
        .unwrap();
        assert_eq!(audit.from_version, 1);
        assert_eq!(audit.to_version, 2);
        assert!(audit.audit_logged, "成功迁移必须留审计");
        // 迁移后可按新契约打开, 版本戳已更新。
        let doc: StoredDoc<Config> = open_single(&path, &target).unwrap();
        assert_eq!(doc.version, 2);
        assert_eq!(doc.body.retries, 2);
        // 审计文件存在且含一行。
        let audit_file = dir.join("config.json.migrate-audit.jsonl");
        let text = std::fs::read_to_string(&audit_file).unwrap();
        assert_eq!(text.lines().count(), 1, "审计文件应恰一行: {text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_migration_never_touches_disk() {
        // 迁移失败不落盘: 目标文件保持原样, 不写审计。
        let dir = test_dir("migrate-fail");
        let path = dir.join("config.json");
        let old = StoredDoc::new(
            "demo-config",
            1,
            vec![1],
            Config {
                label: "keep".into(),
                retries: 1,
            },
        );
        let original = serde_json::to_vec_pretty(&old).unwrap();
        std::fs::write(&path, &original).unwrap();
        let target = compat_v2_readable_1_2();
        let err = migrate_file::<Config, Config, _>(&path, &target, DEFAULT_DOC_MODE, |_body| {
            Err("转换规则拒绝".to_string())
        })
        .unwrap_err();
        assert!(
            matches!(err, StoredDocError::MigrationFailed { .. }),
            "转换失败必须报 MigrationFailed: {err:?}"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            original,
            "迁移失败后目标文件必须保持原样"
        );
        assert!(
            !dir.join("config.json.migrate-audit.jsonl").exists(),
            "失败迁移不得留审计"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // 旧裸体格式兼容: 只读兼容 + 显式迁移留审计
    // -----------------------------------------------------------------------

    fn legacy_bare_bytes() -> Vec<u8> {
        serde_json::to_vec_pretty(&Config {
            label: "keep".into(),
            retries: 7,
        })
        .unwrap()
    }

    #[test]
    fn compat_open_reads_legacy_bare_without_touching_the_file() {
        // 好数据路径零回归: 旧裸体 body 逐字节等价读出, 且默认打开不静默迁移
        // (文件字节原样)。
        let dir = test_dir("compat-open");
        let path = dir.join("config.json");
        let original = legacy_bare_bytes();
        std::fs::write(&path, &original).unwrap();

        let opened = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert!(opened.is_legacy(), "旧裸体必须如实标记为 legacy");
        assert_eq!(opened.body().retries, 7);
        // body 再序列化与旧写端输出逐字节等价。
        assert_eq!(
            serde_json::to_vec_pretty(opened.body()).unwrap(),
            original,
            "好数据路径必须逐字节等价"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            original,
            "默认打开路径不得改写旧文件 (不静默迁移)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compat_open_rejects_envelope_shaped_garbage_without_default_fallback() {
        // 信封形状但 body 写坏: 必须拒开报结构化错误, 不得退化成旧裸体解析而
        // 「读坏用默认」。
        let dir = test_dir("compat-reject");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            br#"{"name":"demo-config","version":2,"compatible_versions":[2],"body":{"retries":"not-a-number"}}"#,
        )
        .unwrap();
        let err = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(matches!(err, StoredDocError::Malformed { .. }), "{err:?}");
        // 旧裸体也解析不了 → 同样拒开。
        std::fs::write(&path, b"{ not json").unwrap();
        let err = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(matches!(err, StoredDocError::Malformed { .. }), "{err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compat_open_still_enforces_identity_and_version_contract() {
        // 兼容打开不放松信封校验: 串档 / 未知未来版本照旧拒开。
        let dir = test_dir("compat-contract");
        let path = dir.join("config.json");
        let wrong_name = StoredDoc::new(
            "other-doc",
            2,
            vec![2],
            Config {
                label: "x".into(),
                retries: 0,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&wrong_name).unwrap()).unwrap();
        let err = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(
            matches!(err, StoredDocError::NameMismatch { .. }),
            "{err:?}"
        );

        let future = StoredDoc::new(
            "demo-config",
            99,
            vec![99],
            Config {
                label: "x".into(),
                retries: 0,
            },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&future).unwrap()).unwrap();
        let err = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap_err();
        assert!(
            matches!(err, StoredDocError::FutureVersion { .. }),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_legacy_migration_wraps_envelope_and_logs_audit() {
        // 显式迁移: 旧裸体 → 信封; 迁移前后都可读, 迁移留审计。
        let dir = test_dir("legacy-migrate");
        let path = dir.join("config.json");
        let original = legacy_bare_bytes();
        std::fs::write(&path, &original).unwrap();

        // 迁移前可读 (旧裸体兼容)。
        let before = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert_eq!(before.body().retries, 7);

        let audit = migrate_file_from_legacy::<Config, Config, _>(
            &path,
            &compat_v2_readable_1_2(),
            DEFAULT_DOC_MODE,
            Ok,
        )
        .unwrap();
        assert_eq!(audit.from_version, LEGACY_DOC_VERSION);
        assert_eq!(audit.to_version, 2);
        assert!(audit.audit_logged, "显式迁移必须留审计");

        // 迁移后可读 (信封)。
        let after = open_single_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert!(!after.is_legacy(), "迁移后必须是信封格式");
        assert_eq!(after.body().retries, 7, "迁移不得改变好数据内容");
        // 审计文件恰一行。
        let text = std::fs::read_to_string(dir.join("config.json.migrate-audit.jsonl")).unwrap();
        assert_eq!(text.lines().count(), 1, "审计文件应恰一行: {text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_migration_refuses_envelopes_and_failed_transform_keeps_file() {
        let dir = test_dir("legacy-migrate-guard");
        let path = dir.join("config.json");
        // 已是信封 → 拒绝 (版本迁移归 migrate_file)。
        save_single(
            &path,
            &compat_v2_readable_1_2(),
            Config {
                label: "a".into(),
                retries: 1,
            },
            DEFAULT_DOC_MODE,
        )
        .unwrap();
        let err = migrate_file_from_legacy::<Config, Config, _>(
            &path,
            &compat_v2_readable_1_2(),
            DEFAULT_DOC_MODE,
            Ok,
        )
        .unwrap_err();
        assert!(
            matches!(err, StoredDocError::MigrationFailed { .. }),
            "{err:?}"
        );

        // 转换失败 → 旧裸体文件原样保留, 无审计。
        let original = legacy_bare_bytes();
        std::fs::write(&path, &original).unwrap();
        let err = migrate_file_from_legacy::<Config, Config, _>(
            &path,
            &compat_v2_readable_1_2(),
            DEFAULT_DOC_MODE,
            |_| Err("转换规则拒绝".to_string()),
        )
        .unwrap_err();
        assert!(
            matches!(err, StoredDocError::MigrationFailed { .. }),
            "{err:?}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), original, "失败迁移不得落盘");
        assert!(!dir.join("config.json.migrate-audit.jsonl").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compat_record_scan_keeps_legacy_rows_and_counts_bad_lines() {
        // 逐记录类: 旧裸体行只读兼容照读, 坏行跳过并计数。
        let dir = test_dir("compat-scan");
        let path = dir.join("records.jsonl");
        let legacy = serde_json::to_string(&Config {
            label: "legacy".into(),
            retries: 1,
        })
        .unwrap();
        let envelope = serde_json::to_string(&StoredDoc::new(
            "demo-config",
            2,
            vec![2],
            Config {
                label: "v2".into(),
                retries: 2,
            },
        ))
        .unwrap();
        std::fs::write(&path, format!("{legacy}\n{{ not json\n{envelope}\n")).unwrap();

        let scan = scan_records_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert_eq!(scan.records.len(), 2, "{:#?}", scan.records);
        assert_eq!(scan.skipped, 1, "坏行必须跳过并计数");
        assert!(scan.records[0].is_legacy(), "旧裸体行必须如实标记");
        assert!(!scan.records[1].is_legacy());
        assert_eq!(scan.records[0].body().label, "legacy");
        assert_eq!(scan.records[1].body().label, "v2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_record_migration_wraps_legacy_rows_and_keeps_others_verbatim() {
        // 逐记录显式迁移: 旧裸体行包信封, 信封行/坏行原样保留 + 审计留痕。
        let dir = test_dir("records-migrate");
        let path = dir.join("records.jsonl");
        let legacy = serde_json::to_string(&Config {
            label: "legacy".into(),
            retries: 1,
        })
        .unwrap();
        let envelope = serde_json::to_string(&StoredDoc::new(
            "demo-config",
            2,
            vec![2],
            Config {
                label: "v2".into(),
                retries: 2,
            },
        ))
        .unwrap();
        let bad = "{ not json".to_string();
        std::fs::write(&path, format!("{legacy}\n{envelope}\n{bad}\n")).unwrap();

        let audit = migrate_records_from_legacy::<Config>(
            &path,
            &compat_v2_readable_1_2(),
            DEFAULT_DOC_MODE,
        )
        .unwrap();
        assert_eq!(audit.from_version, LEGACY_DOC_VERSION);
        assert!(audit.audit_logged, "逐记录迁移必须留审计");

        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "迁移不得丢行: {text}");
        assert_eq!(lines[1], envelope, "信封行必须原样保留");
        assert_eq!(lines[2], bad, "坏行必须原样保留 (不代行裁决)");
        assert_ne!(lines[0], legacy, "旧裸体行必须被包进信封");

        // 迁移后扫描: 3 行中 2 条可读 (1 跳过), 且旧裸体标记消失。
        let scan = scan_records_compat::<Config>(&path, &compat_v2_readable_1_2()).unwrap();
        assert_eq!(scan.records.len(), 2);
        assert_eq!(scan.skipped, 1);
        assert!(scan.records.iter().all(|r| !r.is_legacy()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
