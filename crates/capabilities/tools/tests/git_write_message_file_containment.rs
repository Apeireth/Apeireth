//! `git commit -F` / `git tag -F` 的**操作数路径包含性校验**回归
//! （2026-11-01 安全审计 P3-7 修）。
//!
//! **原先错在哪**：`grade_commit` / `grade_tag` 对 `--file` / `-F` **只置
//! `message_source` 标志、完全不校验操作数路径** ⇒ `git commit -F /etc/passwd` /
//! `git tag -F ~/.aws/credentials` 能把**工作区外**的文件读进提交/附注对象。
//!
//! **现在为什么这样**：分级面捕获 `-F` / `--file` 的**文件路径操作数**，根感知分级
//! （`classify_git_write_under`）按**工作区根**做包含性校验（口径照抄
//! `filesystem.rs` 的 `resolve_contained`：词法层 guardrail → canonicalize →
//! `starts_with(root)` → 凭据面排除）⇒ 越界即拒绝即帧。

use std::path::{Path, PathBuf};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::git_write::{
    authorized_git_write_policy, classify_git_write, classify_git_write_under,
    git_write_capability, GitWritePlan, GitWriteTier, GIT_WRITE_TOOL_NAME, REFUSAL_MORPH_PREFIX,
};

/// 工具调用一帧 (参数面 = 一条 git 写命令)。
fn call(command: &str) -> ToolCall {
    ToolCall {
        id: "call_git_write".into(),
        name: GIT_WRITE_TOOL_NAME.into(),
        arguments: serde_json::json!({ "command": command }),
    }
}

/// 测试工作区: `<tmp>/workspace` (根) + `<tmp>/secret.txt` (根外文件)。
fn workspace(tag: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!(
        "apeireth-git-write-file-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("workspace");
    std::fs::create_dir_all(&root).expect("创建工作区");
    let outside = base.join("secret.txt");
    std::fs::write(&outside, "root:x:0:0::/root:/bin/bash\n").expect("写根外文件");
    (root, outside)
}

/// 拒绝断言: 必须是 Refused 且帧前缀是「变形面拒绝即帧」。
fn refusal(command: &str, root: &Path) -> String {
    match classify_git_write_under(root, command) {
        GitWritePlan::Refused { reason } => {
            assert!(
                reason.starts_with(REFUSAL_MORPH_PREFIX),
                "{command:?} 的拒绝帧前缀不对: {reason}"
            );
            reason
        }
        GitWritePlan::Graded { tier, .. } => panic!("{command:?} 不应放行: {tier:?}"),
    }
}

/// 放行断言: 必须是 Graded 且分级是 L1 可逆。
fn graded(command: &str, root: &Path) {
    match classify_git_write_under(root, command) {
        GitWritePlan::Graded { tier, .. } => assert_eq!(tier, GitWriteTier::ReversibleLocal),
        GitWritePlan::Refused { reason } => panic!("{command:?} 不应被拒: {reason}"),
    }
}

/// ① `git commit -F <工作区外>` 必须被拒（短选项，另起一词形态）。
#[test]
fn commit_file_outside_workspace_is_refused() {
    let (root, outside) = workspace("commit-outside");
    let command = format!("git commit -F {}", outside.display());
    let reason = refusal(&command, &root);
    assert!(reason.contains("操作数路径越界"), "{reason}");
}

/// ② 附着形态 `git commit -F<工作区外>` 同样必须被拒（同一张脸）。
#[test]
fn commit_attached_file_outside_workspace_is_refused() {
    let (root, outside) = workspace("commit-attached");
    let command = format!("git commit -F{}", outside.display());
    refusal(&command, &root);
}

/// ③ 长选项形态 `git commit --file=<工作区外>` 同样必须被拒。
#[test]
fn commit_long_file_option_outside_workspace_is_refused() {
    let (root, outside) = workspace("commit-long");
    let command = format!("git commit --file={}", outside.display());
    refusal(&command, &root);
}

/// ③' git 允许**无歧义缩写**（`--fil` ≡ `--file`）⇒ 缩写形同样必须被捕获并校验
/// （捕获点必须按「选项的下一个词」取操作数, 不能只认精确拼写）。
#[test]
fn commit_file_abbreviation_outside_workspace_is_refused() {
    let (root, outside) = workspace("commit-abbrev");
    let command = format!("git commit -m msg --fil {}", outside.display());
    refusal(&command, &root);
}

/// ③'' 缩写 + `=` 附着形态同样必须被拒。
#[test]
fn commit_file_abbreviation_attached_outside_workspace_is_refused() {
    let (root, outside) = workspace("commit-abbrev-eq");
    let command = format!("git commit -m msg --fil={}", outside.display());
    refusal(&command, &root);
}

/// ④ `git tag -F <工作区外>` 同样必须被拒（附注对象同权）。
#[test]
fn tag_file_outside_workspace_is_refused() {
    let (root, outside) = workspace("tag-outside");
    let command = format!("git tag -F {} v1", outside.display());
    refusal(&command, &root);
}

/// ⑤ 相对穿越（`../`）必须被拒（词法层）。
#[test]
fn commit_file_relative_traversal_is_refused() {
    let (root, _outside) = workspace("commit-traversal");
    refusal("git commit -F ../secret.txt", &root);
}

/// ⑥ 系统敏感绝对路径（`/etc/passwd`）必须被拒（词法层）。
#[test]
fn commit_file_system_sensitive_path_is_refused() {
    let (root, _outside) = workspace("commit-etc");
    let reason = refusal("git commit -F /etc/passwd", &root);
    assert!(reason.contains("操作数路径越界"), "{reason}");
}

/// ⑦ 凭据面排除: 根**内**的凭据文件同样不可进提交对象。
#[test]
fn commit_file_credential_surface_inside_workspace_is_refused() {
    let (root, _outside) = workspace("commit-credential");
    let credential = root.join(".aws").join("credentials");
    std::fs::create_dir_all(credential.parent().expect("父目录")).expect("建目录");
    std::fs::write(&credential, "[default]\naws_access_key_id = AKIA\n").expect("写凭据文件");
    let command = format!("git commit -F {}", credential.display());
    let reason = refusal(&command, &root);
    assert!(reason.contains("凭据面不可读"), "{reason}");
}

/// ⑧ 工作区**内**的正常消息文件照常放行（校验不误伤合法用法）。
#[test]
fn commit_file_inside_workspace_is_allowed() {
    let (root, _outside) = workspace("commit-inside");
    std::fs::write(root.join("msg.txt"), "chore: 提交说明\n").expect("写消息文件");
    graded("git commit -F msg.txt", &root);
    graded(
        &format!("git commit -F {}", root.join("msg.txt").display()),
        &root,
    );
    graded("git tag -F msg.txt v1", &root);
}

/// ⑨ 无根的纯词法分级面**不**做路径判定（边界如实标注）: 根感知的那条才是执行面
/// 的拦截点 —— 工具面（`GitWriteTool`）持有工作区根 ⇒ 走根感知分级 ⇒ 拒绝即帧，
/// git 进程根本不启动。
#[test]
fn rootless_classification_does_not_check_paths() {
    assert!(matches!(
        classify_git_write("git commit -F /etc/passwd"),
        GitWritePlan::Graded {
            tier: GitWriteTier::ReversibleLocal,
            ..
        }
    ));
}

/// ⑩ 端到端: 工具面（持有工作区根）必须把越界的 `-F` 拒掉 —— git 进程根本不
/// 启动, 仓库里**没有**新提交（原先会真的把根外文件读进提交对象）。
#[tokio::test]
async fn tool_refuses_out_of_workspace_message_file_and_creates_no_commit() {
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
        == false
    {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("workspace");
    std::fs::create_dir_all(&root).unwrap();
    let run_git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    };
    assert!(run_git(&["init", "-q"]));
    assert!(run_git(&["config", "user.email", "d-p7@test.local"]));
    assert!(run_git(&["config", "user.name", "D-P7 Test"]));
    std::fs::write(root.join("a.txt"), "hello").unwrap();
    // 根外文件: 工作区根的**兄弟**目录 ⇒ 一定落在根外。
    let outside = dir.path().join("outside-secret.txt");
    std::fs::write(&outside, "root:x:0:0::/root:/bin/bash\n").unwrap();

    let tool = git_write_capability(&root, authorized_git_write_policy());
    let command = format!("git add a.txt && git commit -F {}", outside.display());
    let result = tool.invoke(&call(&command)).await;
    assert!(!result.is_ok(), "越界的 -F 操作数必须被工具面拒绝");
    let rendered = result.render();
    assert!(rendered.contains("操作数路径越界"), "{rendered}");
    assert!(rendered.contains(REFUSAL_MORPH_PREFIX), "{rendered}");
    // 拒绝即帧: 拒绝发生在**执行前** ⇒ 整条命令不落地 (连 add 都没跑)。
    assert!(!run_git(&["log", "-n", "1", "--oneline"]), "不得有提交落地");
    // 暂存区必须为空 (`git diff --cached` 的退出码恒 0 ⇒ 看输出不看退出码)。
    let staged = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["diff", "--cached", "--name-only"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&staged.stdout).trim().is_empty(),
        "不得有暂存残留"
    );
}
