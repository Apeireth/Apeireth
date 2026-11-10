//! G5 备份恢复 (伙伴的「记忆保险箱」): 备份格式 + 导出/恢复/校验引擎 + `tool.backup_export`。
//!
//! 横切能力: 运行时的全部记忆/账本**可备份、可搬家**。一个引擎, 两个出口:
//! 模型可调的 [`BackupExportTool`] (`tool.backup_export`, AI 先用) 与 CLI 的
//! `apeireth backup export|restore|verify` (同一内核的包装出口, 不另起炉灶)。
//!
//! # 备份格式 (目录快照 + manifest.json)
//!
//! ```text
//! <bundle>/manifest.json          清单: 格式版本 + 产品版本 + 时间 + 逐文件摘要
//!                                 + 凭据排除面 (显式列出, 凭据不随行)
//! <bundle>/files/<category>/<relpath>   逐文件原字节快照 (类别 = 七类数据面)
//! ```
//!
//! 选**目录快照**而非归档容器 (tar/zip 类): workspace 纪律是 0 引新外部依赖,
//! 归档容器都需要新依赖; 目录快照 + 逐文件 SHA-256 摘要对「损坏可检出」是
//! 充分条件, 且逐文件原子落位更贴合恢复语义 (见下)。
//!
//! # 三条硬语义 (钉死, 测试为证)
//!
//! 1. **凭据不随行**: 密钥/令牌面 ([`CREDENTIAL_ARTIFACTS_EXCLUDED`] 显式排除表
//!    + 既有敏感路径策略) 永不进包; 排除项在 manifest `excluded` 里**如实列出**
//!    (不静默丢件), 测试逐字节断言打包内零密钥。
//! 2. **宁拒不损**: 恢复前全量校验 (清单可解析 + 版本匹配 + 逐文件摘要) ——
//!    任一不符**拒绝恢复**, 一个字节都不落盘; 落位中途失败按原状回滚。
//! 3. **版本兼容 = 显式拒** (二选一钉死): `format_version` 与本实现不符的备份
//!    **显式拒绝**并报出双方版本号 (迁移语义是被拒的 alternative, 理由见规格)。
//!
//! # 增量语义 (备份后新增数据不丢不覆盖)
//!
//! - 每次导出都是**全量快照**: 备份后新增的数据进入下一份全量 (不丢);
//! - 导出**永不覆盖**已存在的 bundle (目标已存在 = 拒绝即帧);
//! - 恢复只落位 manifest 成员集: 备份后新增的**非成员**文件原样保留 (不丢不覆盖),
//!   成员文件回到 bundle 字节态 (这就是「恢复」的定义, 规格逐条钉死)。

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::CapabilityId;
use apeireth_core::RELEASE_VERSION;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::exec_pipeline::{OutputSchema, SchemaKind};
use crate::self_status::{BackupStatus, StatusProbe};

/// 备份格式版本 (manifest `format_version`)。**版本兼容语义 = 显式拒**:
/// 不等于本常量的备份一律拒绝恢复/校验 (宁拒不损; 迁移语义为被拒 alternative)。
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// bundle 内清单文件名。
pub const MANIFEST_FILE_NAME: &str = "manifest.json";

/// bundle 内逐文件快照的根目录名 (`<bundle>/files/<category>/<relpath>`)。
pub const BUNDLE_FILES_DIR: &str = "files";

/// 数据根下的「最近备份」记录文件名 (self_status 备份节的真值来源)。
pub const BACKUP_LEDGER_FILE_NAME: &str = "last-backup.json";

/// 备份导出工具能力 id (`tool.backup_export`)。
pub const BACKUP_EXPORT_CAPABILITY_ID: &str = "tool.backup_export";

/// 审计行事件名: 备份导出 (记时间与清单数)。
pub const AUDIT_EVENT_EXPORT: &str = "backup.export";

/// 审计行事件名: 备份恢复 (记时间与清单数)。
pub const AUDIT_EVENT_RESTORE: &str = "backup.restore";

/// 凭据不随行: **显式排除**的密钥/令牌面文件名 (数据根下, 大小写不敏感)。
///
/// 这是「凭据不随行」的显式排除表 (密钥/令牌面逐一点名), 与既有敏感路径
/// 策略 ([`crate::sensitive_path`]) 双层生效: 显式表管产品自身的凭据/令牌
/// 落盘件, 敏感路径策略管密钥形状的通用件 (`.env` 族 / 私钥 / apikey 族)。
/// 被排除的路径**逐条记进 manifest `excluded`** (不静默丢件)。
pub const CREDENTIAL_ARTIFACTS_EXCLUDED: &[&str] = &[
    // 文件凭据存储落盘件 (含崩溃残留变体)。
    "creds.json",
    // 运行时凭据库 (加密文件后端的数据件与主密钥)。
    "apeireth-keyring.bin",
    "apeireth-keyring.master.key",
    "keyring",
    // 令牌面: 网关客户端令牌表。
    "client-tokens.json",
    // 密钥面: 渠道配置 (携共享签名密钥)。
    "im-channels.json",
];

/// 恢复落位的临时件后缀 (先写临时再换名; 带 pid 防并发互踩)。
const RESTORE_TMP_SUFFIX: &str = ".apeireth-restore-tmp";

/// 恢复落位的原件保全后缀 (换名期间暂存原文件, 成功后清理/失败后回滚)。
const RESTORE_BAK_SUFFIX: &str = ".apeireth-restore-bak";

/// 导出暂存目录后缀 (bundle 全量写齐后一次换名就位)。
const EXPORT_STAGING_SUFFIX: &str = ".apeireth-export-staging";

/// 备份类别 (七类数据面: 会话库/认知库/记忆库/技能卡/关怀规则/用户表/域文档)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupCategory {
    /// 会话库 (`sessions.sqlite3` + WAL 侧车; 默认域与逐域)。
    Sessions,
    /// 认知库 (默认域 `cognitive.sqlite3` + WAL 侧车)。
    Cognitive,
    /// 记忆库 (逐域 `users/<id>/data/cognitive.sqlite3`, 每户独立记忆域)。
    Memory,
    /// 技能卡 (数据目录 `skills/`, 每卡一 StoredDoc)。
    Skills,
    /// 关怀规则 (`care-rules.json` + `care-rules/`; 现无写入方 = 打包 0 件,
    /// manifest 如实记, 类目为在途关怀面预留)。
    CareRules,
    /// 用户表 (`users.json`)。
    Users,
    /// 域文档 (`users/<id>/domain.json`, 每域一份)。
    Domains,
}

impl BackupCategory {
    /// 类目词面 (manifest `category` 字段与 bundle 路径段共用)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sessions => "sessions",
            Self::Cognitive => "cognitive",
            Self::Memory => "memory",
            Self::Skills => "skills",
            Self::CareRules => "care_rules",
            Self::Users => "users",
            Self::Domains => "domains",
        }
    }
}

/// 一个备份源 (单文件或整树)。
#[derive(Debug, Clone)]
pub struct BackupItem {
    /// 类目归属。
    pub category: BackupCategory,
    /// 源路径 (导出时的原位路径; 同机恢复的默认落位目标)。
    pub origin: PathBuf,
    /// 逻辑归属路径 (相对恢复根的家; 搬家恢复按它重组目录结构)。
    pub relpath: PathBuf,
    /// `true` = origin 是目录, 递归收录 (跳过符号链接与排除面);
    /// `false` = origin 是单文件。
    pub tree: bool,
}

impl BackupItem {
    /// 单文件源 (不存在时导出跳过, 类目计数如实为 0)。
    pub fn file(
        category: BackupCategory,
        origin: impl Into<PathBuf>,
        relpath: impl Into<PathBuf>,
    ) -> Self {
        Self {
            category,
            origin: origin.into(),
            relpath: relpath.into(),
            tree: false,
        }
    }

    /// 整树源 (目录递归收录, 相对路径挂在 `relprefix` 下)。
    pub fn tree(
        category: BackupCategory,
        origin: impl Into<PathBuf>,
        relprefix: impl Into<PathBuf>,
    ) -> Self {
        Self {
            category,
            origin: origin.into(),
            relpath: relprefix.into(),
            tree: true,
        }
    }
}

/// 一次导出的全部数据源 (组装根按实际生效路径解析; 引擎只认这份显式清单)。
#[derive(Debug, Clone)]
pub struct BackupSources {
    /// 数据根 (manifest 记录的出处; 搬家恢复的缺省重组基准)。
    pub data_root: PathBuf,
    /// 源清单 (按声明序枚举, 落包前按 bundle 路径排序)。
    pub items: Vec<BackupItem>,
}

impl BackupSources {
    /// 一组源 (数据根 + 源清单)。
    pub fn new(data_root: impl Into<PathBuf>, items: Vec<BackupItem>) -> Self {
        Self {
            data_root: data_root.into(),
            items,
        }
    }
}

/// 逐文件清单行 (损坏可检出: `bytes` + `sha256` 双查)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupFileEntry {
    /// 类目词面 ([`BackupCategory::as_str`])。
    pub category: String,
    /// bundle 内相对路径 (`files/<category>/<relpath>`, `/` 分隔)。
    pub path: String,
    /// 导出时的原位绝对路径 (同机恢复的默认落位目标; 纯出处信息)。
    pub origin: String,
    /// 逻辑归属路径 (`/` 分隔; `--to <root>` 搬家恢复按它重组)。
    pub relpath: String,
    /// 文件字节数。
    pub bytes: u64,
    /// 逐文件 SHA-256 摘要 (hex)。
    pub sha256: String,
    /// 源文件 mtime (epoch 毫秒; 取不到显式 null, 不编时间)。
    pub mtime_ms: Option<i64>,
}

/// 显式排除行 (凭据不随行: 被排除的路径逐条列出, 不静默丢件)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedEntry {
    /// 被排除路径 (相对数据根; 数据根外的用原位路径)。
    pub path: String,
    /// 排除原因 (凭据面/敏感路径/符号链接, 逐条如实)。
    pub reason: String,
}

/// 备份清单 (manifest.json 的唯一形状)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// 备份格式版本 ([`BACKUP_FORMAT_VERSION`])。
    pub format_version: u32,
    /// 产品版本 (出处真值, 不影响兼容判定)。
    pub product_version: String,
    /// 导出时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 数据根 (出处真值)。
    pub data_root: String,
    /// 逐文件清单 (按 bundle 路径排序)。
    pub files: Vec<BackupFileEntry>,
    /// 显式排除面 (凭据不随行的逐条记录)。
    pub excluded: Vec<ExcludedEntry>,
}

impl BackupManifest {
    /// 清单件数。
    pub fn entry_count(&self) -> u64 {
        self.files.len() as u64
    }

    /// 逐类目件数 (排序稳定)。
    pub fn category_counts(&self) -> BTreeMap<String, u64> {
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        for entry in &self.files {
            *counts.entry(entry.category.clone()).or_insert(0) += 1;
        }
        counts
    }
}

/// 备份引擎错误 (错误即帧: 拒绝原因逐条可分类)。
#[derive(Debug, Error)]
pub enum BackupError {
    /// 文件系统读写失败。
    #[error("backup io failed: {0}")]
    Io(#[from] std::io::Error),
    /// 清单不可解析/结构非法 (损坏可检出 → 拒)。
    #[error("backup manifest is corrupt: {0}")]
    ManifestCorrupt(String),
    /// 目标目录**不是一个备份包**（不存在 / 没有清单）。
    ///
    /// 与 [`Self::ManifestCorrupt`] **分开**：路径打错是客户端输入问题，
    /// 说成「清单损坏」会让人以为自己的备份坏了（真机实测 2026-10-25）。
    #[error("{0}")]
    NotABundle(String),
    /// 备份格式版本不匹配 (版本兼容语义 = 显式拒, 宁拒不损)。
    #[error(
        "backup format version mismatch: bundle={found}, supported={expected} (版本不兼容: 显式拒, 宁拒不损)"
    )]
    VersionMismatch {
        /// bundle 里声明的格式版本。
        found: u32,
        /// 本实现支持的格式版本。
        expected: u32,
    },
    /// 逐文件摘要/大小不符 (损坏可检出 → 拒恢复, 宁拒不损)。
    #[error("backup digest mismatch for `{path}`: {reason} (摘要不符: 拒, 宁拒不损)")]
    DigestMismatch {
        /// 不符的清单行 bundle 路径。
        path: String,
        /// 不符事实 (大小/摘要)。
        reason: String,
    },
    /// 目标面拒绝 (不覆盖 / 非文件占位 / 重复落位目标)。
    #[error("backup operation refused: {0}")]
    Refused(String),
}

/// 「最近备份」记录 (数据根 `last-backup.json`; self_status 备份节真值来源)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupLedger {
    /// 最近一次导出时刻 (epoch 毫秒)。
    pub exported_at_ms: i64,
    /// 该次导出的清单件数。
    pub manifest_entries: u64,
    /// 该次导出的 bundle 路径 (出处信息)。
    pub bundle: String,
    /// 该次导出的备份格式版本。
    pub format_version: u32,
}

/// 备份审计口 (治理面: CLI 与工具两个出口共用同一审计行语义)。
pub trait BackupAudit: Send + Sync {
    /// 记一行审计 (事件名 + 明细; 时间由实现方盖)。
    fn record(&self, event: &str, detail: &str);
}

/// 引擎侧钩子 (审计行 + 最近备份记录; 两个出口注入同一实现)。
#[derive(Clone, Default)]
pub struct BackupHooks {
    /// 「最近备份」记录路径 (缺省不落记录)。
    pub ledger_path: Option<PathBuf>,
    /// 审计口 (缺省不记审计行)。
    pub audit: Option<Arc<dyn BackupAudit>>,
}

impl std::fmt::Debug for BackupHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupHooks")
            .field("ledger_path", &self.ledger_path)
            .field("audit", &self.audit.as_ref().map(|_| "configured"))
            .finish()
    }
}

impl BackupHooks {
    /// 空钩子 (无审计、无记录)。
    pub fn none() -> Self {
        Self::default()
    }

    /// 接上「最近备份」记录路径。
    #[must_use]
    pub fn with_ledger(mut self, ledger_path: impl Into<PathBuf>) -> Self {
        self.ledger_path = Some(ledger_path.into());
        self
    }

    /// 接上审计口。
    #[must_use]
    pub fn with_audit(mut self, audit: Arc<dyn BackupAudit>) -> Self {
        self.audit = Some(audit);
        self
    }

    fn audit(&self, event: &str, detail: &str) {
        if let Some(audit) = &self.audit {
            audit.record(event, detail);
        }
    }
}

/// 恢复选项。
#[derive(Debug, Clone, Default)]
pub struct RestoreOptions {
    /// 搬家恢复的重组根: `Some(root)` = 逐件落到 `<root>/<relpath>`;
    /// `None` = 同机原位恢复 (逐件落到 manifest `origin`)。
    pub to_root: Option<PathBuf>,
}

impl RestoreOptions {
    /// 同机原位恢复。
    pub fn in_place() -> Self {
        Self { to_root: None }
    }

    /// 搬家恢复 (重组到 `root`)。
    pub fn to_root(root: impl Into<PathBuf>) -> Self {
        Self {
            to_root: Some(root.into()),
        }
    }
}

/// 恢复报告 (审计行的「清单数」来源)。
///
/// 命名带 `Backup` 前缀: 同 crate 的脱敏面另有同名 `RestoreReport`, 两面各表其事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRestoreReport {
    /// 校验并落位的清单件数。
    pub entries: u64,
    /// 逐件落位目标 (绝对路径词面, 按落位序)。
    pub placed: Vec<String>,
}

/// 收集阶段的一件候选。
struct CollectedFile {
    category: BackupCategory,
    origin: PathBuf,
    relpath: PathBuf,
}

/// 当前时刻 (epoch 毫秒; 时钟回拨时钳到 0, 不编时间)。
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 文件 mtime (epoch 毫秒; 取不到显式 `None`, 不编时间)。
fn mtime_ms(path: &Path) -> Option<i64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let elapsed = modified.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(elapsed.as_millis()).ok()
}

/// 路径安全闸: 清单里的相对路径只认普通部件 (拒 `..`/绝对/盘符/空路径)。
fn safe_relative(value: &str) -> Result<PathBuf, BackupError> {
    if value.is_empty() {
        return Err(BackupError::ManifestCorrupt("empty relative path".into()));
    }
    // manifest 持久形状是 POSIX 相对路径；反斜杠只可能来自 Windows 绝对/UNC 输入，
    // 在 Linux 上会被当成普通字符，必须显式拒绝（跨平台一致）。
    if value.contains('\\') {
        return Err(BackupError::ManifestCorrupt(format!(
            "unsafe relative path in manifest: {value}"
        )));
    }
    let path = Path::new(value);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            _ => {
                return Err(BackupError::ManifestCorrupt(format!(
                    "unsafe relative path in manifest: {value}"
                )));
            }
        }
    }
    Ok(normalized)
}

/// `/` 分隔的词面 (manifest 持久形状与平台无关)。
fn to_posix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 凭据不随行的排除判定 (显式表 + 敏感路径策略双层)。
///
/// 返回 `Some(原因)` = 该路径不得进包; 原因逐条记进 manifest `excluded`。
fn exclusion_reason(data_root: &Path, path: &Path) -> Option<String> {
    let sensitive_name = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|name| {
            CREDENTIAL_ARTIFACTS_EXCLUDED
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
        });
    if sensitive_name {
        return Some(
            "credential artifact (explicit exclusion list: 凭据不随行, 密钥/令牌面不进包)".into(),
        );
    }
    let parent = path.parent().unwrap_or(data_root);
    if crate::sensitive_path::is_sensitive_path(data_root, path)
        || crate::sensitive_path::is_sensitive_path(parent, path)
    {
        return Some(
            "sensitive path policy (key-shaped material: 凭据不随行, 密钥面不进包)".into(),
        );
    }
    None
}

/// 收集一件单文件源 (不存在 = 跳过; 排除面记账)。
fn collect_file(
    data_root: &Path,
    item: &BackupItem,
    collected: &mut Vec<CollectedFile>,
    excluded: &mut Vec<ExcludedEntry>,
) -> Result<(), BackupError> {
    if !item.origin.is_file() {
        return Ok(());
    }
    if let Some(reason) = exclusion_reason(data_root, &item.origin) {
        excluded.push(ExcludedEntry {
            path: record_path(data_root, &item.origin),
            reason,
        });
        return Ok(());
    }
    collected.push(CollectedFile {
        category: item.category,
        origin: item.origin.clone(),
        relpath: item.relpath.clone(),
    });
    Ok(())
}

/// 收集一件整树源 (递归、按名排序、跳过符号链接并记账)。
fn collect_tree(
    data_root: &Path,
    item: &BackupItem,
    collected: &mut Vec<CollectedFile>,
    excluded: &mut Vec<ExcludedEntry>,
) -> Result<(), BackupError> {
    if !item.origin.is_dir() {
        return Ok(());
    }
    let mut stack = vec![(item.origin.clone(), item.relpath.clone())];
    while let Some((dir, rel_prefix)) = stack.pop() {
        let mut children: Vec<fs::DirEntry> = fs::read_dir(&dir)?.filter_map(Result::ok).collect();
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let path = child.path();
            let rel = rel_prefix.join(child.file_name());
            // 跨线补位 (机械空白, 语义 0 改): clippy `manual_let_else` 门禁断点,
            // 按 clippy 建议的等价 let...else 改写; 真语义归口备份线。
            let Ok(file_type) = child.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                excluded.push(ExcludedEntry {
                    path: record_path(data_root, &path),
                    reason: "symlink (not followed by backup: 不追符号链接)".into(),
                });
                continue;
            }
            if let Some(reason) = exclusion_reason(data_root, &path) {
                excluded.push(ExcludedEntry {
                    path: record_path(data_root, &path),
                    reason,
                });
                continue;
            }
            if file_type.is_dir() {
                stack.push((path, rel));
            } else if file_type.is_file() {
                collected.push(CollectedFile {
                    category: item.category,
                    origin: path,
                    relpath: rel,
                });
            }
        }
    }
    Ok(())
}

/// 记账路径词面 (数据根内用相对路径, 数据根外用原位路径)。
fn record_path(data_root: &Path, path: &Path) -> String {
    match path.strip_prefix(data_root) {
        Ok(relative) => to_posix(relative),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

/// 逐文件 SHA-256 摘要 (hex; 分块读, 大库不进内存)。
fn sha256_file(path: &Path) -> Result<(u64, String), BackupError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
    Ok((total, hex(&hasher.finalize())))
}

/// 摘要字节 → hex 词面。
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 复制并计摘要 (一个读通道同时拿字节与摘要, 拷出件与清单同源)。
fn copy_with_digest(source: &Path, destination: &Path) -> Result<(u64, String), BackupError> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::File::create(destination)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
    output.flush()?;
    Ok((total, hex(&hasher.finalize())))
}

/// 落一份「最近备份」记录 (self_status 备份节的真值来源)。
fn write_ledger(path: &Path, bundle: &Path, manifest: &BackupManifest) -> Result<(), BackupError> {
    let record = BackupLedger {
        exported_at_ms: manifest.created_at_ms,
        manifest_entries: manifest.entry_count(),
        bundle: bundle.to_string_lossy().into_owned(),
        format_version: manifest.format_version,
    };
    let bytes = serde_json::to_vec_pretty(&record)
        .map_err(|error| BackupError::Refused(format!("ledger serialization failed: {error}")))?;
    fs::write(path, bytes)?;
    Ok(())
}

/// 读「最近备份」记录: `Ok(None)` = 尚无备份 (显式 null 语义); 坏档 = `Err(原因)`。
pub fn read_backup_ledger(path: &Path) -> Result<Option<BackupLedger>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| format!("backup ledger read failed: {error}"))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("backup ledger is corrupt: {error}"))
}

/// 自省通道: 备份节探测口 (最近备份时间/清单数真值; 尚无备份显式 null + 原因)。
pub fn backup_status_probe(ledger_path: PathBuf) -> StatusProbe<BackupStatus> {
    Arc::new(move || match read_backup_ledger(&ledger_path) {
        Ok(Some(record)) => Ok(BackupStatus {
            last_backup_at_ms: Some(record.exported_at_ms),
            manifest_entries: Some(record.manifest_entries),
            reason: None,
        }),
        Ok(None) => Ok(BackupStatus {
            last_backup_at_ms: None,
            manifest_entries: None,
            reason: Some("尚无备份记录 (no backup export recorded yet)".into()),
        }),
        Err(reason) => Ok(BackupStatus {
            last_backup_at_ms: None,
            manifest_entries: None,
            reason: Some(reason),
        }),
    })
}

/// 导出全量备份: 收集 → 逐文件快照 + 摘要 → manifest.json → 原子换名就位。
///
/// 目标 bundle **必须不存在** (不覆盖: 已有 bundle 拒绝即帧, 备份永不互踩)。
pub fn export_backup(
    sources: &BackupSources,
    target_dir: &Path,
    hooks: &BackupHooks,
) -> Result<BackupManifest, BackupError> {
    if target_dir.exists() {
        return Err(BackupError::Refused(format!(
            "export target already exists: {} (不覆盖: 已有 bundle/目录拒绝即帧)",
            target_dir.display()
        )));
    }
    let parent = target_dir.parent().unwrap_or_else(|| Path::new("."));
    let target_name = target_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bundle".to_string());
    let staging = parent.join(format!(
        ".{target_name}{EXPORT_STAGING_SUFFIX}-{}",
        std::process::id()
    ));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    let result = export_into_staging(sources, &staging);
    let manifest = match result {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    if let Err(error) = fs::rename(&staging, target_dir) {
        let _ = fs::remove_dir_all(&staging);
        return Err(BackupError::Io(error));
    }
    if let Some(ledger_path) = &hooks.ledger_path {
        write_ledger(ledger_path, target_dir, &manifest)?;
    }
    hooks.audit(
        AUDIT_EVENT_EXPORT,
        &format!(
            "entries={} excluded={} bundle={}",
            manifest.entry_count(),
            manifest.excluded.len(),
            target_dir.display()
        ),
    );
    Ok(manifest)
}

/// 暂存区内完成快照与清单 (就位前的全量构建; 任一步失败 = 零残留)。
fn export_into_staging(
    sources: &BackupSources,
    staging: &Path,
) -> Result<BackupManifest, BackupError> {
    fs::create_dir_all(staging)?;
    let mut collected: Vec<CollectedFile> = Vec::new();
    let mut excluded: Vec<ExcludedEntry> = Vec::new();
    for item in &sources.items {
        if item.tree {
            collect_tree(&sources.data_root, item, &mut collected, &mut excluded)?;
        } else {
            collect_file(&sources.data_root, item, &mut collected, &mut excluded)?;
        }
    }
    collected.sort_by(|left, right| {
        (left.category.as_str(), to_posix(&left.relpath))
            .cmp(&(right.category.as_str(), to_posix(&right.relpath)))
    });

    let mut files: Vec<BackupFileEntry> = Vec::new();
    for file in &collected {
        let bundle_relative = PathBuf::from(BUNDLE_FILES_DIR)
            .join(file.category.as_str())
            .join(&file.relpath);
        let bundle_path = to_posix(&bundle_relative);
        let destination = staging.join(&bundle_relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let (bytes, digest) = copy_with_digest(&file.origin, &destination)?;
        files.push(BackupFileEntry {
            category: file.category.as_str().to_string(),
            path: bundle_path,
            origin: file.origin.to_string_lossy().into_owned(),
            relpath: to_posix(&file.relpath),
            bytes,
            sha256: digest,
            mtime_ms: mtime_ms(&file.origin),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    excluded.sort_by(|left, right| left.path.cmp(&right.path));

    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        product_version: RELEASE_VERSION.to_string(),
        created_at_ms: now_ms(),
        data_root: sources.data_root.to_string_lossy().into_owned(),
        files,
        excluded,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| BackupError::Refused(format!("manifest serialization failed: {error}")))?;
    fs::write(staging.join(MANIFEST_FILE_NAME), manifest_bytes)?;
    Ok(manifest)
}

/// 载入并结构校验清单 (解析 + 版本 + 路径安全 + 唯一性)。
fn load_manifest(bundle_dir: &Path) -> Result<BackupManifest, BackupError> {
    let manifest_path = bundle_dir.join(MANIFEST_FILE_NAME);
    // **先区分「包不存在」与「清单损坏」**：此前二者共用 `ManifestCorrupt`，
    // 路径打错时用户会看到「备份清单损坏」——以为自己的备份坏了，其实是没这个包
    // （真机实测 2026-10-25）。「没有包」是可自救的输入问题，必须说清。
    if !bundle_dir.exists() {
        return Err(BackupError::NotABundle(format!(
            "备份包不存在: {} （路径打错？先用 `apeireth backup export <DIR>` 导出）",
            bundle_dir.display()
        )));
    }
    let bytes = fs::read(&manifest_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            // **清单不见了 ≠ 这目录不是备份包** —— 要看目录里还有没有内容：
            //   * 空目录 ⇒ 确实不是包（路径打错/还没导出）⇒ `NotABundle`（可自救的输入问题）；
            //   * 目录里还有数据文件、只是清单没了 ⇒ **包被破坏/清单被删**
            //     ⇒ `ManifestCorrupt`（宁拒不损：不能把「缺清单的目录」当成合法备份包恢复）。
            // （此前一律判 `NotABundle` ⇒ 「清单被删」被说成「这不是包」，
            //   既有测试 `corrupt_manifest_is_refused_and_nothing_touches_the_data_plane` 因此红。）
            let has_payload = fs::read_dir(bundle_dir)
                .map(|mut entries| {
                    entries.any(|entry| {
                        entry
                            .map(|entry| entry.file_name() != MANIFEST_FILE_NAME)
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if has_payload {
                return BackupError::ManifestCorrupt(format!(
                    "备份包里没有清单 {} —— 清单被删或包已损坏，拒绝恢复（宁拒不损）",
                    MANIFEST_FILE_NAME
                ));
            }
            return BackupError::NotABundle(format!(
                "目录 {} 里没有清单 {} —— 这不是一个备份包；用 `apeireth backup export <DIR>` 导出",
                bundle_dir.display(),
                MANIFEST_FILE_NAME
            ));
        }
        BackupError::ManifestCorrupt(format!("{} unreadable: {error}", manifest_path.display()))
    })?;
    let manifest: BackupManifest = serde_json::from_slice(&bytes)
        .map_err(|error| BackupError::ManifestCorrupt(format!("manifest parse failed: {error}")))?;
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(BackupError::VersionMismatch {
            found: manifest.format_version,
            expected: BACKUP_FORMAT_VERSION,
        });
    }
    let mut seen_bundle: BTreeMap<String, ()> = BTreeMap::new();
    let mut seen_origin: BTreeMap<String, ()> = BTreeMap::new();
    for entry in &manifest.files {
        safe_relative(&entry.path)?;
        safe_relative(&entry.relpath)?;
        if entry.path.is_empty() || entry.relpath.is_empty() {
            return Err(BackupError::ManifestCorrupt(
                "manifest entry carries an empty path".into(),
            ));
        }
        if seen_bundle.insert(entry.path.clone(), ()).is_some() {
            return Err(BackupError::ManifestCorrupt(format!(
                "duplicate bundle path in manifest: {}",
                entry.path
            )));
        }
        if seen_origin.insert(entry.origin.clone(), ()).is_some() {
            return Err(BackupError::ManifestCorrupt(format!(
                "duplicate origin path in manifest: {}",
                entry.origin
            )));
        }
    }
    Ok(manifest)
}

/// 只验不恢复: 清单可解析 + 版本匹配 + 逐文件大小/摘要全对 (任一不符拒)。
///
/// 校验**不触碰数据面**: 数据根的任何文件不读不写 (独立校验语义)。
pub fn verify_backup(bundle_dir: &Path) -> Result<BackupManifest, BackupError> {
    let manifest = load_manifest(bundle_dir)?;
    for entry in &manifest.files {
        let bundle_relative = safe_relative(&entry.path)?;
        let file_path = bundle_dir.join(&bundle_relative);
        let (bytes, digest) =
            sha256_file(&file_path).map_err(|error| BackupError::DigestMismatch {
                path: entry.path.clone(),
                reason: format!("bundle file unreadable: {error}"),
            })?;
        if bytes != entry.bytes {
            return Err(BackupError::DigestMismatch {
                path: entry.path.clone(),
                reason: format!("size mismatch: manifest={} bundle={bytes}", entry.bytes),
            });
        }
        if digest != entry.sha256 {
            return Err(BackupError::DigestMismatch {
                path: entry.path.clone(),
                reason: format!("sha256 mismatch: manifest={} bundle={digest}", entry.sha256),
            });
        }
    }
    Ok(manifest)
}

/// 恢复: 全量校验 (宁拒不损) → 逐件「先写临时再换名」原子落位 → 中途失败回滚原状。
///
/// 落位范围 = manifest 成员集; 备份后新增的非成员文件**不丢不覆盖** (不动)。
pub fn restore_backup(
    bundle_dir: &Path,
    options: &RestoreOptions,
    hooks: &BackupHooks,
) -> Result<BackupRestoreReport, BackupError> {
    // 阶段 0: 全量校验 —— 摘要不符拒恢复, 一个字节都不落盘。
    let manifest = verify_backup(bundle_dir)?;

    // 落位目标解析 + 唯一性闸 (同目标双写 = 拒)。
    let mut targets: Vec<(BackupFileEntry, PathBuf)> = Vec::new();
    let mut seen_targets: BTreeMap<String, ()> = BTreeMap::new();
    for entry in &manifest.files {
        let target = match &options.to_root {
            Some(root) => root.join(safe_relative(&entry.relpath)?),
            None => PathBuf::from(&entry.origin),
        };
        let key = target.to_string_lossy().into_owned();
        if seen_targets.insert(key.clone(), ()).is_some() {
            return Err(BackupError::Refused(format!(
                "duplicate restore target: {key} (同目标双写拒绝即帧)"
            )));
        }
        targets.push((entry.clone(), target));
    }

    // 阶段 1: 逐件写临时件 (任一失败 = 零目标被碰, 原状)。
    let pid = std::process::id();
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (entry, target) in &targets {
        let bundle_relative = safe_relative(&entry.path)?;
        let source = bundle_dir.join(bundle_relative);
        let tmp = tmp_path_for(target, pid);
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        if let Err(error) = fs::copy(&source, &tmp) {
            cleanup_paths(staged.iter().map(|(tmp, _)| tmp.clone()));
            return Err(BackupError::Io(error));
        }
        staged.push((tmp, target.clone()));
    }

    // 阶段 2: 逐件换名落位 (原件先换到 .bak 保全; 任一失败 = 回滚已落位件)。
    let mut placed: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    for (index, (_entry, target)) in targets.iter().enumerate() {
        let (tmp, _) = &staged[index];
        let existing = fs::symlink_metadata(target).ok();
        if existing.as_ref().is_some_and(|meta| !meta.is_file()) {
            cleanup_paths(staged.iter().map(|(tmp, _)| tmp.clone()));
            rollback(&placed);
            return Err(BackupError::Refused(format!(
                "restore target exists but is not a regular file: {} (拒换非文件占位; 已回滚原状)",
                target.display()
            )));
        }
        let backup = if existing.is_some() {
            let backup = bak_path_for(target, pid);
            if let Err(error) = fs::rename(target, &backup) {
                cleanup_paths(staged.iter().map(|(tmp, _)| tmp.clone()));
                rollback(&placed);
                return Err(BackupError::Io(error));
            }
            Some(backup)
        } else {
            None
        };
        if let Err(error) = fs::rename(tmp, target) {
            if let Some(backup) = &backup {
                let _ = fs::rename(backup, target);
            }
            cleanup_paths(staged.iter().map(|(tmp, _)| tmp.clone()));
            rollback(&placed);
            return Err(BackupError::Io(error));
        }
        placed.push((target.clone(), backup));
    }

    // 阶段 3: 成功收尾 (保全件清理; 残留临时件一并清)。
    for (_, backup) in &placed {
        if let Some(backup) = backup {
            let _ = fs::remove_file(backup);
        }
    }
    let report = BackupRestoreReport {
        entries: manifest.entry_count(),
        placed: placed
            .iter()
            .map(|(target, _)| target.to_string_lossy().into_owned())
            .collect(),
    };
    hooks.audit(
        AUDIT_EVENT_RESTORE,
        &format!("entries={} bundle={}", report.entries, bundle_dir.display()),
    );
    Ok(report)
}

/// 临时件路径 (与目标同目录, 先写临时再换名的「临时」侧)。
fn tmp_path_for(target: &Path, pid: u32) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.push_str(&format!("{RESTORE_TMP_SUFFIX}-{pid}"));
    target.with_file_name(name)
}

/// 原件保全件路径 (换名期间暂存原文件)。
fn bak_path_for(target: &Path, pid: u32) -> PathBuf {
    let mut name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.push_str(&format!("{RESTORE_BAK_SUFFIX}-{pid}"));
    target.with_file_name(name)
}

/// 清理一批路径 (尽力而为, 失败不放大)。
fn cleanup_paths(paths: impl IntoIterator<Item = PathBuf>) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

/// 中途失败回滚: 已落位件逐件还原 (新件删除 = 回到「原本不存在」原状)。
fn rollback(placed: &[(PathBuf, Option<PathBuf>)]) {
    for (target, backup) in placed.iter().rev() {
        let _ = fs::remove_file(target);
        if let Some(backup) = backup {
            let _ = fs::rename(backup, target);
        }
    }
}

/// 备份导出工具的注入配置 (数据源 + 引擎钩子; 组装根一份配置两个出口共用)。
#[derive(Clone)]
pub struct BackupExportConfig {
    /// 数据源清单 (实际生效路径)。
    pub sources: Arc<BackupSources>,
    /// 引擎钩子 (审计行 + 最近备份记录)。
    pub hooks: BackupHooks,
}

impl std::fmt::Debug for BackupExportConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupExportConfig")
            .field("sources", &self.sources)
            .field("hooks", &self.hooks)
            .finish()
    }
}

impl BackupExportConfig {
    /// 一份工具配置。
    pub fn new(sources: BackupSources, hooks: BackupHooks) -> Self {
        Self {
            sources: Arc::new(sources),
            hooks,
        }
    }
}

/// 空参数校验: 导出工具只收 `target_dir` (未知字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupExportParams {
    target_dir: String,
}

/// 备份导出工具 (`tool.backup_export`, 声明名 `backup_export`)。
///
/// 模型可把自己的一切记忆/账本打成带清单的全量备份 (AI 先用的第一出口);
/// 恢复/校验是 CLI 面 (`apeireth backup restore|verify`, 本地操作)。
/// **凭据不随行**: 打包内永远零密钥/令牌件 (显式排除 + 测试钉死)。
pub struct BackupExportTool {
    id: CapabilityId,
    config: BackupExportConfig,
}

impl BackupExportTool {
    /// 绑定数据源与钩子。
    pub fn new(config: BackupExportConfig) -> Self {
        Self {
            id: CapabilityId::new(BACKUP_EXPORT_CAPABILITY_ID).unwrap(),
            config,
        }
    }

    /// 输出归一合同: `bundle` (字符串) + `manifest` (对象: 版本/时间/件数/排除数)。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("bundle", SchemaKind::String)
            .require("manifest", SchemaKind::Object)
    }

    /// 采集并投影一次导出的结果 (结构化值)。
    fn render_result(bundle: &Path, manifest: &BackupManifest) -> Value {
        json!({
            "bundle": bundle.to_string_lossy(),
            "manifest": {
                "format_version": manifest.format_version,
                "product_version": manifest.product_version,
                "created_at_ms": manifest.created_at_ms,
                "entries": manifest.entry_count(),
                "excluded": manifest.excluded.len() as u64,
                "categories": manifest.category_counts(),
            }
        })
    }
}

#[async_trait]
impl ToolCapability for BackupExportTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "target_dir": {
                        "type": "string",
                        "description": "Directory to create as the new backup bundle. It must not already exist (existing bundles are never overwritten)."
                    }
                },
                "required": ["target_dir"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new("backup_export")
            .with_description(
                "Export a full backup bundle of the runtime's durable memory and ledgers (session, \
                 cognitive and memory stores, skill cards, care rules, user table, domain \
                 documents) with a manifest.json carrying per-file SHA-256 digests, format \
                 version and timestamps so corruption is detectable. Credentials never travel \
                 with a backup: key/token artifacts are explicitly excluded and listed in the \
                 manifest. The target directory must not already exist.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<BackupExportParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid backup_export parameters: {error}"),
                );
            }
        };
        if params.target_dir.trim().is_empty() {
            return ToolResult::permanent_error(
                &call.id,
                "invalid backup_export parameters: target_dir must not be empty".to_string(),
            );
        }
        // 导出跑在阻塞池上: 大库拷贝卡住时执行链的到期计时器仍能触发
        // (超时归既有 `timeout.*` code 族), 不把回合一起挂死。
        let config = self.config.clone();
        let target = PathBuf::from(params.target_dir);
        match tokio::task::spawn_blocking(move || {
            let manifest = export_backup(&config.sources, &target, &config.hooks)
                .map_err(|error| error.to_string())?;
            Ok::<(PathBuf, BackupManifest), String>((target, manifest))
        })
        .await
        {
            Ok(Ok((target, manifest))) => {
                ToolResult::ok(&call.id, Self::render_result(&target, &manifest))
            }
            Ok(Err(error)) => ToolResult::permanent_error(&call.id, error),
            Err(error) => {
                ToolResult::permanent_error(&call.id, format!("backup_export task failed: {error}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_canonical_vector() {
        // 官方测试向量钉死摘要实现 (空串与 "abc"), 防手写摘要漂移。
        // 直接用「摘要 + hex」这两个真实组成件（`sha256_bytes` 薄封装已删 ——
        // 它在 lib 侧零调用，只有本测试用 ⇒ 留着就是 D-1 要抓的死代码）。
        assert_eq!(
            hex(&Sha256::digest(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifest_relative_paths_reject_traversal_and_absolute_shapes() {
        assert!(safe_relative("files/skills/a.json").is_ok());
        for hostile in [
            "../escape",
            "files/../../escape",
            "/abs/path",
            "C:\\windows\\system32",
            "",
        ] {
            assert!(
                safe_relative(hostile).is_err(),
                "hostile manifest path must be refused: {hostile}"
            );
        }
    }

    #[test]
    fn credential_artifact_names_are_excluded_explicitly() {
        let root = Path::new("/data");
        for name in [
            "creds.json",
            "apeireth-keyring.bin",
            "apeireth-keyring.master.key",
            "keyring",
            "client-tokens.json",
            "im-channels.json",
            "CREDS.JSON",
        ] {
            let path = root.join(name);
            assert!(
                exclusion_reason(root, &path).is_some(),
                "credential artifact must be excluded: {name}"
            );
        }
        // 正常数据面不误伤。
        for name in [
            "sessions.sqlite3",
            "cognitive.sqlite3",
            "users.json",
            "skills",
        ] {
            let path = root.join(name);
            assert!(
                exclusion_reason(root, &path).is_none(),
                "ordinary data file must travel: {name}"
            );
        }
    }
}
