//! git 写分级 (`tool.git_write`, D-P1) 的词表矩阵 / 拒绝即帧 / 变形防护 /
//! 受控执行语义 行为回归。
//!
//! 三级词表: L1 可逆 (add/commit/branch 建·switch 切/tag 本地) / L2 出网
//! (push/fetch/remote) / L3 毁灭 (拒绝即帧, 无工具面)。组合命令逐段分级取最严。

use std::path::Path;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, ToolGuard, ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::{
    authorized_git_write_policy, classify_git_write, git_write_capability,
    git_write_human_approval_floor, git_write_plan_for_arguments, GitWriteBoundaryGuard,
    GitWritePlan, GitWriteReleaseTier, GitWriteTier, GitWriteTierApprovalHook, GitWriteTool,
    GIT_WRITE_CAPABILITY_ID, GIT_WRITE_TOOL_DESCRIPTION, GIT_WRITE_TOOL_NAME, REFUSAL_L3_PREFIX,
    REFUSAL_MORPH_PREFIX, REFUSAL_OFFLIST_PREFIX,
};

/// 分级断言: 必须是 Graded 且分级等于期望。
fn tier_of(command: &str) -> GitWriteTier {
    match classify_git_write(command) {
        GitWritePlan::Graded { tier, .. } => tier,
        GitWritePlan::Refused { reason } => panic!("{command:?} 不应被拒: {reason}"),
    }
}

/// 拒绝断言: 必须是 Refused 且帧前缀等于期望。
fn refusal_prefix(command: &str) -> String {
    match classify_git_write(command) {
        GitWritePlan::Refused { reason } => reason
            .split(':')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
        GitWritePlan::Graded { tier, .. } => panic!("{command:?} 不应放行为 {tier:?}"),
    }
}

/// ① L1 词表矩阵: 逐命令 + 变体 (引号路径/附加消息值/组合短选项/长选项等值
/// 形态) 一律 L1 可逆。
#[test]
fn l1_word_table_grades_reversible_local_commands_and_variants() {
    for command in [
        "git add a.txt",
        "git add -A",
        "git add -u",
        "git add -- src/lib.rs",
        "git add \"my file.txt\"",
        "git add src/a.rs src/b.rs",
        "git commit -m \"hello world\"",
        "git commit -am msg",
        "git commit -m\"attached value\"",
        "git commit --message=hello",
        "git commit -m \"fix $(literal) stays literal\"",
        "git commit -a -m release",
        "git branch feature",
        "git branch feature HEAD~1",
        "git branch --track feature main",
        "git branch -t feature main",
        "git switch main",
        "git switch -c feature",
        "git switch --create feature",
        "git tag v1.0",
        "git tag v1.0 HEAD",
        "git tag -a v1.0 -m \"release\"",
        "git tag -m msg v1.0",
        "git tag --annotate --message=note v1.0",
    ] {
        assert_eq!(
            tier_of(command),
            GitWriteTier::ReversibleLocal,
            "{command} 必须是 L1 可逆"
        );
    }
}

/// ①附 L2 词表矩阵: 逐命令 + 变体一律 L2 出网。
#[test]
fn l2_word_table_grades_network_commands_and_variants() {
    for command in [
        "git push",
        "git push origin main",
        "git push -u origin main",
        "git push --tags",
        "git push origin main:main",
        "git fetch",
        "git fetch origin",
        "git fetch --all",
        "git fetch origin main",
        "git remote add origin https://host.example/repo.git",
        "git remote rename old new",
        "git remote set-url origin https://host.example/other.git",
        "git remote set-url --push origin https://host.example/other.git",
        "git remote update",
    ] {
        assert_eq!(
            tier_of(command),
            GitWriteTier::NetworkEgress,
            "{command} 必须是 L2 出网"
        );
    }
    // L2 地板判定 (预设不得越人) 同步覆盖 push/fetch/remote。
    for command in [
        "git push origin main",
        "git fetch origin",
        "git remote add origin https://host.example/x.git",
    ] {
        let arguments = serde_json::json!({ "command": command });
        assert!(
            git_write_human_approval_floor(&arguments),
            "{command} 必须命中 L2 人工审批地板"
        );
    }
    // L1 不踩地板。
    assert!(!git_write_human_approval_floor(
        &serde_json::json!({ "command": "git commit -m x" })
    ));
}

/// ② L3 拒绝即帧: 词表内的毁灭族 + 伪装尝试 (缩写变形 / 组合短选项 / refspec
/// 变形 / 引用删除族) 逐条给 L3 帧, 无工具面。
#[test]
fn destructive_and_disguised_forms_are_refused_with_the_l3_frame() {
    for command in [
        // reset / clean 族 (含变形: 缩写、加参数、组合短选项)。
        "git reset --hard",
        "git reset --hard HEAD~1",
        "git reset -q --hard",
        "git reset --ha",
        "git reset --hard=true",
        "git clean -f",
        "git clean -fdx",
        "git clean -f -d",
        // 改史族。
        "git rebase -i HEAD~2",
        "git rebase --onto a b c",
        "git commit --amend",
        "git commit --amend --no-edit",
        "git commit --am",
        "git commit -m x --amend",
        // 强推族 (含 refspec 变形与组合短选项)。
        "git push --force origin main",
        "git push -f origin main",
        "git push --force-with-lease",
        "git push --force-with-lease=refs/heads/main",
        "git push --for",
        "git push -qf origin main",
        "git push origin +main",
        "git push origin +refs/heads/main:refs/heads/main",
        "git push --mirror origin",
        // 引用删除族 (与 update-ref -d 同权)。
        "git push --delete origin main",
        "git push -d origin main",
        "git push origin :main",
        "git update-ref -d refs/heads/x",
        "git branch -d feature",
        "git branch -D feature",
        "git branch --delete feature",
        "git tag -d v1.0",
        "git tag --delete v1.0",
        "git tag -f v1.0",
        "git fetch --prune",
        "git fetch --prune-tags",
        "git remote remove origin",
        "git remote prune origin",
        // fetch 覆盖本地引用的 refspec 变形。
        "git fetch origin main:refs/heads/main",
        "git fetch origin main:",
    ] {
        assert_eq!(
            refusal_prefix(command),
            REFUSAL_L3_PREFIX,
            "{command} 必须给 L3 拒绝即帧"
        );
    }
}

/// ②附 词表外 (无工具面): 白名单之外的子命令/形态一律拒绝即帧。
#[test]
fn off_list_subcommands_have_no_tool_surface() {
    for command in [
        "git status",
        "git checkout main",
        "git checkout -- file.txt",
        "git restore .",
        "git stash drop",
        "git merge feature",
        "git cherry-pick abc123",
        "git config user.name someone",
        "git clone https://host.example/repo.git",
        "git some-alias",
        "git commit",
        "git commit -a",
        // 编辑器面 (承诺「本工具不开编辑器面」): 短形 `-e` 与长形 `--edit` 都要拒。
        // 此前 `e` 漏在拒绝表外 ⇒ 这四种形态被**静默放过**。
        "git commit -m x -e",
        "git commit -em x",
        "git commit -m x --edit",
        "git commit --edit -m x",
        "git branch",
        "git branch --list",
        "git tag",
        "echo hi",
    ] {
        assert_eq!(
            refusal_prefix(command),
            REFUSAL_OFFLIST_PREFIX,
            "{command} 必须给词表外拒绝即帧"
        );
    }
}

/// ③ 组合命令逐段分级取最严: 一段 L2 = 整次 L2; 一段 L3/词表外 = 整次拒绝即帧。
#[test]
fn combined_commands_take_the_strictest_segment() {
    assert_eq!(
        tier_of("git add a.txt && git commit -m x"),
        GitWriteTier::ReversibleLocal
    );
    assert_eq!(
        tier_of("git add a.txt && git push origin main"),
        GitWriteTier::NetworkEgress
    );
    assert_eq!(
        tier_of("git commit -m x ; git fetch origin"),
        GitWriteTier::NetworkEgress
    );
    assert_eq!(
        tier_of("git push origin main ; git add a.txt"),
        GitWriteTier::NetworkEgress
    );
    assert_eq!(
        tier_of("git add a.txt\ngit commit -m x"),
        GitWriteTier::ReversibleLocal
    );
    // 毁灭段把整次拖成拒绝即帧 (取最严)。
    assert_eq!(
        refusal_prefix("git add a.txt && git reset --hard"),
        REFUSAL_L3_PREFIX
    );
    assert_eq!(
        refusal_prefix("git commit -m x ; git clean -f"),
        REFUSAL_L3_PREFIX
    );
    // 管道/非 git 段同样整次拒绝。
    assert_eq!(
        refusal_prefix("git add a.txt | tee log.txt"),
        REFUSAL_MORPH_PREFIX
    );
    assert_eq!(
        refusal_prefix("git add a.txt && echo done"),
        REFUSAL_OFFLIST_PREFIX
    );
}

/// ⑤ 参数变体防绕过: 危险全局选项 (别名注入 / 执行路径劫持 / 仓库逃逸) 与
/// shell 面一律拒绝即帧 (变形面前缀)。
#[test]
fn dangerous_global_options_and_shell_shapes_are_refused() {
    for command in [
        "git -c alias.ci='!sh -c x' commit -m m",
        "git -c core.sshCommand=evil push origin main",
        "git --config-env=PATH=EVIL add a.txt",
        "git --exec-path=/tmp/evil add a.txt",
        "git --git-dir=/tmp/evil add a.txt",
        "git --work-tree=/tmp/evil add a.txt",
        "git --namespace=other add a.txt",
        "git -C /elsewhere commit -m x",
    ] {
        assert_eq!(
            refusal_prefix(command),
            REFUSAL_MORPH_PREFIX,
            "{command} 必须给变形面拒绝即帧"
        );
    }
    for command in [
        "git commit -m $(boom)",
        "git commit -m `boom`",
        "git add a.txt > out.txt",
        "git add a.txt >> out.txt",
        "git add a.txt < in.txt",
        "git add a.txt &",
        "git add a.txt && git commit -m x | tee log",
    ] {
        assert_eq!(
            refusal_prefix(command),
            REFUSAL_MORPH_PREFIX,
            "{command} 必须给变形面拒绝即帧"
        );
    }
    // 允许的全局面只有 --no-pager / --paginate。
    assert_eq!(
        tier_of("git --no-pager commit -m x"),
        GitWriteTier::ReversibleLocal
    );
    assert_eq!(
        tier_of("git --paginate commit -m x"),
        GitWriteTier::ReversibleLocal
    );
}

/// ⑦ 拒绝帧稳定前缀 + 工具描述写明 L3 无工具面 (防幻觉: 不给模型想象空间)。
#[test]
fn refusal_frames_are_stable_and_the_description_states_l3_has_no_tool_surface() {
    for prefix in [
        REFUSAL_L3_PREFIX,
        REFUSAL_OFFLIST_PREFIX,
        REFUSAL_MORPH_PREFIX,
    ] {
        assert!(prefix.contains("拒绝即帧"), "{prefix}");
        assert!(prefix.contains("无工具面"), "{prefix}");
    }
    for phrase in [
        "L1 reversible-local",
        "L2 network-egress",
        "L3",
        "no tool surface",
        "always",
        "requires a human decision",
    ] {
        assert!(
            GIT_WRITE_TOOL_DESCRIPTION.contains(phrase),
            "工具描述必须写明 {phrase:?}: {GIT_WRITE_TOOL_DESCRIPTION}"
        );
    }
    // 声明面名字与能力 id 名实相符。
    let tool = GitWriteTool::new(".");
    let declaration = tool.declaration();
    assert_eq!(declaration.name, GIT_WRITE_TOOL_NAME);
    assert_eq!(tool.id().as_str(), GIT_WRITE_CAPABILITY_ID);
    assert_eq!(
        declaration.parameters.get("required"),
        Some(&serde_json::json!(["command"])),
    );
}

// ---------------------------------------------------------------------------
// 真 git 执行面 (受控通道: 结构化 argv + 工作区限域 + 输出归一)
// ---------------------------------------------------------------------------

fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_git(root: &Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn init_repo(root: &Path) -> bool {
    run_git(root, &["init", "-q"])
        && run_git(root, &["config", "user.email", "d-p1@test.local"])
        && run_git(root, &["config", "user.name", "D-P1 Test"])
}

fn call(command: &str) -> ToolCall {
    ToolCall {
        id: "call_git_write".into(),
        name: GIT_WRITE_TOOL_NAME.into(),
        arguments: serde_json::json!({ "command": command }),
    }
}

/// 治理判定请求 (能力 id 与参数须由调用方持有, 借用期覆盖整个判定)。
fn dispatch_request<'a>(
    capability: &'a CapabilityId,
    arguments: &'a serde_json::Value,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
}

/// ⑧ L1 执行面: 分级放行后真落地 (add + commit 一段组合), 输出归一四字段。
#[tokio::test]
async fn l1_execution_lands_add_and_commit_through_the_controlled_channel() {
    if !git_available() {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(init_repo(dir.path()));
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();

    let tool = git_write_capability(dir.path(), authorized_git_write_policy());
    let result = tool
        .invoke(&call("git add a.txt && git commit -m \"first commit\""))
        .await;
    assert!(result.is_ok(), "{}", result.render());
    let value: serde_json::Value =
        serde_json::from_str(&result.render()).expect("structured report json");
    assert_eq!(value["status"], "executed");
    assert_eq!(value["tier"], "L1");
    assert!(
        value["output"]
            .as_str()
            .unwrap_or_default()
            .contains("first commit"),
        "{value}"
    );
    // 真落地: 仓库里真有提交。
    assert!(run_git(dir.path(), &["log", "-n", "1", "--oneline"]));
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&status.stdout).trim().is_empty(),
        "提交后工作区应干净"
    );
}

/// ⑨ 原子性如实语义: 中段失败**不谎称回滚** —— 错误帧逐段报告已落地段,
/// staged 残留照实留存 (git 语义, 非假原子)。
#[tokio::test]
async fn failed_segment_reports_landed_segments_without_rollback_claim() {
    if !git_available() {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(init_repo(dir.path()));
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();

    let tool = git_write_capability(dir.path(), authorized_git_write_policy());
    // 第 2 段 (commit 空消息) 必然失败; 第 1 段 (add) 已落地。
    let result = tool
        .invoke(&call("git add a.txt && git commit -m \"\""))
        .await;
    assert!(!result.is_ok(), "失败必须是错误帧");
    let rendered = result.render();
    assert!(rendered.contains("如实上报"), "{rendered}");
    assert!(rendered.contains("无回滚承诺"), "{rendered}");
    assert!(rendered.contains("第 2/2 段失败"), "{rendered}");
    assert!(rendered.contains("git add a.txt"), "{rendered}");
    assert!(!rendered.contains("已回滚"), "{rendered}");

    // 如实语义钉死: staged 残留仍在 (不假装"没发生过")。
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status.stdout);
    assert!(
        status_text.contains("A  a.txt"),
        "add 已落地的 staged 残留必须照实留存: {status_text}"
    );
}

/// ⑩ 脱敏: remote URL 凭据不入日志/审批面 (命令面、判定理由、执行输出全查)。
#[tokio::test]
async fn remote_url_credentials_are_redacted_in_frames_plans_and_decisions() {
    let command = "git remote add origin https://user:tok123456@host.example/repo.git";
    let arguments = serde_json::json!({ "command": command });

    // 分级计划的命令面已脱敏。
    let plan = git_write_plan_for_arguments(&arguments);
    let GitWritePlan::Graded { segments, .. } = &plan else {
        panic!("remote add 应可分级: {plan:?}");
    };
    for segment in segments {
        assert!(
            !segment.display.contains("tok123456"),
            "{}",
            segment.display
        );
        assert!(!segment.display.contains("user:"), "{}", segment.display);
        assert!(
            segment.display.contains("[REDACTED_URL_CREDENTIAL]"),
            "{}",
            segment.display
        );
    }

    // 审批理由 (进本地面板 / IM 审批卡) 同样脱敏, 且带分级标注。
    let hook = GitWriteTierApprovalHook::new(GitWriteReleaseTier::RequireApprovalEveryWrite);
    let capability = CapabilityId::new(GIT_WRITE_CAPABILITY_ID).unwrap();
    let request = dispatch_request(&capability, &arguments);
    let decision = hook.evaluate(&request).await;
    let Decision::RequireApproval { reason } = decision else {
        panic!("L2 必须停人工审批: {decision:?}");
    };
    assert!(reason.contains("L2"), "{reason}");
    assert!(!reason.contains("tok123456"), "{reason}");
    assert!(!reason.contains("user:"), "{reason}");

    // 执行输出 (git 自己回显 remote 配置时) 也脱敏。
    if git_available() {
        let dir = tempfile::tempdir().unwrap();
        assert!(init_repo(dir.path()));
        let tool = git_write_capability(dir.path(), authorized_git_write_policy());
        let result = tool.invoke(&call(command)).await;
        assert!(result.is_ok(), "{}", result.render());
        let rendered = result.render();
        assert!(!rendered.contains("tok123456"), "{rendered}");
        assert!(!rendered.contains("user:"), "{rendered}");
    }
}

/// ⑪ L1 三档语义 (钩子级): 默认档 L1 审批 / 「自动放行 L1」档 L1 放行;
/// L2 无论哪档都审批; L3/词表外拒绝即帧 (不进审批)。
#[tokio::test]
async fn tier_approval_hook_maps_the_three_release_semantics() {
    let default_tier =
        GitWriteTierApprovalHook::new(GitWriteReleaseTier::RequireApprovalEveryWrite);
    let auto_pass_tier = GitWriteTierApprovalHook::new(GitWriteReleaseTier::AutoPassReversibleL1);
    let capability = CapabilityId::new(GIT_WRITE_CAPABILITY_ID).unwrap();

    // 默认档: L1 / L2 都审批 (L1 理由带 L1 分级标注)。
    let arguments = serde_json::json!({ "command": "git commit -m x" });
    let decision = default_tier
        .evaluate(&dispatch_request(&capability, &arguments))
        .await;
    let Decision::RequireApproval { reason } = &decision else {
        panic!("默认档 L1 必须审批: {decision:?}");
    };
    assert!(reason.contains("L1"), "{reason}");

    // 自动放行档: L1 放行。
    let decision = auto_pass_tier
        .evaluate(&dispatch_request(&capability, &arguments))
        .await;
    assert!(
        matches!(decision, Decision::Allow),
        "自动放行 L1 档的 L1 必须放行: {decision:?}"
    );

    // L2: 两档都审批 (永远人工审批)。
    for hook in [&default_tier, &auto_pass_tier] {
        let arguments = serde_json::json!({ "command": "git push origin main" });
        let decision = hook
            .evaluate(&dispatch_request(&capability, &arguments))
            .await;
        let Decision::RequireApproval { reason } = &decision else {
            panic!("L2 永远人工审批: {decision:?}");
        };
        assert!(reason.contains("L2"), "{reason}");
        assert!(reason.contains("never auto-passes"), "{reason}");
    }

    // L3/词表外: 拒绝即帧, 不进审批。
    for command in ["git reset --hard", "git checkout main"] {
        let arguments = serde_json::json!({ "command": command });
        let decision = auto_pass_tier
            .evaluate(&dispatch_request(&capability, &arguments))
            .await;
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "{command} 必须拒绝即帧: {decision:?}"
        );
    }

    // 非本能力的动作不干预 (钩子不越权)。
    let other = CapabilityId::new("tool.search").unwrap();
    let arguments = serde_json::json!({});
    assert!(matches!(
        default_tier
            .evaluate(&dispatch_request(&other, &arguments))
            .await,
        Decision::Allow
    ));
}

/// ⑫ 单调边界 Guard: L3/词表外/变形面拒绝, 且任何阶段不得翻回放行 (只拒不放)。
#[test]
fn boundary_guard_refuses_refused_plans_and_passes_graded_ones() {
    let guard = GitWriteBoundaryGuard::new();
    let capability = CapabilityId::new(GIT_WRITE_CAPABILITY_ID).unwrap();
    let other = CapabilityId::new("tool.apply_patch").unwrap();

    let refused_arguments = serde_json::json!({ "command": "git reset --hard" });
    let request = ToolGuardRequest {
        capability: &capability,
        tool_name: GIT_WRITE_TOOL_NAME,
        arguments: &refused_arguments,
    };
    let denial = guard.deny(&request).expect("L3 必须被边界 Guard 拒");
    assert!(denial.contains(REFUSAL_L3_PREFIX), "{denial}");

    for command in ["git add a.txt", "git push origin main"] {
        let arguments = serde_json::json!({ "command": command });
        let request = ToolGuardRequest {
            capability: &capability,
            tool_name: GIT_WRITE_TOOL_NAME,
            arguments: &arguments,
        };
        assert!(
            guard.deny(&request).is_none(),
            "{command} 是词表内分级命令, 边界 Guard 不拦 (审批由治理层管)"
        );
    }

    // 非本能力不干预。
    let arguments = serde_json::json!({ "patch": "*** Begin Patch" });
    let request = ToolGuardRequest {
        capability: &other,
        tool_name: "apply_patch",
        arguments: &arguments,
    };
    assert!(guard.deny(&request).is_none());
}

/// 工具面拒绝即帧: L3 变形尝试直接回错误帧, 仓库零变化 (不给幻觉空间)。
#[tokio::test]
async fn destructive_invocations_never_touch_the_repository() {
    if !git_available() {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(init_repo(dir.path()));
    std::fs::write(dir.path().join("a.txt"), "v1").unwrap();
    assert!(run_git(dir.path(), &["add", "a.txt"]));
    assert!(run_git(dir.path(), &["commit", "-q", "-m", "base"]));

    let tool: Arc<dyn ToolCapability> =
        git_write_capability(dir.path(), authorized_git_write_policy());
    for command in [
        "git reset --hard",
        "git clean -f",
        "git commit --amend",
        "git push --force origin main",
    ] {
        let result = tool.invoke(&call(command)).await;
        assert!(!result.is_ok(), "{command} 必须拒绝即帧");
        assert!(result.render().contains("拒绝即帧"), "{}", result.render());
    }
    // 仓库零变化: HEAD 仍在 base, 文件仍在。
    let log = std::process::Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["log", "-n", "1", "--pretty=%s"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&log.stdout).trim(), "base");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "v1"
    );
}

/// 超时归 `timeout.*` 族 / 空命令拒绝即帧 (合同边界补钉)。
#[tokio::test]
async fn empty_commands_are_refused_and_timeout_codes_stay_in_the_timeout_family() {
    let tool = git_write_capability(".", authorized_git_write_policy());
    let result = tool.invoke(&call("")).await;
    assert!(!result.is_ok());
    assert!(
        result.render().contains(REFUSAL_OFFLIST_PREFIX),
        "{}",
        result.render()
    );
    // 超时族 code 语义 (核心库口径): 段执行超时的帧前缀是 timeout.* 家族。
    assert!(
        apeireth_core::deadline::TimeoutErrorCode::DeadlineExpired
            .as_str()
            .starts_with("timeout."),
        "超时帧必须归 timeout.* 族"
    );
}

/// 空动作不构成谎帧: 既不静默成功, 也不给无据的成功报告 (错误即帧)。
#[tokio::test]
async fn unparseable_parameters_are_error_frames() {
    let tool = git_write_capability(".", authorized_git_write_policy());
    let call = ToolCall {
        id: "call_bad".into(),
        name: GIT_WRITE_TOOL_NAME.into(),
        arguments: serde_json::json!({ "command": "git add a.txt", "extra": 1 }),
    };
    let result = tool.invoke(&call).await;
    assert!(!result.is_ok(), "未知参数必须错误帧");
    assert!(
        result.render().contains("invalid git_write parameters"),
        "{}",
        result.render()
    );
}

/// 输出归一合同: 成功报告形状冻结 (status/tier/commands/output 四字段)。
#[tokio::test]
async fn success_report_shape_is_frozen_by_the_output_contract() {
    if !git_available() {
        eprintln!("git is not installed; skipping git_write execution test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(init_repo(dir.path()));
    std::fs::write(dir.path().join("b.txt"), "hello").unwrap();

    let tool = git_write_capability(dir.path(), authorized_git_write_policy());
    let result = tool.invoke(&call("git add b.txt")).await;
    assert!(result.is_ok(), "{}", result.render());
    let value: serde_json::Value =
        serde_json::from_str(&result.render()).expect("structured report json");
    for field in ["status", "tier", "commands", "output"] {
        assert!(value.get(field).is_some(), "报告必须有 {field}: {value}");
    }
    assert_eq!(value["status"], "executed");
    assert_eq!(value["tier"], "L1");
    assert_eq!(value["commands"], serde_json::json!(["git add b.txt"]));
    // 归一形状之外无多余字段 (合同冻结)。
    assert_eq!(value.as_object().map(|map| map.len()), Some(4), "{value}");
}
