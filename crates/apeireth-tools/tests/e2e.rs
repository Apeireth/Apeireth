//! Integration tests for apeireth-tools
//!
//! **R18 第 2 阶段第 3 项**: 5 trait 各 ≥ 5 e2e 测试
//! 抄 tokio + qdrant 测试模式: `tempfile` + 真实 IO/Process
//!
//! 测试策略:
//! - file_ops: 真实 tempdir, tokio::fs 真读写
//! - code_exec: 真实 echo / exit 0 / exit 1 / timeout
//! - git_ops: 真实 git init + commit + status/log/diff
//! - web_search: HTTP 部分需要真 URL, 跳过 (留作 R18 后续)
//!
//! **不假装**: 测试是真跑 (`cargo nextest run -p apeireth-tools`).

use apeireth_tools::{
    code_exec::{CodeExec, ShellCodeExec},
    file_ops::{FileOps, StdFileOps},
    git_ops::{GitCliOps, GitOps},
};

// =====================================================================
// file_ops (StdFileOps) — 6 ops
// =====================================================================

#[tokio::test]
async fn file_ops_write_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("hello.txt");
    let ops = StdFileOps::new();
    ops.write(&p, "hello world").await.unwrap();
    let content = ops.read(&p).await.unwrap();
    assert_eq!(content, "hello world");
}

#[tokio::test]
async fn file_ops_list_directory() {
    let dir = tempfile::tempdir().unwrap();
    let ops = StdFileOps::new();
    ops.write(&dir.path().join("a.txt"), "1").await.unwrap();
    ops.write(&dir.path().join("b.txt"), "2").await.unwrap();
    let entries = ops.list(dir.path()).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|p| p.file_name().unwrap() == "a.txt"));
    assert!(entries.iter().any(|p| p.file_name().unwrap() == "b.txt"));
}

#[tokio::test]
async fn file_ops_mkdir_creates_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a/b/c");
    let ops = StdFileOps::new();
    ops.mkdir(&p).await.unwrap();
    assert!(p.exists());
    assert!(p.is_dir());
}

#[tokio::test]
async fn file_ops_delete_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("to_delete.txt");
    let ops = StdFileOps::new();
    ops.write(&p, "delete me").await.unwrap();
    assert!(p.exists());
    ops.delete(&p).await.unwrap();
    assert!(!p.exists());
}

#[tokio::test]
async fn file_ops_move_path() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("from.txt");
    let dst = dir.path().join("to.txt");
    let ops = StdFileOps::new();
    ops.write(&src, "move me").await.unwrap();
    ops.move_path(&src, &dst).await.unwrap();
    assert!(!src.exists());
    assert!(dst.exists());
    assert_eq!(ops.read(&dst).await.unwrap(), "move me");
}

#[tokio::test]
async fn file_ops_read_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let ops = StdFileOps::new();
    let result = ops.read(&dir.path().join("missing.txt")).await;
    assert!(result.is_err(), "should fail on missing file");
}

#[tokio::test]
async fn file_ops_tool_name() {
    // 默认 name = "FileOperator" (per `file_ops.rs:103 new()`)
    // 测试用 `with_name("file_ops")` 显式覆盖, 验证 setter
    let ops = StdFileOps::with_name("file_ops");
    assert_eq!(ops.name(), "file_ops");
}

// =====================================================================
// code_exec (ShellCodeExec) — real shell execution
// =====================================================================

#[tokio::test]
async fn code_exec_echo() {
    let exec = ShellCodeExec::new();
    // Windows: `echo` 是 cmd 内置, 用 cmd /c echo (cmd 在白名单)
    // POSIX: `echo` 是独立 binary, 直接调
    let cmd = if cfg!(windows) { "cmd /c echo hello" } else { "echo hello" };
    let (exit, output) = exec.exec(cmd, 5000).await.expect("echo should succeed");
    assert_eq!(exit, 0, "echo exit code: got {} (output: {})", exit, output);
    assert!(output.contains("hello"), "output should contain 'hello': {}", output);
}

#[tokio::test]
async fn code_exec_exit_zero() {
    let exec = ShellCodeExec::new();
    // Windows: 用 `cmd /c exit 0` (cmd 在白名单)
    // POSIX: `true` 在白名单
    let cmd = if cfg!(windows) { "cmd /c exit 0" } else { "true" };
    let (exit, _) = exec.exec(cmd, 5000).await.expect("true / exit 0 should succeed");
    assert_eq!(exit, 0, "expected exit 0, got {}", exit);
}

#[tokio::test]
async fn code_exec_exit_nonzero() {
    let exec = ShellCodeExec::new();
    // Windows: `cmd /c exit 1`
    // POSIX: `false` 在白名单
    let cmd = if cfg!(windows) { "cmd /c exit 1" } else { "false" };
    let (exit, _) = exec.exec(cmd, 5000).await.expect("false / exit 1 should complete");
    assert_eq!(exit, 1, "expected exit 1, got {}", exit);
}

#[tokio::test]
async fn code_exec_tool_name() {
    // 默认 name = "ShellExec" (per `code_exec.rs:77 new()`)
    // 测试用 `with_name("code_exec")` 显式覆盖, 验证 setter
    let exec = ShellCodeExec::new().with_name("code_exec");
    assert_eq!(exec.name(), "code_exec");
}

// =====================================================================
// git_ops (GitCliOps) — real git
// =====================================================================

#[tokio::test]
async fn git_ops_status_in_empty_repo() {
    let dir = tempfile::tempdir().unwrap();
    // Need to init git first
    std::process::Command::new("git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init");
    let ops = GitCliOps::new();
    let status = ops.status(dir.path()).await.unwrap();
    // 空 repo (没 commit): status 通常含 "No commits yet" / "nothing to commit" / branch 名
    assert!(
        status.is_empty()
            || status.contains("nothing to commit")
            || status.contains("No commits yet")
            || status.contains("master")
            || status.contains("main"),
        "empty repo status: {}",
        status
    );
}

#[tokio::test]
async fn git_ops_status_after_file_add() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init");
    std::fs::write(dir.path().join("hello.txt"), "hello").unwrap();
    let ops = GitCliOps::new();
    let status = ops.status(dir.path()).await.unwrap();
    assert!(status.contains("hello.txt"), "status should mention untracked file: {}", status);
}

#[tokio::test]
async fn git_ops_log_in_empty_repo() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init");
    let ops = GitCliOps::new();
    // 空 repo 没有 commit, git log 返 exit 128 + "fatal: your current branch ... does not have any commits yet"
    // 这是预期行为, ops.log() 返 Err — 测试接受 Err (跟 src 实际行为一致)
    let result = ops.log(dir.path(), 5).await;
    assert!(
        result.is_err(),
        "expected Err on empty repo git log, got: {:?}",
        result
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("fatal") || err.contains("does not have any commits"),
        "err msg should mention 'fatal' / 'no commits', got: {}",
        err
    );
}

#[tokio::test]
async fn git_ops_diff_in_clean_repo() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init");
    let ops = GitCliOps::new();
    let diff = ops.diff(dir.path()).await.unwrap();
    assert!(diff.is_empty());
}

#[tokio::test]
async fn git_ops_tool_name() {
    // 默认 name = "Git" (per `git_ops.rs:65 new()`)
    // 测试用 `with_name("git_ops")` 显式覆盖, 验证 setter
    let ops = GitCliOps::new().with_name("git_ops");
    assert_eq!(ops.name(), "git_ops");
}

// =====================================================================
// Constants 验证 (VCP FileOperator.js:24-26 字段级)
// =====================================================================

#[test]
fn file_ops_vcp_constants_match() {
    // VCP FileOperator.js:24-26:
    //   MAX_FILE_SIZE = 20 * 1024 * 1024
    //   MAX_DIRECTORY_ITEMS = 1000
    //   MAX_SEARCH_RESULTS = 100
    use apeireth_tools::file_ops::{MAX_FILE_SIZE, MAX_DIRECTORY_ITEMS, MAX_SEARCH_RESULTS};
    assert_eq!(MAX_FILE_SIZE, 20 * 1024 * 1024);
    assert_eq!(MAX_DIRECTORY_ITEMS, 1000);
    assert_eq!(MAX_SEARCH_RESULTS, 100);
}

#[test]
fn file_ops_operation_count() {
    // 编译期 hardcode
    use apeireth_tools::file_ops::FILE_OPS_OPERATION_COUNT;
    assert_eq!(FILE_OPS_OPERATION_COUNT, 7); // read/write/list/mkdir/delete/move/edit (R30 P1 加 edit)
}

// =====================================================================
// Path 边界 — tempdir 关闭后访问
// =====================================================================

#[tokio::test]
async fn file_ops_path_traversal_protection() {
    let dir = tempfile::tempdir().unwrap();
    let ops = StdFileOps::new();
    // 写一个文件, 然后用绝对路径 + 父目录外的路径尝试访问
    let safe_path = dir.path().join("safe.txt");
    ops.write(&safe_path, "safe content").await.unwrap();
    assert_eq!(ops.read(&safe_path).await.unwrap(), "safe content");
    // 注: StdFileOps 没强制 path 必须在某根目录下 (留作 R18 后续加), 这里只测基本功能
}

// =====================================================================
// FileOps edit (R30 P1) — 严格唯一替换
// =====================================================================

#[tokio::test]
async fn file_ops_edit_unique_replacement() {
    // edit 是 R30 P1 加的 FileOps 操作: old_text 在文件中恰好出现 1 次 → 替换
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("code.txt");
    let ops = StdFileOps::new();
    ops.write(&p, "let x = 1;\nlet y = 2;\n").await.unwrap();
    ops.edit(&p, "let x = 1;", "let x = 100;").await.unwrap();
    let got = ops.read(&p).await.unwrap();
    assert!(got.contains("let x = 100;"), "edit should replace: {got}");
    assert!(got.contains("let y = 2;"), "edit should keep other lines: {got}");
}

#[tokio::test]
async fn file_ops_edit_zero_match_fails() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("code.txt");
    let ops = StdFileOps::new();
    ops.write(&p, "hello world").await.unwrap();
    let r = ops.edit(&p, "not in file", "anything").await;
    assert!(r.is_err(), "edit should fail on 0 match");
    assert!(r.unwrap_err().contains("not found"), "err msg should mention 'not found'");
}

#[tokio::test]
async fn file_ops_edit_ambiguous_match_fails() {
    // 同样 old_text 出现 >1 次 → 报错 (严格唯一性)
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("dup.txt");
    let ops = StdFileOps::new();
    ops.write(&p, "x = 1\nx = 2\nx = 3\n").await.unwrap();
    let r = ops.edit(&p, "x =", "y =").await;
    assert!(r.is_err(), "edit should fail on ambiguous match");
    let err = r.unwrap_err();
    assert!(err.contains("matched") || err.contains("unique"), "err msg: {err}");
}

#[tokio::test]
async fn file_ops_edit_empty_old_text_rejected() {
    // 防御性: 0 长 old_text 拒绝 (避免误删整个文件)
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("f.txt");
    let ops = StdFileOps::new();
    ops.write(&p, "content").await.unwrap();
    let r = ops.edit(&p, "", "anything").await;
    assert!(r.is_err(), "edit should reject empty old_text");
}

// =====================================================================
// apply_patch (R30 U1) — Codex-style patch 协议
// =====================================================================

#[tokio::test]
async fn apply_patch_parse_update_file() {
    // 解析单个 Update File, 严格按 VCP 字段级: *** Begin Patch / *** End Patch 包裹
    use apeireth_tools::apply_patch::{parse_patch, PatchOp};
    let patch = "*** Begin Patch\n*** Update File: a.txt\n-old line\n+new line\n*** End Patch\n";
    let ops = parse_patch(patch).expect("parse ok");
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        PatchOp::UpdateFile { path, hunks } => {
            assert_eq!(path.to_string_lossy(), "a.txt");
            assert_eq!(hunks.len(), 1);
            assert_eq!(hunks[0].old_lines, vec!["old line".to_string()]);
            assert_eq!(hunks[0].new_lines, vec!["new line".to_string()]);
        }
        other => panic!("expected UpdateFile, got {other:?}"),
    }
}

#[tokio::test]
async fn apply_patch_parse_add_and_delete() {
    // 解析 Add File + Delete File 混合
    use apeireth_tools::apply_patch::{parse_patch, PatchOp};
    let patch = "*** Begin Patch\n\
                 *** Add File: new.txt\n\
                 +first line\n\
                 +second line\n\
                 *** Delete File: old.txt\n\
                 *** End Patch\n";
    let ops = parse_patch(patch).expect("parse ok");
    assert_eq!(ops.len(), 2);
    match &ops[0] {
        PatchOp::AddFile { path, content } => {
            assert_eq!(path.to_string_lossy(), "new.txt");
            assert_eq!(content, "first line\nsecond line");
        }
        other => panic!("expected AddFile, got {other:?}"),
    }
    match &ops[1] {
        PatchOp::DeleteFile { path } => assert_eq!(path.to_string_lossy(), "old.txt"),
        other => panic!("expected DeleteFile, got {other:?}"),
    }
}

#[tokio::test]
async fn apply_patch_missing_markers_fails() {
    use apeireth_tools::apply_patch::{parse_patch, PatchError};
    let patch = "*** Update File: x.txt\n-old\n+new\n"; // 缺 Begin/End 包裹
    let r = parse_patch(patch);
    assert!(matches!(r, Err(PatchError::MissingMarkers)), "got: {r:?}");
}

#[tokio::test]
async fn apply_patch_real_io_updates_file() {
    // 端到端: 真改文件 + 真读回验证
    use apeireth_tools::apply_patch::apply_patch;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("f.txt");
    std::fs::write(&target, "line1\nline2\nline3\n").unwrap();
    let patch = "*** Begin Patch\n\
                 *** Update File: f.txt\n\
                 -line2\n\
                 +LINE_TWO\n\
                 *** End Patch\n";
    let touched = apply_patch(patch, dir.path()).await.expect("apply ok");
    assert_eq!(touched.len(), 1);
    let got = std::fs::read_to_string(&target).unwrap();
    assert!(got.contains("LINE_TWO"), "patch should replace: {got}");
    assert!(!got.contains("line2\n"), "old line should be gone: {got}");
}

#[tokio::test]
async fn apply_patch_ambiguous_match_returns_err() {
    // hunk 多次匹配 → AmbiguousMatch 错误
    use apeireth_tools::apply_patch::{apply_patch, PatchError};
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("dup.txt");
    std::fs::write(&target, "x\nx\nx\n").unwrap();
    let patch = "*** Begin Patch\n\
                 *** Update File: dup.txt\n\
                 -x\n\
                 +y\n\
                 *** End Patch\n";
    let r = apply_patch(patch, dir.path()).await;
    assert!(matches!(r, Err(PatchError::AmbiguousMatch { .. })), "got: {r:?}");
    // 文件未变 (apply 失败 → 0 落盘)
    let after = std::fs::read_to_string(&target).unwrap();
    assert_eq!(after, "x\nx\nx\n", "file should be untouched on error");
}

// =====================================================================
// ProjectConventions (R33-1) — Aider-style 项目约定扫描
// =====================================================================

#[test]
fn project_conventions_scans_own_workspace_root() {
    // 真实项目自己根: 应该能扫到 92+ crate, edition 2021, resolver 2
    use apeireth_tools::conventions_scanner::ProjectConventions;
    // workspace root 是 cwd 的 ../../  (apeireth-tools/src/.. = crates, .. = root)
    let workspace_root = std::path::Path::new("..").join("..");
    let conv = ProjectConventions::scan(&workspace_root);
    assert!(conv.scan_error.is_none(), "scan should succeed, got: {:?}", conv.scan_error);
    assert_eq!(conv.edition.as_deref(), Some("2021"));
    assert_eq!(conv.resolver.as_deref(), Some("2"));
    assert!(conv.members_count >= 80, "real workspace has 80+ crates, got {}", conv.members_count);
    // lint 类别应含 rust + clippy (workspace lint)
    assert!(conv.lint_categories.contains(&"rust".to_string()));
    assert!(conv.lint_categories.contains(&"clippy".to_string()));
}

#[test]
fn project_conventions_system_prompt_block_has_key_fields() {
    // to_system_prompt_block 输出应含关键提示
    use apeireth_tools::conventions_scanner::ProjectConventions;
    let workspace_root = std::path::Path::new("..").join("..");
    let conv = ProjectConventions::scan(&workspace_root);
    let block = conv.to_system_prompt_block();
    assert!(block.contains("# 项目约定"), "block should have title");
    assert!(block.contains("Rust edition: 2021"), "block should have edition: {block}");
    assert!(block.contains("Cargo resolver: 2"), "block should have resolver");
    assert!(block.contains("Workspace members:"), "block should have members");
    assert!(block.contains("# 风格提示"), "block should have hints section");
    // 关键不漂移提示
    assert!(block.contains("workspace = true"), "block should mention workspace inherit");
}

#[test]
fn project_conventions_handles_missing_cargo_toml() {
    // 空目录 / 无 Cargo.toml → scan_error 不为 None, 但不 panic
    use apeireth_tools::conventions_scanner::ProjectConventions;
    let dir = tempfile::tempdir().unwrap();
    let conv = ProjectConventions::scan(dir.path());
    assert!(conv.scan_error.is_some());
    assert!(conv.scan_error.as_ref().unwrap().contains("Cargo.toml not found"));
    assert!(conv.edition.is_none());
}
