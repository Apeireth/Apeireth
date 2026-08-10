//! `apeireth-tools` — **Apeireth R17 战役 2-5 工具集成** (5 trait 真实现)
//!
//! **目标**: 5 trait 完整真实现 (web_search / file_ops / git_ops / code_exec / tool_result) + 端到端真测.
//!
//! **5 大模块** (字段级引用 VCP `FileOperator.js:1-1673` 真代码):
//! 1. `result` — `ToolResult` enum (统一返回类型, 借战役 2-2 `ExecutionResult` 模式)
//! 2. `web_search` — `WebSearch` trait + `HttpWebSearch` (战役 1-2 HTTP 5 字段) + `WebSearchTool` 适配器
//! 3. `file_ops` — `FileOps` trait + `StdFileOps` (6 ops: read/write/list/mkdir/delete/move) + `FileOpsTool` 适配器
//! 4. `git_ops` — `GitOps` trait + `GitCliOps` (status/log/diff) + `GitOpsTool` 适配器
//! 5. `code_exec` — `CodeExec` trait + `ShellCodeExec` (timeout 控制) + `CodeExecTool` 适配器
//! 6. `register` — `register_all(registry)` 一行注册 4 个工具到战役 2-1 `ToolRegistry`
//!
//! **字段级引用 VCP** (per `research/source/vcptoolbox/Plugin/FileOperator/`):
//! - `FileOperator.js:24-26` `MAX_FILE_SIZE=20MB / MAX_DIRECTORY_ITEMS=1000 / MAX_SEARCH_RESULTS=100`
//!   → `file_ops::MAX_FILE_SIZE / MAX_DIRECTORY_ITEMS / MAX_SEARCH_RESULTS` (1:1)
//! - `plugin-manifest.json:55 ReadFile` → `FileOps::read`
//! - `plugin-manifest.json:63 WriteFile` → `FileOps::write`
//! - `plugin-manifest.json:79 ListDirectory` → `FileOps::list`
//! - `plugin-manifest.json:99 DeleteFile` → `FileOps::delete`
//! - `plugin-manifest.json:91 MoveFile` → `FileOps::move_path`
//! - `plugin-manifest.json:103 CreateDirectory` → `FileOps::mkdir`
//! - VCP `WebReadFile` (plugin-manifest.json:59) HTTP GET 模式 → `WebSearch` trait
//! - VCP `chatCompletionHandler.js:22-28` 5 字段 keep-alive → 战役 1-2 `HttpClient` (我们 import)
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ 5 trait 全部 async_trait + Send + Sync
//! - ✅ 4 impl 全部真实现 (HttpClient / tokio::fs / tokio::process), 不只 mock
//! - ✅ tokio timeout 真用 (`tokio::time::timeout`)
//! - ✅ VCP 字段级常量 1:1 (3 个 const 跟 VCP `config.env` 真值)
//! - ✅ 编译期 hardcode (`FILE_OPS_OPERATION_COUNT = 6` 等)
//! - ✅ register_all 4 个工具 + ToolRegistry 真查 + Tool trait 真路由
//! - ✅ unit tests ≥ 50 (实际约 50+, 远超 DoD ≥ 15)
//!
//! **不修改承诺** (R17 finalize 8 项不修改承诺):
//! - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
//! - ❌ 不改战役 1 / 战役 2-1 / 战役 2-2 / 战役 2-3 / 战役 2-4 全部代码 (用 import, 不改源码)
//! - ❌ 不引入 unsafe (workspace `#![deny(unsafe_code)]` 继承)
//! - ❌ 不假装 "已实现但没真跑" (5 trait 全部真实现)
//! - ❌ 不抄 VCP 业务代码 (借鉴字段名 + 行为, 不抄 fs / process 实现)
//! - ❌ 不删 `apeireth-tools` crate 名字 (R17 决策保留)
//!
//! **架构位置**:
//! ```text
//!   apeireth-api / apeireth-pipeline / 未来消费者
//!          ↓
//!      apeireth-tools (本 crate)
//!      ├── result.rs       : ToolResult enum (统一返回类型)
//!      ├── web_search.rs   : WebSearch trait + HttpWebSearch + WebSearchTool 适配器
//!      ├── file_ops.rs     : FileOps trait + StdFileOps + FileOpsTool 适配器 (6 ops)
//!      ├── git_ops.rs      : GitOps trait + GitCliOps + GitOpsTool 适配器 (3 ops)
//!      ├── code_exec.rs    : CodeExec trait + ShellCodeExec + CodeExecTool 适配器
//!      ├── register.rs     : register_all(registry) 一行注册 4 个工具
//!      └── lib.rs          : 入口 + 编译期 hardcode 守门
//! ```
//!
//! **跨 crate 集成**:
//! - `apeireth-http-client` (战役 1-2) — `HttpWebSearch` 内部用 (5 字段 keep-alive)
//! - `apeireth-tool-registry` (战役 2-1) — 4 个 *Tool 适配器 + `register_all`
//! - `apeireth-tool-runtime` (战役 2-2) — `ToolResult` 设计模式借鉴 (本地 enum 跟 `ExecutionResult` 平行)
//! - `apeireth-core` (R11) — 保留 (兼容)

#![warn(missing_docs)]
#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod code_exec;
pub mod long_task;
pub mod classifier;  // R30 U5: tool classifier (8 类 keyword routing)
pub mod web_fetch;  // R30 U2: lightweight HTTP fetch  // R30 U11: long-running task manager
pub mod apply_patch;  // R30 U1: Codex-style apply_patch
pub mod conventions_scanner;  // R33-1: Aider-style project conventions scanner
pub mod grep_ops;
pub mod file_ops;
pub mod git_ops;
pub mod register;
pub mod result;
pub mod web_search;

pub use code_exec::{CodeExec, CodeExecTool, ShellCodeExec};
pub use conventions_scanner::ProjectConventions;  // R33-1: Aider-style conventions scanner
pub use grep_ops::{GrepHit, GrepOps, GrepTool, RipgrepGrepOps};
pub use file_ops::{
    FileOps, FileOpsTool, StdFileOps, FILE_OPS_OPERATION_COUNT, MAX_DIRECTORY_ITEMS, MAX_FILE_SIZE,
    MAX_SEARCH_RESULTS,
};
pub use git_ops::{GitCliOps, GitOps, GitOpsTool};
pub use register::{register_all, registered_tool_names, REGISTERED_TOOL_COUNT, TOOL_NAMES};
pub use result::ToolResult;
pub use web_search::{HttpWebSearch, WebSearch, WebSearchTool};

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 战役 2-5 实际借鉴 VCP 字段数
/// (FileOperator.js 5 个: MAX_FILE_SIZE / MAX_DIRECTORY_ITEMS / MAX_SEARCH_RESULTS / 6 命令名 + WebReadFile 模式)
pub const BORROWED_VCP_FIELDS: usize = 5;

/// 5 trait 数 (WebSearch / FileOps / GitOps / CodeExec / ToolResult enum) — 编译期 hardcode
pub const TRAIT_COUNT: usize = 7;

/// 4 impl 数 (HttpWebSearch / StdFileOps / GitCliOps / ShellCodeExec) — 编译期 hardcode
pub const IMPL_COUNT: usize = 6;

/// 6 file_ops 操作数 (read / write / list / mkdir / delete / move) — 编译期 hardcode
pub const FILE_OPS_OP_COUNT: usize = FILE_OPS_OPERATION_COUNT;

/// 3 git_ops 操作数 (status / log / diff) — 编译期 hardcode
pub const GIT_OPS_OP_COUNT: usize = 3;

/// 1 code_exec 操作数 (exec) — 编译期 hardcode
pub const CODE_EXEC_OP_COUNT: usize = 1;

/// register_all 一行注册的工具数
pub const REGISTERED_TOOL_COUNT_CONST: usize = REGISTERED_TOOL_COUNT;

/// 战役 2-5 VCP FileOperator config.env 真值
/// (FileOperator.js:24-26 默认 20MB / 1000 / 100)
/// VCP MAX_FILE_SIZE 默认 20MB
pub const VCP_MAX_FILE_SIZE_BYTES: u64 = MAX_FILE_SIZE;
/// VCP MAX_DIRECTORY_ITEMS 默认 1000
pub const VCP_MAX_DIRECTORY_ITEMS: usize = MAX_DIRECTORY_ITEMS;
/// VCP MAX_SEARCH_RESULTS 默认 100
pub const VCP_MAX_SEARCH_RESULTS: usize = MAX_SEARCH_RESULTS;

// ============================================================
// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
// ============================================================

const _: () = {
    // 5 trait + 4 impl
    assert!(
        TRAIT_COUNT == 7,
        "TRAIT_COUNT = 7 (WebSearch / FileOps / GitOps / CodeExec / GrepOps / WebFetch + ToolResult enum)"
    );
    assert!(
        IMPL_COUNT == 6,
        "IMPL_COUNT = 6 (HttpWebSearch / StdFileOps / GitCliOps / ShellCodeExec / RipgrepGrepOps / ReqwestWebFetch)"
    );

    // 6 + 3 + 1 操作数对齐
    assert!(
        FILE_OPS_OP_COUNT == 7,
        "FILE_OPS_OP_COUNT = 7 (read/write/list/mkdir/delete/move/edit)"
    );
    assert!(
        GIT_OPS_OP_COUNT == 3,
        "GIT_OPS_OP_COUNT = 3 (status/log/diff)"
    );
    assert!(CODE_EXEC_OP_COUNT == 1, "CODE_EXEC_OP_COUNT = 1 (exec)");

    // register_all
    assert!(
        REGISTERED_TOOL_COUNT_CONST == 8,
        "REGISTERED_TOOL_COUNT = 8 (8 工具 incl Grep + ApplyPatch + LongTask + WebFetch)"
    );

    // VCP 字段 5
    assert!(
        BORROWED_VCP_FIELDS == 5,
        "BORROWED_VCP_FIELDS = 5 (VCP FileOperator 字段级)"
    );

    // VCP config.env 真值 1:1
    assert!(
        MAX_FILE_SIZE == 20 * 1024 * 1024,
        "VCP FileOperator.js:24 MAX_FILE_SIZE = 20MB"
    );
    assert!(
        MAX_DIRECTORY_ITEMS == 1000,
        "VCP FileOperator.js:25 MAX_DIRECTORY_ITEMS = 1000"
    );
    assert!(
        MAX_SEARCH_RESULTS == 100,
        "VCP FileOperator.js:26 MAX_SEARCH_RESULTS = 100"
    );
};

// ============================================================
// lib 入口测试 (编译期 hardcode 二次断言 + 公开 API 守 + 端到端)
// ============================================================

#[cfg(test)]
mod lib_tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn lib_constants_match_vcp() {
        // 编译期 hardcode 已 assert, 这里再 runtime 测一次
        assert_eq!(TRAIT_COUNT, 7);
        assert_eq!(IMPL_COUNT, 6);
        assert_eq!(FILE_OPS_OP_COUNT, 7);
        assert_eq!(GIT_OPS_OP_COUNT, 3);
        assert_eq!(CODE_EXEC_OP_COUNT, 1);
        assert_eq!(REGISTERED_TOOL_COUNT_CONST, 8);
        assert_eq!(BORROWED_VCP_FIELDS, 5);
        assert_eq!(MAX_FILE_SIZE, 20 * 1024 * 1024);
        assert_eq!(MAX_DIRECTORY_ITEMS, 1000);
        assert_eq!(MAX_SEARCH_RESULTS, 100);
    }

    #[test]
    fn lib_vcp_field_count_breakdown() {
        // 5 = 3 (config.env: MAX_FILE_SIZE / MAX_DIRECTORY_ITEMS / MAX_SEARCH_RESULTS) + 2 (6 命令映射 + WebReadFile 模式)
        assert_eq!(BORROWED_VCP_FIELDS, 5);
    }

    #[test]
    fn lib_public_api_compiles() {
        // 验证 5 trait + 4 impl 公开 API 全部可见
        let _r = ToolResult::ok_str("test");
        let _r = ToolResult::err(404, "missing");

        // 4 impl (类型可达)
        let _: HttpWebSearch = HttpWebSearch::new(
            std::sync::Arc::new(
                apeireth_http_client::HttpClient::with_vcp_defaults().expect("client"),
            ),
            "https://example.com",
            "W",
        );
        let _: StdFileOps = StdFileOps::new();
        let _: GitCliOps = GitCliOps::new();
        let _: ShellCodeExec = ShellCodeExec::new();

        // 4 *Tool 适配器
        let _ = WebSearchTool::new(std::sync::Arc::new(HttpWebSearch::new(
            std::sync::Arc::new(
                apeireth_http_client::HttpClient::with_vcp_defaults().expect("client"),
            ),
            "u",
            "w",
        )));
        let _ = FileOpsTool::new(std::sync::Arc::new(StdFileOps::new()));
        let _ = GitOpsTool::new(std::sync::Arc::new(GitCliOps::new()));
        let _ = CodeExecTool::new(std::sync::Arc::new(ShellCodeExec::new()));
    }

    #[test]
    fn lib_tool_names_match_vcp_19_commands_subset() {
        // 4 注册工具名应 VCP 风格 (FileOperator 真名 / Git / Shell / WebSearch)
        let names = registered_tool_names();
        assert!(names.contains(&"FileOperator"), "VCP FileOperator 真名");
        assert!(names.contains(&"Git"));
        assert!(names.contains(&"ShellExec"));
        assert!(names.contains(&"WebSearch"));
    }

    #[test]
    fn lib_register_all_then_list_4() {
        let registry = apeireth_tool_registry::ToolRegistry::new();
        assert!(registry.is_empty());
        register_all(&registry).expect("register_all");
        assert_eq!(registry.len(), 8);
        let listed = registry.list();
        assert_eq!(listed.len(), 8);
    }

    #[test]
    fn u14_all_8_tools_expose_kind_and_5_axes() {
        // R30 U14: 6 类 enum + 5 轴正交 — 所有 8 工具都要实现 kind() + axes()
        use apeireth_tool_registry::Tool;
        let registry = apeireth_tool_registry::ToolRegistry::new();
        register_all(&registry).expect("register_all");
        for name in registry.list() {
            let t = registry.get(&name).expect("get tool");
            // 6 类之一 (Sync / Async / Static / Service / MessagePreprocessor / Hybridservice)
            let k = t.kind();
            assert!(
                matches!(k,
                    apeireth_tool_registry::ToolKind::Sync
                    | apeireth_tool_registry::ToolKind::Async
                    | apeireth_tool_registry::ToolKind::Static
                    | apeireth_tool_registry::ToolKind::Service
                    | apeireth_tool_registry::ToolKind::MessagePreprocessor
                    | apeireth_tool_registry::ToolKind::Hybridservice
                ),
                "{} kind 不在 6 类里: {:?}", name, k
            );
            // 5 轴 (Trigger / Awaiting / Resident / Transport / Output)
            let a = t.axes();
            let _ = (a.trigger, a.awaiting, a.resident, a.transport, a.output);
        }
    }

    #[tokio::test]
    #[cfg_attr(windows, ignore = "Windows ShellExec spawn echo PATH 限制, R21+ 续")]
    async fn lib_end_to_end_4_traits_via_registry() {
        // 端到端: register_all + 4 trait 各调一次真跑
        use apeireth_tool_registry::Tool;
        let registry = apeireth_tool_registry::ToolRegistry::new();
        register_all(&registry).expect("register_all");

        // 1. FileOperator 真写 + 真读
        let dir = TempDir::new().expect("tempdir");
        let p = dir.path().join("e2e.txt");
        let tool = registry.get("FileOperator").expect("FileOperator");
        tool.call(json!({
            "op": "write",
            "path": p.to_string_lossy(),
            "content": "e2e"
        }))
        .await
        .expect("write");
        let r = tool
            .call(json!({"op": "read", "path": p.to_string_lossy()}))
            .await
            .expect("read");
        assert_eq!(r["content"], "e2e");

        // 2. ShellExec 真 echo
        let tool = registry.get("ShellExec").expect("ShellExec");
        let r = tool
            .call(json!({"cmd": "echo e2e-ok"}))
            .await
            .expect("echo");
        assert_eq!(r["exit_code"], 0);
        assert!(r["output"].as_str().unwrap().contains("e2e-ok"));

        // 3. WebSearch 空 query 必 400 (不真发 HTTP)
        let tool = registry.get("WebSearch").expect("WebSearch");
        let r = tool.call(json!({"query": ""})).await.expect("call");
        assert_eq!(r["error_code"], 400);

        // 4. Git.status (git 不可用时 skip)
        if let Some(repo) = make_git_repo().await {
            let tool = registry.get("Git").expect("Git");
            let r = tool
                .call(json!({"op": "status", "repo": repo.path().to_string_lossy()}))
                .await
                .expect("status");
            assert_eq!(r["op"], "status");
        } else {
            eprintln!("[skip] git 不可用");
        }
    }

    /// 真建 git 仓库
    async fn make_git_repo() -> Option<TempDir> {
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
