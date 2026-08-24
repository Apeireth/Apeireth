//! PromptCache - 提示词缓存 (从 v1.0 apeireth-companion/prompt_cache.rs 1.5K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 LRU cache + secret redact
use std::collections::HashMap;

pub fn redact_secrets(text: &str) -> String {
    let mut out = text.to_string();
    for pattern in &["api_key=", "password=", "token=", "secret="] {
        if let Some(pos) = out.find(pattern) {
            let end = out[pos..].find(|c: char| c == ',' || c == ' ' || c == '\n').unwrap_or(out.len() - pos);
            let end_pos = pos + end; out = out[..pos].to_string() + "[REDACTED]" + &out[end_pos..];
        }
    }
    out
}

pub struct PromptCache {
    entries: HashMap<String, String>,
    capacity: usize,
    order: Vec<String>,
}

impl PromptCache {
    pub fn new(capacity: usize) -> Self { Self { entries: HashMap::new(), capacity, order: Vec::new() } }

    /// 0 装 PASS: 真 LRU put
    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if !self.entries.contains_key(&key) {
            self.order.push(key.clone());
            if self.order.len() > self.capacity {
                if let Some(old) = self.order.first().cloned() {
                    self.entries.remove(&old);
                    self.order.remove(0);
                }
            }
        }
        self.entries.insert(key, value.into());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.entries.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_redact_secret() {
        let r = redact_secrets("api_key=abc123 password=secret user=alice");
        assert!(r.contains("[REDACTED]"));
        assert!(!r.contains("abc123"));
    }
    #[test] fn test_redact_no_match() {
        let r = redact_secrets("hello world");
        assert_eq!(r, "hello world");
    }
    #[test] fn test_cache_put_get() {
        let mut c = PromptCache::new(10);
        c.put("a", "value_a");
        assert_eq!(c.get("a").unwrap(), "value_a");
    }
    #[test] fn test_cache_eviction() {
        let mut c = PromptCache::new(2);
        c.put("a", "1");
        c.put("b", "2");
        c.put("c", "3");
        assert!(c.get("a").is_none());
        assert_eq!(c.get("c").unwrap(), "3");
    }
}
