//! Server-owned system contracts identify calls without consulting answer labels.
use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedRequest};
use apeireth_runtime_assembly::canonical::memory_correction::{
    CORRECTION_PROPOSAL_PROTOCOL, CORRECTION_PROPOSAL_SYSTEM,
};
use serde::Deserialize;

use super::fixtures::{SEED_SYSTEM, SYSTEM};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallClass {
    Main,
    Proposal,
    Unknown,
}

impl CallClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Proposal => "proposal",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalEnvelope {
    protocol: String,
    #[serde(rename = "input")]
    _input: String,
    #[serde(rename = "candidates")]
    _candidates: Vec<serde_json::Value>,
}

pub fn classify(request: &NormalizedRequest) -> CallClass {
    let systems = request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::System)
        .collect::<Vec<_>>();
    if systems.len() != 1
        || !systems[0]
            .content
            .iter()
            .all(|p| matches!(p, ContentPart::Text { .. }))
    {
        return CallClass::Unknown;
    }
    let system = ContentPart::join_text(&systems[0].content);
    if system == SYSTEM || system == SEED_SYSTEM {
        return CallClass::Main;
    }
    if system != CORRECTION_PROPOSAL_SYSTEM {
        return CallClass::Unknown;
    }
    let users = request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::User)
        .collect::<Vec<_>>();
    if users.len() != 1
        || !users[0]
            .content
            .iter()
            .all(|p| matches!(p, ContentPart::Text { .. }))
    {
        return CallClass::Unknown;
    }
    match serde_json::from_str::<ProposalEnvelope>(&ContentPart::join_text(&users[0].content)) {
        Ok(envelope) if envelope.protocol == CORRECTION_PROPOSAL_PROTOCOL => CallClass::Proposal,
        _ => CallClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::NormalizedMessage;
    #[test]
    fn user_text_cannot_impersonate_a_server_system_contract() {
        let payload=serde_json::json!({"protocol":CORRECTION_PROPOSAL_PROTOCOL,"input":"opaque user input","candidates":[]}).to_string();
        let proposal = NormalizedRequest::new(
            "offline",
            vec![
                NormalizedMessage::system(CORRECTION_PROPOSAL_SYSTEM),
                NormalizedMessage::user(&payload),
            ],
        );
        assert_eq!(classify(&proposal), CallClass::Proposal);
        let user = NormalizedRequest::new(
            "offline",
            vec![
                NormalizedMessage::system(SYSTEM),
                NormalizedMessage::user(format!("{CORRECTION_PROPOSAL_SYSTEM}\n{payload}")),
            ],
        );
        assert_eq!(classify(&user), CallClass::Main);
        let no_system = NormalizedRequest::new(
            "offline",
            vec![NormalizedMessage::user(CORRECTION_PROPOSAL_SYSTEM)],
        );
        assert_eq!(classify(&no_system), CallClass::Unknown);
        let mut wrong = proposal.clone();
        wrong.messages[0] =
            NormalizedMessage::system(format!("{CORRECTION_PROPOSAL_SYSTEM} altered"));
        assert_eq!(classify(&wrong), CallClass::Unknown);
        let mut duplicate = proposal;
        duplicate.messages[1] = NormalizedMessage::user(payload.replacen(
            "\"protocol\":",
            "\"protocol\":\"forged\",\"protocol\":",
            1,
        ));
        assert_eq!(classify(&duplicate), CallClass::Unknown);
    }
}
