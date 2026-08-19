//! Integration tests for apeireth-tool-codesearch (post-1.0.0)
//!
//! src/ 8 module 真实现 (search/files/symbols/graph/index/mcp/compat/enhanced).
//! 这里 (tests/) 加跨 API 集成 + 5 语言 symbol 提取 + 边界.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_tool_codesearch::symbols::detect_language;
use apeireth_tool_codesearch::{
    extract_symbols, supported_languages, CodeSearchCommand, CodeSearchCompatRouter, GraphEdge,
    GraphNode, KnowledgeGraph, NodeKind, Symbol, SymbolKind, CODESEARCH_COMMAND_COUNT,
    R140_DELIVERABLES, SUPPORTED_LANGS,
};
use std::path::Path;

// =============================================================================
// Constants
// =============================================================================

#[test]
fn r140_deliverables_count() {
    assert_eq!(R140_DELIVERABLES, 8);
}

#[test]
fn codesearch_command_count() {
    assert_eq!(CODESEARCH_COMMAND_COUNT, 3);
}

#[test]
fn supported_langs_5() {
    assert_eq!(SUPPORTED_LANGS.len(), 5);
    assert_eq!(supported_languages().len(), 5);
    // 集合内容一致
    for lang in SUPPORTED_LANGS {
        assert!(supported_languages().contains(lang));
    }
}

// =============================================================================
// detect_language
// =============================================================================

#[test]
fn detect_language_basic_extensions() {
    assert_eq!(detect_language(Path::new("foo.rs")), Some("rust"));
    assert_eq!(detect_language(Path::new("foo.py")), Some("python"));
    assert_eq!(detect_language(Path::new("foo.js")), Some("javascript"));
    assert_eq!(detect_language(Path::new("foo.ts")), Some("typescript"));
    assert_eq!(detect_language(Path::new("foo.go")), Some("go"));
}

#[test]
fn detect_language_unknown_returns_none() {
    assert_eq!(detect_language(Path::new("foo.md")), None);
    assert_eq!(detect_language(Path::new("foo.txt")), None);
    assert_eq!(detect_language(Path::new("foo")), None);
}

#[test]
fn detect_language_alternate_extensions() {
    assert_eq!(detect_language(Path::new("foo.mjs")), Some("javascript"));
    assert_eq!(detect_language(Path::new("foo.tsx")), Some("typescript"));
}

#[test]
fn detect_language_no_extension() {
    assert_eq!(detect_language(Path::new("foo")), None);
    assert_eq!(detect_language(Path::new("")), None);
}

// =============================================================================
// SymbolKind
// =============================================================================

#[test]
fn symbol_kind_as_str() {
    assert_eq!(SymbolKind::Function.as_str(), "function");
    assert_eq!(SymbolKind::Class.as_str(), "class");
    assert_eq!(SymbolKind::Struct.as_str(), "struct");
    assert_eq!(SymbolKind::Enum.as_str(), "enum");
    assert_eq!(SymbolKind::Constant.as_str(), "constant");
    assert_eq!(SymbolKind::Interface.as_str(), "interface");
    assert_eq!(SymbolKind::Module.as_str(), "module");
    assert_eq!(SymbolKind::Import.as_str(), "import");
}

#[test]
fn symbol_kind_eq_copy_hash() {
    let k = SymbolKind::Function;
    let k2 = k;
    assert_eq!(k, k2);
    let mut set = std::collections::HashSet::new();
    set.insert(k);
    set.insert(k2);
    set.insert(SymbolKind::Struct);
    assert_eq!(set.len(), 2);
}

// =============================================================================
// extract_symbols - rust
// =============================================================================

#[test]
fn rust_extract_function_basic() {
    let content = "pub fn hello() {}\nfn world() {}\n";
    let s = extract_symbols(content, "rust");
    let names: Vec<&str> = s.iter().map(|x| x.name.as_str()).collect();
    assert!(names.contains(&"hello"));
    assert!(names.contains(&"world"));
}

#[test]
fn rust_extract_async_fn() {
    let content = "pub async fn fetch() {}\n";
    let s = extract_symbols(content, "rust");
    assert!(s.iter().any(|x| x.name == "fetch"));
}

#[test]
fn rust_extract_struct() {
    let content = "pub struct Foo {}\nstruct Bar {}";
    let s = extract_symbols(content, "rust");
    assert_eq!(s.len(), 2);
    assert!(s.iter().all(|x| x.kind == SymbolKind::Struct));
}

#[test]
fn rust_extract_enum() {
    let content = "enum Status { Active, Done }";
    let s = extract_symbols(content, "rust");
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].name, "Status");
    assert_eq!(s[0].kind, SymbolKind::Enum);
}

#[test]
fn rust_extract_const() {
    let content = "const MAX_RETRIES: usize = 3;";
    let s = extract_symbols(content, "rust");
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, SymbolKind::Constant);
}

#[test]
fn rust_extract_use() {
    let content = "use std::collections::HashMap;";
    let s = extract_symbols(content, "rust");
    assert!(s.iter().any(|x| x.kind == SymbolKind::Import));
}

#[test]
fn rust_extract_line_numbering() {
    let content = "fn a() {}\n\nfn b() {}\n";
    let s = extract_symbols(content, "rust");
    let a = s.iter().find(|x| x.name == "a").unwrap();
    let b = s.iter().find(|x| x.name == "b").unwrap();
    assert_eq!(a.line, 1);
    assert_eq!(b.line, 3);
}

// =============================================================================
// extract_symbols - python
// =============================================================================

#[test]
fn python_extract_def() {
    let content = "def helper(): pass\n";
    let s = extract_symbols(content, "python");
    assert!(s
        .iter()
        .any(|x| x.name == "helper" && x.kind == SymbolKind::Function));
}

#[test]
fn python_extract_class() {
    let content = "class MyClass:\n    pass\n";
    let s = extract_symbols(content, "python");
    assert!(s
        .iter()
        .any(|x| x.name == "MyClass" && x.kind == SymbolKind::Class));
}

#[test]
fn python_extract_const() {
    let content = "MY_CONST = 1\n";
    let s = extract_symbols(content, "python");
    assert!(s.iter().any(|x| x.name == "MY_CONST"));
}

#[test]
fn python_extract_import() {
    let content = "import os\nfrom sys import argv\n";
    let s = extract_symbols(content, "python");
    assert!(s.iter().any(|x| x.kind == SymbolKind::Import));
}

// =============================================================================
// extract_symbols - javascript
// =============================================================================

#[test]
fn javascript_extract_class() {
    let content = "class Foo {}\n";
    let s = extract_symbols(content, "javascript");
    assert!(s
        .iter()
        .any(|x| x.name == "Foo" && x.kind == SymbolKind::Class));
}

#[test]
fn javascript_extract_function() {
    let content = "function bar() {}\n";
    let s = extract_symbols(content, "javascript");
    assert!(s.iter().any(|x| x.name == "bar"));
}

#[test]
fn javascript_extract_const() {
    let content = "const baz = 1;\n";
    let s = extract_symbols(content, "javascript");
    assert!(s
        .iter()
        .any(|x| x.name == "baz" && x.kind == SymbolKind::Constant));
}

#[test]
fn javascript_extract_async_function() {
    let content = "async function fetch() {}\n";
    let s = extract_symbols(content, "javascript");
    assert!(s.iter().any(|x| x.name == "fetch"));
}

// =============================================================================
// extract_symbols - typescript
// =============================================================================

#[test]
fn typescript_extract_interface() {
    let content = "export interface Foo {}\n";
    let s = extract_symbols(content, "typescript");
    assert!(s
        .iter()
        .any(|x| x.name == "Foo" && x.kind == SymbolKind::Interface));
}

#[test]
fn typescript_extract_class() {
    let content = "export class Bar {}\n";
    let s = extract_symbols(content, "typescript");
    assert!(s
        .iter()
        .any(|x| x.name == "Bar" && x.kind == SymbolKind::Class));
}

#[test]
fn typescript_extract_const() {
    let content = "export const x: number = 1;\n";
    let s = extract_symbols(content, "typescript");
    assert!(s
        .iter()
        .any(|x| x.name == "x" && x.kind == SymbolKind::Constant));
}

// =============================================================================
// extract_symbols - go
// =============================================================================

#[test]
fn go_extract_struct() {
    let content = "type Foo struct {}\n";
    let s = extract_symbols(content, "go");
    assert!(s
        .iter()
        .any(|x| x.name == "Foo" && x.kind == SymbolKind::Struct));
}

#[test]
fn go_extract_interface() {
    let content = "type Bar interface {}\n";
    let s = extract_symbols(content, "go");
    assert!(s
        .iter()
        .any(|x| x.name == "Bar" && x.kind == SymbolKind::Interface));
}

#[test]
fn go_extract_function() {
    let content = "func hello() {}\n";
    let s = extract_symbols(content, "go");
    assert!(s
        .iter()
        .any(|x| x.name == "hello" && x.kind == SymbolKind::Function));
}

#[test]
fn go_extract_const() {
    let content = "const MAX = 10\n";
    let s = extract_symbols(content, "go");
    assert!(s
        .iter()
        .any(|x| x.name == "MAX" && x.kind == SymbolKind::Constant));
}

// =============================================================================
// extract_symbols - boundaries
// =============================================================================

#[test]
fn extract_unknown_language_returns_empty() {
    let s = extract_symbols("hello", "cobol");
    assert!(s.is_empty());
}

#[test]
fn extract_empty_content_returns_empty() {
    assert!(extract_symbols("", "rust").is_empty());
    assert!(extract_symbols("", "python").is_empty());
    assert!(extract_symbols("", "javascript").is_empty());
}

#[test]
fn extract_no_matches_returns_empty() {
    let s = extract_symbols("// just a comment\n", "rust");
    // rust extract 有 use_re, 但不含 fn/struct — 应少于 1
    assert!(s.is_empty() || s.iter().all(|x| x.kind != SymbolKind::Function));
}

// =============================================================================
// Symbol struct
// =============================================================================

#[test]
fn symbol_struct_clone() {
    let s = Symbol {
        name: "x".into(),
        kind: SymbolKind::Function,
        line: 1,
        column: 1,
        language: "rust".into(),
        signature: "fn x() {}".into(),
    };
    let s2 = s.clone();
    assert_eq!(s.name, s2.name);
    assert_eq!(s.kind, s2.kind);
    assert_eq!(s.line, s2.line);
}

// =============================================================================
// NodeKind / GraphEdge
// =============================================================================

#[test]
fn node_kind_variants() {
    assert_ne!(NodeKind::File, NodeKind::Symbol);
}

#[test]
fn graph_edge_as_str() {
    assert_eq!(GraphEdge::DefinedIn.as_str(), "defined_in");
    assert_eq!(GraphEdge::Imports.as_str(), "imports");
    assert_eq!(GraphEdge::Calls.as_str(), "calls");
}

#[test]
fn graph_edge_eq_copy_hash() {
    let e = GraphEdge::DefinedIn;
    let e2 = e;
    assert_eq!(e, e2);
}

// =============================================================================
// KnowledgeGraph
// =============================================================================

#[test]
fn graph_new_empty() {
    let g = KnowledgeGraph::new();
    assert_eq!(g.node_count(), 0);
    assert_eq!(g.edge_count(), 0);
}

#[test]
fn graph_default_empty() {
    let g = KnowledgeGraph::default();
    assert_eq!(g.node_count(), 0);
}

#[test]
fn graph_add_file() {
    let mut g = KnowledgeGraph::new();
    let id = g.add_file("src/main.rs");
    assert_eq!(g.node_count(), 1);
    assert_eq!(g.edges().count(), 0);
    assert!(id.starts_with("file:"));
}

#[test]
fn graph_add_symbol_with_edge() {
    let mut g = KnowledgeGraph::new();
    g.add_file("src/lib.rs");
    let s = Symbol {
        name: "foo".into(),
        kind: SymbolKind::Function,
        line: 1,
        column: 1,
        language: "rust".into(),
        signature: "fn foo() {}".into(),
    };
    let sym_id = g.add_symbol("src/lib.rs", &s);
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.edge_count(), 1);
    assert!(sym_id.starts_with("sym:"));
    let edge = g.edges().next().unwrap();
    assert_eq!(edge.1, GraphEdge::DefinedIn);
}

#[test]
fn graph_add_import_edge() {
    let mut g = KnowledgeGraph::new();
    g.add_file("src/main.rs");
    g.add_file("src/lib.rs");
    g.add_import("src/main.rs", "src/lib.rs");
    assert_eq!(g.edge_count(), 1);
    let edge = g.edges().next().unwrap();
    assert_eq!(edge.1, GraphEdge::Imports);
}

#[test]
fn graph_find_symbols_in_file() {
    let mut g = KnowledgeGraph::new();
    g.add_file("src/lib.rs");
    g.add_symbol("src/lib.rs", &make_sym("a", SymbolKind::Function));
    g.add_symbol("src/lib.rs", &make_sym("b", SymbolKind::Function));
    g.add_file("src/main.rs");
    g.add_symbol("src/main.rs", &make_sym("c", SymbolKind::Function));
    let in_lib = g.find_symbols_in_file("src/lib.rs");
    assert_eq!(in_lib.len(), 2);
    let in_main = g.find_symbols_in_file("src/main.rs");
    assert_eq!(in_main.len(), 1);
}

#[test]
fn graph_find_symbols_empty() {
    let g = KnowledgeGraph::new();
    assert!(g.find_symbols_in_file("nope.rs").is_empty());
}

#[test]
fn graph_multiple_files() {
    let mut g = KnowledgeGraph::new();
    for f in ["src/a.rs", "src/b.rs", "src/c.rs"] {
        g.add_file(f);
        g.add_symbol(f, &make_sym(&format!("f_{}", f), SymbolKind::Function));
    }
    assert_eq!(g.node_count(), 6); // 3 files + 3 symbols
    assert_eq!(g.edge_count(), 3); // 3 defined_in
}

#[test]
fn graph_node_iter() {
    let mut g = KnowledgeGraph::new();
    g.add_file("a.rs");
    g.add_file("b.rs");
    let count = g.nodes().count();
    assert_eq!(count, 2);
}

// =============================================================================
// CodeSearchCommand
// =============================================================================

#[test]
fn codesearch_command_from_str_3() {
    for s in ["CodeSearch", "RepoInspector", "CodeAnalyzer"] {
        assert_ne!(CodeSearchCommand::from_str(s), CodeSearchCommand::Unknown);
    }
}

#[test]
fn codesearch_command_unknown_fallback() {
    assert_eq!(
        CodeSearchCommand::from_str("xyz"),
        CodeSearchCommand::Unknown
    );
    assert_eq!(CodeSearchCommand::from_str(""), CodeSearchCommand::Unknown);
}

#[test]
fn codesearch_command_eq_hash() {
    let a = CodeSearchCommand::CodeSearch;
    let b = CodeSearchCommand::CodeSearch;
    let c = CodeSearchCommand::RepoInspector;
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn codesearch_command_to_mcp_tool() {
    assert!(CodeSearchCommand::CodeSearch.to_mcp_tool().is_some());
    assert!(CodeSearchCommand::RepoInspector.to_mcp_tool().is_some());
    assert!(CodeSearchCommand::CodeAnalyzer.to_mcp_tool().is_some());
    assert!(CodeSearchCommand::Unknown.to_mcp_tool().is_none());
}

// =============================================================================
// CodeSearchCompatRouter
// =============================================================================

#[test]
fn codesearch_router_count() {
    assert_eq!(CodeSearchCompatRouter::command_count(), 3);
}

#[test]
fn codesearch_router_default() {
    let _r = CodeSearchCompatRouter::default();
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_extract_then_graph() {
    // 模拟 index 流程: detect language → extract symbols → add to graph
    let content = "pub fn hello() {}\npub fn world() {}\nstruct Foo {}";
    let syms = extract_symbols(content, "rust");
    let mut g = KnowledgeGraph::new();
    g.add_file("src/main.rs");
    for s in &syms {
        g.add_symbol("src/main.rs", s);
    }
    assert!(g.node_count() >= 3); // 1 file + 3 symbols
    let in_file = g.find_symbols_in_file("src/main.rs");
    assert_eq!(in_file.len(), 3);
}

#[test]
fn integration_cross_language_graph() {
    let mut g = KnowledgeGraph::new();
    g.add_file("src/lib.rs");
    g.add_file("scripts/build.py");
    g.add_file("web/app.ts");

    let rust_syms = extract_symbols("pub fn r_fn() {}", "rust");
    let py_syms = extract_symbols("def py_fn(): pass", "python");
    let ts_syms = extract_symbols("export function ts_fn() {}", "typescript");

    g.add_symbol("src/lib.rs", &rust_syms[0]);
    g.add_symbol("scripts/build.py", &py_syms[0]);
    g.add_symbol("web/app.ts", &ts_syms[0]);

    assert_eq!(g.node_count(), 6); // 3 files + 3 symbols
    let rust_in_lib = g.find_symbols_in_file("src/lib.rs");
    assert_eq!(rust_in_lib.len(), 1);
    let lang = rust_in_lib[0].metadata.get("language").unwrap();
    assert_eq!(lang, "rust");
}

#[test]
fn integration_supported_languages_match_constants() {
    let langs = supported_languages();
    assert_eq!(langs.len(), SUPPORTED_LANGS.len());
    for lang in langs {
        assert!(SUPPORTED_LANGS.contains(lang));
    }
}

#[test]
fn integration_detect_then_extract() {
    // 真实场景: 文件 path → language → extract symbols
    let paths_and_content = [
        ("src/lib.rs", "pub fn hello() {}"),
        ("src/lib.py", "def hello(): pass"),
        ("src/lib.js", "function hello() {}"),
        ("src/lib.ts", "export function hello() {}"),
        ("src/lib.go", "func hello() {}"),
    ];
    for (path_str, content) in paths_and_content {
        let path = Path::new(path_str);
        let lang = detect_language(path).expect("lang detected");
        let syms = extract_symbols(content, lang);
        assert!(
            syms.iter().any(|s| s.name == "hello"),
            "{path_str} 应提取 hello: got {syms:?}"
        );
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn make_sym(name: &str, kind: SymbolKind) -> Symbol {
    Symbol {
        name: name.into(),
        kind,
        line: 1,
        column: 1,
        language: "rust".into(),
        signature: format!("fn {name}() {{}}"),
    }
}
