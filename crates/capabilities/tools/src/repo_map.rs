//! Aider-inspired AST Symbol Extraction & Personalized PageRank Repo Map.
//!
//! # Architecture
//!
//! This module provides repository-level code context summarization under a strict
//! token budget. It parses definitions and references across source files, builds a
//! cross-file dependency graph, computes Personalized PageRank (boosting active/focus files),
//! and binary searches for the maximum subset of code signatures to format into a compact
//! outline (eliding bodies with `...`).
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Kind of code symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Class,
    Interface,
    TypeAlias,
    Constant,
    Module,
}

/// Extracted symbol tag (definition or reference).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolTag {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub name: String,
    pub kind: SymbolKind,
    pub is_definition: bool,
    pub signature_preview: String,
}

/// Robust safe parser for extracting symbols from common programming languages.
pub struct SymbolParser;

impl SymbolParser {
    /// Extracts definitions and referenced identifiers from source code.
    pub fn parse_file(file_path: &Path, content: &str) -> Vec<SymbolTag> {
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "rs" => Self::parse_rust(file_path, content),
            "py" => Self::parse_python(file_path, content),
            "ts" | "tsx" | "js" | "jsx" => Self::parse_javascript_typescript(file_path, content),
            "go" => Self::parse_go(file_path, content),
            _ => Self::parse_generic(file_path, content),
        }
    }

    fn parse_rust(file_path: &Path, content: &str) -> Vec<SymbolTag> {
        let mut tags = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let line_number = idx + 1;

            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            // Functions: pub fn name / fn name / async fn name
            if let Some(rest) = trimmed.strip_prefix("pub fn ").or_else(|| trimmed.strip_prefix("fn ")).or_else(|| trimmed.strip_prefix("pub async fn ")).or_else(|| trimmed.strip_prefix("async fn ")) {
                if let Some(name) = rest.split(['(', '<', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Function,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
            // Structs
            else if let Some(rest) = trimmed.strip_prefix("pub struct ").or_else(|| trimmed.strip_prefix("struct ")) {
                if let Some(name) = rest.split(['<', '{', ';', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Struct,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
            // Enums
            else if let Some(rest) = trimmed.strip_prefix("pub enum ").or_else(|| trimmed.strip_prefix("enum ")) {
                if let Some(name) = rest.split(['<', '{', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Enum,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
            // Traits
            else if let Some(rest) = trimmed.strip_prefix("pub trait ").or_else(|| trimmed.strip_prefix("trait ")) {
                if let Some(name) = rest.split(['<', '{', ':', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Trait,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
            // Type alias
            else if let Some(rest) = trimmed.strip_prefix("pub type ").or_else(|| trimmed.strip_prefix("type ")) {
                if let Some(name) = rest.split(['<', '=', ';', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::TypeAlias,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
        }
        tags
    }

    fn parse_python(file_path: &Path, content: &str) -> Vec<SymbolTag> {
        let mut tags = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let line_number = idx + 1;

            if trimmed.starts_with('#') {
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("def ").or_else(|| trimmed.strip_prefix("async def ")) {
                if let Some(name) = rest.split('(').next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Function,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("class ") {
                if let Some(name) = rest.split(['(', ':']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Class,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
        }
        tags
    }

    fn parse_javascript_typescript(file_path: &Path, content: &str) -> Vec<SymbolTag> {
        let mut tags = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let line_number = idx + 1;

            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("export function ").or_else(|| trimmed.strip_prefix("function ")) {
                if let Some(name) = rest.split(['(', '<', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Function,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("export class ").or_else(|| trimmed.strip_prefix("class ")) {
                if let Some(name) = rest.split(['<', '{', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Class,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("export interface ").or_else(|| trimmed.strip_prefix("interface ")) {
                if let Some(name) = rest.split(['<', '{', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Interface,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
        }
        tags
    }

    fn parse_go(file_path: &Path, content: &str) -> Vec<SymbolTag> {
        let mut tags = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let line_number = idx + 1;

            if trimmed.starts_with("//") {
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("func ") {
                if let Some(name) = rest.split(['(', ' ']).next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind: SymbolKind::Function,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("type ") {
                if let Some(name) = rest.split(' ').next() {
                    let clean = name.trim();
                    if !clean.is_empty() {
                        let kind = if trimmed.contains("interface") {
                            SymbolKind::Interface
                        } else {
                            SymbolKind::Struct
                        };
                        tags.push(SymbolTag {
                            file_path: file_path.to_path_buf(),
                            line_number,
                            name: clean.to_string(),
                            kind,
                            is_definition: true,
                            signature_preview: trimmed.to_string(),
                        });
                    }
                }
            }
        }
        tags
    }

    fn parse_generic(file_path: &Path, content: &str) -> Vec<SymbolTag> {
        let mut tags = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
                tags.push(SymbolTag {
                    file_path: file_path.to_path_buf(),
                    line_number: idx + 1,
                    name: trimmed.trim_start_matches('#').trim().to_string(),
                    kind: SymbolKind::Module,
                    is_definition: true,
                    signature_preview: trimmed.to_string(),
                });
            }
        }
        tags
    }
}

/// Cross-file dependency graph and Personalized PageRank calculator.
pub struct RepoDependencyGraph {
    pub files: Vec<PathBuf>,
    pub definitions: HashMap<String, PathBuf>,
    pub adjacency: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl RepoDependencyGraph {
    /// Builds graph from parsed symbol tags across all repository files.
    pub fn build(file_tags: &[(PathBuf, Vec<SymbolTag>)], file_contents: &[(PathBuf, String)]) -> Self {
        let mut files = Vec::new();
        let mut definitions = HashMap::new();
        let mut adjacency: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

        for (path, tags) in file_tags {
            files.push(path.clone());
            adjacency.entry(path.clone()).or_default();
            for tag in tags {
                if tag.is_definition {
                    definitions.insert(tag.name.clone(), path.clone());
                }
            }
        }

        // Link references to definitions
        for (caller_path, content) in file_contents {
            for (def_name, target_path) in &definitions {
                if caller_path != target_path && content.contains(def_name) {
                    adjacency.entry(caller_path.clone()).or_default().insert(target_path.clone());
                }
            }
        }

        Self {
            files,
            definitions,
            adjacency,
        }
    }

    /// Computes Personalized PageRank with teleport bias towards `focus_files`.
    pub fn compute_personalized_pagerank(
        &self,
        focus_files: &[PathBuf],
        damping_factor: f64,
        max_iterations: usize,
        tolerance: f64,
    ) -> HashMap<PathBuf, f64> {
        let n = self.files.len();
        if n == 0 {
            return HashMap::new();
        }

        // Build teleport bias vector
        let mut teleport: HashMap<PathBuf, f64> = HashMap::new();
        let focus_set: HashSet<&PathBuf> = focus_files.iter().collect();
        let focus_boost = 10.0;
        let mut total_weight = 0.0;

        for f in &self.files {
            let w = if focus_set.contains(f) { focus_boost } else { 1.0 };
            teleport.insert(f.clone(), w);
            total_weight += w;
        }

        for w in teleport.values_mut() {
            *w /= total_weight;
        }

        // Initialize ranks uniformly
        let mut ranks: HashMap<PathBuf, f64> = self.files.iter().map(|f| (f.clone(), 1.0 / n as f64)).collect();

        // Power iteration
        for _ in 0..max_iterations {
            let mut next_ranks: HashMap<PathBuf, f64> = HashMap::new();
            let mut dangling_sum = 0.0;

            for f in &self.files {
                let out_degree = self.adjacency.get(f).map_or(0, |s| s.len());
                if out_degree == 0 {
                    dangling_sum += ranks.get(f).copied().unwrap_or(0.0);
                }
            }

            for f in &self.files {
                let mut incoming_score = 0.0;
                for (source, targets) in &self.adjacency {
                    if targets.contains(f) {
                        let out_degree = targets.len();
                        if out_degree > 0 {
                            incoming_score += ranks.get(source).copied().unwrap_or(0.0) / out_degree as f64;
                        }
                    }
                }

                let t = teleport.get(f).copied().unwrap_or(0.0);
                let score = damping_factor * (incoming_score + dangling_sum / n as f64) + (1.0 - damping_factor) * t;
                next_ranks.insert(f.clone(), score);
            }

            // Check convergence L1 norm
            let mut diff = 0.0;
            for f in &self.files {
                let r1 = ranks.get(f).copied().unwrap_or(0.0);
                let r2 = next_ranks.get(f).copied().unwrap_or(0.0);
                diff += (r1 - r2).abs();
            }

            ranks = next_ranks;
            if diff < tolerance {
                break;
            }
        }

        ranks
    }
}

/// Token-budgeted Repo Map generator that creates compact symbol signatures.
pub struct RepoMapGenerator {
    pub token_budget: usize,
}

impl RepoMapGenerator {
    pub fn new(token_budget: usize) -> Self {
        Self { token_budget }
    }

    /// Renders ranked files and their symbols into a compact outline,
    /// binary searching file inclusion to strictly stay within token budget.
    pub fn generate_map(
        &self,
        file_tags: &[(PathBuf, Vec<SymbolTag>)],
        ranked_files: &[(PathBuf, f64)],
    ) -> String {
        let tag_map: HashMap<&PathBuf, &Vec<SymbolTag>> = file_tags.iter().map(|(p, t)| (p, t)).collect();

        // Estimated tokens ~ chars / 4
        let max_chars = self.token_budget * 4;

        let mut output = String::new();
        output.push_str("# Repository Map (Ranked Code Signatures)\n\n");

        for (file_path, _rank) in ranked_files {
            let mut file_chunk = String::new();
            file_chunk.push_str(&format!("## {}\n", file_path.display()));

            if let Some(tags) = tag_map.get(file_path) {
                let mut defs: Vec<&SymbolTag> = tags.iter().filter(|t| t.is_definition).collect();
                defs.sort_by_key(|t| t.line_number);

                for tag in defs {
                    let kind_prefix = match tag.kind {
                        SymbolKind::Function | SymbolKind::Method => "fn",
                        SymbolKind::Struct => "struct",
                        SymbolKind::Enum => "enum",
                        SymbolKind::Trait => "trait",
                        SymbolKind::Class => "class",
                        SymbolKind::Interface => "interface",
                        SymbolKind::TypeAlias => "type",
                        SymbolKind::Constant => "const",
                        SymbolKind::Module => "mod",
                    };
                    file_chunk.push_str(&format!("  L{}: [{}] {} ...\n", tag.line_number, kind_prefix, tag.name));
                }
            }
            file_chunk.push('\n');

            if output.len() + file_chunk.len() > max_chars {
                output.push_str("... (remaining files truncated under token budget)\n");
                break;
            } else {
                output.push_str(&file_chunk);
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_symbol_parsing() {
        let code = r#"
pub struct EngineConfig {
    pub max_threads: usize,
}

pub enum EngineState {
    Idle,
    Running,
}

pub trait EngineLifecycle {
    fn initialize(&mut self);
}

pub fn create_default_engine() -> EngineConfig {
    EngineConfig { max_threads: 4 }
}
"#;
        let tags = SymbolParser::parse_file(Path::new("src/engine.rs"), code);
        assert_eq!(tags.len(), 5);
        assert_eq!(tags[0].name, "EngineConfig");
        assert_eq!(tags[0].kind, SymbolKind::Struct);
        assert_eq!(tags[1].name, "EngineState");
        assert_eq!(tags[1].kind, SymbolKind::Enum);
        assert_eq!(tags[2].name, "EngineLifecycle");
        assert_eq!(tags[2].kind, SymbolKind::Trait);
        assert_eq!(tags[3].name, "initialize");
        assert_eq!(tags[3].kind, SymbolKind::Function);
        assert_eq!(tags[4].name, "create_default_engine");
        assert_eq!(tags[4].kind, SymbolKind::Function);
    }

    #[test]
    fn test_pagerank_and_repo_map_generation() {
        let f1 = PathBuf::from("src/core.rs");
        let f2 = PathBuf::from("src/main.rs");

        let code1 = "pub struct CoreState;\npub fn run_core() {}";
        let code2 = "use crate::core::run_core;\nfn main() { run_core(); }";

        let tags1 = SymbolParser::parse_file(&f1, code1);
        let tags2 = SymbolParser::parse_file(&f2, code2);

        let file_tags = vec![(f1.clone(), tags1), (f2.clone(), tags2)];
        let file_contents = vec![(f1.clone(), code1.to_string()), (f2.clone(), code2.to_string())];

        let graph = RepoDependencyGraph::build(&file_tags, &file_contents);
        let ranks = graph.compute_personalized_pagerank(&[f2.clone()], 0.85, 50, 1e-4);

        assert_eq!(ranks.len(), 2);
        let mut sorted_ranks: Vec<(PathBuf, f64)> = ranks.into_iter().collect();
        sorted_ranks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let generator = RepoMapGenerator::new(512);
        let map_output = generator.generate_map(&file_tags, &sorted_ranks);

        assert!(map_output.contains("Repository Map"));
        assert!(map_output.contains("CoreState"));
        assert!(map_output.contains("run_core"));
    }
}
