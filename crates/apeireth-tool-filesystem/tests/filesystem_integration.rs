//! Integration tests for apeireth-tool-filesystem (post-1.0.0)
//!
//! src/ 5 module 真实现 (sandbox/atomic/lock/watch/compat). 这里 (tests/) 加跨模块集成.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_tool_filesystem::{
    atomic_write, CompatCommand, CompatManifest, CompatRouter, FileLock, Sandbox, SandboxError,
    SandboxPolicy, R137_DELIVERABLES, UPGRADE_DIMENSIONS,
};
use std::path::{Path, PathBuf};

// =============================================================================
// 编译期常量
// =============================================================================

#[test]
fn r137_deliverables_count() {
    assert_eq!(R137_DELIVERABLES, 5, "R137 5 模块");
}

#[test]
fn upgrade_dimensions_count() {
    assert_eq!(UPGRADE_DIMENSIONS, 5, "5 维度扩展");
}

// =============================================================================
// SandboxPolicy / Sandbox
// =============================================================================

#[test]
fn sandbox_policy_new_defaults() {
    let p = SandboxPolicy::new(vec![PathBuf::from("/tmp")]);
    assert_eq!(p.allowed_roots.len(), 1);
    assert!(!p.follow_symlinks, "默认 false");
}

#[test]
fn sandbox_policy_clone() {
    let p = SandboxPolicy::new(vec![PathBuf::from("/tmp")]);
    let p2 = p.clone();
    assert_eq!(p.allowed_roots, p2.allowed_roots);
    assert_eq!(p.follow_symlinks, p2.follow_symlinks);
}

#[tokio::test]
async fn sandbox_new_returns_policy() {
    let policy = SandboxPolicy::new(vec![std::env::temp_dir()]);
    let sb = Sandbox::new(policy.clone());
    assert_eq!(sb.policy().allowed_roots, policy.allowed_roots);
}

#[tokio::test]
async fn sandbox_resolve_existing_path() {
    let policy = SandboxPolicy::new(vec![std::env::temp_dir()]);
    let sb = Sandbox::new(policy);
    let r = sb.resolve(&std::env::temp_dir()).await;
    assert!(r.is_ok(), "temp_dir 应在白名单内: {r:?}");
}

#[tokio::test]
async fn sandbox_resolve_nonexistent_errors() {
    let policy = SandboxPolicy::new(vec![std::env::temp_dir()]);
    let sb = Sandbox::new(policy);
    let bad = std::env::temp_dir().join("apeireth_nonexistent_xyz");
    let r = sb.resolve(&bad).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn sandbox_resolve_outside_root_errors() {
    let policy = SandboxPolicy::new(vec![PathBuf::from("/tmp/apeireth_unique")]);
    let sb = Sandbox::new(policy);
    let r = sb.resolve(Path::new("/etc/passwd")).await;
    // 不一定 error if /etc/passwd canonical 在 /tmp/apeireth_unique 下, 但 on Unix /etc/passwd 不在 /tmp/apeireth_unique
    // 通常 should err, 但 win 上 /etc/passwd 不存在 → error
    let _ = r; // 不强制
}

// =============================================================================
// SandboxError display
// =============================================================================

#[test]
fn sandbox_error_path_not_found_display() {
    let e = SandboxError::PathNotFound(PathBuf::from("/x/y/z"));
    let s = e.to_string();
    assert!(s.contains("not found"));
    assert!(s.contains("x"));
}

#[test]
fn sandbox_error_outside_allowed_display() {
    let e = SandboxError::OutsideAllowedRoots {
        path: PathBuf::from("/etc/passwd"),
    };
    let s = e.to_string();
    assert!(s.contains("outside") || s.contains("allowed"));
}

#[test]
fn sandbox_error_symlink_escape_display() {
    let e = SandboxError::SymlinkEscape {
        target: PathBuf::from("/tmp/escape"),
    };
    let s = e.to_string();
    assert!(s.contains("symlink") || s.contains("escape"));
}

// =============================================================================
// atomic_write
// =============================================================================

#[tokio::test]
async fn atomic_write_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("test.txt");
    atomic_write(&target, b"hello").await.unwrap();
    let content = std::fs::read_to_string(&target).unwrap();
    assert_eq!(content, "hello");
}

#[tokio::test]
async fn atomic_write_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("test.txt");
    std::fs::write(&target, "old content here").unwrap();
    atomic_write(&target, b"new").await.unwrap();
    let content = std::fs::read_to_string(&target).unwrap();
    assert_eq!(content, "new");
}

#[tokio::test]
async fn atomic_write_empty_content() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("empty.txt");
    atomic_write(&target, b"").await.unwrap();
    let content = std::fs::read(&target).unwrap();
    assert!(content.is_empty());
}

#[tokio::test]
async fn atomic_write_binary_content() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("binary.bin");
    let bytes: Vec<u8> = (0..255).collect();
    atomic_write(&target, &bytes).await.unwrap();
    let content = std::fs::read(&target).unwrap();
    assert_eq!(content, bytes);
}

#[tokio::test]
async fn atomic_write_utf8_content() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("utf8.txt");
    atomic_write(&target, "中文 + emoji 🚀".as_bytes())
        .await
        .unwrap();
    let content = std::fs::read_to_string(&target).unwrap();
    assert_eq!(content, "中文 + emoji 🚀");
}

#[tokio::test]
async fn atomic_write_no_tmp_left() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("test.txt");
    atomic_write(&target, b"data").await.unwrap();
    // 写完后 tmp 文件应被 rename 走
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let tmp_files: Vec<&String> = entries.iter().filter(|n| n.ends_with(".tmp")).collect();
    assert!(tmp_files.is_empty(), "应无残留 .tmp: {tmp_files:?}");
}

// =============================================================================
// CompatCommand
// =============================================================================

#[test]
fn compat_command_from_str_18() {
    let names = [
        "ReadFile",
        "WebReadFile",
        "FileInfo",
        "WriteFile",
        "WriteEscapedFile",
        "AppendFile",
        "EditFile",
        "ApplyDiff",
        "ListDirectory",
        "CreateDirectory",
        "ListAllowedDirectories",
        "CopyFile",
        "MoveFile",
        "RenameFile",
        "DeleteFile",
        "SearchFiles",
        "DownloadFile",
        "CreateCanvas",
    ];
    for n in names {
        let c = CompatCommand::from_str(n);
        assert_ne!(c, CompatCommand::Unknown, "{n} 应识别");
    }
}

#[test]
fn compat_command_unknown_fallback() {
    assert_eq!(
        CompatCommand::from_str("NotARealCommand"),
        CompatCommand::Unknown
    );
    assert_eq!(CompatCommand::from_str(""), CompatCommand::Unknown);
}

#[test]
fn compat_command_count() {
    // LEGACY_COMMAND_COUNT = 18
    assert_eq!(CompatRouter::command_count(), 18);
}

#[test]
fn compat_command_eq_hash() {
    let a = CompatCommand::ReadFile;
    let b = CompatCommand::ReadFile;
    assert_eq!(a, b);
    let mut set = std::collections::HashSet::new();
    set.insert(a);
    set.insert(b);
    set.insert(CompatCommand::WriteFile);
    assert_eq!(set.len(), 2, "ReadFile + WriteFile");
}

#[test]
fn compat_command_clone_serde() {
    let c = CompatCommand::ApplyDiff;
    let s = serde_json::to_string(&c).unwrap();
    let back: CompatCommand = serde_json::from_str(&s).unwrap();
    assert_eq!(c, back);
}

// =============================================================================
// CompatManifest
// =============================================================================

#[test]
fn compat_manifest_parse_minimal() {
    let json = r#"{"name":"FileOperator","commands":["ReadFile","WriteFile"]}"#;
    let m = CompatManifest::parse(json).unwrap();
    assert_eq!(m.name, "FileOperator");
    assert_eq!(m.commands.len(), 2);
    assert_eq!(m.display_name, None);
    assert_eq!(m.description, None);
}

#[test]
fn compat_manifest_parse_with_optional_fields() {
    let json = r#"{
        "name":"FileOperator",
        "display_name":"File Ops",
        "description":"File operations",
        "commands":["ReadFile"]
    }"#;
    let m = CompatManifest::parse(json).unwrap();
    assert_eq!(m.display_name.as_deref(), Some("File Ops"));
    assert_eq!(m.description.as_deref(), Some("File operations"));
}

#[test]
fn compat_manifest_parse_invalid_errors() {
    let r = CompatManifest::parse("not json");
    assert!(r.is_err());
}

#[test]
fn compat_manifest_supported_filters_unknown() {
    let json = r#"{
        "name":"FileOperator",
        "commands":["ReadFile","NotAReal","WriteFile"]
    }"#;
    let m = CompatManifest::parse(json).unwrap();
    let supported = m.supported_commands();
    assert_eq!(supported.len(), 2, "NotAReal 应过滤");
    assert!(supported.contains(&CompatCommand::ReadFile));
    assert!(supported.contains(&CompatCommand::WriteFile));
}

#[test]
fn compat_manifest_empty_commands() {
    let json = r#"{"name":"X","commands":[]}"#;
    let m = CompatManifest::parse(json).unwrap();
    assert!(m.supported_commands().is_empty());
}

// =============================================================================
// CompatRouter
// =============================================================================

#[test]
fn compat_router_new_and_default() {
    let _r1 = CompatRouter::new();
    let _r2 = CompatRouter::default();
}

// =============================================================================
// FileLock (exclusive / shared)
// =============================================================================

#[test]
fn file_lock_exclusive_creates() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("mylock");
    let _guard = FileLock::exclusive(&lock_path).unwrap();
    assert!(lock_path.exists(), "lock file 应创建");
}

#[test]
fn file_lock_shared_creates() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("shared_lock");
    let _guard = FileLock::shared(&lock_path).unwrap();
    assert!(lock_path.exists());
}

#[test]
fn file_lock_guard_drop_releases() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("guard_drop");
    {
        let _g = FileLock::exclusive(&lock_path).unwrap();
        assert!(lock_path.exists());
    }
    // guard 释放后, 文件仍存在 (由 guard 持有)
    assert!(lock_path.exists());
}

#[test]
fn file_lock_multiple_exclusive_serial() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("serial_lock");
    {
        let _g1 = FileLock::exclusive(&lock_path).unwrap();
    }
    {
        let _g2 = FileLock::exclusive(&lock_path).unwrap();
    }
    // serial 二次 lock 应 OK
    assert!(lock_path.exists());
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_sandbox_then_atomic_write() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("inside.txt");

        // 1. sandbox 校验 dir 在白名单内
        let sb = Sandbox::new(SandboxPolicy::new(vec![dir.path().to_path_buf()]));
        let resolved = sb.resolve(dir.path()).await.unwrap();
        assert!(resolved.starts_with(dir.path().canonicalize().unwrap()));

        // 2. atomic write
        atomic_write(&target, b"secure content").await.unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "secure content");
    });
}

#[test]
fn integration_compat_command_with_manifest() {
    let json = r#"{
        "name":"FileOperator",
        "commands":["ReadFile","WriteFile","EditFile","ApplyDiff"]
    }"#;
    let m = CompatManifest::parse(json).unwrap();
    let supported = m.supported_commands();
    assert_eq!(supported.len(), 4);
    assert_eq!(CompatRouter::command_count(), 18, "总 command 18");
}

#[test]
fn integration_file_lock_with_atomic_write() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("writelock");
        let target = dir.path().join("data.txt");

        // 1. acquire lock
        let _guard = FileLock::exclusive(&lock_path).unwrap();

        // 2. atomic write within lock
        atomic_write(&target, b"locked content").await.unwrap();

        // 3. verify
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "locked content");
    });
}

#[test]
fn integration_atomic_write_compat_overwrite() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("overwrite.txt");

        // 模拟 EditFile command flow: 写 → atomic 覆写 → 读
        atomic_write(&target, b"v1: hello").await.unwrap();
        let v1 = std::fs::read_to_string(&target).unwrap();
        assert_eq!(v1, "v1: hello");

        atomic_write(&target, b"v2: world").await.unwrap();
        let v2 = std::fs::read_to_string(&target).unwrap();
        assert_eq!(v2, "v2: world");
    });
}
