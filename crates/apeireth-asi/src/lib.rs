//! apeireth-asi: ASI 北极星指标 (V0.5 24 维 + V1136 9 子测度真实测量) — R14 Phase 2
//!
//! 主 17:58 不假装: V0.5 是工程代理指标 (工程实现有没有受到欺骗或误解), 不假装达到 ASI。
//! round10-12 (qa_engineer): 24 维真实测量 + 9 子测度 + DimensionTrace + MeasurementHook +
//! RegressionAssertion + 3 个 CLI 命令 (asi trace / trend / diagnose)。
//!
//! 模块拆分:
//! - [`measurement`] — 24 measure_dim_* + 9 measure_sub_* 真实测量函数
//! - [`render`] — ASCII 渲染 (24 维详细表 / sparkline / diagnose)
//! - [`history`] — TraceRepository (SQLite 持久化, append-only)


pub mod calibration;
/// R22 ST-A3 per-dimension 深化。
pub mod dim_enhance;
pub mod drift;
pub mod history;
pub mod llm_judge;
pub mod measurement;
pub mod render;
pub mod scheduler;
/// R32-1: 真 token 计算 (替换 R19 启发式)
pub mod tokenizer;
/// R207: ASI 高级统计 utilities (std + auto-vectorization, 0 新依赖)
pub mod stats;
// R177: asi invariants (10 tests + 2 Kani proofs)
mod organ_kani_proofs;

pub use calibration::{
    AdaptiveBaseline, CalibrationCoefficients, CalibrationLoop, Coeff, LinearCalibration,
    UserFeedback,
};
pub use drift::{DriftAlarm, DriftDetector};
pub use history::TraceRepository;
pub use llm_judge::{judge, JudgeResult, LlmJudgeDim};
pub use measurement::{
    is_quiet_mode, measure_dim_01_thread_continuity, measure_dim_02_fact_recall,
    measure_dim_03_context_window, measure_dim_04_session_recovery,
    measure_dim_05_identity_persistence, measure_dim_06_importance_score,
    measure_dim_07_novelty_score, measure_dim_08_actionability_score,
    measure_dim_09_confidence_score, measure_dim_10_temporal_relevance,
    measure_dim_11_core_values_consistency, measure_dim_12_voice_consistency,
    measure_dim_13_behavioral_patterns, measure_dim_14_role_adherence,
    measure_dim_15_philosophy_alignment, measure_dim_16_v1_pass_rate, measure_dim_17_v2_pass_rate,
    measure_dim_18_v3_pass_rate, measure_dim_19_cone_of_truth_rate,
    measure_dim_20_action_guard_rate, measure_dim_21_cross_domain_generalization,
    measure_dim_22_abstraction_level, measure_dim_23_analogy_quality, measure_dim_24_tool_reuse,
    set_quiet_mode, DimensionRegistry, MeasurementHook, MeasurementSample, RegressionAssertion,
    RegressionResult,
};
pub use render::{ascii_sparkline, diagnose_weakest, format_trace_table, DiagnosticReport};
pub use scheduler::{RecalibrationScheduler, ScheduleReport};
/// R32-1: 真 token 计算 (替换 R19 启发式)
pub use tokenizer::{count_tokens, count_tokens_batch};

/// V0.5 北极星指标维度数 = 24 (round10-12 LOCKED)。
pub const V05_DIM_COUNT: usize = 24;

/// V1136 真测子测度数 = 9 (round10-12 LOCKED)。
pub const V1136_SUBMEASURE_COUNT: usize = 9;

/// 24 个 V0.5 维度的稳定名称顺序 (LOCKED)。trace / hook / regression 共享同一索引。
pub const V05_DIMENSION_NAMES: [&str; V05_DIM_COUNT] = [
    // Continuity (5)
    "thread_continuity",
    "fact_recall",
    "context_window",
    "session_recovery",
    "identity_persistence",
    // Salience (5)
    "importance_score",
    "novelty_score",
    "actionability_score",
    "confidence_score",
    "temporal_relevance",
    // Identity (5)
    "core_values_consistency",
    "voice_consistency",
    "behavioral_patterns",
    "role_adherence",
    "philosophy_alignment",
    // Philosophy Guard (5)
    "v1_pass_rate",
    "v2_pass_rate",
    "v3_pass_rate",
    "cone_of_truth_rate",
    "action_guard_rate",
    // Transferability (4)
    "cross_domain_generalization",
    "abstraction_level",
    "analogy_quality",
    "tool_reuse",
];

/// 9 个 V1136 子测度的稳定名称顺序 (LOCKED)。
pub const V1136_SUBMEASURE_NAMES: [&str; V1136_SUBMEASURE_COUNT] = [
    // Continuity 5
    "thread_continuity_score",
    "fact_recall_score",
    "context_window_score",
    "session_recovery_score",
    "identity_persistence_score",
    // Transferability 2
    "cross_domain_generalization_score",
    "tool_reuse_score",
    // Philosophy 2
    "v1_v2_pass_rate",
    "v3_action_guard_rate",
];

/// 旧版 V0.5 5 维投影 (保留兼容, 不让现有结构体字面量失效)。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AsiV05Scores {
    /// continuity (5 维投影均值)
    pub continuity: f64,
    /// salience (5 维投影均值)
    pub salience: f64,
    /// identity (5 维投影均值)
    pub identity: f64,
    /// philosophy_guard (5 维投影均值)
    pub philosophy_guard: f64,
    /// transferability (4 维投影均值)
    pub transferability: f64,
}

/// 旧版 V1136 7 子测度投影 (保留兼容)。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct V1136Submeasures {
    /// 5 continuity 子测度 (兼容旧 7 维布局)
    pub continuity_5: [f64; 5],
    /// 2 transferability 子测度
    pub transferability_2: [f64; 2],
}

impl AsiV05Scores {
    /// 从 24 维 trace 投影到旧 5 维 (按 category 取均值)。
    pub fn from_trace(trace: &DimensionTrace) -> Self {
        let d = &trace.v05_dims;
        Self {
            continuity: (d[0] + d[1] + d[2] + d[3] + d[4]) / 5.0,
            salience: (d[5] + d[6] + d[7] + d[8] + d[9]) / 5.0,
            identity: (d[10] + d[11] + d[12] + d[13] + d[14]) / 5.0,
            philosophy_guard: (d[15] + d[16] + d[17] + d[18] + d[19]) / 5.0,
            transferability: (d[20] + d[21] + d[22] + d[23]) / 4.0,
        }
    }
}

impl V1136Submeasures {
    /// 从 9 子测度 trace 投影到旧 7 维布局。
    pub fn from_trace(trace: &DimensionTrace) -> Self {
        let s = &trace.v1136_subs;
        Self {
            continuity_5: [s[0], s[1], s[2], s[3], s[4]],
            transferability_2: [s[5], s[6]],
        }
    }
}

/// DimensionTrace: 单次完整测量的 24 维 + 9 子测度快照。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DimensionTrace {
    /// 全局唯一 trace id (递增 u64, 由 TraceRepository 分配)。
    pub trace_id: u64,
    /// 关联的 sample id (一次测量 = 一个 sample)。
    pub sample_id: u64,
    /// 时间戳 (epoch 秒)。
    pub timestamp: i64,
    /// V0.5 24 维真实测量值 (按 V05_DIMENSION_NAMES 顺序)。
    pub v05_dims: [f64; V05_DIM_COUNT],
    /// V1136 9 子测度真实测量值 (按 V1136_SUBMEASURE_NAMES 顺序)。
    pub v1136_subs: [f64; V1136_SUBMEASURE_COUNT],
    /// 可选 hook 覆盖记录 (dim/sub name → overridden value)。
    pub hook_overrides: Vec<(String, f64)>,
}

impl DimensionTrace {
    /// 从 MeasurementSample 构造 DimensionTrace, 调用全部 24+9 measure_* 函数。
    /// 若提供 hook, 则 hook.override_* 返回的 Some(value) 覆盖对应位置。
    pub fn from_sample(
        trace_id: u64,
        sample_id: u64,
        timestamp: i64,
        sample: &MeasurementSample,
        hook: Option<&dyn MeasurementHook>,
    ) -> Self {
        let registry = DimensionRegistry::new();
        let raw_dims = registry.compute_all_dims(sample);
        let raw_subs = registry.compute_all_subs(sample);

        let mut v05_dims = raw_dims;
        let mut v1136_subs = raw_subs;
        let mut hook_overrides = Vec::new();

        if let Some(h) = hook {
            for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
                if let Some(v) = h.override_dim(name, v05_dims[i]) {
                    v05_dims[i] = v;
                    hook_overrides.push(((*name).to_string(), v));
                }
            }
            for (i, name) in V1136_SUBMEASURE_NAMES.iter().enumerate() {
                if let Some(v) = h.override_sub(name, v1136_subs[i]) {
                    v1136_subs[i] = v;
                    hook_overrides.push(((*name).to_string(), v));
                }
            }
        }

        Self {
            trace_id,
            sample_id,
            timestamp,
            v05_dims,
            v1136_subs,
            hook_overrides,
        }
    }

    /// 按名字查 24 维中的某一维 (None 表示未找到)。
    pub fn dim_by_name(&self, name: &str) -> Option<f64> {
        V05_DIMENSION_NAMES
            .iter()
            .position(|n| *n == name)
            .map(|i| self.v05_dims[i])
    }

    /// 按名字查 9 子测度中的某一维 (None 表示未找到)。
    pub fn sub_by_name(&self, name: &str) -> Option<f64> {
        V1136_SUBMEASURE_NAMES
            .iter()
            .position(|n| *n == name)
            .map(|i| self.v1136_subs[i])
    }

    /// 计算 24 维平均值。
    pub fn mean_v05(&self) -> f64 {
        self.v05_dims.iter().sum::<f64>() / V05_DIM_COUNT as f64
    }

    /// 计算 9 子测度平均值。
    pub fn mean_v1136(&self) -> f64 {
        self.v1136_subs.iter().sum::<f64>() / V1136_SUBMEASURE_COUNT as f64
    }
}

/// 占位函数 — 标注 round10-12 落地状态。
pub fn placeholder() -> &'static str {
    "apeireth-asi R14 Phase 2 — V0.5 24 维 + V1136 9 子测度真实测量 (round10-12 qa_engineer)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_count_is_24_locked() {
        assert_eq!(V05_DIM_COUNT, 24);
        assert_eq!(V05_DIMENSION_NAMES.len(), 24);
    }

    #[test]
    fn sub_count_is_9_locked() {
        assert_eq!(V1136_SUBMEASURE_COUNT, 9);
        assert_eq!(V1136_SUBMEASURE_NAMES.len(), 9);
    }

    #[test]
    fn dimension_names_unique() {
        let mut sorted = V05_DIMENSION_NAMES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 24, "V05_DIMENSION_NAMES must be unique");
    }

    #[test]
    fn submeasure_names_unique() {
        let mut sorted = V1136_SUBMEASURE_NAMES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 9, "V1136_SUBMEASURE_NAMES must be unique");
    }

    #[test]
    fn placeholder_describes_round10_12() {
        assert!(placeholder().contains("24"));
        assert!(placeholder().contains("9"));
    }

    #[test]
    fn dim_by_name_roundtrip() {
        let trace = DimensionTrace {
            trace_id: 1,
            sample_id: 1,
            timestamp: 1_700_000_000,
            v05_dims: [0.5; V05_DIM_COUNT],
            v1136_subs: [0.5; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        };
        for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
            assert_eq!(trace.dim_by_name(name), Some(0.5));
            assert_eq!(trace.v05_dims[i], 0.5);
        }
        assert_eq!(trace.dim_by_name("not.a.real.dim"), None);
    }

    #[test]
    fn mean_v05_with_uniform_values() {
        let trace = DimensionTrace {
            trace_id: 1,
            sample_id: 1,
            timestamp: 0,
            v05_dims: [0.7; V05_DIM_COUNT],
            v1136_subs: [0.7; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        };
        assert!((trace.mean_v05() - 0.7).abs() < 1e-9);
        assert!((trace.mean_v1136() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn legacy_v05_scores_projection() {
        let trace = DimensionTrace {
            trace_id: 1,
            sample_id: 1,
            timestamp: 0,
            v05_dims: [0.8; V05_DIM_COUNT],
            v1136_subs: [0.6; V1136_SUBMEASURE_COUNT],
            hook_overrides: vec![],
        };
        let scores = AsiV05Scores::from_trace(&trace);
        assert!((scores.continuity - 0.8).abs() < 1e-9);
        assert!((scores.salience - 0.8).abs() < 1e-9);
        assert!((scores.transferability - 0.8).abs() < 1e-9);
    }

    #[test]
    fn legacy_v1136_submeasures_projection() {
        let trace = DimensionTrace {
            trace_id: 1,
            sample_id: 1,
            timestamp: 0,
            v05_dims: [0.5; V05_DIM_COUNT],
            v1136_subs: [0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.1, 0.2],
            hook_overrides: vec![],
        };
        let subs = V1136Submeasures::from_trace(&trace);
        assert_eq!(subs.continuity_5, [0.3, 0.4, 0.5, 0.6, 0.7]);
        assert_eq!(subs.transferability_2, [0.8, 0.9]);
    }
}

