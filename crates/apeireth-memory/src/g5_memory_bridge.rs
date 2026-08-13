
//! g5_memory_bridge - memory insert/retrieve 5 步映射到 apeireth-pipeline-g5 通用 5 阶段 substrate
//!
//! R161: apeireth-memory 接 g5 作为第 5 个生产调用方 (tool-runtime R132.4 + chat R157 + council R159 + runtime R160).
//! 5 阶段: Dispatch (memory kind: episode/note/session) -> Normalize (key sanitization) -> Policy (TTL cap) -> Reliability (fingerprint cache) -> Throttle (rate limit per key).
#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use apeireth_pipeline_g5::{Pipeline, PipelineConfig, PipelineMessage, Stage, StageKind};

#[derive(Debug, Clone, Copy)] pub struct MemoryPipeline;

/// Memory op (insert/retrieve).
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum MemoryOp { Insert, Retrieve, Update, Delete }
impl MemoryOp { pub fn as_str(&self) -> &'static str { match self { MemoryOp::Insert => "insert", MemoryOp::Retrieve => "retrieve", MemoryOp::Update => "update", MemoryOp::Delete => "delete" } } }

/// TTL policy (max age in seconds for Retrieve/Insert operations).
#[derive(Debug, Clone)] pub struct TtlPolicy { pub max_age_secs: u64 }
impl Default for TtlPolicy { fn default() -> Self { Self { max_age_secs: 86400 * 30 } } }  // 30 days

/// Fingerprint cache (deduplicate same-kind same-payload within window).
#[derive(Debug)] pub struct FingerprintCache { entries: Mutex<HashMap<String, Instant>>, pub window: Duration, pub max_size: usize }
impl FingerprintCache {
    pub fn new() -> Self { Self { entries: Mutex::new(HashMap::new()), window: Duration::from_secs(60), max_size: 4096 } }
    pub fn check(&self, fp: &str) -> bool { let mut m = self.entries.lock().unwrap(); if m.len() >= self.max_size { let cutoff = Instant::now() - self.window; m.retain(|_, t| *t > cutoff); if m.len() >= self.max_size { return false; } } if let Some(t) = m.get(fp) { if t.elapsed() < self.window { return true; } } m.insert(fp.to_string(), Instant::now()); false }
}
impl Default for FingerprintCache { fn default() -> Self { Self::new() } }

/// Per-key throttle.
#[derive(Debug)] pub struct KeyThrottle { keys: Mutex<HashMap<String, u32>>, pub max_per_key: u32 }
impl KeyThrottle { pub fn new(max_per_key: u32) -> Self { Self { keys: Mutex::new(HashMap::new()), max_per_key } } pub fn check_and_inc(&self, key: &str) -> bool { let mut m = self.keys.lock().unwrap(); let count = m.entry(key.to_string()).or_insert(0); *count += 1; *count <= self.max_per_key } }

/// Dispatch stage: detect memory op from kind prefix.
#[derive(Debug, Clone, Default)] pub struct MemoryDispatchStage;
impl Stage<PipelineMessage, PipelineMessage> for MemoryDispatchStage {
    fn kind(&self) -> StageKind { StageKind::Dispatch }
    fn name(&self) -> &'static str { "memory-dispatch" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; if m.kind.is_empty() { m.kind = "episode-insert".to_string(); } Ok(m) }
}

/// Normalize stage: key sanitization + max payload size.
#[derive(Debug, Clone)] pub struct MemoryNormalizeStage { pub max_key_len: usize, pub max_payload_len: usize }
impl MemoryNormalizeStage { pub fn new() -> Self { Self { max_key_len: 256, max_payload_len: 1024 * 1024 } } }
impl Stage<PipelineMessage, PipelineMessage> for MemoryNormalizeStage {
    fn kind(&self) -> StageKind { StageKind::Normalize }
    fn name(&self) -> &'static str { "memory-normalize" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; if m.kind.len() > self.max_key_len { m.kind.truncate(self.max_key_len); } if m.payload.len() > self.max_payload_len { let mut idx = self.max_payload_len; while idx > 0 && !m.payload.is_char_boundary(idx) { idx -= 1; } m.payload.truncate(idx); } Ok(m) }
}

/// Policy stage: TTL enforcement on kind.
#[derive(Debug, Clone)] pub struct MemoryPolicyStage { pub policy: TtlPolicy }
impl MemoryPolicyStage { pub fn new(policy: TtlPolicy) -> Self { Self { policy } } }
impl Stage<PipelineMessage, PipelineMessage> for MemoryPolicyStage {
    fn kind(&self) -> StageKind { StageKind::Policy }
    fn name(&self) -> &'static str { "memory-policy" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; let ttl_secs = self.policy.max_age_secs; if !m.trace_id.contains(":ttl=") { m.trace_id = format!("{}:ttl={}", m.trace_id, ttl_secs); } Ok(m) }
}

/// Reliability stage: 60s fingerprint cache dedup.
pub struct MemoryReliabilityStage { pub cache: std::sync::Arc<FingerprintCache> }
impl MemoryReliabilityStage { pub fn new(cache: std::sync::Arc<FingerprintCache>) -> Self { Self { cache } } }
impl Stage<PipelineMessage, PipelineMessage> for MemoryReliabilityStage {
    fn kind(&self) -> StageKind { StageKind::Reliability }
    fn name(&self) -> &'static str { "memory-reliability" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; let mut h = DefaultHasher::new(); m.kind.hash(&mut h); m.payload.hash(&mut h); let fp = format!("{}|{:x}", m.kind, h.finish()); if self.cache.check(&fp) { m.attempt += 1; return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Reliability, source: "memory fingerprint dedup hit".into() }); } m.attempt += 1; Ok(m) }
}

/// Throttle stage: per-key rate limit.
pub struct MemoryThrottleStage { pub throttle: std::sync::Arc<KeyThrottle> }
impl MemoryThrottleStage { pub fn new(throttle: std::sync::Arc<KeyThrottle>) -> Self { Self { throttle } } }
impl Stage<PipelineMessage, PipelineMessage> for MemoryThrottleStage {
    fn kind(&self) -> StageKind { StageKind::Throttle }
    fn name(&self) -> &'static str { "memory-throttle" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let m = input; let key = m.kind.clone(); if !self.throttle.check_and_inc(&key) { return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Throttle, source: format!("key {} over rate limit", key).into() }); } Ok(m) }
}

/// Memory pipeline builder.
pub struct MemoryPipelineBuilder { name: String, ttl: TtlPolicy, max_per_key: u32, fingerprint_cache: std::sync::Arc<FingerprintCache>, key_throttle: std::sync::Arc<KeyThrottle> }
impl MemoryPipelineBuilder {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), ttl: TtlPolicy::default(), max_per_key: 100, fingerprint_cache: std::sync::Arc::new(FingerprintCache::new()), key_throttle: std::sync::Arc::new(KeyThrottle::new(100)) } }
    pub fn with_ttl(mut self, t: TtlPolicy) -> Self { self.ttl = t; self }
    pub fn with_max_per_key(mut self, n: u32) -> Self { self.max_per_key = n; self.key_throttle = std::sync::Arc::new(KeyThrottle::new(n)); self }
    pub fn with_fingerprint_cache(mut self, c: std::sync::Arc<FingerprintCache>) -> Self { self.fingerprint_cache = c; self }
    pub fn build(self) -> Pipeline<MemoryPipeline, PipelineMessage, PipelineMessage> { let config = PipelineConfig::new(self.name, "MemoryPipeline"); Pipeline::new(config).with_stage(MemoryDispatchStage).with_stage(MemoryNormalizeStage::new()).with_stage(MemoryPolicyStage::new(self.ttl)).with_stage(MemoryReliabilityStage::new(self.fingerprint_cache)).with_stage(MemoryThrottleStage::new(self.key_throttle)) }
}

#[cfg(test)] mod tests { use super::*;
    #[test] fn memory_op_as_str() { assert_eq!(MemoryOp::Insert.as_str(), "insert"); assert_eq!(MemoryOp::Retrieve.as_str(), "retrieve"); }
    #[test] fn dispatch_defaults_kind() { let s = MemoryDispatchStage; let m = PipelineMessage::new("", "p"); let o = s.process(m).unwrap(); assert_eq!(o.kind, "episode-insert"); }
    #[test] fn dispatch_preserves_kind() { let s = MemoryDispatchStage; let m = PipelineMessage::new("note-insert", "x"); let o = s.process(m).unwrap(); assert_eq!(o.kind, "note-insert"); }
    #[test] fn normalize_truncates_long_payload() { let s = MemoryNormalizeStage::new(); let m = PipelineMessage::new("k", "a".repeat(2_000_000)); let o = s.process(m).unwrap(); assert!(o.payload.len() <= 1024 * 1024); }
    #[test] fn normalize_truncates_long_kind() { let s = MemoryNormalizeStage::new(); let m = PipelineMessage::new("k".repeat(1000), "p"); let o = s.process(m).unwrap(); assert!(o.kind.len() <= 256); }
    #[test] fn policy_adds_ttl() { let s = MemoryPolicyStage::new(TtlPolicy::default()); let m = PipelineMessage::new("k", "p"); let o = s.process(m).unwrap(); assert!(o.trace_id.contains(":ttl=")); }
    #[test] fn reliability_dedups() { let s = MemoryReliabilityStage::new(std::sync::Arc::new(FingerprintCache::new())); let m = PipelineMessage::new("unique-mem-1", "p"); let _ = s.process(m.clone()).unwrap(); let r = s.process(m); assert!(r.is_err()); }
    #[test] fn throttle_rate_limit_per_key() { let s = MemoryThrottleStage::new(std::sync::Arc::new(KeyThrottle::new(2))); assert!(s.process(PipelineMessage::new("kind-a", "1")).is_ok()); assert!(s.process(PipelineMessage::new("kind-a", "2")).is_ok()); assert!(s.process(PipelineMessage::new("kind-a", "3")).is_err()); assert!(s.process(PipelineMessage::new("kind-b", "1")).is_ok()); }
    #[test] fn full_pipeline_runs() { let p = MemoryPipelineBuilder::new("mem").build(); let r = p.run(PipelineMessage::new("episode-insert", "hello world")); assert!(r.is_ok(), "should pass: {:?}", r.err()); }
    #[test] fn pipeline_stage_order() { let p = MemoryPipelineBuilder::new("mem-order").build(); let k = p.stage_kinds(); assert_eq!(k.len(), 5); assert_eq!(k[0], StageKind::Dispatch); assert_eq!(k[4], StageKind::Throttle); }
}
