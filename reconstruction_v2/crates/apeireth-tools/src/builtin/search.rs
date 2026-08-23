use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub path: Option<String>,
    pub max_results: Option<usize>,
}

pub struct SearchTool;

impl Default for SearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchTool {
    pub fn new() -> Self {
        Self
    }

    fn search_dir(
        dir: &Path,
        query: &str,
        max_matches: usize,
        results: &mut Vec<String>,
        depth: usize,
    ) {
        if depth > 10 || results.len() >= max_matches {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let query_lower = query.to_lowercase();

        for entry in entries.flatten() {
            if results.len() >= max_matches {
                break;
            }

            let path = entry.path();
            let file_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // Skip hidden, target, and git directories
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }

            if path.is_dir() {
                Self::search_dir(&path, query, max_matches, results, depth + 1);
            } else if path.is_file() {
                // Check filename match
                if file_name.to_lowercase().contains(&query_lower) {
                    results.push(format!("[Filename Match] {}", path.display()));
                }

                // Check text file content match (files < 500KB)
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() < 500_000 {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for (line_idx, line) in content.lines().enumerate() {
                                if line.to_lowercase().contains(&query_lower) {
                                    let snippet: String = line.trim().chars().take(120).collect();
                                    results.push(format!("{}:{}: {}", path.display(), line_idx + 1, snippet));
                                    if results.len() >= max_matches {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search".into(),
            description: "Recursively searches codebase files and content for keywords. Parameters: {\"query\": \"keyword\", \"path\": \".\", \"max_results\": 20}".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let params: SearchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid search parameters: {}", e)))?;

        if params.query.trim().is_empty() {
            return Err(ToolError::ValidationFailed("Search query cannot be empty".into()));
        }

        let search_root = params.path.unwrap_or_else(|| ".".into());
        let max_results = params.max_results.unwrap_or(20).min(100);
        let path = Path::new(&search_root);

        if !path.exists() {
            return Err(ToolError::ValidationFailed(format!("Search path does not exist: {}", search_root)));
        }

        let mut results = Vec::new();
        Self::search_dir(path, &params.query, max_results, &mut results, 0);

        if results.is_empty() {
            Ok(ToolResult::success(format!("No matches found for query: \"{}\" in {}", params.query, search_root)))
        } else {
            let count = results.len();
            Ok(ToolResult::success(format!(
                "Found {} match(es) for \"{}\" in {}:\n\n{}",
                count, params.query, search_root, results.join("\n")
            )))
        }
    }
}
