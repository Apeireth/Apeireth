//! SwarmVault & OpenKB inspired Three-Tier Knowledge Vault & Vectorless TOC Tree Router.
//!
//! # Architecture
//!
//! 1. **Three-Tier Storage Model (SwarmVault)**:
//!    - `Raw Tier`: Immutable read-only sources (PDFs, logs, raw chats, captured artifacts).
//!    - `Living Wiki Tier`: Bi-directional linked (`[[wikilink]]`) structured Markdown knowledge.
//!    - `Schema Contract Tier`: Formal definitions of allowed entity types, relations, and validation rules.
//! 2. **Vectorless Reasoning-based TOC Tree Index & Router (OpenKB / PageIndex)**:
//!    - Replaces lossy vector chunking with a structured Table-of-Contents (TOC) hierarchy.
//!    - Performs top-down agentic tree routing across outline branches to pinpoint fine-grained chapters with full macro context.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors related to Three-Tier Vault and TOC operations.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum VaultError {
    #[error("schema violation: {0}")]
    SchemaViolation(String),
    #[error("file not found in vault: {0}")]
    NotFound(String),
    #[error("TOC parsing error: {0}")]
    TocParse(String),
    #[error("IO error: {0}")]
    Io(String),
}

/// The three formal tiers of knowledge storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VaultTier {
    /// Immutable, read-only original input sources.
    Raw,
    /// Incrementally compiled and living markdown wiki pages.
    LivingWiki,
    /// Formal schema constraints and contract validation rules.
    SchemaContract,
}

/// Table-of-Contents (TOC) tree node representing a document section or topic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TocTreeNode {
    pub node_id: String,
    pub title: String,
    pub level: usize,
    pub summary: String,
    pub file_path: PathBuf,
    pub byte_range: (usize, usize),
    pub children: Vec<TocTreeNode>,
}

impl TocTreeNode {
    pub fn new(
        node_id: impl Into<String>,
        title: impl Into<String>,
        level: usize,
        summary: impl Into<String>,
        file_path: impl Into<PathBuf>,
        byte_range: (usize, usize),
    ) -> Self {
        Self {
            node_id: node_id.into(),
            title: title.into(),
            level,
            summary: summary.into(),
            file_path: file_path.into(),
            byte_range,
            children: Vec::new(),
        }
    }
}

/// Vectorless TOC Tree indexer that compiles structured outlines from Markdown documents.
pub struct TocTreeIndexer;

impl TocTreeIndexer {
    /// Builds a hierarchical TOC tree from markdown document content.
    pub fn build_from_markdown(file_path: &Path, content: &str) -> TocTreeNode {
        let root_title = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Root Document");

        let mut root = TocTreeNode::new(
            "root",
            root_title,
            0,
            "Root outline for document",
            file_path,
            (0, content.len()),
        );

        let mut current_offset = 0;
        let mut node_counter = 0;

        for line in content.lines() {
            let line_len = line.len() + 1; // including newline
            let trimmed = line.trim();

            if trimmed.starts_with('#') {
                let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                if hashes > 0 && hashes <= 6 {
                    let title = trimmed.trim_start_matches('#').trim().to_string();
                    node_counter += 1;
                    let node_id = format!("sec_{node_counter}");

                    let child = TocTreeNode::new(
                        node_id,
                        title,
                        hashes,
                        line.to_string(),
                        file_path,
                        (current_offset, current_offset + line_len),
                    );

                    Self::insert_node_into_hierarchy(&mut root, child);
                }
            }
            current_offset += line_len;
        }

        root
    }

    fn insert_node_into_hierarchy(parent: &mut TocTreeNode, new_node: TocTreeNode) {
        if parent.children.is_empty() {
            parent.children.push(new_node);
            return;
        }

        let last_idx = parent.children.len() - 1;
        if new_node.level > parent.children[last_idx].level {
            Self::insert_node_into_hierarchy(&mut parent.children[last_idx], new_node);
        } else {
            parent.children.push(new_node);
        }
    }
}

/// Agentic Tree Routing engine that explores TOC trees top-down.
pub struct TreeReasoningRouter;

impl TreeReasoningRouter {
    /// Routes a natural language query down the TOC tree and returns matching section paths.
    pub fn route_query(tree: &TocTreeNode, query: &str) -> Vec<TocTreeNode> {
        let mut matches = Vec::new();
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();

        Self::recursive_route(tree, &keywords, &mut matches);
        matches
    }

    fn recursive_route(
        node: &TocTreeNode,
        keywords: &[&str],
        results: &mut Vec<TocTreeNode>,
    ) {
        let title_lower = node.title.to_lowercase();
        let summary_lower = node.summary.to_lowercase();

        let mut score = 0;
        for kw in keywords {
            if title_lower.contains(kw) {
                score += 3;
            }
            if summary_lower.contains(kw) {
                score += 1;
            }
        }

        if score > 0 && node.node_id != "root" {
            results.push(node.clone());
        }

        for child in &node.children {
            Self::recursive_route(child, keywords, results);
        }
    }
}

/// Provenance link between raw source and distilled knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub raw_source_id: String,
    pub wiki_concept_id: String,
    pub relation: String,
    pub timestamp_ms: u64,
}

/// Three-Tier Knowledge Vault manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeTierVault {
    pub vault_root: PathBuf,
    pub allowed_entity_types: Vec<String>,
    pub provenance_log: Vec<ProvenanceRecord>,
}

impl ThreeTierVault {
    pub fn new(vault_root: impl Into<PathBuf>, allowed_entity_types: Vec<String>) -> Self {
        Self {
            vault_root: vault_root.into(),
            allowed_entity_types,
            provenance_log: Vec::new(),
        }
    }

    /// Validates an entity type against the Schema Contract tier.
    pub fn validate_entity_type(&self, entity_type: &str) -> Result<(), VaultError> {
        if self.allowed_entity_types.iter().any(|t| t == entity_type) {
            Ok(())
        } else {
            Err(VaultError::SchemaViolation(format!(
                "entity type '{entity_type}' not permitted by schema contract (allowed: {:?})",
                self.allowed_entity_types
            )))
        }
    }

    /// Records provenance linkage between raw source and compiled wiki entity.
    pub fn record_provenance(
        &mut self,
        raw_source_id: impl Into<String>,
        wiki_concept_id: impl Into<String>,
        relation: impl Into<String>,
        timestamp_ms: u64,
    ) {
        self.provenance_log.push(ProvenanceRecord {
            raw_source_id: raw_source_id.into(),
            wiki_concept_id: wiki_concept_id.into(),
            relation: relation.into(),
            timestamp_ms,
        });
    }

    /// Queries all raw sources associated with a given living wiki concept.
    pub fn get_raw_sources_for_concept(&self, wiki_concept_id: &str) -> Vec<String> {
        self.provenance_log
            .iter()
            .filter(|p| p.wiki_concept_id == wiki_concept_id)
            .map(|p| p.raw_source_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toc_tree_indexer_hierarchical_parsing() {
        let md = r#"# Main Title
## Chapter 1: Introduction
Some introductory text.
### Section 1.1: Background
Background details.
## Chapter 2: Deep Dive
Detailed analysis here.
"#;
        let root = TocTreeIndexer::build_from_markdown(Path::new("doc.md"), md);
        assert_eq!(root.title, "doc.md");
        assert_eq!(root.children.len(), 1); // # Main Title

        let main = &root.children[0];
        assert_eq!(main.title, "Main Title");
        assert_eq!(main.children.len(), 2); // Chapter 1 and Chapter 2

        assert_eq!(main.children[0].title, "Chapter 1: Introduction");
        assert_eq!(main.children[0].children.len(), 1); // Section 1.1
        assert_eq!(main.children[0].children[0].title, "Section 1.1: Background");

        assert_eq!(main.children[1].title, "Chapter 2: Deep Dive");
    }

    #[test]
    fn test_tree_reasoning_router() {
        let md = r#"# System Architecture
## Memory Engine
Covers working memory and river topology.
## Governance Guardrails
Covers OWASP security and rate limiting.
"#;
        let root = TocTreeIndexer::build_from_markdown(Path::new("arch.md"), md);
        let matches = TreeReasoningRouter::route_query(&root, "river memory");

        assert!(!matches.is_empty());
        assert_eq!(matches[0].title, "Memory Engine");
    }

    #[test]
    fn test_three_tier_vault_schema_and_provenance() {
        let mut vault = ThreeTierVault::new(
            Path::new("/vault"),
            vec!["Hypothesis".to_string(), "Finding".to_string(), "Component".to_string()],
        );

        assert!(vault.validate_entity_type("Component").is_ok());
        assert!(vault.validate_entity_type("AlienArtifact").is_err());

        vault.record_provenance("raw_pdf_01", "wiki_comp_river", "distilled_from", 1780000000);
        let sources = vault.get_raw_sources_for_concept("wiki_comp_river");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], "raw_pdf_01");
    }
}
