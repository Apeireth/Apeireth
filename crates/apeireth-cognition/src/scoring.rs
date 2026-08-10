//! ASI 评分 (V0.5 5 维 + V1136 7 子测度).
//!
//! 本模块提供 5+ pub fn 给认知器官打分:
//! - `score_v05` — 5 维评分主入口
//! - `score_v1136` — 7 子测度主入口
//! - 5 个子维度评分函数 (continuity/salience/identity/philosophy_guard/transferability)

use apeireth_asi::{AsiV05Scores, V1136Submeasures};
use chrono::Utc;

use crate::{CognitionError, CognitionResult, CognitiveInput};

/// 给 cognitive input 打 ASI V0.5 5 维评分.
pub fn score_v05(input: &CognitiveInput) -> AsiV05Scores {
    AsiV05Scores {
        continuity: continuity_score(input),
        salience: salience_score(input),
        identity: identity_score(input),
        philosophy_guard: philosophy_guard_score(input),
        transferability: transferability_score(input),
    }
}

/// 给 cognitive input 打 ASI V1136 7 子测度评分.
pub fn score_v1136(input: &CognitiveInput) -> V1136Submeasures {
    // 5 continuity + 2 transferability — 简化: 各维度从 V0.5 映射.
    let v05 = score_v05(input);
    V1136Submeasures {
        continuity_5: [
            v05.continuity,
            v05.identity,
            v05.salience * 0.5,
            v05.philosophy_guard * 0.5,
            (v05.continuity + v05.identity) / 2.0,
        ],
        transferability_2: [v05.transferability, v05.transferability * 0.8],
    }
}

/// Continuity 维度评分 — 跨 session 连续性.
pub fn continuity_score(input: &CognitiveInput) -> f64 {
    // session_id 存在 → 连续性高; 不存在 → 低.
    if input.session_id.is_some() {
        0.85
    } else {
        0.45
    }
}

/// Salience 维度评分 — 记忆显著性.
pub fn salience_score(input: &CognitiveInput) -> f64 {
    // 候选行动数越多 = 信号越丰富.
    match input.candidate_targets.len() {
        0 => 0.0,
        1 => 0.50,
        2..=5 => 0.70,
        _ => 0.90,
    }
}

/// Identity 维度评分 — 身份稳定.
pub fn identity_score(input: &CognitiveInput) -> f64 {
    // context_tag 长度近似身份稳定度 (启发式).
    let len = input.context_tag.chars().count();
    ((len as f64) / 64.0).min(1.0).max(0.1)
}

/// Philosophy-guard 维度评分 — 哲学守门通过率 (本周期内).
///
/// 暂以输入合法率近似 — 完整版需配合 V3 9 键 + v4.1 12 键 verdict.
pub fn philosophy_guard_score(input: &CognitiveInput) -> f64 {
    if input.validate().is_ok() {
        0.95
    } else {
        0.20
    }
}

/// Transferability 维度评分 — 知识迁移能力.
pub fn transferability_score(input: &CognitiveInput) -> f64 {
    // 时间戳距今越近 → 迁移价值越高.
    let now = Utc::now().timestamp();
    let age = (now - input.timestamp).abs() as f64;
    (1.0 / (1.0 + age / 3600.0)).clamp(0.1, 1.0)
}

/// 校验 ASI 评分在 [0.0, 1.0] 范围.
pub fn validate_asi_score(score: f64) -> CognitionResult<()> {
    if !(0.0..=1.0).contains(&score) {
        return Err(CognitionError::AsiOutOfRange(score));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::ActionTarget;

    fn sample_input() -> CognitiveInput {
        let target = ActionTarget::NormalAction("noop".to_string());
        CognitiveInput::new(vec![target], "scoring_test")
    }

    #[test]
    fn continuity_score_depends_on_session_id() {
        let mut input = sample_input();
        assert!(continuity_score(&input) < 0.5);
        input.session_id = Some(uuid::Uuid::new_v4());
        assert!(continuity_score(&input) > 0.5);
    }

    #[test]
    fn salience_score_handles_empty() {
        let input = CognitiveInput::new(vec![], "x");
        // validate 失败但 salience 直接基于 len, 不依赖 validate
        assert_eq!(salience_score(&input), 0.0);
    }

    #[test]
    fn salience_score_handles_single_target() {
        let input = sample_input();
        assert!((salience_score(&input) - 0.50).abs() < 0.01);
    }

    #[test]
    fn identity_score_bounds_in_unit_interval() {
        let input = sample_input();
        let s = identity_score(&input);
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn philosophy_guard_score_high_for_valid_input() {
        let input = sample_input();
        assert!(philosophy_guard_score(&input) > 0.5);
    }

    #[test]
    fn transferability_score_recent_input_is_high() {
        let input = sample_input();
        assert!(transferability_score(&input) > 0.5);
    }

    #[test]
    fn validate_asi_score_accepts_unit_interval() {
        assert!(validate_asi_score(0.0).is_ok());
        assert!(validate_asi_score(0.5).is_ok());
        assert!(validate_asi_score(1.0).is_ok());
    }

    #[test]
    fn validate_asi_score_rejects_out_of_range() {
        assert!(validate_asi_score(-0.1).is_err());
        assert!(validate_asi_score(1.1).is_err());
        assert!(validate_asi_score(2.0).is_err());
    }

    #[test]
    fn score_v05_returns_full_struct() {
        let input = sample_input();
        let v05 = score_v05(&input);
        assert_eq!(v05.continuity.is_finite(), true);
        assert_eq!(v05.salience.is_finite(), true);
        assert_eq!(v05.identity.is_finite(), true);
        assert_eq!(v05.philosophy_guard.is_finite(), true);
        assert_eq!(v05.transferability.is_finite(), true);
    }

    #[test]
    fn score_v1136_returns_full_struct() {
        let input = sample_input();
        let v1136 = score_v1136(&input);
        assert_eq!(v1136.continuity_5.len(), 5);
        assert_eq!(v1136.transferability_2.len(), 2);
    }
}
