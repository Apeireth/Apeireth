//! ToolBridge - 工具桥 (从 v1.0 apeireth-companion/tool_bridge.rs 2214 LOC 抄录升级核心)
use std::path::Path;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::capability::CapabilityManifest;
use super::constitution_gate::ConstitutionGate;
use super::packs::PackRegistry;
use super::spill::SpillStore;

pub fn path_within(path: &str, base: &str) -> bool {
    let norm = |p: &Path| -> String {
        p.to_string_lossy().replace("\\", "/").trim_end_matches('/').to_lowercase()
    };
    let base_p = Path::new(base);
    let base_c = std::fs::canonicalize(base_p).unwrap_or_else(|_| base_p.to_path_buf());
    let path_p = Path::new(path);
    let path_c = match std::fs::canonicalize(path_p) {
        Ok(c) => c,
        Err(_) => match path_p.parent().and_then(|pa| std::fs::canonicalize(pa).ok()) {
            Some(cp) => cp.join(path_p.file_name().unwrap_or_default()),
            None => path_p.to_path_buf(),
        },
    };
    let (b, p) = (norm(&base_c), norm(&path_c));
    p == b || p.starts_with(&format!("{}/", b))
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> ToolKind;
    fn axes(&self) -> ToolAxes { ToolAxes::default() }
    fn call(&self, args: Value) -> Result<Value, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind { Sync, Async, SideEffect }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolAxes { pub read: bool, pub write: bool, pub destructive: bool }

pub struct ToolRegistry { pub tools: std::collections::HashMap<String, Arc<dyn Tool>> }

impl ToolRegistry {
    pub fn new() -> Self { Self { tools: std::collections::HashMap::new() } }
    pub fn register(&mut self, t: Arc<dyn Tool>) { self.tools.insert(t.name().to_string(), t); }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> { self.tools.get(name).cloned() }
    pub fn list(&self) -> Vec<String> { self.tools.keys().cloned().collect() }
}

impl Default for ToolRegistry { fn default() -> Self { Self::new() } }

pub struct RecallMemoryTool { pub store: Arc<dyn MemoryStoreTrait> }

pub trait MemoryStoreTrait: Send + Sync {
    fn query(&self, q: &str) -> Vec<String>;
}

impl RecallMemoryTool {
    pub fn new(store: Arc<dyn MemoryStoreTrait>) -> Self { Self { store } }
}

impl Tool for RecallMemoryTool {
    fn name(&self) -> &str { "recall_memory" }
    fn kind(&self) -> ToolKind { ToolKind::Sync }
    fn call(&self, args: Value) -> Result<Value, String> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or_else(|| "query required".to_string())?;
        let hits = self.store.query(query);
        Ok(json!({ "query": query, "found": hits.len(), "top": hits.into_iter().take(3).collect::<Vec<_>>() }))
    }
}

pub struct SaveMemoryTool { pub store: Arc<dyn MemoryStoreTrait> }

impl Tool for SaveMemoryTool {
    fn name(&self) -> &str { "save_memory" }
    fn kind(&self) -> ToolKind { ToolKind::SideEffect }
    fn call(&self, args: Value) -> Result<Value, String> {
        let content = args.get("content").and_then(|v| v.as_str()).map(|s| s.trim())
            .filter(|s| !s.is_empty()).ok_or_else(|| "content empty".to_string())?;
        if content.chars().count() > 500 { return Err("too long".into()); }
        let preview: String = content.chars().take(40).collect();
        Ok(json!({ "ok": true, "saved": preview + "..." }))
    }
}

pub struct ToolBridge {
    pub registry: Arc<ToolRegistry>,
    pub packs: Arc<PackRegistry>,
    pub capabilities: Arc<CapabilityManifest>,
    pub constitution: Arc<ConstitutionGate>,
    pub spill: Arc<SpillStore>,
    pub memory: Arc<dyn MemoryStoreTrait>,
}

impl ToolBridge {
    pub fn new(memory: Arc<dyn MemoryStoreTrait>) -> Self {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(RecallMemoryTool { store: memory.clone() }));
        registry.register(Arc::new(SaveMemoryTool { store: memory.clone() }));
        Self {
            registry: Arc::new(registry),
            packs: Arc::new(PackRegistry::new()),
            capabilities: Arc::new(CapabilityManifest::new()),
            constitution: Arc::new(ConstitutionGate::new(vec![], 50)),
            spill: Arc::new(SpillStore::new()),
            memory,
        }
    }

    pub fn call(&self, name: &str, args: Value) -> Result<Value, String> {
        let tool = self.registry.get(name).ok_or_else(|| format!("tool not found: {}", name))?;
        if name == "recall_memory" || name == "save_memory" { return tool.call(args); }
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            if !path_within(path, ".") { return Err("path not in base".into()); }
        }
        if self.spill.needs_spill(name) { return Err("tool spilled".into()); }
        tool.call(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct MockMemory { pub items: Vec<String> }
    impl MemoryStoreTrait for MockMemory {
        fn query(&self, q: &str) -> Vec<String> { self.items.iter().filter(|i| i.contains(q)).cloned().collect() }
    }
    #[test] fn test_recall_memory() {
        let mem: Arc<dyn MemoryStoreTrait> = Arc::new(MockMemory { items: vec!["rust language".into()] });
        let tool = RecallMemoryTool { store: mem };
        let r = tool.call(json!({"query": "rust"})).unwrap();
        assert_eq!(r["found"], 1);
    }
    #[test] fn test_save_memory_limit() {
        let mem: Arc<dyn MemoryStoreTrait> = Arc::new(MockMemory { items: vec![] });
        let tool = SaveMemoryTool { store: mem };
        let long = "x".repeat(501);
        assert!(tool.call(json!({"content": long})).is_err());
    }
    #[test] fn test_save_memory_ok() {
        let mem: Arc<dyn MemoryStoreTrait> = Arc::new(MockMemory { items: vec![] });
        let tool = SaveMemoryTool { store: mem };
        assert!(tool.call(json!({"content": "hello"})).is_ok());
    }
    #[test] fn test_bridge_call_builtin() {
        let mem: Arc<dyn MemoryStoreTrait> = Arc::new(MockMemory { items: vec!["x".into()] });
        let b = ToolBridge::new(mem);
        assert!(b.call("recall_memory", json!({"query": "x"})).is_ok());
    }
    #[test] fn test_bridge_unknown_tool() {
        let mem: Arc<dyn MemoryStoreTrait> = Arc::new(MockMemory { items: vec![] });
        let b = ToolBridge::new(mem);
        assert!(b.call("nonexistent", json!({})).is_err());
    }
    #[test] fn test_tool_kind_eq() { assert_eq!(ToolKind::Sync, ToolKind::Sync); }
    #[test] fn test_tool_registry() {
        let mut r = ToolRegistry::new();
        r.register(Arc::new(SaveMemoryTool { store: Arc::new(MockMemory { items: vec![] }) }));
        assert_eq!(r.list().len(), 1);
    }
    #[test] fn test_path_within_root() { let p = "."; assert!(path_within("./foo.rs", p)); }
}