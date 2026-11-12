//! Deterministic protocol instrumentation, never a real-model answer or score.
use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedRequest};
use serde_json::{json, Value};

pub fn proposal(request: &NormalizedRequest) -> Option<String> {
    if super::protocol::classify(request) != super::protocol::CallClass::Proposal {
        return None;
    }
    let mut response = json!({"decision":"abstain","target_id":null,"subject":null,"source_kind":null,"evidence":null,"target_anchor":null,"old_fragment":null,"new_fragment":null,"effective_time":null});
    let payload = request
        .messages
        .iter()
        .find(|m| m.role == MessageRole::User)
        .and_then(|m| serde_json::from_str::<Value>(&ContentPart::join_text(&m.content)).ok());
    if let Some(payload) = payload {
        let input = payload["input"].as_str().unwrap_or("");
        let state_change = input.contains("搬到了新家");
        if input.starts_with("更正：") || state_change {
            let clause = |text: &str| -> Option<String> {
                text.split(['。', '，', '：'])
                    .map(str::trim)
                    .find(|part| part.starts_with("我住在"))
                    .map(str::to_owned)
            };
            if let Some(new_clause) = clause(input) {
                let evidence = if state_change {
                    input.trim().trim_end_matches('。').to_owned()
                } else {
                    new_clause.clone()
                };
                let effective = if state_change {
                    input
                        .strip_prefix("我于")
                        .and_then(|text| text.split_once('日'))
                        .map(|(date, _)| format!("{date}日"))
                } else {
                    None
                };
                if let Some(candidate) = payload["candidates"].as_array().and_then(|rows| {
                    rows.iter().find(|row| {
                        row["text"]
                            .as_str()
                            .and_then(clause)
                            .is_some_and(|anchor| anchor != new_clause)
                    })
                }) {
                    let anchor = clause(candidate["text"].as_str().unwrap()).unwrap();
                    response = json!({"decision":if state_change{"state_change"}else{"correct_prior"},"target_id":candidate["id"],"subject":"self","source_kind":"direct_assertion","evidence":evidence,"target_anchor":anchor,"old_fragment":anchor.trim_start_matches("我住在"),"new_fragment":new_clause.trim_start_matches("我住在"),"effective_time":effective});
                }
            }
        }
    }
    Some(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::NormalizedMessage;
    use apeireth_runtime_assembly::canonical::memory_correction::{
        CORRECTION_PROPOSAL_PROTOCOL, CORRECTION_PROPOSAL_SYSTEM,
    };
    #[test]
    fn date_and_city_are_inside_the_exact_continuous_evidence_span() {
        let input = super::super::fixtures::STATE_CHANGE;
        let payload=json!({"protocol":CORRECTION_PROPOSAL_PROTOCOL,"input":input,"candidates":[{"id":"real-target","text":super::super::fixtures::BASIC_SEED}]}).to_string();
        let request = NormalizedRequest::new(
            "offline",
            vec![
                NormalizedMessage::system(CORRECTION_PROPOSAL_SYSTEM),
                NormalizedMessage::user(payload),
            ],
        );
        let answer: Value = serde_json::from_str(&proposal(&request).unwrap()).unwrap();
        assert_eq!(answer["evidence"], input.trim_end_matches('。'));
        assert_eq!(answer["effective_time"], "2026年11月10日");
        assert_eq!(answer["new_fragment"], "南汀市");
        assert!(answer["evidence"]
            .as_str()
            .unwrap()
            .contains(answer["effective_time"].as_str().unwrap()));
        assert_eq!(answer["target_anchor"], "我住在北岚市");
    }
}
