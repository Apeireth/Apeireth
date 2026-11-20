use super::{CanonicalEntryError, ContactId, SessionId};
use serde::{Deserialize, Serialize};

/// Native gateway request. HTTP and CLI transports can both construct it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanonicalChatRequest {
    /// Existing canonical session, or a fresh session when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    /// User input for this turn.
    pub input: String,
    /// Optional model override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// System instruction used only when the session is new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Require the existing session to belong to this contact. Tools are disabled for this mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_contact_id: Option<ContactId>,
}

impl CanonicalChatRequest {
    /// A request containing one user turn.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            session: None,
            input: input.into(),
            model: None,
            system: None,
            expected_contact_id: None,
        }
    }

    pub(super) fn effective_session(&self) -> Result<SessionId, CanonicalEntryError> {
        match self.session {
            Some(session) => Ok(session),
            None if self.expected_contact_id.is_none() => Ok(SessionId::new()),
            None => Err(CanonicalEntryError::InvalidRequest(
                "联系人对话需要显式会话标识。".into(),
            )),
        }
    }
}
