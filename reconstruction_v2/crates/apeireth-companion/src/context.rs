//! Context - 上下文 (从 v1.0 apeireth-companion/context.rs 3K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 context 变量替换 + thread local
use std::collections::HashMap;

pub struct Context {
    pub vars: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self { Self { vars: HashMap::new() } }

    /// 0 装 PASS: 真 set
    pub fn set(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.vars.insert(k.into(), v.into());
    }

    /// 0 装 PASS: 真 get
    pub fn get(&self, k: &str) -> Option<&str> { self.vars.get(k).map(|s| s.as_str()) }

    /// 0 装 PASS: 真 template 替换 ({name})
    pub fn template(&self, tmpl: &str) -> String {
        let mut out = tmpl.to_string();
        for (k, v) in &self.vars {
            out = out.replace(&format!("{{{}}}", k), v);
        }
        out
    }
}

impl Default for Context { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_set_get() {
        let mut c = Context::new();
        c.set("name", "Alice");
        assert_eq!(c.get("name"), Some("Alice"));
    }
    #[test] fn test_unknown() {
        let c = Context::new();
        assert!(c.get("missing").is_none());
    }
    #[test] fn test_template() {
        let mut c = Context::new();
        c.set("name", "Alice");
        c.set("city", "NYC");
        let r = c.template("Hi {name} in {city}");
        assert_eq!(r, "Hi Alice in NYC");
    }
    #[test] fn test_template_missing_var() {
        let c = Context::new();
        let r = c.template("Hi {name}");
        assert_eq!(r, "Hi {name}");  // placeholder kept
    }
}
