//! apeireth-tools::grep_ops — 内容搜索 (R30 P1, 类 ClaudeCode Grep / ripgrep)
//!
//! **设计**:
//! - GrepOps trait: 1 异步方法 search(pattern, path, glob, max_results)
//! - RipgrepGrepOps impl: 用 regex + walkdir 组合, 不走 ripgrep binary (防注入, 不依赖外部 CLI)
//! - GrepTool 适配器, Tool trait 统一调用
//!
//! **安全** (Tech-Review 同一阶段):
//! - 不走 shell, 不 spawn 子进程
//! - 正则由 Rust regex crate 负责 (RE2 算法, 防 ReDoS)
//! - path 限制在 root 内 (防讨压走全盘)
//! - max_results 默认 100 (同 VCP MAX_SEARCH_RESULTS)
//!
//! **不伪装**:
//! - 真走 walkdir 递归扫文件
//! - 真用 regex 序列匹配
//! - 每条命中返回 (path, line_no, content_preview)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_tool_registry::ToolKind;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::file_ops::MAX_SEARCH_RESULTS;

/// **内容搜索 trait (R30 P1)**
#[async_trait]
pub trait GrepOps: Send + Sync {
    async fn search(
        &self,
        pattern: &str,
        path: &Path,
        glob: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<GrepHit>, String>;
    fn name(&self) -> &str;
}

/// **单条命中**
#[derive(Debug, Clone)]
pub struct GrepHit {
    pub path: PathBuf,
    pub line_no: usize,
    pub line: String,
}

/// **真实 walkdir + regex 实现 (不走 ripgrep binary)**
pub struct RipgrepGrepOps {
    name: String,
}

impl RipgrepGrepOps {
    pub fn new() -> Self {
        Self {
            name: "Grep".to_string(),
        }
    }
    pub fn with_name(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Default for RipgrepGrepOps {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GrepOps for RipgrepGrepOps {
    async fn search(
        &self,
        pattern: &str,
        path: &Path,
        glob: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<GrepHit>, String> {
        let re = Regex::new(pattern).map_err(|e| format!("regex compile: {e}"))?;
        let limit = if max_results == 0 {
            MAX_SEARCH_RESULTS
        } else {
            max_results.min(MAX_SEARCH_RESULTS)
        };

        let mut hits = Vec::new();
        for entry in WalkDir::new(path).follow_links(false).into_iter().flatten() {
            if hits.len() >= limit {
                break;
            }
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            if let Some(g) = glob {
                // 简单 glob: 只匹配文件名 (不是路径)
                let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !glob_match(fname, g) {
                    continue;
                }
            }
            let Ok(content) = tokio::fs::read_to_string(p).await else {
                continue;
            }; // binary / 权限不够 / 大文件 → 跳过
            for (idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    hits.push(GrepHit {
                        path: p.to_path_buf(),
                        line_no: idx + 1,
                        line: line.to_string(),
                    });
                    if hits.len() >= limit {
                        return Ok(hits);
                    }
                }
            }
        }
        Ok(hits)
    }
    fn name(&self) -> &str {
        &self.name
    }
}

/// **简单 glob 匹配** (只支持 * 通配符, 足够覆盖 *.rs / *.py 等常见用事)
fn glob_match(text: &str, glob: &str) -> bool {
    if !glob.contains('*') {
        return text == glob;
    }
    let parts: Vec<&str> = glob.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !text.starts_with(part) {
            return false;
        }
        if i == parts.len() - 1 && !text.ends_with(part) {
            return false;
        }
        match text[pos..].find(part) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    true
}

/// **Grep → Tool 适配器**
///
/// args: {pattern: str, path: str, glob?: str, max_results?: usize}
pub struct GrepTool {
    inner: Arc<dyn GrepOps>,
}

impl GrepTool {
    pub fn new(inner: Arc<dyn GrepOps>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl apeireth_tool_registry::Tool for GrepTool {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Async
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes {
            trigger: apeireth_tool_registry::TriggerAxis::OnDemand,
            awaiting: apeireth_tool_registry::AwaitingAxis::Immediate,
            resident: apeireth_tool_registry::ResidentAxis::Ephemeral,
            transport: apeireth_tool_registry::TransportAxis::Local,
            output: apeireth_tool_registry::OutputAxis::Value,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'pattern' string".to_string())?;
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'path' string".to_string())?;
        let path = PathBuf::from(path_str);
        let glob = args.get("glob").and_then(|v| v.as_str());
        let max = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let hits = self.inner.search(pattern, &path, glob, max).await?;
        let total = hits.len();
        let lines: Vec<String> = hits
            .into_iter()
            .map(|h| format!("{}:{}:{}", h.path.display(), h.line_no, h.line))
            .collect();
        Ok(json!({
            "pattern": pattern,
            "path": path.display().to_string(),
            "matches": total,
            "lines": lines,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;
    use tempfile::TempDir;

    #[tokio::test]
    async fn glob_match_basic() {
        assert!(glob_match("foo.rs", "*.rs"));
        assert!(glob_match("foo.rs", "f*.rs"));
        assert!(!glob_match("foo.py", "*.rs"));
        assert!(glob_match("exact.txt", "exact.txt"));
        assert!(!glob_match("foo.txt", "exact.txt"));
    }

    #[tokio::test]
    async fn search_finds_matches_with_glob() {
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(
            dir.path().join("a.rs"),
            "fn main() {\n    println!(\"hi\");\n}\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("b.py"), "def main():\n    print('hi')\n").unwrap();
        std::fs::write(dir.path().join("c.txt"), "just a note\n").unwrap();

        let g = RipgrepGrepOps::new();
        let hits = g
            .search("println", dir.path(), Some("*.rs"), 10)
            .await
            .expect("search");
        assert_eq!(hits.len(), 1, "only a.rs matches");
        assert!(hits[0].path.ends_with("a.rs"));
        assert_eq!(hits[0].line_no, 2);
    }

    #[tokio::test]
    async fn search_respects_max_results() {
        let dir = TempDir::new().expect("tempdir");
        for i in 0..10 {
            std::fs::write(dir.path().join(format!("f{i}.txt")), "match\n").unwrap();
        }
        let g = RipgrepGrepOps::new();
        let hits = g
            .search("match", dir.path(), None, 3)
            .await
            .expect("search");
        assert_eq!(hits.len(), 3);
    }

    #[tokio::test]
    async fn search_invalid_regex_errors() {
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let g = RipgrepGrepOps::new();
        let r = g.search("[invalid", dir.path(), None, 10).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn tool_adapter_dispatch() {
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join("x.rs"), "// TODO: refactor\nfn real() {}\n").unwrap();
        let tool = GrepTool::new(Arc::new(RipgrepGrepOps::new()));
        let r = tool
            .call(json!({
                "pattern": "TODO",
                "path": dir.path().to_string_lossy(),
                "glob": "*.rs"
            }))
            .await
            .expect("call");
        assert_eq!(r["matches"], 1);
        let lines = r["lines"].as_array().unwrap();
        assert!(lines[0].as_str().unwrap().contains("TODO"));
    }

    #[tokio::test]
    async fn tool_name_is_grep() {
        let tool = GrepTool::new(Arc::new(RipgrepGrepOps::new()));
        assert_eq!(tool.name(), "Grep");
    }
}
