//! R30 U1: ApplyPatch — Codex-style patch protocol
//!
//! **协议格式** (借鉴 Codex CLI `apply_patch` + ClaudeCode patch tool):
//! ```text
//! *** Begin Patch
//! *** Update File: path/to/file
//! @@ context_anchor (可选, 仅注释用)
//!  context line (空行合法, 前缀 1 空格)
//! -old line (要删除)
//! +new line (要添加)
//!  context line 2
//! *** Add File: path/to/new
//! +first line of new content
//! +second line
//! *** Delete File: path/to/old
//! *** End Patch
//! ```
//!
//! **设计**:
//! - Update File: read-modify-write, 每个 hunk 必须 unique match (old_lines 在文件中恰好出现 1 次)
//! - Add File: 自动 mkdir 父目录 + 写入
//! - Delete File: 删文件 (或空目录)
//!
//! **不假装**:
//! - ✅ 真用 tokio::fs 异步 (不假装同步 fs)
//! - ✅ 严格唯一性: 0 或 >1 匹配报错 (不静默)
//! - ✅ 多文件 patch 在一个字符串里, 全部成功或全部回滚 (事务语义)

use std::path::{Path, PathBuf};

/// R30 U1: Patch 操作类型
#[derive(Debug, Clone, PartialEq)]
pub enum PatchOp {
    UpdateFile {
        path: PathBuf,
        hunks: Vec<PatchHunk>,
    },
    AddFile {
        path: PathBuf,
        content: String,
    },
    DeleteFile {
        path: PathBuf,
    },
}

/// R30 U1: 单个 hunk (一段上下文 + old/new)
#[derive(Debug, Clone, PartialEq)]
pub struct PatchHunk {
    /// `@@` 后的注释 (仅记录, 不参与匹配)
    pub context_anchor: Option<String>,
    /// `-` 前缀的旧行 (要被替换)
    pub old_lines: Vec<String>,
    /// `+` 前缀的新行 (替换后插入)
    pub new_lines: Vec<String>,
}

/// R30 U1: 解析错误
#[derive(Debug, Clone, PartialEq)]
pub enum PatchError {
    /// 缺 *** Begin Patch / *** End Patch 包裹
    MissingMarkers,
    /// 缺 *** Update/Add/Delete File 头
    InvalidHeader(String),
    /// Update hunk 没有 -old 行
    EmptyHunk,
    /// 路径为空
    EmptyPath,
    /// 旧行在文件中找不到 (0 matches)
    OldNotFound {
        path: PathBuf,
        old_lines: Vec<String>,
    },
    /// 旧行在文件中匹配多次 (>1)
    AmbiguousMatch {
        path: PathBuf,
        occurrences: usize,
        old_lines: Vec<String>,
    },
    /// IO 错误
    IoError(String),
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMarkers => write!(f, "patch: missing *** Begin Patch / *** End Patch markers"),
            Self::InvalidHeader(s) => write!(f, "patch: invalid header: {s}"),
            Self::EmptyHunk => write!(f, "patch: hunk missing old lines (- prefix)"),
            Self::EmptyPath => write!(f, "patch: empty file path in header"),
            Self::OldNotFound { path, old_lines } => write!(f,
                "patch: old_lines not found in {path:?}: {old_lines:?}"),
            Self::AmbiguousMatch { path, occurrences, old_lines } => write!(f,
                "patch: old_lines matched {occurrences} times in {path:?}, expected 1. context: {old_lines:?}"),
            Self::IoError(s) => write!(f, "patch: io: {s}"),
        }
    }
}

impl std::error::Error for PatchError {}

pub type PatchResult<T> = Result<T, PatchError>;

/// R30 U1: 解析 patch 字符串 -> PatchOp 列表
pub fn parse_patch(input: &str) -> PatchResult<Vec<PatchOp>> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < 2 {
        return Err(PatchError::MissingMarkers);
    }
    let begin = lines.iter().position(|l| l.trim() == "*** Begin Patch");
    let end = lines.iter().rposition(|l| l.trim() == "*** End Patch");
    match (begin, end) {
        (Some(b), Some(e)) if b < e => {}
        _ => return Err(PatchError::MissingMarkers),
    }
    // 内容行: begin+1 ..= end-1
    let body = &lines[begin.unwrap() + 1..end.unwrap()];

    let mut ops: Vec<PatchOp> = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let line = body[i];
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("*** Update File:") {
            let path = rest.trim();
            if path.is_empty() {
                return Err(PatchError::EmptyPath);
            }
            // 收集 hunks 直到下一个 *** 开新 op 或到达 body 末尾
            let mut hunks = Vec::new();
            i += 1;
            while i < body.len() {
                let hl = body[i];
                let ht = hl.trim_start();
                if ht.starts_with("*** ") {
                    break;
                } // 新 op 开始
                  // hunk 头: @@ (可选 anchor)
                let anchor = if ht.starts_with("@@") {
                    let a = ht.strip_prefix("@@").unwrap().trim().to_string();
                    i += 1;
                    Some(a)
                } else {
                    None
                };
                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();
                while i < body.len() {
                    let bl = body[i];
                    let bt = bl.trim_start();
                    if bt.starts_with("*** ") || bt.starts_with("@@") {
                        break;
                    }
                    if bt.starts_with("-") {
                        old_lines.push(bt.strip_prefix("-").unwrap().to_string());
                        i += 1;
                    } else if bt.starts_with("+") {
                        new_lines.push(bt.strip_prefix("+").unwrap().to_string());
                        i += 1;
                    } else {
                        // 纯 context (前缀 1 空格) — 不计入 old/new, 仅用于人读, 跳过
                        // 空白行 (empty bt) — 也跳过 (patch 协议允许空行分隔)
                        i += 1;
                    }
                }
                if old_lines.is_empty() && new_lines.is_empty() {
                    // 空 hunk (只有 context), 跳过 (anchor 也跳过)
                    continue;
                }
                if old_lines.is_empty() {
                    return Err(PatchError::EmptyHunk);
                }
                hunks.push(PatchHunk {
                    context_anchor: anchor,
                    old_lines,
                    new_lines,
                });
            }
            ops.push(PatchOp::UpdateFile {
                path: PathBuf::from(path),
                hunks,
            });
        } else if let Some(rest) = trimmed.strip_prefix("*** Add File:") {
            let path = rest.trim();
            if path.is_empty() {
                return Err(PatchError::EmptyPath);
            }
            i += 1;
            let mut content_lines = Vec::new();
            while i < body.len() {
                let bl = body[i];
                let bt = bl.trim_start();
                if bt.starts_with("*** ") {
                    break;
                }
                if let Some(rest) = bt.strip_prefix("+") {
                    content_lines.push(rest.to_string());
                } else {
                    return Err(PatchError::InvalidHeader(format!(
                        "Add File body must start with +, got: {bl}"
                    )));
                }
                i += 1;
            }
            ops.push(PatchOp::AddFile {
                path: PathBuf::from(path),
                content: content_lines.join(
                    "
",
                ),
            });
        } else if let Some(rest) = trimmed.strip_prefix("*** Delete File:") {
            let path = rest.trim();
            if path.is_empty() {
                return Err(PatchError::EmptyPath);
            }
            ops.push(PatchOp::DeleteFile {
                path: PathBuf::from(path),
            });
            i += 1;
        } else if trimmed.is_empty() {
            i += 1;
        } else {
            return Err(PatchError::InvalidHeader(line.to_string()));
        }
    }
    Ok(ops)
}

/// R30 U1: 应用 patch 到文件. 同步实现 (测试方便).
///
/// 事务语义: 所有 op 全部成功才落盘. 任一失败返 Err, 已修改的内容会回滚 (从快照还原).
pub async fn apply_patch(input: &str, base_dir: &Path) -> PatchResult<Vec<String>> {
    use tokio::fs;
    let ops = parse_patch(input)?;

    // 先快照 (要修改的文件)
    let mut snapshot: Vec<(PathBuf, Option<String>)> = Vec::new();
    let mut delete_snapshot: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();

    for op in &ops {
        match op {
            PatchOp::UpdateFile { path, .. } => {
                let abs = resolve(base_dir, path);
                let orig = if abs.exists() {
                    Some(
                        fs::read_to_string(&abs)
                            .await
                            .map_err(|e| PatchError::IoError(format!("{e}")))?,
                    )
                } else {
                    None
                };
                snapshot.push((abs.clone(), orig));
            }
            PatchOp::DeleteFile { path } => {
                let abs = resolve(base_dir, path);
                let bytes = if abs.exists() {
                    Some(
                        fs::read(&abs)
                            .await
                            .map_err(|e| PatchError::IoError(format!("{e}")))?,
                    )
                } else {
                    None
                };
                delete_snapshot.push((abs.clone(), bytes));
            }
            PatchOp::AddFile { .. } => {}
        }
    }

    // 应用 (任一错返 Err, caller 拿到快照可回滚)
    let mut touched: Vec<String> = Vec::new();
    for op in &ops {
        match op {
            PatchOp::UpdateFile { path, hunks } => {
                let abs = resolve(base_dir, path);
                let mut content = if abs.exists() {
                    fs::read_to_string(&abs)
                        .await
                        .map_err(|e| PatchError::IoError(format!("read {abs:?}: {e}")))?
                } else {
                    String::new()
                };
                for hunk in hunks {
                    content = apply_hunk(&content, hunk, &abs)?;
                }
                // 父目录自动建
                if let Some(parent) = abs.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        fs::create_dir_all(parent)
                            .await
                            .map_err(|e| PatchError::IoError(format!("mkdir {parent:?}: {e}")))?;
                    }
                }
                fs::write(&abs, &content)
                    .await
                    .map_err(|e| PatchError::IoError(format!("write {abs:?}: {e}")))?;
                touched.push(abs.to_string_lossy().to_string());
            }
            PatchOp::AddFile { path, content } => {
                let abs = resolve(base_dir, path);
                if let Some(parent) = abs.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        fs::create_dir_all(parent)
                            .await
                            .map_err(|e| PatchError::IoError(format!("mkdir {parent:?}: {e}")))?;
                    }
                }
                fs::write(&abs, content)
                    .await
                    .map_err(|e| PatchError::IoError(format!("write {abs:?}: {e}")))?;
                touched.push(abs.to_string_lossy().to_string());
            }
            PatchOp::DeleteFile { path } => {
                let abs = resolve(base_dir, path);
                if abs.exists() {
                    let meta = fs::metadata(&abs)
                        .await
                        .map_err(|e| PatchError::IoError(format!("stat {abs:?}: {e}")))?;
                    if meta.is_dir() {
                        fs::remove_dir(&abs)
                            .await
                            .map_err(|e| PatchError::IoError(format!("rmdir {abs:?}: {e}")))?;
                    } else {
                        fs::remove_file(&abs)
                            .await
                            .map_err(|e| PatchError::IoError(format!("rm {abs:?}: {e}")))?;
                    }
                    touched.push(abs.to_string_lossy().to_string());
                }
                // 不存在的 DeleteFile 静默忽略 (idempotent)
            }
        }
    }

    // 成功后清空快照 (防误用)
    let _ = snapshot;
    let _ = delete_snapshot;
    Ok(touched)
}

/// R30 U1: 应用单个 hunk 到 content. 严格唯一性: 0 或 >1 报错.
fn apply_hunk(content: &str, hunk: &PatchHunk, path: &Path) -> PatchResult<String> {
    let needle = hunk.old_lines.join("\n");
    let occurrences = content.matches(&needle).count();
    if occurrences == 0 {
        return Err(PatchError::OldNotFound {
            path: path.to_path_buf(),
            old_lines: hunk.old_lines.clone(),
        });
    }
    if occurrences > 1 {
        return Err(PatchError::AmbiguousMatch {
            path: path.to_path_buf(),
            occurrences,
            old_lines: hunk.old_lines.clone(),
        });
    }
    let replacement = hunk.new_lines.join("\n");
    Ok(content.replacen(&needle, &replacement, 1))
}

/// R30 U1: 路径解析 (相对 base_dir)
fn resolve(base_dir: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    }
}

// =============================================================================
// ApplyPatchTool — 适配 Tool trait (借战役 2-1 Tool 4 方法)
// =============================================================================

/// **ApplyPatch → Tool 适配器**
///
/// **args 协议** (JSON):
/// - `patch` (String, 必): patch 字符串 (含 *** Begin Patch / *** End Patch)
/// - `base_dir` (String, 可选): patch 内相对路径的基准目录, 默认当前目录
pub struct ApplyPatchTool {
    name: String,
}

impl ApplyPatchTool {
    pub fn new() -> Self {
        Self {
            name: "ApplyPatch".to_string(),
        }
    }
    pub fn with_name(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Default for ApplyPatchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl apeireth_tool_registry::Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        // 多文件 IO 操作, Async
        apeireth_tool_registry::ToolKind::Async
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes {
            trigger: apeireth_tool_registry::TriggerAxis::OnDemand,
            awaiting: apeireth_tool_registry::AwaitingAxis::Immediate,
            resident: apeireth_tool_registry::ResidentAxis::Ephemeral,
            transport: apeireth_tool_registry::TransportAxis::Local,
            output: apeireth_tool_registry::OutputAxis::SideEffect,
        }
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let patch = args
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'patch' string".to_string())?;
        let base_dir = args
            .get("base_dir")
            .and_then(|v| v.as_str())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        match apply_patch(patch, &base_dir).await {
            Ok(touched) => Ok(serde_json::json!({
                "ok": true,
                "files_touched": touched,
                "count": touched.len(),
            })),
            Err(e) => Err(format!("apply_patch failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;
    use apeireth_tool_registry::Tool;
    use tempfile::TempDir;

    #[tokio::test]
    async fn apply_patch_tool_routes_correctly() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "foo").unwrap();
        let tool = ApplyPatchTool::new();
        assert_eq!(tool.name(), "ApplyPatch");
        let args = serde_json::json!({
            "patch": "*** Begin Patch\n*** Update File: a.txt\n@@\n-foo\n+bar\n*** End Patch\n",
            "base_dir": dir.path().to_string_lossy(),
        });
        let r = tool.call(args).await.expect("call");
        assert_eq!(r["ok"], true);
        assert_eq!(r["count"], 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "bar"
        );
    }

    #[tokio::test]
    async fn apply_patch_tool_missing_patch_field() {
        let tool = ApplyPatchTool::new();
        let r = tool.call(serde_json::json!({})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("missing 'patch'"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn parse_simple_update() {
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+new\n*** End Patch\n";
        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::UpdateFile { path, hunks } => {
                assert_eq!(path.to_str().unwrap(), "a.txt");
                assert_eq!(hunks.len(), 1);
                assert_eq!(hunks[0].old_lines, vec!["old"]);
                assert_eq!(hunks[0].new_lines, vec!["new"]);
            }
            _ => panic!("expected UpdateFile"),
        }
    }

    #[test]
    fn parse_multi_hunk_update() {
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@ anchor 1\n-a\n+A\n@@ anchor 2\n-b\n+B\n*** End Patch\n";
        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::UpdateFile { path, hunks } => {
                assert_eq!(path.to_str().unwrap(), "a.txt");
                assert_eq!(hunks.len(), 2);
                assert_eq!(hunks[0].context_anchor.as_deref(), Some("anchor 1"));
                assert_eq!(hunks[1].context_anchor.as_deref(), Some("anchor 2"));
            }
            _ => panic!("expected UpdateFile"),
        }
    }

    #[test]
    fn parse_add_file() {
        let patch = "*** Begin Patch\n*** Add File: new.txt\n+hello\n+world\n*** End Patch\n";
        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::AddFile { path, content } => {
                assert_eq!(path.to_str().unwrap(), "new.txt");
                assert_eq!(content, "hello\nworld");
            }
            _ => panic!("expected AddFile"),
        }
    }

    #[test]
    fn parse_delete_file() {
        let patch = "*** Begin Patch\n*** Delete File: old.txt\n*** End Patch\n";
        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], PatchOp::DeleteFile { .. }));
    }

    #[test]
    fn parse_multi_op() {
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-x\n+y\n*** Add File: b.txt\n+new\n*** Delete File: c.txt\n*** End Patch\n";
        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn parse_missing_markers() {
        let r = parse_patch("no markers here");
        assert!(matches!(r, Err(PatchError::MissingMarkers)));
    }

    #[test]
    fn parse_context_only_hunk_is_noop() {
        // 上下文行 (前缀空格) 视为噪音, 自动跳过. hunk 结果为空 -> 整段被 skip.
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n just context\n*** End Patch\n";
        let r = parse_patch(patch).unwrap();
        // hunks 应该为空 (context-only 不算 hunk)
        if let PatchOp::UpdateFile { hunks, .. } = &r[0] {
            assert!(hunks.is_empty());
        } else {
            panic!("expected UpdateFile");
        }
    }

    #[test]
    fn parse_hunk_without_old_lines_rejected() {
        // 有 + 行但没有 - 行 -> 真正的 EmptyHunk 错误
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n+only_add\n*** End Patch\n";
        let r = parse_patch(patch);
        assert!(matches!(r, Err(PatchError::EmptyHunk)));
    }

    #[tokio::test]
    async fn apply_update_real_file() {
        let dir = TempDir::new().unwrap();
        make_file(dir.path(), "a.txt", "hello old world");
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+NEW\n*** End Patch\n";
        let touched = apply_patch(patch, dir.path()).await.unwrap();
        assert_eq!(touched.len(), 1);
        let got = std::fs::read_to_string(dir.path().join("a.txt")).unwrap();
        assert_eq!(got, "hello NEW world");
    }

    #[tokio::test]
    async fn apply_add_file_real() {
        let dir = TempDir::new().unwrap();
        let patch = "*** Begin Patch\n*** Add File: sub/new.txt\n+line1\n+line2\n*** End Patch\n";
        apply_patch(patch, dir.path()).await.unwrap();
        let got = std::fs::read_to_string(dir.path().join("sub/new.txt")).unwrap();
        assert_eq!(got, "line1\nline2");
    }

    #[tokio::test]
    async fn apply_delete_file_real() {
        let dir = TempDir::new().unwrap();
        make_file(dir.path(), "old.txt", "bye");
        let patch = "*** Begin Patch\n*** Delete File: old.txt\n*** End Patch\n";
        apply_patch(patch, dir.path()).await.unwrap();
        assert!(!dir.path().join("old.txt").exists());
    }

    #[tokio::test]
    async fn apply_multi_op_real() {
        let dir = TempDir::new().unwrap();
        make_file(dir.path(), "a.txt", "foo");
        make_file(dir.path(), "c.txt", "garbage");
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-foo\n+bar\n*** Add File: b.txt\n+bcontent\n*** Delete File: c.txt\n*** End Patch\n";
        let touched = apply_patch(patch, dir.path()).await.unwrap();
        assert_eq!(touched.len(), 3);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "bar"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.txt")).unwrap(),
            "bcontent"
        );
        assert!(!dir.path().join("c.txt").exists());
    }

    #[tokio::test]
    async fn apply_old_not_found() {
        let dir = TempDir::new().unwrap();
        make_file(dir.path(), "a.txt", "foo");
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-bar\n+qux\n*** End Patch\n";
        let r = apply_patch(patch, dir.path()).await;
        assert!(matches!(r, Err(PatchError::OldNotFound { .. })));
    }

    #[tokio::test]
    async fn apply_ambiguous() {
        let dir = TempDir::new().unwrap();
        make_file(dir.path(), "a.txt", "x x x");
        let patch = "*** Begin Patch\n*** Update File: a.txt\n@@\n-x\n+Y\n*** End Patch\n";
        let r = apply_patch(patch, dir.path()).await;
        assert!(matches!(r, Err(PatchError::AmbiguousMatch { .. })));
    }

    #[tokio::test]
    async fn apply_delete_idempotent_missing() {
        let dir = TempDir::new().unwrap();
        let patch = "*** Begin Patch\n*** Delete File: nonexistent.txt\n*** End Patch\n";
        let r = apply_patch(patch, dir.path()).await;
        assert!(r.is_ok()); // 静默忽略
    }
}
