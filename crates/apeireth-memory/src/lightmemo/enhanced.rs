//! EnhancedLightMemo composed entry.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use super::manager::{MemoryManager, MemoryItem};
use super::decay::DecayEngine;
use super::dream::DreamSubsystem;
use super::mcp::{LightMemoMcp, McpRequest, McpResponse};

pub struct EnhancedLightMemo {
    manager: MemoryManager,
    decay: DecayEngine,
    dream: DreamSubsystem,
    mcp: LightMemoMcp,
}

impl EnhancedLightMemo {
    pub fn new_in_memory() -> Result<Self, super::manager::MemoryError> {
        Ok(Self {
            manager: MemoryManager::new_in_memory()?,
            decay: DecayEngine::new(),
            dream: DreamSubsystem::new(),
            mcp: LightMemoMcp::new(),
        })
    }

    pub fn add_memory(&mut self, content: &str, tags: Vec<String>) -> Result<String, super::manager::MemoryError> {
        self.manager.add(MemoryItem {
            id: String::new(),
            content: content.into(),
            tags,
            embedding: None,
        })
    }

    pub fn decay(&self) -> &DecayEngine { &self.decay }
    pub fn dream(&self) -> &DreamSubsystem { &self.dream }
    pub fn manager(&self) -> &MemoryManager { &self.manager }

    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse { self.mcp.handle(req) }
}

impl Default for EnhancedLightMemo {
    fn default() -> Self { Self::new_in_memory().expect("in-memory") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn add_and_decay() {
        let mut e = EnhancedLightMemo::new_in_memory().unwrap();
        let id = e.add_memory("test", vec!["a".into()]).unwrap();
        // Recent item should have high decay strength
        let strength = e.decay().strength(Utc::now());
        assert!(strength > 0.9);
        let _ = id;
    }

    #[test]
    fn dispatch_mcp() {
        let e = EnhancedLightMemo::new_in_memory().unwrap();
        let r = e.dispatch_mcp(McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        });
        assert!(r.result.is_some());
    }
}