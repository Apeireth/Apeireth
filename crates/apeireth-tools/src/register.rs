//! `apeireth-tools::register` — 统一注册 4 impl 到 tool-registry
//!
//! **战役 2-5**: 一行调 `register_all(registry)` 把 web_search / file_ops / git_ops / code_exec 全注册.
//!
//! **不假装**:
//! - ✅ 真用战役 2-1 `ToolRegistry::register`
//! - ✅ 端到端真测: register 后能从 registry 真查到 4 个工具 + Tool trait 路由可调
//!
//! **TP12 (A2, P0) schema sidecar**:
//! - `apeireth-tool-registry::Tool` trait 在 N15 已锁定, 不挂 schema 字段
//! - 本模块不修改注册路径 (`ToolRegistry::register` 调用方式 0 改)
//! - `default_schema_map()` 返回空 SchemaMap (向后兼容默认 = 全部不校验)
//! - 工具作者按需扩展: `let mut m = default_schema_map(); m.insert("WebSearch", web_search_schema);`
//!   然后把 `m` 传给 `ToolExecutor::with_schema_map(...)`

use std::sync::Arc;

use apeireth_http_client::HttpClient;
use apeireth_tool_registry::{Tool, ToolRegistry};

use crate::apply_patch::ApplyPatchTool;
use crate::code_exec::{CodeExec, CodeExecTool, ShellCodeExec};
use crate::file_ops::{FileOps, FileOpsTool, StdFileOps};
use crate::git_ops::{GitCliOps, GitOps, GitOpsTool};
use crate::grep_ops::{GrepOps, GrepTool, RipgrepGrepOps};
use crate::long_task::LongTaskTool;
use crate::web_fetch::{ReqwestWebFetch, WebFetch, WebFetchTool};
use crate::web_search::{HttpWebSearch, WebSearch, WebSearchTool};

/// 战役 2-5 实际注册 4 个 tool
pub const REGISTERED_TOOL_COUNT: usize = 9;

/// 4 impl 名 (VCP 字段级: WebSearch / FileOperator / Git / ShellExec)
pub const TOOL_NAMES: [&str; REGISTERED_TOOL_COUNT] = [
    "WebSearch",    // HttpWebSearch
    "FileOperator", // StdFileOps (VCP 真名)
    "Git",          // GitCliOps
    "ShellExec",    // ShellCodeExec
    "Grep",         // RipgrepGrepOps (R30 P1)
    "ApplyPatch",   // ApplyPatchTool (R30 U1)
    "LongTask",     // LongTaskTool (R30 U11)
    "WebFetch",     // WebFetchTool (R30 U2)
    "Crawl",        // CrawlTool (R230: 轻量爬虫)
];

/// **统一注册: 4 个工具一次性塞进 registry**
///
/// **实战**:
/// ```ignore
/// use apeireth_tools::register_all;
/// use apeireth_tool_registry::{Tool, ToolRegistry};
///
/// let registry = ToolRegistry::new();
/// register_all(&registry).expect("register_all");
/// // 现在 registry 里有 4 个工具, 都可通过 `Tool::call(args)` 调
/// ```
///
/// **依赖**:
/// - `HttpClient` — 用战役 1-2 VCP 5 字段 keep-alive, 默认 URL 是 minimaxi 域 search endpoint
/// - `GitCliOps` / `ShellCodeExec` / `StdFileOps` — 用默认配置
pub fn register_all(registry: &ToolRegistry) -> Result<(), String> {
    // 1. web_search (用 HttpClient 5 字段 keep-alive)
    let http = HttpClient::with_chat_defaults().map_err(|e| format!("HttpClient: {e}"))?;
    let web_search: Arc<dyn WebSearch> =
        Arc::new(HttpWebSearch::with_minimaxi_default(Arc::new(http)));
    registry.register(
        web_search.name().to_string(),
        Arc::new(WebSearchTool::new(web_search)),
    );

    // 2. file_ops
    let file_ops: Arc<dyn FileOps> = Arc::new(StdFileOps::new());
    registry.register(
        file_ops.name().to_string(),
        Arc::new(FileOpsTool::new(file_ops)),
    );

    // 3. git_ops
    let git_ops: Arc<dyn GitOps> = Arc::new(GitCliOps::new());
    registry.register(
        git_ops.name().to_string(),
        Arc::new(GitOpsTool::new(git_ops)),
    );

    // 4. code_exec
    let code_exec: Arc<dyn CodeExec> = Arc::new(ShellCodeExec::new());
    registry.register(
        code_exec.name().to_string(),
        Arc::new(CodeExecTool::new(code_exec)),
    );

    // 5. grep (R30 P1)
    let grep: Arc<dyn GrepOps> = Arc::new(RipgrepGrepOps::new());
    registry.register(grep.name().to_string(), Arc::new(GrepTool::new(grep)));

    // 6. apply_patch (R30 U1)
    registry.register(
        ApplyPatchTool::new().name().to_string(),
        Arc::new(ApplyPatchTool::new()),
    );

    // 7. long_task (R30 U11)
    registry.register(
        LongTaskTool::new().name().to_string(),
        Arc::new(LongTaskTool::new()),
    );

    // 8. web_fetch (R30 U2)
    let web_fetch: Arc<dyn WebFetch> = Arc::new(ReqwestWebFetch::new());
    registry.register(
        WebFetchTool::new(web_fetch).name().to_string(),
        Arc::new(WebFetchTool::new(Arc::new(ReqwestWebFetch::new()))),
    );

    // 9. crawl (R230: 轻量爬虫)
    registry.register(
        "Crawl".to_string(),
        Arc::new(crate::web_crawl::CrawlTool::default()),
    );

    Ok(())
}

/// 列出已注册的工具名 (静态, 跟 `TOOL_NAMES` 同源)
pub fn registered_tool_names() -> Vec<&'static str> {
    TOOL_NAMES.to_vec()
}

/// **TP12 — 默认空 SchemaMap** (向后兼容默认 = 全部工具不校验)
///
/// **用法**:
/// ```ignore
/// use apeireth_tools::register::{register_all, default_schema_map};
/// use apeireth_tools::schema::{SchemaMap, SchemaNode};
///
/// let registry = apeireth_tool_registry::ToolRegistry::new();
/// register_all(&registry).expect("register_all");
///
/// let mut schemas = default_schema_map();
/// // schemas.insert("WebSearch", web_search_schema); // 工具作者按需开
/// // 然后传给 ToolExecutor::with_schema_map(registry, schemas)
/// ```
///
/// **0 装 PASS**: 返回的 SchemaMap 默认空, 注入到 ToolExecutor 后 = 全部工具不校验,
/// 行为与 TP12 之前完全一致 (向后兼容).
pub fn default_schema_map() -> crate::schema::SchemaMap {
    crate::schema::SchemaMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;
    use apeireth_tool_registry::ToolKind;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn tool_names_match_vcp_and_field_count() {
        assert_eq!(TOOL_NAMES.len(), REGISTERED_TOOL_COUNT);
        assert_eq!(TOOL_NAMES.len(), 9);
        // VCP 字段级 4 名
        assert!(TOOL_NAMES.contains(&"WebSearch"));
        assert!(TOOL_NAMES.contains(&"FileOperator"));
        assert!(TOOL_NAMES.contains(&"Git"));
        assert!(TOOL_NAMES.contains(&"ShellExec"));
        assert!(TOOL_NAMES.contains(&"Grep"));
        assert!(TOOL_NAMES.contains(&"ApplyPatch"));
        assert!(TOOL_NAMES.contains(&"LongTask"));
        assert!(TOOL_NAMES.contains(&"WebFetch"));
    }

    #[test]
    fn registered_tool_names_static_list() {
        let names = registered_tool_names();
        assert_eq!(names.len(), 9);
        // 不重复
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 9, "TOOL_NAMES 必须唯一");
    }

    #[test]
    fn register_all_adds_4_tools_to_registry() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        register_all(&registry).expect("register_all");
        assert_eq!(registry.len(), 9);
        // 4 名应都在
        for name in TOOL_NAMES.iter() {
            assert!(registry.get(name).is_some(), "{name} 应在 registry 中");
        }
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "Windows ShellExec spawn echo PATH 限制, R21+ 续")]
    async fn register_all_tools_dispatch_via_tool_trait() {
        // 端到端: register_all + Tool::call 4 个工具
        let registry = ToolRegistry::new();
        register_all(&registry).expect("register_all");

        // 1. FileOperator.write (真写)
        let dir = TempDir::new().expect("tempdir");
        let p = dir.path().join("hello.txt");
        let tool = registry.get("FileOperator").expect("FileOperator");
        let r = tool
            .call(json!({
                "op": "write",
                "path": p.to_string_lossy(),
                "content": "via registry dispatch"
            }))
            .await
            .expect("write");
        assert_eq!(r["op"], "write");
        let r = tool
            .call(json!({"op": "read", "path": p.to_string_lossy()}))
            .await
            .expect("read");
        assert_eq!(r["content"], "via registry dispatch");

        // 2. ShellExec.echo (真跑)
        let tool = registry.get("ShellExec").expect("ShellExec");
        let r = tool
            .call(json!({"cmd": "echo via-registry"}))
            .await
            .expect("echo");
        assert_eq!(r["exit_code"], 0);
        assert!(r["output"].as_str().unwrap().contains("via-registry"));

        // 3. Git.status (在临时 git 仓库跑, 可能 skip 如果 git 不可用)
        if let Some(repo_dir) = try_make_git_repo().await {
            let tool = registry.get("Git").expect("Git");
            let r = tool
                .call(json!({"op": "status", "repo": repo_dir.path().to_string_lossy()}))
                .await
                .expect("status");
            assert_eq!(r["op"], "status");
        } else {
            eprintln!("[skip] git 不可用, skip Git 测试");
        }

        // 4. WebSearch.empty_query (必返 400, 不真发 HTTP)
        let tool = registry.get("WebSearch").expect("WebSearch");
        let r = tool.call(json!({"query": ""})).await.expect("call");
        assert_eq!(r["error_code"], 400);
    }

    /// 初始化 git 仓库 (测试用)
    async fn try_make_git_repo() -> Option<TempDir> {
        use tokio::process::Command as TokioCommand;
        let dir = TempDir::new().ok()?;
        let d = dir.path();
        let _ = TokioCommand::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(d)
            .output()
            .await
            .ok()?;
        let _ = TokioCommand::new("git")
            .args(["config", "user.email", "t@e.com"])
            .current_dir(d)
            .output()
            .await
            .ok()?;
        let _ = TokioCommand::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(d)
            .output()
            .await
            .ok()?;
        tokio::fs::write(d.join("r"), "x").await.ok()?;
        let _ = TokioCommand::new("git")
            .args(["add", "r"])
            .current_dir(d)
            .output()
            .await
            .ok()?;
        let _ = TokioCommand::new("git")
            .args(["commit", "-m", "i"])
            .current_dir(d)
            .output()
            .await
            .ok()?;
        Some(dir)
    }
}
