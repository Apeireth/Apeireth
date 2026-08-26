//! Built-in example specifications.
use crate::{invariant::Invariant, tla::TlaSpec};
pub fn double_onion_spec()->TlaSpec{TlaSpec{name:"DoubleOnion".into(),variables:vec!["layer".into(),"requires_ha".into()],invariant:"layer = 0 => requires_ha".into()}}
pub fn lock_safety_invariant()->Invariant{crate::invariant::presets::DOUBLE_ONION}
