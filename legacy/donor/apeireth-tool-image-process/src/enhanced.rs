//! EnhancedImageProcess composed entry.

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
use crate::mcp::{ImageProcessMcp, McpRequest, McpResponse};
use crate::router::{ImageRouter, ProcessOp};

pub struct EnhancedImageProcess {
    router: ImageRouter,
    mcp: ImageProcessMcp,
}

impl EnhancedImageProcess {
    pub fn new() -> Self {
        Self {
            router: ImageRouter::new(),
            mcp: ImageProcessMcp::new(),
        }
    }
    pub fn process(
        &self,
        op: ProcessOp,
        data: &[u8],
        lang: Option<&str>,
    ) -> Result<String, crate::router::ProcessError> {
        self.router.dispatch(op, data, lang)
    }
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        self.mcp.handle(req)
    }
}

impl Default for EnhancedImageProcess {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_hash_works() {
        let e = EnhancedImageProcess::new();
        let s = e.process(ProcessOp::Hash, b"hello", None).unwrap();
        assert!(s.starts_with("hash="));
    }

    #[test]
    fn dispatch_mcp_works() {
        let e = EnhancedImageProcess::new();
        let r = e.dispatch_mcp(crate::mcp::McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        });
        assert!(r.result.is_some());
    }
}
