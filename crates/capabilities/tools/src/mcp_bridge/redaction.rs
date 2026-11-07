//! Bounded convergence for server-authored content (declaration + result face).
//!
//! An external tool's **declaration** (the description and the input schema
//! the server offers) and its **result** (the text and content blocks the
//! server returns) are both written outside this process. Both reach the
//! model verbatim unless something bounds them, so this module is the last
//! gate before server-authored bytes enter the model's tool directory or its
//! context.
//!
//! # Three guards, one refusal
//!
//! 1. **Bounded** — every string that reaches the model has a length cap.
//! 2. **Converged** — the declared schema keeps its properties but loses its
//!    unbounded shape: depth, property count, node count, and the dialect
//!    metadata keys are all capped. The convergence is *bounded*, never
//!    *total*: a normal schema keeps its properties verbatim.
//! 3. **Cleaned** — the invisible and directional characters that carry
//!    prompt injections are removed from the declaration. The character
//!    class is the governance crate's own
//!    ([`apeireth_governance::ToolDescAuditor::is_dangerous_char`]) — this
//!    module does not define a second one.
//!
//! # What this module deliberately does not do
//!
//! It does not judge the *meaning* of server-authored text. Remote content
//! is data, not instruction: filtering it by keyword list would be
//! unreliable (it misses every shape it does not know) and destructive (it
//! silently edits the content the human just approved). The defences that
//! hold are the bound, the converged shape, and the per-call human approval
//! the runtime governance chain owns (see the bridge module docs).
//!
//! This module is **not** the egress redaction bridge
//! ([`crate::redaction_bridge`]): that one swaps registered secrets for
//! placeholders on the way out. This one only bounds and converges what a
//! server authored.
//!
//! It sits beside the bridge rather than in the bridge's module root
//! because that root is frozen for this batch; it is wired into the
//! model-facing boundary ([`super::tool`]) as `tool::redaction`.

use apeireth_governance::ToolDescAuditor;
use apeireth_plugin::mcp::ContentBlock;
use apeireth_protocol::canonical::ToolParameters;
use serde_json::{Map, Value};

/// The maximum characters of one declaration description.
///
/// Sits inside the repository's description band
/// (`apeireth_plugin::bounds::MAX_DESC_LEN`, 512 chars) and under the
/// 500-char mark this batch asked for.
pub const DECLARATION_MAX_DESCRIPTION_CHARS: usize = 500;

/// The maximum characters of one string inside a declared schema.
///
/// The short-text band of the repository (`content_feed::TITLE_MAX_CHARS`,
/// 200 chars): a property description is a label, not an essay.
pub const DECLARATION_MAX_SCHEMA_STRING_CHARS: usize = 200;

/// The maximum characters of one schema property's name.
///
/// The identifier band of the repository (`plugin::bounds::MAX_NAME_LEN`,
/// `GUARD_RULE_ID_MAX_BYTES`, 64 chars).
pub const DECLARATION_MAX_PROPERTY_NAME_CHARS: usize = 64;

/// The maximum nesting depth of a declared input schema.
///
/// The model-facing depth band (`organ::prompt_assembly`'s default depth, 8;
/// `search::MAX_DEPTH`, 10). A schema nested deeper than this is replaced by
/// an empty schema node: the shape is unknown, and asserting a wrong type
/// would be worse than admitting it.
pub const DECLARATION_MAX_SCHEMA_DEPTH: usize = 8;

/// The maximum number of properties one schema object may declare.
///
/// The small-list band of the repository (`plugin::bounds::MAX_PERMISSIONS`,
/// 32; `skill::SKILL_SEARCH_CATALOG_MAX`, 50), deliberately above it: a
/// tool's parameter list is exactly what the model must see.
pub const DECLARATION_MAX_SCHEMA_PROPERTIES: usize = 64;

/// The maximum number of nodes one declared input schema may hold.
///
/// The walk budget: a schema that declares more than this is cut short, so
/// the convergence cost stays bounded whatever the server sends.
pub const DECLARATION_MAX_SCHEMA_NODES: usize = 512;

/// The maximum number of items one schema array may hold (`enum`, `required`,
/// `oneOf`, ...).
pub const DECLARATION_MAX_SCHEMA_ARRAY_ITEMS: usize = 64;

/// The maximum characters of one remote result's joined text.
///
/// The tool-output band of the repository: `ProcessLimits::max_stdout_bytes`
/// and `repo::MAX_OUTPUT_BYTES` both sit at 64 KiB — the size of a tool's
/// raw output that enters the model.
pub const RESULT_MAX_TEXT_CHARS: usize = 64 * 1024;

/// The maximum number of content blocks one remote result may carry.
pub const RESULT_MAX_BLOCKS: usize = 8;

/// The maximum characters of one content block's text.
///
/// The same 64 KiB band as the joined text: one block is one text.
pub const RESULT_MAX_BLOCK_TEXT_CHARS: usize = 64 * 1024;

/// The maximum characters of one content block's inline data (image bytes).
///
/// The stderr band of the `external` family (16 KiB): inline data is bulk,
/// not prose.
pub const RESULT_MAX_BLOCK_DATA_CHARS: usize = 16 * 1024;

/// The marker appended where a text was cut.
pub const TRUNCATION_MARKER: &str = "...[truncated]";

/// Whether `character` is an invisible or directional control.
///
/// The base class is the one the governance crate already owns
/// ([`ToolDescAuditor::is_dangerous_char`]): the zero-width family, the
/// bidirectional controls, the soft hyphen, the arabic letter mark and the
/// C0/C1 control characters. This module does not define a second class —
/// it only adds the two families that class does not list and the carriers
/// need: the left/right marks (`U+200E`, `U+200F`) and the tag characters
/// (`U+E0001..=U+E007F`), which encode invisible text no font draws.
fn is_hidden_control(character: char) -> bool {
    ToolDescAuditor::is_dangerous_char(character)
        || matches!(
            character,
            '\u{200E}' | '\u{200F}' | '\u{E0001}'..='\u{E007F}'
        )
}

/// Strip the control and hidden characters from `text`.
///
/// The line structure survives (`\n`, `\t`); every other control character
/// (C0, DEL, C1) and every invisible or directional format character is
/// removed. Carriage return does not survive either: it is the rewrite
/// carrier of the invisible overwrite, and a line ending is `\n` here.
pub fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control() && !matches!(character, '\n' | '\t') {
            continue;
        }
        if is_hidden_control(character) {
            continue;
        }
        out.push(character);
    }
    out
}

/// Cut `text` to `max` characters, marking the cut.
///
/// The result is never longer than `max` characters: the marker is part of
/// the budget, so a caller can bound the length it hands to the model by
/// one number.
pub fn truncate_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let marker_len = TRUNCATION_MARKER.chars().count();
    if max <= marker_len {
        return text.chars().take(max).collect();
    }
    let mut out: String = text.chars().take(max - marker_len).collect();
    out.push_str(TRUNCATION_MARKER);
    out
}

/// The converged description of one external tool.
///
/// The server's own description reaches the model on every turn, so it is
/// cleaned (no invisible carriers) and bounded (no context flooding). A
/// description that fits is kept verbatim.
pub fn converge_description(description: Option<&str>) -> Option<String> {
    description.map(|text| truncate_text(&clean_text(text), DECLARATION_MAX_DESCRIPTION_CHARS))
}

/// The schema of a tool that declares no parameters.
///
/// A server whose schema is not an object schema degrades to this: the
/// model must still see an object schema (a tool takes named arguments),
/// but the server's own shape is not trusted to describe them.
pub fn empty_object_schema() -> ToolParameters {
    let mut schema = ToolParameters::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(Map::new()));
    schema
}

/// The converged input schema of one external tool.
///
/// The server's schema is kept as far as it can be: a normal object schema
/// keeps its properties, their types and their descriptions. What it does
/// not keep is the unbounded shape — depth, property count, node count, and
/// the dialect metadata keys (`$schema`, `$id`) that carry no call guidance
/// and can carry a server-chosen URL.
///
/// A schema that is not an object schema degrades to
/// [`empty_object_schema`]: the tool still appears, and the model still
/// knows it takes named arguments.
pub fn converge_input_schema(schema: Option<&Value>) -> ToolParameters {
    let Some(schema) = schema else {
        return ToolParameters::new();
    };
    let Some(object) = schema.as_object() else {
        return empty_object_schema();
    };
    if let Some(kind) = object.get("type").and_then(Value::as_str) {
        if kind != "object" {
            return empty_object_schema();
        }
    }
    let mut budget = SchemaBudget::new();
    let mut converged = converge_schema_object(object, 0, &mut budget);
    converged
        .entry("type")
        .or_insert_with(|| Value::String("object".to_string()));
    converged
}

/// The walk budget of one schema convergence.
struct SchemaBudget {
    nodes: usize,
}

impl SchemaBudget {
    /// A budget of the full node allowance.
    fn new() -> Self {
        Self {
            nodes: DECLARATION_MAX_SCHEMA_NODES,
        }
    }

    /// Take one node from the budget; `false` when the budget is spent.
    fn take(&mut self) -> bool {
        if self.nodes == 0 {
            return false;
        }
        self.nodes -= 1;
        true
    }
}

/// The property names one schema object marks required.
fn required_names(object: &Map<String, Value>) -> Vec<String> {
    let Some(Value::Array(items)) = object.get("required") else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_str)
        .take(DECLARATION_MAX_SCHEMA_ARRAY_ITEMS)
        .map(|name| truncate_text(&clean_text(name), DECLARATION_MAX_PROPERTY_NAME_CHARS))
        .collect()
}

/// Converge one schema object (a JSON Schema node).
///
/// The property map is converged first: its survivors decide which names
/// `required` may still point at.
fn converge_schema_object(
    object: &Map<String, Value>,
    depth: usize,
    budget: &mut SchemaBudget,
) -> Map<String, Value> {
    let mut out = Map::new();
    let required = required_names(object);

    let mut kept_properties: Option<Vec<String>> = None;
    if let Some(Value::Object(properties)) = object.get("properties") {
        if budget.take() {
            let (converged, kept) = converge_properties(properties, &required, depth + 1, budget);
            kept_properties = Some(kept);
            out.insert("properties".to_string(), Value::Object(converged));
        }
    }

    for (key, value) in object {
        if !budget.take() {
            break;
        }
        match key.as_str() {
            // Dialect and identity metadata: no call guidance, and the
            // identity key carries a server-chosen URL.
            "$schema" | "$id" => continue,
            // Handled above (`properties`) or below (`required`).
            "properties" | "required" => continue,
            _ => {}
        }
        let key = truncate_text(&clean_text(key), DECLARATION_MAX_PROPERTY_NAME_CHARS);
        if out.contains_key(&key) {
            continue;
        }
        let converged = converge_value(value, depth + 1, budget);
        out.insert(key, converged);
    }

    if let Some(Value::Array(items)) = object.get("required") {
        let names: Vec<Value> = items
            .iter()
            .filter_map(Value::as_str)
            .take(DECLARATION_MAX_SCHEMA_ARRAY_ITEMS)
            .map(|name| truncate_text(&clean_text(name), DECLARATION_MAX_PROPERTY_NAME_CHARS))
            .filter(|name| {
                kept_properties
                    .as_ref()
                    .is_none_or(|kept| kept.contains(name))
            })
            .map(Value::String)
            .collect();
        out.insert("required".to_string(), Value::Array(names));
    }

    out
}

/// Converge the `properties` map of one schema object.
///
/// The properties the schema marks required come first: dropping a required
/// property changes what the call means, dropping an optional one does not.
/// Returns the converged map and the names that survived.
fn converge_properties(
    properties: &Map<String, Value>,
    required: &[String],
    depth: usize,
    budget: &mut SchemaBudget,
) -> (Map<String, Value>, Vec<String>) {
    let mut out = Map::new();
    let mut kept: Vec<String> = Vec::new();
    let mut taken = 0usize;

    for pass in [true, false] {
        for (name, value) in properties {
            if taken >= DECLARATION_MAX_SCHEMA_PROPERTIES {
                break;
            }
            if required.iter().any(|candidate| candidate == name) != pass {
                continue;
            }
            if !budget.take() {
                break;
            }
            let key = truncate_text(&clean_text(name), DECLARATION_MAX_PROPERTY_NAME_CHARS);
            if out.contains_key(&key) {
                // Two names cut to the same prefix: the first one wins.
                continue;
            }
            let converged = converge_value(value, depth, budget);
            out.insert(key.clone(), converged);
            kept.push(key);
            taken += 1;
        }
    }

    (out, kept)
}

/// Converge one schema value (any keyword's value).
fn converge_value(value: &Value, depth: usize, budget: &mut SchemaBudget) -> Value {
    match value {
        Value::Object(object) => {
            if depth >= DECLARATION_MAX_SCHEMA_DEPTH {
                // Past the depth cap the shape is unknown: an empty schema
                // node keeps the call possible without asserting a type.
                return Value::Object(Map::new());
            }
            Value::Object(converge_schema_object(object, depth + 1, budget))
        }
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if out.len() >= DECLARATION_MAX_SCHEMA_ARRAY_ITEMS {
                    break;
                }
                if !budget.take() {
                    break;
                }
                out.push(converge_value(item, depth + 1, budget));
            }
            Value::Array(out)
        }
        Value::String(text) => Value::String(truncate_text(
            &clean_text(text),
            DECLARATION_MAX_SCHEMA_STRING_CHARS,
        )),
        other => other.clone(),
    }
}

/// The bounded view of one content block.
///
/// The block's shape is kept — the model still sees whether the server sent
/// text, an image, or a resource — and its payload is bounded.
pub fn bound_content_block(block: &ContentBlock) -> ContentBlock {
    match block {
        ContentBlock::Text { text, mime_type } => ContentBlock::Text {
            text: truncate_text(text, RESULT_MAX_BLOCK_TEXT_CHARS),
            mime_type: mime_type.clone(),
        },
        ContentBlock::Image { data, mime_type } => ContentBlock::Image {
            data: truncate_text(data, RESULT_MAX_BLOCK_DATA_CHARS),
            mime_type: mime_type.clone(),
        },
        ContentBlock::Resource {
            uri,
            text,
            mime_type,
        } => ContentBlock::Resource {
            uri: truncate_text(uri, RESULT_MAX_BLOCK_DATA_CHARS),
            text: text
                .as_ref()
                .map(|text| truncate_text(text, RESULT_MAX_BLOCK_TEXT_CHARS)),
            mime_type: mime_type.clone(),
        },
    }
}

/// The maximum characters one structured result may carry in total.
///
/// The same 64 KiB band as the joined text: the structured payload is the
/// server's other answer, and one answer may not be bigger than the other.
pub const RESULT_MAX_STRUCTURED_CHARS: usize = 64 * 1024;

/// The maximum nesting depth one structured result may have.
///
/// The model-facing depth band (the prompt assembly's default depth, 8).
pub const RESULT_MAX_STRUCTURED_DEPTH: usize = 8;

/// The maximum number of members one structured object may carry.
pub const RESULT_MAX_STRUCTURED_KEYS: usize = 64;

/// The maximum number of items one structured array may carry.
pub const RESULT_MAX_STRUCTURED_ITEMS: usize = 64;

/// The maximum number of nodes one structured result may hold.
///
/// The walk budget: a payload with more nodes than this is cut short, so the
/// convergence cost stays bounded whatever the server sends.
pub const RESULT_MAX_STRUCTURED_NODES: usize = 512;

/// The bounded view of one structured result payload.
///
/// The payload is server-authored like every other server-authored surface,
/// so it is bounded before it reaches the model: the depth, the member count,
/// the item count, the node count and the total character count are all
/// capped, and every string is cleaned of the invisible carriers.
///
/// A payload that fits is kept verbatim — only its invisible carriers go.
pub fn bound_structured_value(value: &Value) -> Value {
    let mut budget = StructuredBudget::new();
    bound_structured(value, 0, &mut budget)
}

/// The walk budget of one structured convergence.
struct StructuredBudget {
    nodes: usize,
    chars: usize,
}

impl StructuredBudget {
    /// A budget of the full node and character allowance.
    fn new() -> Self {
        Self {
            nodes: RESULT_MAX_STRUCTURED_NODES,
            chars: RESULT_MAX_STRUCTURED_CHARS,
        }
    }

    /// Take one node from the budget; `false` when the budget is spent.
    fn take_node(&mut self) -> bool {
        if self.nodes == 0 {
            return false;
        }
        self.nodes -= 1;
        true
    }

    /// Take up to `wanted` characters from the budget; the grant is what the
    /// caller may keep.
    fn take_chars(&mut self, wanted: usize) -> usize {
        let granted = wanted.min(self.chars);
        self.chars -= granted;
        granted
    }
}

/// Converge one structured value (any node of the payload).
fn bound_structured(value: &Value, depth: usize, budget: &mut StructuredBudget) -> Value {
    if !budget.take_node() {
        // The node budget is spent: the shape of the rest is unknown.
        return Value::Null;
    }
    match value {
        Value::Object(map) => {
            if depth >= RESULT_MAX_STRUCTURED_DEPTH {
                // Past the depth cap the shape is unknown: an empty object
                // keeps the payload an object without asserting a shape.
                return Value::Object(Map::new());
            }
            let mut out = Map::new();
            for (key, item) in map {
                if out.len() >= RESULT_MAX_STRUCTURED_KEYS {
                    break;
                }
                let key = truncate_text(&clean_text(key), DECLARATION_MAX_PROPERTY_NAME_CHARS);
                if out.contains_key(&key) {
                    continue;
                }
                let bounded = bound_structured(item, depth + 1, budget);
                out.insert(key, bounded);
            }
            Value::Object(out)
        }
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if out.len() >= RESULT_MAX_STRUCTURED_ITEMS {
                    break;
                }
                out.push(bound_structured(item, depth + 1, budget));
            }
            Value::Array(out)
        }
        Value::String(text) => {
            let clean = clean_text(text);
            let granted = budget.take_chars(clean.chars().count());
            Value::String(truncate_text(&clean, granted))
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clean_text_drops_control_and_hidden_characters() {
        let dirty = "a\u{200B}b\u{202E}c\u{0007}d\te\nf";
        let clean = clean_text(dirty);
        // The carriers go; the server's own order and its line structure
        // stay: "a" "b" "c" "d" TAB "e" LF "f".
        assert_eq!(clean, "abcd\te\nf");
        assert!(!clean.contains('\u{200B}'));
        assert!(!clean.contains('\u{202E}'));
        assert!(!clean.contains('\u{0007}'));
    }

    #[test]
    fn clean_text_drops_the_carriage_return_of_the_invisible_overwrite() {
        // The governance crate's character class counts the carriage return
        // as a control character, so the rewrite carrier cannot survive.
        let clean = clean_text("before\rafter");
        assert_eq!(clean, "beforeafter");
        assert!(!clean.contains('\r'));
    }

    #[test]
    fn truncate_text_marks_the_cut_inside_the_budget() {
        let long = "x".repeat(100);
        let cut = truncate_text(&long, 20);
        assert_eq!(cut.chars().count(), 20);
        assert!(cut.ends_with(TRUNCATION_MARKER));
        assert_eq!(truncate_text("short", 20), "short");
    }

    #[test]
    fn a_normal_schema_keeps_its_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "what to look for" },
                "limit": { "type": "integer" }
            },
            "required": ["query"]
        });
        let converged = converge_input_schema(Some(&schema));
        assert_eq!(converged["type"], json!("object"));
        assert_eq!(converged["properties"]["query"]["type"], json!("string"));
        assert_eq!(
            converged["properties"]["query"]["description"],
            json!("what to look for")
        );
        assert_eq!(converged["properties"]["limit"]["type"], json!("integer"));
        assert_eq!(converged["required"], json!(["query"]));
    }

    #[test]
    fn a_non_object_schema_degrades_to_the_empty_object_schema() {
        let converged = converge_input_schema(Some(&json!({ "type": "string" })));
        assert_eq!(converged["type"], json!("object"));
        assert_eq!(converged["properties"], json!({}));
    }

    #[test]
    fn a_deep_schema_converges_without_recursing_forever() {
        let mut schema =
            json!({ "type": "object", "properties": { "leaf": { "type": "string" } } });
        for _ in 0..64 {
            schema = json!({ "type": "object", "properties": { "next": schema } });
        }
        let converged = converge_input_schema(Some(&schema));
        assert_eq!(converged["type"], json!("object"));
    }
}
