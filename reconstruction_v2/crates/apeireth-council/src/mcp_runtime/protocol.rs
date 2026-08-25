//! JSON-RPC 2.0 protocol types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn construct() {
        let e = JsonRpcError::new(-32601, "not found");
        assert_eq!(e.code, -32601);
    }
}
