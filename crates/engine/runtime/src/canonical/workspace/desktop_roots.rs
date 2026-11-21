//! Host-owned desktop directories and the registry gate for scoped operations.

use super::*;
use std::io::{self, ErrorKind};

/// Source for desktop workspaces provisioned in their own host directory.
pub const DESKTOP_MANAGED_WORKSPACE_SOURCE: &str = "desktop_managed";
const DESKTOP_ROOTS_DIR: &str = "managed-workspace-roots";

pub(super) fn desktop_workspace_required() -> WorkspaceError {
    WorkspaceError::InvalidRequest {
        reason: "an isolated desktop workspace is required".to_string(),
    }
}

fn directory_error(error: io::Error) -> WorkspaceError {
    WorkspaceError::Storage {
        reason: format!(
            "isolated workspace directory is unavailable ({:?})",
            error.kind()
        ),
    }
}

fn is_local_directory(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return false;
        }
    }
    metadata.is_dir() && !metadata.file_type().is_symlink()
}

fn check_directory(path: &Path) -> io::Result<()> {
    if !is_local_directory(&fs::symlink_metadata(path)?) || fs::canonicalize(path)? != path {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "expected a real directory at the host-assigned location",
        ));
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir(path)
    }
}

fn create_desktop_root(parent: &Path, root: &Path) -> io::Result<()> {
    match create_private_directory(parent) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    check_directory(parent)?;
    create_private_directory(root)
}

fn sync_desktop_root(root: &Path) -> io::Result<()> {
    check_directory(root)?;
    // Windows has no equivalent directory fsync in the registry storage layer.
    #[cfg(unix)]
    for directory in root.ancestors().take(3) {
        fs::File::open(directory)?.sync_all()?;
    }
    Ok(())
}

fn remove_empty_root(root: &Path) {
    // An interrupted or uncertain registry commit can leave an empty directory.
    // A directory with user content must never be removed by rollback.
    if root
        .parent()
        .is_some_and(|parent| check_directory(parent).is_ok())
        && fs::symlink_metadata(root).is_ok_and(|metadata| is_local_directory(&metadata))
    {
        let _ = fs::remove_dir(root);
    }
}

impl FileWorkspaceRegistry {
    fn desktop_root_parent(&self) -> Result<PathBuf, WorkspaceError> {
        let data_root = fs::canonicalize(&self.root).map_err(directory_error)?;
        Ok(data_root.join(DESKTOP_ROOTS_DIR))
    }

    pub(super) fn provision_desktop_workspace(
        &self,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        if permission_scope == WorkspacePermissionScope::Write {
            return Err(WorkspaceError::InvalidRequest {
                reason: "isolated workspaces accept read or approval_write".to_string(),
            });
        }
        let parent = self.desktop_root_parent()?;
        let mut workspace =
            Workspace::new_managed(&parent, display_name, permission_scope, now_ms)?;
        workspace.root = parent.join(workspace.workspace_id.to_string());
        workspace.root_display = safe_root_display(&workspace.root);
        workspace.source = DESKTOP_MANAGED_WORKSPACE_SOURCE.to_string();
        let record = self.workspace_file(&workspace.workspace_id);
        let lock = storage_atomic::lock_path_for(&record);
        storage_atomic::with_file_lock(&lock, || {
            if record.try_exists().map_err(directory_error)? {
                return Err(WorkspaceError::Storage {
                    reason: "workspace record already exists".to_string(),
                });
            }
            create_desktop_root(&parent, &workspace.root).map_err(directory_error)?;
            if let Err(error) = sync_desktop_root(&workspace.root) {
                remove_empty_root(&workspace.root);
                return Err(directory_error(error));
            }
            if let Err(error) = self.write_workspace_record(&workspace) {
                // An error can follow the record's atomic replacement. Keep
                // the root unless absence of the record can be established.
                if matches!(record.try_exists(), Ok(false)) {
                    remove_empty_root(&workspace.root);
                }
                return Err(error);
            }
            self.commit(workspace.clone());
            Ok(workspace)
        })
        .map_err(directory_error)?
    }

    pub(super) fn access_desktop_workspace(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        operation: &mut dyn FnMut(&Path, WorkspacePermissionScope),
    ) -> Result<(), WorkspaceError> {
        let record = self.workspace_file(workspace_id);
        // An unknown ID must not create a metadata directory as a side effect.
        if !record.try_exists().map_err(directory_error)? {
            return Err(WorkspaceError::NotFound(*workspace_id));
        }
        let lock = storage_atomic::lock_path_for(&record);
        storage_atomic::with_file_lock(&lock, || {
            let workspace = self.read_workspace_from_disk(workspace_id)?;
            check_mutable_workspace(&workspace, expected_revision)?;
            if workspace.workspace_id != *workspace_id
                || workspace.status != WorkspaceStatus::Ready
                || workspace.root_kind != WorkspaceRootKind::Managed
                || workspace.source != DESKTOP_MANAGED_WORKSPACE_SOURCE
                || workspace.permission_scope == WorkspacePermissionScope::Write
            {
                return Err(desktop_workspace_required());
            }
            let parent = self.desktop_root_parent()?;
            let expected_root = parent.join(workspace_id.to_string());
            if workspace.root != expected_root {
                return Err(desktop_workspace_required());
            }
            check_directory(&parent).map_err(directory_error)?;
            check_directory(&expected_root).map_err(directory_error)?;
            operation(&expected_root, workspace.permission_scope);
            Ok(())
        })
        .map_err(directory_error)?
    }
}
