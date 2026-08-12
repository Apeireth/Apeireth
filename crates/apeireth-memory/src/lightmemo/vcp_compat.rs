//! VCP LightMemo compatibility (1 router).

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpLightMemoCommand {
    LightMemo,
    MemoryConsolidator,
    Unknown,
}

pub const VCP_LIGHTMEMO_COMMAND_COUNT: usize = 2;

impl VcpLightMemoCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "LightMemo" => Self::LightMemo,
            "MemoryConsolidator" => Self::MemoryConsolidator,
            _ => Self::Unknown,
        }
    }
}

pub struct VcpLightMemoRouter;

impl VcpLightMemoRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { VCP_LIGHTMEMO_COMMAND_COUNT }
}

impl Default for VcpLightMemoRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_2_commands() {
        for s in ["LightMemo", "MemoryConsolidator"] {
            assert_ne!(VcpLightMemoCommand::from_str(s), VcpLightMemoCommand::Unknown);
        }
        assert_eq!(VCP_LIGHTMEMO_COMMAND_COUNT, 2);
    }
    #[test]
    fn unknown_maps() {
        assert_eq!(VcpLightMemoCommand::from_str("xyz"), VcpLightMemoCommand::Unknown);
    }
    #[test]
    fn router_count() { assert_eq!(VcpLightMemoRouter::command_count(), 2); }
}