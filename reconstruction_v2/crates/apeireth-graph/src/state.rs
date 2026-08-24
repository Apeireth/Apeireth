//! Graph shared state.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Graph shared state = `String -> serde_json::Value`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub(crate) inner: BTreeMap<String, Value>,
}

impl State {
    pub fn new() -> Self { Self::default() }

    pub fn with(key: impl Into<String>, value: impl Into<Value>) -> Self {
        let mut s = Self::new();
        s.insert(key, value.into());
        s
    }

    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.inner.insert(key.into(), value)
    }

    pub fn get(&self, key: &str) -> Option<&Value> { self.inner.get(key) }

    pub fn remove(&mut self, key: &str) -> Option<Value> { self.inner.remove(key) }

    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    pub fn keys(&self) -> Vec<&str> { self.inner.keys().map(|s| s.as_str()).collect() }
}

/// One node's execution output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeOutput {
    pub id: String,
    pub touched_keys: Vec<String>,
    pub message: Option<String>,
}

impl NodeOutput {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), touched_keys: Vec::new(), message: None }
    }
}

/// Full snapshot of one graph execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalState {
    pub state: State,
    pub outputs: BTreeMap<String, NodeOutput>,
    pub execution_order: Vec<String>,
}

impl FinalState {
    pub fn get(&self, key: &str) -> Option<&Value> { self.state.get(key) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn insert_get_remove() {
        let mut s = State::new();
        s.insert("a", json!(1));
        assert_eq!(s.get("a"), Some(&json!(1)));
        assert_eq!(s.remove("a"), Some(json!(1)));
        assert!(s.is_empty());
    }

    #[test]
    fn state_with() {
        let s = State::with("k", "v");
        assert_eq!(s.get("k"), Some(&json!("v")));
    }

    #[test]
    fn node_output_new() {
        let o = NodeOutput::new("n");
        assert_eq!(o.id, "n");
        assert!(o.touched_keys.is_empty());
        assert!(o.message.is_none());
    }
}
