//! 结果面凭据绊线 —— 批次 1 接线回归测试。
//!
//! 覆盖面: 默认开的三个工具族 (`filesystem` / `search` / `repo`) 的**结果面**
//! —— 统一助手 `crate::guardrail::scan_result_face` (形状层 =
//! `ToolGuardrail::scan_and_sanitize_output`, 字段值层 = 字段值脱敏) 按
//! **形状层 → 字段层** 的层序过, 命中即遮形并挂上可观测的
//! `credential_tripwire` 标记 (字段口径对齐 shell 结果面); 零命中则**逐字节
//! 不变**、不挂任何标记。
//!
//! 验收条目 (逐条对应):
//!   ① 裸密钥形状 ⇒ 结果无明文 + `[REDACTED_OPENAI_KEY]`        (filesystem)
//!   ② PEM 私钥块 ⇒ `[REDACTED_PEM_PRIVATE_KEY]`, 无 `PRIVATE KEY-----`
//!   ③ search 命中行无明文
//!   ④ repo 提交信息里的密钥无明文
//!   ⑤ 干净夹具的输出逐字节全等 (三族各一条)
//!   ⑥ 命中可观测 (`credential_tripwire` 标记)
//!   ⑦ 接线回归守卫: `scan_and_sanitize_output` 的生产接线方恰为四族
//!   ⑧ 层序 (形状层 → 字段层): 明文口令 ⇒ 内容部分与接线前逐字节相同
//!      (`password = [redacted]`) + 绊线报口令族 (真命中)
//!
//! 夹具全是**固定内容** (临时目录里写死的文件 / 写死的 git 仓库) ⇒ 不赌概率。
//! 凭据串一律是合成的固定串, 不是任何真实凭据。
//!
//! git 不可用时按本 crate 既有约定早退 (`repo.rs` 的既有测试同款)。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::filesystem::FilesystemTool;
use apeireth_tools_canonical::repo::RepoTool;
use apeireth_tools_canonical::search::SearchTool;
use serde_json::json;
use tempfile::tempdir;

/// 形状族夹具: 一枚**合成**的裸密钥串 (非真实凭据; 长度 ≥ 20 ⇒ 命中形状族)。
const FAKE_KEY: &str = "sk-0123456789abcdefghijklmnop";

/// 形状族夹具: 一块**合成**的 PEM 私钥块 (非真实私钥)。
const FAKE_PEM: &str = concat!(
    "-----BEGIN RSA PRIVATE KEY-----\n",
    "MIIBOgIBAAJBAKj34GkxFhD90vcNLYLInFEX6Ppy1tPf9Cnzj4p4WGeKLs1Pt8Qu\n",
    "-----END RSA PRIVATE KEY-----\n",
);

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

// ── ① 裸密钥形状 ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn filesystem_result_masks_a_bare_key_shape() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-key.txt"),
        format!("plain line\n{FAKE_KEY}\n"),
    )
    .unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-key.txt").await;

    assert!(
        !rendered.contains(FAKE_KEY),
        "明文密钥不得留在结果面: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_OPENAI_KEY]"),
        "必须带遮形标签: {rendered}"
    );
}

// ── ② PEM 私钥块 ────────────────────────────────────────────────────────────

#[tokio::test]
async fn filesystem_result_masks_a_pem_private_key_block() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("fixture-pem.txt"), FAKE_PEM).unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-pem.txt").await;

    assert!(
        rendered.contains("[REDACTED_PEM_PRIVATE_KEY]"),
        "必须带 PEM 遮形标签: {rendered}"
    );
    assert!(
        !rendered.contains("PRIVATE KEY-----"),
        "私钥块标记不得残留: {rendered}"
    );
}

// ── ③ search 命中行 ─────────────────────────────────────────────────────────

#[tokio::test]
async fn search_hit_line_carries_no_plaintext_key() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-hit.txt"),
        format!("TOKENLINE {FAKE_KEY}\n"),
    )
    .unwrap();

    let rendered = search_query(dir.path(), "tokenline").await;

    assert!(
        !rendered.contains(FAKE_KEY),
        "命中行不得回显明文密钥: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_OPENAI_KEY]"),
        "必须带遮形标签: {rendered}"
    );
}

// ── ④ repo 提交信息 ─────────────────────────────────────────────────────────

#[tokio::test]
async fn repo_log_output_carries_no_plaintext_key() {
    if !git_available() {
        return;
    }
    let dir = tempdir().unwrap();
    assert!(
        init_repo_with_commit(dir.path(), &format!("rotate credential {FAKE_KEY}")),
        "git 夹具建仓失败"
    );

    let rendered = repo_operation(dir.path(), "log").await;

    assert!(
        !rendered.contains(FAKE_KEY),
        "提交信息不得回显明文密钥: {rendered}"
    );
    assert!(
        rendered.contains("[REDACTED_OPENAI_KEY]"),
        "必须带遮形标签: {rendered}"
    );
}

// ── ⑤ 干净夹具: 逐字节全等 ──────────────────────────────────────────────────

#[tokio::test]
async fn clean_filesystem_output_is_byte_identical() {
    let dir = tempdir().unwrap();
    let content = "plain content line\nsecond plain line\n";
    fs::write(dir.path().join("fixture-clean.txt"), content).unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-clean.txt").await;

    assert_eq!(rendered, content, "干净夹具的输出必须逐字节全等");
    assert!(
        !rendered.contains("credential_tripwire"),
        "零命中不得挂侧车: {rendered}"
    );
}

#[tokio::test]
async fn clean_search_output_is_byte_identical() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("fixture-clean.txt"), "plain content line\n").unwrap();

    let rendered = search_query(dir.path(), "plain").await;

    let expected = json!({
        "query": "plain",
        "path": ".",
        "count": 1,
        "truncated": false,
        "matches": [{
            "path": "fixture-clean.txt",
            "line": 1,
            "text": "plain content line",
            "column": 1,
            "occurrences": 1,
        }],
    });
    assert_eq!(
        rendered,
        expected.to_string(),
        "干净夹具的输出必须逐字节全等"
    );
    assert!(
        !rendered.contains("credential_tripwire"),
        "零命中不得挂侧车: {rendered}"
    );
}

#[tokio::test]
async fn clean_repo_output_is_byte_identical() {
    if !git_available() {
        return;
    }
    let dir = tempdir().unwrap();
    assert!(
        init_repo_with_commit(dir.path(), "clean fixture commit"),
        "git 夹具建仓失败"
    );

    let rendered = repo_operation(dir.path(), "status").await;

    let expected = json!({
        "operation": "status",
        "output": "git status completed with empty output",
        "truncated": false,
    });
    assert_eq!(
        rendered,
        expected.to_string(),
        "干净夹具的输出必须逐字节全等"
    );
    assert!(
        !rendered.contains("credential_tripwire"),
        "零命中不得挂侧车: {rendered}"
    );
}

// ── ⑥ 命中可观测 ────────────────────────────────────────────────────────────

#[tokio::test]
async fn hits_are_observable_in_all_three_families() {
    let fs_dir = tempdir().unwrap();
    fs::write(
        fs_dir.path().join("fixture-key.txt"),
        format!("plain line\n{FAKE_KEY}\n"),
    )
    .unwrap();
    assert_tripwire_marker(&filesystem_read(fs_dir.path(), "fixture-key.txt").await);

    let search_dir = tempdir().unwrap();
    fs::write(
        search_dir.path().join("fixture-hit.txt"),
        format!("TOKENLINE {FAKE_KEY}\n"),
    )
    .unwrap();
    assert_tripwire_marker(&search_query(search_dir.path(), "tokenline").await);

    if !git_available() {
        return;
    }
    let repo_dir = tempdir().unwrap();
    assert!(
        init_repo_with_commit(repo_dir.path(), &format!("rotate credential {FAKE_KEY}")),
        "git 夹具建仓失败"
    );
    assert_tripwire_marker(&repo_operation(repo_dir.path(), "log").await);
}

// ── 明文口令赋值 (口令族) ───────────────────────────────────────────────────

#[tokio::test]
async fn filesystem_result_masks_a_plaintext_password_assignment() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-password.txt"),
        format!("plain line\n{FAKE_PASSWORD_LINE}"),
    )
    .unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-password.txt").await;

    assert!(
        !rendered.contains("hunter2-not-a-real-secret"),
        "明文口令不得留在结果面: {rendered}"
    );
    // 层序 (批次 1 裁决): 形状层 → 字段层 ⇒ 口令字段值归**字段层** ⇒ 可见标签
    // 与接线前逐字节相同 (`[redacted]`), 不漂成形状层的标签。
    assert!(
        rendered.contains("password = [redacted]"),
        "口令字段值必须由字段层脱敏 (可见标签与接线前一致): {rendered}"
    );
    // 形状层在**明文**上判定 ⇒ 口令族是真命中 (不是对已遮内容的误命中)。
    assert!(
        rendered.contains("PlaintextPassword"),
        "绊线必须报口令族: {rendered}"
    );
}

// ── 层间交互: 形状层 → 字段层 (期望行为) ───────────────────────────────────

/// **期望行为 (不是现状刻画)**: 结果面按 **形状层 (M1) → 字段值层 (M4)** 的
/// 层序过一遍 ⇒
///   ① 明文口令在**形状层**真命中 (绊线报 `PlaintextPassword`, 判定发生在明文
///      上, 不是对「已遮过口令」的内容报命中);
///   ② **字段层后接管**该字段值 ⇒ 可见标签与接线前**逐字节相同**
///      (`password = [redacted]`, 零可见变化)。
///
/// 旧层序 (字段层 → 形状层) 的两个毛病 (本批裁决要消掉的): 字段层先换成
/// `[redacted]`, 形状层再扫一遍 ⇒ 口令族对**已遮过**的内容报命中; 且可见
/// 标签从 `[redacted]` 漂成形状层的标签 (可见变化)。
#[tokio::test]
async fn the_field_layer_keeps_the_pre_wiring_label_and_the_password_family_still_hits() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("fixture-password.txt"),
        format!("plain line\n{FAKE_PASSWORD_LINE}"),
    )
    .unwrap();

    let rendered = filesystem_read(dir.path(), "fixture-password.txt").await;

    // ① 明文不得留在结果面。
    assert!(
        !rendered.contains("hunter2-not-a-real-secret"),
        "明文口令不得留在结果面: {rendered}"
    );
    // ② 可见标签与接线前逐字节相同 (接线前的渲染 = 明文经字段值层脱敏 ⇒
    //    `plain line\npassword = [redacted]\n`; 接线后只多出末尾那行可观测
    //    的绊线载荷, 内容部分逐字节不变)。
    assert!(
        rendered.starts_with("plain line\npassword = [redacted]\n"),
        "内容部分必须与接线前逐字节相同: {rendered}"
    );
    // ③ 绊线仍报口令族 (真命中)。
    assert!(
        rendered.contains("PlaintextPassword"),
        "绊线必须报口令族: {rendered}"
    );
    assert!(
        rendered.contains("credential_tripwire"),
        "命中必须可观测: {rendered}"
    );
}

// ── ⑦ 接线回归守卫 ──────────────────────────────────────────────────────────

#[test]
fn tripwire_is_wired_for_exactly_the_expected_families() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // ⚠️ **模式已扩**（2026-11-07 批次 2 收口）：原先只认内核入口字面量 `scan_and_sanitize_output`
    // ⇒ 走**共享助手**（`scan_result_face` / `scan_json_string_leaves`）的族**不被枚举**
    // ⇒ ⇒ 守卫变成「少报型」（批次 2 的 fetch / git_write / worktree_apply / recon 三族 + 一族 全漏）
    // ⇒ ⇒ **模式扩为三者**，让守卫回到「枚举型」。
    let output = Command::new("grep")
        .arg("-rn")
        .arg("--include=*.rs")
        .arg("-E")
        .arg("scan_and_sanitize_output|scan_result_face|scan_json_string_leaves")
        .arg(&src)
        .output()
        .expect("grep 必须可用 (接线守卫依赖它)");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut wired: BTreeSet<String> = BTreeSet::new();
    for line in stdout.lines() {
        // 内核定义处 (`guardrail.rs`) 不算「接线方」—— 接线方 = 生产调用点。
        if line.contains("guardrail.rs") {
            continue;
        }
        let Some(path) = line.split(':').next() else {
            continue;
        };
        // ⚠️ **族级判定**：某族可能把接线放在**子模块**里（如 `git_write/tripwire.rs`、
        // `recon/tripwire.rs`）⇒ ⇒ 只取 `file_stem` 会把它们算成 `tripwire` ⇒ ⇒ 改为
        // 「首段目录名优先，否则用文件名去扩展」⇒ ⇒ 族名才稳定。
        // ⚠️ grep 传的是**绝对路径**（`CARGO_MANIFEST_DIR/src`）⇒ 输出也是绝对路径
        // ⇒ ⇒ 必须先切到 **`src` 段之后**（绝对路径以 `/` 开头 ⇒ 直接 split 会让族名塌成空串）
        // ⇒ ⇒ 取**最后一个** `src` 段之后的相对部分（如 `fetch.rs` / `git_write/tripwire.rs`）。
        let after_src = match path.rfind("/src/") {
            Some(index) => &path[index + "/src/".len()..],
            None => path,
        };
        let mut segments = after_src.split('/');
        let first = segments.next().unwrap_or_default();
        let family = if segments.next().is_some() {
            // 位于子目录 ⇒ 首段目录名即族名。
            first.to_string()
        } else {
            Path::new(first)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| first.to_string())
        };
        wired.insert(family);
    }

    // 批次 1 = {shell, filesystem, search, repo}；批次 2 = {fetch, git_write, worktree_apply, recon}；
    // 批次 2c = {external}（10 个外部工具适配器，原样回显最凶的一族）；
    // 批次 3b = {content_feed}（**记忆摄取面**：摄取内容入库前清扫 —— 那是
    //   「凭据进存储」的面，比 transcript 更深 ⇒ ⇒ 接线落在摄取内容构造处）；
    // 裁决 E = {repo_map_tool, doc_ops, sheet_ops, skill}（四族）——
    //   ⚠️ `repo_map_tool` 是**族文件所在目录名**（该族文件在 `src/` 根 ⇒ 族名取文件名）
    //   ⇒ ⇒ 与守卫的「首段目录名优先，否则文件名去扩展」判定一致；
    // 并行线 = {http_write, web_search}（两族已实测接入 ⇒ ⇒ 登记它们是**如实登记**）。
    let expected: BTreeSet<String> = [
        "shell",
        "filesystem",
        "search",
        "repo",
        "fetch",
        "git_write",
        "worktree_apply",
        "recon",
        "external",
        "content_feed",
        "repo_map_tool",
        "doc_ops",
        "sheet_ops",
        "skill",
        "http_write",
        "web_search",
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect();
    assert_eq!(
        wired, expected,
        "绊线的生产接线方必须恰为十六族 (批次 1 四族 + 批次 2 四族 + 批次 2c 一族 + 批次 3b 一族 + 裁决 E 四族 + 并行线两族)"
    );

    // ⚠️ **跨族共享模块对枚举「不可见」是一条隐式契约** —— 现在把它**变成显式断言**。
    //
    // 背景: 证据信封的四面遮形编排已上收到**共享模块** `src/evidence_faces.rs`
    // （`recon` / `external` 两族各注入一次扫描入口，见 §2.6.3）。
    // ⇒ 若共享模块**自己**写了那三个字面量, 守卫的族枚举会多出一族 `evidence_faces`
    // ⇒ ⇒ 守卫红, 而报错信息（"expected 10 families"）**看不出真因** ⇒ ⇒ 极易被误判为
    // 「守卫坏了」而放宽它。
    // ⇒ ⇒ 本断言把「共享模块不得直接绑定内核扫描助手」**显式钉住**:
    // 共享模块只负责「按信封形状编排四面」, 扫描入口**必须由族文件注入**
    // （这样族名才会被枚举到）。
    let shared_module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("evidence_faces.rs");
    let shared_source = std::fs::read_to_string(&shared_module)
        .expect("共享模块 evidence_faces.rs 必须存在 (泛化收口批的产物)");
    for literal in [
        "scan_and_sanitize_output",
        "scan_result_face",
        "scan_json_string_leaves",
    ] {
        assert!(
            !shared_source.contains(literal),
            "共享模块不得直接绑定内核扫描助手 `{literal}` —— \
             扫描入口必须由族文件注入, 否则族枚举会多出一族, 且守卫的报错看不出真因"
        );
    }
}
