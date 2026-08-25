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
    // P2#9: feature "tree-sitter" 开启时, rust 语言优先精确路径 (解析失败降级 regex)
    #[cfg(feature = "tree-sitter")]
    if language == "rust" {
        let ts = extract_symbols_tree_sitter(content);
        if !ts.is_empty() {
            return ts;
        }
    }
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
    let struct_re =
        Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
    let enum_re =
        Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
    let const_re =
        Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+([A-Z_][A-Z0-9_]*)").unwrap();
    let use_re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line: i + 1,
                    column: 1,
                    language: "rust".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = struct_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    line: i + 1,
                    column: 1,
                    language: "rust".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = enum_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Enum,
                    line: i + 1,
                    column: 1,
                    language: "rust".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Constant,
                    line: i + 1,
                    column: 1,
                    language: "rust".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = use_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().trim().to_string(),
                    kind: SymbolKind::Import,
                    line: i + 1,
                    column: 1,
                    language: "rust".to_string(),
                    signature: line.to_string(),
                });
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
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line: i + 1,
                    column: 1,
                    language: "python".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = class_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Class,
                    line: i + 1,
                    column: 1,
                    language: "python".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Constant,
                    line: i + 1,
                    column: 1,
                    language: "python".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = import_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().trim().to_string(),
                    kind: SymbolKind::Import,
                    line: i + 1,
                    column: 1,
                    language: "python".to_string(),
                    signature: line.to_string(),
                });
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
    let import_re =
        Regex::new(r#"(?m)^\s*import\s+(?:\{([^}]+)\}\s+from\s+)?['"]([^'"]+)['"]"#).unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line: i + 1,
                    column: 1,
                    language: "javascript".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = class_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Class,
                    line: i + 1,
                    column: 1,
                    language: "javascript".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Constant,
                    line: i + 1,
                    column: 1,
                    language: "javascript".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = import_re.captures(line) {
            let name = c
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default());
            if !name.is_empty() {
                out.push(Symbol {
                    name,
                    kind: SymbolKind::Import,
                    line: i + 1,
                    column: 1,
                    language: "javascript".to_string(),
                    signature: line.to_string(),
                });
            }
        }
    }
    out
}

fn extract_typescript(content: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let fn_re =
        Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)")
            .unwrap();
    let class_re = Regex::new(r"(?m)^\s*(?:export\s+)?class\s+([A-Z][a-zA-Z0-9_$]*)").unwrap();
    let iface_re = Regex::new(r"(?m)^\s*(?:export\s+)?interface\s+([A-Z][a-zA-Z0-9_$]*)").unwrap();
    let const_re =
        Regex::new(r"(?m)^\s*(?:export\s+)?const\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*[=:]").unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line: i + 1,
                    column: 1,
                    language: "typescript".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = class_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Class,
                    line: i + 1,
                    column: 1,
                    language: "typescript".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = iface_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Interface,
                    line: i + 1,
                    column: 1,
                    language: "typescript".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Constant,
                    line: i + 1,
                    column: 1,
                    language: "typescript".to_string(),
                    signature: line.to_string(),
                });
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
    let import_re =
        Regex::new(r#"(?m)^\s*import\s+(?:\(\s*)?(?:[\x22]([^\x22]+)[\x22]|`([^`]+)`)"#).unwrap();

    for (i, line) in content.lines().enumerate() {
        if let Some(c) = fn_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line: i + 1,
                    column: 1,
                    language: "go".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = struct_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    line: i + 1,
                    column: 1,
                    language: "go".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = iface_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Interface,
                    line: i + 1,
                    column: 1,
                    language: "go".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = const_re.captures(line) {
            if let Some(name) = c.get(1) {
                out.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Constant,
                    line: i + 1,
                    column: 1,
                    language: "go".to_string(),
                    signature: line.to_string(),
                });
            }
        } else if let Some(c) = import_re.captures(line) {
            let name = c
                .get(1)
                .or_else(|| c.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            if !name.is_empty() {
                out.push(Symbol {
                    name,
                    kind: SymbolKind::Import,
                    line: i + 1,
                    column: 1,
                    language: "go".to_string(),
                    signature: line.to_string(),
                });
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
        assert!(s
            .iter()
            .any(|x| x.name == "hello" && x.kind == SymbolKind::Function));
        assert!(s
            .iter()
            .any(|x| x.name == "world" && x.kind == SymbolKind::Function));
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
        let content =
            "def helper(): pass\nclass MyClass:\n    def method(self): pass\nMY_CONST = 1";
        let s = extract_python(content);
        assert!(s
            .iter()
            .any(|x| x.name == "helper" && x.kind == SymbolKind::Function));
        assert!(s
            .iter()
            .any(|x| x.name == "MyClass" && x.kind == SymbolKind::Class));
        assert!(s.iter().any(|x| x.name == "MY_CONST"));
    }

    #[test]
    fn javascript_class() {
        let content = "class Foo {}\nfunction bar() {}\nconst baz = 1;";
        let s = extract_javascript(content);
        assert!(s
            .iter()
            .any(|x| x.name == "Foo" && x.kind == SymbolKind::Class));
        assert!(s.iter().any(|x| x.name == "bar"));
        assert!(s
            .iter()
            .any(|x| x.name == "baz" && x.kind == SymbolKind::Constant));
    }

    #[test]
    fn typescript_interface() {
        let content = "export interface Foo {}\nexport const x: number = 1;";
        let s = extract_typescript(content);
        assert!(s
            .iter()
            .any(|x| x.name == "Foo" && x.kind == SymbolKind::Interface));
        assert!(s
            .iter()
            .any(|x| x.name == "x" && x.kind == SymbolKind::Constant));
    }

    #[test]
    fn go_struct() {
        let content = "type Foo struct {}\ntype Bar interface {}\nfunc hello() {}\nconst MAX = 10";
        let s = extract_go(content);
        assert!(s
            .iter()
            .any(|x| x.name == "Foo" && x.kind == SymbolKind::Struct));
        assert!(s
            .iter()
            .any(|x| x.name == "Bar" && x.kind == SymbolKind::Interface));
        assert!(s
            .iter()
            .any(|x| x.name == "hello" && x.kind == SymbolKind::Function));
        assert!(s
            .iter()
            .any(|x| x.name == "MAX" && x.kind == SymbolKind::Constant));
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

// =====================================================================
// P2#9 (审计 backlog 全清, 2026-08-16): tree-sitter 精确符号提取 (可选 feature)
// =====================================================================

/// tree-sitter 精确提取 Rust 文件符号 (feature "tree-sitter" 开启时可用).
///
/// 与 regex 版的关系 (集成而非分立): 同一 `Symbol` API; `extract_symbols`
/// 在 feature 开启时对 rust 语言优先走本路径, 解析失败降级 regex。
/// 优势: 多行签名/字符串内伪代码不误报/impl 内方法识别 (全语法树)。
///
/// 0 假装: 仅 Rust 语言覆盖 (tree-sitter-rust); 其余 4 语言仍走 regex —
/// 如实标注, 不假装全语言 AST。
#[cfg(feature = "tree-sitter")]
pub fn extract_symbols_tree_sitter(content: &str) -> Vec<Symbol> {
    use tree_sitter::Parser;

    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new(); // 语法不可用 → 空 (调用方降级 regex)
    }
    let Some(tree) = parser.parse(content, None) else {
        return Vec::new();
    };
    let mut out: Vec<Symbol> = Vec::new();
    let mut stack: Vec<tree_sitter::Node> = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        // 子节点先入栈 (DFS)
        let mut children: Vec<tree_sitter::Node> = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .collect();
        stack.append(&mut children);

        let (kind, name_field): (Option<SymbolKind>, &str) = match node.kind() {
            "function_item" => (Some(SymbolKind::Function), "name"),
            "struct_item" => (Some(SymbolKind::Struct), "name"),
            "enum_item" => (Some(SymbolKind::Enum), "name"),
            "const_item" => (Some(SymbolKind::Constant), "name"),
            "static_item" => (Some(SymbolKind::Constant), "name"),
            "trait_item" => (Some(SymbolKind::Interface), "name"),
            "type_item" => (Some(SymbolKind::Interface), "name"),
            "mod_item" => (Some(SymbolKind::Module), "name"),
            "use_declaration" => (Some(SymbolKind::Import), "path"),
            _ => (None, "name"),
        };
        let Some(kind) = kind else { continue };
        // impl 块内的 function_item → Method (按祖先语境: impl_item → declaration_list → fn)
        let kind = if kind == SymbolKind::Function {
            let ancestor_impl = node
                .parent()
                .map(|p| {
                    p.kind() == "impl_item" || p.parent().is_some_and(|g| g.kind() == "impl_item")
                })
                .unwrap_or(false);
            if ancestor_impl {
                SymbolKind::Method
            } else {
                kind
            }
        } else {
            kind
        };
        // use_declaration 的 path: 优先 field, 回退第一个 named child (语法稳健)
        let name_node = if kind == SymbolKind::Import {
            node.child_by_field_name("path")
                .or_else(|| node.named_child(0))
        } else {
            node.child_by_field_name(name_field)
        };
        let Some(name_node) = name_node else {
            continue;
        };
        let name = name_node
            .utf8_text(content.as_bytes())
            .unwrap_or("")
            .to_string();
        if name.trim().is_empty() {
            continue;
        }
        let pos = node.start_position();
        let (row, col) = (pos.row, pos.column);
        // 签名: 声明行到结束行 (tree-sitter 能跨行 — regex 版抓不到的)
        let start = node.start_byte();
        let end = node.end_byte();
        let signature = content
            .get(start..end.min(content.len()))
            .unwrap_or("")
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        out.push(Symbol {
            name,
            kind,
            line: row + 1,
            column: col + 1,
            language: "rust".to_string(),
            signature,
        });
    }
    out
}

#[cfg(feature = "tree-sitter")]
#[cfg(test)]
mod tree_sitter_tests {
    use super::*;

    #[test]
    fn extracts_fn_struct_enum_const_use() {
        let content = r#"
use std::collections::HashMap;

/// doc comment
pub async fn fetch_data<T: Clone>(url: &str) -> Result<T, String> {
    Ok(Default::default())
}

pub struct Config {
    pub name: String,
}

enum Status { Active, Done }

const MAX_RETRIES: usize = 3;
"#;
        let s = extract_symbols_tree_sitter(content);
        assert!(
            s.iter()
                .any(|x| x.name == "fetch_data" && x.kind == SymbolKind::Function),
            "fn 应提取"
        );
        assert!(
            s.iter()
                .any(|x| x.name == "Config" && x.kind == SymbolKind::Struct),
            "struct 应提取"
        );
        assert!(
            s.iter()
                .any(|x| x.name == "Status" && x.kind == SymbolKind::Enum),
            "enum 应提取"
        );
        assert!(
            s.iter()
                .any(|x| x.name == "MAX_RETRIES" && x.kind == SymbolKind::Constant),
            "const 应提取"
        );
        assert!(
            s.iter()
                .any(|x| x.kind == SymbolKind::Import && x.name.contains("HashMap")),
            "use 应提取"
        );
        // 跨行签名 (regex 版抓不到的): 函数声明应含参数行
        let fetch = s.iter().find(|x| x.name == "fetch_data").unwrap();
        assert!(
            fetch.signature.contains("url: &str"),
            "tree-sitter 应抓完整签名: {}",
            fetch.signature
        );
    }

    #[test]
    fn ignores_strings_and_comments() {
        let content = r#"
fn real() {}
// fn fake_in_comment() {}
let s = "fn fake_in_string() {}";
"#;
        let s = extract_symbols_tree_sitter(content);
        assert!(s.iter().any(|x| x.name == "real"), "真函数应提取");
        assert!(
            !s.iter().any(|x| x.name.starts_with("fake")),
            "注释/字符串内伪代码不应误报: {s:?}"
        );
    }

    #[test]
    fn impl_methods_are_method_kind() {
        let content = r#"
impl Config {
    pub fn new() -> Self { Config { name: String::new() } }
}
"#;
        let s = extract_symbols_tree_sitter(content);
        assert!(
            s.iter()
                .any(|x| x.name == "new" && x.kind == SymbolKind::Method),
            "impl 内 fn 应为 Method: {s:?}"
        );
    }

    #[test]
    fn parse_failure_falls_back_to_empty() {
        // 非 Rust 语法内容也应有语法树 (容错), 不 panic
        let s = extract_symbols_tree_sitter("not rust at all {{{");
        let _ = s;
    }
}
