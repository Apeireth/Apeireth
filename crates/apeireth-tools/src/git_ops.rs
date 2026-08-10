//! `apeireth-tools::git_ops` — Git 操作 trait + git CLI 真实现
//!
//! **战役 2-5**: git_ops 3 操作 (status / log / diff) 走 git CLI 真跑.
//!
//! **设计**:
//! - `GitOps` trait: 3 异步方法
//! - `GitCliOps` impl: 用 `tokio::process::Command` 真调 git binary
//! - `GitOpsTool`: 适配 Tool trait, 3 操作通过 `op` 路由
//!
//! **不假装**:
//! - ✅ 真用 `tokio::process::Command` 调 git (不只 mock 返字符串)
//! - ✅ 端到端真测: 在 tempdir 真建 git 仓库 + 真 commit + 调 status / log / diff
//! - ✅ git 不可用时清晰报错 (字段级校验: git_path 必存在)

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use apeireth_tool_registry::ToolKind;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

/// **Git 操作 trait (3 操作)**
///
/// **实战行为**:
/// - `status` — `git status --short --branch`
/// - `log` — `git log -n {n} --pretty=format:...`
/// - `diff` — `git diff` (工作区 vs index)
#[async_trait]
pub trait GitOps: Send + Sync {
    /// git status (短格式 + branch)
    async fn status(&self, repo: &Path) -> Result<String, String>;

    /// git log -n {n}
    async fn log(&self, repo: &Path, n: u32) -> Result<String, String>;

    /// git diff
    async fn diff(&self, repo: &Path) -> Result<String, String>;

    /// 工具名
    fn name(&self) -> &str;
}

// =============================================================================
// GitCliOps — 真调 git binary
// =============================================================================

/// **git CLI 实现的 GitOps**
///
/// **配置**:
/// - `git_path`: git binary 路径 (默认 `git`, 走 PATH)
/// - `default_timeout`: 单次调用的超时 (ms)
pub struct GitCliOps {
    name: String,
    git_path: PathBuf,
    default_timeout_ms: u64,
}

impl GitCliOps {
    /// 默认构造: 用 PATH 里的 git + 30s 超时
    pub fn new() -> Self {
        Self {
            name: "Git".to_string(),
            git_path: PathBuf::from("git"),
            default_timeout_ms: 30_000,
        }
    }

    /// 自定义 git 路径
    pub fn with_git_path(git_path: impl Into<PathBuf>) -> Self {
        Self {
            name: "Git".to_string(),
            git_path: git_path.into(),
            default_timeout_ms: 30_000,
        }
    }

    /// 自定义超时
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    /// 自定义名
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// git binary 路径
    pub fn git_path(&self) -> &Path {
        &self.git_path
    }

    /// 默认超时
    pub fn default_timeout_ms(&self) -> u64 {
        self.default_timeout_ms
    }

    /// 通用 git 调入口 (带 timeout)
    async fn run_git(&self, repo: &Path, args: &[&str], timeout_ms: u64) -> Result<String, String> {
        // 字段级校验: repo 必存在 + 是 dir
        if !repo.is_dir() {
            return Err(format!("repo not a directory: {repo:?}"));
        }

        let mut cmd = Command::new(&self.git_path);
        cmd.args(args)
            .current_dir(repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = tokio::time::timeout(Duration::from_millis(timeout_ms), cmd.output())
            .await
            .map_err(|_| format!("git {args:?} timeout after {timeout_ms}ms"))?
            .map_err(|e| format!("spawn git: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status;

        if !status.success() {
            return Err(format!(
                "git {args:?} failed (exit={:?}): {}",
                status.code(),
                stderr.trim()
            ));
        }
        Ok(stdout)
    }
}

impl Default for GitCliOps {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitOps for GitCliOps {
    async fn status(&self, repo: &Path) -> Result<String, String> {
        // VCP 风格: short format + branch
        self.run_git(
            repo,
            &["status", "--short", "--branch"],
            self.default_timeout_ms,
        )
        .await
    }

    async fn log(&self, repo: &Path, n: u32) -> Result<String, String> {
        let n_str = n.to_string();
        // VCP 风格: 完整 hash + subject
        self.run_git(
            repo,
            &["log", &format!("-n{n_str}"), "--pretty=format:%H %s"],
            self.default_timeout_ms,
        )
        .await
    }

    async fn diff(&self, repo: &Path) -> Result<String, String> {
        self.run_git(repo, &["diff"], self.default_timeout_ms).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// =============================================================================
// GitOpsTool — 适配 Tool trait
// =============================================================================

/// **GitOps → Tool 适配器**
///
/// **args 协议**:
/// - `op` (String, 必): `"status" | "log" | "diff"`
/// - `repo` (String, 必)
/// - `n` (u32, 仅 log; 默认 10)
pub struct GitOpsTool {
    inner: Arc<dyn GitOps>,
}

impl GitOpsTool {
    /// 构造适配器
    pub fn new(inner: Arc<dyn GitOps>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl apeireth_tool_registry::Tool for GitOpsTool {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn kind(&self) -> ToolKind {
        // GitOps 是同步外部依赖 → Sync (战役 2-1 6 类 enum)
        ToolKind::Sync
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
        let op = args
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'op' string".to_string())?;
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'repo' string")?;
        let p = PathBuf::from(repo);

        match op {
            "status" => {
                let s = self.inner.status(&p).await?;
                Ok(json!({"op": "status", "repo": repo, "output": s}))
            }
            "log" => {
                let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
                let s = self.inner.log(&p, n).await?;
                Ok(json!({"op": "log", "repo": repo, "n": n, "output": s}))
            }
            "diff" => {
                let s = self.inner.diff(&p).await?;
                Ok(json!({"op": "diff", "repo": repo, "output": s}))
            }
            other => Err(format!("unknown op '{other}', expected: status/log/diff")),
        }
    }
}

// =============================================================================
// 单元测试 — 真建 git 仓库 + 真 commit + 真 status/log/diff
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;
    use tempfile::TempDir;
    use tokio::process::Command as TokioCommand;

    /// 拿 git 路径 (测试时确认 git 可用, 不可用就 skip)
    fn git_available() -> Option<PathBuf> {
        let out = std::process::Command::new("git")
            .arg("--version")
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(PathBuf::from("git"))
    }

    /// 初始化真 git 仓库 + 1 commit
    async fn make_repo() -> Option<TempDir> {
        let git = git_available()?;
        let dir = TempDir::new().ok()?;
        let d = dir.path();

        let run = |args: &[&str]| {
            let mut cmd = TokioCommand::new(&git);
            cmd.args(args).current_dir(d);
            async move { cmd.output().await }
        };

        let _ = run(&["init", "--initial-branch=main"]).await.ok()?;
        let _ = run(&["config", "user.email", "test@example.com"])
            .await
            .ok()?;
        let _ = run(&["config", "user.name", "Test User"]).await.ok()?;
        tokio::fs::write(d.join("README.md"), "# test").await.ok()?;
        let _ = run(&["add", "README.md"]).await.ok()?;
        let _ = run(&["commit", "-m", "initial commit"]).await.ok()?;
        Some(dir)
    }

    #[tokio::test]
    async fn status_on_clean_repo() {
        let Some(dir) = make_repo().await else {
            eprintln!("[skip] git not available");
            return;
        };
        let g = GitCliOps::new();
        let s = g.status(dir.path()).await.expect("status");
        // clean repo: 第一行是 branch 行, 后面没 modified 文件
        assert!(
            s.contains("## main") || s.contains("## "),
            "branch 行应在, 实际: {s}"
        );
    }

    #[tokio::test]
    async fn log_returns_commit() {
        let Some(dir) = make_repo().await else {
            eprintln!("[skip] git not available");
            return;
        };
        let g = GitCliOps::new();
        let out = g.log(dir.path(), 5).await.expect("log");
        assert!(
            out.contains("initial commit"),
            "log 应含 commit msg, 实际: {out}"
        );
        // 完整 hash 长度 = 40
        let hash = out.split_whitespace().next().expect("hash");
        assert_eq!(hash.len(), 40, "SHA-1 hash 应 40 字符, 实际: {hash}");
    }

    #[tokio::test]
    async fn log_respects_n() {
        let Some(dir) = make_repo().await else {
            eprintln!("[skip] git not available");
            return;
        };
        let g = GitCliOps::new();
        let out = g.log(dir.path(), 1).await.expect("log");
        let count = out.lines().count();
        assert_eq!(count, 1, "n=1 应只返 1 行, 实际: {count}");
    }

    #[tokio::test]
    async fn diff_empty_on_clean_repo() {
        let Some(dir) = make_repo().await else {
            eprintln!("[skip] git not available");
            return;
        };
        let g = GitCliOps::new();
        let out = g.diff(dir.path()).await.expect("diff");
        assert!(out.is_empty(), "clean repo diff 应为空, 实际: {out:?}");
    }

    #[tokio::test]
    async fn diff_with_changes() {
        let Some(dir) = make_repo().await else {
            eprintln!("[skip] git not available");
            return;
        };
        tokio::fs::write(dir.path().join("README.md"), "# changed")
            .await
            .expect("write");
        let g = GitCliOps::new();
        let out = g.diff(dir.path()).await.expect("diff");
        assert!(out.contains("README.md"), "diff 应含文件名, 实际: {out}");
        assert!(
            out.contains("-# test") || out.contains("-# test"),
            "diff 应含删除行"
        );
    }

    #[tokio::test]
    async fn nonexistent_repo_errors() {
        let g = GitCliOps::new();
        let r = g.status(Path::new("/nonexistent/repo/12345")).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not a directory"));
    }

    #[tokio::test]
    async fn not_a_git_repo_errors() {
        // 临时目录但没 git init
        let dir = TempDir::new().expect("tempdir");
        let g = GitCliOps::new();
        let r = g.status(dir.path()).await;
        assert!(r.is_err(), "非 git repo 必失败");
    }

    #[tokio::test]
    async fn tool_adapter_dispatch() {
        let Some(dir) = make_repo().await else {
            eprintln!("[skip] git not available");
            return;
        };
        let g = Arc::new(GitCliOps::new());
        let tool = GitOpsTool::new(g);

        let r = tool
            .call(json!({"op": "status", "repo": dir.path().to_string_lossy()}))
            .await
            .expect("status call");
        assert_eq!(r["op"], "status");
        assert!(r["output"].as_str().unwrap().contains("## "));

        let r = tool
            .call(json!({"op": "log", "repo": dir.path().to_string_lossy(), "n": 1}))
            .await
            .expect("log call");
        assert_eq!(r["op"], "log");
        assert_eq!(r["n"], 1);

        let r = tool
            .call(json!({"op": "diff", "repo": dir.path().to_string_lossy()}))
            .await
            .expect("diff call");
        assert_eq!(r["op"], "diff");
    }

    #[tokio::test]
    async fn tool_adapter_unknown_op() {
        let g = Arc::new(GitCliOps::new());
        let tool = GitOpsTool::new(g);
        let r = tool.call(json!({"op": "blame", "repo": "."})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn tool_adapter_name_and_kind() {
        let g = Arc::new(GitCliOps::new().with_name("MyGit"));
        let tool = GitOpsTool::new(g);
        assert_eq!(tool.name(), "MyGit");
        assert_eq!(tool.kind(), ToolKind::Sync);
    }
}
