//! The A1 contract: a workspace is a first-class managed identity.
//!
//! Every assertion here is decidable from the code under test alone — no
//! tautology, no "the value equals what I just computed with the same
//! function" — so a broken wiring turns the suite red instead of silently
//! passing.

use std::path::Path;

use apeireth_core::kernel::ids::WorkspaceId;
use apeireth_runtime::canonical::workspace::{
    safe_root_display, InMemoryWorkspaceRegistry, Workspace, WorkspaceCreateRequest,
    WorkspaceError, WorkspacePermissionScope, WorkspaceRegistry, WorkspaceRootKind,
    WorkspaceStatus, MAX_DISPLAY_NAME_BYTES, MAX_ROOT_DISPLAY_BYTES, ROOT_DISPLAY_PLACEHOLDER,
};

const NOW_MS: u64 = 1_700_000_000_000;

/// A fresh registry with nothing in it.
fn registry() -> InMemoryWorkspaceRegistry {
    InMemoryWorkspaceRegistry::new()
}

/// A managed-workspace request for `root`.
fn managed_request(
    root: &str,
    display_name: &str,
    permission_scope: WorkspacePermissionScope,
) -> WorkspaceCreateRequest {
    WorkspaceCreateRequest::managed(root, display_name, permission_scope, NOW_MS)
}

#[test]
fn two_workspaces_may_share_a_display_name_and_stay_distinct_by_uuid() {
    let registry = registry();
    let first = registry
        .create(managed_request(
            "/roots/alpha",
            "shared-name",
            WorkspacePermissionScope::Read,
        ))
        .expect("create first");
    let second = registry
        .create(managed_request(
            "/roots/beta",
            "shared-name",
            WorkspacePermissionScope::Read,
        ))
        .expect("create second");

    assert_eq!(first.display_name, second.display_name);
    assert_ne!(
        first.workspace_id, second.workspace_id,
        "同名工作区必须按 UUID 区分，而不是按名字"
    );

    let listed = registry.list();
    assert_eq!(listed.len(), 2, "两个同名工作区都要在列表里");
    assert!(listed.iter().any(|w| w.workspace_id == first.workspace_id));
    assert!(listed.iter().any(|w| w.workspace_id == second.workspace_id));

    registry
        .select(&first.workspace_id, first.revision)
        .expect("select first");
    assert_eq!(
        registry.current_selection(),
        Some(first.workspace_id),
        "当前选择必须是被选中的那个"
    );
    assert_ne!(registry.current_selection(), Some(second.workspace_id));
}

#[test]
fn changing_the_permission_scope_advances_the_revision_by_exactly_one() {
    let registry = registry();
    let created = registry
        .create(managed_request(
            "/roots/gamma",
            "gamma",
            WorkspacePermissionScope::Read,
        ))
        .expect("create");
    assert_eq!(created.revision, 0, "新建工作区的 revision 从 0 起");
    assert!(!created.can_write, "Read 作用域不得可写");

    let updated = registry
        .change_permission_scope(
            &created.workspace_id,
            created.revision,
            WorkspacePermissionScope::Write,
            NOW_MS + 1_000,
        )
        .expect("change scope");

    assert_eq!(
        updated.revision,
        created.revision + 1,
        "作用域变化必须恰好 +1"
    );
    assert_eq!(updated.permission_scope, WorkspacePermissionScope::Write);
    assert!(updated.can_write, "Write 作用域必须可写");

    let stored = registry.get(&created.workspace_id).expect("still stored");
    assert_eq!(stored.revision, updated.revision);
    assert_eq!(stored.permission_scope, WorkspacePermissionScope::Write);
}

#[test]
fn a_stale_revision_conflict_changes_nothing() {
    let registry = registry();
    let created = registry
        .create(managed_request(
            "/roots/delta",
            "delta",
            WorkspacePermissionScope::Read,
        ))
        .expect("create");
    let updated = registry
        .change_permission_scope(
            &created.workspace_id,
            created.revision,
            WorkspacePermissionScope::Write,
            NOW_MS + 1_000,
        )
        .expect("change scope");
    let before = registry.get(&created.workspace_id).expect("stored");

    let conflict = registry
        .change_permission_scope(
            &created.workspace_id,
            created.revision,
            WorkspacePermissionScope::ApprovalWrite,
            NOW_MS + 2_000,
        )
        .expect_err("过期 revision 必须报冲突");

    assert_eq!(
        conflict,
        WorkspaceError::RevisionConflict {
            expected: created.revision,
            actual: updated.revision,
        }
    );

    let after = registry.get(&created.workspace_id).expect("stored");
    assert_eq!(after, before, "冲突后工作区必须逐字段不变");
    assert_eq!(after.revision, updated.revision, "冲突不得改动 revision");
    assert_eq!(
        after.permission_scope,
        WorkspacePermissionScope::Write,
        "冲突不得改动作用域"
    );
    assert_eq!(
        after.updated_at_ms, before.updated_at_ms,
        "冲突不得改动更新时间"
    );
}

#[test]
fn rebinding_the_root_advances_the_revision_and_refreshes_the_safe_display() {
    let registry = registry();
    let created = registry
        .create(managed_request(
            "/roots/secret-parent/work",
            "work",
            WorkspacePermissionScope::Read,
        ))
        .expect("create");
    assert_eq!(created.root_display, "work");
    assert!(
        !created.root_display.contains("secret-parent"),
        "显示名不得带出父目录"
    );

    let rebound = registry
        .rebind_root(
            &created.workspace_id,
            created.revision,
            Path::new("/roots/other-parent/work2"),
            NOW_MS + 1_000,
        )
        .expect("rebind root");

    assert_eq!(rebound.revision, created.revision + 1, "换根必须 +1");
    assert_eq!(rebound.root_display, "work2", "显示名必须换成新叶子");
    assert!(
        !rebound.root_display.contains("secret-parent"),
        "旧父目录名不得残留"
    );
    assert!(
        !rebound.root_display.contains("other-parent"),
        "新父目录名也不得出现"
    );
}

#[test]
fn the_safe_root_display_never_carries_a_parent_or_a_drive_letter() {
    let display = safe_root_display(Path::new(r"C:\Users\Someone\secret-parent\work"));
    assert_eq!(display, "work", "只能取叶子目录名");
    assert!(!display.contains("secret-parent"), "不得带出父目录名");
    assert!(!display.contains('\\'), "不得带出路径分隔符");
    assert!(!display.contains(':'), "不得带出盘符");
    assert!(display.len() <= MAX_ROOT_DISPLAY_BYTES, "长度受限");

    let drive_root = safe_root_display(Path::new(r"C:\"));
    assert_eq!(
        drive_root, ROOT_DISPLAY_PLACEHOLDER,
        "盘根没有叶子，必须落到占位串"
    );
    assert!(!drive_root.contains(':'), "占位串不得是盘符");
    assert!(!drive_root.contains('\\'), "占位串不得含分隔符");

    let overlong = safe_root_display(Path::new(&"x".repeat(200)));
    assert_eq!(overlong.len(), MAX_ROOT_DISPLAY_BYTES, "超长叶子必须截断");
    assert!(!overlong.contains('/'), "截断后不得出现分隔符");
    assert!(!overlong.contains('\\'), "截断后不得出现分隔符");
}

#[test]
fn closing_revokes_the_workspace_and_refuses_a_later_selection() {
    let registry = registry();
    let created = registry
        .create(managed_request(
            "/roots/epsilon",
            "epsilon",
            WorkspacePermissionScope::Write,
        ))
        .expect("create");
    registry
        .select(&created.workspace_id, created.revision)
        .expect("select");

    let closed = registry
        .close(&created.workspace_id, created.revision, NOW_MS + 1_000)
        .expect("close");

    assert_eq!(closed.status, WorkspaceStatus::Revoked, "关闭即吊销");
    assert_eq!(closed.revision, created.revision + 1, "吊销必须 +1");
    assert!(!closed.can_select, "吊销后不得可选");

    let refused = registry
        .select(&closed.workspace_id, closed.revision)
        .expect_err("已吊销的工作区不得被选中");
    assert_eq!(refused, WorkspaceError::Revoked);
}

#[test]
fn the_write_capability_follows_the_confirmed_scope() {
    let registry = registry();
    let read = registry
        .create(managed_request(
            "/roots/zeta",
            "zeta",
            WorkspacePermissionScope::Read,
        ))
        .expect("create read");
    assert!(!read.can_write, "Read 作用域不得可写");

    let write = registry
        .create(managed_request(
            "/roots/eta",
            "eta",
            WorkspacePermissionScope::Write,
        ))
        .expect("create write");
    assert!(write.can_write, "Write 作用域必须可写");

    let approval = registry
        .create(managed_request(
            "/roots/theta",
            "theta",
            WorkspacePermissionScope::ApprovalWrite,
        ))
        .expect("create approval write");
    assert!(
        approval.can_write,
        "ApprovalWrite 作用域必须可写（审批是另一道闸）"
    );
}

#[test]
fn an_invalid_display_name_is_rejected() {
    let registry = registry();
    let too_long = "n".repeat(MAX_DISPLAY_NAME_BYTES + 1);

    for bad in ["", "bad\nname", too_long.as_str()] {
        let error = registry
            .create(managed_request(
                "/roots/iota",
                bad,
                WorkspacePermissionScope::Read,
            ))
            .expect_err("非法显示名必须被拒");
        assert!(
            matches!(error, WorkspaceError::InvalidDisplayName { .. }),
            "非法显示名 {bad:?} 应报 InvalidDisplayName，实际报 {error:?}"
        );
    }

    assert!(registry.list().is_empty(), "被拒的创建不得留下任何工作区");
}

#[test]
fn a_temporary_workspace_carries_its_expiry() {
    let registry = registry();
    let expires_at = NOW_MS + 3_600_000;
    let temporary = registry
        .create_temporary(
            Path::new("/roots/temp"),
            "temp",
            WorkspacePermissionScope::Read,
            NOW_MS,
            expires_at,
        )
        .expect("create temporary");

    assert_eq!(temporary.root_kind, WorkspaceRootKind::Temporary);
    assert_eq!(temporary.expires_at, Some(expires_at), "过期时间必须透传");

    let listed = registry.list();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].expires_at,
        Some(expires_at),
        "读回来的过期时间必须一致"
    );
}

#[test]
fn the_wire_form_carries_no_absolute_root_path() {
    let registry = registry();
    let created = registry
        .create(managed_request(
            "/roots/secret-parent/work",
            "work",
            WorkspacePermissionScope::Read,
        ))
        .expect("create");

    let json = serde_json::to_string(&created).expect("serialize");
    assert!(
        !json.contains("secret-parent"),
        "序列化不得带出父目录：{json}"
    );
    assert!(!json.contains("/roots"), "序列化不得带出根路径：{json}");
    assert!(
        json.contains("\"root_display\":\"work\""),
        "序列化只应带出安全显示名：{json}"
    );
}

#[test]
fn a_deserialized_workspace_never_trusts_a_client_supplied_capability() {
    let json = r#"{
        "workspace_id": "6f1c2d54-9a3b-4c7e-8f21-0b5d3e7a1c90",
        "display_name": "client-supplied",
        "root_kind": "managed",
        "root_display": "work",
        "permission_scope": "read",
        "revision": 7,
        "status": "ready",
        "can_select": true,
        "can_create_temporary": true,
        "can_write": true,
        "expires_at": null,
        "source": "managed",
        "updated_at_ms": 42
    }"#;

    let workspace: Workspace = serde_json::from_str(json).expect("deserialize");

    assert!(
        !workspace.can_write,
        "can_write 只能由 permission_scope 推导，客户端传什么都不算"
    );
    let expected_id: WorkspaceId = "6f1c2d54-9a3b-4c7e-8f21-0b5d3e7a1c90"
        .parse()
        .expect("parse workspace id");
    assert_eq!(workspace.workspace_id, expected_id);
    assert_eq!(workspace.revision, 7);
}
