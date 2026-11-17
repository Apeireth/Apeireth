//! The A3 contract: a workspace registration survives a restart.
//!
//! A "restart" here is literal: the registry is dropped, every file handle is
//! gone, and a **new** registry is opened on the same data root. Every
//! assertion is decidable from the durable state on disk — the tests read the
//! record files back, so a green suite can never be produced by an in-memory
//! shortcut.

use std::fs;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::ids::WorkspaceId;
use apeireth_runtime::canonical::workspace::{
    FileWorkspaceRegistry, Workspace, WorkspaceCreateRequest, WorkspaceError,
    WorkspacePermissionScope, WorkspaceRegistry, WorkspaceRootKind, WorkspaceStatus,
};
use tempfile::TempDir;

const NOW_MS: u64 = 1_700_000_000_000;

/// A temporary data root: the registry lives here, not in the real data dir.
fn data_root() -> TempDir {
    tempfile::tempdir().expect("temp data root")
}

/// A managed-workspace request for `root`.
fn managed_request(
    root: &Path,
    display_name: &str,
    permission_scope: WorkspacePermissionScope,
) -> WorkspaceCreateRequest {
    WorkspaceCreateRequest {
        root_kind: WorkspaceRootKind::Managed,
        root: root.to_path_buf(),
        display_name: display_name.to_string(),
        permission_scope,
        expires_at: None,
        source: Some("test".to_string()),
        now_ms: NOW_MS,
    }
}

/// The record file of one workspace inside a data root.
fn record_file(root: &Path, workspace_id: &WorkspaceId) -> PathBuf {
    root.join("workspaces")
        .join(workspace_id.to_string())
        .join("workspace.json")
}

/// The raw text of one workspace record.
fn record_text(root: &Path, workspace_id: &WorkspaceId) -> String {
    fs::read_to_string(record_file(root, workspace_id)).expect("record file")
}

#[test]
fn a_workspace_is_listed_again_after_a_restart_with_every_field_unchanged() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let created = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "默认工作区",
                WorkspacePermissionScope::Write,
            ))
            .expect("create")
    };

    // Restart: the registry is gone; a brand new one reads the data root back.
    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    let listed = reopened.list();
    assert_eq!(listed.len(), 1, "重启后必须仍能列出那一个工作区");
    assert_eq!(
        listed[0], created,
        "重启后逐字段一致 (身份/展示名/根类型/根/作用域/revision/状态/能力/来源/时间)"
    );
    assert_eq!(reopened.get(&created.workspace_id), Some(created.clone()));
}

#[test]
fn the_revision_keeps_moving_forward_across_a_restart() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let created = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "revision",
                WorkspacePermissionScope::Read,
            ))
            .expect("create")
    };
    assert_eq!(created.revision, 0, "新建工作区的 revision 从 0 起");

    let advanced = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
        registry
            .change_permission_scope(
                &created.workspace_id,
                created.revision,
                WorkspacePermissionScope::Write,
                NOW_MS + 1_000,
            )
            .expect("change scope")
    };
    assert_eq!(advanced.revision, 1, "作用域变化恰好 +1");

    // Restart, then move the revision forward again from the persisted value.
    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    let restored = reopened
        .get(&created.workspace_id)
        .expect("still there after restart");
    assert_eq!(
        restored.revision, 1,
        "重启后 revision 不得重置或回退 (必须仍是 1)"
    );
    assert_eq!(
        restored.permission_scope,
        WorkspacePermissionScope::Write,
        "重启后作用域必须还是落盘时那个"
    );

    let again = reopened
        .change_permission_scope(
            &created.workspace_id,
            restored.revision,
            WorkspacePermissionScope::ApprovalWrite,
            NOW_MS + 2_000,
        )
        .expect("change scope after restart");
    assert_eq!(again.revision, 2, "重启后继续 +1, 不得从 0 重新计数");
    assert_eq!(
        again.permission_scope,
        WorkspacePermissionScope::ApprovalWrite
    );

    // And the fact is durable, not just in memory.
    let after = FileWorkspaceRegistry::open(data.path()).expect("reopen again");
    let final_state = after.get(&created.workspace_id).expect("durable");
    assert_eq!(final_state.revision, 2, "落盘的 revision 必须是 2");
    assert_eq!(
        final_state.permission_scope,
        WorkspacePermissionScope::ApprovalWrite
    );
}

#[test]
fn a_stale_revision_is_refused_on_restore_and_changes_nothing() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let created = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "stale",
                WorkspacePermissionScope::Read,
            ))
            .expect("create")
    };
    {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
        registry
            .change_permission_scope(
                &created.workspace_id,
                created.revision,
                WorkspacePermissionScope::Write,
                NOW_MS + 1_000,
            )
            .expect("change scope");
    }

    let before = record_text(data.path(), &created.workspace_id);
    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");

    let error = reopened
        .restore_exact(&created.workspace_id, created.revision)
        .expect_err("陈旧 revision 必须被拒");
    match error {
        WorkspaceError::RevisionConflict { expected, actual } => {
            assert_eq!(expected, 0, "冲突里必须报出调用方期望的 revision");
            assert_eq!(actual, 1, "冲突里必须报出实际 revision");
        }
        other => panic!("陈旧 revision 必须是 RevisionConflict, 实际是 {other:?}"),
    }

    let after = record_text(data.path(), &created.workspace_id);
    assert_eq!(before, after, "被拒的恢复必须让状态零变化");
    let state = reopened.get(&created.workspace_id).expect("still there");
    assert_eq!(state.revision, 1, "被拒的恢复不得改动 revision");
    assert_eq!(state.permission_scope, WorkspacePermissionScope::Write);
    assert_eq!(state.status, WorkspaceStatus::Ready);
}

#[test]
fn a_revoked_workspace_stays_revoked_after_a_restart_and_cannot_be_revived() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let revoked = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        let created = registry
            .create(managed_request(
                &root,
                "revoked",
                WorkspacePermissionScope::Write,
            ))
            .expect("create");
        registry
            .close(&created.workspace_id, created.revision, NOW_MS + 1_000)
            .expect("close")
    };
    assert_eq!(revoked.status, WorkspaceStatus::Revoked);
    assert_eq!(revoked.revision, 1, "吊销恰好 +1");
    assert!(!revoked.can_select, "吊销后不得可选");

    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    let listed = reopened.list();
    assert_eq!(listed.len(), 1, "吊销的工作区仍是登记事实, 墓碑留着");
    assert_eq!(listed[0].status, WorkspaceStatus::Revoked);
    assert_eq!(listed[0].revision, 1, "吊销后的 revision 必须跨重启保留");

    // The exact revision that carried the revocation is refused: it does not
    // hand a ready workspace back.
    let error = reopened
        .restore_exact(&revoked.workspace_id, revoked.revision)
        .expect_err("已吊销的工作区不得被恢复");
    assert!(
        matches!(error, WorkspaceError::Revoked),
        "已吊销必须报 Revoked, 实际是 {error:?}"
    );

    // A stale revision is still a conflict, and the tombstone is untouched.
    let stale = reopened
        .restore_exact(&revoked.workspace_id, revoked.revision - 1)
        .expect_err("陈旧 revision 必须被拒");
    match stale {
        WorkspaceError::RevisionConflict { expected, actual } => {
            assert_eq!(expected, 0);
            assert_eq!(actual, 1);
        }
        other => panic!("陈旧 revision 必须是 RevisionConflict, 实际是 {other:?}"),
    }
    assert_eq!(
        reopened.get(&revoked.workspace_id).map(|w| w.status),
        Some(WorkspaceStatus::Revoked),
        "被拒的恢复不得复活已吊销工作区"
    );
}

#[test]
fn the_record_holds_the_absolute_root_while_the_line_format_never_does() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let created = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "leaky",
                WorkspacePermissionScope::Write,
            ))
            .expect("create")
    };

    let text = record_text(data.path(), &created.workspace_id);
    let root_text = root.to_str().expect("utf-8 root");
    // The root is physically in the file (JSON escapes the separators, so the
    // escaped form is what the bytes hold).
    let escaped_root = serde_json::to_string(root_text).expect("escape");
    assert!(
        text.contains(&escaped_root),
        "落盘文件必须带着受管根 —— 否则重启后无从解析工作目录"
    );
    // And it is there as the fact it claims to be.
    let doc: serde_json::Value = serde_json::from_str(&text).expect("record json");
    assert_eq!(
        doc["body"]["managed_root"].as_str(),
        Some(root_text),
        "落盘文件里的受管根必须就是宿主给的那一个"
    );
    assert_eq!(
        doc["body"]["workspace_id"].as_str(),
        Some(created.workspace_id.to_string().as_str()),
        "落盘文件必须带着身份"
    );
    assert_eq!(
        doc["body"]["revision"].as_u64(),
        Some(0),
        "落盘文件必须带着 revision"
    );

    // The line format is the read surface's source: it must not carry the root.
    let line = serde_json::to_string(&created).expect("line format");
    assert!(
        !line.contains(root_text),
        "线格式不得带绝对根, 实际是 {line}"
    );
    assert!(
        !line.contains(&escaped_root),
        "线格式不得带绝对根 (含转义形), 实际是 {line}"
    );
    assert!(
        !line.contains("managed_root"),
        "线格式不得带受管根字段, 实际是 {line}"
    );
    assert!(
        !line.contains(":\\\\"),
        "线格式不得出现盘符路径, 实际是 {line}"
    );
}

#[test]
fn several_workspaces_in_one_data_root_do_not_overwrite_each_other() {
    let data = data_root();
    let first_root = data_root();
    let second_root = data_root();
    let third_root = data_root();

    let (first, second, third) = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        let first = registry
            .create(managed_request(
                first_root.path(),
                "same-name",
                WorkspacePermissionScope::Read,
            ))
            .expect("create first");
        let second = registry
            .create(managed_request(
                second_root.path(),
                "same-name",
                WorkspacePermissionScope::Write,
            ))
            .expect("create second");
        let third = registry
            .create(managed_request(
                third_root.path(),
                "same-name",
                WorkspacePermissionScope::ApprovalWrite,
            ))
            .expect("create third");
        (first, second, third)
    };
    assert_ne!(first.workspace_id, second.workspace_id);
    assert_ne!(second.workspace_id, third.workspace_id);

    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    let listed = reopened.list();
    assert_eq!(listed.len(), 3, "三个工作区都要在");
    for expected in [&first, &second, &third] {
        let found = reopened.get(&expected.workspace_id).expect("still there");
        assert_eq!(
            &found, expected,
            "每个工作区都要逐字段保持自己的事实, 不得互相覆盖"
        );
    }
    // And each one owns its own file.
    for expected in [&first, &second, &third] {
        let text = record_text(data.path(), &expected.workspace_id);
        assert!(text.contains(&expected.workspace_id.to_string()));
    }
    let scopes: Vec<_> = listed.iter().map(|w| w.permission_scope).collect();
    assert!(scopes.contains(&WorkspacePermissionScope::Read));
    assert!(scopes.contains(&WorkspacePermissionScope::Write));
    assert!(scopes.contains(&WorkspacePermissionScope::ApprovalWrite));
}

#[test]
fn the_current_selection_survives_a_restart() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let created = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        let created = registry
            .create(managed_request(
                &root,
                "selected",
                WorkspacePermissionScope::Write,
            ))
            .expect("create");
        registry
            .select(&created.workspace_id, created.revision)
            .expect("select");
        created
    };

    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    assert_eq!(
        reopened.current_selection(),
        Some(created.workspace_id),
        "当前选中态必须跨重启保留, 不许静默变成 None"
    );
}

#[test]
fn a_record_that_cannot_be_read_refuses_to_open_instead_of_being_dropped() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    let created = {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "corrupt",
                WorkspacePermissionScope::Write,
            ))
            .expect("create")
    };

    // Hand-edit the record so it is no longer a valid envelope.
    fs::write(
        record_file(data.path(), &created.workspace_id),
        b"{ this is not a stored doc }",
    )
    .expect("corrupt the record");

    let error = FileWorkspaceRegistry::open(data.path())
        .err()
        .expect("坏记录必须让注册表开不出来");
    assert!(
        matches!(error, WorkspaceError::Storage { .. }),
        "坏记录必须报存储故障, 实际是 {error:?}"
    );
}

#[test]
fn a_restored_workspace_reports_the_derived_facts_consistently() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "derived",
                WorkspacePermissionScope::Read,
            ))
            .expect("create");
    }

    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    let restored = reopened.list().pop().expect("one workspace");
    assert_eq!(restored.can_select, true, "Ready 状态的工作区必须可选");
    assert_eq!(restored.can_create_temporary, true);
    assert_eq!(restored.can_write, false, "Read 作用域恢复后不得可写");
    assert_eq!(
        restored.root_display,
        root.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "展示名必须由受管根重算, 且只给叶子名"
    );
    assert!(
        !restored.root_display.contains('\\') && !restored.root_display.contains('/'),
        "展示名不得带路径分隔符, 实际是 {}",
        restored.root_display
    );
}

#[test]
fn a_workspace_can_be_found_by_its_managed_root() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        registry
            .create(managed_request(
                &root,
                "found",
                WorkspacePermissionScope::Write,
            ))
            .expect("create");
    }

    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    let found = reopened.find_by_root(&root).expect("by root");
    assert_eq!(found.root_display.len() <= 64, true);
    assert!(reopened.find_by_root(Path::new("C:/nowhere")).is_none());
}

#[test]
fn a_revoked_workspace_is_never_handed_back_for_host_binding() {
    let data = data_root();
    let managed = data_root();
    let root = managed.path().to_path_buf();

    {
        let registry = FileWorkspaceRegistry::open(data.path()).expect("open registry");
        let created = registry
            .create(managed_request(
                &root,
                "tombstone",
                WorkspacePermissionScope::Write,
            ))
            .expect("create");
        registry
            .close(&created.workspace_id, created.revision, NOW_MS + 1_000)
            .expect("close");
    }

    let reopened = FileWorkspaceRegistry::open(data.path()).expect("reopen registry");
    assert!(
        reopened.find_by_root(&root).is_none(),
        "已吊销的工作区不得被交回宿主去复用 (吊销是终态)"
    );
    assert_eq!(reopened.list().len(), 1, "墓碑本身仍是登记事实");
}
