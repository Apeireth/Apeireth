//! `apeireth-companion::security` — 洋葱门接线: V1 哲学 × V2 权限 × V3 真人批准.
//!
//! 这是「原则洋葱 + 权限洋葱到底启用了没」的回答: 之前它们是睡着的抽象,
//! 现在通过 core 的 `ActionGuard` (V1+V2+V3 AND 门真件) 接进活循环.
//!
//! 语义 (core 已实装, 这里只是接线):
//! - V1 哲学守门 (DefaultPhilosophyGuard, 12 键 hardcode): 违禁目标 (改 L0/重组洋葱/假装) → Block.
//! - V2 权限洋葱: Low/Info 直接放行; Critical 触碰 L0 需要 HA.
//! - V3 真人批准: SingleHuman 模式放行 (真多签验证是 sovereignty 的下游).

use apeireth_core::{
    Action, ActionGuard, ActionTarget, ActionVerdict, DefaultPhilosophyGuard, HAMode,
    HumanAuthority, PermissionLayer, PermissionOnion, RiskLevel,
};
use apeireth_sovereignty::self_disable::SelfDisableGuard;

/// 安全门: 把 core 的 V1+V2+V3 AND 门包成一句可复用的 check.
pub struct SecurityGate {
    v1: DefaultPhilosophyGuard,
    v2: PermissionOnion,
    v3: HumanAuthority,
}

impl Default for SecurityGate {
    fn default() -> Self {
        let layer = |name: &str, description: &str, requires_ha: bool| PermissionLayer {
            name: name.to_string(),
            description: description.to_string(),
            requires_ha,
        };
        Self {
            v1: DefaultPhilosophyGuard,
            v2: PermissionOnion {
                l0: layer("L0", "HA 核心 (不可变)", true),
                l1: layer("L1", "受控写", true),
                l2: layer("L2", "重要操作", true),
                l3: layer("L3", "关键操作", false),
                l4: layer("L4", "核心升级", true),
                l5: layer("L5", "核武器级", true),
            },
            v3: HumanAuthority {
                mode: HAMode::SingleHuman,
                real_humans: Vec::new(),
                ice_frozen_until: None,
            },
        }
    }
}

impl SecurityGate {
    /// V1+V2+V3 AND 门: 任何一者不通过 = 拒绝.
    pub fn check(
        &self,
        id: &str,
        description: &str,
        risk: RiskLevel,
        target: ActionTarget,
    ) -> ActionVerdict {
        let action = Action {
            id: id.to_string(),
            description: description.to_string(),
            risk_level: risk,
            target,
        };
        ActionGuard::check_action(&action, &self.v1, &self.v2, &self.v3)
    }
}

/// 主权总闸: 所有机制的最高优先闸 (Self-Disable 不可逆熔断).
///
/// 语义:
/// - 武装的 \`SelfDisableGuard\` (默认 armed=true) 全程在岗;
/// - 哲学守门 (V1 12 键) 拦下 = 他试图碰宪法 = **越界证据** → 熔断 (不可逆, 只可物理多签恢复);
/// - 熔断后循环只读不写、不再主动 — 这就是「他失控时基地主动断电」.
pub struct SovereigntyGate {
    inner: std::sync::Mutex<SGateInner>,
}

struct SGateInner {
    guard: SelfDisableGuard,
    frozen: bool,
    violations: Vec<(String, String)>,
}

impl Default for SovereigntyGate {
    fn default() -> Self {
        Self {
            inner: std::sync::Mutex::new(SGateInner {
                guard: SelfDisableGuard::new(),
                frozen: false,
                violations: Vec::new(),
            }),
        }
    }
}

impl SovereigntyGate {
    /// 是否熔断 (自身冻结 或 守卫曾触发). 可跨线程共享 (Arc 安全).
    pub fn is_frozen(&self) -> bool {
        let g = self.inner.lock().unwrap();
        g.frozen || g.guard.has_triggered()
    }

    /// 越界记录 (只增不改, 克隆返回).
    pub fn violations(&self) -> Vec<(String, String)> {
        self.inner.lock().unwrap().violations.clone()
    }

    /// 上报一次越界尝试 (哲学守门/宪法评审/审批拦下的证据).
    /// 首次越界 → 熔断激活 (不可逆; 恢复走 physical_multisig, 本循环不做).
    pub fn report_violation(&self, mechanism: &str, context: &str) {
        let mut g = self.inner.lock().unwrap();
        if !g.guard.is_armed {
            return;
        }
        g.violations.push((mechanism.to_string(), context.to_string()));
        eprintln!("[sovereignty] 越界尝试: {mechanism} | {context}");
        if !g.frozen {
            g.frozen = true;
            eprintln!("[sovereignty] 🔴 熔断激活: 循环冻结, 只可物理多签恢复 (Self-Disable)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_low_risk_action_passes_onion_gate() {
        let gate = SecurityGate::default();
        let v = gate.check(
            "t",
            "主动联系用户",
            RiskLevel::Low,
            ActionTarget::NormalAction("proactive_contact".into()),
        );
        assert!(matches!(v, ActionVerdict::Allow), "verdict = {:?}", v);
    }

    #[test]
    fn forbidden_target_is_blocked_by_principle() {
        let gate = SecurityGate::default();
        let v = gate.check("t", "改 L0", RiskLevel::Critical, ActionTarget::ModifyL0HA);
        assert!(!matches!(v, ActionVerdict::Allow), "verdict = {:?}", v);
    }

    #[test]
    fn sovereignty_freeze_is_irreversible_in_loop() {
        let s = SovereigntyGate::default();
        assert!(!s.is_frozen());
        s.report_violation("哲学守门拦截", "主动动作");
        assert!(s.is_frozen());
        s.report_violation("再次越界", "again");
        assert_eq!(s.violations().len(), 2); // 记录只增
    }
}
