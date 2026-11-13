//! Independent Runtime-owned input markers; never added to model context.
use std::sync::{Arc, Mutex};

use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::canonical::{
    BehaviorModule, CommittedEvidenceAccess, CommittedInputOrigin, CommittedUserInput, HookPoint,
    InvocationOrigin, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::io::hash;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceAuditEntry {
    pub input: CommittedUserInput,
    pub assistant_sha256: Option<String>,
}

#[derive(Default)]
pub struct SourceAudit {
    pub entries: Mutex<Vec<SourceAuditEntry>>,
}

impl SourceAudit {
    pub fn verify(&self, session: &str, request: &str, role: &str, raw: &str) -> bool {
        self.entries.lock().unwrap().iter().any(|entry| {
            entry.input.session.to_string() == session
                && entry.input.request.to_string() == request
                && entry.input.origin == CommittedInputOrigin::DirectUser
                && if role == "assistant" {
                    entry.assistant_sha256.as_deref() == Some(hash(raw).as_str())
                } else {
                    entry.input.messages.iter().any(|m| {
                        m.role == MessageRole::User && ContentPart::join_text(&m.content) == raw
                    })
                }
        })
    }
}

pub struct AuditMarker {
    pub manifest: ModuleManifest,
    pub audit: Arc<SourceAudit>,
}

#[async_trait]
impl BehaviorModule for AuditMarker {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn committed_evidence_access(&self) -> CommittedEvidenceAccess {
        CommittedEvidenceAccess::CurrentInput
    }
    async fn on_hook(
        &self,
        hook: HookPoint,
        context: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if (hook == HookPoint::TurnStart || hook == HookPoint::AfterTurn)
            && context.invocation.origin == InvocationOrigin::UserTurn
        {
            let input = context
                .invoker()
                .committed_input()
                .await
                .map_err(|e| ModuleError::Message(e.to_string()))?
                .ok_or_else(|| ModuleError::Message("Runtime input marker unavailable".into()))?;
            let mut entries = self.audit.entries.lock().unwrap();
            if !entries.iter().any(|entry| {
                entry.input.request == input.request && entry.input.session == input.session
            }) {
                entries.push(SourceAuditEntry {
                    input,
                    assistant_sha256: None,
                });
            }
        }
        Ok(ModuleOutcome::continue_())
    }
}
