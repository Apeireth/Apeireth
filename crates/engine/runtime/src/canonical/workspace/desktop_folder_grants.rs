//! Host-selected read-only folders with a process-local lifetime.

use super::*;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Source projected by the desktop catalog for an existing-folder grant.
pub const DESKTOP_GRANTED_WORKSPACE_SOURCE: &str = "desktop_temporary";
/// Maximum duration for a native existing-folder grant, in seconds.
pub const DESKTOP_FOLDER_LEASE_SECONDS: u64 = 30 * 60;
const GRANT_LIFETIME: Duration = Duration::from_secs(DESKTOP_FOLDER_LEASE_SECONDS);
const MAX_LIVE_GRANTS: usize = 32;

/// Windows volume and file identity of the handle held by the native picker.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopFolderIdentity {
    /// The numeric volume serial represented by 16 lowercase hex digits.
    pub volume_serial_number: String,
    /// The 16 FILE_ID_128 bytes in their original order, as lowercase hex.
    pub file_id: String,
}

impl DesktopFolderIdentity {
    fn valid(&self) -> bool {
        fn exact_hex(value: &str, size: usize) -> bool {
            value.len() == size
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }
        exact_hex(&self.volume_serial_number, 16) && exact_hex(&self.file_id, 32)
    }
}

struct DesktopFolderLease {
    root: PathBuf,
    identity: DesktopFolderIdentity,
    revision: u64,
    expires_at: u64,
    deadline: Instant,
    _retained_root: platform::RetainedDirectory,
}

impl DesktopFolderLease {
    fn active(&self, now: Instant) -> bool {
        now < self.deadline && epoch_ms().is_some_and(|epoch| epoch < u128::from(self.expires_at))
    }

    fn matches(&self, workspace: &Workspace, now: Instant) -> bool {
        self.active(now)
            && workspace.root == self.root
            && workspace.revision == self.revision
            && workspace.expires_at == Some(self.expires_at)
            && workspace.root_kind == WorkspaceRootKind::Temporary
            && workspace.permission_scope == WorkspacePermissionScope::Read
            && workspace.status == WorkspaceStatus::Ready
    }
}

#[derive(Default)]
pub(super) struct DesktopFolderLeases(BTreeMap<WorkspaceId, Arc<DesktopFolderLease>>);

impl DesktopFolderLeases {
    pub(super) fn remove(&mut self, id: &WorkspaceId) {
        self.0.remove(id);
    }

    fn prune(&mut self, now: Instant) {
        self.0.retain(|_, lease| lease.active(now));
    }
}

fn epoch_ms() -> Option<u128> {
    Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
}

pub(super) fn folder_grant_unavailable() -> WorkspaceError {
    WorkspaceError::InvalidRequest {
        reason: "the temporary desktop folder grant is unavailable or expired".into(),
    }
}

fn folder_refused() -> WorkspaceError {
    WorkspaceError::InvalidRequest {
        reason: "the folder must be an allowed native local directory with the selected identity"
            .into(),
    }
}

impl Workspace {
    pub(super) fn is_desktop_folder_grant(&self) -> bool {
        self.source == DESKTOP_GRANTED_WORKSPACE_SOURCE
    }

    pub(super) fn require_regular_root(&self) -> Result<(), WorkspaceError> {
        if self.is_desktop_folder_grant() {
            return Err(WorkspaceError::InvalidRequest {
                reason: "a desktop folder grant can only be revoked and granted anew".into(),
            });
        }
        Ok(())
    }
}

impl FileWorkspaceRegistry {
    pub(super) fn provision_desktop_folder_grant(
        &self,
        display_name: &str,
        directory: &Path,
        identity: &DesktopFolderIdentity,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let deadline = Instant::now() + GRANT_LIFETIME;
        if !identity.valid() {
            return Err(folder_refused());
        }
        let selected = platform::SelectedDirectory::open(directory, &self.root)?;
        if &selected.identity != identity {
            return Err(folder_refused());
        }
        let retained_root = selected.retain_root()?;
        let expires_at = now_ms
            .checked_add(GRANT_LIFETIME.as_millis() as u64)
            .ok_or_else(folder_refused)?;
        let mut workspace = Workspace::new_temporary(
            &selected.root,
            display_name,
            WorkspacePermissionScope::Read,
            now_ms,
            expires_at,
        )?;
        workspace.can_create_temporary = false;
        workspace.source = DESKTOP_GRANTED_WORKSPACE_SOURCE.into();
        let record = self.workspace_file(&workspace.workspace_id);
        let lock = storage_atomic::lock_path_for(&record);
        storage_atomic::with_file_lock(&lock, || {
            let id = workspace.workspace_id;
            let now = Instant::now();
            let mut leases = self.desktop_folder_leases.lock();
            leases.prune(now);
            if leases.0.len() >= MAX_LIVE_GRANTS {
                return Err(WorkspaceError::InvalidRequest {
                    reason: "too many live desktop folder grants".into(),
                });
            }
            if record
                .try_exists()
                .map_err(|_| folder_grant_unavailable())?
            {
                return Err(folder_grant_unavailable());
            }
            if now >= deadline || epoch_ms().is_none_or(|epoch| epoch >= u128::from(expires_at)) {
                return Err(folder_grant_unavailable());
            }
            self.write_workspace_record(&workspace)
                .map_err(|_| WorkspaceError::Storage {
                    reason: "the temporary folder grant could not be stored".into(),
                })?;
            leases.0.insert(
                id,
                Arc::new(DesktopFolderLease {
                    root: selected.root.clone(),
                    identity: selected.identity.clone(),
                    revision: workspace.revision,
                    deadline,
                    expires_at,
                    _retained_root: retained_root,
                }),
            );
            drop(leases);
            self.commit(workspace.clone());
            Ok(workspace)
        })
        .map_err(|_| folder_grant_unavailable())?
    }

    pub(super) fn require_live_folder_grant(
        &self,
        workspace: &Workspace,
    ) -> Result<(), WorkspaceError> {
        if !workspace.is_desktop_folder_grant() {
            return Ok(());
        }
        let now = Instant::now();
        let mut leases = self.desktop_folder_leases.lock();
        leases.prune(now);
        if leases
            .0
            .get(&workspace.workspace_id)
            .is_some_and(|lease| lease.matches(workspace, now))
        {
            Ok(())
        } else {
            Err(folder_grant_unavailable())
        }
    }

    pub(super) fn project_folder_grant_availability(&self, workspace: &mut Workspace) {
        if !workspace.is_desktop_folder_grant() {
            return;
        }
        workspace.can_create_temporary = false;
        if workspace.status == WorkspaceStatus::Ready
            && self.require_live_folder_grant(workspace).is_err()
        {
            workspace.status = WorkspaceStatus::Unavailable;
            workspace.can_select = false;
            workspace.can_write = false;
        }
    }

    /// The caller holds the durable record lock, which orders this effect with revocation.
    pub(super) fn access_granted_desktop_folder(
        &self,
        workspace: &Workspace,
        operation: &mut dyn FnMut(&Path, WorkspacePermissionScope),
    ) -> Result<(), WorkspaceError> {
        let lease = {
            let now = Instant::now();
            let mut leases = self.desktop_folder_leases.lock();
            leases.prune(now);
            leases
                .0
                .get(&workspace.workspace_id)
                .filter(|lease| lease.matches(workspace, now))
                .cloned()
                .ok_or_else(folder_grant_unavailable)?
        };
        let selected = platform::SelectedDirectory::open(&workspace.root, &self.root)?;
        if selected.root != workspace.root
            || selected.identity != lease.identity
            || !lease.matches(workspace, Instant::now())
        {
            return Err(folder_grant_unavailable());
        }
        operation(&selected.root, WorkspacePermissionScope::Read);
        Ok(())
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use super::*;
    use std::fmt::Write;
    use std::fs::{File, OpenOptions};
    use std::mem::{size_of, MaybeUninit};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;
    use std::path::{Component, Prefix};
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetDriveTypeW, GetFileInformationByHandleEx, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

    pub(super) struct RetainedDirectory {
        // Keep the directory object alive so another object cannot reuse its file ID.
        // Share deletion: this is an identity reference, not the use-time path pin.
        _handle: File,
    }

    #[cfg(test)]
    impl RetainedDirectory {
        pub(super) fn identity(&self) -> Result<DesktopFolderIdentity, WorkspaceError> {
            identity(&self._handle)
        }
    }

    pub(super) struct SelectedDirectory {
        pub(super) root: PathBuf,
        pub(super) identity: DesktopFolderIdentity,
        // Holding these handles with neither write nor delete sharing keeps
        // the checked path stable through the short operation using it.
        _pinned: Vec<File>,
    }

    impl SelectedDirectory {
        pub(super) fn open(directory: &Path, data: &Path) -> Result<Self, WorkspaceError> {
            validate_local_path(directory)?;
            // The sent path and the resolved path may differ in case or
            // short-name spelling. Pin the supplied path first to reject any
            // junction in the path actually selected by the picker.
            let mut pinned = pin_path(directory)?;
            let canonical = fs::canonicalize(directory).map_err(|_| folder_refused())?;
            validate_local_path(&canonical)?;
            check_protected_path(&canonical, data)?;
            let pinned_identity = identity(pinned.last().expect("validated absolute path"))?;
            if canonical != directory {
                pinned.extend(pin_path(&canonical)?);
                if identity(pinned.last().expect("validated absolute path"))? != pinned_identity {
                    return Err(folder_refused());
                }
            }
            Ok(Self {
                root: canonical,
                identity: pinned_identity,
                _pinned: pinned,
            })
        }

        pub(super) fn retain_root(&self) -> Result<RetainedDirectory, WorkspaceError> {
            let handle = open_directory(
                &self.root,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            )?;
            if identity(&handle)? != self.identity {
                return Err(folder_refused());
            }
            Ok(RetainedDirectory { _handle: handle })
        }
    }

    fn validate_local_path(path: &Path) -> Result<(), WorkspaceError> {
        let mut parts = path.components();
        let Some(Component::Prefix(prefix)) = parts.next() else {
            return Err(folder_refused());
        };
        let (Prefix::Disk(letter) | Prefix::VerbatimDisk(letter)) = prefix.kind() else {
            return Err(folder_refused());
        };
        if parts.next() != Some(Component::RootDir) || parts.as_path().as_os_str().is_empty()
            || parts.clone().count() > 64
            || !parts.all(|part| matches!(part, Component::Normal(name) if name.to_str().is_some_and(
                |name| !name.ends_with([' ', '.']) && !name.chars().any(|c| c.is_control() || c == ':')
            )))
        {
            return Err(folder_refused());
        }
        let drive = [u16::from(letter), ':' as u16, '\\' as u16, 0];
        // SAFETY: drive is a NUL-terminated root on the stack for the call.
        if unsafe { GetDriveTypeW(drive.as_ptr()) } != DRIVE_FIXED {
            return Err(folder_refused());
        }
        Ok(())
    }

    fn check_protected_path(path: &Path, data: &Path) -> Result<(), WorkspaceError> {
        const SENSITIVE: &[&str] = &[
            ".ssh",
            ".aws",
            ".gnupg",
            ".kube",
            ".docker",
            ".git",
            ".secret",
            ".secrets",
            "appdata",
            "windows",
            "program files",
            "program files (x86)",
            "programdata",
            "$recycle.bin",
            "system volume information",
        ];
        if path
            .components()
            .filter_map(|part| part.as_os_str().to_str())
            .any(|part| {
                SENSITIVE
                    .iter()
                    .any(|sensitive| part.eq_ignore_ascii_case(sensitive))
            })
        {
            return Err(folder_refused());
        }
        let data = fs::canonicalize(data).map_err(|_| folder_refused())?;
        let contains =
            |protected: &Path| path.starts_with(protected) || protected.starts_with(path);
        if contains(&data)
            || std::env::var_os("USERPROFILE")
                .and_then(|home| fs::canonicalize(home).ok())
                .is_some_and(|home| home.starts_with(path))
            || [
                "SystemRoot",
                "ProgramFiles",
                "ProgramData",
                "APPDATA",
                "LOCALAPPDATA",
            ]
            .iter()
            .filter_map(std::env::var_os)
            .filter_map(|path| fs::canonicalize(path).ok())
            .any(|protected| contains(&protected))
        {
            return Err(folder_refused());
        }
        Ok(())
    }

    fn pin_path(path: &Path) -> Result<Vec<File>, WorkspaceError> {
        let mut ancestors: Vec<_> = path.ancestors().filter(|part| part.is_absolute()).collect();
        ancestors.reverse();
        ancestors
            .into_iter()
            .map(|path| open_directory(path, FILE_SHARE_READ))
            .collect()
    }

    fn open_directory(path: &Path, share_mode: u32) -> Result<File, WorkspaceError> {
        let handle = OpenOptions::new()
            .read(true)
            .share_mode(share_mode)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|_| folder_refused())?;
        let metadata = handle.metadata().map_err(|_| folder_refused())?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(folder_refused());
        }
        Ok(handle)
    }

    pub(super) fn identity(handle: &File) -> Result<DesktopFolderIdentity, WorkspaceError> {
        let mut result = MaybeUninit::<FILE_ID_INFO>::uninit();
        // SAFETY: the handle remains valid and result owns writable storage for FILE_ID_INFO.
        if unsafe {
            GetFileInformationByHandleEx(
                handle.as_raw_handle().cast(),
                FileIdInfo,
                result.as_mut_ptr().cast(),
                size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
        {
            return Err(folder_refused());
        }
        // SAFETY: the system call succeeded and initialized FILE_ID_INFO.
        let result = unsafe { result.assume_init() };
        let file_id =
            result
                .FileId
                .Identifier
                .iter()
                .fold(String::with_capacity(32), |mut hex, byte| {
                    write!(hex, "{byte:02x}").expect("format into String");
                    hex
                });
        Ok(DesktopFolderIdentity {
            volume_serial_number: format!("{:016x}", result.VolumeSerialNumber),
            file_id,
        })
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, FileWorkspaceRegistry, PathBuf) {
        // The OS temp directory can be in protected AppData. Keep the fixture
        // under the test checkout, and account for its own temp-dir lifetime.
        let tmp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let registry = FileWorkspaceRegistry::open(tmp.path().join("runtime-data")).unwrap();
        let folder = tmp.path().join("selected-folder");
        fs::create_dir(&folder).unwrap();
        (tmp, registry, folder)
    }

    fn grant(registry: &FileWorkspaceRegistry, folder: &Path) -> Workspace {
        let selected = platform::SelectedDirectory::open(folder, registry.root()).unwrap();
        registry
            .grant_desktop_folder(
                "Selected folder",
                folder,
                &selected.identity,
                epoch_ms().unwrap() as u64,
            )
            .unwrap()
    }

    #[test]
    fn grant_is_read_only_and_identity_is_pinned_through_the_effect() {
        let (tmp, registry, folder) = fixture();
        let workspace = grant(&registry, &folder);
        let id = workspace.workspace_id;
        let revision = workspace.revision;
        assert_eq!(workspace.root_kind, WorkspaceRootKind::Temporary);
        assert_eq!(workspace.source, DESKTOP_GRANTED_WORKSPACE_SOURCE);
        assert_eq!(workspace.permission_scope, WorkspacePermissionScope::Read);
        assert!(!workspace.can_write && !workspace.can_create_temporary);
        let replacement = tmp.path().join("renamed-folder");
        let mut checks = 0;
        registry
            .with_desktop_workspace_root(&id, revision, &mut |path, scope| {
                assert_eq!(path, fs::canonicalize(&folder).unwrap());
                assert_eq!(scope, WorkspacePermissionScope::Read);
                assert!(
                    fs::rename(&folder, &replacement).is_err(),
                    "an effect must pin the source directory"
                );
                checks += 1;
            })
            .unwrap();
        assert_eq!(checks, 1);
        assert!(registry
            .change_permission_scope(&id, revision, WorkspacePermissionScope::ApprovalWrite, 10)
            .is_err());
        assert!(registry
            .rebind_root(&id, revision, &replacement, 10)
            .is_err());

        // The same spelling at the same revision cannot point to a new folder.
        fs::rename(&folder, &replacement).unwrap();
        {
            let leases = registry.desktop_folder_leases.lock();
            let lease = &leases.0[&id];
            assert_eq!(lease._retained_root.identity().unwrap(), lease.identity);
        }
        fs::create_dir(&folder).unwrap();
        assert!(registry
            .with_desktop_workspace_root(&id, revision, &mut |_, _| {
                panic!("replacement unexpectedly obtained a grant");
            })
            .is_err());
        // Retaining the identity doesn't require denying removal of the name.
        fs::remove_dir(&replacement).unwrap();
    }

    #[test]
    fn revocation_expiry_and_registry_reopen_refuse_exact_revision() {
        let (_tmp, registry, folder) = fixture();
        let workspace = grant(&registry, &folder);
        let id = workspace.workspace_id;
        let revision = workspace.revision;
        let reopened = FileWorkspaceRegistry::open(registry.root()).unwrap();
        assert_eq!(
            reopened.get(&id).unwrap().status,
            WorkspaceStatus::Unavailable
        );
        assert!(reopened.restore_exact(&id, revision).is_err());
        let mut refuse = |_: &Path, _: WorkspacePermissionScope| panic!("no effect was authorized");
        assert!(reopened
            .with_desktop_workspace_root(&id, revision, &mut refuse)
            .is_err());
        {
            let mut leases = registry.desktop_folder_leases.lock();
            Arc::get_mut(leases.0.get_mut(&id).unwrap())
                .unwrap()
                .deadline = Instant::now();
        }
        assert_eq!(
            registry.get(&id).unwrap().status,
            WorkspaceStatus::Unavailable
        );
        assert!(registry
            .with_desktop_workspace_root(&id, revision, &mut refuse)
            .is_err());
        assert!(registry.select(&id, revision).is_err());
        let now = epoch_ms().unwrap() as u64;
        let closed = registry.close(&id, revision, now).unwrap();
        assert_eq!(closed.status, WorkspaceStatus::Revoked);
        assert_eq!(closed.revision, revision + 1);
        assert_eq!(
            registry.with_desktop_workspace_root(&id, closed.revision, &mut refuse),
            Err(WorkspaceError::Revoked)
        );
    }

    #[test]
    fn malformed_or_wrong_identity_and_protected_roots_are_refused() {
        let (tmp, registry, folder) = fixture();
        let selected = platform::SelectedDirectory::open(&folder, registry.root()).unwrap();
        let now = epoch_ms().unwrap() as u64;
        let mut wrong = selected.identity.clone();
        wrong.file_id = if wrong.file_id == "0".repeat(32) {
            "1"
        } else {
            "0"
        }
        .repeat(32);
        assert!(registry
            .grant_desktop_folder("Wrong", &folder, &wrong, now)
            .is_err());
        wrong.file_id.push('z');
        assert!(registry
            .grant_desktop_folder("Malformed", &folder, &wrong, now)
            .is_err());
        let protected = tmp.path().join(".ssh");
        fs::create_dir(&protected).unwrap();
        assert!(platform::SelectedDirectory::open(&protected, registry.root()).is_err());
        assert!(platform::SelectedDirectory::open(registry.root(), registry.root()).is_err());
        assert!(platform::SelectedDirectory::open(tmp.path(), registry.root()).is_err());
        assert!(registry.list().is_empty());
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub(super) struct RetainedDirectory;

    pub(super) struct SelectedDirectory {
        pub(super) root: PathBuf,
        pub(super) identity: DesktopFolderIdentity,
    }

    impl SelectedDirectory {
        pub(super) fn open(_: &Path, _: &Path) -> Result<Self, WorkspaceError> {
            Err(folder_grant_unavailable())
        }

        pub(super) fn retain_root(&self) -> Result<RetainedDirectory, WorkspaceError> {
            Err(folder_grant_unavailable())
        }
    }
}
