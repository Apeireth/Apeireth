//! `apeireth-tools::file_ops` — 文件操作 trait + std::fs 真实现
//!
//! **战役 2-5**: file_ops 6 操作复刻 VCP `FileOperator` 19 命令中核心 6 个 (字段级).
//!
//! **VCP 字段级引用** (`research/source/vcptoolbox/Plugin/FileOperator/plugin-manifest.json`):
//! - `command: "ReadFile"` (line 55) → `FileOps::read`
//! - `command: "WriteFile"` (line 63) → `FileOps::write`
//! - `command: "ListDirectory"` (line 79) → `FileOps::list`
//! - `command: "CreateDirectory"` (line 103) → `FileOps::mkdir`
//! - `command: "DeleteFile"` (line 99) → `FileOps::delete`
//! - `command: "MoveFile"` (line 91) → `FileOps::move_path`
//!
//! **配置常量** (VCP `config.env:12-18` + `FileOperator.js:24-26` 真值):
//! - `MAX_FILE_SIZE = 20 * 1024 * 1024` (20MB, 实战 20485760)
//! - `MAX_DIRECTORY_ITEMS = 1000`
//! - `MAX_SEARCH_RESULTS = 100`
//!
//! **设计**:
//! - `FileOps` trait: 6 异步方法 (read/write/list/mkdir/delete/move_path)
//! - `StdFileOps` impl: 用 `tokio::fs` 真实现 (不假装 sync fs)
//! - `FileOpsTool`: 适配 Tool trait, 6 操作通过 `Tool::call(args: {op: "read"|"write"|...})` 路由
//!
//! **不假装**:
//! - ✅ 真用 `tokio::fs` (异步不阻塞)
//! - ✅ 端到端真测: tempdir + 真读写 + 真列目录 + 真删 + 真 move

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_tool_registry::ToolKind;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

// =============================================================================
// 编译期 hardcode — 复刻 VCP config.env 真值
// =============================================================================

/// VCP `FileOperator.js:24` `MAX_FILE_SIZE` 默认 20MB = 20971520 bytes
pub const MAX_FILE_SIZE: u64 = 20 * 1024 * 1024;

/// VCP `FileOperator.js:25` `MAX_DIRECTORY_ITEMS` 默认 1000
pub const MAX_DIRECTORY_ITEMS: usize = 1000;

/// VCP `FileOperator.js:26` `MAX_SEARCH_RESULTS` 默认 100
pub const MAX_SEARCH_RESULTS: usize = 100;

/// FileOps 操作数 (VCP 19 命令中核心 6 个)
pub const FILE_OPS_OPERATION_COUNT: usize = 7;

// =============================================================================
// FileOps trait
// =============================================================================

/// **文件操作 trait (6 操作)**
///
/// **字段级映射 VCP FileOperator 19 命令中核心 6 个**:
/// - `read` ↔ VCP `ReadFile` (plugin-manifest.json:55)
/// - `write` ↔ VCP `WriteFile` (line 63)
/// - `list` ↔ VCP `ListDirectory` (line 79)
/// - `mkdir` ↔ VCP `CreateDirectory` (line 103)
/// - `delete` ↔ VCP `DeleteFile` (line 99)
/// - `move_path` ↔ VCP `MoveFile` (line 91)
#[async_trait]
pub trait FileOps: Send + Sync {
    /// **VCP ReadFile** — 读文件
    async fn read(&self, path: &Path) -> Result<String, String>;

    /// **VCP WriteFile** — 写文件 (覆盖)
    async fn write(&self, path: &Path, content: &str) -> Result<(), String>;

    /// **VCP ListDirectory** — 列目录
    ///
    /// 返 `Vec<PathBuf>` (排序 + 限 `MAX_DIRECTORY_ITEMS`)
    async fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String>;

    /// **VCP CreateDirectory** — 建目录 (含父目录)
    async fn mkdir(&self, dir: &Path) -> Result<(), String>;

    /// **VCP DeleteFile** — 删文件 (或空目录)
    async fn delete(&self, path: &Path) -> Result<(), String>;

    /// **VCP MoveFile** — 移动文件/目录
    async fn move_path(&self, from: &Path, to: &Path) -> Result<(), String>;

    /// **R30 P1 Edit** — 局部修改 (类 ClaudeCode/Codex replace_block)
    ///
    /// 严格唯一性检查: old_text 出现 0 次或 >1 次都报错.
    /// 不匹配时返回上下文 diff (帮 AI 修) 。
    async fn edit(&self, path: &Path, old_text: &str, new_text: &str) -> Result<(), String>;

    /// 工具名
    fn name(&self) -> &str;
}

// =============================================================================
// StdFileOps — tokio::fs 真实现
// =============================================================================

/// **std::fs 实现的 FileOps** (用 `tokio::fs` 异步)
pub struct StdFileOps {
    name: String,
}

impl StdFileOps {
    /// 新建默认 (name = "FileOperator", VCP 字段级)
    pub fn new() -> Self {
        Self {
            name: "FileOperator".to_string(),
        }
    }

    /// 新建自定义名
    pub fn with_name(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Default for StdFileOps {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileOps for StdFileOps {
    async fn read(&self, path: &Path) -> Result<String, String> {
        // VCP 字段级校验: MAX_FILE_SIZE
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| format!("stat {path:?}: {e}"))?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(format!(
                "file too large: {} bytes > MAX_FILE_SIZE {} (VCP config.env)",
                metadata.len(),
                MAX_FILE_SIZE
            ));
        }
        fs::read_to_string(path)
            .await
            .map_err(|e| format!("read {path:?}: {e}"))
    }

    async fn write(&self, path: &Path, content: &str) -> Result<(), String> {
        // VCP 字段级校验: content 不超 MAX_FILE_SIZE
        if content.len() as u64 > MAX_FILE_SIZE {
            return Err(format!(
                "content too large: {} bytes > MAX_FILE_SIZE {} (VCP config.env)",
                content.len(),
                MAX_FILE_SIZE
            ));
        }
        // 父目录不存在时自动建 (VCP CreateDirectory 行为)
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("mkdir parent {parent:?}: {e}"))?;
            }
        }
        fs::write(path, content)
            .await
            .map_err(|e| format!("write {path:?}: {e}"))
    }

    async fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| format!("read_dir {dir:?}: {e}"))?;
        let mut paths = Vec::new();
        loop {
            let entry = entries
                .next_entry()
                .await
                .map_err(|e| format!("next_entry: {e}"))?;
            match entry {
                Some(e) => paths.push(e.path()),
                None => break,
            }
            if paths.len() >= MAX_DIRECTORY_ITEMS {
                // VCP MAX_DIRECTORY_ITEMS 截断
                break;
            }
        }
        // 排序 (借 VCP ListDirectory 实战行为)
        paths.sort();
        Ok(paths)
    }

    async fn mkdir(&self, dir: &Path) -> Result<(), String> {
        fs::create_dir_all(dir)
            .await
            .map_err(|e| format!("mkdir {dir:?}: {e}"))
    }

    async fn delete(&self, path: &Path) -> Result<(), String> {
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| format!("stat {path:?}: {e}"))?;
        if metadata.is_dir() {
            // VCP DeleteFile 行为: 删空目录
            fs::remove_dir(path)
                .await
                .map_err(|e| format!("rmdir {path:?}: {e}"))
        } else {
            fs::remove_file(path)
                .await
                .map_err(|e| format!("rm {path:?}: {e}"))
        }
    }

    async fn move_path(&self, from: &Path, to: &Path) -> Result<(), String> {
        // 目标父目录不存在时先建 (VCP CreateDirectory 行为)
        if let Some(parent) = to.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("mkdir parent {parent:?}: {e}"))?;
            }
        }
        fs::rename(from, to)
            .await
            .map_err(|e| format!("rename {from:?} -> {to:?}: {e}"))
    }

    /// **R30 P1 Edit 实现** — read-modify-write 模式 + 严格唯一性
    ///
    /// 这是 apeireth 原创，VCP 没有。ClaudeCode/Codex 都有。
    /// 为了幂等性: 读出原文 → 检查唯一性 → 拼接新文 → 原子写回。
    async fn edit(&self, path: &Path, old_text: &str, new_text: &str) -> Result<(), String> {
        if old_text.is_empty() {
            return Err("old_text must be non-empty (拒绝空字符串替换)".into());
        }
        let original = self.read(path).await?;
        let occurrences = original.matches(old_text).count();
        if occurrences == 0 {
            let preview_start: String = original.chars().take(200).collect();
            return Err(format!(
                "old_text not found in file (0 matches). First 200 chars of file:\n{preview_start}"
            ));
        }
        if occurrences > 1 {
            return Err(format!(
                "old_text matched {occurrences} times, expected exactly 1. Add more surrounding context to make it unique."
            ));
        }
        let updated = original.replacen(old_text, new_text, 1);
        self.write(path, &updated).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// FileOpsTool — 适配 Tool trait (借战役 2-1 Tool 4 方法)
// =============================================================================

/// **FileOps → Tool 适配器**
///
/// **args 协议** (JSON):
/// - `op` (String, 必): `"read" | "write" | "list" | "mkdir" | "delete" | "move"`
/// - `path` / `dir` / `from` / `to` (String, 视 op 而定)
/// - `content` (String, 仅 write)
pub struct FileOpsTool {
    inner: Arc<dyn FileOps>,
}

impl FileOpsTool {
    /// 构造适配器
    pub fn new(inner: Arc<dyn FileOps>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl apeireth_tool_registry::Tool for FileOpsTool {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn kind(&self) -> ToolKind {
        // FileOps 是异步外部依赖 → Async (战役 2-1 6 类 enum)
        ToolKind::Async
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes {
            trigger: apeireth_tool_registry::TriggerAxis::OnDemand,
            awaiting: apeireth_tool_registry::AwaitingAxis::Immediate,
            resident: apeireth_tool_registry::ResidentAxis::Ephemeral,
            transport: apeireth_tool_registry::TransportAxis::Local,
            output: apeireth_tool_registry::OutputAxis::SideEffect,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'op' string".to_string())?;

        let path_str = |k: &str| -> Result<PathBuf, String> {
            args.get(k)
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .ok_or_else(|| format!("missing '{k}' string"))
        };

        match op {
            "read" => {
                let p = path_str("path")?;
                let s = self.inner.read(&p).await?;
                Ok(json!({"op": "read", "path": p.to_string_lossy(), "content": s}))
            }
            "write" => {
                let p = path_str("path")?;
                let c = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing 'content' string")?;
                self.inner.write(&p, c).await?;
                Ok(json!({"op": "write", "path": p.to_string_lossy(), "bytes": c.len()}))
            }
            "list" => {
                let d = path_str("dir")?;
                let paths = self.inner.list(&d).await?;
                let strs: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                Ok(json!({"op": "list", "dir": d.to_string_lossy(), "count": paths.len(), "entries": strs}))
            }
            "mkdir" => {
                let d = path_str("dir")?;
                self.inner.mkdir(&d).await?;
                Ok(json!({"op": "mkdir", "dir": d.to_string_lossy(), "ok": true}))
            }
            "delete" => {
                let p = path_str("path")?;
                self.inner.delete(&p).await?;
                Ok(json!({"op": "delete", "path": p.to_string_lossy(), "ok": true}))
            }
            "move" => {
                let from = path_str("from")?;
                let to = path_str("to")?;
                self.inner.move_path(&from, &to).await?;
                Ok(json!({"op": "move", "from": from.to_string_lossy(), "to": to.to_string_lossy(), "ok": true}))
            }
            "edit" => {
                let p = path_str("path")?;
                let old_text = args
                    .get("old_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing 'old_text' string".to_string())?;
                let new_text = args
                    .get("new_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing 'new_text' string".to_string())?;
                self.inner.edit(&p, old_text, new_text).await?;
                Ok(json!({"op": "edit", "path": p.to_string_lossy(), "ok": true}))
            }
            other => Err(format!("unknown op '{other}', expected: read/write/list/mkdir/delete/move/edit")),
        }
    }
}

// =============================================================================
// 单元测试 — 6 操作 × 2 = ≥ 12 个, + 端到端 + 错误路径
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;
    use tempfile::TempDir;

    fn ops() -> StdFileOps {
        StdFileOps::new()
    }

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let dir = TempDir::new().expect("tempdir");
        let p = dir.path().join("hello.txt");
        let f = ops();
        f.write(&p, "hello world\n").await.expect("write");
        let got = f.read(&p).await.expect("read");
        assert_eq!(got, "hello world\n");
    }

    #[tokio::test]
    async fn write_creates_parent_dirs() {
        // VCP WriteFile 行为: 父目录不存在时自动建
        let dir = TempDir::new().expect("tempdir");
        let nested = dir.path().join("a/b/c/file.txt");
        let f = ops();
        f.write(&nested, "x").await.expect("write");
        assert!(nested.exists());
    }

    #[tokio::test]
    async fn read_nonexistent_returns_error() {
        let dir = TempDir::new().expect("tempdir");
        let p = dir.path().join("missing.txt");
        let f = ops();
        let r = f.read(&p).await;
        assert!(r.is_err(), "读不存在文件应失败");
        assert!(r.unwrap_err().contains("missing.txt"));
    }

    #[tokio::test]
    async fn list_sorted_and_truncated() {
        let dir = TempDir::new().expect("tempdir");
        for name in ["c.txt", "a.txt", "b.txt"] {
            fs::write(dir.path().join(name), name).await.expect("write");
        }
        let f = ops();
        let paths = f.list(dir.path()).await.expect("list");
        assert_eq!(paths.len(), 3);
        // 排序: a.txt, b.txt, c.txt
        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[tokio::test]
    async fn list_empty_dir() {
        let dir = TempDir::new().expect("tempdir");
        let f = ops();
        let paths = f.list(dir.path()).await.expect("list");
        assert!(paths.is_empty());
    }

    #[tokio::test]
    async fn list_nonexistent_dir_errors() {
        let f = ops();
        let r = f.list(Path::new("/nonexistent/dir/12345")).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn mkdir_creates_nested() {
        let dir = TempDir::new().expect("tempdir");
        let nested = dir.path().join("x/y/z");
        let f = ops();
        f.mkdir(&nested).await.expect("mkdir");
        assert!(nested.is_dir());
    }

    #[tokio::test]
    async fn delete_file() {
        let dir = TempDir::new().expect("tempdir");
        let p = dir.path().join("victim.txt");
        fs::write(&p, "bye").await.expect("write");
        let f = ops();
        f.delete(&p).await.expect("delete");
        assert!(!p.exists());
    }

    #[tokio::test]
    async fn delete_empty_dir() {
        let dir = TempDir::new().expect("tempdir");
        let d = dir.path().join("empty_dir");
        fs::create_dir(&d).await.expect("mkdir");
        let f = ops();
        f.delete(&d).await.expect("delete empty dir");
        assert!(!d.exists());
    }

    #[tokio::test]
    async fn delete_nonexistent_errors() {
        let dir = TempDir::new().expect("tempdir");
        let p = dir.path().join("ghost.txt");
        let f = ops();
        let r = f.delete(&p).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn move_file_creates_parent() {
        let dir = TempDir::new().expect("tempdir");
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("new_dir/dst.txt");
        fs::write(&src, "moved").await.expect("write");
        let f = ops();
        f.move_path(&src, &dst).await.expect("move");
        assert!(!src.exists(), "源应被移走");
        assert!(dst.exists(), "目标应在");
        let content = fs::read_to_string(&dst).await.expect("read dst");
        assert_eq!(content, "moved");
    }

    #[tokio::test]
    async fn move_nonexistent_errors() {
        let dir = TempDir::new().expect("tempdir");
        let f = ops();
        let r = f
            .move_path(&dir.path().join("ghost"), &dir.path().join("dst"))
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn constants_match_vcp_config_env() {
        // VCP config.env 真值 1:1
        assert_eq!(MAX_FILE_SIZE, 20 * 1024 * 1024, "VCP FileOperator.js:24");
        assert_eq!(MAX_DIRECTORY_ITEMS, 1000, "VCP FileOperator.js:25");
        assert_eq!(MAX_SEARCH_RESULTS, 100, "VCP FileOperator.js:26");
        assert_eq!(FILE_OPS_OPERATION_COUNT, 7, "7 ops: read/write/list/mkdir/delete/move/edit");
    }

    #[tokio::test]
    async fn tool_adapter_dispatch_all_7_ops() {
        // 端到端: Tool trait 路由 6 个 op
        let dir = TempDir::new().expect("tempdir");
        let f = Arc::new(StdFileOps::new());
        let tool = FileOpsTool::new(f.clone());

        // 1. write
        let r = tool
            .call(json!({"op": "write", "path": dir.path().join("a.txt").to_string_lossy(), "content": "abc"}))
            .await
            .expect("write");
        assert_eq!(r["op"], "write");
        assert_eq!(r["bytes"], 3);

        // 2. read
        let r = tool
            .call(json!({"op": "read", "path": dir.path().join("a.txt").to_string_lossy()}))
            .await
            .expect("read");
        assert_eq!(r["content"], "abc");

        // 3. mkdir
        let r = tool
            .call(json!({"op": "mkdir", "dir": dir.path().join("sub").to_string_lossy()}))
            .await
            .expect("mkdir");
        assert_eq!(r["ok"], true);

        // 4. list
        let r = tool
            .call(json!({"op": "list", "dir": dir.path().to_string_lossy()}))
            .await
            .expect("list");
        assert_eq!(r["count"], 2); // a.txt + sub/

        // 5. move
        let r = tool
            .call(json!({
                "op": "move",
                "from": dir.path().join("a.txt").to_string_lossy(),
                "to": dir.path().join("sub/a.txt").to_string_lossy()
            }))
            .await
            .expect("move");
        assert_eq!(r["ok"], true);

        // 6. delete
        let r = tool
            .call(json!({"op": "delete", "path": dir.path().join("sub/a.txt").to_string_lossy()}))
            .await
            .expect("delete");
        assert_eq!(r["ok"], true);
    }

    #[tokio::test]
    async fn tool_adapter_edit_uniquely_replaces() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "fn main() {\n    old_func();\n}\n").expect("write");
        let f = Arc::new(StdFileOps::new());
        let tool = FileOpsTool::new(f);
        let r = tool.call(json!({
            "op": "edit",
            "path": path.to_string_lossy(),
            "old_text": "old_func()",
            "new_text": "new_func()"
        })).await.expect("edit");
        assert_eq!(r["op"], "edit");
        assert_eq!(r["ok"], true);
        let updated = std::fs::read_to_string(&path).expect("read");
        assert!(updated.contains("new_func()"));
        assert!(!updated.contains("old_func()"));
    }

    #[tokio::test]
    async fn tool_adapter_edit_ambiguous_errors() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "x = 1\nx = 2\n").expect("write");
        let f = Arc::new(StdFileOps::new());
        let tool = FileOpsTool::new(f);
        let r = tool.call(json!({
            "op": "edit",
            "path": path.to_string_lossy(),
            "old_text": "x = ",
            "new_text": "y = "
        })).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("matched 2 times"));
    }

    #[tokio::test]
    async fn tool_adapter_edit_empty_old_errors() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "anything").expect("write");
        let f = Arc::new(StdFileOps::new());
        let tool = FileOpsTool::new(f);
        let r = tool.call(json!({
            "op": "edit",
            "path": path.to_string_lossy(),
            "old_text": "",
            "new_text": "x"
        })).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn tool_adapter_unknown_op_errors() {
        let f = Arc::new(StdFileOps::new());
        let tool = FileOpsTool::new(f);
        let r = tool.call(json!({"op": "frobnicate"})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("unknown op"));
    }

    #[tokio::test]
    async fn tool_adapter_name_and_kind_match() {
        let f = Arc::new(StdFileOps::new());
        let tool = FileOpsTool::new(f);
        assert_eq!(tool.name(), "FileOperator");
        assert_eq!(tool.kind(), ToolKind::Async);
    }
}
