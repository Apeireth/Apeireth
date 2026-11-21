//! Workspace: the managed identity that workspace-scoped bindings point at.
//!
//! # What a workspace is
//!
//! A workspace is a **first-class managed identity**: a UUID, a managed root, a
//! backend-confirmed permission scope, and a monotonic `revision`. It is not
//! "a path on this machine" — the path is an implementation detail of the
//! host. Everything that binds to a workspace binds to the *identity* plus the
//! *revision*.
//!
//! # Why the root is private
//!
//! The absolute root is retained, because the runtime has to resolve a working
//! directory from it. It is not reachable through the public surface: there is
//! no public accessor, and the wire form does not carry it. A read surface can
//! therefore only ever project [`Workspace::root_display`], the bounded leaf
//! name produced by [`safe_root_display`].
//!
//! # Revision rules
//!
//! `revision` advances **only** when the workspace itself changes:
//!
//! * the managed root is rebound,
//! * the permission scope changes,
//! * the workspace is revoked.
//!
//! Selecting a workspace is a *registry-level* fact ("which workspace is the
//! current one"), not a workspace attribute, so selection does **not** move the
//! revision. Every mutation carries an `expected_revision`; a mismatch is
//! rejected with [`WorkspaceError::RevisionConflict`] and leaves the workspace
//! byte-for-byte unchanged, so a stale binding can never be applied to a
//! changed root or a changed scope.
//!
//! # Capability declarations are derived
//!
//! `can_select`, `can_create_temporary`, and `can_write` are computed, never
//! accepted from a caller: the first two follow `status`, the third follows
//! `permission_scope`. Because [`Workspace::root`] is private, no code outside
//! this module can build a workspace with a struct literal, so there is no way
//! to smuggle a widened capability in — and the wire form ignores any
//! capability field a client sends.
//!
//! # Honest scope
//!
//! This module is the domain layer: the types, the validation, the optimistic
//! concurrency rules. Two registry implementations ship here:
//! [`InMemoryWorkspaceRegistry`] — in-process and **not persisted** — and
//! [`FileWorkspaceRegistry`], which keeps the registration facts under a data
//! root, so a restart still lists the same workspaces with the same revisions.
//! There is no HTTP surface and no audit trail here (those live in the gateway
//! adapter), and no multi-root resolution yet.
//!
//! # What is persisted, and what is not
//!
//! The persisted record carries the **registration facts**: identity, display
//! name, root kind, the managed root, the confirmed scope, the revision, the
//! status, the expiry, the source, and the last write time. The managed root
//! is a host fact: the wire form projects only [`safe_root_display`]. Host
//! filesystem operations use the registry's checked desktop callback.
//!
//! The capabilities are **derived** on restore, never read back from the file,
//! so a hand-edited record cannot widen them. The safe display name is
//! likewise re-derived from the stored root, so the file never becomes a
//! second place a path could leak from.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::ids::WorkspaceId;
use apeireth_core::storage_atomic;
use apeireth_core::stored_doc;
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

mod desktop_folder_grants;
mod desktop_roots;

pub use desktop_folder_grants::{
    DesktopFolderIdentity, DESKTOP_FOLDER_LEASE_SECONDS, DESKTOP_GRANTED_WORKSPACE_SOURCE,
};
pub use desktop_roots::DESKTOP_MANAGED_WORKSPACE_SOURCE;

/// Longest permitted safe root display, in bytes.
pub const MAX_ROOT_DISPLAY_BYTES: usize = 64;

/// Stand-in name for a root that has no usable leaf name (a drive root, for
/// instance). It deliberately is not the drive letter.
pub const ROOT_DISPLAY_PLACEHOLDER: &str = "workspace-root";

/// Longest permitted workspace display name, in bytes.
pub const MAX_DISPLAY_NAME_BYTES: usize = 128;

/// The kind of managed root a workspace owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRootKind {
    /// A long-lived root the host owns.
    Managed,
    /// A root created for a scoped task and revoked once it expires.
    Temporary,
    /// A root prepared by the host as a catalog entry.
    Preset,
}

impl WorkspaceRootKind {
    /// The default `source` label recorded for a workspace of this kind.
    pub const fn source_label(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Temporary => "temporary",
            Self::Preset => "preset",
        }
    }
}

/// The confirmed permission scope of a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePermissionScope {
    /// Read only.
    Read,
    /// Read and write.
    Write,
    /// Read and write, but every write still needs a human approval.
    ApprovalWrite,
}

impl WorkspacePermissionScope {
    /// Whether this scope permits writes at all.
    ///
    /// `ApprovalWrite` still requires an approval per operation; it only says
    /// the workspace is *allowed* to write.
    pub const fn allows_write(self) -> bool {
        matches!(self, Self::Write | Self::ApprovalWrite)
    }
}

/// The lifecycle status of a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    /// Usable.
    Ready,
    /// Being provisioned; not usable yet.
    Creating,
    /// Revoked by a close; not usable any more.
    Revoked,
    /// Not reachable by the host right now.
    Unavailable,
}

/// Derives the safe display form of a managed root.
///
/// The result is **only** the leaf directory name: it never carries a parent
/// directory name, a drive letter, or a path separator, so it is safe to
/// project into a read surface. The leaf is taken under both separator
/// conventions (`/` and `\`), so a path written for another platform cannot
/// smuggle a parent directory into the display name.
///
/// The result is bounded by [`MAX_ROOT_DISPLAY_BYTES`] bytes; truncation keeps
/// the character boundary and therefore cannot introduce a separator. A root
/// without a usable leaf name yields [`ROOT_DISPLAY_PLACEHOLDER`] rather than
/// the drive letter.
pub fn safe_root_display(root: &Path) -> String {
    let leaf = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    // The leaf may still carry a separator when the path was written with the
    // other platform's convention; take the last segment either way.
    let leaf = leaf.rsplit(['/', '\\']).next().unwrap_or_default();

    if leaf.is_empty() {
        return ROOT_DISPLAY_PLACEHOLDER.to_string();
    }

    let sanitized: String = leaf
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c == ':' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();

    if sanitized.is_empty() {
        return ROOT_DISPLAY_PLACEHOLDER.to_string();
    }

    truncate_bytes(&sanitized, MAX_ROOT_DISPLAY_BYTES)
}

/// Truncates to at most `max_bytes` bytes without splitting a character.
fn truncate_bytes(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

/// A workspace: a first-class managed identity.
///
/// The struct cannot be built with a literal outside this module, because
/// [`Workspace::root`] is private. Every field is therefore either written by
/// a constructor in this module or derived from another field, which is what
/// keeps the capability declarations honest.
#[derive(Clone, PartialEq, Eq)]
pub struct Workspace {
    /// The identity everything binds to.
    pub workspace_id: WorkspaceId,
    /// The label shown to the user. It is *not* unique: two workspaces may
    /// share one, and are told apart by [`Workspace::workspace_id`].
    pub display_name: String,
    /// Which kind of managed root this workspace owns.
    pub root_kind: WorkspaceRootKind,
    /// The safe display form of the root — leaf only, bounded, produced by
    /// [`safe_root_display`] and never by the caller.
    pub root_display: String,
    /// The authoritative managed root.
    ///
    /// The registry can lend the root to a checked host operation. Read
    /// surfaces project [`Self::root_display`].
    root: PathBuf,
    /// The backend-confirmed permission scope.
    pub permission_scope: WorkspacePermissionScope,
    /// The monotonic version of this workspace.
    pub revision: u64,
    /// The lifecycle status.
    pub status: WorkspaceStatus,
    /// Whether this workspace may be selected right now (derived from
    /// [`Workspace::status`]).
    pub can_select: bool,
    /// Whether a temporary workspace may be derived from this one (derived
    /// from [`Workspace::status`]).
    pub can_create_temporary: bool,
    /// Whether the confirmed scope permits writes (derived from
    /// [`Workspace::permission_scope`]).
    pub can_write: bool,
    /// The expiry (epoch ms) of a temporary workspace.
    pub expires_at: Option<u64>,
    /// Where the workspace came from.
    pub source: String,
    /// The last write time (epoch ms).
    pub updated_at_ms: u64,
}

impl Workspace {
    /// Creates a managed workspace: a long-lived root the host owns.
    pub fn new_managed(
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Self, WorkspaceError> {
        Self::new(
            WorkspaceRootKind::Managed,
            root,
            display_name,
            permission_scope,
            now_ms,
            None,
        )
    }

    /// Creates a temporary workspace that expires at `expires_at`.
    pub fn new_temporary(
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
        expires_at: u64,
    ) -> Result<Self, WorkspaceError> {
        Self::new(
            WorkspaceRootKind::Temporary,
            root,
            display_name,
            permission_scope,
            now_ms,
            Some(expires_at),
        )
    }

    /// Creates a preset workspace: a catalog entry prepared by the host.
    pub fn new_preset(
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Self, WorkspaceError> {
        Self::new(
            WorkspaceRootKind::Preset,
            root,
            display_name,
            permission_scope,
            now_ms,
            None,
        )
    }

    fn new(
        root_kind: WorkspaceRootKind,
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
        expires_at: Option<u64>,
    ) -> Result<Self, WorkspaceError> {
        let display_name = validate_display_name(display_name)?;
        Ok(Self {
            workspace_id: WorkspaceId::new(),
            display_name,
            root_kind,
            root_display: safe_root_display(root),
            root: root.to_path_buf(),
            permission_scope,
            revision: 0,
            status: WorkspaceStatus::Ready,
            can_select: true,
            can_create_temporary: true,
            can_write: permission_scope.allows_write(),
            expires_at,
            source: root_kind.source_label().to_string(),
            updated_at_ms: now_ms,
        })
    }

    /// Whether this workspace has been revoked.
    pub fn is_revoked(&self) -> bool {
        self.status == WorkspaceStatus::Revoked
    }

    /// Rebinds the managed root and advances the revision.
    fn rebind_root(&mut self, root: &Path, now_ms: u64) {
        self.root = root.to_path_buf();
        self.root_display = safe_root_display(&self.root);
        self.revision += 1;
        self.updated_at_ms = now_ms;
    }

    /// Applies a confirmed permission scope and advances the revision.
    fn apply_permission_scope(&mut self, permission_scope: WorkspacePermissionScope, now_ms: u64) {
        self.permission_scope = permission_scope;
        self.can_write = permission_scope.allows_write();
        self.revision += 1;
        self.updated_at_ms = now_ms;
    }

    /// Revokes the workspace and advances the revision.
    fn revoke(&mut self, now_ms: u64) {
        self.status = WorkspaceStatus::Revoked;
        self.can_select = false;
        self.can_create_temporary = false;
        self.revision += 1;
        self.updated_at_ms = now_ms;
    }
}

impl fmt::Debug for Workspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The absolute root is redacted: a debug line reaches a log, and a log
        // is still a place a path can leak from.
        f.debug_struct("Workspace")
            .field("workspace_id", &self.workspace_id)
            .field("display_name", &self.display_name)
            .field("root_kind", &self.root_kind)
            .field("root_display", &self.root_display)
            .field("root", &"<redacted>")
            .field("permission_scope", &self.permission_scope)
            .field("revision", &self.revision)
            .field("status", &self.status)
            .field("can_select", &self.can_select)
            .field("can_create_temporary", &self.can_create_temporary)
            .field("can_write", &self.can_write)
            .field("expires_at", &self.expires_at)
            .field("source", &self.source)
            .field("updated_at_ms", &self.updated_at_ms)
            .finish()
    }
}

/// The wire form a read surface projects.
///
/// It carries the capability declarations but never the absolute root.
#[derive(Debug, Clone, Serialize)]
struct WorkspaceOut {
    workspace_id: WorkspaceId,
    display_name: String,
    root_kind: WorkspaceRootKind,
    root_display: String,
    permission_scope: WorkspacePermissionScope,
    revision: u64,
    status: WorkspaceStatus,
    can_select: bool,
    can_create_temporary: bool,
    can_write: bool,
    expires_at: Option<u64>,
    source: String,
    updated_at_ms: u64,
}

/// The wire form a client may send.
///
/// The capability declarations are deliberately absent: they are derived, so
/// accepting them would let a client widen its own capabilities. Unknown
/// fields are ignored, so a client that sends them anyway changes nothing.
/// The absolute root is absent for the same reason — it never crosses the
/// wire.
#[derive(Debug, Clone, Deserialize)]
struct WorkspaceIn {
    workspace_id: WorkspaceId,
    display_name: String,
    root_kind: WorkspaceRootKind,
    root_display: String,
    permission_scope: WorkspacePermissionScope,
    revision: u64,
    status: WorkspaceStatus,
    #[serde(default)]
    expires_at: Option<u64>,
    #[serde(default)]
    source: String,
    #[serde(default)]
    updated_at_ms: u64,
}

impl Serialize for Workspace {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        WorkspaceOut {
            workspace_id: self.workspace_id,
            display_name: self.display_name.clone(),
            root_kind: self.root_kind,
            root_display: self.root_display.clone(),
            permission_scope: self.permission_scope,
            revision: self.revision,
            status: self.status,
            can_select: self.can_select,
            can_create_temporary: self.can_create_temporary,
            can_write: self.can_write,
            expires_at: self.expires_at,
            source: self.source.clone(),
            updated_at_ms: self.updated_at_ms,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Workspace {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = WorkspaceIn::deserialize(deserializer)?;
        let ready = wire.status == WorkspaceStatus::Ready;
        Ok(Self {
            workspace_id: wire.workspace_id,
            display_name: wire.display_name,
            root_kind: wire.root_kind,
            root_display: wire.root_display,
            // The root never crosses the wire; a deserialized workspace simply
            // has none until the host rebinds it.
            root: PathBuf::new(),
            permission_scope: wire.permission_scope,
            revision: wire.revision,
            status: wire.status,
            can_select: ready,
            can_create_temporary: ready,
            can_write: wire.permission_scope.allows_write(),
            expires_at: wire.expires_at,
            source: wire.source,
            updated_at_ms: wire.updated_at_ms,
        })
    }
}

/// Validates a workspace display name.
fn validate_display_name(raw: &str) -> Result<String, WorkspaceError> {
    if raw.is_empty() {
        return Err(WorkspaceError::InvalidDisplayName {
            reason: "must not be empty".to_string(),
        });
    }
    if raw.len() > MAX_DISPLAY_NAME_BYTES {
        return Err(WorkspaceError::InvalidDisplayName {
            reason: format!("must be at most {MAX_DISPLAY_NAME_BYTES} bytes"),
        });
    }
    if raw.chars().any(char::is_control) {
        return Err(WorkspaceError::InvalidDisplayName {
            reason: "must not carry control characters".to_string(),
        });
    }
    Ok(raw.to_string())
}

/// The errors a workspace operation can fail with.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspaceError {
    /// No workspace with that identity is registered.
    #[error("workspace {0} not found")]
    NotFound(WorkspaceId),
    /// The caller's `expected_revision` did not match the current revision.
    #[error("workspace revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict {
        /// The revision the caller expected.
        expected: u64,
        /// The revision the workspace actually carries.
        actual: u64,
    },
    /// The workspace has been revoked and can no longer be changed or selected.
    #[error("workspace is revoked")]
    Revoked,
    /// The display name is empty, too long, or carries control characters.
    #[error("invalid workspace display name: {reason}")]
    InvalidDisplayName {
        /// Why the name was rejected.
        reason: String,
    },
    /// The request itself is inconsistent.
    #[error("invalid workspace request: {reason}")]
    InvalidRequest {
        /// Why the request was rejected.
        reason: String,
    },
    /// The workspace store could not be read or written.
    ///
    /// A storage failure is **not** a client error: the request was fine, the
    /// host could not carry it out. It gets its own variant so a read surface
    /// can answer "internal error" instead of "bad request" — and so a failed
    /// write is never mistaken for a refused one.
    #[error("workspace storage failure: {reason}")]
    Storage {
        /// What went wrong in the store.
        reason: String,
    },
}

/// A request to create a workspace.
///
/// The scope carried here is the **confirmed** scope: a client asks for one,
/// the host decides which one it grants, and only the granted value ever
/// reaches a workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceCreateRequest {
    /// Which kind of managed root this is.
    pub root_kind: WorkspaceRootKind,
    /// The managed root.
    pub root: PathBuf,
    /// The label shown to the user.
    pub display_name: String,
    /// The confirmed permission scope.
    pub permission_scope: WorkspacePermissionScope,
    /// The expiry (epoch ms). Required for a temporary workspace.
    pub expires_at: Option<u64>,
    /// Where the workspace came from. Defaults to the root kind's label.
    pub source: Option<String>,
    /// The creation time (epoch ms), recorded as `updated_at_ms`.
    pub now_ms: u64,
}

impl WorkspaceCreateRequest {
    /// A request for a managed workspace.
    pub fn managed(
        root: impl Into<PathBuf>,
        display_name: impl Into<String>,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Self {
        Self {
            root_kind: WorkspaceRootKind::Managed,
            root: root.into(),
            display_name: display_name.into(),
            permission_scope,
            expires_at: None,
            source: None,
            now_ms,
        }
    }
}

/// The port a host uses to manage workspaces.
///
/// An implementation owns the storage; the trait fixes the contract: every
/// mutation is guarded by an optimistic-concurrency check, and the registry
/// keeps at most one current selection.
pub trait WorkspaceRegistry: Send + Sync {
    /// Every registered workspace, in a stable order.
    fn list(&self) -> Vec<Workspace>;

    /// One workspace by identity.
    fn get(&self, workspace_id: &WorkspaceId) -> Option<Workspace>;

    /// The current selection of the registry, if any.
    fn current_selection(&self) -> Option<WorkspaceId>;

    /// Creates a workspace.
    fn create(&self, request: WorkspaceCreateRequest) -> Result<Workspace, WorkspaceError>;

    /// Provisions and registers a new isolated directory in the host data root.
    /// Only persistent registries that own the directory layout support this.
    fn create_desktop_workspace(
        &self,
        _display_name: &str,
        _permission_scope: WorkspacePermissionScope,
        _now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        Err(desktop_roots::desktop_workspace_required())
    }

    /// Record a Windows folder selected by the authenticated native host.
    fn grant_desktop_folder(
        &self,
        _display_name: &str,
        _directory: &Path,
        _identity: &DesktopFolderIdentity,
        _now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        Err(desktop_folder_grants::folder_grant_unavailable())
    }

    /// Runs a synchronous operation with a verified desktop root and current scope.
    /// The operation is serialized against revision changes and must not re-enter the registry.
    fn with_desktop_workspace_root(
        &self,
        _workspace_id: &WorkspaceId,
        _expected_revision: u64,
        _operation: &mut dyn FnMut(&Path, WorkspacePermissionScope),
    ) -> Result<(), WorkspaceError> {
        Err(desktop_roots::desktop_workspace_required())
    }

    /// Creates a temporary workspace that expires at `expires_at`.
    fn create_temporary(
        &self,
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
        expires_at: u64,
    ) -> Result<Workspace, WorkspaceError>;

    /// Selects a workspace.
    ///
    /// Selection is a registry-level fact, not a workspace attribute, so it
    /// does **not** advance the workspace revision.
    fn select(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
    ) -> Result<Workspace, WorkspaceError>;

    /// Changes the confirmed permission scope.
    fn change_permission_scope(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError>;

    /// Rebinds the managed root.
    fn rebind_root(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        root: &Path,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError>;

    /// Revokes a workspace.
    fn close(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError>;

    /// Restores a workspace by **exact** identity + revision match.
    ///
    /// This is the read side of the binding contract: a session, a task, or an
    /// approval names `workspace_id` + `revision`, and the host answers with
    /// the workspace only when *both* still match. A mismatch is refused,
    /// never repaired: the workspace is not upgraded to the caller's
    /// revision, the call is not replayed, and nothing is written.
    ///
    /// The check order is fixed: **not found → revision conflict → revoked**.
    /// The revision is checked before the status because it is the primary key
    /// component of this exact-match contract, so a stale binding is reported
    /// as the conflict it is; a workspace revoked *at exactly* the expected
    /// revision is reported as revoked and is never resurrected.
    fn restore_exact(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
    ) -> Result<Workspace, WorkspaceError>;

    /// Finds the workspace already bound to `root`.
    ///
    /// The managed root is a host fact, so the **host** — never the client —
    /// needs to ask "is this root already registered?" before it mints an
    /// identity. Without this, a host that binds its root on every start
    /// would mint a fresh identity per start and the registry would grow
    /// without bound.
    ///
    /// A **revoked** workspace is never returned: revocation is terminal, so
    /// a tombstone must not be handed back as "the workspace for this root".
    /// When several workspaces share a root, the one this returns is the
    /// first in registry iteration order, which is fixed by the workspace
    /// identity — so the same set always yields the same answer, before and
    /// after a restart.
    fn find_by_root(&self, root: &Path) -> Option<Workspace>;
}

/// An in-process workspace registry.
///
/// **进程内，未持久化**: every workspace lives in memory and is lost when the
/// process exits. It exists so the domain contract — identity, the revision
/// check, the safe display rule — can be exercised without a storage backend;
/// a persistent implementation belongs to a later batch.
pub struct InMemoryWorkspaceRegistry {
    inner: Mutex<RegistryInner>,
}

/// The mutable state of [`InMemoryWorkspaceRegistry`].
#[derive(Debug, Default)]
struct RegistryInner {
    workspaces: BTreeMap<WorkspaceId, Workspace>,
    selected: Option<WorkspaceId>,
}

impl InMemoryWorkspaceRegistry {
    /// A new, empty registry.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RegistryInner::default()),
        }
    }
}

impl Default for InMemoryWorkspaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds a workspace from a create request, without touching any registry.
///
/// Shared by both registry implementations so that the *domain* rules (kind,
/// expiry, display name, derived capabilities) live in exactly one place.
fn build_workspace(request: WorkspaceCreateRequest) -> Result<Workspace, WorkspaceError> {
    let mut workspace = match request.root_kind {
        WorkspaceRootKind::Managed => Workspace::new_managed(
            &request.root,
            &request.display_name,
            request.permission_scope,
            request.now_ms,
        )?,
        WorkspaceRootKind::Temporary => {
            let expires_at = request
                .expires_at
                .ok_or_else(|| WorkspaceError::InvalidRequest {
                    reason: "a temporary workspace requires an expiry".to_string(),
                })?;
            Workspace::new_temporary(
                &request.root,
                &request.display_name,
                request.permission_scope,
                request.now_ms,
                expires_at,
            )?
        }
        WorkspaceRootKind::Preset => Workspace::new_preset(
            &request.root,
            &request.display_name,
            request.permission_scope,
            request.now_ms,
        )?,
    };
    if let Some(source) = request.source {
        workspace.source = source;
    }
    Ok(workspace)
}

/// The guard every mutation runs before it touches a workspace.
///
/// The order is fixed: a missing workspace, then a revoked one, then the
/// revision check — so a revoked workspace always reports `Revoked` instead of
/// a revision conflict, and a stale expectation never mutates anything.
fn check_mutable(
    inner: &RegistryInner,
    workspace_id: &WorkspaceId,
    expected_revision: u64,
) -> Result<(), WorkspaceError> {
    let workspace = inner
        .workspaces
        .get(workspace_id)
        .ok_or(WorkspaceError::NotFound(*workspace_id))?;
    if workspace.is_revoked() {
        return Err(WorkspaceError::Revoked);
    }
    if workspace.revision != expected_revision {
        return Err(WorkspaceError::RevisionConflict {
            expected: expected_revision,
            actual: workspace.revision,
        });
    }
    Ok(())
}

impl WorkspaceRegistry for InMemoryWorkspaceRegistry {
    fn list(&self) -> Vec<Workspace> {
        self.inner.lock().workspaces.values().cloned().collect()
    }

    fn get(&self, workspace_id: &WorkspaceId) -> Option<Workspace> {
        self.inner.lock().workspaces.get(workspace_id).cloned()
    }

    fn current_selection(&self) -> Option<WorkspaceId> {
        self.inner.lock().selected
    }

    fn create(&self, request: WorkspaceCreateRequest) -> Result<Workspace, WorkspaceError> {
        let workspace = build_workspace(request)?;
        let mut inner = self.inner.lock();
        inner
            .workspaces
            .insert(workspace.workspace_id, workspace.clone());
        Ok(workspace)
    }

    fn create_temporary(
        &self,
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
        expires_at: u64,
    ) -> Result<Workspace, WorkspaceError> {
        self.create(WorkspaceCreateRequest {
            root_kind: WorkspaceRootKind::Temporary,
            root: root.to_path_buf(),
            display_name: display_name.to_string(),
            permission_scope,
            expires_at: Some(expires_at),
            source: None,
            now_ms,
        })
    }

    fn select(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let mut inner = self.inner.lock();
        check_mutable(&inner, workspace_id, expected_revision)?;
        inner.selected = Some(*workspace_id);
        Ok(inner
            .workspaces
            .get(workspace_id)
            .expect("checked above")
            .clone())
    }

    fn change_permission_scope(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let mut inner = self.inner.lock();
        check_mutable(&inner, workspace_id, expected_revision)?;
        let snapshot = {
            let workspace = inner
                .workspaces
                .get_mut(workspace_id)
                .expect("checked above");
            workspace.apply_permission_scope(permission_scope, now_ms);
            workspace.clone()
        };
        Ok(snapshot)
    }

    fn rebind_root(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        root: &Path,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let mut inner = self.inner.lock();
        check_mutable(&inner, workspace_id, expected_revision)?;
        let snapshot = {
            let workspace = inner
                .workspaces
                .get_mut(workspace_id)
                .expect("checked above");
            workspace.rebind_root(root, now_ms);
            workspace.clone()
        };
        Ok(snapshot)
    }

    fn close(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let mut inner = self.inner.lock();
        check_mutable(&inner, workspace_id, expected_revision)?;
        let snapshot = {
            let workspace = inner
                .workspaces
                .get_mut(workspace_id)
                .expect("checked above");
            workspace.revoke(now_ms);
            workspace.clone()
        };
        if inner.selected == Some(*workspace_id) {
            inner.selected = None;
        }
        Ok(snapshot)
    }

    fn restore_exact(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let inner = self.inner.lock();
        let workspace = inner
            .workspaces
            .get(workspace_id)
            .ok_or(WorkspaceError::NotFound(*workspace_id))?;
        if workspace.revision != expected_revision {
            return Err(WorkspaceError::RevisionConflict {
                expected: expected_revision,
                actual: workspace.revision,
            });
        }
        if workspace.is_revoked() {
            return Err(WorkspaceError::Revoked);
        }
        Ok(workspace.clone())
    }

    fn find_by_root(&self, root: &Path) -> Option<Workspace> {
        self.inner
            .lock()
            .workspaces
            .values()
            .find(|workspace| workspace.root == root && !workspace.is_revoked())
            .cloned()
    }
}

// ---------------------------------------------------------------------------
// Persistent registry
// ---------------------------------------------------------------------------

/// The document identity of a persisted workspace record.
const WORKSPACE_DOC_NAME: &str = "workspace";

/// The document version of a persisted workspace record.
const WORKSPACE_DOC_VERSION: u32 = 1;

/// The file name of a persisted workspace record.
const WORKSPACE_FILE_NAME: &str = "workspace.json";

/// The directory (under the registry root) that holds the workspace records.
const WORKSPACES_DIR_NAME: &str = "workspaces";

/// The document identity of the persisted registry selection.
const SELECTION_DOC_NAME: &str = "workspace-selection";

/// The document version of the persisted registry selection.
const SELECTION_DOC_VERSION: u32 = 1;

/// The file name of the persisted registry selection.
const SELECTION_FILE_NAME: &str = "selection.json";

/// The persisted form of one workspace.
///
/// `managed_root` is the only field that carries a path, and it is a **host
/// fact**: it is written down so the runtime can resolve a working directory
/// again after a restart, and it is never projected to a read surface.
///
/// The capability declarations are deliberately absent: they are derived, so
/// reading them back from a file would let a hand-edited record widen them.
/// The safe display name is absent for the same reason — it is re-derived from
/// the stored root on restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceRecord {
    workspace_id: WorkspaceId,
    display_name: String,
    root_kind: WorkspaceRootKind,
    managed_root: String,
    permission_scope: WorkspacePermissionScope,
    revision: u64,
    status: WorkspaceStatus,
    expires_at: Option<u64>,
    source: String,
    updated_at_ms: u64,
}

/// The persisted form of the registry-level selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionRecord {
    workspace_id: WorkspaceId,
}

impl Workspace {
    /// The persisted form of this workspace.
    ///
    /// A managed root that is not valid UTF-8 is **refused**: writing it with a
    /// replacement character would store a root that is not the root.
    fn to_record(&self) -> Result<WorkspaceRecord, WorkspaceError> {
        let managed_root = self.root.to_str().ok_or_else(|| WorkspaceError::Storage {
            reason: "受管根不是合法 UTF-8, 拒绝落盘 (不写一个会被改写的根)".to_string(),
        })?;
        Ok(WorkspaceRecord {
            workspace_id: self.workspace_id,
            display_name: self.display_name.clone(),
            root_kind: self.root_kind,
            managed_root: managed_root.to_string(),
            permission_scope: self.permission_scope,
            revision: self.revision,
            status: self.status,
            expires_at: self.expires_at,
            source: self.source.clone(),
            updated_at_ms: self.updated_at_ms,
        })
    }

    /// Rebuilds a workspace from its persisted form.
    ///
    /// The derived fields are re-derived, never read back: the capabilities
    /// follow the status and the scope, and the safe display name follows the
    /// stored root.
    fn from_record(record: WorkspaceRecord) -> Result<Self, WorkspaceError> {
        let display_name = validate_display_name(&record.display_name)?;
        let root = PathBuf::from(&record.managed_root);
        let ready = record.status == WorkspaceStatus::Ready;
        Ok(Self {
            workspace_id: record.workspace_id,
            display_name,
            root_kind: record.root_kind,
            root_display: safe_root_display(&root),
            root,
            permission_scope: record.permission_scope,
            revision: record.revision,
            status: record.status,
            can_select: ready,
            can_create_temporary: ready,
            can_write: record.permission_scope.allows_write(),
            expires_at: record.expires_at,
            source: record.source,
            updated_at_ms: record.updated_at_ms,
        })
    }
}

/// The version contract of a workspace record.
fn workspace_doc_compat() -> stored_doc::DocCompat {
    stored_doc::DocCompat::exact(WORKSPACE_DOC_NAME, WORKSPACE_DOC_VERSION)
}

/// The version contract of the selection record.
fn selection_doc_compat() -> stored_doc::DocCompat {
    stored_doc::DocCompat::exact(SELECTION_DOC_NAME, SELECTION_DOC_VERSION)
}

/// Reads one workspace record, refusing to open anything that does not match.
fn read_workspace_record(file: &Path) -> Result<WorkspaceRecord, WorkspaceError> {
    let doc = stored_doc::open_single::<WorkspaceRecord>(file, &workspace_doc_compat()).map_err(
        |error| WorkspaceError::Storage {
            reason: format!("工作区记录拒开 ({}): {error}", file.display()),
        },
    )?;
    Ok(doc.body)
}

/// Wraps an IO failure into a storage error.
fn storage_error(action: &str, path: &Path, error: &std::io::Error) -> WorkspaceError {
    WorkspaceError::Storage {
        reason: format!("{action} ({}): {error}", path.display()),
    }
}

/// The mutation guard, applied to one workspace.
///
/// The order is fixed: a revoked workspace, then the revision check — the same
/// order [`check_mutable`] uses, so a revoked workspace always reports
/// `Revoked` instead of a revision conflict.
fn check_mutable_workspace(
    workspace: &Workspace,
    expected_revision: u64,
) -> Result<(), WorkspaceError> {
    if workspace.is_revoked() {
        return Err(WorkspaceError::Revoked);
    }
    if workspace.revision != expected_revision {
        return Err(WorkspaceError::RevisionConflict {
            expected: expected_revision,
            actual: workspace.revision,
        });
    }
    Ok(())
}

/// A workspace registry that keeps its facts on disk.
///
/// The layout under the registry root is:
///
/// ```text
/// <root>/workspaces/<workspace_id>/workspace.json   # one record per workspace
/// <root>/workspaces/selection.json                   # the registry-level selection
/// ```
///
/// Every file is a [`stored_doc::StoredDoc`] envelope, so a record that is
/// malformed, of the wrong identity, or of an unknown version **refuses to
/// open**: the registry reports a storage error instead of silently dropping a
/// workspace or falling back to an empty registry.
///
/// **Write, then commit.** A mutation is written to disk first and only then
/// committed to the in-memory map, so a failed write leaves the in-memory
/// state exactly as it was. The revision check is made against the record
/// **on disk**, under that workspace's own file lock, so two processes that
/// share a data root cannot both apply a stale expectation to the same
/// workspace.
pub struct FileWorkspaceRegistry {
    root: PathBuf,
    workspaces_dir: PathBuf,
    inner: Mutex<RegistryInner>,
    desktop_folder_leases: Mutex<desktop_folder_grants::DesktopFolderLeases>,
}

impl FileWorkspaceRegistry {
    /// Opens (or creates) a persistent registry rooted at `root`.
    ///
    /// The root is the **data root**, not a workspace root: the workspaces this
    /// registry holds keep their own roots, and those never appear in the
    /// paths this function touches.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, WorkspaceError> {
        let root = root.into();
        let workspaces_dir = root.join(WORKSPACES_DIR_NAME);
        fs::create_dir_all(&workspaces_dir)
            .map_err(|error| storage_error("创建工作区目录失败", &workspaces_dir, &error))?;

        let mut inner = RegistryInner::default();
        let entries = fs::read_dir(&workspaces_dir)
            .map_err(|error| storage_error("读取工作区目录失败", &workspaces_dir, &error))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| storage_error("读取工作区目录项失败", &workspaces_dir, &error))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Ok(workspace_id) = name.parse::<WorkspaceId>() else {
                // A directory that is not named after a workspace identity is
                // not a workspace record; it carries no fact to lose.
                continue;
            };
            let file = path.join(WORKSPACE_FILE_NAME);
            if !file.is_file() {
                // An interrupted write leaves the directory behind without a
                // record. Nothing was committed to memory in that case (the
                // record is written first), so there is no fact to lose.
                continue;
            }
            let record = read_workspace_record(&file)?;
            let workspace = Workspace::from_record(record)?;
            inner.workspaces.insert(workspace_id, workspace);
        }

        // The selection is a registry-level fact: keeping it means a restart
        // does not silently drop the current selection.
        let selection_file = workspaces_dir.join(SELECTION_FILE_NAME);
        if selection_file.is_file() {
            let doc = stored_doc::open_single::<SelectionRecord>(
                &selection_file,
                &selection_doc_compat(),
            )
            .map_err(|error| WorkspaceError::Storage {
                reason: format!("选中态记录拒开 ({}): {error}", selection_file.display()),
            })?;
            if inner.workspaces.contains_key(&doc.body.workspace_id) {
                inner.selected = Some(doc.body.workspace_id);
            }
        }

        Ok(Self {
            root,
            workspaces_dir,
            inner: Mutex::new(inner),
            desktop_folder_leases: Mutex::default(),
        })
    }

    /// The data root this registry was opened at.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory that holds the workspace records.
    pub fn workspaces_dir(&self) -> &Path {
        &self.workspaces_dir
    }

    /// The record file of one workspace.
    fn workspace_file(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.workspaces_dir
            .join(workspace_id.to_string())
            .join(WORKSPACE_FILE_NAME)
    }

    /// Reads the authoritative record of one workspace from disk.
    fn read_workspace_from_disk(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Workspace, WorkspaceError> {
        let file = self.workspace_file(workspace_id);
        if !file.is_file() {
            return Err(WorkspaceError::NotFound(*workspace_id));
        }
        let record = read_workspace_record(&file)?;
        Workspace::from_record(record)
    }

    /// Writes one workspace record. The caller holds that workspace's lock.
    fn write_workspace_record(&self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        let record = workspace.to_record()?;
        let file = self.workspace_file(&workspace.workspace_id);
        stored_doc::save_single(
            &file,
            &workspace_doc_compat(),
            record,
            stored_doc::DEFAULT_DOC_MODE,
        )
        .map_err(|error| WorkspaceError::Storage {
            reason: format!("工作区落盘失败 ({}): {error}", file.display()),
        })?;
        Ok(())
    }

    /// Writes (or clears) the registry-level selection.
    fn write_selection(&self, selected: Option<WorkspaceId>) -> Result<(), WorkspaceError> {
        let file = self.workspaces_dir.join(SELECTION_FILE_NAME);
        let lock = storage_atomic::lock_path_for(&file);
        let outcome = storage_atomic::with_file_lock(&lock, || -> Result<(), String> {
            match selected {
                Some(workspace_id) => stored_doc::save_single(
                    &file,
                    &selection_doc_compat(),
                    SelectionRecord { workspace_id },
                    stored_doc::DEFAULT_DOC_MODE,
                )
                .map(|_| ())
                .map_err(|error| error.to_string()),
                None => match fs::remove_file(&file) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(error) => Err(error.to_string()),
                },
            }
        });
        match outcome {
            Ok(inner) => inner.map_err(|reason| WorkspaceError::Storage {
                reason: format!("选中态落盘失败 ({}): {reason}", file.display()),
            }),
            Err(error) => Err(WorkspaceError::Storage {
                reason: format!("选中态文件锁失败 ({}): {error}", lock.display()),
            }),
        }
    }

    /// Reads, mutates and writes one workspace record under its own file lock.
    ///
    /// The revision check is made against the record **on disk**, so another
    /// process that already advanced the revision makes this one fail with a
    /// conflict instead of overwriting the newer fact.
    fn mutate_workspace(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        apply: impl FnOnce(&mut Workspace) -> Result<(), WorkspaceError>,
    ) -> Result<Workspace, WorkspaceError> {
        let file = self.workspace_file(workspace_id);
        let lock = storage_atomic::lock_path_for(&file);
        let outcome =
            storage_atomic::with_file_lock(&lock, || -> Result<Workspace, WorkspaceError> {
                let current = self.read_workspace_from_disk(workspace_id)?;
                check_mutable_workspace(&current, expected_revision)?;
                let mut next = current;
                apply(&mut next)?;
                self.write_workspace_record(&next)?;
                Ok(next)
            });
        match outcome {
            Ok(inner) => inner,
            Err(error) => Err(WorkspaceError::Storage {
                reason: format!("工作区文件锁失败 ({}): {error}", lock.display()),
            }),
        }
    }

    /// Commits a workspace to the in-memory map; the durable write already happened.
    fn commit(&self, workspace: Workspace) {
        let mut inner = self.inner.lock();
        inner.workspaces.insert(workspace.workspace_id, workspace);
    }
}

impl WorkspaceRegistry for FileWorkspaceRegistry {
    fn list(&self) -> Vec<Workspace> {
        let mut workspaces: Vec<_> = self.inner.lock().workspaces.values().cloned().collect();
        for workspace in &mut workspaces {
            self.project_folder_grant_availability(workspace);
        }
        workspaces
    }

    fn get(&self, workspace_id: &WorkspaceId) -> Option<Workspace> {
        let mut workspace = self.inner.lock().workspaces.get(workspace_id).cloned()?;
        self.project_folder_grant_availability(&mut workspace);
        Some(workspace)
    }

    fn current_selection(&self) -> Option<WorkspaceId> {
        let selected = self.inner.lock().selected?;
        self.get(&selected)
            .filter(|workspace| !workspace.is_desktop_folder_grant() || workspace.can_select)
            .map(|_| selected)
    }

    fn create(&self, request: WorkspaceCreateRequest) -> Result<Workspace, WorkspaceError> {
        let workspace = build_workspace(request)?;
        let file = self.workspace_file(&workspace.workspace_id);
        let lock = storage_atomic::lock_path_for(&file);
        let outcome = storage_atomic::with_file_lock(&lock, || -> Result<(), WorkspaceError> {
            if file.is_file() {
                return Err(WorkspaceError::Storage {
                    reason: format!("工作区记录已存在 ({})", file.display()),
                });
            }
            self.write_workspace_record(&workspace)
        });
        let written = match outcome {
            Ok(inner) => inner,
            Err(error) => {
                return Err(WorkspaceError::Storage {
                    reason: format!("工作区文件锁失败 ({}): {error}", lock.display()),
                })
            }
        };
        written?;
        let mut inner = self.inner.lock();
        inner
            .workspaces
            .insert(workspace.workspace_id, workspace.clone());
        Ok(workspace)
    }

    fn create_desktop_workspace(
        &self,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        self.provision_desktop_workspace(display_name, permission_scope, now_ms)
    }

    fn grant_desktop_folder(
        &self,
        display_name: &str,
        directory: &Path,
        identity: &DesktopFolderIdentity,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        self.provision_desktop_folder_grant(display_name, directory, identity, now_ms)
    }

    fn with_desktop_workspace_root(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        operation: &mut dyn FnMut(&Path, WorkspacePermissionScope),
    ) -> Result<(), WorkspaceError> {
        self.access_desktop_workspace(workspace_id, expected_revision, operation)
    }

    fn create_temporary(
        &self,
        root: &Path,
        display_name: &str,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
        expires_at: u64,
    ) -> Result<Workspace, WorkspaceError> {
        self.create(WorkspaceCreateRequest {
            root_kind: WorkspaceRootKind::Temporary,
            root: root.to_path_buf(),
            display_name: display_name.to_string(),
            permission_scope,
            expires_at: Some(expires_at),
            source: None,
            now_ms,
        })
    }

    fn select(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let mut inner = self.inner.lock();
        check_mutable(&inner, workspace_id, expected_revision)?;
        let workspace = inner
            .workspaces
            .get(workspace_id)
            .expect("checked above")
            .clone();
        self.require_live_folder_grant(&workspace)?;
        // The selection is a registry-level fact: it is written down before the
        // in-memory state moves, so a failed write leaves it as it was.
        self.write_selection(Some(*workspace_id))?;
        inner.selected = Some(*workspace_id);
        Ok(workspace)
    }

    fn change_permission_scope(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        permission_scope: WorkspacePermissionScope,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let next = self.mutate_workspace(workspace_id, expected_revision, |workspace| {
            workspace.require_regular_root()?;
            workspace.apply_permission_scope(permission_scope, now_ms);
            Ok(())
        })?;
        self.commit(next.clone());
        Ok(next)
    }

    fn rebind_root(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        root: &Path,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let next = self.mutate_workspace(workspace_id, expected_revision, |workspace| {
            workspace.require_regular_root()?;
            workspace.rebind_root(root, now_ms);
            Ok(())
        })?;
        self.commit(next.clone());
        Ok(next)
    }

    fn close(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<Workspace, WorkspaceError> {
        let next = self.mutate_workspace(workspace_id, expected_revision, |workspace| {
            workspace.revoke(now_ms);
            Ok(())
        })?;
        self.commit(next.clone());
        self.desktop_folder_leases.lock().remove(workspace_id);
        // A revoked workspace must not stay the current selection. The
        // revocation itself is already durable at this point, so a failure to
        // clear the selection is reported as a storage error *after* the fact —
        // it is never rolled back.
        let mut inner = self.inner.lock();
        if inner.selected == Some(*workspace_id) {
            self.write_selection(None)?;
            inner.selected = None;
        }
        Ok(next)
    }

    fn restore_exact(
        &self,
        workspace_id: &WorkspaceId,
        expected_revision: u64,
    ) -> Result<Workspace, WorkspaceError> {
        // The durable record is the authority here: this is the recovery path.
        let current = self.read_workspace_from_disk(workspace_id)?;
        if current.revision != expected_revision {
            return Err(WorkspaceError::RevisionConflict {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        if current.is_revoked() {
            return Err(WorkspaceError::Revoked);
        }
        self.require_live_folder_grant(&current)?;
        Ok(current)
    }

    fn find_by_root(&self, root: &Path) -> Option<Workspace> {
        self.inner
            .lock()
            .workspaces
            .values()
            .find(|workspace| {
                workspace.root == root
                    && !workspace.is_revoked()
                    && !workspace.is_desktop_folder_grant()
            })
            .cloned()
    }
}
