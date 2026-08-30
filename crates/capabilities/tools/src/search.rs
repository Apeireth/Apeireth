//! Deterministic local search tool capability.
//!
//! Search is local, literal, and case-insensitive. It searches file names and
//! UTF-8 text-file lines under a caller-supplied workspace root. It does not
//! use regex, does not use the network, and returns results in deterministic
//! (path, line, text) order. Known credential and key paths are skipped.

use std::fs;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

use crate::sensitive_path::is_sensitive_path;

const DEFAULT_MAX_RESULTS: usize = 20;
const MAX_RESULTS_CAP: usize = 100;
const DEFAULT_MAX_FILE_SIZE: u64 = 500_000;
const MAX_DEPTH: usize = 10;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchParams {
    query: String,
    path: Option<String>,
    max_results: Option<usize>,
}

#[derive(Debug)]
pub enum SearchError {
    InvalidInput(String),
    NotFound(String),
    PermissionDenied(String),
    Io(String),
}

impl SearchError {
    fn message(&self) -> String {
        match self {
            Self::InvalidInput(m) => format!("invalid search request: {m}"),
            Self::NotFound(m) => format!("not found: {m}"),
            Self::PermissionDenied(m) => format!("permission denied: {m}"),
            Self::Io(m) => format!("search IO error: {m}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchMatch {
    path: String,
    line: usize,
    text: String,
    /// 1-based byte column of the first match on the case-folded line.
    /// `0` for filename matches (no line content).
    column: usize,
    /// Non-overlapping occurrence count on this line (donor CodeSearcher semantics).
    occurrences: usize,
}

pub struct SearchTool {
    id: CapabilityId,
    root: PathBuf,
    max_file_size: u64,
}

impl SearchTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            id: CapabilityId::new("tool.search").unwrap(),
            root: root.into(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        }
    }

    /// Override the maximum UTF-8 file size whose content is searched.
    #[must_use]
    pub fn with_max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = max_file_size;
        self
    }

    fn error_result(&self, call: &ToolCall, error: SearchError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.message())
    }

    fn canonical_root(&self) -> Result<PathBuf, SearchError> {
        fs::canonicalize(&self.root).map_err(|e| {
            SearchError::InvalidInput(format!(
                "workspace root {} is not accessible: {e}",
                self.root.display()
            ))
        })
    }

    fn resolve_contained(&self, requested: &str) -> Result<(PathBuf, PathBuf), SearchError> {
        if requested.trim().is_empty() {
            return Err(SearchError::InvalidInput(
                "search path must not be empty".to_string(),
            ));
        }

        let root = self.canonical_root()?;
        let candidate = if Path::new(requested).is_absolute() {
            PathBuf::from(requested)
        } else {
            root.join(requested)
        };
        let canonical = fs::canonicalize(&candidate).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                SearchError::NotFound(format!("{}", candidate.display()))
            }
            std::io::ErrorKind::PermissionDenied => {
                SearchError::PermissionDenied(format!("{}", candidate.display()))
            }
            _ => SearchError::Io(format!("{}: {e}", candidate.display())),
        })?;

        if !canonical.starts_with(&root) {
            return Err(SearchError::PermissionDenied(format!(
                "{} resolves outside the workspace root",
                candidate.display()
            )));
        }

        if is_sensitive_path(&root, &candidate) || is_sensitive_path(&root, &canonical) {
            return Err(SearchError::PermissionDenied(
                "requested path is protected".to_string(),
            ));
        }

        Ok((root, canonical))
    }

    fn relative_display(canonical: &Path, root: &Path) -> String {
        match canonical.strip_prefix(root) {
            Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
            Ok(rel) => rel.to_string_lossy().to_string(),
            Err(_) => canonical.to_string_lossy().to_string(),
        }
    }

    fn add_match(
        &self,
        results: &mut Vec<SearchMatch>,
        path: String,
        line: usize,
        text: String,
        column: usize,
        occurrences: usize,
        max_results: usize,
        truncated: &mut bool,
    ) -> bool {
        if results.len() >= max_results {
            *truncated = true;
            return false;
        }
        results.push(SearchMatch {
            path,
            line,
            text,
            column,
            occurrences,
        });
        true
    }

    fn search_path(
        &self,
        root: &Path,
        target: &Path,
        query_lower: &str,
        max_results: usize,
        results: &mut Vec<SearchMatch>,
        truncated: &mut bool,
        depth: usize,
    ) {
        if depth > MAX_DEPTH || *truncated || is_sensitive_path(root, target) {
            return;
        }

        let Ok(metadata) = fs::symlink_metadata(target) else {
            return;
        };

        if metadata.is_dir() {
            let Ok(entries) = fs::read_dir(target) else {
                return;
            };

            // Deterministic order independent of filesystem iteration order.
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_cached_key(|entry| entry.file_name().to_string_lossy().to_string());

            for entry in entries {
                if *truncated {
                    break;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                if is_sensitive_path(root, &path) {
                    continue;
                }

                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    continue;
                }
                if file_type.is_dir() && Self::is_skipped_dir(&name) {
                    continue;
                }

                if file_type.is_dir() {
                    self.search_path(
                        root,
                        &path,
                        query_lower,
                        max_results,
                        results,
                        truncated,
                        depth + 1,
                    );
                } else if file_type.is_file() {
                    self.search_file(
                        root,
                        &path,
                        &name,
                        query_lower,
                        max_results,
                        results,
                        truncated,
                    );
                }
            }
        } else if metadata.is_file() {
            let name = target
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| target.to_string_lossy().to_string());
            self.search_file(
                root,
                target,
                &name,
                query_lower,
                max_results,
                results,
                truncated,
            );
        }
    }

    fn is_skipped_dir(name: &str) -> bool {
        name.starts_with('.') || name == "target" || name == "node_modules"
    }

    fn search_file(
        &self,
        root: &Path,
        path: &Path,
        file_name: &str,
        query_lower: &str,
        max_results: usize,
        results: &mut Vec<SearchMatch>,
        truncated: &mut bool,
    ) {
        if is_sensitive_path(root, path) {
            return;
        }
        let rel = Self::relative_display(path, root);

        // Filename match.
        if file_name.to_lowercase().contains(query_lower) {
            if !self.add_match(
                results,
                rel.clone(),
                0,
                format!("[filename match] {file_name}"),
                0,
                1,
                max_results,
                truncated,
            ) {
                return;
            }
        }

        let Ok(metadata) = fs::metadata(path) else {
            return;
        };
        if metadata.len() > self.max_file_size {
            return;
        }

        let Ok(content) = fs::read_to_string(path) else {
            return;
        };

        for (line_idx, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let (occurrences, column) = literal_line_hits(&line_lower, query_lower);
            if occurrences > 0 {
                let snippet: String = line.trim().chars().take(160).collect();
                if !self.add_match(
                    results,
                    rel.clone(),
                    line_idx + 1,
                    snippet,
                    column,
                    occurrences,
                    max_results,
                    truncated,
                ) {
                    break;
                }
            }
        }
    }
}

/// Count non-overlapping literal hits on a case-folded line.
///
/// Returns `(occurrences, first_column)` where `first_column` is 1-based on the
/// case-folded line (donor CodeSearcher: positions computed after lowercasing).
fn literal_line_hits(line_lower: &str, query_lower: &str) -> (usize, usize) {
    if query_lower.is_empty() {
        return (0, 0);
    }
    let mut occurrences = 0;
    let mut first_column = 0;
    let mut start = 0;
    while let Some(pos) = line_lower[start..].find(query_lower) {
        let abs = start + pos;
        occurrences += 1;
        if first_column == 0 {
            first_column = abs + 1;
        }
        start = abs + query_lower.len().max(1);
        if start > line_lower.len() {
            break;
        }
    }
    (occurrences, first_column)
}

#[async_trait]
impl ToolCapability for SearchTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Case-insensitive literal substring to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Path relative to the workspace root to search; defaults to the root"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum matches to return; default 20, capped at 100"
                }
            },
            "required": ["query"],
            "additionalProperties": false
        });
        let mut params = apeireth_protocol::canonical::ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());

        NormalizedTool::new("search")
            .with_description(
                "Search file names and UTF-8 file contents inside the workspace root. \
                 Literal case-insensitive substring search; read-only and local. \
                 Known credential and key paths are not searched.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: SearchParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid search parameters: {e}"),
                )
            }
        };

        if params.query.trim().is_empty() {
            return self.error_result(
                call,
                SearchError::InvalidInput("query must not be empty".to_string()),
            );
        }

        let requested = params.path.as_deref().unwrap_or(".");
        let (root, canonical) = match self.resolve_contained(requested) {
            Ok(pair) => pair,
            Err(e) => return self.error_result(call, e),
        };

        let max_results = params
            .max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .clamp(1, MAX_RESULTS_CAP);

        let mut results = Vec::new();
        let mut truncated = false;
        let query_lower = params.query.to_lowercase();
        self.search_path(
            &root,
            &canonical,
            &query_lower,
            max_results,
            &mut results,
            &mut truncated,
            0,
        );

        // Search collects in filesystem-independent sorted order already, but
        // keep this final sort as the deterministic contract.
        results.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.line.cmp(&b.line))
                .then_with(|| a.text.cmp(&b.text))
        });

        let value = serde_json::json!({
            "query": params.query,
            "path": Self::relative_display(&canonical, &root),
            "count": results.len(),
            "truncated": truncated,
            "matches": results.into_iter().map(|m| serde_json::json!({
                "path": m.path,
                "line": m.line,
                "text": m.text,
                "column": m.column,
                "occurrences": m.occurrences,
            })).collect::<Vec<_>>(),
        });

        ToolResult::ok(&call.id, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(root: &Path) -> SearchTool {
        SearchTool::new(root.to_path_buf())
    }

    async fn invoke(
        tool: &SearchTool,
        query: &str,
        path: &str,
        max_results: Option<usize>,
    ) -> ToolResult {
        let call = ToolCall {
            id: "call_search".into(),
            name: "search".into(),
            arguments: json!({ "query": query, "path": path, "max_results": max_results }),
        };
        tool.invoke(&call).await
    }

    #[tokio::test]
    async fn finds_known_string_in_filename_and_content() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/hello.txt"), "hello world\n").unwrap();
        fs::write(dir.path().join("README.md"), "says hello there\n").unwrap();

        let result = invoke(&tool(dir.path()), "hello", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("hello.txt"), "{rendered}");
        assert!(rendered.contains("says hello"), "{rendered}");
    }

    #[tokio::test]
    async fn no_match_returns_empty_deterministic_result() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "nothing here").unwrap();

        let result = invoke(&tool(dir.path()), "absent", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("\"count\":0"), "{rendered}");
    }

    #[tokio::test]
    async fn result_order_is_by_path_then_line() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.txt"), "needle\nneedle\n").unwrap();
        fs::write(dir.path().join("a.txt"), "needle\n").unwrap();

        let result = invoke(&tool(dir.path()), "needle", ".", None).await;
        let rendered = result.render();
        let a = rendered.find("a.txt").unwrap();
        let b = rendered.find("b.txt").unwrap();
        assert!(a < b, "{rendered}");
    }

    #[tokio::test]
    async fn result_limit_is_respected_and_truncation_reported() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            fs::write(dir.path().join(format!("f{i}.txt")), "needle").unwrap();
        }

        let result = invoke(&tool(dir.path()), "needle", ".", Some(2)).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("\"count\":2"), "{rendered}");
        assert!(rendered.contains("\"truncated\":true"), "{rendered}");
    }

    #[tokio::test]
    async fn unicode_content_is_searchable() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("中文.txt"), "你好，世界\n").unwrap();

        let result = invoke(&tool(dir.path()), "你好", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("中文.txt"), "{rendered}");
        assert!(rendered.contains("你好，世界"), "{rendered}");
    }

    #[tokio::test]
    async fn binary_files_are_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("data.bin"), [0xFF, 0xFE, 0x00, 0x01, 0xFF]).unwrap();

        let result = invoke(&tool(dir.path()), "needle", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("\"count\":0"), "{rendered}");
    }

    #[tokio::test]
    async fn path_traversal_outside_root_is_denied() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "needle").unwrap();

        let result = invoke(&tool(&root), "needle", "../outside", None).await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("permission denied"),
            "{}",
            result.render()
        );
    }

    #[tokio::test]
    async fn skipped_directories_are_not_searched() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/config"), "needle").unwrap();
        fs::write(dir.path().join("kept.txt"), "needle").unwrap();

        let result = invoke(&tool(dir.path()), "needle", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(!rendered.contains(".git"), "{rendered}");
        assert!(rendered.contains("kept.txt"), "{rendered}");
    }

    #[tokio::test]
    async fn sensitive_paths_are_skipped_and_direct_sensitive_search_is_denied() {
        let dir = tempfile::tempdir().unwrap();
        for path in [
            ".env",
            ".env.local",
            "foo.pem",
            "foo.key",
            "id_rsa",
            "id_ed25519",
            "credentials.json",
            "secrets.production",
        ] {
            fs::write(dir.path().join(path), "needle").unwrap();
        }
        fs::create_dir_all(dir.path().join(".ssh")).unwrap();
        fs::write(dir.path().join(".ssh/config"), "needle").unwrap();
        fs::create_dir_all(dir.path().join(".config/gcloud")).unwrap();
        fs::write(
            dir.path()
                .join(".config/gcloud/application_default_credentials.json"),
            "needle",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        for path in ["README.md", "Cargo.toml", "src/lib.rs", ".gitignore"] {
            fs::write(dir.path().join(path), "needle").unwrap();
        }

        let tool = tool(dir.path());
        let result = invoke(&tool, "needle", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        for hidden in [
            ".env",
            ".env.local",
            "foo.pem",
            "foo.key",
            "id_rsa",
            "id_ed25519",
            "credentials.json",
            "secrets.production",
            ".ssh",
            ".config",
        ] {
            assert!(
                !rendered.contains(hidden),
                "sensitive path leaked: {hidden}: {rendered}"
            );
        }
        for visible in ["README.md", "Cargo.toml", ".gitignore"] {
            assert!(
                rendered.contains(visible),
                "normal path missing: {visible}: {rendered}"
            );
        }
        assert!(
            rendered.contains("src") && rendered.contains("lib.rs"),
            "normal path missing: src/lib.rs: {rendered}"
        );

        let direct = invoke(&tool, "needle", ".env", None).await;
        assert!(!direct.is_ok());
        assert!(
            direct.render().contains("permission denied"),
            "{}",
            direct.render()
        );

        let nested_direct = invoke(&tool, "needle", ".ssh/config", None).await;
        assert!(!nested_direct.is_ok());
        assert!(
            nested_direct.render().contains("permission denied"),
            "{}",
            nested_direct.render()
        );
    }

    #[test]
    fn literal_line_hits_count_non_overlapping_and_first_column() {
        assert_eq!(literal_line_hits("hello world", "hello"), (1, 1));
        assert_eq!(literal_line_hits("xx xx xx", "xx"), (3, 1));
        assert_eq!(literal_line_hits("ababa", "aba"), (1, 1));
        assert_eq!(literal_line_hits("nope", "x"), (0, 0));
        assert_eq!(literal_line_hits("pre needle post", "needle"), (1, 5));
    }

    #[tokio::test]
    async fn content_match_reports_occurrences_and_first_column() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "xx xx xx\n").unwrap();

        let result = invoke(&tool(dir.path()), "xx", ".", None).await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("\"occurrences\":3"), "{rendered}");
        assert!(rendered.contains("\"column\":1"), "{rendered}");
    }
}
