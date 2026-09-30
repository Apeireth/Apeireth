//! 受控文件写入生产工具面 (`tool.apply_patch`) 的接线回归 (件一/件三)。
//!
//! 覆盖: 注册可见 (声明面) / 默认审批级 (未授权 → `pipeline.pre_ask` 冻结) /
//! 显式授权放行 / 自动放行档只放行已读文件修改 (删除/新建仍审批) / 工作区外
//! 拒绝 (fail-closed 即帧) / 敏感面拒绝 / 读前观测门禁 (未读覆盖写拒绝) /
//! 补丁原子性 (多文件部分失败全回滚) / 输出归一合同 / 超时归 `timeout.*` /
//! git 写边界文案在工具描述中。

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionPolicy,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_tools_canonical::{
    apply_patch_capability, apply_patch_pipeline, write_release_class_for_patch, ApplyPatchTool,
    ApplyPatchWriteApprovalHook, ApplyPatchWriteBoundaryGuard, AroundPolicy, FilesystemTool,
    ObservedGate, PipelineFailure, ToolExecutionPipeline, WriteReleaseClass,
    APPLY_PATCH_CAPABILITY_ID, APPLY_PATCH_TIMEOUT_MS, APPLY_PATCH_TOOL_NAME,
    DEFAULT_MAX_TIMEOUT_MS, GIT_WRITE_BOUNDARY_NOTE,
};

fn call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        arguments,
    }
}

/// 用共享读前观测门禁的读工具先读一次 (读事件 = 「已读」观测)。
async fn read_first(root: &std::path::Path, gate: &Arc<ObservedGate>, path: &str) {
    let reader = FilesystemTool::new(root.to_path_buf()).with_observed_gate(Arc::clone(gate));
    let result = reader
        .invoke(&call(
            "filesystem",
            serde_json::json!({ "operation": "read", "path": path }),
        ))
        .await;
    assert!(result.is_ok(), "{path}: {}", result.render());
}

/// 治理层放行规则判定 (自动放行档)。
async fn judge_release(hook: &ApplyPatchWriteApprovalHook, patch: String) -> Decision {
    let capability = CapabilityId::new(APPLY_PATCH_CAPABILITY_ID).unwrap();
    let arguments = serde_json::json!({ "patch": patch });
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    hook.evaluate(&request).await
}

fn add_patch(path: &str, body: &str) -> String {
    format!("*** Begin Patch\n*** Add File: {path}\n+{body}\n*** End Patch")
}

fn update_patch(path: &str, old: &str, new: &str) -> String {
    format!("*** Begin Patch\n*** Update File: {path}\n@@\n-{old}\n+{new}\n*** End Patch")
}

fn granted_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            APPLY_PATCH_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 注册可见: 声明面 = 模型侧看到的工具名 + 补丁合同 + 参数面。
#[tokio::test]
async fn registration_exposes_apply_patch_with_the_patch_contract() {
    let dir = tempfile::tempdir().unwrap();
    let tool = ApplyPatchTool::new(dir.path(), Arc::new(ObservedGate::new()));
    assert_eq!(tool.id().as_str(), APPLY_PATCH_CAPABILITY_ID);

    let declaration = tool.declaration();
    assert_eq!(declaration.name, APPLY_PATCH_TOOL_NAME);
    let description = declaration
        .description
        .clone()
        .expect("the file-write tool must carry a description");
    for surface in [
        "*** Add File",
        "*** Update File",
        "*** Delete File",
        "workspace-relative",
        "read-before-overwrite",
        "human approval",
    ] {
        assert!(
            description.contains(surface),
            "description must state the contract surface {surface:?}: {description}"
        );
    }
    let properties = declaration
        .parameters
        .get("properties")
        .and_then(|value| value.as_object())
        .cloned()
        .expect("parameters declare properties");
    assert!(properties.contains_key("patch"), "{properties:?}");
    assert_eq!(
        declaration.parameters.get("required"),
        Some(&serde_json::json!(["patch"])),
    );
}

/// 件三: git 写边界文案在工具描述中 (防模型幻觉出 git 写工具)。
#[test]
fn git_write_boundary_is_stated_in_the_tool_description() {
    let dir = tempfile::tempdir().unwrap();
    let tool = ApplyPatchTool::new(dir.path(), Arc::new(ObservedGate::new()));
    let description = tool
        .declaration()
        .description
        .expect("the file-write tool must carry a description");
    assert!(
        description.contains(GIT_WRITE_BOUNDARY_NOTE),
        "the model-facing description must carry the git write boundary note verbatim: {description}"
    );
    for phrase in [
        "git write operations are",
        "deliberately not provided as tools",
        "no tool",
        "none exists",
    ] {
        assert!(
            GIT_WRITE_BOUNDARY_NOTE.contains(phrase),
            "the boundary note must state {phrase:?}: {GIT_WRITE_BOUNDARY_NOTE}"
        );
    }
}

/// 默认审批级: 未授权的写入调用停在 require-approval 档 (`pipeline.pre_ask`
/// 冻结即帧), 不落盘。
#[tokio::test]
async fn unauthorized_write_freezes_at_pre_ask_and_never_runs() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Arc::new(ObservedGate::new());
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    let tool = apply_patch_capability(dir.path(), gate, policy);

    let result = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": add_patch("new.txt", "hello") }),
        ))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(
        result.render().contains("pipeline.pre_ask"),
        "unauthorized writes must freeze at the approval level: {}",
        result.render()
    );
    assert!(!dir.path().join("new.txt").exists(), "no write may land");
}

/// 显式授权放行: 有 grant 的写入调用穿过五段流水线并落盘。
#[tokio::test]
async fn explicit_grant_authorizes_the_write() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Arc::new(ObservedGate::new());
    let tool = apply_patch_capability(dir.path(), gate, granted_policy());

    let result = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": add_patch("new.txt", "hello") }),
        ))
        .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("new.txt")).unwrap(),
        "hello"
    );
}

/// 自动放行档只放行已读文件的修改 (删除/新建仍审批):
/// ① 修改类补丁过放行规则 (免逐次审批) 且已读文件的修改真正落盘;
/// ② 创建/删除类补丁停在 [`Decision::RequireApproval`] (同一条人工审批链);
/// ③ 未读文件的修改被读前观测门禁拒绝 —— 放行集合恰是「已读文件的修改」。
#[tokio::test]
async fn auto_pass_releases_only_read_file_modifications() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("read.txt"), "hello old world").unwrap();
    std::fs::write(dir.path().join("unread.txt"), "hello old world").unwrap();

    // 放行规则 (治理层, 与本地审批面板 / IM 审批卡同链)。
    let hook = ApplyPatchWriteApprovalHook::new();
    assert_eq!(
        judge_release(&hook, update_patch("read.txt", "old", "NEW")).await,
        Decision::Allow,
        "修改类补丁在自动放行档放行"
    );
    for patch in [
        add_patch("created.txt", "hello"),
        "*** Begin Patch\n*** Delete File: read.txt\n*** End Patch".to_string(),
        format!(
            "{}\n*** Delete File: read.txt\n*** End Patch",
            add_patch("created.txt", "hello").replace("\n*** End Patch", "")
        ),
    ] {
        let decision = judge_release(&hook, patch.clone()).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "删除/新建永不自动放行 (仍审批), got {decision:?} for {patch}"
        );
    }
    // 解析失败同样 fail-closed 向人工审批。
    assert!(matches!(
        judge_release(&hook, "not a patch".to_string()).await,
        Decision::RequireApproval { .. }
    ));
    assert_eq!(
        write_release_class_for_patch(&update_patch("read.txt", "old", "NEW")),
        WriteReleaseClass::ModificationOnly
    );
    assert_eq!(
        write_release_class_for_patch(&add_patch("created.txt", "hello")),
        WriteReleaseClass::ContainsCreateOrDelete
    );

    // 「只放行已读文件的修改」: 共享读前观测门禁 (读工具的观测即「已读」)。
    let gate = Arc::new(ObservedGate::new());
    read_first(dir.path(), &gate, "read.txt").await;

    let tool = apply_patch_capability(dir.path(), Arc::clone(&gate), granted_policy());
    let released = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": update_patch("read.txt", "old", "NEW") }),
        ))
        .await;
    assert!(released.is_ok(), "{}", released.render());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("read.txt")).unwrap(),
        "hello NEW world"
    );

    let unread = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": update_patch("unread.txt", "old", "NEW") }),
        ))
        .await;
    assert!(!unread.is_ok(), "{}", unread.render());
    assert!(
        unread.render().contains("读取观测"),
        "未读文件的修改不属于放行集合: {}",
        unread.render()
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("unread.txt")).unwrap(),
        "hello old world"
    );
}

/// 工作区外路径拒绝 (fail-closed 即帧): `..` 逃逸与绝对路径都不可达。
#[tokio::test]
async fn outside_workspace_paths_are_refused() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let outside = base.path().join("outside.txt");
    std::fs::write(&outside, "secret").unwrap();

    let gate = Arc::new(ObservedGate::new());
    let tool = apply_patch_capability(&root, gate, granted_policy());

    for patch in [
        add_patch("../outside.txt", "pwned"),
        format!(
            "*** Begin Patch\n*** Delete File: {}\n*** End Patch",
            outside.display()
        ),
    ] {
        let result = tool
            .invoke(&call(
                APPLY_PATCH_TOOL_NAME,
                serde_json::json!({ "patch": patch }),
            ))
            .await;
        assert!(!result.is_ok(), "{}", result.render());
        assert!(
            result.render().contains("拒绝写入"),
            "workspace escape must be refused as a frame: {}",
            result.render()
        );
    }
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "secret",
        "nothing outside the workspace may change"
    );
}

/// 敏感面拒绝: 凭据/密钥面复用 `sensitive_path` 拒绝面 (即帧)。
#[tokio::test]
async fn credential_surface_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Arc::new(ObservedGate::new());
    let tool = apply_patch_capability(dir.path(), gate, granted_policy());

    for path in ["data/creds.json", ".env.local"] {
        let result = tool
            .invoke(&call(
                APPLY_PATCH_TOOL_NAME,
                serde_json::json!({ "patch": add_patch(path, "token=1") }),
            ))
            .await;
        assert!(!result.is_ok(), "{path}: {}", result.render());
        assert!(
            result.render().contains("credential surface is unreadable"),
            "sensitive surface must be refused: {}",
            result.render()
        );
        assert!(
            !dir.path().join(path).exists(),
            "{path} must never be written"
        );
    }
}

/// 读前观测门禁: 未读覆盖写拒绝 (工具层即帧, 原语层同门禁兜底)。
#[tokio::test]
async fn unread_overwrite_is_refused_by_the_observation_gate() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello old world").unwrap();

    let gate = Arc::new(ObservedGate::new());
    let tool = ApplyPatchTool::new(dir.path(), Arc::clone(&gate));
    let result = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": update_patch("a.txt", "old", "NEW") }),
        ))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(
        result.render().contains("读取观测"),
        "blind overwrite must be refused with the read-first guidance: {}",
        result.render()
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "hello old world"
    );
}

/// 补丁原子性: 预演失败的多文件补丁不落任何盘 (全有或全无)。
#[tokio::test]
async fn multi_file_patch_context_mismatch_touches_no_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello world").unwrap();

    let gate = Arc::new(ObservedGate::new());
    read_first(dir.path(), &gate, "a.txt").await;
    let tool = apply_patch_capability(dir.path(), gate, granted_policy());
    let patch = "*** Begin Patch\n*** Add File: created.txt\n+hello\n*** Update File: a.txt\n<<<<<<< SEARCH\nno such context\n=======\nx\n>>>>>>>\n*** End Patch";
    let result = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": patch }),
        ))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(!dir.path().join("created.txt").exists(), "no add may land");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "hello world"
    );
}

/// 补丁原子性: 提交阶段部分失败全回滚 (已写入的文件恢复原状)。
#[tokio::test]
async fn multi_file_patch_commit_failure_rolls_back_every_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello old world").unwrap();
    // 提交期故障注入: `sub` 是**文件**, `sub/new.txt` 的父目录建不出来 ——
    // 该动作在提交阶段 IO 失败, 已提交的兄弟动作必须全部回滚。
    std::fs::write(dir.path().join("sub"), "not a directory").unwrap();

    let gate = Arc::new(ObservedGate::new());
    read_first(dir.path(), &gate, "a.txt").await;
    let tool = apply_patch_capability(dir.path(), gate, granted_policy());
    let patch = "*** Begin Patch\n*** Add File: sub/new.txt\n+hello\n*** Update File: a.txt\n@@\n-old\n+NEW\n*** End Patch";
    let result = tool
        .invoke(&call(
            APPLY_PATCH_TOOL_NAME,
            serde_json::json!({ "patch": patch }),
        ))
        .await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(!dir.path().join("sub/new.txt").exists(), "no add may land");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "hello old world",
        "the sibling write must be rolled back"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("sub")).unwrap(),
        "not a directory"
    );
}

/// 一个测试替身: 输出形状与合同不符时必须被归一阶段拒绝。
struct DriftTool;

#[async_trait::async_trait]
impl ToolCapability for DriftTool {
    fn id(&self) -> &apeireth_core::kernel::CapabilityId {
        use std::sync::OnceLock;
        static ID: OnceLock<apeireth_core::kernel::CapabilityId> = OnceLock::new();
        ID.get_or_init(|| apeireth_core::kernel::CapabilityId::new("tool.drift").unwrap())
    }

    fn declaration(&self) -> apeireth_protocol::canonical::NormalizedTool {
        apeireth_protocol::canonical::NormalizedTool::new("drift")
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::json!({ "oops": 1 }))
    }
}

/// 输出归一合同: 成功报告五字段形状冻结; 漂移形状被合同拒绝 (即帧)。
#[tokio::test]
async fn output_contract_freezes_the_report_shape_and_refuses_drift() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Arc::new(ObservedGate::new());
    let tool = ApplyPatchTool::new(dir.path(), Arc::clone(&gate));
    let pipeline = apply_patch_pipeline(dir.path(), gate, granted_policy());

    let executed = pipeline
        .run(
            &tool,
            &call(
                APPLY_PATCH_TOOL_NAME,
                serde_json::json!({ "patch": add_patch("new.txt", "hello") }),
            ),
        )
        .await;
    assert!(executed.outcome.ok, "{}", executed.result.render());
    let structured = executed
        .outcome
        .structured()
        .expect("structured report")
        .clone();
    assert_eq!(structured["status"], "applied");
    for field in ["files_added", "files_updated", "files_deleted"] {
        assert!(
            structured[field].is_array(),
            "{field} must be an array: {structured}"
        );
    }
    assert!(structured["total_actions"].is_number(), "{structured}");

    let drift_pipeline =
        ToolExecutionPipeline::new().with_output_schema(ApplyPatchTool::output_schema());
    let drifted = drift_pipeline
        .run(&DriftTool, &call("drift", serde_json::json!({})))
        .await;
    assert!(drifted.is_failed(), "{}", drifted.result.render());
    assert!(
        matches!(drifted.failure, Some(PipelineFailure::ContractViolation(_))),
        "{:?}",
        drifted.failure
    );
    assert!(
        drifted.result.render().contains("contract"),
        "{}",
        drifted.result.render()
    );
}

/// 超时归 `timeout.*` code 族: 非法到期请求与真到期都在同一族闭合。
#[tokio::test]
async fn timeout_failures_close_in_the_timeout_code_family() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Arc::new(ObservedGate::new());
    let tool = ApplyPatchTool::new(dir.path(), gate);

    // 生产时限本身必须被到期门接受 (接线口径不越界)。
    let production =
        AroundPolicy::new().with_timeout(APPLY_PATCH_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS);
    assert_eq!(
        production.validated_timeout_ms().unwrap(),
        Some(APPLY_PATCH_TIMEOUT_MS)
    );

    // 零值到期请求被到期门拒绝, 归同一 `timeout.*` 族 (可重试帧)。
    let pipeline = ToolExecutionPipeline::new()
        .with_around(AroundPolicy::new().with_timeout(0, DEFAULT_MAX_TIMEOUT_MS))
        .with_output_schema(ApplyPatchTool::output_schema());
    let refused = pipeline
        .run(
            &tool,
            &call(
                APPLY_PATCH_TOOL_NAME,
                serde_json::json!({ "patch": add_patch("new.txt", "hello") }),
            ),
        )
        .await;
    assert!(refused.is_failed(), "{}", refused.result.render());
    match refused.failure {
        Some(PipelineFailure::Timeout { code }) => {
            assert!(code.starts_with("timeout."), "{code}");
            assert!(
                refused.result.render().contains("timeout."),
                "{}",
                refused.result.render()
            );
        }
        other => panic!("expected a timeout-family failure, got {other:?}"),
    }

    // 真到期: 短到期下的慢调用同样归 `timeout.*` 族。
    struct SlowTool;
    #[async_trait::async_trait]
    impl ToolCapability for SlowTool {
        fn id(&self) -> &apeireth_core::kernel::CapabilityId {
            use std::sync::OnceLock;
            static ID: OnceLock<apeireth_core::kernel::CapabilityId> = OnceLock::new();
            ID.get_or_init(|| apeireth_core::kernel::CapabilityId::new("tool.slow").unwrap())
        }
        fn declaration(&self) -> apeireth_protocol::canonical::NormalizedTool {
            apeireth_protocol::canonical::NormalizedTool::new("slow")
        }
        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            ToolResult::ok(&call.id, serde_json::json!({}))
        }
    }
    let expiring = ToolExecutionPipeline::new()
        .with_around(AroundPolicy::new().with_timeout(5, DEFAULT_MAX_TIMEOUT_MS));
    let expired = expiring
        .run(&SlowTool, &call("slow", serde_json::json!({})))
        .await;
    assert!(expired.is_failed(), "{}", expired.result.render());
    match expired.failure {
        Some(PipelineFailure::Timeout { code }) => {
            assert!(code.starts_with("timeout."), "{code}");
        }
        other => panic!("expected a timeout-family failure, got {other:?}"),
    }
}

/// 单调边界 Guard 只拒不放: 自动放行档放行的调用也照穿 (未读覆盖写仍拒)。
#[tokio::test]
async fn boundary_guard_refuses_blind_overwrite_monotonically() {
    use apeireth_governance::{ToolGuard, ToolGuardRequest};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello old world").unwrap();
    let gate = Arc::new(ObservedGate::new());
    let guard = ApplyPatchWriteBoundaryGuard::new(dir.path(), Arc::clone(&gate));

    let capability = apeireth_core::kernel::CapabilityId::new(APPLY_PATCH_CAPABILITY_ID).unwrap();
    let arguments = serde_json::json!({ "patch": update_patch("a.txt", "old", "NEW") });
    let request = ToolGuardRequest::new(&capability, APPLY_PATCH_TOOL_NAME, &arguments);
    let refusal = guard
        .deny(&request)
        .expect("guard must refuse the blind overwrite");
    assert!(refusal.contains("读取观测"), "{refusal}");

    // 读过后同一 Guard 无异议 (只拒不放, 从不表达放行)。
    gate.observe_present(
        &dir.path().join("a.txt"),
        apeireth_tools_canonical::FileVersion::from_metadata(
            &std::fs::metadata(dir.path().join("a.txt")).unwrap(),
        ),
    );
    assert_eq!(guard.deny(&request), None);
}
