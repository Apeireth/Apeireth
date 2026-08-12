//! `apeireth-memory-dailynote` - R141 daily note (4→1 merge per v2 plan §9.5).
//!
//! VCP had 4 separate plugins: DailyNote, DailyNoteSearcher, DailyNoteFolder,
//! DailyNoteExporter. We merge into 1 cohesive crate.
//!
//! Functions:
//! 1. **CRUD** — create/read/update/delete daily notes
//! 2. **Tag index** — reverse-lookup tags to notes
//! 3. **Search** — BM25-lite (substring + tag filter)
//! 4. **Export** — Markdown/JSON serialization
//! 5. **MCP server** — 4 tools

#![warn(missing_docs)]

pub mod note;
pub mod store;
pub mod search;
pub mod export;
pub mod mcp;
pub mod vcp_compat;
pub mod enhanced;

pub use note::{DailyNote, NoteId};
pub use store::{DailyNoteStore, DailyNoteError};
pub use search::{search_notes, SearchHit};
pub use export::{export_markdown, export_json, ExportFormat};
pub use mcp::{DailyNoteMcp, DailyNoteTool};
pub use vcp_compat::{VcpDailyNoteCommand, VcpDailyNoteRouter, VCP_DAILYNOTE_COMMAND_COUNT};
pub use enhanced::EnhancedDailyNote;

/// R141 deliverables for dailynote:
/// - 4 modules (note / store / search / export) + mcp + vcp_compat + enhanced
/// - 4→1 merge per v2 plan §9.5
pub const R141_DAILYNOTE_DELIVERABLES: usize = 7;