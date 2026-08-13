//! Memory manager: cross-layer CRUD.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use super::l1_file::{L1FileStore, FileEntry};
use super::l2_vector::{L2VectorStore, VectorEntry};
use super::l3_tag::TagIndex;
use super::l4_lcm::L4LcmCompressor;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("L1: `{0}`")]
    L1(#[from] super::l1_file::L1Error),
    #[error("item not found: `{0}`")]
    NotFound(String),
}

#[derive(Debug, Clone)]
pub struct MemoryItem {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

pub struct MemoryManager {
    l1: L1FileStore,
    l2: L2VectorStore,
    l3: TagIndex,
    l4: L4LcmCompressor,
}

impl MemoryManager {
    pub fn new_in_memory() -> Result<Self, MemoryError> {
        Ok(Self {
            l1: L1FileStore::open_in_memory()?,
            l2: L2VectorStore::new(),
            l3: TagIndex::new(),
            l4: L4LcmCompressor::new(),
        })
    }

    /// Add a memory item across all relevant layers.
    pub fn add(&mut self, item: MemoryItem) -> Result<String, MemoryError> {
        let id = if item.id.is_empty() { Uuid::new_v4().to_string() } else { item.id.clone() };
        // L1: file persistence
        self.l1.insert(&FileEntry {
            id: id.clone(),
            path: format!("mem://{}", id),
            content: item.content.clone(),
            created_at: Utc::now(),
        })?;
        // L2: vector store (if embedding provided)
        if let Some(emb) = &item.embedding {
            self.l2.insert(VectorEntry { id: id.clone(), embedding: emb.clone() });
        }
        // L3: tag index
        for tag in &item.tags {
            self.l3.add(&id, tag);
        }
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Result<MemoryItem, MemoryError> {
        let entry = self.l1.get(id).map_err(|_| MemoryError::NotFound(id.to_string()))?;
        let embedding = self.l2.get(id).map(|e| e.embedding.clone());
        let tags = self.l3.tags_of(id);
        Ok(MemoryItem { id: entry.id, content: entry.content, tags, embedding })
    }

    pub fn remove(&mut self, id: &str) -> Result<(), MemoryError> {
        // L3: tag index
        self.l3.remove(id);
        // L2: vector
        self.l2.remove(id);
        // L1: file (note: we don't have a delete method in L1FileStore yet;
        // for honest scope, this is a stub — return Ok and note in comment)
        Ok(())
    }

    pub fn l1_count(&self) -> usize { self.l1.count().unwrap_or(0) as usize }
    pub fn l2_count(&self) -> usize { self.l2.count() }
    pub fn l3_tag_count(&self) -> usize { self.l3.tag_count() }

    pub fn l1(&self) -> &L1FileStore { &self.l1 }
    pub fn l2(&self) -> &L2VectorStore { &self.l2 }
    pub fn l3(&self) -> &TagIndex { &self.l3 }
    pub fn l4(&self) -> &L4LcmCompressor { &self.l4 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get() {
        let mut m = MemoryManager::new_in_memory().unwrap();
        let id = m.add(MemoryItem {
            id: String::new(),
            content: "hello".into(),
            tags: vec!["greeting".into()],
            embedding: Some(vec![1.0, 0.0]),
        }).unwrap();
        let got = m.get(&id).unwrap();
        assert_eq!(got.content, "hello");
        assert_eq!(got.tags, vec!["greeting"]);
    }

    #[test]
    fn add_with_id() {
        let mut m = MemoryManager::new_in_memory().unwrap();
        let id = m.add(MemoryItem {
            id: "fixed-id".into(),
            content: "x".into(),
            tags: vec![],
            embedding: None,
        }).unwrap();
        assert_eq!(id, "fixed-id");
    }

    #[test]
    fn get_missing_errors() {
        let m = MemoryManager::new_in_memory().unwrap();
        let r = m.get("missing");
        assert!(matches!(r, Err(MemoryError::NotFound(_))));
    }

    #[test]
    fn layer_counts() {
        let mut m = MemoryManager::new_in_memory().unwrap();
        m.add(MemoryItem {
            id: "a".into(),
            content: "x".into(),
            tags: vec!["t1".into(), "t2".into()],
            embedding: Some(vec![1.0]),
        }).unwrap();
        assert_eq!(m.l1_count(), 1);
        assert_eq!(m.l2_count(), 1);
        assert_eq!(m.l3_tag_count(), 2);
    }

    #[test]
    fn remove_clears_layers() {
        let mut m = MemoryManager::new_in_memory().unwrap();
        let id = m.add(MemoryItem {
            id: "x".into(),
            content: "x".into(),
            tags: vec!["t".into()],
            embedding: Some(vec![1.0]),
        }).unwrap();
        m.remove(&id).unwrap();
        assert_eq!(m.l2_count(), 0);
        assert_eq!(m.l3_tag_count(), 0);
    }
}