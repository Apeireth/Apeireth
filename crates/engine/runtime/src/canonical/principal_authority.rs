//! Host-owned, revocable authorization epochs. No model-facing mutation API.
use super::{Runtime, RuntimeError, RuntimeResult, Session};
use apeireth_core::kernel::{RequestId, SessionId};
use apeireth_plugin::agents::AgentPrincipal;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Mutex;
use tokio::sync::watch;

fn key(principal: &AgentPrincipal) -> String {
    format!("{}/{}", principal.profile, principal.namespace)
}
/// Immutable grant captured by a turn and by its authenticated behavior receipts.
#[derive(Clone)]
pub struct PrincipalGrant {
    epoch: String,
    behaviors: BTreeSet<String>,
    revoked: watch::Sender<bool>,
}
impl PrincipalGrant {
    pub fn epoch(&self) -> &str {
        &self.epoch
    }
    pub fn is_active(&self) -> bool {
        !*self.revoked.borrow()
    }
    pub fn allows_behavior(&self, id: &str) -> bool {
        self.is_active() && self.behaviors.contains(id)
    }
    /// Cancellation drops the canonical execution future, including owned tool tasks.
    /// A second check after completion closes the ready-response/revoke race.
    pub async fn run<T>(&self, future: impl Future<Output = RuntimeResult<T>>) -> RuntimeResult<T> {
        let mut revoked = self.revoked.subscribe();
        if *revoked.borrow() {
            return Err(denied());
        }
        let result = tokio::select! {
            biased;
            _ = revoked.changed() => return Err(denied()),
            result = future => result,
        };
        if !self.is_active() {
            return Err(denied());
        }
        result
    }
}
fn denied() -> RuntimeError {
    RuntimeError::misconfigured(
        "principal authorization changed; a new Host-bound session is required",
    )
}
/// Host authorization table shared by execution views. Registration grants no tool permissions.
#[derive(Default)]
pub struct PrincipalAuthorizations {
    grants: Mutex<BTreeMap<String, PrincipalGrant>>,
}
impl PrincipalAuthorizations {
    pub fn grant(
        &self,
        principal: &AgentPrincipal,
        behaviors: impl IntoIterator<Item = String>,
    ) -> PrincipalGrant {
        let grant = PrincipalGrant {
            epoch: RequestId::new().to_string(),
            behaviors: behaviors.into_iter().collect(),
            revoked: watch::channel(false).0,
        };
        let previous = self
            .grants
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key(principal), grant.clone());
        if let Some(previous) = previous {
            previous.revoked.send_replace(true);
        }
        grant
    }
    pub fn revoke(&self, principal: &AgentPrincipal) {
        if let Some(grant) = self
            .grants
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key(principal))
        {
            grant.revoked.send_replace(true);
        }
    }
    pub fn get(&self, principal: &AgentPrincipal) -> Option<PrincipalGrant> {
        self.grants
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key(principal))
            .filter(|g| g.is_active())
            .cloned()
    }
}
impl Runtime {
    /// Epochs persist with the transcript, so reload/fork cannot resurrect revoked data.
    pub(super) async fn authorize_session(
        &self,
        id: SessionId,
    ) -> RuntimeResult<Option<PrincipalGrant>> {
        let mut session = self.sessions.load_or_create(id).await?;
        self.dispatcher.bind_session_principal(&mut session)?;
        let grant = session
            .principal
            .as_ref()
            .and_then(|p| self.principal_authorizations.as_ref()?.get(p));
        let previous_epoch = session.authorization_epoch.clone();
        self.bind_authorization_epoch(&mut session, grant.as_ref())?;
        if session.principal.is_some() {
            if let Some(parent) = self
                .task
                .as_ref()
                .and_then(|t| t.input.origin.as_ref())
                .map(|origin| origin.session)
            {
                let parent = self.sessions.load(&parent).await?.ok_or_else(denied)?;
                if parent.principal != session.principal
                    || parent.authorization_epoch != session.authorization_epoch
                {
                    return Err(denied());
                }
            }
        }
        self.dispatcher.bind_session_principal(&mut session)?;
        if session.authorization_epoch != previous_epoch {
            self.sessions.save(&session).await?;
        }
        Ok(grant)
    }
    fn bind_authorization_epoch(
        &self,
        session: &mut Session,
        grant: Option<&PrincipalGrant>,
    ) -> RuntimeResult<()> {
        if session.principal.is_none() {
            return Ok(());
        }
        if session.authorization_epoch.is_some() && self.principal_authorizations.is_none() {
            return Err(denied());
        }
        let epoch = grant.map(PrincipalGrant::epoch).unwrap_or("unmapped");
        if let Some(previous) = &session.authorization_epoch {
            if previous != epoch {
                return Err(denied());
            }
        } else if self.principal_authorizations.is_some() {
            // Existing private transcripts may contain data from an earlier Host.
            // Attaching new authority requires a fresh session, never a relabel.
            if !session.messages.is_empty() {
                return Err(denied());
            }
            session.authorization_epoch = Some(epoch.to_owned());
        }
        Ok(())
    }
    pub(super) fn behavior_grant(&self, session: &Session, id: &str) -> Option<PrincipalGrant> {
        let principal = session.principal.as_ref()?;
        let grant = self.principal_authorizations.as_ref()?.get(principal)?;
        (session.authorization_epoch.as_deref() == Some(grant.epoch()) && grant.allows_behavior(id))
            .then_some(grant)
    }
}
