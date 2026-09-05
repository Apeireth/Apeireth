//! VCP layered_memo compatibility (1 router).
//!
//! `layered_memo` keeps the wire/command spelling from the VCP protocol rather
//! than Rust enum style, so call sites can round-trip the original identifier.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayeredMemoCommand {
    #[allow(non_camel_case_types)] // protocol command id `layered_memo`, not a Rust type name
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
        assert_eq!(
            LayeredMemoCommand::from_str("xyz"),
            LayeredMemoCommand::Unknown
        );
    }
    #[test]
    fn router_count() {
        assert_eq!(LayeredMemoCompatRouter::command_count(), 2);
    }
}
