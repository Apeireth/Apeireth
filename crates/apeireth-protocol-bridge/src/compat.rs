//! VCP bridge (5→1 merge) compatibility.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompatBridgeCommand {
    VcpOpenAIAdapter,
    VcpAnthropicAdapter,
    VcpGeminiAdapter,
    VcpResponsesAdapter,
    CompatProtocolMux,
    Unknown,
}

pub const COMPAT_BRIDGE_COMMAND_COUNT: usize = 5;

impl CompatBridgeCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "VcpOpenAIAdapter" => Self::VcpOpenAIAdapter,
            "VcpAnthropicAdapter" => Self::VcpAnthropicAdapter,
            "VcpGeminiAdapter" => Self::VcpGeminiAdapter,
            "VcpResponsesAdapter" => Self::VcpResponsesAdapter,
            "CompatProtocolMux" => Self::CompatProtocolMux,
            _ => Self::Unknown,
        }
    }
}

pub struct CompatBridgeRouter;

impl CompatBridgeRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { COMPAT_BRIDGE_COMMAND_COUNT }
}

impl Default for CompatBridgeRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_5_commands() {
        for s in ["VcpOpenAIAdapter","VcpAnthropicAdapter","VcpGeminiAdapter","VcpResponsesAdapter","CompatProtocolMux"] {
            assert_ne!(CompatBridgeCommand::from_str(s), CompatBridgeCommand::Unknown);
        }
        assert_eq!(COMPAT_BRIDGE_COMMAND_COUNT, 5);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(CompatBridgeCommand::from_str("xyz"), CompatBridgeCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(CompatBridgeRouter::command_count(), 5);
    }
}