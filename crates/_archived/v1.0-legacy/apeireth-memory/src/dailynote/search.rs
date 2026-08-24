//! Substring + tag search (BM25-lite).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use super::note::{DailyNote, NoteId};

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub note_id: NoteId,
    pub score: f32,
    pub matched_in: Vec<String>, // "title" / "content" / "tag"
}

pub fn search_notes(notes: &[DailyNote], query: &str, tag_filter: Option<&str>) -> Vec<SearchHit> {
    let query_lower = query.to_lowercase();
    let mut hits = Vec::new();
    for note in notes {
        if let Some(tag) = tag_filter {
            if !note.tags.iter().any(|t| t == tag) {
                continue;
            }
        }
        let mut score = 0.0f32;
        let mut matched_in = Vec::new();
        if note.title.to_lowercase().contains(&query_lower) {
            score += 2.0;
            matched_in.push("title".to_string());
        }
        if note.content.to_lowercase().contains(&query_lower) {
            score += 1.0;
            matched_in.push("content".to_string());
        }
        for tag in &note.tags {
            if tag.to_lowercase().contains(&query_lower) {
                score += 1.5;
                matched_in.push("tag".to_string());
                break;
            }
        }
        if score > 0.0 {
            hits.push(SearchHit {
                note_id: note.id.clone(),
                score,
                matched_in,
            });
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes() -> Vec<DailyNote> {
        vec![
            DailyNote::new("Rust tutorial", "learn rust ownership").with_tags(vec!["rust".into()]),
            DailyNote::new("Python basics", "python is great").with_tags(vec!["python".into()]),
            DailyNote::new("Mixed", "rust + python").with_tags(vec!["multi".into()]),
        ]
    }

    #[test]
    fn title_match_higher_score() {
        let hits = search_notes(&notes(), "rust", None);
        assert!(!hits.is_empty());
        // Title matches score higher
        assert!(hits[0].score >= 1.5);
    }

    #[test]
    fn content_match() {
        let hits = search_notes(&notes(), "ownership", None);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].matched_in.contains(&"content".to_string()));
    }

    #[test]
    fn tag_filter() {
        let hits = search_notes(&notes(), "rust", Some("python"));
        assert!(hits.is_empty(), "rust shouldn't match python-only tag");
    }

    #[test]
    fn no_match_returns_empty() {
        let hits = search_notes(&notes(), "nonexistent_xyz", None);
        assert!(hits.is_empty());
    }

    #[test]
    fn score_ordering() {
        let hits = search_notes(&notes(), "rust", None);
        // Sorted by score desc
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }
}
