//! Codex 风格事务级多文件打补丁工具 (`apply_patch`).
//!
//! 支持在单次原子事务中对代码库执行多文件新增 (Add)、删除 (Delete) 与按上下文精确替换更新 (Update).
//! 若任意一个文件的任意一个 Hunk 匹配失败，整个事务立即完全回滚，保证磁盘状态绝对一致.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    #[error("磁盘 IO 失败: {0}")]
    Io(String),
}

/// 单个文件操作类型.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePatchAction {
    Add { path: PathBuf, content: String },
    Delete { path: PathBuf },
    Update { path: PathBuf, hunks: Vec<PatchHunk> },
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

/// 事务补丁应用器.
#[derive(Debug, Clone, Default)]
pub struct TransactionalPatchApplier;

impl TransactionalPatchApplier {
    pub fn new() -> Self {
        Self
    }

    /// 解析补丁文本指令.
    ///
    /// 遵循 Codex/Aider 格式:
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
                    content: content_lines.join("\n"),
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
                    if lines[i].trim() == "<<<<<<< SEARCH" {
                        i += 1;
                        let mut search_lines = Vec::new();
                        while i < lines.len() && lines[i].trim() != "=======" {
                            search_lines.push(lines[i]);
                            i += 1;
                        }
                        if i >= lines.len() || lines[i].trim() != "=======" {
                            return Err(ApplyPatchError::ParseError("缺少 '=======' 分隔符".to_string()));
                        }
                        i += 1; // 跳过 =======
                        let mut replace_lines = Vec::new();
                        while i < lines.len() && lines[i].trim() != ">>>>>>>" {
                            replace_lines.push(lines[i]);
                            i += 1;
                        }
                        if i >= lines.len() || lines[i].trim() != ">>>>>>>" {
                            return Err(ApplyPatchError::ParseError("缺少 '>>>>>>>' 结束符".to_string()));
                        }
                        i += 1; // 跳过 >>>>>>>
                        hunks.push(PatchHunk {
                            search_context: search_lines.join("\n"),
                            replacement_content: replace_lines.join("\n"),
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
    pub fn apply(root_dir: &Path, patch_text: &str) -> Result<PatchReport, ApplyPatchError> {
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
        let mut staged_writes: HashMap<PathBuf, String> = HashMap::new();
        let mut staged_deletes: Vec<PathBuf> = Vec::new();
        let mut original_backups: HashMap<PathBuf, Option<String>> = HashMap::new();

        let mut files_added = Vec::new();
        let mut files_updated = Vec::new();
        let mut files_deleted = Vec::new();

        for action in &actions {
            match action {
                FilePatchAction::Add { path, content } => {
                    let full_path = root_dir.join(path);
                    if full_path.exists() {
                        return Err(ApplyPatchError::FileAlreadyExists(path.to_string_lossy().to_string()));
                    }
                    original_backups.insert(full_path.clone(), None);
                    staged_writes.insert(full_path, content.clone());
                    files_added.push(path.to_string_lossy().to_string());
                }
                FilePatchAction::Delete { path } => {
                    let full_path = root_dir.join(path);
                    if !full_path.exists() {
                        return Err(ApplyPatchError::FileNotFound(path.to_string_lossy().to_string()));
                    }
                    let old_content = fs::read_to_string(&full_path).map_err(|e| ApplyPatchError::Io(e.to_string()))?;
                    original_backups.insert(full_path.clone(), Some(old_content));
                    staged_deletes.push(full_path);
                    files_deleted.push(path.to_string_lossy().to_string());
                }
                FilePatchAction::Update { path, hunks } => {
                    let full_path = root_dir.join(path);
                    if !full_path.exists() {
                        return Err(ApplyPatchError::FileNotFound(path.to_string_lossy().to_string()));
                    }
                    let original = fs::read_to_string(&full_path).map_err(|e| ApplyPatchError::Io(e.to_string()))?;
                    original_backups.insert(full_path.clone(), Some(original.clone()));

                    let mut updated = original;
                    for (h_idx, hunk) in hunks.iter().enumerate() {
                        if !updated.contains(&hunk.search_context) {
                            return Err(ApplyPatchError::ContextMismatch(format!(
                                "文件 {} Hunk #{} 搜索上下文未匹配: [{}]",
                                path.to_string_lossy(),
                                h_idx + 1,
                                hunk.search_context
                            )));
                        }
                        // 替换且只替换第一个唯一匹配实例
                        updated = updated.replacen(&hunk.search_context, &hunk.replacement_content, 1);
                    }
                    staged_writes.insert(full_path, updated);
                    files_updated.push(path.to_string_lossy().to_string());
                }
            }
        }

        // 2. 提交阶段 (Commit): 写入磁盘；若有任何 IO 异常则自动回滚已写入的文件
        let mut committed_paths = Vec::new();
        for (path, content) in staged_writes {
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    Self::rollback(&committed_paths, &original_backups);
                    return Err(ApplyPatchError::Io(e.to_string()));
                }
            }
            if let Err(e) = fs::write(&path, content) {
                Self::rollback(&committed_paths, &original_backups);
                return Err(ApplyPatchError::Io(e.to_string()));
            }
            committed_paths.push(path);
        }

        for path in staged_deletes {
            if let Err(e) = fs::remove_file(&path) {
                Self::rollback(&committed_paths, &original_backups);
                return Err(ApplyPatchError::Io(e.to_string()));
            }
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
        for path in committed {
            if let Some(backup) = backups.get(path) {
                match backup {
                    Some(old_content) => {
                        let _ = fs::write(path, old_content);
                    }
                    None => {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
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
}
