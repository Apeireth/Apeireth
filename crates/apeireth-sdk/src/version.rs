//! 版本协商: `S X.Y.Z` + major/minor patch 段 + 比较.

use std::fmt;

use serde::{Deserialize, Serialize};

/// SDK 协议版本 (semver 简版: major.minor.patch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SdkVersion {
    /// 不兼容变更 (breaking)
    pub major: u16,
    /// 新功能 (backward-compatible)
    pub minor: u16,
    /// bug fix (backward-compatible)
    pub patch: u16,
}

impl SdkVersion {
    /// 构造
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// 从 `MAJOR.MINOR.PATCH` 字符串解析 (parse 失败返回 None).
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// 字符串化 (无前缀, 与 [parse] 对应).
    pub fn as_str(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// 鸽笼排序: 主版本号不同即不等.
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }

    /// 排序: major > minor > patch (返回 Ordering).
    /// 命名 compare_versions 避免与 std::cmp::Ord::cmp 混淆 (clippy should_implement_trait).
    pub fn compare_versions(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl fmt::Display for SdkVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_str())
    }
}

impl std::str::FromStr for SdkVersion {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
}

/// 兼容性协商结果.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireCompat {
    /// 完全相等 (三段都同)
    Exact,
    /// 同 major, server 较新 (server 兼容 client)
    ServerNewer,
    /// 同 major, server 较旧 (client 仍能跑, 部分新字段可能缺失)
    ServerOlder,
    /// major 不同, 不可互通
    Incompatible,
}

/// 协商结果: 给定 client 与 server 版本, 返回 [WireCompat].
pub fn negotiate(client: SdkVersion, server: SdkVersion) -> WireCompat {
    if client.major != server.major {
        return WireCompat::Incompatible;
    }
    match client.compare_versions(&server) {
        std::cmp::Ordering::Equal => WireCompat::Exact,
        std::cmp::Ordering::Less => WireCompat::ServerNewer,
        std::cmp::Ordering::Greater => WireCompat::ServerOlder,
    }
}

/// SDK 自身声称的版本 (编译期常量).
pub const SDK_VERSION: SdkVersion = SdkVersion::new(0, 1, 0);
