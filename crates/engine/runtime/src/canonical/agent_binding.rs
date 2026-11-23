//! Agent binding: the contract between a session, a contact, and a workspace.
//!
//! # What this module owns
//!
//! Three things, and nothing else:
//!
//! 1. **The memory scope mapping.** The contract carries a *managed* scope
//!    enum ([`MemoryScopeContract`]); this module maps it onto the scope a
//!    memory write must land in ([`MemoryScopeTarget`]), and it maps `none`
//!    onto *no write at all*.
//! 2. **The binding.** [`AgentBinding`] is the exact pairing of one contact
//!    (one agent identity, one persona version) and one workspace (one
//!    workspace revision), plus the two host-owned echo fields `context_key`
//!    and `incarnation`.
//! 3. **The fail-closed gate.** [`admit_bound_turn`] re-checks a stored
//!    binding against the *current* registry facts and refuses the turn when
//!    the binding is no longer valid — it never falls back to a default agent
//!    or a default domain.
//!
//! # Why the scope mapping is a mirror, not the canonical type
//!
//! The canonical `MemoryScope` lives in `apeireth-memory`. The runtime crate
//! does not depend on that crate (and must not: the dependency edge is not in
//! `Cargo.lock`, and `--locked` forbids adding one), so the mapping layer
//! keeps its own mirror type, [`MemoryScopeTarget`], whose serde shape is
//! identical to the canonical one for the three variants the contract can
//! produce. The assembly layer does the translation.
//!
//! ⛔ The mirror has **no `Global` variant**, on purpose. `none` means
//! "write no memory"; `MemoryScope::Global` means "visible to every user and
//! persona in the installation". They are opposites, and making `Global`
//! unrepresentable here means "none ⇒ Global" cannot be written even by
//! accident.
//!
//! # Fail-closed, in one paragraph
//!
//! A binding is a claim about the world that can go stale: the contact can be
//! archived, the persona or registered binding can change, and the workspace
//! can be revoked or moved.
//! When that happens the turn is **refused** — no message is appended, no
//! model call is made — because the alternative (answering as the default
//! agent) is indistinguishable, from the user's side, from "the persona I
//! asked for was silently swapped for somebody else's".
//!
//! # Honest scope
//!
//! This module is the domain layer, and it is **implemented and unit-tested**.
//! It is **not wired into the production turn path**: the default-domain
//! fallback in `canonical::domain_plane` is still the live path for an
//! unbound session, and no caller invokes [`admit_turn`] yet. The wiring
//! point (and the HTTP surface) belongs to the next batch.

use apeireth_core::agent_contact::{
    Contact, ContactRegistry, ContactStatus, ContactWorkspaceBinding, ContractMemoryScope,
};
use apeireth_core::kernel::ids::{ContactAgentId, ContactId, WorkspaceId};
use apeireth_core::kernel::Clock;
use apeireth_protocol::canonical::NormalizedMessage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::session::Session;
use super::workspace::{Workspace, WorkspaceRegistry};

// ---------------------------------------------------------------------------
// 1. The memory scope mapping layer
// ---------------------------------------------------------------------------

/// The contract-level memory scope of a contact.
///
/// This is a **managed enum**: the four values below are the only shapes the
/// backend contract admits. It is deliberately *not* the canonical
/// `MemoryScope` (that type lives in `apeireth-memory`) and deliberately *not*
/// an extension of it.
///
/// The mapping is:
///
/// | contract  | target                                                  |
/// |-----------|---------------------------------------------------------|
/// | `subject` | `User { user_id: subject_id }`                          |
/// | `contact` | `Persona { persona_id: contact_id, user_id: subject_id }` |
/// | `session` | `Session { session_id }`                                |
/// | `none`    | *no scope, no write*                                    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScopeContract {
    /// The subject's own memory.
    Subject,
    /// The contact's persona memory.
    Contact,
    /// The conversation's own memory.
    Session,
    /// **No memory at all.** Not "shared memory", not "global memory".
    None,
}

impl Default for MemoryScopeContract {
    fn default() -> Self {
        Self::None
    }
}

impl From<ContractMemoryScope> for MemoryScopeContract {
    fn from(value: ContractMemoryScope) -> Self {
        match value {
            ContractMemoryScope::Subject => Self::Subject,
            ContractMemoryScope::Contact => Self::Contact,
            ContractMemoryScope::Session => Self::Session,
            ContractMemoryScope::None => Self::None,
        }
    }
}

/// The scope a memory write must land in.
///
/// This mirrors the canonical `MemoryScope` one-for-one for the three
/// variants the contract can produce, down to the serde shape:
/// `MemoryScopeTarget::User { user_id }` serializes to
/// `{"scope":"user","user_id":"…"}`, which is exactly what the canonical
/// `MemoryScope::User { user_id }` serializes to, so the assembly layer can
/// translate a plan mechanically.
///
/// ⛔ There is **no `Global` variant**, on purpose: `none` means "write no
/// memory" while `MemoryScope::Global` means "visible to every user and
/// persona in the installation". Making `Global` unrepresentable here means
/// "none ⇒ Global" cannot be written even by accident.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum MemoryScopeTarget {
    /// Visible to one user. ← contract `subject`.
    User { user_id: String },
    /// Visible to one persona of one user. ← contract `contact`.
    Persona { persona_id: String, user_id: String },
    /// Visible only to one session. ← contract `session`.
    Session { session_id: String },
}

impl MemoryScopeTarget {
    /// The scope kind tag, matching the canonical scope's kind names.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::User { .. } => "user",
            Self::Persona { .. } => "persona",
            Self::Session { .. } => "session",
        }
    }

    /// Whether a record in this scope is visible through the supplied scopes.
    ///
    /// This mirrors the canonical `MemoryScope::is_visible_in` for the
    /// mirrored variants: matching is exact, and a persona matches only when
    /// **both** the persona identity and the user identity match — so two
    /// contacts of the same subject never see each other's memories, and two
    /// subjects sharing a display name never share a persona scope.
    pub fn is_visible_in(&self, visible_scopes: &[Self]) -> bool {
        visible_scopes.iter().any(|visible| match (self, visible) {
            (Self::User { user_id: a }, Self::User { user_id: b }) => a == b,
            (
                Self::Persona {
                    persona_id: a_persona,
                    user_id: a_user,
                },
                Self::Persona {
                    persona_id: b_persona,
                    user_id: b_user,
                },
            ) => a_persona == b_persona && a_user == b_user,
            (Self::Session { session_id: a }, Self::Session { session_id: b }) => a == b,
            _ => false,
        })
    }
}

/// The identities a contract scope is resolved against.
#[derive(Debug, Clone, Copy)]
pub struct MemoryScopeIdentities<'a> {
    /// The subject (the human principal) the session belongs to.
    pub subject_id: &'a str,
    /// The bound contact.
    pub contact_id: &'a str,
    /// The session that owns the write.
    pub session_id: &'a str,
}

/// The memory scope plan for one session: where a write lands, and whether
/// there is a write at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryScopePlan {
    scope: Option<MemoryScopeTarget>,
    writes: bool,
}

impl MemoryScopePlan {
    /// The scope a write lands in, or `None` when the contract forbids
    /// writing.
    pub fn scope(&self) -> Option<&MemoryScopeTarget> {
        self.scope.as_ref()
    }

    /// Whether memory writes are allowed at all.
    pub fn writes(&self) -> bool {
        self.writes
    }

    /// The scope a write lands in.
    ///
    /// Fails when the contract forbids writing, so a caller that forgot to
    /// check [`MemoryScopePlan::writes`] gets a refusal instead of a silent
    /// write into the wrong scope.
    pub fn require_scope(&self) -> Result<&MemoryScopeTarget, AgentBindingError> {
        self.scope
            .as_ref()
            .ok_or_else(|| AgentBindingError::InvalidRequest {
                reason: "memory scope contract is `none`: this write must not happen".to_string(),
            })
    }
}

/// Maps a contract scope onto the scope a write must land in.
///
/// | contract  | target                                                   |
/// |-----------|----------------------------------------------------------|
/// | `subject` | `User { user_id: subject_id }`                           |
/// | `contact` | `Persona { persona_id: contact_id, user_id: subject_id }` |
/// | `session` | `Session { session_id }`                                 |
/// | `none`    | *no scope, no write*                                     |
///
/// ⛔ `none` is **not** `Global`: `Global` is visible to every user and
/// persona in the installation, `none` writes nothing at all.
pub fn map_memory_scope(
    contract: MemoryScopeContract,
    identities: MemoryScopeIdentities<'_>,
) -> MemoryScopePlan {
    match contract {
        MemoryScopeContract::Subject => MemoryScopePlan {
            scope: Some(MemoryScopeTarget::User {
                user_id: identities.subject_id.to_string(),
            }),
            writes: true,
        },
        MemoryScopeContract::Contact => MemoryScopePlan {
            scope: Some(MemoryScopeTarget::Persona {
                persona_id: identities.contact_id.to_string(),
                user_id: identities.subject_id.to_string(),
            }),
            writes: true,
        },
        MemoryScopeContract::Session => MemoryScopePlan {
            scope: Some(MemoryScopeTarget::Session {
                session_id: identities.session_id.to_string(),
            }),
            writes: true,
        },
        MemoryScopeContract::None => MemoryScopePlan {
            scope: None,
            writes: false,
        },
    }
}

/// The memory scope plan for a contact: the contract scope the contact
/// carries, mapped onto the identities of the session bound to it.
pub fn contact_memory_plan(
    contact: &Contact,
    subject_id: &str,
    session_id: &str,
) -> MemoryScopePlan {
    let contact_id = contact.contact_id().to_string();
    map_memory_scope(
        contact.memory_scope().into(),
        MemoryScopeIdentities {
            subject_id,
            contact_id: &contact_id,
            session_id,
        },
    )
}

// ---------------------------------------------------------------------------
// 2. The binding
// ---------------------------------------------------------------------------

/// The agent binding a session carries.
///
/// A binding is the *exact* pairing of one contact (with one agent identity
/// and one persona version) and one workspace (with one workspace revision),
/// plus the two opaque echo fields `context_key` and `incarnation`.
///
/// The fields are private and reachable only through the accessors, so a
/// binding cannot be assembled from parts outside this module: every binding
/// that exists was validated against the contact and workspace registries
/// that own the facts it claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBinding {
    contact_id: ContactId,
    agent_id: ContactAgentId,
    persona_revision: u64,
    workspace_id: WorkspaceId,
    workspace_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contact_workspace_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    incarnation: Option<String>,
    revision: u64,
    updated_at_ms: u64,
}

impl AgentBinding {
    /// The bound contact.
    pub fn contact_id(&self) -> ContactId {
        self.contact_id
    }

    /// The agent identity the contact owns.
    pub fn agent_id(&self) -> ContactAgentId {
        self.agent_id
    }

    /// The persona version this binding was made against.
    pub fn persona_revision(&self) -> u64 {
        self.persona_revision
    }

    /// The bound workspace.
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    /// The workspace revision this binding was made against.
    pub fn workspace_revision(&self) -> u64 {
        self.workspace_revision
    }

    /// The exact contact registration revision used to select this workspace.
    pub fn contact_workspace_revision(&self) -> Option<u64> {
        self.contact_workspace_revision
    }

    /// The host-owned context key, echoed back verbatim.
    ///
    /// The backend never generates this value; its meaning belongs to the
    /// host, so the backend only carries it and checks it for equality when
    /// the binding is re-bound.
    pub fn context_key(&self) -> Option<&str> {
        self.context_key.as_deref()
    }

    /// The host-owned incarnation marker, echoed back verbatim.
    ///
    /// Like [`AgentBinding::context_key`], the backend never generates it and
    /// only checks it for equality when the binding is re-bound.
    pub fn incarnation(&self) -> Option<&str> {
        self.incarnation.as_deref()
    }

    /// The binding's own revision: it advances on every re-bind.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// When the binding was last written (epoch milliseconds).
    pub fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    /// The auditable identity of this binding.
    ///
    /// An audit record has to name *who* answered and *under which
    /// workspace*, and a display name is not an identity — two contacts may
    /// share one — so this names the backend-minted identifiers.
    pub fn audit_identity(&self) -> String {
        format!(
            "contact={} agent={} workspace={} binding_revision={}",
            self.contact_id, self.agent_id, self.workspace_id, self.revision
        )
    }
}

/// The binding request a caller submits.
///
/// Every identity in it is a **fact the caller claims**; the backend checks
/// each one against the registry that owns it.
///
/// `context_key` and `incarnation` are **echo fields**: the backend never
/// generates them (their semantics belong to the host), so on a re-bind they
/// must come back byte-identical, and a mismatch is a conflict rather than
/// an update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBindingRequest {
    /// The contact to bind.
    pub contact_id: ContactId,
    /// The agent identity the caller believes the contact owns.
    pub agent_id: ContactAgentId,
    /// The persona version the caller believes is current.
    pub persona_revision: u64,
    /// The workspace to bind.
    pub workspace_id: WorkspaceId,
    /// The workspace revision the caller believes is current.
    pub workspace_revision: u64,
    /// The host-owned context key to echo.
    pub context_key: Option<String>,
    /// The host-owned incarnation marker to echo.
    pub incarnation: Option<String>,
}

impl AgentBindingRequest {
    /// A request that binds `contact` to `workspace` at their **current**
    /// revisions.
    ///
    /// The echo fields stay empty: the backend does not invent them.
    pub fn for_current(contact: &Contact, workspace: &Workspace) -> Self {
        Self {
            contact_id: contact.contact_id(),
            agent_id: contact.agent_id(),
            persona_revision: contact.persona_revision(),
            workspace_id: workspace.workspace_id,
            workspace_revision: workspace.revision,
            context_key: None,
            incarnation: None,
        }
    }
}

/// Why a binding was refused.
///
/// Every variant is a refusal: this module has no "degraded" answer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentBindingError {
    /// The contact does not exist.
    #[error("contact not found: {0}")]
    ContactNotFound(ContactId),
    /// The contact is archived (a tombstone): archived contacts are never
    /// bindable, and an existing binding that points at one is refused
    /// instead of being silently replaced.
    #[error("contact is archived: {0}")]
    ContactArchived(ContactId),
    /// The claimed persona version is not the contact's current one.
    #[error("persona revision mismatch: expected {expected}, actual {actual}")]
    PersonaRevisionMismatch {
        /// The version the caller claimed.
        expected: u64,
        /// The version the contact actually carries.
        actual: u64,
    },
    /// The workspace does not exist.
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(WorkspaceId),
    /// The workspace is revoked: a revoked workspace is never bindable.
    #[error("workspace is revoked: {0}")]
    WorkspaceRevoked(WorkspaceId),
    /// The claimed workspace revision is not the workspace's current one.
    #[error("workspace revision mismatch: expected {expected}, actual {actual}")]
    WorkspaceRevisionMismatch {
        /// The revision the caller claimed.
        expected: u64,
        /// The revision the workspace actually carries.
        actual: u64,
    },
    /// The binding conflicts with the binding that is already there.
    ///
    /// `expected` and `actual` are `field=value` strings, so one variant can
    /// carry every kind of conflict: a stale binding revision, a different
    /// contact, a different agent identity, a different workspace, or a
    /// changed echo field.
    #[error("binding conflict: expected {expected}, actual {actual}")]
    BindingConflict {
        /// What the binding on record says.
        expected: String,
        /// What the caller asked for.
        actual: String,
    },
    /// The request itself is malformed.
    #[error("invalid binding request: {reason}")]
    InvalidRequest {
        /// Why the request is refused.
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// 3. Validation
// ---------------------------------------------------------------------------

/// Validates a binding request against the registries that own the facts, and
/// mints the binding.
///
/// # Refusal order (fixed)
///
/// 1. contact not found → [`AgentBindingError::ContactNotFound`]
/// 2. contact archived → [`AgentBindingError::ContactArchived`]
/// 3. agent identity mismatch → [`AgentBindingError::BindingConflict`]
/// 4. persona version mismatch → [`AgentBindingError::PersonaRevisionMismatch`]
/// 5. workspace not found → [`AgentBindingError::WorkspaceNotFound`]
/// 6. workspace revoked → [`AgentBindingError::WorkspaceRevoked`]
/// 7. workspace version mismatch → [`AgentBindingError::WorkspaceRevisionMismatch`]
/// 8. contact registration points elsewhere or was detached → [`AgentBindingError::BindingConflict`]
///
/// Archived and revoked are checked **before** the revision checks on
/// purpose: both are terminal states, and a terminal state must never be
/// reported as "stale, refresh and retry" — that would invite exactly the
/// retry the fail-closed rule forbids.
pub fn resolve_agent_binding(
    request: &AgentBindingRequest,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
    now_ms: u64,
) -> Result<AgentBinding, AgentBindingError> {
    let contact = contacts
        .get(&request.contact_id)
        .ok_or(AgentBindingError::ContactNotFound(request.contact_id))?;
    if contact.status() == ContactStatus::Archived {
        return Err(AgentBindingError::ContactArchived(request.contact_id));
    }
    if contact.agent_id() != request.agent_id {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("agent_id={}", request.agent_id),
            actual: format!("agent_id={}", contact.agent_id()),
        });
    }
    if contact.persona_revision() != request.persona_revision {
        return Err(AgentBindingError::PersonaRevisionMismatch {
            expected: request.persona_revision,
            actual: contact.persona_revision(),
        });
    }
    let workspace = workspaces
        .get(&request.workspace_id)
        .ok_or(AgentBindingError::WorkspaceNotFound(request.workspace_id))?;
    if workspace.is_revoked() {
        return Err(AgentBindingError::WorkspaceRevoked(request.workspace_id));
    }
    if workspace.revision != request.workspace_revision {
        return Err(AgentBindingError::WorkspaceRevisionMismatch {
            expected: request.workspace_revision,
            actual: workspace.revision,
        });
    }
    check_contact_workspace(&contact, request.workspace_id, request.workspace_revision)?;
    Ok(AgentBinding {
        contact_id: contact.contact_id(),
        agent_id: contact.agent_id(),
        persona_revision: contact.persona_revision(),
        workspace_id: workspace.workspace_id,
        workspace_revision: workspace.revision,
        contact_workspace_revision: contact.workspace_binding_revision(),
        context_key: request.context_key.clone(),
        incarnation: request.incarnation.clone(),
        revision: 0,
        updated_at_ms: now_ms,
    })
}

/// Re-binds a session that already carries a binding.
///
/// A re-bind may only **move forward**: the contact, the agent identity and
/// the workspace stay the same, and the two echo fields must come back
/// byte-identical, because the backend does not own them (it never generates
/// them) and silently changing one would change what the caller's context
/// key means. The persona version and the workspace version may move forward
/// to their current values; that is the whole point of a re-bind.
///
/// `expected_revision` is the caller's optimistic-concurrency claim on the
/// binding record itself; a mismatch is a conflict and changes nothing.
pub fn rebind_agent_binding(
    existing: &AgentBinding,
    request: &AgentBindingRequest,
    expected_revision: u64,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
    now_ms: u64,
) -> Result<AgentBinding, AgentBindingError> {
    if existing.revision() != expected_revision {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("binding_revision={expected_revision}"),
            actual: format!("binding_revision={}", existing.revision()),
        });
    }
    if request.contact_id != existing.contact_id() {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("contact_id={}", existing.contact_id()),
            actual: format!("contact_id={}", request.contact_id),
        });
    }
    if request.agent_id != existing.agent_id() {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("agent_id={}", existing.agent_id()),
            actual: format!("agent_id={}", request.agent_id),
        });
    }
    if request.workspace_id != existing.workspace_id() {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("workspace_id={}", existing.workspace_id()),
            actual: format!("workspace_id={}", request.workspace_id),
        });
    }
    if request.context_key.as_deref() != existing.context_key() {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("context_key={}", existing.context_key().unwrap_or("")),
            actual: format!(
                "context_key={}",
                request.context_key.as_deref().unwrap_or("")
            ),
        });
    }
    if request.incarnation.as_deref() != existing.incarnation() {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("incarnation={}", existing.incarnation().unwrap_or("")),
            actual: format!(
                "incarnation={}",
                request.incarnation.as_deref().unwrap_or("")
            ),
        });
    }
    let mut next = resolve_agent_binding(request, contacts, workspaces, now_ms)?;
    if existing.contact_workspace_revision() != next.contact_workspace_revision() {
        return Err(contact_workspace_conflict());
    }
    next.revision = existing.revision().saturating_add(1);
    Ok(next)
}

// ---------------------------------------------------------------------------
// 4. The fail-closed gate
// ---------------------------------------------------------------------------

/// The outcome of admitting one turn for a session that may carry a binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundTurnAdmission {
    /// The session carries no binding: the caller keeps its existing path.
    Unbound,
    /// The session is bound and the binding is valid right now.
    Admitted {
        /// The binding the turn must run under.
        binding: AgentBinding,
        /// The persona text that belongs to the bound contact, if it has one.
        ///
        /// This text is **only** ever a system-text input: it never widens
        /// or narrows a permission, a capability, or a tool scope.
        persona: Option<String>,
    },
}

/// Re-checks a stored binding against the *current* contact and workspace
/// facts.
///
/// This is the fail-closed gate. It never falls back to a default agent or a
/// default domain: a session that carries a binding either resolves to that
/// binding, or it is refused.
///
/// * `Ok(BoundTurnAdmission::Unbound)` — the session carries no binding; the
///   caller keeps its pre-existing path.
/// * `Ok(BoundTurnAdmission::Admitted { .. })` — the binding is valid right
///   now; the caller must run the turn under it.
/// * `Err(reason)` — the binding is present but no longer valid; the turn
///   must be refused.
pub fn admit_bound_turn(
    session: &Session,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
) -> Result<BoundTurnAdmission, AgentBindingError> {
    let Some(binding) = session.agent_binding() else {
        return Ok(BoundTurnAdmission::Unbound);
    };
    let contact = revalidate_binding(binding, contacts, workspaces)?;
    let persona = contact.persona().map(|text| text.as_str().to_string());
    Ok(BoundTurnAdmission::Admitted {
        binding: binding.clone(),
        persona,
    })
}

/// Admits one user turn for a session and appends it to the transcript.
///
/// * **Unbound** session → the turn is appended exactly as before (this batch
///   does not change the unbound path).
/// * **Bound and valid** → the turn is appended and the caller is told which
///   binding and persona to run it under.
/// * **Bound and invalid** → the turn is **refused**: [`Err`] is returned and
///   the session is not touched at all — no message is appended, no revision
///   is advanced, and the caller must not fall back to a default agent or a
///   default domain.
///
/// The refusal is the whole point: an archived contact or a revoked
/// workspace must never be answered by "quietly run as the default agent
/// instead".
///
/// The caller must not append the message itself when it uses this function.
pub fn admit_turn(
    session: &mut Session,
    message: NormalizedMessage,
    clock: &dyn Clock,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
) -> Result<BoundTurnAdmission, AgentBindingError> {
    let admission = admit_bound_turn(session, contacts, workspaces)?;
    session.append(message, clock);
    Ok(admission)
}

/// The persona text of the bound contact.
///
/// The only legal outlet for this text is the session's **system text**
/// (`DomainRuntimeContext::seeded_system`): it is an input to how the agent
/// speaks, never an input to what the agent may do. A persona never widens
/// or narrows a permission, a capability, or a tool scope.
///
/// The binding is re-validated first, so an archived contact yields a
/// refusal rather than a persona.
pub fn resolve_bound_persona(
    binding: &AgentBinding,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
) -> Result<Option<String>, AgentBindingError> {
    let contact = revalidate_binding(binding, contacts, workspaces)?;
    Ok(contact.persona().map(|text| text.as_str().to_string()))
}

/// The memory scope plan for a bound session.
///
/// The contract scope comes from the bound contact; the identities come from
/// the binding and the session. The binding is re-validated first, so an
/// archived contact yields a refusal rather than a plan.
pub fn bound_memory_plan(
    binding: &AgentBinding,
    subject_id: &str,
    session_id: &str,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
) -> Result<MemoryScopePlan, AgentBindingError> {
    let contact = revalidate_binding(binding, contacts, workspaces)?;
    Ok(contact_memory_plan(&contact, subject_id, session_id))
}

/// Re-checks one binding against the current facts of both registries and
/// returns the contact it points at.
///
/// The refusal order is the one documented on [`resolve_agent_binding`]:
/// terminal states (archived, revoked) come before the version checks.
fn revalidate_binding(
    binding: &AgentBinding,
    contacts: &dyn ContactRegistry,
    workspaces: &dyn WorkspaceRegistry,
) -> Result<Contact, AgentBindingError> {
    let contact = contacts
        .get(&binding.contact_id())
        .ok_or(AgentBindingError::ContactNotFound(binding.contact_id()))?;
    if contact.status() == ContactStatus::Archived {
        return Err(AgentBindingError::ContactArchived(binding.contact_id()));
    }
    if contact.agent_id() != binding.agent_id() {
        return Err(AgentBindingError::BindingConflict {
            expected: format!("agent_id={}", binding.agent_id()),
            actual: format!("agent_id={}", contact.agent_id()),
        });
    }
    if contact.persona_revision() != binding.persona_revision() {
        return Err(AgentBindingError::PersonaRevisionMismatch {
            expected: binding.persona_revision(),
            actual: contact.persona_revision(),
        });
    }
    let workspace = workspaces
        .get(&binding.workspace_id())
        .ok_or(AgentBindingError::WorkspaceNotFound(binding.workspace_id()))?;
    if workspace.is_revoked() {
        return Err(AgentBindingError::WorkspaceRevoked(binding.workspace_id()));
    }
    if workspace.revision != binding.workspace_revision() {
        return Err(AgentBindingError::WorkspaceRevisionMismatch {
            expected: binding.workspace_revision(),
            actual: workspace.revision,
        });
    }
    check_contact_workspace(
        &contact,
        binding.workspace_id(),
        binding.workspace_revision(),
    )?;
    if contact.workspace_binding_revision() != binding.contact_workspace_revision() {
        return Err(contact_workspace_conflict());
    }
    Ok(contact)
}

fn check_contact_workspace(
    contact: &Contact,
    workspace_id: WorkspaceId,
    revision: u64,
) -> Result<(), AgentBindingError> {
    if let Some(registered) = contact.workspace_binding() {
        if registered
            != Some(ContactWorkspaceBinding {
                workspace_id,
                revision,
            })
        {
            return Err(contact_workspace_conflict());
        }
    }
    Ok(())
}

fn contact_workspace_conflict() -> AgentBindingError {
    AgentBindingError::BindingConflict {
        expected: "currently registered contact workspace".into(),
        actual: "historical or unregistered contact workspace".into(),
    }
}
