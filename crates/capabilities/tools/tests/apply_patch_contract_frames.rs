//! 受控文件写入 (`tool.apply_patch`) 输出合同的谎帧封堵回归 (内测整改)。
//!
//! 病灶 (实测): 空补丁 / 畸形补丁 / 零 hunk 补丁被解析成空动作集, 工具照常
//! 返回 `status: "applied"` + `total_actions: 0`, 文件零变化 —— 输出合同说谎;
//! 且空动作集在放行判定里被空真值误判为「纯修改」, 连审批卡都跳过。
//!
//! 现口径 (本文件钉死):
//! 1. 空补丁 / 畸形补丁 / 零 hunk 补丁 → **错误帧**, 绝不出现 "applied";
//! 2. "applied" 只在真有动作落地后返回, 计数 (新增/修改/删除) 如实;
//! 3. 空/解析不了的补丁在放行判定里归 `Unparseable` (fail-closed 向人工审批),
//!    不得判 `ModificationOnly` 免审。

use std::sync::{Arc, Mutex};

use apeireth_governance::{Decision, GovernanceHook, Permission, PermissionPolicy};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_tools_canonical::{
    apply_patch_capability, write_release_class_for_patch, ApplyPatchWriteApprovalHook,
    ObservedGate, TransactionalPatchApplier, WriteReleaseClass, APPLY_PATCH_TOOL_NAME,
};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, GovernanceRequest};

fn call(patch: &str) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: APPLY_PATCH_TOOL_NAME.into(),
        arguments: serde_json::json!({ "patch": patch }),
    }
}

fn granted_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            apeireth_tools_canonical::APPLY_PATCH_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 断言一帧是错误帧且从不谎称 "applied"。
fn assert_error_frame(result: &ToolResult, case: &str) {
    assert!(
        !result.is_ok(),
        "{case}: 必须是错误帧, got {}",
        result.render()
    );
    let rendered = result.render();
    assert!(
        !rendered.contains("applied"),
        "{case}: 错误帧绝不许出现 applied: {rendered}"
    );
    assert!(
        !rendered.contains("total_actions"),
        "{case}: 错误帧绝不许出现动作计数: {rendered}"
    );
}

/// 空补丁 = 错误帧, 不是「applied + 零动作」。
#[tokio::test]
async fn empty_patch_is_an_error_frame_not_applied_zero_actions() {
    let dir = tempfile::tempdir().unwrap();
    let tool = apply_patch_capability(dir.path(), Arc::new(ObservedGate::new()), granted_policy());

    for patch in [
        "*** Begin Patch\n*** End Patch",
        "*** Begin Patch\n\n*** End Patch",
        "   *** Begin Patch   \n*** End Patch   ",
    ] {
        let result = tool.invoke(&call(patch)).await;
        assert_error_frame(&result, &format!("空补丁 {patch:?}"));
        assert!(
            result.render().contains("补丁未声明任何动作"),
            "空补丁要给出可读的解析错误: {}",
            result.render()
        );
    }
    // 原语层同口径: 空动作集绝不返回 Ok 报告。
    let err = TransactionalPatchApplier::apply(dir.path(), "*** Begin Patch\n*** End Patch")
        .expect_err("空补丁必须是 Err");
    assert!(
        matches!(
            err,
            apeireth_tools_canonical::ApplyPatchError::ParseError(_)
        ),
        "{err:?}"
    );
}

/// 畸形补丁 = 错误帧 (含实测形态: 整份补丁无真换行、指令拼写错、只有标记)。
#[tokio::test]
async fn malformed_patches_are_error_frames() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let tool = apply_patch_capability(dir.path(), Arc::new(ObservedGate::new()), granted_policy());

    let cases: Vec<String> = vec![
        // 纯乱码: 标记之间全是认不出的行。
        "*** Begin Patch\nrandom noise\n*** End Patch".to_string(),
        // 实测形态: 换行被转义成字面量, 整份补丁挤成一行 —— 旧解析静默吞行
        // 得到空动作集谎帧。
        "*** Begin Patch\\n*** Add File: x.txt\\n+hello\\n*** End Patch".to_string(),
        // 指令拼写错 (缺一个星号 / 多余空格) → 认不出的补丁行。
        "*** Begin Patch\n** Add File: x.txt\n+hello\n*** End Patch".to_string(),
        "*** Begin Patch\n*** Update File : a.txt\n-old\n+new\n*** End Patch".to_string(),
        // 缺结束标记。
        "*** Begin Patch\n*** Add File: x.txt\n+hello".to_string(),
        // 空目标路径。
        "*** Begin Patch\n*** Add File:\n+hello\n*** End Patch".to_string(),
    ];
    for patch in &cases {
        let result = tool.invoke(&call(patch)).await;
        assert_error_frame(&result, &format!("畸形补丁 {patch:?}"));
        assert_eq!(
            write_release_class_for_patch(patch),
            WriteReleaseClass::Unparseable,
            "畸形补丁必须归 Unparseable (fail-closed 向人工审批): {patch:?}"
        );
    }
    assert!(
        !dir.path().join("x.txt").exists(),
        "畸形补丁绝不落盘任何文件"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "hello",
        "畸形补丁绝不改动既有文件"
    );
}

/// 零 hunk 的 Update 段 = 错误帧 (不许写回原内容冒充修改)。
#[tokio::test]
async fn update_without_any_hunk_is_an_error_frame() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let tool = apply_patch_capability(dir.path(), Arc::new(ObservedGate::new()), granted_policy());

    for patch in [
        "*** Begin Patch\n*** Update File: a.txt\n*** End Patch",
        "*** Begin Patch\n*** Update File: a.txt\n@@\n*** End Patch",
        "*** Begin Patch\n*** Update File: a.txt\nno hunk markers here\n*** End Patch",
    ] {
        let result = tool.invoke(&call(patch)).await;
        assert_error_frame(&result, &format!("零 hunk 补丁 {patch:?}"));
        assert!(
            result.render().contains("hunk"),
            "零 hunk 补丁要给出可读的解析错误: {}",
            result.render()
        );
    }
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "hello",
        "零 hunk 补丁绝不改动文件"
    );
}

/// "applied" 必带真实动作: 新增/修改/删除计数逐项如实, 磁盘状态一致。
#[tokio::test]
async fn applied_report_carries_exactly_the_actions_that_landed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("read.txt"), "hello old world").unwrap();
    std::fs::write(dir.path().join("gone.txt"), "to be deleted").unwrap();

    let gate = Arc::new(ObservedGate::new());
    // 读前观测: 被修改/删除的文件先读 (共享门禁的「已读」证据)。
    let reader = apeireth_tools_canonical::FilesystemTool::new(dir.path().to_path_buf())
        .with_observed_gate(Arc::clone(&gate));
    for path in ["read.txt", "gone.txt"] {
        let read = reader
            .invoke(&ToolCall {
                id: "read".into(),
                name: "filesystem".into(),
                arguments: serde_json::json!({ "operation": "read", "path": path }),
            })
            .await;
        assert!(read.is_ok(), "{path}: {}", read.render());
    }

    let tool = apply_patch_capability(dir.path(), gate, granted_policy());
    let patch = "*** Begin Patch\n\
*** Add File: new.txt\n\
+created\n\
*** Update File: read.txt\n\
@@\n\
-old\n\
+NEW\n\
*** Delete File: gone.txt\n\
*** End Patch";
    let result = tool.invoke(&call(patch)).await;
    assert!(result.is_ok(), "{}", result.render());

    let report = match &result.outcome {
        apeireth_protocol::canonical::ToolOutcome::Ok { value } => value.clone(),
        other => panic!("applied 报告必须是成功帧: {other:?}"),
    };
    assert_eq!(report["status"], "applied");
    assert_eq!(report["files_added"], serde_json::json!(["new.txt"]));
    assert_eq!(report["files_updated"], serde_json::json!(["read.txt"]));
    assert_eq!(report["files_deleted"], serde_json::json!(["gone.txt"]));
    assert_eq!(report["total_actions"], 3, "计数必须如实: {report}");

    // 磁盘状态与报告一致 (真有动作落地)。
    assert_eq!(
        std::fs::read_to_string(dir.path().join("new.txt")).unwrap(),
        "created"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("read.txt")).unwrap(),
        "hello NEW world"
    );
    assert!(!dir.path().join("gone.txt").exists());
}

/// 空/解析不了的补丁永不自动放行: 放行类别归 Unparseable, 治理判定停在人工审批
/// (旧病灶: 空集合空真值被判 ModificationOnly, 连审批卡都跳过)。
#[tokio::test]
async fn empty_and_unparseable_patches_never_auto_pass() {
    for patch in [
        "",
        "not a patch",
        "*** Begin Patch\n*** End Patch",
        "*** Begin Patch\njunk line\n*** End Patch",
        "*** Begin Patch\n*** Update File: a.txt\n*** End Patch",
    ] {
        assert_eq!(
            write_release_class_for_patch(patch),
            WriteReleaseClass::Unparseable,
            "空/解析不了的补丁必须归 Unparseable: {patch:?}"
        );
    }
    // 非空补丁的既有语义不变: 纯修改 / 含创建删除。
    assert_eq!(
        write_release_class_for_patch(
            "*** Begin Patch\n*** Update File: a.txt\n@@\n-x\n+y\n*** End Patch"
        ),
        WriteReleaseClass::ModificationOnly
    );
    assert_eq!(
        write_release_class_for_patch("*** Begin Patch\n*** Add File: b.txt\n+x\n*** End Patch"),
        WriteReleaseClass::ContainsCreateOrDelete
    );

    // 治理放行钩子: 空补丁必须停在人工审批, 绝不 Allow。
    let hook = ApplyPatchWriteApprovalHook::new();
    for patch in ["", "*** Begin Patch\n*** End Patch", "junk"] {
        let capability = CapabilityId::new(apeireth_tools_canonical::APPLY_PATCH_CAPABILITY_ID)
            .expect("valid capability id");
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
        let decision = hook.evaluate(&request).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "空/解析不了的补丁必须停在人工审批, got {decision:?} for {patch:?}"
        );
    }
}
