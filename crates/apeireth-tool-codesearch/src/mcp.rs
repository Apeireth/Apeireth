//! MCP server for code search — 10 tools.
//!
//! Per v2 plan §9.4 / codebase-memory-mcp design (15 tools). We expose 10
//! well-scoped tools; the rest deferred to R140+.
//!
//! Tools:
//! 1. `search_text`         — regex/literal search across files
//! 2. `find_files`         — walkdir + glob
//! 3. `extract_symbols`    — symbol extraction for a file
//! 4. `list_languages`     — supported languages
//! 5. `lookup_symbol`      — index lookup by symbol name
//! 6. `index_file`         — add file to persistent index
//! 7. `index_stats`        — counts in persistent index
//! 8. `trace_imports`      — knowledge graph traversal (file → imports)
//! 9. `find_callers`       — knowledge graph: who calls symbol X
//! 10. `project_overview`  — top-level project structure summary

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::search::{CodeSearcher, SearchKind, SearchOptions};
use crate::files::{FileFinder, FindOptions};
use crate::symbols::{extract_symbols, detect_language, supported_languages};
use crate::graph::KnowledgeGraph;
use crate::index::CodeIndex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTool {
    SearchText,
    FindFiles,
    ExtractSymbols,
    ListLanguages,
    LookupSymbol,
    IndexFile,
    IndexStats,
    TraceImports,
    FindCallers,
    ProjectOverview,
}

impl McpTool {
    pub fn name(&self) -> &'static str {
        match self {
            McpTool::SearchText => "search_text",
            McpTool::FindFiles => "find_files",
            McpTool::ExtractSymbols => "extract_symbols",
            McpTool::ListLanguages => "list_languages",
            McpTool::LookupSymbol => "lookup_symbol",
            McpTool::IndexFile => "index_file",
            McpTool::IndexStats => "index_stats",
            McpTool::TraceImports => "trace_imports",
            McpTool::FindCallers => "find_callers",
            McpTool::ProjectOverview => "project_overview",
        }
    }

    pub fn all() -> &'static [McpTool] {
        &[
            McpTool::SearchText,
            McpTool::FindFiles,
            McpTool::ExtractSymbols,
            McpTool::ListLanguages,
            McpTool::LookupSymbol,
            McpTool::IndexFile,
            McpTool::IndexStats,
            McpTool::TraceImports,
            McpTool::FindCallers,
            McpTool::ProjectOverview,
        ]
    }
}

/// Number of MCP tools exposed.
pub const MCP_TOOL_COUNT: usize = 10;

pub struct CodeSearchMcp {
    graph: std::sync::Mutex<KnowledgeGraph>,
    index: std::sync::Mutex<CodeIndex>,
}

impl CodeSearchMcp {
    pub fn new_in_memory() -> Self {
        Self {
            graph: std::sync::Mutex::new(KnowledgeGraph::new()),
            index: std::sync::Mutex::new(CodeIndex::open_in_memory().expect("in-memory index")),
        }
    }

    pub fn handle(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "apeireth-codesearch", "version": "1.2.0"},
                    "capabilities": {"tools": {}}
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<_> = McpTool::all().iter().map(|t| json!({
                    "name": t.name(),
                    "description": match t {
                        McpTool::SearchText => "Search text across files (regex/literal/multi-pattern)",
                        McpTool::FindFiles => "Find files matching glob patterns",
                        McpTool::ExtractSymbols => "Extract symbols from a single file",
                        McpTool::ListLanguages => "List supported programming languages",
                        McpTool::LookupSymbol => "Lookup symbols by name in persistent index",
                        McpTool::IndexFile => "Add a file to the persistent index",
                        McpTool::IndexStats => "Get persistent index statistics",
                        McpTool::TraceImports => "Trace imports for a file via knowledge graph",
                        McpTool::FindCallers => "Find callers of a symbol (knowledge graph)",
                        McpTool::ProjectOverview => "Get project structure overview",
                    }
                })).collect();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(json!({"tools": tools})),
                    error: None,
                }
            }
            "tools/call" => {
                let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
                match tool_name {
                    "list_languages" => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(json!({
                            "content": [{"type": "text", "text": format!("supported: {:?}", supported_languages())}],
                            "isError": false
                        })),
                        error: None,
                    },
                    "index_stats" => {
                        let idx = self.index.lock().expect("poisoned");
                        let f = idx.file_count().unwrap_or(0);
                        let s = idx.symbol_count().unwrap_or(0);
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": format!("files={f} symbols={s}")}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "search_text" => {
                        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
                        let kind_str = args.get("kind").and_then(|v| v.as_str()).unwrap_or("literal");
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                        let kind = match kind_str {
                            "regex" => SearchKind::Regex,
                            "multi" => SearchKind::MultiPattern,
                            _ => SearchKind::Literal,
                        };
                        let searcher = CodeSearcher::new();
                        let finder = FileFinder::new();
                        let files = match finder.find(path, &FindOptions::default()) {
                            Ok(f) => f.into_iter().filter(|e| !e.is_dir).collect::<Vec<_>>(),
                            Err(_) => Vec::new(),
                        };
                        let mut all_matches = Vec::new();
                        for f in &files {
                            if let Ok(matches) = searcher.search_file(
                                std::path::Path::new(&f.path),
                                kind,
                                pattern,
                                &SearchOptions::default(),
                            ) {
                                for m in matches {
                                    all_matches.push(format!("{}:{}:{} {}", m.file, m.line, m.column, m.text.trim()));
                                }
                            }
                        }
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": all_matches.join("\n")}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "find_files" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                        let ext = args.get("extension").and_then(|v| v.as_str());
                        let finder = FileFinder::new();
                        let entries = match ext {
                            Some(e) => finder.find_with_extension(path, e).unwrap_or_default(),
                            None => finder.find(path, &FindOptions::default()).unwrap_or_default(),
                        };
                        let list: Vec<String> = entries.iter().filter(|e| !e.is_dir).map(|e| e.path.clone()).collect();
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": list.join("\n")}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "extract_symbols" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let path_obj = std::path::Path::new(path);
                        let lang = detect_language(path_obj).unwrap_or("");
                        let content = std::fs::read_to_string(path).unwrap_or_default();
                        let symbols = extract_symbols(&content, lang);
                        let summary: Vec<String> = symbols.iter()
                            .map(|s| format!("{} {}:{} {}", s.kind.as_str(), s.line, s.column, s.name))
                            .collect();
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": summary.join("\n")}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "lookup_symbol" => {
                        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let idx = self.index.lock().expect("poisoned");
                        let entries = idx.lookup_symbols_by_name(name).unwrap_or_default();
                        let list: Vec<String> = entries.iter().map(|e| format!("{} (id={})", e.path, e.id)).collect();
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": list.join("\n")}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "index_file" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let path_obj = std::path::Path::new(path);
                        let lang = detect_language(path_obj).unwrap_or("");
                        let content = std::fs::read_to_string(path).unwrap_or_default();
                        let symbols = extract_symbols(&content, lang);
                        let idx = self.index.lock().expect("poisoned");
                        let file_id = idx.upsert_file(path).unwrap_or(0);
                        let mut count = 0;
                        for s in &symbols {
                            if idx.insert_symbol(file_id, s).is_ok() {
                                count += 1;
                            }
                            if s.kind == crate::symbols::SymbolKind::Import {
                                let _ = idx.insert_import(file_id, &s.name);
                            }
                        }
                        // Add to graph
                        let mut graph = self.graph.lock().expect("poisoned");
                        graph.add_file(path);
                        for s in &symbols {
                            graph.add_symbol(path, s);
                            if s.kind == crate::symbols::SymbolKind::Import {
                                graph.add_import(path, &s.name);
                            }
                        }
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": format!("indexed file_id={} symbols={}", file_id, count)}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "trace_imports" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let graph = self.graph.lock().expect("poisoned");
                        let imports: Vec<String> = graph.edges()
                            .filter(|(from, edge, _)| from == &format!("file:{}", path) && *edge == crate::graph::GraphEdge::Imports)
                            .map(|(_, _, to)| to.clone())
                            .collect();
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": if imports.is_empty() { "(no imports indexed)".to_string() } else { imports.join("\n") }}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "find_callers" => {
                        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let graph = self.graph.lock().expect("poisoned");
                        // Find symbols named `name` and list files referencing them via imports
                        let sym_ids: Vec<String> = graph.nodes()
                            .filter(|n| n.kind == crate::graph::NodeKind::Symbol && n.label.contains(name))
                            .map(|n| n.id.clone())
                            .collect();
                        let callers: Vec<String> = graph.edges()
                            .filter(|(_, edge, to)| *edge == crate::graph::GraphEdge::Imports && sym_ids.contains(to))
                            .map(|(from, _, _)| from.clone())
                            .collect();
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": if callers.is_empty() { "(no callers indexed)".to_string() } else { callers.join("\n") }}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    "project_overview" => {
                        let idx = self.index.lock().expect("poisoned");
                        let graph = self.graph.lock().expect("poisoned");
                        let f = idx.file_count().unwrap_or(0);
                        let s = idx.symbol_count().unwrap_or(0);
                        let summary = format!(
                            "files={} symbols={} graph_nodes={} graph_edges={}",
                            f, s, graph.node_count(), graph.edge_count()
                        );
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(json!({
                                "content": [{"type": "text", "text": summary}],
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    other => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(McpError { code: -32602, message: format!("unknown tool: {}", other) }),
                    },
                }
            }
            "ping" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({})),
                error: None,
            },
            other => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(McpError { code: -32601, message: format!("method not found: {}", other) }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_count_is_10() {
        assert_eq!(MCP_TOOL_COUNT, 10);
        assert_eq!(McpTool::all().len(), 10);
    }

    #[test]
    fn initialize_protocol_version() {
        let mcp = CodeSearchMcp::new_in_memory();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = mcp.handle(req);
        assert!(resp.result.is_some());
        let r = resp.result.unwrap();
        assert_eq!(r["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn tools_list_returns_10() {
        let mcp = CodeSearchMcp::new_in_memory();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = mcp.handle(req);
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn list_languages() {
        let mcp = CodeSearchMcp::new_in_memory();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({"name": "list_languages", "arguments": {}}),
        };
        let resp = mcp.handle(req);
        let result = resp.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("rust"));
        assert!(text.contains("python"));
    }

    #[test]
    fn index_stats_initial() {
        let mcp = CodeSearchMcp::new_in_memory();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({"name": "index_stats", "arguments": {}}),
        };
        let resp = mcp.handle(req);
        let result = resp.result.unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("files=0"));
        assert!(text.contains("symbols=0"));
    }

    #[test]
    fn unknown_tool_errors() {
        let mcp = CodeSearchMcp::new_in_memory();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "tools/call".to_string(),
            params: json!({"name": "nope", "arguments": {}}),
        };
        let resp = mcp.handle(req);
        assert!(resp.error.is_some());
    }
}