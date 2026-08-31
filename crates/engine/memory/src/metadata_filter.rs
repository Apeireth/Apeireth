//! JSON property-bag filters for retrieval (salvage of canonical
//! `apeireth-graph-primitive::query` predicates).
//!
//! Engine `NodeQuery` / `EdgeQuery` AND-ed kind + JSON equality over a property
//! bag. Canonical [`MemoryFilter`] only has `as_of` / tombstone / limit, and
//! [`VectorIndex`] stores no metadata. This module is a pure matcher the
//! persistent vector index (and any caller with a JSON bag) can apply **after**
//! scoring — it is not a second search engine.

use serde_json::Value;

/// Equality predicate: `properties[key] == value` (JSON deep equality).
/// A missing key does not match.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyPredicate {
    pub key: String,
    pub value: Value,
}

impl PropertyPredicate {
    pub fn new(key: impl Into<String>, value: Value) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }

    pub fn string(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(key, Value::String(value.into()))
    }

    /// Evaluate against a property bag. Non-object bags never match.
    pub fn matches(&self, properties: &Value) -> bool {
        properties
            .get(&self.key)
            .map(|v| v == &self.value)
            .unwrap_or(false)
    }
}

/// AND-combination of an optional `kind` (read from `metadata["kind"]`) and
/// zero or more property predicates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetadataFilter {
    pub kind: Option<String>,
    pub properties: Vec<PropertyPredicate>,
}

impl MetadataFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kind_eq(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.push(PropertyPredicate::new(key, value));
        self
    }

    pub fn property_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push(PropertyPredicate::string(key, value));
        self
    }

    /// `true` when this filter imposes no constraints (matches everything,
    /// including a missing metadata bag).
    pub fn is_empty(&self) -> bool {
        self.kind.is_none() && self.properties.is_empty()
    }

    /// Match an optional metadata bag.
    ///
    /// Empty filters match even `None`. A non-empty filter against `None`
    /// fails. `kind` is compared to the string value of `metadata["kind"]`.
    pub fn matches(&self, metadata: Option<&Value>) -> bool {
        if self.is_empty() {
            return true;
        }
        let Some(meta) = metadata else {
            return false;
        };
        if let Some(kind) = &self.kind {
            match meta.get("kind").and_then(Value::as_str) {
                Some(actual) if actual == kind => {}
                _ => return false,
            }
        }
        self.properties.iter().all(|p| p.matches(meta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn property_equality_and_missing_key() {
        let pred = PropertyPredicate::string("role", "assistant");
        assert!(pred.matches(&json!({"role": "assistant", "level": 5})));
        assert!(!pred.matches(&json!({"role": "user"})));
        assert!(!pred.matches(&json!({"other": "y"})));
        assert!(!pred.matches(&json!("not-an-object")));
    }

    #[test]
    fn empty_filter_matches_none_and_some() {
        let filter = MetadataFilter::new();
        assert!(filter.matches(None));
        assert!(filter.matches(Some(&json!({"tag": "x"}))));
    }

    #[test]
    fn kind_and_property_and() {
        let filter = MetadataFilter::new()
            .kind_eq("agent")
            .property_string("role", "assistant")
            .property("level", json!(5));
        let alice = json!({"kind": "agent", "role": "assistant", "level": 5});
        let bob = json!({"kind": "agent", "role": "user", "level": 3});
        let tool = json!({"kind": "tool", "role": "assistant", "level": 5});
        assert!(filter.matches(Some(&alice)));
        assert!(!filter.matches(Some(&bob)));
        assert!(!filter.matches(Some(&tool)));
        assert!(!filter.matches(None));
    }

    #[test]
    fn numeric_and_bool_json_equality() {
        let filter = MetadataFilter::new().property("ok", json!(true));
        assert!(filter.matches(Some(&json!({"ok": true}))));
        assert!(!filter.matches(Some(&json!({"ok": false}))));
        assert!(!filter.matches(Some(&json!({"ok": "true"}))));
    }
}
