//! Memory LLMAnalysis - LLM 分析 (抄 v1 apeireth-memory/llm_analysis.rs)
use std::collections::HashMap;
#[derive(Debug, Clone)] pub struct Analysis { pub text: String, pub summary: String, pub confidence: f32, pub model: String }
pub fn analyze_heuristic(text: &str) -> Analysis {
    let summary: String = text.chars().take(50).collect();
    Analysis { text: text.to_string(), summary, confidence: 0.5, model: "heuristic".into() }
}
pub fn analyze_with_cache(cache: &mut HashMap<String, Analysis>, text: &str) -> Analysis {
    if let Some(a) = cache.get(text) { return a.clone(); }
    let a = analyze_heuristic(text);
    cache.insert(text.to_string(), a.clone());
    a
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_heuristic() { let a = analyze_heuristic("hello world"); assert_eq!(a.summary, "hello world"); } #[test] fn test_cache() { let mut c = HashMap::new(); let a = analyze_with_cache(&mut c, "x"); assert!(c.contains_key("x")); let b = analyze_with_cache(&mut c, "x"); assert_eq!(a.summary, b.summary); } }