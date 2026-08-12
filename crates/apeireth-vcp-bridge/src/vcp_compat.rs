//! VCP bridge (5→1 merge) compatibility.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpBridgeCommand {
    VcpOpenAIAdapter,
    VcpAnthropicAdapter,
    VcpGeminiAdapter,
    VcpResponsesAdapter,
    VcpProtocolMux,
    Unknown,
}

pub const VCP_BRIDGE_COMMAND_COUNT: usize = 5;

impl VcpBridgeCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "VcpOpenAIAdapter" => Self::VcpOpenAIAdapter,
            "VcpAnthropicAdapter" => Self::VcpAnthropicAdapter,
            "VcpGeminiAdapter" => Self::VcpGeminiAdapter,
            "VcpResponsesAdapter" => Self::VcpResponsesAdapter,
            "VcpProtocolMux" => Self::VcpProtocolMux,
            _ => Self::Unknown,
        }
    }
}

pub struct VcpBridgeRouter;

impl VcpBridgeRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { VCP_BRIDGE_COMMAND_COUNT }
}

impl Default for VcpBridgeRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_5_commands() {
        for s in ["VcpOpenAIAdapter","VcpAnthropicAdapter","VcpGeminiAdapter","VcpResponsesAdapter","VcpProtocolMux"] {
            assert_ne!(VcpBridgeCommand::from_str(s), VcpBridgeCommand::Unknown);
        }
        assert_eq!(VCP_BRIDGE_COMMAND_COUNT, 5);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(VcpBridgeCommand::from_str("xyz"), VcpBridgeCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(VcpBridgeRouter::command_count(), 5);
    }
}