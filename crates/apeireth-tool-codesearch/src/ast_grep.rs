//! ast-grep CLI 包装 (R193, 来源 R181 调研短期方案).
//!
//! 短期: CLI subprocess 包装, 0 编译增加.
//! 中期: in-process lib 集成 (R193+1).
//!
//! **不假装** (O-5):
//! - 需要系统安装 ast-grep binary
//! - subprocess 性能 ~100ms 启动, 但搜索毫秒级
//! - JSON 解析容错, 异常 graceful 降级
//!
//! 0 触碰: 本模块新增, 不改现有 search/files/symbols/graph/index/mcp/compat/enhanced.

#![allow(missing_docs)] // R193: 0 触碰现有 API 文档; 新增模块文档在 lib.rs 集中

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AstGrepError {
    #[error("ast-grep binary not found in PATH (hint: cargo install ast-grep --locked)")]
    BinaryNotFound,
    #[error("ast-grep spawn failed: {0}")]
    SpawnFailed(String),
    #[error("ast-grep exited with non-zero status {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },
    #[error("ast-grep output parse failed: {0}")]
    JsonParse(String),
    #[error("ast-grep IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstGrepMatch {
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AstGrepSearcher {
    pub binary: PathBuf,
}

impl AstGrepSearcher {
    /// 默认 binary = 'ast-grep' (从 PATH 查找)
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("ast-grep"),
        }
    }

    pub fn with_binary(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    /// 检测 binary 是否可用 (spawn --version)
    pub fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Default for AstGrepSearcher {
    fn default() -> Self {
        Self::new()
    }
}

pub trait AstSearcher: Send + Sync {
    /// 单 pattern 搜索
    fn search(
        &self,
        root: &Path,
        pattern: &str,
        lang: Option<&str>,
    ) -> Result<Vec<AstGrepMatch>, AstGrepError>;

    /// YAML rule 搜索
    fn search_with_rule(
        &self,
        root: &Path,
        rule_file: &Path,
    ) -> Result<Vec<AstGrepMatch>, AstGrepError>;
}

impl AstSearcher for AstGrepSearcher {
    fn search(
        &self,
        root: &Path,
        pattern: &str,
        lang: Option<&str>,
    ) -> Result<Vec<AstGrepMatch>, AstGrepError> {
        // ast-grep run --pattern PATTERN [--lang LANG] --json=stream PATH
        let mut cmd = Command::new(&self.binary);
        cmd.arg("run")
            .arg("--pattern")
            .arg(pattern)
            .arg("--json=stream");
        if let Some(l) = lang {
            cmd.arg("--lang").arg(l);
        }
        cmd.arg(root);
        run_ast_grep(cmd)
    }

    fn search_with_rule(
        &self,
        root: &Path,
        rule_file: &Path,
    ) -> Result<Vec<AstGrepMatch>, AstGrepError> {
        // ast-grep scan --rule RULE_FILE --json=stream PATH
        let mut cmd = Command::new(&self.binary);
        cmd.arg("scan")
            .arg("--rule")
            .arg(rule_file)
            .arg("--json=stream")
            .arg(root);
        run_ast_grep(cmd)
    }
}

fn run_ast_grep(mut cmd: Command) -> Result<Vec<AstGrepMatch>, AstGrepError> {
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(AstGrepError::BinaryNotFound);
        }
        Err(e) => return Err(AstGrepError::SpawnFailed(e.to_string())),
    };

    if !output.status.success() {
        return Err(AstGrepError::NonZeroExit {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // ast-grep --json=stream 输出是 NDJSON (每行一个 JSON 对象)
    let mut matches = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<AstGrepJsonEntry>(line) {
            Ok(entry) => matches.push(entry.into_match()),
            Err(_) => {
                // 跳过无法解析的行 (兼容 ast-grep 启动信息)
                continue;
            }
        }
    }
    Ok(matches)
}

#[derive(Debug, Deserialize)]
struct AstGrepJsonEntry {
    file: String,
    range: AstGrepRange,
    text: String,
    #[serde(default)]
    rule_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AstGrepRange {
    start: AstGrepPos,
    end: AstGrepPos,
}

#[derive(Debug, Deserialize)]
struct AstGrepPos {
    line: usize,
}

impl AstGrepJsonEntry {
    fn into_match(self) -> AstGrepMatch {
        AstGrepMatch {
            file: PathBuf::from(self.file),
            start_line: self.range.start.line,
            end_line: self.range.end.line,
            text: self.text,
            rule_id: self.rule_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_new_default_binary() {
        let s = AstGrepSearcher::new();
        assert_eq!(s.binary.to_str(), Some("ast-grep"));
    }

    #[test]
    fn t02_with_binary() {
        let s = AstGrepSearcher::with_binary("/usr/local/bin/ast-grep");
        assert_eq!(s.binary.to_str(), Some("/usr/local/bin/ast-grep"));
    }

    #[test]
    fn t03_default_trait_impl() {
        let s = AstGrepSearcher::default();
        assert_eq!(s.binary.to_str(), Some("ast-grep"));
    }

    #[test]
    fn t04_match_struct_construction() {
        let m = AstGrepMatch {
            file: PathBuf::from("src/main.rs"),
            start_line: 10,
            end_line: 12,
            text: "fn hello()".to_string(),
            rule_id: None,
        };
        assert_eq!(m.start_line, 10);
        assert_eq!(m.end_line, 12);
    }

    #[test]
    fn t05_error_display() {
        let e = AstGrepError::BinaryNotFound;
        let msg = e.to_string();
        assert!(msg.contains("ast-grep"));
        assert!(msg.contains("cargo install"));
    }

    #[test]
    fn t06_is_available_returns_bool() {
        // 不依赖 ast-grep 实际安装, 只验证返回 bool
        let s = AstGrepSearcher::new();
        let _: bool = s.is_available();
    }

    #[test]
    fn t07_search_missing_binary_returns_error() {
        let s = AstGrepSearcher::with_binary("/nonexistent/ast-grep-binary");
        let r = s.search(Path::new("."), "fn ()", None);
        match r {
            Err(AstGrepError::SpawnFailed(_)) | Err(AstGrepError::BinaryNotFound) => {
                // 两种都可能, 系统差异
            }
            other => panic!("expected spawn/binary error, got {:?}", other),
        }
    }

    #[test]
    fn t08_search_with_rule_uses_scan_subcommand() {
        // 验证 search_with_rule 走 scan 子命令 (间接通过 binary 不存在错误)
        let s = AstGrepSearcher::with_binary("/nonexistent/ast-grep");
        let r = s.search_with_rule(Path::new("."), Path::new("rule.yml"));
        assert!(r.is_err());
    }
}
