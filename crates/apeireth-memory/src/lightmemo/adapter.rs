//! Adapter: multi-source memory provider.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Conversation,
    File,
    Api,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct MemorySource {
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
}

pub trait MemoryAdapter: Send + Sync {
    fn source(&self) -> &MemorySource;
    /// Fetch raw items from the source. Implementations should return
    /// (item_id, content) pairs.
    fn fetch(&self) -> Result<Vec<(String, String)>, String>;
}

pub struct AdapterRegistry {
    adapters: HashMap<String, Box<dyn MemoryAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self { Self { adapters: HashMap::new() } }
    pub fn register(&mut self, adapter: Box<dyn MemoryAdapter>) {
        self.adapters.insert(adapter.source().id.clone(), adapter);
    }
    pub fn get(&self, id: &str) -> Option<&dyn MemoryAdapter> {
        self.adapters.get(id).map(|b| b.as_ref())
    }
    pub fn count(&self) -> usize { self.adapters.len() }
    pub fn sources(&self) -> Vec<MemorySource> {
        self.adapters.values().map(|a| a.source().clone()).collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self { Self::new() }
}

/// Built-in: simple conversation adapter (in-memory).
pub struct ConversationAdapter {
    source: MemorySource,
    messages: Vec<(String, String)>,
}

impl ConversationAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            source: MemorySource {
                id: "conversation-default".to_string(),
                kind: SourceKind::Conversation,
                name: name.into(),
            },
            messages: Vec::new(),
        }
    }
    pub fn add_message(&mut self, id: impl Into<String>, content: impl Into<String>) {
        self.messages.push((id.into(), content.into()));
    }
}

impl MemoryAdapter for ConversationAdapter {
    fn source(&self) -> &MemorySource { &self.source }
    fn fetch(&self) -> Result<Vec<(String, String)>, String> {
        Ok(self.messages.clone())
    }
}

/// Built-in: file adapter (loads files from a directory).
pub struct FileAdapter {
    source: MemorySource,
    paths: Vec<String>,
}

impl FileAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            source: MemorySource {
                id: "file-default".to_string(),
                kind: SourceKind::File,
                name: name.into(),
            },
            paths: Vec::new(),
        }
    }
    pub fn add_path(&mut self, path: impl Into<String>) {
        self.paths.push(path.into());
    }
}

impl MemoryAdapter for FileAdapter {
    fn source(&self) -> &MemorySource { &self.source }
    fn fetch(&self) -> Result<Vec<(String, String)>, String> {
        let mut out = Vec::new();
        for path in &self.paths {
            match std::fs::read_to_string(path) {
                Ok(content) => out.push((path.clone(), content)),
                Err(e) => return Err(format!("read {}: {}", path, e)),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_adapter() {
        let mut a = ConversationAdapter::new("chat1");
        a.add_message("m1", "hello");
        a.add_message("m2", "world");
        let r = a.fetch().unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].1, "hello");
    }

    #[test]
    fn file_adapter() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("test.txt");
        std::fs::write(&p, "file content").unwrap();
        let mut a = FileAdapter::new("files1");
        a.add_path(p.to_string_lossy().to_string());
        let r = a.fetch().unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, "file content");
    }

    #[test]
    fn file_adapter_missing_returns_err() {
        let mut a = FileAdapter::new("files1");
        a.add_path("/nonexistent/path/file.txt");
        let r = a.fetch();
        assert!(r.is_err());
    }

    #[test]
    fn registry_register_get() {
        let mut r = AdapterRegistry::new();
        r.register(Box::new(ConversationAdapter::new("chat1")));
        assert_eq!(r.count(), 1);
        assert!(r.get("conversation-default").is_some());
        assert!(r.get("missing").is_none());
    }

    #[test]
    fn registry_sources() {
        let mut r = AdapterRegistry::new();
        r.register(Box::new(ConversationAdapter::new("c")));
        r.register(Box::new(FileAdapter::new("f")));
        assert_eq!(r.count(), 2);
        assert_eq!(r.sources().len(), 2);
    }

    #[test]
    fn source_kinds_distinct() {
        assert_ne!(SourceKind::Conversation, SourceKind::File);
        assert_ne!(SourceKind::Api, SourceKind::Conversation);
    }
}