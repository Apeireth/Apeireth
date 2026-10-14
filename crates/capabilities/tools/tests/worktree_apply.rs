//! 干活能力轴 P2 (`tool.worktree_plan` / `tool.worktree_apply`) 的四拍受控写
//! 行为回归。
//!
//! 覆盖: 计划拍产改单 / 计划零落盘 / 计划与补丁交叉一致 / apply 按改单执行
//! + 任一失败整批回滚 / 观测门禁 (未读不改) / 审批级语义 (read_only 拒 ·
//! standard 审批 · full 按预设 —— 预设档在组装根测试) / 审批卡载荷 (改单
//! 摘要) / 验证拍失败即回滚 / 与 git_write 分工文案 / 工作区外拒 / 敏感面拒。

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, PermissionPolicy, ToolGuard,
    ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::{
    authorized_worktree_apply_policy, worktree_apply_capability, worktree_plan_capability,
    FileVersion, ObservedGate, PreExecuteHook, PreExecuteRequest, PreVerdict, VerifyCommand,
    WorktreeApplyApprovalHook, WorktreeApplyBoundaryGuard, WorktreePlanBoundaryGuard,
    WorktreePlanTool, REFUSAL_DRIFT_PREFIX, REFUSAL_VERIFY_PREFIX, WORKTREE_APPLY_CAPABILITY_ID,
    WORKTREE_PLAN_TOOL_NAME,
};

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

fn call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call-1".to_string(),
        name: name.to_string(),
        arguments,
    }
}

fn expect_value(result: &ToolResult) -> serde_json::Value {
    match &result.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        ToolOutcome::Error { message, .. } => panic!("期望成功却得错误帧: {message}"),
    }
}

fn expect_error(result: &ToolResult) -> String {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("期望错误帧却得成功: {value}"),
    }
}

/// 改单参数 (`worktree_plan` 参数面 / `worktree_apply` 的 `plan` 面同形)。
fn order_args(goal: &str, files: &[(&str, &str, &str)]) -> serde_json::Value {
    serde_json::json!({
        "goal": goal,
        "files": files
            .iter()
            .map(|(path, action, expect)| serde_json::json!({
                "path": path,
                "action": action,
                "expect": expect,
            }))
            .collect::<Vec<_>>(),
    })
}

/// apply 参数 (改单 + 补丁 + 校验命令)。
fn apply_args(
    goal: &str,
    files: &[(&str, &str, &str)],
    patch: &str,
    verify: &[(&str, Vec<&str>)],
) -> serde_json::Value {
    serde_json::json!({
        "plan": order_args(goal, files),
        "patch": patch,
        "verify": verify
            .iter()
            .map(|(program, args)| serde_json::json!({
                "program": program,
                "args": args,
            }))
            .collect::<Vec<_>>(),
    })
}

/// 模拟会话内「文件读」事件 (读工具记观测, 写侧认它为已读证据)。
fn observe_read(gate: &ObservedGate, path: &Path) {
    let meta = std::fs::metadata(path).expect("metadata");
    gate.observe_present(path, FileVersion::from_metadata(&meta));
}

/// 工作区内容快照 (相对路径 → 内容)。
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .expect("relative")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read_to_string(&path).expect("read file"),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn governance_request<'a>(
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

// ---------------------------------------------------------------------------
// 计划拍: 产改单 / 零落盘 / 拒绝面
// ---------------------------------------------------------------------------

/// ① 计划拍产改单: 文件列表 + 每文件动作 + 预期 + 计数 + 动作分布 + 稳定
/// plan_id (同改单两次产出同 id, 供审批卡/审计配对)。
#[tokio::test]
async fn plan_produces_the_change_order_with_a_stable_plan_id() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());
    let arguments = order_args(
        "多文件改: 更新 a, 删除 b, 新建 c",
        &[
            ("a.txt", "update", "内容更新为新语义"),
            ("b.txt", "delete", "移除废弃件"),
            ("c.txt", "add", "新建承接件"),
        ],
    );

    let first = expect_value(
        &tool
            .invoke(&call(WORKTREE_PLAN_TOOL_NAME, arguments.clone()))
            .await,
    );
    assert_eq!(first["status"], "planned");
    assert_eq!(first["file_count"], 3);
    assert_eq!(
        first["action_distribution"],
        serde_json::json!({ "add": 1, "update": 1, "delete": 1 })
    );
    let plan_id = first["plan_id"].as_str().expect("plan_id 字符串");
    assert_eq!(plan_id.len(), 16, "plan_id 是 FNV-1a 64 hex: {plan_id}");
    assert_eq!(first["files"][0]["path"], "a.txt");
    assert_eq!(first["files"][0]["action"], "update");
    assert_eq!(first["files"][0]["expect"], "内容更新为新语义");

    let second = expect_value(&tool.invoke(&call(WORKTREE_PLAN_TOOL_NAME, arguments)).await);
    assert_eq!(second["plan_id"], first["plan_id"], "同改单必须同 id");
    assert_ne!(
        second["plan_id"],
        expect_value(
            &tool
                .invoke(&call(
                    WORKTREE_PLAN_TOOL_NAME,
                    order_args("换目标", &[("a.txt", "update", "内容更新为新语义")])
                ))
                .await
        )["plan_id"],
        "改单声明变了 id 必须变"
    );
}

/// ①附 计划零落盘: 改单里声明了新建/修改/删除, 工作区逐字节纹丝不动。
#[tokio::test]
async fn plan_never_touches_the_workspace() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "kept a").expect("seed a");
    std::fs::create_dir_all(workspace.path().join("dir")).expect("seed dir");
    std::fs::write(workspace.path().join("dir/b.txt"), "kept b").expect("seed b");
    let before = snapshot(workspace.path());

    let tool = WorktreePlanTool::new(workspace.path());
    let arguments = order_args(
        "只产改单, 不落盘",
        &[
            ("a.txt", "update", "改"),
            ("dir/b.txt", "delete", "删"),
            ("new.txt", "add", "建"),
        ],
    );
    expect_value(&tool.invoke(&call(WORKTREE_PLAN_TOOL_NAME, arguments)).await);

    assert_eq!(
        snapshot(workspace.path()),
        before,
        "计划拍必须零落盘: 任何文件的增删改都算破戒"
    );
}

/// ② 工作区外拒 / 敏感面拒 / 病态改单拒 (计划拍, 零落盘)。
#[tokio::test]
async fn plan_refuses_out_of_workspace_sensitive_and_ill_formed_targets() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = WorktreePlanTool::new(workspace.path());

    // 工作区外: 绝对路径 / `..` 组件。
    let absolute = std::env::temp_dir()
        .join("evil.txt")
        .to_string_lossy()
        .into_owned();
    for (arguments, needle) in [
        (
            order_args("越界", &[(absolute.as_str(), "update", "改")]),
            "禁止绝对路径",
        ),
        (
            order_args("越界", &[("../escape.txt", "update", "改")]),
            "禁止的路径组件",
        ),
        // 敏感面: 凭据/密钥面 (凭据面不可读)。
        (
            order_args("敏感", &[(".env", "update", "改")]),
            "credential surface",
        ),
        (
            order_args("敏感", &[("keys/id_rsa", "delete", "删")]),
            "credential surface",
        ),
        // 病态改单: 重复目标 / 空预期 / 空文件集 / 空目标。
        (
            order_args(
                "重复",
                &[("a.txt", "update", "改"), ("a.txt", "delete", "删")],
            ),
            "重复声明",
        ),
        (
            order_args("空预期", &[("a.txt", "update", "  ")]),
            "预期结果",
        ),
        (order_args("空集", &[]), "至少声明一个文件"),
        (order_args("", &[("a.txt", "update", "改")]), "任务目标"),
    ] {
        let message = expect_error(&tool.invoke(&call(WORKTREE_PLAN_TOOL_NAME, arguments)).await);
        assert!(
            message.contains(needle),
            "拒绝帧必须命中 {needle:?}: {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// 改拍/落拍: 按改单执行 / 原子回滚 / 观测门禁
// ---------------------------------------------------------------------------

/// ③ apply 按改单执行全四拍: 改单与补丁一致 → 暂存面执行 → 验证拍
/// (结构校验 + 受控通道校验命令) → 落拍; 报告冻结形状 + plan_id 与计划拍
/// 同文配对。
#[tokio::test]
async fn apply_lands_the_change_order_through_all_four_beats() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "old line\nkept").expect("seed a");
    std::fs::write(workspace.path().join("b.txt"), "to delete").expect("seed b");

    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    observe_read(&gate, &workspace.path().join("b.txt"));

    let files = [
        ("a.txt", "update", "旧句换新句"),
        ("b.txt", "delete", "删除"),
        ("c.txt", "add", "新建"),
    ];
    let patch = "*** Begin Patch\n\
                 *** Update File: a.txt\n\
                 @@\n\
                 -old line\n\
                 +new line\n\
                 *** Add File: c.txt\n\
                 +created body\n\
                 *** Delete File: b.txt\n\
                 *** End Patch";
    let arguments = apply_args("四拍演示", &files, patch, &[("cargo", vec!["--version"])]);

    // plan_id 配对: 计划拍产物与落盘报告同文。
    let plan_value = expect_value(
        &WorktreePlanTool::new(workspace.path())
            .invoke(&call(
                WORKTREE_PLAN_TOOL_NAME,
                order_args("四拍演示", &files),
            ))
            .await,
    );

    let tool =
        apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));
    let value = expect_value(&tool.invoke(&call("worktree_apply", arguments)).await);

    assert_eq!(value["status"], "applied");
    assert_eq!(
        value["plan_id"], plan_value["plan_id"],
        "审批/审计配对 id 同文"
    );
    assert_eq!(value["file_count"], 3);
    assert_eq!(
        value["action_distribution"],
        serde_json::json!({ "add": 1, "update": 1, "delete": 1 })
    );
    assert_eq!(value["files_added"], serde_json::json!(["c.txt"]));
    assert_eq!(value["files_updated"], serde_json::json!(["a.txt"]));
    assert_eq!(value["files_deleted"], serde_json::json!(["b.txt"]));
    let verification = value["verification"].as_array().expect("verification 数组");
    assert_eq!(
        verification.len(),
        2,
        "结构校验 + 校验命令各一条: {verification:?}"
    );
    assert!(
        verification[0]
            .as_str()
            .expect("首条")
            .contains("结构校验通过"),
        "{verification:?}"
    );
    assert!(
        verification[1]
            .as_str()
            .expect("次条")
            .starts_with("$ cargo --version"),
        "{verification:?}"
    );

    // 工作区终态与改单预期相符。
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "new line\nkept"
    );
    assert!(!workspace.path().join("b.txt").exists());
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("c.txt")).unwrap(),
        "created body"
    );
}

/// ③附 观测门禁 (未读不改): 会话从未读过的目标不得覆盖/删除 —— 计划拍即拒,
/// 工作区零写入; 读过之后同一改单放行。
#[tokio::test]
async fn apply_refuses_untouched_targets_until_they_were_read() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    std::fs::write(workspace.path().join("b.txt"), "two").expect("seed b");
    let gate = Arc::new(ObservedGate::new());
    let tool =
        apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));

    // 未读的修改 → 拒 (未读不改)。
    let message = expect_error(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args(
                    "未读改",
                    &[("a.txt", "update", "改")],
                    "*** Begin Patch\n*** Update File: a.txt\n@@\n-one\n+uno\n*** End Patch",
                    &[],
                ),
            ))
            .await,
    );
    assert!(message.contains("从未被读取观测"), "{message}");

    // 未读的删除 → 拒 (同一道门禁)。
    let message = expect_error(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args(
                    "未读删",
                    &[("b.txt", "delete", "删")],
                    "*** Begin Patch\n*** Delete File: b.txt\n*** End Patch",
                    &[],
                ),
            ))
            .await,
    );
    assert!(message.contains("从未被读取观测"), "{message}");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "one"
    );
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("b.txt")).unwrap(),
        "two"
    );

    // 会话读过 (观测事件) 之后, 同一改单放行。
    observe_read(&gate, &workspace.path().join("a.txt"));
    observe_read(&gate, &workspace.path().join("b.txt"));
    expect_value(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args(
                    "已读",
                    &[("a.txt", "update", "改"), ("b.txt", "delete", "删")],
                    "*** Begin Patch\n*** Update File: a.txt\n@@\n-one\n+uno\n*** Delete File: b.txt\n*** End Patch",
                    &[],
                ),
            ))
            .await,
    );
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "uno"
    );
    assert!(!workspace.path().join("b.txt").exists());
}

/// ④ 计划拍防漂移: 改单与补丁不一致 (补丁多动 / 改单多列 / 动作不符 /
/// 补丁重复目标) 一律拒绝即帧, 零写入。
#[tokio::test]
async fn apply_refuses_patch_drift_against_the_change_order() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    let tool =
        apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));

    let update_a = "*** Update File: a.txt\n@@\n-one\n+uno\n";
    let drift_cases: [(serde_json::Value, &str); 4] = [
        // 补丁动了改单之外的目标。
        (
            apply_args(
                "多动",
                &[("a.txt", "update", "改")],
                &format!("*** Begin Patch\n{update_a}*** Add File: sneak.txt\n+x\n*** End Patch"),
                &[],
            ),
            "补丁声明了改单之外的目标",
        ),
        // 改单列了补丁未动的目标。
        (
            apply_args(
                "多列",
                &[("a.txt", "update", "改"), ("ghost.txt", "add", "建")],
                &format!("*** Begin Patch\n{update_a}*** End Patch"),
                &[],
            ),
            "改单声明了补丁未动的目标",
        ),
        // 动作标签不符。
        (
            apply_args(
                "动作不符",
                &[("a.txt", "add", "建")],
                &format!("*** Begin Patch\n{update_a}*** End Patch"),
                &[],
            ),
            "动作不一致",
        ),
        // 补丁内同一目标重复声明。
        (
            apply_args(
                "重复声明",
                &[("a.txt", "update", "改")],
                &format!("*** Begin Patch\n{update_a}*** Delete File: a.txt\n*** End Patch"),
                &[],
            ),
            "补丁重复声明同一目标",
        ),
    ];
    for (arguments, needle) in drift_cases {
        let message = expect_error(&tool.invoke(&call("worktree_apply", arguments)).await);
        assert!(
            message.contains(REFUSAL_DRIFT_PREFIX),
            "必须是改单漂移拒绝帧: {message}"
        );
        assert!(message.contains(needle), "拒绝帧缺 {needle:?}: {message}");
    }
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "one",
        "计划拍拒绝 = 零写入"
    );
}

/// ⑤ 原子回滚 (任一失败整批回滚): 改拍失败与落拍失败都必须让工作区保持
/// 逐字节原样 —— 批次里成功的一半也不许留。
#[tokio::test]
async fn any_beat_failure_leaves_the_batch_fully_rolled_back() {
    // (a) 改拍失败: 第二个文件的 hunk 上下文不匹配 → 暂存面即爆, 零写入。
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    std::fs::write(workspace.path().join("b.txt"), "two").expect("seed b");
    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    observe_read(&gate, &workspace.path().join("b.txt"));
    let tool =
        apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));
    let message = expect_error(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args(
                    "半批坏 hunk",
                    &[("a.txt", "update", "改"), ("b.txt", "update", "改")],
                    "*** Begin Patch\n\
                     *** Update File: a.txt\n@@\n-one\n+uno\n\
                     *** Update File: b.txt\n@@\n-no-such-context\n+whatever\n\
                     *** End Patch",
                    &[],
                ),
            ))
            .await,
    );
    assert!(message.contains("改拍失败"), "{message}");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "one"
    );
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("b.txt")).unwrap(),
        "two"
    );

    // (b) 落拍失败: 暂存面全绿, 但落拍撞上真面现实 (Add 的父级在真面是文件)
    // → 整批回滚, 先落地的修改也不许留。
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    std::fs::write(workspace.path().join("sub"), "blocker").expect("seed sub file");
    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    let tool =
        apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));
    let message = expect_error(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args(
                    "落拍撞现实",
                    &[("a.txt", "update", "改"), ("sub/child.txt", "add", "建")],
                    "*** Begin Patch\n\
                     *** Update File: a.txt\n@@\n-one\n+uno\n\
                     *** Add File: sub/child.txt\n+child\n\
                     *** End Patch",
                    &[],
                ),
            ))
            .await,
    );
    assert!(message.contains("落拍失败"), "{message}");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "one",
        "任一失败整批回滚: 已落地的修改也必须恢复"
    );
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("sub")).unwrap(),
        "blocker"
    );
    assert!(!workspace.path().join("sub").join("child.txt").exists());
}

/// ⑤附 落拍中途失败的确定性回滚: 写批先落、删除批后走; 删除批撞现实失败时,
/// 先落地的修改必须被事务回滚恢复 (Windows: 占句柄不给 FILE_SHARE_DELETE,
/// 删除必撞共享冲突; 读共享照给, 改拍/预演读取不受影响)。
#[cfg(windows)]
#[tokio::test]
async fn landing_mid_commit_failure_rolls_back_already_committed_files() {
    use std::os::windows::fs::OpenOptionsExt;

    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
    let protected = workspace.path().join("b.txt");
    std::fs::write(&protected, "two").expect("seed b");
    // FILE_SHARE_READ | FILE_SHARE_WRITE, 不含 FILE_SHARE_DELETE:
    // 读得进 (物化/预演), 删不掉 (删除批必失败)。
    let holder = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1 | 0x2)
        .open(&protected)
        .expect("hold b.txt open without delete share");
    let gate = Arc::new(ObservedGate::new());
    observe_read(&gate, &workspace.path().join("a.txt"));
    observe_read(&gate, &protected);
    let tool =
        apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));
    let message = expect_error(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args(
                    "删除撞共享冲突",
                    &[("a.txt", "update", "改"), ("b.txt", "delete", "删")],
                    "*** Begin Patch\n\
                     *** Update File: a.txt\n@@\n-one\n+uno\n\
                     *** Delete File: b.txt\n\
                     *** End Patch",
                    &[],
                ),
            ))
            .await,
    );
    assert!(message.contains("落拍失败"), "{message}");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
        "one",
        "a.txt 已过一次落盘, 必须被事务回滚恢复"
    );
    assert!(protected.exists(), "b.txt 删除失败后仍在");
    assert_eq!(std::fs::read_to_string(&protected).unwrap(), "two");
    drop(holder);
}

// ---------------------------------------------------------------------------
// 验证拍
// ---------------------------------------------------------------------------

/// ⑥ 验证拍失败即回滚: 校验命令非零退出 / 无法启动, 都必须整批未落地。
#[tokio::test]
async fn verification_failure_aborts_before_landing() {
    for verify in [
        // 受控通道真实执行, 非零退出。
        vec![("cargo", vec!["definitely-not-a-real-subcommand-apeireth"])],
        // 无法启动 (fail-closed: 校验跑不起来 ≠ 校验通过)。
        vec![("definitely-not-a-real-program-apeireth", vec![])],
    ] {
        let workspace = tempfile::tempdir().expect("temp workspace");
        std::fs::write(workspace.path().join("a.txt"), "one").expect("seed a");
        let gate = Arc::new(ObservedGate::new());
        observe_read(&gate, &workspace.path().join("a.txt"));
        let tool =
            apeireth_tools_canonical::WorktreeApplyTool::new(workspace.path(), Arc::clone(&gate));
        let message = expect_error(
            &tool
                .invoke(&call(
                    "worktree_apply",
                    apply_args(
                        "验证不过",
                        &[("a.txt", "update", "改"), ("c.txt", "add", "建")],
                        "*** Begin Patch\n\
                         *** Update File: a.txt\n@@\n-one\n+uno\n\
                         *** Add File: c.txt\n+created\n\
                         *** End Patch",
                        &verify,
                    ),
                ))
                .await,
        );
        assert!(
            message.contains(REFUSAL_VERIFY_PREFIX) && message.contains("整批未落地"),
            "验证拍失败帧: {message}"
        );
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("a.txt")).unwrap(),
            "one",
            "验证不过 = 整批未落地"
        );
        assert!(!workspace.path().join("c.txt").exists());
    }
}

// ---------------------------------------------------------------------------
// 治理: 审批级语义 / 审批卡载荷 / 五段流水线
// ---------------------------------------------------------------------------

/// ⑦ 审批卡载荷 = 改单摘要 (文件数 / 动作分布 / 验证命令), 每次落盘都停
/// 人工审批; 计划拍校验不过的调用拒绝即帧, 不进审批。
#[tokio::test]
async fn approval_payload_carries_the_change_order_summary() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let hook = WorktreeApplyApprovalHook::new(workspace.path());
    let capability = CapabilityId::new(WORKTREE_APPLY_CAPABILITY_ID).unwrap();
    let arguments = apply_args(
        "审批摘要演示",
        &[
            ("a.txt", "update", "改"),
            ("b.txt", "delete", "删"),
            ("c.txt", "add", "建"),
        ],
        "*** Begin Patch\n*** Update File: a.txt\n@@\n-one\n+uno\n*** Delete File: b.txt\n*** Add File: c.txt\n+created\n*** End Patch",
        &[("cargo", vec!["--version"])],
    );

    let decision = hook
        .evaluate(&governance_request(&capability, &arguments))
        .await;
    let Decision::RequireApproval { reason } = &decision else {
        panic!("apply 是 RequireApproval 级, 每次都要人批: {decision:?}");
    };
    assert!(reason.contains("文件 3"), "审批卡载荷缺文件数: {reason}");
    assert!(
        reason.contains("add 1/update 1/delete 1"),
        "审批卡载荷缺动作分布: {reason}"
    );
    assert!(
        reason.contains("验证命令 1 条") && reason.contains("cargo --version"),
        "审批卡载荷缺验证命令: {reason}"
    );
    assert!(
        reason.contains("每次都要人批"),
        "审批卡载荷缺档位声明: {reason}"
    );

    // 计划拍校验不过 (漂移) → 拒绝即帧, 不造审批件。
    let drifted = apply_args(
        "漂移",
        &[("a.txt", "update", "改")],
        "*** Begin Patch\n*** Add File: sneak.txt\n+x\n*** End Patch",
        &[],
    );
    let decision = hook
        .evaluate(&governance_request(&capability, &drifted))
        .await;
    let Decision::Deny { reason } = &decision else {
        panic!("漂移调用必须拒绝即帧: {decision:?}");
    };
    assert!(reason.contains(REFUSAL_DRIFT_PREFIX), "{reason}");
}

/// ⑦附 审批级语义 (工具层): 未显式授权停在 require-approval 档
/// (`pipeline.pre_ask` 即帧不落盘); 授权后放行。只读改单面零审批。
#[tokio::test]
async fn apply_sits_at_the_require_approval_level_while_plan_needs_none() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let arguments = apply_args(
        "审批档",
        &[("n.txt", "add", "建")],
        "*** Begin Patch\n*** Add File: n.txt\n+created\n*** End Patch",
        &[],
    );
    let pending_call = call("worktree_apply", arguments.clone());
    let capability = CapabilityId::new(WORKTREE_APPLY_CAPABILITY_ID).unwrap();

    // 授权映射 (pre-execute 瀑布): 无授权 → ask; 有授权 → allow。
    let ungranted = Arc::new(Mutex::new(PermissionPolicy::new()));
    let hook = apeireth_tools_canonical::WorktreeApplyRiskMappingHook::new(ungranted.clone());
    match hook.pre_verdict(&PreExecuteRequest::new(&pending_call, &capability)) {
        PreVerdict::Ask { .. } => {}
        other => panic!("未授权必须停 require-approval 档: {other:?}"),
    }
    let granted = authorized_worktree_apply_policy();
    let hook = apeireth_tools_canonical::WorktreeApplyRiskMappingHook::new(granted);
    match hook.pre_verdict(&PreExecuteRequest::new(&pending_call, &capability)) {
        PreVerdict::Allow => {}
        other => panic!("显式授权后放行: {other:?}"),
    }

    // 五段流水线端到端: 无授权的 apply 停在 pre_ask 即帧, 零写入。
    let gate = Arc::new(ObservedGate::new());
    let tool = worktree_apply_capability(
        workspace.path(),
        gate,
        Arc::new(Mutex::new(PermissionPolicy::new())),
    );
    let result = tool.invoke(&call("worktree_apply", arguments)).await;
    assert!(!result.is_ok(), "无授权不得落盘");
    let rendered = result.render();
    assert!(rendered.contains("pipeline.pre_ask"), "{rendered}");
    assert!(!workspace.path().join("n.txt").exists());

    // 计划拍同一条流水线, 但零审批: 未授权也照常产改单。
    let plan = worktree_plan_capability(workspace.path());
    let value = expect_value(
        &plan
            .invoke(&call(
                WORKTREE_PLAN_TOOL_NAME,
                order_args("零审批", &[("n.txt", "add", "建")]),
            ))
            .await,
    );
    assert_eq!(value["status"], "planned");
}

// ---------------------------------------------------------------------------
// 边界 Guard (单调, 只拒不放) / 分工文案 / 参数面
// ---------------------------------------------------------------------------

/// ⑧ 单调边界 Guard: 工作区外 / 敏感面 / 改单漂移一律拒, 且拒绝后无任何
/// 阶段能翻回放行 (Guard 只有拒, 没有放行出口)。
#[test]
fn boundary_guards_refuse_out_of_workspace_sensitive_and_drifted_targets() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let gate = Arc::new(ObservedGate::new());
    let apply_guard = WorktreeApplyBoundaryGuard::new(workspace.path(), Arc::clone(&gate));
    let apply_capability = CapabilityId::new(WORKTREE_APPLY_CAPABILITY_ID).unwrap();
    let plan_guard = WorktreePlanBoundaryGuard::new(workspace.path());
    let plan_capability = CapabilityId::new("tool.worktree_plan").unwrap();

    let absolute = std::env::temp_dir()
        .join("evil.txt")
        .to_string_lossy()
        .into_owned();
    let cases: [(serde_json::Value, &str); 3] = [
        (
            order_args("越界", &[(absolute.as_str(), "update", "改")]),
            "禁止绝对路径",
        ),
        (
            order_args("敏感", &[(".env", "delete", "删")]),
            "credential surface",
        ),
        (
            order_args("重复", &[("a.txt", "add", "建"), ("a.txt", "add", "建")]),
            "重复声明",
        ),
    ];
    for (arguments, needle) in cases {
        let refusal = plan_guard
            .deny(&ToolGuardRequest::new(
                &plan_capability,
                WORKTREE_PLAN_TOOL_NAME,
                &arguments,
            ))
            .expect("改单边界必须拒");
        assert!(
            refusal.contains(needle),
            "计划 Guard 帧缺 {needle:?}: {refusal}"
        );

        // 同一病灶改单塞进 apply 参数面, 落盘 Guard 同文拒 (预检与 Guard 同判定)。
        let as_apply = serde_json::json!({
            "plan": arguments,
            "patch": "*** Begin Patch\n*** Update File: ghost.txt\n@@\n-x\n+y\n*** End Patch",
        });
        let refusal = apply_guard
            .deny(&ToolGuardRequest::new(
                &apply_capability,
                "worktree_apply",
                &as_apply,
            ))
            .expect("落盘边界必须拒");
        assert!(
            refusal.contains(needle),
            "落盘 Guard 帧缺 {needle:?}: {refusal}"
        );
    }

    // 合规改单 (只建新文件, 无门禁前置) → Guard 无异议 (不是放行, 是不拒)。
    let valid = serde_json::json!({
        "plan": order_args("合规", &[("n.txt", "add", "建")]),
        "patch": "*** Begin Patch\n*** Add File: n.txt\n+created\n*** End Patch",
    });
    assert!(
        apply_guard
            .deny(&ToolGuardRequest::new(
                &apply_capability,
                "worktree_apply",
                &valid
            ))
            .is_none(),
        "合规改单不该被边界 Guard 拒"
    );
}

/// ⑨ 与 git_write 分工文案: 两级工具描述明写「只动文件、不含 git 写,
/// 提交走 git_write L1」—— 防模型在 worktree 工具里幻觉 git 操作。
#[test]
fn tool_descriptions_state_the_git_write_division() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let plan = WorktreePlanTool::new(workspace.path()).declaration();
    let apply = apeireth_tools_canonical::WorktreeApplyTool::new(
        workspace.path(),
        Arc::new(ObservedGate::new()),
    )
    .declaration();
    let plan_description = plan.description.clone().expect("计划拍描述必须在册");
    let apply_description = apply.description.clone().expect("落盘描述必须在册");

    for description in [plan_description.clone(), apply_description.clone()] {
        assert!(
            description.contains("never perform git writes"),
            "分工文案缺『不含 git 写』: {description}"
        );
        assert!(
            description.contains("graded git_write tool"),
            "分工文案缺提交去处: {description}"
        );
        assert!(
            description.contains("L1 reversible local writes"),
            "分工文案缺 L1 分级口径: {description}"
        );
    }
    assert!(
        plan_description.contains("read-only") && plan_description.contains("needs no approval"),
        "计划拍描述必须写明只读零审批: {plan_description}"
    );
    assert!(
        apply_description.contains("four controlled beats")
            && apply_description.contains("all-or-nothing")
            && apply_description.contains("require-approval level"),
        "落盘描述必须写明四拍/全有或全无/审批级: {apply_description}"
    );
    assert_eq!(plan.name, "worktree_plan");
    assert_eq!(apply.name, "worktree_apply");
}

/// ⑩ 参数面即帧: 未知字段 / 缺字段 / 空补丁一律错误帧 (错误即帧, 不静默)。
#[tokio::test]
async fn malformed_parameters_are_refused_as_error_frames() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let tool = apeireth_tools_canonical::WorktreeApplyTool::new(
        workspace.path(),
        Arc::new(ObservedGate::new()),
    );
    for arguments in [
        // 未知字段。
        serde_json::json!({
            "plan": order_args("x", &[("a.txt", "add", "建")]),
            "patch": "*** Begin Patch\n*** Add File: a.txt\n+x\n*** End Patch",
            "extra": 1,
        }),
        // 缺 patch。
        serde_json::json!({ "plan": order_args("x", &[("a.txt", "add", "建")]) }),
        // 动作词表外 (add/update/delete 之外)。
        serde_json::json!({
            "plan": order_args("x", &[("a.txt", "modify", "改")]),
            "patch": "*** Begin Patch\n*** Add File: a.txt\n+x\n*** End Patch",
        }),
    ] {
        let message = expect_error(&tool.invoke(&call("worktree_apply", arguments)).await);
        assert!(
            message.contains("invalid worktree_apply parameters"),
            "{message}"
        );
    }
    // 空补丁 = 解析错误帧 (不是"成功零动作"的谎帧)。
    let message = expect_error(
        &tool
            .invoke(&call(
                "worktree_apply",
                apply_args("空补丁", &[("a.txt", "add", "建")], "", &[]),
            ))
            .await,
    );
    assert!(!message.is_empty(), "空补丁必须给错误帧");
}

/// ⑪ 校验命令载荷面: 显示串脱敏 (凭据不入审批卡/日志)。
#[test]
fn verify_command_display_redacts_secret_surfaces() {
    let command = VerifyCommand {
        program: "cargo".to_string(),
        args: vec![
            "--version".to_string(),
            "https://user:pass-token-12345@host.example/x".to_string(),
        ],
    };
    let display = command.display();
    assert!(display.starts_with("cargo --version"), "{display}");
    assert!(
        !display.contains("pass-token-12345"),
        "凭据不得进入显示面: {display}"
    );
}
