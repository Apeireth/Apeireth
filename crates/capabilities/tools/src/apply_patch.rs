//! 事务级多文件打补丁工具 (`apply_patch`).
//!
//! 支持在单次原子事务中对代码库执行多文件新增 (Add)、删除 (Delete) 与按上下文精确替换更新 (Update).
//! 若任意一个文件的任意一个 Hunk 匹配失败，整个事务立即完全回滚，保证磁盘状态绝对一致.
//!
//! 模块语义:
//! - Update hunks require a **strict unique** match (0 → [`ApplyPatchError::ContextMismatch`],
//!   >1 → [`ApplyPatchError::AmbiguousMatch`]). Silent first-match replacement is rejected.
//! - 行式 hunks (`@@` 注释锚, `-old` / `+new`) 与 SEARCH/REPLACE 格式并存, 自动识别.
//! - 提交写盘走统一存储基础件的持久档原子写 (同目录独占临时文件 + 落盘 + 替换,
//!   见 `apeireth_core::storage_atomic`): 崩溃安全替换, 半途中断不留截断目标.
//! - 提交写盘叠加读前观测门禁 (见 `crate::observed_gate`): 预演阶段读到的
//!   原始内容即观测事件 (记 (len, mtime) 版本), 提交写携带该版本过版本 CAS
//!   双钥匙 —— 预演与提交之间的外部改动被拒, Add 走 createIfAbsent 钥匙。
//!   门禁叠加在既有版本语义 (严格唯一上下文匹配 + 原内容备份) 之上, 不改写它们。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

use apeireth_core::storage_atomic;

use crate::observed_gate::{
    gated_write_atomic_durable, FileVersion, FsVersionProbe, GatedWriteError, ObservedGate,
    VersionProbe, WriteIntent, WriteKind, WriteRequest,
};

/// Patch 应用错误.
#[derive(Debug, Error, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ApplyPatchError {
    #[error("格式解析错误: {0}")]
    ParseError(String),
    #[error("文件未找到: {0}")]
    FileNotFound(String),
    #[error("文件已存在: {0}")]
    FileAlreadyExists(String),
    #[error("上下文未匹配或产生歧义: {0}")]
    ContextMismatch(String),
    /// Strict-unique match failed because the search context occurred more than once.
    #[error("上下文匹配产生歧义: {path} ({occurrences} 处)")]
    AmbiguousMatch { path: String, occurrences: usize },
    #[error("磁盘 IO 失败: {0}")]
    Io(String),
    /// H4 (2026-09-24 审计): 补丁目标路径越出根目录包含边界 (绝对路径 / `..`
    /// 组件 / symlink 逃逸) —— fail-closed, 整个事务在预演阶段即拒绝。
    #[error("补丁路径越出工作区边界: {0}")]
    PathViolation(String),
    /// 读前观测门禁拒绝 (未读先覆盖 / 版本 CAS 钥匙不符 / createIfAbsent 撞目标)。
    #[error("读前观测门禁拒绝: {0}")]
    GateDenied(String),
}

/// 单个文件操作类型.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePatchAction {
    Add {
        path: PathBuf,
        content: String,
    },
    Delete {
        path: PathBuf,
    },
    Update {
        path: PathBuf,
        hunks: Vec<PatchHunk>,
    },
}

/// 补丁修改块 (Hunk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchHunk {
    pub search_context: String,
    pub replacement_content: String,
}

/// 事务应用执行报告.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchReport {
    pub files_added: Vec<String>,
    pub files_updated: Vec<String>,
    pub files_deleted: Vec<String>,
    pub total_actions: usize,
}

/// 提交阶段的暂存写入: 内容 + 门禁意图 + 预演观测版本。
#[derive(Debug, Clone)]
struct StagedWrite {
    content: String,
    intent: WriteKind,
    observed: Option<FileVersion>,
}

/// 事务补丁应用器.
#[derive(Debug, Clone, Default)]
pub struct TransactionalPatchApplier;

impl TransactionalPatchApplier {
    pub fn new() -> Self {
        Self
    }

    /// 解析补丁文本指令.
    ///
    /// 支持如下补丁文本格式:
    /// ```text
    /// *** Begin Patch
    /// *** Add File: src/new_file.rs
    /// pub fn new_fn() {}
    /// *** Update File: src/main.rs
    /// <<<<<<< SEARCH
    /// fn old() {}
    /// =======
    /// fn updated() {}
    /// >>>>>>>
    /// *** Delete File: src/legacy.rs
    /// *** End Patch
    /// ```
    ///
    /// 行式 Update hunks 同样接受:
    /// ```text
    /// *** Update File: src/main.rs
    /// @@ optional_anchor
    /// -old line
    /// +new line
    /// ```
    pub fn parse_patch(patch_text: &str) -> Result<Vec<FilePatchAction>, ApplyPatchError> {
        let trimmed = patch_text.trim();
        if !trimmed.starts_with("*** Begin Patch") || !trimmed.ends_with("*** End Patch") {
            return Err(ApplyPatchError::ParseError(
                "补丁必须以 '*** Begin Patch' 开头并以 '*** End Patch' 结尾".to_string(),
            ));
        }

        let inner = &trimmed["*** Begin Patch".len()..trimmed.len() - "*** End Patch".len()];
        let mut actions = Vec::new();
        let lines: Vec<&str> = inner.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            if let Some(stripped) = line.strip_prefix("*** Add File:") {
                let file_path = PathBuf::from(stripped.trim());
                i += 1;
                let mut content_lines = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("***") {
                    content_lines.push(lines[i]);
                    i += 1;
                }
                actions.push(FilePatchAction::Add {
                    path: file_path,
                    content: decode_add_file_content(&content_lines),
                });
            } else if let Some(stripped) = line.strip_prefix("*** Delete File:") {
                let file_path = PathBuf::from(stripped.trim());
                i += 1;
                actions.push(FilePatchAction::Delete { path: file_path });
            } else if let Some(stripped) = line.strip_prefix("*** Update File:") {
                let file_path = PathBuf::from(stripped.trim());
                i += 1;
                let mut hunks = Vec::new();

                while i < lines.len() && !lines[i].trim().starts_with("***") {
                    let trimmed_line = lines[i].trim();
                    if trimmed_line == "<<<<<<< SEARCH" {
                        i += 1;
                        let mut search_lines = Vec::new();
                        while i < lines.len() && lines[i].trim() != "=======" {
                            search_lines.push(lines[i]);
                            i += 1;
                        }
                        if i >= lines.len() || lines[i].trim() != "=======" {
                            return Err(ApplyPatchError::ParseError(
                                "缺少 '=======' 分隔符".to_string(),
                            ));
                        }
                        i += 1; // 跳过 =======
                        let mut replace_lines = Vec::new();
                        while i < lines.len() && lines[i].trim() != ">>>>>>>" {
                            replace_lines.push(lines[i]);
                            i += 1;
                        }
                        if i >= lines.len() || lines[i].trim() != ">>>>>>>" {
                            return Err(ApplyPatchError::ParseError(
                                "缺少 '>>>>>>>' 结束符".to_string(),
                            ));
                        }
                        i += 1; // 跳过 >>>>>>>
                        hunks.push(PatchHunk {
                            search_context: search_lines.join("\n"),
                            replacement_content: replace_lines.join("\n"),
                        });
                    } else if is_line_hunk_start(trimmed_line) {
                        // `@@` is an optional human-readable anchor (comment only).
                        if trimmed_line.starts_with("@@") {
                            i += 1;
                        }
                        let mut old_lines = Vec::new();
                        let mut new_lines = Vec::new();
                        while i < lines.len() {
                            let bt = lines[i].trim_start();
                            if bt.starts_with("***")
                                || bt.starts_with("@@")
                                || bt == "<<<<<<< SEARCH"
                            {
                                break;
                            }
                            if let Some(rest) = bt.strip_prefix('-') {
                                old_lines.push(rest.to_string());
                                i += 1;
                            } else if let Some(rest) = bt.strip_prefix('+') {
                                new_lines.push(rest.to_string());
                                i += 1;
                            } else {
                                // Context line (leading space) or blank — comment only.
                                i += 1;
                            }
                        }
                        if old_lines.is_empty() && new_lines.is_empty() {
                            continue;
                        }
                        if old_lines.is_empty() {
                            return Err(ApplyPatchError::ParseError(
                                "行式 hunk 缺少旧内容行 (- 前缀)".to_string(),
                            ));
                        }
                        hunks.push(PatchHunk {
                            search_context: old_lines.join("\n"),
                            replacement_content: new_lines.join("\n"),
                        });
                    } else {
                        i += 1;
                    }
                }
                actions.push(FilePatchAction::Update {
                    path: file_path,
                    hunks,
                });
            } else {
                i += 1;
            }
        }

        Ok(actions)
    }

    /// 在指定根目录下原子性应用整个补丁事务.
    ///
    /// 使用调用内建的观测门禁 (补丁自读自写自洽)。需要与文件读工具共享
    /// 会话观测时用 [`Self::apply_with_gate`]。
    pub fn apply(root_dir: &Path, patch_text: &str) -> Result<PatchReport, ApplyPatchError> {
        Self::apply_with_gate(root_dir, patch_text, &ObservedGate::new())
    }

    /// 同 [`Self::apply`], 但提交写盘过共享的读前观测门禁.
    ///
    /// 叠加语义: 预演读取把 (len, mtime) 版本记入 `gate` (观测事件), 提交写
    /// 携带该版本走版本 CAS 双钥匙 (Update/Delete 钥匙 = 预演版本, Add 钥匙 =
    /// createIfAbsent); 事务收尾把写后现状回填为最新观测, 同会话后续写免重读。
    pub fn apply_with_gate(
        root_dir: &Path,
        patch_text: &str,
        gate: &ObservedGate,
    ) -> Result<PatchReport, ApplyPatchError> {
        let actions = Self::parse_patch(patch_text)?;
        if actions.is_empty() {
            return Ok(PatchReport {
                files_added: vec![],
                files_updated: vec![],
                files_deleted: vec![],
                total_actions: 0,
            });
        }

        // 1. 预演阶段 (Dry-run): 在内存中计算并校验所有更改
        let mut staged_writes: HashMap<PathBuf, StagedWrite> = HashMap::new();
        let mut staged_deletes: Vec<(PathBuf, FileVersion)> = Vec::new();
        let mut original_backups: HashMap<PathBuf, Option<String>> = HashMap::new();

        let mut files_added = Vec::new();
        let mut files_updated = Vec::new();
        let mut files_deleted = Vec::new();

        for action in &actions {
            match action {
                FilePatchAction::Add { path, content } => {
                    // H4: 入口逐 action 校验路径包含 (绝对路径 / `..` / symlink
                    // 越界在此拒绝, 磁盘写入从不可达边界之外)。
                    let full_path = Self::resolve_contained_target(root_dir, path)?;
                    if full_path.exists() {
                        return Err(ApplyPatchError::FileAlreadyExists(
                            path.to_string_lossy().to_string(),
                        ));
                    }
                    // 预演已核对目标缺席 = 「已观测为不存在」观测事件。
                    gate.observe_missing(&full_path);
                    original_backups.insert(full_path.clone(), None);
                    staged_writes.insert(
                        full_path,
                        StagedWrite {
                            content: content.clone(),
                            intent: WriteKind::Create,
                            observed: None,
                        },
                    );
                    files_added.push(path.to_string_lossy().to_string());
                }
                FilePatchAction::Delete { path } => {
                    let full_path = Self::resolve_contained_target(root_dir, path)?;
                    if !full_path.exists() {
                        return Err(ApplyPatchError::FileNotFound(
                            path.to_string_lossy().to_string(),
                        ));
                    }
                    let (old_content, observed) = read_observed(&full_path, gate)?;
                    original_backups.insert(full_path.clone(), Some(old_content));
                    staged_deletes.push((full_path, observed));
                    files_deleted.push(path.to_string_lossy().to_string());
                }
                FilePatchAction::Update { path, hunks } => {
                    let full_path = Self::resolve_contained_target(root_dir, path)?;
                    if !full_path.exists() {
                        return Err(ApplyPatchError::FileNotFound(
                            path.to_string_lossy().to_string(),
                        ));
                    }
                    let (original, observed) = read_observed(&full_path, gate)?;
                    original_backups.insert(full_path.clone(), Some(original.clone()));

                    let mut updated = original;
                    for (h_idx, hunk) in hunks.iter().enumerate() {
                        updated = apply_unique_replace(
                            &updated,
                            &hunk.search_context,
                            &hunk.replacement_content,
                            path,
                            h_idx,
                        )?;
                    }
                    staged_writes.insert(
                        full_path,
                        StagedWrite {
                            content: updated,
                            intent: WriteKind::Replace,
                            observed: Some(observed),
                        },
                    );
                    files_updated.push(path.to_string_lossy().to_string());
                }
            }
        }

        // 2. 提交阶段 (Commit): 过门禁的原子写入磁盘；若有任何 IO 异常则自动回滚已写入的文件
        let mut committed_paths = Vec::new();
        for (path, staged) in staged_writes {
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    Self::rollback(&committed_paths, &original_backups);
                    return Err(ApplyPatchError::Io(e.to_string()));
                }
            }
            let request = WriteRequest {
                path: path.clone(),
                kind: staged.intent,
                replace_if_version: staged.observed,
                create_if_absent: staged.intent == WriteKind::Create,
            };
            if let Err(e) = gated_write_atomic_durable(
                gate,
                &FsVersionProbe,
                &request,
                staged.content.as_bytes(),
                target_file_mode(&path),
            ) {
                Self::rollback(&committed_paths, &original_backups);
                return Err(match e {
                    GatedWriteError::Denied(denial) => {
                        ApplyPatchError::GateDenied(denial.message())
                    }
                    GatedWriteError::Io(detail) => ApplyPatchError::Io(detail),
                });
            }
            committed_paths.push(path);
        }

        for (path, observed) in staged_deletes {
            // 删除同属覆盖类: 过同一道门禁 (钥匙 = 预演观测版本)。
            let intent = WriteIntent {
                path: &path,
                kind: WriteKind::Replace,
                replace_if_version: Some(observed),
                create_if_absent: false,
                current: FsVersionProbe.version(&path),
            };
            if let Err(denial) = gate.check_write(&intent) {
                Self::rollback(&committed_paths, &original_backups);
                return Err(ApplyPatchError::GateDenied(denial.message()));
            }
            if let Err(e) = fs::remove_file(&path) {
                Self::rollback(&committed_paths, &original_backups);
                return Err(ApplyPatchError::Io(e.to_string()));
            }
            gate.observe_missing(&path);
            committed_paths.push(path);
        }

        Ok(PatchReport {
            total_actions: files_added.len() + files_updated.len() + files_deleted.len(),
            files_added,
            files_updated,
            files_deleted,
        })
    }

    fn rollback(committed: &[PathBuf], backups: &HashMap<PathBuf, Option<String>>) {
        // 原子性边界 (审计 I7/L 组): 回滚是 best-effort —— 已 rename 到位的目标
        // 用备份内容原子重写, 已删除的目标靠原内容重建; 但回滚自身若遇 IO
        // 失败 (如备份写盘时磁盘满), 磁盘可能停留在部分回滚状态。提交阶段
        // 的顺序 (先写后删) 与每文件的 tmp+rename 原子性保证的是"进行中的
        // 损坏不会发生", 而非"回滚必然成功"。调用方需要把 report/err 当权威。
        //
        // 回滚恢复的是本次事务刚观测过的原内容, 属内部恢复, 不再过读前观测
        // 门禁; 回滚后门禁观测可能短暂停在旧版本 —— 偏向"下次写被拒、要求
        // 重读"的安全方向。
        for path in committed {
            if let Some(backup) = backups.get(path) {
                match backup {
                    Some(old_content) => {
                        let _ = atomic_write_file(path, old_content);
                    }
                    None => {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    }

    /// H4 (2026-09-24 审计): 把补丁内的相对路径解析为 root 之下、且经
    /// symlink 现实校验的绝对路径。`root_dir.join(path)` 的裸语义允许绝对
    /// 路径替换 base (Windows `C:\...` / Unix `/etc/...`) 与 `..` 逃逸,
    /// 模型或被 prompt 注入诱导的补丁文本即可任意写/删 —— 在此逐 action
    /// 拒绝:
    ///
    /// 1. 词法层: 拒绝绝对路径、盘符/UNC 前缀与任何 `..` 组件;
    /// 2. 现实层: root 与目标分别 canonicalize (目标不存在时对最近存在
    ///    祖先 canonicalize 后 join 余量), 要求 `starts_with(root)` ——
    ///    root 内指向外部的 symlink 在此现形。
    fn resolve_contained_target(root_dir: &Path, path: &Path) -> Result<PathBuf, ApplyPatchError> {
        if path.as_os_str().is_empty() {
            return Err(ApplyPatchError::PathViolation("补丁路径为空".to_string()));
        }
        if path.is_absolute() {
            return Err(ApplyPatchError::PathViolation(format!(
                "禁止绝对路径 (必须是工作区相对路径): {}",
                path.display()
            )));
        }
        for component in path.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                _ => {
                    return Err(ApplyPatchError::PathViolation(format!(
                        "禁止的路径组件 ({component:?}): {}",
                        path.display()
                    )));
                }
            }
        }

        let canonical_root = fs::canonicalize(root_dir).map_err(|e| {
            ApplyPatchError::Io(format!(
                "workspace root {} is not accessible: {e}",
                root_dir.display()
            ))
        })?;

        let joined = canonical_root.join(path);

        // 目标可能尚不存在 (Add / 新建子目录): 向上找最近的存在祖先,
        // canonicalize 它 (展开沿途 symlink), 再把余量拼回去。
        let mut probe = joined.as_path();
        let mut remainder: Vec<&OsStr> = Vec::new();
        loop {
            match fs::canonicalize(probe) {
                Ok(canonical) => {
                    if !canonical.starts_with(&canonical_root) {
                        return Err(ApplyPatchError::PathViolation(format!(
                            "{} 解析到工作区之外 (symlink 越界或穿越)",
                            joined.display()
                        )));
                    }
                    let mut resolved = canonical;
                    for name in remainder.iter().rev() {
                        resolved.push(name);
                    }
                    return Ok(resolved);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    match (probe.file_name(), probe.parent()) {
                        (Some(name), Some(parent)) => {
                            remainder.push(name);
                            probe = parent;
                        }
                        _ => {
                            return Err(ApplyPatchError::PathViolation(format!(
                                "{} 无法在工作区内解析",
                                joined.display()
                            )));
                        }
                    }
                }
                Err(e) => {
                    return Err(ApplyPatchError::Io(format!(
                        "{} 解析失败: {e}",
                        joined.display()
                    )));
                }
            }
        }
    }
}

/// True when `line` (already trimmed) starts a line-style hunk.
fn is_line_hunk_start(trimmed_line: &str) -> bool {
    trimmed_line.starts_with("@@") || trimmed_line.starts_with('-') || trimmed_line.starts_with('+')
}

/// Add File body: 行式格式要求每行 `+` 前缀; SEARCH/REPLACE 格式则是原始文件内容.
/// 自动识别: 若所有行都以 `+` 开头, 剥掉此前缀.
fn decode_add_file_content(content_lines: &[&str]) -> String {
    if !content_lines.is_empty()
        && content_lines
            .iter()
            .all(|line| line.trim_start().starts_with('+'))
    {
        content_lines
            .iter()
            .map(|line| {
                line.trim_start()
                    .strip_prefix('+')
                    .unwrap_or(line)
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        content_lines.join("\n")
    }
}

/// Apply one hunk with strict-unique semantics: 0 matches → ContextMismatch,
/// >1 matches → AmbiguousMatch. Never silently replaces the first hit.
fn apply_unique_replace(
    haystack: &str,
    needle: &str,
    replacement: &str,
    path: &Path,
    hunk_idx: usize,
) -> Result<String, ApplyPatchError> {
    if needle.is_empty() {
        return Err(ApplyPatchError::ContextMismatch(format!(
            "文件 {} Hunk #{} 搜索上下文为空",
            path.to_string_lossy(),
            hunk_idx + 1
        )));
    }
    let occurrences = haystack.matches(needle).count();
    if occurrences == 0 {
        return Err(ApplyPatchError::ContextMismatch(format!(
            "文件 {} Hunk #{} 搜索上下文未匹配: [{}]",
            path.to_string_lossy(),
            hunk_idx + 1,
            needle
        )));
    }
    if occurrences > 1 {
        return Err(ApplyPatchError::AmbiguousMatch {
            path: path.to_string_lossy().to_string(),
            occurrences,
        });
    }
    Ok(haystack.replacen(needle, replacement, 1))
}

/// 预演读取: 读当前内容并把 (len, mtime) 版本记入读前观测门禁.
///
/// 版本取自已开句柄的元数据 (fstat, 零额外 IO), 与读到的内容同源; 该版本
/// 即提交写携带的版本 CAS 钥匙。在既有版本语义 (原内容备份 + 严格唯一
/// 上下文匹配) 之上叠加观测, 不改写它们。
fn read_observed(
    path: &Path,
    gate: &ObservedGate,
) -> Result<(String, FileVersion), ApplyPatchError> {
    let mut file = fs::File::open(path).map_err(|e| ApplyPatchError::Io(e.to_string()))?;
    let version = FileVersion::from_metadata(
        &file
            .metadata()
            .map_err(|e| ApplyPatchError::Io(e.to_string()))?,
    );
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| ApplyPatchError::Io(e.to_string()))?;
    gate.observe_present(path, version);
    Ok((content, version))
}

/// 崩溃安全的文件替换: 委托统一存储基础件的持久档原子写
/// (同目录 `create_new` 独占临时文件 + `sync_all` + 替换)。
///
/// 语义: 新字节在临时文件内完整落盘后才替换目标 —— 半途中断不会留下截断的
/// 目标文件; 独占创建拒绝被预置的同名临时文件/symlink 抢占 (抢占者直接失败,
/// 不会静默写穿)。目标的 unix 权限在替换中保留; 新文件用
/// [`storage_atomic::DEFAULT_FILE_MODE`]。
fn atomic_write_file(path: &Path, content: &str) -> Result<(), ApplyPatchError> {
    storage_atomic::write_atomic_durable(path, content.as_bytes(), target_file_mode(path))
        .map_err(|e| ApplyPatchError::Io(e.to_string()))
}

/// 目标 unix 权限: 更新既有文件时保留原 mode (可执行位等不丢), 新文件用默认 mode。
fn target_file_mode(path: &Path) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fs::metadata(path) {
            Ok(meta) => meta.permissions().mode() & 0o7777,
            Err(_) => storage_atomic::DEFAULT_FILE_MODE,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        storage_atomic::DEFAULT_FILE_MODE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_and_apply_patch_transaction() {
        let temp_dir = std::env::temp_dir().join(format!("patch_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let initial_file = temp_dir.join("main.rs");
        fs::write(&initial_file, "fn old_code() {\n    println!(\"old\");\n}").unwrap();

        let patch = r#"*** Begin Patch
*** Add File: helper.rs
pub fn helper() -> bool { true }
*** Update File: main.rs
<<<<<<< SEARCH
fn old_code() {
    println!("old");
}
=======
fn new_code() {
    println!("new");
}
>>>>>>>
*** End Patch"#;

        let report = TransactionalPatchApplier::apply(&temp_dir, patch).unwrap();
        assert_eq!(report.files_added, vec!["helper.rs"]);
        assert_eq!(report.files_updated, vec!["main.rs"]);
        assert_eq!(report.total_actions, 2);

        let updated_content = fs::read_to_string(&initial_file).unwrap();
        assert!(updated_content.contains("fn new_code()"));
        assert!(temp_dir.join("helper.rs").exists());

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rollback_on_context_mismatch() {
        let temp_dir = std::env::temp_dir().join(format!("patch_fail_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let file = temp_dir.join("a.rs");
        fs::write(&file, "hello world").unwrap();

        let bad_patch = r#"*** Begin Patch
*** Update File: a.rs
<<<<<<< SEARCH
non_existent_context
=======
replaced
>>>>>>>
*** End Patch"#;

        let err = TransactionalPatchApplier::apply(&temp_dir, bad_patch).unwrap_err();
        assert!(matches!(err, ApplyPatchError::ContextMismatch(_)));
        assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn unique_match_rejects_ambiguous_search_replace() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "foo\nfoo\n").unwrap();

        let patch = "*** Begin Patch\n*** Update File: a.rs\n-foo\n+bar\n*** End Patch";

        let err = TransactionalPatchApplier::apply(dir.path(), patch).unwrap_err();
        match err {
            ApplyPatchError::AmbiguousMatch { occurrences, .. } => {
                assert_eq!(occurrences, 2);
            }
            other => panic!("expected AmbiguousMatch, got {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(dir.path().join("a.rs")).unwrap(),
            "foo\nfoo\n"
        );
    }

    #[test]
    fn parse_line_style_update_hunk() {
        let patch =
            "*** Begin Patch\n*** Update File: a.txt\n@@ anchor 1\n-old\n+new\n*** End Patch";
        let ops = TransactionalPatchApplier::parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            FilePatchAction::Update { path, hunks } => {
                assert_eq!(path.to_str().unwrap(), "a.txt");
                assert_eq!(hunks.len(), 1);
                assert_eq!(hunks[0].search_context, "old");
                assert_eq!(hunks[0].replacement_content, "new");
            }
            other => panic!("expected Update, got {other:?}"),
        }
    }

    #[test]
    fn parse_add_file_strips_plus_prefix() {
        let patch = "*** Begin Patch\n*** Add File: new.txt\n+hello\n+world\n*** End Patch";
        let ops = TransactionalPatchApplier::parse_patch(patch).unwrap();
        match &ops[0] {
            FilePatchAction::Add { path, content } => {
                assert_eq!(path.to_str().unwrap(), "new.txt");
                assert_eq!(content, "hello\nworld");
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn apply_line_style_update_real_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "hello old world").unwrap();
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+NEW\n*** End Patch";
        TransactionalPatchApplier::apply(dir.path(), patch).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "hello NEW world"
        );
    }

    #[test]
    fn apply_ambiguous_match_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "x x x").unwrap();
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-x\n+Y\n*** End Patch";
        let err = TransactionalPatchApplier::apply(dir.path(), patch).unwrap_err();
        match err {
            ApplyPatchError::AmbiguousMatch { occurrences, .. } => {
                assert_eq!(occurrences, 3);
            }
            other => panic!("expected AmbiguousMatch, got {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "x x x"
        );
    }

    #[test]
    fn apply_old_context_not_found() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "foo").unwrap();
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-bar\n+qux\n*** End Patch";
        let err = TransactionalPatchApplier::apply(dir.path(), patch).unwrap_err();
        assert!(matches!(err, ApplyPatchError::ContextMismatch(_)));
    }

    #[test]
    fn parse_line_hunk_without_old_lines_rejected() {
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n+only_add\n*** End Patch";
        let err = TransactionalPatchApplier::parse_patch(patch).unwrap_err();
        assert!(matches!(err, ApplyPatchError::ParseError(_)));
    }

    #[test]
    fn atomic_write_creates_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.txt");
        atomic_write_file(&target, "hello atomic").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello atomic");
        atomic_write_file(&target, "new").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        // No leftover tmp/bak siblings.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".tmp-") || n.ends_with(".bak"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn unique_match_replaces_the_single_occurrence() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "alpha\nfoo\nbeta\n").unwrap();
        let patch = "*** Begin Patch\n*** Update File: a.rs\n-foo\n+bar\n*** End Patch";
        TransactionalPatchApplier::apply(dir.path(), patch).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("a.rs")).unwrap(),
            "alpha\nbar\nbeta\n"
        );
    }

    // ---- H4 (2026-09-24 审计): 根目录包含校验 ----

    #[test]
    fn apply_rejects_absolute_add_path() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside.txt");
        let patch = format!(
            "*** Begin Patch\n*** Add File: {}\npwned\n*** End Patch",
            outside.display()
        );
        let err = TransactionalPatchApplier::apply(dir.path(), &patch).unwrap_err();
        assert!(
            matches!(err, ApplyPatchError::PathViolation(_)),
            "expected PathViolation, got {err:?}"
        );
        assert!(!outside.exists(), "absolute path must never be written");
    }

    #[test]
    fn apply_rejects_absolute_delete_path() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim.txt");
        fs::write(&victim, "keep me").unwrap();
        let patch = format!(
            "*** Begin Patch\n*** Delete File: {}\n*** End Patch",
            victim.display()
        );
        let err = TransactionalPatchApplier::apply(dir.path(), &patch).unwrap_err();
        assert!(matches!(err, ApplyPatchError::PathViolation(_)), "{err:?}");
        assert!(victim.exists(), "absolute path must never be deleted");
    }

    #[test]
    fn apply_rejects_parent_dir_escape() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside.txt");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "secret").unwrap();

        let patch = "*** Begin Patch\n*** Delete File: ../outside.txt\n*** End Patch";
        let err = TransactionalPatchApplier::apply(&root, patch).unwrap_err();
        assert!(matches!(err, ApplyPatchError::PathViolation(_)), "{err:?}");
        assert!(
            outside.exists(),
            "`..` traversal must never delete outside the root"
        );
    }

    #[cfg(windows)]
    #[test]
    fn apply_rejects_windows_style_parent_escape() {
        // Windows 词法: `..\` 是真正的父目录组件 (Unix 上反斜杠不是分隔符,
        // 该字面量只是个怪文件名, 无逃逸能力, 故此用例仅 Windows 断言)。
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        fs::create_dir(&root).unwrap();
        let patch = "*** Begin Patch\n*** Add File: ..\\..\\evil.bat\npwned\n*** End Patch";
        let err = TransactionalPatchApplier::apply(&root, patch).unwrap_err();
        assert!(matches!(err, ApplyPatchError::PathViolation(_)), "{err:?}");
    }

    #[test]
    fn apply_resolves_nested_new_file_inside_root() {
        // 目标不存在时对最近存在祖先 canonicalize 后 join 余量 —— 深层新文件
        // 仍可正常落盘 (H4 不得把合法用例挡掉)。
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        let patch =
            "*** Begin Patch\n*** Add File: src/deep/new_file.rs\npub fn f() {}\n*** End Patch";
        let report = TransactionalPatchApplier::apply(dir.path(), patch).unwrap();
        assert_eq!(report.files_added, vec!["src/deep/new_file.rs"]);
        assert_eq!(
            fs::read_to_string(dir.path().join("src/deep/new_file.rs")).unwrap(),
            "pub fn f() {}"
        );
    }

    #[test]
    fn apply_rejects_symlink_escape() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside.txt");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "secret").unwrap();

        let link = root.join("link.txt");
        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&outside, &link).is_ok();
        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_file(&outside, &link).is_ok();
        if !created {
            // Symlink creation often needs extra privileges on Windows.
            return;
        }

        let patch = "*** Begin Patch\n*** Update File: link.txt\n<<<<<<< SEARCH\nsecret\n=======\npwned\n>>>>>>>\n*** End Patch";
        let err = TransactionalPatchApplier::apply(&root, patch).unwrap_err();
        assert!(
            matches!(err, ApplyPatchError::PathViolation(_)),
            "symlink escape must be rejected, got {err:?}"
        );
        assert_eq!(fs::read_to_string(&outside).unwrap(), "secret");
    }

    #[test]
    fn apply_rejects_symlinked_directory_escape() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("target.txt"), "secret").unwrap();

        let link = root.join("escape_dir");
        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&outside, &link).is_ok();
        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_dir(&outside, &link).is_ok();
        if !created {
            return;
        }

        let patch = "*** Begin Patch\n*** Delete File: escape_dir/target.txt\n*** End Patch";
        let err = TransactionalPatchApplier::apply(&root, patch).unwrap_err();
        assert!(
            matches!(err, ApplyPatchError::PathViolation(_)),
            "symlinked directory escape must be rejected, got {err:?}"
        );
        assert!(outside.join("target.txt").exists());
    }

    #[cfg(windows)]
    #[test]
    fn apply_rejects_drive_prefixed_path() {
        // Windows 词法: `C:evil.txt` 带 Prefix 组件 (is_absolute 为 false 但
        // join 会替换 base) —— components 枚举必须把 Prefix 拒绝。
        let dir = tempfile::tempdir().unwrap();
        let patch = "*** Begin Patch\n*** Add File: C:evil.txt\npwned\n*** End Patch";
        let err = TransactionalPatchApplier::apply(dir.path(), patch).unwrap_err();
        assert!(
            matches!(err, ApplyPatchError::PathViolation(_)),
            "drive-relative path must be rejected, got {err:?}"
        );
    }

    #[test]
    fn atomic_write_uses_exclusive_create() {
        // create_new 语义 (统一存储基础件): 预置的同名 tmp 必须让写入失败而不是
        // 被静默覆盖 (防同路径 symlink 预置抢占)。tmp 名含 pid + 进程内原子计数器,
        // 并行测试下本次调用拿到的 seq 不可预测 —— 预占一段远超本测试二进制写调用
        // 总数的 seq 区间保证必中, 退出前统一清理。
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("t.txt");
        let pid = std::process::id();
        let mut occupied = Vec::new();
        // 预占 0..256 已远超本测试二进制的全部写调用数, 必中; 同名预置只影响
        // 同目标名 (t.txt) 的写, 不干扰其它用例的并行写。
        for seq in 0..256u64 {
            let candidate = dir.path().join(format!("t.txt.tmp-{pid}-{seq}"));
            if fs::write(&candidate, "occupied").is_ok() {
                occupied.push(candidate);
            }
        }
        let err = atomic_write_file(&target, "hello").unwrap_err();
        assert!(
            matches!(err, ApplyPatchError::Io(_)),
            "occupied tmp name must fail the write, got {err:?}"
        );
        assert!(!target.exists());
        for candidate in occupied {
            let _ = fs::remove_file(candidate);
        }
    }

    // ---- 读前观测门禁叠加 (提交写盘走版本 CAS 双钥匙) ----

    #[test]
    fn apply_with_gate_records_observations_and_enforces_version_cas() {
        use crate::observed_gate::{
            gated_write_atomic, GateDenial, GatedWriteError, WriteKind as GateWriteKind,
            WriteRequest,
        };

        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "hello old world").unwrap();
        let gate = ObservedGate::new();
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+NEW\n*** End Patch";
        TransactionalPatchApplier::apply_with_gate(dir.path(), patch, &gate).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "hello NEW world"
        );

        // 事务收尾回填观测: 同会话后续覆盖写凭回填版本直接过 CAS。
        let target = dir.path().join("a.txt");
        let observed = gate
            .observed_version(&target)
            .expect("写后必须回填观测版本");
        assert_eq!(observed.len, fs::metadata(&target).unwrap().len());
        let request = WriteRequest {
            path: target.clone(),
            kind: GateWriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
        };
        gated_write_atomic(
            &gate,
            &FsVersionProbe,
            &request,
            b"second",
            storage_atomic::DEFAULT_FILE_MODE,
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "second");

        // 外部改动后, 旧钥匙被版本 CAS 拒绝, 外部内容不被覆盖。
        fs::write(&target, "external").unwrap();
        let stale = WriteRequest {
            path: target.clone(),
            kind: GateWriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
        };
        let err = gated_write_atomic(
            &gate,
            &FsVersionProbe,
            &stale,
            b"third",
            storage_atomic::DEFAULT_FILE_MODE,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                GatedWriteError::Denied(GateDenial::VersionMismatch { .. })
            ),
            "外部改动后旧钥匙必须被拒, got {err:?}"
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "external");
    }

    #[test]
    fn apply_with_gate_creates_only_when_target_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let gate = ObservedGate::new();
        let patch = "*** Begin Patch\n*** Add File: new.txt\n+created\n*** End Patch";
        let report = TransactionalPatchApplier::apply_with_gate(dir.path(), patch, &gate).unwrap();
        assert_eq!(report.files_added, vec!["new.txt"]);
        assert_eq!(
            fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "created"
        );

        // 目标已出现: 创建路径拒绝覆盖 (预演 FileAlreadyExists 与提交
        // createIfAbsent 钥匙同口径, 任一触发都不落盘)。
        let again = "*** Begin Patch\n*** Add File: new.txt\n+clobbered\n*** End Patch";
        let err = TransactionalPatchApplier::apply_with_gate(dir.path(), again, &gate).unwrap_err();
        assert!(
            matches!(err, ApplyPatchError::FileAlreadyExists(_)),
            "创建撞已有文件必须被拒, got {err:?}"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "created"
        );
    }
}
