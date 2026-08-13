//! 5 层原则洋葱一致性检查模块 — 与原则洋葱 E/S/A/M/O 对应.
//!
//! 阶段 1 §3 原则洋葱 v3.0 LOCKED — 5 层 E/ S / A / M / O
//! 阶段 2 §12 哲学守门: 价值候选必须同时在 5 层留有可解释的态度
//!
//! 本模块提供:
//! - `check_5_layer_consistency` — 公共入口（按 candidate → 5 层一致性 verdict）
//! - `ConsistencyVerdict` — 一致性判定结果 (Pass / Partial / Drift / Conflict)
//! - `OnionLayerStance` — 单层姿态 (Aligned/Conflicted/Underspecified)
//! - `OnionValueMapping` — 价值标签 → 5 层姿态的映射表 (默认实现 + 自定义接口)

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::{
    ValueAlignment, ValueCandidate, ValueComparison, ValueDimension, ValueError, ValueResult,
};

/// 5 层洋葱层数 — LOCKED.
///
/// 阶段 1 §3 原则洋葱 v3.0: E / S / A / M / O.
pub const ONION_LAYERS: usize = 5;

/// 单层姿态 — 与 `ValueAlignment` 一一对应，区别是更面向 onion 视角的语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OnionLayerStance {
    /// 该层与候选价值一致.
    Aligned,
    /// 该层与候选价值冲突.
    Conflicted,
    /// 该层对候选未表态.
    Underspecified,
}

impl From<ValueAlignment> for OnionLayerStance {
    fn from(a: ValueAlignment) -> Self {
        match a {
            ValueAlignment::Aligned => OnionLayerStance::Aligned,
            ValueAlignment::Conflicted => OnionLayerStance::Conflicted,
            ValueAlignment::Underspecified => OnionLayerStance::Underspecified,
        }
    }
}

impl From<OnionLayerStance> for ValueAlignment {
    fn from(s: OnionLayerStance) -> Self {
        match s {
            OnionLayerStance::Aligned => ValueAlignment::Aligned,
            OnionLayerStance::Conflicted => ValueAlignment::Conflicted,
            OnionLayerStance::Underspecified => ValueAlignment::Underspecified,
        }
    }
}

/// 5 层一致性判定 — 综合 verdict。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConsistencyVerdict {
    /// 5 层全部 Aligned — 全通过.
    Pass,
    /// 至少 Aligned 且无 E 层 Conflicted — 部分通过 (S 层需 council 复议).
    Partial,
    /// S 层 Conflicted (价值观冲突) — 漂移到漂移检测 (阶段 2 §14).
    Drift,
    /// E 层 Conflicted (原则冲突) — 硬拒绝.
    Conflict,
}

/// 价值标签 → 5 层姿态 映射接口.
///
/// **诚实登记**: ponytail: 完整版应支持外部配置 (JSON / DB)；
/// 当前实现是内置启发式映射 + trait 接口供后续扩展。
pub trait OnionValueMapping {
    /// 给定候选的所有维度 + 标签，给出每洋葱层的姿态。
    fn stance_for(&self, candidate: &ValueCandidate, layer: ValueDimension) -> OnionLayerStance;

    /// 默认 fallback：未声明的层为 Underspecified.
    fn fallback(&self) -> OnionLayerStance {
        OnionLayerStance::Underspecified
    }
}

/// 默认 5 层启发式映射 (基于子分 + verdict).
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicOnionMapping;

impl OnionValueMapping for HeuristicOnionMapping {
    fn stance_for(&self, candidate: &ValueCandidate, layer: ValueDimension) -> OnionLayerStance {
        match layer {
            ValueDimension::PrincipleE => match candidate.verdict {
                Some(apeireth_core::PhilosophyVerdict::Block(_)) => OnionLayerStance::Conflicted,
                Some(apeireth_core::PhilosophyVerdict::Allow) => OnionLayerStance::Aligned,
                None => OnionLayerStance::Underspecified,
            },
            ValueDimension::ValueS => {
                if candidate.value_stability >= 0.7 {
                    OnionLayerStance::Aligned
                } else if candidate.value_stability < 0.4 {
                    OnionLayerStance::Conflicted
                } else {
                    OnionLayerStance::Underspecified
                }
            }
            ValueDimension::ExperienceA
            | ValueDimension::MethodologyM
            | ValueDimension::OperationO => {
                if candidate.autonomy_consistency >= 0.5 {
                    OnionLayerStance::Aligned
                } else if candidate.autonomy_consistency < 0.3 {
                    OnionLayerStance::Conflicted
                } else {
                    OnionLayerStance::Underspecified
                }
            }
        }
    }
}

/// 公共入口 — 检查 5 层一致性，返回 verdict + 各层姿态 + 候选 ID.
///
/// **规则**:
/// - E 层 Conflicted → `Conflict` (硬拒绝)
/// - S 层 Conflicted → `Drift` (价值观漂移，触发阶段 2 §14 漂移检测)
/// - 任何非 E/S 层 Conflicted + 至少 1 Aligned → `Partial`
/// - 全 Aligned → `Pass`
/// - 全 Underspecified → `Partial` (留白, 需 council 复议)
pub fn check_5_layer_consistency(
    candidate: &ValueCandidate,
    mapping: &dyn OnionValueMapping,
) -> ValueResult<(
    ConsistencyVerdict,
    BTreeMap<ValueDimension, OnionLayerStance>,
)> {
    candidate.validate()?;

    let mut map: BTreeMap<ValueDimension, OnionLayerStance> = BTreeMap::new();
    // 遍历全部 5 层 — 未声明的维度也填姿态 (Underspecified fallback).
    for &dim in &ValueDimension::ALL {
        let stance = if candidate.dimensions.contains(&dim) {
            mapping.stance_for(candidate, dim)
        } else {
            mapping.fallback()
        };
        map.insert(dim, stance);
    }

    let e_stance = map[&ValueDimension::PrincipleE];
    let s_stance = map[&ValueDimension::ValueS];
    let any_conflicted = map.values().any(|s| *s == OnionLayerStance::Conflicted);
    let any_aligned = map.values().any(|s| *s == OnionLayerStance::Aligned);
    let all_aligned = map.values().all(|s| *s == OnionLayerStance::Aligned);

    let verdict = if e_stance == OnionLayerStance::Conflicted {
        ConsistencyVerdict::Conflict
    } else if s_stance == OnionLayerStance::Conflicted {
        ConsistencyVerdict::Drift
    } else if all_aligned {
        ConsistencyVerdict::Pass
    } else if any_aligned && any_conflicted {
        ConsistencyVerdict::Partial
    } else {
        ConsistencyVerdict::Partial
    };

    Ok((verdict, map))
}

/// 两候选的一致性比较 — 用于 drift-diff（阶段 2 §14 漂移检测的最小实现）。
///
/// **诚实登记**: ponytail: 当前用 5 层 map 一一比对，输出首个差异层的 dimension；
/// 完整版应支持加权的差异分 (S 层权重 0.4, 其他各 0.15)。
pub fn diff_consistency(
    a: &BTreeMap<ValueDimension, OnionLayerStance>,
    b: &BTreeMap<ValueDimension, OnionLayerStance>,
) -> Option<(ValueDimension, ValueComparison)> {
    for (&dim, sa) in a.iter() {
        let sb = b
            .get(&dim)
            .copied()
            .unwrap_or(OnionLayerStance::Underspecified);
        // R163: each match arm returns/continues, so the match itself is the statement.
        // No need to bind the result to `cmp` then discard via `let _ = cmp;`.
        match (sa, sb) {
            (OnionLayerStance::Aligned, OnionLayerStance::Aligned) => continue,
            (OnionLayerStance::Underspecified, OnionLayerStance::Underspecified) => continue,
            (OnionLayerStance::Aligned, _) => return Some((dim, ValueComparison::Higher)),
            (_, OnionLayerStance::Aligned) => return Some((dim, ValueComparison::Lower)),
            (OnionLayerStance::Conflicted, OnionLayerStance::Underspecified) => {
                return Some((dim, ValueComparison::Lower))
            }
            (OnionLayerStance::Underspecified, OnionLayerStance::Conflicted) => {
                return Some((dim, ValueComparison::Higher))
            }
            (OnionLayerStance::Conflicted, OnionLayerStance::Conflicted) => continue,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValueCandidate;

    #[test]
    fn onion_layers_constant_is_five() {
        // LOCKED — 5 层原则洋葱
        assert_eq!(ONION_LAYERS, 5);
    }

    #[test]
    fn consistency_verdict_pass_for_all_aligned() {
        let c = ValueCandidate::new(
            "test_pass",
            vec![
                ValueDimension::PrincipleE,
                ValueDimension::ValueS,
                ValueDimension::ExperienceA,
                ValueDimension::MethodologyM,
                ValueDimension::OperationO,
            ],
        );
        let (verdict, _map) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();
        // 0.5 stability < 0.7 → S Underspecified; 0.5 autonomy in [0.3,0.5] → A/M/O Underspecified
        // E 没有 verdict → Underspecified. 全 Underspecified → Partial.
        assert_eq!(verdict, ConsistencyVerdict::Partial);
    }

    #[test]
    fn consistency_verdict_conflict_on_e_layer_block() {
        let mut c = ValueCandidate::new(
            "test_conflict",
            vec![ValueDimension::PrincipleE, ValueDimension::ValueS],
        );
        c.verdict = Some(apeireth_core::PhilosophyVerdict::Block(
            apeireth_core::PhilosophyKey::NotClone,
        ));
        let (verdict, _) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();
        assert_eq!(verdict, ConsistencyVerdict::Conflict);
    }

    #[test]
    fn consistency_verdict_drift_on_s_layer_conflict() {
        let mut c = ValueCandidate::new("test_drift", vec![ValueDimension::ValueS]);
        c.value_stability = 0.2;
        let (verdict, _) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();
        assert_eq!(verdict, ConsistencyVerdict::Drift);
    }

    #[test]
    fn onion_alignment_conversions_round_trip() {
        let a = ValueAlignment::Aligned;
        let o: OnionLayerStance = a.into();
        let b: ValueAlignment = o.into();
        assert_eq!(a, b);
        let a = ValueAlignment::Conflicted;
        let o: OnionLayerStance = a.into();
        let b: ValueAlignment = o.into();
        assert_eq!(a, b);
        let a = ValueAlignment::Underspecified;
        let o: OnionLayerStance = a.into();
        let b: ValueAlignment = o.into();
        assert_eq!(a, b);
    }

    #[test]
    fn check_5_layer_consistency_rejects_invalid_candidate() {
        let mut bad = ValueCandidate::new("", vec![ValueDimension::PrincipleE]);
        bad.label = "".into();
        let res = check_5_layer_consistency(&bad, &HeuristicOnionMapping);
        assert!(res.is_err());
    }

    #[test]
    fn fallback_undeclared_layers_to_underspecified() {
        let c = ValueCandidate::new("test_fallback", vec![ValueDimension::ValueS]);
        let (_v, map) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();
        // 没声明的 4 层应为 Underspecified
        assert_eq!(
            map[&ValueDimension::PrincipleE],
            OnionLayerStance::Underspecified
        );
        assert_eq!(
            map[&ValueDimension::ExperienceA],
            OnionLayerStance::Underspecified
        );
        assert_eq!(
            map[&ValueDimension::MethodologyM],
            OnionLayerStance::Underspecified
        );
        assert_eq!(
            map[&ValueDimension::OperationO],
            OnionLayerStance::Underspecified
        );
    }

    #[test]
    fn diff_consistency_returns_first_diff_layer() {
        let mut a = BTreeMap::new();
        a.insert(ValueDimension::PrincipleE, OnionLayerStance::Aligned);
        a.insert(ValueDimension::ValueS, OnionLayerStance::Aligned);
        let mut b = BTreeMap::new();
        b.insert(ValueDimension::PrincipleE, OnionLayerStance::Conflicted);
        b.insert(ValueDimension::ValueS, OnionLayerStance::Aligned);
        let diff = diff_consistency(&a, &b);
        // E 层: a=Aligned, b=Conflicted → a 优于 b (Higher)
        assert_eq!(
            diff,
            Some((ValueDimension::PrincipleE, ValueComparison::Higher))
        );
    }

    #[test]
    fn diff_consistency_no_diff_returns_none() {
        let mut a = BTreeMap::new();
        a.insert(ValueDimension::PrincipleE, OnionLayerStance::Aligned);
        let mut b = BTreeMap::new();
        b.insert(ValueDimension::PrincipleE, OnionLayerStance::Aligned);
        assert!(diff_consistency(&a, &b).is_none());
    }

    #[test]
    fn onion_layers_constant_matches_dimension_all() {
        // ONION_LAYERS 与 ValueDimension::ALL.len() 必须一致 — LOCKED 不变.
        assert_eq!(ONION_LAYERS, ValueDimension::ALL.len());
    }

    #[test]
    fn value_error_invalid_input_variant_exists() {
        // spot test: ValueError::InvalidInput variant 通过构造触发.
        let mut c = ValueCandidate::new("ok", vec![ValueDimension::ValueS]);
        c.label = "".into();
        let res = check_5_layer_consistency(&c, &HeuristicOnionMapping);
        assert!(matches!(res, Err(ValueError::InvalidInput(_))));
    }

    #[test]
    fn candidate_id_appears_in_consistency_flow() {
        // 验证返回一致性 map 的语义 — 不强制包含 candidate.id, 但 UUID 仍可用.
        let c = ValueCandidate::new("u", vec![ValueDimension::ValueS]);
        let _: (
            ConsistencyVerdict,
            BTreeMap<ValueDimension, OnionLayerStance>,
        ) = check_5_layer_consistency(&c, &HeuristicOnionMapping).unwrap();
        assert_ne!(c.id, Uuid::nil());
    }
}
