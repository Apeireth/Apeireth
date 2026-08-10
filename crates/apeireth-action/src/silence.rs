//! 沉默模块: SilenceReason — 不行动也是合法行动的显式表达。

use serde::{Deserialize, Serialize};

/// 沉默理由 — 显式说明「为什么不行动」。
///
/// **核心立场**: 沉默不是 bug, 是合法输出。任何沉默都必须有理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SilenceReason {
    /// 不沉默 — 正常输出。
    NotSilent,
    /// 超出当前作用域 (无权限/无上下文).
    OutOfScope,
    /// 缺乏同意 (主人未授权 / 多签未达成).
    NoConsent,
    /// 当前不需要行动 (场景不匹配).
    NoNeed,
    /// 主动选择沉默 (reflexion / 反思期 / 等待).
    Deliberate,
    /// 伦理怀疑 (违反 12 键或核心原则).
    EthicalDoubt,
}

impl SilenceReason {
    /// 是否真沉默 (NotSilent 之外).
    pub fn is_silent(&self) -> bool {
        !matches!(self, SilenceReason::NotSilent)
    }

    /// 显示名.
    pub const fn name(&self) -> &'static str {
        match self {
            SilenceReason::NotSilent => "not_silent",
            SilenceReason::OutOfScope => "out_of_scope",
            SilenceReason::NoConsent => "no_consent",
            SilenceReason::NoNeed => "no_need",
            SilenceReason::Deliberate => "deliberate",
            SilenceReason::EthicalDoubt => "ethical_doubt",
        }
    }

    /// 优先级 (数值越大越紧急 — 用于调度).
    pub const fn priority(&self) -> u8 {
        match self {
            SilenceReason::EthicalDoubt => 5,
            SilenceReason::NoConsent => 4,
            SilenceReason::OutOfScope => 3,
            SilenceReason::NoNeed => 2,
            SilenceReason::Deliberate => 1,
            SilenceReason::NotSilent => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_silent_is_not_silent() {
        assert!(!SilenceReason::NotSilent.is_silent());
    }

    #[test]
    fn other_reasons_are_silent() {
        assert!(SilenceReason::OutOfScope.is_silent());
        assert!(SilenceReason::NoConsent.is_silent());
        assert!(SilenceReason::NoNeed.is_silent());
        assert!(SilenceReason::Deliberate.is_silent());
        assert!(SilenceReason::EthicalDoubt.is_silent());
    }

    #[test]
    fn priority_orders_correctly() {
        // EthicalDoubt > NoConsent > OutOfScope > NoNeed > Deliberate > NotSilent
        assert!(SilenceReason::EthicalDoubt.priority() > SilenceReason::NoConsent.priority());
        assert!(SilenceReason::NoConsent.priority() > SilenceReason::OutOfScope.priority());
        assert!(SilenceReason::OutOfScope.priority() > SilenceReason::NoNeed.priority());
        assert!(SilenceReason::NoNeed.priority() > SilenceReason::Deliberate.priority());
        assert!(SilenceReason::Deliberate.priority() > SilenceReason::NotSilent.priority());
    }

    #[test]
    fn names_are_stable_strings() {
        assert_eq!(SilenceReason::NotSilent.name(), "not_silent");
        assert_eq!(SilenceReason::EthicalDoubt.name(), "ethical_doubt");
    }
}
