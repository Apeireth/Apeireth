//! Exact project workspace identity for canonical turns and approval resumes.

use apeireth_core::kernel::ids::WorkspaceId;
use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_protocol::canonical::NormalizedTool;
use serde::{Deserialize, Serialize};

use super::workspace::WorkspacePermissionScope;
use super::{Runtime, RuntimeError, RuntimeResult, Session};

/// The backend workspace revision a project session was opened against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectWorkspaceBinding {
    pub workspace_id: WorkspaceId,
    pub revision: u64,
}

/// The two capabilities that may be dispatched by a project turn.
#[derive(Clone, Copy)]
pub(super) enum ProjectFileCapability {
    Read,
    Write,
}

impl ProjectFileCapability {
    pub(super) fn classify(id: &CapabilityId) -> Option<Self> {
        match id.as_str() {
            "tool.project_files_read" => Some(Self::Read),
            "tool.project_files_write" => Some(Self::Write),
            _ => None,
        }
    }

    fn allowed_in(self, session: &Session, permission: WorkspacePermissionScope) -> bool {
        matches!(self, Self::Read)
            || (permission == WorkspacePermissionScope::ApprovalWrite
                && session.settings.permission_preset != super::PermissionPreset::ReadOnly)
    }
}

impl Runtime {
    pub(super) fn confirm_project_workspace(
        &self,
        session: SessionId,
        binding: ProjectWorkspaceBinding,
    ) -> RuntimeResult<WorkspacePermissionScope> {
        let registry = self
            .workspace_registry
            .as_ref()
            .ok_or_else(|| refused(session, "the project workspace registry is unavailable"))?;
        let mut permission = None;
        registry
            .with_desktop_workspace_root(
                &binding.workspace_id,
                binding.revision,
                &mut |_, confirmed| permission = Some(confirmed),
            )
            .map_err(|_| {
                refused(
                    session,
                    "the project workspace binding is unavailable or stale",
                )
            })?;
        permission.ok_or_else(|| refused(session, "the project workspace root is unavailable"))
    }

    /// Verify exact session association and live workspace authority before replaying a receipt.
    pub async fn verify_project_workspace_session(
        &self,
        session_id: SessionId,
        binding: ProjectWorkspaceBinding,
    ) -> RuntimeResult<()> {
        let session = self.sessions.load(&session_id).await?;
        if !session.is_some_and(|session| {
            session.project_workspace_binding() == Some(binding)
                && session.agent_binding().is_none()
        }) {
            return Err(refused(
                session_id,
                "the session does not belong to this project workspace",
            ));
        }
        self.confirm_project_workspace(session_id, binding)
            .map(|_| ())
    }

    pub(super) fn admit_project_workspace(
        &self,
        session: &mut Session,
        claim: Option<ProjectWorkspaceBinding>,
    ) -> RuntimeResult<()> {
        let stored = session.project_workspace_binding();
        let Some(claim) = claim else {
            return match stored {
                None => Ok(()),
                Some(_) => Err(refused(
                    session.id,
                    "the project workspace binding is required",
                )),
            };
        };
        if self.task.is_some() || session.agent_binding().is_some() {
            return Err(refused(
                session.id,
                "this session cannot open a project workspace",
            ));
        }
        match stored {
            Some(expected) if claim != expected => {
                return Err(refused(
                    session.id,
                    "the session already belongs to another workspace revision",
                ));
            }
            None if !session.is_empty()
                || !session.log.is_empty()
                || !session.approvals.is_empty() =>
            {
                return Err(refused(
                    session.id,
                    "only an empty ordinary session can open a project workspace",
                ));
            }
            _ => {}
        }
        self.confirm_project_workspace(session.id, claim)?;
        if stored.is_none() {
            session.set_project_workspace_binding(claim, self.clock.as_ref());
        }
        Ok(())
    }

    pub(super) fn turn_tool_declarations(
        &self,
        session: &Session,
        binding: Option<ProjectWorkspaceBinding>,
    ) -> RuntimeResult<Vec<NormalizedTool>> {
        let Some(binding) = binding else {
            return Ok(self.tool_declarations_for_session(session.id));
        };
        let permission = self.confirm_project_workspace(session.id, binding)?;
        Ok(self
            .capabilities
            .capabilities()
            .into_iter()
            .filter(|tool| {
                tool.model_visible()
                    && tool.module_contract().model_visible()
                    && self.dispatcher.allows(session.id, tool.id())
                    && ProjectFileCapability::classify(tool.id())
                        .is_some_and(|capability| capability.allowed_in(session, permission))
            })
            .map(|tool| tool.declaration())
            .collect())
    }

    pub(super) fn project_dispatch_allowed(
        &self,
        session: &Session,
        name: &str,
        automatic: bool,
    ) -> bool {
        let capability = self
            .resolve_tool(name)
            .and_then(|tool| ProjectFileCapability::classify(tool.id()));
        match (session.project_workspace_binding(), capability) {
            (None, None) => true,
            (Some(binding), Some(capability)) if !automatic => self
                .confirm_project_workspace(session.id, binding)
                .is_ok_and(|permission| capability.allowed_in(session, permission)),
            _ => false,
        }
    }
}

pub(super) fn refused(session: SessionId, reason: &'static str) -> RuntimeError {
    RuntimeError::ProjectWorkspaceRejected { session, reason }
}
