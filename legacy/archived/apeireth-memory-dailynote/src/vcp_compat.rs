//! VCP daily note (4→1 merge) compatibility.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpDailyNoteCommand {
    DailyNote,
    DailyNoteSearcher,
    DailyNoteFolder,
    DailyNoteExporter,
    Unknown,
}

pub const VCP_DAILYNOTE_COMMAND_COUNT: usize = 4;

impl VcpDailyNoteCommand {
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

pub struct VcpDailyNoteRouter;

impl VcpDailyNoteRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { VCP_DAILYNOTE_COMMAND_COUNT }
}

impl Default for VcpDailyNoteRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_4_commands() {
        for s in ["DailyNote", "DailyNoteSearcher", "DailyNoteFolder", "DailyNoteExporter"] {
            assert_ne!(VcpDailyNoteCommand::from_str(s), VcpDailyNoteCommand::Unknown);
        }
        assert_eq!(VCP_DAILYNOTE_COMMAND_COUNT, 4);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(VcpDailyNoteCommand::from_str("xyz"), VcpDailyNoteCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(VcpDailyNoteRouter::command_count(), 4);
    }
}