
//! g5_runtime_bridge - runtime task lifecycle 5 stages mapping to apeireth-pipeline-g5
//!
//! R160: apeireth-runtime 接 g5 作为第 4 个生产调用方 (tool-runtime R132.4 + chat R157 + council R159).
//! 5 stages: Dispatch (task register) -> Normalize (payload validate) -> Policy (concurrency cap) -> Reliability (retry fingerprint) -> Throttle (per-tick rate limit).
#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use apeireth_pipeline_g5::{Pipeline, PipelineConfig, PipelineMessage, Stage, StageKind};

#[derive(Debug, Clone, Copy)] pub struct RuntimePipeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus { Pending, Running, Completed, Failed }
impl TaskStatus { pub fn as_str(&self) -> &'static str { match self { TaskStatus::Pending => "pending", TaskStatus::Running => "running", TaskStatus::Completed => "completed", TaskStatus::Failed => "failed" } } }

#[derive(Debug, Clone)]
pub struct PolicyConfig { pub max_concurrent: usize }
impl Default for PolicyConfig { fn default() -> Self { Self { max_concurrent: 16 } } }

#[derive(Debug, Clone)]
pub struct ThrottleConfig { pub max_per_tick: usize, pub tick_interval: Duration }
impl Default for ThrottleConfig { fn default() -> Self { Self { max_per_tick: 100, tick_interval: Duration::from_millis(100) } } }

#[derive(Debug)]
pub struct ReliabilityState { fingerprints: Mutex<HashMap<String, Instant>>, pub window: Duration }
impl ReliabilityState { pub fn new() -> Self { Self { fingerprints: Mutex::new(HashMap::new()), window: Duration::from_secs(30) } } pub fn is_suppressed(&self, fp: &str) -> bool { let mut m = self.fingerprints.lock().unwrap(); if let Some(t) = m.get(fp) { if t.elapsed() < self.window { return true; } } m.insert(fp.to_string(), Instant::now()); false } }
impl Default for ReliabilityState { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct ConcurrencyState { current: Mutex<usize> }
impl ConcurrencyState { pub fn new() -> Self { Self { current: Mutex::new(0) } } pub fn try_acquire(&self, max: usize) -> bool { let mut c = self.current.lock().unwrap(); if *c >= max { false } else { *c += 1; true } } pub fn release(&self) { let mut c = self.current.lock().unwrap(); if *c > 0 { *c -= 1; } } pub fn current(&self) -> usize { *self.current.lock().unwrap() } }

/// Dispatch stage: register task in Pending.
#[derive(Debug, Clone, Default)] pub struct RuntimeDispatchStage;
impl Stage<PipelineMessage, PipelineMessage> for RuntimeDispatchStage {
    fn kind(&self) -> StageKind { StageKind::Dispatch }
    fn name(&self) -> &'static str { "runtime-dispatch" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; if m.kind.is_empty() { m.kind = "task-pending".to_string(); } Ok(m) }
}

/// Normalize stage: payload size cap + status field.
#[derive(Debug, Clone)] pub struct RuntimeNormalizeStage { pub max_payload_len: usize }
impl RuntimeNormalizeStage { pub fn new(max_payload_len: usize) -> Self { Self { max_payload_len } } }
impl Stage<PipelineMessage, PipelineMessage> for RuntimeNormalizeStage {
    fn kind(&self) -> StageKind { StageKind::Normalize }
    fn name(&self) -> &'static str { "runtime-normalize" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; if m.payload.len() > self.max_payload_len { let mut idx = self.max_payload_len; while idx > 0 && !m.payload.is_char_boundary(idx) { idx -= 1; } m.payload.truncate(idx); } Ok(m) }
}

/// Policy stage: concurrency cap.
pub struct RuntimePolicyStage { pub config: PolicyConfig, pub state: std::sync::Arc<ConcurrencyState> }
impl RuntimePolicyStage { pub fn new(config: PolicyConfig, state: std::sync::Arc<ConcurrencyState>) -> Self { Self { config, state } } }
impl Stage<PipelineMessage, PipelineMessage> for RuntimePolicyStage {
    fn kind(&self) -> StageKind { StageKind::Policy }
    fn name(&self) -> &'static str { "runtime-policy" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let m = input; if !self.state.try_acquire(self.config.max_concurrent) { return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Policy, source: "concurrency cap exceeded".into() }); } Ok(m) }
}

/// Reliability stage: 30s suppression window for retries.
pub struct RuntimeReliabilityStage { pub state: std::sync::Arc<ReliabilityState> }
impl RuntimeReliabilityStage { pub fn new(state: std::sync::Arc<ReliabilityState>) -> Self { Self { state } } }
impl Stage<PipelineMessage, PipelineMessage> for RuntimeReliabilityStage {
    fn kind(&self) -> StageKind { StageKind::Reliability }
    fn name(&self) -> &'static str { "runtime-reliability" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let mut m = input; let mut h = DefaultHasher::new(); m.kind.hash(&mut h); m.payload.hash(&mut h); let fp = format!("{}|{:x}", m.kind, h.finish()); if self.state.is_suppressed(&fp) { m.attempt += 1; return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Reliability, source: format!("task {} suppressed", fp).into() }); } m.attempt += 1; Ok(m) }
}

/// Throttle stage: per-tick rate limit.
pub struct RuntimeThrottleStage { pub state: std::sync::Arc<Mutex<HashMap<Instant, usize>>>, pub config: ThrottleConfig }
impl RuntimeThrottleStage { pub fn new(config: ThrottleConfig) -> Self { Self { state: std::sync::Arc::new(Mutex::new(HashMap::new())), config } } }
impl Stage<PipelineMessage, PipelineMessage> for RuntimeThrottleStage {
    fn kind(&self) -> StageKind { StageKind::Throttle }
    fn name(&self) -> &'static str { "runtime-throttle" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> { let m = input; let mut map = self.state.lock().unwrap(); let now = Instant::now(); let cutoff = now - self.config.tick_interval; map.retain(|t, _| *t > cutoff); let count: usize = map.values().sum(); if count >= self.config.max_per_tick { return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Throttle, source: "tick rate exceeded".into() }); } *map.entry(now).or_insert(0) += 1; Ok(m) }
}

/// Runtime pipeline builder.
pub struct RuntimePipelineBuilder { name: String, normalize_max: usize, policy_config: PolicyConfig, throttle_config: ThrottleConfig, reliability_state: std::sync::Arc<ReliabilityState>, concurrency_state: std::sync::Arc<ConcurrencyState> }
impl RuntimePipelineBuilder {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), normalize_max: 16 * 1024, policy_config: PolicyConfig::default(), throttle_config: ThrottleConfig::default(), reliability_state: std::sync::Arc::new(ReliabilityState::new()), concurrency_state: std::sync::Arc::new(ConcurrencyState::new()) } }
    pub fn with_normalize_max(mut self, max: usize) -> Self { self.normalize_max = max; self }
    pub fn with_policy(mut self, c: PolicyConfig) -> Self { self.policy_config = c; self }
    pub fn with_throttle(mut self, c: ThrottleConfig) -> Self { self.throttle_config = c; self }
    pub fn with_reliability(mut self, s: std::sync::Arc<ReliabilityState>) -> Self { self.reliability_state = s; self }
    pub fn build(self) -> Pipeline<RuntimePipeline, PipelineMessage, PipelineMessage> { let config = PipelineConfig::new(self.name, "RuntimePipeline"); Pipeline::new(config).with_stage(RuntimeDispatchStage).with_stage(RuntimeNormalizeStage::new(self.normalize_max)).with_stage(RuntimePolicyStage::new(self.policy_config, self.concurrency_state)).with_stage(RuntimeReliabilityStage::new(self.reliability_state)).with_stage(RuntimeThrottleStage::new(self.throttle_config)) }
}

#[cfg(test)]
mod tests { use super::*;
    #[test] fn task_status_as_str() { assert_eq!(TaskStatus::Pending.as_str(), "pending"); assert_eq!(TaskStatus::Completed.as_str(), "completed"); }
    #[test] fn dispatch_defaults_kind() { let s = RuntimeDispatchStage; let m = PipelineMessage::new("", "task"); let o = s.process(m).unwrap(); assert_eq!(o.kind, "task-pending"); }
    #[test] fn dispatch_preserves_kind() { let s = RuntimeDispatchStage; let m = PipelineMessage::new("task-running", "x"); let o = s.process(m).unwrap(); assert_eq!(o.kind, "task-running"); }
    #[test] fn normalize_truncates() { let s = RuntimeNormalizeStage::new(10); let m = PipelineMessage::new("task", "a".repeat(100)); let o = s.process(m).unwrap(); assert!(o.payload.starts_with(&"a".repeat(10))); }
    #[test] fn policy_caps_concurrency() { let state = std::sync::Arc::new(ConcurrencyState::new()); let s = RuntimePolicyStage::new(PolicyConfig { max_concurrent: 1 }, state.clone()); assert!(s.process(PipelineMessage::new("task", "a")).is_ok()); assert!(s.process(PipelineMessage::new("task", "b")).is_err()); state.release(); assert!(s.process(PipelineMessage::new("task", "c")).is_ok()); }
    #[test] fn reliability_suppresses_repeat() { let s = RuntimeReliabilityStage::new(std::sync::Arc::new(ReliabilityState::new())); let m = PipelineMessage::new("task", "unique-r-1"); let _ = s.process(m.clone()).unwrap(); let r = s.process(m); assert!(r.is_err()); }
    #[test] fn throttle_blocks_over_limit() { let s = RuntimeThrottleStage::new(ThrottleConfig { max_per_tick: 1, tick_interval: Duration::from_millis(100) }); assert!(s.process(PipelineMessage::new("task", "a")).is_ok()); assert!(s.process(PipelineMessage::new("task", "b")).is_err()); }
    #[test] fn full_pipeline_runs() { let p = RuntimePipelineBuilder::new("rt").with_policy(PolicyConfig { max_concurrent: 8 }).build(); let r = p.run(PipelineMessage::new("task", "classify-request")); assert!(r.is_ok(), "should pass: {:?}", r.err()); }
    #[test] fn pipeline_stage_order() { let p = RuntimePipelineBuilder::new("rt-order").build(); let k = p.stage_kinds(); assert_eq!(k.len(), 5); assert_eq!(k[0], StageKind::Dispatch); assert_eq!(k[4], StageKind::Throttle); }
}
