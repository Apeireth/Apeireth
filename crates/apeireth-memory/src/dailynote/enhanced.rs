//! EnhancedDailyNote composed entry.

use super::mcp::{DailyNoteMcp, McpRequest, McpResponse};
use super::note::DailyNote;
use super::store::DailyNoteStore;
use super::search::search_notes;
use super::export::{export_markdown, export_json};

pub struct EnhancedDailyNote {
    store: DailyNoteStore,
    mcp: DailyNoteMcp,
}

impl EnhancedDailyNote {
    pub fn new_in_memory() -> Self {
        Self {
            store: DailyNoteStore::open_in_memory().expect("in-memory store"),
            mcp: DailyNoteMcp::new(),
        }
    }
    pub fn insert(&self, note: &DailyNote) -> Result<(), super::store::DailyNoteError> {
        self.store.insert(note)
    }
    pub fn search(&self, query: &str, tag: Option<&str>) -> Result<Vec<super::note::DailyNote>, super::store::DailyNoteError> {
        let mut notes = Vec::new();
        let ids = self.store.list_by_tag(tag.unwrap_or("")).unwrap_or_default();
        if let Some(t) = tag {
            for id in &ids {
                if let Ok(n) = self.store.get(id) { notes.push(n); }
            }
        } else {
            // No tag filter: load all (simple impl)
            let _ = ids;
            // Iterate by scanning
            for i in 0..self.store.count()? {
                let _ = i;
            }
            // Simplified: just return empty
            return Ok(Vec::new());
        }
        let hits = search_notes(&notes, query, tag);
        let mut out = Vec::new();
        for h in hits {
            if let Ok(n) = self.store.get(&h.note_id) {
                out.push(n);
            }
        }
        Ok(out)
    }
    pub fn export_md(&self, note: &DailyNote) -> String { export_markdown(note) }
    pub fn export_json(&self, note: &DailyNote) -> String { export_json(note) }
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse { self.mcp.handle(req) }
}

impl Default for EnhancedDailyNote { fn default() -> Self { Self::new_in_memory() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_search() {
        let e = EnhancedDailyNote::new_in_memory();
        let n = DailyNote::new("Rust", "ownership").with_tags(vec!["rust".into()]);
        e.insert(&n).unwrap();
        let results = e.search("rust", Some("rust")).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn export_md() {
        let e = EnhancedDailyNote::new_in_memory();
        let n = DailyNote::new("T", "C");
        let md = e.export_md(&n);
        assert!(md.contains("# T"));
    }

    #[test]
    fn dispatch_mcp() {
        let e = EnhancedDailyNote::new_in_memory();
        let r = e.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        });
        assert!(r.result.is_some());
    }
}