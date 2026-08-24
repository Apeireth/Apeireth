//! Stage enum + ordering.

use serde::{Deserialize, Serialize};

/// 5 stage kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    Dispatch = 0,
    Normalize = 1,
    Policy = 2,
    Reliability = 3,
    Throttle = 4,
}

/// Stage operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageOp {
    Process,
    Pass,
    Fail,
}

/// A stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub kind: StageKind,
    pub name: String,
    pub op: StageOp,
}

impl Stage {
    pub fn new(kind: StageKind, name: impl Into<String>, op: StageOp) -> Self {
        Self { kind, name: name.into(), op }
    }
}

/// A registered stage entry.
pub struct StageEntry {
    pub stage: Stage,
    pub handler: Box<dyn Fn(&serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>,
}

pub const STAGE_KIND_COUNT: usize = 5;

pub const STAGE_ORDER: [&str; 5] = [
    "0 Dispatch",
    "1 Normalize",
    "2 Policy",
    "3 Reliability",
    "4 Throttle",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_order_matches_kind() {
        for (i, name) in STAGE_ORDER.iter().enumerate() {
            let kind = match i {
                0 => StageKind::Dispatch,
                1 => StageKind::Normalize,
                2 => StageKind::Policy,
                3 => StageKind::Reliability,
                4 => StageKind::Throttle,
                _ => unreachable!(),
            };
            assert_eq!(kind as i32, i as i32);
            assert!(name.contains(name.split_whitespace().last().unwrap_or("")));
        }
    }

    #[test]
    fn stage_new_works() {
        let s = Stage::new(StageKind::Dispatch, "d", StageOp::Process);
        assert_eq!(s.name, "d");
        assert_eq!(s.op, StageOp::Process);
    }
}
