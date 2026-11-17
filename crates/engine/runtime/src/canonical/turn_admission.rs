//! Turn admission: the read-only gate a turn passes before it writes anything.
//!
//! A session may carry an agent binding (one contact, one workspace). The gate
//! revalidates that binding against the registries the composition root
//! installed, **before** the turn writes anything: a refused turn appends no
//! message and moves no revision, so it is never replayed against a binding
//! that changed in the meantime.
//!
//! The gate is read-only. It mutates neither the session nor a registry.

use apeireth_core::agent_contact::ContactRegistry;
use apeireth_core::kernel::Timestamp;

use super::agent_binding::{admit_bound_turn, BoundTurnAdmission};
use super::approval::ApprovalStatus;
use super::error::{RuntimeError, RuntimeResult};
use super::runtime::Runtime;
use super::session::{Session, SessionEventKind};
use super::workspace::WorkspaceRegistry;

/// Admits one turn, then does the housekeeping the turn depends on.
///
/// The order is the contract: **admit first, write later**. A refusal returns
/// before anything is written, so a refused turn leaves the transcript and the
/// session revision exactly as they were.
pub(super) async fn admit_turn(runtime: &Runtime, session: &mut Session) -> RuntimeResult<()> {
    admit_bound_binding(runtime, session)?;
    runtime.expire_active_approval_if_needed(session).await
}

/// Revalidates the session's agent binding, read-only.
///
/// * A session without a binding is admitted: it keeps the pre-existing
///   default-domain path, unchanged.
/// * A session with a binding but no installed registry is refused
///   (fail closed): the gate cannot prove the binding still holds, so it does
///   not let the turn through.
fn admit_bound_binding(runtime: &Runtime, session: &Session) -> RuntimeResult<()> {
    if session.agent_binding().is_none() {
        return Ok(());
    }
    let (Some(contacts), Some(workspaces)) =
        (runtime.contact_registry(), runtime.workspace_registry())
    else {
        return Err(RuntimeError::AgentBindingRejected {
            session: session.id,
            reason: "绑定校验所需的联系人/工作区注册表未装配".to_string(),
        });
    };
    let contacts: &dyn ContactRegistry = contacts.as_ref();
    let workspaces: &dyn WorkspaceRegistry = workspaces.as_ref();
    match admit_bound_turn(session, contacts, workspaces) {
        Ok(BoundTurnAdmission::Unbound) | Ok(BoundTurnAdmission::Admitted { .. }) => Ok(()),
        Err(error) => Err(RuntimeError::AgentBindingRejected {
            session: session.id,
            reason: error.to_string(),
        }),
    }
}

impl Runtime {
    /// Expire a pending approval that has outlived its TTL so a later turn can start.
    ///
    /// Claimed approvals are not expired here: their effect may already have
    /// started, and automatic retry is unsafe.
    pub(super) async fn expire_active_approval_if_needed(
        &self,
        session: &mut Session,
    ) -> RuntimeResult<()> {
        let Some(approval_id) = session.active_approval_id else {
            return Ok(());
        };
        let Some(approval) = session.approvals.get(&approval_id).cloned() else {
            session.active_approval_id = None;
            self.sessions.save(session).await?;
            return Ok(());
        };
        if approval.status != ApprovalStatus::Pending {
            return Ok(());
        }
        let now = Timestamp::from_clock(self.clock.as_ref());
        if !approval.is_expired(now) {
            return Ok(());
        }

        let expired = {
            let mut expired = approval.clone();
            expired.status = ApprovalStatus::Expired;
            expired.human_reason = None;
            expired
        };
        session.approvals.insert(approval_id, expired);
        session.active_approval_id = None;
        session.record(
            approval.request_id,
            approval.trace_id,
            SessionEventKind::ApprovalResolved {
                approval_id,
                decision: "expired".into(),
                round: approval.round,
                human_reason: None,
            },
            self.clock.as_ref(),
        );
        Self::append_skipped_tool_results(
            session,
            &approval.continuation.tool_calls,
            approval.continuation.next_tool_index,
            "operation expired before tool dispatch",
            self.clock.as_ref(),
        );
        self.sessions.save(session).await?;
        Ok(())
    }
}
