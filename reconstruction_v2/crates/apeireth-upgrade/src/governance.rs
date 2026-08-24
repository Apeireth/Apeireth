//! 5 重治理 (v6 §102: 修改 E 层时第 6 层防御).
//!
//! 5 重治理 = 5 个机制分类 (v1 历史保留):
//! 1. 编译时 hardcode
//! 2. 运行时拦截
//! 3. 多 AI 审议
//! 4. 物理隔离 (委托给 sandbox)
//! 5. 反思期审计
//!
//! 本模块提供 trait 框架 + 默认实现 (接受非 E 层修改, E 层修改需 5 重全部通过).

use crate::manifest::UpgradeManifest;

/// 5 重治理 verdict — 单一维度守门结果.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceVerdict {
    /// 编译时 hardcode 通过.
    CompileTime,
    /// 运行时拦截通过.
    Runtime,
    /// 多 AI 审议通过.
    MultiAi,
    /// 物理隔离通过 (委托给 sandbox).
    PhysicalIsolation,
    /// 反思期审计通过.
    ReflectionPeriod,
    /// 守门失败 (含失败维度).
    Failed(GovernanceDimension),
}

/// 治理维度.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceDimension {
    /// 编译时 hardcode.
    CompileTime,
    /// 运行时拦截.
    Runtime,
    /// 多 AI 审议.
    MultiAi,
    /// 物理隔离.
    PhysicalIsolation,
    /// 反思期审计.
    ReflectionPeriod,
}

/// 5 重治理决策.
#[derive(Debug, Clone)]
pub enum GovernanceDecision {
    /// 接受 (非 E 层修改或 5 重全通过).
    Accept,
    /// 拒绝 (5 重中任一失败).
    Reject(GovernanceVerdict),
}

/// 5 重治理 trait — 评估升级 manifest.
pub trait Governance {
    /// 评估 manifest 是否可通过 5 重治理.
    fn evaluate(&self, manifest: &UpgradeManifest) -> GovernanceDecision;
}

/// 默认 5 重治理实现.
#[derive(Debug, Default, Clone, Copy)]
pub struct FiveFoldGovernance;

impl FiveFoldGovernance {
    /// 构造新实例.
    pub fn new() -> Self {
        Self
    }

    /// 校验单一治理维度 (占位 — 完整实现需接入 v6 §177 五重守门细节).
    fn check_dimension(
        &self,
        manifest: &UpgradeManifest,
        dim: GovernanceDimension,
    ) -> GovernanceVerdict {
        // 占位: 非 E 层修改视为通过; E 层修改需调用具体守门实现 (A18 范围)
        if manifest.kind == crate::manifest::UpgradeKind::ELayerMutation {
            // E 层修改默认拒绝 (保守基线 — 主人确认后才放行)
            GovernanceVerdict::Failed(dim)
        } else {
            match dim {
                GovernanceDimension::CompileTime => GovernanceVerdict::CompileTime,
                GovernanceDimension::Runtime => GovernanceVerdict::Runtime,
                GovernanceDimension::MultiAi => GovernanceVerdict::MultiAi,
                GovernanceDimension::PhysicalIsolation => GovernanceVerdict::PhysicalIsolation,
                GovernanceDimension::ReflectionPeriod => GovernanceVerdict::ReflectionPeriod,
            }
        }
    }
}

impl Governance for FiveFoldGovernance {
    fn evaluate(&self, manifest: &UpgradeManifest) -> GovernanceDecision {
        let dims = [
            GovernanceDimension::CompileTime,
            GovernanceDimension::Runtime,
            GovernanceDimension::MultiAi,
            GovernanceDimension::PhysicalIsolation,
            GovernanceDimension::ReflectionPeriod,
        ];
        for dim in dims {
            let verdict = self.check_dimension(manifest, dim);
            if let GovernanceVerdict::Failed(_) = verdict {
                return GovernanceDecision::Reject(verdict);
            }
        }
        GovernanceDecision::Accept
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ManifestBuilder, UpgradeKind};

    fn patch_manifest() -> UpgradeManifest {
        ManifestBuilder::new("v1.0.0", UpgradeKind::Patch)
            .with_description("governance test")
            .with_content_hash("hash")
            .build()
    }

    #[test]
    fn five_fold_governance_accepts_patch() {
        let m = patch_manifest();
        let g = FiveFoldGovernance::new();
        let decision = g.evaluate(&m);
        assert!(matches!(decision, GovernanceDecision::Accept));
    }

    #[test]
    fn five_fold_governance_rejects_e_layer_by_default() {
        let m = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation).build();
        let g = FiveFoldGovernance::new();
        let decision = g.evaluate(&m);
        assert!(matches!(decision, GovernanceDecision::Reject(_)));
    }

    #[test]
    fn governance_dimension_variants_count() {
        let dims = [
            GovernanceDimension::CompileTime,
            GovernanceDimension::Runtime,
            GovernanceDimension::MultiAi,
            GovernanceDimension::PhysicalIsolation,
            GovernanceDimension::ReflectionPeriod,
        ];
        assert_eq!(dims.len(), 5);
    }

    #[test]
    fn governance_verdict_failed_carries_dimension() {
        let v = GovernanceVerdict::Failed(GovernanceDimension::CompileTime);
        match v {
            GovernanceVerdict::Failed(d) => assert_eq!(d, GovernanceDimension::CompileTime),
            _ => panic!("expected Failed variant"),
        }
    }
}
