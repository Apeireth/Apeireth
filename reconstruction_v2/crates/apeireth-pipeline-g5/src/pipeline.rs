//! Pipeline runner + config.

use serde::{Deserialize, Serialize};

pub const PIPELINE_MIN_STAGES: usize = 1;
pub const PIPELINE_MAX_STAGES: usize = 16;
pub const PIPELINE_STAGE_NAME_MAX_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub platform: String,
    pub schema_version: String,
    pub stage_names: Vec<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            platform: "apeireth".into(),
            schema_version: "1".into(),
            stage_names: vec!["0 Dispatch".into(), "1 Normalize".into(), "2 Policy".into(), "3 Reliability".into(), "4 Throttle".into()],
        }
    }
}

impl PipelineConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.stage_names.len() < PIPELINE_MIN_STAGES {
            return Err(format!("too few stages: {}", self.stage_names.len()));
        }
        if self.stage_names.len() > PIPELINE_MAX_STAGES {
            return Err(format!("too many stages: {}", self.stage_names.len()));
        }
        for n in &self.stage_names {
            if n.len() > PIPELINE_STAGE_NAME_MAX_LEN {
                return Err(format!("stage name too long: {}", n));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub config: PipelineConfig,
}

impl Pipeline {
    pub fn new(config: PipelineConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn stage_count(&self) -> usize { self.config.stage_names.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(PIPELINE_MIN_STAGES, 1);
        assert_eq!(PIPELINE_MAX_STAGES, 16);
        assert_eq!(PIPELINE_STAGE_NAME_MAX_LEN, 64);
    }

    #[test]
    fn default_validates() {
        let cfg = PipelineConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_too_few() {
        let cfg = PipelineConfig { stage_names: vec![], ..Default::default() };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_too_many() {
        let cfg = PipelineConfig { stage_names: (0..20).map(|i| format!("s{i}")).collect(), ..Default::default() };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn pipeline_new() {
        let p = Pipeline::new(PipelineConfig::default()).unwrap();
        assert_eq!(p.stage_count(), 5);
    }
}
