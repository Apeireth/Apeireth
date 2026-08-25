//! Memory ONNX - ONNX 推理 stub (抄 v1 apeireth-memory/onnx.rs)
#[derive(Debug, Clone)] pub struct OnnxSession { pub model_path: String }
impl OnnxSession {
    pub fn new(model_path: impl Into<String>) -> Self { Self { model_path: model_path.into() } }
    /// 0 装 PASS stub: 真 ONNX 推理需 ort crate
    pub fn run(&self, _inputs: &std::collections::HashMap<String, Vec<f32>>) -> Result<std::collections::HashMap<String, Vec<f32>>, String> {
        if self.model_path.is_empty() { return Err("empty model path".into()); }
        // stub: 返回空 map
        Ok(std::collections::HashMap::new())
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_run() { let s = OnnxSession::new("m.onnx"); let mut inputs = std::collections::HashMap::new(); inputs.insert("x".to_string(), vec![1.0]); assert!(s.run(&inputs).is_ok()); } }