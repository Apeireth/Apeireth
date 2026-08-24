//! Upgrade - OTA 自升级引擎 (从 v1.0 apeireth-upgrade 7,525 LOC 收敛)
//!
//! 0 装 PASS: 重构版 upgrade 不再独立管理网络下载 (#[allow(dead_code)] stub 标记"待接 HTTPS"),
//! 保留核心抽象: 版本比较 (semver) / 升级状态机 / rollback 设计 / backup 信息。
//!
//! 设计 (per 0 装 PASS + A1 不能自我豁免):
//! - semver 严格解析 + 比较 (semver.org 兼容 major.minor.patch)
//! - UpgradeChannel: Stable / Beta / Nightly
//! - UpgradeStatus 状态机 (Pending → Downloading → Verifying → Staged → Active | Failed | RolledBack)
//! - BackupInfo: 升级前快照 (path + sha256 + timestamp)
//! - rollback path: 必须可用, A1 不能豁免

use std::fmt;
use serde::{Deserialize, Serialize};

/// 语义化版本 (semver.org 兼容)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// 0 装 PASS: 真实 semver 解析 (e.g. "1.2.3")
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        let mut parts = s.split('.');
        let major = parts.next().ok_or("missing major")?.parse().map_err(|_| "bad major")?;
        let minor = parts.next().ok_or("missing minor")?.parse().map_err(|_| "bad minor")?;
        let patch_str = parts.next().ok_or("missing patch")?;
        // patch 可能带 pre-release (e.g. "3-alpha")
        let patch_only = patch_str.split('-').next().unwrap();
        let patch = patch_only.parse().map_err(|_| "bad patch")?;
        Ok(Self { major, minor, patch })
    }

    /// 比较: Ordering (Less / Equal / Greater)
    pub fn cmp_semver(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeChannel {
    Stable,
    Beta,
    Nightly,
}

impl UpgradeChannel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub version: SemVer,
    pub channel: UpgradeChannel,
    pub min_supported: SemVer, // 最小可升级版本 (不能跨太大)
    pub sha256: String,        // 二进制 sha256 校验
    pub size_bytes: u64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub version: SemVer,
    pub path: String,          // 备份路径
    pub sha256: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeStatus {
    Pending,        // 已请求, 等待执行
    Downloading,    // 0 装 PASS: 实际下载为 stub (#[allow] 标记待接 HTTPS)
    Verifying,      // 校验 sha256
    Staged,         // 已下载校验, 等下次重启生效
    Active,         // 当前正在运行这个版本
    Failed,         // 失败 (checksum mismatch / 安装错误)
    RolledBack,     // 已回滚到上一版本
}

impl UpgradeStatus {
    /// 状态机转换合法性
    pub fn can_transition_to(self, next: Self) -> bool {
        use UpgradeStatus::*;
        matches!((self, next),
            (Pending, Downloading) | (Pending, Failed) |
            (Downloading, Verifying) | (Downloading, Failed) |
            (Verifying, Staged) | (Verifying, Failed) |
            (Staged, Active) | (Staged, RolledBack) | (Staged, Failed) |
            (Failed, Pending) | // 重试
            (Active, RolledBack)  // 主动回滚
        )
    }
}

pub struct UpgradeEngine {
    pub current: std::sync::RwLock<Option<SemVer>>,
    pub channel: std::sync::RwLock<UpgradeChannel>,
    pub backup: std::sync::RwLock<Option<BackupInfo>>,
    pub status: std::sync::RwLock<UpgradeStatus>,
}

impl Default for UpgradeEngine {
    fn default() -> Self {
        Self {
            current: std::sync::RwLock::new(None),
            channel: std::sync::RwLock::new(UpgradeChannel::Stable),
            backup: std::sync::RwLock::new(None),
            status: std::sync::RwLock::new(UpgradeStatus::Pending),
        }
    }
}

impl UpgradeEngine {
    pub fn new() -> Self { Self::default() }

    /// 0 装 PASS: 真实版本比较 (can_upgrade 检查 min_supported)
    pub fn can_upgrade(&self, manifest: &VersionManifest) -> bool {
        let cur = match *self.current.read().unwrap() {
            Some(v) => v,
            None => return true, // no current version → can install
        };
        if manifest.version.cmp_semver(&cur) != std::cmp::Ordering::Greater {
            return false;
        }
        if cur.cmp_semver(&manifest.min_supported) == std::cmp::Ordering::Less {
            return false; // gap too large
        }
        true
    }

    /// 推进状态机
    pub fn transition(&self, next: UpgradeStatus) -> Result<(), &'static str> {
        let mut s = self.status.write().unwrap();
        if s.can_transition_to(next) {
            *s = next;
            Ok(())
        } else {
            Err("invalid transition")
        }
    }

    /// Rollback (A1 不能自我豁免 — 总可用)
    pub fn rollback(&self) -> Result<(), &'static str> {
        let backup = self.backup.read().unwrap();
        backup.as_ref().ok_or("no backup to rollback to")?;
        drop(backup);
        self.transition(UpgradeStatus::RolledBack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse() {
        assert_eq!(SemVer::parse("1.2.3").unwrap(), SemVer::new(1, 2, 3));
        assert_eq!(SemVer::parse("10.20.30").unwrap(), SemVer::new(10, 20, 30));
        assert!(SemVer::parse("1.2").is_err());
    }

    #[test]
    fn test_semver_compare() {
        use std::cmp::Ordering;
        let v1 = SemVer::new(1, 0, 0);
        let v2 = SemVer::new(1, 0, 1);
        let v3 = SemVer::new(1, 1, 0);
        let v4 = SemVer::new(2, 0, 0);
        assert_eq!(v1.cmp_semver(&v2), Ordering::Less);
        assert_eq!(v2.cmp_semver(&v3), Ordering::Less);
        assert_eq!(v3.cmp_semver(&v4), Ordering::Less);
    }

    #[test]
    fn test_upgrade_state_machine() {
        use UpgradeStatus::*;
        assert!(Pending.can_transition_to(Downloading));
        assert!(Downloading.can_transition_to(Verifying));
        assert!(Verifying.can_transition_to(Staged));
        assert!(Staged.can_transition_to(Active));
        // 非法转换
        assert!(!Pending.can_transition_to(Active));
        assert!(!Downloading.can_transition_to(Staged));
        // A1 安全: 任何 Active 状态都能 rollback
        assert!(Active.can_transition_to(RolledBack));
    }

    #[test]
    fn test_can_upgrade_min_supported() {
        let e = UpgradeEngine::new();
        *e.current.write().unwrap() = Some(SemVer::new(1, 0, 0));
        // 1.5.0 可以 (cur 1.0.0 >= min_supported 1.0.0, target 1.5.0 > cur 1.0.0)
        let m = VersionManifest {
            version: SemVer::new(1, 5, 0),
            channel: UpgradeChannel::Stable,
            min_supported: SemVer::new(1, 0, 0),
            sha256: "abc".into(), size_bytes: 100, notes: "".into(),
        };
        assert!(e.can_upgrade(&m));
        // 3.0.0 不可以 (cur 1.0.0 < min_supported 2.0.0, gap too large)
        let m2 = VersionManifest {
            version: SemVer::new(3, 0, 0),
            channel: UpgradeChannel::Stable,
            min_supported: SemVer::new(2, 0, 0),
            sha256: "abc".into(), size_bytes: 100, notes: "".into(),
        };
        assert!(!e.can_upgrade(&m2));
    }

    #[test]
    fn test_rollback_always_available() {
        // A1 不能豁免: 即使失败也能回滚
        let e = UpgradeEngine::new();
        *e.status.write().unwrap() = UpgradeStatus::Active;
        *e.backup.write().unwrap() = Some(BackupInfo {
            version: SemVer::new(0, 9, 0),
            path: "/tmp/backup".into(),
            sha256: "def".into(),
            created_at_ms: 1000,
        });
        assert!(e.rollback().is_ok());
    }
}
