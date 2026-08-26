//! VCP shell plugin compatibility layer.
//!
//! Maps VCP `LinuxShellExecutor` + `PowerShellExecutor` plugin manifest
//! command names to Rust types. Real execution routes through EnhancedShell.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShellCommand {
    /// VCP LinuxShellExecutor (113KB, 六层安全)
    LinuxShellExecutor,
    /// VCP PowerShellExecutor (3KB)
    PowerShellExecutor,
    /// VCP SciCalculator
    SciCalculator,
    Unknown,
}

pub const LEGACY_SHELL_COMMAND_COUNT: usize = 3;

impl ShellCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "LinuxShellExecutor" => Self::LinuxShellExecutor,
            "PowerShellExecutor" => Self::PowerShellExecutor,
            "SciCalculator" => Self::SciCalculator,
            _ => Self::Unknown,
        }
    }
}

pub struct ShellCompatRouter;

impl ShellCompatRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        LEGACY_SHELL_COMMAND_COUNT
    }
}

impl Default for ShellCompatRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_3_commands() {
        for s in ["LinuxShellExecutor", "PowerShellExecutor", "SciCalculator"] {
            assert_ne!(ShellCommand::from_str(s), ShellCommand::Unknown);
        }
        assert_eq!(LEGACY_SHELL_COMMAND_COUNT, 3);
    }

    #[test]
    fn unknown_maps_correctly() {
        assert_eq!(ShellCommand::from_str("Nonexistent"), ShellCommand::Unknown);
    }
}
