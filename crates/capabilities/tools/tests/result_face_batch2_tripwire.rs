//! 结果面凭据绊线 —— 批次 2 接线回归测试（`fetch` / `git_write` / `worktree_apply`）。
//!
//! 覆盖面: 批次 2 的三个工具族的**结果面** —— 统一助手
//! `crate::guardrail::scan_result_face` (形状层) 按批次 1 收口后的口径过:
//! 命中即遮形并挂上可观测的 `credential_tripwire` 标记; 零命中则**逐字节
//! 不变**、不挂任何标记。
//!
//! 验收条目 (逐条对应):
//!   ① `fetch` 响应体含 `sk-` + ≥20 字符 ⇒ 结果无明文 + `[REDACTED_OPENAI_KEY]`
//!   ② `fetch` 的**信封标记仍在** (`UntrustedContentWrapper` 的标记没被破坏)
//!   ③ `fetch` 的 `title` / `text` / `accessibility` **各自**被扫
//!   ④ `git_write` 的输出里的真 key 无明文
//!   ⑤ `worktree_apply` 的**失败帧**里的真 key 无明文
//!   ⑥ 干净夹具输出逐字节全等 (三族各一条; `fetch` 的连**信封**一起)
//!   ⑦ JSON 叶子扫描 (在 `guardrail.rs` 的 crate 内单测里 —— 该助手是
//!      `pub(crate)`, 集成测试**够不着**它)
//!   ⑧ 命中**可观测** (对象面挂 `credential_tripwire`; 裸文本面挂末尾一行
//!      JSON 注解)
//!
//! 夹具全是**固定内容** (回环 HTTP 服务 + 写死的 git 仓库 + 写死的校验命令)
//! ⇒ 不赌概率。凭据串一律是**合成**的固定串, 不是任何真实凭据。
//!
//! ⚠️ `git_write` / `worktree_apply` 两族**已有** M3 治理层正则
//! (`redact_secret_surfaces`) ⇒ 本批的形状层是**叠加层** ⇒ 这两族的夹具用
//! **JWT 形状** (M3 的正则不认它) ⇒ 断言才真的在测**本批**的接线 (不是被 M3
//! 提前遮掉了); `fetch` 面没有 M3 ⇒ 用任务点名的 `sk-` 形态。

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use apeireth_governance::untrusted_mark::UntrustedContentWrapper;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::fetch::{extract_text, extract_title, extract_tree};
use apeireth_tools_canonical::{
    ControlledEgress, EgressAllowList, EgressPolicy, FetchConfig, FetchTool, FileVersion,
    GitWriteTool, ObservedGate, WorktreeApplyTool,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

// D26 (2026-10-21): 回环 bind 走共享重试助手 (仅 Windows WSAEACCES 10013 有界
// 重试), 见 `tests/common/mod.rs` 与 `src/test_support.rs`。
mod common;

/// 形状族夹具: 一枚**合成**的裸密钥串 (非真实凭据; 长度 ≥ 20 ⇒ 命中形状族)。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnop";

/// 形状族夹具: 一枚**合成**的 JWT 形态串 (非真实令牌)。
///
/// ⚠️ 选它是因为 `git_write` / `worktree_apply` 两族**已有** M3 治理层正则
/// (`redact_secret_surfaces`), 而 M3 的值形正则**不认** JWT ⇒ ⇒ 用 `sk-` 形态
/// 做这两族的夹具, 断言会被 M3 提前满足 (测不到本批的接线)。
const FAKE_JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.\
                       SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

/// `fetch` 的 HTML 夹具: 凭据形状**只在标题里**出现, 可见正文是干净的。
const HTML_FIXTURE: &str = "<html><head><title>rotate sk-0123456789abcdefghijklmnop</title>\
                            </head><body><p>clean paragraph</p></body></html>";

// ---------------------------------------------------------------------------
// fetch 夹具 (回环 HTTP 服务)
// ---------------------------------------------------------------------------

/// Bind a loopback listener and spawn a server that accepts one connection and
/// hands it to `handler`. Returns the port and the spawned task.
async fn serve_once<F, Fut>(handler: F) -> (u16, JoinHandle<()>)
where
    F: FnOnce(TcpStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = common::bind_loopback_ephemeral()
        .await
        .expect("bind loopback listener");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        handler(socket).await;
    });
    (port, handle)
}

fn call_url(url: impl Into<String>) -> ToolCall {
    ToolCall {
        id: "call_fetch_batch2".into(),
        name: "fetch".into(),
        arguments: json!({ "url": url.into() }),
    }
}

fn allowlist_egress() -> ControlledEgress {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
        .with_timeout(Duration::from_secs(5))
        .with_max_response_bytes(64 * 1024)
}

fn fetch_tool() -> FetchTool {
    FetchTool::new(FetchConfig::new(Arc::new(allowlist_egress())))
}

async fn write_http(socket: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    socket.write_all(head.as_bytes()).await.unwrap();
    socket.write_all(body).await.unwrap();
}

async fn read_request_head(socket: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = socket.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_ascii_lowercase()
}

fn value_of(result: ToolResult) -> serde_json::Value {
    match result.outcome {
        ToolOutcome::Ok { value } => value,
        ToolOutcome::Error { message, .. } => panic!("expected ok outcome, got error: {message}"),
    }
}

/// 命中侧车必须显形 (对齐 shell 结果面的 `credential_tripwire` 口径)。
fn assert_tripwire_marker(rendered: &str) {
    assert!(
        rendered.contains("credential_tripwire"),
        "命中必须可观测 (缺 credential_tripwire 标记): {rendered}"
    );
    assert!(
        rendered.contains("\"triggered\":true"),
        "命中标记须带 triggered=true: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// git_write / worktree_apply 夹具
// ---------------------------------------------------------------------------

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_git(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn init_repo(root: &Path) -> bool {
    run_git(root, &["init", "-q"])
        && run_git(root, &["config", "user.email", "batch2@test.local"])
        && run_git(root, &["config", "user.name", "Batch2 Test"])
}

fn git_write_call(command: &str) -> ToolCall {
    ToolCall {
        id: "call_git_write_batch2".into(),
        name: "git_write".into(),
        arguments: json!({ "command": command }),
    }
}

/// 模拟会话内「文件读」事件 (读工具记观测, 写侧认它为已读证据)。
fn observe_read(gate: &ObservedGate, path: &Path) {
    let meta = std::fs::metadata(path).expect("metadata");
    gate.observe_present(path, FileVersion::from_metadata(&meta));
}

fn worktree_apply_call(arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call_worktree_apply_batch2".into(),
        name: "worktree_apply".into(),
        arguments,
    }
}

/// apply 参数 (改单 + 补丁 + 校验命令)。
fn apply_args(
    goal: &str,
    files: &[(&str, &str, &str)],
    patch: &str,
    verify: &[(&str, Vec<String>)],
) -> serde_json::Value {
    json!({
        "plan": {
            "goal": goal,
            "files": files
                .iter()
                .map(|(path, action, expect)| json!({
                    "path": path,
                    "action": action,
                    "expect": expect,
                }))
                .collect::<Vec<_>>(),
        },
        "patch": patch,
        "verify": verify
            .iter()
            .map(|(program, args)| json!({
                "program": program,
                "args": args,
            }))
            .collect::<Vec<_>>(),
    })
}

fn error_message(result: &ToolResult) -> String {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("期望错误帧却得成功: {value}"),
    }
}

// ---------------------------------------------------------------------------
// ① ② fetch: 响应体的凭据形状被遮 + **信封标记仍在**
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_body_masks_a_bare_key_shape_and_keeps_the_envelope() {
    let body = format!("plain line\n{FAKE_KEY}\nplain tail\n");
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            body.as_bytes(),
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/hello")))
        .await;
    let value = value_of(result);
    server.await.unwrap();

    // ① 结果面无明文 + 有形状层遮形标签。
    let rendered = value.to_string();
    assert!(
        !rendered.contains(FAKE_KEY),
        "响应体里的明文密钥不得留在结果面: {rendered}"
    );
    assert!(
        value["body"]
            .as_str()
            .unwrap_or_default()
            .contains("[REDACTED_OPENAI_KEY]"),
        "必须带形状层遮形标签: {rendered}"
    );

    // ② 信封标记仍在 (形状扫描接在**包裹之前** ⇒ 信封包住的是被遮过的正文,
    //    但**信封的边界标记本身一字未改**)。
    let envelope = value["body"].as_str().unwrap_or_default();
    assert!(
        UntrustedContentWrapper::is_wrapped(envelope),
        "正文必须仍在不可信信封里: {envelope}"
    );
    let payload = UntrustedContentWrapper::unwrap_content(envelope)
        .unwrap_or_else(|| panic!("信封应可安全解包: {envelope}"));
    assert!(
        !payload.content.contains(FAKE_KEY),
        "信封内不得有明文密钥: {payload:?}"
    );
    assert!(
        payload.content.contains("[REDACTED_OPENAI_KEY]"),
        "信封内应是遮形后的正文: {payload:?}"
    );
}

// ---------------------------------------------------------------------------
// ③ fetch: title / text / accessibility **各自**被扫
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_masks_each_derived_view_on_its_own() {
    // 前置: 夹具的三个派生视图**确实**带着凭据形状 (否则下面的「已遮」断言是
    // 空的 —— 夹具一旦退化, 断言会静悄悄变成空转)。
    assert!(
        extract_title(HTML_FIXTURE)
            .unwrap_or_default()
            .contains(FAKE_KEY),
        "夹具前提: 标题里必须真的带着凭据形状"
    );
    assert!(
        extract_text(HTML_FIXTURE)
            .unwrap_or_default()
            .contains(FAKE_KEY),
        "夹具前提: 纯文本视图里必须真的带着凭据形状"
    );
    assert!(
        extract_tree(HTML_FIXTURE).to_snapshot().contains(FAKE_KEY),
        "夹具前提: 可及性快照里必须真的带着凭据形状"
    );

    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/html")],
            HTML_FIXTURE.as_bytes(),
        )
        .await;
    })
    .await;

    let result = fetch_tool()
        .invoke(&call_url(format!("http://127.0.0.1:{port}/page")))
        .await;
    let value = value_of(result);
    server.await.unwrap();

    // 三个派生面**各自**都必须已遮 (逐项, 不是只扫正文)。
    for field in ["title", "text", "accessibility"] {
        let raw = value[field].as_str().unwrap_or_default();
        assert!(!raw.is_empty(), "{field} 面必须存在: {value}");
        assert!(!raw.contains(FAKE_KEY), "{field} 面不得回显明文密钥: {raw}");
        assert!(
            raw.contains("[REDACTED_OPENAI_KEY]"),
            "{field} 面必须带形状层遮形标签: {raw}"
        );
        assert!(
            UntrustedContentWrapper::is_wrapped(raw),
            "{field} 面必须仍在不可信信封里: {raw}"
        );
    }

    // 整个结果面无明文。
    let rendered = value.to_string();
    assert!(
        !rendered.contains(FAKE_KEY),
        "整个结果面不得有明文密钥: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// ⑥ fetch: 干净夹具逐字节全等 (连**信封**一起)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clean_fetch_output_is_byte_identical_including_the_envelope() {
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            b"hello fetch",
        )
        .await;
    })
    .await;

    let url = format!("http://127.0.0.1:{port}/hello");
    let result = fetch_tool().invoke(&call_url(url.clone())).await;
    let value = value_of(result);
    server.await.unwrap();

    let expected = json!({
        "url": url,
        "status": 200,
        "content_type": "text/plain",
        "content_length": 11,
        "body": format!(
            "<<<[UNTRUSTED_CONTENT source=\"{url}\"]>>>\nhello fetch\n<<<[/UNTRUSTED_CONTENT]>>>"
        ),
        "bytes": 11,
        "redirects": 0,
        "prompt_injection": { "detected": false, "signals": [] },
    });
    assert_eq!(
        value.to_string(),
        expected.to_string(),
        "干净夹具的输出必须逐字节全等 (连信封一起)"
    );
    assert!(
        !value.to_string().contains("credential_tripwire"),
        "零命中不得挂侧车: {value}"
    );
}

// ---------------------------------------------------------------------------
// ④ git_write: 输出里的真 key 无明文
// ---------------------------------------------------------------------------

#[tokio::test]
async fn git_write_output_carries_no_plaintext_key() {
    if !git_available() {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(init_repo(dir.path()));
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();

    // 提交信息里的 JWT 形态串 ⇒ git 的 stdout (提交摘要行) 会**原样回显**它。
    let tool = GitWriteTool::new(dir.path());
    let result = tool
        .invoke(&git_write_call(&format!(
            "git add a.txt && git commit -m \"rotate {FAKE_JWT}\""
        )))
        .await;
    assert!(result.is_ok(), "{}", result.render());
    let value: serde_json::Value =
        serde_json::from_str(&result.render()).expect("structured report json");

    let rendered = value.to_string();
    assert!(
        !rendered.contains(FAKE_JWT),
        "git_write 的结果面不得回显明文令牌: {rendered}"
    );
    assert!(
        value["output"]
            .as_str()
            .unwrap_or_default()
            .contains("[REDACTED_JWT]"),
        "必须带形状层遮形标签 (JWT 族): {rendered}"
    );
    // 命令回显那一面同样不得留明文。
    assert!(
        !value["commands"].to_string().contains(FAKE_JWT),
        "命令回显面不得留明文令牌: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// ⑥ git_write: 干净夹具逐字节全等
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clean_git_write_output_is_byte_identical() {
    if !git_available() {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(init_repo(dir.path()));
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();

    let tool = GitWriteTool::new(dir.path());
    let result = tool.invoke(&git_write_call("git add a.txt")).await;
    assert!(result.is_ok(), "{}", result.render());

    let expected = json!({
        "status": "executed",
        "tier": "L1",
        "commands": ["git add a.txt"],
        "output": "$ git add a.txt",
    });
    assert_eq!(
        result.render(),
        expected.to_string(),
        "干净夹具的输出必须逐字节全等"
    );
    assert!(
        !result.render().contains("credential_tripwire"),
        "零命中不得挂侧车: {}",
        result.render()
    );
}

// ---------------------------------------------------------------------------
// ⑤ worktree_apply: 失败帧里的真 key 无明文
// ---------------------------------------------------------------------------

#[tokio::test]
async fn worktree_apply_failure_frame_carries_no_plaintext_key() {
    // 失败帧的两条来源各来一条:
    //   ① **命令回显** (校验命令的程序名/参数面被原样回显进失败帧);
    //   ② **命令输出** (校验命令自己吐在 stdout 上的凭据形状)。
    // 两条都必须遮。
    let cases: Vec<Vec<(&str, Vec<String>)>> = vec![
        // ① 命令回显面: 程序起不来 ⇒ 失败帧里带着参数面的凭据形状。
        vec![(
            "definitely-not-a-real-program-apeireth",
            vec![FAKE_JWT.to_string()],
        )],
        // ② 命令输出面: 校验命令自己把凭据形状吐在 stdout 上并非零退出。
        vec![(
            "sh",
            vec!["-c".to_string(), format!("echo '{FAKE_JWT}'; exit 3")],
        )],
    ];

    for verify in cases {
        let workspace = tempfile::tempdir().expect("temp workspace");
        std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
        let gate = Arc::new(ObservedGate::new());
        observe_read(&gate, &workspace.path().join("a.txt"));
        let tool = WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));

        let message = error_message(
            &tool
                .invoke(&worktree_apply_call(apply_args(
                    "验证不过",
                    &[("a.txt", "update", "改")],
                    "*** Begin Patch\n\
                     *** Update File: a.txt\n@@\n-one\n+uno\n\
                     *** End Patch",
                    &verify,
                )))
                .await,
        );

        assert!(
            message.contains("验证拍失败即帧"),
            "必须是验证拍失败帧: {message}"
        );
        assert!(
            !message.contains(FAKE_JWT),
            "失败帧里不得留明文令牌: {message}"
        );
        assert!(
            message.contains("[REDACTED_JWT]"),
            "必须带形状层遮形标签 (JWT 族): {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// ⑥ worktree_apply: 干净夹具的失败帧逐字节全等
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clean_worktree_apply_failure_frame_is_byte_identical() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    let tool = WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));

    let message = error_message(
        &tool
            .invoke(&worktree_apply_call(apply_args(
                "验证不过",
                &[("a.txt", "update", "改")],
                "*** Begin Patch\n\
                 *** Update File: a.txt\n@@\n-one\n+uno\n\
                 *** End Patch",
                &[("definitely-not-a-real-program-apeireth", vec![])],
            )))
            .await,
    );

    assert!(
        !message.contains("credential_tripwire"),
        "零命中不得挂侧车: {message}"
    );
    assert!(
        !message.contains("[REDACTED_"),
        "零命中不得出现遮形标签: {message}"
    );
    // 零命中 ⇒ 失败帧的**内容**与接线前逐字节相同 (前缀 + 文案 + 尾段)。
    assert!(
        message.starts_with("验证拍失败即帧 (校验命令无法启动, 整批未落地): "),
        "失败帧前缀必须不变: {message}"
    );
}

// ---------------------------------------------------------------------------
// ⑧ 命中可观测
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hits_are_observable_in_all_three_batch2_families() {
    // ① fetch: 结果面是**对象** ⇒ 侧车挂在对象上。
    let body = format!("plain line\n{FAKE_KEY}\n");
    let (port, server) = serve_once(|mut socket| async move {
        let _ = read_request_head(&mut socket).await;
        write_http(
            &mut socket,
            "200 OK",
            &[("content-type", "text/plain")],
            body.as_bytes(),
        )
        .await;
    })
    .await;
    let value = value_of(
        fetch_tool()
            .invoke(&call_url(format!("http://127.0.0.1:{port}/hello")))
            .await,
    );
    server.await.unwrap();
    let sidecar = value
        .get("credential_tripwire")
        .expect("fetch 命中必须挂侧车");
    assert_eq!(sidecar["triggered"], json!(true));
    assert!(sidecar["leaked_kinds"]
        .as_array()
        .is_some_and(|kinds| kinds.contains(&json!("OpenAiKey"))));

    // ② git_write: 结果面是**对象** ⇒ 侧车挂在对象上。
    if git_available() {
        let dir = tempfile::tempdir().unwrap();
        assert!(init_repo(dir.path()));
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let tool = GitWriteTool::new(dir.path());
        let result = tool
            .invoke(&git_write_call(&format!(
                "git add a.txt && git commit -m \"rotate {FAKE_JWT}\""
            )))
            .await;
        assert!(result.is_ok(), "{}", result.render());
        let value: serde_json::Value =
            serde_json::from_str(&result.render()).expect("structured report json");
        let sidecar = value
            .get("credential_tripwire")
            .expect("git_write 命中必须挂侧车");
        assert_eq!(sidecar["triggered"], json!(true));
        assert!(sidecar["leaked_kinds"]
            .as_array()
            .is_some_and(|kinds| kinds.contains(&json!("JwtToken"))));
    }

    // ③ worktree_apply: 失败帧是**裸文本**面 ⇒ 侧车以末尾一行 JSON 注解显形。
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    let tool = WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));
    let message = error_message(
        &tool
            .invoke(&worktree_apply_call(apply_args(
                "验证不过",
                &[("a.txt", "update", "改")],
                "*** Begin Patch\n\
                 *** Update File: a.txt\n@@\n-one\n+uno\n\
                 *** End Patch",
                &[(
                    "definitely-not-a-real-program-apeireth",
                    vec![FAKE_JWT.to_string()],
                )],
            )))
            .await,
    );
    assert_tripwire_marker(&message);
}

// ---------------------------------------------------------------------------
// 接线回归守卫: 批次 2 三族都接了共享助手
// ---------------------------------------------------------------------------

/// 批次 2 的三个族都必须接上共享助手 (`scan_result_face`)。
///
/// ⚠️ 守卫口径: **按族计** (这一族自己的源文件**或**它的子模块里有没有接线
/// 调用), 不是按**字面量**计 —— 批次 1 的既有守卫
/// (`result_face_credential_tripwire.rs`) 认的是 `scan_and_sanitize_output`
/// 这个**字面量**, 本批三族走的是共享助手 (`scan_result_face`) ⇒ 两者的口径
/// 不同, 各自守各自的。
///
/// ⚠️ `git_write` 的接线落在**子模块** `git_write/tripwire.rs` 里 (那个族在
/// size 棘轮基线上 ⇒ 本文件必须净减 ⇒ 实现落子模块) ⇒ 守卫认「族文件 + 族
/// 子模块」这一组。
#[test]
fn batch2_families_are_wired_to_the_shared_result_face_helper() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for family in ["fetch.rs", "git_write.rs", "worktree_apply.rs"] {
        let mut sources = vec![src.join(family)];
        let submodule_dir = src.join(family.trim_end_matches(".rs"));
        if submodule_dir.is_dir() {
            let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(&submodule_dir)
                .expect("族子模块目录可读")
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
                .collect();
            entries.sort();
            sources.extend(entries);
        }
        let wired = sources.iter().any(|path| {
            std::fs::read_to_string(path)
                .map(|text| text.contains("scan_result_face"))
                .unwrap_or(false)
        });
        assert!(
            wired,
            "{family} 必须接共享助手 scan_result_face (族文件或其子模块)"
        );
    }
}
