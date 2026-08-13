//! Symbol extraction (regex-based, NOT full AST parser).
//!
//! Honest scope per O-5 不假装:
//! - 5 languages supported (Rust / Python / JS / TS / Go)
//! - Regex patterns for common declarations
//! - NOT a full AST parser — mis-parses pathological syntax
//! - For full AST support, use tree-sitter (deferred R140+; compile cost)
//!
//! Per v2 plan §9.4 Decision 1, we borrow **codebase-memory-mcp** (158 langs,
//! 6768 tests, arXiv paper) design but implement in Rust regex (lighter).
//! Adding a language requires a new entry in `extract_symbols`.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Class,
    Method,
    Variable,
    Constant,
    Interface,
    Struct,
    Enum,
    Module,
    Import,
}

impl SymbolKind {
    pub fn as_str(&self) -> &str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Class => "class",
            SymbolKind::Method => "method",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Interface => "interface",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Module => "module",
            SymbolKind::Import => "import",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
    pub language: String,
    /// Full signature / declaration line
    pub signature: String,
}

pub fn supported_languages() -> &'static [&'static str] {
    &["rust", "python", "javascript", "typescript", "go"]
}

/// Detect language from file extension.
pub fn detect_language(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "go" => Some("go"),
        _ => None,
    }
}

pub fn extract_symbols(content: &str, language: &str) -> Vec<Symbol> {
    match language {
        "rust" => extract_rust(content),
        "python" => extract_python(content),
        "javascript" => extract_javascript(content),
        "typescript" => extract_typescript(content),
        "go" => extract_go(content),
        _ => Vec::new(),
    }
}

fn extract_rust(content: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let fn_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+|unsafe\s+|extern\s+(?:\x22[^\x22]*\x22\s+)?)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let struct_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
    let enum_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
    let const_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+([A-Z_][A-Z0-9_]*)").unwrap();
    let use_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Function, line: i + 1, column: 1, language: "rust".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = struct_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Struct, line: i + 1, column: 1, language: "rust".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = enum_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Enum, line: i + 1, column: 1, language: "rust".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Constant, line: i + 1, column: 1, language: "rust".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = use_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().trim().to_string(), kind: SymbolKind::Import, line: i + 1, column: 1, language: "rust".to_string(), signature: line.to_string() });
            }
        }
    }
    out
}

fn extract_python(content: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let fn_re = Regex::new(r"(?m)^\s*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let class_re = Regex::new(r"(?m)^\s*class\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
    let const_re = Regex::new(r"(?m)^([A-Z_][A-Z0-9_]*)\s*=").unwrap();
    let import_re = Regex::new(r"(?m)^\s*(?:from\s+[\w.]+\s+)?import\s+([^\n]+)").unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Function, line: i + 1, column: 1, language: "python".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = class_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Class, line: i + 1, column: 1, language: "python".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Constant, line: i + 1, column: 1, language: "python".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = import_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().trim().to_string(), kind: SymbolKind::Import, line: i + 1, column: 1, language: "python".to_string(), signature: line.to_string() });
            }
        }
    }
    out
}

fn extract_javascript(content: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let fn_re = Regex::new(r"(?m)^\s*(?:async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap();
    let class_re = Regex::new(r"(?m)^\s*class\s+([A-Z][a-zA-Z0-9_$]*)").unwrap();
    let const_re = Regex::new(r"(?m)^\s*const\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=").unwrap();
    let import_re = Regex::new(r#"(?m)^\s*import\s+(?:\{([^}]+)\}\s+from\s+)?['"]([^'"]+)['"]"#).unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Function, line: i + 1, column: 1, language: "javascript".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = class_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Class, line: i + 1, column: 1, language: "javascript".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Constant, line: i + 1, column: 1, language: "javascript".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = import_re.captures(line) {
            let name = c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| {
                c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default()
            });
            if !name.is_empty() {
                out.push(Symbol { name, kind: SymbolKind::Import, line: i + 1, column: 1, language: "javascript".to_string(), signature: line.to_string() });
            }
        }
    }
    out
}

fn extract_typescript(content: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let fn_re = Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap();
    let class_re = Regex::new(r"(?m)^\s*(?:export\s+)?class\s+([A-Z][a-zA-Z0-9_$]*)").unwrap();
    let iface_re = Regex::new(r"(?m)^\s*(?:export\s+)?interface\s+([A-Z][a-zA-Z0-9_$]*)").unwrap();
    let const_re = Regex::new(r"(?m)^\s*(?:export\s+)?const\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*[=:]").unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Function, line: i + 1, column: 1, language: "typescript".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = class_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Class, line: i + 1, column: 1, language: "typescript".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = iface_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Interface, line: i + 1, column: 1, language: "typescript".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Constant, line: i + 1, column: 1, language: "typescript".to_string(), signature: line.to_string() });
            }
        }
    }
    out
}

fn extract_go(content: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let fn_re = Regex::new(r"(?m)^\s*func\s+(?:\([^)]+\)\s+)?([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let struct_re = Regex::new(r"(?m)^\s*type\s+([A-Z][a-zA-Z0-9_]*)\s+struct").unwrap();
    let iface_re = Regex::new(r"(?m)^\s*type\s+([A-Z][a-zA-Z0-9_]*)\s+interface").unwrap();
    let const_re = Regex::new(r"(?m)^\s*(?:const|var)\s+([A-Z_][A-Z0-9_]*)\s*=").unwrap();
    let import_re = Regex::new(r#"(?m)^\s*import\s+(?:\(\s*)?(?:[\x22]([^\x22]+)[\x22]|`([^`]+)`)"#).unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Function, line: i + 1, column: 1, language: "go".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = struct_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Struct, line: i + 1, column: 1, language: "go".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = iface_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Interface, line: i + 1, column: 1, language: "go".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol { name: name.as_str().to_string(), kind: SymbolKind::Constant, line: i + 1, column: 1, language: "go".to_string(), signature: line.to_string() });
            }
        } else if let Some(c) = import_re.captures(line) {
            let name = c.get(1).or_else(|| c.get(2)).map(|m| m.as_str().to_string()).unwrap_or_default();
            if !name.is_empty() {
                out.push(Symbol { name, kind: SymbolKind::Import, line: i + 1, column: 1, language: "go".to_string(), signature: line.to_string() });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language(Path::new("foo.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("foo.py")), Some("python"));
        assert_eq!(detect_language(Path::new("foo.js")), Some("javascript"));
        assert_eq!(detect_language(Path::new("foo.ts")), Some("typescript"));
        assert_eq!(detect_language(Path::new("foo.go")), Some("go"));
        assert_eq!(detect_language(Path::new("foo.md")), None);
    }

    #[test]
    fn rust_fn_extraction() {
        let content = "pub fn hello() {}\nfn world() {}\npub async fn async_fn() {}";
        let s = extract_rust(content);
        assert!(s.iter().any(|x| x.name == "hello" && x.kind == SymbolKind::Function));
        assert!(s.iter().any(|x| x.name == "world" && x.kind == SymbolKind::Function));
        assert!(s.iter().any(|x| x.name == "async_fn"));
    }

    #[test]
    fn rust_struct_extraction() {
        let content = "pub struct Foo {}\nstruct Bar {}";
        let s = extract_rust(content);
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|x| x.kind == SymbolKind::Struct));
    }

    #[test]
    fn python_class_and_def() {
        let content = "def helper(): pass\nclass MyClass:\n    def method(self): pass\nMY_CONST = 1";
        let s = extract_python(content);
        assert!(s.iter().any(|x| x.name == "helper" && x.kind == SymbolKind::Function));
        assert!(s.iter().any(|x| x.name == "MyClass" && x.kind == SymbolKind::Class));
        assert!(s.iter().any(|x| x.name == "MY_CONST"));
    }

    #[test]
    fn javascript_class() {
        let content = "class Foo {}\nfunction bar() {}\nconst baz = 1;";
        let s = extract_javascript(content);
        assert!(s.iter().any(|x| x.name == "Foo" && x.kind == SymbolKind::Class));
        assert!(s.iter().any(|x| x.name == "bar"));
        assert!(s.iter().any(|x| x.name == "baz" && x.kind == SymbolKind::Constant));
    }

    #[test]
    fn typescript_interface() {
        let content = "export interface Foo {}\nexport const x: number = 1;";
        let s = extract_typescript(content);
        assert!(s.iter().any(|x| x.name == "Foo" && x.kind == SymbolKind::Interface));
        assert!(s.iter().any(|x| x.name == "x" && x.kind == SymbolKind::Constant));
    }

    #[test]
    fn go_struct() {
        let content = "type Foo struct {}\ntype Bar interface {}\nfunc hello() {}\nconst MAX = 10";
        let s = extract_go(content);
        assert!(s.iter().any(|x| x.name == "Foo" && x.kind == SymbolKind::Struct));
        assert!(s.iter().any(|x| x.name == "Bar" && x.kind == SymbolKind::Interface));
        assert!(s.iter().any(|x| x.name == "hello" && x.kind == SymbolKind::Function));
        assert!(s.iter().any(|x| x.name == "MAX" && x.kind == SymbolKind::Constant));
    }

    #[test]
    fn supported_languages_count() {
        assert_eq!(supported_languages().len(), 5);
    }

    #[test]
    fn unknown_language_returns_empty() {
        let s = extract_symbols("hello", "cobol");
        assert!(s.is_empty());
    }

    #[test]
    fn empty_content_returns_empty() {
        assert!(extract_rust("").is_empty());
        assert!(extract_python("").is_empty());
    }
}