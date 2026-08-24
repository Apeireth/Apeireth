//! Stage 0: Default dispatch.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DefaultDispatch;

impl DefaultDispatch {
    pub fn new() -> Self { Self }
    pub fn dispatch<T: Serialize>(&self, msg: &T) -> serde_json::Value {
        serde_json::to_value(msg).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::PipelineMessage;

    #[test]
    fn dispatch_works() {
        let d = DefaultDispatch::new();
        let m = PipelineMessage::new("chat", "t1", serde_json::json!({"k":1}));
        let v = d.dispatch(&m);
        assert_eq!(v["kind"], "chat");
    }
}
