//! Onnx - ONNX 推理 (从 v1.0 apeireth-memory/onnx.rs 245 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 OnnxSession + run (stub 真实加载 model)

use std::collections::HashMap;

pub struct OnnxSession { pub model_path: String, pub loaded: bool }

impl OnnxSession {
    pub fn new(model_path: impl Into<String>) -> Self { Self { model_path: model_path.into(), loaded: false } }
    /// 0 装 PASS stub: 真加载 (需 onnxruntime)
    pub fn load(&mut self) -> Result<(), String> {
        if self.model_path.is_empty() { return Err("empty model path".into()); }
        self.loaded = true;
        Ok(())
    }
    /// 0 装 PASS stub: 真 run (mock 输出)
    pub fn run(&self, inputs: &HashMap<String, Vec<f32>>) -> Result<HashMap<String, Vec<f32>>, String> {
        if !self.loaded { return Err("not loaded".into()); }
        let mut out = HashMap::new();
        for (k, v) in inputs { out.insert(format!("{}_out", k), v.clone()); }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_load() {
        let mut s = OnnxSession::new("model.onnx");
        assert!(s.load().is_ok());
    }
    #[test] fn test_run() {
        let mut s = OnnxSession::new("m.onnx");
        s.load().unwrap();
        let mut inputs = HashMap::new();
        inputs.insert("x".to_string(), vec![1.0, 2.0]);
        let r = s.run(&inputs).unwrap();
        assert!(r.contains_key("x_out"));
    }
    #[test] fn test_unload_run() {
        let s = OnnxSession::new("m.onnx");
        let mut inputs = HashMap::new();
        inputs.insert("x".to_string(), vec![1.0]);
        assert!(s.run(&inputs).is_err());
    }
    #[test] fn test_empty_model() {
        let mut s = OnnxSession::new("");
        assert!(s.load().is_err());
    }
}
