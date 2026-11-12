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
    source_time_utc: String,
    utc_offset_seconds: i32,
    #[serde(rename = "input")]
    _input: String,
    #[serde(rename = "candidates")]
    _candidates: Vec<serde_json::Value>,
}

impl ProposalEnvelope {
    fn has_valid_host_clock(&self) -> bool {
        chrono::DateTime::parse_from_rfc3339(&self.source_time_utc)
            .is_ok_and(|time| time.offset().local_minus_utc() == 0)
            && chrono::FixedOffset::east_opt(self.utc_offset_seconds).is_some()
    }
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
        Ok(envelope)
            if envelope.protocol == CORRECTION_PROPOSAL_PROTOCOL
                && envelope.has_valid_host_clock() =>
        {
            CallClass::Proposal
        }
        _ => CallClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::NormalizedMessage;

    fn payload() -> serde_json::Value {
        serde_json::json!({"protocol":CORRECTION_PROPOSAL_PROTOCOL,"source_time_utc":"2000-01-02T03:04:05.006Z","utc_offset_seconds":0,"input":"opaque user input","candidates":[]})
    }

    fn proposal(payload: impl Into<String>) -> NormalizedRequest {
        NormalizedRequest::new(
            "offline",
            vec![
                NormalizedMessage::system(CORRECTION_PROPOSAL_SYSTEM),
                NormalizedMessage::user(payload),
            ],
        )
    }
    #[test]
    fn user_text_cannot_impersonate_a_server_system_contract() {
        let payload = payload().to_string();
        let proposal = proposal(payload.clone());
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

    #[test]
    fn proposal_clock_fields_are_required_and_reject_fake_dates_and_invalid_offsets() {
        let valid = payload();
        assert_eq!(classify(&proposal(valid.to_string())), CallClass::Proposal);
        let mut zero_offset = valid.clone();
        zero_offset["source_time_utc"] = serde_json::json!("2000-01-02T03:04:05.006+00:00");
        zero_offset["utc_offset_seconds"] = serde_json::json!(28_800);
        assert_eq!(
            classify(&proposal(zero_offset.to_string())),
            CallClass::Proposal
        );
        for field in ["source_time_utc", "utc_offset_seconds"] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert_eq!(classify(&proposal(missing.to_string())), CallClass::Unknown);
        }
        for date in [
            "not-a-time",
            "2000-02-30T03:04:05.006Z",
            "2000-01-02",
            "2000-01-02T03:04:05.006+08:00",
            "2000-01-02T03:04:05.006UTC",
        ] {
            let mut fake = valid.clone();
            fake["source_time_utc"] = serde_json::json!(date);
            assert_eq!(
                classify(&proposal(fake.to_string())),
                CallClass::Unknown,
                "{date}"
            );
        }
        for offset in [
            serde_json::json!(86_400),
            serde_json::json!(-86_400),
            serde_json::json!(i32::MAX),
            serde_json::json!("0"),
            serde_json::json!(0.5),
        ] {
            let mut fake = valid.clone();
            fake["utc_offset_seconds"] = offset;
            assert_eq!(classify(&proposal(fake.to_string())), CallClass::Unknown);
        }
    }

    #[test]
    fn clock_protocol_rejects_duplicates_unknown_fields_and_previous_version() {
        let valid = payload();
        for field in ["source_time_utc", "utc_offset_seconds"] {
            let duplicate = valid.to_string().replacen(
                &format!("\"{field}\":"),
                &format!("\"{field}\":{},\"{field}\":", valid[field]),
                1,
            );
            assert_eq!(classify(&proposal(duplicate)), CallClass::Unknown);
        }
        let mut unknown = valid.clone();
        unknown["oracle"] = serde_json::json!("not part of the protocol");
        assert_eq!(classify(&proposal(unknown.to_string())), CallClass::Unknown);
        let mut old_version = valid;
        old_version["protocol"] = serde_json::json!("apeireth.memory-correction.v1");
        assert_eq!(
            classify(&proposal(old_version.to_string())),
            CallClass::Unknown
        );
    }
}
