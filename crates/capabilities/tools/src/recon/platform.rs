//! 工具平台元数据：宿主支持、目标类型、provider 与成熟度。
//!
//! 注册表按宿主过滤：不可用的工具返回 [`PlatformError::Unsupported`]，
//! 明确告知而不是静默失败（对齐 0 装 PASS）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 宿主操作系统。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostOs {
    /// Linux。
    Linux,
    /// Windows。
    Windows,
    /// macOS。
    Macos,
}

/// 当前进程所在的宿主系统。
pub const fn current_host() -> HostOs {
    if cfg!(target_os = "windows") {
        HostOs::Windows
    } else if cfg!(target_os = "macos") {
        HostOs::Macos
    } else {
        HostOs::Linux
    }
}

/// 一个工具的平台能力描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPlatform {
    /// 支持的宿主系统。
    pub host: Vec<HostOs>,
    /// 可分析的目标类型（`elf` / `pe` / `macho` / `apk` / `pcap` …）。
    pub target: Vec<String>,
    /// 依赖的 provider（`native` / `ghidra` / `adb` / `frida` …）。
    pub provider: String,
    /// 按宿主标注成熟度（如 `windows: experimental`）。
    #[serde(default)]
    pub maturity: BTreeMap<String, String>,
}

/// 平台校验失败。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlatformError {
    /// 当前宿主不支持该工具。
    #[error("tool is not supported on host {host:?}")]
    Unsupported {
        /// 当前宿主。
        host: HostOs,
    },
}

impl ToolPlatform {
    /// 构造最小平台描述。
    pub fn new(host: Vec<HostOs>, target: Vec<String>, provider: impl Into<String>) -> Self {
        Self {
            host,
            target,
            provider: provider.into(),
            maturity: BTreeMap::new(),
        }
    }

    /// 标注某宿主的成熟度（builder 风格）。
    #[must_use]
    pub fn with_maturity(mut self, host: HostOs, level: impl Into<String>) -> Self {
        self.maturity
            .insert(format!("{host:?}").to_ascii_lowercase(), level.into());
        self
    }

    /// 是否支持给定宿主。
    pub fn supports_host(&self, host: HostOs) -> bool {
        self.host.contains(&host)
    }

    /// 要求当前宿主受支持，否则返回显式错误。
    pub fn require_host(&self, host: HostOs) -> Result<(), PlatformError> {
        if self.supports_host(host) {
            Ok(())
        } else {
            Err(PlatformError::Unsupported { host })
        }
    }

    /// 某宿主上的成熟度标注（无标注返回 `None`）。
    pub fn maturity_of(&self, host: HostOs) -> Option<&str> {
        self.maturity
            .get(&format!("{host:?}").to_ascii_lowercase())
            .map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_host_matches_build_target() {
        let expected = if cfg!(target_os = "windows") {
            HostOs::Windows
        } else if cfg!(target_os = "macos") {
            HostOs::Macos
        } else {
            HostOs::Linux
        };
        assert_eq!(current_host(), expected);
    }

    #[test]
    fn host_support_is_explicit_and_fail_closed() {
        let platform = ToolPlatform::new(
            vec![HostOs::Linux, HostOs::Macos],
            vec!["elf".into(), "macho".into()],
            "ghidra",
        )
        .with_maturity(HostOs::Macos, "stable");

        assert!(platform.supports_host(HostOs::Linux));
        assert!(!platform.supports_host(HostOs::Windows));
        assert_eq!(
            platform.require_host(HostOs::Windows),
            Err(PlatformError::Unsupported {
                host: HostOs::Windows
            })
        );
        assert_eq!(platform.maturity_of(HostOs::Macos), Some("stable"));
        assert_eq!(platform.maturity_of(HostOs::Linux), None);
    }
}
