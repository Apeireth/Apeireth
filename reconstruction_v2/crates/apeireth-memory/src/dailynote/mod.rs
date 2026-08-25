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

pub mod compat;
pub mod enhanced;
pub mod export;
pub mod mcp;
pub mod note;
pub mod search;
pub mod store;

pub use compat::{DailyNoteCommand, DailyNoteCompatRouter, DAILYNOTE_COMMAND_COUNT};
pub use enhanced::EnhancedDailyNote;
pub use export::{export_json, export_markdown, ExportFormat};
pub use mcp::{DailyNoteMcp, DailyNoteTool};
pub use note::{DailyNote, NoteId};
pub use search::{search_notes, SearchHit};
pub use store::{DailyNoteError, DailyNoteStore};

/// R141 deliverables for dailynote:
/// - 4 modules (note / store / search / export) + mcp + compat + enhanced
/// - 4→1 merge per v2 plan §9.5
pub const R141_DAILYNOTE_DELIVERABLES: usize = 7;
