pub mod domain;
pub mod naming;
pub mod philosophy;
pub mod bus;
pub mod lbr;
pub mod i18n;
pub mod clock;
pub mod lifecycle;

// v1-compatible surface (kept for cognition / action / governance / life-force / sovereignty)
pub mod action_target;
pub mod verdict_v1;

// Re-export v1-style types at crate root for easy import
pub use action_target::ActionTarget;
pub use verdict_v1::{
    verdict_for_target, PhilosophyKey, PhilosophyVerdict, ALL_THIRTEEN_KEYS, ALL_TWELVE_KEYS,
};

// v1 R14 surface — IdentityCard (continuity_id / birth_time / carriers / migration_history)
// + Migration. apeireth-life-force + apeireth-sovereignty 直接 import.
pub use domain::{IdentityCard, Migration};
