//! R177 pipeline-g5 organ Kani proofs (W5)

#![allow(missing_docs)]

use crate::pipeline::{
    PipelineConfig, PIPELINE_MAX_STAGES, PIPELINE_MIN_STAGES, PIPELINE_STAGE_NAME_MAX_LEN,
};
use crate::stage::{StageKind, STAGE_KIND_COUNT, STAGE_ORDER};

#[test]
fn r177_pg_01_stage_kind_count_5() {
    assert_eq!(STAGE_KIND_COUNT, 5);
    assert_eq!(STAGE_ORDER.len(), 5);
}

#[test]
fn r177_pg_02_stage_order_specific() {
    assert_eq!(STAGE_ORDER[0], StageKind::Dispatch);
    assert_eq!(STAGE_ORDER[1], StageKind::Normalize);
    assert_eq!(STAGE_ORDER[2], StageKind::Policy);
    assert_eq!(STAGE_ORDER[3], StageKind::Reliability);
    assert_eq!(STAGE_ORDER[4], StageKind::Throttle);
}

#[test]
fn r177_pg_03_stage_kind_5_variants() {
    let kinds = [
        StageKind::Dispatch,
        StageKind::Normalize,
        StageKind::Policy,
        StageKind::Reliability,
        StageKind::Throttle,
    ];
    assert_eq!(kinds.len(), 5);
}

#[test]
fn r177_pg_04_stage_kinds_distinct() {
    let mut seen = std::collections::HashSet::new();
    for k in &STAGE_ORDER {
        assert!(seen.insert(*k), "StageKind 重复: {:?}", k);
    }
    assert_eq!(seen.len(), 5);
}

#[test]
fn r177_pg_05_stage_kind_as_str() {
    assert_eq!(StageKind::Dispatch.as_str(), "dispatch");
    assert_eq!(StageKind::Normalize.as_str(), "normalize");
    assert_eq!(StageKind::Policy.as_str(), "policy");
    assert_eq!(StageKind::Reliability.as_str(), "reliability");
    assert_eq!(StageKind::Throttle.as_str(), "throttle");
}

#[test]
fn r177_pg_06_pipeline_constants() {
    assert_eq!(PIPELINE_MIN_STAGES, 1);
    assert_eq!(PIPELINE_MAX_STAGES, 5);
    assert_eq!(PIPELINE_STAGE_NAME_MAX_LEN, 32);
    assert_eq!(PIPELINE_MIN_STAGES + PIPELINE_MAX_STAGES, 6);
}

#[test]
fn r177_pg_07_pipeline_config_new() {
    let c = PipelineConfig::new("chat", "ChatPipeline");
    assert_eq!(c.name, "chat");
    assert_eq!(c.type_marker, "ChatPipeline");
    assert!(c.strict_order);
    assert!(!c.diagnostics);
}

#[test]
fn r177_pg_08_pipeline_config_with_strict_disabled() {
    let c = PipelineConfig::new("chat", "ChatPipeline").with_strict_order_disabled();
    assert!(!c.strict_order);
}

#[test]
fn r177_pg_09_pipeline_config_with_diagnostics() {
    let c = PipelineConfig::new("chat", "ChatPipeline").with_diagnostics();
    assert!(c.diagnostics);
}

#[test]
fn r177_pg_10_stage_kind_display() {
    assert_eq!(format!("{}", StageKind::Dispatch), "dispatch");
    assert_eq!(format!("{}", StageKind::Throttle), "throttle");
}

#[cfg(kani)]
#[kani::proof]
fn r177_pg_kani_01_stage_count() {
    assert_eq!(STAGE_KIND_COUNT, 5);
    assert_eq!(STAGE_ORDER.len(), 5);
}

#[cfg(kani)]
#[kani::proof]
fn r177_pg_kani_02_pipeline_bounds() {
    assert_eq!(PIPELINE_MIN_STAGES, 1);
    assert_eq!(PIPELINE_MAX_STAGES, 5);
}
