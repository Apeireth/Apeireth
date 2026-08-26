//! L2: Vector store (in-memory cosine similarity).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub id: String,
    pub embedding: Vec<f32>,
}

pub struct L2VectorStore {
    entries: HashMap<String, VectorEntry>,
}

impl L2VectorStore {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entry: VectorEntry) {
        self.entries.insert(entry.id.clone(), entry);
    }

    pub fn remove(&mut self, id: &str) -> Option<VectorEntry> {
        self.entries.remove(id)
    }

    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.entries.get(id)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Cosine similarity search. Returns top-k (id, score) sorted by score desc.
    pub fn cosine_search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .entries
            .values()
            .map(|e| (e.id.clone(), cosine(query, &e.embedding)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

impl Default for L2VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_store() {
        let s = L2VectorStore::new();
        assert_eq!(s.count(), 0);
        assert!(s.cosine_search(&[1.0, 0.0], 5).is_empty());
    }

    #[test]
    fn insert_and_search() {
        let mut s = L2VectorStore::new();
        s.insert(VectorEntry {
            id: "a".into(),
            embedding: vec![1.0, 0.0],
        });
        s.insert(VectorEntry {
            id: "b".into(),
            embedding: vec![0.0, 1.0],
        });
        s.insert(VectorEntry {
            id: "c".into(),
            embedding: vec![1.0, 1.0],
        });
        let r = s.cosine_search(&[1.0, 0.0], 2);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "a");
    }

    #[test]
    fn cosine_identical_is_1() {
        let v = vec![0.5, 0.5, 0.5];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_is_0() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn remove_works() {
        let mut s = L2VectorStore::new();
        s.insert(VectorEntry {
            id: "x".into(),
            embedding: vec![1.0],
        });
        assert_eq!(s.count(), 1);
        let r = s.remove("x");
        assert!(r.is_some());
        assert_eq!(s.count(), 0);
    }
}
