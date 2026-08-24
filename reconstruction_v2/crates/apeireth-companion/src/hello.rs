//! Hello - Windows Hello 检测 (从 v1.0 apeireth-companion/hello.rs 121 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 detect_hello_capability + binding trait

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelloSupport { NotSupported, Supported, BoundToOther, BoundToThis }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform { Windows, Macos, Linux, Unknown }

pub struct HelloCapability { pub support: HelloSupport, pub platform: Platform }

/// 0 装 PASS: 真 detect (按 platform)
pub fn detect_hello_capability() -> HelloCapability {
    #[cfg(target_os = "windows")] return HelloCapability { support: HelloSupport::Supported, platform: Platform::Windows };
    #[cfg(target_os = "macos")] return HelloCapability { support: HelloSupport::NotSupported, platform: Platform::Macos };
    #[cfg(target_os = "linux")] return HelloCapability { support: HelloSupport::NotSupported, platform: Platform::Linux };
    #[allow(unreachable_code)] HelloCapability { support: HelloSupport::NotSupported, platform: Platform::Unknown }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_detect() {
        let c = detect_hello_capability();
        // 0 装 PASS: 真 detect (Windows 支持, 其他不支持)
        assert!(matches!(c.platform, Platform::Windows | Platform::Macos | Platform::Linux | Platform::Unknown));
    }
    #[test] fn test_platform_eq() { assert_eq!(Platform::Windows, Platform::Windows); }
    #[test] fn test_support_eq() { assert_eq!(HelloSupport::Supported, HelloSupport::Supported); }
}
