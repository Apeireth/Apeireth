//! **R227 — apeireth-bus topic pattern matching** (wildcard subscribe)
//!
//! **设计**: 借鉴 Kafka-style topic pattern:
//! - `*` — 匹配单段 (非 `.` 字符)
//! - `#` — 匹配多段 (在 pattern 末尾时匹配剩余所有)
//! - 其他字符按字面匹配
//!
//! **例子**:
//! - `agent.*`     匹配 `agent.bob`, `agent.alice`, 不匹配 `agent.team.lead`
//! - `agent.#`     匹配 `agent.bob`, `agent.team.lead`, `agent.x.y.z`
//! - `*`           匹配任何单段 topic
//! - `#`           匹配任何 topic
//! - `agent.bob`   仅匹配 `agent.bob` (精确)
//!
//! **不假装**: 0 引 regex dep — 手写段切分 + 模式匹配, O(n+m) 线性.
//!
//! **L0 Bus 集成**: subscribe_pattern 在 L0Bus 上加 Arc<PatternRegistry>,
//!   publish 时遍历注册 pattern, 命中则转发到对应 broadcast channel.

#![allow(missing_docs)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// **Topic pattern** — 编译期解析 wildcard 元字符到内部 enum, 加速匹配.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternSegment {
    /// 字面段 (e.g. "agent")
    Literal(String),
    /// `*` 单段通配
    SingleWildcard,
    /// `#` 多段通配 (仅允许出现在末尾)
    MultiWildcard,
}

/// **Parsed topic pattern**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPattern {
    segments: Vec<PatternSegment>,
    raw: String,
}

impl TopicPattern {
    /// 解析 pattern 字符串 (e.g. "agent.*.foo.#")
    pub fn parse(pattern: &str) -> Self {
        let mut segments = Vec::new();
        for part in pattern.split('.') {
            match part {
                "*" => segments.push(PatternSegment::SingleWildcard),
                "#" => segments.push(PatternSegment::MultiWildcard),
                other => segments.push(PatternSegment::Literal(other.to_string())),
            }
        }
        Self {
            segments,
            raw: pattern.to_string(),
        }
    }

    /// **匹配 topic 字符串**
    ///
    /// **算法**: 段对齐 + 通配语义
    /// - Literal 必须相等
    /// - SingleWildcard 匹配任何单段 (任意非空段)
    /// - MultiWildcard 匹配 0 段或多段 (含整个剩余)
    ///
    /// **返回**: true = 匹配, false = 不匹配
    pub fn matches(&self, topic: &str) -> bool {
        let topic_parts: Vec<&str> = topic.split('.').collect();

        // 找到最后一个非 MultiWildcard 段索引
        let last_literal_idx = self
            .segments
            .iter()
            .rposition(|s| !matches!(s, PatternSegment::MultiWildcard));

        match last_literal_idx {
            // 全是 MultiWildcard (e.g. "#")
            None => return true,
            Some(li) => {
                // 段数必须 >= 末尾 literal 段数
                if topic_parts.len() < li + 1 {
                    return false;
                }
                // 末尾 literal 段对齐匹配
                for (i, seg) in self.segments.iter().enumerate().take(li + 1) {
                    match seg {
                        PatternSegment::Literal(s) => {
                            if s != &topic_parts[i] {
                                return false;
                            }
                        }
                        PatternSegment::SingleWildcard => {
                            // 任意非空段 — split 不会产生空段除非 ..
                            if topic_parts[i].is_empty() {
                                return false;
                            }
                        }
                        PatternSegment::MultiWildcard => {
                            // 只可能出现在 li 之后 (即末尾), 此处不应到达
                            // (因为 last_literal_idx 已经过滤掉末尾的 MultiWildcard)
                            unreachable!("MultiWildcard before last_literal_idx");
                        }
                    }
                }
                // 如果 self.segments 末尾是 MultiWildcard, 剩余 topic_parts 必须 >= 1 段
                //   (Kafka 风格: # = 1 or more segments, 不匹配空剩余)
                if matches!(self.segments.last(), Some(PatternSegment::MultiWildcard)) {
                    return topic_parts.len() > li + 1;
                }
                // 否则段数必须完全相等
                topic_parts.len() == self.segments.len()
            }
        }
    }

    /// 原 pattern 字符串
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

/// **Pattern registry** — 维护 pattern → 订阅者映射, 提供按 topic 反查
///
/// **线程安全**: Mutex<HashSet<String>> 简化 (pattern 自身是不可变, 增删用 HashSet)
pub struct PatternRegistry {
    patterns: Mutex<HashSet<String>>,
}

impl PatternRegistry {
    pub fn new() -> Self {
        Self {
            patterns: Mutex::new(HashSet::new()),
        }
    }

    /// 注册 pattern (返回 true 表示新增, false 表示已存在)
    pub fn register(&self, pattern: &str) -> bool {
        self.patterns
            .lock()
            .expect("patterns lock poisoned")
            .insert(pattern.to_string())
    }

    /// 注销 pattern
    pub fn unregister(&self, pattern: &str) -> bool {
        self.patterns
            .lock()
            .expect("patterns lock poisoned")
            .remove(pattern)
    }

    /// 返回所有匹配 topic 的 pattern
    pub fn matching(&self, topic: &str) -> Vec<String> {
        self.patterns
            .lock()
            .expect("patterns lock poisoned")
            .iter()
            .filter(|p| TopicPattern::parse(p).matches(topic))
            .cloned()
            .collect()
    }

    /// 当前注册 pattern 数
    pub fn len(&self) -> usize {
        self.patterns.lock().expect("patterns lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PatternRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// **共享 PatternRegistry 工厂**
pub fn shared_registry() -> Arc<PatternRegistry> {
    Arc::new(PatternRegistry::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_no_wildcard() {
        let p = TopicPattern::parse("agent.bob");
        assert!(p.matches("agent.bob"));
        assert!(!p.matches("agent.alice"));
        assert!(!p.matches("agent"));
        assert!(!p.matches("agent.bob.x"));
    }

    #[test]
    fn single_wildcard_matches_one_segment() {
        let p = TopicPattern::parse("agent.*");
        assert!(p.matches("agent.bob"));
        assert!(p.matches("agent.alice"));
        assert!(!p.matches("agent.team.lead")); // 多段
        assert!(!p.matches("agent")); // 0 段
    }

    #[test]
    fn multi_wildcard_matches_many_segments() {
        let p = TopicPattern::parse("agent.#");
        assert!(p.matches("agent.bob"));
        assert!(p.matches("agent.team.lead"));
        assert!(p.matches("agent.x.y.z"));
        assert!(!p.matches("agent")); // # 必须 ≥ 1 段 (因为 agent 是 literal 段)
    }

    #[test]
    fn pure_single_wildcard() {
        let p = TopicPattern::parse("*");
        assert!(p.matches("foo"));
        assert!(p.matches("bar"));
        assert!(!p.matches("foo.bar")); // 多段
    }

    #[test]
    fn pure_multi_wildcard_matches_everything() {
        let p = TopicPattern::parse("#");
        assert!(p.matches("foo"));
        assert!(p.matches("foo.bar"));
        assert!(p.matches("a.b.c.d.e"));
    }

    #[test]
    fn mixed_literal_and_wildcard() {
        let p = TopicPattern::parse("agent.*.foo");
        assert!(p.matches("agent.bob.foo"));
        assert!(p.matches("agent.alice.foo"));
        assert!(!p.matches("agent.foo")); // 段数不够
        assert!(!p.matches("agent.bob.bar")); // 末尾不对
    }

    #[test]
    fn registry_register_and_query() {
        let r = PatternRegistry::new();
        assert!(r.is_empty());
        assert!(r.register("agent.*"));
        assert!(!r.register("agent.*")); // duplicate
        assert!(r.register("system.#"));
        assert_eq!(r.len(), 2);

        let m = r.matching("agent.bob");
        assert_eq!(m.len(), 1);
        assert!(m.contains(&"agent.*".to_string()));

        let m = r.matching("agent.team.lead");
        assert_eq!(m.len(), 0); // agent.* 不匹配多段

        let m = r.matching("system.cpu.high");
        assert_eq!(m.len(), 1);
        assert!(m.contains(&"system.#".to_string()));

        assert!(r.unregister("agent.*"));
        assert_eq!(r.len(), 1);
        let m = r.matching("agent.bob");
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn shared_registry_factory() {
        let r1 = shared_registry();
        let r2 = shared_registry();
        // 不同 Arc 实例, 各自独立 (factory 每次新建)
        r1.register("a.*");
        assert_eq!(r1.len(), 1);
        assert_eq!(r2.len(), 0);
    }
}
