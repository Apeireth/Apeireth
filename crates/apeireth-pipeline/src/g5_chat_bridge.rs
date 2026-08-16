//! g5_chat_bridge - chat pipeline 5 steps mapping to apeireth-pipeline-g5
//!
//! R157: add g5 substrate as 2nd production caller (1st was tool-runtime R132.4).
//! 5 stages: Dispatch -> Normalize -> Policy -> Reliability -> Throttle.
//!
//! O-5: docs in parent crate README, this file is implementation.
#![allow(missing_docs)]

use apeireth_pipeline_g5::{Pipeline, PipelineConfig, PipelineMessage, Stage, StageKind};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct ChatPipeline;

#[derive(Debug, Clone, Default)]
pub struct ChatPlaceholderContext {
    pub vars: HashMap<String, String>,
}

impl ChatPlaceholderContext {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_var(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.vars.insert(k.into(), v.into());
        self
    }
    pub fn resolve(&self, input: &str) -> String {
        let mut out = input.to_string();
        let mut safety = 0u32;
        loop {
            safety += 1;
            if safety > 64 {
                break;
            }
            let mut replaced = false;
            let mut new_out = String::with_capacity(out.len());
            let bytes = out.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == 123 && i + 1 < bytes.len() && bytes[i + 1] == 123 {
                    let mut j = i + 2;
                    while j + 1 < bytes.len() && !(bytes[j] == 125 && bytes[j + 1] == 125) {
                        j += 1;
                    }
                    if j + 1 < bytes.len() {
                        let name = out[i + 2..j].trim().to_string();
                        if let Some(val) = self.vars.get(&name) {
                            new_out.push_str(val);
                            replaced = true;
                        } else {
                            new_out.push_str(&out[i..j + 2]);
                        }
                        i = j + 2;
                        continue;
                    } else {
                        new_out.push_str(&out[i..]);
                        break;
                    }
                } else {
                    let c = out[i..].chars().next().unwrap();
                    new_out.push(c);
                    i += c.len_utf8();
                }
            }
            if !replaced || new_out == out {
                return new_out;
            }
            out = new_out;
        }
        out
    }
}
#[derive(Debug, Clone)]
pub struct TokenBudgetConfig {
    pub max_chars: usize,
    pub max_tokens: usize,
}
impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self {
            max_chars: 64 * 1024,
            max_tokens: 0,
        }
    }
}

pub struct RetrySuppressState {
    fingerprints: Mutex<HashMap<String, Instant>>,
    pub window: Duration,
}
impl RetrySuppressState {
    pub fn new() -> Self {
        Self {
            fingerprints: Mutex::new(HashMap::new()),
            window: Duration::from_secs(15),
        }
    }
    pub fn is_suppressed(&self, fp: &str) -> bool {
        let mut m = self.fingerprints.lock().unwrap();
        if let Some(t) = m.get(fp) {
            if t.elapsed() < self.window {
                return true;
            }
        }
        m.insert(fp.to_string(), Instant::now());
        false
    }
}
impl Default for RetrySuppressState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatDispatchStage;
impl Stage<PipelineMessage, PipelineMessage> for ChatDispatchStage {
    fn kind(&self) -> StageKind {
        StageKind::Dispatch
    }
    fn name(&self) -> &str {
        "chat-dispatch"
    }
    fn process(
        &self,
        input: PipelineMessage,
    ) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        if m.kind.is_empty() {
            m.kind = "chat".to_string();
        }
        Ok(m)
    }
}

#[derive(Debug, Clone)]
pub struct ChatNormalizeStage {
    pub ctx: ChatPlaceholderContext,
}
impl ChatNormalizeStage {
    pub fn new(ctx: ChatPlaceholderContext) -> Self {
        Self { ctx }
    }
}
impl Stage<PipelineMessage, PipelineMessage> for ChatNormalizeStage {
    fn kind(&self) -> StageKind {
        StageKind::Normalize
    }
    fn name(&self) -> &str {
        "chat-normalize"
    }
    fn process(
        &self,
        input: PipelineMessage,
    ) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        m.payload = self.ctx.resolve(&m.payload);
        Ok(m)
    }
}

#[derive(Debug, Clone)]
pub struct ChatPolicyStage {
    pub config: TokenBudgetConfig,
}
impl ChatPolicyStage {
    pub fn new(config: TokenBudgetConfig) -> Self {
        Self { config }
    }
}
impl Stage<PipelineMessage, PipelineMessage> for ChatPolicyStage {
    fn kind(&self) -> StageKind {
        StageKind::Policy
    }
    fn name(&self) -> &str {
        "chat-policy"
    }
    fn process(
        &self,
        input: PipelineMessage,
    ) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        if m.payload.len() > self.config.max_chars {
            let mut idx = self.config.max_chars;
            while idx > 0 && !m.payload.is_char_boundary(idx) {
                idx -= 1;
            }
            m.payload.truncate(idx);
            m.payload.push_str("...[truncated by token budget]");
        }
        Ok(m)
    }
}

pub struct ChatReliabilityStage {
    pub state: std::sync::Arc<RetrySuppressState>,
}
impl ChatReliabilityStage {
    pub fn new(state: std::sync::Arc<RetrySuppressState>) -> Self {
        Self { state }
    }
}
impl Stage<PipelineMessage, PipelineMessage> for ChatReliabilityStage {
    fn kind(&self) -> StageKind {
        StageKind::Reliability
    }
    fn name(&self) -> &str {
        "chat-reliability"
    }
    fn process(
        &self,
        input: PipelineMessage,
    ) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        let mut m = input;
        let mut h = DefaultHasher::new();
        m.kind.hash(&mut h);
        m.payload.hash(&mut h);
        let payload_hash = h.finish();
        let fingerprint = format!("{}|{:x}", m.kind, payload_hash);
        if self.state.is_suppressed(&fingerprint) {
            m.attempt += 1;
            return Err(apeireth_pipeline_g5::PipelineError::Stage {
                kind: StageKind::Reliability,
                source: format!("fingerprint {} suppressed", fingerprint).into(),
            });
        }
        m.attempt += 1;
        Ok(m)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatThrottleStage;
impl Stage<PipelineMessage, PipelineMessage> for ChatThrottleStage {
    fn kind(&self) -> StageKind {
        StageKind::Throttle
    }
    fn name(&self) -> &str {
        "chat-throttle"
    }
    fn process(
        &self,
        input: PipelineMessage,
    ) -> Result<PipelineMessage, apeireth_pipeline_g5::PipelineError> {
        Ok(input)
    }
}

pub struct ChatPipelineBuilder {
    name: String,
    ctx: ChatPlaceholderContext,
    token_config: TokenBudgetConfig,
    suppress_state: std::sync::Arc<RetrySuppressState>,
}

impl ChatPipelineBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ctx: ChatPlaceholderContext::new(),
            token_config: TokenBudgetConfig::default(),
            suppress_state: std::sync::Arc::new(RetrySuppressState::new()),
        }
    }
    pub fn with_placeholder(mut self, ctx: ChatPlaceholderContext) -> Self {
        self.ctx = ctx;
        self
    }
    pub fn with_token_budget(mut self, c: TokenBudgetConfig) -> Self {
        self.token_config = c;
        self
    }
    pub fn with_suppress_state(mut self, s: std::sync::Arc<RetrySuppressState>) -> Self {
        self.suppress_state = s;
        self
    }
    pub fn build(self) -> Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> {
        let config = PipelineConfig::new(self.name, "ChatPipeline");
        Pipeline::new(config)
            .with_stage(ChatDispatchStage)
            .with_stage(ChatNormalizeStage::new(self.ctx))
            .with_stage(ChatPolicyStage::new(self.token_config))
            .with_stage(ChatReliabilityStage::new(self.suppress_state))
            .with_stage(ChatThrottleStage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_defaults_to_chat() {
        let s = ChatDispatchStage;
        let m = PipelineMessage::new("", "hello");
        let o = s.process(m).unwrap();
        assert_eq!(o.kind, "chat");
    }
    #[test]
    fn dispatch_preserves_kind() {
        let s = ChatDispatchStage;
        let m = PipelineMessage::new("task", "x");
        let o = s.process(m).unwrap();
        assert_eq!(o.kind, "task");
    }
    #[test]
    fn normalize_resolves_vars() {
        let s = ChatNormalizeStage::new(
            ChatPlaceholderContext::new()
                .with_var("a", "A")
                .with_var("b", "B"),
        );
        let m = PipelineMessage::new("chat", "{{a}} {{b}}");
        let o = s.process(m).unwrap();
        assert_eq!(o.payload, "A B");
    }
    #[test]
    fn normalize_keeps_unknown() {
        let s = ChatNormalizeStage::new(ChatPlaceholderContext::new());
        let m = PipelineMessage::new("chat", "{{x}}");
        let o = s.process(m).unwrap();
        assert_eq!(o.payload, "{{x}}");
    }
    #[test]
    fn normalize_prevents_loops() {
        let s = ChatNormalizeStage::new(
            ChatPlaceholderContext::new()
                .with_var("a", "{{b}}")
                .with_var("b", "{{a}}"),
        );
        let m = PipelineMessage::new("chat", "{{a}}");
        let o = s.process(m).unwrap();
        assert!(!o.payload.is_empty());
    }
    #[test]
    fn policy_truncates_long() {
        let s = ChatPolicyStage::new(TokenBudgetConfig {
            max_chars: 10,
            max_tokens: 0,
        });
        let m = PipelineMessage::new("chat", "aaaaaaaaaaaaaaa");
        let o = s.process(m).unwrap();
        assert!(o.payload.starts_with(&"aaaaaaaaaa"));
        assert!(o.payload.contains("truncated"));
    }
    #[test]
    fn policy_passes_short() {
        let s = ChatPolicyStage::new(TokenBudgetConfig::default());
        let m = PipelineMessage::new("chat", "short");
        let o = s.process(m).unwrap();
        assert_eq!(o.payload, "short");
    }
    #[test]
    fn reliability_first_run() {
        let s = ChatReliabilityStage::new(std::sync::Arc::new(RetrySuppressState::new()));
        let m = PipelineMessage::new("chat", "unique1");
        let o = s.process(m).unwrap();
        assert_eq!(o.attempt, 1);
    }
    #[test]
    fn reliability_second_suppressed() {
        let s = ChatReliabilityStage::new(std::sync::Arc::new(RetrySuppressState::new()));
        let m = PipelineMessage::new("chat", "unique2");
        let _ = s.process(m.clone()).unwrap();
        let r = s.process(m);
        assert!(r.is_err());
    }
    #[test]
    fn throttle_passes() {
        let s = ChatThrottleStage;
        let m = PipelineMessage::new("chat", "hi");
        let o = s.process(m).unwrap();
        assert_eq!(o.payload, "hi");
    }
    #[test]
    fn full_pipeline_runs() {
        let p = ChatPipelineBuilder::new("t")
            .with_placeholder(ChatPlaceholderContext::new().with_var("g", "Hi"))
            .build();
        let m = PipelineMessage::new("chat", "{{g}} world");
        let r = p.run(m);
        assert!(r.is_ok(), "should pass: {:?}", r.err());
        assert_eq!(r.unwrap().payload, "Hi world");
    }
    #[test]
    fn pipeline_suppresses_repeat() {
        let p = ChatPipelineBuilder::new("t-suppress").build();
        let r1 = p.run(PipelineMessage::new("chat", "same-unique-3"));
        assert!(r1.is_ok());
        let r2 = p.run(PipelineMessage::new("chat", "same-unique-3"));
        assert!(r2.is_err());
    }
    #[test]
    fn stage_order_is_dispatch_normalize_policy_reliability_throttle() {
        let p = ChatPipelineBuilder::new("t-order").build();
        let k = p.stage_kinds();
        assert_eq!(k.len(), 5);
        assert_eq!(k[0], StageKind::Dispatch);
        assert_eq!(k[1], StageKind::Normalize);
        assert_eq!(k[2], StageKind::Policy);
        assert_eq!(k[3], StageKind::Reliability);
        assert_eq!(k[4], StageKind::Throttle);
    }
}
