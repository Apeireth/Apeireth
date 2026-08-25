//! Local pipeline runtime — minimal Pipeline<T,I,O> 5-stage substrate (v2 适配).
//!
//! **v2 适配**:
//! v1 依赖 `apeireth_pipeline_g5` crate (提供 `Pipeline` / `Stage` / `PipelineMessage` / `StageKind`).
//! v2 没有 apeireth-pipeline-g5 crate. 在本模块本地定义等价 runtime,
//! 公共 API 与 v1 等价, 让 g5_council_bridge.rs 0 改业务代码即可编译.
//!
//! 设计: 同步 Pipeline, I/O 都是 same-type (`PipelineMessage -> PipelineMessage`) 不做转换.
//! 5 stage kinds: Dispatch / Normalize / Policy / Reliability / Throttle (per v1).

#![allow(missing_docs)]

use std::marker::PhantomData;
use thiserror::Error;

/// Pipeline message (per v1: kind + payload + trace_id + attempt)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineMessage {
    pub kind: String,
    pub payload: String,
    pub trace_id: String,
    pub attempt: u32,
}

impl PipelineMessage {
    pub fn new(kind: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            payload: payload.into(),
            trace_id: String::new(),
            attempt: 0,
        }
    }

    pub fn with_trace_id(mut self, t: impl Into<String>) -> Self {
        self.trace_id = t.into();
        self
    }
}

/// Stage kind (per v1 5-stage substrate)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageKind {
    Dispatch,
    Normalize,
    Policy,
    Reliability,
    Throttle,
}

impl StageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dispatch => "dispatch",
            Self::Normalize => "normalize",
            Self::Policy => "policy",
            Self::Reliability => "reliability",
            Self::Throttle => "throttle",
        }
    }
}

/// Pipeline error
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("stage {kind:?} failed: {source}")]
    Stage {
        kind: StageKind,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("pipeline misconfigured: {0}")]
    Misconfigured(String),
}

/// Stage trait
pub trait Stage<I, O>: Send + Sync {
    fn kind(&self) -> StageKind;
    fn name(&self) -> &str;
    fn process(&self, input: I) -> Result<O, PipelineError>;
}

/// Pipeline config (name + tag for compile-time marker)
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub name: String,
    pub tag: String,
}

impl PipelineConfig {
    pub fn new(name: impl Into<String>, tag: impl Into<String>) -> Self {
        Self { name: name.into(), tag: tag.into() }
    }
}

/// Pipeline<T,I,O>: 同 type pipeline (I = O = PipelineMessage in council 5 步)
pub struct Pipeline<T, I, O> {
    config: PipelineConfig,
    stages: Vec<Box<dyn Stage<O, O>>>,
    _marker: PhantomData<(T, I, O)>,
}

impl<T, I, O> Pipeline<T, I, O> {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config, stages: Vec::new(), _marker: PhantomData }
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    pub fn stage_kinds(&self) -> Vec<StageKind> {
        self.stages.iter().map(|s| s.kind()).collect()
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

// 我们需要 .with_stage 接受任何同型 stage, 这里简化为 O -> O
impl<T, I> Pipeline<T, I, I> {
    pub fn with_stage<S: Stage<I, I> + 'static>(mut self, stage: S) -> Self {
        self.stages.push(Box::new(stage));
        self
    }

    /// 同步跑 pipeline (per v1 Pipeline::run)
    pub fn run(&self, input: I) -> Result<I, PipelineError> {
        let mut current = input;
        for stage in &self.stages {
            current = stage.process(current)?;
        }
        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct IdentityStage;
    impl Stage<PipelineMessage, PipelineMessage> for IdentityStage {
        fn kind(&self) -> StageKind { StageKind::Dispatch }
        fn name(&self) -> &str { "id" }
        fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> { Ok(input) }
    }

    struct ErrorStage;
    impl Stage<PipelineMessage, PipelineMessage> for ErrorStage {
        fn kind(&self) -> StageKind { StageKind::Throttle }
        fn name(&self) -> &str { "err" }
        fn process(&self, _: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
            Err(PipelineError::Stage {
                kind: StageKind::Throttle,
                source: "test error".to_string().into(),
            })
        }
    }

    #[test]
    fn empty_pipeline_passes_through() {
        let p: Pipeline<(), PipelineMessage, PipelineMessage> = Pipeline::new(
            PipelineConfig::new("e", "t")
        );
        let m = PipelineMessage::new("k", "p");
        assert!(p.run(m).is_ok());
    }

    #[test]
    fn identity_stage_preserves() {
        let p: Pipeline<(), PipelineMessage, PipelineMessage> = Pipeline::new(
            PipelineConfig::new("e", "t")
        ).with_stage(IdentityStage);
        let m = PipelineMessage::new("k", "p");
        let out = p.run(m).unwrap();
        assert_eq!(out.kind, "k");
        assert_eq!(out.payload, "p");
    }

    #[test]
    fn error_stage_propagates() {
        let p: Pipeline<(), PipelineMessage, PipelineMessage> = Pipeline::new(
            PipelineConfig::new("e", "t")
        ).with_stage(ErrorStage);
        let r = p.run(PipelineMessage::new("k", "p"));
        assert!(r.is_err());
    }

    #[test]
    fn stage_kinds_returns_in_order() {
        let p: Pipeline<(), PipelineMessage, PipelineMessage> = Pipeline::new(
            PipelineConfig::new("e", "t")
        ).with_stage(IdentityStage)
         .with_stage(ErrorStage);
        let k = p.stage_kinds();
        assert_eq!(k.len(), 2);
        assert_eq!(k[0], StageKind::Dispatch);
        assert_eq!(k[1], StageKind::Throttle);
    }
}
