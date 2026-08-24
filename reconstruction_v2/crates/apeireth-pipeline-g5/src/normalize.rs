//! Stage 1: Default normalize.

use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct DefaultNormalize;

impl DefaultNormalize {
    pub fn new() -> Self { Self }
    pub fn normalize(&self, v: Value) -> Value {
        match v {
            Value::String(s) => Value::String(s.trim().to_string()),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_strings() {
        let n = DefaultNormalize::new();
        let r = n.normalize(Value::String("  hi  ".into()));
        assert_eq!(r, Value::String("hi".into()));
    }

    #[test]
    fn non_string_unchanged() {
        let n = DefaultNormalize::new();
        let r = n.normalize(Value::Number(42.into()));
        assert_eq!(r, Value::Number(42.into()));
    }
}
