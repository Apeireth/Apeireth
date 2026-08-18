//! VCP daily note (4→1 merge) compatibility.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DailyNoteCommand {
    DailyNote,
    DailyNoteSearcher,
    DailyNoteFolder,
    DailyNoteExporter,
    Unknown,
}

pub const DAILYNOTE_COMMAND_COUNT: usize = 4;

impl DailyNoteCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "DailyNote" => Self::DailyNote,
            "DailyNoteSearcher" => Self::DailyNoteSearcher,
            "DailyNoteFolder" => Self::DailyNoteFolder,
            "DailyNoteExporter" => Self::DailyNoteExporter,
            _ => Self::Unknown,
        }
    }
}

pub struct DailyNoteCompatRouter;

impl DailyNoteCompatRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        DAILYNOTE_COMMAND_COUNT
    }
}

impl Default for DailyNoteCompatRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_4_commands() {
        for s in [
            "DailyNote",
            "DailyNoteSearcher",
            "DailyNoteFolder",
            "DailyNoteExporter",
        ] {
            assert_ne!(DailyNoteCommand::from_str(s), DailyNoteCommand::Unknown);
        }
        assert_eq!(DAILYNOTE_COMMAND_COUNT, 4);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(DailyNoteCommand::from_str("xyz"), DailyNoteCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(DailyNoteCompatRouter::command_count(), 4);
    }
}
