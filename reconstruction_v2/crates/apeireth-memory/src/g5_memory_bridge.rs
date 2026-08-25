//! g5_memory_bridge - memory insert/retrieve 5 steps (v2 apeireth-pipeline-g5 surface).
//!
//! v1 used ``impl Stage<I,O> for X`` + ``Pipeline::new(config).with_stage(X).with_stage(Y) ... run(msg)``.
//! v2 g5 has Stage as plain data struct + StageEntry(handler: Box<dyn Fn>) + Pipeline = config wrapper.
//! Bridge preserves v1 names + 5-stage pipeline semantics, but stages are now standalone structs
//! each with ``kind()`` + ``name()`` + ``process(msg)`` returning Result<PipelineMessage, PipelineError>.
//! MemoryPipeline wraps 5 stages and runs them in order.

#![allow(missing_docs)]

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use apeireth_pipeline_g5::{PipelineMessage, StageKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOp {
    Insert,
    Retrieve,
    Update,
    Delete,
}
impl MemoryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryOp::Insert => "insert",
            MemoryOp::Retrieve => "retrieve",
            MemoryOp::Update => "update",
            MemoryOp::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TtlPolicy {
    pub max_age_secs: u64,
}
impl Default for TtlPolicy {
    fn default() -> Self { Self { max_age_secs: 86400 * 30 } }
}

#[derive(Debug)]
pub struct FingerprintCache {
    entries: Mutex<HashMap<String, Instant>>,
    pub window: Duration,
    pub max_size: usize,
}
impl FingerprintCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            window: Duration::from_secs(60),
            max_size: 10_000,
        }
    }
    pub fn check(&self, fp: &str) -> bool {
        let now = Instant::now();
        let mut g = self.entries.lock().expect("fingerprint lock");
        g.retain(|_, t| now.duration_since(*t) < self.window);
        if g.contains_key(fp) { return true; }
        g.insert(fp.to_string(), now);
        if g.len() > self.max_size {
            if let Some(first) = g.keys().next().cloned() {
                g.remove(&first);
            }
        }
        false
    }
}
impl Default for FingerprintCache { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct KeyThrottle {
    counts: Mutex<HashMap<String, u32>>,
    pub max_per_key: u32,
}
impl KeyThrottle {
    pub fn new(max_per_key: u32) -> Self {
        Self { counts: Mutex::new(HashMap::new()), max_per_key }
    }
    pub fn check_and_inc(&self, key: &str) -> bool {
        let mut g = self.counts.lock().expect("throttle lock");
        let count = g.entry(key.to_string()).or_insert(0);
        *count += 1;
        *count <= self.max_per_key
    }
}

// 5 stages

#[derive(Debug, Clone, Default)]
pub struct MemoryDispatchStage;
impl MemoryDispatchStage {
    pub fn kind(&self) -> StageKind { StageKind::Dispatch }
    pub fn name(&self) -> &'static str { "memory-dispatch" }
    pub fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        if m.kind.is_empty() {
            m.kind = "episode-insert".to_string();
        }
        Ok(m)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryNormalizeStage {
    pub max_key_len: usize,
    pub max_payload_len: usize,
}
impl MemoryNormalizeStage {
    pub fn new() -> Self { Self { max_key_len: 64, max_payload_len: 256 * 1024 } }
    pub fn kind(&self) -> StageKind { StageKind::Normalize }
    pub fn name(&self) -> &'static str { "memory-normalize" }
    pub fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        if m.kind.len() > self.max_key_len {
            m.kind.truncate(self.max_key_len);
        }
        Ok(m)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPolicyStage {
    pub policy: TtlPolicy,
}
impl MemoryPolicyStage {
    pub fn new(policy: TtlPolicy) -> Self { Self { policy } }
    pub fn kind(&self) -> StageKind { StageKind::Policy }
    pub fn name(&self) -> &'static str { "memory-policy" }
    pub fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        let ttl_secs = self.policy.max_age_secs;
        if !m.trace_id.contains(":ttl=") {
            m.trace_id = format!("{}:ttl={}", m.trace_id, ttl_secs);
        }
        Ok(m)
    }
}

pub struct MemoryReliabilityStage {
    pub cache: std::sync::Arc<FingerprintCache>,
}
impl MemoryReliabilityStage {
    pub fn new(cache: std::sync::Arc<FingerprintCache>) -> Self { Self { cache } }
    pub fn kind(&self) -> StageKind { StageKind::Reliability }
    pub fn name(&self) -> &'static str { "memory-reliability" }
    pub fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let m = input;
        let mut h = DefaultHasher::new();
        m.kind.hash(&mut h);
        if let Ok(s) = serde_json::to_string(&m.payload) {
            s.hash(&mut h);
        }
        let fp = format!("{}|{:x}", m.kind, h.finish());
        if self.cache.check(&fp) {
            return Err(apeireth_pipeline_g5::PipelineError::ReliabilityExhausted(1));
        }
        Ok(m)
    }
}

pub struct MemoryThrottleStage {
    pub throttle: std::sync::Arc<KeyThrottle>,
}
impl MemoryThrottleStage {
    pub fn new(throttle: std::sync::Arc<KeyThrottle>) -> Self { Self { throttle } }
    pub fn kind(&self) -> StageKind { StageKind::Throttle }
    pub fn name(&self) -> &'static str { "memory-throttle" }
    pub fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let m = input;
        let key = m.kind.clone();
        if !self.throttle.check_and_inc(&key) {
            return Err(apeireth_pipeline_g5::PipelineError::ThrottleRejection(format!("key {} over rate limit", key)));
        }
        Ok(m)
    }
}

// Memory pipeline (v2 surface)

pub struct MemoryPipeline {
    pub dispatch: MemoryDispatchStage,
    pub normalize: MemoryNormalizeStage,
    pub policy: MemoryPolicyStage,
    pub reliability: MemoryReliabilityStage,
    pub throttle: MemoryThrottleStage,
}

impl MemoryPipeline {
    pub fn run(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let m = self.dispatch.process(input)?;
        let m = self.normalize.process(m)?;
        let m = self.policy.process(m)?;
        let m = self.reliability.process(m)?;
        self.throttle.process(m)
    }

    pub fn stage_kinds(&self) -> Vec<StageKind> {
        vec![
            self.dispatch.kind(),
            self.normalize.kind(),
            self.policy.kind(),
            self.reliability.kind(),
            self.throttle.kind(),
        ]
    }
}

pub struct MemoryPipelineBuilder {
    name: String,
    ttl: TtlPolicy,
    max_per_key: u32,
    fingerprint_cache: std::sync::Arc<FingerprintCache>,
    key_throttle: std::sync::Arc<KeyThrottle>,
}

impl MemoryPipelineBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ttl: TtlPolicy::default(),
            max_per_key: 100,
            fingerprint_cache: std::sync::Arc::new(FingerprintCache::new()),
            key_throttle: std::sync::Arc::new(KeyThrottle::new(100)),
        }
    }
    pub fn with_ttl(mut self, t: TtlPolicy) -> Self { self.ttl = t; self }
    pub fn with_max_per_key(mut self, n: u32) -> Self {
        self.max_per_key = n;
        self.key_throttle = std::sync::Arc::new(KeyThrottle::new(n));
        self
    }
    pub fn with_fingerprint_cache(mut self, c: std::sync::Arc<FingerprintCache>) -> Self {
        self.fingerprint_cache = c;
        self
    }
    pub fn build(self) -> MemoryPipeline {
        let _ = self.name;
        MemoryPipeline {
            dispatch: MemoryDispatchStage,
            normalize: MemoryNormalizeStage::new(),
            policy: MemoryPolicyStage::new(self.ttl),
            reliability: MemoryReliabilityStage::new(self.fingerprint_cache),
            throttle: MemoryThrottleStage::new(self.key_throttle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_op_as_str() {
        assert_eq!(MemoryOp::Insert.as_str(), "insert");
        assert_eq!(MemoryOp::Retrieve.as_str(), "retrieve");
    }
    #[test]
    fn dispatch_defaults_kind() {
        let s = MemoryDispatchStage;
        let m = PipelineMessage::new("", "p", serde_json::json!({}));
        let o = s.process(m).unwrap();
        assert_eq!(o.kind, "episode-insert");
    }
    #[test]
    fn dispatch_preserves_kind() {
        let s = MemoryDispatchStage;
        let m = PipelineMessage::new("note-insert", "x", serde_json::json!({}));
        let o = s.process(m).unwrap();
        assert_eq!(o.kind, "note-insert");
    }
    #[test]
    fn normalize_truncates_long_kind() {
        let s = MemoryNormalizeStage::new();
        let m = PipelineMessage::new("k".repeat(1000), "p", serde_json::json!({}));
        let o = s.process(m).unwrap();
        assert!(o.kind.len() <= 64);
    }
    #[test]
    fn policy_adds_ttl() {
        let s = MemoryPolicyStage::new(TtlPolicy::default());
        let m = PipelineMessage::new("k", "p", serde_json::json!({}));
        let o = s.process(m).unwrap();
        assert!(o.trace_id.contains(":ttl="));
    }
    #[test]
    fn reliability_dedups() {
        let s = MemoryReliabilityStage::new(std::sync::Arc::new(FingerprintCache::new()));
        let m = PipelineMessage::new("unique-mem-1", "p", serde_json::json!({"x": 1}));
        let _ = s.process(m.clone()).unwrap();
        let r = s.process(m);
        assert!(r.is_err());
    }
    #[test]
    fn throttle_rate_limit_per_key() {
        let s = MemoryThrottleStage::new(std::sync::Arc::new(KeyThrottle::new(2)));
        assert!(s.process(PipelineMessage::new("kind-a", "1", serde_json::json!({}))).is_ok());
        assert!(s.process(PipelineMessage::new("kind-a", "2", serde_json::json!({}))).is_ok());
        assert!(s.process(PipelineMessage::new("kind-a", "3", serde_json::json!({}))).is_err());
        assert!(s.process(PipelineMessage::new("kind-b", "1", serde_json::json!({}))).is_ok());
    }
    #[test]
    fn full_pipeline_runs() {
        let p = MemoryPipelineBuilder::new("mem").build();
        let r = p.run(PipelineMessage::new("episode-insert", "trace-1", serde_json::json!("hello world")));
        assert!(r.is_ok(), "should pass: {:?}", r.err());
    }
    #[test]
    fn pipeline_stage_order() {
        let p = MemoryPipelineBuilder::new("mem-order").build();
        let k = p.stage_kinds();
        assert_eq!(k.len(), 5);
        assert_eq!(k[0], StageKind::Dispatch);
        assert_eq!(k[4], StageKind::Throttle);
    }
}
