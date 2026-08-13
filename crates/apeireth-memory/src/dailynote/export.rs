//! Export daily notes.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use super::note::DailyNote;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Json,
}

pub fn export_markdown(note: &DailyNote) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", note.title));
    out.push_str(&format!("*Date: {}*\n\n", note.date.format("%Y-%m-%d")));
    if !note.tags.is_empty() {
        out.push_str(&format!("*Tags: {}*\n\n", note.tags.join(", ")));
    }
    out.push_str(&note.content);
    out.push('\n');
    out
}

pub fn export_json(note: &DailyNote) -> String {
    serde_json::json!({
        "id": note.id.0,
        "date": note.date.to_rfc3339(),
        "title": note.title,
        "content": note.content,
        "tags": note.tags,
        "created_at": note.created_at.to_rfc3339(),
        "updated_at": note.updated_at.to_rfc3339(),
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_includes_title_and_tags() {
        let n = DailyNote::new("Title", "Body").with_tags(vec!["a".into()]);
        let md = export_markdown(&n);
        assert!(md.contains("# Title"));
        assert!(md.contains("Body"));
        assert!(md.contains("Tags: a"));
    }

    #[test]
    fn json_includes_all_fields() {
        let n = DailyNote::new("T", "C").with_tags(vec!["x".into()]);
        let js = export_json(&n);
        assert!(js.contains("\"title\":\"T\""));
        assert!(js.contains("\"content\":\"C\""));
        assert!(js.contains("\"tags\":[\"x\"]"));
    }
}