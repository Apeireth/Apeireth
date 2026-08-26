//! EnhancedCodeSearch — composed entry point for code search tool.
//!
//! Composes search + files + symbols + graph + index + MCP. This is what
//! `apeireth-tools` registers as a single Tool entry.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use thiserror::Error;

use crate::graph::KnowledgeGraph;
use crate::index::CodeIndex;
use crate::mcp::{CodeSearchMcp, McpRequest, McpResponse};
use crate::search::{CodeSearcher, SearchKind, SearchOptions};
use crate::symbols::{extract_symbols, Symbol};

#[derive(Debug, Error)]
pub enum EnhancedCodeSearchError {
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("file-finder: `{0}`")]
    FileFinder(#[from] crate::files::FileFinderError),
    #[error("search: `{0}`")]
    Search(String),
}

/// Composed code search tool: holds the MCP facade + delegates.
pub struct EnhancedCodeSearch {
    mcp: CodeSearchMcp,
}

impl EnhancedCodeSearch {
    pub fn new_in_memory() -> Self {
        Self {
            mcp: CodeSearchMcp::new_in_memory(),
        }
    }

    /// Direct text search (bypass MCP, returns Vec<String> summaries).
    pub fn search_text(
        &self,
        root: &str,
        pattern: &str,
        kind: SearchKind,
    ) -> Result<Vec<String>, EnhancedCodeSearchError> {
        use crate::files::{FileFinder, FindOptions};
        let finder = FileFinder::new();
        let entries = finder.find(root, &FindOptions::default())?;
        let searcher = CodeSearcher::new();
        let mut out = Vec::new();
        for entry in entries.iter().filter(|e| !e.is_dir) {
            let matches = searcher
                .search_file(
                    std::path::Path::new(&entry.path),
                    kind,
                    pattern,
                    &SearchOptions::default(),
                )
                .map_err(|e| EnhancedCodeSearchError::Search(e.to_string()))?;
            for m in matches {
                out.push(format!(
                    "{}:{}:{} {}",
                    m.file,
                    m.line,
                    m.column,
                    m.text.trim()
                ));
            }
        }
        Ok(out)
    }

    /// Extract symbols from a single file (delegates to symbols module).
    pub fn extract_symbols(&self, path: &str) -> Result<Vec<Symbol>, EnhancedCodeSearchError> {
        let content = std::fs::read_to_string(path)?;
        let lang = crate::symbols::detect_language(std::path::Path::new(path)).unwrap_or("");
        Ok(extract_symbols(&content, lang))
    }

    /// Dispatch an MCP request through the underlying server.
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        self.mcp.handle(req)
    }

    pub fn graph(&self) -> &KnowledgeGraph {
        // Mutex poisoning is a programmer error; for inspection only,
        // use the in-memory graph accessor via dispatch_mcp.
        // Here we expose the count for tests.
        unimplemented!("use dispatch_mcp with project_overview instead")
    }

    pub fn index(&self) -> &CodeIndex {
        unimplemented!("use dispatch_mcp with index_stats instead")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn search_text_finds_in_temp_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.rs"), "fn hello() {}\nfn world() {}\n").unwrap();
        fs::write(tmp.path().join("b.rs"), "fn foo() {}\n").unwrap();
        let e = EnhancedCodeSearch::new_in_memory();
        let r = e
            .search_text(tmp.path().to_str().unwrap(), "hello", SearchKind::Literal)
            .unwrap();
        assert!(!r.is_empty());
        assert!(r[0].contains("hello"));
    }

    #[test]
    fn extract_symbols_from_rust_file() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("x.rs");
        fs::write(&p, "fn foo() {}\nstruct Bar {}\n").unwrap();
        let e = EnhancedCodeSearch::new_in_memory();
        let syms = e.extract_symbols(p.to_str().unwrap()).unwrap();
        assert!(syms.iter().any(|s| s.name == "foo"));
        assert!(syms.iter().any(|s| s.name == "Bar"));
    }

    #[test]
    fn dispatch_mcp_works() {
        let e = EnhancedCodeSearch::new_in_memory();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };
        let resp = e.dispatch_mcp(req);
        assert!(resp.result.is_some());
    }
}
