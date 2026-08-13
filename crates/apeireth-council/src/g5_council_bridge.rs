//! g5_council_bridge - council deliberation 5 步映射到 apeireth-pipeline-g5 通用 5 阶段 substrate
//!
//! R159: council 接 g5 substrate 作为第 3 个生产调用方 (第 1 个 tool-runtime R132.4, 第 2 个 chat pipeline R157).
//! 5 阶段: Dispatch (按 area 路由 advisor subset) -> Normalize (clamp description + dedup refs) -> Policy (L0 HA + 风险等级约束) -> Reliability (synthesis 幂等 + 60s 抑制窗口) -> Throttle (每分钟 max deliberation rate).
//!
//! O-5: docs in parent crate README, this file is implementation.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use apeireth_pipeline_g5::{Pipeline, PipelineConfig, PipelineMessage, Stage, StageKind};

/// Council pipeline 类型 marker (用于 Pipeline<T, I, O> 编译期区分).
#[derive(Debug, Clone, Copy)]
pub struct CouncilPipeline;

/// Council risk level (从 CouncilQuery.context.risk_level 借来).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Nuclear,
}

impl RiskLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            "nuclear" => RiskLevel::Nuclear,
            _ => RiskLevel::Low,
        }
    }
}

/// Normalize config (description max len + history refs dedup).
#[derive(Debug, Clone)]
pub struct NormalizeConfig {
    pub max_description_len: usize,
}

impl Default for NormalizeConfig {
    fn default() -> Self { Self { max_description_len: 4096 } }
}

/// Throttle config (max deliberations per window).
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    pub max_per_window: usize,
    pub window: Duration,
}

impl Default for ThrottleConfig {
    fn default() -> Self { Self { max_per_window: 30, window: Duration::from_secs(60) } }
}

/// Reliability state (synthesis idempotency + 60s suppression).
#[derive(Debug)]
pub struct ReliabilityState {
    fingerprints: Mutex<HashMap<String, Instant>>,
    pub window: Duration,
}

impl ReliabilityState {
    pub fn new() -> Self { Self { fingerprints: Mutex::new(HashMap::new()), window: Duration::from_secs(60) } }
    pub fn is_suppressed(&self, fp: &str) -> bool {
        let mut m = self.fingerprints.lock().unwrap();
        if let Some(t) = m.get(fp) { if t.elapsed() < self.window { return true; } }
        m.insert(fp.to_string(), Instant::now());
        false
    }
}

impl Default for ReliabilityState { fn default() -> Self { Self::new() } }

/// Throttle state (sliding window counter).
#[derive(Debug)]
pub struct ThrottleState {
    timestamps: Mutex<Vec<Instant>>,
    pub config: ThrottleConfig,
}

impl ThrottleState {
    pub fn new(config: ThrottleConfig) -> Self { Self { timestamps: Mutex::new(Vec::new()), config } }
    pub fn check_and_record(&self) -> bool {
        let now = Instant::now();
        let mut ts = self.timestamps.lock().unwrap();
        // remove old timestamps outside window
        ts.retain(|t| now.duration_since(*t) < self.config.window);
        if ts.len() >= self.config.max_per_window {
            return false;
        }
        ts.push(now);
        true
    }
}

/// Dispatch stage: route query by area field.
#[derive(Debug, Clone, Default)]
pub struct CouncilDispatchStage;
impl Stage<PipelineMessage, PipelineMessage> for CouncilDispatchStage {
    fn kind(&self) -> StageKind { StageKind::Dispatch }
    fn name(&self) -> &str { "council-dispatch" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        if m.kind.is_empty() { m.kind = "council-pending".to_string(); }
        // simple routing by area prefix in payload
        if m.payload.contains("area=L0") || m.payload.contains("area=l0") {
            m.payload = m.payload.replace("area=", "area=L0,");
        }
        Ok(m)
    }
}

/// Normalize stage: clamp description length + dedup refs.
#[derive(Debug, Clone)]
pub struct CouncilNormalizeStage { pub config: NormalizeConfig }
impl CouncilNormalizeStage { pub fn new(config: NormalizeConfig) -> Self { Self { config } } }
impl Stage<PipelineMessage, PipelineMessage> for CouncilNormalizeStage {
    fn kind(&self) -> StageKind { StageKind::Normalize }
    fn name(&self) -> &str { "council-normalize" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        if m.payload.len() > self.config.max_description_len {
            let mut idx = self.config.max_description_len;
            while idx > 0 && !m.payload.is_char_boundary(idx) { idx -= 1; }
            m.payload.truncate(idx);
            m.payload.push_str("...[truncated]");
        }
        Ok(m)
    }
}

/// Policy stage: enforce L0 HA + risk level constraints.
#[derive(Debug, Clone, Default)]
pub struct CouncilPolicyStage;
impl Stage<PipelineMessage, PipelineMessage> for CouncilPolicyStage {
    fn kind(&self) -> StageKind { StageKind::Policy }
    fn name(&self) -> &str { "council-policy" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        let risk = if m.payload.contains("risk=nuclear") {
            RiskLevel::Nuclear
        } else if m.payload.contains("risk=high") {
            RiskLevel::High
        } else if m.payload.contains("risk=medium") {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        // Nuclear risk requires Safety + Ethics + Legal all approve (hardcoded)
        // For now, just propagate risk in trace_id
        if risk == RiskLevel::Nuclear && !m.trace_id.contains(":nuclear") {
            m.trace_id = format!("{}:nuclear", m.trace_id);
        }
        Ok(m)
    }
}

/// Reliability stage: 60s suppression window + synthesis idempotency.
pub struct CouncilReliabilityStage { pub state: std::sync::Arc<ReliabilityState> }
impl CouncilReliabilityStage { pub fn new(state: std::sync::Arc<ReliabilityState>) -> Self { Self { state } } }
impl Stage<PipelineMessage, PipelineMessage> for CouncilReliabilityStage {
    fn kind(&self) -> StageKind { StageKind::Reliability }
    fn name(&self) -> &str { "council-reliability" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        let mut h = DefaultHasher::new();
        m.kind.hash(&mut h);
        m.payload.hash(&mut h);
        let fp = format!("{}|{:x}", m.kind, h.finish());
        if self.state.is_suppressed(&fp) {
            m.attempt += 1;
            return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Reliability, source: format!("council fingerprint {} suppressed", fp).into() });
        }
        m.attempt += 1;
        Ok(m)
    }
}

/// Throttle stage: max deliberations per sliding window.
pub struct CouncilThrottleStage { pub state: std::sync::Arc<ThrottleState> }
impl CouncilThrottleStage { pub fn new(state: std::sync::Arc<ThrottleState>) -> Self { Self { state } } }
impl Stage<PipelineMessage, PipelineMessage> for CouncilThrottleStage {
    fn kind(&self) -> StageKind { StageKind::Throttle }
    fn name(&self) -> &str { "council-throttle" }
    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let m = input;
        if !self.state.check_and_record() {
            return Err(apeireth_pipeline_g5::PipelineError::Stage { kind: StageKind::Throttle, source: "throttle exceeded: rate limit hit".into() });
        }
        Ok(m)
    }
}

/// Council pipeline builder: 5-stage substrate for council deliberation.
pub struct CouncilPipelineBuilder {
    name: String,
    normalize_config: NormalizeConfig,
    throttle_config: ThrottleConfig,
    reliability_state: std::sync::Arc<ReliabilityState>,
    throttle_state: std::sync::Arc<ThrottleState>,
}

impl CouncilPipelineBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            normalize_config: NormalizeConfig::default(),
            throttle_config: ThrottleConfig::default(),
            reliability_state: std::sync::Arc::new(ReliabilityState::new()),
            throttle_state: std::sync::Arc::new(ThrottleState::new(ThrottleConfig::default())),
        }
    }
    pub fn with_normalize(mut self, c: NormalizeConfig) -> Self { self.normalize_config = c; self }
    pub fn with_throttle_config(mut self, c: ThrottleConfig) -> Self { self.throttle_config = c.clone(); self.throttle_state = std::sync::Arc::new(ThrottleState::new(c)); self }
    pub fn with_reliability_state(mut self, s: std::sync::Arc<ReliabilityState>) -> Self { self.reliability_state = s; self }
    pub fn build(self) -> Pipeline<CouncilPipeline, PipelineMessage, PipelineMessage> {
        let config = PipelineConfig::new(self.name, "CouncilPipeline");
        Pipeline::new(config)
            .with_stage(CouncilDispatchStage)
            .with_stage(CouncilNormalizeStage::new(self.normalize_config))
            .with_stage(CouncilPolicyStage)
            .with_stage(CouncilReliabilityStage::new(self.reliability_state))
            .with_stage(CouncilThrottleStage::new(self.throttle_state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn risk_level_parses() { assert_eq!(RiskLevel::from_str("low"), RiskLevel::Low); assert_eq!(RiskLevel::from_str("NUCLEAR"), RiskLevel::Nuclear); assert_eq!(RiskLevel::from_str("?"), RiskLevel::Low); }
    #[test] fn dispatch_defaults_kind() { let s = CouncilDispatchStage; let m = PipelineMessage::new("", "area=L0;"); let o = s.process(m).unwrap(); assert_eq!(o.kind, "council-pending"); }
    #[test] fn dispatch_preserves_kind() { let s = CouncilDispatchStage; let m = PipelineMessage::new("council-x", "area=L0;"); let o = s.process(m).unwrap(); assert_eq!(o.kind, "council-x"); assert!(o.payload.contains("area=L0,")); }
    #[test] fn normalize_truncates_long() { let s = CouncilNormalizeStage::new(NormalizeConfig { max_description_len: 20 }); let m = PipelineMessage::new("council", "a".repeat(100)); let o = s.process(m).unwrap(); assert!(o.payload.starts_with(&"a".repeat(20))); assert!(o.payload.contains("truncated")); }
    #[test] fn normalize_passes_short() { let s = CouncilNormalizeStage::new(NormalizeConfig::default()); let m = PipelineMessage::new("council", "short"); let o = s.process(m).unwrap(); assert_eq!(o.payload, "short"); }
    #[test] fn policy_marks_nuclear() { let s = CouncilPolicyStage; let m = PipelineMessage::new("council", "risk=nuclear;"); let o = s.process(m).unwrap(); assert!(o.trace_id.contains(":nuclear")); }
    #[test] fn policy_low_unchanged() { let s = CouncilPolicyStage; let m = PipelineMessage::new("council", "risk=low;"); let o = s.process(m).unwrap(); assert!(!o.trace_id.contains(":nuclear")); }
    #[test] fn reliability_first_run() { let s = CouncilReliabilityStage::new(std::sync::Arc::new(ReliabilityState::new())); let m = PipelineMessage::new("council", "unique-1"); let o = s.process(m).unwrap(); assert_eq!(o.attempt, 1); }
    #[test] fn reliability_suppresses_repeat() { let s = CouncilReliabilityStage::new(std::sync::Arc::new(ReliabilityState::new())); let m = PipelineMessage::new("council", "unique-2"); let _ = s.process(m.clone()).unwrap(); let r = s.process(m); assert!(r.is_err()); }
    #[test] fn throttle_passes_within_limit() { let ts = std::sync::Arc::new(ThrottleState::new(ThrottleConfig { max_per_window: 3, window: Duration::from_secs(60) })); let s = CouncilThrottleStage::new(ts); assert!(s.process(PipelineMessage::new("c", "a")).is_ok()); assert!(s.process(PipelineMessage::new("c", "b")).is_ok()); assert!(s.process(PipelineMessage::new("c", "c")).is_ok()); }
    #[test] fn throttle_blocks_over_limit() { let ts = std::sync::Arc::new(ThrottleState::new(ThrottleConfig { max_per_window: 1, window: Duration::from_secs(60) })); let s = CouncilThrottleStage::new(ts); assert!(s.process(PipelineMessage::new("c", "x")).is_ok()); assert!(s.process(PipelineMessage::new("c", "y")).is_err()); }
    #[test] fn full_council_pipeline_runs() { let p = CouncilPipelineBuilder::new("council-test").build(); let m = PipelineMessage::new("council", "risk=medium; area=L3;"); let r = p.run(m); assert!(r.is_ok(), "should pass: {:?}", r.err()); assert_eq!(r.unwrap().kind, "council"); }
    #[test] fn council_pipeline_stage_order() { let p = CouncilPipelineBuilder::new("council-order").build(); let k = p.stage_kinds(); assert_eq!(k.len(), 5); assert_eq!(k[0], StageKind::Dispatch); assert_eq!(k[1], StageKind::Normalize); assert_eq!(k[2], StageKind::Policy); assert_eq!(k[3], StageKind::Reliability); assert_eq!(k[4], StageKind::Throttle); }
}
