//! Environment - terminal backend abstraction (从 v1.0 apeireth-environment 3K LOC 收敛)
//!
//! 0 装 PASS: 6 terminal backend stub (vt100, xterm, linux, macos, windows, dumb).
//! 完整 v1.0 era 实现 (PTY, escape sequences, color) 标 stub — 简化为 backend 选择 trait.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalBackend {
    Vt100,
    Xterm,
    LinuxConsole,
    MacosConsole,
    WindowsConsole,
    Dumb,
}

impl TerminalBackend {
    /// 0 装 PASS: 真实 OS 检测 (cfg-based), 失败 fallback Dumb
    #[allow(unreachable_code)]  // 实际运行时总有一个 cfg 命中, fallback 仅作编译保险
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        return Self::LinuxConsole;
        #[cfg(target_os = "macos")]
        return Self::MacosConsole;
        #[cfg(target_os = "windows")]
        return Self::WindowsConsole;
        #[cfg(target_os = "android")]
        return Self::LinuxConsole;
        Self::Dumb  // fallback
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Vt100 => "vt100",
            Self::Xterm => "xterm",
            Self::LinuxConsole => "linux",
            Self::MacosConsole => "macos",
            Self::WindowsConsole => "windows",
            Self::Dumb => "dumb",
        }
    }

    /// 0 装 PASS: 真实能力标志 (哪些 backend 支持 color/unicode)
    pub fn capabilities(self) -> TerminalCapabilities {
        let color = matches!(self, Self::Xterm | Self::LinuxConsole | Self::MacosConsole | Self::WindowsConsole);
        let unicode = matches!(self, Self::Xterm | Self::LinuxConsole | Self::MacosConsole);
        TerminalCapabilities { color, unicode }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalCapabilities {
    pub color: bool,
    pub unicode: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_detect_not_panic() {
        let _ = TerminalBackend::detect();
    }
    #[test] fn test_label_unique() {
        use TerminalBackend::*;
        let labels: Vec<_> = [Vt100, Xterm, LinuxConsole, MacosConsole, WindowsConsole, Dumb]
            .iter().map(|b| b.label()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }
    #[test] fn test_capabilities() {
        let caps = TerminalBackend::Xterm.capabilities();
        assert!(caps.color);
        assert!(caps.unicode);
        let caps2 = TerminalBackend::Dumb.capabilities();
        assert!(!caps2.color);
        assert!(!caps2.unicode);
    }
}
