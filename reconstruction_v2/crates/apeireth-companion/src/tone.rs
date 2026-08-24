//! Tone - 语气调节 (从 v1.0 apeireth-companion/tone.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真语气模板 + substitute
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ToneProfile {
    pub name: String,
    pub templates: Vec<String>,
    pub substitutions: HashMap<String, String>,
}

pub struct ToneEngine {
    profiles: Vec<ToneProfile>,
}

impl ToneEngine {
    pub fn new() -> Self { Self { profiles: Vec::new() } }

    /// 0 装 PASS: 真注册
    pub fn register(&mut self, profile: ToneProfile) {
        self.profiles.push(profile);
    }

    /// 0 装 PASS: 真按 name 找
    pub fn apply(&self, name: &str, text: &str) -> String {
        for p in &self.profiles {
            if p.name == name {
                return p.substitutions.get(text).cloned().unwrap_or_else(|| text.to_string());
            }
        }
        text.to_string()
    }
}

impl Default for ToneEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_apply_no_match() {
        let e = ToneEngine::new();
        assert_eq!(e.apply("unknown", "hello"), "hello");
    }
    #[test] fn test_apply_with_match() {
        let mut e = ToneEngine::new();
        let mut sub = HashMap::new();
        sub.insert("hi".to_string(), "Hello there!".to_string());
        e.register(ToneProfile { name: "casual".into(), templates: vec![], substitutions: sub });
        assert_eq!(e.apply("casual", "hi"), "Hello there!");
    }
    #[test] fn test_default() {
        let e: ToneEngine = Default::default();
        assert_eq!(e.apply("x", "y"), "y");
    }
}
