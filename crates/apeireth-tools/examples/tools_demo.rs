//! **战役 2-5 / `apeireth-tools` 端到端 demo**
//!
//! **目标**: 演示 5 trait 全部真跑 (web_search / file_ops / git_ops / code_exec / tool_result)
//!
//! **跑法**: `cargo run -p apeireth-tools --example tools_demo`
//!
//! **5 步全跑通**:
//! 1. ToolResult enum 演示: Ok / Err 两种构造
//! 2. WebSearch 真接: 起本地 HTTP echo server + 真发 GET + 验 query 透传
//! 3. FileOps 6 操作真跑: write → read → mkdir → list → move → delete (全在 tempdir)
//! 4. GitOps 3 操作真跑: 真建 git 仓库 + 真 commit + status / log / diff
//! 5. CodeExec 真跑: echo / exit 0 / 模拟 timeout / stderr 捕获
//!
//! **6 步**: register_all 一次性塞 4 个工具到 ToolRegistry, 然后通过 Tool::call 路由调

use apeireth_http_client::HttpClient;
#[allow(unused_imports)]
use apeireth_tool_registry::Tool; // trait methods (kind/axes/call) via Arc<dyn Tool>
use apeireth_tools::{
    register_all, CodeExec, FileOps, GitCliOps, GitOps, HttpWebSearch, ShellCodeExec, StdFileOps,
    ToolResult, WebSearch,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn separator(title: &str) {
    println!("\n{}", "=".repeat(70));
    println!("  {title}");
    println!("{}\n", "=".repeat(70));
}

#[tokio::main]
async fn main() {
    separator("战役 2-5 / `apeireth-tools` 端到端 demo");
    println!("VCP 借鉴: FileOperator.js 68KB (5 字段级 + 6 命令映射 + WebReadFile 模式)");
    println!("跨 crate 集成: 战役 1-2 HttpClient + 战役 2-1 ToolRegistry + 战役 2-2 ExecutionResult 模式");

    // ============================================================
    // Step 1: ToolResult enum 演示
    // ============================================================
    separator("Step 1: ToolResult enum 演示 (统一返回类型, 借战役 2-2 ExecutionResult 模式)");

    let ok = ToolResult::ok(json!({"echo": "hello", "count": 42}));
    println!("[OK]   ToolResult::Ok({:?}) = {ok:?}", "Value");
    assert!(ok.is_ok());
    assert_eq!(ok.value().unwrap()["echo"], "hello");

    let err = ToolResult::err(404, "Resource not found");
    println!("[OK]   ToolResult::Err {{ code: 404, message: \"Resource not found\" }}");
    assert!(err.is_err());
    assert_eq!(err.err_code(), Some(404));

    // serde 兼容
    let serialized = serde_json::to_string(&err).unwrap();
    let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();
    println!("[OK]   serde round-trip: {serialized} -> {deserialized:?}");
    assert_eq!(deserialized, err);

    // ============================================================
    // Step 2: WebSearch 真接 — 起本地 HTTP echo server
    // ============================================================
    separator("Step 2: WebSearch 真接 (起本地 TCP HTTP server, 真发 GET)");

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let local_addr = listener.local_addr().expect("addr");
    let server_task = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else { break };
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                if socket.read(&mut buf).await.is_err() {
                    return;
                }
                let req = String::from_utf8_lossy(&buf);
                let path = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/");
                // 模拟搜索 API 返 JSON
                let body = format!(
                    r#"{{"results":[{{"title":"Result for {path}","url":"https://example.com/r1"}},{{"title":"Another hit","url":"https://example.com/r2"}}]}}"#
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    let client = Arc::new(HttpClient::with_vcp_defaults().expect("http client"));
    let url = format!("http://{local_addr}/search?q={{query}}&n={{max}}");
    let search = HttpWebSearch::new(client, url, "LocalDemoSearch");
    println!(
        "[OK]   HttpWebSearch 配 search URL: http://{local_addr}/search?q={{query}}&n={{max}}"
    );

    let r = search.search("rust async", 5).await.expect("search outer");
    assert!(r.is_ok(), "本地 server 应返 200, 实际: {r:?}");
    let v = r.value().expect("value");
    println!("[OK]   search(\"rust async\", 5) 返:");
    println!("       query      = {}", v["query"]);
    println!("       max_results = {}", v["max_results"]);
    println!("       elapsed_ms = {}", v["elapsed_ms"]);
    println!("       status     = {}", v["status"]);
    println!(
        "       results    = [{} result(s), 1st: {:?}]",
        v["results"]["results"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
        v["results"]["results"][0]["title"]
    );

    // 字段级校验: 空 query 必 400
    let r = search.search("", 5).await.expect("empty query outer");
    assert!(r.is_err());
    assert_eq!(r.err_code(), Some(400));
    println!("[OK]   字段级校验: 空 query 必返 ToolResult::Err {{ code: 400, .. }}");

    server_task.abort();
    let _ = server_task.await;

    // ============================================================
    // Step 3: FileOps 6 操作真跑 (tempdir)
    // ============================================================
    separator("Step 3: FileOps 6 操作真跑 (VCP plugin-manifest.json 字段级映射)");

    let dir = TempDir::new().expect("tempdir");
    let f = StdFileOps::new();
    println!("[init] tempdir = {}", dir.path().display());

    // 1. write
    let p1 = dir.path().join("a.txt");
    f.write(&p1, "hello from file_ops demo\n")
        .await
        .expect("write");
    println!(
        "[OK]   write {:?} = {} bytes",
        "a.txt",
        "hello from file_ops demo\n".len()
    );

    // 2. read
    let content = f.read(&p1).await.expect("read");
    println!("[OK]   read a.txt = {content:?}");
    assert_eq!(content, "hello from file_ops demo\n");

    // 3. mkdir (建 nested)
    let nested = dir.path().join("sub/nested");
    f.mkdir(&nested).await.expect("mkdir");
    println!("[OK]   mkdir sub/nested");

    // 4. list
    let entries = f.list(dir.path()).await.expect("list");
    println!("[OK]   list dir = {} entries: {:?}", entries.len(), entries);
    assert!(entries.len() >= 2);

    // 5. move (a.txt -> sub/a.txt)
    let dst = nested.join("a.txt");
    f.move_path(&p1, &dst).await.expect("move");
    println!("[OK]   move a.txt -> sub/nested/a.txt");
    assert!(!p1.exists());
    assert!(dst.exists());

    // 6. delete
    f.delete(&dst).await.expect("delete");
    println!("[OK]   delete sub/nested/a.txt");
    assert!(!dst.exists());

    // ============================================================
    // Step 4: GitOps 3 操作真跑 (真建 git 仓库)
    // ============================================================
    separator("Step 4: GitOps 3 操作真跑 (真建 git 仓库 + commit)");

    if let Some(git) = check_git() {
        let repo = TempDir::new().expect("git tempdir");
        let d = repo.path();
        println!("[init] git repo = {}", d.display());

        let run = |args: &[&str]| {
            let mut cmd = tokio::process::Command::new(&git);
            cmd.args(args).current_dir(d);
            async move { cmd.output().await }
        };

        let _ = run(&["init", "--initial-branch=main"])
            .await
            .expect("git init");
        let _ = run(&["config", "user.email", "demo@example.com"])
            .await
            .expect("config email");
        let _ = run(&["config", "user.name", "Demo"])
            .await
            .expect("config name");
        tokio::fs::write(d.join("README.md"), "# demo")
            .await
            .expect("write");
        let _ = run(&["add", "README.md"]).await.expect("add");
        let _ = run(&["commit", "-m", "initial"]).await.expect("commit");
        println!("[OK]   git init + commit 1 file (README.md)");

        let g = GitCliOps::new();
        let s = g.status(d).await.expect("status");
        println!("[OK]   git status =");
        for line in s.lines() {
            println!("         {line}");
        }
        assert!(s.contains("## main"));

        let l = g.log(d, 5).await.expect("log");
        println!("[OK]   git log -5 =");
        for line in l.lines() {
            println!("         {line}");
        }
        assert!(l.contains("initial"));

        // diff: clean 应空
        let diff = g.diff(d).await.expect("diff");
        println!("[OK]   git diff (clean) = {diff:?} (期望空)");
        assert!(diff.is_empty());
    } else {
        println!("[--]   git 不可用, skip GitOps 演示");
    }

    // ============================================================
    // Step 5: CodeExec 真跑 (echo / exit 0 / timeout / stderr)
    // ============================================================
    separator("Step 5: CodeExec 真跑 (tokio::process + tokio::time::timeout)");

    let e = ShellCodeExec::new();

    // 1. echo
    let (code, out) = e.exec("echo code-exec-ok", 0).await.expect("echo");
    println!("[OK]   echo: exit_code={code}, output={out:?}");
    assert_eq!(code, 0);
    assert!(out.contains("code-exec-ok"));

    // 2. 非 0 exit code
    let cmd = if cfg!(windows) {
        "cmd /c exit 7"
    } else {
        "exit 7"
    };
    let (code, _) = e.exec(cmd, 0).await.expect("exit 7");
    println!("[OK]   exit 7: exit_code={code} (应保留 7)");
    assert_eq!(code, 7);

    // 3. timeout
    let long_cmd = if cfg!(windows) {
        "ping -n 5 127.0.0.1"
    } else {
        "sleep 5"
    };
    let start = std::time::Instant::now();
    let r = e.exec(long_cmd, 50).await;
    let elapsed = start.elapsed();
    assert!(r.is_err());
    println!("[OK]   timeout 50ms 触发: {elapsed:?} 内返错 (应远小于 5s)");
    assert!(elapsed < Duration::from_secs(1));

    // 4. stderr 捕获
    let cmd = if cfg!(windows) {
        "cmd /c echo to-stderr 1>&2"
    } else {
        "echo to-stderr 1>&2"
    };
    let (_code, out) = e.exec(cmd, 0).await.expect("stderr");
    println!("[OK]   stderr 捕获: output={out:?}");
    assert!(out.contains("to-stderr"));

    // ============================================================
    // Step 6: register_all 一行注册 4 工具 + Tool::call 路由
    // ============================================================
    separator("Step 6: register_all 一行注册 4 工具 + Tool::call 路由");

    let registry = apeireth_tool_registry::ToolRegistry::new();
    register_all(&registry).expect("register_all");
    let listed = registry.list();
    println!("[OK]   registry 装好, 工具列表 = {listed:?}");
    assert_eq!(listed.len(), 4);

    // 通过 Tool::call 路由 4 工具
    for name in listed.iter() {
        let tool = registry.get(name).expect("get");
        let (kind_str, axes) = (format!("{:?}", tool.kind()), tool.axes());
        println!(
            "[OK]   {name}: kind={kind_str}, transport={:?}, output={:?}",
            axes.transport, axes.output
        );
    }

    // FileOperator 通过 Tool trait 真写
    let dir2 = TempDir::new().expect("tempdir2");
    let p = dir2.path().join("via_registry.txt");
    let tool = registry.get("FileOperator").expect("FileOperator");
    let r = tool
        .call(json!({
            "op": "write",
            "path": p.to_string_lossy(),
            "content": "dispatched via Tool::call"
        }))
        .await
        .expect("write via registry");
    println!("[OK]   FileOperator.write via Tool::call = {r:?}");
    assert_eq!(r["op"], "write");

    // ShellExec 通过 Tool trait 真跑
    let tool = registry.get("ShellExec").expect("ShellExec");
    let r = tool
        .call(json!({"cmd": "echo via-registry-dispatch"}))
        .await
        .expect("echo via registry");
    println!(
        "[OK]   ShellExec.exec via Tool::call = exit={}, output={:?}",
        r["exit_code"], r["output"]
    );
    assert_eq!(r["exit_code"], 0);
    assert!(r["output"]
        .as_str()
        .unwrap()
        .contains("via-registry-dispatch"));

    println!(
        "\n{}\n  战役 2-5 tools_demo 完结 ✓\n{}",
        "=".repeat(70),
        "=".repeat(70)
    );
    println!("5 trait 全部真接:");
    println!("  ✓ ToolResult   (统一返回类型)");
    println!("  ✓ WebSearch    (HttpClient 5 字段 + 本地 HTTP 真发 GET)");
    println!("  ✓ FileOps      (6 ops: write/read/mkdir/list/move/delete)");
    println!("  ✓ GitOps       (3 ops: status/log/diff, 真建 git 仓库)");
    println!("  ✓ CodeExec     (echo / exit code / timeout / stderr)");
    println!("  ✓ register_all (4 工具 + Tool::call 路由 端到端)");
}

/// 检查 git 是否可用
fn check_git() -> Option<std::path::PathBuf> {
    let out = std::process::Command::new("git")
        .arg("--version")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(std::path::PathBuf::from("git"))
}
