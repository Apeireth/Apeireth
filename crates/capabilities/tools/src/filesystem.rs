//! Read-only filesystem tool capability.
//!
//! The tool reads, lists, and stats paths under a caller-supplied workspace
//! root. Path resolution is canonicalized and checked for containment inside
//! that root before any operation, so `..` and symlink traversal do not escape
//! the root for the operations implemented here. Known credential and key
//! paths are protected by the shared workspace path policy.
//!
//! Write/delete/rename/copy are deliberately not implemented in M2A. They are
//! deferred to the sandbox phase (M2B). This tool does **not** claim to be a
//! process/filesystem sandbox.
//!
//! 读前观测接线: `read` 把读到的版本 (len, mtime) 记入共享的
//! [`ObservedGate`] (读事件捎带版本, 零额外 IO); 读到「目标不存在」时记
//! 「已观测为不存在」。写入端凭这些观测过读前观测门禁 (未读不得覆盖写)。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

use crate::observed_gate::{FileVersion, ObservedGate};
use crate::sensitive_path::is_sensitive_path;

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
    observed_gate: Arc<ObservedGate>,
}

impl FilesystemTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            id: CapabilityId::new("tool.filesystem").unwrap(),
            root: root.into(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            observed_gate: Arc::new(ObservedGate::new()),
        }
    }

    /// Override the maximum file size accepted by `read`.
    #[must_use]
    pub fn with_max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = max_file_size;
        self
    }

    /// 接入共享的读前观测门禁 (读工具记录的观测与写入端共用一张表)。
    #[must_use]
    pub fn with_observed_gate(mut self, observed_gate: Arc<ObservedGate>) -> Self {
        self.observed_gate = observed_gate;
        self
    }

    /// 本工具写入观测的门禁 (供写入端共享)。
    pub fn observed_gate(&self) -> &Arc<ObservedGate> {
        &self.observed_gate
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

        // M13 (2026-09-24 审计): guardrail 前置守门接线 —— 路径解析前先过
        // 词法层 (相对穿越 / 绝对敏感系统路径) 与绝对路径的 root 包含检查。
        // 与下方 canonicalize 现实层校验构成双层: 守门拒词法攻击面, 现实层
        // 拒 symlink 逃逸。
        crate::guardrail::ToolGuardrail::verify_path_access(&root, &candidate).map_err(|e| {
            // 就地提示: 目录边界拒绝携带一次性升级引导 (缺什么模式 / 需要
            // 什么理由字段); 词法攻击面与受保护路径拒绝维持原文 —— 那两类
            // 没有可申请的升级档位。
            let message = match e {
                crate::guardrail::PreCallGuardError::OutsideWorkspace(detail) => format!(
                    "{detail}; {}",
                    crate::escalation::UpgradeHint::out_of_workspace_access()
                ),
                ref other => other.to_string(),
            };
            FilesystemError::PermissionDenied(message)
        })?;

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
            // 就地提示: 拒绝消息直接携带"如何申请本次升级"的结构化引导
            // (缺什么模式 / 需要什么理由字段), 在决策点引导一次性目录外授权。
            return Err(FilesystemError::PermissionDenied(format!(
                "{} resolves outside the workspace root; {}",
                candidate.display(),
                crate::escalation::UpgradeHint::out_of_workspace_access()
            )));
        }

        if is_sensitive_path(&root, &candidate) || is_sensitive_path(&root, &canonical) {
            return Err(FilesystemError::PermissionDenied(
                "requested path is protected".to_string(),
            ));
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

    /// 请求拼写的候选路径 (未经 canonicalize) —— 观测键的第二种拼写,
    /// 与 `resolve_contained` 的候选同构, 跨工具拼写差异不丢观测。
    fn observed_spelling(&self, requested: &str) -> PathBuf {
        let candidate = Path::new(requested);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.root.join(candidate)
        }
    }

    fn tool_result_for_error(&self, call: &ToolCall, error: FilesystemError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.message())
    }

    async fn read(&self, call: &ToolCall, path: &str) -> ToolResult {
        let request_path = self.observed_spelling(path);
        let canonical = match self.resolve_contained(path) {
            Ok(p) => p,
            Err(e) => {
                if matches!(e, FilesystemError::NotFound(_)) {
                    // 读到「目标不存在」也是观测事件: 记「已观测为不存在」,
                    // 创建类写入凭它过门禁 (createIfAbsent 钥匙仍会再核对现状)。
                    self.observed_gate.observe_missing(&request_path);
                }
                return self.tool_result_for_error(call, e);
            }
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
        let _ = metadata;

        // L 组 (2026-09-24 审计): 不再信任 metadata 预检的大小 —— 检查与使用
        // 之间文件可能增长 (TOCTOU), read_to_string 会把增长后的整个文件读进
        // 内存。改为 open + `Read::take(max+1)` 按实读字节判定: 实读 > max
        // 即超限, 与增长竞态无关。
        let mut content_bytes = Vec::new();
        // 声明不初始化: 下方 Ok 分支必赋值, Err 分支早退 —— 初始化 None 是
        // 死值 (unused_assignments)。
        let mut observed_version;
        match fs::OpenOptions::new().read(true).open(&canonical) {
            Ok(file) => {
                // 版本取自已开句柄的元数据 (fstat, 零额外 IO), 与实读内容同源。
                observed_version = file.metadata().ok().map(|m| FileVersion::from_metadata(&m));
                let mut limited = file.take(self.max_file_size.saturating_add(1));
                if let Err(e) = limited.read_to_end(&mut content_bytes) {
                    return self.tool_result_for_error(call, FilesystemError::Io(e.to_string()));
                }
            }
            Err(e) => return self.tool_result_for_error(call, FilesystemError::Io(e.to_string())),
        }

        if content_bytes.len() as u64 > self.max_file_size {
            return self.tool_result_for_error(
                call,
                FilesystemError::TooLarge(format!(
                    "{} exceeds the {} byte limit",
                    canonical.display(),
                    self.max_file_size
                )),
            );
        }

        match String::from_utf8(content_bytes) {
            Ok(content) => {
                if let Some(version) = observed_version {
                    // 读事件记录观测: canonical 与请求拼写都记,
                    // 跨工具拼写差异落到同一观测。
                    self.observed_gate.observe_present(&canonical, version);
                    self.observed_gate.observe_present(&request_path, version);
                }
                ToolResult::ok(&call.id, serde_json::Value::String(content))
            }
            Err(_) => self.tool_result_for_error(
                call,
                FilesystemError::NotUtf8(canonical.display().to_string()),
            ),
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
                    if is_sensitive_path(&root, &entry.path()) {
                        continue;
                    }
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
            .with_description("Read, list, or stat non-sensitive files and directories inside the workspace root. Read-only; write/delete are not available.")
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
    async fn read_records_observed_version_for_the_write_gate() {
        // 读事件捎带版本: read 成功后门禁记「已观测为存在 + (len, mtime)」,
        // 请求拼写与 canonical 拼写都能查到同一观测。
        use crate::observed_gate::ObservationState;

        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.txt"), "hello").unwrap();
        let tool = tool(dir.path());

        let result = invoke(&tool, "read", "hello.txt").await;
        assert!(result.is_ok());

        let requested = dir.path().join("hello.txt");
        match tool.observed_gate().state(&requested) {
            ObservationState::Present(version) => assert_eq!(version.len, 5),
            other => panic!("expected Present observation, got {other:?}"),
        }
        let on_disk = FileVersion::from_metadata(&fs::metadata(&requested).unwrap());
        assert_eq!(
            tool.observed_gate().observed_version(&requested),
            Some(on_disk),
            "观测版本必须与磁盘现状一致 (写入端 CAS 钥匙取自它)"
        );
        let canonical = fs::canonicalize(&requested).unwrap();
        assert_eq!(
            tool.observed_gate().state(&canonical),
            ObservationState::Present(on_disk)
        );
    }

    #[tokio::test]
    async fn read_missing_file_records_missing_observation_for_create() {
        // 读到「目标不存在」→ 记「已观测为不存在」, 创建类写入凭它过门禁。
        use crate::observed_gate::ObservationState;

        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        let result = invoke(&tool, "read", "missing.txt").await;
        assert!(!result.is_ok());
        assert_eq!(
            tool.observed_gate().state(&dir.path().join("missing.txt")),
            ObservationState::Missing
        );
    }

    #[tokio::test]
    async fn read_observation_lets_the_gated_write_pass_the_cas_key() {
        // 闭环: 读工具记录的观测直接让写入端过「读后可写」的版本 CAS;
        // 未读过的兄弟文件仍被「未读不得覆盖写」拦下。
        use crate::observed_gate::{
            gated_write_atomic, GateDenial, GatedWriteError, WriteKind, WriteRequest,
        };

        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("cfg.toml"), "a = 1").unwrap();
        fs::write(dir.path().join("sibling.toml"), "b = 1").unwrap();
        let gate = Arc::new(ObservedGate::new());
        let tool = FilesystemTool::new(dir.path().to_path_buf()).with_observed_gate(gate.clone());

        assert!(invoke(&tool, "read", "cfg.toml").await.is_ok());
        let target = dir.path().join("cfg.toml");
        let observed = gate.observed_version(&target).expect("read 必须已记录观测");
        let request = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
        };
        gated_write_atomic(
            &gate,
            &crate::observed_gate::FsVersionProbe,
            &request,
            b"a = 2",
            apeireth_core::storage_atomic::DEFAULT_FILE_MODE,
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "a = 2");

        let sibling = WriteRequest {
            path: dir.path().join("sibling.toml"),
            kind: WriteKind::Replace,
            replace_if_version: None,
            create_if_absent: false,
        };
        let err = gated_write_atomic(
            &gate,
            &crate::observed_gate::FsVersionProbe,
            &sibling,
            b"b = 2",
            apeireth_core::storage_atomic::DEFAULT_FILE_MODE,
        )
        .unwrap_err();
        assert!(
            matches!(err, GatedWriteError::Denied(GateDenial::NotObserved(_))),
            "未读过的文件必须被拒, got {err:?}"
        );
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
    async fn read_accepts_file_at_exactly_the_limit() {
        // L 组: take(max+1) 语义 —— 恰好等于上限必须放行 (实读 == max 不截断)。
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("exact.txt"), "0123456789").unwrap();
        let tool = FilesystemTool::new(dir.path().to_path_buf()).with_max_file_size(10);

        let result = invoke(&tool, "read", "exact.txt").await;
        assert!(result.is_ok(), "{}", result.render());
        assert_eq!(result.render(), "0123456789");
    }

    #[tokio::test]
    async fn read_rejects_one_byte_over_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("over.txt"), "0123456789a").unwrap();
        let tool = FilesystemTool::new(dir.path().to_path_buf()).with_max_file_size(10);

        let result = invoke(&tool, "read", "over.txt").await;
        assert!(!result.is_ok());
        assert!(result.render().contains("too large"), "{}", result.render());
    }

    #[tokio::test]
    async fn read_rejects_absolute_path_outside_root() {
        // M13 接线 + 现实层双重拒绝: root 外的绝对路径一律 permission denied。
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside.txt");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "secret").unwrap();

        let result = invoke(&tool(&root), "read", outside.to_str().unwrap()).await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("permission denied"),
            "{}",
            result.render()
        );
    }

    /// 就地提示: 目录边界的拒绝消息必须携带结构化升级引导 (缺什么模式 /
    /// 需要什么理由字段 / 一次性范围), 在决策点引导而非让用户去翻设置。
    #[tokio::test]
    async fn outside_workspace_refusal_carries_the_upgrade_hint() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = base.path().join("outside.txt");
        fs::create_dir(&root).unwrap();
        fs::write(&outside, "secret").unwrap();

        let result = invoke(&tool(&root), "read", outside.to_str().unwrap()).await;
        let rendered = result.render();
        assert!(!result.is_ok());
        for expected in [
            "\"missing_mode\":\"relaxed\"",
            "\"justification_field\":\"justification\"",
            "\"grant_scope\":\"one_call\"",
            "\"request_key\":\"sandbox_escalation\"",
        ] {
            assert!(rendered.contains(expected), "{expected} in {rendered}");
        }
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

    #[tokio::test]
    async fn sensitive_paths_are_denied_for_read_and_stat() {
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
            ".npmrc",
            "token.txt",
        ] {
            fs::write(dir.path().join(path), "protected").unwrap();
        }
        fs::create_dir_all(dir.path().join(".ssh")).unwrap();
        fs::write(dir.path().join(".ssh/config"), "protected").unwrap();
        fs::create_dir_all(dir.path().join(".kube")).unwrap();
        fs::write(dir.path().join(".kube/config"), "protected").unwrap();
        fs::create_dir_all(dir.path().join(".docker")).unwrap();
        fs::write(dir.path().join(".docker/config.json"), "protected").unwrap();
        fs::create_dir_all(dir.path().join(".config/gcloud")).unwrap();
        fs::write(
            dir.path()
                .join(".config/gcloud/application_default_credentials.json"),
            "protected",
        )
        .unwrap();

        let tool = tool(dir.path());
        for path in [
            ".env",
            ".env.local",
            "foo.pem",
            "foo.key",
            "id_rsa",
            "id_ed25519",
            "credentials.json",
            "secrets.production",
            ".npmrc",
            "token.txt",
            ".ssh/config",
            ".kube/config",
            ".docker/config.json",
            ".config/gcloud/application_default_credentials.json",
        ] {
            let read = invoke(&tool, "read", path).await;
            assert!(!read.is_ok(), "read unexpectedly allowed: {path}");
            assert!(
                read.render().contains("protected"),
                "{path}: {}",
                read.render()
            );

            let stat = invoke(&tool, "stat", path).await;
            assert!(!stat.is_ok(), "stat unexpectedly allowed: {path}");
            assert!(
                stat.render().contains("protected"),
                "{path}: {}",
                stat.render()
            );
        }
    }

    #[tokio::test]
    async fn list_filters_sensitive_entries_but_keeps_normal_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".env"), "protected").unwrap();
        fs::write(dir.path().join("credentials.json"), "protected").unwrap();
        fs::write(dir.path().join("foo.pem"), "protected").unwrap();
        fs::create_dir_all(dir.path().join(".ssh")).unwrap();
        fs::write(dir.path().join(".ssh/config"), "protected").unwrap();
        fs::create_dir_all(dir.path().join(".cargo")).unwrap();
        fs::write(dir.path().join(".cargo/config.toml"), "normal").unwrap();
        fs::write(dir.path().join(".gitignore"), "normal").unwrap();
        fs::write(dir.path().join("README.md"), "normal").unwrap();

        let result = invoke(&tool(dir.path()), "list", ".").await;
        assert!(result.is_ok());
        let rendered = result.render();
        for hidden in [".env", "credentials.json", "foo.pem", ".ssh"] {
            assert!(
                !rendered.contains(hidden),
                "sensitive entry leaked: {hidden}: {rendered}"
            );
        }
        for visible in [".cargo", ".gitignore", "README.md"] {
            assert!(
                rendered.contains(visible),
                "normal entry missing: {visible}: {rendered}"
            );
        }
    }

    #[tokio::test]
    async fn normal_project_files_and_dotfiles_remain_readable() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        for path in ["README.md", "Cargo.toml", "src/lib.rs", ".gitignore"] {
            fs::write(dir.path().join(path), "normal").unwrap();
        }

        let tool = tool(dir.path());
        for path in ["README.md", "Cargo.toml", "src/lib.rs", ".gitignore"] {
            let result = invoke(&tool, "read", path).await;
            assert!(
                result.is_ok(),
                "normal file was blocked: {path}: {}",
                result.render()
            );
            assert_eq!(result.render(), "normal");
        }
    }
}
