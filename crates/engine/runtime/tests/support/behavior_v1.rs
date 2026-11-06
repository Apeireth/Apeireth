//! S3 representation migration: decode authenticated V1 payloads for the old
//! content/order assertions, while independently requiring closed tool pairing.
use apeireth_protocol::canonical::{
    validate_closed_tool_context, ContentPart, MessageRole, NormalizedMessage,
};
pub fn texts(messages: &[NormalizedMessage]) -> Vec<String> {
    validate_closed_tool_context(messages).expect("V1 call/result pairing");
    messages
        .iter()
        .flat_map(|message| {
            let text = ContentPart::join_text(&message.content);
            if message.role == MessageRole::Tool {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if value["origin"] == "module"
                        && value["payload"]["value"]["untrusted_data"] == true
                    {
                        return value["payload"]["value"]["context"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|block| block["text"].as_str().unwrap().to_owned())
                            .collect::<Vec<_>>();
                    }
                }
            }
            if message.role == MessageRole::Assistant
                && !message.tool_calls.is_empty()
                && message
                    .tool_calls
                    .iter()
                    .all(|call| call.id.starts_with("module_"))
            {
                return Vec::new();
            }
            vec![text]
        })
        .collect()
}
