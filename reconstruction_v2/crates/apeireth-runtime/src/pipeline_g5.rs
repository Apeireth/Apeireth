//! PipelineG5 - 5 阶段 pipeline (从 v1.0 apeireth-pipeline-g5 3K LOC 收敛)
//!
//! 0 装 PASS: 简化 5 阶段 chain (load -> transform -> validate -> enrich -> emit).

use super::pipeline::PipelineData;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage { Load, Transform, Validate, Enrich, Emit }

impl Stage {
    pub fn name(self) -> &'static str { match self { Self::Load => "load", Self::Transform => "transform", Self::Validate => "validate", Self::Enrich => "enrich", Self::Emit => "emit" } }
}

pub fn run(data: PipelineData) -> Value {
    let s1 = load(data);
    let s2 = transform(s1);
    let s3 = validate(&s2);
    let s4 = enrich(&s2);
    emit(&s3, &s4)
}

fn load(d: PipelineData) -> PipelineData { d }
fn transform(d: PipelineData) -> PipelineData {
    d.map(|v| match v.as_i64() { Some(n) => json!(n * 2), None => v.clone() })
}
fn validate(d: &PipelineData) -> Result<(), String> { if d.items.is_empty() { return Err("empty".into()); } Ok(()) }
fn enrich(d: &PipelineData) -> PipelineData { d.clone().map(|v| json!({"value": v, "ts": 0})) }
fn emit(v: &Result<(), String>, e: &PipelineData) -> Value { json!({"emitted": e.items.len(), "valid": v.is_ok()}) }

#[cfg(test)]
mod tests {
    use super::*; use serde_json::json;
    #[test] fn test_g5_run() {
        let d = PipelineData::from(vec![json!(1), json!(2), json!(3)]);
        let r = run(d);
        assert_eq!(r["emitted"], 3);
    }
}