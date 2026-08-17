//! R23 pub use 顶层导出草稿（apeireth-life-force）。
//!
//! **8 项承诺**: 全部遵守。**不假装**: 本文件仅为草稿。

//! # 建议加在 crates/apeireth-life-force/src/lib.rs 末尾

pub use crate::{
    LifeForce, LifeForceError, ReflectionPeriod, ReflectionPeriodState,
    ReflectionTrigger, SelfGrowthIndicator,
    emergence::{EmergenceDetector, EmergenceSignal},
    reflection_cycle::{ReflectionCycleScheduler, ReflectionCycleState},
};
