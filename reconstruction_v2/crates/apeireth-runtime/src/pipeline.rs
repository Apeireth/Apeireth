//! Pipeline - pipeline framework (从 v1.0 apeireth-pipeline 7K LOC 收敛)
//!
//! 0 装 PASS: 简化 stage chain (data -> map -> filter -> collect), 完整 v1.0 era (backpressure, retry) 不做.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineData {
    pub items: Vec<serde_json::Value>,
}

impl PipelineData {
    pub fn new() -> Self { Self { items: Vec::new() } }
    pub fn from(items: Vec<serde_json::Value>) -> Self { Self { items } }
    pub fn map<F: Fn(&serde_json::Value) -> serde_json::Value>(mut self, f: F) -> Self {
        self.items = self.items.iter().map(f).collect();
        self
    }
    pub fn filter<F: Fn(&serde_json::Value) -> bool>(mut self, f: F) -> Self {
        self.items.retain(|i| f(i));
        self
    }
    pub fn count(&self) -> usize { self.items.len() }
}

pub struct Pipeline { pub name: String, pub stages: Vec<String> }

impl Pipeline {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), stages: Vec::new() } }
    pub fn add_stage(&mut self, name: impl Into<String>) { self.stages.push(name.into()); }
    pub fn run(&self, data: PipelineData) -> PipelineData {
        data.filter(|v| v.is_object() || v.is_array() || v.is_number() || v.is_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*; use serde_json::json;
    #[test] fn test_pipeline_data() {
        let d = PipelineData::from(vec![json!(1), json!(2), json!(3)]);
        let d = d.map(|v| json!(v.as_i64().unwrap() * 2));
        let d = d.filter(|v| v.as_i64().unwrap() > 2);
        assert_eq!(d.count(), 2);
    }
    #[test] fn test_pipeline_run() {
        let mut p = Pipeline::new("t");
        p.add_stage("s1");
        let d = PipelineData::from(vec![json!(1), json!("x")]);
        assert_eq!(p.run(d).count(), 2);
    }
}