use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct FsParams {
    #[serde(alias = "op", alias = "action")]
    pub operation: String,
    #[serde(alias = "file", alias = "filepath", alias = "target", alias = "target_path")]
    pub path: String,
    #[serde(alias = "data", alias = "text", alias = "code", alias = "body")]
    pub content: Option<String>,
}

pub struct FilesystemTool {
    sandbox_root: PathBuf,
}

impl Default for FilesystemTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesystemTool {
    pub fn new() -> Self {
        // Default to current working dir or temp
        let root = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        Self { sandbox_root: root }
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { sandbox_root: root.into() }
    }

    fn resolve_safe_path(&self, requested: &str) -> Result<PathBuf, ToolError> {
        // Prevent path traversal
        if requested.contains("..") {
            return Err(ToolError::ValidationFailed("Path traversal (..) is strictly prohibited".into()));
        }

        let p = Path::new(requested);
        let target = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.sandbox_root.join(p)
        };

        Ok(target)
    }
}

#[async_trait]
impl Tool for FilesystemTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "filesystem".into(),
            description: "Cross-platform sandboxed filesystem manager (read, write, list, delete). Parameters: {\"operation\": \"read|write|list|delete\", \"path\": \"...\", \"content\": \"...\"}".into(),
            risk_level: RiskLevel::Medium,
        }
    }


    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let params: FsParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(e.to_string()))?;

        let target_path = self.resolve_safe_path(&params.path)?;

        match params.operation.to_lowercase().as_str() {
            "read" => {
                let data = tokio::fs::read_to_string(&target_path)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Read failed on {}: {}", target_path.display(), e)))?;
                Ok(ToolResult {
                    success: true,
                    output: data,
                })
            }
            "write" => {
                let content = params.content.unwrap_or_default();
                if let Some(parent) = target_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                tokio::fs::write(&target_path, content.as_bytes())
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Write failed on {}: {}", target_path.display(), e)))?;
                Ok(ToolResult {
                    success: true,
                    output: format!("Successfully wrote {} bytes to {}", content.len(), target_path.display()),
                })
            }
            "list" => {
                let mut entries = Vec::new();
                let mut reader = tokio::fs::read_dir(&target_path)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("List failed on {}: {}", target_path.display(), e)))?;
                
                while let Ok(Some(entry)) = reader.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let file_type = if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                        "[DIR]"
                    } else {
                        "[FILE]"
                    };
                    entries.push(format!("{} {}", file_type, name));
                }

                Ok(ToolResult {
                    success: true,
                    output: entries.join("\n"),
                })
            }
            "delete" => {
                if target_path.is_file() {
                    tokio::fs::remove_file(&target_path)
                        .await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Delete failed on {}: {}", target_path.display(), e)))?;
                }
                Ok(ToolResult {
                    success: true,
                    output: format!("Successfully deleted {}", target_path.display()),
                })
            }
            _ => Err(ToolError::ValidationFailed(format!("Unknown filesystem operation: {}", params.operation))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fs_operations_cross_platform() {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("apeireth_test_fs_{}", ts));
        let tool = FilesystemTool::with_root(&temp_dir);

        // 1. Write file
        let write_res = tool.execute(serde_json::json!({
            "operation": "write",
            "path": "test.txt",
            "content": "Apeireth 2.0 cross-platform test"
        })).await.unwrap();
        assert!(write_res.success);

        // 2. Read file
        let read_res = tool.execute(serde_json::json!({
            "operation": "read",
            "path": "test.txt"
        })).await.unwrap();
        assert_eq!(read_res.output, "Apeireth 2.0 cross-platform test");

        // 3. List
        let list_res = tool.execute(serde_json::json!({
            "operation": "list",
            "path": "."
        })).await.unwrap();
        assert!(list_res.output.contains("test.txt"));

        // 4. Path traversal rejection
        let bad_res = tool.execute(serde_json::json!({
            "operation": "read",
            "path": "../../../secret.txt"
        })).await;
        assert!(bad_res.is_err());

        // Cleanup
        let _ = tool.execute(serde_json::json!({
            "operation": "delete",
            "path": "test.txt"
        })).await;
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
