//! Sub-supervisor + 5-kind classification + default plan.
//!
//! `SubSupervisorKind` is the public taxonomy of supervision subtrees. The
//! `default_plan()` returns the canonical 5-subtree + 21-child-spec plan
//! matching the eve design.

use crate::child::ChildSpec;
use crate::strategy::RestartStrategy;

/// 5 sub-supervisor kinds (canonical taxonomy).
///
/// ponytail: this is an enum, not a string; the ceiling is dynamic registration,
/// the upgrade path is `Box<dyn SubSupervisorKind>`; not needed yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubSupervisorKind {
    /// 3 child specs — core organs (perception, action, memory).
    Core,
    /// 4 child specs — cognition + reasoning + reflection.
    Cognition,
    /// 7 child specs — council + 7 advisors.
    Council,
    /// 3 child specs — upgrade pipeline + sandbox + manifest.
    Upgrade,
    /// 4 child specs — extension host + 4 plugin slots.
    Plugin,
}

impl SubSupervisorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Cognition => "Cognition",
            Self::Council => "Council",
            Self::Upgrade => "Upgrade",
            Self::Plugin => "Plugin",
        }
    }

    pub fn default_strategy(&self) -> RestartStrategy {
        match self {
            Self::Core | Self::Council | Self::Plugin => RestartStrategy::OneForOne,
            Self::Cognition => RestartStrategy::RestForOne,
            Self::Upgrade => RestartStrategy::Transient,
        }
    }

    pub fn default_count(&self) -> usize {
        match self {
            Self::Core => 3,
            Self::Cognition => 4,
            Self::Council => 7,
            Self::Upgrade => 3,
            Self::Plugin => 4,
        }
    }
}

/// Default child specs for a given kind (placeholder IDs/labels — labels are descriptive).
///
/// ponytail: trivial builders; the ceiling is "pull from live registry", the
/// upgrade path is a `fn() -> Vec<ChildSpec>` injected at startup.
pub fn default_plan() -> Vec<(SubSupervisorKind, Vec<ChildSpec>)> {
    let mut plan = Vec::new();

    // Core: 3 children, OneForOne
    plan.push((
        SubSupervisorKind::Core,
        vec![
            ChildSpec::new(
                "core.perception",
                "core.perception",
                "true",
                RestartStrategy::OneForOne,
            ),
            ChildSpec::new(
                "core.action",
                "core.action",
                "true",
                RestartStrategy::OneForOne,
            ),
            ChildSpec::new(
                "core.memory",
                "core.memory",
                "true",
                RestartStrategy::OneForOne,
            ),
        ],
    ));

    // Cognition: 4 children, RestForOne
    plan.push((
        SubSupervisorKind::Cognition,
        vec![
            ChildSpec::new(
                "cog.cognition",
                "cog.cognition",
                "true",
                RestartStrategy::RestForOne,
            ),
            ChildSpec::new(
                "cog.intuition",
                "cog.intuition",
                "true",
                RestartStrategy::RestForOne,
            ),
            ChildSpec::new(
                "cog.reasoning",
                "cog.reasoning",
                "true",
                RestartStrategy::RestForOne,
            ),
            ChildSpec::new("cog.meta", "cog.meta", "true", RestartStrategy::RestForOne),
        ],
    ));

    // Council: 7 children (1 supervisor + 7 advisor slots wait that's actually 7+1... the spec says 7 advisor slots total, so 7 children), OneForOne
    let mut council = Vec::new();
    for i in 0..7 {
        council.push(ChildSpec::new(
            format!("council.advisor.{}", i),
            format!("council.advisor.{}", i),
            "true",
            RestartStrategy::OneForOne,
        ));
    }
    plan.push((SubSupervisorKind::Council, council));

    // Upgrade: 3 children, Transient
    plan.push((
        SubSupervisorKind::Upgrade,
        vec![
            ChildSpec::new(
                "upg.pipeline",
                "upg.pipeline",
                "true",
                RestartStrategy::Transient,
            ),
            ChildSpec::new(
                "upg.sandbox",
                "upg.sandbox",
                "true",
                RestartStrategy::Transient,
            ),
            ChildSpec::new(
                "upg.manifest",
                "upg.manifest",
                "true",
                RestartStrategy::Transient,
            ),
        ],
    ));

    // Plugin: 4 children, OneForOne
    plan.push((
        SubSupervisorKind::Plugin,
        vec![
            ChildSpec::new("plg.host", "plg.host", "true", RestartStrategy::OneForOne),
            ChildSpec::new(
                "plg.extension.0",
                "plg.extension.0",
                "true",
                RestartStrategy::OneForOne,
            ),
            ChildSpec::new(
                "plg.extension.1",
                "plg.extension.1",
                "true",
                RestartStrategy::OneForOne,
            ),
            ChildSpec::new(
                "plg.extension.2",
                "plg.extension.2",
                "true",
                RestartStrategy::OneForOne,
            ),
        ],
    ));

    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_kinds_present() {
        let plan = default_plan();
        let kinds: Vec<SubSupervisorKind> = plan.iter().map(|(k, _)| *k).collect();
        assert!(kinds.contains(&SubSupervisorKind::Core));
        assert!(kinds.contains(&SubSupervisorKind::Cognition));
        assert!(kinds.contains(&SubSupervisorKind::Council));
        assert!(kinds.contains(&SubSupervisorKind::Upgrade));
        assert!(kinds.contains(&SubSupervisorKind::Plugin));
    }

    #[test]
    fn total_child_specs_match_eve_design() {
        let plan = default_plan();
        let total: usize = plan.iter().map(|(_, specs)| specs.len()).sum();
        assert_eq!(total, 21, "expected 3+4+7+3+4 = 21");
    }
}
