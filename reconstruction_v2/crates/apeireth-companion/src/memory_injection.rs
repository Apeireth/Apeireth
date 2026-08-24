//! MemoryInjection - 记忆注入 (从 v1.0 apeireth-companion/memory_injection.rs 66 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 build_memory_injection
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInjection { pub session_id: String, pub content: String, pub importance: u8 }

/// 0 装 PASS: 真 build
pub fn build_memory_injection(session_id: impl Into<String>, content: impl Into<String>) -> MemoryInjection {
    MemoryInjection { session_id: session_id.into(), content: content.into(), importance: 50 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_build() {
        let m = build_memory_injection("s1", "hello");
        assert_eq!(m.session_id, "s1");
        assert_eq!(m.importance, 50);
    }
    #[test] fn test_importance_default() {
        let m = build_memory_injection("s", "x");
        assert_eq!(m.importance, 50);
    }
}
