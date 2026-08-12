//! VCP shell plugin compatibility layer.
//!
//! Maps VCP `LinuxShellExecutor` + `PowerShellExecutor` plugin manifest
//! command names to Rust types. Real execution routes through EnhancedShell.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpShellCommand {
    /// VCP LinuxShellExecutor (113KB, 六层安全)
    LinuxShellExecutor,
    /// VCP PowerShellExecutor (3KB)
    PowerShellExecutor,
    /// VCP SciCalculator
    SciCalculator,
    Unknown,
}

pub const VCP_SHELL_COMMAND_COUNT: usize = 3;

impl VcpShellCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "LinuxShellExecutor" => Self::LinuxShellExecutor,
            "PowerShellExecutor" => Self::PowerShellExecutor,
            "SciCalculator" => Self::SciCalculator,
            _ => Self::Unknown,
        }
    }
}

pub struct VcpShellRouter;

impl VcpShellRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        VCP_SHELL_COMMAND_COUNT
    }
}

impl Default for VcpShellRouter {
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
            assert_ne!(VcpShellCommand::from_str(s), VcpShellCommand::Unknown);
        }
        assert_eq!(VCP_SHELL_COMMAND_COUNT, 3);
    }

    #[test]
    fn unknown_maps_correctly() {
        assert_eq!(VcpShellCommand::from_str("Nonexistent"), VcpShellCommand::Unknown);
    }
}
