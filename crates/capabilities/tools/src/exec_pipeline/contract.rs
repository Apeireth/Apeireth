//! The output normalization contract: one tool result, one frozen shape.
//!
//! Tool outputs drift: a tool that returns a number today and an object
//! tomorrow poisons everything downstream of it. This module freezes whatever
//! came back into [`ToolOutcome { ok, output_text, meta }`] — the only shape the
//! execution chain hands onward — and validates the structured side against a
//! declared [`OutputSchema`]. A shape that violates its schema is a
//! **normalization error** reported in the open, never a silently coerced or
//! silently dropped value.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use apeireth_protocol::canonical::ToolResult;

/// The frozen normalized shape of one tool result.
///
/// This is the execution chain's output contract. Whatever the tool produced,
/// the pipeline emits exactly `ok`, `output_text`, and `meta`:
///
/// * `ok` — whether the call succeeded;
/// * `output_text` — the canonical rendered text of the result (what a model
///   reads, produced by the same rendering the wire path uses);
/// * `meta` — the structured sidecar: the raw structured value on success, or
///   the failure description on error.
///
/// This is the execution-side contract and is deliberately distinct from the
/// wire-level call-outcome enum in the protocol crate: that one classifies how
/// a call ended for transport purposes, this one freezes what the execution
/// chain may pass on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutcome {
    /// Whether the call succeeded.
    pub ok: bool,
    /// The canonical rendered text of the result.
    pub output_text: String,
    /// Structured sidecar; always a JSON object.
    pub meta: Value,
}

impl ToolOutcome {
    /// Freeze one result into the contract, projecting without coercion.
    ///
    /// The projection is total and lossless in the only direction that
    /// matters: `output_text` is exactly what the wire path would render, and
    /// `meta` carries the structured value or the failure description.
    pub fn freeze(result: &ToolResult) -> Self {
        let meta = match &result.outcome {
            apeireth_protocol::canonical::ToolOutcome::Ok { value } => {
                serde_json::json!({ "structured": value })
            }
            apeireth_protocol::canonical::ToolOutcome::Error { message, retryable } => {
                serde_json::json!({
                    "error": { "message": message, "retryable": retryable }
                })
            }
        };
        Self {
            ok: result.is_ok(),
            output_text: result.render(),
            meta,
        }
    }

    /// Freeze one result after validating its structured output.
    ///
    /// When `schema` is declared and the result succeeded, the structured
    /// output must satisfy it; a violation is a [`NormalizationError`]
    /// surfaced here, not swallowed. Failed results carry no structured
    /// output to validate: their failure shape is already closed.
    pub fn normalize(
        result: &ToolResult,
        schema: Option<&OutputSchema>,
    ) -> Result<Self, NormalizationError> {
        if let (Some(schema), apeireth_protocol::canonical::ToolOutcome::Ok { value }) =
            (schema, &result.outcome)
        {
            schema.validate(value)?;
        }
        Ok(Self::freeze(result))
    }

    /// The structured value frozen into `meta`, when the call succeeded.
    pub fn structured(&self) -> Option<&Value> {
        self.meta.get("structured")
    }
}

/// The JSON kind a [`SchemaField`] demands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaKind {
    /// A JSON string.
    String,
    /// A JSON number.
    Number,
    /// A JSON boolean.
    Boolean,
    /// A JSON object.
    Object,
    /// A JSON array.
    Array,
}

impl SchemaKind {
    /// Stable label used in errors and serialized schemas.
    pub const fn label(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Object => "object",
            Self::Array => "array",
        }
    }

    /// Whether `value` has this kind.
    pub fn matches(self, value: &Value) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Number => value.is_number(),
            Self::Boolean => value.is_boolean(),
            Self::Object => value.is_object(),
            Self::Array => value.is_array(),
        }
    }
}

/// One required field of the declared output shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaField {
    /// Required field name.
    pub name: String,
    /// Required JSON kind.
    pub kind: SchemaKind,
}

impl SchemaField {
    /// A required field of the given name and kind.
    pub fn new(name: impl Into<String>, kind: SchemaKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

/// The declared shape of a tool's structured output.
///
/// Every declared field is required with its exact kind; fields not declared
/// are left alone. The check exists to catch drift in the shape consumers
/// depend on — it does not attempt to be a full schema language.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSchema {
    fields: Vec<SchemaField>,
}

impl OutputSchema {
    /// A schema with no required fields: every structured value conforms.
    pub fn new() -> Self {
        Self::default()
    }

    /// Require one more field.
    #[must_use]
    pub fn require(mut self, name: impl Into<String>, kind: SchemaKind) -> Self {
        self.fields.push(SchemaField::new(name, kind));
        self
    }

    /// The declared required fields, in declaration order.
    pub fn fields(&self) -> &[SchemaField] {
        &self.fields
    }

    /// Whether the schema demands nothing.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Validate one structured output value against the declared shape.
    pub fn validate(&self, value: &Value) -> Result<(), NormalizationError> {
        let Some(object) = value.as_object() else {
            return Err(NormalizationError::NotStructured {
                found: kind_label(value).to_string(),
            });
        };
        for field in &self.fields {
            let Some(found) = object.get(&field.name) else {
                return Err(NormalizationError::SchemaMismatch {
                    field: field.name.clone(),
                    expected: field.kind.label().to_string(),
                    found: "missing".to_string(),
                });
            };
            if !field.kind.matches(found) {
                return Err(NormalizationError::SchemaMismatch {
                    field: field.name.clone(),
                    expected: field.kind.label().to_string(),
                    found: kind_label(found).to_string(),
                });
            }
        }
        Ok(())
    }
}

/// A normalization failure. It is reported, never swallowed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NormalizationError {
    /// The structured output is not a JSON object at all.
    #[error("structured output must be a JSON object, found {found}")]
    NotStructured {
        /// The JSON kind that was found instead.
        found: String,
    },
    /// A declared field is missing or carries the wrong JSON kind.
    #[error("structured output field {field} must be {expected}, found {found}")]
    SchemaMismatch {
        /// The declared field that failed.
        field: String,
        /// The declared kind.
        expected: String,
        /// The kind found instead (`missing` when the field is absent).
        found: String,
    },
}

impl NormalizationError {
    /// Stable code of the normalization failure family.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotStructured { .. } => "output_contract.not_structured",
            Self::SchemaMismatch { .. } => "output_contract.schema_mismatch",
        }
    }
}

/// JSON kind label of a value, for error messages.
fn kind_label(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeze_projects_without_coercion() {
        let ok = ToolResult::ok("call_1", serde_json::json!({ "sum": 2 }));
        let outcome = ToolOutcome::freeze(&ok);
        assert!(outcome.ok);
        assert_eq!(outcome.output_text, r#"{"sum":2}"#);
        assert_eq!(outcome.structured(), Some(&serde_json::json!({ "sum": 2 })));

        let err = ToolResult::permanent_error("call_2", "boom");
        let outcome = ToolOutcome::freeze(&err);
        assert!(!outcome.ok);
        assert_eq!(outcome.output_text, "error: boom");
        assert_eq!(
            outcome.meta.get("error").and_then(|e| e.get("retryable")),
            Some(&serde_json::Value::Bool(false))
        );
    }

    #[test]
    fn schema_mismatch_is_an_error_not_a_silent_pass() {
        let schema = OutputSchema::new()
            .require("summary", SchemaKind::String)
            .require("count", SchemaKind::Number);
        let good = ToolResult::ok("c", serde_json::json!({ "summary": "x", "count": 1 }));
        assert!(ToolOutcome::normalize(&good, Some(&schema)).is_ok());

        let drifted = ToolResult::ok("c", serde_json::json!({ "summary": "x" }));
        let error = ToolOutcome::normalize(&drifted, Some(&schema)).unwrap_err();
        assert_eq!(error.code(), "output_contract.schema_mismatch");
        assert!(error.to_string().contains("count"), "{error}");

        let wrong_kind = ToolResult::ok("c", serde_json::json!({ "summary": 7, "count": 1 }));
        let error = ToolOutcome::normalize(&wrong_kind, Some(&schema)).unwrap_err();
        assert!(matches!(error, NormalizationError::SchemaMismatch { .. }));

        let not_object = ToolResult::ok("c", serde_json::json!([1, 2]));
        let error = ToolOutcome::normalize(&not_object, Some(&schema)).unwrap_err();
        assert_eq!(error.code(), "output_contract.not_structured");
    }
}
