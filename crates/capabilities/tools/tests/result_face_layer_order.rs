//! 结果面两层 (形状层 → 字段层) 的层序回归 —— 批次 1 裁决的期望行为。
//!
//! 覆盖面: 统一助手 (`crate::guardrail::scan_result_face`) 把两层按
//! **形状层 (M1, 凭据形状) → 字段值层 (M4, 字段值脱敏)** 的层序过一遍:
//!   ① 形状层先在**明文**上判定 ⇒ 命中是真的 (明文口令在明文上命中, 不是对
//!      「已遮过口令」的内容报命中);
//!   ② 字段层后接管**字段值** ⇒ 含口令赋值的内容, 终稿渲染与接线前**逐字节
//!      相同** (可见标签仍是 `[redacted]`, 零可见变化)。
//!
//! 本文件钉的是**期望行为** (不是现状刻画):
//!   · 裸形状 (不在字段值位) 归形状层 ⇒ 标签是形状层的;
//!   · 字段值位里的形状归字段层 ⇒ 标签是字段层的, 但形状族**仍报命中**
//!     (旧层序下形状先被字段层遮掉 ⇒ 绊线不响);
//!   · 形状层在字段值位与裸形状位上的**分工**互不吞并。
//!
//! 夹具全是**固定内容** (临时目录里写死的文件 / 写死的 git 仓库) ⇒ 不赌概率。
//! 凭据串一律是合成的固定串, 不是任何真实凭据。
//!
//! git 不可用时按本 crate 既有约定早退 (`repo.rs` 的既有测试同款)。

use std::fs;
use std::path::Path;
use std::process::Command;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::filesystem::FilesystemTool;
use apeireth_tools_canonical::guardrail::{LeakedCredentialKind, ToolGuardrail};
use apeireth_tools_canonical::repo::RepoTool;
use apeireth_tools_canonical::search::SearchTool;
use serde_json::json;
use tempfile::tempdir;

/// 形状族夹具: 一枚**合成**的裸密钥串 (非真实凭据; 长度 ≥ 20 ⇒ 命中形状族)。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnop";

/// 形状族夹具: 一段**合成**的明文口令赋值 (非真实口令)。
const FAKE_PASSWORD_LINE: &str = "password = hunter2-not-a-real-secret\n";

async fn filesystem_read(root: &Path, path: &str) -> String {
    let tool = FilesystemTool::new(root.to_path_buf());
    let call = ToolCall {
        id: "call_fs".into(),
        name: "filesystem".into(),
        arguments: json!({ "operation": "read", "path": path }),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "filesystem read 失败: {}", result.render());
    result.render()
}

async fn search_query(root: &Path, query: &str) -> String {
    let tool = SearchTool::new(root.to_path_buf());
    let call = ToolCall {
        id: "call_search".into(),
        name: "search".into(),
        arguments: json!({ "query": query, "path": "." }),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "search 失败: {}", result.render());
    result.render()
}

async fn repo_operation(root: &Path, operation: &str) -> String {
    let tool = RepoTool::new(root.to_path_buf());
    let call = ToolCall {
        id: "call_repo".into(),
        name: "repo".into(),
        arguments: json!({ "operation": operation }),
    };
    let result = tool.invoke(&call).await;
    result.render()
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 建一个**固定**的 git 仓库 (作者/邮箱/时间钉死 ⇒ 提交号可复现)。
fn init_repo_with_commit(root: &Path, message: &str) -> bool {
    let run = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_AUTHOR_DATE", "2026-01-02T03:04:05+00:00")
            .env("GIT_COMMITTER_DATE", "2026-01-02T03:04:05+00:00")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@test.local")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@test.local")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };
    if !run(&["init", "-q"]) {
        return false;
    }
    if !run(&["config", "user.email", "fixture@test.local"])
        || !run(&["config", "user.name", "Fixture"])
    {
        return false;
    }
    fs::write(root.join("fixture-file.txt"), "fixture content\n").unwrap();
    if !run(&["add", "-A"]) {
        return false;
    }
    run(&["commit", "-q", "-m", message])
}

// ── 裸形状: 归形状层 ────────────────────────────────────────────────────────

/// 裸形状 (不在字段值位) 归**形状层** ⇒ 可见标签是形状层的标签; 字段层不插手。
#[tokio::test]
async fn a_bare_shape_keeps_the_shape_layer_label() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-bare.txt"),
        format!("plain line\n{FAKE_KEY}\n"),
    )
    .unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-bare.txt").await;

    assert!(
        !rendered.contains(FAKE_KEY),
        "明文密钥不得留在结果面: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_OPENAI_KEY]"),
        "裸形状归形状层 (标签必须是形状层的): {rendered}"
    );
    assert!(
        rendered.contains("OpenAiKey"),
        "形状族必须报命中: {rendered}"
    );
}

// ── 字段值位里的形状: 归字段层, 但形状族仍报命中 ────────────────────────────

/// 字段值位里的形状归**字段层** ⇒ 可见标签与接线前逐字节相同
/// (`api_key = "[redacted]"`), 而形状族**仍报命中**。
///
/// 这一条是层序换向的净收益: 旧层序 (字段层 → 形状层) 下形状先被字段层遮掉
/// ⇒ 绊线不响 (命中静默); 新层序下形状层先在明文上判定 ⇒ 绊线报命中, 而
/// 可见标签仍与接线前一致。
#[tokio::test]
async fn a_shape_inside_a_field_value_keeps_the_pre_wiring_rendering_and_still_hits() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-field.txt"),
        format!("api_key = \"{FAKE_KEY}\"\n"),
    )
    .unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-field.txt").await;

    assert!(
        !rendered.contains(FAKE_KEY),
        "明文密钥不得留在结果面: {rendered}"
    );
    assert!(
        rendered.starts_with("api_key = \"[redacted]\"\n"),
        "字段值位归字段层 (可见标签必须与接线前逐字节相同): {rendered}"
    );
    assert!(
        rendered.contains("OpenAiKey"),
        "形状族必须报命中 (命中不静默): {rendered}"
    );
}

// ── search 命中行: 层序同样生效 ─────────────────────────────────────────────

/// search 的命中行同样按 **形状层 → 字段层** 过 ⇒ 口令字段值的可见标签与接线
/// 前逐字节相同, 且口令族真命中。
#[tokio::test]
async fn the_search_hit_line_keeps_the_pre_wiring_rendering() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-hit.txt"),
        format!("plain line\n{FAKE_PASSWORD_LINE}"),
    )
    .unwrap();

    let rendered = search_query(dir.path(), "hunter2").await;

    assert!(
        !rendered.contains("hunter2-not-a-real-secret"),
        "明文口令不得留在结果面: {rendered}"
    );
    assert!(
        rendered.contains("password = [redacted]"),
        "命中行的口令字段值归字段层 (可见标签与接线前一致): {rendered}"
    );
    assert!(
        rendered.contains("PlaintextPassword"),
        "口令族必须报命中: {rendered}"
    );
}

// ── repo 族: 形状层单独 (与前批行为等价) ───────────────────────────────────

/// repo 族的结果面 (git 输出 / git 报错) 这一面**没有字段值层** (接线前也没有)
/// ⇒ 只过形状层。
///
/// 断言只钉**与层序无关**的不变量 (明文不出境 + 口令族报命中): 字段层要不要
/// 扩到这一面, 是主代理的裁决点, 本条不预判。
#[tokio::test]
async fn the_repo_face_never_leaks_a_plaintext_password() {
    if !git_available() {
        return;
    }
    let dir = tempdir().unwrap();
    assert!(
        init_repo_with_commit(dir.path(), "rotate password = hunter2-not-a-real-secret"),
        "git 夹具建仓失败"
    );

    let rendered = repo_operation(dir.path(), "log").await;

    assert!(
        !rendered.contains("hunter2-not-a-real-secret"),
        "明文口令不得留在结果面: {rendered}"
    );
    assert!(
        rendered.contains("PlaintextPassword"),
        "口令族必须报命中: {rendered}"
    );
}

// ── 内核事实 (未随层序改动) ────────────────────────────────────────────────

/// 内核的**豁免范围**没随层序改动: 形状层只豁免自己那套大写 `[REDACTED_…]`
/// 标签 ⇒ 字段层的小写 `[redacted]` 标签**不**被口令族豁免。
///
/// 这正是「必须把形状层排到字段层之前」的原因: 旧层序下形状层扫的是字段层
/// 已经遮过的内容 ⇒ 口令族对已遮内容报命中 (不是对明文的真命中)。
#[test]
fn the_kernel_exemption_scope_is_unchanged() {
    let scan = ToolGuardrail::scan_and_sanitize_output("password = [redacted]");
    assert!(
        scan.leaked_kinds
            .contains(&LeakedCredentialKind::PlaintextPassword),
        "内核豁免范围未改: 口令族仍不豁免字段层的小写标签: {:?}",
        scan.leaked_kinds
    );

    // 对照: 非口令字段 (key/token 族) 的字段层标签**不**触发形状层 ⇒ 不误伤。
    let clean = ToolGuardrail::scan_and_sanitize_output("api_key = [redacted]");
    assert!(
        clean.is_clean,
        "非口令字段的字段层标签不得误伤: {:?}",
        clean.leaked_kinds
    );
}
