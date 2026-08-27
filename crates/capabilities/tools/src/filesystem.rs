//! Read-only filesystem tool capability.
//!
//! The tool reads, lists, and stats paths under a caller-supplied workspace
//! root. Path resolution is canonicalized and checked for containment inside
//! that root before any operation, so `..` and symlink traversal do not escape
//! the root for the operations implemented here.
//!
//! Write/delete/rename/copy are deliberately not implemented in M2A. They are
//! deferred to the sandbox phase (M2B). This tool does **not** claim to be a
//! process/filesystem sandbox.

use std::fs;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

/// Default maximum file size for `read` (1 MiB).
pub const DEFAULT_MAX_FILE_SIZE: u64 = 1024 * 1024;

/// Maximum entries returned by `list` before truncation is reported.
pub const MAX_LIST_ENTRIES: usize = 10_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilesystemParams {
    operation: String,
    path: String,
}

#[derive(Debug)]
pub enum FilesystemError {
    InvalidInput(String),
    NotFound(String),
    PermissionDenied(String),
    TooLarge(String),
    NotUtf8(String),
    Io(String),
}

impl FilesystemError {
    fn message(&self) -> String {
        match self {
            Self::InvalidInput(m) => format!("invalid filesystem request: {m}"),
            Self::NotFound(m) => format!("not found: {m}"),
            Self::PermissionDenied(m) => format!("permission denied: {m}"),
            Self::TooLarge(m) => format!("file too large: {m}"),
            Self::NotUtf8(m) => format!("file is not valid UTF-8: {m}"),
            Self::Io(m) => format!("filesystem IO error: {m}"),
        }
    }
}

pub struct FilesystemTool {
    id: CapabilityId,
    root: PathBuf,
    max_file_size: u64,
}

impl FilesystemTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            id: CapabilityId::new("tool.filesystem").unwrap(),
            root: root.into(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        }
    }

    /// Override the maximum file size accepted by `read`.
    #[must_use]
    pub fn with_max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = max_file_size;
        self
    }

    /// Canonicalize the workspace root.
    fn canonical_root(&self) -> Result<PathBuf, FilesystemError> {
        fs::canonicalize(&self.root).map_err(|e| {
            FilesystemError::InvalidInput(format!(
                "workspace root {} is not accessible: {e}",
                self.root.display()
            ))
        })
    }

    /// Resolve `requested` under the workspace root and prove containment.
    ///
    /// The path must exist for read/list/stat, so canonicalization both
    /// resolves `..` and symlinks and gives us the path that must still be
    /// inside the root.
    fn resolve_contained(&self, requested: &str) -> Result<PathBuf, FilesystemError> {
        if requested.trim().is_empty() {
            return Err(FilesystemError::InvalidInput(
                "path must not be empty".to_string(),
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
                FilesystemError::NotFound(format!("{}", candidate.display()))
            }
            std::io::ErrorKind::PermissionDenied => {
                FilesystemError::PermissionDenied(format!("{}", candidate.display()))
            }
            _ => FilesystemError::Io(format!("{}: {e}", candidate.display())),
        })?;

        if !canonical.starts_with(&root) {
            return Err(FilesystemError::PermissionDenied(format!(
                "{} resolves outside the workspace root",
                candidate.display()
            )));
        }

        Ok(canonical)
    }

    fn relative_display<'a>(&self, canonical: &'a Path, root: &Path) -> String {
        match canonical.strip_prefix(root) {
            Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
            Ok(rel) => rel.to_string_lossy().to_string(),
            Err(_) => canonical.to_string_lossy().to_string(),
        }
    }

    fn tool_result_for_error(&self, call: &ToolCall, error: FilesystemError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.message())
    }

    async fn read(&self, call: &ToolCall, path: &str) -> ToolResult {
        let canonical = match self.resolve_contained(path) {
            Ok(p) => p,
            Err(e) => return self.tool_result_for_error(call, e),
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(m) if m.is_file() => m,
            Ok(_) => {
                return self.tool_result_for_error(
                    call,
                    FilesystemError::InvalidInput(format!("{} is not a file", canonical.display())),
                )
            }
            Err(e) => return self.tool_result_for_error(call, FilesystemError::Io(e.to_string())),
        };

        if metadata.len() > self.max_file_size {
            return self.tool_result_for_error(
                call,
                FilesystemError::TooLarge(format!(
                    "{} is {} bytes (limit {} bytes)",
                    canonical.display(),
                    metadata.len(),
                    self.max_file_size
                )),
            );
        }

        match fs::read_to_string(&canonical) {
            Ok(content) => ToolResult::ok(&call.id, serde_json::Value::String(content)),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => self.tool_result_for_error(
                call,
                FilesystemError::NotUtf8(canonical.display().to_string()),
            ),
            Err(e) => self.tool_result_for_error(call, FilesystemError::Io(e.to_string())),
        }
    }

    async fn list(&self, call: &ToolCall, path: &str) -> ToolResult {
        let canonical = match self.resolve_contained(path) {
            Ok(p) => p,
            Err(e) => return self.tool_result_for_error(call, e),
        };
        let root = match self.canonical_root() {
            Ok(r) => r,
            Err(e) => return self.tool_result_for_error(call, e),
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(m) if m.is_dir() => m,
            Ok(_) => {
                return self.tool_result_for_error(
                    call,
                    FilesystemError::InvalidInput(format!(
                        "{} is not a directory",
                        canonical.display()
                    )),
                )
            }
            Err(e) => return self.tool_result_for_error(call, FilesystemError::Io(e.to_string())),
        };
        let _ = metadata;

        let mut entries = Vec::new();
        match fs::read_dir(&canonical) {
            Ok(reader) => {
                for entry in reader {
                    let Ok(entry) = entry else { continue };
                    let name = entry.file_name().to_string_lossy().to_string();
                    let kind = match entry.file_type() {
                        Ok(t) if t.is_dir() => "dir",
                        Ok(t) if t.is_file() => "file",
                        _ => "other",
                    };
                    entries.push((name.clone(), kind.to_string(), name));
                }
            }
            Err(e) => return self.tool_result_for_error(call, FilesystemError::Io(e.to_string())),
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let total = entries.len();
        let truncated = total > MAX_LIST_ENTRIES;
        entries.truncate(MAX_LIST_ENTRIES);

        let value = serde_json::json!({
            "path": self.relative_display(&canonical, &root),
            "count": entries.len(),
            "truncated": truncated,
            "entries": entries.into_iter().map(|(_, kind, name)| serde_json::json!({ "name": name, "kind": kind })).collect::<Vec<_>>(),
        });

        ToolResult::ok(&call.id, value)
    }

    async fn stat(&self, call: &ToolCall, path: &str) -> ToolResult {
        let canonical = match self.resolve_contained(path) {
            Ok(p) => p,
            Err(e) => return self.tool_result_for_error(call, e),
        };
        let root = match self.canonical_root() {
            Ok(r) => r,
            Err(e) => return self.tool_result_for_error(call, e),
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(m) => m,
            Err(e) => return self.tool_result_for_error(call, FilesystemError::Io(e.to_string())),
        };

        let kind = if metadata.is_dir() {
            "dir"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };

        let value = serde_json::json!({
            "path": self.relative_display(&canonical, &root),
            "kind": kind,
            "size": metadata.len(),
            "readonly": metadata.permissions().readonly(),
        });

        ToolResult::ok(&call.id, value)
    }
}

#[async_trait]
impl ToolCapability for FilesystemTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Filesystem operation: read, list, or stat",
                    "enum": ["read", "list", "stat"]
                },
                "path": {
                    "type": "string",
                    "description": "Path relative to the workspace root, or an absolute path inside it"
                }
            },
            "required": ["operation", "path"],
            "additionalProperties": false
        });
        let mut params = apeireth_protocol::canonical::ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());

        NormalizedTool::new("filesystem")
            .with_description("Read, list, or stat files and directories inside the workspace root. Read-only; write/delete are not available.")
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: FilesystemParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid filesystem parameters: {e}"),
                )
            }
        };

        match params.operation.to_lowercase().as_str() {
            "read" => self.read(call, &params.path).await,
            "list" => self.list(call, &params.path).await,
            "stat" => self.stat(call, &params.path).await,
            other => ToolResult::permanent_error(
                &call.id,
                format!(
                    "unknown filesystem operation {other:?}; allowed operations: read, list, stat"
                ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(root: &Path) -> FilesystemTool {
        FilesystemTool::new(root.to_path_buf())
    }

    async fn invoke(tool: &FilesystemTool, operation: &str, path: &str) -> ToolResult {
        let call = ToolCall {
            id: "call_1".into(),
            name: "filesystem".into(),
            arguments: json!({ "operation": operation, "path": path }),
        };
        tool.invoke(&call).await
    }

    #[tokio::test]
    async fn read_file_returns_utf8_contents() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.txt"), "hello").unwrap();

        let result = invoke(&tool(dir.path()), "read", "hello.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.render(), "hello");
    }

    #[tokio::test]
    async fn read_missing_file_is_a_structured_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = invoke(&tool(dir.path()), "read", "missing.txt").await;
        assert!(!result.is_ok());
        assert!(result.render().contains("not found"), "{}", result.render());
    }

    #[tokio::test]
    async fn read_rejects_files_over_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("big.txt"), b"0123456789".repeat(10)).unwrap();
        let tool = FilesystemTool::new(dir.path().to_path_buf()).with_max_file_size(16);

        let result = invoke(&tool, "read", "big.txt").await;
        assert!(!result.is_ok());
        assert!(result.render().contains("too large"), "{}", result.render());
    }

    #[tokio::test]
    async fn read_rejects_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.bin"), [0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let result = invoke(&tool(dir.path()), "read", "bad.bin").await;
        assert!(!result.is_ok());
        assert!(result.render().contains("UTF-8"), "{}", result.render());
    }

    #[tokio::test]
    async fn list_dir_is_sorted_and_contains_nested_unicode_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("b.txt"), "b").unwrap();
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        fs::write(dir.path().join("嵌套.txt"), "unicode").unwrap();

        let result = invoke(&tool(dir.path()), "list", ".").await;
        assert!(result.is_ok());
        let value = result.render();
        assert!(value.contains("a.txt"), "{value}");
        assert!(value.contains("b.txt"), "{value}");
        assert!(value.contains("嵌套.txt"), "{value}");
        let a = value.find("a.txt").unwrap();
        let b = value.find("b.txt").unwrap();
        assert!(a < b, "entries should be sorted: {value}");
    }

    #[tokio::test]
    async fn stat_returns_file_and_dir_metadata() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/a.txt"), b"0123456789").unwrap();

        let file = invoke(&tool(dir.path()), "stat", "sub/a.txt").await;
        assert!(file.is_ok());
        let file_value = file.render();
        assert!(file_value.contains("\"kind\":\"file\""), "{file_value}");

        let sub = invoke(&tool(dir.path()), "stat", "sub").await;
        assert!(sub.is_ok());
        let sub_value = sub.render();
        assert!(sub_value.contains("\"kind\":\"dir\""), "{sub_value}");
    }

    #[tokio::test]
    async fn path_traversal_outside_root_is_denied() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside.txt");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "secret").unwrap();

        let result = invoke(&tool(&root), "read", "../outside.txt").await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("permission denied"),
            "{}",
            result.render()
        );
    }

    #[tokio::test]
    async fn symlink_escape_is_denied_when_supported() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside.txt");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "secret").unwrap();

        let link = root.join("link.txt");
        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&outside, &link).is_ok();
        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_file(&outside, &link).is_ok();
        if !created {
            // Symlink creation often needs extra privileges on Windows.
            return;
        }

        let result = invoke(&tool(&root), "read", "link.txt").await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("permission denied"),
            "{}",
            result.render()
        );
    }
}
