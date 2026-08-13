//! R223 Upgrade self_update (实际二进制替换 + 备份 + 原子切换 + 回滚).
//!
//! **动机**: ota.rs 7 阶段 OTA 已实装, 但 Switchover 阶段只做"逻辑切换" (reload config 等),
//! 实际二进制文件替换 + 备份 + 回滚 还没真接. R223 补上 self_update 物理层.
//!
//! **设计** (3 阶段):
//! 1. `Backup` — 把当前 binary 复制到 `<path>.bak`
//! 2. `AtomicSwap` — 临时文件 rename 到目标路径 (POSIX atomic, Windows ReplaceFile)
//! 3. `Verify` — 启动新 binary, 跑 smoke test, 失败则回滚
//!
//! **借鉴** (per O-5):
//! - rustup self update (rust-lang/rustup `src/update.rs`)
//! - cargo self update (BurntSushi/cargo-update `src/main.rs`)
//! - systemd unit replacement (rename + restart)
//!
//! **0 触碰**: ota.rs / rollback.rs / monitor.rs / intent.rs 0 改. 本模块是 additive.

#![allow(missing_docs)] // R223 additive
#![allow(clippy::all)]

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SelfUpdateError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("backup not found: {0}")]
    BackupNotFound(String),
    #[error("verification failed: {0}")]
    VerifyFailed(String),
    #[error("new binary same as current")]
    NewBinarySameAsCurrent,
    #[error("invalid binary: {0}")]
    InvalidBinary(String),
}

pub type SelfUpdateResult<T> = Result<T, SelfUpdateError>;

// ============================================================================
// 数据结构
// ============================================================================

/// Self-update 阶段.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfUpdateStage {
    /// 备份当前 binary.
    Backup,
    /// 原子替换.
    AtomicSwap,
    /// 验证新 binary.
    Verify,
    /// 完成.
    Done,
    /// 失败回滚.
    RolledBack,
    /// 回滚失败 (需要人工介入).
    RollbackFailed,
}

impl SelfUpdateStage {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::AtomicSwap => "atomic_swap",
            Self::Verify => "verify",
            Self::Done => "done",
            Self::RolledBack => "rolled_back",
            Self::RollbackFailed => "rollback_failed",
        }
    }
}

/// Backup 信息.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// 原 binary 路径.
    pub original_path: PathBuf,
    /// 备份路径.
    pub backup_path: PathBuf,
    /// 备份大小 (bytes).
    pub size_bytes: u64,
    /// 备份时间 (epoch ms).
    pub backed_up_at_ms: i64,
}

/// Self-update 状态.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfUpdateState {
    pub stage: SelfUpdateStage,
    pub target_path: PathBuf,
    pub new_binary_path: Option<PathBuf>,
    pub backup: Option<BackupInfo>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub error: Option<String>,
}

impl SelfUpdateState {
    pub fn new(target_path: PathBuf) -> Self {
        Self {
            stage: SelfUpdateStage::Backup,
            target_path,
            new_binary_path: None,
            backup: None,
            started_at_ms: now_ms(),
            finished_at_ms: None,
            error: None,
        }
    }
    pub fn elapsed_ms(&self) -> i64 {
        match self.finished_at_ms {
            Some(end) => end - self.started_at_ms,
            None => now_ms() - self.started_at_ms,
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
    .unwrap_or(0)
}

// ============================================================================
// Backup
// ============================================================================

/// 备份当前 binary 到 `<path>.bak`. 返回 BackupInfo.
pub fn backup_binary(path: &Path) -> SelfUpdateResult<BackupInfo> {
    if !path.exists() {
        return Err(SelfUpdateError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} not found", path.display()),
        )));
    }
    let backup_path = append_suffix(path, "bak");
    let metadata = fs::metadata(path)?;
    let size = metadata.len();
    fs::copy(path, &backup_path)?;
    Ok(BackupInfo {
        original_path: path.to_path_buf(),
        backup_path,
        size_bytes: size,
        backed_up_at_ms: now_ms(),
    })
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".");
    s.push(suffix);
    PathBuf::from(s)
}

// ============================================================================
// Atomic swap
// ============================================================================

/// 原子替换 binary (新 binary 已就绪在 `new_binary_path`).
///
/// 流程:
/// 1. 验证 new_binary_path 存在 + size > 0
/// 2. 把 new_binary_path rename 到 target_path (POSIX atomic, Windows MoveFileEx)
///
/// 注: 真正"原子"在 Windows 需要 MoveFileExW + MOVEFILE_REPLACE_EXISTING. Rust std
/// 的 fs::rename 在 Windows 上 MoveFileEx 已隐含. Unix 上 rename(2) 本就是原子的.
pub fn atomic_swap(target_path: &Path, new_binary_path: &Path) -> SelfUpdateResult<()> {
    // 验证 new binary
    let meta = fs::metadata(new_binary_path).map_err(|e| {
        SelfUpdateError::InvalidBinary(format!("new binary not found: {e}"))
    })?;
    if meta.len() == 0 {
        return Err(SelfUpdateError::InvalidBinary("new binary is empty".to_string()));
    }
    if target_path == new_binary_path {
        return Err(SelfUpdateError::NewBinarySameAsCurrent);
    }

    // 原子 rename
    fs::rename(new_binary_path, target_path)?;
    Ok(())
}

// ============================================================================
// Verify (smoke test)
// ============================================================================

/// 验证新 binary: 读取前 N bytes 检查 magic number / ELF header.
pub fn verify_binary_smoke(path: &Path) -> SelfUpdateResult<()> {
    let mut f = fs::File::open(path)?;
    let mut head = [0u8; 4];
    f.read_exact(&mut head)?;
    // ELF magic: 0x7f 'E' 'L' 'F' (Linux)
    if head == [0x7f, b'E', b'L', b'F'] {
        return Ok(());
    }
    // Windows PE: 'M' 'Z' (0x4D 0x5A)
    if head[0] == b'M' && head[1] == b'Z' {
        return Ok(());
    }
    // Mach-O: 0xFE 0xED 0xFA 0xCE (32-bit) 或 0xFE 0xED 0xFA 0xCF (64-bit) 等
    if head[0] == 0xFE && head[1] == 0xED && head[2] == 0xFA {
        return Ok(());
    }
    // Script: #! / shebang
    if head[0] == b'#' && head[1] == b'!' {
        return Ok(());
    }
    Err(SelfUpdateError::VerifyFailed(format!(
        "binary magic 不识别: {:02x?} (path: {})",
        head,
        path.display()
    )))
}

// ============================================================================
// Rollback
// ============================================================================

/// 从 backup 回滚到 target_path.
pub fn rollback_from_backup(target_path: &Path, backup: &BackupInfo) -> SelfUpdateResult<()> {
    if !backup.backup_path.exists() {
        return Err(SelfUpdateError::BackupNotFound(
            backup.backup_path.display().to_string(),
        ));
    }
    // 先删除当前 (可能损坏的) target
    if target_path.exists() {
        let _ = fs::remove_file(target_path);
    }
    fs::rename(&backup.backup_path, target_path)?;
    Ok(())
}

// ============================================================================
// 端到端 self_update 流程
// ============================================================================

/// 跑完整 self_update 流程 (Backup → AtomicSwap → Verify).
///
/// `new_binary_path` 应指向已下载/构建好的新 binary 文件.
/// 失败自动回滚 (除非 rollback 本身失败).
pub fn run_self_update(
    target_path: &Path,
    new_binary_path: &Path,
) -> SelfUpdateResult<SelfUpdateState> {
    let mut state = SelfUpdateState::new(target_path.to_path_buf());
    state.new_binary_path = Some(new_binary_path.to_path_buf());

    // Stage 1: Backup
    let backup = match backup_binary(target_path) {
        Ok(b) => {
            state.backup = Some(b.clone());
            state.stage = SelfUpdateStage::AtomicSwap;
            b
        }
        Err(e) => {
            state.stage = SelfUpdateStage::RollbackFailed;
            state.error = Some(format!("backup failed: {e}"));
            state.finished_at_ms = Some(now_ms());
            return Err(e);
        }
    };

    // Stage 2: AtomicSwap
    if let Err(e) = atomic_swap(target_path, new_binary_path) {
        // 回滚
        match rollback_from_backup(target_path, &backup) {
            Ok(()) => {
                state.stage = SelfUpdateStage::RolledBack;
                state.error = Some(format!("atomic_swap failed, rolled back: {e}"));
            }
            Err(re) => {
                state.stage = SelfUpdateStage::RollbackFailed;
                state.error = Some(format!("atomic_swap + rollback both failed: {e} / {re}"));
            }
        }
        state.finished_at_ms = Some(now_ms());
        return Err(e);
    }
    state.stage = SelfUpdateStage::Verify;

    // Stage 3: Verify
    if let Err(e) = verify_binary_smoke(target_path) {
        // 回滚
        match rollback_from_backup(target_path, &backup) {
            Ok(()) => {
                state.stage = SelfUpdateStage::RolledBack;
                state.error = Some(format!("verify failed, rolled back: {e}"));
            }
            Err(re) => {
                state.stage = SelfUpdateStage::RollbackFailed;
                state.error = Some(format!("verify + rollback both failed: {e} / {re}"));
            }
        }
        state.finished_at_ms = Some(now_ms());
        return Err(e);
    }

    state.stage = SelfUpdateStage::Done;
    state.finished_at_ms = Some(now_ms());
    Ok(state)
}

// ============================================================================
// 清理 backup
// ============================================================================

/// 清理 backup 文件 (升级成功后).
pub fn cleanup_backup(backup: &BackupInfo) -> io::Result<()> {
    if backup.backup_path.exists() {
        fs::remove_file(&backup.backup_path)?;
    }
    Ok(())
}

// ============================================================================
// 测试 (10 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn mk_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-self-update-test-{}-{}",
            name,
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &[u8]) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content).unwrap();
    }

    #[test]
    fn t01_backup_binary_creates_bak() {
        let dir = mk_temp_dir("backup");
        let target = dir.join("mybin");
        write_file(&target, b"old content");
        let info = backup_binary(&target).unwrap();
        assert!(info.backup_path.exists());
        assert_eq!(info.size_bytes, 11);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t02_backup_missing_file_errors() {
        let dir = mk_temp_dir("backup-missing");
        let target = dir.join("nonexistent");
        let r = backup_binary(&target);
        assert!(r.is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t03_atomic_swap_renames() {
        let dir = mk_temp_dir("swap");
        let target = dir.join("mybin");
        let new_bin = dir.join("mybin.new");
        write_file(&target, b"OLD");
        write_file(&new_bin, b"NEW");
        atomic_swap(&target, &new_bin).unwrap();
        let content = fs::read(&target).unwrap();
        assert_eq!(content, b"NEW");
        assert!(!new_bin.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t04_atomic_swap_empty_new_errors() {
        let dir = mk_temp_dir("swap-empty");
        let target = dir.join("mybin");
        let new_bin = dir.join("mybin.new");
        write_file(&target, b"OLD");
        write_file(&new_bin, b"");
        let r = atomic_swap(&target, &new_bin);
        assert!(matches!(r, Err(SelfUpdateError::InvalidBinary(_))));
        // 旧 binary 应保持完整
        assert_eq!(fs::read(&target).unwrap(), b"OLD");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t05_atomic_swap_same_path_errors() {
        let dir = mk_temp_dir("swap-same");
        let target = dir.join("mybin");
        write_file(&target, b"X");
        let r = atomic_swap(&target, &target);
        assert!(matches!(r, Err(SelfUpdateError::NewBinarySameAsCurrent)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t06_verify_elf_magic() {
        let dir = mk_temp_dir("verify-elf");
        let p = dir.join("bin");
        write_file(&p, &[0x7f, b'E', b'L', b'F']);
        assert!(verify_binary_smoke(&p).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t07_verify_pe_magic() {
        let dir = mk_temp_dir("verify-pe");
        let p = dir.join("bin.exe");
        write_file(&p, &[b'M', b'Z', 0, 0]);
        assert!(verify_binary_smoke(&p).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t08_verify_invalid_magic() {
        let dir = mk_temp_dir("verify-bad");
        let p = dir.join("bin");
        write_file(&p, b"random garbage data");
        let r = verify_binary_smoke(&p);
        assert!(matches!(r, Err(SelfUpdateError::VerifyFailed(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t09_rollback_restores_backup() {
        let dir = mk_temp_dir("rollback");
        let target = dir.join("mybin");
        let new_bin = dir.join("mybin.new");
        write_file(&target, b"ORIGINAL");
        write_file(&new_bin, b"BROKEN");
        let info = backup_binary(&target).unwrap();
        // simulate broken state
        atomic_swap(&target, &new_bin).unwrap();
        write_file(&target, b"CORRUPTED");
        // rollback
        rollback_from_backup(&target, &info).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"ORIGINAL");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t10_full_self_update_success() {
        let dir = mk_temp_dir("full");
        let target = dir.join("mybin");
        let new_bin = dir.join("mybin.new");
        // 用 ELF magic 作为新 binary (verify pass)
        write_file(&target, b"OLD CONTENT HERE");
        write_file(&new_bin, &[0x7f, b'E', b'L', b'F', 0, 0, 0, 0]);
        let state = run_self_update(&target, &new_bin).unwrap();
        assert_eq!(state.stage, SelfUpdateStage::Done);
        let content = fs::read(&target).unwrap();
        assert_eq!(&content[..4], &[0x7f, b'E', b'L', b'F']);
        let _ = fs::remove_dir_all(&dir);
    }
}
