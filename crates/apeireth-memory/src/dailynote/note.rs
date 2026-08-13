//! DailyNote + NoteId types.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteId(pub String);

impl NoteId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl Default for NoteId {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct DailyNote {
    pub id: NoteId,
    pub date: DateTime<Utc>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DailyNote {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: NoteId::new(),
            date: now,
            title: title.into(),
            content: content.into(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
    pub fn with_tags(mut self, tags: Vec<String>) -> Self { self.tags = tags; self }
    pub fn with_date(mut self, date: DateTime<Utc>) -> Self { self.date = date; self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_construction() {
        let n = DailyNote::new("Test", "Content");
        assert_eq!(n.title, "Test");
        assert_eq!(n.content, "Content");
        assert!(n.tags.is_empty());
    }

    #[test]
    fn note_with_tags() {
        let n = DailyNote::new("T", "C").with_tags(vec!["work".into(), "idea".into()]);
        assert_eq!(n.tags.len(), 2);
        assert!(n.tags.contains(&"work".to_string()));
    }

    #[test]
    fn note_id_unique() {
        let a = NoteId::new();
        let b = NoteId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn note_id_as_str() {
        let n = DailyNote::new("T", "C");
        assert!(!n.id.as_str().is_empty());
    }
}