//! Free-form key/value annotation attached to canonical objects.
//!
//! Backed by a `BTreeMap` so iteration and serialization are deterministic. That
//! matters more than lookup speed here: metadata ends up in logs, manifests, and
//! test assertions, and a `HashMap` would make all three order-dependent.

use std::collections::btree_map::Iter;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Ordered string-to-string annotations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Metadata(BTreeMap<String, String>);

impl Metadata {
    /// An empty annotation set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a key, returning the previous value.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.0.insert(key.into(), value.into())
    }

    /// Builder-style insert.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.insert(key, value);
        self
    }

    /// Look up a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Whether a key is present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Remove a key, returning its value.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate entries in key order.
    pub fn iter(&self) -> Iter<'_, String, String> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Metadata {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<K, V> FromIterator<(K, V)> for Metadata
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iteration_and_serialization_are_key_ordered() {
        let md = Metadata::new()
            .with("zebra", "1")
            .with("alpha", "2")
            .with("middle", "3");

        let keys: Vec<&str> = md.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["alpha", "middle", "zebra"]);

        let json = serde_json::to_string(&md).unwrap();
        assert_eq!(json, r#"{"alpha":"2","middle":"3","zebra":"1"}"#);
    }

    #[test]
    fn insert_replaces_and_returns_the_previous_value() {
        let mut md = Metadata::new();
        assert_eq!(md.insert("k", "old"), None);
        assert_eq!(md.insert("k", "new"), Some("old".to_string()));
        assert_eq!(md.get("k"), Some("new"));
        assert_eq!(md.len(), 1);
    }

    #[test]
    fn round_trips_through_json() {
        let md = Metadata::new().with("a", "1").with("b", "2");
        let back: Metadata = serde_json::from_str(&serde_json::to_string(&md).unwrap()).unwrap();
        assert_eq!(md, back);
    }
}
