//! VCP LightMemo compatibility (1 router).

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LightMemoCommand {
    LightMemo,
    MemoryConsolidator,
    Unknown,
}

pub const LIGHTMEMO_COMMAND_COUNT: usize = 2;

impl LightMemoCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "LightMemo" => Self::LightMemo,
            "MemoryConsolidator" => Self::MemoryConsolidator,
            _ => Self::Unknown,
        }
    }
}

pub struct LightMemoCompatRouter;

impl LightMemoCompatRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { LIGHTMEMO_COMMAND_COUNT }
}

impl Default for LightMemoCompatRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_2_commands() {
        for s in ["LightMemo", "MemoryConsolidator"] {
            assert_ne!(LightMemoCommand::from_str(s), LightMemoCommand::Unknown);
        }
        assert_eq!(LIGHTMEMO_COMMAND_COUNT, 2);
    }
    #[test]
    fn unknown_maps() {
        assert_eq!(LightMemoCommand::from_str("xyz"), LightMemoCommand::Unknown);
    }
    #[test]
    fn router_count() { assert_eq!(LightMemoCompatRouter::command_count(), 2); }
}