//! Knowledge graph: file → symbol → imports → references.
//!
//! Per v2 plan §9.4 Decision 1, borrows codebase-memory-mcp design (file →
//! symbol → dependency graph). In-memory only for R140; persistent index via
//! rusqlite deferred.
//!
//! Graph model:
//! - File nodes (one per indexed file)
//! - Symbol nodes (one per extracted symbol)
//! - Edges: File→Symbol (defined_in), Symbol→Symbol (imports/calls),
//!   File→File (imports)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use crate::symbols::Symbol;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    File,
    Symbol,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphEdge {
    DefinedIn,
    Imports,
    Calls,
}

impl GraphEdge {
    pub fn as_str(&self) -> &str {
        match self {
            GraphEdge::DefinedIn => "defined_in",
            GraphEdge::Imports => "imports",
            GraphEdge::Calls => "calls",
        }
    }
}

pub struct KnowledgeGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<(String, GraphEdge, String)>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: &str) -> String {
        let id = format!("file:{}", path);
        self.nodes.entry(id.clone()).or_insert_with(|| GraphNode {
            id: id.clone(),
            kind: NodeKind::File,
            label: path.to_string(),
            metadata: HashMap::new(),
        });
        id
    }

    pub fn add_symbol(&mut self, file_path: &str, sym: &Symbol) -> String {
        let sym_id = format!("sym:{}::{}", file_path, sym.name);
        self.nodes
            .entry(sym_id.clone())
            .or_insert_with(|| GraphNode {
                id: sym_id.clone(),
                kind: NodeKind::Symbol,
                label: format!("{} {}", sym.kind.as_str(), sym.name),
                metadata: HashMap::from([
                    ("line".to_string(), sym.line.to_string()),
                    ("language".to_string(), sym.language.clone()),
                ]),
            });
        // File→Symbol edge (DefinedIn)
        let file_id = format!("file:{}", file_path);
        if self.nodes.contains_key(&file_id) {
            self.edges
                .push((file_id, GraphEdge::DefinedIn, sym_id.clone()));
        }
        sym_id
    }

    pub fn add_import(&mut self, from_file: &str, to_module: &str) {
        let from_id = format!("file:{}", from_file);
        let to_id = format!("sym:{}", to_module); // symbol id convention
        if self.nodes.contains_key(&from_id) {
            self.edges.push((from_id, GraphEdge::Imports, to_id));
        }
    }

    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    pub fn edges(&self) -> impl Iterator<Item = &(String, GraphEdge, String)> {
        self.edges.iter()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn find_symbols_in_file(&self, file_path: &str) -> Vec<&GraphNode> {
        let file_id = format!("file:{}", file_path);
        let edges_vec: Vec<_> = self.edges.iter().collect();
        self.nodes
            .values()
            .filter(|n| {
                n.kind == NodeKind::Symbol
                    && edges_vec.iter().any(|(from, edge, to)| {
                        from == &file_id && *edge == GraphEdge::DefinedIn && *to == n.id
                    })
            })
            .collect()
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{Symbol, SymbolKind};

    fn sym(name: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line: 1,
            column: 1,
            language: "rust".to_string(),
            signature: format!("fn {}() {{}}", name),
        }
    }

    #[test]
    fn add_file_creates_node() {
        let mut g = KnowledgeGraph::new();
        let id = g.add_file("src/main.rs");
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.nodes[&id].label, "src/main.rs");
    }

    #[test]
    fn add_symbol_creates_edge_to_file() {
        let mut g = KnowledgeGraph::new();
        g.add_file("src/main.rs");
        let sym_id = g.add_symbol("src/main.rs", &sym("hello"));
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        let edge = g.edges().next().unwrap();
        assert_eq!(edge.1, GraphEdge::DefinedIn);
        assert_eq!(edge.2, sym_id);
    }

    #[test]
    fn find_symbols_in_file() {
        let mut g = KnowledgeGraph::new();
        g.add_file("src/lib.rs");
        g.add_symbol("src/lib.rs", &sym("a"));
        g.add_symbol("src/lib.rs", &sym("b"));
        g.add_file("src/main.rs");
        g.add_symbol("src/main.rs", &sym("c"));
        let in_lib = g.find_symbols_in_file("src/lib.rs");
        assert_eq!(in_lib.len(), 2);
    }

    #[test]
    fn import_edge() {
        let mut g = KnowledgeGraph::new();
        g.add_file("src/main.rs");
        g.add_file("src/lib.rs");
        g.add_import("src/main.rs", "src/lib.rs");
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn empty_graph() {
        let g = KnowledgeGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }
}
