//! Behavioral checks of the versioned event boundary and existing wire adapters.

use apeireth_core::kernel::{PluginId, TaskId};
use apeireth_protocol::adapter::ProtocolAdapter;
use apeireth_protocol::adapters::{
    AnthropicMessagesAdapter, GeminiAdapter, OpenAiChatAdapter, OpenAiResponsesAdapter,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModuleHookV1, NormalizedMessage, NormalizedRequest, NormalizedTool,
    NormalizedToolChoice, ToolCall, ToolEventBatch, ToolEventError, ToolEventInsertion,
    ToolEventLimits, ToolEventSourceV1, ToolEventV1, ToolResult,
};
use serde_json::{json, Value};

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "memory_retrieve".into(),
        arguments: json!({"query": "last meeting"}),
    }
}

fn event(id: TaskId, wire_id: &str, payload: Value) -> ToolEventV1 {
    ToolEventV1::new(
        id,
        ToolEventSourceV1::Module {
            module_id: PluginId::new("memory").unwrap(),
            hook: ModuleHookV1::BeforeModelCall,
        },
        call(wire_id),
        ToolResult::ok(wire_id, payload),
    )
    .unwrap()
}

fn batch() -> ToolEventBatch {
    ToolEventBatch::new(ToolEventLimits::default())
}

fn projected_request() -> NormalizedRequest {
    let mut records = batch();
    records
        .push(event(
            TaskId::new(),
            "call_memory_1",
            json!(["remembered fact"]),
        ))
        .unwrap();
    let mut messages = vec![NormalizedMessage::user("what did we decide?")];
    records.append_to_closed_context(&mut messages).unwrap();
    let mut request = NormalizedRequest::new("test-model", messages);
    request.max_tokens = Some(128);
    request.tools = vec![NormalizedTool::new("memory_retrieve").with_parameters(
        serde_json::from_value(json!({
            "type": "object", "properties": {"query": {"type": "string"}},
            "required": ["query"], "additionalProperties": false,
        }))
        .unwrap(),
    )];
    request
}

#[test]
fn module_provenance_is_a_tool_payload_with_a_matched_assistant_call() {
    let id = TaskId::new();
    let record = event(id, "call_1", json!({"fact": "周五"}));
    let [assistant, result] = record.provider_messages();
    assert_eq!(assistant.role, MessageRole::Assistant);
    assert_eq!(assistant.tool_calls[0].id, "call_1");
    assert_eq!(result.role, MessageRole::Tool);
    assert_eq!(result.tool_call_id.as_deref(), Some("call_1"));
    let body: Value = serde_json::from_str(&ContentPart::join_text(&result.content)).unwrap();
    assert_eq!(body["origin"], "module");
    assert_eq!(body["module_id"], "memory");
    assert_eq!(body["call_id"], id.to_string());
    assert_eq!(body["hook"], "before_model_call");
    assert_eq!(
        body["payload"],
        json!({"status": "ok", "value": {"fact": "周五"}})
    );
}

#[test]
fn model_errors_keep_the_existing_renderer_and_canonical_retryability() {
    let record = ToolEventV1::new(
        TaskId::new(),
        ToolEventSourceV1::Model { module_id: None },
        call("wire"),
        ToolResult::retryable_error("wire", "try later"),
    )
    .unwrap();
    assert!(record.result().outcome.is_retryable());
    assert_eq!(
        ContentPart::join_text(&record.provider_messages()[1].content),
        "error: try later"
    );
}

#[test]
fn constructors_and_deserialization_both_reject_mismatched_results() {
    assert_eq!(
        ToolEventV1::new(
            TaskId::new(),
            ToolEventSourceV1::Model { module_id: None },
            call("a"),
            ToolResult::ok("b", json!(1)),
        )
        .unwrap_err(),
        ToolEventError::Correlation
    );
    let mut raw = serde_json::to_value(event(TaskId::new(), "a", json!(1))).unwrap();
    raw["result"]["tool_call_id"] = json!("b");
    assert!(serde_json::from_value::<ToolEventV1>(raw).is_err());
}

#[test]
fn names_arguments_and_version_are_validated_without_exposing_payloads() {
    let id = TaskId::new();
    assert_eq!(
        ToolEventV1::new(
            id,
            ToolEventSourceV1::Model { module_id: None },
            call("a"),
            ToolResult::ok("a", json!(1)).with_name("different"),
        )
        .unwrap_err(),
        ToolEventError::ToolName
    );
    let mut invalid = call("a");
    invalid.arguments = json!([1]);
    assert_eq!(
        ToolEventV1::new(
            id,
            ToolEventSourceV1::Model { module_id: None },
            invalid,
            ToolResult::ok("a", json!(1)),
        )
        .unwrap_err(),
        ToolEventError::InvalidArguments
    );
    for wire_id in ["", " padded", "line\nbreak"] {
        assert!(ToolEventV1::new(
            id,
            ToolEventSourceV1::Model { module_id: None },
            call(wire_id),
            ToolResult::ok(wire_id, json!("private payload")),
        )
        .is_err());
    }
    let good = event(id, "a", json!(1));
    let raw = serde_json::to_value(&good).unwrap();
    assert_eq!(
        serde_json::from_value::<ToolEventV1>(raw.clone()).unwrap(),
        good
    );
    let mut future = raw;
    future["version"] = json!(2);
    assert!(serde_json::from_value::<ToolEventV1>(future).is_err());
}

#[test]
fn identical_delivery_is_idempotent_even_after_projection() {
    let record = event(TaskId::new(), "a", json!(1));
    let mut records = batch();
    assert_eq!(
        records.push(record.clone()).unwrap(),
        ToolEventInsertion::Inserted
    );
    let bytes = records.bytes();
    let mut context = vec![NormalizedMessage::user("question")];
    records.append_to_closed_context(&mut context).unwrap();
    let once = context.clone();
    assert_eq!(records.push(record).unwrap(), ToolEventInsertion::Duplicate);
    records.append_to_closed_context(&mut context).unwrap();
    assert_eq!(context, once);
    assert_eq!(records.bytes(), bytes);
}

#[test]
fn conflicting_delivery_and_late_batch_errors_are_atomic() {
    let id = TaskId::new();
    let mut records = batch();
    let original = event(id, "a", json!(1));
    records.push(original.clone()).unwrap();
    let bytes = records.bytes();
    assert_eq!(
        records
            .extend_atomic([
                event(TaskId::new(), "b", json!(2)),
                event(id, "a", json!(3)),
            ])
            .unwrap_err(),
        ToolEventError::ConflictingExecution(id)
    );
    assert_eq!(records.events(), &[original]);
    assert_eq!(records.bytes(), bytes);
}

#[test]
fn budgets_count_utf8_bytes_and_do_not_charge_or_insert_failed_records() {
    let record = event(TaskId::new(), "a", json!("中文"));
    let encoded = serde_json::to_vec(&record).unwrap();
    assert!(encoded.len() > String::from_utf8(encoded.clone()).unwrap().chars().count());
    let mut records = ToolEventBatch::new(ToolEventLimits {
        max_events: 1,
        max_bytes: encoded.len() - 1,
    });
    assert_eq!(
        records.push(record.clone()).unwrap_err(),
        ToolEventError::Budget("bytes")
    );
    assert!(records.events().is_empty());
    assert_eq!(records.bytes(), 0);
    let mut records = ToolEventBatch::new(ToolEventLimits {
        max_events: 1,
        max_bytes: encoded.len(),
    });
    records.push(record.clone()).unwrap();
    assert_eq!(records.push(record).unwrap(), ToolEventInsertion::Duplicate);
    assert_eq!(
        records
            .push(event(TaskId::new(), "b", json!(1)))
            .unwrap_err(),
        ToolEventError::Budget("events")
    );
    assert_eq!(records.events().len(), 1);
}

#[test]
fn an_open_or_orphaned_tool_history_is_never_partially_extended() {
    let mut records = batch();
    records.push(event(TaskId::new(), "new", json!(1))).unwrap();
    let mut open = vec![NormalizedMessage::assistant_with_tool_calls(
        "",
        vec![call("pending")],
    )];
    let before = open.clone();
    assert_eq!(
        records.append_to_closed_context(&mut open).unwrap_err(),
        ToolEventError::OpenCalls
    );
    assert_eq!(open, before);
    let mut orphan = vec![ToolResult::ok("missing", json!(1)).into_message()];
    let before = orphan.clone();
    assert_eq!(
        records.append_to_closed_context(&mut orphan).unwrap_err(),
        ToolEventError::Correlation
    );
    assert_eq!(orphan, before);
    let mut closed = vec![NormalizedMessage::user("ok")];
    records.append_to_closed_context(&mut closed).unwrap();
    assert_eq!(
        closed.len(),
        3,
        "failed projection did not advance the cursor"
    );
}

#[test]
fn execution_identity_does_not_conflate_closed_rounds_that_reuse_a_wire_id() {
    let mut records = batch();
    records
        .extend_atomic([
            event(TaskId::new(), "reused", json!(1)),
            event(TaskId::new(), "reused", json!(2)),
        ])
        .unwrap();
    let mut context = vec![NormalizedMessage::user("question")];
    records.append_to_closed_context(&mut context).unwrap();
    assert_eq!(context.len(), 5);
    let mut more = batch();
    more.push(event(TaskId::new(), "next", json!(3))).unwrap();
    more.append_to_closed_context(&mut context).unwrap();
    assert_eq!(context.len(), 7);
}

#[test]
fn pending_parallel_calls_must_all_be_answered_before_injection() {
    let mut context = vec![
        NormalizedMessage::assistant_with_tool_calls("", vec![call("a"), call("b")]),
        ToolResult::ok("b", json!(2)).into_message(),
    ];
    let mut records = batch();
    records.push(event(TaskId::new(), "c", json!(3))).unwrap();
    assert_eq!(
        records.append_to_closed_context(&mut context).unwrap_err(),
        ToolEventError::OpenCalls
    );
    context.push(ToolResult::ok("a", json!(1)).into_message());
    records.append_to_closed_context(&mut context).unwrap();
    assert_eq!(context.len(), 5);
}

#[test]
fn chat_and_messages_formats_keep_matched_ids_and_only_schema_fields() {
    let request = projected_request();
    let chat = OpenAiChatAdapter::new().adapt_request(&request).unwrap();
    assert_eq!(chat["messages"][1]["tool_calls"][0]["id"], "call_memory_1");
    assert_eq!(chat["messages"][2]["tool_call_id"], "call_memory_1");
    assert!(chat["messages"][2].get("name").is_none());
    let blocks = AnthropicMessagesAdapter::new()
        .adapt_request(&request)
        .unwrap();
    assert_eq!(blocks["messages"][1]["content"][0]["id"], "call_memory_1");
    assert_eq!(
        blocks["messages"][2]["content"][0]["tool_use_id"],
        "call_memory_1"
    );
    assert!(blocks["messages"][2]["content"][0]
        .get("tool_name")
        .is_none());
}

#[test]
fn response_items_keep_each_call_including_assistant_text_and_flat_tool_schema() {
    let mut request = projected_request();
    request.messages[1].content = vec![ContentPart::text_only("checking")];
    request.messages[1].tool_calls.push(call("call_memory_2"));
    request.messages.push(
        ToolResult::ok("call_memory_2", json!(2))
            .with_name("memory_retrieve")
            .into_message(),
    );
    let wire = OpenAiResponsesAdapter::new()
        .adapt_request(&request)
        .unwrap();
    let input = wire["input"].as_array().unwrap();
    let calls: Vec<_> = input
        .iter()
        .filter(|item| item["type"] == "function_call")
        .collect();
    let results: Vec<_> = input
        .iter()
        .filter(|item| item["type"] == "function_call_output")
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(results.len(), 2);
    for (call, result) in calls.iter().zip(results) {
        assert_eq!(call["call_id"], result["call_id"]);
        assert_eq!(call["name"], "memory_retrieve");
        assert!(
            serde_json::from_str::<Value>(call["arguments"].as_str().unwrap())
                .unwrap()
                .is_object()
        );
    }
    assert!(input
        .iter()
        .any(|item| item["role"] == "assistant" && item["content"] == "checking"));
    assert_eq!(wire["tools"][0]["name"], "memory_retrieve");
    assert!(wire["tools"][0].get("function").is_none());
}

#[test]
fn function_response_format_preserves_the_required_tool_name() {
    let request = projected_request();
    let wire = GeminiAdapter::new().adapt_request(&request).unwrap();
    assert_eq!(
        wire["contents"][1]["parts"][0]["functionCall"]["name"],
        "memory_retrieve"
    );
    assert_eq!(
        wire["contents"][2]["parts"][0]["functionResponse"]["name"],
        "memory_retrieve"
    );
    let body: Value = serde_json::from_str(
        wire["contents"][2]["parts"][0]["functionResponse"]["response"]["content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["origin"], "module");
}

#[test]
fn specific_selection_keeps_history_and_resolves_an_optional_result_name_echo() {
    let mut request = projected_request();
    request.tool_choice = Some(NormalizedToolChoice::Specific {
        name: "memory_retrieve".into(),
    });
    request.messages[2].name = None;
    let wire = GeminiAdapter::new().adapt_request(&request).unwrap();
    assert_eq!(wire["contents"].as_array().unwrap().len(), 3);
    assert_eq!(
        wire["contents"][2]["parts"][0]["functionResponse"]["name"],
        "memory_retrieve"
    );
    assert_eq!(
        wire["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"],
        json!(["memory_retrieve"])
    );
    let wire = OpenAiResponsesAdapter::new()
        .adapt_request(&request)
        .unwrap();
    assert_eq!(
        wire["tool_choice"],
        json!({"type": "function", "name": "memory_retrieve"})
    );
}

#[test]
fn parallel_same_name_calls_receive_distinct_ids_and_duplicate_vendor_ids_fail() {
    let raw = json!({"responseId": "response-parallel", "modelVersion": "test-model", "candidates": [{"content": {"parts": [
        {"functionCall": {"name": "search", "args": {"q": "one"}}},
        {"functionCall": {"name": "search", "args": {"q": "two"}}},
    ]}, "finishReason": "STOP"}]});
    let response = GeminiAdapter::new().adapt_response(&raw).unwrap();
    assert_eq!(response.tool_calls.len(), 2);
    assert_ne!(response.tool_calls[0].id, response.tool_calls[1].id);
    let mut duplicate = raw;
    for part in duplicate["candidates"][0]["content"]["parts"]
        .as_array_mut()
        .unwrap()
    {
        part["functionCall"]["id"] = json!("same");
    }
    assert!(GeminiAdapter::new().adapt_response(&duplicate).is_err());
}

#[test]
fn every_adapter_rejects_orphan_results_and_unanswered_calls() {
    let adapters: Vec<Box<dyn ProtocolAdapter>> = vec![
        Box::new(OpenAiChatAdapter::new()),
        Box::new(AnthropicMessagesAdapter::new()),
        Box::new(OpenAiResponsesAdapter::new()),
        Box::new(GeminiAdapter::new()),
    ];
    for adapter in adapters {
        for messages in [
            vec![ToolResult::ok("orphan", json!(1)).into_message()],
            vec![NormalizedMessage::assistant_with_tool_calls(
                "",
                vec![call("unanswered")],
            )],
        ] {
            let mut request = NormalizedRequest::new("test-model", messages);
            request.max_tokens = Some(128);
            assert!(
                adapter.adapt_request(&request).is_err(),
                "{} accepted an open tool history",
                adapter.name()
            );
        }
    }
}

#[test]
fn parallel_results_share_one_response_turn_and_keep_call_ids() {
    let mut request = projected_request();
    request.messages[1].tool_calls.push(call("second"));
    request.messages.push(
        ToolResult::ok("second", json!(2))
            .with_name("memory_retrieve")
            .into_message(),
    );
    let messages = AnthropicMessagesAdapter::new()
        .adapt_request(&request)
        .unwrap();
    assert_eq!(messages["messages"].as_array().unwrap().len(), 3);
    assert_eq!(
        messages["messages"][2]["content"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        messages["messages"][2]["content"][0]["tool_use_id"],
        "call_memory_1"
    );
    assert_eq!(
        messages["messages"][2]["content"][1]["tool_use_id"],
        "second"
    );
    let gemini = GeminiAdapter::new().adapt_request(&request).unwrap();
    assert_eq!(gemini["contents"].as_array().unwrap().len(), 3);
    assert_eq!(gemini["contents"][2]["parts"].as_array().unwrap().len(), 2);
    for i in 0..2 {
        assert_eq!(
            gemini["contents"][1]["parts"][i]["functionCall"]["id"],
            gemini["contents"][2]["parts"][i]["functionResponse"]["id"]
        );
    }
}
