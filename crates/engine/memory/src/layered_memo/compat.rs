//! VCP layered_memo compatibility (1 router).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayeredMemoCommand {
    layered_memo,
    MemoryConsolidator,
    Unknown,
}

pub const LAYERED_MEMO_COMMAND_COUNT: usize = 2;

impl LayeredMemoCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "layered_memo" => Self::layered_memo,
            "MemoryConsolidator" => Self::MemoryConsolidator,
            _ => Self::Unknown,
        }
    }
}

pub struct LayeredMemoCompatRouter;

impl LayeredMemoCompatRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        LAYERED_MEMO_COMMAND_COUNT
    }
}

impl Default for LayeredMemoCompatRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_2_commands() {
        for s in ["layered_memo", "MemoryConsolidator"] {
            assert_ne!(LayeredMemoCommand::from_str(s), LayeredMemoCommand::Unknown);
        }
        assert_eq!(LAYERED_MEMO_COMMAND_COUNT, 2);
    }
    #[test]
    fn unknown_maps() {
        assert_eq!(LayeredMemoCommand::from_str("xyz"), LayeredMemoCommand::Unknown);
    }
    #[test]
    fn router_count() {
        assert_eq!(LayeredMemoCompatRouter::command_count(), 2);
    }
}
