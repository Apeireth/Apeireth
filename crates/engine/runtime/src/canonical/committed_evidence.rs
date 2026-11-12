//! Read-only, turn-owned evidence from the canonical committed transcript.
use super::module::RuntimeModuleInvoker;
use super::{ModuleInvocationError, Session, SessionEventKind};
use apeireth_core::kernel::{RequestId, SessionId, Timestamp, TraceId};
use apeireth_plugin::agents::AgentPrincipal;
use apeireth_protocol::canonical::{MessageRole, NormalizedMessage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Host-registered modules must explicitly request this read-only authority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommittedEvidenceAccess {
    #[default]
    None,
    CurrentInput,
    OwnedSources,
    OwnedHistory,
}

/// Origin is assigned by Runtime, never inferred from a message's role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommittedInputOrigin {
    DirectUser,
    AgentTask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedUserInput {
    pub session: SessionId,
    pub request: RequestId,
    pub trace: TraceId,
    pub principal: Option<AgentPrincipal>,
    pub authorization_epoch: Option<String>,
    pub at: Timestamp,
    pub messages: Vec<NormalizedMessage>,
    pub origin: CommittedInputOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedTurnSnapshot {
    pub input: CommittedUserInput,
    pub response_id: String,
    pub response_content: String,
    pub committed_at: Timestamp,
    pub cursor: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedEvidencePage {
    pub turns: Vec<CommittedTurnSnapshot>,
    pub prepared_operations: Vec<CommittedOperationIntent>,
    pub next_cursor: Option<String>,
}

/// Frozen module operation evidence kept by the same store as its accepted input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedOperationIntent {
    pub input: CommittedUserInput,
    pub operation_id: String,
    pub module_id: String,
    pub payload: serde_json::Value,
    pub payload_hash: String,
    pub prepared_at: Timestamp,
    pub sequence: u64,
}

pub fn operation_payload_hash(payload: &serde_json::Value) -> Result<String, serde_json::Error> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(payload)?)
    ))
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceCursor {
    version: u8,
    streams: BTreeMap<String, usize>,
}

fn denied(reason: &str) -> ModuleInvocationError {
    ModuleInvocationError::Denied {
        reason: reason.into(),
    }
}

fn input_at(session: &Session, request: RequestId) -> Option<CommittedUserInput> {
    let inherited = session.inherited_event_count.unwrap_or(0);
    let (index, entry, origin, start_seq, end_seq) = session
        .log
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, entry)| {
            if index < inherited || entry.request != Some(request) {
                return None;
            }
            match &entry.event {
                SessionEventKind::InputCommitted {
                    version: 1,
                    origin,
                    start_seq,
                    end_seq,
                } => Some((index, entry, *origin, *start_seq, *end_seq)),
                _ => None,
            }
        })?;
    if start_seq > end_seq || end_seq > session.messages.len() {
        return None;
    }
    let trace = entry.trace?;
    if !session.log[index + 1..].iter().any(|event| {
        event.request == Some(request)
            && event.trace == Some(trace)
            && matches!(event.event, SessionEventKind::TurnStarted)
    }) {
        return None;
    }
    let messages = session.messages[start_seq..end_seq].to_vec();
    if session.log.iter().any(|entry| entry.request == Some(request)
        && matches!(entry.event, SessionEventKind::TurnResponseCommitted { version: 1, assistant_seq, .. }
            if session.log.iter().any(|record| matches!(record.event, SessionEventKind::Masked { seq, .. } if seq == assistant_seq)))) { return None; }
    if session.log.iter().any(|entry| {
        matches!(entry.event, SessionEventKind::Masked { seq, .. }
        if (start_seq..end_seq).contains(&seq))
    }) {
        return None;
    }
    if messages.iter().enumerate().any(|(offset, message)| {
        !session.log[..index].iter().any(|entry| {
            matches!(&entry.event, SessionEventKind::MessageAppended { seq, message: original }
            if *seq == start_seq + offset && original == message)
        })
    }) {
        return None;
    }
    if origin == CommittedInputOrigin::DirectUser
        && (messages.len() != 1 || messages[0].role != MessageRole::User)
    {
        return None;
    }
    Some(CommittedUserInput {
        session: session.id,
        request,
        trace,
        principal: session.principal.clone(),
        authorization_epoch: session.authorization_epoch.clone(),
        at: entry.at,
        messages,
        origin,
    })
}

fn cursor(session: SessionId, index: usize) -> String {
    serde_json::to_string(&(session, index)).expect("typed evidence cursor")
}

impl RuntimeModuleInvoker {
    pub(crate) async fn persist_operation(
        &self,
        operation_id: String,
        payload: serde_json::Value,
    ) -> Result<CommittedOperationIntent, ModuleInvocationError> {
        let owner = self.evidence_owner().await?;
        let permitted = self
            .evidence_runtime()
            .modules()
            .iter()
            .find(|module| module.manifest().id == self.evidence_module_id())
            .is_some_and(|module| {
                module.persists_recoverable_operations_for_principal(owner.principal.as_ref())
            });
        if !permitted || operation_id.is_empty() || operation_id.len() > 256 {
            return Err(denied("module has no operation evidence grant"));
        }
        let input = self
            .read_committed_input()
            .await?
            .ok_or_else(|| denied("accepted operation input is unavailable"))?;
        if input.origin != CommittedInputOrigin::DirectUser {
            return Err(denied("task input cannot prepare a user operation"));
        }
        let bytes = serde_json::to_vec(&payload)
            .map_err(|_| denied("operation payload cannot be serialized"))?;
        if bytes.len() > 1_048_576 {
            return Err(denied("operation payload exceeds evidence budget"));
        }
        let intent = CommittedOperationIntent {
            input,
            operation_id,
            module_id: self.evidence_module_id().into(),
            payload_hash: operation_payload_hash(&payload)
                .map_err(|_| denied("operation hash failed"))?,
            payload,
            prepared_at: Timestamp::from_clock(self.evidence_runtime().sessions().clock().as_ref()),
            sequence: 0,
        };
        let persisted = self
            .evidence_runtime()
            .sessions()
            .store()
            .put_operation_evidence(&intent)
            .await
            .map_err(|_| denied("operation evidence could not be committed"))?;
        self.evidence_owner().await?;
        if persisted.input != intent.input
            || persisted.operation_id != intent.operation_id
            || persisted.module_id != intent.module_id
            || persisted.payload_hash != intent.payload_hash
            || persisted.payload != intent.payload
            || persisted.sequence == 0
            || operation_payload_hash(&persisted.payload)
                .map_err(|_| denied("persisted operation hash failed"))?
                != persisted.payload_hash
        {
            return Err(denied("committed operation evidence integrity mismatch"));
        }
        Ok(persisted)
    }
    fn evidence_access(&self, owner: &Session) -> CommittedEvidenceAccess {
        self.evidence_runtime()
            .modules()
            .iter()
            .find(|module| module.manifest().id == self.evidence_module_id())
            .map(|module| module.committed_evidence_access_for_principal(owner.principal.as_ref()))
            .unwrap_or_default()
    }

    async fn evidence_owner(&self) -> Result<Session, ModuleInvocationError> {
        let runtime = self.evidence_runtime();
        let grant = runtime
            .authorize_session(self.evidence_origin().session)
            .await
            .map_err(|_| denied("committed evidence authorization is not active"))?;
        if grant
            .as_ref()
            .is_some_and(|grant| !grant.allows_behavior(self.evidence_module_id()))
        {
            return Err(denied("module is not authorized for committed evidence"));
        }
        let owner = runtime
            .sessions()
            .load(&self.evidence_origin().session)
            .await
            .map_err(|_| denied("committed evidence store is unavailable"))?
            .ok_or_else(|| denied("committed evidence session is missing"))?;
        // Store reads may yield. Revocation or owner replacement during that
        // await cannot reuse the grant checked before it.
        let active = match (&grant, &owner.principal) {
            (None, None) => owner.authorization_epoch.is_none(),
            (Some(grant), Some(principal)) => {
                grant.is_active()
                    && owner.authorization_epoch.as_deref() == Some(grant.epoch())
                    && runtime
                        .principal_authorizations
                        .as_ref()
                        .and_then(|authority| authority.get(principal))
                        .is_some_and(|current| current.epoch() == grant.epoch())
            }
            _ => false,
        };
        if !active {
            return Err(denied("committed evidence authority changed during read"));
        }
        Ok(owner)
    }

    fn same_owner(owner: &Session, source: &Session) -> bool {
        source.id == owner.id || (owner.principal.is_some() && owner.principal == source.principal)
    }

    pub(crate) async fn read_committed_input(
        &self,
    ) -> Result<Option<CommittedUserInput>, ModuleInvocationError> {
        let owner = self.evidence_owner().await?;
        if self.evidence_access(&owner) < CommittedEvidenceAccess::CurrentInput {
            return Err(denied("module has no committed input grant"));
        }
        Ok(input_at(&owner, self.evidence_origin().request))
    }

    pub(crate) async fn read_committed_input_for(
        &self,
        session: SessionId,
        request: RequestId,
    ) -> Result<Option<CommittedUserInput>, ModuleInvocationError> {
        let owner = self.evidence_owner().await?;
        if self.evidence_access(&owner) < CommittedEvidenceAccess::OwnedSources {
            return Err(denied("module has no owned source grant"));
        }
        let source = self
            .evidence_runtime()
            .sessions()
            .load(&session)
            .await
            .map_err(|_| denied("committed source store is unavailable"))?;
        let result = source
            .filter(|source| Self::same_owner(&owner, source))
            .and_then(|source| input_at(&source, request));
        self.evidence_owner().await?;
        Ok(result)
    }

    pub(crate) async fn read_committed_turns(
        &self,
        after: Option<String>,
        limit: usize,
    ) -> Result<CommittedEvidencePage, ModuleInvocationError> {
        let owner = self.evidence_owner().await?;
        if self.evidence_access(&owner) < CommittedEvidenceAccess::OwnedHistory {
            return Err(denied("module has no owned history grant"));
        }
        if !(1..=64).contains(&limit) {
            return Err(denied("evidence page limit must be between 1 and 64"));
        }
        let mut watermarks: EvidenceCursor = match after {
            Some(text) => {
                if text.len() > 1_048_576 {
                    return Err(denied("invalid evidence cursor"));
                }
                let value: EvidenceCursor =
                    serde_json::from_str(&text).map_err(|_| denied("invalid evidence cursor"))?;
                if value.version != 1 {
                    return Err(denied("unsupported evidence cursor version"));
                }
                value
            }
            None => EvidenceCursor {
                version: 1,
                streams: BTreeMap::new(),
            },
        };
        let mut sources: Vec<Session> = if owner.principal.is_none() {
            vec![owner.clone()]
        } else {
            self.evidence_runtime()
                .sessions()
                .store()
                .list()
                .await
                .map_err(|_| denied("committed history store is unavailable"))?
                .into_iter()
                .filter(|source| Self::same_owner(&owner, source))
                .collect()
        };
        sources.sort_by_key(|source| source.id.to_string());
        watermarks.streams.retain(|key, _| {
            sources.iter().any(|source| {
                key == &format!("turn:{}", source.id) || key == &format!("operation:{}", source.id)
            })
        });
        let mut turns = Vec::new();
        let mut prepared_operations = Vec::new();
        for source in sources {
            let source_key = format!("turn:{}", source.id);
            let previous = watermarks.streams.get(&source_key).copied();
            for (index, entry) in source
                .log
                .iter()
                .enumerate()
                .skip(source.inherited_event_count.unwrap_or(0))
            {
                if previous.is_some_and(|seq| index <= seq) {
                    continue;
                }
                watermarks.streams.insert(source_key.clone(), index);
                let SessionEventKind::TurnResponseCommitted {
                    version: 1,
                    response_id,
                    assistant_seq,
                } = &entry.event
                else {
                    continue;
                };
                let Some(request) = entry.request else {
                    continue;
                };
                let Some(input) = input_at(&source, request) else {
                    continue;
                };
                if entry.trace != Some(input.trace) {
                    continue;
                }
                let Some(response) = source.messages.get(*assistant_seq) else {
                    continue;
                };
                if response.role != MessageRole::Assistant || !response.tool_calls.is_empty() {
                    continue;
                }
                if source.log.iter().any(|entry| {
                    matches!(entry.event,
                    SessionEventKind::Masked { seq, .. } if seq == *assistant_seq)
                }) {
                    continue;
                }
                if !source.log[..index].iter().any(|entry| matches!(&entry.event,
                    SessionEventKind::MessageAppended { seq, message } if seq == assistant_seq && message == response)) { continue; }
                if !source.log[index + 1..].iter().any(|event| {
                    event.request == Some(request)
                        && event.trace == Some(input.trace)
                        && matches!(event.event, SessionEventKind::TurnCompleted { .. })
                }) {
                    continue;
                }
                turns.push(CommittedTurnSnapshot {
                    input,
                    response_id: response_id.clone(),
                    response_content: apeireth_protocol::canonical::ContentPart::join_text(
                        &response.content,
                    ),
                    committed_at: entry.at,
                    cursor: cursor(source.id, index),
                });
                if turns.len() + prepared_operations.len() == limit {
                    self.evidence_owner().await?;
                    return Ok(CommittedEvidencePage {
                        prepared_operations,
                        turns,
                        next_cursor: Some(
                            serde_json::to_string(&watermarks).expect("typed evidence watermarks"),
                        ),
                    });
                }
            }
            let key = format!("operation:{}", source.id);
            let previous = watermarks.streams.get(&key).copied();
            if self
                .evidence_runtime()
                .sessions()
                .store()
                .supports_operation_evidence()
            {
                let mut intents = self
                    .evidence_runtime()
                    .sessions()
                    .store()
                    .operation_evidence(&source.id)
                    .await
                    .map_err(|_| denied("operation evidence store is unavailable"))?;
                intents.sort_by_key(|intent| intent.sequence);
                if intents.iter().any(|intent| intent.sequence == 0)
                    || intents
                        .windows(2)
                        .any(|pair| pair[0].sequence == pair[1].sequence)
                {
                    return Err(denied("operation evidence sequence is invalid"));
                }
                for intent in intents {
                    let sequence = usize::try_from(intent.sequence)
                        .map_err(|_| denied("operation sequence overflow"))?;
                    if previous.is_some_and(|seq| sequence <= seq) {
                        continue;
                    }
                    watermarks.streams.insert(key.clone(), sequence);
                    let Some(input) = input_at(&source, intent.input.request) else {
                        continue;
                    };
                    if input != intent.input
                        || input.origin != CommittedInputOrigin::DirectUser
                        || intent.input.session != source.id
                        || operation_payload_hash(&intent.payload)
                            .map_err(|_| denied("operation payload hash failed"))?
                            != intent.payload_hash
                    {
                        continue;
                    }
                    prepared_operations.push(intent);
                    if turns.len() + prepared_operations.len() == limit {
                        self.evidence_owner().await?;
                        return Ok(CommittedEvidencePage {
                            turns,
                            prepared_operations,
                            next_cursor: Some(
                                serde_json::to_string(&watermarks)
                                    .expect("typed evidence watermarks"),
                            ),
                        });
                    }
                }
            }
        }
        self.evidence_owner().await?;
        Ok(CommittedEvidencePage {
            turns,
            prepared_operations,
            next_cursor: Some(
                serde_json::to_string(&watermarks).expect("typed evidence watermarks"),
            ),
        })
    }
}
