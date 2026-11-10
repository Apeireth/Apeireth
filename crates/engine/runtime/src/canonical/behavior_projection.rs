//! Budget authenticated compatibility data in the derived provider view only.
//! Facts and call/result identities remain immutable; no dynamic system entry.
use apeireth_orchestration::context_budget::SpillWriter;
use apeireth_protocol::canonical::{
    NormalizedMessage, ToolEventSourceV1, ToolEventV1, ToolOutcome,
};

use super::error::{RuntimeError, RuntimeResult};
use super::module::PromptOverlay;
use super::session::{Session, SessionEventKind};

pub(super) fn budget_module_context(
    messages: &[NormalizedMessage],
    session: &Session,
    budget: usize,
    spill: Option<(&SpillWriter, &str)>,
) -> RuntimeResult<Vec<NormalizedMessage>> {
    let facts = session
        .events
        .iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::ToolEventRecorded { fact }
                if matches!(fact.source(), ToolEventSourceV1::Module { .. }) =>
            {
                Some(fact)
            }
            _ => None,
        })
        .map(|fact| (fact.call().id.as_str(), fact))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut selected = Vec::new();
    let mut overlays = Vec::new();
    for (index, pair) in messages.windows(2).enumerate() {
        let Some(fact) = pair[0]
            .tool_calls
            .first()
            .and_then(|call| facts.get(call.id.as_str()))
        else {
            continue;
        };
        // A text-shaped imitation or unrelated tool result is never rewritten.
        if pair != fact.provider_messages() {
            continue;
        }
        let ToolOutcome::Ok { value } = &fact.result().outcome else {
            continue;
        };
        if value["untrusted_data"] != true {
            continue;
        }
        let Some(context) = value["context"].as_array() else {
            continue;
        };
        let start = overlays.len();
        for block in context {
            let Some(text) = block["text"].as_str() else {
                return Err(RuntimeError::misconfigured(
                    "invalid behavior context block",
                ));
            };
            overlays.push(match block["source"].as_str() {
                Some(source) => PromptOverlay::system_cross_source(source, text),
                None => PromptOverlay::system(text),
            });
        }
        selected.push((index, *fact, start..overlays.len()));
    }
    let budgeted = super::execute::budget_injected_overlays(&overlays, messages, budget, spill);
    let mut output = messages.to_vec();
    for (index, fact, range) in selected {
        let mut result = fact.result().clone();
        if let ToolOutcome::Ok { value } = &mut result.outcome {
            for (block, overlay) in value["context"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .zip(&budgeted[range])
            {
                block["text"] = apeireth_protocol::canonical::ContentPart::join_text(
                    &overlay.message().content,
                )
                .into();
            }
        }
        let derived = ToolEventV1::new(
            fact.call_id(),
            fact.source().clone(),
            fact.call().clone(),
            result,
        )
        .map_err(|e| RuntimeError::misconfigured(e.to_string()))?;
        let pair = derived.provider_messages();
        output[index..index + 2].clone_from_slice(&pair);
    }
    Ok(output)
}
