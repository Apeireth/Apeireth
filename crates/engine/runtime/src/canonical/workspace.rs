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
//! concurrency rules. The registry implementation shipped here is in-process
//! and **not persisted**; there is no HTTP surface, no audit trail, and no
//! multi-root resolution yet — those belong to later batches.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::ids::WorkspaceId;
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

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
    /// Private on purpose: there is no public accessor and the wire form does
    /// not carry it, so no read surface can project the absolute path.
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
        let mut workspace = match request.root_kind {
            WorkspaceRootKind::Managed => Workspace::new_managed(
                &request.root,
                &request.display_name,
                request.permission_scope,
                request.now_ms,
            )?,
            WorkspaceRootKind::Temporary => {
                let expires_at =
                    request
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
}
